# Anti-Pattern Catalog — PLUGIN: 코덱·플러그인 확장 구조

이 문서는 더 큰 안티패턴 카탈로그의 일부이며(전체 색인은 `docs/anti-patterns/INDEX.md`, Wave 1~3 기완료), Phase 4 웨이브로서 Bitvue처럼 워크스페이스 안에 코덱별 크레이트(AV1/HEVC/AVC/VP9/VVC/AV3/MPEG-2 등)가 사실상의 "플러그인" 후보로 존재하는 구조를 다룬다. Wave 1의 `CODEC.md`가 컨테이너-코덱 사이의 아키텍처 계층/경계(누가 무엇을 알아야 하는가)를 다뤘다면, 이 문서는 그 경계를 넘어 **확장 메커니즘 자체** — 새 코덱이나 새 지표를 중앙 코드를 건드리지 않고 추가할 수 있는가, capability를 어떻게 발견하는가, plugin의 lifecycle(초기화/실패/언로드)을 어떻게 관리하는가 — 를 다룬다. Bitvue의 코덱들은 현재 동적 로딩이 아닌 컴파일 타임 Rust 크레이트이므로, 일부 항목(특히 dynamic loading을 전제로 하는 항목)은 즉시 적용 가능한 결함이라기보다 향후 진짜 plugin 시스템을 도입할 경우를 대비한 전방주시적(forward-looking) 항목으로 표시했다.

---

### PLUGIN-001: 코덱 추가마다 중앙 match 수정
**분류**: 확장성 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-decode/src/dispatch.rs
pub fn parse_unit(codec: CodecId, data: &[u8]) -> Result<ParsedUnit, ParseError> {
    match codec {
        CodecId::Avc  => bitvue_avc::parse(data).map(ParsedUnit::Avc),
        CodecId::Hevc => bitvue_hevc::parse(data).map(ParsedUnit::Hevc),
        CodecId::Vp9  => bitvue_vp9::parse(data).map(ParsedUnit::Vp9),
        CodecId::Av1  => bitvue_av1::parse(data).map(ParsedUnit::Av1),
        // VVC, AV3, MPEG-2 를 추가할 때마다 이 함수와
        // ParsedUnit enum, 아래쪽 UI dispatch, 직렬화 코드를 전부 고쳐야 한다
    }
}
```

**문제**:
- 새 코덱 하나를 추가하는 작업이 "코덱 크레이트 작성"이 아니라 "워크스페이스 전역에 흩어진 N개의 match를 찾아서 고치는 작업"이 된다.
- 컴파일러가 `match`가 exhaustive해야 한다고 강제하는 것은 장점처럼 보이지만, 실제로는 central dispatcher, 결과 enum, UI 라우팅, 직렬화, capability 테이블이 모두 같은 축으로 커플링되어 있다는 신호다 — 코덱 크레이트가 "플러그인"이 아니라 core의 일부가 되어버린 것.
- 리뷰어가 새 코덱 PR에서 "central match 수정 빠뜨림"을 매번 수동으로 확인해야 하고, 빠뜨리면 컴파일 에러가 아니라(exhaustive match라서 걸리긴 하지만) 여러 파일에 걸친 대규모 diff가 하나의 PR에 뭉쳐진다.

**발생 조건**:
- 워크스페이스에 11번째 코덱 크레이트를 추가할 때, 혹은 실험적 코덱(AV2 draft, 사내 커스텀 코덱)을 잠깐 붙였다 떼는 경우.
- 코덱별 신규 지표(overlay, hex-view offset 매핑 등)를 추가할 때도 동일한 패턴이 반복된다.

**권장**:
```rust
// bitvue-decode/src/registry.rs
pub trait CodecPlugin: Send + Sync {
    fn id(&self) -> CodecId;
    fn parse(&self, data: &[u8]) -> Result<Box<dyn ParsedUnit>, ParseError>;
}

inventory::submit! {
    &bitvue_avc::AvcPlugin as &'static dyn CodecPlugin
}
// bitvue-avc 크레이트 자신이 등록을 담당한다.
// dispatch.rs는 레지스트리를 순회할 뿐 코덱 이름을 알 필요가 없다
pub fn parse_unit(codec: CodecId, data: &[u8]) -> Result<Box<dyn ParsedUnit>, ParseError> {
    registry::lookup(codec)?.parse(data)
}
```
- `inventory`/`linkme` 같은 distributed-slice 레지스트리, 혹은 `build.rs`가 생성하는 등록 테이블을 사용해 "코덱 크레이트가 스스로를 등록"하는 방향으로 뒤집는다.
- 최소한 등록 지점을 하나의 매크로/매크로 호출 목록(`register_all_codecs!(avc, hevc, vp9, ...)`)으로 모아 diff 범위를 한 파일로 좁힌다.

**탐지 방법**:
- Structural: 새 코덱 크레이트를 추가하는 PR의 diff가 몇 개 파일에 걸치는지 측정 — 코덱 크레이트 외부 파일이 3개 이상 바뀌면 경고.
- Static: `match codec` / `match self.codec_id` 패턴을 grep해 central dispatcher 후보를 나열.

**예외**:
- 코덱 개수가 명확히 유한하고(예: 정확히 표준화된 5~6종만 영구히 지원) 향후 추가 계획이 없다면, 레지스트리 추상화 자체가 과설계일 수 있다. 이 경우 명시적 exhaustive match가 오히려 "빠진 코덱을 컴파일 타임에 잡아준다"는 장점이 크다.

**Bitvue 판정**: Confirmed — (2026-08-18 Electron 전환 후 재검증, 구 `src-tauri`/`bitvue-core` 인용은 폐기) 실사용 경로는 `crates/bitvue-cli/src/commands/decode.rs:207`(`resolve_codec`)와 `:248`(`extract_frames`, `main.rs:348`에서 실제 호출됨) — 코덱 하나 추가할 때마다 이 두 match를 고쳐야 하는 정확히 나쁜 예 패턴. `crates/bitvue-engine/src/index_extractor.rs:663`(`ExtractorFactory::create`, 옛 `bitvue-core`가 `bitvue-engine`으로 개명된 동일 코드)도 동일 패턴이지만 이건 자체 테스트 외 호출자가 없는 사실상 죽은 코드(grep 결과 `index_extractor_test.rs`/`tests/index_extractor.rs`뿐). "HEVC/VP9/VVC indexers are disabled due to cyclic dependency" 주석은 여전히 남아있음.

---

### PLUGIN-002: plugin interface가 내부 구조체를 그대로 노출
**분류**: 캡슐화 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-hevc/src/lib.rs — public API가 파서 내부 AST를 그대로 노출
pub struct SliceHeader {
    pub first_slice_in_pic_flag: bool,
    pub pic_parameter_set_id: u32,
    pub slice_type: u8,
    pub(crate) rps_scratch: Vec<RefPicSetEntry>, // 내부 재사용 버퍼가 실수로 pub(crate) 노출
    // ... 표준서 필드 60개
}

pub fn parse_slice_header(rbsp: &[u8]) -> SliceHeader { /* ... */ }
```
core는 이 `SliceHeader`를 직접 들고 UI/직렬화/다른 코덱 비교 로직까지 전파시킨다.

**문제**:
- HEVC 표준의 syntax 필드 이름과 구조가 그대로 core의 공개 타입이 되어, `bitvue-hevc`의 내부 리팩터링(예: RPS 파싱 방식 변경, 필드 이름을 표준서 개정판에 맞춰 변경)이 core와 UI 코드까지 깨뜨린다.
- 코덱마다 필드 이름/단위/enum 값이 제각각이라(`slice_type: u8` vs 다른 코덱의 `SliceType` enum) core가 코덱별 특이사항을 다시 알아야 하는 상황이 재발한다 — plugin 경계가 사실상 없는 것과 같다.
- 파서 내부에서만 쓰는 scratch 필드가 `pub`/`pub(crate)` 경계 실수로 새어나가면, 외부에서 그 필드를 읽고 쓰는 코드가 생겨버려 나중에 못 지운다.

**발생 조건**:
- 코덱 파서를 "일단 파싱 결과 구조체를 만들고 pub으로 열어서 빠르게 쓰기" 위해 만들 때.
- 신규 codec crate를 기존 crate 복사해서 시작했는데, 기존 crate의 안일한 공개 범위까지 그대로 답습할 때.

**권장**:
```rust
// bitvue-hevc/src/lib.rs
mod internal; // SliceHeader 등 표준서 그대로의 구조체는 여기 숨김

pub struct SliceSummary {
    pub slice_type: SliceKind,     // 코덱 공통 enum으로 정규화
    pub qp: i32,
    pub is_reference: bool,
}

impl CodecPlugin for HevcPlugin {
    fn summarize_slice(&self, rbsp: &[u8]) -> Result<SliceSummary, ParseError> {
        let raw = internal::parse_slice_header(rbsp)?;
        Ok(SliceSummary::from(raw)) // 여기서만 내부->외부 변환
    }
}
```
- plugin trait의 반환 타입은 core/UI가 실제로 필요로 하는 "안정적 뷰(view)" 타입으로 좁힌다. 표준서 그대로의 raw struct는 크레이트 내부(`pub(crate)` 혹은 별도 `internal` 모듈)에 가둔다.
- 코덱별 원본 필드가 꼭 필요한 고급 사용자(hex-view, syntax tree 탐색기)를 위해서는 별도의 "raw 모드" API를 명시적으로 분리해 제공하고, 이 API는 안정성을 보장하지 않음을 문서화한다.

**탐지 방법**:
- Static: 코덱 크레이트의 `pub struct`가 core 크레이트에서 필드 단위로 `.field_name`으로 직접 접근되는지 grep.
- Structural: 코덱 크레이트의 공개 타입 수와 표준서 syntax element 수를 비교 — 거의 1:1이면 경계 없음 신호.

**예외**:
- hex-view/syntax-tree 인스펙터처럼 "raw 구조를 있는 그대로 보여주는 것"이 기능 요구사항 자체인 도구에서는 raw 노출이 버그가 아니라 의도다. 다만 이 경우도 "다른 모듈이 그 raw 타입에 의존하는 것"과 "UI가 read-only로 보여주기만 하는 것"은 구분해야 한다.

**Bitvue 판정**: Confirmed — (2026-08-18 재검증, 경로 불변) `crates/bitvue-hevc/src/lib.rs:54`가 `pub use slice::{SliceHeader, SliceType}`로 재노출하고, `crates/bitvue-hevc/src/slice.rs:101` `SliceHeader`는 표준서 필드 30여 개가 전부 `pub`인 raw struct(나쁜 예와 거의 동일). `SliceHeader` 식별자를 grep하면 다른 코덱 크레이트(avc/mpeg2/vvc)가 각자 자기 자신의 동명 타입을 갖고 있을 뿐, `bitvue-hevc`의 것을 crate 밖(core/CLI/UI)에서 쓰는 코드는 여전히 없어 실질 피해는 아직 발현 전.

---

### PLUGIN-003: ABI 안정성을 고려하지 않은 dynamic plugin
**분류**: ABI/바이너리 호환성 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// 가상의 동적 로딩 시나리오: 코덱을 .dylib/.so로 분리해 런타임에 로드한다고 가정
#[no_mangle]
pub extern "C" fn parse_frame(data: *const u8, len: usize) -> ParsedFrame {
    // ParsedFrame이 Vec<T>, String, enum with data 등
    // repr(Rust)인 채로 FFI 경계를 넘어간다
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    internal_parse(slice)
}

#[repr(Rust)] // 기본값이라 명시조차 안 하는 경우가 대부분
pub struct ParsedFrame {
    pub nal_units: Vec<NalUnit>,
    pub codec_name: String,
}
```

**문제**:
- `Vec<T>`, `String`, `enum`의 메모리 레이아웃은 Rust 컴파일러 버전/최적화 옵션에 따라 달라질 수 있는(`repr(Rust)`, 안정성 미보장) 표현이다. host와 plugin이 서로 다른 rustc 버전으로 빌드되면 레이아웃이 어긋나 UB가 발생할 수 있다.
- plugin을 host와 별도로 재컴파일/재배포할 수 있다는 것이 동적 로딩의 핵심 가치인데, ABI가 host의 rustc 버전에 암묵적으로 고정되면 사실상 "항상 함께 재빌드해야 하는 static 링크"와 다를 게 없어지고, 그 대가로 동적 로딩의 성능/안전성 비용만 남는다.
- 크래시가 나도 어느 필드의 레이아웃이 어긋났는지 디버깅하기 매우 어렵다 (UB는 종종 한참 뒤에, 관련 없어 보이는 곳에서 터진다).

**발생 조건**:
- **Bitvue 현재 상태와의 구분**: Bitvue의 코덱들은 지금 워크스페이스 안의 컴파일 타임 Rust 크레이트이며 dylib으로 동적 로드되지 않는다 — 이 항목은 당장 적용되는 결함이 아니라, 향후 "서드파티가 코덱 플러그인을 별도로 배포해 런타임에 붙이는" 진짜 dynamic plugin 시스템을 도입할 경우에 대비한 전방주시적 참조 항목이다.
- 그런 시스템이 생긴다면: 코덱 벤더가 자체 dylib을 배포하는 시나리오, 혹은 플러그인을 격리된 프로세스가 아니라 같은 프로세스에 `dlopen`하는 시나리오에서 즉시 문제가 된다.

**권장**:
```rust
// C ABI로 고정하고 버전을 명시적으로 박아넣는다
#[repr(C)]
pub struct FfiParsedFrame {
    pub abi_version: u32,       // 이 struct layout의 버전
    pub nal_units_ptr: *mut FfiNalUnit,
    pub nal_units_len: usize,
    pub codec_name: *const c_char,
}

