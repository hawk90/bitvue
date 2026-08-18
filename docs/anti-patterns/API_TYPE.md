# Anti-Pattern Catalog — API_TYPE: API·타입 설계

이 문서는 Bitvue 안티패턴 카탈로그의 일부입니다 (전체 색인은 별도로 작성 중인 `docs/anti-patterns/INDEX.md` 참고). 이 카테고리는 원 설계 논의 문서의 요약 표에 "API·타입 설계 — 25 items"로 항목 수만 언급되고 개별 항목이 나열되지 않았던 것을, 카탈로그 작성 단계에서 같은 논의의 다른 발견 사항(파사드 무차별 재수출, 크레이트 과분할, offset/index newtype 부재, String 기반 dispatch, hot path의 dyn trait, trait 경계 배치)을 근거로 새로 전개하여 채운 것입니다.

---

### API-001: usize의 offset/index 혼용 (newtype 부재)
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
// bitvue-formats: MP4 박스 파서
pub struct BoxHeader {
    pub offset: usize,      // 파일 offset인가? 박스 내부 offset인가?
    pub size: usize,
}

// bitvue-codecs-parser: NAL 유닛 파서
pub fn find_nal_start(data: &[u8], from: usize) -> Option<usize> {
    // from이 byte offset인지 bit offset인지 시그니처만 봐선 알 수 없음
    ...
}

// bitvue-core: 프레임 인덱싱
pub fn seek_frame(frame: usize) -> Result<(), SeekError> { ... }

// 호출부에서 실수로 bit offset을 byte offset 함수에 넘겨도 컴파일러가 못 잡음
let bit_pos: usize = read_ue_bit_position(&reader);
find_nal_start(&data, bit_pos); // 버그: byte offset 함수에 bit offset을 전달
```

**문제**:
- `usize` 하나가 "파일 byte offset", "비트스트림 bit offset", "프레임 인덱스", "트랙 ID"를 전부 대신하면서 타입 시스템이 이들을 구분하지 못함
- 함수 시그니처만 보고는 단위(byte vs bit)와 도메인(offset vs index vs id)을 알 수 없어 코드 리뷰와 온보딩 비용이 커짐
- 잘못된 종류의 값을 잘못된 함수에 넘겨도 컴파일 타임에 잡히지 않고, 파싱 오류나 크래시로 런타임에만 드러남
- 여러 크레이트(bitvue-formats, bitvue-codecs-parser, bitvue-core) 경계를 넘나들며 같은 실수가 반복 재생산됨

**발생 조건**:
- 파일 offset, 비트 offset, 프레임/샘플/트랙 인덱스가 모두 같은 함수 근처에서 다뤄지는 파서·디먹서 코드
- 프로토타이핑 단계에서 "일단 usize로" 시작한 뒤 타입을 굳히지 않고 그대로 공개 API가 된 경우

**권장**:
```rust
// bitvue-core::types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileOffset(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BitOffset(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameIndex(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u32);

pub struct BoxHeader {
    pub offset: FileOffset,
    pub size: u64,
}

pub fn find_nal_start(data: &[u8], from: FileOffset) -> Option<FileOffset> { ... }
pub fn seek_frame(frame: FrameIndex) -> Result<(), SeekError> { ... }

// 컴파일 타임에 실수 차단
let bit_pos: BitOffset = read_ue_bit_position(&reader);
find_nal_start(&data, bit_pos); // 컴파일 에러: BitOffset은 FileOffset이 아님
```
- 각 newtype에 필요한 산술(`Add<u64>`, `Sub` 등)만 최소한으로 구현해 표현력과 안전성의 균형을 맞춘다
- `#[repr(transparent)]`를 붙이면 FFI/직렬화 시 성능 손실 없이 안전성만 얻는다

**탐지 방법**:
- Static: clippy `disallowed_types`로 공개 API 시그니처에서 bare `usize`/`u64` offset 파라미터 금지 목록화
- Structural: 크레이트 경계를 넘는 함수 시그니처 중 offset/index/id 의미를 가진 이름(`offset`, `pos`, `idx`, `frame`, `track`)의 파라미터 타입이 primitive인 것을 grep으로 스캔

**예외**:
- 크레이트 내부(비공개) 핫 루프에서 newtype 래핑/언래핑 오버헤드가 프로파일링으로 실측 확인된 경우, 내부 전용으로만 primitive 사용 가능 (경계에서는 반드시 변환)
- 단발성 CLI 유틸리티 등 API 안정성이 중요하지 않은 코드

**Bitvue 판정**: Confirmed — 파일 전역에서 offset/index/id가 전부 bare `usize`/`u64`, byte-offset과 bit-offset 구분 타입 없음 (crates/bitvue-formats/src/mp4.rs:69-75 `BoxHeader.data_offset: u64`; crates/bitvue-avc/src/nal.rs:166 `NalUnit.offset: usize`; crates/bitvue-core/src/frame_identity.rs:23-34 `FrameIndexMap`가 display/decode index를 전부 `Vec<usize>`로 보관)

---

### API-002: String 기반 codec/type dispatch
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// bitvue-codecs-parser
pub fn create_parser(codec_name: &str) -> Box<dyn CodecParser> {
    match codec_name {
        "h264" | "avc" => Box::new(bitvue_avc::AvcParser::new()),
        "hevc" | "h265" => Box::new(bitvue_hevc::HevcParser::new()),
        "vp9" => Box::new(bitvue_vp9::Vp9Parser::new()),
        "av1" => Box::new(bitvue_av1::Av1Parser::new()),
        _ => panic!("unknown codec: {codec_name}"),
    }
}

// 여기저기 흩어진 레지스트리
static PARSERS: Lazy<HashMap<String, Box<dyn Fn() -> Box<dyn CodecParser>>>> = ...;
```

**문제**:
- 오타("hvec", "Av1")가 컴파일 타임이 아닌 실행 중 `panic!`이나 `None`으로만 드러남
- 지원 코덱 목록이 `match` 문자열 리터럴 여러 곳에 중복 산재해 새 코덱 추가 시 빠뜨리기 쉬움
- `HashMap<String, Box<dyn Parser>>` 레지스트리는 순회할 때 정렬 순서가 불명확하고, exhaustiveness 검사(컴파일러가 새 코덱 케이스 누락을 알려주는 것)를 포기하게 됨
- IDE 자동완성/rename-refactor 지원을 받지 못함

**발생 조건**:
- 파일 확장자, MIME 타입, CLI `--codec` 플래그, 설정 파일 등 "신뢰 경계(trust boundary)"에서 외부 문자열을 처음 받는 지점
- 이 경계에서 받은 문자열을 내부 로직 전체에 그대로 문자열째로 전파시키는 경우

**권장**:
```rust
// bitvue-core::types — 신뢰 경계를 통과하면 즉시 enum으로 변환
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecKind {
    Avc,
    Hevc,
    Vp9,
    Av1,
    Vvc,
}

impl FromStr for CodecKind {
    type Err = UnknownCodecError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "h264" | "avc" => Ok(Self::Avc),
            "hevc" | "h265" => Ok(Self::Hevc),
            "vp9" => Ok(Self::Vp9),
            "av1" => Ok(Self::Av1),
            "vvc" | "h266" => Ok(Self::Vvc),
            other => Err(UnknownCodecError(other.to_string())),
        }
    }
}

// 이후 내부 로직은 전부 CodecKind로 dispatch — 컴파일러가 exhaustiveness 강제
pub fn create_parser(kind: CodecKind) -> Box<dyn CodecParser> {
    match kind {
        CodecKind::Avc => Box::new(bitvue_avc::AvcParser::new()),
        CodecKind::Hevc => Box::new(bitvue_hevc::HevcParser::new()),
        CodecKind::Vp9 => Box::new(bitvue_vp9::Vp9Parser::new()),
        CodecKind::Av1 => Box::new(bitvue_av1::Av1Parser::new()),
        CodecKind::Vvc => Box::new(bitvue_vvc::VvcParser::new()),
    }
}
```
- 문자열 파싱은 `FromStr`/`TryFrom<&str>` 한 곳으로 모으고, 그 뒤로는 절대 문자열로 되돌아가지 않는다
- Tauri command 경계, CLI 인자 경계에서만 문자열을 받고 즉시 enum으로 변환

**탐지 방법**:
- Static: `match .*\.as_str\(\)` 및 `HashMap<String,` 패턴을 codec/type 관련 식별자 근처에서 grep
- Runtime: 코덱 이름 오타 fuzz 테스트로 panic 대신 `Result` 에러가 나오는지 확인

**예외**:
- 진짜 신뢰 경계(CLI 파싱, 설정 파일 역직렬화, HTTP 헤더) 그 자체에서는 문자열을 받는 것이 당연하며 안티패턴이 아님 — 문제는 그 문자열을 enum으로 즉시 변환하지 않고 내부까지 전파시키는 것
- 플러그인처럼 컴파일 타임에 전체 집합을 알 수 없는 동적 확장 지점(외부 코덱 플러그인 로딩 등)은 문자열/레지스트리 방식이 불가피

**Bitvue 판정**: N/A (문서 근거 stale) — 인용된 `src-tauri`는 2026-08-08 Electron 이관으로 저장소에서 완전히 삭제됨(현재 `find`로 존재 확인 불가). 후속 아키텍처(`crates/bitvue-sidecar`, `crates/bitvue-codecs-parser`)에서 codec dispatch를 재확인한 결과 전부 typed enum 사용 중 — `CodecType`(crates/bitvue-codecs-parser/src/parser_strategy.rs:19), `ForceCodec`(crates/bitvue-cli/src/commands/decode.rs:254 `match codec {`) — String 기반 codec match/HashMap 레지스트리는 grep으로 못 찾음

---

### API-003: 비트 레벨 hot path까지 침투한 trait 추상화
**분류**: API·타입 설계 · **심각도**: Critical · **탐지**: Runtime/Structural

**나쁜 예**:
```rust
// bitvue-codecs-parser: 모든 비트 리더를 trait object로
pub trait BitReader {
    fn read_bit(&mut self) -> bool;
    fn read_bits(&mut self, n: u32) -> u64;
    fn read_ue(&mut self) -> u32; // Exp-Golomb
}

pub struct SliceHeaderParser<'a> {
    reader: &'a mut dyn BitReader, // 비트 단위 호출마다 동적 디스패치
}

impl<'a> SliceHeaderParser<'a> {
    pub fn parse(&mut self) -> SliceHeader {
        let first_mb = self.reader.read_ue(); // vtable 호출, 인라인 불가
        let slice_type = self.reader.read_ue();
        // 프레임당 수만~수십만 번 호출되는 루프에서 매번 vtable 경유
        ...
    }
}
```

**문제**:
- `read_bit`/`read_bits`처럼 프레임당 수십만~수백만 번 호출되는 최하위 연산에 `dyn Trait`을 쓰면 매 호출이 vtable 인다이렉션을 거쳐 인라이닝이 완전히 막힘
- 컴파일러가 비트 시프트/마스킹 연산을 레지스터에 유지한 채 최적화할 기회를 잃어 실측 성능이 정적 디스패치 대비 수 배 느려질 수 있음
- 분기 예측 실패, 캐시 미스가 늘어나는데 프로파일러에는 "그냥 느린 파서"로만 보여 원인 추적이 어려움

**발생 조건**:
- "테스트에서 mock BitReader를 주입하고 싶다"는 이유만으로 최하위 비트 연산까지 trait화한 경우
- 코드베이스 전체에 "trait으로 추상화하면 좋다"는 원칙을 기계적으로 적용해 추상화 단위 크기를 고려하지 않은 경우

**권장**:
```rust
// 비트 레벨은 concrete 타입 + 제네릭으로, 인라이닝을 보장
pub struct BitReader<R: AsRef<[u8]>> {
    data: R,
    byte_pos: usize,
    bit_pos: u8,
}

impl<R: AsRef<[u8]>> BitReader<R> {
    #[inline(always)]
    pub fn read_bit(&mut self) -> bool { ... }
    #[inline(always)]
    pub fn read_ue(&mut self) -> u32 { ... }
}

// trait object는 "프레임 파서" 같은 더 큰 단위 경계에서만 사용
pub trait CodecParser {
    fn parse_frame(&mut self, data: &[u8]) -> Result<Frame, ParseError>;
}
// 프레임당 1회 호출되는 이 레벨에서는 vtable 비용이 무시할 수준
```
- 추상화가 필요한 지점을 "얼마나 자주 호출되는가"로 판단: 프레임/유닛/패킷 단위(수백~수천 회/초)는 trait object 허용, 비트/바이트 단위(수백만 회/초)는 concrete 타입+제네릭 또는 매크로
- 테스트용 mock이 필요하면 제네릭 파라미터(`BitReader<Cursor<Vec<u8>>>` vs `BitReader<&[u8]>`)로 해결하고 trait object는 피한다

**탐지 방법**:
- Runtime: `cargo flamegraph`/`perf`로 파싱 hot path에서 간접 호출(`call *%rax` 계열) 비중 확인, `#[inline(never)]` 강제 후 벤치마크 대비 실측
- Structural: `dyn BitReader`, `dyn Fn` 등이 루프 본문(특히 `for`/`while` 내부) 안에서 호출되는 패턴을 정적 스캔

**예외**:
- 파서가 실제로 다양한 입력 소스(파일, 메모리, 네트워크 스트림)를 런타임에 스위칭해야 하고, 비트 파싱 자체가 성능 크리티컬하지 않은 경로(예: 헤더 몇 바이트만 읽는 컨테이너 파서)라면 trait object도 허용 가능
- 초기 프로토타입 단계에서 정확성 검증이 우선이고 성능은 나중에 최적화할 계획이 명시된 경우 (단, 반드시 후속 작업으로 추적)

