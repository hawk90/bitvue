# Anti-Pattern Catalog — TAURI_EVT: Event 남용

이 문서는 Bitvue 안티패턴 카탈로그의 한 갈래이며, 전체 카탈로그의 색인은 별도로 작성 중인 `docs/anti-patterns/INDEX.md`를 참고한다. UI/UX + Tauri Phase 3 웨이브에 속하며, 같은 웨이브의 `TAURI_CMD.md`(Command 설계), `TAURI_WEB.md`(Webview/윈도우 경계)와 형제 문서다. 이 문서는 Tauri의 `emit()`/`listen()` 이벤트 브리지를 상태 전파(broadcast) 수단으로 오남용할 때 발생하는 문제를 다룬다.

---

### TAURI-EVT-001: 모든 상태 변경을 event로 broadcast

**분류**: 상태 전파 설계 · **심각도**: High · **탐지**: Code

**나쁜 예**:
```rust
#[tauri::command]
fn set_current_frame(app: tauri::AppHandle, state: tauri::State<AppState>, frame_index: u32) {
    let mut s = state.lock().unwrap();
    s.current_frame = frame_index;
    // 값이 바뀔 때마다 이벤트를 쏜다 — zoom, overlay 토글, 커서 위치도 전부 같은 패턴
    app.emit("frame-changed", frame_index).ok();
}

#[tauri::command]
fn toggle_overlay(app: tauri::AppHandle, state: tauri::State<AppState>, layer: String, on: bool) {
    let mut s = state.lock().unwrap();
    s.overlays.insert(layer.clone(), on);
    app.emit("overlay-toggled", (layer, on)).ok();
}
```
```typescript
// frontend: 모든 화면 조각이 이벤트에 반응하도록 짜여 있다
listen("frame-changed", (e) => setCurrentFrame(e.payload));
listen("overlay-toggled", (e) => updateOverlayFlag(e.payload));
listen("zoom-changed", (e) => setZoom(e.payload));
// ... 수십 개의 유사한 listen()이 여러 컴포넌트에 흩어짐
```

**문제**:
- `invoke()`의 반환값으로 바로 줄 수 있는 결과까지 이벤트 왕복으로 처리해, 호출자가 "성공했는가"와 "값이 뭔가"를 분리된 두 채널(커맨드 리턴 + 이벤트)로 추적해야 한다.
- 프런트엔드가 이벤트 핸들러 스파게티가 되어, 어떤 상태가 어떤 이벤트로 갱신되는지 추적하려면 grep을 여러 번 해야 한다.
- 이벤트는 Tauri IPC 상에서 fire-and-forget이라 실패를 감지할 방법이 없다 — 커맨드 호출이었다면 `Result`로 실패가 드러났을 것이 조용히 사라진다.
- 사용자가 직접 트리거한 동기적 상태 변경(프레임 이동, 줌)까지 이벤트로 우회시키면 command → event → re-render라는 불필요한 추가 hop이 매 조작마다 생긴다.

**발생 조건**:
- Bitvue처럼 패널이 많은 앱(hex view, filmstrip, overlay renderer, waveform)에서 "상태가 바뀌면 일단 이벤트로 알리자"는 습관이 굳어진 경우.
- 초기 프로토타입에서 커맨드/이벤트 역할을 구분하지 않고 둘 다 "백엔드와 통신하는 방법"으로만 취급한 경우.

**권장**:
```typescript
// 사용자가 직접 트리거한, 결과가 즉시 필요한 변경은 command 호출 결과로 직접 반영
async function goToFrame(index: number) {
  const frame = await invoke<FrameSummary>("set_current_frame", { frameIndex: index });
  setCurrentFrame(frame); // 이벤트 왕복 없이 바로 상태 갱신
}
```
- 사용자 액션 → 즉시 결과가 필요한 상태 변경은 `invoke()` 반환값으로 처리한다.
- event는 "백엔드가 자발적으로/비동기로 알려야 하는 것"(디코드 완료, 파일 변경 감지, 장시간 분석 진행률)에만 쓴다.
- 두 채널을 섞을 필요가 있는 값(예: 여러 창이 같은 프레임 상태를 공유해야 함)은 별도 항목(TAURI-EVT-007)에서 다룬다.

**탐지 방법**:
- Code: `app_handle.emit(` 호출부와 대응하는 `#[tauri::command]`의 반환 타입이 `()`인 경우(즉 결과를 리턴하지 않고 이벤트로만 알리는 패턴)를 정적으로 스캔.
- Code: 프런트엔드에서 `invoke()` 직후 곧바로 관련 `listen()`이 대기하는 패턴(사실상 request/response를 이벤트로 흉내내는 코드)을 검색.

**예외**:
- 여러 윈도우가 동일한 상태를 실시간으로 반영해야 하는 진짜 broadcast 요구(멀티 모니터에서 같은 프레임을 보는 detached 뷰)는 이벤트가 적절하다.

**Bitvue 판정**: N/A — **정정(재감사)**: `src-tauri`는 커밋 `e7194cc`("refactor: retire src-tauri, the working Tauri fallback app")로 완전히 삭제됐고 저장소는 Electron으로 이관됐다(`CLAUDE.md` "Migrated off Tauri 2026-08-08"). 이 항목에 이전에 적혀 있던 판정(`App.tsx:373/591/450`, `src-tauri/src/commands/file.rs` 인용)은 이관 이전 상태를 감사한 것으로 현재 `App.tsx`와 전혀 일치하지 않는다 — 재확인 결과 `onReloadFile`(현재 `App.tsx:393-395`)과 `handleOpenRecent`(`App.tsx:576-579`) 둘 다 `useAppFileOperations`의 `openFileAtPath()`를 직접 호출해 결과를 즉시 받으며, 이벤트 왕복이 전혀 없다. 실제 IPC 계층(`bitvue-desktop/electron/preload.cjs`의 `window.bitvue.*`, `ipcMain.handle`/`ipcRenderer.invoke` 35개 채널)은 전부 요청-응답이며, 진짜 broadcast 이벤트는 `bitvue:sidecar-restarted`(`main.ts:1308`) 단 하나뿐 — 이는 "모든 상태 변경을 broadcast"가 아니라 "백엔드 프로세스가 죽었다 재시작됨"이라는 좁은 1회성 알림이라 이 안티패턴 자체가 적용되지 않는다(단 이 이벤트를 실제로 구독하는 프런트 코드가 없다는 별개 문제는 TAURI-EVT-010에서 다룸).