#[no_mangle]
pub extern "C" fn plugin_abi_version() -> u32 { 1 }

#[no_mangle]
pub extern "C" fn parse_frame(data: *const u8, len: usize, out: *mut FfiParsedFrame) -> i32 {
    // host가 plugin_abi_version()을 먼저 확인하고 호출
    ...
}
```
- 진짜 동적 플러그인이 필요하다면 (a) `#[repr(C)]` + 버전 negotiation, (b) 프로세스 격리(별도 프로세스 + IPC, 예: stdin/stdout 프로토콜이나 gRPC), (c) WASM 컴포넌트(wasmtime 등, 언어 중립적이고 샌드박스도 겸함) 중 하나를 택한다. Rust struct를 FFI 경계에 그대로 노출하지 않는다.
- Bitvue처럼 compile-time crate로 충분한 동안은 이 복잡도를 들이지 않는 것이 합리적 선택이며, "언젠가 동적 로딩이 필요해질 수 있다"는 이유만으로 미리 C ABI 계층을 만들 필요는 없다.

**탐지 방법**:
- Static: `#[no_mangle] extern "C"` 함수의 시그니처에 `repr(Rust)` 타입(String, Vec, non-repr(C) enum)이 그대로 나타나는지 검사.
- Structural: 워크스페이스에 `.so`/`.dylib`/`.dll` 빌드 타깃이나 `libloading`/`dlopen` 의존성이 등장하는 순간부터 이 검사를 CI에 추가.

**예외**:
- 컴파일 타임에 정적 링크되는 워크스페이스 크레이트(Bitvue 현재 상태)에는 이 항목이 적용되지 않는다 — Rust ABI가 동일 빌드 내에서는 안정적이기 때문.

**Bitvue 판정**: N/A — (2026-08-18 재검증, Electron 전환 후에도 불변) 문서 자체가 명시한 예외에 정확히 해당. 워크스페이스 어떤 `Cargo.toml`에도 `libloading`/`dlopen`/`wasmtime` 의존성이 없음(grep 0건) — 모든 코덱 크레이트가 컴파일 타임 workspace path 의존성으로 정적 링크됨. Electron 마이그레이션으로 `bitvue-sidecar`가 별도 프로세스(stdio IPC)가 됐지만 이는 코덱 dylib 격리가 아니라 Rust 엔진 전체 대 Electron 렌더러 격리이므로 이 항목과 무관.

---