**Bitvue 판정**: N/A — 각 코덱 크레이트의 `BitReader`는 concrete struct(제네릭 라이프타임만 사용)이며 `dyn BitReader`는 어디에도 없음 (crates/bitvue-avc/src/bitreader.rs:16, crates/bitvue-hevc/src/bitreader.rs 등); `dyn ReadSeek`은 프레임당 1회 수준인 컨테이너 인덱스 추출부(crates/bitvue-core/src/index_extractor.rs)에서만 쓰여 예외 조건에 해당

---

### API-004: 파사드 크레이트의 무차별 재수출
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
// bitvue (root facade crate) lib.rs
pub use bitvue_core::*;
pub use bitvue_formats::*;
pub use bitvue_codecs::*;
pub use bitvue_codecs_parser::*;
pub use bitvue_decode::*;
pub use bitvue_metrics::*;
// ... 20개 크레이트 전부
```

**문제**:
- 여러 크레이트가 우연히 같은 이름(`Error`, `Config`, `Frame`, `Header`)을 export하면 glob 재수출 순서에 따라 이름 충돌이 나거나, 나중에 추가된 크레이트가 조용히 앞의 타입을 가려버림(shadowing)
- 각 하위 크레이트의 "구현 세부사항으로 남기려던" 타입까지 전부 루트 파사드의 공개 API가 되어버려, semver상 하위 크레이트 내부를 건드릴 때마다 파사드의 breaking change 여부를 매번 전수 검토해야 함
- rustdoc에서 수백 개 심볼이 한 페이지에 평면적으로 나열되어 사용자가 "이 기능은 어느 크레이트 소관인가"를 알 수 없어짐
- 파사드를 통해서만 依存하는 다운스트림 코드가 실제로는 어떤 하위 크레이트에 의존하는지 `Cargo.toml`만 봐서는 알 수 없어 의존성 그래프가 사실상 불투명해짐

**발생 조건**:
- 워크스페이스를 여러 크레이트로 쪼갠 뒤 "사용 편의성"을 이유로 파사드에서 전부 `pub use *`로 재수출한 경우
- 하위 크레이트 API가 아직 안정화되지 않은 초기 개발 단계에서 편의를 위해 임시로 넣었다가 방치된 경우

**권장**:
```rust
// bitvue (root facade) lib.rs — 선택적, 이름 지정 재수출만 허용
pub use bitvue_core::{FrameIndex, TrackId, FileOffset};
pub use bitvue_codecs_parser::{CodecKind, ParsedFrame};
pub use bitvue_decode::DecodeSession;

// 나머지는 모듈 경로로 명시적 노출
pub mod formats {
    pub use bitvue_formats::{Mp4Demuxer, MkvDemuxer, IvfDemuxer};
}
pub mod metrics {
    pub use bitvue_metrics::{PsnrCalculator, SsimCalculator};
}
// 크레이트 내부 헬퍼 타입은 애초에 재수출하지 않음 — 필요하면 사용자가
// bitvue-metrics를 직접 의존성에 추가하게 한다
```
- 파사드가 "무엇을 재수출할지"를 매 릴리스마다 의도적으로 결정하는 큐레이션 지점으로 삼는다
- `#[doc(inline)]`와 모듈 네임스페이스를 활용해 rustdoc 탐색성을 유지한다
- CI에 "새 `pub use` 추가 시 CHANGELOG 갱신 필요" 체크를 넣어 공개 API 확장을 리뷰 프로세스에 태운다

**탐지 방법**:
- Static: `pub use .*::\*` 패턴을 grep, 특히 파사드 크레이트의 `lib.rs`에서 발견 시 flag
- Structural: `cargo public-api`로 파사드의 공개 API 목록을 스냅샷하고 하위 크레이트 수와 비교해 재수출 비율 측정

**예외**:
- 워크스페이스가 2~3개 크레이트로 작고 API 표면이 안정적이며, 파사드가 명시적으로 "convenience re-export only, no independent API design" 문서화된 경우는 허용 가능
- 내부(비공개) 전용 파사드로 workspace 안에서만 쓰이고 외부에 배포되지 않는 크레이트

**Bitvue 판정**: Confirmed — 루트 파사드(crates/bitvue/src/lib.rs)와 crates/bitvue-codecs/src/lib.rs는 이름 지정 재수출(`pub use X as name`)로 안전하지만, crates/bitvue-codecs-parser/src/lib.rs:12에서 `pub use bitvue_av1_codec::*;` 글롭 재수출을 사용 중

---

### API-005: 크레이트 과분할과 의존성 그래프 역전
**분류**: API·타입 설계 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-codecs/src/lib.rs — 순수 재수출 shim, 존재 이유가 불명확
pub use bitvue_avc::*;
pub use bitvue_hevc::*;
pub use bitvue_vp9::*;
pub use bitvue_av1::*;

// bitvue-codecs-parser/src/lib.rs — 사실상 switch/adapter 한 함수뿐
pub fn parse(kind: CodecKind, data: &[u8]) -> ParsedFrame {
    match kind {
        CodecKind::Avc => bitvue_avc::parse(data).into(),
        CodecKind::Hevc => bitvue_hevc::parse(data).into(),
        // ...
    }
}

// bitvue-core/src/lib.rs — "core"가 코덱별 타입을 재학습해버림
pub enum SliceType { I, P, B } // HEVC/AVC 개념이 core로 역류
pub struct MacroblockInfo { ... } // AVC 전용 개념이 core에 정의됨

// bitvue-formats (컨테이너 레벨)가 계층을 건너뛰고 코덱 파서를 직접 호출
// bitvue-formats/src/mp4.rs
use bitvue_avc::AvcParser; // 컨테이너가 특정 코덱을 직접 알아야 함(계층 위반)
```

**문제**:
- `bitvue-codecs`가 하는 일이 재수출뿐이라면 별도 크레이트로 존재할 이유가 없고, 컴파일 단위만 늘려 빌드 시간과 크레이트 그래프 복잡도를 키움
- `bitvue-codecs-parser`가 한 함수짜리 switch라면 "크레이트"라는 배포/버전 단위를 부여할 만큼의 응집도가 없음 — 함수 하나가 워크스페이스 그래프에 노드 하나를 추가한 셈
- `bitvue-core`가 `SliceType`, `MacroblockInfo` 같은 코덱 특화 개념을 알게 되면, core는 더 이상 "코덱에 무지한 공통 기반"이 아니게 되어 새 코덱(VVC, AV2) 추가 시 core를 계속 건드려야 하는 역방향 의존이 생김
- `bitvue-formats`(컨테이너: MP4/MKV/IVF)가 `bitvue-avc`(코덱 파서)를 직접 참조하면, "컨테이너는 코덱을 몰라야 한다"는 계층 원칙이 깨지고 컨테이너 크레이트 수만큼 코덱 크레이트 조합의 의존이 폭발적으로 늘어남
- 이런 역전이 누적되면 워크스페이스 의존 그래프가 사실상 완전 그래프에 가까워져 하나를 고치면 전체를 재컴파일해야 하는 상황이 됨

**발생 조건**:
- "관심사 분리"를 기계적으로 적용해 실제 응집도 없이 크레이트를 쪼갠 경우
- core 크레이트에 "일단 여기 두면 다들 쓸 수 있으니까" 하며 특정 코덱 개념을 얹은 경우
- 컨테이너 파서가 "이 코덱이면 이렇게 특별 처리"라는 최적화/편의를 위해 코덱 크레이트를 직접 참조하게 된 경우

**권장**:
```rust
// 계층을 명시적으로 고정한다 (의존 방향은 아래로만):
//
//   bitvue-core        (코덱/컨테이너 무지: 공통 타입, 에러, newtype)
//        ^
//   bitvue-formats      (컨테이너: MP4/MKV/IVF — 코덱을 모름, opaque payload만 추출)
//        ^
//   bitvue-codecs-parser (코덱 dispatch: CodecKind -> dyn CodecParser, 여기서만 코덱별 크레이트 참조)
//        ^
//   bitvue-avc / bitvue-hevc / bitvue-vp9 / bitvue-av1 / bitvue-vvc (개별 코덱 구현)
//
// bitvue-core: 코덱 특화 타입 없음
pub enum SliceType { /* 없음 — bitvue-avc, bitvue-hevc 각자 정의 */ }

// bitvue-formats: opaque 바이트만 다룬다 (코덱을 모른다)
pub struct EncodedSample {
    pub codec_hint: CodecKind, // core의 enum만 참조, 파싱은 안 함
    pub payload: Bytes,
}

// bitvue-codecs-parser: 유일하게 모든 코덱 크레이트를 참조하는 "조립 지점"
pub fn parse(sample: &EncodedSample) -> Result<ParsedFrame, ParseError> {
    match sample.codec_hint {
        CodecKind::Avc => bitvue_avc::parse(&sample.payload).map(Into::into),
        CodecKind::Hevc => bitvue_hevc::parse(&sample.payload).map(Into::into),
        ...
    }
}
```
- 크레이트 분리 기준은 "독립적으로 버전 관리/재사용될 이유가 있는가"이지 "폴더를 나누고 싶은가"가 아니다
- 순수 재수출 shim(`bitvue-codecs`)은 존재 가치를 재검토하고, 없앨 수 없다면 최소한 "왜 필요한지"를 크레이트 문서에 명시
- `cargo depgraph`나 workspace lint로 "core가 codec-* 크레이트를 의존하면 실패"하는 아키텍처 테스트를 CI에 추가

**탐지 방법**:
- Structural: `cargo metadata`로 의존성 그래프를 뽑아 core/formats가 codec-* 크레이트를 직접 의존하는지 자동 검사
- Structural: 각 크레이트의 `pub` 심볼 수 대비 코드 라인 수 비율이 지나치게 낮은(즉 재수출만 하는) 크레이트를 스캔
- Manual: 아키텍처 리뷰에서 "이 크레이트가 없어지면 무엇이 깨지는가"를 크레이트별로 질문

**예외**:
- 워크스페이스 초기 단계에서 향후 분리를 예정하고 임시로 얇은 어댑터 크레이트를 둔 경우(단, TODO/이슈로 추적)
- 코덱별 크레이트를 정말 독립 배포(crates.io 공개, 서드파티가 코덱 하나만 의존)할 계획이 있다면 각 코덱 크레이트 분리 자체는 정당함 — 문제는 core/formats가 그 방향을 거슬러 의존하는 것

**Bitvue 판정**: Confirmed — crates/bitvue-codecs/src/lib.rs(24줄, 순수 재수출 shim)와 사실상 플레이스홀더인 crates/bitvue-codecs-parser/src/lib.rs가 API-005 예시와 일치; crates/bitvue-core/Cargo.toml:24-29 주석이 core가 코덱 인덱서 feature를 가지려다 순환 의존이 생겨 비활성화했음을 명시적으로 기록

---

### API-006: 과도한 제네릭 확산으로 인한 모노모피제이션 비대화
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// bitvue-formats: 컨테이너 파서가 모든 입력 소스 타입에 제네릭
pub struct Demuxer<R: Read + Seek, C: Clock = SystemClock, L: Logger = NullLogger> {
    reader: R,
    clock: C,
    logger: L,
}

impl<R: Read + Seek, C: Clock, L: Logger> Demuxer<R, C, L> {
    pub fn parse_box_tree(&mut self) -> Result<BoxTree, ParseError> { ... }
    pub fn extract_track(&mut self, id: TrackId) -> Result<Track, ParseError> { ... }
    // 이런 메서드 30개가 전부 R, C, L 세 제네릭에 걸쳐 재인스턴스화됨
}

// 호출부: File, Cursor<Vec<u8>>, MmapReader, 각각의 Clock/Logger 조합마다
// 위 30개 메서드 전체가 별도로 컴파일됨 (LLVM IR 중복 생성)
let d1: Demuxer<File, SystemClock, NullLogger> = ...;
let d2: Demuxer<Cursor<Vec<u8>>, SystemClock, NullLogger> = ...;
let d3: Demuxer<MmapReader, TestClock, StdoutLogger> = ...;
```

**문제**:
- `Demuxer<R, C, L>`을 사용하는 조합 수만큼 30여 개 메서드 전체가 각각 별도의 기계어로 모노모피제이션되어 바이너리 크기가 조합 수에 비례해 증가
- 컴파일 시간이 늘어나 CI 피드백 루프가 느려지고, 특히 릴리스 빌드(LTO 포함)에서 체감이 큼
- 제네릭 파라미터 3개를 매 타입 시그니처에 반복해야 해서 호출부 코드가 장황해지고 타입 추론 에러 메시지가 길어져 가독성이 떨어짐
- `Clock`, `Logger`는 프레임 파싱 hot path와 무관한데도 hot path 코드(`parse_box_tree`)까지 제네릭 파라미터로 얽혀 있어, 실제로 성능이 중요한 부분과 그렇지 않은 부분을 구분하지 못함

**발생 조건**:
- "테스트 가능성을 위해 전부 주입 가능하게 하자"는 목표로 모든 협력 객체를 제네릭 파라미터화한 경우
- 실제로는 `Read + Seek` 하나만 다형성이 필요한데 습관적으로 부수적인 의존성(Clock, Logger)까지 함께 제네릭화한 경우

**권장**:
```rust
// hot path에 실제로 필요한 제네릭만 남긴다
pub struct Demuxer<R: Read + Seek> {
    reader: R,
    clock: Box<dyn Clock>,   // 호출 빈도 낮음 → trait object로 충분
    logger: Box<dyn Logger>, // 로깅은 애초에 동적 디스패치 비용이 무의미
}

impl<R: Read + Seek> Demuxer<R> {
    pub fn parse_box_tree(&mut self) -> Result<BoxTree, ParseError> { ... }
}

// 정말 다형성이 필요 없는 경우 dyn Read + Seek로 아예 타입을 지운다
pub struct AnyDemuxer {
    reader: Box<dyn ReadSeek>, // Read + Seek를 합친 sealed trait
}
```
- "이 제네릭이 hot path 성능을 위해 꼭 정적 디스패치여야 하는가?"를 파라미터마다 검토
- 호출 빈도가 낮은 협력 객체(로거, 클록, 메트릭 싱크)는 trait object로 지워 모노모피제이션 대상에서 제외
- `cargo bloat --crates`와 `-Z self-profile`(nightly) 또는 `cargo build --timings`로 실제 비대화를 측정하고 나서 최적화 대상을 정한다