---

### TAURI-EVT-002: event 순서에 의존

**분류**: 동시성/순서 보장 · **심각도**: Critical · **탐지**: Code

**나쁜 예**:
```rust
#[tauri::command]
fn seek_and_decode(app: tauri::AppHandle, state: tauri::State<AppState>, frame_index: u32) {
    let handle = app.clone();
    tokio::spawn(async move {
        handle.emit("decode-started", frame_index).ok();
        let frame = decode_frame(frame_index).await; // 프레임마다 소요 시간 다름
        handle.emit("frame-ready", &frame).ok();
        handle.emit("decode-finished", frame_index).ok();
    });
}
```
```typescript
// 사용자가 스크럽바를 빠르게 드래그하면 seek_and_decode가 연속 호출된다
listen("frame-ready", (e) => setDisplayedFrame(e.payload)); // "가장 최근에 도착한 게 최신"이라 가정
```

**문제**:
- `tokio::spawn`으로 각 요청이 독립 태스크로 뜨면, 늦게 요청한 프레임이 먼저 디코드를 끝내 먼저 도착할 수 있다(디코드 시간은 프레임 타입/복잡도에 따라 다름).
- 프런트엔드가 "이벤트는 emit 순서대로 도착한다"고 암묵적으로 가정하면, 빠른 스크러빙 중 오래된 프레임이 최신 프레임을 덮어써 화면이 실제 재생 위치와 어긋난다.
- Tauri의 이벤트 전달은 emit 호출 순서를 보장하지 않는다 — 특히 서로 다른 스레드/비동기 태스크에서 emit할 때는 더더욱.

**발생 조건**:
- 타임라인 스크럽, 빠른 프레임 탐색처럼 같은 종류의 비동기 작업이 겹쳐서 여러 번 트리거될 때.
- 백엔드가 요청마다 새 태스크/스레드를 스폰해 병렬로 처리하는 구조일 때.

**권장**:
```rust
#[tauri::command]
fn seek_and_decode(app: tauri::AppHandle, state: tauri::State<AppState>, frame_index: u32) {
    let seq = state.next_seek_seq(); // 요청마다 단조 증가 시퀀스 발급
    let handle = app.clone();
    tokio::spawn(async move {
        let frame = decode_frame(frame_index).await;
        handle.emit("frame-ready", FrameReadyPayload { seq, frame }).ok();
    });
}
```
```typescript
let latestSeq = 0;
listen<FrameReadyPayload>("frame-ready", (e) => {
  if (e.payload.seq < latestSeq) return; // 오래된 응답은 폐기
  latestSeq = e.payload.seq;
  setDisplayedFrame(e.payload.frame);
});
```
- 순서가 중요한 흐름은 이벤트 자체가 아니라 payload에 담긴 시퀀스/타임스탬프로 최신성을 판단한다.
- 가능하면 이전 요청을 취소(`AbortController` 상당의 취소 토큰)해 낭비 자체를 없앤다.

**탐지 방법**:
- Code: 동일 이벤트를 여러 비동기 태스크에서 emit하면서 payload에 순서 식별자가 없는 패턴을 검색.
- Interaction: 스크럽바를 빠르게 드래그하며 인위적으로 지연을 주입해(예: 특정 프레임 디코드에 슬립 삽입) 화면이 최신 위치와 어긋나는지 수동 테스트.

**예외**:
- 백엔드가 단일 순차 큐/단일 워커 스레드로 이벤트를 직렬화해서 내보내는 구조라면 순서 보장이 실제로 성립하므로 문제되지 않는다.

**Bitvue 판정**: N/A — **정정(재감사, Electron 이관 후 기준)**: fire-and-forget emit 자체가 없으므로(EVT-001 참고) 순서 비보장 경합도 성립하지 않는다. 다만 진짜 request/response 경쟁 시나리오(빠른 프레임 탐색으로 `getDecodedFrameYuv` invoke가 겹쳐 나갈 때)는 별도로 안전하게 처리돼 있다: `bitvue-desktop/src/sidecarClient.ts`가 `pending: Map<number, PendingRequest>`로 모든 요청/응답을 `correlationId`로 상호 연관시켜(`sidecarClient.ts:118, 237-248`) 응답이 어느 순서로 와도 올바른 호출자에게만 매칭되고, 프런트도 `YuvViewerPanel/index.tsx:164-189`에서 표준 React `cancelled` 플래그로 오래된 응답을 폐기한다 — 카탈로그가 권장하는 "시퀀스로 최신성 판단" 원칙이 이미 구조적으로 지켜지고 있다.

---

### TAURI-EVT-003: event 유실 시 복구할 snapshot query 없음

**분류**: 초기 동기화 부재 · **심각도**: Critical · **탐지**: Interaction

**나쁜 예**:
```rust
#[tauri::command]
fn start_full_parse(app: tauri::AppHandle, path: String) {
    tokio::spawn(async move {
        for pct in parse_progress_stream(&path) {
            app.emit("parse-progress", pct).ok(); // 리스너가 아직 없어도 그냥 쏜다
        }
        app.emit("parse-complete", ()).ok();
    });
}
```
```typescript
function ProgressPanel() {
  const [pct, setPct] = useState(0);
  useEffect(() => {
    const unlisten = listen<number>("parse-progress", (e) => setPct(e.payload));
    return () => { unlisten.then(f => f()); };
  }, []); // 패널이 이미 진행 중인 파싱 도중에 열리면 0%에서 멈춘 채 보인다
  return <ProgressBar value={pct} />;
}
```