### PLUGIN-004: plugin panic이 host를 종료
**분류**: 장애 격리 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
// bitvue-decode/src/dispatch.rs
pub fn parse_unit(codec: CodecId, data: &[u8]) -> Result<Box<dyn ParsedUnit>, ParseError> {
    let plugin = registry::lookup(codec)?;
    Ok(plugin.parse(data)?) // AVC 파서 내부에서 data[i]가 index out of bounds로 panic하면
                            // 이 호출 스택 전체가 unwind되며, 스레드가 GUI 메인 스레드라면
                            // Tauri 앱 전체가 죽거나(panic=abort 빌드) 최소한 현재 세션의
                            // 프레임 렌더링 파이프라인이 통째로 중단된다
}
```

**문제**:
- 조작되었거나 손상된(fuzzed, truncated) 비트스트림 하나 때문에 분석기 전체가 죽으면, 사용자는 "어느 코덱의 어느 유닛이 문제였는지"조차 알 수 없이 세션을 통째로 잃는다.
- `panic = "abort"`로 빌드된 바이너리에서는 `catch_unwind`조차 무력하므로, panic 경계 설계는 반드시 빌드 설정과 함께 검토해야 한다.
- 코덱 크레이트가 10개인 워크스페이스에서 이 문제는 "어느 한 코덱 파서의 버그가 다른 9개 코덱을 보던 세션까지 끌고 내려간다"는 형태로 나타난다 — 플러그인 격리가 없다는 것의 직접적 대가.

**발생 조건**:
- **Bitvue 현재 상태와의 구분**: dynamic plugin 여부와 무관하게, in-process로 링크된 컴파일 타임 크레이트라도 panic 경계가 없으면 동일하게 프로세스/스레드가 죽는다 — 이 문제 자체는 지금도 유효하다. 다만 진짜 격리(별도 프로세스로 plugin을 격리해 panic이 host와 무관해지는 구조)는 dynamic/out-of-process plugin 시스템을 전제로 하므로, "완전한 해법"은 forward-looking이고 "최소한의 방어선(`catch_unwind`)"은 지금 바로 적용 가능하다.
- 퍼징된 입력, 표준을 위반하는 malformed 스트림, 혹은 아직 완전히 구현되지 않은 코덱 profile을 열었을 때 흔히 발생.

**권장**:
```rust
pub fn parse_unit(codec: CodecId, data: &[u8]) -> Result<Box<dyn ParsedUnit>, ParseError> {
    let plugin = registry::lookup(codec)?;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| plugin.parse(data)))
        .map_err(|payload| ParseError::PluginPanicked {
            codec,
            message: panic_message(&payload),
        })?
}
```
- 최소한 `catch_unwind`로 코덱별 호출을 감싸 panic을 `Result`의 에러 variant로 변환하고, 어느 codec/unit에서 발생했는지 로그에 남긴다 (단, `panic=abort` 빌드에서는 무력하므로 별도 검토 필요).
- 더 강한 격리가 필요하다면 (특히 서드파티가 배포하는 진짜 plugin이 생긴다면) 파싱을 별도 프로세스/워커 스레드 풀에서 실행하고 crash를 IPC 경계에서 흡수한다.
- fuzzing corpus(`cargo fuzz`)를 코덱 크레이트마다 CI에 상시 돌려 panic을 배포 전에 잡는다.

**탐지 방법**:
- Runtime: malformed/truncated 비트스트림 fixture로 회귀 테스트를 돌려 프로세스가 살아남는지 확인.
- Static: 코덱 dispatch 경로에 `catch_unwind`가 없는지 grep, `panic = "abort"` 설정 여부를 `Cargo.toml`에서 확인.

**예외**:
- 배치/CLI 도구처럼 "한 파일이 깨지면 그 파일만 실패로 표시하고 프로세스는 종료해도 무방한" 짧은 수명의 단발성 실행에서는 panic=abort로 두고 프로세스 자체를 재시작 단위로 삼는 것이 오히려 단순하다. 장시간 떠 있는 GUI 세션에는 해당하지 않는다.

**Bitvue 판정**: Confirmed — (2026-08-18 재검증) 루트 `Cargo.toml:131` `[profile.release]`에 `panic = "abort"`가 여전히 설정되어 있고(구 인용 125행은 Electron 전환 과정에서 밀린 것), 코덱 dispatch/파싱 경로(`bitvue-sidecar/src/*.rs`, `bitvue-cli/src/commands/decode.rs`, `bitvue-avc`/`bitvue-hevc` 등) 어디에도 `catch_unwind` 호출이 없음(전체 grep 결과 테스트 코드 2건과 `vendor/abseil`에만 존재). 특히 `bitvue-sidecar`는 이제 장시간 떠 있는 별도 프로세스(stdio IPC)라 panic=abort로 죽으면 Electron 메인 프로세스와의 세션 전체가 끊김 — 격리 없이 위험이 그대로 이전됨. 손상된 비트스트림이 코덱 파서에서 panic을 내면 release 빌드에서 sidecar 프로세스 전체가 abort됨.

---

### PLUGIN-005: capability discovery 없음
**분류**: 확장성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// UI가 "이 코덱이 무엇을 지원하는지"를 하드코딩된 if/else로 판단
fn available_overlays(codec: CodecId) -> Vec<OverlayKind> {
    let mut overlays = vec![OverlayKind::MbType, OverlayKind::RefIndex];
    if codec == CodecId::Avc || codec == CodecId::Hevc {
        overlays.push(OverlayKind::MotionVector);
    }
    if codec == CodecId::Av1 {
        overlays.push(OverlayKind::CdefStrength); // VP9는 지원하는데 깜빡 빠뜨림
    }
    overlays
}
```

**문제**:
- "이 코덱이 무엇을 할 수 있는가"라는 정보가 코덱 크레이트가 아니라 UI 레이어의 하드코딩된 분기 안에 흩어져 산다 — 정보의 소유자와 정보가 쓰이는 곳이 분리되어 있다.
- 코덱 크레이트를 새로 추가하거나 기존 크레이트에 새 지표(overlay, metric)를 추가할 때, "UI의 이 if/else를 업데이트하는 것을 잊는" 사일런트 누락이 반복된다 — 컴파일 에러가 안 나기 때문에 리뷰에서 놓치기 쉽다.
- "AV1은 CDEF strength를 지원하는데 VP9도 사실 loop filter level을 지원한다"처럼 지식이 늘어날수록 이 분기가 점점 더 커지고 정확도가 떨어진다.

**발생 조건**:
- 새 overlay/metric을 하나의 코덱에만 우선 구현하고 나머지 코덱은 나중으로 미룰 때 (흔한 점진적 개발 패턴이지만 discovery 메커니즘이 없으면 "나중"이 영원히 안 온다).
- 여러 코덱을 동시에 지원하는 비교 뷰(dual-stream)에서 "두 스트림이 공통으로 지원하는 overlay만 보여줘야 하는" 교집합 계산이 필요할 때.

**권장**:
```rust
bitflags::bitflags! {
    pub struct CodecCapabilities: u32 {
        const MOTION_VECTORS   = 1 << 0;
        const MB_TYPE          = 1 << 1;
        const REF_INDEX        = 1 << 2;
        const LOOP_FILTER      = 1 << 3;
        const CDEF             = 1 << 4;
    }
}

pub trait CodecPlugin: Send + Sync {
    fn capabilities(&self) -> CodecCapabilities;
}

impl CodecPlugin for Av1Plugin {
    fn capabilities(&self) -> CodecCapabilities {
        CodecCapabilities::MOTION_VECTORS | CodecCapabilities::MB_TYPE
            | CodecCapabilities::REF_INDEX | CodecCapabilities::CDEF
    }
}

// UI는 코덱 이름을 몰라도 된다
fn available_overlays(plugin: &dyn CodecPlugin) -> Vec<OverlayKind> {
    OverlayKind::ALL.iter().filter(|k| plugin.capabilities().contains(k.required_flag())).collect()
}
```
- 각 코덱 플러그인이 자신의 capability를 선언하는 단일 진실 공급원(single source of truth)이 되게 하고, UI/dispatch/직렬화는 이를 질의만 한다.
- capability는 정적 상수뿐 아니라 런타임에 달라질 수도 있다(예: 특정 profile/level에서만 지원)는 점을 감안해 `fn capabilities(&self, ctx: &StreamContext) -> CodecCapabilities`처럼 컨텍스트를 받을 여지를 남긴다.

**탐지 방법**:
- Structural: UI/core 레이어에 `if codec == CodecId::X` 형태의 코덱별 분기가 몇 곳에 있는지 집계 — capability API가 있다면 이 숫자는 0에 가까워야 한다.
- Manual: 새 코덱 추가 체크리스트에 "capabilities() 구현 여부"를 필수 항목으로 넣고 리뷰에서 확인.

**예외**:
- codec 수가 아주 적고(2~3개) capability 축도 거의 겹치지 않는 초기 프로토타입 단계에서는 명시적 bitflags 도입이 과설계일 수 있다. 다만 코덱이 4개를 넘어가는 시점부터는 이 항목의 비용이 빠르게 이익을 넘어선다.

**Bitvue 판정**: Confirmed — (2026-08-18 재검증) Rust 백엔드에는 capability 질의 API가 전혀 없음(`IndexExtractor` trait은 `is_supported() -> bool` 이진 플래그뿐, `crates/bitvue-engine/src/index_extractor.rs`, 구 `bitvue-core`에서 개명). 프런트엔드는 `frontend/utils/codecModeRegistry.ts`(`CODEC_MODE_REGISTRY`, `getModesForCodec()` 등)라는 단일 소스가 있어 부분적으로 해결했지만, `frontend/components/panels/SyntaxDetailPanel/index.tsx:85-90`·`frontend/components/Player/views/DeblockingView.tsx:318-343`는 이 registry를 우회해 여전히 코덱 하드코딩 분기를 쓴다(PLUGIN-012와 동일 코드, 경로 둘 다 현재도 유효).

---

### PLUGIN-006: codec 이름 문자열 비교
**분류**: 타입 안전성 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn is_hevc(codec_name: &str) -> bool {
    codec_name == "hevc" // 호출부에 따라 "HEVC", "h265", "H.265"가 섞여 들어온다
}

fn dispatch(codec_name: &str, data: &[u8]) {
    match codec_name {
        "avc" | "h264" | "H264" => bitvue_avc::parse(data),
        "hevc" => bitvue_hevc::parse(data),
        // "HEVC" (대문자)로 들어오면 기본 분기로 빠져서 알 수 없는 코덱 취급됨
        _ => panic!("unknown codec: {codec_name}"),
    };
}
```

**문제**:
- 코덱 식별자가 문자열이면 오타, 대소문자 불일치, 별칭(`h264` vs `avc`, `h265` vs `hevc`)이 코드베이스 곳곳에서 서로 다르게 처리되어 "같은 코덱인데 다르게 취급되는" 버그가 생긴다.
- 컴파일러가 exhaustiveness를 검증해줄 수 없다 — 새 코덱을 추가했는데 특정 문자열 비교 지점 하나를 빠뜨려도 컴파일은 성공하고 런타임에만 조용히 틀리게 동작한다.
- IPC/파일 경계(설정 파일, 세션 저장, Tauri command 인자)에서 문자열을 그대로 주고받으면 이 문제가 프로세스 경계를 넘어서까지 전파된다.

**발생 조건**:
- 컨테이너 메타데이터(fourcc, MIME 서브타입 등)에서 얻은 원시 문자열을 파싱 없이 그대로 내부 로직에 흘려보낼 때.
- 여러 개발자가 각자 다른 파일에서 "이 코덱인지 확인하는" 헬퍼를 독립적으로 작성했을 때(중복 구현이 서로 다른 정규화 규칙을 가짐).

**권장**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodecId { Avc, Hevc, Vp9, Av1, Vvc, Av3, Mpeg2 }

impl std::str::FromStr for CodecId {
    type Err = UnknownCodecError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "avc" | "h264" | "h.264" => Ok(CodecId::Avc),
            "hevc" | "h265" | "h.265" => Ok(CodecId::Hevc),
            "vp9" => Ok(CodecId::Vp9),
            "av1" => Ok(CodecId::Av1),
            other => Err(UnknownCodecError(other.to_string())),
        }
    }
}
// 문자열 파싱은 시스템 경계(fourcc 디코딩, CLI 인자)에서 딱 한 번만 일어나고,
// 그 안쪽 모든 로직은 CodecId를 주고받는다
```
- 외부 경계(fourcc/MIME/CLI/저장 파일)에서만 문자열 → `CodecId` 변환을 수행하고, 그 즉시 강타입으로 전환해 내부로 전파한다.
- `match`는 항상 `CodecId` enum에 대해서만 수행해 컴파일러가 exhaustiveness를 보장하게 한다.

**탐지 방법**:
- Static: `codec_name == "..."`, `codec.as_str() == "..."` 패턴을 grep해 문자열 비교 지점을 나열.
- Structural: 코덱 식별에 `&str`/`String` 타입이 함수 시그니처를 넘나드는 곳이 코덱 파싱 경계 바깥에도 있는지 확인.

**예외**:
- 로깅/디버그 출력, 사용자에게 보여줄 display name 등 "보여주기 전용" 경로에서는 문자열이 자연스럽다 — 문제는 문자열이 *비교/분기 로직*의 근거가 될 때다.

**Bitvue 판정**: Confirmed — (2026-08-18 재검증, `src-tauri` 경로는 Electron 전환으로 소멸해 폐기) 현재 실사용 경로 기준: `crates/bitvue-engine/src/index_extractor.rs:663`(`ExtractorFactory::create`)와 `:681`(`from_extension`)이 `codec.to_lowercase().as_str()`/`ext.to_lowercase().as_str()` match+별칭 리스트("h264"|"h.264"|"avc" 등). `bitvue-cli/src/commands/decode.rs`는 오히려 권장 패턴에 가까운 `ForceCodec` enum(라인 19-29)을 쓰지만, `resolve_codec()`(라인 207)이 IVF FourCC 바이트를 파싱해 이 enum으로 변환하는 지점 자체는 여전히 raw byte-string 매칭. 프런트엔드는 동명의 `CodecType` 타입이 두 곳에서 다른 대소문자 컨벤션으로 정의됨(`frontend/types/video.ts:254` UPPERCASE enum vs `frontend/hooks/useFileOperations.ts:29` lowercase union), 게다가 `SyntaxDetailPanel/index.tsx:32`는 셋 중 어느 것도 아닌 자체 `detectCodecFromPath(): string`(lowercase)을 씀 — 나쁜 예가 그대로 재현됨.

---

### PLUGIN-007: version negotiation 없음
**분류**: 호환성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-decode/src/registry.rs
pub trait CodecPlugin {
    fn parse(&self, data: &[u8]) -> Result<Box<dyn ParsedUnit>, ParseError>;
    fn capabilities(&self) -> CodecCapabilities;
    // v2에서 이 메서드가 새로 추가됨
    fn metric_dependencies(&self) -> &[MetricId];
}
// 기존에 이 trait을 구현한 코덱 크레이트가 있다면
// 새 메서드가 추가되는 즉시 컴파일이 깨지거나(트레이트 오브젝트 안전성 붕괴),
// default impl을 넣어 조용히 넘어가면 그 코덱은 "의존성 없음"으로
// 잘못 취급되어 런타임에만 드러나는 버그가 된다
```

**문제**:
- plugin trait 자체에 버전 개념이 없으면, trait에 메서드를 추가/변경할 때마다 "이미 구현된 모든 플러그인이 새 계약을 만족하는가"를 사람이 일일이 확인해야 한다.
- `default impl`로 하위 호환을 흉내 내면 컴파일은 통과하지만, 새 메서드의 의미론적 요구사항(예: "의존하는 metric을 정확히 나열해야 함")을 구현하지 않은 플러그인이 잘못된 기본값으로 조용히 동작한다.
- 세션 파일/캐시 포맷처럼 plugin의 출력이 디스크에 저장되는 경우, plugin 코드는 업그레이드됐는데 저장된 데이터의 스키마 버전을 모르면 구버전 데이터를 신버전 플러그인이 잘못 해석할 위험이 있다.

**발생 조건**:
- 코덱 플러그인 trait에 새 필수 기능(신규 capability, 신규 메타데이터)을 추가하는 리팩터링을 할 때.
- 플러그인이 만들어내는 직렬화 포맷(세션 저장, 캐시)이 core보다 독립적으로 진화할 때.

**권장**:
```rust
pub const PLUGIN_API_VERSION: u32 = 2;

pub trait CodecPlugin {
    fn api_version(&self) -> u32 { PLUGIN_API_VERSION }
    fn parse(&self, data: &[u8]) -> Result<Box<dyn ParsedUnit>, ParseError>;
    fn capabilities(&self) -> CodecCapabilities;
    fn metric_dependencies(&self) -> &[MetricId];
}

pub fn register(plugin: &'static dyn CodecPlugin) -> Result<(), RegistryError> {
    if plugin.api_version() != PLUGIN_API_VERSION {
        return Err(RegistryError::ApiVersionMismatch {
            expected: PLUGIN_API_VERSION,
            got: plugin.api_version(),
        });
    }
    REGISTRY.lock().unwrap().push(plugin);
    Ok(())
}
```
- trait 자체에 `api_version`을 두고, 등록 시점에 host가 명시적으로 검증한다 — 컴파일러 default impl에 기대지 않는다.
- trait을 깨는 변경(breaking change)을 할 때는 버전을 올리고, 마이그레이션 가이드(어떤 메서드가 추가/변경됐고 각 플러그인이 무엇을 해야 하는지)를 문서화한다.
- 직렬화되는 데이터에는 별도의 `schema_version` 필드를 둬 코드 버전과 데이터 버전을 독립적으로 추적한다.

**탐지 방법**:
- Static: `trait CodecPlugin`의 변경 이력에 `api_version` 유사 필드가 없는지 확인.
- Structural: trait에 default impl이 있는 메서드 수를 감사 — "필수인데 default로 눈속임하는" 메서드가 있는지.

**예외**:
- 워크스페이스 내부에서만 쓰이고 host/plugin이 항상 동시에 컴파일·배포되는 구조(현재 Bitvue처럼)에서는, 컴파일러의 exhaustive trait 구현 검증 자체가 사실상 버전 negotiation 역할을 한다 — 별도의 런타임 버전 필드는 과설계일 수 있다. 다만 직렬화 스키마 버전은 이 경우에도 여전히 유효한 관심사다(코드는 동시 배포되지만 저장된 옛 세션 파일은 그렇지 않으므로).

**Bitvue 판정**: N/A — (2026-08-18 재검증) `crates/bitvue-codecs-parser/src/parser_strategy.rs:222` `trait ParserStrategy`(+ `ParserFactory::create`, 라인 804)가 문서의 가상 `CodecPlugin`과 정확히 같은 모양으로 실존하긴 하지만, `api_version` 유사 필드 없음은 동일하고 무엇보다 이 trait/factory를 소비하는 코드가 자기 자신의 테스트 외에 워크스페이스 어디에도 없음(`bitvue-sidecar`/`bitvue-cli` 전부 개별 코덱 크레이트에 직접 의존, 이 crate를 감싸는 상위 facade crate `crates/bitvue/src/lib.rs`도 그 자체를 의존하는 크레이트가 전무) — 사실상 죽은 스캐폴드(각 코덱의 `parse_frame`도 `bytes_consumed: min(len,100)` placeholder 뿐, 실제 파싱 없음)라 trait이 "진화"할 대상 자체가 없어 버전 negotiation 문제가 발현될 여지가 없음.

---

### PLUGIN-008: plugin별 thread model 불명확
**분류**: 동시성 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
// bitvue-av1은 내부적으로 rayon을 써서 tile을 병렬 파싱한다
impl CodecPlugin for Av1Plugin {
    fn parse(&self, data: &[u8]) -> Result<Box<dyn ParsedUnit>, ParseError> {
        let tiles = split_tiles(data);
        tiles.par_iter().map(|t| parse_tile(t)).collect() // 내부에 자체 rayon 풀 사용
    }
}

// host는 이미 프레임 단위로 rayon을 사용해 여러 코덱 유닛을 동시에 파싱한다
frames.par_iter().for_each(|f| {
    dispatch(f.codec, &f.data); // AV1 프레임 안에서 또 par_iter가 걸림
});
// 결과: 중첩된 스레드풀이 CPU 코어 수 대비 과다한 스레드를 스폰하거나,
// rayon의 전역 풀을 다른 플러그인과 경합하며, HEVC 플러그인은 반대로
// "호출자가 이미 스레드를 나눠줄 것"이라 가정하고 단일 스레드로만 동작해
// 코어를 놀린다 — 코덱마다 가정이 다른데 아무도 문서화하지 않았다
```

**문제**:
- host는 "이 플러그인을 동시에 몇 번 호출해도 되는지", "플러그인이 내부적으로 스레드를 스폰하는지"를 알 방법이 없어, 최악의 경우(중첩 스레드풀, 과다 컨텍스트 스위칭) 혹은 최선을 놓치는 경우(단일 스레드 플러그인을 순차 호출) 중 하나로 귀결된다.
- 코덱 크레이트마다 `Send`/`Sync` 여부, 재진입(reentrant) 가능 여부가 암묵적으로만 정해져 있으면, 컴파일러가 `Send + Sync` bound로 일부는 잡아주지만 "내부 전역 캐시에 락 없이 접근"처럼 타입 시스템이 못 잡는 데이터 레이스는 그대로 남는다.
- GUI 세션에서 "동시에 여러 프레임을 프리페치하며 파싱"하는 흔한 최적화가, thread model이 불명확한 플러그인 하나 때문에 간헐적으로만 재현되는 디버깅하기 매우 어려운 버그를 만든다.

**발생 조건**:
- 여러 코덱 크레이트가 서로 다른 시점에, 서로 다른 개발자에 의해 "일단 되게" 작성되어 병렬화 전략이 통일되지 않았을 때.
- host 쪽에서 프레임 파이프라인에 새로운 병렬화 레이어(프리페치, 배치 처리)를 추가할 때 기존 플러그인들의 thread 가정을 재검토하지 않을 때.

**권장**:
```rust
pub trait CodecPlugin: Send + Sync {
    /// 이 플러그인의 parse()가 내부적으로 스레드를 스폰하는지 여부.
    /// true면 host는 이 플러그인을 자신의 병렬 루프 안에서 추가로 par_iter 하지 않는다.
    fn threading_model(&self) -> ThreadingModel;
}

pub enum ThreadingModel {
    /// 호출자가 원하는 만큼 동시에 호출해도 안전 (내부 전역 상태 없음)
    ReentrantSingleThreaded,
    /// 내부적으로 자체 스레드풀을 사용 — host는 동시 호출 수를 제한해야 함
    InternallyParallel,
}
```
- 각 플러그인이 자신의 thread model을 capability처럼 명시적으로 선언하게 하고, host의 스케줄러가 이를 존중해 병렬도를 조정한다.
- 워크스페이스 전체가 하나의 병렬화 전략(예: 전역 rayon 풀 하나만 사용하고 코덱 크레이트는 절대 자체 풀을 만들지 않는다)을 컨벤션으로 강제하는 것이 가장 단순하고 안전한 대안이다.

**탐지 방법**:
- Static: 코덱 크레이트 내부에서 `rayon::ThreadPoolBuilder`, `std::thread::spawn`, 자체 풀 생성이 있는지 grep.
- Runtime: 여러 코덱을 동시에 여는 스트레스 테스트에서 CPU 사용률/컨텍스트 스위치 수를 프로파일링 — 코어 수 대비 비정상적으로 높은 스레드 수가 관찰되면 신호.

**예외**:
- 워크스페이스에 단 하나의 전역 병렬화 지점만 있고(예: 최상위 프레임 루프에서만 `par_iter`, 그 아래로는 모두 순차) 모든 코덱 크레이트가 이 컨벤션을 코드 리뷰로 강제한다면, 명시적 `ThreadingModel` API 없이도 문제가 발생하지 않는다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) 코덱 파서 크레이트(bitvue-avc/hevc/vp9/vvc/av1-codec/av3-codec/mpeg2-codec/avs3/jpegxs/vc3) `src/` 어디에도 `rayon`/`ThreadPoolBuilder`/`thread::spawn`이 없음(grep 0건) — 코덱 파서들은 사실상 전부 순차 실행. `rayon`은 host 레벨(`bitvue-cli`, `--md5` 배치 처리용, `Cargo.toml`에 명시)에만 등장하고 코덱 크레이트 내부로 스며들지 않아 중첩 스레드풀 문제가 발생할 여지 자체가 없음.

---

### PLUGIN-009: plugin result가 공통 giant enum으로 합쳐짐
**분류**: 확장성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub enum ParsedUnit {
    Avc(AvcSlice),
    Hevc(HevcSlice),
    Vp9(Vp9Frame),
    Av1(Av1Frame),
    Vvc(VvcSlice),
    Av3(Av3Frame),
    Mpeg2(Mpeg2Picture),
    // 코덱이 늘어날 때마다 variant가 늘어난다
}

// 이 enum을 소비하는 모든 곳(overlay 렌더러, hex-view, 통계 집계, 세션 직렬화...)이
// 전부 이런 exhaustive match를 갖고 있다
fn frame_type_label(unit: &ParsedUnit) -> &'static str {
    match unit {
        ParsedUnit::Avc(s) => avc_slice_type_label(s.slice_type),
        ParsedUnit::Hevc(s) => hevc_slice_type_label(s.slice_type),
        ParsedUnit::Vp9(f) => vp9_frame_type_label(f.frame_type),
        ParsedUnit::Av1(f) => av1_frame_type_label(f.frame_type),
        ParsedUnit::Vvc(s) => vvc_slice_type_label(s.slice_type),
        ParsedUnit::Av3(f) => av3_frame_type_label(f.frame_type),
        ParsedUnit::Mpeg2(p) => mpeg2_picture_type_label(p.picture_coding_type),
    }
}
```

**문제**:
- PLUGIN-001과 대칭적인 문제지만 이번엔 "입력 dispatch"가 아니라 "출력 타입"에서 발생한다 — 코덱을 하나 추가할 때마다 이 giant enum과, 이를 소비하는 모든 consumer 함수의 match가 함께 늘어난다.
- consumer 함수 하나하나가 "모든 코덱을 알아야 한다"는 요구사항을 갖게 되어, 원래는 코덱 플러그인 내부에 있어야 할 로직(`frame_type_label`처럼 코덱별 지식이 필요한 변환)이 core 레이어로 흘러나온다.
- 이 giant enum이 크면 클수록(코덱이 늘수록) `size_of::<ParsedUnit>()`도 가장 큰 variant에 맞춰 커져, 작은 코덱(예: MPEG-2)의 결과도 불필요하게 큰 메모리를 차지하게 된다(Box로 감싸지 않은 경우).

**발생 조건**:
- "일단 매치 가능한 하나의 enum이 있으면 편하다"는 이유로 초기 설계 단계에서 만들어졌다가, 코덱이 늘어나면서 부채가 누적될 때.
- 세션 직렬화 포맷이 이 enum을 그대로 `serde`로 직렬화하고 있어, enum 구조를 바꾸는 리팩터링이 저장 포맷 마이그레이션까지 동반해야 할 때(더 굳어짐).

**권장**:
```rust
pub trait ParsedUnit: Send + Sync {
    fn codec(&self) -> CodecId;
    fn frame_type_label(&self) -> &'static str; // 코덱별 지식은 구현체 안에 캡슐화
    fn as_any(&self) -> &dyn std::any::Any; // 필요할 때만 downcast로 코덱별 접근
}

impl ParsedUnit for AvcSlice {
    fn codec(&self) -> CodecId { CodecId::Avc }
    fn frame_type_label(&self) -> &'static str { avc_slice_type_label(self.slice_type) }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// consumer는 코덱을 몰라도 공통 동작을 호출할 수 있다
fn frame_type_label(unit: &dyn ParsedUnit) -> &'static str { unit.frame_type_label() }

// 정말 코덱별 특수 처리가 필요한 소수의 소비자만 downcast
if let Some(av1) = unit.as_any().downcast_ref::<Av1Frame>() {
    // AV1 전용 CDEF 정보 접근 등
}
```
- 공통으로 필요한 동작(`frame_type_label`, `is_keyframe`, `qp_summary` 등)은 trait 메서드로 정의해 각 코덱 구현체 안에 캡슐화하고, consumer는 trait object(`Box<dyn ParsedUnit>` / `&dyn ParsedUnit`)만 다룬다.
- 정말 코덱별 특수 처리가 필요한 소수 지점(예: AV1 전용 CDEF overlay)만 `Any::downcast_ref`로 명시적으로 예외 처리한다 — "가끔 필요한 특수 케이스"와 "항상 모든 코덱을 알아야 하는 공통 경로"를 구분하는 것이 핵심.

**탐지 방법**:
- Structural: 하나의 enum에 대한 `match`가 워크스페이스 내 몇 개 파일에 등장하는지 세어본다 — 코덱 크레이트 외부에 5곳 이상이면 경고.
- Static: `enum ParsedUnit`류의 정의에서 variant 수와 코덱 크레이트 수를 비교해 1:1 대응 여부 확인.

**예외**:
- consumer가 정말로 코덱별 처리가 근본적으로 다를 수밖에 없는 경우(예: 코덱마다 완전히 다른 렌더링 UI가 필요한 overlay 패널)라면, giant enum이나 trait object보다 명시적 match가 오히려 "이 코드가 코덱별로 분기한다"는 사실을 더 정직하게 드러낸다. 이 경우엔 PLUGIN-012(코덱별 UI 하드코딩)와의 경계를 신중히 그어야 한다 — 문제는 giant enum 자체가 아니라 "공통 동작까지 강제로 giant enum을 거치게 만드는 것"이다.

**Bitvue 판정**: Confirmed — (2026-08-18 재검증, 경로만 `bitvue-core`→`bitvue-engine` 개명, 내용 불변) `crates/bitvue-engine/src/frame.rs:226` `pub enum CodecMetadata { None, Avc{..}, Hevc{..}, Vp9{..}, Av1{..} }`가 나쁜 예와 동일 패턴이며, 여전히 Vvc/Av3/Mpeg2/Avs3/JpegXs/Vc3 variant가 누락되어 있음(코덱 크레이트는 10개인데 4개만 커버) — "코덱 늘 때 잊고 안 고침"이 실물로 확인됨. 다만 이 enum을 소비하는 곳이 아직 없어(정의부 `frame.rs` 자신 외 grep 0건) consumer 폭발 피해 자체는 미발현.

---

### PLUGIN-010: host allocator와 plugin allocator 혼용
**분류**: 메모리 안전성 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
// bitvue-hevc가 내부적으로 커스텀 아레나(bump allocator)를 써서 슬라이스 파싱 중
// 생성되는 임시 노드들을 빠르게 할당한다
pub struct HevcArena {
    bump: bumpalo::Bump,
}

impl HevcArena {
    pub fn parse_slice<'a>(&'a self, data: &[u8]) -> &'a SliceHeader {
        self.bump.alloc(parse_internal(data)) // 아레나 라이프타임에 묶인 참조
    }
}

// host가 아레나보다 오래 사는 곳에 참조를 저장해버림
struct FrameCache {
    // 'static이 아닌데 캐시에 넣으려고 unsafe transmute로 라이프타임을 늘림
    slices: Vec<&'static SliceHeader>,
}
fn cache_slice(cache: &mut FrameCache, arena: &HevcArena, data: &[u8]) {
    let header = arena.parse_slice(data);
    let extended: &'static SliceHeader = unsafe { std::mem::transmute(header) };
    cache.slices.push(extended); // arena가 drop되면 dangling reference
}
```

**문제**:
- 코덱 플러그인이 자체 아레나/풀 할당자를 쓰고 그 결과를 참조(라이프타임이 아레나에 묶인 `&'a T`)로 돌려주는데, host가 이를 아레나보다 오래 살아야 하는 곳(캐시, 다른 스레드로 전달)에 저장하려 하면 라이프타임이 맞지 않는다 — 이를 `unsafe transmute`로 억지로 늘리면 아레나 drop 이후 dangling reference를 읽는 UB가 된다.
- 여러 코덱이 각자 다른 할당 전략(하나는 아레나, 하나는 시스템 기본 할당자, 하나는 커스텀 풀)을 쓰면 host는 "이 결과를 얼마나 오래 들고 있어도 되는지"를 코덱마다 다르게 알아야 한다.
- 진짜 동적 로딩(dylib)이 개입하는 미래 시나리오에서는 이 문제가 더 심각해진다 — plugin이 자신의 allocator로 할당한 메모리를 host가 host의 allocator로 `free`하면 즉시 UB.

**발생 조건**:
- 파싱 성능을 위해 코덱 크레이트 내부에서 임시 노드용 아레나 할당자를 도입했는데, 그 결과를 빌린(borrow) 채로 host가 더 오래 보관하려 할 때.
- 코덱별로 "결과를 소유(owned)로 반환하는지 빌린 참조로 반환하는지"에 대한 일관된 규약이 없을 때.

**권장**:
```rust
// 아레나는 파싱 함수의 로컬 스코프 안에서만 쓰고,
// 함수 경계를 넘어가는 결과는 항상 소유권을 가진 안전한 타입으로 변환한다
pub fn parse_slice(data: &[u8]) -> SliceSummary { // 'a 라이프타임이 시그니처에 없음
    let arena = bumpalo::Bump::new();
    let raw = parse_internal(data, &arena); // 아레나는 이 함수 안에서만 산다
    SliceSummary::from(raw) // owned 데이터로 복사해 반환 — 아레나가 죽어도 안전
}
```
- 플러그인 경계(trait 메서드의 반환 타입)에는 항상 소유권을 가진(owned) 타입만 사용한다. 내부 성능 최적화(아레나, 풀 할당자)는 그 함수 내부에 완전히 숨긴다.
- 전역 할당자를 하나로 통일(`#[global_allocator]`)하면 최소한 "서로 다른 할당자를 섞어 쓰다 발생하는 free 불일치" 클래스의 버그는 원천 차단된다.
- 정말 아레나를 경계 너머로 공유해야 한다면(고성능 요구), 아레나의 소유권/라이프타임을 타입 시스템으로 명시(`'arena` 라이프타임 파라미터를 host까지 전파)하고 `unsafe transmute`로 우회하지 않는다.

**탐지 방법**:
- Static: `unsafe { std::mem::transmute }`로 라이프타임을 늘리는 코드를 grep — 특히 `'static`으로의 transmute는 강한 위험 신호.
- Runtime: Miri나 AddressSanitizer로 파싱 경로를 실행해 use-after-free를 탐지.

**예외**:
- 아레나가 함수 호출 하나의 스코프 안에서 생성되고 소비되며 절대 함수 경계를 넘어가지 않는다면(가장 흔하고 안전한 사용법) 문제 없다. 이 항목이 경고하는 것은 "아레나 참조가 함수 경계, 특히 plugin-host 경계를 넘어가는" 경우다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) 워크스페이스 어떤 `Cargo.toml`에도 `bumpalo` 의존성이 없음(grep 0건). `mem::transmute` 사용은 `crates/bitvue-metrics/src/simd.rs`(SIMD 레지스터 재해석)와 테스트 코드(`bitvue-engine/src/tests/endianness_edge_cases_test.rs`) 뿐 — host/plugin 경계를 넘는 라이프타임 연장 사례 없음.

---

### PLUGIN-011: plugin unload 후 callback 생존
**분류**: 생명주기 관리 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
// 가상의 동적 plugin 시나리오: 코덱 플러그인이 progress callback을 등록한다
pub struct ProgressRegistry {
    callbacks: Vec<Box<dyn Fn(f32) + Send>>,
}

impl ProgressRegistry {
    pub fn register(&mut self, cb: Box<dyn Fn(f32) + Send>) {
        self.callbacks.push(cb); // 이 클로저가 plugin dylib 안의 함수 포인터를 캡처
    }
}

// plugin dylib을 dlclose()로 언로드한 뒤에도 registry.callbacks에는
// 여전히 그 dylib 코드 세그먼트를 가리키는 함수 포인터가 남아있다
fn unload_plugin(handle: libloading::Library) {
    drop(handle); // dylib이 프로세스 메모리에서 언맵됨
    // registry.callbacks 안의 클로저는 여전히 살아있고,
    // 다음번 progress 업데이트에서 이를 호출하면 언맵된 코드 영역을 실행 -> 크래시
}
```

**문제**:
- 동적으로 로드된 plugin이 host의 전역/장수(long-lived) 자료구조(콜백 레지스트리, 이벤트 리스너, 캐시)에 자신의 코드를 참조하는 값(함수 포인터, 클로저, trait object)을 남겨두면, plugin이 언로드된 뒤 그 참조가 dangling된다.
- 이 클래스의 버그는 재현이 매우 비결정적이다 — 언로드 직후 바로 크래시하지 않고, 메모리가 다른 목적으로 재사용된 한참 뒤에야 이상한 동작이나 크래시로 나타난다.
- host가 "이 plugin이 무엇을 등록했는지"를 추적하지 않으면, 언로드 시점에 무엇을 정리해야 하는지조차 알 수 없다.

**발생 조건**:
- **Bitvue 현재 상태와의 구분**: 이 항목은 dylib을 `dlopen`/`dlclose`(또는 Rust의 `libloading`)로 런타임에 로드/언로드하는 진짜 동적 plugin 시스템을 전제로 한다. Bitvue의 코덱은 현재 컴파일 타임에 정적 링크되는 워크스페이스 크레이트이며 프로세스 수명 동안 "언로드"라는 개념 자체가 없으므로, 이 항목은 지금 당장 해당 위험이 존재하지 않는 전방주시적 참조 항목이다 — 향후 서드파티 플러그인을 런타임에 교체/제거할 수 있는 기능(예: "코덱 플러그인 설정에서 비활성화")을 추가할 경우에 대비해 기록해 둔다.
- 그런 기능이 생긴다면: 사용자가 UI에서 "이 코덱 플러그인 비활성화" 후 재활성화하거나, 플러그인 hot-reload(개발 중 dylib 재빌드 후 자동 재로드)를 지원하는 경우 즉시 문제가 된다.

**권장**:
```rust
// 콜백을 plugin 핸들의 소유권 아래 묶어, 언로드 시 자동으로 함께 정리되게 한다
pub struct PluginHandle {
    library: libloading::Library,
    registered_callbacks: Vec<CallbackId>,
}

impl Drop for PluginHandle {
    fn drop(&mut self) {
        for id in &self.registered_callbacks {
            global_registry().unregister(*id); // 언로드 전에 반드시 등록 해제
        }
        // 이 시점 이후에 self.library가 drop되어 언맵된다
    }
}
```
- 콜백/리스너 등록에는 항상 대응하는 해제(unregister) 경로를 만들고, plugin의 `Drop`/`shutdown()` 훅에서 명시적으로 호출되게 강제한다(등록만 있고 해제가 옵션인 API를 만들지 않는다).
- 가능하면 dylib을 아예 언로드하지 않는 전략(프로세스 수명 동안 로드된 채로 유지, "비활성화"는 언로드가 아니라 단순히 호출을 건너뛰는 플래그로 구현)을 택해 이 클래스의 버그 전체를 회피한다.
- 콜백을 raw 함수 포인터/클로저 대신 `Weak` 참조나 ID 기반 간접 참조로 설계해, plugin이 죽어도 host가 이를 감지하고 안전하게 무시할 수 있게 한다.

**탐지 방법**:
- Runtime: 로드→언로드→콜백 트리거 시나리오를 반복하는 stress test를 AddressSanitizer 아래에서 실행.
- Structural: `register_callback` 류의 API가 대응하는 `unregister`를 가지고 있는지, 그리고 plugin 언로드 경로가 이를 호출하는지 감사.

**예외**:
- 정적 링크된 컴파일 타임 크레이트(현재 Bitvue)에는 적용되지 않는다 — "언로드"라는 이벤트가 프로세스 종료와 같으므로 dangling 참조 문제 자체가 발생하지 않는다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) PLUGIN-003과 동일 근거로 dynamic loading/unloading 개념 자체가 없음(`libloading`/`dlopen`/`wasmtime` 의존성 0건) — "언로드" 이벤트가 존재하지 않으므로 dangling callback 위험이 발생할 여지가 없음. Electron sidecar 프로세스(`bitvue-sidecar`) 자체의 생명주기는 OS 프로세스 단위이지 코덱 plugin 단위 unload가 아니므로 이 항목과 무관.

---

### PLUGIN-012: codec-specific UI가 core UI에 하드코딩
**분류**: 관심사 분리 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```tsx
// frontend/components/OverlayPanel.tsx — core UI 컴포넌트
function OverlayPanel({ codec, frame }: Props) {
  if (codec === "hevc") {
    return <HevcMbTypeOverlay frame={frame} />;
  } else if (codec === "avc") {
    return <AvcMbTypeOverlay frame={frame} />;
  } else if (codec === "av1") {
    return <Av1CdefOverlay frame={frame} />;
  }
  // VP9를 새로 지원할 때 이 컴포넌트를 또 고쳐야 하고,
  // 이 파일을 아는 사람만 그 사실을 알 수 있다
  return null;
}
```

**문제**:
- core UI 컴포넌트가 모든 코덱의 존재와 각 코덱에 맞는 하위 컴포넌트를 직접 알아야 하므로, PLUGIN-001과 동일한 "중앙 수정 지점" 문제가 프론트엔드에서도 반복된다.
- 코덱별 오버레이 컴포넌트가 core UI 파일에 import되어 있으면, 코드 스플리팅/지연 로딩이 어려워지고 사용하지 않는 코덱의 UI 코드까지 번들에 포함될 수 있다.
- 새 코덱을 담당하는 개발자가 파서/코덱 로직뿐 아니라 core UI 파일까지 수정 권한과 이해가 필요해, 코덱 크레이트의 "플러그인다움"이 프론트엔드에서 깨진다.

**발생 조건**:
- 코덱별 오버레이/패널을 처음 하나둘 추가할 때는 if/else가 빠르고 간단해 보이지만, 코덱이 4~5개를 넘어가면서 컴포넌트가 비대해질 때.
- Rust 쪽에서는 PLUGIN-001~005로 registry 패턴을 잘 도입해놓고, 프론트엔드 쪽에는 대칭적인 registry가 없어 그 경계에서만 결합도가 남을 때.

**권장**:
```tsx
// registry/overlays.ts — 각 코덱 모듈이 자신의 오버레이를 등록
type OverlayDescriptor = {
  codec: CodecId;
  id: string;
  label: string;
  Component: React.ComponentType<{ frame: FrameData }>;
};

const overlayRegistry: OverlayDescriptor[] = [];
export function registerOverlay(desc: OverlayDescriptor) { overlayRegistry.push(desc); }

// codecs/hevc/overlays.ts
registerOverlay({ codec: "hevc", id: "mb-type", label: "MB Type", Component: HevcMbTypeOverlay });

// components/OverlayPanel.tsx — core는 코덱 이름을 모른다
function OverlayPanel({ codec, frame }: Props) {
  const overlays = overlayRegistry.filter(o => o.codec === codec);
  return <>{overlays.map(o => <o.Component key={o.id} frame={frame} />)}</>;
}
```
- Rust 쪽 registry 패턴(PLUGIN-001)과 대칭적으로, 프론트엔드에도 코덱별 모듈이 자신의 오버레이/패널을 등록하는 registry를 둔다.
- 번들 크기가 우려되면 `React.lazy` + dynamic import로 코덱별 오버레이 모듈을 지연 로딩한다.

**탐지 방법**:
- Static: core UI 컴포넌트 파일에서 `codec === "..."` 문자열/enum 분기 개수를 grep.
- Structural: core UI 디렉터리가 코덱별 하위 컴포넌트를 몇 개 import하는지 집계 — 늘어나는 추세라면 registry 부재 신호.

**예외**:
- 코덱별 UI 차이가 극히 사소하고(라벨 텍스트 한두 개 차이) 코덱 수가 적게 고정되어 있다면, registry 인프라를 만드는 비용이 if/else 몇 줄을 유지하는 비용보다 클 수 있다.

**Bitvue 판정**: Confirmed — `frontend/components/panels/SyntaxDetailPanel/index.tsx:85-90`(`codec === "hevc"/"vp9"/"vvc"`)와 `frontend/components/Player/views/DeblockingView.tsx:338-365`(`codec === "AV1"/"HEVC"/"VVC"/"AVC"/"VP9"`)가 core UI 컴포넌트 안에서 코덱별 분기를 직접 하드코딩— 정작 같은 저장소에 이미 있는 `frontend/utils/codecModeRegistry.ts` 중앙 registry를 우회함.

---

### PLUGIN-013: metric dependency를 런타임까지 발견하지 못함
**분류**: 의존성 관리 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// QP heatmap metric은 MB-type 분석이 이미 실행되어 채워놓은 필드를 읽는다
// 하지만 이 의존성이 코드 어디에도 선언되어 있지 않다
pub struct QpHeatmapMetric;

impl Metric for QpHeatmapMetric {
    fn compute(&self, frame: &AnalyzedFrame) -> MetricResult {
        // frame.mb_types가 None이면(사용자가 MB-type 분석을 켜지 않았다면)
        // 조용히 0/기본값으로 채워진 엉뚱한 히트맵이 나온다 — 에러도, 경고도 없이
        let mb_types = frame.mb_types.as_ref().unwrap_or(&EMPTY_MB_TYPES);
        build_heatmap(mb_types, frame.qp_map.as_ref())
    }
}
```

**문제**:
- metric이 다른 analysis 단계의 산출물에 암묵적으로 의존하는데, 그 의존관계가 코드 어디에도 선언되어 있지 않으면 "사용자가 필요한 사전 분석을 안 켰다"는 사실이 에러가 아니라 조용히 틀린 결과(0으로 채워진 히트맵, 빈 그래프)로 나타난다.
- 이 버그는 "왜 히트맵이 다 회색이지?"처럼 사용자가 화면을 한참 들여다본 뒤에야 알아채는 형태로 드러나서, 근본 원인(사전 분석 미실행)까지 거슬러 올라가는 데 오래 걸린다.
- metric을 추가하는 개발자가 의존관계를 명시적으로 선언하지 않아도 컴파일이 되고 테스트도 (해당 사전 분석을 항상 켜놓은 채로 작성했다면) 통과하므로, 이 문제는 리뷰에서도 잡히지 않는다.

**발생 조건**:
- 신규 metric이 기존 analysis pass의 출력을 재사용하려 할 때(예: reference-frame 그래프가 motion-vector 분석 결과를 재사용).
- 사용자가 성능을 위해 일부 분석을 선택적으로 끌 수 있는 UI(예: "MB-type 분석 건너뛰기" 옵션)가 있을 때.

**권장**:
```rust
pub trait Metric {
    fn id(&self) -> MetricId;
    fn depends_on(&self) -> &[MetricId]; // 명시적 선언
    fn compute(&self, frame: &AnalyzedFrame) -> MetricResult;
}

impl Metric for QpHeatmapMetric {
    fn id(&self) -> MetricId { MetricId::QpHeatmap }
    fn depends_on(&self) -> &[MetricId] { &[MetricId::MbType, MetricId::QpMap] }
    fn compute(&self, frame: &AnalyzedFrame) -> MetricResult { /* ... */ }
}

