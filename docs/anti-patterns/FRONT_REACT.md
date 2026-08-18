# Anti-Pattern Catalog — FRONT_REACT: React 상태·렌더링

이 카탈로그는 더 큰 Anti-Pattern Catalog(`docs/anti-patterns/INDEX.md`, 별도 작성 예정)의 한 파트이며, UI/UX+Tauri Phase 3 웨이브에 속한다. Phase 1에서 먼저 작성된 `docs/anti-patterns/FRONTEND.md`(FE-01~FE-18)가 캔버스 레이어 분리, 좌표계, HiDPI, WebGL 업로드, 가상화 등 "렌더링 메커니즘·레이어 분리" 중심의 프런트엔드-렌더링 연계 이슈를 다뤘다면, 이 문서는 그보다 한 층 위인 "React 상태 아키텍처(store/selector/구독 설계)"와 "렌더 비용(render-cost) 패턴" 자체에 초점을 맞춘다. 즉 FRONTEND.md가 *무엇을 어느 레이어/캔버스에 그릴 것인가*를 다룬다면, 이 문서는 *어떤 상태를 어디에 어떻게 보관하고, 무엇이 다시 렌더링을 유발하는가*를 다룬다. 두 문서는 실제로 여러 항목에서 같은 증상(예: 오버레이 재계산, 레이스 컨디션, 워커 오프로딩)을 서로 다른 각도에서 건드리므로 함께 읽는 것을 권장하며, 아래 각 항목에서 FRONTEND.md의 대응/인접 항목을 명시적으로 교차 참조했다.

카테고리: **FRONT_REACT — React 계열 프런트 상태·렌더링 (State & Rendering Architecture)**. State(`FRONT-STATE-*`)와 Rendering(`FRONT-RENDER-*`) 두 하위 절로 구성된다.

---

## State — 상태 아키텍처

### FRONT-STATE-001: 백엔드 도메인 객체 전체를 global store에 보관

**분류**: 상태 소유권 설계 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```typescript
// invoke 응답 타입을 그대로 전역 store 스키마로 재사용
interface AppState {
  bitstream: BitstreamDomainObject; // NAL 트리, 프레임 배열, MV/QP 원본 등 백엔드 응답 전체
}

const useAppStore = create<AppState>(() => ({ bitstream: EMPTY_BITSTREAM }));

useEffect(() => {
  invoke<BitstreamDomainObject>('parse_bitstream', { path }).then((data) =>
    useAppStore.setState({ bitstream: data }), // 응답을 가공 없이 통째로 저장
  );
}, [path]);
```

**문제**:
- 전체 도메인 객체가 하나의 reactive 슬롯이므로, 그 안의 필드 하나(예: 특정 프레임의 재디코드 결과)만 바뀌어도 `bitstream` 참조 전체가 갱신되어 이를 구독하는 모든 컴포넌트가 리렌더 후보가 된다.
- IPC 응답 스키마가 곧 프런트 상태 스키마가 되어버려, 백엔드 구조체가 바뀌면 그 필드를 실제로 쓰지 않는 화면까지 타입/런타임 영향을 받는다 — 뷰 전용 필드와 도메인 필드가 분리되지 않음.
- raw motion vector 배열처럼 대부분의 화면이 쓰지 않는 대용량 필드까지 store에 상주해 메모리를 점유한다.

**발생 조건**:
- `invoke(...).then(setState)`를 최초로 배선하는 시점에 "일단 다 저장해두고 나중에 골라 쓰자"는 식으로 통합할 때 흔히 생긴다.

**권장**:
```typescript
// 정규화 + 선택적 슬라이스: store에는 뷰가 실제로 구독할 최소 단위만 둔다.
const useFrameIndexStore = create<{ ids: number[] }>(() => ({ ids: [] }));
const frameCache = new Map<number, FrameData>(); // 원본은 반응성 경계 밖의 캐시에

async function loadBitstream(path: string) {
  const data = await invoke<BitstreamDomainObject>('parse_bitstream', { path });
  data.frames.forEach((f) => frameCache.set(f.index, f));
  useFrameIndexStore.setState({ ids: data.frames.map((f) => f.index) });
}
```
- 원본 데이터는 ref/모듈 스코프 캐시에 두고, store에는 "무엇이 존재하는가"를 나타내는 가벼운 식별자만 반영한다.
- 화면이 실제로 구독해야 하는 슬라이스 단위로 store를 쪼갠다(FRONT-STATE-004 참고).

**탐지 방법**:
- Structural: store 타입 정의가 IPC 응답 타입(`invoke<T>`의 `T`)을 그대로 재사용하는지 검사.
- Static: `invoke(...).then(setState)` 패턴에서 응답 객체를 가공 없이 그대로 저장하는 지점 grep.

**예외**:
- 도메인 자체가 작은 설정값 몇 개(예: 사용자 환경설정)라면 통째로 저장해도 무방.

**Bitvue 판정**: N/A — 이미 반대 방향으로 리팩터링됨: `StreamDataContext.tsx`가 명시적으로 "deprecated ... split into multiple focused contexts for better performance"라 적고 FrameDataContext(프레임 메타데이터)/FileStateContext(로딩)/CurrentFrameContext(인덱스)로 분리했다 — `invoke<T>` 응답을 그대로 store 스키마로 쓰는 지점 없음.

---

### FRONT-STATE-002: derived state를 별도로 저장해 불일치

**분류**: 파생 상태 관리 · **심각도**: High · **탐지**: Structural

> FRONTEND.md의 FE-15(동일 IPC payload가 여러 store에 원본째로 복제되는 문제)와는 다른 각도다. FE-15는 "원본 데이터의 사본"이 여러 곳에 있는 문제이고, 이 항목은 "원본으로부터 계산 가능한 값"을 별도 상태로 저장해두고 소스가 바뀌는 모든 경로에서 그 계산값 갱신을 빠짐없이 호출해야 하는 동기화 부담 문제다.

**나쁜 예**:
```typescript
const [blocks, setBlocks] = useState<Block[]>([]);
const [avgQp, setAvgQp] = useState(0); // blocks에서 파생 가능한데 별도 state로 보관

function onBlocksLoaded(newBlocks: Block[]) {
  setBlocks(newBlocks);
  setAvgQp(computeAvg(newBlocks)); // 여기서 갱신을 깜빡하면 즉시 화면과 불일치
}

function onFilterApplied(filtered: Block[]) {
  setBlocks(filtered);
  // avgQp 갱신을 빠뜨림 — 필터링 후에도 필터 전 평균이 계속 표시됨
}
```

**문제**:
- `blocks`가 바뀔 수 있는 경로(초기 로드, 필터, 재디코드, undo)가 늘어날수록 `avgQp`를 함께 갱신해야 하는 지점도 늘어나고, 하나라도 빠뜨리면 UI가 소스와 어긋난 값을 보여준다.
- 리뷰 시점에는 "두 setState가 항상 짝을 이룬다"는 불변식이 코드에 명시되어 있지 않아 눈에 잘 안 띈다.
- 파생값이 여러 컴포넌트에서 각자 다시 계산+저장되면(FE-15와 결합) 소스 하나에 대해 두 종류의 불일치(사본 불일치 + 파생값 불일치)가 겹칠 수 있다.

**발생 조건**:
- 통계/요약 값(평균 QP, 총 비트레이트, 필터링된 개수 등)을 "매번 계산하기 아까워서" 캐싱하려는 최적화 시도에서 흔히 생긴다.

**권장**:
```typescript
const [blocks, setBlocks] = useState<Block[]>([]);
const avgQp = useMemo(() => computeAvg(blocks), [blocks]); // 저장하지 않고 렌더 시점에 파생
```
- 저장하지 않고 렌더 시점에 `useMemo`로 파생한다 — 그러면 `blocks`가 바뀌는 경로가 몇 개든 파생값은 항상 최신이다.
- 계산이 정말 비싸면(워커 결과 등) 저장은 하되, "무효화 지점"을 명시적으로 하나의 함수(`invalidateAvgQp()`)로 모아 호출 지점을 흩뿌리지 않는다.

**탐지 방법**:
- Structural: 하나의 state 변수가 다른 state 변수로부터 순수 함수로 계산 가능한지(파생 가능성) 검사, 계산 함수의 입력이 다른 state뿐인 `set*` 쌍을 찾는다.
- Static: `set상태A`와 `set상태B(computeXFromA(...))`가 항상 같은 호출 지점에 붙어 있는지, 안 붙어 있는 호출 지점이 있는지 grep.

**예외**:
- 계산이 매우 무겁고 소스 변경 지점이 구조적으로 단 하나뿐이며 무효화 로직이 명시적으로 문서화되어 있다면 저장이 허용될 수 있다.

