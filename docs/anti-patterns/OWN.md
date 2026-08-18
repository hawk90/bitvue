# Anti-Pattern Catalog — OWN: Rust 소유권·수명·복사

이 문서는 Bitvue(Tauri + Rust + React 기반 비디오 비트스트림 분석기) 안티패턴 카탈로그의 일부입니다. 전체 목록과 카테고리 구성은 `docs/anti-patterns/INDEX.md`(별도 작성 예정)를 참고하십시오. 1단계(본 문서)는 일반적·도메인 특화 참조 카탈로그이며, 실제 저장소에 대한 감사와 판정은 2단계에서 수행합니다.

---

### OWN-001: 불필요한 clone으로 ownership 문제 회피
**분류**: OWN · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn find_slice_headers(nalus: &[NalUnit]) -> Vec<NalUnit> {
    let mut result = Vec::new();
    for nalu in nalus {
        if nalu.nal_type == NalType::SliceHeader {
            // 컴파일러가 borrow 오류를 내자 그냥 clone으로 회피
            result.push(nalu.clone());
        }
    }
    result
}
```

**문제**:
- 대여 검사기 오류를 진짜 원인 분석 없이 `.clone()`으로 "일단 컴파일되게" 만드는 습관이 굳어짐
- `NalUnit`이 payload `Vec<u8>`를 포함하면 NAL 하나마다 수백 바이트~수 KB가 복사됨
- 코드 리뷰에서 "왜 여기서 clone이 필요한가"라는 질문에 답할 수 없는 경우가 대부분 → 실제로는 불필요
- 프레임 단위로 반복되면 프로파일러의 malloc/memcpy 비중이 비정상적으로 커짐

**발생 조건**:
- 대형 NAL 유닛 리스트, syntax element 배열, 프레임별 MV(motion vector) 테이블을 순회하며 필터링/집계할 때
- 대여 검사기 오류 메시지를 그대로 따라가며 clone을 추가하는 "컴파일 통과 우선" 개발 습관

**권장**:
```rust
fn find_slice_headers(nalus: &[NalUnit]) -> Vec<&NalUnit> {
    nalus.iter().filter(|n| n.nal_type == NalType::SliceHeader).collect()
}
```
- 반환 타입을 참조(`&T`) 또는 인덱스로 바꿀 수 있는지 먼저 검토
- 정말 소유권 이전이 필요한 경우에만 clone하고, 주석으로 이유를 남긴다
- `clippy::redundant_clone` 활성화

**탐지 방법**:
- `cargo clippy -- -W clippy::redundant_clone -W clippy::clone_on_copy`
- grep: `\.clone\(\)` 발생 빈도를 파일별로 집계해 상위 파일 리뷰
- 코드 리뷰 체크리스트에 "이 clone이 없으면 컴파일이 안 되는가, 아니면 안 되길래 넣었는가" 질문 추가

**예외**:
- 실제로 두 스코프가 동시에 독립적으로 데이터를 소유해야 하는 경우(예: undo 스냅샷, diff 비교용 이전 프레임 보관)
- 데이터가 작고(수십 바이트 이하) 핫패스가 아니라면 가독성을 위해 clone이 더 나을 수 있음

**Bitvue 판정**: N/A — 원래 근거였던 `src-tauri/src/services/decode_service.rs`는 2026-08-08 Tauri 폐기 커밋(`e7194cc`)으로 저장소에서 완전히 삭제됨. 현재 픽셀 디코드 경로(`crates/bitvue-sidecar/src/decode_bridge.rs`)는 디코더 세션 캐싱 자체가 없다고 모듈 doc에 명시("no decoder-session caching")되어 있어 캐시 히트발 이중 clone이 구조적으로 발생 불가. 후속 캐시 계층(`CachedFrame`, crates/bitvue-engine/src/stream_state.rs:658-694)의 `rgb()`/`yuv()` 접근자는 단일 clone만 수행하며 호출부 자체가 grep상 0건(죽은 코드) — 이중 clone 인스턴스를 찾지 못함. 관련된 대형 버퍼 clone 위험은 OWN-013으로 재분류.

---

### OWN-002: Arc 남용
**분류**: OWN · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct DecoderContext {
    sps: Arc<SequenceParameterSet>,
    pps: Arc<PictureParameterSet>,
    frame_width: Arc<u32>,      // 절대 필요 없음
    codec_name: Arc<String>,    // &str 나 Copy 타입으로 충분
}
```

**문제**:
- `Arc<u32>`처럼 `Copy` 가능한 작은 타입까지 `Arc`로 감싸면 atomic refcount 증감 비용만 추가되고 얻는 것이 없음
- 실제로는 단일 스레드/단일 소유 구조인데 "나중에 멀티스레드가 될 수도 있으니" 미리 `Arc`를 씌우는 방어적 설계
- 코드를 읽는 사람이 "이 값이 여러 스레드에서 공유되는가?"를 판단하기 어려워짐(모든 것이 Arc면 신호가 사라짐)
- `Arc<Mutex<...>>`가 아니라 `Arc<T>` 단독이면 내부 가변성이 없다는 뜻인데, 이 구분이 흐려짐

**발생 조건**:
- 프로젝트 초기에 "동시성 안전하게" 설계하겠다는 목표로 모든 구조체 필드를 습관적으로 Arc로 감쌀 때
- 디코더 컨텍스트, 파서 상태 등 실제로는 단일 스레드에서만 쓰이는 값에 Arc를 붙일 때

**권장**:
```rust
struct DecoderContext {
    sps: Arc<SequenceParameterSet>, // 여러 프레임/스레드가 참조 공유 → Arc 타당
    pps: Arc<PictureParameterSet>,
    frame_width: u32,               // Copy 타입은 그냥 값으로
    codec_name: String,             // 단일 소유면 String, 공유 필요시에만 Arc<str>
}
```
- "이 값이 실제로 여러 소유자/스레드 간에 공유되는가?"에 Yes일 때만 Arc 사용
- `Copy` 타입(`u8/u16/u32/u64/f32/bool` 등)은 절대 Arc로 감싸지 않는다
- 공유가 필요 없어지면 리팩터링으로 Arc를 제거하는 것도 관리 대상으로 취급

**탐지 방법**:
- grep: `Arc<u8>|Arc<u16>|Arc<u32>|Arc<u64>|Arc<bool>|Arc<f32>|Arc<f64>` — Copy 타입을 감싼 Arc는 거의 항상 안티패턴
- `cargo clippy -- -W clippy::arc_with_non_send_sync` (내부 타입이 Send/Sync가 아닌데 Arc로 감싼 경우 탐지)
- 구조체별 필드 중 `Arc<T>` 비율이 비정상적으로 높은 파일을 구조적으로 스캔

**예외**:
- 실제로 여러 스레드(예: 병렬 프레임 디코딩 워커 풀)나 여러 장기 소유자가 동일 데이터를 참조해야 하는 경우
- 향후 병렬화가 설계 문서에 명시된 로드맵이고, 이번 스프린트에 도입 예정이라면 선제 도입도 허용 가능(단 팀 합의 필요)

**Bitvue 판정**: N/A — `Arc<u8>|Arc<u16>|Arc<u32>|Arc<u64>|Arc<bool>|Arc<f32>|Arc<f64>` grep 전수 검색 결과 0건. `Core`(crates/bitvue-engine/src/core.rs:43-55, `bitvue-core`에서 `c2a0e44`로 개명)의 `Arc<RwLock<StreamState>>` 필드들은 `get_stream()`(225행)으로 sidecar의 요청별 워커 스레드(`bitvue-sidecar/src/main.rs`의 `spawn_request`)에 실제로 clone되어 동시 접근되므로 예외 조건(진짜 다중 소유자)에 해당.

---

### OWN-003: Arc<Mutex<T>>를 기본 공유 모델로 사용
**분류**: OWN · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct AppState {
    current_frame: Arc<Mutex<Option<DecodedFrame>>>,
    playback_position: Arc<Mutex<u64>>,
    parse_cache: Arc<Mutex<HashMap<u64, ParsedUnit>>>,
}

#[tauri::command]
fn get_playback_position(state: tauri::State<AppState>) -> u64 {
    *state.playback_position.lock().unwrap()
}
```

**문제**:
- 단일 락 하나면 충분한 곳에 필드마다 개별 `Arc<Mutex<>>`를 씌우면 락 획득 순서가 흩어져 데드락 위험 증가
- `.unwrap()`으로 락 poisoning을 무시하는 습관이 함께 따라오는 경우가 많음(패널 하나가 panic하면 이후 모든 접근이 연쇄 panic)
- 실제로는 GUI 이벤트 루프/Tauri 커맨드가 순차적으로 호출되는 경우가 많아 진짜 동시 접근이 없는데도 락 오버헤드를 감수
- `Mutex`를 프레임 버퍼처럼 큰 데이터에 걸면 락을 쥔 채로 memcpy가 일어나 다른 스레드가 오래 대기

**발생 조건**:
- Tauri의 `State<T>`를 스레드 안전하게 만들어야 한다는 요구 때문에 모든 공유 상태를 기계적으로 `Arc<Mutex<>>`로 감쌀 때
- 디코더 워커 스레드와 UI 이벤트 루프가 상태를 주고받아야 하는 구조에서 세밀한 락 설계 없이 필드 단위로 락을 남발할 때

**권장**:
```rust
struct AppState {
    // 관련 있는 상태를 하나의 락으로 묶어 락 순서 문제를 원천 차단
    inner: Mutex<AppStateInner>,
}

struct AppStateInner {
    current_frame: Option<DecodedFrame>,
    playback_position: u64,
    parse_cache: HashMap<u64, ParsedUnit>,
}

#[tauri::command]
fn get_playback_position(state: tauri::State<AppState>) -> Result<u64, String> {
    let guard = state.inner.lock().map_err(|_| "state poisoned".to_string())?;
    Ok(guard.playback_position)
}
```
- Tauri `State<T>` 자체가 이미 앱 전역에서 공유되므로 내부에 다시 `Arc`를 씌울 필요는 보통 없음(`Arc` 없이 `Mutex<T>`만으로 충분)
- 관련 필드를 하나의 구조체로 묶어 락 개수를 줄이고 락 순서 문제를 없앤다
- `.lock().unwrap()` 대신 poison 처리(`unwrap_or_else(|e| e.into_inner())` 등)를 명시적으로 결정
- 진짜 병렬 접근이 필요 없다면 `RefCell`이나 단순 소유 이동으로 충분한지 재검토

**탐지 방법**:
- grep: `Arc<Mutex<` 발생 개수와 구조체당 락 필드 개수 집계 — 한 구조체에 락이 3개 이상이면 리뷰 대상
- grep: `.lock().unwrap()` — poison 처리를 무시하는 지점 전수 조사
- Runtime: 락 경합 프로파일링(`parking_lot`의 `deadlock_detection` 기능 또는 `tokio-console`)으로 실제 경합 여부 확인

**예외**:
- 정말 독립적인 여러 백그라운드 워커(디코딩, VMAF 계산, 파일 인덱싱)가 서로 다른 자원을 동시에 갱신해야 하는 경우, 필드별 락 분리가 오히려 병렬성을 높일 수 있음
- 락 범위가 명확히 분리되어 있고 락 순서 규칙이 문서화되어 있다면 다중 락도 정당화 가능

**Bitvue 판정**: Confirmed — 원 근거였던 Tauri `AppState`(src-tauri/src/commands/mod.rs)는 `e7194cc`로 삭제됐으나 동일 패턴이 현재 코드에도 남아있음: `IndexSession`(crates/bitvue-engine/src/index_session.rs:63-79, `bitvue-core`에서 개명)이 state/quick_index/full_index/evidence_manager 4개 필드를 각각 별도 `Arc<Mutex<>>`로 감싸 필드 단위 락을 남발(권장안의 통합 `Mutex<Inner>` 대신). 신규 sidecar 계층(`bitvue-sidecar/src/main.rs:96-108`)도 `CancelRegistry`/`DebugYuvSlot`/`writer`를 각각 독립 `Arc<Mutex<>>`로 top-level 분리하는 동일 계열 패턴이나, 이쪽은 서로 무관한 자원(취소 레지스트리·직렬 stdout 쓰기·디버그 세션)이라 예외 조건("독립적인 여러 워커가 서로 다른 자원을 동시에 갱신")에 더 부합해 Confirmed 근거에서 제외.

---

### OWN-004: Cow를 넣었지만 항상 Owned가 되는 구조
**분류**: OWN · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn normalize_profile_string(raw: &str) -> Cow<'_, str> {
    // 항상 새 String을 만들어 반환 — Cow를 쓴 의미가 없음
    Cow::Owned(raw.trim().to_lowercase())
}

fn get_codec_tag<'a>(data: &'a [u8]) -> Cow<'a, str> {
    let s = String::from_utf8_lossy(data); // 이미 Cow<str> 반환
    Cow::Owned(s.into_owned())              // 굳이 다시 Owned로 강제 변환
}
```

