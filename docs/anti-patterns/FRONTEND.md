# Anti-Pattern Catalog — FRONTEND: 프런트엔드 렌더링 연계

이 카탈로그는 더 큰 Anti-Pattern Catalog(`docs/anti-patterns/INDEX.md`, 별도 작성 예정)의 한 파트이며, 원 설계 논의에서는 요약 표에 "프런트엔드 렌더링 연계 — 15 items"로 규모만 언급되고 개별 항목은 나열되지 않았던 카테고리를 카탈로그 저술 과정에서 새로 항목화한 것이다. Rust 쪽의 `PIXEL`(디코드 YUV → RGBA → PNG/base64 → WebView 전달 파이프라인)과 `IPC`(Tauri 커맨드/이벤트 설계) 카테고리가 다루는 문제의 프런트엔드측 거울상(mirror image)에 해당하며, 두 문서와 짝을 이루어 읽는 것을 권장한다. 이 문서가 캔버스 레이어 분리·좌표계·HiDPI·WebGL 업로드·가상화 등 "렌더링 메커니즘·레이어 분리" 중심이라면, 같은 Phase 3 웨이브의 `docs/anti-patterns/FRONT_REACT.md`는 그 위 층위인 React 상태 아키텍처(store/selector 설계)와 렌더 비용 패턴을 다루는 짝 문서다.

---

### FE-01: 레이어 미분리 — 프레임/오버레이/선택윤곽 단일 캔버스·단일 렌더패스
**분류**: 렌더 레이어 아키텍처 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```typescript
function FrameCanvas({ frame, overlayMode, zoom, pan, hoveredBlock, selection }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // frame, overlayMode, zoom, pan, hoveredBlock, selection 중 무엇이 바뀌어도
  // 텍스처 디코드 + 오버레이 계산 + 선택 윤곽을 전부 한 캔버스에 처음부터 다시 그린다.
  useEffect(() => {
    const ctx = canvasRef.current!.getContext('2d')!;
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    drawYuvFrame(ctx, frame, zoom, pan);           // 무거움: 프레임이 안 바뀌어도 매번 실행
    if (overlayMode !== 'none') drawOverlay(ctx, frame.blocks, overlayMode, zoom, pan); // 무거움
    if (hoveredBlock) drawHoverHighlight(ctx, hoveredBlock, zoom, pan); // 가벼움인데 위 둘을 다시 태움
    if (selection) drawSelectionOutline(ctx, selection, zoom, pan);
  }, [frame, overlayMode, zoom, pan, hoveredBlock, selection]);

  return <canvas ref={canvasRef} width={1920} height={1080} />;
}
```

**문제**:
- 마우스가 블록 위를 지나가며 `hoveredBlock`만 바뀌어도(초당 수십 회) 프레임 디코드 텍스처 드로우와 QP 히트맵/MV 필드 오버레이 계산까지 매번 다시 수행된다.
- "frame이 바뀔 때만 텍스처 갱신, mode/zoom 변경시만 overlay 갱신, selection outline은 즉시 렌더"라는 3단 갱신 주기 요구사항을 단일 `useEffect`/단일 캔버스가 구조적으로 표현할 수 없다.
- 프레임과 오버레이가 같은 픽셀 버퍼를 공유하므로, 오버레이만 지우고 다시 그리려 해도 프레임까지 다시 디코드해야 함 — 부분 무효화(invalidation)가 불가능.

**발생 조건**:
- 4K/8K 프레임 + 조밀한 MB 단위 오버레이(수천 블록)에서 60fps 호버 반응성을 요구할 때 프레임 드롭이 즉시 드러남.
- 타임라인 스크럽 중 프레임은 그대로인데 확대/축소만 조작하는 경우에도 매 프레임 텍스처를 재업로드하게 됨.

**권장**:
```typescript
// 3개의 스택된 캔버스(or WebGL 레이어)로 분리하고 갱신 트리거를 독립시킨다.
function FrameCanvasStack({ frame, overlayMode, zoom, pan, hoveredBlock, selection }: Props) {
  const frameLayer = useCanvasLayer();
  const overlayLayer = useCanvasLayer();
  const selectionLayer = useCanvasLayer();

  useEffect(() => {                      // frame이 바뀔 때만
    drawYuvFrame(frameLayer.ctx, frame, zoom, pan);
  }, [frame, zoom, pan]);

  useEffect(() => {                      // overlayMode/frame/zoom 변경 시에만
    if (overlayMode === 'none') { overlayLayer.clear(); return; }
    drawOverlay(overlayLayer.ctx, frame.blocks, overlayMode, zoom, pan);
  }, [frame, overlayMode, zoom, pan]);

  useEffect(() => {                      // 선택/호버는 별도 저비용 레이어, 즉시 갱신
    selectionLayer.clear();
    if (hoveredBlock) drawHoverHighlight(selectionLayer.ctx, hoveredBlock, zoom, pan);
    if (selection) drawSelectionOutline(selectionLayer.ctx, selection, zoom, pan);
  }, [hoveredBlock, selection, zoom, pan]);

  return (
    <div className="canvas-stack">
      <canvas ref={frameLayer.ref} />
      <canvas ref={overlayLayer.ref} />
      <canvas ref={selectionLayer.ref} />
    </div>
  );
}
```
- CSS `position: absolute`로 3개 캔버스를 스택하고 각 레이어를 독립된 의존성 배열의 `useEffect`(또는 별도 컴포넌트 + `memo`)로 갱신한다.
- WebGL을 쓰는 경우 프레임 텍스처와 오버레이를 별도 FBO/텍스처 유닛으로 두고, 선택 윤곽만 매 프레임 다시 그리는 저비용 draw call로 분리한다.

**탐지 방법**:
- Structural: 캔버스/WebGL 컨텍스트 개수가 1개인데 `useEffect` 의존성 배열에 hover류 state와 frame류 state가 섞여 있는지 검사.
- Runtime: React DevTools Profiler + Chrome Performance 패널에서 호버만 발생했는데 프레임 디코드/오버레이 함수 호출이 잡히는지 확인.

**예외**:
- 오버레이가 없는 단순 뷰어(원본 프레임만 표시)이거나 정적 스냅샷 내보내기 용도라면 단일 캔버스로 충분.

**Bitvue 판정**: Confirmed — `frontend/components/panels/YuvViewerPanel/VideoCanvas.tsx:157-222`. 단일 `<canvas>`(`canvasRef`)의 동일 `ctx`에 YUV 프레임 렌더(`rendererRef.current.render`)와 모드 오버레이(`renderModeOverlay`)를 하나의 `useEffect`(deps: `frameImage, yuvData, currentMode, currentFrame, activeOverlays, av1Features, colorspace, channelMode`)에서 순서대로 그린다. 별도 WebGL 캔버스(`webglCanvasRef`)가 있지만 이는 MV 필드 전용 고밀도 경로일 뿐, 프레임/오버레이 분리 레이어는 아니다. 다만 현재 hover/selection 상태가 이 캔버스 렌더링에 연결돼 있지 않아(코드베이스에 `hoveredBlock`류 픽셀 단위 호버가 프레임 뷰어에 없음) 원문이 지적하는 "호버마다 전체 재드로우" 체감 문제는 아직 실증되지 않음.

---

### FE-02: base64 data-URL로 프레임 이미지를 표시 (ImageBitmap/텍스처 업로드 미사용)
**분류**: 데이터 전달·디코드 경로 · **심각도**: High · **탐지**: Static

**나쁜 예**:
```typescript
function FramePreview({ frameIndex }: { frameIndex: number }) {
  const [dataUrl, setDataUrl] = useState<string>('');

  useEffect(() => {
    invoke<string>('get_frame_png_base64', { frameIndex }).then((b64) => {
      setDataUrl(`data:image/png;base64,${b64}`);
    });
  }, [frameIndex]);

  return <img src={dataUrl} />; // 매 프레임마다 base64 디코드 + <img> 파이프라인 재구동
}
```

**문제**:
- Rust 쪽에서 이미 PNG 인코딩 비용을 지불했는데(이는 `PIXEL` 카탈로그의 안티패턴이기도 함), 프런트엔드가 이를 다시 base64 문자열 → `<img>` → 브라우저 내부 디코더 → GPU 업로드라는 긴 경로로 흘려보낸다.
- base64는 원본 바이너리 대비 ~33% 크기 증가, 문자열 처리이므로 V8/JS 힙에 큰 문자열 객체가 매 프레임 생성되어 GC 압력 유발.
- `<img>` 엘리먼트는 캔버스/WebGL 오버레이와 픽셀 좌표계를 공유하기 어렵고, 줌/팬 시 별도 CSS transform 동기화가 필요해 좌표계 불일치(FE-10) 위험을 키운다.

**발생 조건**:
- 타임라인을 빠르게 스크럽하며 프레임을 연속 전환할 때(초당 수 프레임) 매번 새 data URL을 만들면서 이전 프레임 파싱이 채 끝나기도 전에 다음 요청이 들어와 화면이 버벅임.
- 4K 프레임에서 특히 체감 지연이 큼.

**권장**:
```typescript
async function loadFrameBitmap(frameIndex: number): Promise<ImageBitmap> {
  // Rust 쪽에서 raw RGBA 버퍼(또는 압축이 필요하면 별도 채널)를 바이너리로 전달
  const raw: Uint8ClampedArray = await invoke('get_frame_rgba_raw', { frameIndex });
  const imageData = new ImageData(raw, width, height);
  return createImageBitmap(imageData); // 디코드를 워커 스레드에서 수행 가능
}

function FrameCanvas({ frameIndex }: { frameIndex: number }) {
  const layer = useCanvasLayer();
  useEffect(() => {
    let cancelled = false;
    loadFrameBitmap(frameIndex).then((bitmap) => {
      if (cancelled) return;
      layer.ctx.drawImage(bitmap, 0, 0);   // 또는 WebGL texImage2D(bitmap)
      bitmap.close();
    });
    return () => { cancelled = true; };
  }, [frameIndex]);
  return <canvas ref={layer.ref} />;
}
```
- `createImageBitmap`은 메인 스레드를 막지 않고 디코드하며, WebGL `texImage2D`에 직접 넘길 수 있어 중간 `<img>`/DOM 경로가 사라진다.
- 가능하면 Rust ↔ WebView 사이에 Tauri의 raw byte 채널(zero-copy에 가까운 IPC)을 쓰고, PNG 인코딩 자체를 생략한다 — 이는 `PIXEL.md`가 다루는 인코드측 안티패턴과 대칭.

