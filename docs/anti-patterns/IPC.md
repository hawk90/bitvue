# Anti-Pattern Catalog — IPC: Tauri IPC와 직렬화

이 문서는 Bitvue 안티패턴 카탈로그의 한 갈래이며, 전체 카탈로그의 색인은 별도로 작성 중인 `docs/anti-patterns/INDEX.md`를 참고한다.

---

### IPC-001: 대형 FrameAnalysis를 JSON 반환
**분류**: 대용량 페이로드 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct FrameAnalysis {
    frame_index: u32,
    syntax_tree: Vec<SyntaxNode>,      // NAL/CTU/CU 트리 전체
    hex_dump: Vec<u8>,                 // 프레임 원본 바이트
    qp_map: Vec<Vec<u8>>,              // block 단위 QP
    mv_field: Vec<MotionVector>,       // block 단위 MV
    reference_lists: Vec<RefPic>,
    residuals: Vec<TransformBlock>,
}

#[tauri::command]
fn analyze_frame(state: tauri::State<AppState>, frame_index: u32) -> FrameAnalysis {
    let decoder = state.decoder.lock().unwrap();
    decoder.full_analysis(frame_index) // 모든 필드를 한 번에 채워서 반환
}
```

**문제**:
- 프런트가 QP 오버레이만 켜고 싶어도 syntax tree, hex dump, MV field까지 강제로 직렬화·전송된다.
- 4K 프레임 하나의 `FrameAnalysis`가 수 MB~수십 MB에 달해 JSON 직렬화 자체가 메인 스레드를 수백 ms 점유할 수 있다.
- IPC 응답 전체가 단일 원자적 단위이므로 부분 갱신이 불가능하고, 실패 시 전체를 재요청해야 한다.
- 반환 타입이 codec마다(AVC/HEVC/VP9/AV1) 사실상 다른 의미를 가지는데 하나의 구조체로 뭉뚱그려져 있어 Optional 필드가 폭증한다(→ IPC-012).

**발생 조건**:
- 4K/8K 프레임에서 macroblock/CU가 수천~수만 개인 경우.
- 사용자가 프레임 탐색(다음/이전 프레임, 빠른 스크럽)을 반복할 때마다 매번 전체 분석 결과를 새로 받는 경우.
- 여러 패널(hex view, QP overlay, MV overlay)이 동시에 열려 있는데 커맨드는 하나로 통합되어 있는 경우.

**권장**:
```rust
#[tauri::command]
fn get_frame_summary(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary {
    // frame_type, poc, qp_avg, block_count 등 가벼운 메타데이터만
}

#[tauri::command]
fn get_syntax_tree(state: tauri::State<AppState>, frame_index: u32, depth: u8) -> SyntaxTreeSlice { /* ... */ }

#[tauri::command]
fn get_qp_map(state: tauri::State<AppState>, frame_index: u32, viewport: Rect) -> Vec<u8> { /* ... */ }
```
- 분석 결과를 목적별(요약/트리/QP/MV/hex) 커맨드로 분리하고, 프런트는 열린 패널이 필요로 하는 것만 요청한다.
- 대용량 수치 배열은 IPC-002/003 권장안(바이너리 버퍼 + viewport 쿼리)을 따른다.
- 필요 시 커맨드 사이에 캐시 키(frame_index + params 해시)를 공유해 중복 계산을 막는다.

**탐지 방법**:
- Structural: `#[tauri::command]` 반환 타입 구조체의 필드 수/중첩 깊이를 정적으로 스캔해 임계값(예: 필드 8개 이상 또는 `Vec<T>` 3개 이상) 초과 시 경고.
- Runtime: IPC 응답 payload 크기를 프레임 탐색 이벤트당 로깅해 평균/최대치 추적.

**예외**:
- 프레임 수가 적고(예: 정지 이미지 분석 도구) 패널이 항상 전부 열려 있는 워크플로라면 통합 반환이 요청 왕복 수를 줄여 오히려 유리할 수 있다.

**Bitvue 판정**: Confirmed — `get_frame_analysis`(src-tauri/src/commands/analysis/mod.rs:256-288)은 매 호출마다 `FrameAnalysisData`의 qp/mv/partition/prediction_mode/transform/mb_type/ref_idx grid를 전부 채워 반환한다(extractors.rs의 `extract_*_analysis` 함수들이 모든 grid를 무조건 계산). `tauri::ipc::Response` 바이너리 경로는 코드베이스 전체에서 사용된 적이 없다(grep 0건).

---

### IPC-002: 수만 개 syntax node를 한 번에 전달
**분류**: 대용량 페이로드 / 세분화 부재 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct BlockInfo {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    qp: u32,
    mode: String,          // "INTRA_4x4" 같은 문자열 (IPC-010 참조)
    mv: Option<(i32, i32)>,
}

#[tauri::command]
fn get_block_grid(state: tauri::State<AppState>, frame_index: u32) -> Vec<BlockInfo> {
    // 4K 프레임 4x4 블록 기준 약 8,160 x 3,840 / 16 ≈ 129,600개 블록
    state.frames[frame_index as usize].blocks.clone()
}
```

**문제**:
- 블록 하나당 JSON으로 `{"x":..,"y":..,"width":..,"height":..,"qp":..,"mode":"INTRA_4x4","mv":[1,-2]}` 형태의 키-값 텍스트가 반복되어, 실제 정보량 대비 10배 이상의 바이트를 전송한다.
- serde_json 직렬화/역직렬화 자체가 O(n) 텍스트 파싱이므로 노드 수만 개 규모에서 프레임 전환마다 GC 압박과 프레임 드랍을 유발한다.
- 프런트는 대부분 화면에 보이는 viewport 영역의 블록만 렌더링하는데도 전체 블록을 받는다(→ IPC-008과 결합해 악화).

**발생 조건**:
- 4K 이상 해상도에서 CU/PU/TU 단위가 8x8~64x64로 세분화된 HEVC/AV1 스트림.
- 사용자가 빠르게 프레임을 이동(스크럽, 재생)해 초당 여러 번 커맨드가 호출되는 경우.

**권장**:
```rust
// struct-of-arrays를 고정 크기 바이너리 레코드로 직렬화
#[repr(C)]
struct BlockRecord {
    x: u16,
    y: u16,
    width_log2: u8,
    height_log2: u8,
    qp: u8,
    mode: u8,       // enum discriminant
    mv_x: i16,
    mv_y: i16,
}
// size_of::<BlockRecord>() == 12 bytes, alignment 고정

#[tauri::command]
fn get_block_grid_binary(
    state: tauri::State<AppState>,
    frame_index: u32,
    viewport: Rect,           // 화면에 보이는 픽셀 범위만
) -> tauri::ipc::Response {
    let records: Vec<BlockRecord> = state
        .frames[frame_index as usize]
        .blocks_in(viewport)   // viewport 안의 블록만 필터
        .map(BlockRecord::from)
        .collect();
    let mut buf = Vec::with_capacity(records.len() * std::mem::size_of::<BlockRecord>());
    for r in &records {
        buf.extend_from_slice(bytemuck::bytes_of(r));
    }
    tauri::ipc::Response::new(buf)
}
```
- 필드를 `Vec<SyntaxNode>`(AoS, array-of-structs) 대신 고정폭 바이너리 레코드(struct-of-arrays 또는 packed struct)로 직렬화해 JSON 오버헤드를 제거한다.
- `viewport: Rect` 파라미터로 화면에 보이는 블록만 계산·전송한다(IPC-008).
- 프런트에서는 `ArrayBuffer` + `DataView`로 파싱해 zero-copy에 가깝게 소비한다.

**탐지 방법**:
- Structural: 커맨드 반환 타입이 `Vec<T>`이고 T가 여러 primitive 필드를 가진 struct일 때, 예상 요소 개수(프레임 해상도 기반 추정)가 임계값을 넘으면 경고.
- Runtime: 프레임 전환 시 IPC 메시지 크기와 직렬화 소요 시간을 계측해 그래프화.

**예외**:
- 노드 수가 수백 개 이하로 확정된(SD 해상도, 큰 블록 크기 강제) 상황이거나 프로토타입 단계에서는 JSON `Vec<BlockInfo>`로도 충분할 수 있다.

**Bitvue 판정**: Confirmed — `MVGridData.mv_l0/mv_l1: Vec<MotionVectorData>`, `PartitionGridData.blocks: Vec<PartitionBlockData>` 등(src-tauri/src/commands/mod.rs:167-272)이 AoS 그대로 표준 JSON(Result<T,String>)으로 직렬화되며, viewport 필터링이나 고정폭 바이너리 레코드 인코딩이 전혀 없다.

---

### IPC-003: binary 데이터를 number[]로 직렬화
**분류**: 직렬화 포맷 · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct HexViewChunk {
    offset: u64,
    bytes: Vec<u8>,   // serde_json 기본 동작: [0, 255, 16, 3, ...] 형태로 직렬화됨
}

#[tauri::command]
fn get_hex_chunk(state: tauri::State<AppState>, offset: u64, len: u32) -> HexViewChunk {
    HexViewChunk { offset, bytes: state.read_bytes(offset, len) }
}
```

**문제**:
- `Vec<u8>`가 serde_json 기본 규칙에 따라 `[0,255,16,3,...]` 형태의 숫자 배열 텍스트로 직렬화되어, 실제 1바이트당 평균 3~4 문자(콤마 포함)를 사용한다 — 원본 대비 3~4배 팽창.
- V8/WebKit의 JSON 파서가 각 숫자를 개별 토큰으로 파싱해야 하므로 대형 hex dump(수 MB)에서 파싱 비용이 이진 전송보다 수 배 크다.
- 프런트에서 다시 `Uint8Array`로 변환하는 추가 복사가 발생한다.

**발생 조건**:
- hex view 패널에서 사용자가 큰 범위를 스크롤하거나 "전체 NAL unit 덤프 보기"를 요청할 때.
- raw YUV 프레임 미리보기, 압축 해제 전 원본 바이트 등 바이너리 자체가 목적인 데이터.

**권장**:
```rust
#[tauri::command]
fn get_hex_chunk_binary(
    state: tauri::State<AppState>,
    offset: u64,
    len: u32,
) -> tauri::ipc::Response {
    let bytes = state.read_bytes(offset, len);
    tauri::ipc::Response::new(bytes) // raw byte 스트림으로 전송
}
```
- Tauri의 `ipc::Response` (또는 커스텀 프로토콜 `tauri://`, `asset://`)를 사용해 raw 바이너리를 그대로 전달한다.
- 메타데이터(offset 등)가 필요하면 별도의 작은 JSON 커맨드로 분리하거나, 바이너리 앞에 고정 길이 헤더를 붙여 단일 스트림으로 합친다.

**탐지 방법**:
- Static: `#[derive(Serialize)]`가 붙은 struct 필드 중 `Vec<u8>` / `[u8; N]` 타입을 grep으로 검출.
- Runtime: 실제 응답 payload에서 `[` 로 시작하는 숫자 배열 패턴과 원본 바이트 수의 비율을 측정.

**예외**:
- 바이트 수가 매우 작고(수십 바이트 이하) 빈도도 낮은 경우(예: 헤더의 4바이트 magic number)는 가독성을 위해 JSON 숫자 배열이나 hex 문자열로 두어도 무방하다.

**Bitvue 판정**: Confirmed — `FrameHexData.data: Vec<u8>`(src-tauri/src/commands/frame.rs:467)가 `tauri::ipc::Response`가 아닌 일반 serde_json 경로로 반환되며, 요청당 최대 `limits::MAX_HEX_BYTES` = 1MB(src-tauri/src/constants.rs:42)까지 숫자 배열 텍스트로 직렬화된다.

---

### IPC-004: base64를 기본 전송 방식으로 사용
**분류**: 직렬화 포맷 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
use base64::{engine::general_purpose, Engine as _};

#[derive(serde::Serialize)]
struct FrameThumbnail {
    frame_index: u32,
    png_base64: String,
}

#[tauri::command]
fn get_thumbnail(state: tauri::State<AppState>, frame_index: u32) -> FrameThumbnail {
    let png_bytes = state.render_thumbnail_png(frame_index);
    FrameThumbnail {
        frame_index,
        png_base64: general_purpose::STANDARD.encode(png_bytes), // 항상 base64
    }
}
```

**문제**:
- base64는 원본 대비 약 33% 크기 증가를 항상 강제한다 — 이미지/hex dump처럼 이미 바이너리인 데이터에 불필요한 세금을 매긴다.
- 인코딩(Rust)과 디코딩(JS `atob`)에 각각 CPU 사이클과 임시 버퍼 복사가 들어가, 특히 수백 KB급 썸네일을 다수 렌더링하는 필름스트립(filmstrip) 뷰에서 누적 비용이 커진다.
- "일단 문자열로 감싸면 JSON에 넣기 쉽다"는 이유로 습관적으로 base64를 쓰는 경우가 많아, 정말 텍스트 임베딩이 필요한 경우(예: `<img src="data:...">`)와 구분되지 않는다.

**발생 조건**:
- 필름스트립처럼 수십~수백 개의 프레임 썸네일을 한 번에 로드하는 경우.
- QP 히트맵, 오버레이 PNG 등 이미지성 바이너리를 커맨드 반환값에 문자열로 끼워 넣는 경우.

**권장**:
```rust
#[tauri::command]
fn get_thumbnail_binary(state: tauri::State<AppState>, frame_index: u32) -> tauri::ipc::Response {
    let png_bytes = state.render_thumbnail_png(frame_index);
    tauri::ipc::Response::new(png_bytes)
}
```
```ts
// 프런트: fetch 기반 커스텀 프로토콜이나 invoke의 raw response를 Blob/ArrayBuffer로 직접 소비
const bytes = await invoke<ArrayBuffer>('get_thumbnail_binary', { frameIndex });
const blob = new Blob([bytes], { type: 'image/png' });
img.src = URL.createObjectURL(blob);
```
- 바이너리 그대로 전송 가능한 경로(`ipc::Response`, 커스텀 protocol handler, `convertFileSrc`)를 우선 사용한다.
- base64는 정말로 텍스트 컨텍스트(예: JSON 안에 반드시 문자열로 끼워야 하는 레거시 API, `data:` URI를 즉시 캐시해야 하는 경우)에 한정한다.

**탐지 방법**:
- Static: `base64::encode` / `general_purpose::STANDARD.encode` 호출부를 grep해 반환 타입이 이미지·hex·raw 바이트인지 확인.
- Manual: 코드 리뷰에서 "이 문자열이 왜 String이어야 하는가"를 질문.

**예외**:
- 웹뷰 `<img src="data:image/png;base64,...">`처럼 base64 자체가 목적지 API의 요구사항인 경우.
- 데이터 크기가 매우 작고(아이콘, 1KB 미만) 인코딩 비용이 무시 가능한 경우.

**Bitvue 판정**: Confirmed — `get_decoded_frame`/`get_decoded_frame_yuv`(src-tauri/src/commands/frame.rs:14-45, "Base64 encoded PNG (full resolution)")와 `get_thumbnails`(src-tauri/src/commands/thumbnails.rs:312-313, src-tauri/src/services/thumbnail_service.rs:272-273)가 항상 base64 data URL로 인코딩한다. 필름스트립 대량 썸네일 로딩이라는 문서의 발생 조건과 정확히 일치한다.

---

### IPC-005: frame마다 전체 project state 전송
**분류**: 상태 동기화 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct ProjectState {
    open_files: Vec<StreamInfo>,
    all_frames: Vec<FrameSummary>,   // 전체 시퀀스의 모든 프레임 메타데이터
    current_frame_index: u32,
    overlay_settings: OverlaySettings,
    playback_state: PlaybackState,
}

#[tauri::command]
fn seek_frame(state: tauri::State<AppState>, frame_index: u32) -> ProjectState {
    let mut app = state.lock().unwrap();
    app.current_frame_index = frame_index;
    app.snapshot() // 전체 상태를 매번 복제해 반환
}
```

**문제**:
- 프레임 하나를 이동했을 뿐인데 수천 프레임짜리 시퀀스의 `all_frames` 메타데이터 전체가 다시 직렬화·전송된다.
- 실제로 바뀐 값은 `current_frame_index`(4바이트) 하나뿐인데 payload는 MB 단위가 될 수 있다.
- 프런트가 `ProjectState` 전체를 매번 교체하면 React 리렌더 범위도 불필요하게 커진다(참조 동등성 깨짐).

**발생 조건**:
- 긴 스트림(수천~수만 프레임)을 열어놓고 프레임 탐색을 반복하는 일반적인 사용 패턴.
- "상태는 하나의 진실 소스로 통합 관리"라는 원칙을 IPC 경계에도 그대로 적용해버린 설계.

**권장**:
```rust
#[derive(serde::Serialize)]
struct SeekResult {
    frame_index: u32,
    frame_summary: FrameSummary, // 현재 프레임 하나에 대한 요약만
}

#[tauri::command]
fn seek_frame(state: tauri::State<AppState>, frame_index: u32) -> SeekResult {
    let mut app = state.lock().unwrap();
    app.current_frame_index = frame_index;
    SeekResult { frame_index, frame_summary: app.frame_summary(frame_index) }
}
```
- 프레임 탐색처럼 빈번한 커맨드는 변경된 최소 단위(delta)만 반환한다.
- `all_frames` 같은 정적/저빈도 데이터는 스트림을 처음 열 때 한 번만 로드하고 프런트에 캐싱한다.
- 필요하면 커맨드를 `seek_frame`(가벼움)과 `get_project_snapshot`(무거움, 저빈도)으로 분리한다.

**탐지 방법**:
- Structural: 호출 빈도가 높은 것으로 추정되는 커맨드(이름에 seek/next/prev/scrub 등)의 반환 타입이 최상위 상태 구조체 전체인지 확인.
- Runtime: 초당 커맨드 호출 횟수 대비 평균 payload 크기를 곱해 대역폭 사용량을 추정.

**예외**:
- 프레임 수가 적고(수십 개) 탐색 빈도가 낮은 짧은 클립 분석 도구에서는 단순화를 위해 전체 상태 반환이 실용적일 수 있다.

**중복 참고**: `TAURI_CMD.md` TAURI-CMD-005와 거의 동일한 관심사 — 감사 시 하나로 취급 권장.

**Bitvue 판정**: N/A — `seek_frame`류 커맨드나 `ProjectState` 전체 스냅샷 반환 패턴이 코드베이스에 없다. 프레임 탐색용 커맨드(`get_frame_analysis`, `get_decoded_frame_yuv`, `get_frame_hex_data` 등)는 모두 frame_index 하나에 대한 데이터만 반환한다.

---

### IPC-006: 작은 UI 변경에도 분석 결과 재직렬화
**분류**: 캐싱/무효화 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
#[tauri::command]
fn set_overlay_visibility(
    state: tauri::State<AppState>,
    frame_index: u32,
    show_mv: bool,
    show_qp: bool,
) -> FrameAnalysis {
    let mut app = state.lock().unwrap();
    app.overlay.show_mv = show_mv;
    app.overlay.show_qp = show_qp;
    app.full_analysis(frame_index) // 토글 하나 바꾸려고 전체 분석을 재직렬화
}
```
```ts
// 프런트: 체크박스 하나 클릭할 때마다 전체 재분석 결과를 다시 받음
function onToggleMv(checked: boolean) {
  invoke('set_overlay_visibility', { frameIndex, showMv: checked, showQp });
}
```

**문제**:
- MV 오버레이 체크박스 하나를 켜고 끄는 순수 UI 상태 변경이 백엔드 재직렬화(수십~수백 KB)를 유발한다.
- 오버레이 표시 여부는 렌더링 관심사이지 분석 데이터 자체가 바뀌는 것이 아닌데, 이 둘을 하나의 커맨드로 결합했다.
- 체크박스를 빠르게 여러 번 토글하면(예: MV/QP/reference index를 순서대로 확인) 매번 전체 재직렬화가 큐잉되어 UI가 버벅인다.

**발생 조건**:
- 오버레이 토글, 색상 스케일 변경, 확대/축소처럼 "이미 받은 데이터의 표현 방식"만 바뀌는 조작.
- 분석 데이터 커맨드와 UI 설정 커맨드가 분리되어 있지 않은 초기 설계.

**권장**:
```rust
#[tauri::command]
fn set_overlay_visibility(state: tauri::State<AppState>, show_mv: bool, show_qp: bool) {
    let mut app = state.lock().unwrap();
    app.overlay.show_mv = show_mv;
    app.overlay.show_qp = show_qp;
    // 반환값 없음 — 순수 UI 설정이며 분석 데이터는 건드리지 않는다
}
```
```ts
// 프런트: 이미 받아 캐싱된 MV/QP 데이터를 그대로 두고 렌더 레이어만 토글
function onToggleMv(checked: boolean) {
  setOverlaySettings((s) => ({ ...s, showMv: checked })); // 로컬 상태만 변경
  invoke('set_overlay_visibility', { showMv: checked, showQp }); // fire-and-forget, 반환값 무시
}
```
- 분석 데이터(불변에 가까움)와 표시 설정(자주 바뀜)을 서로 다른 커맨드/상태로 분리한다.
- 오버레이 on/off는 이미 프런트가 들고 있는 데이터를 재사용하고, 백엔드 호출은 필요하면 설정 저장 목적의 non-blocking 호출로만 둔다.

**탐지 방법**:
- Runtime: UI 토글 이벤트와 IPC 트래픽을 상관 분석해, 순수 표시 상태 변경이 대형 payload 재전송을 유발하는지 확인.
- Manual: 커맨드 이름이 `set_*`/`toggle_*`인데 반환 타입이 대형 분석 구조체인 경우를 리뷰에서 지적.

**예외**:
- 표시 설정 변경이 실제로 새로운 계산을 요구하는 경우(예: QP 컬러 스케일의 min/max를 데이터 기반으로 재계산해야 하는 통계 오버레이)는 재계산이 정당할 수 있다.

**Bitvue 판정**: N/A — 오버레이 표시/숨김 토글(frontend/components/panels/OverlayRenderer/index.tsx 및 renderers)은 invoke() 호출이 전혀 없는 순수 프런트 로컬 상태다. 단, 이는 IPC-001에서 확인된 대로 `get_frame_analysis`가 이미 모든 grid를 한꺼번에 내려주기 때문에 가능한 것이며 근본 문제는 해소되지 않는다.

---

### IPC-007: pagination 부재
**분류**: 대용량 페이로드 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn list_nal_units(state: tauri::State<AppState>) -> Vec<NalUnitInfo> {
    // 몇 시간 분량 스트림이면 NAL unit이 수백만 개일 수 있다
    state.stream.nal_units.iter().map(NalUnitInfo::from).collect()
}
```

**문제**:
- 스트림 전체의 NAL unit(또는 packet, syntax element) 목록을 한 번에 반환하면 긴 영상에서 요소 수가 수백만에 달할 수 있다.
- 프런트의 리스트 UI(가상 스크롤이라 하더라도)는 실제로 화면에 보이는 수십~수백 개만 필요하다.
- 최초 로딩 시간이 스트림 길이에 선형으로 늘어나 "파일을 열었는데 몇 초간 응답이 없다"는 체감을 만든다.

**발생 조건**:
- 긴 방송/OTT 캡처본, 장시간 회의 녹화본처럼 NAL/프레임 수가 매우 많은 스트림을 분석할 때.
- syntax tree, 프레임 목록, 로그 뷰어 등 본질적으로 "리스트"인 모든 IPC 커맨드.

**권장**:
```rust
#[derive(serde::Deserialize)]
struct Page { offset: u32, limit: u32 }

#[tauri::command]
fn list_nal_units(state: tauri::State<AppState>, page: Page) -> Vec<NalUnitInfo> {
    state.stream.nal_units
        .iter()
        .skip(page.offset as usize)
        .take(page.limit.min(500) as usize) // 서버 측 상한도 강제
        .map(NalUnitInfo::from)
        .collect()
}
```
- 모든 리스트형 커맨드에 `offset`/`limit`(또는 커서 기반) 파라미터를 기본으로 둔다.
- 서버 측에서도 `limit`의 상한을 강제해, 프런트 버그로 인한 과도한 요청을 방어한다.
- 총 개수(`total_count`)는 별도의 가벼운 커맨드나 응답 헤더성 필드로 제공해 프런트가 스크롤바/페이지 UI를 구성할 수 있게 한다.

**탐지 방법**:
- Structural: `Vec<T>`를 반환하면서 파라미터에 offset/limit/cursor 류가 없는 커맨드를 정적으로 스캔.
- Runtime: 긴 스트림 로드 시 최초 응답 시간과 payload 크기를 스트림 길이 대비로 측정.

**예외**:
- 목록 크기의 상한이 설계상 보장되는 경우(예: 프레임당 최대 8개의 reference picture 목록)는 페이지네이션이 불필요하다.

**Bitvue 판정**: Confirmed — `get_frames_chunk`(src-tauri/src/commands/file.rs:498-540, offset/limit)로 페이지네이션을 구현해뒀음에도, 페이지네이션이 없는 `get_frames`(file.rs:453-490, 전체 `Vec<FrameData>` 반환)가 여전히 존재하며 frontend/contexts/LegacyStreamDataContext.tsx:57, frontend/utils/progressiveLoader.ts:128, frontend/utils/exportData.ts:286, ReferenceGraphPanel.tsx:57, BitrateGraphPanel.tsx:45에서 그대로 호출된다.
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-006 참고.

---

### IPC-008: viewport query 부재
**분류**: 쿼리 설계 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn get_mv_field(state: tauri::State<AppState>, frame_index: u32) -> Vec<MotionVector> {
    // 사용자가 25% 축소해서 프레임 우측 상단 귀퉁이만 보고 있어도
    // 전체 프레임의 모든 block에 대한 MV를 계산·직렬화한다
    state.frames[frame_index as usize].compute_all_mvs()
}
```

**문제**:
- QP map, MV field, 오버레이류 커맨드가 항상 "전체 프레임" 단위로만 데이터를 계산·반환하면, 확대(zoom)/스크롤로 실제로 보이는 영역이 전체의 일부여도 낭비가 그대로 발생한다.
- 계산 자체(엔트로피 디코딩 후 MV 재구성 등)가 CPU 비용이 크다면, viewport 밖 영역까지 계산하는 것은 전송 낭비를 넘어 CPU 낭비로 이어진다.
- 고배율 확대 상태에서는 반대로 정밀도가 부족할 수 있는데(픽셀당 서브블록을 더 세밀히 보여줘야 함) viewport 파라미터가 없으면 이런 LOD(level of detail) 전략 자체를 구현할 수 없다.

**발생 조건**:
- QP heatmap, MV field, block grid 오버레이를 사용자가 확대/축소/팬(pan)하며 탐색하는 모든 시나리오.
- 4K/8K처럼 전체 프레임의 block 수가 애초에 커서 "일부만 보이는 것이 기본값"인 해상도.

**권장**:
```rust
#[derive(serde::Deserialize)]
struct Rect { x: u32, y: u32, width: u32, height: u32 }

#[tauri::command]
fn get_mv_field(
    state: tauri::State<AppState>,
    frame_index: u32,
    viewport: Rect,
    lod: u8, // 0 = 원본 해상도, 1 = 1/4, 2 = 1/16 ... 축소 시 다운샘플
) -> tauri::ipc::Response {
    let mvs = state.frames[frame_index as usize].mvs_in(viewport, lod);
    tauri::ipc::Response::new(encode_mv_binary(&mvs))
}
```
- 좌표/픽셀 단위 `viewport: Rect`를 표준 파라미터로 모든 지도형(map-like) 오버레이 커맨드에 추가한다.
- 확대 배율에 따른 LOD(다운샘플링) 옵션을 함께 두면 축소 상태에서 불필요한 세부 데이터 전송도 줄일 수 있다.
- IPC-002의 바이너리 레코드 방식과 결합해 사용한다.

**탐지 방법**:
- Structural: "map/field/grid/heatmap" 계열 이름을 가진 커맨드 중 좌표 범위 파라미터가 없는 것을 스캔.
- Manual: 오버레이 확대/축소 시 IPC 트래픽이 화면에 보이는 영역과 무관하게 일정한지 리뷰.

**예외**:
- 오버레이 데이터 총량이 애초에 작아(저해상도 스트림, 큰 블록 크기) 전체 계산·전송 비용이 무시할 만한 경우.

**Bitvue 판정**: Confirmed — src-tauri/src/commands 전체에서 `viewport`/`Rect` 파라미터가 전혀 없다(grep 0건). `get_frame_analysis`와 내부 `extract_*_grid` 함수들은 항상 프레임 전체(coded_width x coded_height)에 대해 grid를 계산한다.

---

### IPC-009: frontend와 backend 양쪽에 동일 대형 복사본
**분류**: 상태 동기화 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// Rust 쪽: AppState가 전체 분석 결과를 계속 들고 있음
struct AppState {
    frames: Vec<FrameAnalysis>, // 프레임 수 x 수 MB
}
```
```ts
// TS 쪽: 같은 데이터를 다시 전부 받아 store에 통째로 보관
const useAnalysisStore = create<{ frames: FrameAnalysis[] }>((set) => ({
  frames: [],
  loadAll: async () => {
    const frames = await invoke<FrameAnalysis[]>('get_all_frames');
    set({ frames }); // 백엔드가 이미 갖고 있는 것과 동일한 사본을 프런트도 통째로 보관
  },
}));
```

**문제**:
- 동일한 대형 데이터(예: 전체 시퀀스의 QP map)가 Rust 프로세스 메모리와 웹뷰 힙에 이중으로 존재해 메모리 사용량이 두 배 이상이 된다.
- 두 사본 중 어느 쪽이 "진실"인지 불명확해지고, 백엔드에서 재계산(예: 파일 리로드) 후 프런트 캐시를 무효화하는 로직이 누락되면 화면에 오래된 데이터가 표시된다(→ IPC-017과 연결).
- 대형 초기 로드(`get_all_frames`)가 앱 시작을 느리게 만들고, 이후 거의 갱신되지 않는 데이터를 위해 상시 메모리를 점유한다.

**발생 조건**:
- "프런트에서 매번 IPC 호출하기 귀찮으니 한 번에 다 받아서 로컬 캐시에 넣자"는 결정을 한 경우.
- 백엔드가 스트리밍/증분 API를 제공하지 않아 프런트가 어쩔 수 없이 전체를 캐싱하는 경우.

**권장**:
```rust
#[tauri::command]
fn get_frame_analysis(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary {
    // 백엔드가 단일 진실 소스(source of truth) 역할을 하고,
    // 프런트는 최근 N개 프레임만 LRU 캐시로 들고 있는다.
    state.frame_summary(frame_index)
}
```
- 백엔드를 단일 진실 소스로 두고, 프런트는 화면에 필요한 범위(현재 프레임 ± N)만 LRU 캐시로 보관한다.
- 프런트 캐시 항목에는 반드시 무효화 키(frame_index, 분석 버전, 파일 해시 등)를 붙여 백엔드 상태 변경 시 갱신되도록 한다.
- 대형 데이터의 "소유자"를 설계 문서에 명시한다(백엔드 소유 vs 프런트 소유를 표로 정리).

**탐지 방법**:
- Semantic: 동일한 도메인 개념(FrameAnalysis, QpMap 등)을 표현하는 타입이 Rust와 TS 양쪽에 독립적으로 정의되어 있고, 둘 다 전체 데이터셋 규모로 보관되는지 코드베이스 교차 검토.
- Manual: 메모리 프로파일러로 대형 파일 로드 후 Rust 프로세스와 웹뷰 프로세스의 메모리 사용량을 각각 측정.

**예외**:
- 데이터가 작고(수백 KB 이하) 변경 빈도가 낮다면 단순성을 위해 프런트 전체 캐싱이 합리적일 수 있다.

**Bitvue 판정**: Suspected — frontend의 `frames` 배열(frontend/contexts/FileStateContext.tsx, YuvViewerPanel/index.tsx:271-291에서 qp/mv/partition 등 grid를 merge)이 방문한 프레임마다 무한정 누적되는 것으로 보이나(eviction 로직 미발견), 백엔드 AppState가 동일 grid를 캐싱해 이중 보유하는지는 확인하지 못했다(`get_frame_analysis`는 호출마다 재계산하는 것으로 보임). 완전한 이중 대형 사본이라는 근거까지는 아니다.

---

### IPC-010: String enum 남용
**분류**: 스키마 설계 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct BlockInfo {
    // ...
    prediction_mode: String,  // "INTRA_4x4" | "INTRA_16x16" | "INTER_SKIP" | ...
    frame_type: String,       // "I" | "P" | "B"
    nal_unit_type: String,    // "IDR_W_RADL" | "TRAIL_R" | ...
}
```

**문제**:
- 블록 수만 개 규모에서 각 문자열이 8~15바이트씩 반복되어 전체 payload를 눈에 띄게 키운다(정수 enum이면 1바이트로 충분).
- 오타/철자 불일치("INTER_SKIP" vs "Inter_Skip")가 컴파일 타임에 잡히지 않고 런타임 매칭 실패로만 드러난다.
- 프런트 TS 쪽에서 문자열 유니온 타입을 백엔드 Rust enum과 수동으로 동기화해야 하며, 코덱이 늘어날수록(AVC/HEVC/VP9/AV1/VVC) 문자열 후보군이 선형으로 늘어 유지보수 비용이 커진다.

**발생 조건**:
- 여러 코덱의 예측 모드/프레임 타입/NAL 타입처럼 값의 종류가 많고 자주 등장하는 필드.
- 디버깅 편의를 위해 "그냥 사람이 읽을 수 있게 문자열로 하자"는 초기 결정이 그대로 굳어진 경우.

**권장**:
```rust
#[derive(serde::Serialize_repr, Clone, Copy)]
#[repr(u8)]
enum PredictionMode {
    Intra4x4 = 0,
    Intra16x16 = 1,
    InterSkip = 2,
    // ...
}

#[derive(serde::Serialize)]
struct BlockInfo {
    prediction_mode: PredictionMode, // 정수 1바이트로 직렬화
}
```
```ts
// 프런트: 코드젠으로 생성한 상수/enum 매핑 사용
export enum PredictionMode { Intra4x4 = 0, Intra16x16 = 1, InterSkip = 2 /* ... */ }
```
- `serde_repr`(또는 수동 `u8` discriminant)로 enum을 정수로 직렬화하고, 사람이 읽을 이름은 프런트/디버그 도구에서 룩업 테이블로 매핑한다.
- 가능하면 Rust enum 정의로부터 TS enum/상수를 코드 생성(build script, `ts-rs`, `specta` 등)해 수동 동기화를 없앤다.
- 디버그 로그처럼 사람이 직접 읽어야 하는 경로에는 별도의 `Display`/`to_debug_string()`을 사용하고, IPC 경로와 분리한다.

**탐지 방법**:
- Static: `#[derive(Serialize)]` struct 필드 중 값의 후보가 명확히 유한 집합인데 타입이 `String`인 것을 grep/AST 분석으로 검출.
- Manual: 프런트 TS 타입 정의에서 문자열 리터럴 유니온과 Rust enum 이름이 수동으로 나열되어 있는지 리뷰.

**예외**:
- 값의 종류가 3~4개 이하이고 등장 빈도가 낮은(예: 파일 헤더의 container format 이름) 필드는 가독성을 위해 문자열로 두어도 무방하다.
- 외부 API/플러그인과의 호환을 위해 안정적인 문자열 식별자가 계약으로 요구되는 경우.

**Bitvue 판정**: Confirmed — `FrameData.frame_type: String`(src-tauri/src/commands/mod.rs:78, syntax.rs:90)이 "I"/"P"/"B" 문자열로 스트림 전체 프레임 수만큼 반복 직렬화된다(`get_frames`가 반환하는 `Vec<FrameData>`). 다만 block 단위 필드(mode/mb_types/tx_sizes, mod.rs:176,234,246,258)는 이미 u8 정수로 인코딩되어 있어 물량이 큰 부분은 이 안티패턴을 피했다.

---

### IPC-011: serde flatten으로 schema 경계 붕괴
**분류**: 스키마 설계 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct BaseBlockInfo { x: u32, y: u32, qp: u8 }

#[derive(serde::Serialize)]
struct HevcBlockExtra { cu_depth: u8, split_flag: bool }

#[derive(serde::Serialize)]
struct BlockInfo {
    #[serde(flatten)]
    base: BaseBlockInfo,
    #[serde(flatten)]
    hevc: HevcBlockExtra, // AVC/VP9/AV1 분석 시에는 아예 다른 Extra 타입이 flatten됨
}
```

**문제**:
- `#[serde(flatten)]`은 중첩 구조를 최상위로 "펼쳐서" JSON을 만들기 때문에, 어떤 필드가 base에서 왔고 어떤 필드가 codec별 extra에서 왔는지 출력 JSON만 보고는 알 수 없다.
- 서로 다른 codec extra 타입(HevcBlockExtra vs Av1BlockExtra)이 필드 이름을 우연히 공유하거나 충돌하면 컴파일 타임/런타임에 조용히 뒤섞일 수 있다.
- TS 쪽에서 discriminated union으로 안전하게 타입을 좁히기 어렵다 — flatten된 JSON에는 "이 객체가 어떤 스키마인지" 표시하는 태그가 자연스럽게 들어가지 않는 경우가 많다.
- `#[serde(flatten)]`은 내부적으로 `serde_json::Value`를 경유하는 경로를 타 성능이 저하될 수 있다(특히 대량 배열의 각 요소에 적용될 때 IPC-002 문제를 악화시킴).

**발생 조건**:
- 여러 코덱의 블록 정보를 하나의 공통 스키마 + 코덱별 확장으로 모델링하려는 시도.
- "공통 필드는 재사용하고 싶다"는 DRY 욕구가 IPC 경계에도 그대로 적용된 경우.

**권장**:
```rust
#[derive(serde::Serialize)]
#[serde(tag = "codec", content = "extra")]
enum BlockExtra {
    Hevc(HevcBlockExtra),
    Avc(AvcBlockExtra),
    Av1(Av1BlockExtra),
}

#[derive(serde::Serialize)]
struct BlockInfo {
    x: u32,
    y: u32,
    qp: u8,
    extra: BlockExtra, // flatten 대신 태그가 있는 하위 객체로 명시
}
```
- `#[serde(flatten)]` 대신 `#[serde(tag = "...", content = "...")]` 기반의 명시적 discriminated union을 사용해 스키마 경계를 JSON 구조에 그대로 보존한다.
- 코덱별 확장 필드는 중첩 객체로 두어 TS `switch (block.extra.codec)`로 안전하게 타입을 좁힐 수 있게 한다.
- 정말 순수하게 "동일 스키마의 선택적 확장"일 때만(같은 codec 내에서 버전만 다른 경우 등) flatten을 제한적으로 허용한다.

**탐지 방법**:
- Static: `#[serde(flatten)]` 사용처를 grep하고, flatten 대상 타입이 codec/variant별로 달라지는지 확인.
- Manual: 생성된 JSON 샘플을 보고 필드 출처를 사람이 구분할 수 있는지 리뷰.

**예외**:
- 확장 필드가 항상 하나의 고정된 타입이고(버저닝 목적의 단순 추가 필드) discriminated union이 과설계인 경우.

**Bitvue 판정**: N/A — `#[serde(flatten)]` 사용처가 src-tauri/src 전체에서 발견되지 않는다(grep 0건).

---

### IPC-012: Option 필드가 지나치게 많은 DTO
**분류**: 스키마 설계 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct BlockInfo {
    x: u32, y: u32, qp: u8,
    // AVC 전용
    mb_type: Option<u8>,
    // HEVC 전용
    cu_depth: Option<u8>,
    split_flag: Option<bool>,
    // VP9 전용
    tx_size: Option<u8>,
    skip_coeff: Option<bool>,
    // AV1 전용
    segment_id: Option<u8>,
    cdef_idx: Option<u8>,
    // ... 코덱이 늘어날 때마다 Option 필드가 계속 추가됨
}
```

**문제**:
- 실제로는 "이 블록의 코덱이 무엇이냐"에 따라 필드 집합이 상호 배타적으로 정해지는데, 타입 시스템이 그 사실을 표현하지 못해 모든 조합이 이론상 가능한 것처럼 보인다.
- 블록 수만 개 규모에서 `null` 값도 JSON 필드명 + `null` 텍스트로 직렬화되어(예: `"tx_size":null`) 실제 쓰이는 필드보다 안 쓰이는 필드의 텍스트가 더 많을 수 있다.
- 프런트에서 "이 블록이 어느 코덱인지"를 옵셔널 필드 존재 여부로 역추론해야 해서(`if (block.cu_depth !== undefined)`) 로직이 암묵적이고 취약해진다.
- 코덱이 하나 추가될 때마다 기존 DTO에 계속 Option 필드를 얹는 방식이 반복되어 구조체가 무한정 커진다.

**발생 조건**:
- 여러 코덱을 하나의 공통 DTO로 표현하려는 초기 설계가 코덱 지원 확대와 함께 누적적으로 나빠지는 경우(IPC-011과 함께 나타나는 경우가 많음).

**권장**:
```rust
#[derive(serde::Serialize)]
#[serde(tag = "codec")]
enum BlockDetail {
    Avc { mb_type: u8 },
    Hevc { cu_depth: u8, split_flag: bool },
    Vp9 { tx_size: u8, skip_coeff: bool },
    Av1 { segment_id: u8, cdef_idx: u8 },
}

#[derive(serde::Serialize)]
struct BlockInfo {
    x: u32, y: u32, qp: u8,
    detail: BlockDetail, // 코덱별 필드 집합을 타입으로 강제
}
```
- IPC-011과 동일한 해법: 상호 배타적인 필드 묶음은 `enum` variant로 모델링해 "이 조합만 유효하다"는 사실을 타입 수준에서 강제한다.
- 대량 배열에 대해서는 IPC-002처럼 codec을 스트림/커맨드 단위로 이미 알고 있다면(같은 스트림 안에서는 codec이 고정) DTO 자체를 코덱 전용으로 분리해 Option을 아예 없앨 수도 있다.

**탐지 방법**:
- Structural: struct의 `Option<T>` 필드 개수가 임계값(예: 5개 이상)을 넘는 DTO를 정적으로 스캔.
- Manual: 필드 이름에 특정 코덱 접두어(mb_, cu_, tx_, cdef_ 등)가 섞여 있는지 리뷰.

**예외**:
- 진짜로 선택적인(코덱과 무관하게 항상 있을 수도 없을 수도 있는) 필드, 예: "이 블록에 대한 사용자 주석" 같은 부가 정보는 Option이 적절하다.

**Bitvue 판정**: Confirmed — `FrameAnalysisData`(src-tauri/src/commands/mod.rs:276-287)에 codec별로 상호 배타적인 `Option<...GridData>` 필드가 7개(qp/mv/partition/prediction_mode/transform/mb_type/ref_idx) 존재하며, mb_type_grid/ref_idx_grid처럼 특정 코덱(AVC) 전용 필드가 공통 구조체에 계속 추가되는 형태다.

---

### IPC-013: 내부 domain model을 그대로 IPC 공개
**분류**: 계층 경계 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// 파서 내부 표현: 성능을 위해 arena 인덱스와 raw offset을 그대로 사용
pub struct CuNode {
    pub(crate) arena_idx: u32,       // 내부 arena allocator 인덱스
    pub(crate) parent_idx: Option<u32>,
    pub(crate) bitstream_offset: u64, // 비트 단위 원시 오프셋
    pub(crate) cabac_state_ptr: usize, // 디버깅용으로 남겨둔 raw pointer 주소값
    pub depth: u8,
}

#[derive(serde::Serialize)]
struct CuNodeDto(CuNode); // 내부 타입에 그대로 Serialize를 씌워 외부에 노출

#[tauri::command]
fn get_cu_tree(state: tauri::State<AppState>, frame_index: u32) -> Vec<CuNodeDto> {
    state.frames[frame_index as usize].cu_nodes.iter().map(|n| CuNodeDto(n.clone())).collect()
}
```

**문제**:
- `arena_idx`, `cabac_state_ptr` 같은 구현 세부사항이 그대로 외부(프런트)에 노출되어, 파서 내부 리팩터링(예: arena 구현을 다른 자료구조로 교체)이 곧바로 프런트 breaking change가 된다.
- 포인터 주소값처럼 프로세스 재시작마다 달라지고 웹뷰 쪽에서는 아무 의미도 없는 값이 payload에 섞여 들어가 대역폭을 낭비하고 혼란을 준다.
- 내부 모델은 파서 성능을 위해 최적화된 표현(압축 인덱스, 비트팩킹 등)인 경우가 많은데, 이를 DTO로 그대로 쓰면 "IPC 친화적으로 재설계"할 기회 자체가 사라진다.
- 내부 모델에 `pub(crate)` 필드가 있다는 것은 애초에 외부 공개를 의도하지 않았다는 신호인데, `Serialize`를 씌우는 순간 이 경계가 깨진다.

**발생 조건**:
- "이미 있는 구조체에 `#[derive(Serialize)]`만 추가하면 되니까" 하는 식으로 빠르게 커맨드를 붙일 때.
- 파서 모듈과 IPC 모듈을 같은 사람이 개발해 경계 의식이 느슨해질 때.

**권장**:
```rust
// IPC 전용 DTO를 별도로 정의하고, From/변환 함수로 명시적으로 매핑한다
#[derive(serde::Serialize)]
struct CuNodeDto {
    x: u16, y: u16,
    depth: u8,
    split: bool,
}

impl From<&CuNode> for CuNodeDto {
    fn from(n: &CuNode) -> Self {
        CuNodeDto { x: n.x(), y: n.y(), depth: n.depth, split: n.is_split() }
    }
}

#[tauri::command]
fn get_cu_tree(state: tauri::State<AppState>, frame_index: u32) -> Vec<CuNodeDto> {
    state.frames[frame_index as usize].cu_nodes.iter().map(CuNodeDto::from).collect()
}
```
- 내부 domain model과 IPC DTO를 항상 별도 타입으로 분리하고, `From`/`TryFrom`으로 명시적 변환 계층을 둔다.
- DTO에는 프런트가 실제로 필요로 하는 안정적인 필드만 남기고, 내부 최적화용 필드(인덱스, 포인터, 캐시 상태)는 제외한다.
- 이 경계가 있으면 파서 내부를 자유롭게 리팩터링해도 DTO 변환 함수만 갱신하면 되어 프런트 영향이 없다.

**탐지 방법**:
- Structural: `pub(crate)` 또는 내부 전용 필드를 가진 타입에 `#[derive(Serialize)]`가 직접 붙어 있고, 그 타입이 `#[tauri::command]` 반환 타입 경로에 등장하는지 정적 분석.
- Manual: "이 DTO 필드가 프런트 UI/로직에서 실제로 쓰이는가"를 필드별로 리뷰.

**예외**:
- 내부 모델이 애초에 IPC 경계를 염두에 두고 설계된 작은 값 타입(예: `Point { x: u16, y: u16 }`)이라면 별도 DTO 없이 재사용해도 무방하다.

**Bitvue 판정**: N/A — 내부 파서 타입(`bitvue_core::UnitNode`, `bitvue_decode::DecodedFrame`, 각 코덱 crate의 grid 타입)이 `#[tauri::command]` 반환 타입으로 직접 노출되는 사례를 찾지 못했다. `extractors.rs`와 `frame.rs`의 커맨드들은 모두 필드를 수동으로 매핑한 전용 DTO(QPGridData, DecodedFrameData 등)로 변환 후 반환한다.

---

### IPC-014: version 없는 IPC schema
**분류**: 스키마 버저닝 · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct FrameSummary {
    frame_index: u32,
    qp_avg: f32,
    frame_type: u8,
    // 버전 필드도, 스키마 식별자도 없음
}

#[tauri::command]
fn get_frame_summary(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary { /* ... */ }
```

**문제**:
- 개발 중 hot-reload(Vite dev server)로 프런트만 재빌드되고 백엔드(Rust)는 이전 빌드로 떠 있는 상태가 흔히 발생하는데, DTO에 버전 정보가 없으면 필드가 추가/삭제/타입 변경되어도 조용히 깨진다 — 런타임에서 `undefined` 접근이나 타입 불일치로만 드러난다.
- 여러 창(멀티 윈도우)이나 플러그인이 서로 다른 시점에 빌드된 프런트/백엔드 조합으로 실행될 가능성이 있는 배포 형태에서는 버전 불일치가 사용자 환경에서까지 발생할 수 있다.
- 마이그레이션(예: `mv_x/mv_y` 두 필드를 `mv: [i16;2]`로 통합) 시 프런트와 백엔드를 원자적으로 동시 배포할 수 없다면 과도기 동안 파싱 실패가 발생한다.

**발생 조건**:
- 활발히 개발 중인 초기 단계에서 DTO 필드가 자주 바뀌는 시기.
- 여러 릴리스 채널(stable/beta)이 공존하거나, 플러그인/확장이 별도로 버전 관리되는 경우.

**권장**:
```rust
const IPC_SCHEMA_VERSION: u32 = 3;

#[derive(serde::Serialize)]
struct FrameSummary {
    schema_version: u32, // 또는 커맨드 자체에 `_v2` 접미어로 버전을 명시
    frame_index: u32,
    qp_avg: f32,
    frame_type: u8,
}

#[tauri::command]
fn get_frame_summary_v2(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary {
    FrameSummary { schema_version: IPC_SCHEMA_VERSION, frame_index, qp_avg: /* .. */ 0.0, frame_type: 0 }
}
```
- DTO에 `schema_version` 필드를 두거나, breaking change 시 커맨드 이름 자체를 버저닝(`get_frame_summary_v2`)한다.
- 프런트 부팅 시 `get_backend_version()` 같은 커맨드로 백엔드 스키마 버전을 확인하고, 불일치 시 명확한 에러 메시지("앱을 재시작해주세요")를 표시한다.
- CI에 프런트 TS 타입과 Rust DTO 간 스키마 diff를 검사하는 단계를 추가한다(수동 동기화 오류 조기 발견).

**탐지 방법**:
- Manual: DTO 정의와 프런트 타입 정의를 함께 리뷰하며 버전 필드/버저닝 전략 유무를 확인.
- Runtime: 프런트에서 예상치 못한 필드 누락/타입 불일치를 감지하면 경고 로그를 남기는 방어 코드가 있는지 확인.

**예외**:
- 프런트와 백엔드가 반드시 단일 아티팩트로 함께 빌드·배포되어 버전 불일치가 구조적으로 불가능한 경우(예: 완전히 정적 링크된 단일 바이너리 + 자동 업데이트로 항상 동기화).

**Bitvue 판정**: Confirmed — src-tauri/src 및 frontend 전체에서 `schema_version`/`api_version`류 필드나 커맨드 버저닝(`_v2` 접미어 등)이 전혀 발견되지 않는다(grep 0건). `FrameAnalysisData`, `FrameData` 등 핵심 DTO에 버전 마커가 없다.

---

### IPC-015: command마다 global state lock
**분류**: 동시성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct AppState {
    inner: std::sync::Mutex<AppStateInner>, // 하나의 큰 Mutex로 모든 것을 보호
}

struct AppStateInner {
    decoder: Decoder,
    hex_cache: HexCache,
    qp_cache: QpCache,
    mv_cache: MvCache,
}

#[tauri::command]
fn get_hex_chunk(state: tauri::State<AppState>, offset: u64, len: u32) -> Vec<u8> {
    let inner = state.inner.lock().unwrap(); // hex view 조회가 QP/MV 조회까지 블로킹
    inner.hex_cache.read(offset, len)
}

#[tauri::command]
fn get_qp_map(state: tauri::State<AppState>, frame_index: u32) -> Vec<u8> {
    let inner = state.inner.lock().unwrap(); // 동일 lock을 경합
    inner.qp_cache.get(frame_index)
}
```

**문제**:
- hex view, QP map, MV field 패널이 동시에 열려 있으면 서로 무관한 데이터임에도 동일한 전역 `Mutex`를 두고 경합해, 한 패널의 느린 계산이 다른 패널의 응답을 지연시킨다.
- Tauri 커맨드는 기본적으로 비동기 런타임의 워커에서 실행될 수 있는데, 동기 `Mutex`를 커맨드 경계에서 통째로 잡으면 다른 비동기 태스크들이 블로킹되어 앱 전체의 반응성이 떨어진다.
- lock을 쥔 채로 무거운 계산(디코딩, 직렬화)까지 수행하면 lock 보유 시간이 늘어나 경합이 더 악화된다.

**발생 조건**:
- 여러 패널(hex/QP/MV/syntax tree)을 동시에 띄워 놓고 각각 독립적으로 프레임을 넘기거나 스크롤하는 일반적인 사용 패턴.
- 상태 구조체를 처음 설계할 때 "일단 하나의 Mutex로 감싸면 안전하다"는 이유로 세분화를 생략한 경우.

**권장**:
```rust
struct AppState {
    decoder: std::sync::Mutex<Decoder>,   // 디코더 상태만 보호
    hex_cache: HexCache,                  // 내부적으로 lock-free 또는 세분화된 락 사용
    qp_cache: QpCache,
    mv_cache: MvCache,
}
// 또는 read-heavy 워크로드라면 RwLock, 혹은 각 캐시를 Arc<DashMap<..>>로 교체

#[tauri::command]
async fn get_hex_chunk(state: tauri::State<'_, AppState>, offset: u64, len: u32) -> Result<Vec<u8>, String> {
    Ok(state.hex_cache.read(offset, len)) // 다른 패널의 조회와 경합하지 않음
}
```
- 상태를 기능 단위(디코더, hex cache, QP cache, MV cache)로 분리하고 각각 독립된 락(또는 lock-free 자료구조)을 사용한다.
- lock을 쥐는 범위를 "데이터 조회"로 최소화하고, 무거운 계산은 lock 밖에서(또는 lock을 짧게 여러 번 나눠) 수행한다.
- 읽기가 대부분인 캐시는 `RwLock` 또는 `DashMap` 등으로 교체해 동시 읽기를 허용한다.

**탐지 방법**:
- Structural: 여러 `#[tauri::command]`가 동일한 `Mutex<T>` 필드를 잠그는지, 그 T가 여러 독립적인 서브시스템을 포함하는지 정적 분석.
- Runtime: 여러 패널을 동시에 조작했을 때 커맨드 응답 지연을 프로파일링해 lock 경합 여부 확인(예: `tokio-console`, 자체 타이밍 로그).

**예외**:
- 서브시스템 간 실제로 강한 일관성이 필요한 경우(예: 디코더 상태와 캐시가 항상 함께 갱신되어야 하는 트랜잭션성 갱신)는 단일 락이 정당할 수 있다.

**중복 참고**: `TAURI_CMD.md` TAURI-CMD-004와 거의 동일한 관심사 — 감사 시 하나로 취급 권장.

**Bitvue 판정**: Confirmed — `AppState.core: Arc<Mutex<Core>>`(src-tauri/src/commands/mod.rs:104)를 `state.core.lock()`으로 잠그는 지점이 file.rs/frame.rs/thumbnails.rs/quality.rs/analysis/{mod,views}.rs/export.rs/debug_yuv.rs 등 21곳에 달한다. `Core` 내부는 stream_a/stream_b/selection이 이미 개별 `RwLock`으로 세분화되어 있는데(crates/bitvue-core/src/core.rs:43-55), 바깥의 단일 `Mutex<Core>`가 hex/QP/MV/썸네일/품질 커맨드를 서로 직렬화시켜 세분화 효과를 무력화한다.

---

### IPC-016: progress를 프레임/block 단위로 전송
**분류**: 이벤트/스트리밍 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
#[tauri::command]
fn analyze_stream(app: tauri::AppHandle, state: tauri::State<AppState>) {
    let total_frames = state.stream.frame_count();
    for i in 0..total_frames {
        state.analyze_one_frame(i);
        app.emit_all("analysis-progress", ProgressEvent {
            frame_index: i,
            total_frames,
        }).unwrap(); // 프레임마다, 수만 프레임짜리 스트림이면 수만 번 emit
    }
}
```

**문제**:
- 수만 프레임짜리 스트림을 분석하면 `emit_all`이 수만 번 호출되어 IPC 이벤트 채널과 프런트 이벤트 리스너를 과부하시킨다.
- 프런트에서 progress bar 하나 갱신하기 위해 초당 수백~수천 번 리렌더가 트리거될 수 있어, 정작 progress bar를 보여주려던 목적이 UI 프리징으로 역효과를 낸다.
- block 단위로 progress를 emit하면(예: CU 하나 분석할 때마다) 이벤트 수가 프레임의 블록 수만큼 곱해져 사실상 대응 불가능한 수준이 된다.

**발생 조건**:
- "정확한 진행률을 보여주고 싶다"는 의도로 세밀한 단위마다 progress 이벤트를 보내는 경우.
- 긴 스트림 전체 분석, 배치 처리처럼 반복 횟수가 매우 큰 작업.

**권장**:
```rust
#[tauri::command]
fn analyze_stream(app: tauri::AppHandle, state: tauri::State<AppState>) {
    let total_frames = state.stream.frame_count();
    let mut last_emit = std::time::Instant::now();
    for i in 0..total_frames {
        state.analyze_one_frame(i);
        // 100ms 간격 또는 1% 진행률 단위로만 emit (throttle/coalesce)
        if last_emit.elapsed() > std::time::Duration::from_millis(100) {
            app.emit_all("analysis-progress", ProgressEvent { frame_index: i, total_frames }).ok();
            last_emit = std::time::Instant::now();
        }
    }
    app.emit_all("analysis-progress", ProgressEvent { frame_index: total_frames, total_frames }).ok();
}
```
- progress 이벤트를 시간 기반(예: 100ms 간격) 또는 비율 기반(1% 단위)으로 throttle/coalesce한다.
- 프런트에서도 `requestAnimationFrame` 등으로 렌더링을 배칭해 이벤트 수신 빈도와 렌더 빈도를 분리한다.
- 정말 세밀한 상태가 필요하면 polling 커맨드(`get_analysis_progress`)를 프런트가 원하는 주기로 호출하게 해 제어권을 프런트에 준다.

**탐지 방법**:
- Runtime: 긴 작업 동안 `emit`/`emit_all` 호출 횟수를 계측해 초당 이벤트 수가 임계값(예: 30/sec)을 넘는지 확인.
- Structural: 루프 내부에서 반복 변수마다 무조건 emit하는 패턴을 정적으로 스캔.

**예외**:
- 반복 횟수가 작다고 보장되는 경우(예: 최대 10개 파일 배치 임포트)는 매 항목마다 emit해도 무방하다.

**Bitvue 판정**: N/A — src-tauri/src 전체에서 `emit`/`emit_all` 호출이 전혀 없다(grep 0건). 장시간 분석에 대한 progress 이벤트 스트리밍 자체가 아직 구현되어 있지 않다.

---

### IPC-017: 오래된 응답을 프런트가 그대로 적용
**분류**: 동시성/레이스 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```ts
// 프런트: 요청 순서와 응답 순서가 다를 수 있다는 것을 고려하지 않음
async function onSeek(frameIndex: number) {
  const analysis = await invoke<FrameAnalysis>('analyze_frame', { frameIndex });
  setCurrentAnalysis(analysis); // 늦게 도착한 이전 프레임 응답이 최신 프레임 응답을 덮어쓸 수 있음
}

// 사용자가 frame 10 -> frame 11로 빠르게 스크럽하면:
// invoke(10) 시작 (느림, 무거운 CU 트리 포함)
// invoke(11) 시작 (빠름, 캐시 hit)
// invoke(11) 응답 도착 -> setCurrentAnalysis(11)
// invoke(10) 응답 도착 -> setCurrentAnalysis(10)  <- 화면이 11에서 10으로 되돌아감
```

**문제**:
- 비동기 IPC 호출은 요청 순서대로 응답이 도착한다는 보장이 없다(백엔드 처리 시간이 요청마다 다르므로).
- 빠른 프레임 탐색(scrub) 시 오래된 요청의 응답이 최신 요청의 응답보다 늦게 도착해 화면을 "과거 상태로 되돌리는" 눈에 띄는 버그가 발생한다.
- 이 문제는 재현이 타이밍에 의존적이라 로컬 개발 환경(빠른 머신)에서는 잘 드러나지 않고 실사용 환경에서 간헐적으로만 나타나 디버깅이 어렵다.

**발생 조건**:
- 필름스트립 스크럽, 빠른 연속 클릭/키보드 탐색처럼 동일 대상에 대해 짧은 시간 내 여러 요청이 발생하는 모든 상호작용.
- 요청마다 처리 시간이 크게 달라지는 경우(캐시 hit/miss, 프레임별 복잡도 차이).

**권장**:
```ts
let latestRequestId = 0;

async function onSeek(frameIndex: number) {
  const requestId = ++latestRequestId;
  const analysis = await invoke<FrameAnalysis>('analyze_frame', { frameIndex });
  if (requestId !== latestRequestId) {
    return; // 그 사이 더 최신 요청이 발생했으면 이 응답은 폐기
  }
  setCurrentAnalysis(analysis);
}
```
```rust
// 필요하다면 백엔드에서도 취소 가능한 작업으로 만들어 무의미한 계산을 조기 중단한다
#[tauri::command]
async fn analyze_frame(
    state: tauri::State<'_, AppState>,
    frame_index: u32,
    request_id: u64,
) -> Result<FrameAnalysis, String> {
    if !state.is_latest_request(request_id) {
        return Err("stale_request".into());
    }
    Ok(state.analyze(frame_index))
}
```
- 프런트에서 요청 ID(또는 AbortController 유사 패턴)를 두어, 응답 도착 시점에 "이 요청이 여전히 최신인가"를 확인 후에만 상태를 갱신한다.
- 가능하면 백엔드에도 request_id를 전달해, 이미 stale해진 요청의 계산을 조기에 중단(취소)할 수 있게 한다 — 불필요한 CPU 낭비도 함께 줄인다.
- react-query, SWR 등 요청 취소/최신성 관리를 내장한 데이터 페칭 라이브러리 사용을 고려한다.

**탐지 방법**:
- Runtime: 인위적으로 응답 지연을 다르게 주입하는 테스트(느린 요청을 먼저 보내고 빠른 요청을 나중에 보내기)로 최종 UI 상태가 최신 요청과 일치하는지 검증.
- Manual: `await invoke(...)` 직후 곧바로 전역/컴포넌트 상태를 덮어쓰는 코드 패턴을 리뷰에서 검출.

**예외**:
- 요청이 항상 순차적으로 발생하고 이전 요청의 완료를 기다린 뒤에만 다음 요청을 보내도록 UI가 이미 직렬화되어 있는 경우(예: 버튼이 요청 중 비활성화됨).

**중복 참고**: `TAURI_CMD.md` TAURI-CMD-008과 거의 동일한 관심사 — 감사 시 하나로 취급 권장.

**Bitvue 판정**: Confirmed (일부) — frontend/components/panels/SyntaxDetailPanel/FrameSyntaxTab.tsx:67-85는 `frame` 변경 시 `invoke("get_frame_syntax", ...)`을 호출하며 취소/request-id 가드 없이 `.then(setSyntaxTree)`로 바로 상태를 덮어써, 빠른 프레임 이동 시 오래된 응답이 최신 화면을 덮어쓸 수 있다. 반면 frontend/components/panels/YuvViewerPanel/index.tsx:171-318은 `cancelled` 플래그로 동일 문제를 방어하고 있어 코드베이스 내 일관성이 없다.

---

### IPC-018: 오류를 단일 문자열로 변환
**분류**: 오류 처리 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn parse_bitstream(path: String) -> Result<StreamInfo, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let stream = Parser::new(&bytes)
        .parse()
        .map_err(|e| e.to_string())?; // ParseError::UnexpectedEof, InvalidStartCode 등 구조가 모두 문자열로 뭉개짐
    Ok(stream)
}
```
```ts
try {
  const info = await invoke('parse_bitstream', { path });
} catch (e) {
  // e는 그냥 문자열: "invalid start code at offset 4821"
  // 프런트는 이걸 파싱해서 다시 구조화하거나, 그냥 사용자에게 그대로 노출할 수밖에 없다
  showToast(e as string);
}
```

**문제**:
- `Result<T, String>`은 에러의 종류(파일 없음 vs 권한 없음 vs 파싱 실패 vs 지원하지 않는 코덱)를 프런트가 구분할 방법을 없앤다.
- "재시도 가능한 오류"와 "재시도해도 소용없는 오류"를 구분할 수 없어, 프런트가 적절한 복구 UX(재시도 버튼, 파일 재선택 안내 등)를 제공하지 못한다.
- 에러 메시지에 포함된 offset, 파일 경로 등 구조화된 정보를 프런트가 다시 정규식으로 파싱해야 한다면, 메시지 문구가 바뀌는 순간 프런트 로직이 깨진다.
- 다국어 지원 시 에러 메시지를 백엔드(Rust)에서 고정 문자열로 만들어버리면 프런트에서 로케일에 맞게 번역할 수 없다.

**발생 조건**:
- 파일 열기, 비트스트림 파싱, 코덱 미지원 등 사용자에게 각기 다른 대응을 요구하는 다양한 실패 경로가 있는 커맨드.

**권장**:
```rust
#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "detail")]
enum ParseErrorDto {
    FileNotFound { path: String },
    PermissionDenied { path: String },
    InvalidStartCode { offset: u64 },
    UnsupportedCodec { fourcc: String },
    UnexpectedEof { offset: u64 },
}

#[tauri::command]
fn parse_bitstream(path: String) -> Result<StreamInfo, ParseErrorDto> {
    let bytes = std::fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ParseErrorDto::FileNotFound { path: path.clone() },
        std::io::ErrorKind::PermissionDenied => ParseErrorDto::PermissionDenied { path: path.clone() },
        _ => ParseErrorDto::UnexpectedEof { offset: 0 },
    })?;
    Parser::new(&bytes).parse().map_err(ParseErrorDto::from)
}
```
```ts
try {
  const info = await invoke<StreamInfo>('parse_bitstream', { path });
} catch (e) {
  const err = e as ParseErrorDto;
  switch (err.kind) {
    case 'FileNotFound': return showFileNotFoundDialog(err.detail.path);
    case 'UnsupportedCodec': return showUnsupportedCodecDialog(err.detail.fourcc);
    // ...
  }
}
```
- 도메인 에러를 `#[serde(tag=..., content=...)]` 구조화 enum(IPC-011에서 다룬 패턴과 동일)으로 정의해 `Result<T, ErrorDto>`로 반환한다.
- 메시지 문구(사람이 읽는 텍스트)는 프런트에서 `kind`를 기반으로 로케일에 맞게 생성하고, 백엔드는 구조화된 사실(offset, path, fourcc 등)만 전달한다.
- `thiserror` 등으로 내부 에러 타입을 만들고, IPC 경계에서 `From<InternalError> for ErrorDto` 변환을 명시적으로 둔다.

**탐지 방법**:
- Structural: `Result<_, String>` 시그니처를 가진 `#[tauri::command]`를 정적으로 스캔.
- Manual: 프런트에서 `catch (e)` 블록이 에러 문자열을 정규식/포함 여부로 분기하는 코드가 있는지 리뷰(구조화 실패의 강한 신호).

**예외**:
- 사용자에게 노출할 필요가 없는 내부 디버그 전용 커맨드나, 실패 종류가 사실상 하나뿐인 단순 커맨드는 문자열 에러로도 충분하다.

**관련**: `TAURI_CMD.md` TAURI-CMD-009 참고 — 이쪽은 에러 종류(도메인 오류 taxonomy)를 다루고, TAURI-CMD-009는 "실패 지점이 직렬화 단계였는지" 자체를 프런트가 식별할 수 있는가를 다뤄 관심사가 다르다.

**Bitvue 판정**: Confirmed — src-tauri/src/commands/*.rs 전반에 `Result<_, String>` 시그니처가 88건 존재하며(예: file.rs:407 `get_stream_info`), 전부 `.map_err(|e| e.to_string())` 패턴이다. 태그가 있는 구조화 에러 enum은 어디에도 없다(`#[serde(tag=...)]` grep 0건).
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-005 참고.

---

### IPC-019: 압축 여부를 측정하지 않고 무조건 압축
**분류**: 압축 · **심각도**: Low · **탐지**: Runtime

**나쁜 예**:
```rust
#[tauri::command]
fn get_frame_flags(state: tauri::State<AppState>, frame_index: u32) -> tauri::ipc::Response {
    let flags = state.frame_flags(frame_index); // 예: 프레임 타입, IDR 여부 등 수 바이트짜리 정보
    let json = serde_json::to_vec(&flags).unwrap();
    let compressed = zstd::encode_all(&json[..], 3).unwrap(); // 몇 바이트짜리 데이터를 항상 압축
    tauri::ipc::Response::new(compressed)
}
```

**문제**:
- 이미 수 바이트~수십 바이트에 불과한 payload를 압축하면, 압축 알고리즘의 헤더/사전(dictionary) 오버헤드 때문에 오히려 결과물이 원본보다 커질 수 있다.
- 압축/해제에 드는 CPU 사이클이 로컬 IPC(같은 머신 내 프로세스 간 통신, 네트워크 왕복이 없음)에서 아끼는 전송 비용보다 클 수 있다 — 특히 이런 작은 커맨드가 프레임마다 반복 호출되면 누적 CPU 낭비가 된다.
- 반대로, 정말 압축 효과가 큰 대형 payload(QP map, MV field 등 반복 패턴이 많은 수치 배열)에는 압축을 전혀 적용하지 않아 얻을 수 있었던 이득을 놓치는 경우도 같은 안티패턴의 반대 사례다.

**발생 조건**:
- "IPC는 무조건 압축하는 게 좋다"는 일반론을 크기 측정 없이 모든 커맨드에 일괄 적용한 경우.
- 반대로, 대형 오버레이 데이터 커맨드를 설계하면서 압축을 전혀 고려하지 않은 경우.

**권장**:
```rust
const COMPRESSION_THRESHOLD: usize = 4096; // 이 이상일 때만 압축을 고려

#[tauri::command]
fn get_qp_map(state: tauri::State<AppState>, frame_index: u32, viewport: Rect) -> tauri::ipc::Response {
    let raw = encode_qp_binary(&state.qp_map_in(frame_index, viewport));
    if raw.len() > COMPRESSION_THRESHOLD {
        let compressed = zstd::encode_all(&raw[..], 1).unwrap(); // 낮은 레벨로 CPU/비율 균형
        tauri::ipc::Response::new(compressed)
    } else {
        tauri::ipc::Response::new(raw) // 작은 payload는 압축하지 않음
    }
}
```
- payload 크기에 임계값을 두어, 그 이상일 때만 압축을 적용한다(로컬 IPC이므로 임계값은 네트워크 API보다 훨씬 크게 잡아도 된다 — 예: 수 KB 이상).
- 압축을 적용할 때는 실제로 원본보다 작아졌는지 확인하고, 그렇지 않으면 비압축본을 그대로 전송하는 폴백을 둔다.
- QP map/MV field처럼 반복 패턴이 많아 압축률이 높을 것으로 예상되는 대형 수치 배열에는 적극적으로 압축(zstd 등)을 적용해 대역폭을 줄인다.

**탐지 방법**:
- Runtime: 커맨드별로 압축 전/후 크기 비율을 계측해, 비율이 1에 가깝거나 1을 넘는(압축이 손해인) 커맨드를 찾아낸다.
- Manual: 모든 커맨드에 동일한 압축 로직이 무조건 적용되어 있는지, 아니면 크기 조건부인지 코드 리뷰.

**예외**:
- payload 크기가 항상 크다고 보장되는 커맨드(예: 항상 수 MB인 전체 hex dump)라면 조건 분기 없이 항상 압축해도 된다.

**Bitvue 판정**: N/A — src-tauri Cargo.toml 및 소스 전체에서 zstd/flate2/lz4 등 압축 라이브러리 사용이 발견되지 않는다. 압축을 과도하게도, 적절하게도 적용하지 않는 상태라 이 안티패턴 자체가 성립하지 않는다.

---

### IPC-020: 백엔드 계산과 프런트 계산이 중복
**분류**: 계층 경계/중복 계산 · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct FrameSummary {
    frame_index: u32,
    qp_values: Vec<u8>, // block별 원시 QP 값만 전달
}

#[tauri::command]
fn get_frame_summary(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary {
    FrameSummary { frame_index, qp_values: state.raw_qp_values(frame_index) }
}
```
```ts
// 프런트: 매 렌더마다 평균/표준편차/히스토그램을 다시 계산
function FrameStatsPanel({ summary }: { summary: FrameSummary }) {
  const avgQp = useMemo(
    () => summary.qp_values.reduce((a, b) => a + b, 0) / summary.qp_values.length,
    [summary.qp_values],
  );
  const stdDev = useMemo(() => computeStdDev(summary.qp_values), [summary.qp_values]);
  // 이 통계는 Rust 쪽에도 거의 동일한 로직으로 이미 구현되어 있다(다른 화면에서 사용 중)
  // ...
}
```

**문제**:
- 평균 QP, 표준편차, 히스토그램 같은 파생 통계를 Rust와 TypeScript 양쪽에서 각각 구현하면 두 구현이 시간이 지나며 미묘하게 달라질 위험이 있다(반올림 방식, NaN/빈 배열 처리 등).
- 원시 `qp_values` 배열 전체를 프런트로 보내고 거기서 통계를 계산하는 방식은, 정작 필요한 것이 스칼라 값(평균 하나) 뿐인데도 배열 전체 전송을 강제해 IPC-001/002류 낭비와 결합된다.
- 통계 로직에 버그가 있을 때 "어느 쪽 구현이 틀렸는가"를 양쪽 다 확인해야 하고, 코드 리뷰/테스트도 두 언어로 중복된다.
- 대량 데이터(수만 블록)에 대한 통계 계산을 JS 싱글 스레드에서 매 렌더마다(혹은 `useMemo` 캐시가 무효화될 때마다) 수행하면 Rust에서 미리 계산해 보내는 것보다 훨씬 느리다.

**발생 조건**:
- "일단 원시 데이터를 보내고 프런트에서 필요한 대로 가공하자"는 편의적 설계가 여러 패널에 걸쳐 반복되는 경우.
- 백엔드 개발자와 프런트 개발자가 같은 통계 요구사항을 각자 독립적으로 구현하는 협업 구조.

**권장**:
```rust
#[derive(serde::Serialize)]
struct QpStats { avg: f32, min: u8, max: u8, std_dev: f32, histogram: [u32; 64] }

#[derive(serde::Serialize)]
struct FrameSummary {
    frame_index: u32,
    qp_stats: QpStats, // 파생 통계는 백엔드가 단일 구현으로 계산해 전달
}

#[tauri::command]
fn get_frame_summary(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary {
    let raw = state.raw_qp_values(frame_index);
    FrameSummary { frame_index, qp_stats: compute_qp_stats(&raw) } // 단일 진실 소스
}
```
- 파생 통계/집계는 원본 데이터를 소유한 백엔드에서 한 번만 계산해 결과(스칼라/작은 구조체)만 전달한다 — "계산 로직의 단일 진실 소스"를 명확히 한다.
- 프런트는 오직 표시/상호작용 목적의 가벼운 변환(포맷팅, 색상 매핑)만 담당하고, 통계적 의미가 있는 계산은 하지 않는다.
- 만약 프런트에서도 같은 로직이 필요하다면(예: 사용자가 실시간으로 필터링하며 재계산해야 하는 대화형 통계) Rust 로직을 WASM으로 컴파일해 공유하는 방법도 고려한다.

**탐지 방법**:
- Semantic: 동일한 이름/의미의 통계 함수(average, stddev, histogram 등)가 Rust와 TS 코드베이스 양쪽에 존재하는지 교차 검색.
- Manual: 프런트 컴포넌트에서 `reduce`, `sort`, 반복문으로 원시 배열을 가공해 통계를 만드는 코드가 있는지, 그 배열이 IPC로 받은 원시 데이터인지 리뷰.

**예외**:
- 순수하게 표시 목적의 가벼운 변환(단위 환산, 반올림, 색상 스케일 매핑)은 프런트에서 하는 것이 자연스럽고 중복이라 보기 어렵다.
- 사용자가 프런트에서만 국소적으로 필터링/정렬한 부분집합에 대한 통계처럼, 백엔드가 애초에 알 수 없는 범위의 계산은 프런트에서 하는 것이 맞다.

**Bitvue 판정**: Confirmed — src-tauri/src/commands/export.rs:342가 `export_analysis_report`용으로 평균 프레임 크기(avg_size)를 Rust에서 계산하는 한편, frontend/components/Filmstrip/views/FrameSizesView.tsx:82-92가 동일한 원시 `size` 값들로부터 평균/최소/최대 프레임 크기를 TypeScript에서 별도로 재계산한다 — 동일 통계의 이중 구현.