**문제**:
- Tauri의 `emit`은 버퍼링되지 않는다 — 리스너가 등록되기 전에 emit된 이벤트는 그냥 사라진다.
- 패널이 파싱 시작 이후에 열리거나, 윈도우가 늦게 마운트되거나, 리스너 등록이 `invoke()` 호출보다 뒤에 실행되면 초기 이벤트들을 영영 놓친다.
- 놓친 뒤에는 다음 델타 이벤트가 와도 기준점이 없어 화면이 실제 상태(예: 이미 70%까지 진행됨)를 반영하지 못하고 고정된다.

**발생 조건**:
- 앱 시작 직후 자동으로 시작되는 백그라운드 작업(초기 파일 인덱싱)에 리스너 등록이 경쟁 상태로 들어갈 때.
- 이미 진행 중인 장시간 작업(대용량 파일 전체 파싱) 도중 사용자가 관련 패널을 처음 열 때.
- 윈도우 새로고침/재오픈 후.

**권장**:
```typescript
function ProgressPanel() {
  const [pct, setPct] = useState<number | null>(null);
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      // 1) 먼저 현재 스냅샷을 쿼리해 기준점을 맞춘다
      const snapshot = await invoke<number | null>("get_parse_progress");
      setPct(snapshot);
      // 2) 그 다음부터의 델타만 이벤트로 받는다
      unlisten = await listen<number>("parse-progress", (e) => setPct(e.payload));
    })();
    return () => unlisten?.();
  }, []);
  return pct === null ? <Idle /> : <ProgressBar value={pct} />;
}
```
- 모든 "델타만 알리는" 이벤트에는 짝이 되는 "현재 값을 조회하는" 커맨드를 반드시 만든다.
- 리스너 등록 → 스냅샷 조회 → 스냅샷 이후 이벤트 반영의 순서를 고정된 패턴(커스텀 훅)으로 강제한다.

**탐지 방법**:
- Code: `listen(...)`만 있고 같은 컴포넌트/훅 내에 대응하는 `invoke("get_*")` 스냅샷 호출이 없는 패턴을 검색.
- Interaction: 장시간 작업을 백엔드에서 먼저 트리거한 뒤 관련 패널을 나중에 여는 시나리오를 수동으로 재현.

**예외**:
- 작업 자체가 매우 짧아(수십 ms) 리스너가 창 렌더링 완료 전에 이미 등록되는 것이 구조적으로 보장되는 경우.

**Bitvue 판정**: N/A — **정정(재감사)**: 델타-스트림 형태의 progress 이벤트는 현재 Electron 브리지에도 존재하지 않는다 — 대용량 로딩/인덱싱은 `getFramesChunk`/`indexStream` 같은 요청-응답 invoke 폴링으로 구현되어(`electronBridgeService.ts`) 이 패턴 자체가 우회된다(이 결론은 이전 감사와 동일하되, `src-tauri`가 아니라 현재의 `bitvue-desktop` sidecar 브리지 기준으로 재확인). 단, 관련은 있지만 이 항목이 다루는 "델타 스트림"과는 다른 별개의 진짜 유실 문제가 EVT-010에 있다 — `bitvue:sidecar-restarted`(유일한 push 이벤트)를 구독하는 프런트 코드가 아예 없다.

---

### TAURI-EVT-004: progress를 지나치게 자주 emit

**분류**: 이벤트 빈도 · **심각도**: High · **탐지**: Performance

**나쁜 예**:
```rust
fn parse_bitstream(app: &tauri::AppHandle, nal_units: &[NalUnit]) {
    for (i, nal) in nal_units.iter().enumerate() {
        process_nal(nal);
        // 8K 스트림이면 NAL 유닛이 수십만 개 — 유닛마다 이벤트 emit
        app.emit("parse-progress", (i, nal_units.len())).ok();
    }
}
```

**문제**:
- 이벤트 하나마다 직렬화 → IPC 브리지 → 프런트엔드 역직렬화 → React 상태 갱신 → 리렌더의 전체 파이프라인이 도는데, 이를 초당 수천~수만 번 반복하면 메인 스레드가 progress 갱신만으로 포화된다.
- 실제로 사용자가 인지할 수 있는 것은 1% 단위 진행률 정도인데, NAL/매크로블록 단위로 emit하면 인지 가능한 정보 대비 수백 배 과도한 트래픽이 발생한다.
- UI가 progress 이벤트 처리에 밀려 다른 사용자 입력(취소 버튼 클릭 등)에 응답하지 못하는 역설적 상황이 생긴다.

**발생 조건**:
- 4K/8K급 대용량 스트림의 전체 파싱, 긴 GOP의 프레임 단위 디코드 진행률.
- 진행 단위가 사용자 관점의 의미 단위(퍼센트)보다 훨씬 촘촘한 내부 단위(NAL, macroblock, sample)로 되어 있을 때.