**탐지 방법**:
- Static: `data:image/`, `btoa(`, `base64` 문자열이 프레임/픽셀 데이터 경로에 등장하는지 grep.
- Runtime: Performance 패널에서 `Image Decode` + 대형 문자열 할당이 프레임 전환마다 반복되는지 확인.

**예외**:
- 프레임 하나짜리 정적 썸네일(파일 저장/내보내기, 공유 링크 미리보기)처럼 실시간성이 필요 없는 경로는 base64도 무방.

**Bitvue 판정**: Confirmed — `frontend/components/panels/YuvViewerPanel/index.tsx:223-248`. YUV 경로가 실패했을 때 폴백으로 `get_decoded_frame`이 반환한 PNG를 `img.src = \`data:image/png;base64,${result.frame_data}\`` 형태로 그대로 `<img>`에 흘려보내는, 문서의 나쁜 예와 정확히 일치하는 코드가 존재. 같은 파일 `:56-64`의 `base64ToUint8`(`atob` + 1바이트씩 `charCodeAt` 루프)도 YUV Y/U/V 평면을 매 프레임 base64 문자열로 받아 수동 디코드하는 동일 계열 패턴.

---

### FE-03: 수천 개의 신택스 트리 노드를 평면 React state로 보관
**분류**: 상태 설계 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```typescript
function SyntaxTreePanel({ frameIndex }: { frameIndex: number }) {
  const [nodes, setNodes] = useState<SyntaxNode[]>([]); // 수만 개 노드가 통째로 하나의 state

  useEffect(() => {
    invoke<SyntaxNode[]>('get_syntax_tree', { frameIndex }).then(setNodes);
  }, [frameIndex]);

  return (
    <div>
      {nodes.map((n) => (
        <SyntaxNodeRow key={n.id} node={n} allNodes={nodes} /> // allNodes 전체를 자식에 전달
      ))}
    </div>
  );
}
```

**문제**:
- 노드 하나를 펼치거나 접을 때(`expanded` 토글)마다 부모의 `nodes` state 전체가 새 참조로 바뀌면서 React가 수만 개 자식 컴포넌트의 reconciliation을 다시 수행.
- 자식에게 `allNodes` 전체 배열을 내려주면 `React.memo`가 얕은 비교로 무력화되어 메모이제이션 효과가 사라짐.
- 트리 구조(부모-자식 포인터/중첩 배열)를 매번 통째로 setState하면 불변성 유지를 위한 깊은 복사 비용도 커진다.

**발생 조건**:
- HEVC/AV1처럼 신택스 계층이 깊고(SPS/PPS → 슬라이스 → CTU → CU → TU → ...) 프레임당 노드 수가 수천~수만에 달하는 코덱에서 두드러짐.
- 트리 검색/필터링 기능과 결합되면 매 키 입력마다 전체 재계산까지 겹쳐 입력 렉이 발생.

**권장**:
```typescript
// 정규화된 상태: id -> node, 그리고 펼침 여부는 별도의 작은 Set으로 분리
const nodesById = useMemo(() => normalizeToMap(rawTree), [rawTree]); // frameIndex 바뀔 때만
const [expandedIds, setExpandedIds] = useState<ReadonlySet<string>>(new Set());

const toggle = useCallback((id: string) => {
  setExpandedIds((prev) => {
    const next = new Set(prev);
    next.has(id) ? next.delete(id) : next.add(id);
    return next;
  });
}, []);

const SyntaxNodeRow = React.memo(function SyntaxNodeRow({ id }: { id: string }) {
  const node = nodesById.get(id)!;         // context/selector로 조회, props로 큰 배열 안 내려줌
  const expanded = useIsExpanded(id);      // expandedIds만 구독하는 selector 훅
  // ...
});
```
- 트리 데이터(자주 안 바뀜)와 UI 상태(펼침/선택, 자주 바뀜)를 별도의 state/컨텍스트로 분리한다.
- `React.memo` + `id` 기반 props(전체 객체 대신 id)로 자식이 자신과 무관한 갱신에 반응하지 않게 한다.
- 대규모 트리는 FE-04의 가상화와 결합해야 실효가 있다.

**탐지 방법**:
- Structural: 컴포넌트 props로 원본 배열/트리 전체가 그대로 전달되는지, `useState`에 파싱 결과 원본이 그대로 들어가는지 검사.
- Runtime: React DevTools Profiler "why did this render" — 무관한 상태 변경 시 트리 전체가 재렌더되는지 확인.

**예외**:
- 노드 수가 수십~수백 개로 작은 컨테이너(MP4 box 트리 등)라면 굳이 정규화하지 않아도 체감 문제 없음.

**Bitvue 판정**: Confirmed — `frontend/components/panels/SyntaxDetailPanel/index.tsx:100`에서 `setExpandedNodes(prev => new Set(prev)...)`로 토글마다 새 `Set` 참조를 만들고, 이 `expandedNodes` 전체 Set을 `FrameSyntaxTab.tsx`의 `SyntaxTreeNode`(`React.memo`, `:342`)/`VirtualSyntaxTree`(`:233`)에 통째로 prop으로 내려준다. 자식이 `memo`로 감싸져 있어도 prop이 매번 새 참조라 얕은 비교가 무력화되는, 문서가 지적한 정확한 패턴. 다만 노드 자체(`nodesById`류 정규화 맵)는 없지만 대신 아래 FE-04처럼 가시 범위만 평탄화하는 자체 가상화(`flattenVisible`)가 있어 실제 DOM 재조정 규모는 제한적.

---

### FE-04: 신택스 트리/헥스 뷰/필름스트립에 가상화(virtualization) 부재
**분류**: 대량 리스트 렌더링 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```typescript
function HexView({ bytes }: { bytes: Uint8Array }) {
  const rows = chunk(bytes, 16); // 4K 프레임의 NAL unit이면 수십만 행

  return (
    <div className="hex-view" style={{ overflowY: 'auto' }}>
      {rows.map((row, i) => (
        <HexRow key={i} offset={i * 16} bytes={row} /> // 뷰포트 밖 행까지 전부 DOM에 마운트
      ))}
    </div>
  );
}
```

**문제**:
- 뷰포트에 실제로 보이는 것은 20~40행뿐인데 수만~수십만 개의 DOM 노드를 생성 — 최초 렌더/스크롤 모두 느려짐.
- 필름스트립도 동일: 1만 프레임짜리 시퀀스를 열면 1만 개의 썸네일 `<canvas>`/`<img>`를 한 번에 마운트해 GPU 메모리와 레이아웃 비용이 폭증.
- 브라우저 레이아웃/페인트가 리스트 길이에 선형 이상으로 느려져, 스크롤 자체가 끊기는 체감 프레임 드롭으로 나타남.

**발생 조건**:
- 대용량 파일(수백 MB급 비트스트림), 고프레임률/장시간 시퀀스, 세밀한 바이트 단위 헥스 덤프에서 즉시 문제화.
- 특히 CI가 없는 수동 QA에서는 "짧은 샘플 파일"로만 테스트하면 이 문제가 드러나지 않고 그대로 릴리스되기 쉬움.

**권장**:
```typescript
import { useVirtualizer } from '@tanstack/react-virtual';

function HexView({ bytes }: { bytes: Uint8Array }) {
  const rowCount = Math.ceil(bytes.length / 16);
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 20,
    overscan: 8,
  });

  return (
    <div ref={parentRef} className="hex-view" style={{ overflowY: 'auto' }}>
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualizer.getVirtualItems().map((v) => (
          <HexRow
            key={v.index}
            style={{ position: 'absolute', top: v.start, height: v.size }}
            offset={v.index * 16}
            bytes={bytes.subarray(v.index * 16, v.index * 16 + 16)}
          />
        ))}
      </div>
    </div>
  );
}
```
- 신택스 트리는 펼쳐진 노드만 평탄화한 리스트를 가상화(`react-arborist`, `react-window` 등)로 렌더링.
- 필름스트립은 화면에 보이는 범위 ± overscan만 실제 썸네일을 요청/렌더하고, 나머지는 플레이스홀더로 대체.

**탐지 방법**:
- Structural: 리스트 렌더링 지점에서 `.map()`이 가상화 라이브러리 훅 없이 원본 배열 길이만큼 그대로 도는지 검사.
- Runtime: 대용량 픽스처로 열었을 때 DOM 노드 수(`document.querySelectorAll('.hex-row').length`)와 초기 렌더 시간을 측정.

**예외**:
- 리스트 길이가 항상 작다는 것이 도메인상 보장되는 경우(예: SPS/PPS 개수처럼 본질적으로 적은 개수)는 가상화가 과설계.

**Bitvue 판정**: N/A — 세 대상 모두 이미 완화되어 있음: (1) 필름스트립은 `frontend/components/Filmstrip.tsx:65,255`의 `useVirtualizedView = frames.length >= VIRTUALIZATION_THRESHOLD` 분기로 `VirtualizedThumbnailsView`를 사용. (2) 신택스 트리는 `FrameSyntaxTab.tsx:190-328`에 자체 windowing(`VIRTUALIZATION_THRESHOLD=120`, `scrollTop`/`containerHeight` 기반 슬라이스)이 구현됨. (3) 헥스 뷰는 `HexViewTab.tsx:60`의 `get_frame_hex_data` 호출이 `maxBytes: 2048`로 백엔드에서 미리 잘라 최대 ~128줄만 렌더 — 라이브러리 기반 가상화는 아니지만 DOM 노드 폭증 문제 자체가 발생하지 않음.