**Bitvue 판정**: N/A — 통계/파생값은 일관되게 `useMemo`로 렌더 시점 계산됨(StatisticsPanel.tsx:45,48,64,91의 stats/effectiveFrameRate/frameSizes/maxSizeRange, BitrateGraphPanel.tsx:59-75의 smoothedSizes/maxSize/totalSize) — 짝을 이뤄야 하는 별도 `set*` 호출 쌍을 찾지 못함.

---

### FRONT-STATE-003: current frame 변경 시 모든 컴포넌트 rerender

**분류**: 전역 상태 분할(state splitting) · **심각도**: Critical · **탐지**: Structural

> FRONTEND.md의 FE-06(호버/툴팁처럼 마우스 이동 단위의 초고빈도 상태가 트리 루트에 있는 문제, 레이어 분리로 해결)과는 트리거 빈도와 해법의 층위가 다르다. 이 항목은 타임라인 이동/키보드 네비게이션으로 바뀌는, 앱의 "현재 무엇을 보고 있는가"를 정의하는 중심 상태(`currentFrame`)가 무거운 페이로드와 한 덩어리로 묶여 Context/전역 상태에 있는 구조적 문제이며, 해법도 레이어 분리가 아니라 상태 자체의 분할(경량 인덱스 vs 중량 데이터)이다.

**나쁜 예**:
```typescript
const AppContext = createContext<{ currentFrame: FrameData; /* 그 외 다수 필드 */ } | null>(null);

function AppProvider({ children }: PropsWithChildren) {
  const [currentFrame, setCurrentFrame] = useState<FrameData>(initialFrame);
  // value가 매 렌더 새 객체 -> Context를 구독하는 모든 컴포넌트가 무조건 리렌더
  return <AppContext.Provider value={{ currentFrame, setCurrentFrame }}>{children}</AppContext.Provider>;
}

function FrameCounterBadge() {
  const { currentFrame } = useContext(AppContext)!; // "42 / 500"만 표시하면 되는데
  return <span>{currentFrame.index} / {currentFrame.totalCount}</span>; // frameData 전체 변경에 얽힘
}
```

**문제**:
- React Context는 selector가 없어 value 중 무엇을 실제로 쓰는지와 무관하게, value 참조가 바뀌면 구독자 전체가 리렌더된다.
- `FrameCounterBadge`처럼 `frameIndex`라는 가벼운 필드만 필요한 컴포넌트도 신택스 트리·픽셀 등을 포함한 무거운 `FrameData` 객체가 통째로 바뀔 때마다 리렌더 대상이 된다.
- 재생(play) 모드처럼 프레임 전환이 초당 여러 번 발생하면, 이 구조는 프레임 전환 하나가 앱 전체 리렌더 폭풍으로 번지는 근본 원인이 된다.

**발생 조건**:
- 재생 모드, 빠른 프레임 네비게이션(키보드 연타, 타임라인 드래그)에서 체감 프레임 드롭으로 즉시 드러난다.

**권장**:
```typescript
// frameIndex(가벼움)와 frameData(무거움)를 별도 슬라이스로 분리
const useFrameIndexStore = create<{ index: number; total: number }>(() => ({ index: 0, total: 0 }));
const useFrameDataStore = create<{ data: FrameData | null }>(() => ({ data: null }));

function FrameCounterBadge() {
  const index = useFrameIndexStore((s) => s.index); // frameData 변경과 완전히 무관
  const total = useFrameIndexStore((s) => s.total);
  return <span>{index} / {total}</span>;
}
```
- "누구나 자주 읽는 가벼운 값"과 "일부만 필요한 무거운 값"을 애초에 다른 슬라이스/store로 분리한다.
- Context를 꼭 써야 한다면 값 종류별로 Provider를 분할(context splitting)해 구독 범위를 좁힌다.

**탐지 방법**:
- Structural: Context Provider의 `value` prop이 매 렌더 새 객체 리터럴인지, 그 값을 구독하는 컴포넌트 수 대비 실제 사용 필드 수의 비율을 조사.
- Runtime: 프레임을 빠르게 연속 전환하며 React Profiler에서 무관 컴포넌트(카운터, 툴바 등)가 함께 커밋되는지 확인.

**예외**:
- 컴포넌트 트리가 작고(패널 3개 이하) 리렌더 비용이 무시할 만한 초기 프로토타입 단계.

**Bitvue 판정**: N/A — CurrentFrameContext.tsx가 이미 인덱스(가벼움, ValueCtx)와 setter를 분리했고, 무거운 프레임 데이터는 FrameDataContext(메타데이터만, 픽셀 없음)에 별도로 있어 "currentFrame이 무거운 페이로드와 한 덩어리"인 구조 자체가 없음. 파일 헤더에 "Separated ... to prevent unnecessary re-renders"라고 명시.

---

### FRONT-STATE-004: selector 없이 store 전체 subscribe

**분류**: 구독 범위 설계 · **심각도**: High · **탐지**: Static / Structural

**나쁜 예**:
```typescript
const useAppStore = create<{
  frameIndex: number; overlayConfig: OverlayConfig; hexViewState: HexViewState; /* ... */
}>(() => ({ /* ... */ }));

function FrameBadge() {
  const store = useAppStore(); // 인자 없이 전체 반환
  return <span>{store.frameIndex}</span>; // overlayConfig/hexViewState가 바뀌어도 리렌더됨
}
```

**문제**:
- 인자 없이 훅을 호출하면(또는 selector가 항등 함수이면) store의 어떤 필드가 바뀌어도 이를 구독하는 모든 컴포넌트가 리렌더 대상이 된다.
- 여러 기능이 하나의 store로 합쳐질수록(개발 편의상 필드를 계속 한 store에 추가) 이 문제는 기능 수에 비례해 악화된다.
- 얼핏 "store를 나눴으니 괜찮다"고 생각하기 쉽지만, 나눈 store 안에서도 selector 없이 구독하면 동일한 문제가 그 store 내부 필드 수만큼 재현된다.

**발생 조건**:
- store가 여러 기능(오버레이 설정, 헥스뷰 상태, 프레임 인덱스 등)을 함께 담기 시작하는 시점부터 잠재하다가, 필드가 늘어나며 체감 리렌더 빈도가 누적된다.

**권장**:
```typescript
function FrameBadge() {
  const frameIndex = useAppStore((s) => s.frameIndex); // 이 필드만 구독
  return <span>{frameIndex}</span>;
}

function OverlayToolbar() {
  const { mode, opacity } = useAppStore(
    (s) => ({ mode: s.overlayConfig.mode, opacity: s.overlayConfig.opacity }),
    shallow, // 여러 필드가 필요하면 얕은 비교로 불필요한 리렌더 방지
  );
  // ...
}
```
- 컴포넌트마다 실제로 쓰는 필드만 selector로 뽑는다.
- 여러 필드가 필요하면 `shallow` 비교(zustand의 `useShallow` 등)를 함께 써서 참조가 매번 달라지는 객체 리터럴 반환을 방지한다.

**탐지 방법**:
- Static: `useStore()`처럼 selector 인자 없이 호출하는 지점을 grep.
- Structural: destructuring 결과 중 실제 JSX/로직에서 쓰이는 필드 수와 destructure된 필드 수를 비교.

**예외**:
- store 자체가 작고(필드 5개 이하) 모든 소비자가 사실상 전체를 다 쓰는 경우 selector 도입 비용이 더 클 수 있다.

**Bitvue 판정**: Confirmed — zustand류 selector store는 없지만 동일 증상이 Context에서 재현: SelectionContext.tsx:194-204와 LayoutContext.tsx:284-292는 `value`를 `useMemo` 없이 매 렌더 새 객체로 만들고(다른 대부분 컨텍스트는 useMemo 사용, 이 둘만 예외), `useSelection()`/`useLayout()` 소비자는 필드 선택 없이 객체 전체를 받아 무관한 필드 변경에도 리렌더된다.

---

### FRONT-STATE-005: 대형 typed array를 reactive state에 넣음

**분류**: 대용량 데이터·반응성 경계 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```typescript
function usePixelBuffer(frameIndex: number) {
  const [pixels, setPixels] = useState<Uint8ClampedArray | null>(null); // 수 MB 버퍼를 state로 관리

  useEffect(() => {
    invoke<Uint8ClampedArray>('get_frame_pixels', { frameIndex }).then(setPixels);
  }, [frameIndex]);

  return pixels;
}
```

