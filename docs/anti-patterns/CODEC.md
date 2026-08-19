# Anti-Pattern Catalog — CODEC: 컨테이너와 코덱 경계

이 문서는 더 큰 안티패턴 카탈로그의 일부이며(전체 색인은 `docs/anti-patterns/INDEX.md`, 별도 작성 예정), Bitvue와 같이 `bitvue-formats`(MP4/MKV/IVF/TS) → `bitvue-codecs` → 코덱별 크레이트(AV1/HEVC/AVC/VP9/VVC/AV3/MPEG-2) → `bitvue-decode` → `bitvue-core`로 이어지는 워크스페이스를 가정한, 컨테이너/코덱 경계에서 반복적으로 관찰되는 설계 실수를 다룬다.

---

### CODEC-001: container parser가 codec syntax를 직접 이해
**분류**: CODEC · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-formats/src/mp4/sample_table.rs
fn build_sample_index(stsz: &Stsz, stco: &Stco, avcc: &[u8]) -> Vec<SampleInfo> {
    let mut infos = Vec::new();
    for (offset, size) in stco.chunk_offsets.iter().zip(&stsz.sample_sizes) {
        // MP4 파서가 AVC NAL 구조를 직접 스캔해서 키프레임을 판정한다
        let nal_type = (avcc[0] & 0x1F) as u8;
        let is_keyframe = nal_type == 5; // IDR
        infos.push(SampleInfo { offset: *offset, size: *size, is_keyframe });
    }
    infos
}
```

**문제**:
- `bitvue-formats`가 AVC NAL 헤더 비트 레이아웃을 알아야만 컴파일된다 — 코덱이 바뀌면 컨테이너 코드가 깨진다.
- HEVC/VVC/AV1을 추가할 때마다 동일한 `if nal_type == ...` 분기가 컨테이너 레이어에 계속 늘어난다.
- 컨테이너 크레이트의 단위 테스트가 코덱별 비트스트림 픽스처에 의존하게 되어 테스트 매트릭스가 폭발한다.

**발생 조건**:
- "일단 키프레임 판정만 빨리 넣자"는 식으로 컨테이너 레이어에서 지름길을 탈 때.
- 새 코덱 지원을 추가하는 사람이 기존 컨테이너 코드의 관례를 그대로 답습할 때.

**권장**:
```rust
// bitvue-formats/src/mp4/sample_table.rs
fn build_sample_index(stsz: &Stsz, stco: &Stco) -> Vec<RawSample> {
    stco.chunk_offsets.iter().zip(&stsz.sample_sizes)
        .map(|(offset, size)| RawSample { offset: *offset, size: *size })
        .collect()
}

// bitvue-codecs/src/avc/keyframe.rs — 코덱 크레이트가 판정 로직을 소유
pub fn is_keyframe(sample: &[u8]) -> bool {
    sample.first().map_or(false, |b| (b & 0x1F) == 5)
}
```
- 컨테이너는 "바이트 범위 + 메타데이터"만 알고, "이게 키프레임인가"는 코덱 크레이트에 위임한다.
- `RawSample`(offset/size/duration/flags)과 `CodecUnit`(파싱된 신택스)을 별도 타입으로 분리한다.

**탐지 방법**:
- Structural: `bitvue-formats` 크레이트 의존성 그래프에서 `bitvue-avc`, `bitvue-hevc` 등 코덱 크레이트로의 직접 의존이 있는지 `cargo tree -p bitvue-formats` 로 확인.
- Static: 컨테이너 모듈 내 `nal_type`, `obu_type`, `sc_start_code` 같은 코덱 고유 상수 리터럴 grep.

**예외**:
- 컨테이너 자체가 코덱을 전제하는 포맷(IVF는 사실상 VP8/VP9/AV1 전용)이라면 최소한의 "코덱 힌트 태그"만 읽는 것은 허용되지만, 신택스 파싱까지 넘어가면 안 된다.

**Bitvue 판정**: Confirmed — `bitvue-formats/src/ts.rs:465-486`의 `is_avc_keyframe`/`is_hevc_keyframe`가 TS 컨테이너 파서 내부에서 NAL type 비트(`& 0x1F`, IRAP 범위 16..=23)를 직접 해석해 키프레임을 판정한다.

---

### CODEC-002: codec parser가 파일 offset 정책을 이해
**분류**: CODEC · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-hevc/src/nal.rs
pub fn parse_nal_unit(data: &[u8], mp4_chunk_offset: u64) -> NalUnit {
    // 코덱 파서가 MP4 chunk offset을 알고 절대 파일 offset을 계산한다
    let absolute_file_offset = mp4_chunk_offset + local_cursor_position(data);
    NalUnit { file_offset: absolute_file_offset, /* ... */ }
}
```

**문제**:
- HEVC 파서가 MP4 offset 계약을 알아야 하므로, 같은 HEVC 스트림을 MKV나 raw Annex-B에서 읽을 때 함수 시그니처 자체를 바꿔야 한다.
- 컨테이너마다 offset 산정 방식(청크 기반 vs 클러스터 기반 vs 파일 전체 오프셋)이 다른데, 그 지식이 코덱 크레이트에 새어 들어온다.
- offset 계산 버그가 발생했을 때 어느 레이어(컨테이너 vs 코덱) 책임인지 구분하기 어려워진다.

**발생 조건**:
- hex view나 오프셋 하이라이트 기능처럼 "원본 파일에서 이 바이트가 어디 있는가"를 UI에 보여줘야 할 때, 가장 쉬운 경로로 코덱 파서에 offset 파라미터를 얹기 시작하면서 발생.

**권장**:
```rust
// bitvue-hevc/src/nal.rs — 코덱 파서는 상대 위치(local offset/size)만 리턴
pub fn parse_nal_unit(data: &[u8]) -> NalUnit {
    NalUnit { local_range: 0..data.len(), /* ... */ }
}

// bitvue-formats/src/offset.rs — 파일 offset 합성은 컨테이너/조합 레이어 책임
pub fn to_absolute(container_base: u64, unit: &NalUnit) -> std::ops::Range<u64> {
    container_base + unit.local_range.start as u64 .. container_base + unit.local_range.end as u64
}
```
- 코덱 크레이트가 리턴하는 것은 "이 버퍼 안에서의" 상대 위치뿐이어야 한다.
- 절대 파일 offset은 항상 컨테이너 파싱 결과와 합성하는 최상위 계층(`bitvue-decode` 혹은 UnitNode 조립 지점)에서 계산한다.

**탐지 방법**:
- Static: 코덱 크레이트 함수 시그니처에 `file_offset`, `chunk_offset`, `mp4_*`, `mkv_*` 같은 컨테이너 고유 이름이 파라미터로 들어오는지 grep.
- Structural: 코덱 크레이트가 `bitvue-formats`를 의존성으로 갖는지 `cargo tree` 역방향 확인 — 있으면 레이어 역전 신호.

**예외**:
- 없음 — offset 합성은 항상 상위 레이어의 책임이며, 코덱 파서가 컨테이너 오프셋 정책을 알아야 하는 정당한 이유는 사실상 없다.

**Bitvue 판정**: N/A — `bitvue-hevc`/`bitvue-avc`/`bitvue-vp9`/`bitvue-av1-codec` 파서 함수 시그니처에서 `file_offset`/`chunk_offset`/`mp4_*` 류 컨테이너 오프셋 파라미터를 grep했으나 발견되지 않음; 오프셋 합성은 코덱 크레이트 밖(`bitvue-core`/UnitNode 조립부)에서 처리되는 것으로 보임.

---

### CODEC-003: decode 결과와 syntax 결과를 동일 모델로 표현
**분류**: CODEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-decode/src/frame.rs
pub struct Frame {
    pub syntax: SliceHeader,      // 파싱된 신택스 (분석 목적)
    pub pixels: Option<Vec<u8>>,  // 실제 디코딩된 픽셀 (디코드 목적)
    pub mv_field: Option<MvField>,
}
// 분석 전용 경로(신택스만 필요)에서도 Frame 전체를 만들고
// pixels/mv_field는 None으로 채운 채 여기저기 Option 언랩이 반복된다
```

**문제**:
- "신택스만 보고 싶다"는 요청과 "실제로 디코딩하라"는 요청이 같은 타입을 쓰면서 `Option` 필드가 늘어나고, 호출부마다 `unwrap`/`if let` 방어 코드가 중복된다.
- 신택스 전용 분석 경로에서도 픽셀 버퍼용 메모리 레이아웃을 신경 써야 해서 성능이 손해를 본다(불필요한 할당/초기화).
- 두 관심사가 한 타입에 묶여 있으면 "신택스만 바뀌었는데 디코드 경로 테스트가 깨진다" 같은 결합이 생긴다.

**발생 조건**:
- 분석기(analyzer) 기능을 디코더 파이프라인에 얹어서 빠르게 만들 때, 새 타입을 만드는 대신 기존 `Frame`에 필드를 추가하는 방식으로 확장할 때.

**권장**:
```rust
// bitvue-codecs/src/hevc/syntax.rs — 신택스 전용, 디코드 개념 없음
pub struct SliceSyntax {
    pub header: SliceHeader,
    pub mb_partitions: Vec<PartitionInfo>,
}

// bitvue-decode/src/frame.rs — 디코드 결과는 syntax를 참조할 뿐 소유하지 않음
pub struct DecodedFrame<'s> {
    pub syntax: &'s SliceSyntax,
    pub pixels: PixelBuffer,
}
```
- "구문 분석 결과"와 "디코드(픽셀 재구성) 결과"를 별도 타입으로 분리하고, 후자가 전자를 참조/포함하는 단방향 관계로 만든다.
- 분석 전용 CLI/UI 경로는 `SliceSyntax`만 사용하고 `DecodedFrame`을 아예 구성하지 않도록 한다.

**탐지 방법**:
- Structural: 하나의 struct 안에 `Option<PixelBuffer>` 류 필드와 순수 신택스 필드가 섞여 있는지 검사.
- Manual: 코드 리뷰에서 "이 필드는 분석 경로에서 항상 None인가?"를 질문.

**예외**:
- 참조 디코더처럼 신택스와 재구성이 1:1로 강결합되어 있고 분석 전용 경로가 애초에 존재하지 않는 도구라면 분리 비용이 이득보다 클 수 있다.

**Bitvue 판정**: N/A — `bitvue-decode/src/decoder.rs:33` `DecodedFrame`은 픽셀 플레인(y/u/v_plane)만 갖는 순수 디코드 결과 타입이고, 신택스 필드와 `Option<PixelBuffer>`가 한 struct에 섞인 사례를 찾지 못함.

---

### CODEC-004: codec별 공통 타입이 의미를 잃은 거대 enum
**분류**: CODEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-core/src/unit.rs
pub enum SyntaxElement {
    HevcSps(HevcSps), HevcPps(HevcPps), HevcVps(HevcVps), HevcSlice(HevcSlice),
    AvcSps(AvcSps), AvcPps(AvcPps), AvcSlice(AvcSlice),
    Av1SequenceHeader(Av1SeqHdr), Av1FrameHeader(Av1FrameHdr), Av1TileGroup(Av1TileGroup),
    Vp9UncompressedHeader(Vp9Header),
    VvcSps(VvcSps), VvcPps(VvcPps), VvcVps(VvcVps),
    Av3FrameHeader(Av3FrameHdr),
    Mpeg2SequenceHeader(Mpeg2SeqHdr), Mpeg2PictureHeader(Mpeg2PicHdr),
    // 코덱이 늘어날 때마다 여기 계속 추가...
}

fn describe(elem: &SyntaxElement) -> &'static str {
    match elem {
        SyntaxElement::HevcSps(_) => "SPS",
        SyntaxElement::AvcSps(_) => "SPS",
        SyntaxElement::VvcSps(_) => "SPS",
        // 결국 각 variant가 "이름이 같은 것"들의 나열일 뿐, 공통 동작은 없다
        _ => "unknown",
    }
}
```

**문제**:
- 7개 코덱 × 코덱당 3~10개 유닛 타입이 하나의 flat enum에 모이면서, 이 enum을 매치하는 모든 함수가 거대한 match 문이 되고 코덱을 추가할 때마다 전 파일을 수정해야 한다.
- enum이 "공통 추상"이 아니라 "모든 걸 나열한 목록"이라 실제로는 각 variant가 서로 아무 관계도 공유하지 않는다 — 다형성의 이점이 없다.
- 새 코덱 크레이트를 추가하려면 `bitvue-core`(모든 크레이트가 의존하는 최하위 레이어)를 수정해야 해서, 코덱 크레이트를 플러그인처럼 독립적으로 추가할 수 없다.

**발생 조건**:
- "모든 신택스 요소를 하나의 트리로 보여주는 UnitNode 트리" 같은 기능을 구현하면서, 제네릭/트레이트 설계 대신 가장 손쉬운 거대 enum으로 시작해 방치할 때.

**권장**:
```rust
// bitvue-core/src/unit.rs — 코덱 비의존 공통 트레이트/모델
pub trait SyntaxUnit {
    fn kind_label(&self) -> &'static str;
    fn byte_range(&self) -> std::ops::Range<usize>;
    fn children(&self) -> &[Box<dyn SyntaxUnit>];
}

// bitvue-hevc/src/sps.rs — 코덱 크레이트가 자신의 타입에 대해서만 구현
impl SyntaxUnit for HevcSps {
    fn kind_label(&self) -> &'static str { "SPS" }
    fn byte_range(&self) -> std::ops::Range<usize> { self.range.clone() }
    fn children(&self) -> &[Box<dyn SyntaxUnit>] { &[] }
}
```
- 공통 레이어는 "모든 코덱의 신택스 타입을 나열"하지 않고, 코덱 크레이트가 구현하는 **좁은 공통 인터페이스**만 정의한다.
- 코덱 추가는 새 크레이트가 트레이트를 impl하는 것으로 끝나고, `bitvue-core`를 건드리지 않는다.