---

### FE-05: 백엔드 페이로드를 매 렌더마다 재파싱/재변환 (메모이제이션 부재)
**분류**: 계산 캐싱 · **심각도**: High · **탐지**: Static

**나쁜 예**:
```typescript
function QpHeatmapOverlay({ frame, colorScale }: { frame: FrameData; colorScale: ColorScale }) {
  // 렌더될 때마다(부모 리렌더 포함) blocks를 순회하며 색상 버퍼를 새로 계산
  const heatmapPixels = frame.blocks.map((b) => qpToColor(b.qp, colorScale));

  return <OverlayCanvas pixels={heatmapPixels} />;
}
```

**문제**:
- `frame`과 `colorScale`이 안 바뀌었는데도 부모의 관련 없는 state(예: 사이드바 토글) 변경으로 리렌더될 때마다 블록 수천 개에 대한 색상 매핑을 다시 계산.
- 계산 비용이 렌더 경로(커밋 전 렌더 단계)에 그대로 얹혀 프레임 드롭으로 직결.
- 파생 데이터를 매번 새 배열로 만들면 하위 캔버스 컴포넌트의 `memo`도 무력화(참조가 매번 바뀌므로).

**발생 조건**:
- 오버레이 계산이 O(블록 수) 이상(예: MV 필드처럼 인접 블록 보간이 들어가는 경우)일 때, 그리고 상위 트리에 빈번히 바뀌는 무관 state(호버, 툴팁, 타이머)가 섞여 있을 때.

**권장**:
```typescript
function QpHeatmapOverlay({ frame, colorScale }: { frame: FrameData; colorScale: ColorScale }) {
  const heatmapPixels = useMemo(
    () => frame.blocks.map((b) => qpToColor(b.qp, colorScale)),
    [frame.id, colorScale], // frame 객체 전체가 아니라 안정적인 식별자로 의존성 지정
  );

  return <OverlayCanvas pixels={heatmapPixels} />;
}
```
- 파생 계산은 `useMemo`로 감싸고 의존성은 객체 참조가 아니라 `frame.id`처럼 안정적인 값으로 잡는다(참조가 매 IPC 응답마다 새로 생성되는 경우가 많으므로).
- 계산이 무겁다면 Web Worker로 옮기고 결과만 상태로 반영(FE-14 참고).

**탐지 방법**:
- Static: 렌더 함수 본문에 `.map()/.filter()/.reduce()` 등 O(n) 이상 연산이 `useMemo` 없이 노출돼 있는지 grep.
- Runtime: React DevTools Profiler에서 무관 state 변경 시 해당 컴포넌트의 렌더 시간이 비정상적으로 긴지 확인.

**예외**:
- 계산이 충분히 저렴(수십 개 이하 항목, O(1)에 가까운 변환)하면 메모이제이션 자체의 오버헤드(의존성 비교)가 더 클 수 있음.

**Bitvue 판정**: N/A — `frontend/components/panels/YuvViewerPanel/index.tsx:505-508`에서 `convertedYuvFrame`을 `useMemo(..., [yuvData])`로 명시적으로 감싸고 주석까지 "avoid re-running on every render"로 남겨둠. 오버레이 계산(QP/MV 등)도 렌더 본문이 아니라 `VideoCanvas.tsx`의 `useEffect`(안정적 deps) 내부에서 명령형으로 실행되므로 부모의 무관한 리렌더에 반응하지 않음.

---

### FE-06: 호버/툴팁 상태가 트리 루트에 위치해 전체 서브트리 리렌더 유발
**분류**: 상태 위치·전파 범위 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```typescript
function AnalyzerRoot() {
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null); // 최상위에 위치

  return (
    <>
      <SyntaxTreePanel hoveredNodeId={hoveredNodeId} onHover={setHoveredNodeId} />
      <HexViewPanel hoveredNodeId={hoveredNodeId} />       {/* 무관한데 매 호버마다 리렌더 */}
      <FrameCanvas hoveredNodeId={hoveredNodeId} />         {/* 무관한데 매 호버마다 리렌더 */}
      <FilmstripPanel />
    </>
  );
}
```

**문제**:
- 마우스 이동마다 바뀌는 `hoveredNodeId`가 앱 최상위 state이면, 이를 구독하는 모든 자식 트리(신택스 트리, 헥스 뷰, 프레임 캔버스)가 초당 수십 회 리렌더된다.
- 실제로 호버 하이라이트가 필요한 것은 "해당 노드 행"과 "해당 바이트 범위/블록" 정도인데, 관련 없는 패널까지 리렌더 파급.
- 컨텍스트로 전역 공유해도 구독 범위를 좁히지 않으면 동일한 문제가 컨텍스트 소비자 전체로 옮겨갈 뿐.

**발생 조건**:
- 여러 패널이 "같은 신택스 노드/바이트 범위를 하이라이트로 동기화"해야 하는 교차 패널 하이라이트 기능에서 특히 두드러짐.

**권장**:
```typescript
// 호버 상태를 구독 범위가 좁은 별도 store(예: 작은 pub/sub 또는 selector 지원 컨텍스트)로 분리
const hoverStore = createHoverStore(); // zustand 등, selector 기반

function useHoveredForRange(start: number, end: number) {
  return hoverStore((s) => s.hoveredByteOffset != null &&
    s.hoveredByteOffset >= start && s.hoveredByteOffset < end);
}

function HexRow({ offset }: { offset: number }) {
  const isHovered = useHoveredForRange(offset, offset + 16); // 이 행과 무관한 호버는 리렌더 안 됨
  // ...
}
```
- 호버/툴팁처럼 고빈도로 바뀌는 상태는 React state가 아니라 selector 기반 외부 store(zustand, jotai atom family 등)로 분리해 "관련된 컴포넌트만" 구독하게 한다.
- 정 React state로 가야 한다면 상태를 실제로 필요한 최소 공통 조상까지 내려서(context 분할) 구독 범위를 좁힌다.

**탐지 방법**:
- Structural: 최상위/공통 조상 컴포넌트의 state 중 이벤트 핸들러 이름이 `onMouseMove`/`onHover`류인 것이 몇 단계 아래까지 props로 전파되는지 추적.
- Runtime: 호버 이동 중 React Profiler flamegraph에서 무관 패널까지 커밋되는지 확인.

**예외**:
- 패널 수가 적고(2~3개) 트리 깊이가 얕아 리렌더 비용이 무시할 만한 초기 프로토타입 단계.

**Bitvue 판정**: Suspected — `frontend/contexts/SyntaxHexLinkContext.tsx`가 selector 없는 일반 React Context로 `highlightedByteOffset`을 관리하고 `SyntaxDetailPanel`/`HexViewTab` 등 여러 소비자가 구독하는 구조는 형태상 원문과 같다. 다만 현재는 신택스 노드 "클릭" 시에만 값이 바뀌고(`onJumpToHex`) `mousemove` 같은 고빈도 이벤트에 연결돼 있지 않아, 원문이 지적하는 초당 수십 회 리렌더 체감 문제가 실제로 발생하는지는 확인하지 못함.

---

### FE-07: 스크럽/줌/팬 이벤트에 debounce/throttle 없이 IPC를 그대로 흘려보냄
**분류**: 이벤트 → IPC 트래픽 제어 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```typescript
function Timeline({ totalFrames }: { totalFrames: number }) {
  const handleScrub = (e: React.MouseEvent) => {
    const frameIndex = xToFrameIndex(e.clientX, totalFrames);
    invoke('seek_and_decode_frame', { frameIndex }); // mousemove마다 그대로 호출
  };

  return <div className="timeline" onMouseMove={handleScrub} />;
}
```

**문제**:
- `mousemove`는 브라우저에서 초당 수십~백여 회 발생할 수 있는데, 이를 그대로 Tauri IPC 커맨드(디코드를 트리거하는 무거운 커맨드)에 매핑하면 백엔드 디코드 큐가 폭주.
- 응답이 순서대로 안 돌아올 수 있어(느린 디코드가 나중에 완료) FE-17의 레이스 컨디션과 결합해 "스크럽을 멈췄는데 화면이 계속 늦게 바뀌는" 체감 버그가 발생.
- 프런트엔드 이벤트 루프와 백엔드 디코드 스레드 모두 불필요한 작업으로 점유되어 실제 사용자가 원하는 마지막 프레임 표시가 오히려 늦어짐.

**발생 조건**:
- 긴 시퀀스에서 빠르게 스크럽하거나, 트랙패드로 관성 스크롤/줌을 할 때 이벤트 빈도가 특히 높음.

**권장**:
```typescript
function Timeline({ totalFrames }: { totalFrames: number }) {
  const requestSeek = useMemo(
    () => throttle((frameIndex: number) => invoke('seek_and_decode_frame', { frameIndex }), 50,
      { leading: true, trailing: true }),
    [],
  );

  // 손을 뗀 시점에는 정확한 최종 프레임을 반드시 한 번 더 요청(trailing 보장) + 취소 가능한 in-flight 관리
  const handlePointerUp = (e: React.PointerEvent) => {
    requestSeek.flush();
  };

  const handleScrub = (e: React.PointerEvent) => {
    requestSeek(xToFrameIndex(e.clientX, totalFrames));
  };

  useEffect(() => () => requestSeek.cancel(), [requestSeek]);

  return <div className="timeline" onPointerMove={handleScrub} onPointerUp={handlePointerUp} />;
}
```
- 커서 이동/프리뷰처럼 "중간 값은 버려도 되는" 이벤트는 throttle, "마지막 값은 반드시 반영돼야 하는" 액션은 trailing debounce로 보완.
- 요청 자체에 세대(generation)/토큰을 부여해 오래된 응답이 최신 화면을 덮어쓰지 않게 한다(FE-17과 연동).