**탐지 방법**:
- Static: 공개 구조체/함수의 제네릭 파라미터 개수가 3개 이상인 항목을 스캔
- Runtime: `cargo bloat`로 심볼별 바이너리 기여도 상위 항목이 특정 제네릭 타입의 여러 인스턴스화인지 확인
- Runtime: `cargo build --timings`로 크레이트별 컴파일 시간 추이를 릴리스 태그마다 비교

**예외**:
- 실제로 hot path에서 반복 호출되어 정적 디스패치·인라이닝 이득이 벤치마크로 확인된 제네릭(예: `BitReader<R>`, API-003 참고)은 정당한 사용
- 조합 수가 적고(2~3개) 고정되어 있어 바이너리 크기 증가가 무시할 수준인 경우

**Bitvue 판정**: N/A — 공개 struct/fn 중 제네릭 타입 파라미터 3개 이상인 것을 찾지 못함 (LruCache<K,V>, StateMachine<S,E> 등 2개 이하만 존재, crates/bitvue-core/src/state_machine.rs:64-338)

---

### API-007: 단순 구조체에 대한 빌더 패턴 남용
**분류**: API·타입 설계 · **심각도**: Low · **탐지**: Manual/Structural

**나쁜 예**:
```rust
// bitvue-core: 필드 3개짜리 단순 값 타입에 풀 빌더
pub struct FrameRegionBuilder {
    x: Option<u32>,
    y: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
}

impl FrameRegionBuilder {
    pub fn new() -> Self { Self { x: None, y: None, width: None, height: None } }
    pub fn x(mut self, x: u32) -> Self { self.x = Some(x); self }
    pub fn y(mut self, y: u32) -> Self { self.y = Some(y); self }
    pub fn width(mut self, w: u32) -> Self { self.width = Some(w); self }
    pub fn height(mut self, h: u32) -> Self { self.height = Some(h); self }
    pub fn build(self) -> Result<FrameRegion, BuildError> {
        Ok(FrameRegion {
            x: self.x.ok_or(BuildError::MissingField("x"))?,
            y: self.y.ok_or(BuildError::MissingField("y"))?,
            width: self.width.ok_or(BuildError::MissingField("width"))?,
            height: self.height.ok_or(BuildError::MissingField("height"))?,
        })
    }
}

// 호출부: 4줄이면 될 걸 6줄 + 런타임 에러 가능성
let region = FrameRegionBuilder::new().x(0).y(0).width(1920).height(1080).build()?;
```

**문제**:
- 모든 필드가 필수이고 타입이 단순한데도 빌더를 쓰면 `build()` 시점까지 "필드 누락"이 런타임 에러로 지연됨 — 생성자면 컴파일 타임에 끝날 문제
- 보일러플레이트(4개 필드에 setter 4개 + build)가 실제 얻는 이득(선택적 필드 조합, 가독성) 없이 코드량만 늘림
- 호출부가 `Result`를 처리해야 해서 단순 값 생성에 `?` 전파가 강제됨

**발생 조건**:
- 모든 필드가 필수이고 타입이 간단한(primitive, 소수 필드) 구조체에 관성적으로 빌더를 적용한 경우
- "다른 곳에서 빌더를 쓰니까 일관성 있게"라는 이유만으로 불필요한 곳까지 확장한 경우

**권장**:
```rust
// 필수 필드만 있으면 그냥 생성자 함수로 충분
#[derive(Debug, Clone, Copy)]
pub struct FrameRegion { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }

impl FrameRegion {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

let region = FrameRegion::new(0, 0, 1920, 1080); // 컴파일 타임에 필드 누락 불가능
```
- 빌더는 "선택적 필드가 5개 이상이거나 조합에 따라 유효성 검증이 필요한 경우"로 한정
- 단순 필수 필드 구조체는 생성자 함수 또는 구조체 리터럴로 충분

**탐지 방법**:
- Structural: `*Builder` 구조체 중 옵션 필드가 전혀 없거나(모두 필수) 필드 수가 3개 이하인 것을 스캔
- Manual: 코드 리뷰에서 "이 빌더가 없으면 호출부가 더 나빠지는가"를 질문

**예외**:
- 선택적 필드가 많고 서로 배타적/의존적인 조합 검증이 필요한 경우(예: 인코더 옵션 수십 개) 빌더가 정당
- typestate 빌더로 컴파일 타임에 필수 필드 설정을 강제하는 고급 패턴은 이 안티패턴에 해당하지 않음(API-016 참고)

**Bitvue 판정**: Confirmed — AvcFrameBuilder/HevcFrameBuilder/Vp9FrameBuilder가 12~14개 필드 대부분을 `.ok_or_else(...)?`로 필수 처리하면서 `build()`가 `Result<_, String>`을 반환, 생성자로 대체 가능한 필드 누락 검증이 런타임으로 지연됨 (crates/bitvue-avc/src/frames.rs:67-172)

---

### API-008: 놀랍거나 비용이 큰 `impl Default`
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Runtime/Manual

**나쁜 예**:
```rust
// bitvue-decode: Default가 실제로는 무거운 초기화를 수행
pub struct DecodeSession {
    hw_context: HwAccelContext,
    frame_pool: Vec<FrameBuffer>,
}

impl Default for DecodeSession {
    fn default() -> Self {
        // GPU 컨텍스트 생성, 수십 MB 프레임 풀 사전 할당 — "기본값"이라는 이름과 맞지 않게 무거움
        let hw_context = HwAccelContext::probe_and_init()
            .expect("no hardware decoder found"); // Default에서 panic 가능
        let frame_pool = (0..32).map(|_| FrameBuffer::alloc(1920 * 1080 * 3 / 2)).collect();
        Self { hw_context, frame_pool }
    }
}

// 무해해 보이는 호출이 실제로는 GPU 프로빙 + 수십 MB 할당을 유발
let session = DecodeSession::default(); // 놀랍게도 느리고 panic 가능
```

**문제**:
- `Default::default()`는 관례적으로 "값싸고 실패하지 않는" 초기화로 여겨지는데, 하드웨어 프로빙·대용량 할당·panic 가능성이 그 기대를 깨뜨림
- `#[derive(Default)]`가 붙은 다른 필드와 섞여 있으면 리뷰어가 무거운 부분을 놓치기 쉬움
- `Option<DecodeSession>::unwrap_or_default()` 같은 흔한 관용구가 예상 밖의 GPU 초기화를 유발할 수 있음
- 실패 가능한 초기화가 `Default`(반환 타입이 `Result`가 아님) 안에서 `expect`/`panic`으로 처리되어 에러 핸들링 경로가 사라짐

**발생 조건**:
- "이 타입도 기본값이 있으면 편하겠다"는 이유로 실패 가능하거나 비용이 큰 초기화 로직에 `Default`를 구현한 경우
- 테스트 편의를 위해 `Default`를 붙였는데 그것이 그대로 프로덕션 코드 경로에 노출된 경우

**권장**:
```rust
pub struct DecodeSession {
    hw_context: HwAccelContext,
    frame_pool: Vec<FrameBuffer>,
}

impl DecodeSession {
    // 이름이 비용/실패 가능성을 드러낸다
    pub fn new_with_hw_accel() -> Result<Self, DecodeSessionError> {
        let hw_context = HwAccelContext::probe_and_init()?;
        let frame_pool = (0..32).map(|_| FrameBuffer::alloc(1920 * 1080 * 3 / 2)).collect();
        Ok(Self { hw_context, frame_pool })
    }
}
// Default는 구현하지 않거나, 정말 값싼 fallback(소프트웨어 디코더 등)이 있을 때만 구현
```
- `Default`는 "할당/실패가 없고 즉시 반환되는" 초기화에만 사용
- 비용이 크거나 실패 가능한 생성은 의도가 드러나는 이름의 명시적 생성자(`new_with_hw_accel`, `try_new`)로 노출

**탐지 방법**:
- Manual: `impl Default` 본문에서 I/O, 하드웨어 접근, `Vec`/`HashMap`의 대용량 사전 할당, `.expect()`/`.unwrap()`을 코드 리뷰에서 확인
- Runtime: 벤치마크로 `T::default()` 호출 비용을 측정해 마이크로초 단위를 넘어가면 flag

**예외**:
- 소프트웨어 fallback처럼 항상 성공하고 비용이 상수 시간인 초기화라면 무거워 보여도 `Default`가 적절할 수 있음
- 테스트 전용 mock 타입의 `Default`는 테스트 코드에 한정된다면 허용

**Bitvue 판정**: N/A (근거 갱신) — 인용된 src-tauri/src/services/decode_service.rs는 삭제됨; 현재 아키텍처(crates/bitvue-decode/src/strategy/*, crates/bitvue-engine 전역 50+ `impl Default`)를 재확인해도 하드웨어 프로빙/대용량 사전할당/panic이 있는 `impl Default`는 없음 — 하드웨어 가속 후보(MetalStrategy::new(), crates/bitvue-decode/src/strategy/metal.rs:27-29)조차 zero-sized `Self` 반환뿐, 실제 GPU 디바이스 획득(`MTLCreateSystemDefaultDevice`, metal.rs:122-124)은 별도의 실패 가능한 `fn new() -> Option<Self>`로 분리돼 있어 원 판정과 동일 결론 유지

---

### API-009: semver를 흔드는 공개 필드 vs 접근자 메서드 왔다갔다
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Structural/Manual

**나쁜 예**:
```rust
// v0.3.0: 공개 필드로 시작
pub struct ParsedFrame {
    pub frame_index: u32,
    pub is_keyframe: bool,
    pub pts: i64,
}

// v0.4.0: 내부적으로 lazy 계산이 필요해져서 필드를 접근자로 바꿈 (breaking change)
pub struct ParsedFrame {
    frame_index: u32,
    is_keyframe: bool,
    pts_raw: i64,
    timescale: u32,
}
impl ParsedFrame {
    pub fn pts(&self) -> i64 { self.pts_raw * 1000 / self.timescale as i64 }
    // frame.pts 로 접근하던 모든 다운스트림 코드가 컴파일 에러
}
```

**문제**:
- 공개 필드로 시작하면 구조체 리터럴 생성(`ParsedFrame { .. }`)과 필드 직접 접근이 즉시 공개 API 계약의 일부가 되어, 이후 필드를 추가/이름 변경/계산식으로 전환할 때마다 breaking change가 됨
- 반대로 처음부터 모든 필드에 getter를 강제하면 단순 데이터 홀더에도 불필요한 보일러플레이트가 쌓임
- 필드 공개 여부에 대한 팀 내 일관된 기준이 없으면 크레이트마다, 심지어 같은 크레이트의 구조체마다 정책이 달라져 API 표면이 들쭉날쭉해짐

**발생 조건**:
- 초기 프로토타입에서 편의상 필드를 전부 `pub`으로 열어두고, 나중에 계산 로직이나 불변조건이 필요해지면서 뒤늦게 캡슐화가 필요해진 경우
- 워크스페이스 내부 크레이트 간에도 semver 계약을 진지하게 다루지 않아 이런 변경이 "사소한 리팩터"로 취급되는 경우

**권장**:
```rust
// 처음부터 정책을 정한다: "값 타입(POD)은 필드 공개, 불변조건/계산이 있는 타입은 비공개+접근자"
#[derive(Debug, Clone, Copy)]
pub struct FrameRegion { pub x: u32, pub y: u32, pub width: u32, pub height: u32 } // 순수 데이터 → pub 필드 OK

pub struct ParsedFrame {
    frame_index: u32,
    is_keyframe: bool,
    pts_raw: i64,
    timescale: u32,
} // 파생 계산/불변조건 있음 → 처음부터 비공개 + 접근자
impl ParsedFrame {
    pub fn frame_index(&self) -> u32 { self.frame_index }
    pub fn is_keyframe(&self) -> bool { self.is_keyframe }
    pub fn pts(&self) -> i64 { self.pts_raw * 1000 / self.timescale as i64 }
}
```
- 구조체를 처음 설계할 때 "이 타입에 향후 계산/불변조건이 생길 가능성이 있는가"를 기준으로 필드 공개 여부를 결정하고, 크레이트 전체에 일관되게 적용
- `#[non_exhaustive]`를 구조체에도 적용하면(필드 추가는 여전히 breaking이지만) 외부에서 구조체 리터럴로 생성하는 것을 막아 향후 필드 추가의 파급을 줄일 수 있음

**탐지 방법**:
- Structural: `cargo public-api` 또는 `cargo semver-checks`로 릴리스 간 필드 공개 상태 변경을 자동 감지
- Manual: API 리뷰 체크리스트에 "이 구조체는 POD인가, 불변조건이 있는가"를 명시적 질문으로 포함

**예외**:
- 워크스페이스 내부에서만 쓰이고 외부에 배포되지 않는 크레이트는 semver 부담이 없으므로 자유롭게 필드를 공개해도 무방
- 성능이 극도로 중요해 접근자 호출조차(인라인되지 않을 경우) 부담되는 극히 좁은 hot path 타입은 필드 공개가 실용적일 수 있음(단, `#[inline]` 접근자로 대부분 해결됨을 먼저 확인)

**Bitvue 판정**: N/A — 전 크레이트가 `version.workspace = true`로 lockstep 버전 관리되고 crates.io에 배포되지 않는 내부 모노레포(bitvue-benchmarks만 명시적 `publish = false`)라 문서 자체의 예외 조항(워크스페이스 내부 전용 크레이트)에 해당

---

### API-010: 문자열 기반 에러 vs 구조화된 에러 enum
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
// bitvue-formats
pub fn parse_mp4(data: &[u8]) -> Result<BoxTree, String> {
    if data.len() < 8 {
        return Err("file too short to contain a valid box header".to_string());
    }
    if &data[4..8] != b"ftyp" && &data[4..8] != b"moov" {
        return Err(format!("unexpected box type at offset 0: {:?}", &data[4..8]));
    }
    ...
}

// 호출부는 문자열 매칭으로 에러를 구분해야 함(!)
match parse_mp4(&data) {
    Err(e) if e.contains("too short") => { /* ... */ }
    Err(e) if e.starts_with("unexpected box type") => { /* ... */ }
    Err(e) => { /* 나머지는 뭉뚱그려 처리 */ }
    Ok(tree) => { ... }
}
```

**문제**:
- 문자열 에러는 호출부가 원인을 프로그램적으로 구분하려면 문자열 파싱/매칭에 의존해야 해서 취약하고(메시지 문구를 바꾸면 조용히 깨짐), i18n도 불가능
- 에러에 부가 정보(offset, expected vs actual box type)를 실어도 구조화되지 않아 UI(Tauri 프런트엔드)에서 다국어 메시지나 구조화된 diagnostics를 만들 수 없음
- `std::error::Error` trait을 구현하지 않아 `?`로 다른 에러 타입과 조합하거나 `source()` 체인을 구성하기 어려움
- 어떤 에러가 발생 가능한지 함수 시그니처만 봐서는 알 수 없어(모두 동일하게 `String`) 호출부가 방어적으로 모든 경우를 문자열 매칭해야 함

**발생 조건**:
- 프로토타입 단계에서 `format!()`로 빠르게 에러 메시지를 만들다가 그대로 공개 API의 에러 타입이 된 경우
- 여러 실패 원인을 급하게 하나의 함수에 몰아넣으면서 "그냥 문자열로" 처리한 경우

**권장**:
```rust
// bitvue-formats::error
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Mp4ParseError {
    #[error("file too short: need at least {needed} bytes, got {actual}")]
    TooShort { needed: usize, actual: usize },

    #[error("unexpected box type at offset {offset}: expected one of {expected:?}, got {actual:?}")]
    UnexpectedBoxType { offset: FileOffset, expected: &'static [&'static str], actual: [u8; 4] },

    #[error("I/O error while reading box tree")]
    Io(#[from] std::io::Error),
}