**탐지 방법**:
- Structural: `bitvue-core`의 enum variant 수가 코덱 크레이트 수 × 유닛 타입 수에 비례해서 선형 증가하는지 git blame/history로 추적.
- Static: 하나의 enum에 서로 다른 코덱 접두사(`Hevc*`, `Avc*`, `Av1*`, `Vvc*`)가 5개 이상 섞여 있는지 grep.

**예외**:
- variant 수가 적고(2~3개) 향후 확장 계획이 없는 닫힌 도메인(예: "컨테이너 종류 4가지 고정")이라면 enum이 트레이트보다 단순하고 적절하다.

**Bitvue 판정**: N/A — `HevcSps(...)`/`AvcSps(...)` 같은 코덱별 owned 타입을 나열한 거대 flat enum을 전체 크레이트에서 찾지 못함; `bitvue-core/src/evidence.rs`의 `SyntaxNodeType`은 `Custom(String)` 탈출구가 있는 얕은 라벨 enum으로, 권장안(트레이트/좁은 공통 인터페이스)에 더 가까움.

---

### CODEC-005: 모든 codec을 하나의 Frame 구조체에 억지로 통합
**분류**: CODEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-core/src/frame.rs
pub struct Frame {
    pub width: u32,
    pub height: u32,
    // HEVC/AVC 전용
    pub slice_type: Option<SliceType>,
    pub mb_qp_deltas: Option<Vec<i8>>,
    // AV1 전용
    pub film_grain_params: Option<FilmGrainParams>,
    pub superres_denom: Option<u8>,
    // VP9 전용
    pub segmentation_map: Option<Vec<u8>>,
    // VVC 전용
    pub gci_constraints: Option<GciConstraints>,
    // MPEG-2 전용
    pub dc_precision: Option<u8>,
}
```

**문제**:
- 필드 대부분이 특정 코덱 하나에서만 `Some`이 되므로, 코덱 A를 다룰 때도 코덱 B~F의 필드 존재를 알아야 하는 인지 부담이 생긴다.
- 새 코덱(AV3 같은)을 추가할 때마다 이 공용 struct에 필드를 계속 추가해야 하고, 결국 CODEC-004와 동일한 "모든 걸 아는 신"(god struct) 문제가 된다.
- `Frame` 하나를 직렬화(JSON 등)하면 대부분 `null`인 필드로 가득 찬 응답이 되어 프론트엔드도 코덱별 분기를 떠안는다.

**발생 조건**:
- "UI가 코덱에 상관없이 프레임 하나를 렌더링할 수 있게 하자"는 목표를 잘못된 방식(공통 struct에 모든 필드 합치기)으로 달성하려 할 때.

**권장**:
```rust
// bitvue-core/src/frame.rs — 코덱 비의존 공통 필드 + 확장 슬롯
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub codec_specific: CodecSpecificFrame,
}

pub enum CodecSpecificFrame {
    Hevc(bitvue_hevc::FrameInfo),
    Av1(bitvue_av1::FrameInfo),
    Vp9(bitvue_vp9::FrameInfo),
    // ...
}
```
- 코덱 공통 필드(width/height/pts/dts 등)만 최상위에 두고, 코덱 고유 정보는 각 코덱 크레이트가 정의한 타입을 감싸는 한 단계로 위임한다.
- 이러면 CODEC-004처럼 enum이 커지긴 하지만 최소한 "코덱 하나당 variant 하나"로 억제되어 필드 단위 폭발은 막는다(트레이트 기반이 더 낫다면 CODEC-004의 해법과 결합).

**탐지 방법**:
- Static: 공용 `Frame`/`Picture` struct에서 `Option<T>` 필드 비율이 절반을 넘는지 계산하는 스크립트.
- Manual: 코드 리뷰에서 "이 필드가 Some인 코덱이 몇 개인가"를 각 필드마다 확인.

**예외**:
- 다루는 코덱이 2개뿐이고 공통 필드가 압도적으로 많다면(예: AVC와 HEVC만 지원) 단일 struct + 소수의 Option이 실용적일 수 있다.

**Bitvue 판정**: N/A (설계는 존재하나 미사용) — `bitvue-core/src/frame.rs`의 `VideoFrame`/`CodecMetadata`가 정확히 권장 패턴(코덱별 enum variant)으로 구현되어 있지만 grep 결과 다른 크레이트에서 전혀 소비되지 않는 죽은 코드이고, 실제로는 각 코덱 크레이트가 `AvcFrame`/`HevcFrame`/`Vp9Frame` 등 독립 타입을 따로 소유해 god-struct가 실사용 경로에는 없음.

---

### CODEC-006: codec-specific field를 HashMap<String, Value>로 저장
**분류**: CODEC · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
// bitvue-decode/src/frame.rs
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// bitvue-av1/src/frame_header.rs
frame.extra.insert("film_grain_enabled".into(), json!(hdr.film_grain_params.is_some()));
frame.extra.insert("superres_denom".into(), json!(hdr.superres_denom));

// 프론트엔드/다른 코드에서 사용할 때
let denom = frame.extra.get("superres_denom")
    .and_then(|v| v.as_u64())
    .unwrap_or(8); // 오타나 필드 누락이 조용히 기본값으로 흡수된다
```

**문제**:
- 컴파일 타임 타입 체크가 완전히 사라진다 — 필드 이름 오타, 타입 불일치가 런타임까지 발견되지 않는다.
- `"superres_denom"` 같은 문자열 키가 코드 전체에 흩어져 리팩터링(이름 변경)이 grep 의존적이 되고 안전하지 않다.
- IDE 자동완성/`cargo check`의 이점을 모두 잃어, 코덱 신택스 필드가 사실상 동적 타입 언어 수준으로 후퇴한다.

**발생 조건**:
- "코덱마다 필드가 달라서 정적 타입으로 표현하기 귀찮다"는 이유로 범용 맵을 도입할 때. 프로토타입 단계에서 시작해 그대로 프로덕션에 남을 때 특히 흔하다.

**권장**:
```rust
// bitvue-av1/src/frame_header.rs — 코덱 고유 타입으로 명시
pub struct Av1FrameInfo {
    pub film_grain_enabled: bool,
    pub superres_denom: u8,
}

// bitvue-core/src/frame.rs
pub enum CodecSpecificFrame {
    Av1(Av1FrameInfo),
    Hevc(HevcFrameInfo),
    // ...
}
```
- 코덱별 필드는 코덱 크레이트가 소유한 명시적 struct로 정의한다(CODEC-005의 해법과 동일 방향).
- "정말로" 스키마가 동적이어야 하는 경우(예: 사용자 정의 SEI/메타데이터 사전 정보 없는 확장 데이터)에 한해서만 `HashMap`을 국소적으로 허용한다.

**탐지 방법**:
- Static: `HashMap<String, serde_json::Value>` 또는 `BTreeMap<String, Value>` 타입 선언을 grep, 특히 코덱/디코드 크레이트 내부.
- Structural: 해당 맵에 대한 `.insert("...")` 호출이 여러 코덱 크레이트에 흩어져 있는지 확인.

**예외**:
- 정말 스키마가 없는 확장 메타데이터(사용자 정의 SEI payload, 벤더 사설 확장 등)를 "그대로 통과시키는" 용도라면 타입화 자체가 불가능하므로 맵이 적절하다 — 단, 이 경우 필드명에 "unknown/vendor" 등으로 명시해 알려진 필드와 섞이지 않게 한다.

**Bitvue 판정**: N/A — `HashMap<String, serde_json::Value>`는 `bitvue-core/src/performance.rs:313`의 `PerfEvent.extra`(성능 이벤트 확장 필드) 한 곳뿐이며, 코덱 신택스 필드 저장 용도로 쓰이는 사례는 발견되지 않음(문서의 예외 조항에 해당하는 범용 확장 메타데이터에 가까움).

---

### CODEC-007: 문자열 codec dispatch
**분류**: CODEC · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
// bitvue-decode/src/dispatch.rs
pub fn parse_bitstream(codec_name: &str, data: &[u8]) -> Result<ParsedUnit, ParseError> {
    match codec_name {
        "hevc" | "h265" => bitvue_hevc::parse(data),
        "avc" | "h264" => bitvue_avc::parse(data),
        "av1" => bitvue_av1::parse(data),
        "vp9" => bitvue_vp9::parse(data),
        _ => Err(ParseError::UnknownCodec(codec_name.to_string())),
    }
}
```

**문제**:
- 코덱 이름 문자열이 호출부마다 하드코딩되어 `"hevc"`, `"HEVC"`, `"h265"`, `"hev1"` 같은 표기 불일치가 조용히 `UnknownCodec`으로 빠진다.
- 컴파일러가 "이 코덱 case를 빠뜨렸다"를 알려주지 못한다 — 새 코덱 추가 시 이 dispatch 함수를 빼먹어도 컴파일은 성공한다.
- 문자열 비교 비용과 오타 취약성이 hot path(프레임마다 호출되는 경로)에 들어가면 성능과 안정성 모두 손해다.

**발생 조건**:
- 컨테이너 메타데이터(fourcc, codec tag)를 그대로 문자열로 들고 다니다가, 그 문자열을 그대로 dispatch key로 재사용할 때.

**권장**:
```rust
// bitvue-core/src/codec_id.rs — 닫힌 enum으로 컴파일 타임 보장
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CodecId { Hevc, Avc, Av1, Vp9, Vvc, Av3, Mpeg2 }

impl TryFrom<Fourcc> for CodecId {
    type Error = UnknownCodecTag;
    fn try_from(tag: Fourcc) -> Result<Self, Self::Error> {
        match tag.as_bytes() {
            b"hev1" | b"hvc1" => Ok(CodecId::Hevc),
            b"avc1" | b"avc3" => Ok(CodecId::Avc),
            b"av01" => Ok(CodecId::Av1),
            _ => Err(UnknownCodecTag(tag)),
        }
    }
}

pub fn parse_bitstream(codec: CodecId, data: &[u8]) -> Result<ParsedUnit, ParseError> {
    match codec {
        CodecId::Hevc => bitvue_hevc::parse(data),
        CodecId::Avc => bitvue_avc::parse(data),
        CodecId::Av1 => bitvue_av1::parse(data),
        CodecId::Vp9 => bitvue_vp9::parse(data),
        CodecId::Vvc => bitvue_vvc::parse(data),
        CodecId::Av3 => bitvue_av3::parse(data),
        CodecId::Mpeg2 => bitvue_mpeg2::parse(data),
    } // match가 non-exhaustive면 컴파일 에러 — 새 variant 추가 시 강제로 잡힘
}
```
- fourcc/문자열 → `CodecId` enum 변환은 경계(컨테이너 파싱 직후) 한 곳에서만 수행하고, 이후 모든 내부 로직은 enum으로 dispatch한다.
- `match`를 non-exhaustive 상태로 두지 않아 컴파일러가 누락된 코덱 case를 강제로 알려주게 한다.

**탐지 방법**:
- Static: `match codec_name` / `if codec == "..."` 형태의 문자열 비교 기반 dispatch grep.
- Structural: 동일한 코덱 판별 문자열 리터럴 집합이 여러 파일에 중복 등장하는지 확인.

**예외**:
- 외부 설정 파일/CLI 인자처럼 원래 문자열로 들어오는 최초 입력 지점에서 파싱하는 것은 당연히 필요하다 — 문제는 그 문자열이 내부 로직 전체로 전파되는 것이다.

**Bitvue 판정**: Confirmed — `src-tauri/src/commands/frame.rs:694-713`의 `is_hevc_codec`/`is_avc_codec`가 `codec.to_lowercase().contains("265")`/`.contains("264")` 같은 느슨한 부분 문자열 매칭으로 dispatch하며, `bitvue-core`에 `Codec` enum이 `codec_error.rs`/`semantic_evidence.rs`/`player/extractor.rs` 세 곳에 중복 정의되어 있어 단일 닫힌 enum 경계가 지켜지지 않음.

---

### CODEC-008: hot primitive까지 dyn trait 사용
**분류**: CODEC · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
// bitvue-codecs/src/bitreader.rs
pub trait BitSource {
    fn read_bit(&mut self) -> bool;
    fn read_bits(&mut self, n: u32) -> u32;
}

// 모든 코덱 파서가 dyn 참조로 비트를 읽는다
pub fn parse_slice_header(reader: &mut dyn BitSource) -> SliceHeader {
    let mut sh = SliceHeader::default();
    sh.first_slice = reader.read_bit(); // 비트 하나마다 vtable indirect call
    for _ in 0..64 {
        sh.mb_skip_flags.push(reader.read_bit()); // 매크로블록마다 vtable 호출 반복
    }
    sh
}
```

**문제**:
- `read_bit`처럼 프레임당 수십만~수백만 번 호출되는 극히 hot한 primitive를 `dyn Trait`로 감싸면, 매 호출이 vtable indirect call이 되어 인라이닝이 완전히 막힌다.
- CPU 분기 예측기와 인라이닝 최적화의 이점을 모두 잃어, 순수 구조적 유연성 대비 실측 디코드/파싱 성능이 눈에 띄게 저하된다(비트리더는 전형적인 프로파일링 핫스팟).
- 동적 디스패치가 필요한 이유(런타임에 구현체를 바꿔야 함)가 실제로는 거의 없는데도 습관적으로 trait object를 쓰는 경우가 많다.