**탐지 방법**:
- Runtime: 스크럽 중 `invoke` 호출 횟수를 계측해 마우스 이벤트 수와 1:1로 따라가는지 확인.
- Static: `onMouseMove`/`onWheel`/`onPointerMove` 핸들러 내부에서 직접 `invoke(...)`가 호출되는 패턴을 grep.

**예외**:
- IPC 커맨드가 즉시 반환되는 순수 로컬 캐시 조회(디코드를 트리거하지 않는 조회)라면 throttle 없이도 부담이 적을 수 있음.

**Bitvue 판정**: N/A — `frontend/components/Timeline.tsx:140-171`의 드래그 스크럽(`handleDragMove`)은 `mousemove`마다 로컬 state(`setHighlightedFrameIndex`, `setHoverPosition`)만 갱신하고, 실제 프레임 선택/디코드를 트리거하는 `setFrameSelection` 호출은 `handleDragUp`(mouseup)에서 단 한 번만 실행된다. 즉 IPC 자체가 스크럽 중에는 발생하지 않도록 설계돼 있어 throttle이 불필요. 코드베이스 전체에서 `onMouseMove`/`onPointerMove` 핸들러 내부에 직접 `invoke(...)`가 있는 지점은 찾지 못함.

---

### FE-08: 캔버스 컨텍스트를 매 렌더마다 새로 생성
**분류**: 렌더 리소스 재사용 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```typescript
function OverlayCanvas({ pixels }: { pixels: Uint8ClampedArray }) {
  return (
    <canvas
      ref={(el) => {
        if (!el) return;
        const ctx = el.getContext('2d')!; // 렌더될 때마다 콜백 ref가 재실행되며 컨텍스트 재획득
        ctx.putImageData(new ImageData(pixels, el.width, el.height), 0, 0);
      }}
    />
  );
}
```

**문제**:
- 콜백 `ref`를 인라인 화살표 함수로 넘기면 매 렌더마다 새 함수 참조가 되어, React가 이전 ref를 `null`로 호출한 뒤 다시 새 값으로 호출 — 불필요한 마운트/언마운트성 작업 반복.
- `getContext('2d')`는 매번 새 컨텍스트를 만드는 게 아니라 동일 캔버스에 대해 같은 컨텍스트를 반환하지만, 이 패턴은 매 렌더 그 자체가 다시 실행되어 `putImageData` 같은 무거운 픽셀 쓰기 작업이 렌더 사이클에 종속되어 버린다(FE-05와 유사한 결과).
- WebGL의 경우 컨텍스트 재획득 로직을 잘못 작성하면 실수로 `getContext` 옵션이 매번 다르게 전달되어 브라우저가 새 컨텍스트를 만들려다 실패(캔버스당 컨텍스트는 1회 확정)하는 사례도 흔하다.

**발생 조건**:
- 함수형 컴포넌트에서 인라인 콜백 ref + 그 안에서 직접 드로잉을 수행하는 초기 구현에서 흔함.
- 오버레이 모드를 자주 토글하며 컴포넌트가 조건부로 마운트/언마운트되는 구조와 결합되면 컨텍스트 재획득 비용이 체감됨.

**권장**:
```typescript
function OverlayCanvas({ pixels, width, height }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const ctxRef = useRef<CanvasRenderingContext2D | null>(null);

  useEffect(() => {
    ctxRef.current = canvasRef.current!.getContext('2d'); // 마운트 시 1회만 획득
  }, []);

  useEffect(() => {
    const ctx = ctxRef.current;
    if (!ctx) return;
    ctx.putImageData(new ImageData(pixels, width, height), 0, 0);
  }, [pixels, width, height]);

  return <canvas ref={canvasRef} width={width} height={height} />;
}
```
- 컨텍스트 획득은 마운트 시 1회로 고정하고(`ref` + 빈 의존성 `useEffect`), 실제 드로잉은 데이터가 바뀔 때만 실행되는 별도 effect로 분리.
- WebGL이면 컨텍스트뿐 아니라 셰이더 프로그램/버퍼/텍스처 핸들도 동일하게 마운트 시 1회 생성 후 재사용.

**탐지 방법**:
- Static: `<canvas ref={(el) => { ... el.getContext ... }}>` 형태의 인라인 콜백 ref grep.
- Structural: `getContext` 호출 지점이 `useEffect(() => {...}, [])`(빈 배열) 밖에 있는지 검사.

**예외**:
- 캔버스가 조건부로 완전히 언마운트/재마운트되는 것이 의도된 설계(예: 모드 전환 시 캔버스 자체를 갈아끼우는 구조)라면 재획득이 자연스러움.

**Bitvue 판정**: N/A — `grep`으로 인라인 콜백 `ref={(el) => { ... getContext ... }}` 패턴을 찾지 못함. `getContext('2d')`가 호출되는 지점(`VideoCanvas.tsx:161`, `utils/yuv/renderer.ts:136,185`, `HRDBufferPanel.tsx:98`, `mv-webgl.ts:90`)은 모두 `useEffect`/전용 클래스 메서드 내부에서 데이터 변경 시에만 실행되며, WebGL 경로(`mv-webgl.ts:47`)는 `WeakMap<canvas, state>` 캐시로 컨텍스트/프로그램/버퍼를 마운트 1회만 생성해 재사용한다.

---

### FE-09: WebGL 텍스처 업로드를 메인 스레드에서 동기적으로 수행해 인터랙션 블로킹
**분류**: 메인 스레드 블로킹 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```typescript
function useFrameTexture(gl: WebGL2RenderingContext, frame: FrameData) {
  useEffect(() => {
    const texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, texture);
    // 4K RGBA 프레임(약 33MB)을 메인 스레드에서 그대로 texImage2D — 수십 ms 블로킹 가능
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, frame.width, frame.height, 0,
      gl.RGBA, gl.UNSIGNED_BYTE, frame.rgba);
    gl.generateMipmap(gl.TEXTURE_2D);
  }, [frame]);
}
```

**문제**:
- `texImage2D`에 대용량 버퍼를 동기적으로 넘기면 드라이버가 CPU→GPU 전송 및 포맷 변환을 메인 스레드 호출 안에서 처리 — 그동안 클릭/스크롤/키 입력 등 UI 이벤트 루프가 멈춘다.
- 프레임 전환이 빠른 스크럽 상황과 겹치면 매 프레임 전환마다 짧은 "끊김"이 누적되어 전체적으로 버벅이는 느낌을 준다.
- 메인 스레드 블로킹은 오버레이 레이어(FE-01에서 분리한 selection layer)의 즉시성 요구(호버는 지연 없이 반응해야 함)를 깨뜨린다.

**발생 조건**:
- 4K 이상 해상도, 또는 밉맵 생성처럼 부가 비용이 큰 텍스처 파라미터를 매 프레임 다시 설정할 때.
- 저사양 통합 GPU 환경(CI 러너, 저전력 노트북)에서 특히 두드러짐.

**권장**:
```typescript
// 1) 픽셀 버퍼 준비(디코드/색공간 변환)는 Worker에서, 메인 스레드는 업로드만 담당
// 2) PBO(Pixel Buffer Object) 또는 texSubImage2D로 점진 업로드해 프레임 예산 안에서 분할

function useFrameTexture(gl: WebGL2RenderingContext, frame: FrameData) {
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const bitmap = await frame.bitmapPromise; // 이미 워커/createImageBitmap에서 준비됨
      if (cancelled) return;
      requestAnimationFrame(() => {             // rAF 경계에서 업로드해 입력 이벤트 처리와 스케줄 공존
        gl.bindTexture(gl.TEXTURE_2D, texturePool.acquire());
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, bitmap);
      });
    })();
    return () => { cancelled = true; };
  }, [frame]);
}
```
- 무거운 픽셀 준비 단계는 Web Worker + `OffscreenCanvas`로 옮기고, 메인 스레드는 이미 준비된 `ImageBitmap`을 업로드하는 짧은 작업만 수행.
- 텍스처 객체를 매번 `createTexture`/`deleteTexture`하지 않고 풀링하여 GC/드라이버 오버헤드를 줄인다.
- 대형 텍스처는 `texSubImage2D`로 타일 단위 분할 업로드해 한 프레임 예산(예: 8ms) 내로 쪼갠다.

**탐지 방법**:
- Runtime: Chrome Performance 패널에서 `texImage2D` 호출이 Long Task(50ms 이상)로 잡히는지 확인.
- Manual: 프레임 전환 중 UI(버튼 hover 등)가 순간적으로 먹통이 되는지 수동 QA로 체크.

**예외**:
- 저해상도 프리뷰/썸네일처럼 텍스처가 작아(수십 KB 이하) 업로드 비용이 무시 가능한 경우.

**Bitvue 판정**: Confirmed(변형) — 코드베이스의 메인 프레임 렌더 경로는 WebGL `texImage2D`가 아니라 2D 캔버스지만, 동일한 근본 문제(대용량 픽셀 처리가 메인 스레드 동기 실행)가 그대로 존재한다: `utils/yuv/renderer.ts:210-221`의 `YUVRenderer.render()`가 `yuvToImageData`(프레임 전체 YUV→RGBA 변환, 4K면 830만 픽셀)를 호출한 뒤 `ctx.putImageData`로 그리는데, 이는 `VideoCanvas.tsx`의 프레임 전환 `useEffect`에서 매번 동기 호출된다. 코드베이스 전체에서 Web Worker는 `workers/frameStatsWorker.ts` 하나뿐이고(FE-14 참고) 픽셀 변환/오버레이 계산에는 워커가 전혀 쓰이지 않는다.

---