**권장**:
```rust
fn parse_bitstream(app: &tauri::AppHandle, nal_units: &[NalUnit]) {
    let mut last_emitted_pct = 0u8;
    let mut last_emit = std::time::Instant::now();
    for (i, nal) in nal_units.iter().enumerate() {
        process_nal(nal);
        let pct = ((i + 1) * 100 / nal_units.len()) as u8;
        // 1%p 이상 변했고, 마지막 emit 이후 최소 간격(예: 50ms)이 지났을 때만 보고
        if pct != last_emitted_pct && last_emit.elapsed().as_millis() >= 50 {
            app.emit("parse-progress", pct).ok();
            last_emitted_pct = pct;
            last_emit = std::time::Instant::now();
        }
    }
    app.emit("parse-progress", 100u8).ok(); // 마지막 값은 반드시 보낸다
}
```
- 값 기반 임계값(1%p 변화)과 시간 기반 임계값(최소 간격)을 함께 적용해 스로틀링한다.
- 마지막 100% 이벤트는 스로틀 조건과 무관하게 항상 emit해 진행률 바가 "멈춘 것처럼" 보이지 않게 한다.
- 프런트엔드에서도 `requestAnimationFrame` 배칭으로 한 번 더 안전장치를 둘 수 있다.

**탐지 방법**:
- Performance: 런타임에서 특정 이벤트 이름의 emit 횟수/초를 계측해 임계값(예: 초당 20회) 초과 시 경고.
- Code: 루프 본문 안에서 반복마다 무조건 emit하는 패턴(스로틀 변수 없이)을 정적으로 검색.

**예외**:
- 진행 단계 수가 원래 적은 작업(예: 10단계 파이프라인의 단계 전환 알림)은 스로틀링이 불필요하다.

**Bitvue 판정**: N/A — **정정(재감사)**: 현재 Electron 브리지(`bitvue-desktop/electron/main.ts`, `ipcMain.handle` 35개 채널)에도 "progress"성 emit이나 스로틀링 로직이 없다. 대용량 스트림 파싱/디코드 진행률은 여전히 이벤트가 아니라 `getFramesChunk` 같은 chunked invoke 폴링으로 처리되어, 단위별 emit 폭주가 발생할 코드 경로 자체가 없다(결론은 이전 감사와 동일, 근거만 `src-tauri`에서 현재 sidecar 브리지로 교체).

---

### TAURI-EVT-005: listener 해제 누락

**분류**: 리소스 정리 · **심각도**: Medium · **탐지**: Code

**나쁜 예**:
```typescript
function FrameInspector() {
  useEffect(() => {
    // Promise를 그냥 던져두고 반환값(unlisten)을 무시
    listen<FrameMeta>("frame-changed", (e) => {
      setMeta(e.payload);
    });
    // cleanup 함수를 반환하지 않음
  }, []);
  ...
}
```

**문제**:
- `listen()`은 `Promise<UnlistenFn>`을 반환하는데, 이를 저장하지 않고 버리면 컴포넌트가 언마운트돼도 리스너가 살아남는다.
- 패널을 열고 닫기를 반복하면(예: 검사 패널 토글) 같은 이벤트에 대한 핸들러가 계속 누적되어, 한 번의 emit에 언마운트된 컴포넌트의 `setState`까지 호출되며 경고/누수가 쌓인다.
- React StrictMode의 이중 마운트나 라우트 전환이 잦은 SPA 구조에서 특히 두드러진다.

**발생 조건**:
- 자주 마운트/언마운트되는 컴포넌트(토글 가능한 사이드 패널, 탭 전환).
- `useEffect` 내부에서 `await` 없이 `listen()`을 fire-and-forget으로 호출한 경우.

**권장**:
```typescript
function FrameInspector() {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<FrameMeta>("frame-changed", (e) => {
      setMeta(e.payload);
    }).then((f) => {
      if (cancelled) f(); // 등록 완료 전에 이미 언마운트됐으면 즉시 해제
      else unlisten = f;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);
  ...
}
```
- `listen()`의 반환 Promise를 항상 `await`하거나 `.then()`으로 받아 `unlisten`을 보관하고, cleanup에서 반드시 호출한다.
- 반복되는 패턴이므로 `useTauriEvent(eventName, handler)` 같은 공용 훅으로 감싸 실수를 구조적으로 방지한다.
- 등록이 비동기로 완료되기 전에 언마운트되는 경쟁 상태까지 `cancelled` 플래그로 처리한다.

**탐지 방법**:
- Code: `listen(` 호출부에서 반환값을 변수에 받지 않거나, `useEffect` cleanup에서 `unlisten`을 호출하지 않는 패턴을 정적 검색(ESLint 커스텀 룰로 강제 가능).
- Runtime: 개발 모드에서 동일 이벤트에 대해 등록된 리스너 개수를 주기적으로 로깅해 패널 토글 반복 후 누적되는지 확인.

**예외**:
- 앱 전체 생명주기 동안 한 번만 등록되고 절대 언마운트되지 않는 최상위 컴포넌트(`App.tsx` 루트)의 전역 리스너는 정리 로직이 없어도 실질적 위험이 없다.

**Bitvue 판정**: N/A — **정정(재감사)**: `App.tsx`에는 이제 `listen()` 호출이 전혀 없다(EVT-001 참고 — 재작성되어 이벤트 왕복 없이 `openFileAtPath()` 직접 호출로 바뀜). 저장소 전체에서 `@tauri-apps/api/event`의 `listen`을 실제로 쓰는 곳은 `hooks/useFileOperations.ts:22` 단 한 곳뿐이며(cleanup 자체는 갖추고 있음, `:248-266`), grep 결과 이 훅은 자기 테스트(`tests/hooks/useFileOperations.test.ts`) 외에는 어디서도 import되지 않는 죽은 코드다(실제 경로는 `useAppFileOperations`) — 즉 리스너가 살아서 등록되는 경로 자체가 현재 앱에 없다. Electron 쪽의 유일한 push 채널 `onSidecarRestarted`(`preload.cjs:99-101`)도 타입 선언과 테스트 mock 외에는 아무도 호출하지 않아 등록/해제 자체가 발생하지 않는다.

---

### TAURI-EVT-006: 컴포넌트 mount마다 중복 listener 등록

**분류**: 리소스 정리 / 중복 실행 · **심각도**: Medium · **탐지**: Code