**문제**:
- React state는 "변경을 감지해 리렌더를 트리거하고, 렌더 사이클/시간여행 디버깅을 위해 스냅샷을 유지"하도록 설계된 것이지, 수 MB짜리 바이너리 버퍼의 보관소로 설계된 것이 아니다.
- `setState`마다 참조가 새로 생성되므로 이 버퍼를 참조하는 모든 `memo`/`useMemo`가 무효화되고, React DevTools의 상태 스냅샷/타임트래블 기능이 대형 배열을 계속 들고 있어 메모리 프로파일이 눈에 띄게 나빠진다.
- 렌더 함수 인자로 대형 배열이 넘어가면 Strict Mode의 이중 렌더링, Concurrent 렌더링의 중간 스냅샷 유지 등과 결합해 순간 메모리 사용량이 배가될 수 있다.

**발생 조건**:
- 픽셀 버퍼, 헥스 덤프 바이트 배열, MV 필드 등 원본 바이너리/타입 배열을 "다른 데이터와 마찬가지로" state로 넘기는 최초 구현에서 흔하다.

**권장**:
```typescript
// 바이너리 데이터는 반응성 경계 밖(ref/모듈 캐시)에 두고, state에는 가벼운 식별자만 둔다.
const pixelCache = new Map<number, Uint8ClampedArray>();

function useFramePixelsRef(frameIndex: number) {
  const ref = useRef<Uint8ClampedArray | null>(pixelCache.get(frameIndex) ?? null);

  useEffect(() => {
    let cancelled = false;
    invoke<Uint8ClampedArray>('get_frame_pixels', { frameIndex }).then((buf) => {
      if (cancelled) return;
      pixelCache.set(frameIndex, buf);
      ref.current = buf;
      draw(ref.current); // imperative하게 캔버스에 반영, state를 거치지 않음
    });
    return () => { cancelled = true; };
  }, [frameIndex]);

  return ref;
}
```
- 실제 그리기는 ref 기반 imperative 코드(캔버스 draw 함수 직접 호출)에서 수행하고, React state에는 "지금 몇 번 프레임을 보고 있는가" 같은 가벼운 식별자만 둔다.

**탐지 방법**:
- Structural: `useState`/store 필드 타입 시그니처에 `TypedArray`/`ArrayBuffer`/`Uint8Array` 등이 등장하는지 grep.
- Runtime: 힙 스냅샷에서 대형 typed array가 React Fiber 트리(state)에 붙어 여러 세대에 걸쳐 보존되는지 확인.

**예외**:
- 매우 작은 버퍼(수백 바이트 이하, 팔레트/작은 룩업 테이블 등)는 state로 둬도 실질적 문제가 없다.

**Bitvue 판정**: Confirmed — YuvViewerPanel/index.tsx:90 `useState<YUVFrame | null>`(decodedFrame)가 프레임 전환마다(index.tsx:179,191, `bridgeYuvToFrame()`으로 세팅) Y/U/V 평면 원본 `Uint8Array` 전체를 React state로 담아 갱신함(타입 정의: types/yuv.ts:57-66) — 수백 KB~수 MB급 raw 픽셀 버퍼가 그대로 state에 상주. 부수적으로 UnitHexPanel/HexViewTab.tsx:37의 `useState<Uint8Array>`도 있으나 `maxBytes: 2048`로 캡(HexViewTab.tsx:60)되어 있어 항목이 언급한 예외(수백 바이트~수KB 이하)에 더 가깝다.

---

### FRONT-STATE-006: server/backend state와 local UI state 혼용

**분류**: 상태 종류 분리 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```typescript
interface WorkspaceState {
  frames: FrameMeta[];        // 서버(백엔드) 상태 — 원본 진실은 Rust 쪽
  sidebarCollapsed: boolean;  // 로컬 UI 상태 — 원본 진실은 프런트 자신
  activeTool: 'select' | 'zoom';
}

const useWorkspaceStore = create<WorkspaceState>(() => ({
  frames: [], sidebarCollapsed: false, activeTool: 'select',
}));
// 하나의 reducer/setState 경로로 서버 상태와 UI 상태를 함께 관리
```

**문제**:
- 서버 상태(캐시/재요청/무효화가 필요하고, 백엔드 이벤트로 바뀔 수 있음)와 로컬 UI 상태(영속화 불필요, 프런트가 유일한 소유자)는 수명주기와 갱신 트리거가 근본적으로 다른데, 같은 저장소·같은 갱신 경로에 섞이면 한쪽을 위한 로직 변경(예: 캐시 무효화 시 store 리셋)이 다른 쪽(사이드바 접힘 상태)까지 실수로 초기화시킨다.
- 반대 방향으로도, UI 토글 하나(`activeTool` 변경)가 같은 store의 reducer/미들웨어를 거치면서 의도치 않게 서버 데이터 재요청 로직과 결합될 위험이 있다.
- 서버 상태 전용 라이브러리(React Query 등)가 제공하는 재시도/캐시/포커스 시 재검증 같은 기능을 자체 구현해야 하는 중복 비용도 발생.

**발생 조건**:
- 앱 초기에 store를 하나만 두고 시작해 기능이 늘어날 때마다 필드를 계속 추가하는 구조에서 누적된다.

**권장**:
```typescript
// 서버 상태: React Query/SWR류 전용 계층
function useFrames() {
  return useQuery({ queryKey: ['frames'], queryFn: () => invoke<FrameMeta[]>('list_frames') });
}

// 로컬 UI 상태: 별도의 얇은 store
const useUiStore = create<{ sidebarCollapsed: boolean; activeTool: 'select' | 'zoom' }>(() => ({
  sidebarCollapsed: false, activeTool: 'select',
}));
```
- 서버 상태와 로컬 UI 상태를 처음부터 별도 계층(다른 라이브러리 또는 최소한 다른 store)으로 분리한다.

**탐지 방법**:
- Structural: 하나의 store 정의 안에 `invoke()`/`listen()` 결과로 채워지는 필드와 순수 UI 토글(boolean/enum) 필드가 함께 있는지 검사.

**예외**:
- 극소 규모 앱, 상태 슬라이스 자체가 1~2개뿐이라 분리 이득이 거의 없는 경우.

**Bitvue 판정**: Suspected — 핵심 스트림 컨텍스트(FileStateContext=서버상태, CurrentFrameContext=네비게이션, LayoutContext/ModeContext=순수 UI 토글)는 깔끔히 분리되어 있으나, CompareContext.tsx:56-62는 workspace/isLoading/error(서버성)와 pathA·pathB·currentFrameA·currentFrameB를 한 컨텍스트에 함께 둔다 — 다만 이는 Compare 기능 하나로 스코프된 것이라 카탈로그가 말하는 "전역 store가 기능마다 계속 불어나는" 성장형 문제와는 규모가 다르다.

---

### FRONT-STATE-007: loading boolean 하나로 여러 요청 관리

**분류**: 비동기 상태 모델링 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```typescript
function AnalyzerPanel() {
  const [loading, setLoading] = useState(false); // 세 개의 독립 요청이 공유

  async function loadFrame(i: number) {
    setLoading(true);
    await invoke('decode_frame', { frameIndex: i });
    setLoading(false);
  }
  async function loadSyntaxTree(i: number) {
    setLoading(true);
    await invoke('get_syntax_tree', { frameIndex: i });
    setLoading(false); // 다른 요청(loadFrame)이 아직 진행 중이어도 로딩 해제됨
  }
  // loadHexDump도 동일한 loading을 공유
}
```

**문제**:
- 세 요청 중 아무거나 하나가 진행 중이어도 전체 UI가 "로딩 중"으로 표시되어 이미 끝난 패널까지 스피너에 덮인다.
- 두 요청이 겹치면(A 시작 → B 시작 → A 끝나며 `setLoading(false)` 호출 → B는 아직 안 끝났는데 로딩 상태가 풀림) 실제 진행 상황과 표시가 어긋난다.
- 에러 처리도 공유되기 쉬워, "어떤 요청이 실패했는가"를 구분할 수 없는 단일 에러 상태로 합쳐지는 문제로 이어진다.

**발생 조건**:
- 패널이 여러 개의 독립적인 IPC 요청을 갖게 되는 순간(신택스 트리 + 헥스뷰 + 프레임 미리보기가 한 화면에 공존)부터 잠재.

**권장**:
```typescript
type RequestKey = 'frame' | 'syntaxTree' | 'hexDump';
const [status, setStatus] = useState<Record<RequestKey, 'idle' | 'loading' | 'error' | 'success'>>({
  frame: 'idle', syntaxTree: 'idle', hexDump: 'idle',
});

async function loadSyntaxTree(i: number) {
  setStatus((s) => ({ ...s, syntaxTree: 'loading' }));
  try {
    await invoke('get_syntax_tree', { frameIndex: i });
    setStatus((s) => ({ ...s, syntaxTree: 'success' }));
  } catch {
    setStatus((s) => ({ ...s, syntaxTree: 'error' }));
  }
}
```
- 요청별 독립 상태를 키로 구분해 관리하거나, React Query처럼 쿼리 키 단위로 로딩/에러가 자동 분리되는 라이브러리를 사용한다.