// host: 실행 계획을 세울 때 위상 정렬하고, 의존성이 비활성화되어 있으면
// 실행 전에 명확한 에러/경고로 사용자에게 알린다
fn plan_execution(requested: &[MetricId], enabled: &HashSet<MetricId>) -> Result<Vec<MetricId>, PlanError> {
    for m in requested {
        for dep in registry::get(*m).depends_on() {
            if !enabled.contains(dep) {
                return Err(PlanError::MissingDependency { metric: *m, dependency: *dep });
            }
        }
    }
    Ok(topo_sort(requested))
}
```
- 각 metric이 `depends_on()`으로 자신의 선행 조건을 명시적으로 선언하게 하고, host는 실행 계획(plan) 단계에서 이를 위상 정렬하거나 누락을 즉시 에러로 보고한다.
- "조용히 기본값으로 채워서 진행"을 절대 기본 동작으로 삼지 않는다 — 의존성 미충족은 항상 명시적 에러/경고여야 한다.

**탐지 방법**:
- Semantic: metric 구현체가 `frame.<field>.unwrap_or_default()` / `.unwrap_or(&EMPTY_*)` 패턴으로 다른 analysis pass의 출력에 접근하면서 `depends_on`에 그 필드를 만드는 metric이 나열되어 있지 않은 경우를 코드 리뷰/lint로 탐지.
- Runtime: 사전 분석을 의도적으로 끈 상태에서 각 metric을 실행하는 조합 테스트를 CI에 추가해, 조용한 기본값 대체가 아니라 명시적 에러가 나는지 확인.

**예외**:
- 모든 사전 분석이 항상 무조건 함께 실행되는 아키텍처(선택적 비활성화 자체가 불가능)라면 의존성 선언이 실질적 가치가 낮을 수 있다. 다만 이 경우도 문서화 목적으로는 여전히 유용하다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) `bitvue-metrics`는 PSNR/SSIM/VMAF 계산 함수 모음일 뿐 Metric plugin/의존성 그래프 시스템이 없음(`trait Metric`/`depends_on`/`MetricId` grep 0건, 워크스페이스 전체). `crates/bitvue-engine/src/cache_validation.rs`/`cache_provenance.rs`(구 인용은 `bitvue-metrics` 소속으로 잘못 표기돼 있었음, 실제로는 `bitvue-engine`)는 캐시 무효화 추적이지 분석 패스 간 암묵적 데이터 의존성 문제와는 다른 관심사.

---

### PLUGIN-014: feature flag 조합이 폭발
**분류**: 빌드 구성 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
# Cargo.toml
[features]
default = ["avc", "hevc"]
avc = []
hevc = []
vp9 = []
av1 = []
vvc = []
av3 = []
mpeg2 = []
gpu-accel = []
simd = []
```
```rust
// bitvue-core/src/pipeline.rs — 코덱별 cfg가 core 로직 곳곳에 스며듦
#[cfg(all(feature = "hevc", feature = "gpu-accel"))]
fn build_hevc_pipeline() -> Pipeline { /* GPU 경로 */ }
#[cfg(all(feature = "hevc", not(feature = "gpu-accel")))]
fn build_hevc_pipeline() -> Pipeline { /* CPU 경로 */ }
#[cfg(all(feature = "vp9", feature = "simd"))]
fn vp9_idct() -> ... { /* ... */ }
// 코덱 7개 x 옵션 기능 2~3개 = 수십~수백 가지 조합, CI는 이 중 극히 일부만 검증
```

