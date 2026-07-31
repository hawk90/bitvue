# Anti-Pattern Catalog — TAURI_CMD: Command 중심 설계

이 문서는 Bitvue 안티패턴 카탈로그의 한 갈래이며, 전체 카탈로그의 색인은 별도로 작성 중인 `docs/anti-patterns/INDEX.md`를 참고한다. UI/UX+Tauri Phase 3 물결(wave)에 속하며, 같은 물결의 `TAURI_EVT.md`(이벤트/구독 설계), `TAURI_WEB.md`(webview/window 경계)와 형제 문서다. Phase 1의 `IPC.md`가 커맨드 하나의 **페이로드 크기·직렬화 형식**을 다뤘다면, 이 문서는 그보다 한 단계 위 — **커맨드를 몇 개로 얼마나 잘게 나누고, 누가 그 호출 순서를 조립하며, 실패와 지연을 어떻게 다룰지**, 즉 command granularity와 orchestration을 다룬다. 개별 커맨드의 응답 바이트 형식은 이 문서의 관심사가 아니다.

---

### TAURI-CMD-001: UI 동작 하나당 세세한 command 여러 개 호출

**분류**: Command 세분화 과다 · **심각도**: Medium · **탐지**: Code

**나쁜 예**:
```typescript
async function onFrameTypeFilterToggle(type: "I" | "P" | "B") {
  await invoke("set_filter_flag", { type, enabled: true });
  await invoke("recompute_visible_indices");
  await invoke("update_filter_badge_count");
  await invoke("mark_filter_dirty");
}
```

**문제**:
- 사용자 관점에서는 "필터 토글"이라는 단일 행위인데, 프런트가 이를 4개의 순차 IPC 왕복으로 쪼개 놓았다.
- 각 `invoke`는 최소한 직렬화 + IPC 큐잉 + 역직렬화 오버헤드를 가지므로, 커맨드 하나면 될 일이 네 번의 컨텍스트 스위치로 늘어난다.
- 중간 커맨드(`recompute_visible_indices`)가 실패하면 `set_filter_flag`는 이미 적용된 상태로 남아 있어 UI 상태와 backend 상태가 어긋난다.
- 이런 호출 체인이 여러 곳에 흩어지면, "필터 토글"의 진짜 의미(원자적으로 무엇이 바뀌어야 하는가)가 backend 어디에도 명시적으로 존재하지 않고 프런트 호출 순서에만 암묵적으로 존재하게 된다.

**발생 조건**:
- Rust 쪽 함수를 만들 때마다 그대로 `#[tauri::command]`를 붙여 노출하는 습관이 있을 때.
- 기능이 점진적으로 추가되며 매번 "커맨드 하나만 더" 식으로 늘어난 경우.

**권장**:
```rust
#[tauri::command]
fn toggle_frame_type_filter(state: tauri::State<AppState>, frame_type: FrameType, enabled: bool) -> FilterState {
    let mut filters = state.filters.lock().unwrap();
    filters.set(frame_type, enabled);
    filters.recompute(); // 내부 단계는 backend가 원자적으로 처리
    filters.snapshot()
}
```
```typescript
const filterState = await invoke<FilterState>("toggle_frame_type_filter", { frameType: "I", enabled: true });
```
- 하나의 사용자 행위에 대응하는 하나의 커맨드로 묶고, 내부 단계(재계산, 카운트 갱신, dirty 마킹)는 backend 함수 호출로 처리한다.
- 반환값에 UI가 그 즉시 필요로 하는 최소 상태(예: 갱신된 필터 상태)를 포함시켜 후속 조회 커맨드를 없앤다.

**탐지 방법**:
- Code: 하나의 이벤트 핸들러(`onClick`, `onToggle` 등) 안에서 `invoke`가 2회 이상 순차 호출되는 패턴을 정적 스캔.
- Structural: 커맨드 이름들이 `set_x`, `recompute_x`, `update_x_badge`처럼 같은 명사(`x`)의 하위 단계로 계열을 이루는지 검토.

**예외**:
- 각 단계가 독립적으로 취소 가능해야 하거나(예: 사용자가 중간에 다른 동작으로 전환), 단계별로 진행률 UI를 보여줘야 하는 경우에는 의도적인 분리가 정당하다. 이때는 TAURI-CMD-007의 트랜잭션/롤백 설계와 함께 고려해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-002: command가 domain service와 presentation을 모두 담당

**분류**: 책임 혼재 · **심각도**: High · **탐지**: Code

**나쁜 예**:
```rust
#[tauri::command]
fn get_mv_overlay_svg(state: tauri::State<AppState>, frame_index: u32, zoom: f32) -> String {
    let decoder = state.decoder.lock().unwrap();
    let mvs = decoder.motion_vectors(frame_index); // 도메인 계산

    // 이 아래부터는 순수 presentation 로직인데 같은 함수 안에 있다
    let mut svg = String::from("<svg>");
    for mv in &mvs {
        let x = mv.x as f32 * zoom;
        let y = mv.y as f32 * zoom;
        let color = if mv.magnitude() > 8.0 { "#ff4444" } else { "#44aaff" };
        svg.push_str(&format!(r#"<line x1="{x}" y1="{y}" stroke="{color}"/>"#));
    }
    svg.push_str("</svg>");
    svg
}
```