**탐지 방법**:
- Static: 하나의 `loading`/`isLoading` state 변수가 서로 다른 여러 `invoke` 호출부에서 `set` 되는지 grep.

**예외**:
- 요청이 애초에 순차적이고 절대 동시 진행되지 않는 것이 구조적으로 보장된 단일 파이프라인이라면 공유 플래그로 충분하다.

**Bitvue 판정**: N/A — 여러 독립 요청이 boolean 하나를 공유하는 사례를 찾지 못함: SyntaxDetailPanel의 각 탭(ApsTab/ProbsTab/QmTab/RefListTab)과 HexViewTab이 각자 자기 `loading` state를 소유하고, ThumbnailContext.tsx:54는 아예 `Set<number>`로 항목별 로딩을 추적해 이 안티패턴을 구조적으로 피해간다.

---

### FRONT-STATE-008: request cancellation 없음

**분류**: 비동기 생명주기 관리 · **심각도**: High · **탐지**: Structural

> FRONTEND.md의 FE-17(비동기 IPC 응답에 프레임/버전 가드가 없어 레이스 컨디션으로 화면이 잘못 덮이는 문제)과 인접하지만 초점이 다르다. FE-17은 "이미 도착한 낡은 응답을 화면에 반영할지 말지"를 세대(generation) 토큰으로 가드하는 *표시 정합성* 문제였다. 이 항목은 그보다 넓게, 프레임 디코드뿐 아니라 트리 확장, 검색, 내보내기 등 모든 비동기 요청 유형에 걸쳐 "이미 필요 없어진 작업 자체를 실제로 멈추는" 취소 인프라 자체의 부재를 다룬다 — 세대 토큰만으로는 화면 오염은 막아도 백엔드가 그 작업을 계속 수행하는 낭비 자체는 막지 못한다.

**나쁜 예**:
```typescript
function SyntaxTreeNode({ nodeId }: { nodeId: string }) {
  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<SyntaxNode[] | null>(null);

  const onExpand = () => {
    setExpanded(true);
    invoke<SyntaxNode[]>('get_children', { nodeId }).then(setChildren);
    // 사용자가 즉시 접거나 패널을 닫아도 이 invoke는 계속 진행되고,
    // 끝나면 이미 사라진 컴포넌트에 setChildren을 시도한다.
  };
  // ...
}
```

**문제**:
- `AbortController`/취소 신호 없이 fire-and-forget으로 `invoke`만 호출하면, 사용자가 이미 떠난 화면을 위해 백엔드가 계속 일한다(디코드 스레드/CPU 낭비).
- 언마운트된 컴포넌트에 대한 지연된 `setState`는 경고를 유발하거나, 클린업이 부실한 구버전 패턴에서는 메모리 누수로 이어진다.
- FE-17의 세대 토큰은 "화면에 반영할지"만 가드할 뿐 작업 자체를 멈추지 않으므로, 두 문제(정합성 vs 자원 낭비)는 별도로 해결해야 한다 — 세대 토큰만 도입하고 취소 인프라를 갖췄다고 착각하기 쉽다.

**발생 조건**:
- 트리 확장, 검색-as-you-type, 필터 변경처럼 사용자가 빠르게 여러 번 트리거할 수 있는 요청 유형 전반에서, 특히 패널을 여닫는 조작과 결합될 때.

**권장**:
```typescript
function useCancellableInvoke<T>(cmd: string, args: Record<string, unknown>, deps: unknown[]) {
  const [data, setData] = useState<T | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    invokeWithAbort<T>(cmd, args, controller.signal).then((result) => {
      if (!controller.signal.aborted) setData(result);
    });
    return () => controller.abort(); // 언마운트/재요청 시 이전 작업에 취소 신호 전달
  }, deps);
  return data;
}
```
- 모든 `invoke` 래퍼가 `AbortSignal`을 받아 정리 함수에서 호출하도록 통일한다.
- 가능하면 백엔드 커맨드도 취소 신호를 인지하도록 설계한다(Tauri에서는 별도 `cancel_request` 커맨드 + 요청 id 매핑으로 이미 진행 중인 Rust 작업에 취소를 전파).

**탐지 방법**:
- Structural: `useEffect` cleanup 함수에서 `abort()`/`cancel()` 호출 없이 `invoke()`만 있는 지점 전수 조사.
- Runtime: 패널을 빠르게 여닫으며 네트워크/IPC 탭에서 취소되지 않고 끝까지 진행되는 요청이 있는지 확인.

**예외**:
- 매우 빠르게 끝나는(수 ms) 로컬 조회는 취소 인프라가 과설계일 수 있다.

**Bitvue 판정**: Confirmed — 실사용 코드 경로에는 `AbortController` 사용이 전무하고(유일한 예외인 `utils/progressiveLoader.ts`는 Tauri-era `@tauri-apps/api/core`를 import하는 죽은 코드로, 어디에서도 import되지 않음 — grep 결과 tsconfig.json 외 참조 없음), 대신 `cancelled` 불리언 클로저 가드가 10곳 가까이 반복된다(YuvViewerPanel/index.tsx:174, SyntaxDetailPanel/{Aps,Probs,Qm,RefList}Tab.tsx, UnitHexPanel/HexViewTab.tsx:51, hooks/useAv1Features.ts:45, CompareWorkspace/{DiffOverlay,StreamPlayer}.tsx). 이 가드는 화면 반영만 막을 뿐, 실제 `invoke()` 자체를 멈추는 back-end `cancel_request` 커맨드는 `src-tauri/src`에 전혀 없음(grep 무결과) — 항목이 지적한 정확한 간극.

---

### FRONT-STATE-009: useEffect dependency 누락·과다

**분류**: effect 의존성 설계 · **심각도**: High · **탐지**: Static

> FRONTEND.md의 FE-11은 RAF/interval 콜백에서 의존성 배열을 비워 "1회만 등록"하려다 발생하는 stale closure(누락 쪽)를 다뤘다. 이 항목은 반대쪽 극단, 즉 의존성이 과다·불안정한 경우(매 렌더 새 참조가 deps에 들어가 effect가 필요 이상으로 재실행되는 문제)를 중심으로 다루되, 두 극단이 사실 같은 근본 원인(의존성 배열 설계 미숙)의 양면임을 함께 짚는다.

**나쁜 예**:
```typescript
useEffect(() => {
  const unlisten = listen('frame-updated', handleUpdate);
  invoke('subscribe_frame_updates', { config: { mode: overlayMode, opts: {} } });
  return () => { unlisten.then((f) => f()); };
}, [{ mode: overlayMode, opts: {} }]); // 객체 리터럴을 deps에 직접 사용
```

**문제**:
- 배열/객체 리터럴, 인라인 함수를 deps에 그대로 넣으면 매 렌더 새 참조이므로 항상 "바뀜"으로 판정되어 effect가 매 렌더 재실행된다.
- 구독 해제/재구독(`listen`/`unlisten`)이 매 렌더 반복되면 IPC 리스너 등록이 스팸처럼 발생하고, effect 안에 `setState`가 있으면 렌더 → effect → setState → 재렌더 → effect… 로 이어지는 실질적 무한 루프로 발전할 수 있다.
- 반대로 deps를 필요 이상으로 줄이면(FE-11의 케이스) 클로저가 오래된 값을 붙든 채 굳어버려 정확성 버그가 된다 — 둘 다 "deps 배열이 실제 코드 동작과 일치하지 않는다"는 같은 문제의 양면이다.

**발생 조건**:
- 설정 객체를 매번 인라인으로 구성해 effect에 넘기는 코드, 또는 ESLint `exhaustive-deps` 경고를 `eslint-disable-next-line`으로 넘긴 지점에서 흔히 발견된다.

**권장**:
```typescript
useEffect(() => {
  const unlisten = listen('frame-updated', handleUpdate);
  invoke('subscribe_frame_updates', { config: { mode: overlayMode, opts: EMPTY_OPTS } }); // 안정적 상수 참조
  return () => { unlisten.then((f) => f()); };
}, [overlayMode]); // primitive만 deps에 사용
```
- deps에는 primitive 값만 넣거나, 객체/함수가 꼭 필요하면 `useMemo`/`useCallback`으로 참조를 안정화한다.
- `eslint-plugin-react-hooks`의 `exhaustive-deps`를 경고가 아닌 빌드 실패로 강제하고, 억제 지점은 코드 리뷰에서 반드시 사유를 명시한다.