pub fn parse_mp4(data: &[u8]) -> Result<BoxTree, Mp4ParseError> { ... }

// 호출부는 타입 안전하게 매칭 가능, 컴파일러가 exhaustiveness 강제(non_exhaustive라 catch-all 요구)
match parse_mp4(&data) {
    Err(Mp4ParseError::TooShort { needed, actual }) => { /* UI에 구조화된 메시지 표시 */ }
    Err(Mp4ParseError::UnexpectedBoxType { offset, .. }) => { /* offset으로 hex view 점프 */ }
    Err(e) => { /* 나머지 */ }
    Ok(tree) => { ... }
}
```
- `thiserror`로 에러 enum을 정의하고 `#[from]`으로 하위 에러를 자동 변환
- 각 variant에 UI/로깅에 필요한 구조화된 필드(offset, expected/actual)를 담아 문자열 파싱 없이 프로그램적으로 활용 가능하게 함
- 라이브러리 크레이트는 `anyhow` 대신 구조화된 에러 enum을 반환하고, `anyhow`는 애플리케이션 최상위(bitvue-cli의 `main`)에서만 사용

**탐지 방법**:
- Static: 공개 함수 시그니처에서 `Result<_, String>` 또는 `Result<_, Box<dyn Error>>`(구조화되지 않은 catch-all)를 grep
- Structural: 라이브러리 크레이트의 `Cargo.toml`에서 `anyhow` 의존이 있는지 확인(애플리케이션 크레이트는 예외)

**예외**:
- `bitvue-cli`의 `main()`처럼 에러를 프로그램적으로 구분할 필요 없이 사용자에게 출력만 하면 되는 최종 소비 지점에서는 `anyhow::Error`가 적절
- 내부 전용(비공개) 헬퍼 함수의 에러는 문자열이어도 파급이 제한적

**Bitvue 판정**: Confirmed — 대다수 코덱 크레이트는 thiserror 기반 에러(AvcError, HevcError 등)를 쓰지만 일부 공개 API는 `Result<_, String>`으로 남아있음 (crates/bitvue-decode/src/traits.rs:79,291의 `Decoder` trait; crates/bitvue-avc/src/frames.rs:158; crates/bitvue-vp9/src/frames.rs:129)

---

### API-011: Trait object-safety 위반으로 인한 우회 설계
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
// bitvue-codecs-parser: object-safe하지 않은 trait을 dyn으로 쓰려다 발생하는 문제
pub trait CodecParser {
    fn parse_frame(&mut self, data: &[u8]) -> Result<Frame, ParseError>;

    // 제네릭 메서드 — trait object에 담을 수 없음(object-safety 위반)
    fn parse_into<T: FromFrame>(&mut self, data: &[u8]) -> Result<T, ParseError> {
        Ok(T::from_frame(self.parse_frame(data)?))
    }

    // Self 반환 — 역시 object-safety 위반
    fn clone_parser(&self) -> Self where Self: Sized;
}

// 결과: Box<dyn CodecParser>를 쓰고 싶은데 컴파일 안 됨
// 우회책으로 각 코덱마다 별도 enum 분기를 손으로 다시 작성하게 됨
pub enum AnyParser {
    Avc(bitvue_avc::AvcParser),
    Hevc(bitvue_hevc::HevcParser),
    // parse_into, clone_parser를 쓰려면 결국 여기서 다시 match
}
```

**문제**:
- `CodecParser`를 애초에 `dyn CodecParser`로 쓸 계획이었다면 제네릭 메서드나 `Self` 반환 메서드를 trait에 넣은 순간부터 object-safety가 깨져 컴파일 에러가 남
- 문제를 뒤늦게 발견하면 "trait 하나로 다형성을 얻겠다"는 원래 설계가 무산되고, 결국 손으로 enum 분기를 다시 작성하는 이중 작업이 발생
- 팀원들이 object-safety 규칙(제네릭 메서드 금지, `Self` 반환 금지, `Sized` 바운드 없는 `where Self: Sized` 예외 규칙 등)을 숙지하지 못하면 같은 실수가 반복됨

**발생 조건**:
- trait을 설계할 때 "이걸 나중에 `Box<dyn>`으로 쓸지"를 미리 결정하지 않고 메서드를 자유롭게 추가한 경우
- 제네릭 편의 메서드(`parse_into<T>`)를 trait 안에 넣고 싶은 유혹이 있을 때(제네릭 코드와 다형성 코드를 한 trait에 섞으려는 시도)

**권장**:
```rust
// dyn 대상 trait은 처음부터 object-safe하게 좁게 설계
pub trait CodecParser {
    fn parse_frame(&mut self, data: &[u8]) -> Result<Frame, ParseError>;
    fn codec_kind(&self) -> CodecKind;
}
// Box<dyn CodecParser>로 자유롭게 사용 가능

// 제네릭 편의 기능은 trait 밖의 free function이나 별도 확장 trait으로 분리
pub fn parse_into<T: FromFrame>(
    parser: &mut dyn CodecParser,
    data: &[u8],
) -> Result<T, ParseError> {
    Ok(T::from_frame(parser.parse_frame(data)?))
}

// Clone이 필요하면 object-safe한 우회(예: dyn_clone 크레이트, 또는 명시적 enum)를 사용
```
- "이 trait을 `dyn`으로 쓸 것인가, 제네릭 바운드로만 쓸 것인가"를 설계 시점에 먼저 결정
- object-safe해야 하는 trait은 제네릭 메서드, `Self` 반환, 연관 상수를 배제하고 최소한으로 유지
- 두 용도가 모두 필요하면 trait을 분리(`CodecParser`는 dyn용, `CodecParserExt`는 제네릭용)

**탐지 방법**:
- Static: `cargo check`가 `Box<dyn Trait>` 사용 시점에 object-safety 에러를 즉시 보고하므로, CI에서 이 에러가 우회 코드(수동 enum 재작성)로 "해결"되지 않았는지 리뷰
- Manual: trait 설계 리뷰에서 "이 trait의 모든 메서드가 object-safe한가"를 체크리스트화

**예외**:
- 처음부터 제네릭 바운드(`fn f<P: CodecParser>(p: &mut P)`)로만 쓸 계획이고 `dyn`이 전혀 필요 없는 trait이라면 object-safety를 신경 쓸 필요 없음

**Bitvue 판정**: N/A — `ParserStrategy` trait(crates/bitvue-codecs-parser/src/parser_strategy.rs:222)은 object-safe하게 설계되어 실제로 `Box<dyn ParserStrategy>`(같은 파일 804행)로 쓰이고 있으며, 제네릭 메서드나 `Self` 반환 위반을 찾지 못함

---

### API-012: 호출부에서 이름 짓기 힘든 과도한 제네릭 공개 API
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Manual/Structural

**나쁜 예**:
```rust
// bitvue-metrics: 타입 파라미터 4개짜리 공개 API
pub struct MetricPipeline<S, F, A, O>
where
    S: FrameSource,
    F: Fn(&Frame) -> Sample,
    A: Aggregator<Sample>,
    O: OutputSink<A::Output>,
{
    source: S,
    extractor: F,
    aggregator: A,
    sink: O,
}

// 호출부에서 타입을 명시하려면 이렇게 써야 함
fn build_pipeline() -> MetricPipeline<
    FileFrameSource,
    impl Fn(&Frame) -> Sample,
    PsnrAggregator,
    JsonFileSink<<PsnrAggregator as Aggregator<Sample>>::Output>,
> { ... }
// 반환 타입을 함수 시그니처에 쓸 수조차 없어 impl Trait이나 Box<dyn>으로 우회해야 함
```

**문제**:
- 타입 파라미터가 4개 이상이면 구체 타입을 호출부에서 이름 짓는 것 자체가 거의 불가능해지고(특히 클로저 타입은 이름이 없음), 구조체를 필드로 저장하려는 다운스트림 코드가 막힘
- 에러 메시지가 `MetricPipeline<FileFrameSource, [closure@...], PsnrAggregator, JsonFileSink<f64>>` 처럼 읽기 어려워짐
- 제네릭 하나를 바꾸면(예: `Aggregator` trait에 메서드 추가) 연쇄적으로 다른 타입 파라미터의 바운드까지 영향받아 breaking change 파급이 커짐

**발생 조건**:
- "런타임 비용 없이 모든 구성 요소를 다형화하자"는 목표로 설계 초기에 파이프라인/조합기 패턴에 제네릭을 아낌없이 추가한 경우
- 클로저를 타입 파라미터로 받는 API를 공개 반환 타입에 그대로 노출한 경우

**권장**:
```rust
// 성능이 중요하지 않은 조합 지점은 trait object로 타입을 지운다
pub struct MetricPipeline {
    source: Box<dyn FrameSource>,
    extractor: Box<dyn Fn(&Frame) -> Sample>,
    aggregator: Box<dyn Aggregator<Sample, Output = MetricValue>>,
    sink: Box<dyn OutputSink<MetricValue>>,
}

impl MetricPipeline {
    pub fn new(
        source: impl FrameSource + 'static,
        extractor: impl Fn(&Frame) -> Sample + 'static,
        aggregator: impl Aggregator<Sample, Output = MetricValue> + 'static,
        sink: impl OutputSink<MetricValue> + 'static,
    ) -> Self {
        Self { source: Box::new(source), extractor: Box::new(extractor), aggregator: Box::new(aggregator), sink: Box::new(sink) }
    }
}
// 반환 타입이 간단해지고, 필드로 저장하거나 Vec<MetricPipeline>에 담기도 쉬움
```
- 정말 hot path인 부분(프레임당 반복 호출)만 제네릭을 유지하고, 조립/설정 단계는 trait object로 단순화
- 제네릭 파라미터가 2개를 넘어가면 "이 중 몇 개가 실제로 정적 디스패치 성능 이득이 있는가"를 재검토
- 타입 별칭(`pub type PsnrPipeline = MetricPipeline<...>`)으로 흔한 조합에 이름을 부여하는 것도 완화책이 될 수 있음

**탐지 방법**:
- Structural: 공개 구조체/함수의 타입 파라미터 개수가 3개 이상인 항목을 스캔
- Manual: "이 타입을 변수에 저장하거나 구조체 필드에 담으려면 몇 줄이 필요한가"를 API 리뷰에서 시험

**예외**:
- 라이브러리 내부(비공개) 구현에서 성능이 검증된 제네릭 파이프라인은 문제 없음 — 공개 API로 그 복잡도가 새어나가는 것이 문제
- 제네릭 파라미터에 합리적 기본값(`= DefaultAggregator`)이 있어 대부분의 호출부가 타입을 명시할 필요가 없는 경우

**Bitvue 판정**: N/A — 공개 API에서 제네릭 파라미터 3개 이상인 struct/fn을 찾지 못함(API-006과 동일 근거)

---

### API-013: 향후 변형이 필요한 enum에 `#[non_exhaustive]` 누락
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
// bitvue-core v0.5.0
pub enum CodecKind {
    Avc,
    Hevc,
    Vp9,
    Av1,
}

// 다운스트림(bitvue-mcp, bitvue-cli, 서드파티 플러그인)에서
match codec {
    CodecKind::Avc => ...,
    CodecKind::Hevc => ...,
    CodecKind::Vp9 => ...,
    CodecKind::Av1 => ...,
    // catch-all 없이 exhaustive하게 매칭 — 컴파일러가 "다 커버했다"고 인정
}