**문제**:
- 색상 임계값, zoom 배율 적용, SVG 마크업 생성 같은 표시 로직이 Rust 커맨드 안에 하드코딩되어, 프런트에서 테마(다크모드)나 색맹 대응 팔레트를 바꾸려면 backend를 다시 빌드해야 한다.
- 도메인 데이터(MV 목록)와 presentation 산출물(SVG 문자열)이 한 응답에 뒤섞여, MV 데이터만 다른 용도(예: CSV 내보내기, 통계)로 재사용하려 해도 SVG를 다시 파싱해야 한다.
- 커맨드를 테스트하려면 SVG 문자열을 파싱해 도메인 로직이 맞는지 검증해야 하는데, 이는 순수 계산 로직 테스트와 렌더링 테스트를 억지로 하나로 묶는 것이다.
- zoom처럼 순수 UI 상태(사용자가 마우스 휠로 계속 바꾸는 값)가 매번 IPC 파라미터로 backend까지 왕복해야 한다.

**발생 조건**:
- 초기 프로토타입에서 "일단 화면에 그려지는 걸 빨리 보자"는 압력으로 backend가 SVG/HTML 문자열을 직접 만들어 반환하는 경우.
- 팀에 프런트 전담 인력이 없어 Rust 개발자가 렌더링까지 겸하는 경우.

**권장**:
```rust
#[derive(serde::Serialize)]
struct MotionVector { x: i16, y: i16, magnitude: f32 }

#[tauri::command]
fn get_motion_vectors(state: tauri::State<AppState>, frame_index: u32) -> Vec<MotionVector> {
    let decoder = state.decoder.lock().unwrap();
    decoder.motion_vectors(frame_index).into_iter().map(Into::into).collect()
}
```
```typescript
const mvs = await invoke<MotionVector[]>("get_motion_vectors", { frameIndex });
const svg = renderMvOverlaySvg(mvs, { zoom, theme }); // presentation은 프런트 책임
```
- command는 도메인 데이터(측정값·구조)만 반환하고, zoom/색상/좌표 변환 같은 순수 표시 로직은 프런트에 둔다.
- "이 값이 바뀌면 재계산이 필요한가(backend), 다시 그리기만 하면 되는가(frontend)"를 기준으로 경계를 가른다.

**탐지 방법**:
- Code: 커맨드 반환 타입이 `String`이면서 그 값이 HTML/SVG/CSS 태그를 포함하는지(`<svg`, `<div`, `style=` 등 문자열 검색) 정적 스캔.
- Manual: 커맨드 함수 본문에 색상 코드(`#rrggbb`)나 zoom/scale 같은 UI 전용 파라미터가 등장하는지 리뷰.

**예외**:
- 서버사이드 렌더링이 필수인 export 기능(예: "PNG로 내보내기" 커맨드가 실제로 이미지 바이트를 생성해야 하는 경우)은 backend가 최종 픽셀을 만드는 것이 목적 자체이므로 이 패턴에 해당하지 않는다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-003: command 이름이 화면 컴포넌트에 종속

**분류**: 네이밍/결합도 · **심각도**: Medium · **탐지**: Code

**나쁜 예**:
```rust
#[tauri::command]
fn get_qp_overlay_panel_data(state: tauri::State<AppState>, frame_index: u32) -> QpOverlayPanelData { /* ... */ }

#[tauri::command]
fn get_mv_overlay_panel_right_sidebar_data(state: tauri::State<AppState>, frame_index: u32) -> MvSidebarData { /* ... */ }
```

**문제**:
- 커맨드 이름에 `panel`, `sidebar`, `right_` 같은 레이아웃 어휘가 들어가면, backend API가 도메인 개념이 아니라 특정 시점의 화면 배치를 인코딩하게 된다.
- 프런트에서 QP 오버레이를 사이드바가 아니라 모달로 옮기거나 두 패널을 합치는 리팩터링을 하면, 데이터 자체는 그대로인데도 커맨드 이름을 바꾸거나(→ 모든 호출부 수정) 이름과 실제 용도가 어긋난 채로 방치된다.
- 같은 QP 데이터를 다른 화면(예: 새로 추가되는 "비교 보기")에서 재사용하려 할 때, 이름만 보고는 재사용 가능한지 판단하기 어렵다.
- backend와 frontend 저장소/PR을 분리 관리하는 팀이라면 화면 리네이밍 하나가 API 계약 변경으로 번져 두 저장소를 동시에 고쳐야 한다.

**발생 조건**:
- "이 패널이 필요로 하는 데이터"를 기준으로 커맨드를 하나씩 새로 만드는 개발 방식이 굳어진 경우.
- 화면 구조가 아직 유동적인 초기 개발 단계에서 성급하게 컴포넌트 이름을 API에 박아 넣는 경우.