**탐지 방법**:
- Static: deps 배열에 객체/배열 리터럴이 인라인으로 있는지 grep; `eslint-disable-next-line react-hooks/exhaustive-deps` 지점 전수 조사.
- Runtime: 무관해 보이는 렌더에서도 해당 effect가 재실행(IPC 재호출, 리스너 재등록)되는지 로깅으로 확인.

**예외**:
- 없음 — 안정화가 항상 가능하거나(값을 memo화), deps 배열 자체가 필요 없는 구조(effect 밖으로 로직 이동)로 재설계하는 것이 항상 더 나은 대안이다.

**Bitvue 판정**: N/A — deps 배열에 객체/배열 리터럴이 인라인으로 들어간 지점을 grep으로 찾지 못했고, 저장소 전체에서 `react-hooks/exhaustive-deps` 경고는 단 1건(FileStateContext.tsx, setFrames 누락 — 사실상 무해)뿐이며 규칙 자체는 .eslintrc.cjs에서 억제 없이 활성화되어 있다.

---

### FRONT-STATE-010: effect에서 backend와 양방향 무한 동기화

**분류**: 상태 동기화 루프 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```typescript
useEffect(() => {
  invoke('set_zoom_level', { zoom }); // zoom이 바뀔 때마다 백엔드에 알림
}, [zoom]);

useEffect(() => {
  const unlisten = listen<number>('zoom-changed', (e) => setZoom(e.payload)); // 백엔드가 확인 응답을 되돌려줌
  return () => { unlisten.then((f) => f()); };
}, []);
```

**문제**:
- 백엔드가 "zoom 변경을 확인했다"는 의미로 동일한 `zoom-changed` 이벤트를 되돌려주면, 첫 번째 effect가 그 값을 새 `zoom`으로 인식해 다시 `invoke`를 호출하고, 백엔드는 다시 이벤트를 발행하는 순환이 생긴다.
- 값이 완전히 동일하면(엄격한 동등 비교) 이 순환이 첫 사이클에서 멈출 수도 있지만, 부동소수점 반올림/단위 변환이 왕복 중 미세하게 값을 바꾸면 매번 "살짝 다른 값"으로 판정되어 영원히 반복되는 사례가 실무에서 흔하다.
- 로컬 개발 중에는 응답이 즉시 오고 값도 대개 정확히 같아 재현되지 않다가, 실제 배포 환경의 반올림/타이밍 조건에서만 나타나 디버깅이 까다롭다.

**발생 조건**:
- "프런트가 백엔드에 알린다"와 "백엔드가 프런트에 확인 응답을 보낸다"가 같은 이벤트 채널/같은 논리적 값을 공유하도록 설계되었을 때.

**권장**:
```typescript
const lastSentZoom = useRef(zoom);

useEffect(() => {
  if (zoom === lastSentZoom.current) return; // 이미 이 값으로 보낸 적 있으면 스킵
  lastSentZoom.current = zoom;
  invoke('set_zoom_level', { zoom });
}, [zoom]);

useEffect(() => {
  const unlisten = listen<number>('zoom-changed', (e) => {
    lastSentZoom.current = e.payload; // 백엔드발 갱신은 "보낸 값"으로도 기록해 되돌아오는 걸 흡수
    setZoom(e.payload);
  });
  return () => { unlisten.then((f) => f()); };
}, []);
```
- 값의 출처(source of truth)를 하나로 고정한다 — zoom은 프런트가 owner이고 백엔드는 fire-and-forget 알림만 받거나, 반대로 백엔드가 owner면 프런트는 낙관적 업데이트 없이 이벤트만 반영하는 식으로 방향을 단일화한다.
- 부득이 양방향이 필요하면 "마지막으로 보낸/받은 값"을 기록해 동일 값 왕복을 흡수하는 가드를 반드시 둔다.

**탐지 방법**:
- Structural: 같은 논리적 값에 대해 "프런트→백엔드" `invoke`와 "백엔드→프런트" `listen`이 각각 별도 effect에 있는지 매핑해 순환 경로 여부를 확인.
- Runtime: 해당 값 변경 시 IPC 호출/이벤트 발생 횟수를 로깅해 짧은 시간 내 반복 횟수가 비정상적으로 증가하는지 확인.

**예외**:
- 한쪽이 명백히 idempotent하고 동일 값이면 즉시 종료되는 가드가 이미 존재한다면 양방향 구조 자체는 무방하다.

**Bitvue 판정**: N/A — 프런트→백엔드→프런트로 되돌아오는 값 왕복 구조 자체가 없음. zoom은 useCanvasInteraction의 순수 로컬 state로 backend invoke가 없고, 프런트 전체에서 `listen()` 호출은 App.tsx와 useFileOperations.ts 단 2곳뿐인데 둘 다 단방향 `file-opened` OS 이벤트이지 프런트가 보낸 값의 확인 응답이 아니다.

---

## Rendering — 렌더 비용

> 아래 10개 후보 중 2개는 FRONTEND.md와 사실상 동일한 문제라 항목화하지 않고 스킵했다: **FRONT-RENDER-001**(수만 행 비가상화)은 FE-04(신택스 트리/헥스뷰/필름스트립 가상화 부재)와 완전히 중복되어 스킵. **FRONT-RENDER-010**(워커로 옮길 계산을 메인 스레드에서 수행)은 FE-14(QP 히트맵/MV 필드 계산의 워커 오프로딩)와 완전히 중복되어 스킵. 나머지 8개는 FRONTEND.md의 인접 항목과 겹치는 부분이 있으면 그와 다른 각도로 날을 세워 남겼다.

### FRONT-RENDER-002: canvas 상태를 React state로 매 frame 갱신

**분류**: 렌더 루프 vs React state 분리 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```typescript
function PlayheadCanvas() {
  const [x, setX] = useState(0);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let raf: number;
    function tick(t: number) {
      setX(computeX(t)); // 매 프레임 React state 갱신 -> 매 프레임 컴포넌트 재렌더
      raf = requestAnimationFrame(tick);
    }
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  useEffect(() => { drawPlayhead(canvasRef.current!, x); }, [x]);
  return <canvas ref={canvasRef} />;
}
```

**문제**:
- 60fps 재생이면 초당 60회 `setState` → 60회 리렌더 + reconciliation이 발생하는데, 캔버스 그리기 자체는 imperative(ref 직접 조작)로 충분해 React 렌더 사이클을 거칠 이유가 없다.
- state 배칭/우선순위 스케줄링(Concurrent 렌더링)과 `requestAnimationFrame` 타이밍이 서로 다른 스케줄러이므로, 두 스케줄이 어긋나며 프레임 드롭·지터가 생길 수 있다.
- 이 컴포넌트가 다른 무거운 형제 컴포넌트와 같은 트리에 있으면, 매 프레임 리렌더가 그 형제까지 재조정 대상으로 끌어들일 위험도 있다(memo가 없다면).

**발생 조건**:
- 재생 헤드, 실시간 파형/레벨 미터처럼 고빈도로 갱신되는 시각 요소를 "React스럽게" state로 관리하려 할 때.

**권장**:
```typescript
function PlayheadCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    let raf: number;
    function tick(t: number) {
      drawPlayhead(canvasRef.current!, computeX(t)); // state를 거치지 않고 직접 그림
      raf = requestAnimationFrame(tick);
    }
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
  return <canvas ref={canvasRef} />;
}
```
- 애니메이션 값은 ref에만 저장하고 draw 함수를 직접 호출한다 — React는 "재생 중/정지"처럼 저빈도로 바뀌는 상태만 관리한다.

**탐지 방법**:
- Static: `requestAnimationFrame`/`setInterval` 콜백 내부에서 `setState` 호출이 있는지 grep.
- Runtime: 재생 중 React Profiler에서 해당 컴포넌트가 매 프레임 커밋되는지 확인.

**예외**:
- 저빈도(초당 1~2회 이하) 갱신이거나, 캔버스가 아닌 실제 DOM/텍스트 표시를 위한 값이라면 state 경로가 적절하다.

**Bitvue 판정**: N/A — 캔버스 좌표를 매 프레임 setState로 갱신하는 rAF 루프를 찾지 못함. 유일한 rAF 사용처인 VirtualizedFilmstrip.tsx:53-65는 스크롤 이벤트를 rAF 1회로 스로틀링하는 용도(이벤트 기반)이고, 재생 프레임 전환은 setTimeout으로 frameIndex를 바꾸는 것(YuvViewerPanel/index.tsx:358)이라 재렌더가 의도된 정상 상태 변경이다.

---

### FRONT-RENDER-003: pointer move마다 전체 store update