### FE-10: 백엔드 블록 좌표계와 캔버스 줌/팬 변환의 좌표계 불일치
**분류**: 좌표 변환 정합성 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```typescript
function blockToCanvasRect(block: Block, zoom: number, pan: { x: number; y: number }): Rect {
  // block.x/y는 Rust에서 "16px MB 그리드 인덱스"로 오는데, 프런트는 픽셀 좌표로 착각하고 그대로 곱함
  return {
    x: block.x * zoom + pan.x,
    y: block.y * zoom + pan.y,
    w: block.width * zoom,
    h: block.height * zoom,
  };
}
```

**문제**:
- 백엔드가 블록 좌표를 "픽셀 단위"로 주는지 "그리드 인덱스(예: 16px MB 단위)"로 주는지에 대한 계약이 코드 어디에도 명시되지 않아, 프런트가 임의로 해석 — 오버레이가 실제 블록과 반 블록~한 블록씩 밀려서 그려지는 전형적 off-by-one/scale 버그.
- 크로마 서브샘플링(4:2:0 등)이 있는 코덱에서는 루마/크로마 블록 좌표계가 다를 수 있는데 이를 구분하지 않고 동일 변환식을 적용하면 크로마 관련 오버레이(색차 QP 등)만 어긋남.
- `zoom`/`pan`을 적용하는 기준점(캔버스 좌상단 vs 프레임 중심)이 프레임 렌더링 코드와 오버레이 렌더링 코드에서 서로 다르게 구현되면, 줌 배율이 커질수록 오차가 배수로 벌어져 작은 줌에서는 안 보이던 버그가 고배율에서 두드러진다.

**발생 조건**:
- 코덱이 바뀔 때(블록 그리드 크기가 4x4/8x8/16x16/64x64 등으로 다양) 좌표 스케일 상수를 하드코딩해 재사용하면 특히 잘 드러남.
- 화면 확대(핀치 줌, 200% 이상) 상태에서 오버레이 검증을 처음 해볼 때 발견되는 경우가 많음 — 기본 배율(100%)에서는 오차가 1px 미만이라 안 보일 수 있음.

**권장**:
```typescript
// 좌표계를 명시적 타입으로 구분해 컴파일 타임에 섞이지 않게 한다.
type PixelCoord = { readonly _brand: 'pixel'; x: number; y: number };
type BlockGridCoord = { readonly _brand: 'blockGrid'; col: number; row: number };

function blockGridToPixel(coord: BlockGridCoord, blockSizePx: number): PixelCoord {
  return { _brand: 'pixel', x: coord.col * blockSizePx, y: coord.row * blockSizePx };
}

// 캔버스 변환은 "프레임 원본 픽셀 좌표 -> 화면 좌표" 단 하나의 함수로 통일해
// 프레임 드로잉과 오버레이 드로잉이 동일한 변환 함수를 공유하게 강제한다.
function frameToScreen(p: PixelCoord, view: ViewTransform): ScreenCoord {
  return { x: (p.x - view.originX) * view.zoom + view.panX,
           y: (p.y - view.originY) * view.zoom + view.panY };
}
```
- 백엔드 IPC 응답 스키마(타입 정의)에 좌표 단위를 명시(`blockCol`/`blockRow` vs `pixelX`/`pixelY`)하고, 프런트 타입에도 `brand`로 단위를 구분해 실수로 섞어 쓰면 타입 에러가 나게 만든다.
- 프레임 렌더링과 오버레이 렌더링이 동일한 `frameToScreen` 유틸 함수 하나만 사용하도록 강제(별도 구현 금지).
- 루마/크로마처럼 그리드가 다른 블록은 별도 변환 상수(서브샘플링 비율)를 명시적으로 곱한다.

**탐지 방법**:
- Semantic: IPC로 오는 좌표 필드명과 실제 단위(백엔드 구현)를 대조 — 이름만으로는 판별 불가하므로 Rust측 구조체 문서/주석과 교차 확인 필요.
- Manual/Runtime: 알려진 블록(예: 프레임 좌상단 첫 MB)에 대해 오버레이 사각형과 실제 픽셀 그리드 눈금자를 200%, 400% 줌에서 겹쳐 비교하는 시각적 회귀 테스트.

**예외**:
- 프레임 전체를 덮는 단일 오버레이(블록 단위 세분화가 없는 히트맵 등)는 좌표계가 프레임과 1:1이라 이 문제에서 자유로움.

**Bitvue 판정**: N/A — `QPMapRenderer.tsx:38` 등 오버레이 렌더러들은 백엔드가 준 `block_w`/`block_h`(픽셀 단위)로 `col * block_w`를 그대로 사용해 캔버스 버퍼(=프레임 원본 해상도) 좌표계에 그린다. `zoom`/`pan`은 렌더러 내부에서 곱해지지 않고 `VideoCanvas.tsx:129-135`의 `canvasStyle`(`transform: scale(zoom) translate(...)`) CSS 변환 하나로만 적용되므로, 프레임 드로잉과 오버레이 드로잉이 "같은 변환 함수(사실상 변환 없음 + 단일 CSS transform)"를 공유해 원문이 우려하는 이중 스케일링 불일치가 구조적으로 발생하기 어렵다. 다만 마우스 좌표 → 블록 좌표 역변환이 필요한 호버/클릭-투-셀렉트 블록 기능 자체가 아직 존재하지 않아(`useCanvasInteraction.ts`에 pan/zoom만 있고 블록 피킹 없음) 이 절반의 시나리오는 검증 대상이 없음.

---

### FE-11: 이벤트 핸들러/RAF 콜백의 stale closure가 오래된 frame index를 캡처
**분류**: 클로저·상태 최신성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```typescript
function PlaybackController({ frameIndex, onFrameChange }: Props) {
  useEffect(() => {
    const id = setInterval(() => {
      // frameIndex는 effect가 마운트될 때의 값으로 클로저에 고정 — 이후 갱신되어도 안 바뀜
      onFrameChange(frameIndex + 1);
    }, 33);
    return () => clearInterval(id);
  }, []); // frameIndex를 의존성에서 빠뜨림(의도적으로 "1회만 등록"하려다 발생)

  return null;
}
```

**문제**:
- `setInterval`/`requestAnimationFrame` 콜백이 등록 시점의 `frameIndex`를 클로저로 캡처한 채 고정되어, 재생 중 항상 "초기 프레임 + 1"만 반복 요청하거나 실제 최신 인덱스와 어긋난 프레임을 요청.
- 의존성 배열을 비워 "한 번만 등록"하고 싶은 성능 의도와 "최신 상태를 읽어야 한다"는 정확성 요구가 충돌해서 생기는 전형적 버그 — ESLint `react-hooks/exhaustive-deps`를 끄거나 무시할 때 흔히 발생.
- 키보드 단축키 핸들러(다음/이전 프레임, 줌 등)를 `window.addEventListener`로 마운트 시 1회 등록하는 패턴에서도 동일하게 발생하기 쉽다.

**발생 조건**:
- 재생/자동 진행 기능, 키보드 네비게이션, 마우스 휠 줌처럼 "한 번 등록해서 계속 쓰는" 전역 리스너와 자주 바뀌는 상태(frameIndex, zoom)가 만날 때.

**권장**:
```typescript
function PlaybackController({ frameIndex, onFrameChange }: Props) {
  const frameIndexRef = useRef(frameIndex);
  useEffect(() => { frameIndexRef.current = frameIndex; }, [frameIndex]); // 항상 최신값 유지

  useEffect(() => {
    const id = setInterval(() => {
      onFrameChange(frameIndexRef.current + 1); // ref로 최신값을 읽어 stale closure 회피
    }, 33);
    return () => clearInterval(id);
  }, [onFrameChange]);

  return null;
}
```
- "리스너/타이머는 1회만 등록하되 최신 상태를 읽어야 하는" 경우 `useRef`로 최신값을 미러링하거나, `onFrameChange`에 함수형 업데이트(`setFrameIndex((prev) => prev + 1)`)를 사용해 클로저 캡처 자체를 피한다.
- 커스텀 훅(`useEvent`/`useLatest` 패턴)으로 이 미러링을 공통화해 반복 실수를 줄인다.

**탐지 방법**:
- Static: ESLint `react-hooks/exhaustive-deps` 룰 활성화 및 위반 지점(특히 `eslint-disable-next-line`으로 억제된 곳) 전수 조사.
- Runtime: 재생 중 요청되는 frameIndex 시퀀스를 로깅해 실제 진행 상태와 어긋나는지 확인.

**예외**:
- 클로저가 캡처하는 값이 컴포넌트 라이프사이클 동안 절대 안 바뀌는 상수(예: `frameCount` 총합)라면 문제 없음.

**Bitvue 판정**: N/A — `YuvViewerPanel/index.tsx:348-380`의 재생 타이머는 `setTimeout`을 매번 재등록하는 `useEffect`이며 의존성 배열에 `currentFrameIndex`가 포함돼 있어(“1회만 등록” 시도가 아니라 프레임이 바뀔 때마다 재스케줄) stale closure가 구조적으로 발생하지 않는다. 키보드 단축키 핸들러(`:429-492`)도 `togglePlay`/`goToPrevFrame` 등 관련 콜백을 의존성 배열에 모두 나열하고 cleanup에서 `removeEventListener`한다.

---

### FE-12: 프레임 이동 시 이벤트 리스너/RAF 루프 정리 누락으로 인한 누수
**분류**: 리소스 정리(cleanup) · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```typescript
function MotionVectorOverlay({ frame }: { frame: FrameData }) {
  useEffect(() => {
    function tick() {
      drawMvArrows(ctx, frame.motionVectors); // frame이 바뀌어도 이전 tick 루프가 계속 돎
      requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
    // return 정리 함수 없음 — 다음 프레임으로 넘어가도 이전 루프가 살아있음
  }, [frame]);

  return null;
}
```