**권장**:
```rust
#[tauri::command]
fn get_qp_statistics(state: tauri::State<AppState>, frame_index: u32) -> QpStatistics { /* ... */ }

#[tauri::command]
fn get_motion_vectors(state: tauri::State<AppState>, frame_index: u32) -> Vec<MotionVector> { /* ... */ }
```
- 커맨드 이름은 "무엇을 반환하는가(도메인 명사)"로 짓고, "어디에 쓰이는가(화면 위치)"는 이름에서 배제한다.
- 여러 패널이 같은 커맨드를 구독/호출해도 무방하도록 설계하고, 화면별 조합·배치는 프런트 상태 관리 계층의 책임으로 둔다.

**탐지 방법**:
- Code: 커맨드 함수/모듈 이름에 `panel`, `sidebar`, `modal`, `tab`, `view` 등 레이아웃 어휘가 포함되는지 정적 grep.
- Domain review: 커맨드 목록을 도메인 전문가에게 보여주고, 화면을 본 적 없어도 이름만으로 반환 데이터를 추측할 수 있는지 확인.

**예외**:
- 특정 화면 전용으로 설계 의도가 명확하고 재사용 가능성이 실질적으로 없는 export/디버그 커맨드(예: `dump_debug_overlay_state_for_screenshot_test`)는 이름에 용도를 명시하는 편이 오히려 명확할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-004: 모든 command가 global state lock

**분류**: 동시성/orchestration · **심각도**: High · **탐지**: Performance

**나쁜 예**:
```rust
struct AppState {
    inner: Mutex<AppStateInner>, // 디코더, 프레임 캐시, 필터, UI 힌트까지 전부 하나의 Mutex 안
}

#[tauri::command]
fn get_frame_thumbnail(state: tauri::State<AppState>, frame_index: u32) -> Vec<u8> {
    let inner = state.inner.lock().unwrap(); // 전체 lock
    inner.thumbnail_cache.get(frame_index)
}

#[tauri::command]
fn parse_next_gop(state: tauri::State<AppState>) -> GopInfo {
    let mut inner = state.inner.lock().unwrap(); // 같은 lock — 수백 ms 걸릴 수 있음
    inner.decoder.parse_gop()
}
```

**문제**:
- 썸네일 하나 가져오는 가벼운 조회 커맨드가, 무거운 GOP 파싱 커맨드가 잡고 있는 동일한 `Mutex`를 기다려야 해서 UI가 이유 없이 멈춘 것처럼 보인다.
- 서로 관련 없는 상태(디코더 vs 썸네일 캐시 vs UI 힌트)가 같은 lock 아래 있으면, 실제 데이터 경쟁이 없는데도 직렬화(serialize)가 강제된다.
- 사용자가 여러 패널을 동시에 열어 병렬로 데이터를 요청해도 backend에서 사실상 순차 처리되어, Tauri의 비동기 커맨드 모델이 갖는 이점이 사라진다.
- 락 경합이 심해지면 어떤 커맨드가 병목인지 특정하기 어렵다 — 모든 커맨드가 "같은 자원을 기다리는 중"으로만 보인다.

**발생 조건**:
- 상태 구조체를 처음 설계할 때 "일단 하나로 묶어두면 편하다"는 이유로 단일 `Mutex<BigStruct>`를 채택한 경우.
- 프레임 탐색이 빈번한 뷰어처럼, 가벼운 조회와 무거운 파싱이 같은 세션 안에서 섞여 호출되는 워크로드.

**권장**:
```rust
struct AppState {
    decoder: Mutex<Decoder>,             // 파싱/디코딩 전용
    thumbnail_cache: RwLock<LruCache<u32, Vec<u8>>>, // 조회 위주 → RwLock
    ui_hints: Mutex<UiHints>,            // 작고 자주 바뀌는 상태는 별도 분리
}

#[tauri::command]
fn get_frame_thumbnail(state: tauri::State<AppState>, frame_index: u32) -> Option<Vec<u8>> {
    state.thumbnail_cache.read().unwrap().get(&frame_index).cloned()
}
```
- 상태를 실제 접근 패턴(읽기 위주/쓰기 위주, 자주 바뀜/드물게 바뀜)에 따라 여러 개의 lock으로 쪼갠다.
- 읽기가 압도적으로 많은 캐시성 상태는 `Mutex` 대신 `RwLock`을 고려한다.
- 무거운 파싱/디코딩 작업은 `tauri::async_runtime::spawn` 등으로 lock을 짧게 잡고 반환하도록 설계해, lock을 잡은 채로 오래 걸리는 연산을 하지 않는다.

**탐지 방법**:
- Structural: `AppState` 정의에서 필드 개수 대비 최상위 `Mutex`/`RwLock` 개수 비율을 스캔 — 필드는 많은데 lock은 하나뿐이면 경고.
- Performance: 커맨드별 lock 대기 시간을 계측(예: `parking_lot`의 계측 기능 또는 자체 wrapper)해 P95 대기 시간이 실제 작업 시간보다 큰 커맨드를 식별.

**예외**:
- 상태 크기가 작고 커맨드 호출 빈도가 낮은 도구(배치 변환 CLI에 가까운 워크플로)라면 단일 lock의 단순함이 분리 비용보다 나을 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-005: command 응답이 거대한 JSON snapshot