**문제**:
- `Cow`를 시그니처에 넣으면 호출부는 "빌린 경우도 있으니 값을 오래 들고 있으면 안 되겠다"는 기대를 하게 되는데, 실제로는 항상 `Owned`라 그 기대가 거짓
- `to_lowercase()`처럼 애초에 항상 할당하는 연산을 `Cow`로 감싸는 것은 `String`을 반환하는 것보다 API가 복잡해지기만 함
- `String::from_utf8_lossy`가 이미 `Cow<str>`를 반환하는데 이를 다시 풀었다가 강제로 `Owned`로 만드는 이중 작업
- 호출자가 "이 함수는 자주 borrowed 경로를 타니 저렴하겠지"라고 오판하고 핫루프에서 반복 호출 → 매번 할당 발생

**발생 조건**:
- "일단 유연하게 만들어두자"는 생각으로 문자열/버퍼 반환 타입에 습관적으로 `Cow`를 붙일 때
- 실제 분기(빌린 경로 vs 새로 만든 경로)가 있는지 확인하지 않고 API를 설계할 때

**권장**:
```rust
fn normalize_profile_string(raw: &str) -> String {
    raw.trim().to_lowercase()
}

fn get_codec_tag(data: &[u8]) -> Cow<'_, str> {
    // 실제로 non-UTF8일 때만 Owned, 대부분은 Borrowed — Cow가 의미를 가짐
    String::from_utf8_lossy(data)
}
```
- `Cow`를 쓰기 전에 "이 함수가 입력을 그대로 반환하는 경로가 실제로 존재하는가"를 먼저 확인
- 항상 변형/할당이 일어난다면 그냥 소유 타입(`String`, `Vec<u8>`)을 반환
- `Cow` 사용 시 두 분기(Borrowed/Owned)를 테스트로 모두 커버

**탐지 방법**:
- Static: 함수 본문에서 `Cow::Borrowed`가 등장하지 않고 `Cow::Owned`만 있는 함수를 grep으로 탐색
- 코드 리뷰: `-> Cow<'_, T>` 시그니처가 있는 함수마다 "Borrowed 분기가 실제로 있는가?" 체크
- `cargo clippy -- -W clippy::ptr_arg`류 규칙과 함께 API 설계 리뷰에서 수동 확인(자동 탐지가 어려운 항목이라 Manual 비중 높음)

**예외**:
- 지금은 항상 Owned지만, 캐싱/interning 레이어를 곧 추가할 계획이 명확히 있고 API 안정성을 위해 미리 Cow를 노출해두는 경우(단, TODO로 근거를 남길 것)

**Bitvue 판정**: N/A — `crates/bitvue-formats/src/mp4.rs`의 `extract_av1_samples`/`extract_avc_samples`/`extract_hevc_samples`(189-360행 부근)는 실제로 원본 버퍼를 빌리는 `Cow::Borrowed` 경로와 emulation-prevention 제거가 필요한 `Cow::Owned` 경로가 모두 존재하는 정상적인 zero-copy 설계로, 안티패턴이 지적하는 "항상 Owned" 상황이 아님.

---

### OWN-005: 참조 수명을 피하려고 전부 String/Vec로 소유
**분류**: OWN · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct SyntaxElement {
    name: String,       // 항상 정적 문자열 리터럴에서 옴 ("nal_unit_type" 등)
    raw_bits: Vec<u8>,  // 원본 비트스트림 버퍼의 부분 복사본
}

fn parse_nal_header(nalu: &[u8]) -> Vec<SyntaxElement> {
    let mut elements = Vec::new();
    elements.push(SyntaxElement {
        name: "nal_unit_type".to_string(),   // &'static str이면 충분
        raw_bits: nalu[0..1].to_vec(),        // 슬라이스로 충분한데 복사
    });
    elements
}
```

**문제**:
- 대여 검사기와 씨름하기 싫어서 모든 필드를 `String`/`Vec<u8>`로 만들면, 파서가 syntax tree 하나를 만들 때마다 필드 수만큼 힙 할당이 발생
- 필드 이름처럼 사실상 상수인 값까지 `String`으로 만들면 `&'static str`이면 됐을 자리에 할당·해제 비용을 지불
- 원본 비트스트림 버퍼(보통 mmap)에 대한 슬라이스로 충분한 `raw_bits`를 매번 `to_vec()`으로 복사하면 mmap의 zero-copy 이점이 사라짐
- 트리 노드 수가 많은 코덱(HEVC CTU/CU 트리, AV1 partition tree)에서는 이 비용이 프레임당 수만 번 반복됨

**발생 조건**:
- syntax tree, NAL 파싱 결과처럼 원본 버퍼의 부분 구간을 가리키기만 하면 되는 구조에서, 수명 파라미터 설계가 번거로워 전부 소유 타입으로 바꿀 때
- 파서 초기 프로토타입에서 "일단 동작하게" 만들려고 소유 타입을 쓰고, 이후 최적화를 미룰 때

**권장**:
```rust
struct SyntaxElement<'a> {
    name: &'static str,   // 상수 문자열은 리터럴 참조로
    raw_bits: &'a [u8],   // 원본 버퍼를 빌린다 — zero-copy
}

fn parse_nal_header<'a>(nalu: &'a [u8]) -> Vec<SyntaxElement<'a>> {
    vec![SyntaxElement { name: "nal_unit_type", raw_bits: &nalu[0..1] }]
}
```
- 수명 파라미터 하나(`'a`)로 대부분의 파서 결과 구조체를 커버할 수 있는 경우가 많음 — 무조건 소유로 도피하기 전에 시도
- 상수/식별자 문자열은 `&'static str`
- 원본 버퍼가 mmap이고 파싱 결과의 생존 기간이 버퍼보다 짧다면 슬라이스 참조가 안전하고 저렴

**탐지 방법**:
- grep: 파서 결과 구조체 정의에서 `String`/`Vec<u8>` 필드 비율이 높은 타입을 구조적으로 나열
- Manual: 해당 구조체 인스턴스의 생존 기간이 원본 버퍼(mmap/패킷 버퍼)의 생존 기간보다 짧은지 확인 — 짧다면 참조로 전환 가능
- 벤치마크로 파싱 함수의 할당 횟수(예: `dhat`/`heaptrack`)를 측정해 필드 수 대비 비정상적으로 높은 할당 수를 가진 파서를 표시

**예외**:
- 파싱 결과가 원본 버퍼보다 오래 살아야 하는 경우(예: UI에 표시하기 위해 캐시에 저장, 비동기 채널로 다른 스레드에 전달)에는 소유 타입이 정답
- 수명 파라미터가 3개 이상 얽혀 API가 실제로 쓰기 어려워지는 경우는 OWN-015 참고

**Bitvue 판정**: Suspected — `SyntaxNode`(crates/bitvue-hevc/src/syntax/mod.rs:9-22, bitvue-vp9/bitvue-vvc 동일 구조)의 `name: String`은 상수 문자열임에도 `&'static str`이 아니지만, 이 타입에 `Serialize`/`Deserialize`가 붙어 있어 Tauri IPC로 프론트엔드에 전달되는 구조이므로 owned 타입이 실제로 필요할 수 있음(OWN-005의 예외 조건과 부합) — 습관적 소유 회피인지 IPC 요구사항 때문인지 히스토리 없이는 단정 불가.

---

### OWN-006: 대형 구조체를 값으로 반복 이동
**분류**: OWN · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
#[derive(Clone)]
struct DecodedFrame {
    width: u32,
    height: u32,
    y_plane: Vec<u8>,   // 수 MB
    u_plane: Vec<u8>,
    v_plane: Vec<u8>,
    metadata: FrameMetadata,
}

fn apply_deblock_filter(mut frame: DecodedFrame) -> DecodedFrame {
    // frame을 값으로 받고 값으로 반환 — 매 호출마다 구조체 전체가 move
    // Vec 필드 자체는 move라 얕은 복사지만, 체인이 길어지면
    // 컴파일러 최적화가 깨져 실제 memcpy가 삽입되는 경우가 흔하다
    frame.metadata.filtered = true;
    frame
}

fn pipeline(frame: DecodedFrame) -> DecodedFrame {
    let f = apply_deblock_filter(frame);
    let f = apply_sao_filter(f);
    let f = apply_color_convert(f);
    f
}
```

**문제**:
- `Vec` 필드의 move 자체는 포인터 3개(ptr/len/cap)만 옮기므로 이론적으로는 저렴하지만, 구조체가 커지고(예: 여러 plane + 메타데이터 + 통계) 함수 인라인이 실패하면 스택 복사(memcpy)가 실제로 발생
- 함수 체인이 길어질수록(필터 파이프라인) 각 단계에서 구조체 전체가 스택을 오가며 디버그 빌드에서는 특히 느려짐
- 값으로 주고받는 스타일이 굳어지면 나중에 필드를 추가할 때마다(예: HDR 메타데이터 추가) 모든 이동 비용이 조용히 커짐 — 아무도 알아채지 못함
- `#[derive(Clone)]`이 함께 붙어 있으면 실수로 `.clone()`을 호출했을 때 수 MB가 복사되는데 타입 시그니처만 봐서는 위험성이 드러나지 않음

**발생 조건**:
- 필터/변환 파이프라인을 함수형 스타일로 짤 때 "불변성이 좋으니까"라며 대형 프레임 구조체를 값으로 계속 주고받을 때
- 디버그 빌드나 최적화가 제한된 빌드(예: `opt-level=1`인 dev-release 하이브리드)에서 인라인이 실패해 실제 복사가 발생할 때

**권장**:
```rust
fn apply_deblock_filter(frame: &mut DecodedFrame) {
    frame.metadata.filtered = true;
}

fn pipeline(frame: &mut DecodedFrame) {
    apply_deblock_filter(frame);
    apply_sao_filter(frame);
    apply_color_convert(frame);
}
```
- 파이프라인 단계마다 소유권을 주고받을 필요가 없다면 `&mut T`로 제자리 수정
- 정말 소유권 이전이 필요한 API(예: 다른 스레드로 전달)에는 `Box<DecodedFrame>`을 고려해 스택 복사를 힙 포인터 이동으로 대체
- 벤치마크로 파이프라인 전체의 memcpy 총량을 측정해 회귀를 감시

**탐지 방법**:
- Runtime: `perf stat -e cache-misses`나 `valgrind --tool=cachegrind`로 필터 파이프라인의 메모리 이동량 측정
- Static: 함수 시그니처에서 대형 구조체(필드에 `Vec<u8>` plane 3개 이상 포함)를 값으로 받고 값으로 반환하는 함수를 나열
- `cargo clippy -- -W clippy::large_types_passed_by_value`

**예외**:
- 구조체가 실제로 작고(수십 바이트 이하) `Copy`가 아니더라도 이동 비용이 무시할 만한 수준이면 값 전달이 더 읽기 쉬움
- 소유권 이전 자체가 의미론적으로 중요한 경우(예: 프레임을 큐에 넣고 더 이상 로컬에서 쓰지 않음을 타입으로 보장)는 값 전달이 올바른 설계

**Bitvue 판정**: N/A — `DecodedFrame`을 값으로 받아 값으로 반환하는 함수(`fn foo(x: DecodedFrame ...)` 형태)를 grep으로 찾지 못함; 필터/파이프라인성 함수들은 참조 또는 `Arc<[u8]>`/`Arc<Vec<u8>>`로 감싼 plane 데이터를 사용해 대형 구조체 값 이동 자체가 관찰되지 않음.