// v0.6.0에서 VVC 지원 추가
pub enum CodecKind {
    Avc, Hevc, Vp9, Av1,
    Vvc, // 새 variant 추가 → 이건 semver major bump가 필요한 breaking change
}
// 위 다운스트림 코드가 전부 컴파일 에러(match not exhaustive) → 강제 major 버전 업
```

**문제**:
- 코덱 지원 확장은 이 프로젝트에서 사실상 확정된 미래(VVC, AV2 등 신규 코덱 추가)인데 `#[non_exhaustive]` 없이 enum을 공개하면, variant 추가마다 semver major bump와 모든 다운스트림 `match`의 강제 수정이 뒤따름
- 워크스페이스 내부 크레이트 사이에서도 같은 문제가 발생 — `bitvue-core`의 enum에 새 코덱을 추가할 때마다 `bitvue-cli`, `bitvue-mcp` 등 최상위 크레이트까지 연쇄 수정이 필요해짐
- 반대로 항상 `#[non_exhaustive]`를 남발하면 다운스트림이 항상 catch-all(`_ => ...`)을 강제당해, 새 코덱이 추가됐을 때 "처리를 빠뜨렸다"는 컴파일 경고조차 받지 못하는 트레이드오프도 있음

**발생 조건**:
- 향후 확장이 명백한 도메인 enum(코덱 종류, 컨테이너 포맷, 색공간, 에러 종류)을 처음 설계할 때 `#[non_exhaustive]`를 고려하지 않은 경우
- 반대로 이미 안정적이고 확장 계획이 없는 enum(예: `Endianness { Big, Little }`)에 습관적으로 `#[non_exhaustive]`를 붙여 불필요한 catch-all을 강제하는 경우도 동일 카테고리의 실수

**권장**:
```rust
// 확장이 예정된 도메인 enum
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecKind {
    Avc,
    Hevc,
    Vp9,
    Av1,
    // 향후 Vvc, Av2 추가 시 이것만으로는 breaking change 아님(단, 값 생성자는 별도 필요)
}

// 워크스페이스 내부(같은 저장소, 항상 함께 버전업되는) 크레이트 간에는
// non_exhaustive가 오히려 불필요한 catch-all을 강제할 수 있으므로,
// "외부에 배포되는 공개 API 경계"에만 선택적으로 적용
```
- 판단 기준: 이 enum이 (a) 도메인상 확장이 예정되어 있고 (b) 크레이트 경계를 넘어(특히 워크스페이스 밖 서드파티로) 노출되는가. 둘 다 해당하면 `#[non_exhaustive]` 적용
- 이미 안정적이고 수학적/물리적으로 닫힌 집합(예: `Endianness`, `BitOrder`)에는 붙이지 않아 exhaustiveness 검사의 이점을 유지
- `#[non_exhaustive]` 적용 시 생성자 함수(`CodecKind::from_fourcc`)도 함께 제공해 외부에서 값 생성이 막히지 않게 함

**탐지 방법**:
- Structural: `cargo semver-checks`로 과거 릴리스에서 enum에 variant를 추가했을 때 major bump 없이 넘어간 이력이 있는지 검사
- Manual: 도메인 enum 목록(코덱, 컨테이너, 색공간, 프로파일/레벨)을 뽑아 "향후 확장 가능성"과 "`#[non_exhaustive]` 여부"를 매핑한 표로 리뷰

**예외**:
- 워크스페이스 내부에서만 쓰이고 항상 모든 크레이트가 동시에 재컴파일/재배포되는(lockstep 버전) 경우, `#[non_exhaustive]`의 이점이 크지 않을 수 있음
- 성능이 극도로 중요한 hot path의 enum에서 `#[non_exhaustive]`로 인한 catch-all 분기가 최적화를 방해한다면(드문 경우) 예외 검토

**Bitvue 판정**: Suspected — 코드베이스 전체에 `#[non_exhaustive]`가 한 곳도 없고 `Codec`/`CodecType`/`VideoCodec` 등 코덱 enum이 계속 늘어나는 중(crates/bitvue-core/src/codec_error.rs:11, crates/bitvue-codecs-parser/src/parser_strategy.rs:19)이라 구조적으로는 해당하나, lockstep 버전의 내부 모노레포라 문서가 언급한 breaking-change 리스크가 실제로 발현되는지는 확인 못함

---

### API-014: 수십 개 `Option<T>` 필드로 이루어진 설정 구조체
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Structural/Manual

**나쁜 예**:
```rust
// bitvue-decode: 디코더 옵션이 전부 Option<T>로 평면 나열됨
#[derive(Default)]
pub struct DecodeOptions {
    pub hw_accel: Option<bool>,
    pub hw_device_index: Option<u32>,
    pub thread_count: Option<u32>,
    pub output_format: Option<PixelFormat>,
    pub max_reorder_frames: Option<u32>,
    pub crop_to_display_window: Option<bool>,
    pub error_concealment: Option<ErrorConcealmentMode>,
    pub low_latency: Option<bool>,
    // ... 총 24개 필드
}

pub fn decode(data: &[u8], opts: DecodeOptions) -> Result<Vec<Frame>, DecodeError> {
    let hw = opts.hw_accel.unwrap_or(false);
    let device = if hw { opts.hw_device_index.unwrap_or(0) } else {
        if opts.hw_device_index.is_some() {
            // hw_accel=false인데 hw_device_index가 설정된 모순 조합 — 런타임에만 발견
            log::warn!("hw_device_index ignored because hw_accel is false");
        }
        0
    };
    ...
}
```

**문제**:
- 필드 간 상호 의존/배타 관계(`hw_accel=false`인데 `hw_device_index`가 설정된 경우 등)가 타입으로 표현되지 않아 런타임에 경고 로그나 무시로만 처리됨
- 24개 필드 전부가 `Option`이라 어떤 조합이 유효한지 문서 없이는 알 수 없고, 테스트해야 할 조합의 수가 조합적으로 폭발
- `unwrap_or(default)`가 함수 본문 곳곳에 흩어져 "진짜 기본값이 무엇인가"가 구조체 정의가 아니라 사용처 코드에 숨어 있음
- 호출부에서 `DecodeOptions { hw_accel: Some(true), ..Default::default() }`처럼 대부분 필드를 비워야 하는 장황한 리터럴이 강요됨

**발생 조건**:
- 옵션이 시간이 지나며 하나씩 추가되어 온 경우("이 옵션 하나만 더 추가하면 되니까 Option 필드로")
- 상호 배타/의존 관계가 있는 옵션들을 그룹화하지 않고 평면 구조체에 계속 누적한 경우

**권장**:
```rust
// 관련 옵션을 의미 단위로 그룹화하고, 상호 배타 관계는 enum으로 표현
#[derive(Default)]
pub struct DecodeOptions {
    pub acceleration: AccelMode,       // enum이 hw_accel + hw_device_index를 대체
    pub threading: ThreadingOptions,   // 서브 구조체로 그룹화
    pub output: OutputOptions,
    pub error_handling: ErrorHandlingOptions,
}

pub enum AccelMode {
    Software,
    Hardware { device_index: u32 }, // 모순 조합 자체가 표현 불가능
}
impl Default for AccelMode { fn default() -> Self { Self::Software } }

pub struct ThreadingOptions { pub thread_count: Option<u32>, pub low_latency: bool }
pub struct OutputOptions { pub pixel_format: Option<PixelFormat>, pub crop_to_display_window: bool }
pub struct ErrorHandlingOptions { pub max_reorder_frames: u32, pub concealment: ErrorConcealmentMode }

// 호출부는 필요한 그룹만 채우고 나머지는 Default
let opts = DecodeOptions {
    acceleration: AccelMode::Hardware { device_index: 0 },
    ..Default::default()
};
```
- 정말 "설정하거나 안 하거나"인 필드만 `Option`으로 남기고, 상호 배타적인 선택지는 enum으로, 관련 옵션 묶음은 서브 구조체로 그룹화
- 필드 20개를 넘어가면 typestate 빌더나 프리셋 함수(`DecodeOptions::low_latency_preset()`)를 함께 제공해 흔한 조합에 이름을 부여

**탐지 방법**:
- Structural: 구조체 내 `Option<T>` 필드 비율이 70% 이상이고 필드 수가 10개를 넘는 구조체를 스캔
- Manual: 필드 쌍 사이에 "A가 Some일 때만 B가 의미 있다"는 문서/주석이 있는지, 있다면 그것이 타입으로 표현 가능한지 리뷰

**예외**:
- 필드 수가 적고(10개 미만) 서로 독립적이며 상호 의존 관계가 전혀 없는 경우는 평면 `Option` 구조체도 무방
- 외부 라이브러리/FFI 옵션 구조체를 그대로 미러링해야 하는 바인딩 레이어는 원본 구조를 유지하는 것이 오히려 유지보수에 유리할 수 있음

**Bitvue 판정**: N/A — Option 필드 비율이 높고 필드 10개 이상인 설정 struct를 찾지 못함; 디코더 가속 백엔드 선택은 Option 나열이 아니라 strategy 패턴(crates/bitvue-decode/src/strategy/)으로 구현됨

---

### API-015: 지나치게 관대한 모듈 가시성 (`pub` 남용)
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Static/Structural

**나쁜 예**:
```rust
// bitvue-hevc/src/cabac.rs
pub struct CabacContext {
    pub state: [u8; 1024],       // 내부 산술 상태, 외부에서 직접 조작하면 불변조건 깨짐
    pub range: u32,
    pub offset: u32,
}

pub fn init_context_variables(ctx: &mut CabacContext, qp: u8) { ... } // 내부 헬퍼인데 pub
pub fn renorm(ctx: &mut CabacContext) { ... }                         // 역시 내부 전용

// 크레이트 밖에서 실수로 CABAC 내부 상태를 직접 건드리는 코드가 작성 가능해짐
use bitvue_hevc::cabac::CabacContext;
let mut ctx = CabacContext { state: [0; 1024], range: 0, offset: 0 }; // 불변조건 무시하고 생성
ctx.range = 999999; // 스펙상 불가능한 값도 그냥 대입 가능
```

**문제**:
- CABAC 상태 머신처럼 강한 불변조건(스펙에 정의된 범위, 초기화 순서)을 가진 내부 구현이 `pub`으로 노출되면 크레이트 경계 밖에서 불변조건을 깨는 코드가 컴파일됨
- 크레이트 내부 리팩터링(필드 이름 변경, 알고리즘 교체)이 semver breaking change로 취급되어야 해서 자유도가 사라짐
- "일단 pub으로 해두면 나중에 편하겠지"라는 습관이 쌓이면 실제 공개 API 표면과 구현 세부사항의 경계가 rustdoc에서도 구분되지 않음

**발생 조건**:
- 크레이트 초기 개발 단계에서 가시성을 신경 쓰지 않고 전부 `pub`으로 작성한 뒤 방치한 경우
- 같은 크레이트의 다른 모듈에서 접근해야 해서 `pub`으로 열었는데, 실제로는 `pub(crate)`로 충분한 경우

**권장**:
```rust
// bitvue-hevc/src/cabac.rs
pub(crate) struct CabacContext {
    state: [u8; 1024], // 크레이트 내부에서도 필드는 비공개, 메서드로만 조작
    range: u32,
    offset: u32,
}

impl CabacContext {
    pub(crate) fn new(qp: u8) -> Self { /* init_context_variables 로직을 생성자로 캡슐화 */ ... }
    pub(crate) fn renorm(&mut self) { ... } // 불변조건을 지키는 유일한 통로
}

// 크레이트의 진짜 공개 API는 상위 모듈에서 명시적으로 좁게 노출
// bitvue-hevc/src/lib.rs
pub struct HevcParser { /* CabacContext는 필드로만 내부 보유, 외부에 노출 안 함 */ }
pub fn parse(data: &[u8]) -> Result<HevcBitstream, ParseError> { ... }
```
- 기본값을 `pub(crate)` 또는 비공개로 시작하고, 실제로 크레이트 경계 밖에서 필요해질 때만 의도적으로 `pub`으로 승격
- CI에 "새로 추가된 `pub` 항목"을 diff로 보여주는 `cargo public-api` 체크를 넣어 가시성 확장을 리뷰 대상으로 만듦
- `#![warn(unreachable_pub)]` 린트를 활성화해 실제로는 아무도 크레이트 밖에서 쓰지 않는 `pub` 항목을 자동 검출

**탐지 방법**:
- Static: `#![warn(unreachable_pub)]` 컴파일러 린트로 사실상 도달 불가능한 `pub`을 자동 검출
- Structural: `cargo public-api`로 크레이트별 공개 API 심볼 수를 추적하고, 코드량 대비 비정상적으로 높은 크레이트를 flag

**예외**:
- 워크스페이스 내부 크레이트 간 공유가 실제로 빈번하고 의도된 경우(`bitvue-core`의 공통 타입)는 `pub`이 정당
- 테스트 전용 헬퍼를 `#[cfg(test)] pub`으로 노출하는 것은 테스트 크레이트 경계 안에서는 무방

**Bitvue 판정**: Suspected — `#![warn(unreachable_pub)]` 린트가 워크스페이스 어디에도 설정되어 있지 않아 구조적으로는 가능하지만, 예시로 든 CABAC 상태(crates/bitvue-avc/src/overlay_extraction.rs:1158, crates/bitvue-hevc/src/overlay_extraction.rs:546)는 실제로는 `pub` 없이 올바르게 비공개 처리되어 있음

---