**분류**: Query/Snapshot 혼재 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn get_app_snapshot(state: tauri::State<AppState>) -> AppSnapshot {
    // 열린 파일, 전체 프레임 목록, 현재 필터, 모든 오버레이 설정, 재생 상태...
    // 프런트가 "뭔가 하나 바뀔 때마다" 이 커맨드를 통째로 다시 호출한다
    state.build_full_snapshot()
}
```
```typescript
// 필터 하나만 바뀌었는데도 전체 앱 상태를 다시 받는다
async function refreshEverything() {
  const snapshot = await invoke<AppSnapshot>("get_app_snapshot");
  applyEntireStateToStore(snapshot);
}
```

**문제**:
- 커맨드가 "지금 상태 전체"를 매번 되돌려주는 스냅샷 역할과, 특정 질의에 답하는 query 역할을 구분하지 않아, 사소한 변경 하나에도 전체 앱 상태가 재직렬화·재전송된다.
- 프런트 상태 스토어가 `applyEntireStateToStore`처럼 통째로 덮어쓰는 방식에 의존하게 되어, 부분 갱신을 표현할 방법이 없고 리렌더링 범위를 좁힐 수 없다.
- 스냅샷 크기가 세션이 길어질수록(열어본 프레임 수, 오버레이 히스토리 등) 자라날 수 있어 호출 비용이 세션 초반과 후반에 크게 달라진다.
- "무엇이 바뀌어서 다시 불렀는가"라는 정보가 사라지므로, 변경 원인과 무관한 UI(예: 재생 컨트롤)까지 매번 재계산될 위험이 있다.

**발생 조건**:
- "동기화가 안 맞는 것 같으면 그냥 전체를 다시 받아온다"는 방어적 패턴이 반복될 때.
- 세밀한 이벤트/query 설계보다 스냅샷 하나로 모든 것을 해결하려는 초기 아키텍처 선택.

**권장**:
```rust
#[tauri::command]
fn get_filter_state(state: tauri::State<AppState>) -> FilterState { /* 좁은 query */ }