---

### OWN-007: clone-on-write가 실제로는 clone-always
**분류**: OWN · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct FrameCache {
    entries: HashMap<u64, Rc<DecodedFrame>>,
}

impl FrameCache {
    fn get_mut(&mut self, key: u64) -> Option<&mut DecodedFrame> {
        let rc = self.entries.get_mut(&key)?;
        // "공유 중이 아니면" 그냥 쓰고, 공유 중이면 clone해야 하는데
        // 항상 Rc::make_mut를 호출 → refcount가 1이어도 안전 검사 비용 + 종종 2 이상이라 매번 clone
        let owned = Rc::make_mut(rc);
        Some(owned)
    }
}
```

**문제**:
- 이름은 "copy-on-write"인데 실제 사용 패턴에서 refcount가 거의 항상 1보다 크면(캐시라서 UI와 디코더가 동시에 참조) `Rc::make_mut`가 매번 실질적으로 clone을 수행 — CoW의 이점이 전혀 없음
- 얼마나 자주 실제 clone이 발생하는지 로그/메트릭이 없으면 "우리는 CoW를 쓰니까 효율적"이라는 잘못된 믿음이 유지됨
- `Arc`/`Rc` + `make_mut` 패턴은 대형 프레임 버퍼에 쓰면 clone 발생 시점의 비용이 매우 큰데, 그 시점이 예측 불가능해 프레임 드랍이 간헐적으로 발생

**발생 조건**:
- 프레임 캐시, 파싱 결과 캐시처럼 "읽기는 여럿, 쓰기는 드묾"을 가정하고 `Rc<T>`/`Arc<T>` + `make_mut`를 도입했지만, 실제로는 UI 패널이 프레임을 계속 들고 있어 refcount가 항상 2 이상인 경우
- CoW 도입 시 실제 refcount 분포를 측정하지 않고 설계만으로 이점을 가정할 때

**권장**:
```rust
// 실측 결과 refcount가 거의 항상 >1이라면, 애초에
// "읽기 전용 공유 뷰"와 "쓰기용 소유 버퍼"를 API 레벨에서 분리한다
struct FrameCache {
    entries: HashMap<u64, Arc<DecodedFrame>>, // 조회는 Arc<DecodedFrame> (읽기 전용)
}

fn mutate_frame(original: &DecodedFrame) -> DecodedFrame {
    // 수정이 필요하면 명시적으로 새 프레임을 만든다 — 비용이 코드에 드러남
    let mut copy = original.clone();
    copy.metadata.filtered = true;
    copy
}
```
- `Rc::strong_count`/`Arc::strong_count`를 계측해 실제 CoW 적중률을 확인한 뒤 패턴 채택 여부 결정
- 적중률이 낮다면 CoW를 걷어내고 "읽기는 공유 참조, 쓰기는 명시적 새 값 생성"으로 단순화하는 편이 예측 가능
- 비용이 큰 clone이 발생하는 지점을 로그로 남겨 회귀 감시

**탐지 방법**:
- Runtime: `Rc::strong_count`를 주기적으로 샘플링하는 디버그 계측 추가 후 실제 분포 확인
- grep: `Rc::make_mut|Arc::make_mut` 호출부를 모두 찾아 각각의 refcount 가정을 리뷰
- 프로파일러에서 `make_mut` 경로의 memcpy 비중이 예상보다 크면 CoW가 사실상 무력화된 신호

**예외**:
- 실측 결과 refcount가 대부분 1이고 드물게만 2 이상이 되는 워크로드(예: 짧게 스냅샷을 참조하는 undo 기능)에서는 CoW가 실제로 유효

**Bitvue 판정**: N/A — 앱 코드 전역에서 `Rc<RefCell<`/`Rc::make_mut`/`Arc::make_mut` 사용례 없음(유일한 `make_mut` 호출은 crates/vendor/abseil/src/absl_utility/identity.rs:101로 서드파티 벤더 유틸리티이며 Bitvue 도메인 캐시와 무관).

---

### OWN-008: API 경계마다 to_owned
**분류**: OWN · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
mod parser {
    pub fn extract_sps_id(nalu: &[u8]) -> String {
        // 내부적으로는 &str이면 충분한데 모듈 경계를 넘을 때마다 습관적으로 owned 반환
        format!("{}", nalu[3] & 0x1F)
    }
}

mod ui_bridge {
    pub fn label_for_nalu(nalu: &[u8]) -> String {
        let id = super::parser::extract_sps_id(nalu);
        id.to_owned() // 이미 String인데 또 to_owned — 무의미한 복사
    }
}
```

**문제**:
- "모듈 경계를 넘을 때는 항상 소유 타입으로 변환해야 안전하다"는 근거 없는 습관이 함수 체인 곳곳에 중복 할당을 만듦
- 이미 소유 타입(`String`)인 값을 다시 `.to_owned()`/`.clone()`하는 코드가 리뷰에서 지적되지 않고 누적됨
- 계층이 3~4단(파서 → 브리지 → Tauri 커맨드 → 프론트엔드 직렬화)을 거치는 구조에서 각 층마다 한 번씩 복사가 붙으면 계층 수만큼 할당이 선형으로 증가

**발생 조건**:
- 여러 모듈 계층을 거치는 파이프라인(파서 → 어댑터 → Tauri 커맨드)에서 각 계층 작성자가 서로 다른 시점에 독립적으로 "안전하게 owned로" 습관을 적용할 때
- 코드 리뷰에서 "이미 owned인데 왜 또 owned로 만드나"를 체크하지 않을 때

**권장**:
```rust
mod ui_bridge {
    pub fn label_for_nalu(nalu: &[u8]) -> String {
        super::parser::extract_sps_id(nalu) // 이미 String이므로 그대로 반환
    }
}
```
- 값을 넘겨받은 시점에 이미 원하는 소유 타입이면 추가 변환 없이 그대로 전달
- 함수 시그니처를 보고 "여기서 정말 새로운 소유가 필요한가"를 매 계층마다 질문
- 계층이 많은 파이프라인은 데이터 흐름을 문서화해 어느 계층에서 소유권이 정해지는지 한 곳에 정리

**탐지 방법**:
- grep: 반환값이 이미 `String`/`Vec<T>`인 함수 호출 직후에 `.to_owned()` 또는 `.clone()`이 붙는 패턴
- `cargo clippy -- -W clippy::redundant_clone`이 상당수 잡아줌
- 코드 리뷰에서 계층 간 데이터 전달 다이어그램을 그려보고 중복 변환 지점을 표시

**예외**:
- 각 계층이 서로 다른 생명주기를 가지며 원본을 실제로 변형해야 하는 경우(예: 원본은 캐시에 유지, 브리지 계층은 변형된 복사본을 프론트로 전송)

**Bitvue 판정**: N/A — 비테스트 앱 코드에서 `.to_owned()` 호출 0건(grep 전수 검색). 계층 간 중복 owned 변환 패턴이 관찰되지 않음.

---

### OWN-009: callback에 'static을 맞추기 위한 과도한 Arc
**분류**: OWN · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn spawn_decode_worker(window: tauri::Window, frame_source: FrameSource) {
    // frame_source가 참조를 담고 있어 'static이 안 되니 통째로 Arc로 감싸 강제로 맞춤
    let frame_source = Arc::new(frame_source);
    let fs = frame_source.clone();
    std::thread::spawn(move || {
        loop {
            let frame = fs.next_frame();
            let _ = window.emit("frame-decoded", frame.id);
        }
    });
    // frame_source 원본은 이 함수를 벗어나면 더 이상 안 쓰이는데도
    // Arc로 감싼 탓에 소유권 모델이 다음 유지보수자에게 불명확해짐
}
```

**문제**:
- `thread::spawn`이 요구하는 `'static` 경계를 맞추기 위해, 원래는 단일 소유로 충분한 값을 습관적으로 `Arc`로 감쌈
- 이 패턴이 반복되면 "왜 Arc인가"에 대한 답이 전부 "'static을 맞추려고"가 되어 실제 공유 의도가 있는 Arc와 구분이 안 됨
- 원본 값이 클로저로 move된 이후 원래 스코프에서는 더 이상 필요 없는데도 `Arc`의 존재가 "다른 곳에서도 참조 중일 수 있다"는 오해를 유발해 라이프사이클 추론을 어렵게 함

**발생 조건**:
- `std::thread::spawn`, `tokio::spawn`, Tauri 이벤트 콜백처럼 `'static` 클로저를 요구하는 API에 참조를 담은 값을 넘기려 할 때
- 스레드/태스크가 실제로는 하나만 그 값을 쓰는데도 컴파일 오류를 없애려고 `Arc`부터 도입할 때

**권장**:
```rust
fn spawn_decode_worker(window: tauri::Window, frame_source: FrameSource) {
    // 다른 소유자가 없다면 그냥 move — Arc/clone 불필요
    std::thread::spawn(move || {
        let frame_source = frame_source;
        loop {
            let frame = frame_source.next_frame();
            let _ = window.emit("frame-decoded", frame.id);
        }
    });
}
```
- 값을 클로저로 그냥 `move`할 수 있는지 먼저 확인 — 단일 소유자면 `Arc` 없이도 `'static` 경계를 만족
- 정말 원본 스코프와 새 스레드가 동시에 같은 값을 참조해야 할 때만 `Arc` 도입
- `tauri::Window`, `AppHandle`처럼 프레임워크가 이미 내부적으로 `Clone`+공유를 지원하는 핸들 타입은 그대로 clone(저렴한 핸들 clone임)해도 무방 — 이 경우는 예외에 해당

**탐지 방법**:
- Static: `thread::spawn`/`tokio::spawn` 직전에 `Arc::new`가 나타나는 패턴을 grep, 이후 그 Arc가 clone되어 여러 곳에서 쓰이는지 확인
- Manual: Arc로 감싼 값이 클로저 밖에서 실제로도 참조되는지(진짜 공유) 아니면 클로저 안에서만 쓰이는지(가짜 공유) 확인

**예외**:
- `tauri::AppHandle`, `Window` 등 프레임워크 제공 핸들은 원래 저렴한 clone을 의도한 타입이므로 그대로 사용
- 여러 워커 스레드가 동일 소스를 정말로 동시에 읽어야 하는 경우

**Bitvue 판정**: N/A — `crates/bitvue-engine/src/worker.rs:255`의 `Arc<Mutex<[StreamQueue; 2]>>`은 제출 스레드와 워커 스레드가 실제로 동시에 참조하는 진짜 공유 상태. src-tauri 삭제 후 `thread::spawn`은 `bitvue-sidecar/src/main.rs:196`(`spawn_request`, 요청마다 워커 스레드 생성)과 2405행(동시성 테스트)에 존재하지만, 여기서 clone되는 `Arc<Core>`/`Arc<Mutex<io::Stdout>>`/`CancelRegistry`는 여러 in-flight 요청 스레드가 실제로 동시에 접근해야 하는 정당한 공유 상태(모듈 doc이 이 동시성 모델을 명시적으로 문서화)이며, 단일 소유값을 `'static` 경계 때문에 방어적으로 Arc로 감싼 사례는 확인되지 않음.

---

### OWN-010: self-referential 구조를 억지로 흉내 내기
**분류**: OWN · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
struct ParsedBitstream {
    raw: Vec<u8>,
    // raw 내부를 가리키는 슬라이스를 같은 구조체 필드로 두고 싶지만
    // 러스트는 이를 직접 표현할 수 없으므로 raw pointer로 흉내
    nalu_view: *const [u8], // raw를 가리키는 댕글링 위험 포인터
}

impl ParsedBitstream {
    fn new(raw: Vec<u8>) -> Self {
        let nalu_view = &raw[4..20] as *const [u8];
        ParsedBitstream { raw, nalu_view } // raw가 이동하면 nalu_view는 무효화될 수 있음
    }