**분류**: 고빈도 이벤트 → 전역 store 확산 · **심각도**: High · **탐지**: Runtime

> FRONTEND.md의 FE-06(호버 state가 React 트리 루트에 있어 props로 퍼지는 문제)·FE-07(포인터/스크럽 이벤트를 그대로 IPC로 흘려보내는 문제)과 인접하지만 다른 층위다. 이 항목은 이미 selector 기반 외부 store(zustand 등)를 도입한 뒤에도, pointer move 핸들러가 store의 여러 무관한 필드를 한 번의 `set` 호출로 묶어 갱신함으로써 해당 store를 구독하는 모든 selector가 재평가되는 문제를 다룬다.

**나쁜 예**:
```typescript
const useAppStore = create<{
  cursor: { x: number; y: number };
  hoveredBlockId: string | null;
  tooltipText: string;
  /* ... 수십 개의 무관한 필드 */
}>((set) => ({
  cursor: { x: 0, y: 0 }, hoveredBlockId: null, tooltipText: '',
  updatePointer: (x: number, y: number, blockId: string | null, text: string) =>
    set({ cursor: { x, y }, hoveredBlockId: blockId, tooltipText: text }), // 여러 필드를 한 번에 교체
}));

<div onPointerMove={(e) => useAppStore.getState().updatePointer(e.clientX, e.clientY, id, text)} />
```

**문제**:
- `set({...})`가 store의 여러 최상위 필드를 한 번에 교체하면, 이 store를 구독하는 모든 selector 함수가 재평가되어야 하고, 결과적으로 pointer move 한 번이 store 구독자 수만큼의 비교/재계산을 유발한다.
- selector를 필드 단위로 세분화해두어도, 하나의 대형 `set` 호출 안에서 여러 무관 필드가 동시에 바뀌면 그중 어느 필드를 구독하는 컴포넌트든 "무언가 바뀌었다"는 알림을 받아 재평가 대상이 된다.
- `cursor: { x, y }`처럼 객체를 매번 새로 만들면, 좌표 자체가 실질적으로 동일해도(예: 클램핑된 경계값에서 반복 이벤트) 참조가 달라 항상 변경으로 판정된다.

**발생 조건**:
- 마우스 이동에 반응해 커서 좌표·호버 블록·툴팁 텍스트를 "한 번에" 갱신하는 통합 액션을 만들 때, 특히 store 구독자가 여러 패널에 걸쳐 있을 때 체감된다.

**권장**:
```typescript
// pointer 관련 필드를 별도의 작은 슬라이스로 격리하고, 실제 값이 바뀐 필드만 반영
const usePointerStore = create<{ hoveredBlockId: string | null }>((set) => ({
  hoveredBlockId: null,
  setHoveredBlock: (id: string | null) =>
    set((s) => (s.hoveredBlockId === id ? s : { hoveredBlockId: id })), // 동일 값이면 갱신 스킵
}));
// 커서 좌표 자체는 store에 두지 않고 ref + imperative 처리(FRONT-RENDER-002와 동일 원칙)
```
- pointer 관련 필드를 무관한 필드와 분리된 작은 슬라이스/store로 격리한다.
- "의미 있는 전이"(호버 블록이 실제로 바뀜)만 store에 반영하고, 연속적인 좌표값은 store가 아니라 ref로 처리한다.

**탐지 방법**:
- Structural: 하나의 `set`/`setState` 호출이 몇 개의 논리적으로 무관한 필드를 동시에 바꾸는지 검사.
- Runtime: pointer move 중 store 구독자별 재평가 횟수를 계측(zustand devtools 등)해 무관 구독자가 함께 트리거되는지 확인.

**예외**:
- store 구독자가 1~2개뿐이고 전체 재평가 비용이 무시할 만한 소규모 화면.

**Bitvue 판정**: N/A — 전역 store 자체가 없고(zustand 미사용), 마우스 이동 핸들러는 지역 컴포넌트 state만 갱신한다(Timeline.tsx:23,117의 hoverPosition). 공유되는 SelectionContext는 클릭/키보드로 프레임이 바뀔 때만 갱신되며(Filmstrip.tsx:180, Timeline.tsx:165-206) mousemove에서 호출되는 지점은 찾지 못함.

---

### FRONT-RENDER-004: chart data를 매 render 재생성

**분류**: 차트 데이터 참조 안정성 · **심각도**: Medium · **탐지**: Static

> FRONTEND.md의 FE-05(오버레이 픽셀 버퍼 재계산에 메모이제이션이 없는 문제)와 같은 뿌리(참조 불안정)를 갖지만, 이 항목은 커스텀 캔버스가 아니라 Recharts/visx/D3 기반 차트 컴포넌트에 특유한 결과(트랜지션 재생, 축 재스케일 깜빡임)에 초점을 둔다.

**나쁜 예**:
```typescript
function BitrateChart({ frames }: { frames: FrameMeta[] }) {
  return (
    <LineChart data={frames.map((f) => ({ x: f.index, y: f.bitrate }))}> {/* 매 렌더 새 배열+새 원소들 */}
      <Line dataKey="y" />
    </LineChart>
  );
}
```

**문제**:
- 차트 라이브러리 내부는 보통 `data` 배열/원소의 참조 동일성으로 "데이터가 실제로 바뀌었는지"를 판단해 진입 애니메이션·트랜지션을 재생하는데, 매 렌더 새 배열을 주면 부모의 무관한 리렌더(예: 사이드바 토글)에도 차트가 매번 트랜지션을 재생하거나 순간적으로 깜빡인다.
- 대용량 프레임 목록(수만 프레임)이면 `.map()` 자체의 O(n) 비용도 매 렌더 반복된다.
- 축 스케일 계산이 데이터 배열 참조에 의존하는 라이브러리에서는 무관한 리렌더마다 축 범위가 재계산되어 미세한 레이아웃 흔들림으로 나타나기도 한다.

**발생 조건**:
- 비트레이트/QP 분포 등 시계열 차트를 프레임 목록으로부터 매핑해 그리는 패널에서, 특히 부모가 다른 이유로 자주 리렌더될 때.

**권장**:
```typescript
function BitrateChart({ frames }: { frames: FrameMeta[] }) {
  const data = useMemo(() => frames.map((f) => ({ x: f.index, y: f.bitrate })), [frames]);
  return (
    <LineChart data={data}>
      <Line dataKey="y" />
    </LineChart>
  );
}
```
- `useMemo`로 참조를 안정화하거나, 가능하면 백엔드/store가 애초에 차트가 바로 소비할 수 있는 형태로 데이터를 내려줘 매핑 자체를 없앤다.

**탐지 방법**:
- Static: 차트 컴포넌트의 `data=` prop에 인라인 `.map()`/`.filter()`가 `useMemo` 없이 있는지 grep.

**예외**:
- 프레임 수가 매우 적어(수십 개 이하) 매핑 비용이 무시 가능하고, 트랜지션 재생이 오히려 의도된 시각 효과인 경우.

**Bitvue 판정**: N/A — 차트가 Recharts/D3 같은 참조-동일성 기반 트랜지션 라이브러리가 아니라 자체 제작 SVG 컴포넌트(BarChart.tsx, LineChart.tsx)이고, 그 데이터 가공은 이미 `useMemo`로 감싸져 있다(RDCurvesPanel.tsx의 chartSeries, BitrateGraphPanel.tsx의 smoothedSizes/maxSize) — 라이브러리발 트랜지션 재생/깜빡임 증상 자체가 발생할 여지가 없는 구조.

---

### FRONT-RENDER-005: object identity 변화로 memoization 무효

**분류**: 참조 안정성(referential stability) · **심각도**: High · **탐지**: Structural

> FE-05(오버레이 픽셀 재계산), FE-13(선택 변경 시 전체 오버레이 재계산), 그리고 위 FRONT-RENDER-004(차트 데이터)는 모두 이 항목이 다루는 일반 원칙의 서로 다른 도메인 사례다. 이 항목은 개별 사례를 반복하는 대신, 카탈로그 전체에 걸쳐 재발하는 근본 원인을 코드 리뷰 체크리스트 수준으로 명문화한다.

**나쁜 예**:
```typescript
function Panel({ frame }: { frame: FrameData }) {
  return (
    <ExpensiveChild
      options={{ mode: 'qp', scale: 1 }}   // 매 렌더 새 객체
      onSelect={(id) => select(id)}         // 매 렌더 새 함수
    />
  ); // React.memo(ExpensiveChild)가 있어도 무력화됨
}
```