### API-016: Tauri command 경계의 타입 불안정 (raw JSON/문자열 페이로드)
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// src-tauri/src/commands/frame.rs
#[tauri::command]
pub fn get_frame_info(payload: serde_json::Value) -> Result<serde_json::Value, String> {
    let frame_index = payload["frameIndex"].as_u64().ok_or("missing frameIndex")?;
    let track_id = payload.get("trackId").and_then(|v| v.as_u64()).unwrap_or(0);
    // 프런트엔드와 필드 이름/타입을 문자열 키로만 암묵적으로 계약 — 오타는 런타임에만 발견
    let frame = fetch_frame(frame_index as u32, track_id as u32)?;
    Ok(serde_json::json!({ "frameIdx": frame.index, "isKey": frame.is_keyframe }))
    // 응답 필드 이름도 임의 문자열 — 프런트엔드 TS 타입과 별도로 손으로 동기화해야 함
}
```

**문제**:
- 요청/응답이 `serde_json::Value`이면 Rust 컴파일러도, TypeScript 컴파일러(`tauri-specta` 등을 안 쓰는 한)도 필드 이름/타입 불일치를 잡아주지 못해 프런트엔드-백엔드 계약이 순전히 "문서와 관례"에만 의존
- 필드 이름 오타(`frameIdx` vs `frameIndex`)가 리뷰에서 걸러지지 않으면 런타임에 `undefined`로만 드러나 디버깅 비용이 큼
- API-001의 newtype 이점(FrameIndex/TrackId 구분)이 이 경계에서 전부 무너짐 — `as_u64()`로 다시 primitive로 되돌아감

**발생 조건**:
- 프런트엔드 요구사항이 빠르게 바뀌는 초기 개발 단계에서 "일단 JSON Value로 유연하게" 만들고 나중에 타입을 굳히지 않은 경우
- Tauri command가 여러 개 생기면서 매번 구조체를 새로 정의하기 귀찮아 공통 `Value` 패턴을 재사용한 경우

**권장**:
```rust
// src-tauri/src/commands/frame.rs
#[derive(Debug, serde::Deserialize)]
pub struct GetFrameInfoRequest {
    pub frame_index: FrameIndex,
    #[serde(default)]
    pub track_id: TrackId,
}

#[derive(Debug, serde::Serialize)]
pub struct GetFrameInfoResponse {
    pub frame_index: FrameIndex,
    pub is_keyframe: bool,
}

#[tauri::command]
pub fn get_frame_info(payload: GetFrameInfoRequest) -> Result<GetFrameInfoResponse, FrameCommandError> {
    let frame = fetch_frame(payload.frame_index, payload.track_id)?;
    Ok(GetFrameInfoResponse { frame_index: frame.index, is_keyframe: frame.is_keyframe })
}
```
- 모든 Tauri command에 구체 요청/응답 타입을 정의해 serde가 필드 이름/타입 불일치를 역직렬화 실패로 표면화하게 함
- `specta`/`tauri-specta` 같은 도구로 Rust 타입에서 TypeScript 타입을 자동 생성해 프런트엔드-백엔드 계약을 단일 소스로 관리
- 에러도 `String` 대신 구조화된 에러 enum(API-010)을 반환해 프런트엔드가 에러 종류별로 다른 UI를 보여줄 수 있게 함

**탐지 방법**:
- Static: `#[tauri::command]` 함수 시그니처에서 파라미터/반환 타입이 `serde_json::Value` 또는 `String`인 것을 grep
- Structural: 생성된 TypeScript 바인딩(있다면)과 실제 프런트엔드 호출부 필드 이름이 일치하는지 CI에서 diff

**예외**:
- 정말 스키마가 동적인(플러그인이 임의 JSON을 주고받는) 극소수 command는 `Value`가 불가피할 수 있음 — 이 경우 별도로 런타임 스키마 검증(예: `jsonschema`)을 추가

**Bitvue 판정**: Confirmed (부분, 근거 갱신) — 인용된 `src-tauri`는 삭제됨; 후속 IPC 경계는 `crates/bitvue-protocol`(Electron main ↔ sidecar stdio)로, `Request.params`가 여전히 raw `serde_json::Value`(crates/bitvue-protocol/src/lib.rs:79)라 요청 측 나쁜 예는 그대로 존재. 다만 응답 에러는 문서가 가정한 flat `String`이 아니라 구조화된 `WireError{code: WireErrorCode, message, offset}`(lib.rs:113-146, 17개 variant)로 이미 개선돼 있어 API-010 문제가 응답 측에는 해당 안 됨 — 내부 함수들의 `Result<_, String>`(예: crates/bitvue-sidecar/src/frame_analysis.rs:55)이 main.rs에서 대부분 `WireErrorCode::InvalidData`로 뭉뚱그려 매핑되는 것(main.rs:246-260 등)이 남은 갭

---

### API-017: 크레이트마다 제각각인 에러 타입과 변환 전략 부재
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
// bitvue-formats: 자체 에러 타입, From 구현 없음
pub struct FormatError(pub String);