    unsafe fn view(&self) -> &[u8] {
        &*self.nalu_view // raw가 move된 적이 있다면 UB
    }
}
```

**문제**:
- `Vec<u8>`가 이동(move)하면 힙 버퍼 자체 주소는 바뀌지 않지만, 구조체가 재배치되거나 raw가 재할당(예: `push`로 growth)되면 저장해둔 포인터가 즉시 댕글링
- `unsafe`로 대여 검사기를 우회했기 때문에 컴파일러가 더 이상 유효성을 보장하지 않고, 버그가 발생해도 컴파일 시점에 잡히지 않음
- self-referential 구조를 raw pointer로 흉내 내는 코드는 리뷰어가 안전성을 증명하기 매우 어렵고, 대부분의 경우 실제로 안전하지 않음
- 이런 타입은 `Unpin`을 잃거나 이동 시 UB를 유발할 수 있어 조용히 크래시하거나 더 나쁘게는 조용히 잘못된 데이터를 반환

**발생 조건**:
- 원본 버퍼와 그 버퍼를 가리키는 파싱 뷰를 "한 구조체 안에" 담고 싶다는 욕심에서 self-referential 구조를 시도할 때
- `ouroboros`, `self_cell` 같은 검증된 crate를 모르거나 의존성 추가를 피하려고 직접 raw pointer로 구현할 때

**권장**:
```rust
// 방법 1: 인덱스/오프셋으로 표현 — self-reference를 완전히 회피
struct ParsedBitstream {
    raw: Vec<u8>,
    nalu_range: std::ops::Range<usize>, // 포인터 대신 오프셋
}

impl ParsedBitstream {
    fn view(&self) -> &[u8] {
        &self.raw[self.nalu_range.clone()]
    }
}

// 방법 2: 수명 파라미터로 원본과 뷰를 분리된 타입으로 유지
struct BitstreamView<'a> {
    nalu: &'a [u8],
}
```
- 대부분의 "self-referential처럼 보이는" 문제는 포인터 대신 인덱스/오프셋(`Range<usize>`)으로 표현하면 완전히 해소됨
- 정말 self-referential이 필요하다면 직접 `unsafe`를 짜지 말고 `ouroboros`/`self_cell` 등 검증된 crate 사용
- 원본과 뷰를 애초에 분리된 타입(부모 소유 구조체 + 자식 수명 참조 구조체)으로 설계해 소유자가 뷰보다 먼저 사라지지 않음을 타입으로 보장

**탐지 방법**:
- grep: 구조체 필드에 `*const`/`*mut` 원시 포인터가 있고, 같은 구조체의 다른 필드(예: `Vec`/`String`)를 가리키도록 생성자에서 초기화되는 패턴
- Static: `unsafe impl` 또는 필드 초기화에서 `&self.field as *const _` 형태로 자기 자신을 참조하는 코드 검색
- Manual/Semantic: Miri(`cargo miri test`)로 실행해 UB(use-after-move, dangling pointer) 탐지

**예외**:
- 검증된 crate(`ouroboros`, `self_cell`, `yoke`) 위에서 구현되어 안전성이 라이브러리 수준에서 보장되는 경우
- 오프셋/인덱스 기반 설계가 성능상 불가능한 극히 예외적인 경우(거의 없음)에 한해 신중히 리뷰된 `unsafe` 블록으로 구현하고 Miri 테스트를 필수로 동반

**Bitvue 판정**: N/A — 앱 코드의 원시 포인터(`*const`/`*mut`) 필드는 모두 `crates/bitvue-decode/src/vvdec.rs`의 `mod ffi` 블록 내 C 구조체 미러(VvdecAccessUnit, VvdecPlane 등)뿐이며, 같은 구조체의 다른 필드를 자기참조하는 패턴은 발견되지 않음.

---

### OWN-011: 인덱스 기반 arena로 충분한데 Rc<RefCell<>> 트리 사용
**분류**: OWN · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct SyntaxNode {
    name: String,
    children: Vec<Rc<RefCell<SyntaxNode>>>,
    parent: Option<Weak<RefCell<SyntaxNode>>>,
}

fn add_child(parent: &Rc<RefCell<SyntaxNode>>, child: SyntaxNode) {
    let child_rc = Rc::new(RefCell::new(child));
    child_rc.borrow_mut().parent = Some(Rc::downgrade(parent));
    parent.borrow_mut().children.push(child_rc);
}
```

**문제**:
- HEVC CTU/CU 트리, AV1 partition tree처럼 노드 수가 프레임당 수천~수만 개인 트리에서 노드마다 `Rc`(힙 할당 + refcount) + `RefCell`(런타임 borrow 체크)을 쓰면 할당 개수와 캐시 미스가 폭증
- `RefCell::borrow_mut()`가 런타임에 이미 대여 중인 노드를 다시 빌리면 panic — 트리 순회 로직이 복잡해질수록 이 위험이 커지고 컴파일 타임에 잡히지 않음
- 트리 전체를 순회할 때마다 포인터를 따라가야 해서(포인터 체이싱) 캐시 지역성이 나쁨 — 배열 기반 arena라면 순차 접근으로 훨씬 빠름
- `Weak` 참조를 부모에 두더라도 사이클/누수 관리가 여전히 수동이라 실수하기 쉬움(OWN-022 참고)

**발생 조건**:
- 트리/그래프 구조를 다른 언어(Java/Python)의 습관대로 "노드가 노드를 참조"하는 방식으로 직역 이식할 때
- 파서가 만드는 syntax tree, partition tree, MV 예측 그래프처럼 노드 수가 많고 생애 주기가 짧은(프레임당 재생성) 구조에 객체지향적 참조 그래프를 그대로 적용할 때

**권장**:
```rust
struct SyntaxTree {
    nodes: Vec<SyntaxNodeData>,
}

struct SyntaxNodeData {
    name: &'static str,
    children: Vec<u32>,  // arena 인덱스
    parent: Option<u32>,
}

impl SyntaxTree {
    fn add_child(&mut self, parent: u32, data: SyntaxNodeData) -> u32 {
        let idx = self.nodes.len() as u32;
        self.nodes.push(data);
        self.nodes[parent as usize].children.push(idx);
        idx
    }
}
```
- 트리/그래프는 `Vec<Node>` + 인덱스(arena pattern)로 표현하면 `Rc`/`RefCell`/`Weak` 없이도 부모-자식-형제 참조를 자유롭게 표현 가능
- arena 전체를 한 번에 `Vec`으로 재사용(프레임마다 `clear()` 후 재사용)하면 할당 자체를 없앨 수 있음
- 순회가 배열 인덱스 기반이라 캐시 친화적이고, 대여 검사기가 컴파일 타임에 대부분의 실수를 잡아줌

**탐지 방법**:
- grep: `Rc<RefCell<` 또는 `Rc<Cell<`가 트리/그래프성 구조체(자기 자신 타입을 필드로 갖는 구조체) 정의에 등장하는지 검색
- Runtime: 프레임당 할당 횟수를 `dhat`/`heaptrack`으로 측정해 트리 노드 수와 비례하는 할당 스파이크 확인
- 코드 리뷰: "이 트리가 노드 단위로 개별 소유권 이전/공유가 필요한가, 아니면 통째로 한 번에 생성·폐기되는가"를 질문 — 후자면 arena가 적합

**예외**:
- 트리 노드가 정말로 서로 다른 생애주기를 가지고 개별적으로 공유/해제되어야 하는 드문 경우(예: 플러그인 시스템에서 노드를 외부에 장기 대여)
- 노드 수가 적고(수십 개 이하) 트리가 프레임마다 재생성되지 않는 정적 설정 트리라면 Rc/RefCell의 비용이 무시할 만함

**Bitvue 판정**: N/A — HEVC/VP9/VVC `SyntaxNode`, AV1 `PartitionNode`, `UnitNode`(crates/bitvue-engine/src/stream_state.rs:197-242, `bitvue-core`에서 개명) 등 트리 구조가 전부 `children: Vec<Self>` 형태의 단순 소유 재귀 구조를 사용하며 `Rc<RefCell<>>` 기반 트리는 코드베이스에 없음(벤더 abseil의 제네릭 `identity.rs`만 예외이며 도메인 트리와 무관).

---

### OWN-012: FFI 경계에서 잘못된 참조 수명 가정
**분류**: OWN · **심각도**: Critical · **탐지**: Manual

**나쁜 예**:
```rust
extern "C" {
    fn dav1d_get_picture(ctx: *mut Dav1dContext, out: *mut Dav1dPicture) -> i32;
}

fn decode_frame(ctx: *mut Dav1dContext) -> &'static [u8] {
    unsafe {
        let mut pic: Dav1dPicture = std::mem::zeroed();
        dav1d_get_picture(ctx, &mut pic);
        // pic.data[0]는 dav1d 내부 버퍼를 가리키는 포인터인데
        // dav1d_picture_unref가 호출되면 즉시 무효화됨.
        // 그런데 'static으로 반환해 "영원히 유효하다"고 러스트에 거짓말을 함
        std::slice::from_raw_parts(pic.data[0] as *const u8, pic.stride[0] as usize)
    }
}
```

**문제**:
- FFI로 받은 포인터의 실제 유효 기간은 C 라이브러리(dav1d/libvmaf)의 내부 refcount 규칙에 의해 결정되는데, 이를 러스트 수명으로 정직하게 표현하지 않고 `'static`으로 캐스팅하면 대여 검사기가 더 이상 도와줄 수 없음
- `dav1d_picture_unref` 호출 이후에도 슬라이스가 살아있다고 타입 시스템이 믿게 되어, 해제된 메모리를 읽는 use-after-free가 컴파일러 경고 없이 발생
- 이런 버그는 대부분 디버그 빌드에서는 우연히 살아남은 메모리 때문에 재현되지 않다가, 릴리즈 빌드나 압박 상황(다른 프레임 디코딩으로 버퍼 재사용)에서만 터짐

**발생 조건**:
- dav1d, libvmaf 등 C FFI가 반환하는 포인터를 러스트 슬라이스/참조로 감쌀 때, 실제 소유권·해제 시점(언제 `_unref`/`_free`를 호출해야 하는지)을 정확히 추적하지 않을 때
- "컴파일이 안 되니 일단 `'static`으로 캐스팅"하는 임시방편이 그대로 남을 때

**권장**:
```rust
struct Dav1dFrame<'ctx> {
    pic: Dav1dPicture,
    _marker: std::marker::PhantomData<&'ctx Dav1dContext>,
}

impl<'ctx> Dav1dFrame<'ctx> {
    fn data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.pic.data[0] as *const u8, self.pic.stride[0] as usize)
        }
    }
}

impl Drop for Dav1dFrame<'_> {
    fn drop(&mut self) {
        unsafe { dav1d_picture_unref(&mut self.pic) };
    }
}
```
- FFI 리소스는 RAII 래퍼 타입으로 감싸 `Drop`에서 반드시 해제 함수를 호출하도록 강제
- 수명 파라미터를 컨텍스트에 묶어(`'ctx`) 컨텍스트보다 프레임이 오래 살 수 없음을 타입으로 표현
- 반환 슬라이스는 래퍼의 메서드를 통해서만 얻게 하여 `'static`으로 탈출할 길을 원천 차단

**탐지 방법**:
- grep: `unsafe`, `extern "C"`, `as *const`, `'static` 캐스팅이 FFI 함수 반환값 처리부에 함께 등장하는 패턴
- Manual: FFI 함수마다 대응하는 `_free`/`_unref`/`_close` 호출이 `Drop` 구현으로 보장되는지 페어링 확인
- Runtime: ASan/Miri(가능한 범위 내) 또는 valgrind로 FFI 경계 use-after-free를 실행 시점에 탐지
- 코드 리뷰 체크리스트: "이 포인터의 유효 기간을 결정하는 C 라이브러리 문서/헤더를 확인했는가"

**예외**:
- FFI 라이브러리가 명시적으로 데이터를 복사해 반환하고(`out` 파라미터가 호출자 소유 버퍼) 별도 해제가 필요 없다고 문서화된 경우
- 프로세스 전체 생애주기 동안 해제되지 않는 정말로 정적인 리소스(예: 코덱 이름 상수 테이블)라면 `'static`이 정당

**Bitvue 판정**: N/A — `crates/bitvue-decode/src/vvdec.rs:494-549`의 `convert_frame`은 FFI 포인터 유효성 요구사항을 주석으로 상세히 문서화하고 `plane_utils::extract_plane()`으로 즉시 owned `Vec<u8>`로 복사한 뒤에만 반환(raw slice가 함수 밖으로 탈출하지 않음); dav1d 경로는 safe wrapper crate(`dav1d::Picture`)를 통해서만 접근하며 `'static` 캐스팅이 발견되지 않음.