**발생 조건**:
- "모든 코덱이 같은 인터페이스를 쓰게 하자"는 목표를 제네릭이 아니라 trait object로 달성했을 때, 특히 그 인터페이스가 비트/바이트 단위로 호출되는 최하위 primitive일 때.

**권장**:
```rust
// bitvue-codecs/src/bitreader.rs — 제네릭 + 정적 디스패치
pub trait BitSource {
    fn read_bit(&mut self) -> bool;
    fn read_bits(&mut self, n: u32) -> u32;
}

#[inline]
pub fn parse_slice_header<R: BitSource>(reader: &mut R) -> SliceHeader {
    let mut sh = SliceHeader::default();
    sh.first_slice = reader.read_bit(); // 컴파일 타임에 구체 타입으로 monomorphize, 인라인 가능
    for _ in 0..64 {
        sh.mb_skip_flags.push(reader.read_bit());
    }
    sh
}
```
- 프레임/매크로블록/비트 단위로 호출되는 hot primitive는 제네릭(`impl Trait` 혹은 `<R: BitSource>`)으로 정적 디스패치하여 인라이닝을 열어둔다.
- `dyn Trait`는 호출 빈도가 낮고(파일당 1~수십 회) 구현체 다형성이 실제로 필요한 상위 레벨 경계(예: 코덱 dispatch 자체)에만 남긴다.

**탐지 방법**:
- Runtime: `perf`/flamegraph에서 비트리더 관련 vtable/`<dyn ... as Trait>::call` 프레임이 상위권에 나타나는지 확인.
- Static: `&mut dyn BitSource`, `Box<dyn BitSource>` 같은 시그니처가 파일/매크로블록 단위 루프 안에서 호출되는지 grep.

**예외**:
- 비트리더가 아니라 "파일당 한 번" 수준으로 호출되는 상위 계층(코덱 선택, 컨테이너 데먹서 선택)이라면 `dyn Trait`의 오버헤드는 무시할 수 있는 수준이다.

**Bitvue 판정**: N/A — `dyn BitSource`/`Box<dyn BitReader>` 류 trait object를 어떤 코덱 크레이트에서도 찾지 못함; 비트리더는 전부 구체 타입(concrete struct)으로 구현되어 있음.

---

### CODEC-009: 모든 계층을 generic으로 만들어 binary bloat 발생
**분류**: CODEC · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-decode/src/pipeline.rs
pub struct DecodePipeline<C, R, W>
where
    C: CodecParser,
    R: BitSource,
    W: OutputWriter,
{
    parser: C,
    reader: R,
    writer: W,
}

impl<C: CodecParser, R: BitSource, W: OutputWriter> DecodePipeline<C, R, W> {
    pub fn run(&mut self) { /* 수천 줄짜리 파이프라인 전체가 제네릭 */ }
}
// 7개 코덱 × 여러 BitSource 구현체 × 여러 OutputWriter 조합마다
// run()을 포함한 전체 파이프라인 코드가 통째로 monomorphize된다
```

**문제**:
- `DecodePipeline<C, R, W>::run`처럼 코드량이 큰 함수를 제네릭 파라미터 3개로 감으면, 코덱 7종 × 구현체 조합 수만큼 동일 로직이 바이너리에 복제(monomorphization)되어 바이너리 크기와 컴파일 시간이 폭증한다.
- 명령줄 도구나 데스크톱 앱(Tauri)에서는 바이너리 크기가 실제 배포/시작 시간에 영향을 주는데, 이런 비용이 프로파일링 없이 누적된다.
- 컴파일 타임 증가는 개발 루프(코덱 파서 수정 → 재빌드)를 느리게 만들어 생산성에도 영향을 준다.

**발생 조건**:
- CODEC-008의 교훈(hot primitive는 generic이 좋다)을 과잉 적용해서, 정말로 hot하지 않은 상위 파이프라인 레벨까지 전부 제네릭화할 때.

**권장**:
```rust
// bitvue-decode/src/pipeline.rs — hot한 내부 루프만 generic, 바깥 껍질은 dyn/enum
pub struct DecodePipeline {
    parser: Box<dyn CodecParser>,
    writer: Box<dyn OutputWriter>,
}

impl DecodePipeline {
    pub fn run(&mut self) {
        // 파이프라인 오케스트레이션 자체는 파일당 1회 수준이므로 dyn 비용 무시 가능
        while let Some(unit) = self.parser.next_unit() {
            // 정작 비트 단위 hot loop는 내부에서 generic BitSource로 처리(CODEC-008)
            self.writer.write(unit);
        }
    }
}
```
- "얼마나 자주 호출되는가"를 기준으로 제네릭(정적 디스패치)과 `dyn`(동적 디스패치)의 경계를 의도적으로 그린다: hot loop 내부는 generic, 파일/스트림 단위 오케스트레이션은 dyn으로도 충분하다.
- `cargo bloat` 등으로 실측한 뒤 제네릭화 범위를 결정한다 — 추측으로 전면 제네릭화하지 않는다.

**탐지 방법**:
- Structural: `cargo bloat --release -n 50`으로 monomorphization 기인 심볼 중복 확인.
- Static: 제네릭 파라미터가 3개 이상이면서 함수 본문이 수백 줄 이상인 함수 탐색.

**예외**:
- 실제로 여러 구현체가 빈번히 교체되며 그 교체가 성능에 결정적인 경로(예: 여러 픽셀 포맷에 대한 SIMD 커널)라면 제네릭이 정당하다 — 이때도 파이프라인 전체가 아니라 커널 함수 단위로 국소화한다.

**Bitvue 판정**: N/A — 제네릭 파라미터 3개 이상의 대형 파이프라인 struct를 찾지 못함; `bitvue-core/src/player/pipeline.rs`의 `PlayerPipeline`은 비제네릭 struct임.

---

### CODEC-010: codec별 BitReader 전체 복제
**분류**: CODEC · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-hevc/src/bitreader.rs
pub struct HevcBitReader<'a> { data: &'a [u8], byte_pos: usize, bit_pos: u8 }
impl<'a> HevcBitReader<'a> {
    pub fn read_bit(&mut self) -> bool { /* ... 20줄 ... */ }
    pub fn read_bits(&mut self, n: u32) -> u32 { /* ... 15줄 ... */ }
    pub fn read_ue(&mut self) -> u32 { /* exp-golomb, 25줄 */ }
    pub fn read_se(&mut self) -> i32 { /* ... */ }
}

// bitvue-avc/src/bitreader.rs — 위와 거의 동일한 코드를 복붙
pub struct AvcBitReader<'a> { data: &'a [u8], byte_pos: usize, bit_pos: u8 }
impl<'a> AvcBitReader<'a> {
    pub fn read_bit(&mut self) -> bool { /* ... 동일한 20줄 ... */ }
    pub fn read_ue(&mut self) -> u32 { /* ... 동일한 exp-golomb 25줄 ... */ }
    // ...
}
// bitvue-vvc, bitvue-av3에도 각각 유사 복제본 존재
```

**문제**:
- HEVC/AVC/VVC는 모두 exp-golomb(`ue(v)`/`se(v)`) 기반 비트리더를 쓰는데, 이 로직이 4~5개 크레이트에 복제되어 있으면 버그(예: emulation prevention byte 처리 실수)를 고치려면 크레이트 수만큼 반복 수정해야 한다.
- 복제본들이 시간이 지나며 미묘하게 달라져(하나는 buffer overrun 체크를 하고 하나는 안 하는 등) 코덱마다 안정성이 들쭉날쭉해진다.
- 코드 리뷰/테스트도 크레이트마다 중복되어 전체 유지보수 비용이 코덱 수에 비례해 커진다.

**발생 조건**:
- 새 코덱 크레이트를 만들 때 기존 크레이트를 복사해서 시작(CODEC-016과 동일한 뿌리)하고, 공통 부분을 나중에 정리하기로 미뤘다가 방치될 때.

**권장**:
```rust
// bitvue-codecs/src/bitreader.rs — Annex-B 계열이 공유하는 공통 구현
pub struct EmulationPreventionBitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}
impl<'a> EmulationPreventionBitReader<'a> {
    pub fn read_bit(&mut self) -> bool { /* emulation prevention 처리 포함, 한 곳에만 존재 */ }
    pub fn read_ue(&mut self) -> u32 { /* exp-golomb, 공용 구현 */ }
    pub fn read_se(&mut self) -> i32 { /* ue 기반, 공용 구현 */ }
}

// bitvue-hevc/src/nal.rs — 공용 리더를 그대로 사용
use bitvue_codecs::bitreader::EmulationPreventionBitReader;
```
- Exp-golomb, emulation-prevention-byte 스킵처럼 여러 코덱(H.264/HEVC/VVC 계열)이 진짜로 공유하는 저수준 로직은 `bitvue-codecs` 공통 모듈로 한 번만 구현한다.
- AV1의 leb128/uvlc처럼 코덱군이 다른 인코딩은 억지로 같은 함수에 합치지 않고 별도 공용 함수로 분리한다 — "공통화"가 "억지 통합"이 되지 않도록 실제 신택스 유사성을 기준으로 판단한다.

**탐지 방법**:
- Structural: 여러 코덱 크레이트에서 함수 바디의 토큰 유사도를 비교하는 중복 탐지 도구(`cargo-similar`류) 실행.
- Manual: `read_ue`/`read_se`/exp-golomb 구현이 몇 개 크레이트에 존재하는지 grep으로 카운트.

**예외**:
- AV1의 leb128과 HEVC의 exp-golomb처럼 근본적으로 다른 인코딩을 억지로 하나의 함수로 합치는 것은 오히려 CODEC-012와 같은 과잉 추상화가 된다 — "정말 같은 알고리즘"인 것만 공유한다.

**Bitvue 판정**: N/A — `bitvue-avc`/`bitvue-hevc`/`bitvue-vvc`/`bitvue-mpeg2-codec`의 `bitreader.rs`가 모두 `bitvue_core::BitReader` + `ExpGolombReader` 트레이트 + `remove_emulation_prevention_bytes`를 감싸는 얇은 wrapper로 구현되어 있어, 문서가 권장하는 '공통 저수준 구현 공유' 패턴을 그대로 따름(전체 복제가 아님).

---

### CODEC-011: 코덱별 오류 타입이 문자열로 평탄화
**분류**: CODEC · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
// bitvue-hevc/src/sps.rs
pub fn parse_sps(data: &[u8]) -> Result<HevcSps, String> {
    if data.len() < 4 {
        return Err(format!("SPS too short: {} bytes", data.len()));
    }
    if profile_idc > 12 {
        return Err(format!("invalid profile_idc: {profile_idc}"));
    }
    Ok(sps)
}

// 호출부
match parse_sps(data) {
    Err(e) if e.contains("too short") => { /* 문자열 내용을 파싱해서 분기 */ }
    Err(e) => log::error!("{e}"),
    Ok(sps) => { /* ... */ }
}
```

**문제**:
- 에러를 문자열로만 표현하면 호출부가 원인별로 다르게 대응(예: "짧은 데이터는 재시도, 잘못된 profile은 즉시 실패")하려 할 때 문자열 내용을 파싱하는 안티패턴(`e.contains(...)`)을 유발한다.
- 다국어 UI(한국어/영어 전환)에서 에러 메시지를 그대로 노출하면 로케일 대응이 불가능해지고, 에러 문자열을 UI 문구로 재사용하면 관심사가 섞인다.
- 에러 발생 위치(어느 필드, 어느 바이트 오프셋)가 문자열 안에 섞여 있어 구조적으로 캡처·집계(예: "이번 스트림에서 profile_idc 오류가 몇 번 났는가")하기 어렵다.

**발생 조건**:
- 빠른 프로토타이핑 단계에서 `Result<T, String>`으로 시작한 뒤 정식 에러 타입으로 마이그레이션하지 않고 그대로 굳어질 때.

**권장**:
```rust
// bitvue-hevc/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum HevcParseError {
    #[error("SPS too short: {actual} bytes (need at least {min})")]
    SpsTooShort { actual: usize, min: usize },
    #[error("invalid profile_idc: {value}")]
    InvalidProfileIdc { value: u8, offset: usize },
}

pub fn parse_sps(data: &[u8]) -> Result<HevcSps, HevcParseError> {
    if data.len() < 4 {
        return Err(HevcParseError::SpsTooShort { actual: data.len(), min: 4 });
    }
    // ...
    Ok(sps)
}