**나쁜 예**:
```typescript
// Filmstrip.tsx, OverlayRenderer/index.tsx, YuvViewerPanel.tsx 각각이
// 동일한 "frame-changed" 이벤트를 독립적으로 listen한다
function Filmstrip() {
  useEffect(() => {
    const p = listen("frame-changed", (e) => recomputeThumbnailWindow(e.payload));
    return () => { p.then(f => f()); };
  }, []);
}
function OverlayRenderer() {
  useEffect(() => {
    const p = listen("frame-changed", (e) => refetchOverlayData(e.payload)); // 같은 이벤트, 별도 등록
    return () => { p.then(f => f()); };
  }, []);
}
```

**문제**:
- 개별 등록 자체는 (해제만 잘 되면) 메모리 누수는 아니지만, 같은 논리적 상태 변화를 여러 컴포넌트가 각자 다른 시점에 각자 다르게 해석하며 중복 계산을 반복한다.
- 비교 모드처럼 같은 컴포넌트가 여러 인스턴스로 동시에 떠 있으면(A/B 스트림 뷰) 동일 이벤트에 리스너가 N배로 등록되어, 의도치 않게 "두 번 실행되는 버그"처럼 보이는 문제를 만든다.
- 여러 곳에서 각자 파생 상태를 다시 계산하다 보니 서로 다른 타이밍에 서로 다른 결과를 반영해 화면 간 미묘한 불일치가 생긴다.

**발생 조건**:
- 여러 패널이 동일한 백엔드 이벤트를 각자 구독하는 구조.
- 동일 컴포넌트가 다중 인스턴스로 렌더링되는 비교/분할 화면 레이아웃.

**권장**:
```typescript
// 이벤트는 단 한 곳(전역 store)에서만 구독하고, 컴포넌트는 store를 구독한다
// store/frameStore.ts
export const useFrameStore = create<FrameState>((set) => {
  listen<FrameMeta>("frame-changed", (e) => set({ current: e.payload })); // 앱 생명주기 동안 1회만 등록
  return { current: null };
});

// 각 컴포넌트는 Tauri 이벤트를 직접 모른다
function Filmstrip() {
  const current = useFrameStore((s) => s.current);
  ...
}
```
- 원시 Tauri 이벤트 구독은 전역 store/context 한 곳으로 모으고, 컴포넌트들은 store를 통해서만 상태를 읽는다.
- 컴포넌트가 이벤트 이름 문자열을 직접 아는 경우가 여러 곳이면 이미 이 안티패턴의 신호다.

**탐지 방법**:
- Code: 동일 이벤트 이름(`"frame-changed"` 등) 문자열이 여러 파일에서 `listen(` 인자로 등장하는지 grep.
- Interaction: 비교/분할 뷰를 열고 이벤트 하나를 트리거했을 때 부수 효과(네트워크 호출, 로그)가 인스턴스 수만큼 반복되는지 확인.

**예외**:
- 핸들러가 진짜로 순수하고 비용이 무시할 만한 수준(단순 상태 플래그 반영)이라면 중복 등록이 실질적 해는 없다 — 다만 구조적 확장성을 위해 여전히 지양할 가치는 있다.

**Bitvue 판정**: N/A — **정정(재감사)**: 이전 판정이 인용한 `App.tsx:450`의 `listen<FileOpenedEvent>(...)`는 더 이상 존재하지 않는다(EVT-001/005 참고 — `App.tsx`는 이제 `listen()`을 전혀 쓰지 않고 `openFileAtPath()`를 직접 호출). 따라서 "두 곳이 동일 이벤트를 각자 구독"하는 상황 자체가 재현되지 않는다 — 살아있는 구독자가 하나도 없다(EVT-005 참고). 다만 이전 판정이 지적한 잠재 위험 구조는 형태를 바꿔 여전히 유효하다: `hooks/useFileOperations.ts`(죽은 훅, `"file-opened"` 구독)와 `hooks/useAppFileOperations.ts`(실제 사용 경로)가 병렬로 공존하는 것 자체가 향후 실수로 죽은 훅이 다시 연결되면 중복 구독이 실현될 수 있는 구조 — `feedback` 메모리에도 "ThumbnailContext/useThumbnail은 죽은 병렬 구현"이라는 동일 패턴이 기록돼 있어, Bitvue 저장소에 반복되는 습관으로 보인다.

---

### TAURI-EVT-007: 창마다 필요 없는 이벤트까지 수신

**분류**: 창 경계 미분리 · **심각도**: Medium · **탐지**: Code

**나쁜 예**:
```rust
#[tauri::command]
fn update_hex_cursor(app: tauri::AppHandle, offset: u64) {
    // detached hex-view 창만 필요로 하는데 앱 전체에 broadcast
    app.emit("hex-cursor-moved", offset).ok();
}
```
```typescript
// main window, waveform 창, hex-view 창 모두 이 리스너를 갖고 있어
// 자신과 무관한 이벤트까지 매번 처리(또는 조용히 무시)한다
listen("hex-cursor-moved", (e) => { /* hex-view가 아니면 사실상 no-op */ });
```

**문제**:
- Tauri의 `AppHandle::emit`은 기본적으로 모든 창에 전달된다. hex 커서처럼 특정 detached 창(팝아웃된 hex viewer)만 관심 있는 이벤트도 메인 창, waveform 창 등 무관한 창까지 매번 역직렬화하고 핸들러를 실행(또는 no-op 판별)해야 한다.
- 창 수가 늘어날수록(다중 모니터에서 여러 패널을 detach) 이벤트 하나의 실질 비용이 창 개수에 비례해 커진다.
- 무관한 창의 상태(DOM/컴포넌트 트리)를 전제로 하지 않은 핸들러가 실수로 전역 등록되면, 해당 창에 없는 엘리먼트를 참조하다 조용히 실패하거나 예외를 던질 수 있다.