#[tauri::command]
fn get_playback_state(state: tauri::State<AppState>) -> PlaybackState { /* 좁은 query */ }
```
```typescript
// 필터가 바뀔 때만 필터 상태를 다시 조회한다
const filterState = await invoke<FilterState>("get_filter_state");
```
- 스냅샷은 "세션 복구"(TAURI-CMD-006 참고, 예: 앱 재시작 후 마지막 상태 복원) 같은 명확한 목적에 한정하고, 평상시 갱신은 좁은 범위의 query 커맨드로 대체한다.
- 상태가 바뀌었다는 사실 자체는 TAURI_EVT.md에서 다루는 이벤트로 알리고, 프런트는 그 이벤트를 받은 뒤 필요한 좁은 query만 호출한다.

**탐지 방법**:
- Structural: 반환 타입 구조체의 필드 수가 임계값(예: 10개)을 넘고, 여러 호출 사이트에서 필드 중 일부만 실제로 읽히는지(dead field 비율) 정적 분석.
- Runtime: 동일 커맨드 호출 시 응답 payload의 diff 비율(이전 호출과 얼마나 달라졌는가)을 로깅 — diff 비율이 낮게 반복되면 과도한 스냅샷 재조회 신호.

**예외**:
- 앱 시작/파일 열기 직후처럼 "어차피 전체 상태가 다 필요한" 시점의 최초 로드에는 통합 스냅샷이 자연스럽고, 오히려 이를 여러 query로 쪼개면 최초 로드가 왕복(round-trip) 수 증가로 더 느려진다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-006: frontend가 backend 호출 순서를 직접 조립

**분류**: Orchestration 위치 오류 · **심각도**: High · **탐지**: Code

**나쁜 예**:
```typescript
await invoke("open_file");
await invoke("parse_tracks");
await invoke("build_index");
await invoke("load_first_frame");
await invoke("get_syntax");
```

**문제**:
- 중간 실패 시 복구하기 어렵고 네트워크형 왕복이 많아진다.
- "파일을 연다"는 하나의 사용자 의도가 프런트 코드 안에 5단계 절차로 흩어져 있어, 이 순서 자체가 도메인 지식(트랙 파싱 전에는 인덱스를 만들 수 없다 등)인데도 backend가 아니라 프런트 개발자의 머릿속과 호출 코드에만 존재한다.
- 이 순서를 아는 프런트 코드가 여러 진입점(파일 열기 버튼, 최근 파일 목록, 드래그앤드롭)에 중복 구현되기 쉽고, 한 곳에서 순서를 바꾸면 다른 곳은 낡은 순서로 남는다.
- 세 번째 단계(`build_index`)가 실패하면 `open_file`과 `parse_tracks`는 이미 backend에 부수효과를 남긴 상태라, 프런트가 그 부분 실패를 감지하고 되돌리는 로직까지 직접 작성해야 한다(TAURI-CMD-007과 직결).

**발생 조건**:
- backend 함수가 하나씩 만들어질 때마다 프런트가 그때그때 필요한 순서로 이어붙이며 기능을 완성해 온 경우.
- "파일 열기"처럼 여러 단계로 구성된 하나의 의미 있는 사용자 행위가 명시적인 backend API로 승격되지 못한 경우.

**권장**:
```typescript
// 사용자 의도 단위로 하나의 커맨드를 호출한다
const project = await invoke<ProjectHandle>("open_project", { path, options });
```
```rust
#[tauri::command]
async fn open_project(state: tauri::State<'_, AppState>, path: String, options: OpenOptions) -> Result<ProjectHandle, OpenError> {
    // 트랙 파싱 → 인덱스 빌드 → 첫 프레임 로드까지의 순서와 실패 시 롤백을
    // backend 한 곳에서 책임진다. 순서 자체가 도메인 지식이므로 backend가 소유한다.
    let session = state.sessions.start(&path, options).await?;
    Ok(session.handle())
}
```
- 여러 backend 호출이 고정된 순서로 항상 함께 일어난다면, 그 순서는 도메인 지식이므로 backend 안의 단일 커맨드(혹은 세션 시작 절차)로 캡슐화한다.
- 프런트는 "무엇을 하고 싶은가(의도)"만 표현하고, "어떤 순서로 어떻게(절차)"는 backend가 책임진다.

**탐지 방법**:
- Code: 동일한 `invoke` 호출 시퀀스(3개 이상)가 서로 다른 파일/컴포넌트에 중복 등장하는지 정적 검색.
- Manual: 프런트 코드 리뷰 시 "이 호출 순서가 바뀌면 무엇이 깨지는가"를 물었을 때 답이 backend 문서가 아니라 프런트 코드 자체인 경우 이 패턴으로 판단.

**예외**:
- 반대쪽 극단도 안티패턴이다: 하나의 커맨드가 "파일 열기 + 전체 트랙 파싱 + 인덱스 구축 + 초기 분석"까지 모두 끝날 때까지 blocking하도록 만들면, 대용량 파일에서 UI가 그 전체 시간 동안 완전히 응답 불가 상태가 된다. 올바른 절충은 `open_project`가 즉시 세션 핸들(또는 진행 상태를 추적할 수 있는 ID)을 반환하고, 이후 진행 상황은 TAURI_EVT.md가 다루는 이벤트로 스트리밍하거나 `get_open_progress(session_id)` 같은 좁은 query로 폴링하는 구조다. 즉, 순서 조립은 backend가 갖되, 그 실행은 프런트를 막지 않아야 한다.
- 각 단계가 사용자에게 독립적으로 유의미하고(예: "트랙만 먼저 보고 인덱싱은 나중에") 취소·재시도가 단계별로 필요한 디버깅/개발자 도구 성격의 화면이라면, 단계별 커맨드 노출이 오히려 의도적으로 맞는 설계일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-007: 여러 command 중 하나 실패 시 상태가 부분 적용

**분류**: 트랜잭션/일관성 · **심각도**: Critical · **탐지**: Code

**나쁜 예**:
```typescript
async function applyOverlayPreset(preset: OverlayPreset) {
  await invoke("set_qp_overlay_enabled", { enabled: preset.qp });
  await invoke("set_mv_overlay_enabled", { enabled: preset.mv });
  await invoke("set_mb_type_overlay_enabled", { enabled: preset.mbType }); // 여기서 예외 발생 시
  await invoke("set_ref_index_overlay_enabled", { enabled: preset.refIndex }); // 이 줄은 실행 안 됨
}
```

**문제**:
- 세 번째 호출이 실패(예: 아직 로드되지 않은 코덱에서 mb-type 정보 없음)하면 QP/MV 오버레이는 이미 새 preset으로 바뀌었는데 ref-index 오버레이는 이전 값 그대로 남아, backend 상태가 "이전도 아니고 새 preset도 아닌" 제3의 상태가 된다.
- 이 부분 적용 상태는 UI에 명시적으로 드러나지 않아 사용자는 "preset을 적용했다"고 믿지만 실제로는 절반만 적용된 것을 알아채지 못한다.
- 재시도 시 이미 성공한 앞의 두 커맨드를 다시 호출하는 것이 안전한지(멱등성이 있는지) 프런트가 알 방법이 없어, 재시도 로직을 짜기 어렵다.
- 실패 처리를 프런트의 각 `try/catch`에 맡기면, 롤백 로직이 커맨드 호출 순서를 아는 프런트 코드 여러 곳에 중복 구현된다.

**발생 조건**:
- 하나의 의미 있는 상태 전이(preset 적용, 설정 일괄 변경)가 여러 개의 독립적인 setter 커맨드 호출로 쪼개져 있을 때.
- 각 setter가 서로 다른 실패 조건(코덱 미지원, 아직 파싱 안 됨 등)을 가질 수 있는 경우.

**권장**:
```rust
#[tauri::command]
fn apply_overlay_preset(state: tauri::State<AppState>, preset: OverlayPreset) -> Result<OverlayState, ApplyPresetError> {
    let mut overlays = state.overlays.lock().unwrap();
    let previous = overlays.snapshot();
    match overlays.try_apply(&preset) { // backend 내부에서 all-or-nothing으로 적용
        Ok(()) => Ok(overlays.snapshot()),
        Err(e) => {
            overlays.restore(previous); // 실패 시 명시적 롤백
            Err(e)
        }
    }
}
```
- 여러 하위 상태 변경이 논리적으로 하나의 단위라면, 그 단위를 backend 안에서 all-or-nothing으로 처리하는 단일 커맨드로 묶는다.
- 실패 시 backend가 이전 상태로 명시적으로 롤백하고, 프런트에는 "성공 또는 (부분 적용 없이) 실패"만 보이도록 한다.
- 부분 성공이 도메인상 의미가 있는 경우(예: 4개 중 3개 적용 가능)라면, 이를 애매한 예외가 아니라 `PartialApplyResult { applied: [...], skipped: [...] }`처럼 명시적인 반환 타입으로 표현한다.

**탐지 방법**:
- Code: 프런트에서 여러 `invoke` 호출이 순차로 나열되어 있고 그 사이에 `try/catch`나 롤백 호출이 없는 패턴을 정적 스캔.
- Manual: 각 setter 계열 커맨드에 대해 "이 커맨드가 실패하면 앞서 성공한 커맨드는 누가 되돌리는가"를 리뷰 체크리스트로 질문.

**예외**:
- 각 하위 설정이 서로 완전히 독립적이고 사용자가 그중 일부만 적용되어도 명확히 인지·수용 가능한 UI(예: 개별 체크박스에 개별 저장 버튼이 있는 설정 화면)라면 부분 적용이 오히려 자연스러운 설계다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-008: request ID가 없어 stale response 판별 불가

**분류**: 동시성/경쟁 상태 · **심각도**: High · **탐지**: Interaction

**나쁜 예**:
```typescript
async function onFrameScrub(frameIndex: number) {
  const data = await invoke<FrameSummary>("get_frame_summary", { frameIndex });
  setCurrentFrameSummary(data); // 어느 frameIndex에 대한 응답인지 확인 안 함
}
```

**문제**:
- 사용자가 스크럽바를 빠르게 드래그하면 `frameIndex=100`, `101`, `102` 요청이 연달아 나가는데, 각 요청의 backend 처리 시간이 다르면 `100`에 대한 응답이 `102`에 대한 응답보다 늦게 도착할 수 있다.
- 응답에 어떤 요청에 대한 것인지 식별자가 없으므로, 프런트는 "가장 최근에 도착한 응답"을 무조건 최신 상태로 덮어쓰게 되어 화면에는 사용자가 이미 지나친 프레임(`100`)의 정보가 표시된다.
- 이 문제는 네트워크 지연이 불규칙한 것처럼 보이는 로컬 IPC에서도 backend 쪽 lock 경합(TAURI-CMD-004)이나 캐시 미스 여부에 따라 얼마든지 발생할 수 있다.
- 재현이 타이밍에 의존하므로 QA에서 "가끔 스크럽 시 프레임 정보가 한 박자 늦게 보인다"는 식으로만 보고되고 원인을 특정하기 어렵다.

**발생 조건**:
- 사용자가 짧은 시간 안에 같은 커맨드를 다른 파라미터로 여러 번 호출할 수 있는 모든 상호작용(스크럽, 빠른 탭 전환, 타이핑에 따른 검색 등).
- 커맨드 처리 시간이 파라미터(예: 프레임 복잡도, 캐시 여부)에 따라 크게 달라지는 경우.

**권장**:
```typescript
let latestRequestId = 0;