// 호출부 — 타입으로 안전하게 분기
match parse_sps(data) {
    Err(HevcParseError::SpsTooShort { actual, min }) => { /* 구조적으로 처리 */ }
    Err(e @ HevcParseError::InvalidProfileIdc { .. }) => log::error!("{e}"),
    Ok(sps) => { /* ... */ }
}
```
- 코덱별 파싱 에러는 `thiserror` 등으로 구조화된 enum으로 정의하고, 상위(`bitvue-decode`, UI)에서 이를 감싸는 최상위 에러 enum으로 합성한다.
- 사용자 표시용 메시지는 `Display`/i18n 레이어에서 별도로 생성하고, 내부 로직은 절대 문자열 매칭으로 분기하지 않는다.

**탐지 방법**:
- Static: `Result<_, String>` 시그니처 및 `.contains(`/`.starts_with(` 패턴으로 에러 문자열을 검사하는 호출부 grep.
- Structural: 코덱 크레이트에 `thiserror`/`std::error::Error` 구현 여부 확인.

**예외**:
- 정말 일회성 디버그 로그나 CLI 진단 출력처럼, 구조적으로 분기할 필요가 전혀 없는 말단 소비처라면 단순 문자열 에러도 실용적이다.

**Bitvue 판정**: N/A — `bitvue-hevc`의 핵심 파싱 경로(`parse_sps`/`parse_pps`)는 `thiserror` 기반 `HevcError` enum을 리턴함; `Result<_, String>`은 `frames.rs`의 빌더 검증 헬퍼 등 지엽적인 곳에서만 발견됨.

---

### CODEC-012: reference frame semantics를 공통 모델로 과도하게 추상화
**분류**: CODEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// bitvue-core/src/reference.rs — "모든 코덱에 맞는" 범용 참조 모델을 시도
pub struct ReferenceSlot {
    pub index: u8,
    pub poc_or_frame_num: i32, // HEVC의 POC와 AVC의 frame_num을 억지로 한 필드에
    pub is_long_term: bool,    // VP9/AV1엔 이 개념이 아예 다르게 존재
}

pub struct ReferencePicSet {
    pub slots: [Option<ReferenceSlot>; 8], // AV1은 참조 슬롯이 8개, HEVC/AVC는 다른 규칙
}
// AV1의 ref_frame_idx[7], HEVC의 RPS(RefPicSetStCurrBefore/After/LtCurr),
// VP9의 ref_frame_sign_bias까지 전부 이 struct 하나로 표현하려다
// 결국 코덱마다 절반은 안 쓰는 필드 + 코덱별 특수 규칙을 우겨넣는 if문 범벅이 됨
fn resolve_hevc(rps: &ReferencePicSet) -> HevcRefLists { /* 억지 변환 로직 200줄 */ }
```

**문제**:
- HEVC의 RPS(단기/장기, Curr/Foll 분류), AVC의 sliding window/MMCO, VP9의 8-slot 순환 버퍼, AV1의 `ref_frame_idx` + virtual buffer는 참조 프레임 관리 모델 자체가 근본적으로 다른데, 이를 하나의 공통 struct로 표현하려 하면 각 코덱의 고유 규칙이 "공통 모델로 억지 변환하는 어댑터 함수" 뒤에 숨어버린다.
- 공통 모델이 실제로는 어느 코덱의 semantics도 정확히 담지 못해서("최소공배수"가 아니라 "최대공약수"만 담는 모델이 되어), 코덱별 특수 케이스(예: HEVC의 RASL/RADL 처리)를 표현할 자리가 없어 별도 사이드 채널로 새어나간다.
- 참조 관리 버그는 디코드 정합성에 직결되는데, 추상화 계층이 두꺼울수록 "실제 신택스가 이렇게 말하는데 공통 모델이 이렇게 변환했다"는 이중 진실을 디버깅해야 한다.

**발생 조건**:
- "참조 프레임 UI 패널을 코덱에 상관없이 재사용하자"는 UI 요구사항을 데이터 모델 레벨의 강제 통합으로 해결하려 할 때. 특히 두 코덱만 보고 설계하다가 세 번째, 네 번째 코덱에서 모델이 깨지는 패턴으로 나타난다.

**권장**:
```rust
// 코덱별로 자신의 semantics를 정확히 표현하는 고유 타입을 유지
// bitvue-hevc/src/rps.rs
pub struct HevcRefPicSet {
    pub st_curr_before: Vec<PocLsb>,
    pub st_curr_after: Vec<PocLsb>,
    pub lt_curr: Vec<PocLsb>,
}
// bitvue-av1/src/refs.rs
pub struct Av1RefFrames {
    pub ref_frame_idx: [u8; 7],
    pub virtual_buffer: [RefFrameSlot; 8],
}

// bitvue-core/src/reference.rs — UI가 요구하는 "표시용" 뷰만 얕게 공통화
pub trait ReferenceView {
    fn display_entries(&self) -> Vec<RefDisplayEntry>; // 화면에 보여줄 최소 정보만
}
impl ReferenceView for HevcRefPicSet { /* st/lt를 순회하며 RefDisplayEntry로 매핑 */ }
impl ReferenceView for Av1RefFrames { /* virtual_buffer를 순회하며 매핑 */ }
```
- 참조 프레임의 **진짜 semantics**(POC 산정 규칙, 장단기 구분, 슬롯 갱신 규칙)는 코덱별 고유 타입으로 정확히 유지하고, UI 등 상위 소비자를 위한 것은 "표시용 뷰"라는 훨씬 얕고 손실 있는 변환만 공통화한다.
- 공통화 대상이 "정확한 재구성이 필요한 신택스 모델"인지 "화면에 목록 하나 보여주면 되는 뷰"인지를 먼저 구분한다.

**탐지 방법**:
- Semantic: 공통 참조 모델에서 특정 코덱만을 위한 `if codec == X { ... }` 특수 케이스가 얼마나 많은지 카운트 — 많을수록 추상화가 잘못된 신호.
- Manual: 코덱 전문가 리뷰로 "이 공통 모델이 실제 스펙의 참조 관리 규칙을 정확히 표현하는가"를 코덱별로 확인.

**예외**:
- 정말로 표시/통계 목적의 얕은 뷰(예: "이 프레임이 참조하는 프레임이 몇 개인가" 카운트)라면 공통 인터페이스가 적절하다 — 문제는 "재구성에 필요한 정확한 규칙"까지 공통 모델에 밀어넣을 때다.

**Bitvue 판정**: Confirmed(단, 문서의 예외 조항 해당 가능) — `bitvue-core/src/temporal_state.rs:94` `ReferenceSlot`이 AV1의 8-slot, H.264의 slot 수, VP9 전용 참조 이름(`Vp9Last`/`Vp9Golden`/`Vp9Altref`), short/long-term 플래그를 하나의 struct/enum(`TemporalRefType`)에 뭉쳐 담고 있음; 다만 필드 구성(slot_idx, usage_count, age 등)으로 볼 때 DPB 디버그/시각화용 얕은 뷰로 보여 문서의 '표시 목적' 예외에 해당할 가능성이 있음.

---

### CODEC-013: decode order와 display order 혼동
**분류**: CODEC · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
// bitvue-decode/src/timeline.rs
pub fn build_timeline(units: Vec<ParsedUnit>) -> Vec<TimelineEntry> {
    units.into_iter().enumerate()
        .map(|(i, u)| TimelineEntry {
            frame_index: i as u32, // 비트스트림에 등장한 순서 = decode order를
            unit: u,                // 그대로 "프레임 번호"로 사용
        })
        .collect()
    // B-frame이 있는 스트림에서 이 timeline을 그대로 필름스트립에 그리면
    // 재생 순서가 아니라 디코드 순서로 프레임이 나열된다
}
```

**문제**:
- B-frame(양방향 예측)이 있는 GOP 구조에서는 비트스트림 등장 순서(decode order)와 실제 화면 표시 순서(display/output order, POC 기반)가 다른데, 이를 구분하지 않으면 필름스트립/타임라인 UI가 프레임을 잘못된 순서로 보여준다.
- "다음 프레임" 탐색, 프레임 간 diff, GOP 경계 판정 등 UI 상호작용 전체가 뒤틀린 순서 위에서 동작하게 되어 사용자가 관찰하는 모든 것이 미묘하게 틀어진다.
- 이 버그는 IDR-only 스트림이나 P-frame만 있는 저지연 스트림으로 테스트하면 전혀 드러나지 않고, B-frame이 포함된 일반적인 스트림에서만 나타나 회귀 테스트 커버리지가 없으면 오래 방치된다.

**발생 조건**:
- 테스트/개발 과정에서 저지연 프로파일(B-frame 없음) 샘플만 사용했을 때 특히 잘 숨는다.
- 컨테이너의 샘플 순서(decode order로 저장됨, CTTS/edit list로 display 순서를 표현)와 코덱의 POC를 별도 개념으로 다루지 않고 하나로 뭉뚱그릴 때.

**권장**:
```rust
// bitvue-decode/src/timeline.rs
pub struct TimelineEntry {
    pub decode_order: u32,       // 비트스트림/샘플 테이블 등장 순서
    pub display_order: u32,      // POC(HEVC/AVC) 또는 order_hint(AV1/VP9) 기반 정렬 순서
    pub unit: ParsedUnit,
}

pub fn build_timeline(units: Vec<ParsedUnit>) -> Vec<TimelineEntry> {
    let mut entries: Vec<TimelineEntry> = units.into_iter().enumerate()
        .map(|(i, u)| {
            let poc = u.compute_poc(); // 코덱별 POC/order_hint 계산 로직에 위임
            TimelineEntry { decode_order: i as u32, display_order: poc, unit: u }
        })
        .collect();
    entries // decode_order와 display_order를 둘 다 보존, 정렬은 소비자가 목적에 맞게 선택
}

// 필름스트립은 명시적으로 display_order 기준 정렬을 요청
let mut for_display = timeline.clone();
for_display.sort_by_key(|e| e.display_order);
```
- 두 순서를 별개 필드로 항상 함께 보존하고, "정렬 기준이 무엇인가"를 호출부가 명시적으로 선택하게 한다.
- POC/order_hint 계산은 코덱마다 규칙이 다르므로(HEVC의 POC MSB/LSB 조합, AV1의 `OrderHint` wrap 처리 등) 코덱 크레이트에 위임하고 공통 레이어는 계산된 값만 소비한다.

**탐지 방법**:
- Semantic: B-frame이 포함된 테스트 픽스처(예: hierarchical-B GOP)로 필름스트립 순서를 육안/스냅샷 테스트로 검증.
- Manual: `frame_index`, `frame_num`처럼 순서 의미가 모호한 필드명이 decode/display 구분 없이 쓰이는지 리뷰.

**예외**:
- 저지연/올인트라 전용 도구처럼 B-frame을 애초에 지원하지 않는다고 명시적으로 스펙에 박아둔 경우, 두 순서가 항상 같으므로 단일 필드로 단순화하는 것이 정당할 수 있다 — 단 이 가정을 코드에 주석/타입으로 명시해야 한다.

**Bitvue 판정**: N/A — `bitvue-core/src/frame_identity.rs`의 `FrameIndexMap`이 `decode_idx`↔`display_idx`를 PTS 기준으로 정렬해 양방향으로 명시적으로 보존하는 구조로, 권장 패턴(두 순서를 별도 필드로 유지)을 그대로 구현함.

---

### CODEC-014: track timestamp와 codec timestamp 혼동
**분류**: CODEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// bitvue-formats/src/mp4/track.rs
pub fn sample_time_seconds(sample_index: u32, timescale: u32, stts: &Stts) -> f64 {
    let decode_time = stts.cumulative_delta(sample_index);
    decode_time as f64 / timescale as f64
}

// bitvue-decode/src/present.rs
pub fn present(frame: &Frame, sample_index: u32) {
    // MP4 track timescale 기반 시간을 그대로 "코덱 레벨 PTS"인 것처럼 사용하고
    // AV1 OBU의 timing_info나 HEVC VUI의 time_scale/num_units_in_tick은 무시
    let t = sample_time_seconds(sample_index, track_timescale, &stts);
    render_at(t);
}
```

**문제**:
- 컨테이너 트랙 타임스케일(예: MP4의 `mvhd`/`mdhd` timescale)과 코덱 내부의 타이밍 정보(HEVC VUI의 `time_scale`/`num_units_in_tick`, AV1 시퀀스 헤더의 `timing_info`)는 서로 다른 출처인데, 코드가 트랙 타임스케일만 보고 "이게 곧 코덱이 말하는 프레임 표시 시각"이라고 가정하면 VFR(가변 프레임레이트) 스트림이나 pulldown/telecine이 섞인 스트림에서 어긋난다.
- CTTS(composition time offset)를 무시하고 decode timestamp를 그대로 표시 시각으로 쓰면 CODEC-013과 유사하게 B-frame이 있는 스트림에서 타임라인이 틀어진다.
- raw 코덱 스트림(컨테이너 없는 Annex-B/IVF)을 열었을 때는애초에 트랙 타임스케일 자체가 없으므로, 트랙 타임스케일에 의존하는 코드 경로가 널 참조나 잘못된 기본값(예: 억지로 30fps 가정)으로 빠진다.

**발생 조건**:
- MP4처럼 컨테이너가 타이밍 정보를 명시적으로 제공하는 포맷만 테스트하고, IVF/raw Annex-B처럼 컨테이너가 타이밍을 안 주거나 코덱이 자체 타이밍 정보를 갖는 포맷을 다룰 때 드러난다.
- VFR 콘텐츠나 CTTS가 0이 아닌(B-frame 존재) 샘플을 테스트 픽스처에 포함하지 않았을 때.

**권장**:
```rust
// bitvue-core/src/timing.rs — 출처를 명시적으로 구분해서 보존
pub struct FrameTiming {
    pub container_dts: Option<Duration>, // 컨테이너가 제공하는 decode timestamp (없을 수 있음)
    pub container_pts: Option<Duration>, // DTS + CTTS(composition offset) 반영된 표시 시각
    pub codec_timing_hint: Option<CodecTimingInfo>, // VUI/timing_info 등 코덱 자체 힌트
}

pub fn resolve_display_time(t: &FrameTiming, fallback_fps: f64, frame_idx: u32) -> Duration {
    // 우선순위: 컨테이너 PTS > 코덱 타이밍 힌트 기반 계산 > 고정 fps 폴백
    t.container_pts
        .or_else(|| t.codec_timing_hint.as_ref().map(|h| h.compute_pts(frame_idx)))
        .unwrap_or_else(|| Duration::from_secs_f64(frame_idx as f64 / fallback_fps))
}
```
- 컨테이너 타이밍(트랙 timescale + DTS/CTTS)과 코덱 타이밍(VUI/timing_info)을 별개 필드로 보존하고, "표시 시각을 어떻게 결정할지"의 우선순위/폴백 규칙을 한 함수에 명시적으로 문서화한다.
- 컨테이너가 없는 raw 스트림 입력 경로를 반드시 테스트 매트릭스에 포함해 "트랙 타임스케일이 없다"는 경우를 널 처리가 아니라 정상 분기로 다룬다.

**탐지 방법**:
- Semantic: VFR 픽스처와 raw(컨테이너 없는) 픽스처 각각으로 표시 시각 계산 결과를 스냅샷 비교.
- Manual: "PTS"/"timestamp"라는 이름의 변수가 실제로 컨테이너 값인지 코덱 값인지 타입/주석으로 구분되어 있는지 리뷰.

**예외**:
- 컨테이너가 없는 raw 코덱 스트림만 지원하는 도구라면 애초에 "트랙 타임스케일"이라는 개념이 없으므로 이 구분이 무의미하다 — 이 경우 코덱 타이밍 힌트만 다루면 된다.

**Bitvue 판정**: Suspected — `bitvue-core`에 `container_dts`/`container_pts`/`codec_timing_hint` 같은 출처 구분 타입이 없고, HEVC/AV1 VUI `timing_info`/`num_units_in_tick`은 파싱은 되지만(`bitvue-hevc/src/sps.rs`, `bitvue-av1-codec/src/sequence.rs`) 이를 컨테이너 타임스케일과 명시적 우선순위로 합성하는 코드를 찾지 못함; 실제 결함까지는 확인하지 못해 Suspected로 남김.

---

### CODEC-015: frame/packet/access unit 개념 혼용
**분류**: CODEC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
// 여러 곳에서 "Frame"이라는 이름을 서로 다른 의미로 사용
// bitvue-formats/src/demux.rs
pub struct Frame { pub data: Vec<u8> } // 실제로는 컨테이너의 "샘플/패킷" 하나(디먹싱 단위)

// bitvue-hevc/src/au.rs
pub struct Frame { pub nal_units: Vec<NalUnit> } // 실제로는 "access unit"(NAL 여러 개 묶음)

// bitvue-av1/src/tu.rs
pub struct Frame { pub obus: Vec<Obu> } // 실제로는 "temporal unit"(show_existing_frame 포함 가능,
                                          // 하나의 temporal unit이 여러 "frame"을 표현할 수도 있음)

fn count_frames(units: &[Frame]) -> usize { units.len() } // 호출부는 이 셋 중 뭘 세는지 모른 채 호출
```

**문제**:
- 컨테이너의 "샘플/패킷"(디먹싱 최소 단위), 코덱의 "access unit"(하나의 출력 프레임에 대응하는 NAL/OBU 묶음), 그리고 "frame"(실제 디코딩되어 화면에 나오는 픽처) 개념이 코덱과 컨테이너마다 미묘하게 다른데도 전부 `Frame`이라는 같은 이름을 쓰면, 코드 리뷰/디버깅에서 "이 카운트가 무엇을 세고 있는지"를 매번 문맥으로 추론해야 한다.
- AV1의 temporal unit은 `show_existing_frame`으로 인해 하나의 unit이 여러 출력 프레임을 만들 수도, 반대로 여러 unit이 모여 하나의 프레임이 될 수도 있어(scalability) "1 unit = 1 frame" 가정이 코덱에 따라 깨진다 — 이 가정이 암묵적으로 깔린 `count_frames` 같은 함수는 AV1에서 조용히 틀린 값을 낸다.
- MPEG-2/HEVC의 필드 인코딩(interlaced, top/bottom field가 별도 picture)에서도 "frame"과 "picture/field"가 1:1이 아닌데, 이름이 뭉개져 있으면 필드 쌍을 프레임 하나로 합쳐야 하는지 아닌지가 코드에서 드러나지 않는다.

**발생 조건**:
- 여러 코덱/컨테이너를 다루는 코드베이스에서 초기에는 "프레임 하나 = access unit 하나 = 컨테이너 샘플 하나"였던(단순 P-frame-only, progressive 콘텐츠) 가정이 그대로 이름에 남아, 이후 B-frame/필드/scalability/`show_existing_frame` 같은 케이스가 추가되며 깨질 때.

**권장**:
```rust
// bitvue-core/src/units.rs — 각 개념에 별도 이름과 명확한 관계를 부여
pub struct ContainerSample {   // 디먹싱 최소 단위 (컨테이너 관점)
    pub data: Vec<u8>,
    pub dts: Duration,
}

pub struct AccessUnit {        // 코덱 관점: 하나의 출력 픽처에 대응하는 NAL/OBU 묶음
    pub nal_or_obu: Vec<CodedUnit>,
}

pub struct TemporalUnit {      // AV1처럼 access unit 여러 개(스케일러빌리티) 또는
    pub access_units: Vec<AccessUnit>, // show_existing_frame으로 0개 신규 디코드를 가질 수 있는 단위
}

pub struct OutputFrame {       // 실제로 화면에 표시되는 최종 픽처
    pub source: OutputFrameSource, // AccessUnit에서 디코드됨 or TemporalUnit 내 참조 재사용(show_existing)
}
```
- 컨테이너 샘플, access unit, temporal/superframe 단위, 최종 출력 프레임을 각각 별도 타입으로 명명하고, 그 사이 관계(1:1이 아닐 수 있음)를 타입 시그니처로 드러낸다.
- "frame"이라는 범용 단어는 코드베이스에서 의도적으로 피하거나, 반드시 어떤 레벨의 frame인지 접두사(`OutputFrame`, `AccessUnit` 등)로 구분한다.

**탐지 방법**:
- Semantic: `show_existing_frame`이 있는 AV1 픽스처, 필드 인코딩 MPEG-2/H.264 픽스처로 "unit 개수 == output frame 개수" 가정이 깨지는지 테스트.
- Static: 코드베이스 전역에서 `Frame`이라는 이름이 정의된 곳이 몇 군데인지, 각각의 필드가 서로 다른 개념을 담고 있는지 grep + 리뷰.

**예외**:
- 하나의 코덱/컨테이너 조합만 다루고 그 조합에서 "1 sample = 1 access unit = 1 output frame"이 항상 참임이 스펙상 보장된다면(예: 순수 progressive, scalability 없는 baseline profile) 단일 개념으로 단순화해도 무방하다 — 다만 다중 코덱 지원이 목표라면 이 가정은 오래 유지되지 않는다.

**Bitvue 판정**: Confirmed — `bitvue-core`에 서로 다른 필드 구성의 `TimelineFrame` struct가 `stream_state.rs:531`과 `timeline.rs:49` 두 곳에 중복 정의되어 있고, 각 코덱 크레이트가 `AvcFrame`/`HevcFrame`/`Vp9Frame` 등 별도 `*Frame` 타입을 독립적으로 소유해 'Frame'이라는 이름이 컨테이너/코덱/타임라인 레이어마다 다른 개념으로 쓰임.

---

### CODEC-016: 새 코덱 크레이트를 기존 크레이트 복붙으로 생성해 로직이 조용히 drift
**분류**: CODEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-vvc는 bitvue-hevc를 통째로 복사해서 시작 (NAL 구조가 유사하므로)
// bitvue-hevc/src/nal.rs
pub fn parse_nal_header(b: [u8; 2]) -> NalHeader {
    NalHeader { nal_type: (b[0] >> 1) & 0x3F, layer_id: 0, temporal_id: (b[1] & 0x07) - 1 }
}

// bitvue-vvc/src/nal.rs — 복사 후 VVC의 확장 필드(6-bit layer_id 등)를 위해 수정했지만
// 원본 HEVC 크레이트에서 이후 발견된 temporal_id underflow 버그 픽스는 반영되지 않음
pub fn parse_nal_header(b: [u8; 2]) -> NalHeader {
    let layer_id = ((b[0] & 0x01) << 5) | (b[1] >> 3); // VVC용으로 수정됨
    NalHeader { nal_type: (b[0] >> 1) & 0x3F, layer_id, temporal_id: (b[1] & 0x07) - 1 } // 버그가 그대로 복제됨
}
```

**문제**:
- 복사 시점의 버그(`temporal_id: (b[1] & 0x07) - 1`이 0일 때 underflow하는 문제 등)가 새 크레이트에 그대로 복제되고, 원본에서 나중에 고쳐져도 두 크레이트가 독립적으로 관리되므로 전파되지 않는다.
- "HEVC와 VVC의 NAL 헤더가 얼마나 실제로 같은가"에 대한 설계 판단이 문서화되지 않은 채, 복붙 시점의 스냅샷으로만 남아 이후 유지보수자가 두 구현이 왜 다른지(의도적 차이 vs 그냥 안 고침) 구분할 수 없다.
- 코드 리뷰가 "이 크레이트만" 보고 진행되면 원본과의 diff가 리뷰 대상이 아니므로, drift가 몇 년간 감지되지 않고 누적된다.

**발생 조건**:
- 신택스가 실제로 상당히 유사한 코덱 쌍(HEVC/VVC, VP9/AV1의 일부 구조, MPEG-2/AVC의 일부 헤더)에서 새 크레이트를 "빈 데서 시작하기 아까워서" 복사로 시작할 때, 특히 마감 압박이 있을 때 흔하다.

**권장**:
```rust
// 진짜 공유 가능한 부분만 bitvue-codecs 공통 모듈로 추출
// bitvue-codecs/src/nal_common.rs
pub fn read_temporal_id(low_nibble: u8) -> Result<u8, NalHeaderError> {
    low_nibble.checked_sub(1).ok_or(NalHeaderError::ZeroTemporalIdPlus1)
}

// bitvue-hevc/src/nal.rs, bitvue-vvc/src/nal.rs 둘 다 이 함수를 사용
// 코덱 고유 비트 배치(layer_id 폭 등)만 각자 구현
```
- 두 코덱이 "당장 비슷해 보인다"는 이유로 파일 전체를 복사하지 말고, 실제로 스펙 레벨에서 동일하게 유지될 것으로 기대되는 최소 단위(예: temporal_id 파싱 규칙)만 공통 함수로 추출한다.
- 부득이 복사로 시작했다면 원본 크레이트를 가리키는 추적 이슈/주석(`// forked from bitvue-hevc nal.rs @ <commit>, diverged for 6-bit layer_id`)을 남겨 향후 버그 픽스 이식 여부를 판단할 근거를 남긴다.

**탐지 방법**:
- Structural: 코덱 크레이트 간 파일 단위 유사도 검사(`cargo-similar`, 혹은 단순 `diff`)를 CI에 주기적으로 돌려 유사도가 높은 파일 쌍을 리포트.
- Manual: 새 코덱 크레이트 PR 리뷰 시 "이 파일이 다른 크레이트에서 복사되었는가"를 체크리스트 항목으로 명시.

**예외**:
- 코덱 신택스가 스펙상 우연히 비슷해 보일 뿐 독립적으로 진화할 것으로 예상된다면(예: 서로 다른 표준화 기구), 처음부터 공통화를 시도하지 않고 각자 구현하는 편이 오히려 더 정직한 설계다.

**Bitvue 판정**: Suspected — `bitvue-hevc/src/nal.rs`와 `bitvue-vvc/src/nal.rs`가 구조적으로 유사한 NAL 헤더 파서를 각자 독립 구현하고 있고(공유 모듈 없음, forked-from 추적 주석도 없음) 드리프트 위험은 구조적으로 존재하나, 두 구현 모두 `nuh_temporal_id_plus1.saturating_sub(1)`로 동일하게 언더플로를 안전하게 처리하고 있어 실제 드리프트 버그는 확인되지 않음.

---

### CODEC-017: superframe/temporal-unit grouping 로직이 컨테이너마다 중복 구현
**분류**: CODEC · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-formats/src/ivf.rs — IVF 컨테이너에서 VP9 superframe을 분해
fn split_vp9_superframe(packet: &[u8]) -> Vec<&[u8]> {
    let marker = packet[packet.len() - 1];
    if marker & 0xE0 != 0xC0 { return vec![packet]; }
    // ... superframe index 파싱, 서브프레임 분해 로직 40줄
}

// bitvue-formats/src/mkv.rs — WebM(MKV) 컨테이너에서도 동일한 VP9 superframe 분해가
// 거의 동일한 코드로 다시 구현되어 있음 (복붙 후 살짝 다름)
fn extract_vp9_frames_from_block(block: &[u8]) -> Vec<&[u8]> {
    let marker = block[block.len() - 1];
    if (marker & 0xE0) != 0xC0 { return vec![block]; } // 원본과 스타일만 다르고 로직은 동일
    // ... 거의 동일한 40줄이 다시 존재, 이번엔 off-by-one 버그가 있음
}
```

**문제**:
- Superframe(VP9)/temporal-unit(AV1) 분해는 순수하게 코덱 신택스 규칙인데, 이를 컨테이너 파서(IVF, MKV)마다 재구현하면 CODEC-001(컨테이너가 코덱을 이해)과 CODEC-016(중복으로 인한 drift)이 동시에 발생한다.
- 위 예시처럼 한쪽 구현에만 off-by-one 버그가 있으면, "IVF로 열면 정상인데 MKV로 열면 마지막 서브프레임이 잘린다"는 컨테이너 종속적 버그가 생겨 원인 추적이 오래 걸린다.
- TS 컨테이너에 향후 VP9/AV1을 추가로 지원하게 되면 세 번째 복제본이 또 생길 위험이 있다.

**발생 조건**:
- 컨테이너 파서를 담당하는 사람이 다르고, "이 컨테이너에서 이 코덱 꺼내려면 superframe도 풀어야 한다"는 요구사항을 그때그때 국소적으로 해결할 때.

**권장**:
```rust
// bitvue-vp9/src/superframe.rs — superframe 분해는 코덱 크레이트의 공개 API
pub fn split_superframe(packet: &[u8]) -> Vec<&[u8]> {
    let marker = packet[packet.len() - 1];
    if marker & 0xE0 != 0xC0 { return vec![packet]; }
    // ... 단일 구현
}

// bitvue-formats/src/ivf.rs, bitvue-formats/src/mkv.rs
use bitvue_vp9::superframe::split_superframe;
fn frames_from_packet(packet: &[u8]) -> Vec<&[u8]> {
    bitvue_vp9::superframe::split_superframe(packet)
}
```
- Superframe/temporal-unit 분해처럼 "코덱 신택스 규칙이지만 컨테이너 경계에서 호출되어야 하는" 로직은 코덱 크레이트가 공개 함수로 제공하고, 모든 컨테이너 파서가 그 함수를 호출만 하게 한다.
- 이는 CODEC-001과 모순되지 않는다 — 컨테이너가 "규칙을 아는 것"과 "코덱이 제공하는 함수를 호출하는 것"은 다르다.

**탐지 방법**:
- Structural: `superframe`, `temporal_unit`, `show_existing_frame` 등 코덱 고유 grouping 키워드가 `bitvue-formats` 내 여러 파일(ivf.rs, mkv.rs, ts.rs)에 중복 등장하는지 grep.
- Manual: 동일 코덱을 서로 다른 컨테이너로 감싼 동일 픽스처로 프레임 분해 결과가 100% 일치하는지 교차 테스트.

**예외**:
- 컨테이너별로 superframe 표현 규칙 자체가 다르다면(예: 어떤 컨테이너는 이미 분해된 상태로 저장) 그 컨테이너 고유의 재조립 로직은 정당하게 컨테이너 레이어에 있어야 한다 — 다만 "분해된 이후 조각 하나를 해석하는" 코덱 로직 자체는 여전히 코덱 크레이트에 있어야 한다.

**Bitvue 판정**: N/A — VP9 superframe 분해는 `bitvue-vp9/src/superframe.rs`가 단독 소유하며, `bitvue-formats`(ivf.rs/mkv.rs)에 중복 구현된 사례를 찾지 못해 권장 패턴을 따르고 있음.

---

### CODEC-018: codec capability flag가 모델링되지 않아 문자열/버전 비교로 임시 감지
**분류**: CODEC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
// bitvue-decode/src/features.rs
pub fn supports_film_grain(codec_name: &str, profile: &str) -> bool {
    // "AV1이면서 profile이 이거면 film grain 지원한다고 치자"는 식의 임시 추정
    codec_name == "av1" && (profile == "main" || profile == "high")
}

pub fn supports_scalability(codec_name: &str) -> bool {
    // HEVC의 SHVC, VVC의 다중 레이어, AV1의 operating point를
    // "코덱 이름이 vvc거나 av1이면 true"라는 식으로 뭉뚱그려 판단
    matches!(codec_name, "vvc" | "av1")
}
```

**문제**:
- "이 코덱 인스턴스가 film grain을 지원하는가"는 실제로는 시퀀스 헤더/SPS의 특정 신택스 요소(예: AV1 `film_grain_params_present`, HEVC의 관련 SEI 존재 여부)로 스트림별로 결정되는데, 이를 코덱 이름/프로파일 문자열만 보고 추정하면 실제 스트림 내용과 어긋나는 오탐/미탐이 발생한다.
- 새 capability(예: AV3의 특정 도구)가 추가될 때마다 이런 임시 함수가 계속 늘어나고, 함수마다 판단 기준(이름 vs 프로파일 vs 버전)이 제각각이라 일관성이 없다.
- UI가 이 capability 함수를 신뢰해서 "이 스트림은 film grain을 지원하니 관련 패널을 보여준다"처럼 분기하면, 실제로는 해당 스트림에 film grain 파라미터가 없는데도 패널이 나타나는 버그로 이어진다.

**발생 조건**:
- 특정 기능(오버레이 패널, 통계 뷰)을 "코덱이 이 기능을 지원하는지"에 따라 켜고 꺼야 하는데, 파싱된 신택스를 직접 조회하는 API가 없어서 이름 기반으로 즉흥적으로 판단할 때.

**권장**:
```rust
// bitvue-core/src/capabilities.rs — capability를 신택스에서 유도되는 명시적 모델로 정의
#[derive(Default)]
pub struct StreamCapabilities {
    pub film_grain: bool,
    pub scalability: Option<ScalabilityInfo>,
    pub lossless: bool,
}

// bitvue-av1/src/sequence_header.rs — 코덱 크레이트가 실제 신택스에서 capability를 유도
impl Av1SequenceHeader {
    pub fn capabilities(&self) -> StreamCapabilities {
        StreamCapabilities {
            film_grain: self.film_grain_params_present,
            scalability: self.operating_points.len().gt(&1).then(|| self.scalability_info()),
            lossless: self.coded_lossless,
        }
    }
}
```
- Capability는 코덱 이름이나 프로파일 문자열의 추정이 아니라, 실제로 파싱된 시퀀스/파라미터 셋의 신택스 요소로부터 코덱 크레이트가 직접 계산해서 제공한다.
- 공통 `StreamCapabilities` 구조체를 `bitvue-core`에 정의하고, 각 코덱 크레이트가 이를 채우는 방식으로 CODEC-004/005와 같은 "얕은 공통 뷰" 패턴을 재사용한다.

**탐지 방법**:
- Static: `codec_name ==`/`profile ==` 문자열 비교로 boolean capability를 리턴하는 함수 grep.
- Semantic: 실제로 film grain 파라미터가 없는 AV1 스트림과 있는 스트림 픽스처로 capability 함수 결과를 교차 검증.

**예외**:
- 파싱 없이 컨테이너 메타데이터(코덱 태그, 프로파일 박스)만으로도 스펙상 100% 결정되는 성질(예: "이 프로파일 자체가 특정 도구를 아예 배제한다"는 스펙 레벨 제약)이라면 이름/프로파일 기반 판단이 정확할 수 있다 — 이 경우도 그 근거를 주석으로 스펙 조항과 함께 남긴다.

**Bitvue 판정**: N/A — `has_film_grain`(`bitvue-core/src/frame_identity.rs`)과 `film_grain_params_present`(`bitvue-av1-codec/src/sequence.rs:301`)는 실제 파싱된 신택스 값에서 유도되며, `FrameExtractor` capability 메서드도 코덱별 trait impl로 구현되어 있어 코덱 이름/프로파일 문자열 추정 패턴은 발견되지 않음.

---

### CODEC-019: multi-track/multi-layer 스트림이 single-track 가정으로 데이터 모델 깊숙이 붕괴
**분류**: CODEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-core/src/session.rs — 분석 세션 하나 = 트랙 하나라는 가정이 최상위부터 박혀 있음
pub struct AnalysisSession {
    pub track: Track,           // 단수형 — SHVC의 base+enhancement layer,
    pub timeline: Vec<Frame>,   // VVC의 여러 OLS(Output Layer Set),
    pub current_frame: usize,   // AV1의 여러 operating point를 표현할 자리가 아예 없음
}

// bitvue-hevc/src/shvc.rs에서 enhancement layer를 지원하려 할 때
impl AnalysisSession {
    pub fn switch_to_enhancement_layer(&mut self) {
        // Track이 하나뿐이라 "레이어 전환"이 곧 "트랙 전체 교체"가 되어버림
        self.track = load_enhancement_layer_as_new_track(); // base layer 정보를 잃어버림
    }
}
```

**문제**:
- `AnalysisSession`이 트랙 하나를 전제로 설계되어 있어서, SHVC(base+enhancement), VVC의 다중 OLS, AV1의 다중 operating point처럼 "논리적으로 여러 서브스트림이 하나의 파일 안에 공존"하는 구조를 표현할 자리가 없다.
- Enhancement layer를 "새 트랙으로 통째로 교체"하는 식의 임시방편은 base layer와 enhancement layer를 동시에 비교/오버레이해야 하는(스케일러블 코덱 분석의 핵심 유스케이스) 기능 자체를 원천적으로 막는다.
- 이 가정이 세션 최상위 struct에 박혀 있으면, 나중에 멀티 레이어 지원을 추가할 때 UI부터 상태 관리, IPC 커맨드까지 전 계층을 다시 설계해야 하는 큰 리팩터링이 된다.

**발생 조건**:
- 초기 지원 코덱(AVC, 단일 레이어 HEVC)이 모두 single-layer였을 때 설계된 세션 모델을, 이후 SHVC/VVC 다중 레이어나 AV1 다중 operating point 지원을 추가하면서 그대로 재사용하려 할 때.

**권장**:
```rust
// bitvue-core/src/session.rs — 처음부터 "1개 이상의 서브스트림"을 전제로 모델링
pub struct AnalysisSession {
    pub substreams: Vec<Substream>, // 단일 레이어 코덱도 substreams.len() == 1로 표현
    pub active_substream: SubstreamId,
}

pub struct Substream {
    pub id: SubstreamId,
    pub kind: SubstreamKind, // Layer(layer_id) | OperatingPoint(idx) | Track(track_id)
    pub timeline: Vec<Frame>,
}
```
- "트랙/레이어/오퍼레이팅 포인트가 여러 개일 수 있다"를 처음부터 컬렉션 타입(`Vec<Substream>`)으로 모델링하고, 단일 레이어 코덱은 그저 길이 1인 특수 케이스로 자연스럽게 처리되게 한다.
- UI 상태(현재 보고 있는 레이어)는 세션 구조와 분리해서 "여러 substream 중 무엇이 활성인가"로 표현하고, base/enhancement를 동시에 비교하는 뷰도 이 모델 위에서 자연스럽게 구성 가능하게 한다.

**탐지 방법**:
- Structural: 최상위 세션/상태 관리 타입에 단수형 필드(`track: Track`, `layer: Layer`)가 있는지, 코덱 지원 로드맵에 SHVC/VVC 다중 레이어/AV1 operating point가 있는지 대조.
- Manual: "이 파일이 레이어를 2개 가지고 있다면 이 코드는 어떻게 동작하는가"를 설계 리뷰 질문으로 명시적으로 던진다.

**예외**:
- 도구의 스코프가 명시적으로 "단일 레이어/단일 트랙 분석"으로 한정되어 있고 로드맵에도 다중 레이어 지원이 없다면, 이 복잡도를 미리 감당할 필요는 없다 — 다만 이 경우 "다중 레이어는 지원 범위 밖"이라는 결정을 문서화해 향후 오해를 막는다.

**Bitvue 판정**: N/A — SHVC/VVC 다중 레이어, AV1 다중 operating point 관련 개념(`Substream`/`OperatingPoint`/`EnhancementLayer`/`SHVC`)이 코드베이스 전체에 전혀 존재하지 않아, 아직 깨질 '단일 트랙 가정'조차 없음(범위 밖 기능이며 이 결정이 문서화되어 있는지는 별도 확인 필요).

---

### CODEC-020: container seek index와 codec random access point 개념 혼동
**분류**: CODEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// bitvue-formats/src/mp4/seek.rs
pub fn find_seek_point(stss: &Stss, target_sample: u32) -> u32 {
    // MP4 sync sample table(stss)에 있는 샘플이면 무조건 "여기서부터 디코딩 가능"으로 간주
    stss.sync_samples.iter()
        .filter(|&&s| s <= target_sample)
        .max().copied().unwrap_or(0)
}

// 호출부: 이 결과를 그대로 코덱 레벨 랜덤 액세스 지점으로 사용
fn seek_and_decode(target: u32) {
    let sync = find_seek_point(&stss, target);
    decode_from(sync); // HEVC의 CRA(CLVSS 아닌 경우 RASL 문제) 같은 코덱별 세부사항을 무시
}
```

**문제**:
- MP4의 sync sample(stss)이나 MKV의 CuePoint는 "컨테이너가 생각하는 랜덤 액세스 지점"일 뿐이고, 실제로 그 지점부터 정확히 디코딩 가능한지는 코덱 레벨 개념(HEVC의 IRAP 종류 — IDR/CRA/BLA, AVC의 recovery point SEI, AV1의 `frame_type == KEY_FRAME` 여부)에 달려 있는데, 이를 그대로 동일시하면 CRA 프레임에서 seek했을 때 RASL 픽처를 잘못 디코딩/표시하는 문제가 생긴다.
- 인코더가 sync sample 플래그를 부정확하게 찍은 파일(실무에서 드물지 않음)에서는 컨테이너 인덱스와 코덱 실제 상태가 어긋나는데, 컨테이너 인덱스를 맹신하면 디코드 실패나 아티팩트로 이어진다.
- Open-GOP(CRA + RASL) 구조를 분석 도구가 표현하지 못하면, "이 지점부터 seek 가능"이라는 UI 힌트가 사실과 다르게 표시되어 사용자가 잘못된 프레임에서 분석을 시작하게 된다.

**발생 조건**:
- Closed-GOP(IDR-only) 테스트 콘텐츠로만 검증했을 때 특히 잘 숨는다 — Open-GOP/CRA 콘텐츠에서만 컨테이너 인덱스와 코덱 실제 랜덤 액세스 지점의 괴리가 드러난다.

**권장**:
```rust
// bitvue-core/src/random_access.rs
pub struct SeekCandidate {
    pub container_sync_sample: bool,      // 컨테이너가 주장하는 sync 여부
    pub codec_access_point: Option<RandomAccessKind>, // 코덱이 실제로 파싱해서 확인한 종류
}

pub enum RandomAccessKind { Idr, Cra, Bla, RecoveryPoint { frame_cnt: u16 } }

pub fn resolve_seek_target(container_hint: u32, parsed_units: &[ParsedUnit]) -> SeekDecision {
    // 컨테이너 힌트는 "탐색 시작 후보"일 뿐, 최종 판단은 코덱 파서가 실제 유닛을 열어 확인
    let unit = &parsed_units[container_hint as usize];
    match unit.random_access_kind() {
        Some(RandomAccessKind::Cra) => SeekDecision::StartWithRaslCaveat,
        Some(RandomAccessKind::Idr) => SeekDecision::CleanStart,
        _ => SeekDecision::FallBackToPreviousIdr,
    }
}
```
- 컨테이너 인덱스는 "탐색을 어디서 시작해볼지"에 대한 힌트로만 쓰고, 실제로 그 지점이 클린 랜덤 액세스 지점인지는 반드시 코덱 파서가 해당 유닛을 열어 확인하게 한다.
- IRAP 종류(IDR/CRA/BLA)별로 다른 처리(RASL 폐기 여부 등)를 명시적인 enum으로 표현해 "컨테이너가 sync라고 했다 = 코덱이 깨끗하게 디코딩된다"는 암묵적 동일시를 코드에서 제거한다.

**탐지 방법**:
- Semantic: Open-GOP(CRA 포함) 픽스처로 seek 후 첫 프레임들이 올바르게(또는 명시적으로 RASL 주의 표시와 함께) 처리되는지 테스트.
- Manual: seek 관련 코드에서 `stss`/`CuePoint` 조회 결과를 코덱 파서 확인 없이 바로 "디코딩 가능 지점"으로 취급하는 곳 리뷰.

**예외**:
- 컨테이너가 sync sample을 신뢰할 수 있게 생성한다고 보장되는 폐쇄된 파이프라인(자체 인코더로만 생성된 파일만 다루는 경우)이라면 이 이중 검증을 생략하는 실용적 타협이 가능하다 — 다만 범용 분석 도구라면 이 가정이 위험하다.

**Bitvue 판정**: Confirmed — `bitvue-core/src/indexing.rs`의 `SeekPoint`가 IRAP 종류 구분 없는 단일 `is_keyframe: bool` 필드만 가지며, `bitvue-formats/src/ts.rs`의 `is_hevc_keyframe`은 BLA/IDR/CRA(16..=23)를 RASL 구분 없이 동일하게 키프레임으로 취급하고, MP4 경로(`mp4.rs` key_frames)도 `stss`를 코덱 레벨 검증 없이 그대로 신뢰함.

---

### CODEC-021: bitvue-codecs가 bitvue-formats에 역의존하는 레이어 역전
**분류**: CODEC · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```toml
# bitvue-codecs/Cargo.toml
[dependencies]
bitvue-formats = { path = "../bitvue-formats" }  # 워크스페이스 레이어 순서를 거스르는 의존성
```
```rust
// bitvue-hevc/src/extradata.rs
use bitvue_formats::mp4::HvccBox; // 코덱 크레이트가 MP4 박스 타입을 직접 참조

pub fn parse_from_extradata(hvcc: &HvccBox) -> HevcSps {
    // HEVC 파서가 "MP4의 hvcC 박스"라는 컨테이너 특정 포맷을 직접 안다
    parse_sps(&hvcc.sps_data)
}
```

**문제**:
- 의도된 레이어(`bitvue-formats` → `bitvue-codecs` → 코덱별 크레이트)가 역전되어, HEVC 파서 하나가 MP4 특정 박스 구조에 의존하게 되므로 MKV/TS 컨테이너 안의 HEVC를 파싱할 때도 같은 함수가 재사용 불가능해진다.
- Cargo 워크스페이스에서 순환 의존은 컴파일 자체가 안 되므로 즉시 드러나지만, 순환까지는 아니어도 "역방향" 의존은 컴파일은 되면서 아키텍처만 조용히 망가뜨려 탐지가 늦어진다.
- 코덱 크레이트를 독립 배포(예: 다른 프로젝트에서 `bitvue-hevc`만 가져다 쓰기)하려 할 때 불필요하게 `bitvue-formats` 전체가 딸려온다.

**발생 조건**:
- "extradata(hvcC/avcC 등)에서 SPS/PPS를 꺼내야 한다"는 실용적 필요를 가장 가까운 곳(컨테이너 타입을 그냥 import)에서 해결할 때.

**권장**:
```rust
// bitvue-hevc/src/extradata.rs — 코덱 크레이트는 "바이트 슬라이스"만 안다
pub fn parse_sps_from_hvcc_bytes(raw: &[u8]) -> HevcSps {
    // hvcC의 바이트 레이아웃 자체는 실제로 코덱 표준 부속서가 정의하므로
    // 코덱 크레이트가 이 포맷을 아는 것은 정당하지만, MP4 박스 타입에 의존하진 않는다
    parse_sps(&extract_sps_nal(raw))
}

// bitvue-formats/src/mp4/hvcc.rs — 컨테이너가 박스를 파싱해 바이트를 넘겨줌
use bitvue_hevc::extradata::parse_sps_from_hvcc_bytes;
pub fn decode_hvcc_box(box_bytes: &[u8]) -> HevcSps {
    parse_sps_from_hvcc_bytes(box_bytes)
}
```
- 워크스페이스 의존 방향을 `cargo metadata`로 CI에서 강제한다: `bitvue-codecs`/코덱별 크레이트가 `bitvue-formats`를 dependencies에 절대 갖지 않도록 lint.
- extradata처럼 "코덱 표준이 자체적으로 정의하는 바이트 레이아웃"(hvcC/avcC는 실제로 ISO/IEC 14496-15가 정의)은 코덱 크레이트가 알아도 되지만, 그 지식은 "MP4 박스 구조체"가 아니라 "바이트 슬라이스 → 파싱 결과" 함수로 노출해 컨테이너 타입 의존을 끊는다.

**탐지 방법**:
- Structural: `cargo tree -p bitvue-hevc -i bitvue-formats` (역방향 확인) 또는 워크스페이스 의존성 그래프를 CI에서 정적으로 검사해 금지된 엣지가 있으면 실패.
- Static: 코덱 크레이트 소스에서 `use bitvue_formats::` import grep.

**예외**:
- 없음 — 레이어 역전은 아키텍처 규칙 위반이며 예외를 두면 워크스페이스 전체 레이어링이 무의미해진다. 정말 공유가 필요한 타입이 있다면 더 하위의 공통 크레이트(`bitvue-core`)로 내려야 한다.

**Bitvue 판정**: Confirmed — `crates/bitvue-av1-codec/Cargo.toml`이 `bitvue-formats`에 의존하고, `bitvue-av1-codec/src/lib.rs:194-243`가 `bitvue_formats::mp4/mkv/ts::extract_av1_samples`를 직접 호출해 코덱 크레이트가 컨테이너 크레이트를 참조하는 레이어 역전이 실재함(트랜지티브하게 `bitvue-codecs`도 영향받음).

---

### CODEC-022: NAL/OBU 등 코덱 고유 delimiter 파싱을 컨테이너 레이어에서 수행
**분류**: CODEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-formats/src/ts.rs (MPEG-2 TS 컨테이너 파서)
fn extract_pes_payload_units(pes_payload: &[u8]) -> Vec<&[u8]> {
    // TS 컨테이너 파서가 Annex-B start code(0x000001)를 직접 스캔해서
    // NAL 유닛 경계를 자르는 로직을 갖고 있다 — 이건 코덱(AVC/HEVC) 신택스 지식이다
    let mut units = Vec::new();
    let mut i = 0;
    while i + 3 < pes_payload.len() {
        if &pes_payload[i..i+3] == [0, 0, 1] {
            let start = i + 3;
            let end = find_next_start_code(pes_payload, start).unwrap_or(pes_payload.len());
            units.push(&pes_payload[start..end]);
            i = end;
        } else { i += 1; }
    }
    units
}
```

**문제**:
- Annex-B start code 스캔은 "AVC/HEVC 스트림이 어떻게 자기 자신을 구분하는가"에 대한 코덱 표준 규칙인데, 이를 TS 컨테이너 파서가 직접 구현하면 CODEC-001과 동일한 레이어 위반이 발생한다 — 같은 로직이 MKV나 raw Annex-B 파일 입력 경로에도 각각 또 구현될 위험(CODEC-017과 동일한 패턴)이 있다.
- Emulation prevention byte(`0x03`) 처리를 빠뜨리기 쉬운 위치(컨테이너 레이어는 이 규칙에 익숙하지 않은 사람이 작성하는 경우가 많음)라, `00 00 03 01`처럼 이스케이프된 바이트를 실제 start code로 오인하는 미묘한 파싱 버그가 생기기 쉽다.
- TS/PES 레벨에서 이미 access unit 경계를 알 수 있는 다른 신호(PES 패킷 경계 자체)가 있는데도 굳이 코덱 레벨 delimiter를 재구현하면서 정보를 중복 사용하게 된다.

**발생 조건**:
- TS처럼 PES 페이로드 안에 Annex-B 스트림이 그대로 들어있는 컨테이너를 지원할 때, "여기서 NAL 단위로 잘라야 다음 단계로 넘길 수 있다"는 필요를 컨테이너 파서 내부에서 즉석으로 해결할 때.

**권장**:
```rust
// bitvue-codecs/src/annexb.rs — Annex-B start code 스캔은 코덱 계층 공통 유틸
pub fn split_annex_b_units(data: &[u8]) -> Vec<&[u8]> {
    // emulation prevention까지 정확히 처리하는 단일 구현 (CODEC-010과 동일 원칙)
    scan_start_codes(data)
}

// bitvue-formats/src/ts.rs
use bitvue_codecs::annexb::split_annex_b_units;
fn extract_pes_payload_units(pes_payload: &[u8]) -> Vec<&[u8]> {
    split_annex_b_units(pes_payload) // 컨테이너는 호출만 한다
}
```
- Start code/OBU 헤더 스캔처럼 "코덱이 자신을 어떻게 구분하는가"에 대한 규칙은 `bitvue-codecs`(코덱군 공통) 또는 개별 코덱 크레이트가 소유하고, 모든 컨테이너 파서는 이를 함수 호출로만 사용한다.
- 이는 CODEC-017(superframe 분해 중복)과 본질적으로 같은 원칙의 다른 사례이므로, 두 규칙을 같은 리뷰 체크리스트 항목("컨테이너 파서에 코덱 신택스 상수/로직이 새로 추가되지 않았는가")으로 묶어 관리한다.

**탐지 방법**:
- Static: `bitvue-formats` 내에서 `0x000001`, `emulation_prevention`, `start_code` 같은 코덱 고유 리터럴/용어 grep.
- Structural: 동일한 start-code 스캔 로직이 `bitvue-formats`와 `bitvue-codecs` 양쪽에 존재하는지 대조.

**예외**:
- 컨테이너 포맷 자체의 스펙이 델리미터 스캔을 컨테이너 레벨에서 요구하는 극히 드문 경우(예: 컨테이너가 자체적으로 별도의 스타트코드 규약을 정의)라면 예외일 수 있으나, TS/MP4/MKV의 일반적인 코덱 임베딩에는 해당하지 않는다.

**Bitvue 판정**: Confirmed — `bitvue-formats/src/ts.rs:421`의 `split_annex_b_nal_units`가 TS 컨테이너 파서 내부에서 Annex-B start code 스캔을 직접 구현하고 있으며, 공유 `bitvue-codecs` 유틸리티로 위임하지 않음.

---

### CODEC-023: Annex-B vs length-prefixed 같은 컨테이너별 byte-stream 변형을 코덱 파서가 알아야 하는 구조
**분류**: CODEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// bitvue-hevc/src/parser.rs
pub fn parse_stream(data: &[u8], is_mp4: bool) -> Vec<NalUnit> {
    // 코덱 파서 함수 시그니처에 "이게 MP4냐 아니냐"라는 컨테이너 지식이 파라미터로 새어 들어옴
    if is_mp4 {
        parse_length_prefixed(data, 4) // hvcC의 NALUnitLength 필드 크기(보통 4, 설정 가능)
    } else {
        parse_annex_b(data) // start code 기반
    }
}
```

**문제**:
- HEVC 파서 함수가 "MP4냐 아니냐"라는 boolean을 받는 순간, 실제로는 length-prefix 크기가 1/2/4바이트로 가변인 MP4의 `lengthSizeMinusOne` 설정이나 MKV의 다른 관례를 전혀 표현하지 못하는 이분법적 설계가 되어버린다.
- 새로운 byte-stream 변형(예: 특정 컨테이너가 2바이트 length prefix를 쓰는 경우)이 생기면 이 boolean 파라미터로는 대응이 안 되어 결국 파라미터가 enum으로, 또 여러 개로 늘어나며 함수 시그니처가 계속 불어난다.
- "바이트 스트림을 개별 NAL 유닛으로 나누는 방식"(컨테이너 관심사)과 "NAL 유닛 하나를 신택스로 파싱하는 방식"(코덱 관심사)이 한 함수에 뒤섞여, 후자만 테스트하고 싶어도 항상 전자의 선택지를 함께 고려해야 한다.

**발생 조건**:
- MP4(length-prefixed)와 raw Annex-B(start-code) 양쪽을 지원해야 하는 코덱(AVC/HEVC/VVC)에서, "일단 분기 하나 추가해서 둘 다 되게 하자"는 식으로 코덱 파서 내부에 조건문을 심을 때.

**권장**:
```rust
// bitvue-codecs/src/byte_stream.rs — "유닛 분리 전략"을 코덱 파서와 분리된 관심사로 모델링
pub enum UnitFraming {
    AnnexB,
    LengthPrefixed { length_size: u8 }, // 1/2/4 바이트 모두 표현 가능
}

pub fn split_units(data: &[u8], framing: UnitFraming) -> Vec<&[u8]> {
    match framing {
        UnitFraming::AnnexB => split_annex_b_units(data),
        UnitFraming::LengthPrefixed { length_size } => split_length_prefixed(data, length_size),
    }
}

// bitvue-hevc/src/parser.rs — 코덱 파서는 "이미 나뉜 NAL 유닛"만 받는다, framing을 모른다
pub fn parse_nal_unit(unit: &[u8]) -> NalUnit { /* framing에 대한 지식 전혀 없음 */ }
```
- "바이트 스트림을 유닛으로 나누는" 책임을 `UnitFraming`이라는 명시적 값으로 분리해, 코덱 파서 자체는 항상 "이미 잘린 개별 유닛"만 받도록 시그니처를 고정한다.
- length-prefix 크기 같은 컨테이너별 설정값(MP4의 `lengthSizeMinusOne`)은 컨테이너 파서가 읽어 `UnitFraming`을 구성하는 재료로만 쓰고, 코덱 파서 시그니처에 boolean/컨테이너 이름으로 새어 들어가지 않게 한다.

**탐지 방법**:
- Static: 코덱 파서 함수 시그니처에 `is_mp4`, `is_annexb`, `container: &str` 같은 파라미터가 있는지 grep.
- Structural: 코덱 파서 함수가 호출하는 분기 수가 지원 컨테이너 수에 비례해서 늘어나는지 이력 추적.

**예외**:
- 코덱 자체가 표준으로 두 가지 이하의 고정된 byte-stream 포맷만 가지며 향후 변형이 늘어날 가능성이 없다고 확신할 수 있다면 간단한 enum 파라미터 정도는 실용적 타협으로 남을 수 있다 — 다만 이 경우도 "framing"과 "신택스 파싱"의 함수 분리는 유지하는 것이 좋다.

**Bitvue 판정**: N/A — `bitvue-hevc`/`bitvue-avc`/`bitvue-vvc` 파서 함수 시그니처에서 `is_mp4`/`is_annexb`/`container: &str` 류 파라미터를 찾지 못함; byte-stream framing 분리가 코덱 크레이트 밖에서 처리되는 것으로 보임.

---

### CODEC-024: parameter set(SPS/PPS/VPS 등) 참조 해석을 파싱 시점에 즉시 수행해 순서 의존성 발생
**분류**: CODEC · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
// bitvue-hevc/src/pps.rs
pub fn parse_pps(data: &[u8], sps_table: &HashMap<u8, HevcSps>) -> HevcPps {
    let sps_id = read_ue(data);
    // PPS를 파싱하는 그 순간 참조하는 SPS가 반드시 이미 테이블에 있어야 한다고 가정
    let sps = sps_table.get(&sps_id)
        .expect("SPS must be parsed before PPS"); // 스트리밍/부분 파일/역순 등장 시 즉시 panic
    HevcPps { sps_id, /* sps 필드를 참조해 즉시 유도 계산된 필드들 */ }
}
```

**문제**:
- 실제 스트림에서는 PPS가 참조하는 SPS가 아직 도착하지 않았거나(스트리밍 중간부터 분석 시작), 파일 일부만 잘라서 분석하는 경우 SPS가 아예 없을 수도 있는데, 파싱 시점에 즉시 참조를 강제로 해석하면 이런 정상적인 상황에서도 panic/에러로 이어진다.
- "파싱"(신택스 트리를 만드는 것)과 "참조 해석"(다른 파라미터 셋 값에 의존하는 필드를 계산하는 것)이 한 단계에 뭉쳐 있으면, "SPS 없이 PPS 구조만 보고 싶다"는 분석 도구의 흔한 요구(예: 손상된 파일에서 최대한 정보를 뽑아내는 diagnostic 모드)를 충족할 수 없다.
- 파라미터 셋 갱신(같은 id로 SPS가 스트림 중간에 다시 등장, MP4의 `hvcC` extradata에 있는 것과 다를 경우)이 있는 스트림에서, "파싱 시점에 즉시 바인딩"하면 이후 갱신을 반영하지 못하는 stale reference 버그도 생긴다.

**발생 조건**:
- 항상 SPS→PPS→Slice 순서로 깨끗하게 도착하는 잘 만들어진 파일로만 테스트했을 때, 그리고 파일 일부만 로드하는 "빠른 미리보기"나 스트리밍 분석 기능을 나중에 추가할 때 드러난다.

**권장**:
```rust
// bitvue-hevc/src/pps.rs — 파싱은 참조를 "지연된 핸들"로만 남기고 즉시 해석하지 않는다
pub struct HevcPps {
    pub sps_id: u8,          // 참조 id만 보존, 아직 해석하지 않음
    pub raw_fields: RawPpsFields,
}

pub fn parse_pps(data: &[u8]) -> HevcPps {
    let sps_id = read_ue(data);
    HevcPps { sps_id, raw_fields: parse_raw_fields(data) } // SPS 테이블 불필요
}

// bitvue-decode/src/resolve.rs — 참조 해석은 필요한 시점(디코드 직전, UI 표시 직전)에 별도 단계로
pub fn resolve_pps<'a>(pps: &HevcPps, sps_table: &'a SpsTable) -> Result<ResolvedPps<'a>, RefError> {
    let sps = sps_table.get(pps.sps_id).ok_or(RefError::MissingSps(pps.sps_id))?;
    Ok(ResolvedPps { pps, sps })
}
```
- "구문 파싱"과 "참조 해석"을 별도 단계로 분리해, 파싱은 항상 부분 데이터에 대해서도 실패하지 않게 하고, 참조가 필요한 소비자만 명시적으로 `resolve_*` 단계를 거치며 결측을 `Result`로 다루게 한다.
- 파라미터 셋 테이블은 갱신 가능한 상태로 유지하고, 참조 해석은 "그 시점의 최신 테이블"을 사용하도록 해 stale reference를 방지한다.

**탐지 방법**:
- Runtime: 파일 일부만 잘라 SPS 없이 PPS/Slice부터 시작하는 픽스처로 파서가 panic 없이 부분 결과를 반환하는지 테스트.
- Static: 파싱 함수 시그니처에 다른 파라미터 셋 테이블(`&HashMap<_, Sps>` 등)이 필수 인자로 들어가 있는지, 그 안에서 `.expect(`/`.unwrap()`으로 조회하는지 grep.

**예외**:
- 항상 완전한 파일 전체를 한 번에 메모리에 올려 처음부터 끝까지만 분석하는 배치 전용 도구라면, 2-패스(먼저 모든 파라미터 셋 수집, 이후 해석)를 강제해도 실용적 문제가 없을 수 있다 — 다만 이 경우도 "즉시 panic"보다는 "수집 단계에서 순서를 보장"하는 명시적 설계가 낫다.

**Bitvue 판정**: Confirmed(패닉은 회피) — `bitvue-hevc/src/slice.rs:262,290`의 `parse_slice_header`가 `sps_map`/`pps_map`을 필수 인자로 받아 SPS/PPS 없이는 부분 결과를 낼 수 없는 구조적 결합이 존재함; 다만 `.expect()`/`.unwrap()` 대신 `ok_or_else(HevcError::InvalidData)`로 `Result`를 반환해 문서가 지적한 '즉시 panic' 증상 자체는 회피하고 있음.

---

### CODEC-025: extradata/codec private data 형식을 컨테이너와 코덱이 각자 다르게 해석
**분류**: CODEC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
// bitvue-formats/src/mp4/hvcc.rs — MP4 쪽에서 hvcC를 해석해 SPS/PPS 바이트를 추출
pub fn extract_parameter_sets(hvcc: &[u8]) -> Vec<(u8, Vec<u8>)> {
    // hvcC box를 파싱해 (nal_unit_type, nal_bytes) 목록을 만듦 — length prefix 크기 등 자체 해석
    parse_hvcc_arrays(hvcc)
}

// bitvue-hevc/src/decoder_config.rs — 같은 hvcC 바이트를 코덱 크레이트가 "독자적으로" 다시 해석
pub fn parse_decoder_config(hvcc: &[u8]) -> DecoderConfig {
    // MP4 파서와 별개로 작성된 hvcC 파서 — 필드 오프셋 계산이 미묘하게 다름
    // (예: general_profile_compatibility_flags를 읽는 바이트 순서가 반대)
    parse_hvcc_independently(hvcc)
}
// 두 구현이 하나의 파일에 대해 서로 다른 SPS 바이트를 뽑아내는 경우가 실제로 발생
```

**문제**:
- hvcC/avcC/`VPCC`(VP9) 같은 "codec private data" 포맷은 컨테이너 표준(ISOBMFF 계열)이 정의하지만 내용물은 코덱 신택스이므로, 어느 레이어가 이걸 파싱할지 애매한 경계 지대인데, 이를 컨테이너 크레이트와 코덱 크레이트가 각자 독립적으로 구현하면 CODEC-016/CODEC-010과 같은 drift가 발생하기 쉽다.
- 위 예시처럼 필드 해석이 미묘하게 어긋나면 "같은 파일인데 어느 코드 경로로 여느냐에 따라 SPS 내용이 달라진다"는 재현이 어려운 버그가 생긴다 — 특히 hvcC의 배열 파싱(NAL 개별 배열 개수, `array_completeness` 비트 등)은 세부 규칙이 많아 실수가 잦다.
- 두 구현 중 어느 쪽이 "진실"인지 코드만 봐서는 알 수 없어, 버그 발생 시 두 구현을 나란히 비교하는 디버깅 비용이 추가로 든다.

**발생 조건**:
- extradata 파싱이 필요한 기능(컨테이너 레벨의 코덱 정보 요약 UI, 코덱 레벨의 SPS 상세 분석)이 서로 다른 시점/다른 담당자에 의해 각각 구현될 때.

**권장**:
```rust
// bitvue-hevc/src/decoder_config.rs — hvcC 파싱은 코덱 크레이트가 단일 소유(CODEC-021과 동일 원칙:
// 코덱 표준이 정의하는 바이트 레이아웃이므로 코덱 크레이트가 알아도 되지만, 구현은 하나여야 한다)
pub fn parse_hvcc(raw: &[u8]) -> Result<HvccConfig, HvccError> {
    // 유일한 구현
}

// bitvue-formats/src/mp4/hvcc.rs — 컨테이너는 박스에서 바이트만 꺼내 코덱 크레이트에 위임
pub fn read_hvcc_box(box_bytes: &[u8]) -> Result<HvccConfig, HvccError> {
    bitvue_hevc::decoder_config::parse_hvcc(box_bytes)
}
```
- Extradata 포맷별로 파싱 구현을 정확히 하나만 두고(어느 크레이트가 소유할지는 CODEC-021의 원칙에 따라 "바이트 레이아웃이 코덱 표준에 정의됨"을 근거로 코덱 크레이트가 소유), 컨테이너는 박스에서 바이트를 추출해 넘기는 역할만 한다.
- 같은 원본 바이트에 대해 두 구현이 존재하는지를 주기적으로(코드베이스 전체 검색) 점검해 CODEC-016/017/022와 동일한 패턴이 extradata에서도 재발하지 않게 한다.

**탐지 방법**:
- Semantic: 동일 hvcC/avcC 바이트 픽스처를 두 경로(컨테이너 경유 vs 코덱 크레이트 직접 호출)로 각각 파싱해 결과가 바이트 단위로 일치하는지 회귀 테스트.
- Structural: `hvcc`, `avcc`, `vpcc`, `av1c` 등 extradata 관련 파싱 함수가 `bitvue-formats`와 코덱 크레이트 양쪽에 정의되어 있는지 grep.

**예외**:
- 컨테이너가 필요로 하는 정보가 "코덱 타입 하나 식별" 수준으로 극히 얕다면(예: 첫 바이트의 `configurationVersion`만 읽으면 됨) 전체 파서를 공유할 필요 없이 컨테이너가 그 한 필드만 직접 읽는 것은 실용적이다 — 문제는 SPS/PPS 바이트 추출처럼 실질적인 코덱 신택스 해석까지 중복될 때다.

**Bitvue 판정**: N/A — `hvcC`/`avcC` 등 decoder-config-record(extradata) 파싱 코드가 `bitvue-formats`와 코덱 크레이트 어디에도 존재하지 않음(grep 무결과); MP4/MKV 샘플 추출은 in-band NAL 유닛에 의존하므로 '두 곳에서 각자 해석' 시나리오 자체가 아직 발생할 수 없음.