// bitvue-codecs-parser: 다른 스타일의 자체 에러 타입, thiserror 사용
#[derive(thiserror::Error, Debug)]
pub enum ParseError { #[error("bad NAL")] BadNal }

// bitvue-decode: anyhow를 라이브러리 반환 타입으로 그대로 사용
pub fn decode(data: &[u8]) -> anyhow::Result<Vec<Frame>> { ... }

// bitvue-cli: 세 크레이트를 조합하려니 변환 지옥
fn run() -> Result<(), String> {
    let tree = bitvue_formats::parse(&data).map_err(|e| e.0)?; // FormatError -> String
    let frames = bitvue_codecs_parser::parse(&tree)
        .map_err(|e| format!("{e}"))?; // ParseError -> String (Display로 우회)
    let decoded = bitvue_decode::decode(&frames)
        .map_err(|e| e.to_string())?; // anyhow::Error -> String
    Ok(())
}
```

**문제**:
- 각 크레이트가 에러 표현 방식(튜플 구조체, thiserror enum, anyhow)을 통일하지 않으면, 여러 크레이트를 조합하는 상위 코드가 매번 `String`으로 뭉개서 변환하는 "최소공배수" 패턴에 빠짐 — API-010의 문제가 크레이트 경계에서 반복됨
- `anyhow::Result`를 라이브러리(bitvue-decode)의 공개 반환 타입으로 쓰면, 그 크레이트를 의존하는 다른 라이브러리도 구조화된 에러 매칭을 할 수 없게 되어 오염이 전파됨
- `source()` 체인이 일관되지 않아 최종 사용자(CLI, Tauri UI)에게 "원인까지 포함한" 에러 메시지를 만들기 어려움

**발생 조건**:
- 크레이트를 각자 다른 시점/다른 작성자가 만들면서 에러 처리 컨벤션을 워크스페이스 레벨에서 정하지 않은 경우
- "anyhow가 편하니까"라는 이유로 라이브러리 크레이트에도 애플리케이션 전용 도구를 그대로 사용한 경우

**권장**:
```rust
// 워크스페이스 전체에 적용되는 컨벤션:
// 1. 라이브러리 크레이트(bitvue-*)는 항상 thiserror 기반 구조화된 에러 enum을 반환
// 2. 상위 크레이트로 전파할 하위 에러는 #[from]으로 감싼다 (변환 지옥 제거)
// 3. anyhow는 bitvue-cli::main, bitvue-mcp의 최상위 핸들러에서만 사용

// bitvue-formats
#[derive(thiserror::Error, Debug)]
pub enum FormatError {
    #[error("box parse failed at {offset:?}")]
    BoxParse { offset: FileOffset, #[source] source: BoxParseError },
}

// bitvue-codecs-parser
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("container error")]
    Format(#[from] bitvue_formats::FormatError), // 자동 변환, ? 로 전파 가능
    #[error("bad NAL unit")]
    BadNal,
}

// bitvue-decode
#[derive(thiserror::Error, Debug)]
pub enum DecodeError {
    #[error("parse stage failed")]
    Parse(#[from] bitvue_codecs_parser::ParseError),
}

// bitvue-cli: ? 만으로 조합, map_err 지옥 없음
fn run() -> Result<(), bitvue_decode::DecodeError> {
    let frames = bitvue_decode::decode(&data)?;
    Ok(())
}
// main()에서만 anyhow로 최종 변환해 사용자에게 출력
fn main() -> anyhow::Result<()> { run()?; Ok(()) }
```
- 워크스페이스 컨벤션 문서에 "라이브러리는 thiserror, 애플리케이션 진입점은 anyhow"를 명문화
- 크레이트 간 에러는 `#[from]`으로 체이닝해 하위 원인을 잃지 않고 상위로 전파

**탐지 방법**:
- Structural: 각 `bitvue-*` 라이브러리 크레이트의 `Cargo.toml`에서 `anyhow` 의존 여부와 공개 함수의 반환 타입을 대조
- Static: 공개 함수가 `anyhow::Result`를 반환하는지 grep, 라이브러리 크레이트(애플리케이션 크레이트 제외)에서 발견 시 flag

**예외**:
- `bitvue-cli`, `bitvue-mcp`처럼 최종 사용자에게 직접 에러를 보고하고 프로그램적 매칭이 필요 없는 애플리케이션 크레이트는 `anyhow` 사용이 적절

**Bitvue 판정**: Confirmed (근거 갱신) — 인용된 src-tauri/src/commands/frame.rs는 삭제됨; 같은 패턴이 후속 크레이트에 그대로 이전돼 있음을 재확인 — crates/bitvue-sidecar/src/*.rs 전역에 `Result<_, String>` 14곳 + `.map_err(...to_string())` 계열 25회(frame_analysis.rs 7회, debug_yuv.rs 5회 등), 각 크레이트가 자체 에러 타입(AvcError/HevcError 등 thiserror)을 갖고도 sidecar 레이어에서 전부 String으로 뭉개진 뒤 main.rs가 대부분 `WireErrorCode::InvalidData`로 재매핑 — `#[from]` 체이닝 없이 두 번 정보 손실. bitvue-codecs-parser는 여전히 Cargo.toml:17에 `anyhow = { workspace = true }` 의존을 선언 중(라이브러리 크레이트에 anyhow 오염, 원 판정과 동일하게 유효)

---

### API-018: 파서 출력 타입의 내부 표현 누수
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Manual/Structural

**나쁜 예**:
```rust
// bitvue-avc: 파서 결과가 내부 비트스트림 표현을 그대로 노출
pub struct SliceHeader {
    pub raw_bytes: Vec<u8>,          // RBSP를 그대로 노출 — 필드 접근이 곧 재파싱을 요구
    pub emulation_prevention_positions: Vec<usize>, // 완전히 내부 구현 디테일
}

impl SliceHeader {
    // 사용자가 실제 slice_type을 얻으려면 raw_bytes를 직접 비트파싱해야 함
    // (파서가 이미 파싱했는데 결과를 다시 버림)
}
```

**문제**:
- 파서가 이미 의미 있는 필드(slice_type, first_mb_in_slice 등)로 파싱했음에도 결과 타입이 원시 바이트를 그대로 노출하면, 소비자가 파서의 일을 중복으로 다시 해야 함
- `emulation_prevention_positions`처럼 AVC/HEVC의 RBSP 이스케이핑 같은 내부 구현 디테일이 공개 타입 필드가 되면, 파서 내부 구현(이스케이프 처리 방식)을 바꿀 때마다 breaking change가 됨
- 소비자(hex view, 오버레이 렌더러)가 원시 바이트에 의존하게 되면 코덱마다 다른 파싱 로직을 소비자 쪽에 재구현하게 되어 계층 분리가 무의미해짐

**발생 조건**:
- 파서를 빠르게 동작시키는 데 집중하면서 "일단 원시 데이터도 같이 넘겨주면 나중에 뭐든 할 수 있겠지"라는 생각으로 raw 필드를 끼워 넣은 경우
- 디버깅 편의를 위해 임시로 추가한 raw 필드가 정식 API로 굳어진 경우

**권장**:
```rust
// 의미 있는 필드로 파싱 결과를 노출하고, 원시 바이트가 필요한 경우(hex view)는
// 별도의 명시적 API로 분리한다
pub struct SliceHeader {
    pub slice_type: SliceType,
    pub first_mb_in_slice: u32,
    pub pic_parameter_set_id: u8,
    // 내부 구현 디테일(emulation prevention 등)은 필드에서 제거
}

// hex view처럼 원시 바이트/오프셋이 정말 필요한 소비자를 위한 별도 API
// (API-001의 FileOffset/BitOffset 활용, bitvue-hex-view 등 전용 경로)
pub struct SliceHeaderUnit {
    pub header: SliceHeader,
    pub byte_range: Range<FileOffset>, // "원본 바이트를 보고 싶으면 이 범위를 다시 읽어라"
}
```
- 파서 출력은 "의미 있는 도메인 모델"을 우선하고, 원시 바이트 접근이 필요한 소비자(hex view, 오버레이)는 offset/size 참조로 원본을 재조회하게 함(이미 커밋된 `get_frame_hex_data`의 UnitNode offset/size 코드아그노스틱 접근 방식과 같은 원칙)
- 내부 구현 디테일(이스케이프 위치 등)은 필드로 노출하지 말고 크레이트 내부에서만 사용

**탐지 방법**:
- Manual: 파서 출력 타입의 필드 목록을 훑어 "이 필드가 스펙 문서의 syntax element에 대응하는가, 아니면 구현 디테일인가"를 분류
- Structural: `Vec<u8>` 타입 필드가 파서 출력 구조체에 있는 경우를 스캔해 정말 필요한지 검토

**예외**:
- Hex view, raw dump 같은 명시적으로 "원시 바이트를 보여주는" 기능의 API는 raw bytes 노출이 목적 자체이므로 해당하지 않음
- 파싱되지 않은 vendor-specific/reserved 필드처럼 의미를 알 수 없는 데이터는 raw bytes로 남기는 것이 유일한 선택지일 수 있음

**Bitvue 판정**: N/A — `SliceHeader`(crates/bitvue-avc/src/slice.rs:88)는 raw bytes가 아닌 완전히 파싱된 의미 필드로 구성되어 문서의 '권장' 상태와 일치; `NalUnit.raw_payload`(crates/bitvue-avc/src/nal.rs:172)는 하위 파서/hex view가 실제로 소비하는 정당한 중간 표현이지 파싱 결과를 대신 버리는 사례가 아님

---

### API-019: 인덱스 newtype에 인체공학적 연산자/변환 부재
**분류**: API·타입 설계 · **심각도**: Low · **탐지**: Manual/Structural

**나쁜 예**:
```rust
// API-001의 권장을 따라 newtype을 도입했지만 연산자를 하나도 안 붙인 경우
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameIndex(pub u32);

// 호출부: 매번 .0으로 언랩해야 해서 newtype 도입 전보다 코드가 더 장황해짐
let next = FrameIndex(current.0 + 1);
if target.0 > FrameIndex(total_frames).0 { ... }
for i in 0..total_frames { process(FrameIndex(i)); } // 매번 재래핑
// 결국 다들 귀찮아서 나중엔 usize로 되돌아가는 크레이트가 하나둘 생김
```

**문제**:
- newtype이 산술/비교 연산자, `From`/`Into` 변환을 전혀 제공하지 않으면 `.0`으로 매번 언랩해야 해서 원래 목표(오용 방지, 가독성)와 반대로 코드가 더 장황해짐
- 사용이 불편하면 팀원들이 점점 newtype을 우회해 primitive로 되돌아가려는 유혹이 생기고, 결국 일부 크레이트만 newtype을 쓰고 일부는 usize를 쓰는 불일치가 발생(API-001이 반쯤만 적용된 상태)
- `Iterator` 구현이 없으면 `for i in FrameIndex(0)..FrameIndex(total)` 같은 자연스러운 순회도 불가능

**발생 조건**:
- newtype 패턴을 "일단 감싸기만" 적용하고 인체공학을 신경 쓰지 않은 경우
- `derive_more`, `nutype` 같은 보일러플레이트 감소 크레이트를 도입하지 않고 손으로 전부 구현하려다 지쳐 최소한만 구현한 경우

**권장**:
```rust
use derive_more::{Add, Sub, Display, From, Into};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         Add, Sub, Display, From, Into)]
pub struct FrameIndex(pub u32);

impl FrameIndex {
    pub const ZERO: Self = Self(0);
    pub fn next(self) -> Self { Self(self.0 + 1) }
}

// 필요하면 범위 순회를 위한 소량의 헬퍼도 제공
pub fn frame_range(total: u32) -> impl Iterator<Item = FrameIndex> {
    (0..total).map(FrameIndex)
}

// 호출부가 다시 자연스러워짐
let next = current.next();
for i in frame_range(total_frames) { process(i); }
```
- `derive_more`나 `nutype` 같은 크레이트로 산술/변환 보일러플레이트를 자동 생성해 수작업 비용을 낮춘다
- 최소한 `Display`, `From<u32>`/`Into<u32>`, 필요시 `Add<u32>`/`Sub<u32>` 정도는 갖춰야 newtype이 "쓰기 편한 안전한 타입"으로 자리잡음

**탐지 방법**:
- Manual: newtype 정의 근처에 `.0` 언랩이 호출부 여러 곳에 반복되는지 grep으로 확인 — 많다면 인체공학이 부족하다는 신호
- Structural: 같은 개념(프레임 인덱스 등)에 대해 일부 크레이트는 newtype, 일부는 primitive를 쓰는 불일치를 크레이트 간 시그니처 비교로 탐지

**예외**:
- FFI 경계처럼 newtype이 `#[repr(transparent)]`로만 존재하고 실제 산술은 항상 언랩 후 primitive로 수행하는 것이 명확한 지점에서는 최소 구현으로 충분

**Bitvue 판정**: N/A — 전제 조건 불충족: API-001에서 확인했듯 offset/index newtype 자체가 아직 도입되지 않아 '인체공학 부족한 newtype' 문제가 성립할 대상이 없음

---

### API-020: 무분별한 `From`/`Into` 블랭킷 변환으로 인한 모호성
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
// bitvue-codecs-parser: 여러 코덱의 프레임 타입에서 공통 Frame으로 변환하는 From을 남발
impl From<bitvue_avc::AvcFrame> for Frame { fn from(f: bitvue_avc::AvcFrame) -> Self { ... } }
impl From<bitvue_hevc::HevcFrame> for Frame { fn from(f: bitvue_hevc::HevcFrame) -> Self { ... } }

// 그런데 AvcFrame -> HevcFrame 같은 코덱 간 변환까지 "편의상" 추가
impl From<bitvue_avc::AvcFrame> for bitvue_hevc::HevcFrame {
    fn from(f: bitvue_avc::AvcFrame) -> Self {
        // 실제로는 SliceType 등 일부 필드만 억지로 매핑, 손실 변환인데 From이 이를 숨김
        HevcFrame { slice_type: f.slice_type.into(), ..Default::default() }
    }
}

// 호출부: .into()가 여러 타입으로 해석 가능해 타입 추론이 모호해지는 상황 발생
let x: SomeGenericSink<_> = avc_frame.into(); // 어떤 into()가 선택될지 문맥에 크게 의존
```

**문제**:
- `From`은 "무손실이고 항상 성공하는 변환"이라는 관례적 기대를 가지는데, `AvcFrame -> HevcFrame`처럼 필드 대부분을 기본값으로 채우는 손실 변환에 `From`을 쓰면 이 관례를 깨고 호출부를 오도함
- 같은 소스 타입에서 여러 목적지 타입으로 가는 `From`이 늘어나면 제네릭 문맥(`Into<T>` 바운드)에서 타입 추론이 모호해지거나 뜻밖의 변환이 암묵적으로 선택될 위험이 커짐
- `.into()` 체인이 크레이트 경계를 넘어 여러 단계로 이어지면(예: A -> B -> C) 어디서 정보가 손실되는지 호출부 코드만 봐서는 추적하기 어려움

**발생 조건**:
- "이 타입에서 저 타입으로 자주 변환하니까 편의상 `From`을 구현해두자"는 이유로 변환의 손실 여부를 따지지 않고 추가한 경우
- 코덱 간에는 원래 의미상 변환이 존재하지 않는데(AVC 프레임을 HEVC 프레임으로 바꾼다는 것 자체가 도메인상 이상함) 공통 trait 편의를 위해 무리하게 구현한 경우

**권장**:
```rust
// 무손실 변환만 From/Into로 표현 (같은 도메인 계층 내에서 정보 손실 없이)
impl From<bitvue_avc::AvcFrame> for Frame { ... } // AvcFrame -> 공통 Frame: 필드가 다 매핑됨(무손실)

// 손실 변환이거나 실패 가능한 변환은 명시적 이름의 메서드로
impl bitvue_avc::AvcFrame {
    // 이름 자체가 "일부 필드만 근사 매핑함"을 드러냄
    pub fn approximate_as_hevc_frame(&self) -> bitvue_hevc::HevcFrame { ... }
}

// 실패 가능한 변환은 TryFrom
impl TryFrom<Frame> for bitvue_avc::AvcFrame {
    type Error = NotAnAvcFrameError;
    fn try_from(f: Frame) -> Result<Self, Self::Error> { ... }
}
```
- `From`은 무손실·항상 성공하는 변환에만 사용하고, 정보가 줄어들거나 도메인상 억지스러운 변환은 이름이 의도를 드러내는 일반 메서드로 표현
- 실패 가능하지만 무손실인 변환은 `TryFrom`으로 표현해 `?` 전파가 자연스럽게 되도록 함

**탐지 방법**:
- Static: `impl From<X> for Y`가 코덱 크레이트 간(예: `bitvue_avc` -> `bitvue_hevc`)에 직접 존재하는 경우를 grep으로 스캔 — 보통 공통 중간 타입을 거쳐야 정상
- Manual: `From` 구현 본문에서 `..Default::default()`로 채워지는 필드 비율이 높으면 손실 변환일 가능성이 높다고 판단

**예외**:
- 뉴타입 wrap/unwrap처럼 명백히 무손실이고 대칭적인 변환은 `From`/`Into`가 정확히 맞는 도구
- 같은 정보를 다른 표현으로만 바꾸는 변환(예: `BitOffset -> ByteOffset` 근사가 아니라 정확한 단위 변환)은 무손실이면 허용

**Bitvue 판정**: N/A — 코덱 간(AvcFrame→HevcFrame 등) `From`/`Into` 구현을 찾지 못함

---

### API-021: 와이어 포맷과 런타임 상태를 겸하는 이중 역할 enum
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Manual/Structural

**나쁜 예**:
```rust
// bitvue-mcp: MCP 프로토콜 직렬화와 내부 세션 상태를 같은 enum으로 겸함
#[derive(serde::Serialize, serde::Deserialize)]
pub enum AnalysisState {
    Idle,
    Parsing { progress: f32 },
    Parsed { frame_count: u32, handle: Arc<ParsedBitstream> }, // 직렬화 불가능한 런타임 핸들이 섞임
    Failed { error: String },
}

// serde(skip)로 우회하지만 그러면 이 variant는 역직렬화 시 항상 깨진 상태가 됨
```

**문제**:
- 와이어 포맷(외부와 주고받는 직렬화 가능한 표현)과 런타임 내부 상태(`Arc<ParsedBitstream>` 같은 프로세스 로컬 핸들)가 같은 타입에 섞이면, 직렬화 불가능한 필드에 `#[serde(skip)]`를 붙여야 하고 역직렬화된 값은 그 필드가 항상 기본값/부재 상태가 되는 함정이 생김
- 프로토콜이 진화(새 필드 추가)할 때 런타임 상태 쪽 요구사항과 와이어 포맷 쪽 요구사항이 충돌해 둘 다 타협된 설계가 되기 쉬움
- 같은 enum이 두 가지 문맥(네트워크로 나가는 값 / 프로세스 내부에서 도는 값)에서 다른 불변조건을 가지므로, 코드를 읽는 사람이 "지금 이 값이 직렬화 경로에 있는지 런타임 경로에 있는지"를 매번 문맥으로 판단해야 함

**발생 조건**:
- MCP 서버, Tauri IPC처럼 "상태 머신을 그대로 외부에 보고하면 편하다"는 이유로 내부 상태 enum에 바로 `Serialize`를 붙인 경우
- 두 표현이 처음엔 우연히 거의 같아서 "굳이 두 타입으로 나눌 필요 있나"라고 판단했다가, 이후 각자 다르게 진화하며 문제가 누적된 경우

**권장**:
```rust
// 런타임 내부 상태 (직렬화 불가능한 핸들 포함, 프로세스 로컬 전용)
pub enum AnalysisState {
    Idle,
    Parsing { progress: f32 },
    Parsed { frame_count: u32, handle: Arc<ParsedBitstream> },
    Failed { error: AnalysisError },
}

// 와이어 포맷 (MCP/Tauri로 나가는 표현, 항상 직렬화 가능)
#[derive(serde::Serialize, serde::Deserialize)]
pub enum AnalysisStateDto {
    Idle,
    Parsing { progress: f32 },
    Parsed { frame_count: u32 }, // 핸들 대신 opaque id나 요약 정보만
    Failed { error: String },
}

impl From<&AnalysisState> for AnalysisStateDto {
    fn from(s: &AnalysisState) -> Self {
        match s {
            AnalysisState::Idle => Self::Idle,
            AnalysisState::Parsing { progress } => Self::Parsing { progress: *progress },
            AnalysisState::Parsed { frame_count, .. } => Self::Parsed { frame_count: *frame_count },
            AnalysisState::Failed { error } => Self::Failed { error: error.to_string() },
        }
    }
}
```
- "이 타입이 프로세스 경계를 넘는가"를 기준으로 런타임 상태 타입과 와이어 DTO 타입을 처음부터 분리
- 두 타입 사이 변환은 명시적 `From`/`TryFrom`으로 좁은 지점에 모아 관리

**탐지 방법**:
- Structural: `#[derive(Serialize, Deserialize)]`가 붙은 enum/struct 안에 `Arc<`, `Rc<`, `Box<dyn`, 핸들류 필드가 `#[serde(skip)]`와 함께 있는 패턴을 스캔
- Manual: MCP/Tauri IPC 경계에서 사용되는 타입이 내부 상태 머신과 동일한 정의를 공유하는지 확인

**예외**:
- 정말 단순하고(필드가 전부 원시 타입/String) 향후에도 두 문맥이 갈라질 가능성이 낮은 작은 enum이라면 분리 비용이 이득보다 클 수 있음

**Bitvue 판정**: Confirmed — `Obu`(crates/bitvue-av1-codec/src/obu.rs:128-150)가 `Serialize`와 `Deserialize`를 함께 derive하면서 `payload: Arc<[u8]>`와 `frame_header`에 `#[serde(skip)]`를 붙여, 역직렬화 시 payload/frame_header가 조용히 빈 상태가 되는 함정이 실존함

---

### API-022: 대용량 비트스트림 데이터에 `Vec<T>` 반환 강제
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Runtime/Static

**나쁜 예**:
```rust
// bitvue-codecs-parser: 전체 프레임 목록을 항상 Vec로 강제 수집
pub fn parse_all_frames(data: &[u8]) -> Result<Vec<ParsedFrame>, ParseError> {
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < data.len() {
        let (frame, consumed) = parse_one_frame(&data[cursor..])?;
        frames.push(frame); // 4K/8K 장시간 녹화본이면 수십만 프레임이 전부 메모리에 상주
        cursor += consumed;
    }
    Ok(frames)
}

// 호출부는 프레임 하나씩만 필요한데도 항상 전체를 기다려야 함
let frames = parse_all_frames(&huge_file_data)?; // 몇 분짜리 8K 영상이면 메모리 수 GB
for frame in frames.iter().take(10) { render_thumbnail(frame); } // 10개만 쓰는데 전부 파싱/보관
```

**문제**:
- 긴 녹화본(특히 8K, 장시간)에서 프레임 전체를 `Vec`에 모으면 실제 필요한 것이 앞부분 몇 프레임뿐이어도 전체를 파싱하고 메모리에 유지해야 해서 메모리 사용량과 지연시간이 파일 크기에 비례해 폭증
- UI가 "첫 프레임이 준비되는 즉시 보여주기" 같은 스트리밍 UX를 구현하려 해도 API가 `Vec` 전체 반환을 강제하면 불가능
- 에러가 파일 중간(예: 프레임 50000번째)에서 발생하면, 그 앞의 49999개를 이미 다 파싱해놓고도 `Result::Err`로 전부 버려야 함(부분 성공을 표현할 수 없음)

**발생 조건**:
- 프로토타입 단계에서 "일단 전체 파싱해서 리스트로 주면 다루기 쉽다"는 이유로 즉시 수집(eager collect) API를 만든 경우
- 소규모 테스트 파일로만 검증하다 보니 대용량 입력에서의 메모리 문제를 뒤늦게 발견하는 경우

**권장**:
```rust
// 이터레이터/스트리밍 API로 지연 평가를 제공
pub fn iter_frames<'a>(data: &'a [u8]) -> impl Iterator<Item = Result<ParsedFrame, ParseError>> + 'a {
    let mut cursor = 0;
    std::iter::from_fn(move || {
        if cursor >= data.len() { return None; }
        match parse_one_frame(&data[cursor..]) {
            Ok((frame, consumed)) => { cursor += consumed; Some(Ok(frame)) }
            Err(e) => { cursor = data.len(); Some(Err(e)) } // 마지막 에러 하나만 내보내고 종료
        }
    })
}

// 호출부는 필요한 만큼만 소비 — 나머지는 파싱조차 안 됨
for frame in iter_frames(&huge_file_data).take(10) {
    render_thumbnail(&frame?);
}

// 전체 목록이 정말 필요한 소수의 호출부만 명시적으로 collect
let all: Vec<_> = iter_frames(&data).collect::<Result<_, _>>()?; // 의도가 드러남
```
- 크기가 입력에 비례해 커질 수 있는 반환값은 기본적으로 이터레이터/스트림으로 설계하고, `Vec` 수집은 호출부가 명시적으로 선택하게 함
- Tauri command처럼 IPC로 나가는 경계에서는 페이지네이션(오프셋+개수)이나 커서 기반 API로 청크 단위 전달을 제공

**탐지 방법**:
- Runtime: 대용량(수 GB) 테스트 파일로 메모리 프로파일링해 파싱 API 호출 시 피크 메모리가 파일 크기에 선형 비례하는지 측정
- Static: 반환 타입이 `Vec<T>`이고 T가 "프레임", "샘플", "패킷" 등 파일 크기에 비례해 개수가 늘어나는 도메인 타입인 공개 함수를 스캔

**예외**:
- 반환 항목 수가 파일 크기와 무관하게 작다고 보장되는 경우(예: 트랙 목록, 파라미터 셋 목록)는 `Vec` 반환이 적절하고 오히려 이터레이터가 과설계
- 이미 메모리에 전체가 있어야만 의미 있는 연산(예: 전역 정렬, 통계 집계)의 결과 자체를 반환하는 API는 `Vec`/`HashMap`이 자연스러움

**Bitvue 판정**: Confirmed — '메모리 효율적'이라고 문서화된 `decode_from_file`(crates/bitvue-decode/src/decoder.rs:556, 스트리밍 I/O 설명은 524-556행 주석)조차 최종적으로 `Result<Vec<DecodedFrame>>`을 반환해 전체 프레임을 메모리에 모음; 코드베이스 전체에 이터레이터 기반 프레임 API가 전혀 없음

---

### API-023: 너무 많은 책임을 가진 God Trait
**분류**: API·타입 설계 · **심각도**: High · **탐지**: Structural/Manual

**나쁜 예**:
```rust
// bitvue-codecs-parser: 코덱 파서 trait 하나에 모든 책임을 몰아넣음
pub trait CodecParser {
    fn parse_frame(&mut self, data: &[u8]) -> Result<Frame, ParseError>;
    fn validate_conformance(&self, frame: &Frame) -> Result<(), ConformanceError>;
    fn render_overlay(&self, frame: &Frame, canvas: &mut Canvas) -> Result<(), RenderError>;
    fn serialize_to_json(&self, frame: &Frame) -> Result<String, SerError>;
    fn compute_metrics(&self, frame: &Frame, reference: &Frame) -> Metrics;
    fn export_hex_dump(&self, frame: &Frame) -> Vec<u8>;
    // 코덱마다 이 8~10개 메서드를 전부 구현해야 함 — 대부분은 공통 로직인데 trait 안에 갇힘
}
```

**문제**:
- 파싱, 검증, 렌더링, 직렬화, 메트릭 계산, 덤프처럼 서로 다른 관심사가 한 trait에 몰리면 새 코덱을 추가하는 사람이 실제로 코덱 고유 로직(파싱)만 필요해도 나머지 관계없는 메서드까지 전부 구현해야 함
- `render_overlay`, `export_hex_dump`처럼 대부분 코덱에 걸쳐 공통 구현이 가능한 로직도 trait 메서드로 묶이면 코덱마다 재구현되거나, 결국 기본 구현(default impl)에 의존하면서 trait의 존재 의미가 희석됨
- 이 trait을 사용하는 쪽도 "파싱만 필요한데" 전체 trait 바운드를 요구받아, 실제로 필요 없는 기능까지 링크되거나 목업해야 함(테스트 더블 작성 비용 증가)

**발생 조건**:
- "코덱이라는 개념을 하나의 trait으로 표현하자"는 목표로 관련된 모든 동작을 점진적으로 같은 trait에 추가해온 경우
- 초기에는 메서드가 2~3개였는데 기능이 늘어나며 trait이 자연스럽게 비대해지고, 리팩터링 시점을 놓친 경우

**권장**:
```rust
// 관심사별로 trait을 쪼갠다 (Interface Segregation)
pub trait CodecParser {
    fn parse_frame(&mut self, data: &[u8]) -> Result<Frame, ParseError>;
}

pub trait ConformanceValidator {
    fn validate_conformance(&self, frame: &Frame) -> Result<(), ConformanceError>;
}

// 코덱 고유 지식이 필요 없는 것들은 trait이 아니라 Frame에 대한 공통 free function으로
pub fn render_overlay(frame: &Frame, canvas: &mut Canvas) -> Result<(), RenderError> { ... }
pub fn export_hex_dump(frame: &Frame) -> Vec<u8> { ... }
pub fn compute_metrics(frame: &Frame, reference: &Frame) -> Metrics { ... }

// 코덱 구현체는 정말 코덱 고유 로직인 CodecParser만 구현
impl CodecParser for bitvue_hevc::HevcParser {
    fn parse_frame(&mut self, data: &[u8]) -> Result<Frame, ParseError> { ... }
}
// 필요하면 ConformanceValidator도 선택적으로 구현
impl ConformanceValidator for bitvue_hevc::HevcParser { ... }
```
- trait 메서드를 추가하기 전에 "이게 정말 구현체마다 달라지는 동작인가, 아니면 공통 데이터(`Frame`)에 대한 연산인가"를 질문 — 후자라면 trait이 아니라 free function
- 관심사별로 trait을 쪼개 필요한 기능만 바운드로 요구할 수 있게 함(Interface Segregation Principle)

**탐지 방법**:
- Structural: trait의 메서드 개수와, 구현체들이 실제로 non-default 오버라이드하는 메서드 비율을 측정 — 대부분 default impl에 의존한다면 분리 후보
- Manual: 새 구현체(신규 코덱)를 추가할 때 "이 중 몇 개 메서드가 코덱 고유 로직인가"를 리뷰에서 질문

**예외**:
- 메서드들이 실제로 강하게 응집되어 있고(같은 내부 상태를 공유해야만 구현 가능) 분리 시 오히려 상태를 중복 보관해야 하는 경우는 하나의 trait이 정당할 수 있음

**Bitvue 판정**: N/A — 발견된 trait(ParserStrategy, Decoder 등)은 각자의 도메인에 응집되어 있고 파싱과 렌더링/직렬화/메트릭/hex-dump를 한 trait에 섞은 사례를 찾지 못함

---

### API-024: 제로카피 파서의 라이프타임이 전파되는 공개 API
**분류**: API·타입 설계 · **심각도**: Medium · **탐지**: Structural/Manual

**나쁜 예**:
```rust
// bitvue-formats: 제로카피를 위해 원본 버퍼를 빌려온 채로 반환
pub struct BoxTree<'a> {
    pub boxes: Vec<BoxRef<'a>>,
    data: &'a [u8],
}

pub struct BoxRef<'a> {
    pub header: BoxHeader,
    pub payload: &'a [u8],
}

pub fn parse_box_tree<'a>(data: &'a [u8]) -> Result<BoxTree<'a>, ParseError> { ... }

// 다운스트림: bitvue-codecs-parser, bitvue-decode, bitvue-mcp 전부가
// 이 라이프타임을 계속 짊어져야 함
pub struct AnalysisSession<'a> { // 원치 않는데 라이프타임 전파
    tree: BoxTree<'a>,
    ...
}
// Tauri command나 비동기 태스크 경계를 넘기려 하면 라이프타임이 막혀
// 결국 어딘가에서 강제로 clone하거나 'static으로 우회하게 됨
```

**문제**:
- 제로카피 자체는 성능상 정당한 선택이지만, 그 라이프타임을 파서 크레이트 경계 밖(비동기 Tauri command, 여러 프레임에 걸쳐 캐시되는 세션 상태)까지 그대로 노출하면 다운스트림 전체가 라이프타임 파라미터를 떠안게 됨
- 비동기 함수, `Arc`로 여러 스레드에 공유되어야 하는 상태, Tauri command의 `'static` 요구사항과 라이프타임이 충돌하면 결국 어딘가에서 `.to_vec()`으로 강제 복사하게 되어, 애초에 제로카피를 도입한 이유가 무색해짐
- 라이프타임 파라미터가 3~4단계 크레이트를 관통하면 각 단계의 타입 시그니처가 전부 `<'a>`를 짊어져야 해서 API-012와 비슷하게 타입을 이름 짓기 어려워짐

**발생 조건**:
- 파서의 hot path(반복 호출, 짧은 수명)에서는 제로카피가 명확히 이득인데, 그 출력 타입을 세션처럼 오래 살아야 하는 상위 구조체 안에 그대로 저장하려 한 경우
- "복사 비용이 아까우니 최대한 빌림으로"라는 원칙을 전체 파이프라인에 기계적으로 적용한 경우

**권장**:
```rust
// 짧은 수명의 hot path 파싱 단계에서는 제로카피 유지
pub fn parse_box_tree<'a>(data: &'a [u8]) -> Result<BoxTree<'a>, ParseError> { ... }

// 이 호출 프레임을 벗어나 세션에 보관되어야 하는 결과는 경계에서 소유권 있는 형태로 전환
pub struct OwnedBoxTree {
    pub boxes: Vec<OwnedBoxRef>, // payload: Vec<u8> (또는 Bytes로 참조 카운트 공유)
}

impl From<BoxTree<'_>> for OwnedBoxTree {
    fn from(tree: BoxTree<'_>) -> Self { /* 이 경계에서 한 번만 복사/전환 */ ... }
}