async function onFrameScrub(frameIndex: number) {
  const requestId = ++latestRequestId;
  const data = await invoke<FrameSummary>("get_frame_summary", { frameIndex, requestId });
  if (requestId !== latestRequestId) return; // stale 응답 폐기
  setCurrentFrameSummary(data);
}
```
- 매 호출마다 단조 증가하는 request ID(또는 `AbortController`로 이전 요청 자체를 취소)를 부여하고, 응답이 도착했을 때 그것이 여전히 "가장 최근 요청"인지 확인한 뒤에만 상태를 반영한다.
- 가능하다면 Tauri v2의 `AbortSignal` 지원 등을 활용해 stale 요청을 backend에서도 조기 취소해 불필요한 계산 자체를 줄인다.
- 프레임 인덱스처럼 파라미터 자체가 사실상 식별자 역할을 할 수 있는 경우, 응답에 요청 파라미터를 에코백해 프런트가 스스로 매칭하게 할 수도 있다.

**탐지 방법**:
- Interaction: 스크럽/빠른 연속 클릭 같은 상호작용을 자동화 테스트로 재현하며 인위적으로 backend 응답 지연을 다르게 주입(예: 짝수 프레임은 느리게)해 화면에 오래된 데이터가 표시되는지 확인.
- Code: `invoke` 호출 결과를 그대로 `setState`에 넘기는 위치에서, 호출 시점과 응답 반영 시점 사이에 최신성 검사(request ID 비교, AbortController 등)가 있는지 정적 검토.

**예외**:
- 응답이 항상 멱등이고 순서에 무관하게 최종적으로 옳은 값에 수렴하는 경우(예: 단순 카운터 증가), 또는 호출 빈도가 사람이 낼 수 없는 수준으로 낮아 경쟁이 실질적으로 불가능한 경우에는 생략해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-009: serialization 실패가 일반 backend 오류로 보임

**분류**: 오류 처리/진단성 · **심각도**: Medium · **탐지**: Code

**나쁜 예**:
```rust
#[tauri::command]
fn get_syntax_tree(state: tauri::State<AppState>, frame_index: u32) -> Result<SyntaxTreeSlice, String> {
    let tree = state.decoder.lock().unwrap().build_syntax_tree(frame_index);
    Ok(tree) // f32::NAN, 순환 참조, u64 정밀도 손실 등은 여기서 안 걸리고
             // Tauri IPC 계층의 JSON 직렬화 단계에서야 실패한다
}
```
```typescript
try {
  const tree = await invoke<SyntaxTreeSlice>("get_syntax_tree", { frameIndex });
} catch (e) {
  console.error("Failed to load syntax tree:", e); // "Failed to load"라는 메시지만 보고
  showToast("구문 트리를 불러오지 못했습니다."); // 사용자는 직렬화 문제인지 backend 로직 문제인지 알 수 없음
}
```

**문제**:
- Rust 함수 자체는 `Ok(tree)`를 반환했지만, 그 값이 IPC 경계를 넘어 JSON으로 직렬화되는 단계(예: `f64::NAN`/`Infinity`는 JSON에 없음, `u64`가 JS `Number`의 안전 정수 범위를 벗어남, `HashMap<NonStringKey, _>` 직렬화 불가)에서 실패하면, 프런트는 이를 backend 로직 오류와 구분할 수 없는 동일한 형태의 에러로 받는다.
- 사용자에게는 "구문 트리를 불러오지 못했습니다"라는 메시지만 보이고, 개발자에게도 콘솔에 찍히는 에러가 Tauri/serde 내부 메시지라 어느 필드가 문제인지 즉시 알기 어렵다.
- 이런 실패는 특정 프레임(예: QP가 이례적으로 NaN이 되는 코덱 파싱 버그, 또는 4K 이상에서 오프셋 값이 `u64` 안전 정수 범위를 넘는 경우)에서만 재현되어 디버깅이 오래 걸린다.
- IPC.md가 다루는 "직렬화 형식 자체의 낭비" 문제와는 다른 축의 문제로, 여기서는 형식이 아니라 "실패했을 때 그것이 직렬화 단계였다는 사실"이 진단 정보에서 사라지는 것이 핵심이다.

**발생 조건**:
- 부동소수점(NaN/Infinity 가능성이 있는 계산 결과)이나 64비트 정수를 그대로 직렬화 대상 구조체 필드로 쓸 때.
- backend 타입을 바꿀 때(예: `u32` → `u64`) 프런트/직렬화 경계에 미치는 영향을 검토하지 않는 경우.

**권장**:
```rust
#[derive(serde::Serialize)]
struct SyntaxTreeSlice {
    #[serde(serialize_with = "finite_f32")]
    qp_avg: f32,
    byte_offset: String, // u64는 문자열로 내보내 JS Number 정밀도 손실 방지
}