**문제**:
- feature flag의 조합 수는 flag 개수에 지수적으로 증가한다(7개 코덱 x gpu-accel x simd만 해도 128가지 조합). CI가 현실적으로 검증할 수 있는 조합은 이 중 극소수(`--all-features`, `--no-default-features` 정도)뿐이라, 대부분의 조합은 실제로 컴파일되는지조차 확인되지 않는다.
- 코덱별 `#[cfg(feature = "...")]`가 core 로직 안에 흩어져 있으면(코덱 크레이트 내부가 아니라), core 코드 자체가 "어떤 feature 조합으로 빌드되었는가"에 따라 동작이 달라지는 갈래를 만들어내 테스트 매트릭스가 core까지 오염된다.
- "고객 A는 avc+hevc만, 고객 B는 전체" 같은 커스텀 빌드가 늘어날수록, 어떤 조합이 실제로 프로덕션에서 쓰이고 어떤 조합이 이론상으로만 존재하는지 아무도 추적하지 못하게 된다.

**발생 조건**:
- 바이너리 크기를 줄이려고(PLUGIN-015) 코덱별 opt-in feature를 도입했는데, core 로직까지 feature-gate를 걸기 시작했을 때.
- GPU 가속처럼 코덱과 직교하는(orthogonal) 기능 축이 추가되어 feature 조합이 2차원, 3차원으로 늘어날 때.