---

### OWN-013: 대형 버퍼를 포함한 타입에 무심코 derive(Clone)
**분류**: OWN · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Clone, Debug)]
struct DecodedFrame {
    width: u32,
    height: u32,
    y_plane: Vec<u8>,   // 1920x1080 기준 약 2MB
    u_plane: Vec<u8>,   // 약 0.5MB
    v_plane: Vec<u8>,   // 약 0.5MB
}

// 다른 파일, 다른 개발자가 몇 달 뒤 작성
fn cache_recent_frames(history: &mut Vec<DecodedFrame>, frame: &DecodedFrame) {
    history.push(frame.clone()); // 시그니처만 보면 "그냥 clone"이지 3MB 복사인지 알 수 없음
}
```

**문제**:
- `#[derive(Clone)]`은 타입 시그니처에 "clone이 저렴하다"는 어떠한 신호도 남기지 않으므로, 몇 달 뒤 다른 개발자가 무심코 `.clone()`을 호출하면 프레임당 수 MB가 복사됨
- 이런 clone은 코드 리뷰에서 `.clone()` 한 줄로만 보이기 때문에 실제 비용이 숨겨짐 — grep으로 찾기 전까지는 아무도 인지하지 못함
- 재생 루프처럼 프레임마다 반복 실행되는 경로에 이런 clone이 하나라도 섞이면 프레임 드랍의 원인이 되는데, 원인 추적이 어려움(프로파일러 없이는 "왜 느린지" 코드만 봐서는 안 보임)

**발생 조건**:
- 구조체를 처음 정의할 때 습관적으로 `#[derive(Debug, Clone)]`을 붙이는 템플릿을 그대로 사용
- 이후 필드에 대형 버퍼(`Vec<u8>` 여러 개)가 추가되었는데 derive는 그대로 남아있을 때
- 이 타입이 캐시, 히스토리, undo 스택 등 "여러 개를 저장"하는 컨테이너에 들어갈 때

**권장**:
```rust
#[derive(Debug)] // Clone을 derive하지 않음 — 실수로 값 복사를 못 하게 원천 차단
struct DecodedFrame {
    width: u32,
    height: u32,
    y_plane: Vec<u8>,
    u_plane: Vec<u8>,
    v_plane: Vec<u8>,
}

impl DecodedFrame {
    /// 명시적 이름으로 비용을 코드에 드러낸다.
    fn deep_copy(&self) -> Self {
        DecodedFrame {
            width: self.width,
            height: self.height,
            y_plane: self.y_plane.clone(),
            u_plane: self.u_plane.clone(),
            v_plane: self.v_plane.clone(),
        }
    }
}
```
- 대형 버퍼를 포함한 타입은 `Clone`을 derive하지 말고, 정말 필요하면 `deep_copy()`처럼 비용을 이름에 드러내는 명시적 메서드 제공
- 여러 곳에서 공유해야 한다면 clone 대신 `Arc<DecodedFrame>`으로 감싸 참조 공유
- `Clone`이 꼭 필요한 트레이트 바운드(예: 제네릭 캐시 컨테이너 요구사항) 때문이라면, 그 바운드 자체가 대형 타입에 적절한지 재검토

**탐지 방법**:
- grep: `#[derive(` 라인에 `Clone`이 포함된 구조체 중 `Vec<u8>`/`Box<[u8]>` 필드가 2개 이상인 타입을 구조적으로 나열
- `cargo clippy -- -W clippy::large_enum_variant`는 열거형에는 도움이 되지만 구조체 clone 비용은 별도로 수동 검토 필요
- Runtime: 힙 프로파일러(`dhat`)에서 `DecodedFrame::clone` 심볼의 총 할당 바이트를 확인해 임계치 초과 시 경고

**예외**:
- 타입이 실제로 작거나(참조/핸들만 포함), clone이 드물게(예: 앱 시작 시 1회) 일어나는 것이 확실한 경우
- 테스트 코드에서만 사용되는 타입이라 성능이 문제되지 않는 경우 — 이 경우 `#[cfg(test)]` 전용 `Clone` impl로 분리하는 것도 방법

**Bitvue 판정**: Confirmed — 원 근거(src-tauri decode_service.rs)는 삭제됐으나 동일 패턴이 현재 `CachedFrame`(crates/bitvue-engine/src/stream_state.rs:657-689)에서 뒤집힌 형태로 재현됨: `#[derive(Debug, Clone)]`이 붙어 있고 `y_plane`/`u_plane`/`v_plane`은 doc comment대로 "Arc-wrapped for cheap cloning"(679-685행, `Arc<Vec<u8>>`)인데 `rgb_data: Vec<u8>`(661-662행)만 Arc로 감싸지 않은 원본 그대로 남아있음 — 1920x1080 RGB8 기준 프레임당 약 6MB. 이 타입을 담는 `FrameModel`(576-585행, 32프레임 LRU)도 수동 `Clone` 구현(634-648행)에서 캐시 항목 전체(최대 32개)를 순회하며 `v.clone()`으로 복제해 위험이 증폭되고, `StreamState` 자체도 `#[derive(Clone)]`(53행). 다만 `CachedFrame::rgb()`/`FrameModel`/`StreamState`를 실제로 clone하는 호출부는 grep상 발견되지 않아(rgb()/yuv() 접근자 자체가 무호출 죽은 코드) 현재는 구조적 위험(타입 정의 자체가 안티패턴)이지 실측된 핫패스 clone은 아님.

---

### OWN-014: 핫패스 파서 함수가 빌린 뷰 대신 소유 데이터를 반환
**분류**: OWN · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn read_slice_data(bitstream: &[u8], offset: usize, len: usize) -> Vec<u8> {
    // 원본 버퍼(mmap)에서 그냥 슬라이스하면 되는데 매번 새로 할당해서 복사
    bitstream[offset..offset + len].to_vec()
}

fn decode_all_slices(bitstream: &[u8], slice_offsets: &[(usize, usize)]) -> Vec<Vec<u8>> {
    slice_offsets.iter()
        .map(|&(off, len)| read_slice_data(bitstream, off, len))
        .collect()
}
```

**문제**:
- 프레임 하나에 슬라이스가 수십~수백 개 있는 코덱(특히 저지연 스트리밍 프로파일)에서 매 슬라이스마다 `to_vec()`을 호출하면 프레임당 할당 횟수가 슬라이스 수에 비례해 증가
- 원본 버퍼가 mmap이라면 이미 페이지 캐시에 매핑되어 zero-copy로 읽을 수 있는데, 굳이 힙에 복사본을 만들어 mmap의 핵심 이점(페이지 폴트만으로 로드, 복사 없음)을 스스로 포기
- 이 패턴은 함수 시그니처(`-> Vec<u8>`)만 봐서는 "당연히 그런가보다" 싶어 리뷰에서 잘 걸러지지 않음

**발생 조건**:
- 파서 함수를 처음 작성할 때 대여 수명을 다루기 귀찮아서 반환 타입을 소유 타입으로 잡고, 이후 호출부가 늘어나면서 굳어질 때
- 슬라이스/서브레인지 추출처럼 본질적으로 "뷰"만 있으면 되는 연산에 소유 반환 타입을 습관적으로 쓸 때

**권장**:
```rust
fn read_slice_data<'a>(bitstream: &'a [u8], offset: usize, len: usize) -> &'a [u8] {
    &bitstream[offset..offset + len]
}

fn decode_all_slices<'a>(bitstream: &'a [u8], slice_offsets: &[(usize, usize)]) -> Vec<&'a [u8]> {
    slice_offsets.iter()
        .map(|&(off, len)| read_slice_data(bitstream, off, len))
        .collect()
}
```
- 파서의 1차 결과(슬라이스 경계, syntax element 위치)는 가능한 한 원본 버퍼에 대한 뷰(`&[u8]`)로 유지
- 소유 데이터가 필요한 시점(예: 다른 스레드로 전달, 원본 버퍼 해제 이후에도 유지)까지 소유화를 미룬다 — "늦은 소유(late ownership)" 원칙
- 벤치마크로 슬라이스 추출 경로의 할당 횟수를 프레임당 0에 가깝게 유지하는 것을 목표로 설정

**탐지 방법**:
- grep: 파서 계층 함수 중 반환 타입이 `Vec<u8>`/`String`이면서 함수 본문이 `[..].to_vec()` 또는 `String::from_utf8(...)` 한 줄로 끝나는 패턴(뷰로 대체 가능한 신호)
- Runtime: 프레임 디코딩 경로를 `dhat`으로 프로파일링해 슬라이스 수 대비 할당 수 비율 확인 — 비율이 1:1에 가까우면 뷰 미사용 신호

**예외**:
- 슬라이스가 이후 재정렬/병합/디코딩(예: emulation prevention byte 제거, RBSP 변환)이 필요해 어차피 새 버퍼를 만들어야 하는 경우는 소유 반환이 자연스러움
- 반환값이 캐시에 장기 저장되어 원본 mmap보다 오래 살아야 하는 경우

**Bitvue 판정**: Suspected — 파서 핫패스의 `.to_vec()` 호출은 전체적으로 10건 정도로 제한적이며(bitvue-avc/vp9/hevc/vvc/av1-codec), 대부분 emulation-prevention 제거나 NalUnit 캐시 보존처럼 예외 조건에 해당하는 것으로 보이나(예: crates/bitvue-avc/src/nal.rs:280), 모든 지점을 개별적으로 "필요한 소유화"인지 "습관적 회피"인지 확정하지는 못함.

---

### OWN-015: 수명 파라미터 폭발로 인해 결국 전부 owned로 도피
**분류**: OWN · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
// 처음엔 이렇게 시작했지만...
struct ParseContext<'buf, 'sps, 'pps, 'vps> {
    bitstream: &'buf [u8],
    sps: &'sps SequenceParameterSet,
    pps: &'pps PictureParameterSet,
    vps: &'vps VideoParameterSet,
}

struct SliceHeader<'buf, 'sps, 'pps, 'vps, 'ctx> {
    ctx: &'ctx ParseContext<'buf, 'sps, 'pps, 'vps>,
    // 함수를 하나 추가할 때마다 수명 파라미터가 전파되어 시그니처가 통제 불능이 됨
}

// 결국 참다못해 팀 전체가 이렇게 "포기"
struct SliceHeaderOwned {
    bitstream: Vec<u8>,
    sps: SequenceParameterSet,
    pps: PictureParameterSet,
    vps: VideoParameterSet,
}
```

**문제**:
- 수명 파라미터가 3~4개 이상 얽히면 함수 시그니처가 읽기/쓰기 모두 어려워지고, 새 필드를 추가할 때마다 수명 전파 범위가 넓어짐
- 이 복잡도에 지친 팀이 "그냥 다 owned로 만들자"는 결정을 내리면, 원래 참조로 충분했던 곳까지 전부 복사가 발생해 OWN-005와 동일한 성능 문제로 귀결
- 진짜 문제(수명 설계가 과도하게 세분화됨)를 해결하지 않고 증상(컴파일 오류)만 없앤 것이라, 이후 비슷한 구조를 만들 때 같은 패턴이 반복됨

**발생 조건**:
- SPS/PPS/VPS처럼 서로 다른 스코프에서 오는 참조를 하나의 파싱 컨텍스트에 모두 참조로 담으려 할 때
- 파서 구조를 지나치게 세밀하게 쪼개(레이어마다 별도 구조체) 각 레이어가 상위 레이어의 여러 참조를 모두 전달받아야 할 때

**권장**:
```rust
// 서로 다른 수명을 하나로 묶는다 — 실제로 모두 같은 "파싱 세션" 동안만 유효하면 충분
struct ParseContext<'a> {
    bitstream: &'a [u8],
    sps: &'a SequenceParameterSet,
    pps: &'a PictureParameterSet,
    vps: &'a VideoParameterSet,
}

struct SliceHeader<'a> {
    ctx: &'a ParseContext<'a>,
}
```
- 실제로 서로 다른 시점에 해제되지 않는 참조들은 수명 파라미터를 하나로 통합(`'a`)해도 안전한 경우가 대부분 — 진짜로 독립적인 수명이 필요한지 먼저 검증
- 그래도 시그니처가 복잡하면 참조들을 하나의 "핸들" 구조체로 묶어 파라미터 개수를 줄임
- 부분적으로만 소유화(예: 자주 바뀌지 않는 SPS/PPS/VPS는 `Arc`로 공유 소유, 매 프레임 바뀌는 bitstream만 참조로 유지)하는 절충안도 고려