**발생 조건**:
- Bitvue처럼 hex view, overlay, waveform을 별도 창으로 분리(detach)할 수 있는 멀티 윈도우 구조.
- 멀티 모니터 환경에서 특정 창에만 해당하는 상호작용(커서 이동, 로컬 줌)이 잦은 경우.

**권장**:
```rust
#[tauri::command]
fn update_hex_cursor(app: tauri::AppHandle, window_label: String, offset: u64) {
    if let Some(win) = app.get_webview_window(&window_label) {
        win.emit("hex-cursor-moved", offset).ok(); // 해당 창에만 전달
    }
}
```
```typescript
// 진짜 전역 이벤트(테마 변경 등)만 app 전체에 listen하고,
// 창 국소적 이벤트는 emit_to로 스코프를 좁힌다
```
- `app_handle.emit`(전체 broadcast) 대신 `window.emit`/`emit_to(label, ...)`로 대상 창을 명시한다.
- 이벤트 이름에 창 역할을 접두어로 넣는(`hexview://cursor-moved`) 네이밍 컨벤션도 실수를 줄이는 데 도움이 된다.

**탐지 방법**:
- Code: 멀티 윈도우 앱에서 `.emit(` 호출 중 `_to`/`window.emit` 형태로 스코프가 좁혀지지 않은 것을 검색.
- Interaction: detached 창을 열어 두고 다른 창에서만 의미 있는 조작을 한 뒤, detached 창의 개발자 콘솔에 무관한 이벤트 로그가 찍히는지 확인.

**예외**:
- 테마 변경, 앱 종료 알림, 전역 설정 변경처럼 정의상 모든 창이 알아야 하는 이벤트는 broadcast가 맞다.

**Bitvue 판정**: N/A — **정정(재감사, 근거를 현재 Electron 코드로 교체)**: `bitvue-desktop/electron/main.ts`는 `createWindow()`를 정확히 1회만 호출한다(`main.ts:1301`) — 단일 `BrowserWindow` 아키텍처로, 멀티 윈도우/detached 패널이 없다. `webContents.send`도 저장소 전체에 `bitvue:sidecar-restarted`(`main.ts:1308`) 단 한 번만 쓰이며 이는 유일한 창에 보내는 것이라 "무관한 창까지 broadcast"할 대상 자체가 없다. hex view/waveform을 별도 창으로 분리하는 기능은 여전히 미구현 — 결론은 이전 감사와 동일.

---

### TAURI-EVT-008: event payload에 대형 binary 포함

**분류**: 대용량 페이로드 · **심각도**: Critical · **탐지**: Performance

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct FrameDecodedPayload {
    frame_index: u32,
    yuv_data: Vec<u8>, // 4K 프레임이면 수 MB~수십 MB
}

fn on_frame_decoded(app: &tauri::AppHandle, frame_index: u32, yuv: Vec<u8>) {
    app.emit("frame-decoded", FrameDecodedPayload { frame_index, yuv_data: yuv }).ok();
}
```

**문제**:
- Tauri 이벤트 payload는 JSON(필요 시 base64)으로 직렬화되는 경로를 거치는데, 이는 원래 작은 알림용으로 설계된 것이지 수 MB급 바이너리 전송에 최적화되어 있지 않다.
- base64 인코딩만으로도 원본 대비 약 33%의 크기 증가와 상당한 CPU 비용이 추가되며, 이 작업이 메인/이벤트 루프 스레드를 막을 수 있다.
- 동일 프레임 데이터가 이벤트 payload와 (별도로 필요할 수 있는) 커맨드 응답 양쪽에 중복 보관/전송되는 낭비가 생기기 쉽다.
- 여러 창이 broadcast로 이 이벤트를 받으면(TAURI-EVT-007과 결합) 무관한 창까지 대형 바이너리를 역직렬화하게 된다.

**발생 조건**:
- 디코드된 프레임(YUV/RGB), hex dump, 파형 샘플 배열처럼 원본 크기가 큰 데이터를 "다 됐다"는 알림과 함께 통째로 보내고 싶을 때.

**권장**:
```rust
fn on_frame_decoded(app: &tauri::AppHandle, frame_index: u32) {
    // payload는 참조 정보만 — 실제 바이트는 별도 경로로 가져간다
    app.emit("frame-decoded", frame_index).ok();
}