pub struct AnalysisSession {
    tree: OwnedBoxTree, // 라이프타임 없음 — Tauri command, 비동기, Arc 공유 전부 자유
}
```
- "이 타입이 함수 호출 하나의 스코프 안에서만 쓰이는가, 아니면 더 오래(세션, 캐시) 살아야 하는가"를 기준으로 빌림(zero-copy)과 소유(owned) 경계를 명시적으로 설계
- 복사 비용이 걱정되면 `Vec<u8>` 대신 `Bytes`(참조 카운트 기반 공유 버퍼)를 owned 타입에 사용해 완전한 복사 없이도 라이프타임 파라미터를 제거할 수 있음
- 라이프타임은 "파싱 함수 → 즉시 소비하는 호출부" 같은 좁은 범위에만 허용하고, 크레이트 경계(특히 async/Tauri 경계)를 넘는 공개 타입에는 라이프타임 파라미터를 두지 않는 것을 원칙으로 함

**탐지 방법**:
- Structural: 공개 구조체에 라이프타임 파라미터가 있고, 그 구조체가 `Arc<Mutex<_>>`나 `async fn` 반환 타입, 세션/캐시류 상위 구조체 필드로 쓰이는 경우를 스캔
- Manual: 라이프타임 파라미터가 2단계 이상 크레이트를 관통하는 타입 체인을 추적해 "정말 전 구간에서 제로카피 이득이 있는가"를 검토

**예외**:
- 파싱 함수 호출과 그 결과 소비가 같은 동기 스코프 안에서 끝나는 경우(예: CLI에서 파일을 읽어 즉시 순회하고 끝내는 일회성 처리) 라이프타임 전파는 문제가 되지 않고 오히려 최선의 선택
- 성능이 검증된 hot path(예: 프레임 단위 반복 파싱)에서 호출자가 매 반복마다 명시적으로 짧은 수명 안에서만 결과를 쓰는 것이 문서화되어 있다면 허용

**Bitvue 판정**: N/A — 라이프타임을 가진 타입(BitReader<'a>, avs3 NalUnit<'a>, FrameEvidence<'a>)은 모두 짧은 함수/로컬 스코프에 한정되어 있고, AppState/Core 같은 세션 수준 구조체 필드로 전파되는 사례를 찾지 못함