**문제**:
- `frame`이 바뀔 때마다 새 `useEffect`가 실행되어 새 RAF 루프가 시작되지만, 이전 루프를 취소하지 않아 여러 개의 `tick` 루프가 동시에 누적 — 프레임을 빠르게 넘길수록 CPU 사용량이 선형으로 증가.
- 각 루프가 클로저로 서로 다른(오래된) `frame`을 캡처하고 있어, 화면에는 여러 프레임의 MV 화살표가 겹쳐 그려지는 시각적 오염까지 발생.
- `addEventListener`(휠 줌, 키보드 네비게이션 등)도 동일 패턴으로 정리 누락 시 리스너가 프레임 전환마다 중복 등록되어 이벤트 핸들러가 N배로 실행되는 문제로 이어진다.

**발생 조건**:
- 프레임을 빠르게 연속 전환(스크럽, 자동재생)할 때 특히 빠르게 누적되어 체감됨 — 몇 초 안에 수십 개의 좀비 루프/리스너가 쌓일 수 있음.
- 컴포넌트가 언마운트되지 않고 props(`frame`)만 바뀌는 구조(단일 페이지 뷰어)에서 "언마운트 시 정리하면 되겠지"라는 가정이 깨지는 경우.

**권장**:
```typescript
function MotionVectorOverlay({ frame }: { frame: FrameData }) {
  useEffect(() => {
    let rafId: number;
    function tick() {
      drawMvArrows(ctx, frame.motionVectors);
      rafId = requestAnimationFrame(tick);
    }
    rafId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafId); // frame이 바뀌거나 언마운트될 때 반드시 취소
  }, [frame]);

  return null;
}
```
- `useEffect`가 등록하는 모든 구독(RAF, 타이머, DOM 리스너, Tauri `listen()` 이벤트 언리슨 함수)은 예외 없이 cleanup 함수에서 해제한다 — "이번 effect가 정리한 것만 이번 effect가 책임진다" 원칙.
- Tauri의 `listen()`이 반환하는 unlisten 함수도 동일하게 `useEffect` cleanup에서 호출해야 하며, 특히 프레임/모드가 바뀔 때마다 새로 `listen`을 거는 코드에서 이 누락이 잦다.
- 정적 분석으로 "RAF/setInterval/addEventListener/listen() 호출은 있는데 대응하는 cancel/clear/removeEventListener/unlisten이 없는 effect"를 룰로 잡을 수 있다.

**탐지 방법**:
- Structural: `useEffect` 본문에서 `requestAnimationFrame`/`addEventListener`/`listen(` 호출은 있으나 반환하는 cleanup 함수가 없거나 대응 해제 호출이 없는 패턴 grep.
- Runtime: 프레임을 수십 회 빠르게 전환한 뒤 CPU 사용률/활성 RAF 콜백 수(`chrome://tracing` 또는 커스텀 카운터)를 측정.

**예외**:
- 컴포넌트/effect가 앱 생명주기 전체에서 단 한 번만 마운트되고 절대 재실행되지 않는 것이 구조적으로 보장된 최상위 루프(예: 앱 전역 렌더 루프)라면 매 프레임 정리가 불필요할 수 있음 — 다만 이 경우도 언마운트 정리는 반드시 필요.

**Bitvue 판정**: N/A — 코드베이스에서 `requestAnimationFrame`을 쓰는 곳은 `components/VirtualizedFilmstrip.tsx`뿐이며 `cancelAnimationFrame`으로 일관되게 정리한다(`:57,71`). Tauri `listen()` 구독도 `App.tsx:449-470`, `hooks/useFileOperations.ts:247-263`에서 반환된 unlisten 함수를 `useEffect` cleanup에서 호출한다.

---

### FE-13: 선택 변경만으로도 오버레이 전체를 처음부터 다시 계산 (증분 갱신 부재)
**분류**: 부분 무효화(invalidation) · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```typescript
function BlockOverlay({ blocks, selectedBlockId }: { blocks: Block[]; selectedBlockId: string | null }) {
  // selectedBlockId 하나만 바뀌어도 blocks 전체를 순회해 픽셀 버퍼를 처음부터 재생성
  const pixels = useMemo(() => {
    const buf = new Uint8ClampedArray(width * height * 4);
    for (const b of blocks) {
      const color = b.id === selectedBlockId ? SELECTED_COLOR : qpToColor(b.qp);
      paintBlock(buf, b, color);
    }
    return buf;
  }, [blocks, selectedBlockId]);

  return <OverlayCanvas pixels={pixels} />;
}
```

**문제**:
- `useMemo`를 쓰긴 했지만 의존성에 `selectedBlockId`가 포함되어 있어, 선택이 바뀔 때마다(빈번함) 전체 블록 재계산이 다시 트리거된다 — 캐싱이 "틀린 것을 캐싱"하고 있는 상태.
- QP 히트맵 색상 계산 자체는 선택과 무관한데도 선택이 바뀔 때마다 다시 계산되어 낭비.
- 블록 수가 많을수록(고해상도, 세밀한 그리드) 선택 반응성이 나빠져 "클릭했는데 하이라이트가 한 박자 늦게 나타나는" 체감으로 이어짐.

**발생 조건**:
- 블록을 클릭/화살표 키로 옮겨가며 연속 선택할 때, 혹은 다중 선택 드래그 중 매 픽셀 이동마다 갱신이 걸릴 때 두드러짐.

**권장**:
```typescript
function BlockOverlay({ blocks, selectedBlockId }: Props) {
  // 1) 선택과 무관한 베이스 히트맵은 blocks가 바뀔 때만 계산
  const basePixels = useMemo(() => renderHeatmap(blocks), [blocks]);

  // 2) 선택 하이라이트는 별도의 작은 레이어(FE-01의 selectionLayer)에 그려 베이스와 합성
  const selectionLayer = useCanvasLayer();
  useEffect(() => {
    selectionLayer.clear();
    const block = blocks.find((b) => b.id === selectedBlockId);
    if (block) drawSelectionOutline(selectionLayer.ctx, block);
  }, [selectedBlockId]); // blocks 전체 재계산과 분리

  return (
    <div className="overlay-stack">
      <OverlayCanvas pixels={basePixels} />
      <canvas ref={selectionLayer.ref} />
    </div>
  );
}
```
- "값이 바뀌는 빈도"가 다른 데이터는 애초에 같은 캐시/버퍼에 묶지 않는다 — FE-01의 레이어 분리 원칙을 오버레이 내부의 서브레이어 단위에도 동일하게 적용.
- 정말 단일 버퍼에 합성해야 한다면(예: PNG로 내보내기 위해) 최종 합성 단계에서만 결합하고, 화면 렌더링 경로는 분리된 채로 유지한다.

**탐지 방법**:
- Structural: `useMemo`/`useCallback` 의존성 배열에 "갱신 빈도가 크게 다른 값들"(예: 원본 데이터 vs 선택/호버 UI 상태)이 함께 들어있는지 검사.
- Runtime: 선택 변경 시 프로파일러에서 무거운 계산 함수(`renderHeatmap` 등)가 다시 호출되는지 확인.

**예외**:
- 블록 수가 적어(수십 개 이하) 전체 재계산 비용이 무시할 만한 경우.

**Bitvue 판정**: N/A — 현재 `VideoCanvas`/`OverlayRenderer` 경로에 블록 단위 `selectedBlockId`가 오버레이 재계산 의존성으로 연결된 코드가 없음(블록 클릭 선택 기능 자체가 아직 프레임 뷰어에 없음). 필름스트립 쪽 선택(`selectedFrameIndex`)은 별도 리스트 아이템 단위라 이 안티패턴의 "선택 하나 바뀔 때 블록 수천 개 재계산"과는 성격이 다름.

---

### FE-14: QP 히트맵/MV 필드 등 무거운 오버레이 계산을 메인 스레드에서 수행
**분류**: 워커 오프로딩 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```typescript
function MvFieldOverlay({ frame }: { frame: FrameData }) {
  const arrows = useMemo(() => {
    // 블록별 MV를 인접 블록과 보간해 부드러운 화살표 필드를 만드는 비용 큰 연산
    return computeSmoothedMvField(frame.blocks); // 수천 블록 x 보간 -> 수십 ms
  }, [frame.id]);

  return <ArrowOverlay arrows={arrows} />; // 메인 스레드에서 계산하는 동안 UI 전체가 멈춤
}
```

**문제**:
- `useMemo`로 감쌌어도 계산 자체는 여전히 메인 스레드의 렌더 커밋 경로에서 동기 실행되어, 계산 중 사용자 입력(클릭, 스크롤, 키보드)이 전혀 처리되지 않는 Long Task를 만든다.
- 프레임 전환 시마다 이 계산이 반복되면 빠른 스크럽 중 UI가 사실상 얼어붙은 것처럼 보인다.
- React 18의 `useTransition`/`startTransition`만으로는 "무거운 동기 계산 자체"를 비동기로 만들어주지 않는다 — 렌더 우선순위만 낮출 뿐 메인 스레드 점유 시간은 그대로.

**발생 조건**:
- MV 필드 보간, 대규모 블록의 색상 그라디언트 합성처럼 O(블록 수 × 이웃 수) 이상인 연산에서, 특히 고해상도/조밀 그리드 코덱(작은 블록 크기)일 때.

**권장**:
```typescript
// worker.ts
self.onmessage = (e: MessageEvent<{ blocks: Block[] }>) => {
  const arrows = computeSmoothedMvField(e.data.blocks);
  self.postMessage(arrows, [arrows.buffer]); // transferable로 복사 비용 최소화
};

// 컴포넌트
function MvFieldOverlay({ frame }: { frame: FrameData }) {
  const [arrows, setArrows] = useState<ArrowField | null>(null);
  const workerRef = useMvWorker(); // 마운트 시 1회 생성, 언마운트 시 terminate

  useEffect(() => {
    let cancelled = false;
    workerRef.current!.compute(frame.blocks).then((result) => {
      if (!cancelled) setArrows(result);
    });
    return () => { cancelled = true; };
  }, [frame.id]);

  return arrows ? <ArrowOverlay arrows={arrows} /> : <ArrowOverlaySkeleton />;
}
```
- 순수 계산(입력 → 출력, DOM 접근 없음)은 Web Worker로 이전하고, `postMessage`에 transferable object(`ArrayBuffer`)를 써서 구조적 복제 비용도 줄인다.
- 계산 중에는 이전 프레임의 오버레이를 유지하거나 스켈레톤을 보여줘 "멈춘 것처럼" 보이지 않게 한다.
- 워커 풀을 두고 오래된 요청(프레임이 이미 바뀐 요청)은 취소/무시하도록 세대 토큰을 붙인다(FE-17과 연동).