#[tauri::command]
fn get_frame_pixels(state: tauri::State<AppState>, frame_index: u32) -> tauri::ipc::Response {
    let bytes = state.lock().unwrap().frame_buffer(frame_index);
    tauri::ipc::Response::new(bytes) // raw 바이트, base64 왕복 없이 전송
}
```
```typescript
listen<number>("frame-decoded", async (e) => {
  const pixels = await invoke<ArrayBuffer>("get_frame_pixels", { frameIndex: e.payload });
  renderToCanvas(pixels);
});
```
- 이벤트는 "프레임 N이 준비됐다"는 신호만 담고, 실제 바이트는 raw response(`tauri::ipc::Response`)나 asset protocol처럼 base64 오버헤드가 없는 경로로 별도 요청한다.
- 필요하다면 Tauri의 `Channel` API로 스트리밍 전송을 검토한다(단, 이 역시 대형 payload를 이벤트 시스템과 동일한 방식으로 오남용하지 않도록 크기/빈도를 관리해야 한다).

**탐지 방법**:
- Performance: 이벤트 payload 직렬화에 걸리는 시간과 크기를 런타임에 로깅해 임계값(예: 100KB) 초과 emit을 경고.
- Code: `emit` 호출의 payload 타입에 `Vec<u8>` 또는 대형 배열 필드가 포함된 구조체를 정적으로 검색.

**예외**:
- 이미 충분히 축소된 썸네일(필름스트립용 64×64 등)처럼 원래도 작은 데이터는 이벤트에 인라인으로 포함해도 실질적 문제가 없다.

**Bitvue 판정**: N/A — **정정(재감사, 근거를 현재 Electron 브리지로 교체)**: 디코드된 프레임(YUV/RGB), 썸네일, hex 바이트는 전부 `ipcRenderer.invoke()` 반환값(`getDecodedFrameYuv`, `getThumbnails`, `getHexRange` — `preload.cjs`)으로 요청-응답 경로를 통해서만 전달되고, event payload로 나가는 대형 바이너리는 없다(broadcast emit 자체가 없음). 오히려 `sidecarClient.ts`의 `getHexRange`/`getDecodedFrameYuv`는 메타데이터(JSON)와 raw 바이트를 `dataPromise`/`metadataPromise`로 분리해 base64 팽창을 피하는, 카탈로그가 권장하는 방향과 정확히 일치하는 설계다.

---

### TAURI-EVT-009: 서로 다른 작업의 event를 ID 없이 혼합

**분류**: 식별자 부재 · **심각도**: High · **탐지**: Code

**나쁜 예**:
```rust
// 파일 A 파싱, 파일 B 디코드가 동시에 진행 중일 수 있는데
// 둘 다 같은 이름의 이벤트를 공유한다
app.emit("progress", pct_a).ok(); // 어느 작업의 진행률인지 payload만 봐선 알 수 없다
app.emit("progress", pct_b).ok();
```
```typescript
// 비교 모드에서 두 개의 ProgressBar가 같은 이벤트를 구독
listen<number>("progress", (e) => setProgressA(e.payload)); // B의 진행률이 A에 섞여 들어올 수 있다
```

**문제**:
- 비교(A/B) 모드나 다중 탭처럼 동시에 여러 독립 작업이 진행될 수 있는 앱에서, 이벤트에 작업 식별자가 없으면 어느 리스너가 어느 작업의 진행 상황인지 구분할 근거가 없다.
- 한쪽 작업을 취소해도 다른 작업의 동일 이벤트가 계속 들어오는 한, 취소된 작업의 UI가 여전히 진행 중인 것처럼 보이거나 반대로 살아있는 작업의 진행률이 취소된 작업의 마지막 값으로 잘못 표시될 수 있다.
- 디버깅 시 로그만 보고는 "이 progress 이벤트가 어떤 요청에 대한 응답인지" 재구성할 수 없다.

**발생 조건**:
- 두 스트림을 나란히 비교하는 A/B 뷰, 여러 파일을 동시에 여는 다중 탭 워크플로.
- 배치 파싱(여러 파일을 순차/병렬로 처리)에서 파일별 진행률을 각각 보여줘야 할 때.

**권장**:
```rust
#[derive(serde::Serialize, Clone)]
struct ProgressPayload { task_id: String, pct: u8 }

fn report_progress(app: &tauri::AppHandle, task_id: &str, pct: u8) {
    app.emit("progress", ProgressPayload { task_id: task_id.to_string(), pct }).ok();
}
```
```typescript
function useTaskProgress(taskId: string) {
  const [pct, setPct] = useState(0);
  useEffect(() => {
    const p = listen<{ task_id: string; pct: number }>("progress", (e) => {
      if (e.payload.task_id === taskId) setPct(e.payload.pct); // 자신의 작업만 반영
    });
    return () => { p.then(f => f()); };
  }, [taskId]);
  return pct;
}
```
- 동시성이 가능한 모든 작업의 이벤트 payload에 `task_id`(또는 correlation id)를 필수 필드로 둔다.
- 부하가 크다면 이벤트 이름 자체를 `progress://{task_id}`처럼 스코프해 리스너 단계에서부터 필터링 비용을 없앨 수도 있다.

**탐지 방법**:
- Code: 동일 이벤트 이름을 사용하는 백엔드 emit 호출이 두 곳 이상이면서 payload 타입에 식별자 필드가 없는지 검사.
- Domain review: 앱이 지원하는 동시 작업 시나리오(비교 모드, 다중 탭, 배치 처리) 목록을 뽑아 각각에 대응하는 이벤트가 식별자를 갖는지 점검.

**예외**:
- 앱 구조상 특정 작업 종류가 항상 단일 인스턴스로만 존재한다는 것이 보장되면(예: 파일을 한 번에 하나만 열 수 있는 모드) id 없이도 안전하다.

**Bitvue 판정**: N/A — **정정(재감사)**: broadcast 이벤트가 전무하므로 이름 공유로 섞일 코드 경로 자체가 없다(EVT-001 참고). A/B 비교 모드 커맨드(`openStream`, `getFramesChunk`, `selectFrame`, `getDecodedFrameYuv` 등)는 전부 `stream: "A"|"B"` 매개변수를 명시적으로 받고, `sidecarClient.ts`의 `correlationId`로 호출별 상호 연관까지 보장돼(EVT-002 참고) 동시 작업이 요청-응답 계층에서부터 이미 구조적으로 구분된다. 단, 이를 실제로 소비하는 `CompareWorkspace.tsx`(및 `StreamPlayer.tsx`/`DiffOverlay.tsx`)는 grep 확인 결과 자기 디렉터리 밖 어디서도 import되지 않는 죽은 트리라 이 경로는 현재 앱에서 도달 불가 — 이론상으로만 유효한 N/A.

---

### TAURI-EVT-010: frontend 재시작 후 backend 상태와 재동기화 불가

**분류**: 재기동 복원력 · **심각도**: High · **탐지**: Interaction