**탐지 방법**:
- Static: 구조체/함수 시그니처에 수명 파라미터가 3개 이상 선언된 경우를 grep(`<'[a-z]+,\s*'[a-z]+,\s*'[a-z]+`)으로 탐색
- Manual/Semantic: git blame으로 "참조 구조 → 전부 owned 구조"로 리팩터링된 커밋 이력을 찾아, 그 리팩터링이 수명 복잡도 회피 때문이었는지 커밋 메시지/PR 논의에서 확인

**예외**:
- 수명이 실제로 서로 다른 시점에 해제되어야 하는 경우(예: 장기 캐시된 SPS/PPS와 매 호출마다 새로 오는 bitstream)는 파라미터 분리가 정당
- 라이브러리의 공개 API로 노출되어 호출자에게 유연성을 줘야 하는 경우, 약간의 복잡도를 감수하고 참조 기반 API를 유지하는 것이 나을 수 있음

**Bitvue 판정**: N/A — 수명 파라미터 3개 이상이 함께 선언된 구조체/함수를 grep(`<'[a-z]+,\s*'[a-z]+,\s*'[a-z]+`)으로 찾지 못함; 파서 컨텍스트 구조체들이 SPS/PPS/VPS를 참조 대신 값으로 들고 다니는 경우가 있으나 이는 수명 폭발 회피의 흔적이라기보다 구조 자체가 처음부터 owned로 설계된 것으로 보임(별도 git blame 확인 없이는 단정 불가).

---

### OWN-016: Arc<str>/Arc<[u8]> vs Arc<String>/Arc<Vec<u8>> 혼동
**분류**: OWN · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
struct CodecInfo {
    name: Arc<String>,     // 이중 간접 참조: Arc -> String -> heap buffer
    profile_bytes: Arc<Vec<u8>>, // Arc -> Vec -> heap buffer
}

fn make_codec_info(name: &str, profile: &[u8]) -> CodecInfo {
    CodecInfo {
        name: Arc::new(name.to_string()),
        profile_bytes: Arc::new(profile.to_vec()),
    }
}
```

**문제**:
- `Arc<String>`은 `Arc` 헤더(포인터+refcount) 뒤에 다시 `String`(포인터+len+cap)이 있고 그 뒤에야 실제 바이트가 있는 이중 간접 참조 구조 — `Arc<str>`이면 `Arc` 헤더 뒤에 바로 바이트가 오는 단일 할당 구조로 축소 가능
- 이 값을 clone할 때마다 `String`/`Vec`의 포인터+len+cap 3워드를 함께 복사하는데, `Arc<str>`/`Arc<[u8]>`이면 fat pointer 하나(ptr+len)만 복사하면 됨
- 메모리 지역성도 나빠짐: `Arc<String>`은 refcount와 실제 데이터가 서로 다른 캐시라인에 있을 가능성이 `Arc<str>`보다 큼(할당이 2번 나뉘므로)
- 코덱 이름, 프로파일 문자열처럼 immutable하고 자주 공유·clone되는 값에는 특히 이 차이가 누적됨

**발생 조건**:
- "문자열은 String, 바이트는 Vec<u8>"이라는 기본 반사작용으로 `Arc`를 씌울 때도 내부 타입을 그대로 String/Vec로 두는 경우
- 값이 생성된 이후 절대 변경되지 않는(append/mutate가 없는) 불변 공유 데이터인데도 가변 가능한 컨테이너(String/Vec)를 그대로 감쌀 때

**권장**:
```rust
struct CodecInfo {
    name: Arc<str>,          // 단일 할당, fat pointer clone
    profile_bytes: Arc<[u8]>,
}

fn make_codec_info(name: &str, profile: &[u8]) -> CodecInfo {
    CodecInfo {
        name: Arc::from(name),
        profile_bytes: Arc::from(profile),
    }
}
```
- 생성 이후 절대 변경되지 않는 공유 문자열/바이트열은 `Arc<str>`/`Arc<[u8]>`로 선언
- `String::into()`/`Arc::from(&str)` 등으로 변환은 간단하며, 대신 이후 mutate가 불가능해짐(이는 오히려 불변성을 타입으로 보장하는 장점)
- 이미 변경 가능성이 있는(예: 나중에 append) 필드라면 `Arc<Mutex<String>>` 등으로 별도 설계 필요 — 이 최적화는 순수 불변 공유 데이터에만 적용

**탐지 방법**:
- grep: `Arc<String>|Arc<Vec<u8>>|Rc<String>|Rc<Vec<u8>>` 패턴 검색 — 대부분 `Arc<str>`/`Arc<[u8]>`로 대체 가능
- `cargo clippy -- -W clippy::rc_buffer` 가 정확히 이 패턴(`Rc<String>`, `Rc<Vec<T>>`, `Arc<String>`, `Arc<Vec<T>>`)을 탐지해줌

**예외**:
- 해당 필드가 이후에도 `push_str`/`push` 등으로 변경되어야 하는 경우(불변이 아님)
- 이미 `String`/`Vec<u8>`로 존재하는 값을 소유권만 옮기려는 임시 변환 과정에서는 성능 차이가 무시할 만함

**Bitvue 판정**: Confirmed — `Arc<Vec<u8>>` 패턴이 다수 발견됨(경로는 `bitvue-core`→`bitvue-engine` 개명 반영): crates/bitvue-engine/src/player/mod.rs:140,143(`yuv_data`, `rgba_data`), crates/bitvue-engine/src/stream_state.rs:679-685·862-868(y/u/v plane 6곳) — 전부 `Arc<[u8]>`이면 충분한데 이중 간접 참조(`Arc`→`Vec`→heap)를 그대로 사용. (원 근거 중 src-tauri/decode_service.rs 부분은 `e7194cc`로 삭제되어 제외.)

---

### OWN-017: &'static를 얻기 위한 Box::leak 남용
**분류**: OWN · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
fn register_codec_plugin(name: String, config: PluginConfig) -> &'static PluginConfig {
    // 플러그인 레지스트리 API가 &'static PluginConfig를 요구해서
    // 매번 새 플러그인을 등록할 때마다 Box::leak으로 강제로 'static을 만들어냄
    let boxed = Box::new(config);
    Box::leak(boxed)
}

fn reload_plugins_on_settings_change(configs: Vec<PluginConfig>) {
    for cfg in configs {
        // 설정이 바뀔 때마다 호출되는 함수인데 매번 leak — 이전 leak은 절대 회수되지 않음
        register_codec_plugin("x".to_string(), cfg);
    }
}
```

**문제**:
- `Box::leak`은 말 그대로 메모리를 프로세스 종료 시까지 회수 불가능하게 만드는데, 이것이 "설정 변경마다 반복 호출되는" 경로에 있으면 명백한 메모리 누수
- 최초 1회만 실행되는 초기화 코드라면 무해하지만, 이 함수가 재호출 가능한 경로(설정 리로드, 핫플러그인 교체, 테스트 반복 실행)에 있는지 확인하지 않고 도입되는 경우가 많음
- API가 `&'static T`를 요구하도록 설계된 것 자체가 종종 설계 결함의 신호 — 진짜 필요한 것은 "레지스트리가 소유하고 나눠주는 참조"인데 `'static`으로 뭉뚱그린 것
- 테스트에서 이 함수를 여러 번 호출하면 테스트 프로세스의 메모리가 테스트 스위트 실행 내내 계속 누적됨

**발생 조건**:
- 전역 레지스트리/플러그인 시스템 API를 설계할 때 수명 파라미터를 다루기 싫어서 반환 타입을 `&'static T`로 단순화했을 때
- 설정 리로드, 플러그인 핫스왑처럼 반복 실행 가능한 경로에서 최초 1회성 초기화 함수를 실수로 재사용할 때

**권장**:
```rust
struct PluginRegistry {
    plugins: Vec<Box<PluginConfig>>, // 레지스트리가 소유권을 갖고 관리
}

impl PluginRegistry {
    fn register(&mut self, config: PluginConfig) -> &PluginConfig {
        self.plugins.push(Box::new(config));
        self.plugins.last().unwrap()
    }

    fn unregister_all(&mut self) {
        self.plugins.clear(); // 명시적으로 회수 가능
    }
}
```
- `'static` 참조가 필요해 보이는 대부분의 경우, 실제로는 "레지스트리/컨테이너가 소유하고 그 컨테이너의 수명만큼 유효한 참조"로 충분함 — 레지스트리를 명시적 소유자로 만든다
- 정말 프로세스 생애주기 동안 유지되어야 하는(예: `once_cell::sync::Lazy`로 만든 전역 상수 테이블) 경우에만 `'static`을 사용하고, 그 경우도 `Box::leak`보다 `OnceLock`/`Lazy`가 의도를 더 명확히 드러냄
- `Box::leak`을 쓸 수밖에 없다면 호출 횟수가 유한(이상적으로는 1회)함을 타입/구조로 보장(예: `fn init_once() -> &'static T`를 프로그램 시작 시 1회만 호출되는 `main`에서만 노출)

**탐지 방법**:
- grep: `Box::leak|Vec::leak|String::leak` 전수 검색 — 각 호출부가 정말 1회성인지, 재호출 가능한 경로에 있는지 수동 확인
- Runtime: 반복 실행되는 통합 테스트/설정 리로드 테스트에서 RSS(상주 메모리)가 단조 증가하는지 관찰

**예외**:
- 프로그램 시작 시 정확히 1회만 실행되며, 리로드/재초기화 경로가 설계상 존재하지 않는 전역 상수/설정 테이블 초기화
- `once_cell`/`std::sync::OnceLock` 등으로 이미 "1회성"이 구조적으로 보장된 컨텍스트에서의 leak은 상대적으로 안전(그래도 가능하면 `OnceLock` 자체를 쓰는 것이 더 명확함)

**Bitvue 판정**: N/A — `Box::leak` 호출은 저장소 전체에 crates/vendor/abseil/src/absl_memory/tagged_ptr.rs:322 단 1건뿐이며 이는 서드파티 벤더 태그드포인터 구현의 일부로 Bitvue 자체 플러그인/레지스트리 코드가 아니고, 반복 호출 경로(설정 리로드 등)에 있지도 않음.

---

### OWN-018: 이중 박싱 (Box<dyn Trait>를 다시 감싸기)
**분류**: OWN · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
trait FrameFilter {
    fn apply(&self, frame: &mut DecodedFrame);
}

struct FilterPipeline {
    // Box<dyn Trait>는 이미 힙 할당 + vtable 포인터인데
    // 이를 다시 Box로 감싸 불필요한 2단 간접 참조를 만듦
    filters: Vec<Box<Box<dyn FrameFilter>>>,
}

fn make_filter() -> Box<Box<dyn FrameFilter>> {
    Box::new(Box::new(DeblockFilter::default()))
}
```

**문제**:
- `Box<dyn Trait>`은 이미 "힙에 할당된 값 + vtable 포인터"로 트레이트 객체를 완전히 표현하므로, 이를 또 `Box`로 감싸면 포인터를 한 번 더 따라가야 하는 무의미한 간접 참조가 추가됨
- 보통 제네릭 코드나 팩토리 함수를 리팩터링하는 과정에서 실수로 발생(`Box::new(Box::new(...))`처럼 이미 박싱된 값을 다시 박싱)하며, 컴파일은 문제없이 되기 때문에 발견되지 않고 남아있기 쉬움
- 트레이트 객체 벡터를 순회할 때마다(필터 파이프라인 등 핫패스) 이중 포인터 역참조 비용이 누적

**발생 조건**:
- 팩토리 함수가 이미 `Box<dyn Trait>`을 반환하는데, 호출부에서 또 `Box::new()`로 감싸 저장할 때
- 제네릭 함수의 반환 타입을 `Box<T>`로 통일하려다, `T` 자체가 이미 `Box<dyn Trait>`인 경우를 놓칠 때

**권장**:
```rust
struct FilterPipeline {
    filters: Vec<Box<dyn FrameFilter>>,
}