**문제**:
- `React.memo`/`useMemo`/`useCallback`은 모두 참조(reference) 동일성에 의존하는데, 부모가 렌더될 때마다 새로운 객체 리터럴·배열 리터럴·인라인 함수를 자식에게 props로 내려주면 자식의 메모이제이션이 원천적으로 무력화된다.
- 이 패턴은 개별 사례(FE-05의 오버레이 픽셀 배열, FE-13의 선택 상태 결합, FRONT-RENDER-004의 차트 데이터)로 카탈로그 전체에서 반복 관찰되는 동일한 근본 원인이므로, 개별 인스턴스를 매번 새로 발견하기보다 구조적으로 예방하는 편이 비용 효율적이다.
- 특히 `React.memo`로 감쌌다는 사실 자체가 "이미 최적화됐다"는 잘못된 안도감을 주기 쉬워, 이 무효화가 오래 방치되는 경향이 있다.

**발생 조건**:
- props로 객체/배열/함수를 넘기는 모든 지점에서 잠재하며, 부모가 자주 리렌더되는 상위 컴포넌트(예: 전역 상태를 구독하는 레이아웃 컴포넌트) 바로 아래에서 특히 체감된다.

**권장**:
```typescript
const STABLE_OPTIONS = { mode: 'qp', scale: 1 } as const; // 컴포넌트 밖 상수로 참조 고정

function Panel({ frame }: { frame: FrameData }) {
  const handleSelect = useCallback((id: string) => select(id), []); // 함수 참조 안정화
  return <ExpensiveChild options={STABLE_OPTIONS} onSelect={handleSelect} />;
}
```
- (1) 객체/배열 props는 `useMemo`로, 함수 props는 `useCallback`으로 안정화하거나, (2) 애초에 객체를 props로 넘기지 않고 primitive만 넘긴 뒤 자식 내부에서 store/selector로 조회하거나, (3) 코드 리뷰 체크리스트에 "`React.memo` 대상 컴포넌트 호출부에 인라인 리터럴 props가 없는가"를 명시적으로 포함한다.

**탐지 방법**:
- Static: `React.memo`로 감싼 컴포넌트 호출부에서 인라인 `{}`/`[]`/화살표 함수 props를 grep.
- Runtime: React DevTools Profiler의 "why did this render"에서 "props changed"인데 실제 값은 동일한 경우를 확인.

**예외**:
- 자식이 애초에 memo화되어 있지 않거나 렌더 비용이 낮은 리프 컴포넌트라면 무효화 자체가 문제되지 않는다.

**Bitvue 판정**: Confirmed — StatisticsPanel.tsx:137이 `memo()`로 감싼 BarChart(components/charts/BarChart.tsx:51)에 인라인 `colors={{...}}` 객체 리터럴을 매 렌더 새로 만들어 넘겨, 해당 컴포넌트의 memo 비교를 무력화한다.

---

### FRONT-RENDER-006: 모든 tooltip을 DOM으로 생성

**분류**: DOM 노드 수 관리 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```typescript
function BlockGrid({ blocks }: { blocks: Block[] }) {
  return (
    <>
      {blocks.map((b) => (
        <div key={b.id} className="block">
          <div className="tooltip">{b.qp} / MV({b.mvX},{b.mvY})</div> {/* 수천 개가 항상 DOM에 존재 */}
        </div>
      ))}
    </>
  );
}
```

**문제**:
- 실제로 한 번에 보이는 툴팁은 최대 1개인데, 블록 수만큼 툴팁 DOM 노드를 CSS `visibility`/`opacity`로만 숨긴 채 항상 마운트해둔다.
- 초기 마운트 비용, 레이아웃 계산 대상(reflow 후보), 브라우저의 스타일 재계산 대상이 모두 블록 수에 비례해 증가한다.
- 신택스 트리·헥스뷰처럼 이미 노드 수가 많은 화면(FE-04가 다루는 가상화 대상)과 결합되면 DOM 노드 총량이 사실상 배가되어, 가상화로 얻은 이득의 상당 부분을 툴팁이 다시 갉아먹는다.

**발생 조건**:
- 블록 단위 오버레이, 신택스 트리 노드 등 항목 수가 많은 화면에서 "각 항목이 자기 툴팁을 갖는다"는 단순한 멘탈 모델로 구현할 때.

**권장**:
```typescript
function useSharedTooltip() {
  const [content, setContent] = useState<{ text: string; x: number; y: number } | null>(null);
  const Tooltip = () =>
    content && createPortal(
      <div className="tooltip" style={{ left: content.x, top: content.y }}>{content.text}</div>,
      document.body,
    );
  return { show: setContent, hide: () => setContent(null), Tooltip };
}
```
- 툴팁 1개를 포탈(portal)로 공유하고, hover된 대상의 데이터만 그 공유 툴팁에 전달해 위치/내용을 갱신한다.

**탐지 방법**:
- Structural: `.tooltip`류 클래스를 가진 엘리먼트가 리스트 `.map()` 내부에서 항목마다 렌더되는지 grep.
- Runtime: `document.querySelectorAll('.tooltip').length`로 실제 DOM 노드 수를 항목 수와 비교.

**예외**:
- 항목 수가 애초에 적은(수십 개 이하) 화면이라면 굳이 공유 툴팁으로 리팩터링할 이득이 작다.

**Bitvue 판정**: N/A — 항목 수만큼 `.tooltip` 노드가 상주하는 패턴을 찾지 못함. FilmstripTooltip.tsx는 Filmstrip.tsx에서 단 1곳(hover 대상 1개)만 렌더되고, utils/interactiveTooltips/TooltipManager.ts는 애초에 싱글턴 DOM 노드 하나(`currentTooltip`)만 관리하는 구조.

---

### FRONT-RENDER-007: resize observer가 연쇄 layout 발생

**분류**: 레이아웃 스래싱 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```typescript
function SplitPanel({ children }: PropsWithChildren) {
  const [width, setWidth] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const ro = new ResizeObserver(([entry]) => {
      setWidth(entry.contentRect.width); // setState -> 리렌더 -> 인접 패널 크기 변화 -> 콜백 재트리거...
    });
    ro.observe(containerRef.current!);
    return () => ro.disconnect();
  }, []);

  return <div ref={containerRef}>{children}</div>;
}
```

**문제**:
- ResizeObserver 콜백에서 `setState`로 다시 스타일에 영향을 주는 값을 갱신하면, 그 리렌더가 관찰 대상의 크기를 다시 바꿔 콜백을 재트리거하는 순환이 생길 수 있다 — 브라우저가 "ResizeObserver loop limit exceeded" 경고를 내는 전형적 상황.
- 여러 패널이 서로의 크기에 의존하는 레이아웃(스플릿 패널, 사이드바+메인)에서는 하나의 리사이즈가 연쇄적으로 다른 패널의 재계산을 유발해, 읽기(레이아웃 값 조회)와 쓰기(스타일 변경)가 번갈아 반복되는 강제 동기 레이아웃(layout thrashing)으로 이어진다.
- 이 문제는 리사이즈 자체보다 "콜백 안에서 읽기·쓰기를 분리하지 않은 것"이 원인이라, 관찰 대상이 하나뿐이어도 다른 무관한 레이아웃 변경과 겹치면 체감 지연으로 나타날 수 있다.

**발생 조건**:
- 사용자가 스플릿 패널 경계를 드래그하거나 창 크기를 조절할 때, 특히 여러 ResizeObserver가 서로 다른 컴포넌트에서 독립적으로 같은 영역을 관찰할 때.

**권장**:
```typescript
useEffect(() => {
  let rafId: number | null = null;
  let lastWidth = -1;
  const ro = new ResizeObserver(([entry]) => {
    const next = entry.contentRect.width;
    if (Math.abs(next - lastWidth) < 1) return; // 문턱값 이하 변화는 무시
    if (rafId != null) cancelAnimationFrame(rafId);
    rafId = requestAnimationFrame(() => { lastWidth = next; setWidth(next); }); // 쓰기를 rAF로 배치
  });
  ro.observe(containerRef.current!);
  return () => { ro.disconnect(); if (rafId != null) cancelAnimationFrame(rafId); };
}, []);
```
- 콜백에서 읽기와 쓰기를 분리하고 rAF로 배치하며, 실제로 유의미하게 값이 바뀐 경우에만 `setState`한다.
- 가능하면 CSS(container queries, flexbox/grid)로 해결해 JS 기반 리사이즈 관찰 자체를 줄인다.

**탐지 방법**:
- Static: ResizeObserver 콜백 내부에서 `setState`가 debounce/rAF/문턱값 비교 없이 즉시 호출되는지 grep.
- Runtime: 브라우저 콘솔에 "ResizeObserver loop" 경고가 뜨는지, 패널 리사이즈 중 Performance 패널에 Layout/Recalculate Style이 반복되는지 확인.