**탐지 방법**:
- Runtime: Performance 패널에서 오버레이 계산 함수가 Long Task(50ms+)로 잡히는지, 그 동안 Input Delay가 발생하는지 확인.
- Structural: `useMemo`/렌더 경로 안에 O(n log n) 이상 또는 중첩 루프 연산이 워커 위임 없이 존재하는지 검사.

**예외**:
- 계산이 수 ms 이내로 끝나는 경량 연산이거나, 결과가 이미 백엔드(Rust)에서 계산되어 프런트는 단순 매핑만 하는 경우 워커 도입은 과설계.

**Bitvue 판정**: Suspected — `components/panels/OverlayRenderer/renderers/*.tsx`의 오버레이 렌더러들은 전부 메인 스레드의 `useEffect` 내부에서 O(grid_w×grid_h) 이중 루프로 `fillRect`를 직접 호출하며(예: `QPMapRenderer.tsx:29-40`) Web Worker offload는 어디에도 없다(`workers/frameStatsWorker.ts`가 유일한 워커지만 통계용이지 오버레이 렌더링용이 아님). 다만 실제 연산이 원문 예시(인접 블록 보간을 포함하는 MV 스무딩)만큼 무겁지 않고 단순 색상 매핑+`fillRect`라, MV 필드만 밀도 300블록 초과 시 WebGL 경로(`mv-webgl.ts`)로 전환하는 것 외에는 다른 오버레이가 실제로 Long Task를 유발하는지는 프로파일링 없이 확인하지 못함.

---

### FE-15: 동일 IPC 이벤트 페이로드가 여러 상태 저장소에 중복 보관되어 소스가 어긋남
**분류**: 단일 진실 공급원(SSOT) · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```typescript
// store A: 프레임 메타데이터 캐시
const useFrameStore = create<{ frames: Record<number, FrameMeta> }>(() => ({ frames: {} }));

// store B: 타임라인 UI가 별도로 자기만의 프레임 목록을 들고 있음
const useTimelineStore = create<{ frameList: FrameMeta[] }>(() => ({ frameList: [] }));

useEffect(() => {
  const unlisten = listen<FrameMeta>('frame-decoded', (event) => {
    useFrameStore.setState((s) => ({ frames: { ...s.frames, [event.payload.index]: event.payload } }));
    useTimelineStore.setState((s) => ({ frameList: [...s.frameList, event.payload] })); // 별도 사본 유지
  });
  return () => { unlisten.then((f) => f()); };
}, []);
```

**문제**:
- 같은 백엔드 이벤트를 두 store가 각자 반영하다 보면, 한쪽만 업데이트에 실패(예외, 순서 역전, 부분 실패)했을 때 두 store가 서로 다른 "현재 프레임 상태"를 보여주는 소스 불일치가 발생.
- 특정 프레임을 재디코드(캐시 무효화)했을 때 한쪽 store만 갱신하고 다른 쪽은 갱신 로직을 빠뜨리는 실수가 나기 쉽다 — 두 곳을 항상 같이 고쳐야 하는 부담이 유지보수 비용으로 누적.
- 디버깅 시 "화면 A는 최신인데 화면 B는 이전 프레임을 보여준다"는 재현하기 까다로운 버그로 나타난다.

**발생 조건**:
- 여러 패널(타임라인, 필름스트립, 현재 프레임 정보 패널)이 같은 백엔드 이벤트를 각자 구독해 자기 상태를 관리하도록 독립적으로 개발되었을 때.

**권장**:
```typescript
// 단일 저장소가 원본 데이터를 소유하고, 다른 뷰는 selector로 파생만 한다.
const useFrameStore = create<{ frames: Map<number, FrameMeta> }>(() => ({ frames: new Map() }));

useEffect(() => {
  const unlisten = listen<FrameMeta>('frame-decoded', (event) => {
    useFrameStore.setState((s) => {
      const next = new Map(s.frames);
      next.set(event.payload.index, event.payload);
      return { frames: next };
    });
  });
  return () => { unlisten.then((f) => f()); };
}, []);

// 타임라인은 원본을 복제하지 않고 selector로 파생 뷰만 얻는다.
function useTimelineFrameList() {
  return useFrameStore((s) => Array.from(s.frames.values()).sort((a, b) => a.index - b.index));
}
```
- IPC 이벤트를 반영하는 지점을 애플리케이션 전체에서 단일 진입점(하나의 store, 하나의 리스너)으로 고정하고, 다른 화면은 그 store에서 파생된 selector/computed 값만 사용한다.
- 파생 값이 비싸면 selector 결과를 메모이제이션(`reselect` 패턴)하되, 원본 데이터 자체는 절대 복제하지 않는다.

**탐지 방법**:
- Structural: 동일한 Tauri 이벤트 이름(`listen('frame-decoded', ...)`)을 구독하는 지점이 코드베이스에 2곳 이상 있는지 grep.
- Manual: 여러 패널을 동시에 띄운 상태에서 강제로 느린 네트워크/디코드를 시뮬레이션해 패널 간 상태가 어긋나는지 수동 확인.

**예외**:
- 서로 다른 store가 같은 이벤트에서 "의도적으로 다른 파생 데이터"(예: 하나는 원본, 하나는 집계 통계)를 만드는 것은 문제가 아님 — 원본 자체가 중복 보관되는 경우만 해당.

**Bitvue 판정**: Suspected — `frontend/contexts/ThumbnailContext.tsx`가 `frontend/components/useFilmstripState.ts`와 별개로 독립된 썸네일 캐시/로딩 로직을 구현하고 있어 원본 데이터가 두 곳에 중복 보관될 수 있는 구조이지만, `ThumbnailContext`는 `contexts/index.ts`에서 export만 되고 `App.tsx`나 실제 컴포넌트 트리 어디에서도 `ThumbnailProvider`/`useThumbnails`를 소비하는 지점을 찾지 못했다(테스트에서만 사용). 즉 현재는 죽은 코드라 실사용 중 상태 불일치가 관측되지는 않지만, 두 저장소가 병존하는 구조 자체는 향후 재도입 시 SSOT 위반으로 이어질 위험이 있음.

---

### FE-16: 재정렬 가능한 프레임 썸네일 목록에서 인덱스 기반 key 사용
**분류**: 리스트 재조정(reconciliation) 정합성 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```typescript
function FilmstripThumbnails({ thumbnails }: { thumbnails: Thumbnail[] }) {
  return (
    <div className="filmstrip">
      {thumbnails.map((thumb, index) => (
        <ThumbnailCanvas key={index} thumbnail={thumb} /> // 필터링/정렬/삽입 시 index가 재사용됨
      ))}
    </div>
  );
}
```

**문제**:
- 마커 필터링(예: "키프레임만 보기" 토글), 구간 삭제, 정렬 기준 변경 등으로 목록 중간 항목이 추가/제거되면, `index` key는 실제 썸네일이 아니라 "그 자리"에 묶여 React가 엉뚱한 DOM/캔버스 노드를 재사용한다.
- 캔버스 기반 썸네일은 내부에 이미 그려진 픽셀 버퍼를 갖고 있어, key가 잘못 매칭되면 "다른 프레임의 캔버스에 새 프레임 데이터를 그리지 않고 이전 픽셀을 그대로 재사용"하는 시각적 오염(잘못된 프레임 썸네일 표시)으로 이어질 수 있다.
- 애니메이션 전환(정렬 변경 시 부드러운 재배치)도 index key로는 올바르게 동작하지 않는다 — React가 항목이 이동한 것을 인식하지 못하고 매번 다른 항목으로 취급.

**발생 조건**:
- 필름스트립에 필터/정렬/북마크 삭제 등 목록 순서·구성이 바뀌는 인터랙션이 추가되는 순간 잠재되어 있던 버그가 표면화.

**권장**:
```typescript
function FilmstripThumbnails({ thumbnails }: { thumbnails: Thumbnail[] }) {
  return (
    <div className="filmstrip">
      {thumbnails.map((thumb) => (
        <ThumbnailCanvas key={thumb.frameIndex} thumbnail={thumb} /> // 안정적 고유 식별자
      ))}
    </div>
  );
}
```
- key는 배열 내 위치가 아니라 데이터 자체의 안정적 식별자(`frameIndex`, 백엔드가 부여한 `id`)를 사용한다.
- 리스트가 매우 크고 항목 자체가 순수 값 기반(재정렬 시 재사용해도 무방)이라면 index key가 오히려 성능상 유리할 수 있으나, 이 경우(캔버스 내부 상태 보유 + 재정렬 가능)에는 해당하지 않는다.

**탐지 방법**:
- Static: `.map((x, i) => <... key={i} ...>)` 패턴 grep, 특히 캔버스/이미지처럼 내부 렌더 상태를 갖는 컴포넌트에 대해 우선순위 높게 검사.
- Manual: 필터/정렬 토글 전후로 썸네일 내용이 프레임 번호 라벨과 일치하는지 시각 검증.

**예외**:
- 목록이 절대 재정렬/필터링되지 않고 항상 끝에만 추가되는 append-only 리스트라면 index key도 실질적으로 안전.