fn finite_f32<S: serde::Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    if v.is_finite() { s.serialize_f32(*v) } else { s.serialize_none()?; Err(serde::ser::Error::custom("non-finite f32")) }
}
```
```typescript
try {
  const tree = await invoke<SyntaxTreeSlice>("get_syntax_tree", { frameIndex });
} catch (e) {
  if (isSerializationError(e)) {
    logDiagnostic("syntax_tree_serialization_failed", { frameIndex, raw: e });
    showToast("이 프레임의 데이터 형식에 문제가 있습니다 (진단 정보 전송됨).");
  } else {
    showToast("구문 트리를 불러오지 못했습니다.");
  }
}
```
- 직렬화 위험이 있는 값(NaN/Infinity 가능한 float, JS 안전 정수 범위를 벗어나는 u64/i64)은 커스텀 시리얼라이저나 문자열 인코딩으로 방어하고, 방어 자체가 실패를 명시적인 에러로 표면화하게 한다.
- Rust 쪽 도메인 오류(`Result::Err`)와 IPC 계층 직렬화 오류를 프런트에서 구분할 수 있도록 오류 타입이나 코드 접두사를 다르게 설계한다.
- 개발 빌드에서는 커맨드 반환 직전에 직렬화를 한 번 시도해보는 assert/테스트를 추가해 프로덕션에서 처음 발견되는 일을 줄인다.

**탐지 방법**:
- Code: 커맨드 반환 타입에 `f32`/`f64`/`u64`/`i64` 필드가 그대로 노출되어 있는지, 커스텀 시리얼라이저나 범위 검증 없이 직렬화되는지 정적 스캔.
- Manual: 의도적으로 NaN이나 `u64::MAX` 근처 값을 만들어내는 fixture로 각 커맨드를 호출해보고, 실패 메시지가 "직렬화 실패"임을 프런트까지 식별 가능한 형태로 전달하는지 수동 검증.

**예외**:
- 모든 수치 필드가 이미 안전한 범위(예: `u16` 이하, 항상 유한한 계산 결과)로 제한되어 있음이 타입 시스템 수준에서 보장되는 경우에는 별도 방어 계층이 불필요할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-CMD-010: command latency를 측정하지 않음

**분류**: 관측성 부재 · **심각도**: Medium · **탐지**: Performance

**나쁜 예**:
```rust
#[tauri::command]
fn get_frame_summary(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary {
    state.decoder.lock().unwrap().summarize(frame_index) // 소요 시간 계측 없음
}
```
```typescript
// 프런트도 호출 소요 시간을 기록하지 않는다
const summary = await invoke<FrameSummary>("get_frame_summary", { frameIndex });
```

**문제**:
- "스크럽할 때 가끔 버벅인다"는 사용자 보고가 들어와도, 어느 커맨드가 느렸는지, 얼마나 느렸는지, 특정 프레임/코덱/파일 크기에서만 그런지 재구성할 데이터가 전혀 없다.
- lock 대기(TAURI-CMD-004)인지, 직렬화 비용(IPC.md 영역)인지, 도메인 계산 자체가 느린지 구분할 방법이 없어, 문제가 보고될 때마다 코드를 처음부터 다시 추적해야 한다.
- 회귀(regression)를 조기에 발견할 수 없다 — 어떤 리팩터링이 특정 커맨드를 2배 느리게 만들었어도, 그 사실이 드러나는 시점은 사용자가 체감하고 불평할 때뿐이다.
- 최적화 우선순위를 정할 근거(어느 커맨드가 호출 빈도 × 지연시간 기준으로 가장 큰 비용인지)가 없어 직관에 의존해 튜닝하게 된다.

**발생 조건**:
- 커맨드 수가 늘어나 서로 다른 개발자가 서로 다른 시점에 추가하면서, 계측을 넣는 관례가 애초에 없었던 경우.
- "느린 것 같다"는 정성적 인상은 있지만 이를 뒷받침할 수치가 없는 프로젝트 초중반 단계.

**권장**:
```rust
#[tauri::command]
fn get_frame_summary(state: tauri::State<AppState>, frame_index: u32) -> FrameSummary {
    let _timer = metrics::command_timer("get_frame_summary"); // scope 종료 시 자동 기록
    state.decoder.lock().unwrap().summarize(frame_index)
}
```
```typescript
async function timedInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const start = performance.now();
  try {
    return await invoke<T>(cmd, args);
  } finally {
    recordIpcLatency(cmd, performance.now() - start); // 프런트 체감 왕복 시간(직렬화+IPC 포함)
  }
}
```
- backend 커맨드 진입/종료 시점에 경량 타이머를 두어 커맨드 이름별 처리 시간을 수집한다(운영 오버헤드가 걱정되면 샘플링).
- 프런트에서도 `invoke` 왕복 전체(직렬화 포함) 시간을 별도로 측정해, backend 내부 시간과 IPC 오버헤드를 구분할 수 있게 한다.
- 수집한 지표를 개발 빌드에서 간단한 오버레이(HUD)나 로그로 노출해, 최적화 전후 비교가 가능하게 한다.

**탐지 방법**:
- Structural: `#[tauri::command]` 함수 목록과 계측 코드(타이머 매크로/래퍼 호출) 존재 여부를 대조해 계측 커버리지 비율을 산출.
- Performance: 계측이 있는 커맨드에 대해 P50/P95/P99 지연시간을 대시보드로 추적하고, 배포/리팩터링 전후로 비교.