**권장**:
```rust
// core는 feature-gate 없이 항상 컴파일되고, 코덱별 분기는 registry를 통한
// 런타임 dispatch로 흡수한다 (PLUGIN-001 패턴 재사용)
fn build_pipeline(codec: CodecId) -> Pipeline {
    registry::lookup(codec).build_pipeline() // core에 cfg(feature) 없음
}

// feature flag는 "이 코덱 크레이트를 워크스페이스에 포함할지"에만 쓰이고,
// 그 크레이트 내부에서만 자신의 GPU/SIMD 옵션을 다룬다
// bitvue-hevc/Cargo.toml
[features]
gpu-accel = ["dep:wgpu"]
```
- core 크레이트/모듈에는 코덱별 `#[cfg(feature = ...)]`를 두지 않는다 — 코덱을 포함할지 말지는 "이 크레이트를 워크스페이스 의존성에 넣을지"로 결정하고, core는 런타임 registry로 다룬다.
- CI 빌드 매트릭스를 "실제로 배포되는 조합"(예: `default`, `all-features`, `no-default-features`, 그리고 고객사별 실제 구성 1~2개)으로 좁혀 명시적으로 관리하고, 그 목록 자체를 문서화한다.
- feature 축이 2개 이상 직교하기 시작하면(코덱 x 가속 방식) 그 시점에 feature flag 대신 별도 빌드 프로파일이나 플러그인 시스템으로 전환을 검토한다.

**탐지 방법**:
- Static: `#[cfg(feature = ...)]`가 코덱 크레이트 바깥(core, UI 레이어)에 등장하는지 grep.
- Structural: `cargo tree --features` 혹은 `cargo hack check --feature-powerset`(일부만이라도)으로 실제 컴파일되는 조합 수를 측정하고 CI 커버리지와 비교.

**예외**:
- feature flag가 순수하게 "이 코덱 크레이트를 링크할지 말지"만 결정하고 core 코드에는 전혀 `cfg`가 없다면, 조합 폭발이 있어도 각 조합이 "포함된 코덱 집합"이라는 단일 축으로만 달라지므로 실질적 위험이 크지 않다. 문제는 여러 독립적인 축(코덱 x 가속 방식 x 플랫폼)이 core 코드의 `cfg` 안에서 서로 교차할 때다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) 루트 `Cargo.toml`의 workspace members는 코덱 크레이트 전부(현재 `bitvue-avc/hevc/vp9/vvc/av1-codec/av3-codec/mpeg2-codec/avs3/jpegxs/vc3` 10개)를 무조건 컴파일에 포함(코덱별 feature flag 자체가 없음). `bitvue-decode/Cargo.toml`의 `ffmpeg`/`vvdec` 2개만 실제 feature이고, `bitvue-engine`/`bitvue-sidecar`/`bitvue-cli` 어디에도 `#[cfg(feature=...)]` 코덱 분기가 없어(grep 0건) 조합 폭발이 일어날 축이 없음.

---

### PLUGIN-015: optional dependency가 binary size를 크게 증가
**분류**: 빌드 구성 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
# bitvue-vvc/Cargo.toml
[dependencies]
# VVC 레퍼런스 디코더를 검증용으로 링크 — 수 MB짜리 C 라이브러리
vvc-reference-sys = "0.3"

# bitvue-decode/Cargo.toml — default feature에 VVC가 기본 포함되어 있어서
# HEVC/AVC만 필요한 사용자도 이 무거운 의존성을 강제로 받는다
[dependencies]
bitvue-vvc = { path = "../bitvue-vvc" } # optional이 아님, 항상 링크됨
```

**문제**:
- 소수의 사용자만 필요로 하는 코덱(예: 아직 실무에서 드문 VVC/AV3)을 위한 무거운 의존성(레퍼런스 디코더, 대형 룩업 테이블, GPU 런타임)이 기본 빌드에 항상 포함되면, AVC/HEVC만 보는 압도적 다수 사용자의 다운로드 크기와 시작 시간이 불필요하게 늘어난다.
- 바이너리 크기 증가는 눈에 잘 안 띄는 형태로 누적된다 — 코덱 하나 추가할 때마다 "몇 MB 정도야 괜찮겠지"라는 판단이 반복되면 어느 순간 전체 바이너리가 처음의 몇 배가 되어 있다.
- Tauri 앱처럼 설치 파일 크기가 사용자 경험(다운로드 시간, 앱스토어 심사 기준)에 직결되는 배포 형태에서는 이 문제가 특히 치명적이다.

**발생 조건**:
- 신규 코덱을 추가하면서 "정확성 검증을 위해" 무거운 레퍼런스 구현이나 대형 테이블을 기본 의존성으로 끌어올 때.
- feature flag(PLUGIN-014)는 존재하지만 기본값(`default = [...]`)에 무심코 새 코덱을 추가해버려, 사실상 opt-out이 아니라 opt-in이어야 할 것이 opt-in처럼 보이지만 실제로는 항상 켜져 있을 때.

**권장**:
```toml
# bitvue-decode/Cargo.toml
[dependencies]
bitvue-vvc = { path = "../bitvue-vvc", optional = true }

[features]
default = ["avc", "hevc", "vp9", "av1"] # 흔한 코덱만 기본 포함
vvc = ["dep:bitvue-vvc"]                # 무거운 코덱은 opt-in
```
- 바이너리 크기에 큰 영향을 주는 의존성은 반드시 `optional = true` + 별도 feature로 opt-in화하고, `default` feature 목록에 무심코 추가되지 않도록 PR 리뷰 체크리스트에 명시한다.
- CI에 바이너리 크기 회귀 테스트(예: 이전 릴리스 대비 N% 이상 증가 시 실패)를 추가해 "조금씩 커지는" 것을 조기에 알아챈다.
- 정말 드물게 쓰이는 무거운 코덱은 별도의 선택적 다운로드 플러그인(런처가 필요 시 내려받는 부가 모듈)으로 분리하는 것도 고려한다.

**탐지 방법**:
- Structural: `cargo bloat` 또는 `cargo tree`로 크레이트별 기여 크기를 측정하고, `default` feature에 포함된 optional dependency 목록을 정기적으로 감사.
- Static: `Cargo.toml`의 `default = [...]`에 새 코덱이 추가되는 PR을 자동으로 플래그.

**예외**:
- 타깃 사용자 전원이 항상 모든 코덱을 필요로 하는 배포 형태(예: 사내 전용 풀-피처 빌드 하나만 배포)라면 바이너리 크기 최적화의 우선순위가 낮을 수 있다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) `bitvue-vvc`/`bitvue-av3-codec`/`bitvue-mpeg2-codec`/`bitvue-avs3`/`bitvue-jpegxs`/`bitvue-vc3`의 `Cargo.toml`은 전부 `bitvue-engine`(구 `bitvue-core`)/`abseil`/`thiserror`/`tracing`/`serde`만 의존(무거운 C 레퍼런스 디코더 없음). 유일하게 무거운 `ffmpeg-next`/`vvdec`(`bitvue-decode/Cargo.toml`)는 여전히 `optional = true` + non-default feature(`ffmpeg`/`vvdec`)로 opt-in 처리되어 있어 권장 사항을 이미 따르고 있음.

---

### PLUGIN-016: plugin별 설정 schema가 없음
**분류**: 구성 관리 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub trait CodecPlugin {
    // 설정을 문자열 맵으로 느슨하게 받음 — 어떤 키가 유효한지 타입이 말해주지 않는다
    fn configure(&mut self, options: &HashMap<String, String>);
}

impl CodecPlugin for Av1Plugin {
    fn configure(&mut self, options: &HashMap<String, String>) {
        if let Some(v) = options.get("max_tile_threads") {
            self.max_tile_threads = v.parse().unwrap_or(4); // 파싱 실패는 조용히 기본값
        }
        // "max_tiles_threads"처럼 오타난 키는 그냥 무시된다 — 경고도 없음
    }
}
```