**Bitvue 판정**: N/A — `components/Filmstrip/views/ThumbnailsView.tsx:187`와 `VirtualizedThumbnailsView.tsx:95` 모두 `key={frame.frame_index}`를 사용해 배열 위치가 아닌 안정적 식별자로 key를 부여한다. 코드베이스 전체에서 캔버스/썸네일류 반복 렌더에 `key={i}`/`key={idx}`/`key={index}`를 쓰는 지점은 발견하지 못함.

---

### FE-17: 비동기 IPC 응답에 프레임/버전 가드가 없어 레이스 컨디션으로 이전 프레임이 최신을 덮어씀
**분류**: 비동기 응답 정합성 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```typescript
function FrameViewer() {
  const [frameIndex, setFrameIndex] = useState(0);
  const [frameData, setFrameData] = useState<FrameData | null>(null);

  useEffect(() => {
    invoke<FrameData>('decode_frame', { frameIndex }).then(setFrameData);
    // 이전 요청이 늦게 끝나면 그 결과가 나중에 도착해 최신 frameIndex의 화면을 덮어씀
  }, [frameIndex]);

  return frameData ? <FrameCanvas frame={frameData} /> : <Spinner />;
}
```

**문제**:
- 프레임 5를 요청한 직후 사용자가 곧바로 프레임 9로 넘어갔을 때, 프레임 5의 디코드가 (더 무거운 인트라 프레임이라서, 혹은 캐시 미스라서) 더 늦게 끝나면 프레임 9 화면 위에 프레임 5 데이터가 덧씌워진다.
- 이 버그는 네트워크/디코드 지연이 불규칙한 조건에서만 재현되어(로컬 개발 환경에서는 거의 항상 순서대로 응답이 옴) 테스트 환경에서 잘 안 잡히고 실사용 환경(대용량 파일, 저사양 머신)에서만 보고되는 경우가 많다.
- FE-07(throttle 없는 IPC 폭주)과 결합되면 동시에 여러 개의 in-flight 요청이 쌓여 레이스 발생 확률이 크게 올라간다.

**발생 조건**:
- 프레임 크기/타입에 따라 디코드 시간이 들쭉날쭉할 때(IDR vs P/B 프레임), 스크럽처럼 짧은 시간에 여러 프레임을 연속 요청할 때.

**권장**:
```typescript
function FrameViewer() {
  const [frameIndex, setFrameIndex] = useState(0);
  const [frameData, setFrameData] = useState<FrameData | null>(null);
  const latestRequestId = useRef(0);

  useEffect(() => {
    const requestId = ++latestRequestId.current; // 이 요청의 세대 번호
    invoke<FrameData>('decode_frame', { frameIndex }).then((data) => {
      if (requestId !== latestRequestId.current) return; // 더 최신 요청이 이미 나갔으면 무시
      setFrameData(data);
    });
  }, [frameIndex]);

  return frameData ? <FrameCanvas frame={frameData} /> : <Spinner />;
}
```
- 요청마다 단조 증가 토큰(또는 `AbortController`)을 부여하고, 응답 처리 시점에 "지금도 여전히 최신 요청인가"를 검사한 뒤에만 상태에 반영한다.
- 가능하면 백엔드도 `AbortController`/취소 신호를 지원해 이미 필요 없어진 디코드 작업 자체를 중단시켜 리소스 낭비까지 줄인다.
- React Query/SWR류 라이브러리를 쓰면 이 가드가 기본 제공되므로, 자체 구현 대신 검증된 라이브러리 사용도 고려.

**탐지 방법**:
- Runtime: 인위적으로 응답 지연에 지터를 주입(예: 목 IPC 레이어에서 랜덤 딜레이)한 뒤 빠른 프레임 전환을 반복해 화면과 실제 frameIndex가 어긋나는지 확인.
- Structural: `invoke(...).then(setState)` 패턴에서 요청 세대/취소 가드 없이 바로 상태를 설정하는 지점 grep.

**예외**:
- IPC 커맨드가 항상 프레임 인덱스를 함께 반환하고, 컴포넌트가 응답의 인덱스와 현재 `frameIndex`를 비교해 다르면 버리는 방식(암묵적 가드)을 이미 채택했다면 별도 토큰 없이도 안전.

**Bitvue 판정**: Confirmed — `frontend/components/panels/SyntaxDetailPanel/FrameSyntaxTab.tsx:67-87`의 `get_frame_syntax` 요청은 `invoke(...).then(setSyntaxTree).catch(...).finally(...)`만 있고 `cancelled` 플래그도, `useEffect` cleanup 함수도 없다. 같은 디렉터리의 `ApsTab.tsx`, `RefListTab.tsx`, `QmTab.tsx`, `ProbsTab.tsx`는 전부 `let cancelled = false` + `return () => { cancelled = true }` 가드를 일관되게 사용하는 것과 대조적 — `FrameSyntaxTab`만 이 가드가 빠져 있어, 빠른 프레임 전환 시 느리게 끝난 이전 프레임의 신택스 트리가 최신 프레임 화면을 덮어쓸 수 있는 실질적 레이스 컨디션. `YuvViewerPanel/index.tsx:174-263`(프레임/YUV 로드)와 `HexViewTab.tsx:48-93`은 정상적으로 `cancelled` 가드를 사용 중.

---

### FE-18: devicePixelRatio/HiDPI 처리 누락으로 오버레이가 흐리거나 프레임과 어긋남
**분류**: 디스플레이 스케일링 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```typescript
function OverlayCanvas({ width, height }: { width: number; height: number }) {
  return (
    <canvas
      width={width}   // CSS 픽셀 크기를 그대로 캔버스 버퍼 크기로 사용
      height={height}
      style={{ width, height }}
    />
  );
}
```

**문제**:
- HiDPI(Retina, 200%+ 스케일링) 디스플레이에서는 CSS 픽셀 1개가 실제 물리 픽셀 여러 개에 대응하는데, 캔버스 버퍼를 CSS 픽셀 크기로만 만들면 브라우저가 이를 확대해서 그리며 오버레이 선/텍스트가 흐릿해진다.
- 더 나쁜 경우, 프레임을 그리는 캔버스는 `devicePixelRatio`를 반영해 만들고 오버레이 캔버스는 반영하지 않는(혹은 그 반대) 불일치가 생기면 두 레이어가 서브픽셀 단위로 어긋나 보이는 정합성 버그가 발생 — FE-01의 레이어 분리 이점을 스케일링 버그가 상쇄시킨다.
- 마우스 이벤트 좌표(`clientX/clientY`, CSS 픽셀 기준)를 캔버스 버퍼 좌표(물리 픽셀 기준)로 변환할 때 `devicePixelRatio`를 빠뜨리면 호버/클릭 판정이 실제 블록 위치와 어긋나는 문제까지 겹친다(FE-10과 유사한 증상으로 나타남).

**발생 조건**:
- Retina 디스플레이가 흔한 macOS 환경, 또는 Windows에서 125%/150% 디스플레이 배율을 쓰는 사용자에게서 보고되지만 100% 배율 개발 환경에서는 재현되지 않아 놓치기 쉽다.

**권장**:
```typescript
function useHiDpiCanvas(cssWidth: number, cssHeight: number) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current!;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(cssWidth * dpr);
    canvas.height = Math.round(cssHeight * dpr);
    canvas.style.width = `${cssWidth}px`;
    canvas.style.height = `${cssHeight}px`;
    const ctx = canvas.getContext('2d')!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0); // 이후 모든 draw 호출은 CSS 픽셀 단위로 작성 가능
  }, [cssWidth, cssHeight]);

  return ref;
}

function toCanvasCoord(e: React.MouseEvent, canvas: HTMLCanvasElement) {
  const rect = canvas.getBoundingClientRect();
  // CSS 픽셀 기준으로 통일(setTransform을 이미 적용했으므로 dpr을 다시 곱하지 않음)
  return { x: e.clientX - rect.left, y: e.clientY - rect.top };
}
```
- 모든 캔버스 레이어(프레임/오버레이/선택)가 동일한 `devicePixelRatio` 처리 유틸을 공유해 스케일링 기준을 통일한다.
- `ctx.setTransform(dpr, 0, 0, dpr, 0, 0)`을 적용해두면 이후 드로잉 코드는 CSS 픽셀 단위로 작성할 수 있어 FE-10의 좌표 변환 로직과도 자연스럽게 맞물린다.
- `window.matchMedia('(resolution: ...)')` 변경 이벤트를 구독해 창을 다른 배율의 모니터로 옮겼을 때도 재계산한다.

**탐지 방법**:
- Static: `canvas.width = cssWidth` 형태로 `devicePixelRatio` 곱셈 없이 캔버스 버퍼 크기를 설정하는 지점 grep.
- Runtime: HiDPI 디스플레이(또는 브라우저 확대 150%+)에서 오버레이 선의 흐림/레이어 간 어긋남을 시각 확인.

**예외**:
- 서버사이드 렌더링/헤드리스 캡처(스크린샷 내보내기용 오프스크린 캔버스)처럼 물리 디스플레이와 무관하게 고정 해상도로만 출력하는 경로는 dpr 처리가 불필요.

**Bitvue 판정**: Suspected — `VideoCanvas.tsx`는 캔버스 버퍼 크기를 `devicePixelRatio` 없이 프레임 원본 해상도(`yuvData.width/height`)로 고정하고 오버레이 텍스트/선도 같은 버퍼에 그린다 — `ctx.setTransform(dpr,...)` 류 처리가 코드에 없음. 반면 같은 코드베이스의 `HRDBufferPanel.tsx:99-106`은 `window.devicePixelRatio`를 명시적으로 곱해 캔버스를 그린다 — 처리 방식이 일관되지 않음. 다만 `VideoCanvas`는 CSS 표시 크기가 아니라 "비디오 원본 픽셀 1:1 버퍼 + CSS transform으로 줌"을 의도한 설계로 보여, 이것이 실수인지 의도적 트레이드오프인지는 코드만으로 단정하기 어려움.