fn make_filter() -> Box<dyn FrameFilter> {
    Box::new(DeblockFilter::default())
}
```
- 트레이트 객체는 `Box<dyn Trait>` 한 겹으로 충분 — 팩토리 함수의 반환 타입과 저장 컨테이너의 타입을 나란히 놓고 이중 박싱이 없는지 확인
- 여러 소유자가 공유해야 한다면 `Box` 대신 `Arc<dyn Trait>`로 바로 전환(역시 한 겹)

**탐지 방법**:
- grep: `Box<Box<|Box::new(Box::new(|Rc<Rc<|Arc<Arc<` 패턴 검색 — 대부분 실수
- `cargo clippy -- -W clippy::redundant_allocation`이 이 패턴을 정확히 잡아줌

**예외**:
- 사실상 없음 — 이중 박싱이 의도적으로 필요한 경우는 거의 없으며, 필요하다고 느껴진다면 대개 설계 문제(예: 트레이트 객체를 담은 컨테이너 자체를 또 다른 트레이트 객체로 다뤄야 하는 과도한 추상화)의 신호

**Bitvue 판정**: N/A — `Box<Box<`, `Box::new(Box::new`, `Rc<Rc<`, `Arc<Arc<` 패턴 전수 검색 결과 0건.

---

### OWN-019: 거대한 Result<T, E>를 값으로 이동시켜 happy path까지 느려짐
**분류**: OWN · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Debug)]
enum ParseError {
    InvalidNalHeader { raw_bytes: [u8; 64], context: String, sps_snapshot: SequenceParameterSet },
    UnsupportedProfile { profile_id: u8, supported: Vec<u8>, full_bitstream_dump: Vec<u8> },
    // ... 각 variant가 디버깅 편의를 위해 대량의 컨텍스트를 통째로 들고 있음
}

fn parse_slice_header(data: &[u8]) -> Result<SliceHeader, ParseError> {
    // Ok 경로든 Err 경로든, Result 값 자체의 크기는 가장 큰 variant(ParseError)에 맞춰짐
    // 즉 성공 경로에서도 매번 그 큰 크기만큼 스택 공간과 이동 비용을 지불
    Ok(SliceHeader::default())
}
```

**문제**:
- `Result<T, E>`의 메모리 크기는 `max(size_of::<T>(), size_of::<E>()) + discriminant`이므로, `E`가 커지면 `T`가 아무리 작아도(성공 경로) `Result` 전체가 커짐
- 슬라이스 헤더 파싱처럼 프레임당 수십~수백 번 호출되는 함수가 매번 큰 `Result`를 반환/이동시키면, 실패가 거의 없는 happy path까지 오류 케이스의 크기 비용을 함께 지불
- 에러 variant에 "디버깅에 도움이 될까봐" 원본 바이트 배열, 전체 컨텍스트 스냅샷 등을 통째로 넣는 습관이 이 문제를 키움
- `size_of::<Result<T,E>>()`는 눈에 잘 안 보이는 비용이라 프로파일링 전까지 아무도 의심하지 않음

**발생 조건**:
- 에러 타입에 원본 바이트, 전체 구조체 스냅샷 등 대형 디버깅 정보를 그대로 담아 열거형 variant를 설계할 때
- 파싱 함수가 핫루프(프레임/슬라이스 단위)에서 반복 호출되는데 에러 타입 설계 시 호출 빈도를 고려하지 않았을 때

**권장**:
```rust
#[derive(Debug)]
enum ParseError {
    InvalidNalHeader(Box<InvalidNalHeaderInfo>),
    UnsupportedProfile(Box<UnsupportedProfileInfo>),
}

struct InvalidNalHeaderInfo {
    raw_bytes: [u8; 64],
    context: String,
    sps_snapshot: SequenceParameterSet,
}
```
- 에러 variant의 payload가 크면 `Box`로 감싸 `Result`의 크기를 포인터 크기로 고정 — 실패 시에만 힙 할당이 발생하고, 성공 경로는 항상 저렴
- `#[derive(Debug)]`에 필요한 컨텍스트는 정말 필요한 최소한으로 줄이고, 원본 바이트 전체 덤프 같은 것은 별도 로깅 경로로 분리
- `static_assertions::const_assert!(size_of::<Result<T, E>>() <= N)` 같은 컴파일 타임 가드를 핫패스 함수에 추가해 회귀 방지

**탐지 방법**:
- Static: `size_of::<Result<...>>()`를 유닛 테스트나 `const_assert`로 측정해 임계치(예: 32바이트) 초과 여부 확인
- grep: 에러 열거형 variant 정의에서 `Vec<u8>`, `[u8; N]`(N이 크게), 전체 구조체 타입이 직접(Box 없이) 포함된 경우 검색
- `cargo clippy -- -W clippy::result_large_err` — 정확히 이 패턴을 겨냥한 lint

**예외**:
- 호출 빈도가 낮은(초기화, 설정 파싱 등 1회성) 함수라면 에러 크기가 커도 실질적 영향이 미미
- 에러 payload를 Box로 감싸면 에러 발생 시점에 추가 할당이 필요해지는데, 에러가 매우 빈번하게 발생하는 정상 흐름의 일부(예: "찾을 수 없음"이 흔한 조회 함수)라면 오히려 Box가 손해일 수 있음 — 이런 경우는 애초에 에러 payload를 작게 설계하는 것이 정답

**Bitvue 판정**: N/A — 표본 조사한 에러 타입(`BitvueError`: crates/bitvue-engine/src/error.rs, `bitvue-core`에서 개명; sidecar 응답용 `WireError{code,message,offset}`: crates/bitvue-protocol/src/lib.rs:116)은 variant/필드가 `String`/`u64`/`Option<u64>` 등 소형 타입으로 구성됨(원 근거 중 삭제된 src-tauri/error.rs는 제외). 저장소 내 `[u8; N]`(N≥16) 배열은 SEI `UserDataUnregistered { uuid: [u8;16], .. }`나 상수 조회 테이블(TRANS_MPS 등)뿐으로 에러 variant에 통째로 박힌 대형 컨텍스트는 발견되지 않음.

---

### OWN-020: mmap 슬라이스를 불필요하게 소유 버퍼로 승격
**분류**: OWN · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct BitstreamFile {
    mmap: memmap2::Mmap,
}

impl BitstreamFile {
    fn read_annexb_unit(&self, offset: usize, len: usize) -> Vec<u8> {
        // mmap은 이미 OS 페이지 캐시에 매핑되어 있어 슬라이스만으로 zero-copy 접근이 가능한데
        // 매 유닛마다 to_vec()으로 힙에 복사 — mmap을 쓰는 이유 자체를 무력화
        self.mmap[offset..offset + len].to_vec()
    }
}

fn scan_all_units(file: &BitstreamFile, offsets: &[(usize, usize)]) -> Vec<Vec<u8>> {
    offsets.iter().map(|&(o, l)| file.read_annexb_unit(o, l)).collect()
}
```

**문제**:
- mmap 기반 파일 I/O를 도입하는 핵심 이유는 대형 비트스트림 파일(수 GB)을 읽을 때 전체를 메모리에 복사하지 않고 OS 페이지 캐시를 그대로 활용하는 것인데, 유닛 단위로 `to_vec()`을 호출하면 결국 파일 전체를 순회하면서 힙에 다시 복사하는 것과 다를 바 없어짐
- 특히 "전체 파일 스캔"(인덱싱, 프레임 경계 탐색) 같은 작업에서 이 패턴을 쓰면 mmap의 이점이 완전히 사라지고 일반 `read()` 기반 I/O보다 오히려 느려질 수 있음(mmap 페이지 폴트 + 추가 복사)
- 대형 파일(4K/8K 소스, 긴 시퀀스)에서는 이 복사가 누적되어 메모리 사용량이 파일 크기에 비례해 증가, OOM 위험

**발생 조건**:
- 파일 인덱싱/스캔 단계에서 각 NAL/패킷 유닛을 참조가 아닌 소유 버퍼로 뽑아내 리스트에 모아둘 때
- API 설계 시 "호출자가 mmap의 존재를 몰라도 되게" 추상화하려다 반환 타입을 소유 타입(`Vec<u8>`)으로 고정할 때

**권장**:
```rust
struct BitstreamFile {
    mmap: memmap2::Mmap,
}

impl BitstreamFile {
    fn read_annexb_unit(&self, offset: usize, len: usize) -> &[u8] {
        &self.mmap[offset..offset + len]
    }
}

fn scan_all_units<'a>(file: &'a BitstreamFile, offsets: &[(usize, usize)]) -> Vec<&'a [u8]> {
    offsets.iter().map(|&(o, l)| file.read_annexb_unit(o, l)).collect()
}
```
- 인덱싱/스캔 단계는 오프셋과 길이만 기록하고, 실제 바이트 접근은 필요한 순간에 mmap 슬라이스로 지연 수행
- 정말 디코딩 등으로 소유 버퍼가 필요한 유닛(예: emulation prevention 제거된 RBSP)만 그 시점에 `to_vec()`
- 대용량 파일 벤치마크(수 GB급 샘플)로 인덱싱 단계의 피크 메모리 사용량이 파일 크기에 비례해 증가하지 않는지 회귀 감시

**탐지 방법**:
- grep: mmap 필드를 가진 구조체의 메서드 중 `.to_vec()`/`.to_owned()`로 끝나는 반환문 검색
- Runtime: 대용량 샘플 파일로 인덱싱/스캔 단계의 RSS를 측정해 파일 크기 대비 비정상적으로 큰 메모리 사용을 탐지(`/usr/bin/time -l` 또는 `heaptrack`)

**예외**:
- 유닛이 이후 mmap 파일이 닫히거나 재매핑된 뒤에도 유지되어야 하는 경우(예: 파일을 닫고 다음 파일을 열어야 하는 배치 처리)
- 유닛에 대해 emulation prevention byte 제거 등 필연적으로 새 버퍼가 필요한 변환이 뒤따르는 경우

**Bitvue 판정**: N/A — `ByteCache::read_range`(crates/bitvue-engine/src/byte_cache.rs:140, `bitvue-core`에서 개명)는 mmap에서 직접 `&[u8]` 슬라이스를 반환하는 zero-copy 경로이며, 소유 버퍼로 복사하는 유일한 경로(`get_segment`, 178행)는 LRU 세그먼트 캐싱이라는 명시적 목적을 위한 것으로 안티패턴이 지적하는 "유닛마다 무조건 to_vec()" 패턴이 아님.

---

### OWN-021: Tauri 커맨드 경계에서 매 호출마다 전체 상태를 deep clone
**분류**: OWN · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Clone, serde::Serialize)]
struct FrameInspectorState {
    current_frame: DecodedFrame,        // 수 MB
    syntax_tree: Vec<SyntaxElement>,    // 수천 개 노드
    parse_cache: HashMap<u64, ParsedUnit>,
}

#[tauri::command]
fn get_frame_summary(state: tauri::State<Mutex<FrameInspectorState>>) -> FrameSummaryDto {
    let guard = state.lock().unwrap();
    // summary 몇 필드만 필요한데 상태 전체를 clone한 뒤 일부만 뽑아 씀
    let snapshot = guard.clone();
    FrameSummaryDto {
        width: snapshot.current_frame.width,
        height: snapshot.current_frame.height,
        node_count: snapshot.syntax_tree.len(),
    }
}
```

**문제**:
- 프론트엔드가 필요한 것은 요약 정보 몇 개인데, 그 전에 상태 전체(프레임 버퍼 수 MB + 트리 수천 노드 + 캐시)를 clone하면 실제 필요 데이터 대비 수백~수천 배의 낭비
- Tauri 커맨드는 프론트엔드의 UI 상호작용(호버, 스크롤, 패널 전환)마다 자주 호출될 수 있어, 이 낭비가 누적되면 UI가 버벅이는 원인이 됨
- `#[derive(Clone)]`이 상태 구조체 전체에 붙어 있으면(OWN-013과 유사) 이런 실수가 컴파일 타임에 전혀 드러나지 않음
- 락을 쥔 채로(`guard.clone()`) 대형 clone을 수행하면 락 보유 시간이 늘어나 다른 스레드(디코더 워커)가 대기 — OWN-003의 문제와 결합되어 악화

**발생 조건**:
- Tauri 커맨드가 앱 전역 상태를 통째로 들고 있고, 프론트엔드에 일부 필드만 전달하면 되는데 "일단 clone해서 쓰면 편하니까" 관성적으로 전체를 clone할 때
- DTO(프론트 전송용 타입)를 따로 설계하지 않고 내부 상태 타입을 그대로 재사용하려 할 때

**권장**:
```rust
#[tauri::command]
fn get_frame_summary(state: tauri::State<Mutex<FrameInspectorState>>) -> FrameSummaryDto {
    let guard = state.lock().unwrap();
    // 필요한 필드만 읽어서 락을 짧게 유지하고, 대형 필드는 애초에 건드리지 않는다
    FrameSummaryDto {
        width: guard.current_frame.width,
        height: guard.current_frame.height,
        node_count: guard.syntax_tree.len(),
    }
} // 락은 여기서 즉시 해제됨
```
- Tauri 커맨드는 항상 "프론트엔드가 실제로 필요한 최소 DTO"를 반환하도록 설계하고, 내부 상태를 그대로 clone/직렬화하지 않는다
- 락 보유 구간을 최소화 — 필요한 필드만 읽고 즉시 락을 놓는다
- 대형 데이터(프레임 픽셀 등)를 프론트에 보내야 한다면 clone이 아니라 별도 스트리밍/청크 전송 커맨드로 분리

**탐지 방법**:
- grep: `#[tauri::command]` 함수 본문에서 `state` 잠금 직후 `.clone()`이 등장하는 패턴 전수 조사
- Runtime: Tauri 커맨드 호출 지연시간을 프론트엔드 devtools 또는 `tracing` 계측으로 측정해 상태 크기에 비례해 느려지는 커맨드 식별

**예외**:
- 상태 자체가 원래 작고(수십 바이트~수 KB) DTO 설계 비용이 clone 비용보다 큰 경우
- 프론트엔드가 실제로 전체 상태의 스냅샷(디버그 패널, 상태 덤프 기능)을 필요로 하는 경우는 전체 clone이 목적에 부합

**Bitvue 판정**: N/A — 원 근거였던 src-tauri/src/commands는 삭제됨; 대체된 sidecar 커맨드 계층(`bitvue-sidecar/src/main.rs`, 요청 핸들러 다수)을 조사한 결과 `.clone()` 대부분(38건)이 `request.params.clone()`(들어온 JSON 파라미터, 소형)이며 `state.clone()`/`core.clone()` 형태의 전체 상태 deep clone 패턴은 발견되지 않음 — `Arc<Core>`는 스레드 진입 시 포인터만 clone(`Arc::clone`)되어 저렴함.

---

### OWN-022: Weak 미사용으로 인한 Rc 순환 참조 누수
**분류**: OWN · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
struct CacheEntry {
    frame: DecodedFrame,
    next: RefCell<Option<Rc<CacheEntry>>>,
    prev: RefCell<Option<Rc<CacheEntry>>>, // 부모 방향도 강한 참조
}

fn link(a: &Rc<CacheEntry>, b: &Rc<CacheEntry>) {
    *a.next.borrow_mut() = Some(b.clone());
    *b.prev.borrow_mut() = Some(a.clone()); // a <-> b가 서로를 강하게 참조하는 순환 형성
}
```

**문제**:
- LRU 캐시를 이중 연결 리스트로 구현하면서 `next`/`prev`를 모두 `Rc`(강한 참조)로 만들면, 두 노드가 서로를 참조하는 순환이 생겨 refcount가 절대 0으로 떨어지지 않음
- 캐시에서 엔트리를 "제거"해도(HashMap에서 키만 지워도) 연결 리스트 상의 강한 참조 순환 때문에 실제 `DecodedFrame` 메모리(수 MB)는 해제되지 않고 누적됨
- 이런 누수는 단위 테스트에서는 잘 드러나지 않고(개별 노드 생성/삭제 테스트는 통과), 장시간 재생/스크러빙처럼 캐시 교체가 계속 일어나는 실사용 시나리오에서만 RSS가 서서히 증가하는 형태로 나타남

**발생 조건**:
- LRU/LFU 캐시, 재생목록, undo/redo 히스토리처럼 이중 연결 구조를 `Rc`로 직접 구현할 때, 역방향 참조(prev, parent)까지 습관적으로 `Rc`로 만들 때
- 순환 참조 자체를 인지하지 못하고 "연결 리스트니까 양방향 다 Rc여야지"라고 단순하게 설계할 때

**권장**:
```rust
struct CacheEntry {
    frame: DecodedFrame,
    next: RefCell<Option<Rc<CacheEntry>>>,
    prev: RefCell<Option<Weak<CacheEntry>>>, // 역방향은 Weak로 순환을 끊는다
}

fn link(a: &Rc<CacheEntry>, b: &Rc<CacheEntry>) {
    *a.next.borrow_mut() = Some(b.clone());
    *b.prev.borrow_mut() = Some(Rc::downgrade(a));
}
```
- 소유 관계가 명확한 방향(예: 리스트 헤드 → 다음 노드)만 `Rc`(강한 참조)로 두고, 역방향(자식 → 부모, next → prev)은 `Weak`로 순환을 끊는다
- 애초에 연결 리스트를 직접 `Rc`/`RefCell`로 구현하기보다 `VecDeque`/`LinkedHashMap` 같은 검증된 자료구조나 arena+인덱스(OWN-011 참고)로 대체하는 것을 우선 고려
- 캐시류는 명시적 `evict()`가 실제로 메모리를 해제하는지 확인하는 테스트(예: `Rc::strong_count`가 0에 수렴하는지, 또는 `Drop`에 카운터를 심어 검증)를 추가

**탐지 방법**:
- Static: 같은 타입을 서로 참조하는 두 필드가 모두 `Rc<...>`(둘 다 `Weak`가 아님)인 구조체를 grep으로 탐색
- Runtime: `valgrind --leak-check=full`이나 장시간 실행 테스트에서 RSS가 캐시 크기 상한을 넘어 계속 증가하는지 관찰
- Manual: 캐시 evict 경로 이후 `Rc::strong_count(&entry) == 0`(또는 1, 소유자 기준)이 되는지 단위 테스트로 명시적 검증

**예외**:
- 순환이 의도적이고 애플리케이션 생애주기 동안 딱 한 번만 만들어지며 해제될 필요가 없는 정적 구조(드묾)
- `Weak`를 쓰면 매번 `upgrade()` 처리(실패 가능성 핸들링)가 필요해 코드가 복잡해지는데, 대신 애초에 인덱스 기반 arena로 재설계하는 것이 더 나은 경우가 많음 — 이 항목의 "권장"은 최소 수정안이고, 더 나은 해법은 OWN-011

**Bitvue 판정**: N/A — 저장소 전체에 `Weak<` 사용례가 0건이며 `Rc<RefCell<>>` 기반 양방향 연결 구조 자체가 없음(OWN-011 조사와 동일한 결론) — 순환 참조가 발생할 구조가 애초에 존재하지 않음.

---

### OWN-023: FFI 디코더 핸들에 Clone을 derive해 이중 해제 위험 노출
**분류**: OWN · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Clone)] // 컴파일러가 막지 않으니 그냥 습관적으로 붙임
struct VmafContext {
    raw: *mut VmafContextFfi, // libvmaf가 소유하는 C 리소스에 대한 핸들
}

impl Drop for VmafContext {
    fn drop(&mut self) {
        unsafe { vmaf_close(self.raw) }; // clone된 두 인스턴스가 각각 drop되면 동일 포인터를 두 번 close
    }
}

fn compute_scores(ctx: VmafContext) -> Vec<f64> {
    let ctx2 = ctx.clone(); // raw 포인터가 그대로 복사됨 — 두 VmafContext가 같은 C 객체를 "소유"한다고 믿음
    std::thread::spawn(move || { /* ctx2 사용 */ });
    vec![]
}
```

**문제**:
- `*mut T` 필드는 `Copy`이기 때문에 `#[derive(Clone)]`이 아무 경고 없이 성공하지만, 의미상 이 핸들은 단일 소유(C 라이브러리 리소스 하나)를 나타내야 함
- clone된 두 `VmafContext`가 각각 스코프를 벗어나 `Drop`이 두 번 호출되면 동일한 raw 포인터에 대해 `vmaf_close`가 두 번 실행 — 이중 해제(double-free)로 UB, 크래시 또는 힙 손상
- 이런 버그는 정상 경로에서는 거의 재현되지 않다가, 두 소유자의 drop 순서/타이밍이 겹치는 특정 조건(에러 처리 경로, 조기 반환, 스레드 종료 타이밍)에서만 터져 재현이 매우 어려움
- `derive(Clone)`은 코드상 아주 무해해 보이는 한 줄이라 리뷰에서 "왜 위험한가"를 즉시 알아채기 어려움

**발생 조건**:
- FFI 리소스를 감싸는 래퍼 구조체를 만들 때 다른 일반 구조체와 마찬가지로 관성적으로 `#[derive(Debug, Clone)]` 템플릿을 적용할 때
- 코드 리뷰에서 FFI 핸들 타입과 일반 데이터 타입을 구분해서 보지 않을 때

**권장**:
```rust
struct VmafContext {
    raw: *mut VmafContextFfi,
}
// Clone을 derive하지 않는다 — 필요하다면 Arc<VmafContext>로 공유 소유권을 명시적으로 표현

impl Drop for VmafContext {
    fn drop(&mut self) {
        unsafe { vmaf_close(self.raw) };
    }
}

fn compute_scores(ctx: Arc<VmafContext>) -> Vec<f64> {
    let ctx2 = Arc::clone(&ctx); // 참조 카운트 공유 — close는 마지막 소유자가 drop될 때 정확히 1번
    std::thread::spawn(move || { let _ = ctx2; });
    vec![]
}
```
- `Drop`에서 FFI 해제 함수를 호출하는 타입에는 절대 `#[derive(Clone)]`을 붙이지 않는다 — 필요하면 `Arc<T>`로 감싸 refcount 기반 단일 해제를 보장
- 정말 값 복제(C 라이브러리에 별도 컨텍스트를 새로 만드는 API가 있는 경우)가 필요하다면 `impl Clone`을 수동으로 작성해 내부적으로 FFI의 "복제" 함수를 호출하도록 명시
- 코드 리뷰 규칙: FFI 리소스를 감싸는 모든 타입은 `Clone` derive 여부를 반드시 명시적으로 검토 항목에 포함

**탐지 방법**:
- grep: `impl Drop for`가 있는 타입 중 `#[derive(..., Clone, ...)]`도 함께 붙어 있는 타입을 교차 검색 — 대부분 위험 신호
- Static: raw 포인터(`*mut`/`*const`) 필드를 가진 구조체에 `Clone`이 derive되어 있는지 전수 검사
- Runtime: ASan(`RUSTFLAGS="-Z sanitizer=address"`, nightly) 또는 valgrind로 FFI 핸들 이중 해제를 실행 시점에 탐지

**예외**:
- 해당 FFI 리소스가 참조 카운팅 기반이라 C API 자체에 "retain"에 해당하는 함수가 있고, 커스텀 `Clone` 구현이 그 retain 함수를 정확히 호출하도록 작성된 경우(derive가 아닌 수동 impl)
- 리소스가 `Drop`을 구현하지 않는(해제할 것이 없는, 예: 순수 조회용 read-only 핸들) 경우라면 Clone derive가 안전할 수 있음 — 단, 이 경우도 "정말 Drop이 없어도 되는가"를 명확히 확인해야 함

**Bitvue 판정**: N/A — `impl Drop for` 4곳(vvdec.rs의 DecoderGuard/AccessUnitGuard/VvcDecoder, performance.rs의 PerfTimer) 전수 확인 결과 어느 것도 `#[derive(Clone)]`이 붙어있지 않음; FFI 리소스 핸들과 Clone 가능 값 타입이 명확히 분리되어 있음.