**문제**:
- 설정 키/값이 문자열이면 오타(`max_tiles_threads` vs `max_tile_threads`)가 컴파일도, 런타임 에러도 없이 조용히 무시되어 "설정했는데 반영이 안 되는" 문제를 사용자가 스스로 진단할 방법이 없다.
- 각 플러그인이 어떤 설정 키를 받는지 문서 없이는 알 수 없고, 설정 UI(있다면)를 자동 생성할 근거도 없어 매번 손으로 폼을 만들어야 한다.
- 잘못된 타입/범위의 값(음수 스레드 수, 범위를 벗어난 QP 오프셋)이 `unwrap_or(기본값)`으로 조용히 대체되면 사용자가 의도한 설정과 실제 동작이 어긋난 채로 분석이 진행된다.

**발생 조건**:
- 코덱별 튜닝 옵션(스레드 수, 디버그 verbosity, 실험적 파싱 옵션)이 하나둘 늘어나면서 처음엔 간단해 보였던 `HashMap<String, String>`이 감당 못 할 정도로 커질 때.
- 설정을 세션 파일에 저장했다가 다음 실행에 불러오는 기능이 추가되어, 스키마 없는 설정의 하위 호환성 문제까지 겹칠 때.

**권장**:
```rust
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Av1PluginConfig {
    #[serde(default = "default_tile_threads")]
    pub max_tile_threads: u32,
    #[serde(default)]
    pub enable_cdef_overlay: bool,
}
fn default_tile_threads() -> u32 { 4 }

pub trait CodecPlugin {
    type Config: schemars::JsonSchema + serde::de::DeserializeOwned;
    fn configure(&mut self, config: Self::Config);
}

// host: 저장된 JSON을 역직렬화 단계에서 검증 — 알 수 없는 키는 에러(deny_unknown_fields)
#[serde(deny_unknown_fields)]
```
- 각 플러그인이 강타입 `Config` struct(`serde::Deserialize` + `schemars::JsonSchema` 등)를 노출하게 하고, host는 이를 역직렬화 시점에 검증한다. `#[serde(deny_unknown_fields)]`로 오타난 키를 명시적 에러로 잡는다.
- JSON Schema를 함께 노출하면 설정 UI를 스키마로부터 자동 생성하거나 최소한 유효성 검사를 공유할 수 있다.

**탐지 방법**:
- Static: `HashMap<String, String>` 형태의 설정 파라미터가 플러그인 trait에 남아있는지 grep.
- Structural: 각 코덱 플러그인의 설정 옵션이 문서(주석이 아니라 타입)로 표현되어 있는지 감사.

**예외**:
- 설정 항목이 1~2개뿐이고 절대 늘어날 계획이 없는 극히 단순한 플러그인이라면 강타입 스키마 인프라가 과할 수 있다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) `fn configure(`/`PluginConfig`/`CodecConfig` 류의 API가 코드베이스 어디에도 없음(grep 0건, `bitvue-codecs-parser`의 `ParserStrategy` trait에도 설정 메서드 없음) — 코덱별 튜닝 옵션 시스템 자체가(좋든 나쁘든) 아직 구축되지 않음.

---

### PLUGIN-017: 실패한 plugin을 매번 다시 초기화
**분류**: 성능/견고성 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
// 프레임마다 호출되는 파싱 루프
fn parse_frame(codec: CodecId, data: &[u8]) -> Result<ParsedUnit, ParseError> {
    // 매 프레임마다 플러그인을 새로 생성 — 이 profile은 이미 3천 프레임 전에
    // "unsupported profile"로 실패했다는 사실을 기억하지 못한다
    let plugin = Av1Plugin::new_with_tables(); // 내부적으로 큰 룩업 테이블을 매번 재구축
    plugin.parse(data) // 동일한 이유로 또 실패
}
```

**문제**:
- 동일한 원인(지원하지 않는 profile, 손상된 헤더 등 결정적으로 재현되는 실패)으로 이미 실패한 적이 있는 플러그인 초기화를, 매 프레임/유닛마다 처음부터 다시 시도하면 실패가 확실한데도 비용이 매번 지불된다.
- 초기화 비용이 큰 플러그인(대형 룩업 테이블 구축, GPU 컨텍스트 생성)일수록 이 낭비가 전체 파이프라인 처리량에 눈에 띄는 영향을 준다 — 특히 손상된 스트림을 프레임 단위로 계속 재시도하는 경우 사실상 매 프레임이 "느린 실패"가 된다.
- 사용자 입장에서는 "왜 실패한 파일을 여는데 이렇게 오래 걸리지"로 나타나는데, 원인이 "매 프레임 재시도"라는 걸 로그 없이는 알기 어렵다.

**발생 조건**:
- 손상되었거나 부분적으로만 지원되는 스트림(신규 profile, 실험적 확장)을 프레임 단위 루프로 반복 파싱할 때.
- 플러그인 인스턴스를 캐싱하지 않고 매 호출마다 새로 생성하는 stateless-style dispatch를 택했을 때.

**권장**:
```rust
pub struct PluginRegistry {
    instances: HashMap<CodecId, Result<Box<dyn CodecPlugin>, InitError>>,
}

impl PluginRegistry {
    pub fn get(&mut self, codec: CodecId) -> Result<&dyn CodecPlugin, &InitError> {
        self.instances
            .entry(codec)
            .or_insert_with(|| Av1Plugin::try_new().map(|p| Box::new(p) as Box<dyn CodecPlugin>))
            .as_deref()
            .map_err(|e| e) // 두 번째 호출부터는 캐시된 에러를 즉시 반환, 재초기화 없음
    }
}
```
- 플러그인 인스턴스(성공/실패 결과 포함)를 `OnceLock`/registry 캐시에 저장해, 초기화는 코덱당 한 번만 수행하고 이후로는 캐시된 결과(성공한 인스턴스 또는 실패 사유)를 재사용한다.
- 프레임 단위로 반복되는 파싱 실패(초기화가 아니라 개별 데이터 실패)는 별도 문제이므로 혼동하지 않는다 — 이 항목은 "초기화"처럼 결정적이고 비용이 큰 단계의 무의미한 반복을 다룬다.

**탐지 방법**:
- Runtime: 손상된 스트림 fixture로 처리 시간을 프로파일링해, 실패가 반복될수록 시간이 선형으로 늘어나는지(캐시 안 됨) 확인.
- Static: 코덱 dispatch 경로에서 플러그인 생성자가 프레임 루프 안에 있는지, 아니면 루프 바깥/캐시된 registry에 있는지 확인.

**예외**:
- 초기화 비용이 무시할 만큼 작은(단순 struct 생성 수준) 플러그인이라면 캐싱의 이득이 별로 없고, 오히려 상태 없는(stateless) 단순함을 유지하는 편이 나을 수 있다.

**Bitvue 판정**: Confirmed(범위 확장) — (2026-08-18 재검증, `src-tauri`는 Electron 전환으로 소멸해 폐기, 후속 IPC 커맨드 레이어인 `bitvue-sidecar`에서 동일 패턴 재확인) `crates/bitvue-sidecar/src/decode_bridge.rs:49`(`get_decoded_frame_yuv`)와 `:162`(`get_thumbnails`), `crates/bitvue-sidecar/src/debug_yuv.rs:604`까지 총 3개 호출부가 프레임/썸네일 요청마다 `Av1Decoder::new()`로 디코더를 처음부터 새로 생성함(`OnceLock`/`static`/`DecoderCache` 류가 `bitvue-sidecar` 어디에도 없음, grep 0건) — 문서가 말하는 "실패만 재시도"보다 넓게, 성공/실패 무관하게 매 요청마다 비용이 큰 초기화가 반복됨. sidecar가 이제 장시간 떠 있는 별도 프로세스가 된 만큼(PLUGIN-004 참고) 이 비용은 세션 내내 누적됨.

---

### PLUGIN-018: codec parser와 decoder registration 혼용
**분류**: 인터페이스 설계 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// 하나의 trait이 "비트스트림 구조 분석"과 "픽셀 복원"을 모두 강제한다
pub trait Codec {
    fn parse(&self, data: &[u8]) -> Result<ParsedUnit, ParseError>;
    fn decode_to_pixels(&self, unit: &ParsedUnit) -> Result<Frame, DecodeError>; // 필수 메서드
}

// bitvue-vvc는 아직 syntax 분석만 지원하고 실제 픽셀 디코딩은 구현이 없는데
// trait이 강제하니 어쩔 수 없이 이렇게 채워넣는다
impl Codec for VvcPlugin {
    fn parse(&self, data: &[u8]) -> Result<ParsedUnit, ParseError> { /* 실제 구현 */ }
    fn decode_to_pixels(&self, unit: &ParsedUnit) -> Result<Frame, DecodeError> {
        unimplemented!("VVC pixel decode not yet supported") // 런타임에만 발견됨
    }
}
```

**문제**:
- "비트스트림 구조를 구문 분석해 syntax element를 뽑아내는 것(parse)"과 "그 syntax로부터 실제 픽셀을 복원하는 것(decode)"은 전혀 다른 난이도와 목적을 가진 별개의 능력인데, 하나의 trait으로 묶으면 parse만 필요한 분석 전용 코덱(대부분의 Bitvue 코덱이 여기 해당— 비트스트림 분석기이지 인코더/디코더가 아님)도 decode 메서드를 억지로 채워야 한다.
- `unimplemented!()`나 `Err(DecodeError::NotSupported)`로 채워진 메서드는 호출 전까지는 겉보기에 "이 코덱은 decode를 지원한다"는 착각을 준다 — capability discovery(PLUGIN-005)가 있어도 trait 자체의 존재가 거짓 신호를 준다.
- 반대로 픽셀 디코딩까지 진짜로 지원하는 코덱이 생겼을 때, 그 기능이 "당연히 있어야 하는 필수 메서드"가 아니라 "선택적으로 있을 수도 있는 능력"이라는 것을 표현할 방법이 애초에 없다.

**발생 조건**:
- 프로젝트 초기에 "언젠가는 디코딩도 지원할 수도 있으니" 하는 예측성 설계로 trait에 decode 메서드를 미리 넣어둘 때.
- 분석 전용 코덱 크레이트와 (있다면) 실제 픽셀 디코딩까지 지원하는 코덱 크레이트가 섞여 있는데 인터페이스가 이를 구분하지 않을 때.

**권장**:
```rust
// 관심사를 별도 trait으로 분리하고, 각 코덱은 자신이 실제로 구현하는 것만 구현한다
pub trait CodecParser: Send + Sync {
    fn parse(&self, data: &[u8]) -> Result<ParsedUnit, ParseError>;
}

pub trait CodecDecoder: Send + Sync {
    fn decode_to_pixels(&self, unit: &ParsedUnit) -> Result<Frame, DecodeError>;
}

// VVC는 CodecParser만 구현 — decode 미지원이 타입 레벨에서 드러난다
impl CodecParser for VvcPlugin { /* ... */ }

// registry는 두 능력을 독립적으로 질의할 수 있다
fn can_decode(codec: CodecId) -> bool {
    registry::lookup_decoder(codec).is_some()
}
```
- "구조 분석(parse)"과 "픽셀 복원(decode)"을 별도 trait으로 분리하고, registry도 두 종류를 독립적으로 등록/조회한다.
- 코덱이 어떤 조합(parser만, decoder만, 둘 다)을 구현하는지가 타입 시스템과 registry 조회로 즉시 드러나게 해 capability discovery(PLUGIN-005)와 자연스럽게 통합한다.

**탐지 방법**:
- Static: trait 메서드 본문에 `unimplemented!()`, `todo!()`, `Err(NotSupported)`만 있는 구현체가 몇 개인지 grep — 많다면 trait이 너무 넓다는 신호.
- Structural: 코덱 크레이트별로 실제 구현되는 메서드 비율을 감사.