**예외**:
- 관찰 대상이 하나뿐이고 다른 레이아웃에 영향을 주지 않는 독립적인 위젯이라면 단순한 구현으로도 충분하다.

**Bitvue 판정**: Confirmed — SyntaxDetailPanel/FrameSyntaxTab.tsx:258-259가 ResizeObserver 콜백 안에서 문턱값/rAF 배치 없이 `setContainerHeight(entry.contentRect.height)`를 즉시 호출해 나쁜 예와 거의 동일하다. 반대로 HRDBufferPanel.tsx:279가 같은 상황에서 setState 대신 imperative `drawCanvas()`를 직접 호출해 이 문제를 피해간다 — 같은 저장소 안에 안티패턴과 권장 패턴이 공존.

---

### FRONT-RENDER-008: hidden tab도 계속 rendering

**분류**: 가시성 기반 렌더 억제 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```typescript
function AnalyzerTabs({ activeTab }: { activeTab: 'hex' | 'tree' | 'filmstrip' }) {
  return (
    <>
      <div style={{ display: activeTab === 'hex' ? 'block' : 'none' }}><HexViewPanel /></div>
      <div style={{ display: activeTab === 'tree' ? 'block' : 'none' }}><SyntaxTreePanel /></div>
      <div style={{ display: activeTab === 'filmstrip' ? 'block' : 'none' }}><FilmstripPanel /></div>
      {/* 세 패널 모두 항상 마운트된 채, 내부 rAF/폴링도 항상 동작 */}
    </>
  );
}
```

**문제**:
- `display: none`은 레이아웃/페인트만 건너뛸 뿐, React 컴포넌트의 effect·내부 rAF 루프·타이머·IPC 폴링은 그대로 실행되어 보이지 않는 탭이 CPU/메모리를 계속 소비한다.
- 헥스뷰·신택스 트리·필름스트립처럼 무거운 패널 세 개가 모두 이 방식이면, 사용자가 활성 탭 하나만 보고 있어도 세 패널 몫의 작업을 전부 지불하게 된다.
- 특히 필름스트립처럼 자체 애니메이션/폴링을 가진 패널이 비활성 상태에서도 계속 백엔드에 요청을 보내면, FRONT-STATE-008이 다루는 취소 인프라 부재와 결합해 낭비가 배가된다.

**발생 조건**:
- 탭 전환 시 스크롤 위치/입력 상태를 유지하고 싶어 언마운트를 피하고 `display: none`만 쓰는 구현에서 흔하다.

**권장**:
```typescript
function useActiveTabEffect(isActiveTab: boolean, tick: () => void) {
  useEffect(() => {
    if (!isActiveTab) return; // 비활성 탭에서는 루프 자체를 시작하지 않음
    let raf: number;
    function loop() { tick(); raf = requestAnimationFrame(loop); }
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [isActiveTab]);
}
```
- Page Visibility API/탭 활성 상태를 effect 의존성으로 사용해 비활성 탭에서는 rAF/폴링을 일시정지한다.
- 정말 무거운 탭은 언마운트하고, 스크롤 위치 등 꼭 보존해야 할 로컬 상태만 탭 외부의 작은 store에 별도로 저장한다.

**탐지 방법**:
- Structural: `display: none`으로 숨겨지는 컨테이너 내부에 rAF/`setInterval`/폴링 `invoke`가 활성 상태 체크 없이 존재하는지 검사.
- Runtime: 특정 탭만 활성화한 채 다른 탭에서 CPU 사용률/네트워크(IPC) 활동이 계속되는지 측정.

**예외**:
- 탭 전환 시 상태를 완전히 잃으면 안 되는 매우 가벼운 위젯(입력 폼 등)은 계속 마운트해도 비용이 무시할 만하다.

**Bitvue 판정**: N/A (반대 패턴 확인) — DockableLayout.tsx:111,184는 활성 패널 컴포넌트 하나만 `{ActivePanel && <ActivePanel />}`로 렌더하고, TabContent(common/TabContainer.tsx:123)는 비활성일 때 `return null`로 언마운트한다 — `display:none`으로 전부 마운트 유지하는 코드는 grep(`display:.*none`)으로도 전혀 발견되지 않음.

---

### FRONT-RENDER-009: WebGL texture를 frame마다 재생성

**분류**: GPU 리소스 수명 관리 · **심각도**: High · **탐지**: Runtime

> FRONTEND.md의 FE-09는 텍스처 업로드가 메인 스레드에서 동기적으로 일어나 인터랙션을 블로킹하는 문제(균일하게 느려지는 Long Task 프로파일)를 다루며, 해법 중 하나로 "텍스처 풀링"을 짧게 언급했다. 이 항목은 그 풀링 미비 자체를 분리해, 업로드가 비동기/논블로킹이더라도 `createTexture`/`deleteTexture`를 프레임마다 반복하면 드라이버 레벨의 할당 스래싱으로 인해 균일한 느려짐이 아니라 간헐적 스파이크(끊김)가 발생하는, FE-09와는 다른 성능 프로파일을 다룬다.

**나쁜 예**:
```typescript
useEffect(() => {
  const texture = gl.createTexture(); // 매 프레임 새 텍스처 객체 생성
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, frame.width, frame.height, 0, gl.RGBA, gl.UNSIGNED_BYTE, frame.rgba);
  return () => gl.deleteTexture(texture); // 다음 프레임에 또 생성/삭제 반복
}, [frame]);
```

**문제**:
- `createTexture`/`deleteTexture`를 프레임마다 반복하면 드라이버 내부 할당자가 매번 GPU 메모리를 확보/해제해야 하고, 이 패턴이 누적되면 메모리 단편화와 드라이버 내부 락 경합으로 이어져 특정 시점에 몰아서 프레임이 끊기는 스파이크성 현상이 나타난다 — FE-09가 다루는 "매 프레임 균일하게 느려짐"과는 다른 증상이라 같은 프로파일링 방법으로는 원인이 잘 안 잡힌다.
- 텍스처 크기가 매 프레임 동일한데도 매번 새로 할당하면, `texSubImage2D`로 내용만 갱신했을 때 대비 불필요한 재할당 비용을 반복 지불하는 셈이다.
- 텍스처 파라미터(필터링, 밉맵 등)를 재생성 시마다 다시 설정하는 코드가 흔한데, 이 설정 비용도 매 프레임 누적된다.

**발생 조건**:
- 프레임 전환이 빠른 스크럽/재생 상황에서 스파이크가 주기적으로 나타나며, 저사양 통합 GPU 환경(CI 러너, 저전력 노트북)에서 드라이버 오버헤드가 더 크게 체감된다.

**권장**:
```typescript
// 고정 크기 텍스처 풀을 마운트 시 미리 생성
const texturePool = useTexturePool(gl, { width: MAX_WIDTH, height: MAX_HEIGHT, count: 3 });

useEffect(() => {
  const texture = texturePool.acquire();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, frame.width, frame.height, gl.RGBA, gl.UNSIGNED_BYTE, frame.rgba);
  // 해상도가 실제로 바뀔 때만 texturePool.resize() 등 예외적 재할당 경로를 탄다.
}, [frame]);
```
- 고정 크기 텍스처 풀을 마운트 시 미리 만들어두고 `texSubImage2D`로 내용만 갱신한다 — 해상도가 프레임마다 바뀌지 않는다면 재할당 자체가 불필요하다.
- 해상도가 바뀌는 경우에만 예외적으로 재생성 경로를 타도록 분기한다.

**탐지 방법**:
- Structural: `createTexture`/`deleteTexture` 호출이 `useEffect(() => {...}, [frame])`처럼 프레임 단위 의존성 안에 있는지 grep.
- Runtime: GPU 메모리 사용량 그래프에서 생성-해제가 반복되는 톱니 패턴이 나타나는지, 프레임 전환 중 간헐적 스파이크가 발생하는지 확인.

**예외**:
- 텍스처 크기가 프레임마다 실제로 달라지는 가변 해상도 스트림, 또는 텍스처 수명이 애초에 짧고 재사용 가치가 없는 원샷 내보내기 경로.

**Bitvue 판정**: N/A — 저장소에 유일한 WebGL 코드(OverlayRenderer/webgl/mv-webgl.ts)는 MV 라인을 WeakMap에 캐시된 program/buffer로 그리며 텍스처 자체를 아예 쓰지 않는다. 실제 비디오 프레임 렌더링은 Canvas2D `putImageData`/`drawImage`(utils/yuv/renderer.ts:136-157, YuvViewerPanel/VideoCanvas.tsx:161-199) 기반이라 `createTexture`/`deleteTexture` 호출 자체가 저장소 어디에도 없다.