**나쁜 예**:
```rust
// AppState는 백엔드에 그대로 살아있다 — 로드된 파일, 디코드된 프레임 캐시, 분석 결과
pub struct AppState {
    pub loaded_file: Option<PathBuf>,
    pub decoded_frames: HashMap<u32, FrameSummary>,
    pub current_frame: u32,
}
// 하지만 프런트엔드에 이 상태를 통째로 되돌려줄 커맨드가 없다.
// 프런트는 오직 과거에 수신한 이벤트들을 리듀서처럼 누적해 상태를 재구성해 왔다.
```
```typescript
// App.tsx — 새로고침되면 이 초기값에서 다시 시작하고, 백엔드가 이미 파일을 들고 있어도 알 방법이 없다
const [session, setSession] = useState<Session>({ loadedFile: null, currentFrame: 0 });
```

**문제**:
- 개발 중 HMR, 창 크래시 후 Tauri의 자동 재오픈, 사용자의 수동 새로고침(Cmd+R) 등 webview만 리셋되고 Rust 프로세스는 살아있는 상황이 흔한데, 이때 프런트가 오직 "지금까지 받은 이벤트의 누적"으로만 상태를 구성했다면 완전히 리셋된다.
- 사용자 입장에서는 이미 불러온 대용량 파일을 다시 열고 처음부터 파싱을 재시작해야 하는 것처럼 보인다 — 백엔드는 이미 결과를 갖고 있는데도.
- 이벤트를 상태의 유일한 원천으로 삼으면, 이벤트 시스템의 어떤 결함(리스너 등록 타이밍, 유실)도 곧바로 전체 세션 유실로 이어진다.

**발생 조건**:
- 개발 환경에서의 잦은 HMR/리로드.
- 여러 창을 열고 닫을 때 특정 창만 재시작되는 경우.
- 크래시 리포터가 webview를 재생성하지만 백엔드 프로세스는 유지하는 복구 경로.

**권장**:
```rust
#[tauri::command]
fn get_session_state(state: tauri::State<AppState>) -> SessionSnapshot {
    let s = state.lock().unwrap();
    SessionSnapshot {
        loaded_file: s.loaded_file.clone(),
        current_frame: s.current_frame,
        frame_count: s.decoded_frames.len(),
    } // 백엔드 상태를 있는 그대로 스냅샷으로 노출
}
```
```typescript
// App.tsx 초기화 시 항상 백엔드를 진실의 원천으로 취급해 재동기화한다
useEffect(() => {
  invoke<SessionSnapshot>("get_session_state").then((snapshot) => {
    setSession(snapshot); // 이벤트 누적이 아니라 쿼리로 상태를 복원
  });
}, []);
```
- Rust `AppState`를 진실의 원천(source of truth)으로 삼고, 프런트엔드 상태는 그 투영(projection)으로 설계한다.
- 창/앱 초기화 시점에 항상 전체 스냅샷 쿼리를 먼저 호출하고, 그 이후부터만 이벤트로 델타를 반영한다(TAURI-EVT-003과 동일한 원칙의 앱 전역 버전).

**탐지 방법**:
- Interaction: 파일을 로드하고 파싱이 진행 중이거나 끝난 상태에서 webview를 새로고침(Cmd+R)해, 백엔드가 이미 가진 상태가 화면에 복원되는지 수동으로 확인.
- Code: 앱 최상위 초기화 로직(`App.tsx` 등)에 전체 세션을 되돌려주는 `invoke` 호출이 없는지 검토.

**예외**:
- 상태가 없는(stateless) 단일 화면 워크플로(파일을 고르기 전 초기 랜딩 화면 등)는 새로고침 시 처음부터 시작하는 것이 자연스러운 동작이라 재동기화가 불필요하다.

**Bitvue 판정**: Confirmed — **정정(재감사, src-tauri 대신 현재 sidecar 아키텍처로 재확인, 동일 결론이 더 강하게 재확인됨)**: 이제 상태를 들고 있는 것은 Tauri `AppState`가 아니라 별도 프로세스인 `bitvue-sidecar`(`bitvue_engine::Core`)이며, `bitvue-desktop/electron/main.ts`의 주석(~1276-1280줄)이 이를 직접 인정한다 — "이건 프로세스를 재시작하는 것이지 `Core`의 인메모리 상태를 재시작하는 게 아니다: 크래시에서 살아남는 열린 스트림/선택 상태는 없다"(원문 요지). 이 사실을 알리는 유일한 통로가 `bitvue:sidecar-restarted` push 이벤트인데, (1) 저장소 전체에서 이 이벤트를 실제로 구독하는 프런트 코드가 전무하다(`onSidecarRestarted`는 `electronBridgeService.ts`의 타입 선언과 테스트 mock에만 존재, 호출부 0건 — grep으로 확인) — `main.ts` 자신도 "실제 UI가 반응할 게 아직 없어서 이 셸은 그렇게 안 한다"고 명시. (2) 설령 리스너가 있어도 되돌려 받을 스냅샷 쿼리 자체가 브리지 API에 없다 — `electronBridgeService.ts` 전체에 `getSessionState`/`SessionSnapshot`류 함수가 0건. `App.tsx`의 어떤 `useEffect`도 마운트 시점에 "지금 뭐가 열려있나"를 되묻지 않는다(`getStreamInfo`는 오직 `utils/progressiveLoader.ts`의 활성 로딩 흐름 중에만 호출됨). 따라서 sidecar 크래시+자동재시작이든 renderer 새로고침(Cmd+R)이든, 카탈로그가 경고하는 "이벤트만이 유일한 진실 원천이라 재동기화 불가" 상태가 이전보다 더 명확하게(코드 주석으로 자인된 채) 재현된다.

---

## 원칙

event 자체를 truth source로 만들면 안 된다 — event는 변화의 알림일 뿐, 상태 자체는 query로 조회해야 한다. Command(요청-응답), Query(현재 상태 조회), Event(변화 알림), Snapshot(초기/재동기화 시 전체 상태 복원)의 역할을 분리하고, 이벤트는 항상 그에 대응하는 스냅샷 쿼리와 짝을 이루도록 설계한다.