**예외**:
- 프로젝트의 모든 코덱이 실제로 parse와 decode를 항상 함께 지원하고 그럴 계획이 확고하다면(예: 순수 인코더/디코더 프로젝트로, 분석 전용 코덱이 존재하지 않는다면) 단일 trait이 오히려 단순하다. Bitvue처럼 "비트스트림 분석기"가 주 목적이고 픽셀 디코딩이 부가 기능인 프로젝트에서는 분리가 거의 항상 유리하다.

**Bitvue 판정**: N/A(오히려 권장 패턴을 따름) — (2026-08-18 재검증, 경로 불변) parse(구조 분석)와 decode(픽셀 복원)가 이미 별도 계층: 코덱 크레이트(bitvue-avc/hevc/vp9/vvc/av1-codec/av3-codec/mpeg2-codec/avs3/jpegxs/vc3)는 구문 분석만 담당하고, 픽셀 디코딩은 `bitvue-decode`의 별도 `Decoder` trait(`crates/bitvue-decode/src/traits.rs:127`, 여전히 같은 위치)이 전담(현재 실제로는 AV1만 `Av1Decoder`로 구현) — 대부분의 코덱 크레이트는 decode 능력 자체가 없고 이를 강제하는 통합 trait도 없음(`unimplemented!()` 스텁 grep 0건).

---

### PLUGIN-019: stable ID 없이 display name을 key로 사용
**분류**: 데이터 안정성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// 세션 저장 포맷 — 코덱을 사용자에게 보여주는 라벨 문자열로 식별한다
#[derive(Serialize, Deserialize)]
pub struct SavedSession {
    pub codec_label: String, // "H.264/AVC (Baseline Profile)" 같은 표시용 문자열
    pub overlay_prefs: HashMap<String, bool>, // 오버레이도 표시 라벨이 key
}

// UI 문구를 다듬는 나중의 리팩터링
- "H.264/AVC (Baseline Profile)"
+ "H.264 / AVC — Baseline"   // 단순 표기 개선인데
// 이 라벨을 key로 저장했던 기존 세션 파일들이 전부 "알 수 없는 코덱"으로 로드 실패
```

**문제**:
- 사람이 읽기 위한 display name은 UX 개선, 오타 수정, 국제화(i18n) 등의 이유로 자유롭게 바뀔 수 있어야 하는데, 이를 저장 포맷이나 내부 로직의 key로 쓰면 그 자유가 사라진다 — 라벨을 고치는 순간 하위 호환성이 깨진다.
- 국제화가 도입되면 문제가 더 심각해진다 — "H.264/AVC"라는 영어 라벨을 key로 저장했는데 UI 언어가 한국어로 바뀌면 표시 로직과 저장 로직이 서로 다른 문자열을 기대하게 된다.
- display name은 대소문자, 공백, 괄호, 유니코드 기호(예: "—" vs "-") 같은 사소한 변형에도 민감해서, 같은 코덱을 가리키는 라벨이 코드 안에서 미묘하게 다른 문자열로 여러 번 등장하면 동등성 비교가 깨지기 쉽다.

**발생 조건**:
- 프로토타입 단계에서 "일단 사람이 읽을 수 있는 문자열이니 그대로 key로 쓰자"는 편의적 선택을 했다가, 그 포맷이 세션 저장/설정 파일처럼 오래 남는 데이터로 굳어질 때.
- UI 카피라이팅 리뷰나 국제화 작업이 뒤늦게 들어와 라벨 문자열이 바뀌는 시점.

**권장**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecId { Avc, Hevc, Vp9, Av1, Vvc, Av3, Mpeg2 } // 저장/비교에는 항상 이 안정 ID

impl CodecId {
    pub fn display_name(&self, locale: Locale) -> &'static str {
        match (self, locale) {
            (CodecId::Avc, Locale::En) => "H.264 / AVC",
            (CodecId::Avc, Locale::Ko) => "H.264/AVC",
            // 라벨은 여기서만 관리되고 마음껏 바꿀 수 있다 — 저장 포맷과 무관
            _ => "Unknown",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SavedSession {
    pub codec: CodecId, // display name이 아니라 안정 ID를 직렬화
}
```
- 저장/비교/키(HashMap key, 직렬화 필드)로 쓰이는 식별자는 항상 안정적인 machine ID(enum variant, 짧은 코드, UUID)로 하고, display name은 그로부터 조회되는 순수 프레젠테이션 값으로만 둔다.
- `serde(rename_all)`이나 명시적 `#[serde(rename = "...")]`로 직렬화되는 문자열 표현 자체도 enum variant 이름 리팩터링에 영향받지 않게 고정한다.

**탐지 방법**:
- Static: `HashMap<String, _>`나 직렬화 struct의 필드가 UI 라벨/사용자 가시 문자열을 그대로 key/value로 쓰는지 grep(문자열 리터럴에 공백, 괄호, 대문자 혼용이 있으면 display name일 가능성이 높다).
- Structural: 저장 포맷의 필드 타입이 enum인지 String인지 감사 — 코덱/오버레이/메트릭 식별자가 String이면 후보로 표시.

**예외**:
- 순수히 휘발성인 런타임 전용 데이터(디스크에 저장되지 않고 프로세스 재시작 시 사라지는 캐시)라면 display name을 key로 써도 하위 호환성 문제가 생기지 않는다 — 다만 이 경우도 등가성 버그(대소문자 등) 위험은 남는다.

**Bitvue 판정**: Confirmed — (2026-08-18 재검증, `bitvue-core`→`bitvue-engine` 개명 후 라인번호만 소폭 이동) `crates/bitvue-engine/src/qp_heatmap.rs:385` `QPHeatmapCacheKey.codec: String`가 캐시 키에 쓰이고, `stream_state.rs:146`/`types.rs:88`/`event_observer.rs:262`/`index_extractor.rs:700`/`diagnostics.rs:184`(`with_codec(codec: String)`) 등에도 `codec: String` 필드가 여전히 산재 — 코덱 식별에 안정적 enum(CodecId)이 단 하나도 통일되어 있지 않아, PLUGIN-006에서 확인된 대소문자/별칭 불일치가 캐시 키/조회 키 레벨까지 그대로 전파됨.

---

### PLUGIN-020: third-party plugin 권한 경계 없음
**분류**: 보안/격리 · **심각도**: High · **탐지**: Manual

**나쁜 예**:
```rust
pub trait CodecPlugin: Send + Sync {
    fn parse(&self, data: &[u8]) -> Result<ParsedUnit, ParseError>;
}

// 어떤 코덱 크레이트든 워크스페이스에 링크되는 순간 프로세스 전체와
// 동일한 권한을 갖는다 — parse() 안에서 파일 시스템, 네트워크, 환경변수,
// 임의 syscall까지 아무 제약 없이 사용할 수 있다
impl CodecPlugin for SomeThirdPartyVendorCodec {
    fn parse(&self, data: &[u8]) -> Result<ParsedUnit, ParseError> {
        // 원격 라이선스 서버에 "전화홈"하거나, 사용자 홈 디렉터리를 읽거나,
        // 텔레메트리를 무단으로 전송해도 host는 이를 막을 방법이 없다
        std::fs::read_to_string(std::env::var("HOME").unwrap() + "/.ssh/config").ok();
        internal_parse(data)
    }
}
```

**문제**:
- Rust의 크레이트/trait 경계는 컴파일 타임 타입 안전성은 제공하지만 런타임 권한 격리는 전혀 제공하지 않는다 — 링크된 크레이트는 host 프로세스와 정확히 동일한 권한(파일, 네트워크, 프로세스 실행)을 갖는다.
- "코덱 파서"라는 좁은 역할을 부여받은 컴포넌트가 실제로는 임의 파일 읽기, 임의 네트워크 접근을 할 수 있다는 것은, 그 코덱 구현이 사내에서 완전히 신뢰할 수 있는 코드가 아니라면(서드파티 벤더 제공, 오픈소스 fork, 커뮤니티 기여) 공급망 공격(supply-chain attack)의 표면이 된다.
- 사용자가 열어보는 비트스트림 파일이 공격자가 조작한 것이고, 코덱 파서 자체에 버그가 있어 임의 코드 실행으로 이어진다면(메모리 안전성 버그, unsafe 블록의 실수), 그 권한이 프로세스 전체와 동일하므로 파일 하나 잘못 열었다가 시스템 전체가 노출될 수 있다.

**발생 조건**:
- 사내에서 전부 직접 작성/리뷰하는 코덱 크레이트만 있는 동안은 이 위험이 상대적으로 낮다(신뢰 경계가 곧 조직 경계와 같으므로) — 문제는 서드파티가 제공하는 코덱 플러그인, 오픈소스 커뮤니티 기여, 혹은 사용자가 직접 설치하는 플러그인이 생기는 순간부터 현실화된다.
- 퍼징되지 않은 서드파티 파서로 신뢰할 수 없는 입력(사용자가 다운로드한 임의의 비트스트림 파일)을 처리할 때 — 코덱 파서는 태생적으로 "신뢰할 수 없는 바이트를 파싱하는" 코드이므로 공격 표면이 이미 넓다.

**권장**:
```rust
// 최소한의 방어: trait 경계 자체를 좁혀 I/O 능력을 아예 노출하지 않는다
pub trait CodecPlugin: Send + Sync {
    // &[u8] 입력, ParsedUnit 출력 외에 어떤 시스템 자원에도 접근할 방법이
    // 시그니처 상에 없다 — 파일/네트워크가 필요하면 host가 명시적으로 주입한다
    fn parse(&self, data: &[u8]) -> Result<ParsedUnit, ParseError>;
}
// 강한 격리가 필요한 진짜 서드파티/미신뢰 플러그인이라면 WASM 컴포넌트로 실행해
// 능력 기반(capability-based) 샌드박스를 강제한다
fn run_untrusted_plugin(wasm_bytes: &[u8], data: &[u8]) -> Result<ParsedUnit, ParseError> {
    let engine = wasmtime::Engine::default();
    // WASI 능력을 부여하지 않으면 이 컴포넌트는 파일/네트워크에 원천적으로 접근 불가
    ...
}
```
- 최소한의 방어선으로, plugin trait의 시그니처 자체를 순수 함수(`&[u8] -> Result<T, E>`)로 좁혀 I/O 능력이 애초에 시그니처에 존재하지 않게 한다 — 필요한 I/O(로그, 캐시 경로 등)는 host가 명시적으로 주입하는 형태로만 허용한다.
- 진짜로 신뢰할 수 없는 서드파티 코드를 실행해야 한다면 WASM(wasmtime/wasmer) 같은 능력 기반 샌드박스로 프로세스 권한과 완전히 분리하거나, 최소한 별도 프로세스 + OS 레벨 샌드박싱(seccomp, App Sandbox)으로 격리한다.
- 사내에서 작성하는 코덱 크레이트라도 `cargo-geiger` 등으로 `unsafe` 사용을 감사하고, 의존성 공급망(각 코덱 크레이트가 끌어오는 하위 의존성)을 정기적으로 점검한다.

**탐지 방법**:
- Manual: 코덱 플러그인 trait의 메서드 시그니처에 파일/네트워크 접근이 필요할 이유가 없는데도 그런 접근이 가능한 컨텍스트(예: 임의 side-effect를 가진 클로저, 전역 상태 접근)가 열려 있는지 코드 리뷰로 확인.
- Structural: 각 코덱 크레이트의 의존성 그래프에서 `std::fs`, `std::net`, `std::process` 직접 사용 여부를 감사 — 파서 크레이트에는 원칙적으로 나타날 이유가 없다.

**예외**:
- 모든 코덱 크레이트가 100% 사내에서 작성·리뷰되고 서드파티/커뮤니티 기여를 받지 않는 동안은, 신뢰 경계가 조직 경계와 일치하므로 샌드박싱 인프라를 미리 구축하는 것이 우선순위가 낮을 수 있다. 다만 이 경우도 "실수로 인한 버그"(공급망 공격이 아니라 단순 실수)에 대한 최소 방어(trait 시그니처를 좁게 유지)는 비용이 낮으므로 여전히 권장된다.

**Bitvue 판정**: N/A — (2026-08-18 재검증) PLUGIN-003/011과 동일 근거(서드파티/동적 로딩 플러그인 시스템 자체가 없음, 모든 코덱 크레이트가 사내 작성·정적 링크) — 문서가 스스로 명시한 예외("사내에서 전부 직접 작성/리뷰하는 코덱 크레이트만 있는 동안은…")에 정확히 해당. Electron sidecar 분리(`bitvue-sidecar`가 별도 OS 프로세스로 stdio IPC)도 신뢰 경계 완화가 아니라 프로세스 통신 방식 변경일 뿐이라 결론에 영향 없음.