**예외**:
- 호출 빈도가 극히 낮고(예: 앱 시작 시 한 번뿐인 초기화 커맨드) 이미 명백히 빠른 커맨드까지 일괄 계측하는 것은 코드 잡음 대비 이득이 적을 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

## 원칙

이 카테고리 전체를 관통하는 기준은 다음 네 가지 역할을 섞지 않는 것이다.

- **Command** = 의도를 전달 (사용자가 무엇을 하고 싶어 하는가 — backend가 절차와 원자성을 책임진다)
- **Query** = 현재 상태 조회 (지금 이 값이 무엇인가 — 좁고, 자주 불려도 싸다)
- **Event** = 변화가 있었음을 알림 (무엇이 바뀌었는가 — TAURI_EVT.md 참고)
- **Snapshot** = 유실된 상태 복구 (다시 처음부터 무엇이었는지 알아야 할 때만 — 평상시 갱신 수단이 아니다)

TAURI-CMD-001·006·007은 Command의 경계와 원자성을, TAURI-CMD-002·003은 Command 안에 다른 역할(presentation, 화면 구조)이 섞이는 문제를, TAURI-CMD-004·010은 그 실행이 얼마나 비싸고 관측 가능한지를, TAURI-CMD-005는 Query와 Snapshot의 혼동을, TAURI-CMD-008·009는 응답이 도착했을 때 그것을 신뢰할 수 있는 근거(최신성, 형식 무결성)를 다룬다. 새로운 커맨드를 추가하기 전에 "이것은 네 가지 중 정확히 무엇인가"를 먼저 답하면 이 문서의 항목 대부분을 사전에 피할 수 있다.
