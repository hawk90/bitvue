# Anti-Pattern Catalog — LAYOUT: 데이터 레이아웃과 캐시 지역성
이 문서는 더 큰 안티패턴 카탈로그의 일부입니다 (전체 목차는 별도 작성 중인 `docs/anti-patterns/INDEX.md` 참고). 이 파일은 1단계(일반 참조 카탈로그)이며, 2단계에서 Bitvue 저장소를 실제로 감사해 각 항목의 "Bitvue 판정"을 채웁니다.

---

### LAYOUT-001: pointer-heavy syntax tree
**분류**: LAYOUT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// HEVC/AV1류 신택스 트리를 Box/Rc 포인터 체인으로 구성
pub struct SyntaxNode {
    pub kind: NodeKind,
    pub value: i64,
    pub children: Vec<Box<SyntaxNode>>,
    pub parent: Option<*const SyntaxNode>,
}

pub struct Bitstream {
    pub root: Box<SyntaxNode>,
}
```

**문제**:
- `Box<SyntaxNode>` 자식마다 별도 힙 할당 — NALU 하나에 수천 개 노드가 생기면 할당기 호출도 수천 번.
- 트리 순회(예: 특정 syntax element 하나를 찾기 위한 depth-first walk)가 포인터를 따라가며 캐시라인을 매번 새로 로드 — prefetch가 사실상 불가능.
- 노드 크기가 커서(포인터 3~4개 + enum) 캐시라인(64B)에 노드 1개도 다 안 들어가는 경우가 흔함.
- 직렬화/디버깅 시 트리 전체를 힙에서 재구성해야 해서 스냅샷 비용이 큼.

**발생 조건**:
- 4K/8K 프레임의 CTU/CU 트리처럼 노드 수가 수만~수십만 개인 경우.
- "신택스 트리를 파싱하자마자 UI 오버레이로 즉시 순회"하는 hot path에서 특히 체감됨.

**권장**:
```rust
// 노드를 flat arena에 저장하고 자식 관계는 인덱스 범위로 표현
pub struct SyntaxArena {
    pub kinds: Vec<NodeKind>,
    pub values: Vec<i64>,
    // children[i] = (start, end) into `kinds`/`values`
    pub children_range: Vec<(u32, u32)>,
    pub child_ids: Vec<u32>,
}
```
- 파싱 순서(전위 순회 순서)대로 arena에 push하면 순회 시 순차 접근에 가까워짐.
- 부모-자식 관계는 `Vec<u32>` 인덱스로, 포인터 대신 오프셋으로 표현.
- 트리 전체를 한 번에 `Vec::with_capacity`로 예약해 재할당 방지.

**탐지 방법**:
- Structural: `grep -rn "Box<.*Node>\|Rc<.*Node>"` 로 트리형 타입 정의 스캔.
- Runtime: `perf stat -e cache-misses,cache-references` 로 트리 순회 함수의 캐시 미스율 측정, arena 버전과 비교.
- Manual: 노드 수 × `size_of::<SyntaxNode>()` 를 실제 할당 횟수(heaptrack/dhat)와 대조.

**예외**:
- 트리가 파싱 후 즉시 한 번만 순회되고 버려지는 소규모 구조(예: 헤더 몇십 개 필드)라면 포인터 트리도 무방 — arena 전환의 이득이 구현 복잡도를 못 넘음.

**Bitvue 판정**: Confirmed — HEVC/VP9/VVC 신택스 트리는 재귀적 `Vec<SyntaxNode>`로 구성되고(crates/bitvue-hevc/src/syntax/mod.rs:9-24 등), core의 `SyntaxModel`은 `HashMap<String, SyntaxNode>` + String parent/children ID로 트리를 표현(crates/bitvue-core/src/types.rs:514-579) — arena/인덱스 대신 노드별 힙 할당과 문자열 키 조회에 의존.

---

### LAYOUT-002: Array of Structures만 고집
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct MotionVector {
    pub x: i16,
    pub y: i16,
    pub ref_idx: i8,
    pub block_x: u16,
    pub block_y: u16,
    pub block_size: u8,
}

pub struct MvField {
    pub vectors: Vec<MotionVector>, // AoS
}

fn sum_dx(field: &MvField) -> i64 {
    field.vectors.iter().map(|v| v.x as i64).sum()
}
```

**문제**:
- `x` 필드만 필요한 연산(예: 전체 프레임의 평균 MV 크기 계산)인데도 `MotionVector` 전체(11~12바이트 + 패딩)를 캐시라인에 끌고 옴.
- 필드별 SIMD 벡터화가 어려움 — 컴파일러가 non-contiguous `x` 값들을 gather해야 함.
- MV 필드 하나가 프레임당 수만 개면, "x만 훑는" 작업의 유효 대역폭이 구조체 크기에 비례해 낭비됨.

**발생 조건**:
- 오버레이 렌더링, MV 히스토그램, per-axis 통계처럼 특정 필드만 열 단위로 스캔하는 연산이 반복될 때.

**권장**:
```rust
pub struct MvFieldSoA {
    pub x: Vec<i16>,
    pub y: Vec<i16>,
    pub ref_idx: Vec<i8>,
    pub block_x: Vec<u16>,
    pub block_y: Vec<u16>,
    pub block_size: Vec<u8>,
}
```
- 열 단위 접근이 캐시라인을 100% 활용 — 같은 캐시라인에 `x` 값 32개(16비트 기준)가 들어옴.
- 자동 벡터화(auto-vectorization)가 훨씬 쉬워짐.
- 다만 "한 블록의 모든 필드를 동시에" 접근하는 패턴이 많다면 오히려 AoS가 나을 수 있음(LAYOUT-003 참고).

**탐지 방법**:
- Structural: 구조체 정의에서 필드 수 ≥ 4개이고, 해당 구조체의 `Vec<T>`를 순회하며 필드 1~2개만 읽는 함수가 있는지 코드 검색.
- Runtime: perf로 열 단위 스캔 함수의 IPC(instructions per cycle)와 캐시 미스 비교.

**예외**:
- 항목이 항상 "레코드 전체"로 함께 쓰이는 경우(예: 블록 하나를 통째로 직렬화) AoS가 더 단순하고 빠름.

**Bitvue 판정**: Confirmed — `Macroblock`/`CodingUnit` 구조체가 mb_addr/x/y/mb_type/qp/mv/ref_idx를 한 레코드에 AoS로 보관(crates/bitvue-avc/src/overlay_extraction.rs:87-106, crates/bitvue-hevc/src/overlay_extraction.rs:74-99); `extract_qp_grid`는 전체 `Vec<Macroblock>`을 파싱해 `.qp` 필드 하나만 사용(같은 파일 154-197줄).

---

### LAYOUT-003: SoA가 필요한 overlay 데이터에 AoS 사용
**분류**: LAYOUT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub struct BlockOverlayCell {
    pub qp: u8,
    pub mb_type: MbType,
    pub skip_flag: bool,
    pub intra_pred_mode: u8,
    pub color: [u8; 4], // RGBA
}

pub struct BlockOverlayMap {
    pub cells: Vec<BlockOverlayCell>, // width*height / block_size 개
    pub width_blocks: u32,
    pub height_blocks: u32,
}

fn render_qp_heatmap(map: &BlockOverlayMap, canvas: &mut [u8]) {
    for cell in &map.cells {
        // qp 하나만 쓰는데 나머지 필드까지 로드됨
        write_pixel_color(canvas, qp_to_color(cell.qp));
    }
}
```

**문제**:
- QP 히트맵, skip-flag 오버레이, MV 오버레이처럼 "레이어 하나씩 켜고 끄는" UI에서는 매 프레임 특정 필드 하나만 스캔.
- AoS 상태에서는 QP만 그리는데도 `mb_type`, `color` 등 불필요한 바이트까지 캐시에 로드 — 4K 기준 블록 수만 개일 때 체감되는 렌더 지연.
- 오버레이 토글(체크박스)마다 다른 필드를 스캔하므로, 필드별 분리 저장이 자연스러운 접근 패턴과 일치.

**발생 조건**:
- 실시간 오버레이 토글 UI(QP/MV/skip/reference-index 레이어를 개별적으로 켜고 끄는 뷰어)에서 프레임마다 재렌더링될 때.

**권장**:
```rust
pub struct BlockOverlayMapSoA {
    pub qp: Vec<u8>,
    pub mb_type: Vec<MbType>,
    pub skip_flag: BitVec, // LAYOUT-023 참고
    pub intra_pred_mode: Vec<u8>,
    pub width_blocks: u32,
    pub height_blocks: u32,
}
```
- 레이어별로 독립된 `Vec`를 스캔 → 캐시라인 활용률 최대화, SIMD 색상 매핑도 용이.
- 새 오버레이 레이어 추가 시 기존 배열에 영향 없음(스키마 확장이 쉬움).

**탐지 방법**:
- Semantic: 오버레이 렌더 함수가 구조체 필드 중 1개만 읽는데 순회 대상은 AoS `Vec<Struct>`인 패턴 검색.
- Runtime: 각 오버레이 토글의 프레임 렌더 시간 측정, AoS→SoA 전환 전후 비교.

**예외**:
- 오버레이 레이어가 항상 "QP+MV+skip을 동시에 합성"하는 단일 패스로만 그려진다면 AoS가 더 단순하고 성능 차이도 작을 수 있음.

**Bitvue 판정**: Confirmed — 위 LAYOUT-002와 동일 근거: `extract_qp_grid`/`extract_mv_grid`/`extract_partition_grid`/`extract_mb_type_grid`(crates/bitvue-avc/src/overlay_extraction.rs:154-1637)가 각각 매크로블록 전체를 재파싱해 필드 하나만 추출. 단, 최종 그리드 표현(QPGrid/MVGrid/BlockMetricsGrid)은 이미 SoA(flat Vec)로 잘 설계되어 있어 문제는 파싱 중간 단계에 국한.

---

### LAYOUT-004: bool 필드 다수로 padding 증가
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
pub struct BlockFlags {
    pub is_skip: bool,
    pub is_intra: bool,
    pub qp: u32,
    pub has_residual: bool,
    pub tx_size: u8,
    pub is_ipcm: bool,
    pub cbf_luma: bool,
}
// size_of::<BlockFlags>() == 16 (정렬 때문에 bool 5개가 5바이트인데 패딩으로 부풀려짐)
```

**문제**:
- `bool`은 1바이트지만 `u32` 필드 앞뒤로 흩어져 있으면 정렬 규칙 때문에 패딩이 삽입됨.
- 블록 단위 구조체가 4K 프레임 기준 수만 개 생성되므로, 패딩 몇 바이트가 전체 메모리 사용량과 캐시 점유율에 곱연산으로 반영됨.
- 필드 선언 순서가 "의미상 그룹"이 아니라 "작성 순서"를 따르고 있어 레이아웃이 우연에 맡겨짐.

**발생 조건**:
- 블록/CU/PU 단위 플래그 구조체가 프레임당 수만 개 인스턴스로 생성되는 파서 내부 표현.

**권장**:
```rust
pub struct BlockFlags {
    pub qp: u32,
    pub tx_size: u8,
    pub is_skip: bool,
    pub is_intra: bool,
    pub has_residual: bool,
    pub is_ipcm: bool,
    pub cbf_luma: bool,
}
// 큰 필드부터 작은 필드 순으로 배치 → size_of 최소화
```
- 필드를 크기 내림차순으로 배치하는 것이 기본 원칙(러스트 컴파일러가 자동 재배열하기도 하지만 `#[repr(C)]`나 FFI 경계에서는 수동 정렬이 필수).
- bool 5개는 비트플래그(`u8`의 비트 5개, LAYOUT-023)로 합치면 1바이트로 압축 가능.

**탐지 방법**:
- Static: `std::mem::size_of::<T>()`를 필드 크기 합과 비교하는 테스트/CI 체크 추가.
- Static: `cargo +nightly rustc -- -Zprint-type-sizes` 또는 `pahole`류 도구로 실제 레이아웃과 패딩 바이트 확인.

**예외**:
- 인스턴스 수가 적은(수십~수백 개) 설정/헤더 구조체는 패딩이 실질적 영향이 없으므로 가독성 우선 배치도 괜찮음.

**Bitvue 판정**: Suspected — HEVC `Pps`(crates/bitvue-hevc/src/pps.rs:12, bool 26개), AVC `Sps`(16개) 등 bool 다수 구조체가 존재하지만 모두 시퀀스/파라미터셋 단위(인스턴스 소수)로, 카탈로그가 전제하는 '블록당 수만 개' 스케일과는 다름 — 패딩 낭비의 실질 영향은 제한적.

---

### LAYOUT-005: Option<T>로 구조체 크기 폭증
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
pub struct RefPicEntry {
    pub poc: i32,
    pub long_term: bool,
    pub mv_scale: Option<f64>,       // niche 최적화 불가 → 9바이트+패딩
    pub cost: Option<u64>,
    pub weight: Option<(i16, i16)>,
}
```

**문제**:
- `Option<f64>`, `Option<u64>`처럼 niche(빈 비트 패턴)가 없는 타입을 감싸면 별도의 discriminant 바이트 + 정렬 패딩이 추가되어 구조체가 필요 이상으로 커짐.
- 참조 프레임 리스트(`RefPicEntry`)가 프레임당 여러 개, 여러 슬라이스에 걸쳐 반복 생성되므로 몇 바이트 차이가 누적됨.
- `Option` 남용은 "이 필드가 실제로 항상 있는지/드물게 없는지"에 대한 설계 결정을 회피한 결과인 경우가 많음.

**발생 조건**:
- 참조 프레임 리스트, MV 후보 리스트처럼 프레임/슬라이스마다 반복 생성되는 소형 구조체에서 선택적 필드가 여럿일 때.

**권장**:
```rust
pub struct RefPicEntry {
    pub poc: i32,
    pub long_term: bool,
    pub mv_scale: f64,   // 없으면 sentinel(예: f64::NAN 또는 1.0) 사용
    pub cost: u64,       // 없으면 u64::MAX를 sentinel로
    pub weight: (i16, i16),
}
```
- niche 최적화가 가능한 타입(`Option<NonZeroU32>`, `Option<&T>`)으로 바꾸면 discriminant 없이 0 크기 오버헤드.
- 정말 sparse한 필드라면 별도 `Vec<(u32, T)>` 인덱스 맵으로 분리해 hot path 구조체에서 아예 제거.

**탐지 방법**:
- Static: `size_of::<Option<T>>()` vs `size_of::<T>()` 차이를 CI에서 어서션.
- Manual: 구조체 필드 중 `Option<T>` 비율이 30% 넘으면 리뷰 대상으로 플래그.

**예외**:
- 실제로 "있을 수도 없을 수도" 있는 의미이고 인스턴스 수가 적다면(코덱 전역 설정 등) `Option`이 명확성 면에서 낫다.

**Bitvue 판정**: Confirmed — AVC/HEVC/VVC/AV3 overlay_extraction.rs의 CU/Macroblock 구조체마다 `Option<MotionVector>`×2 + `Option<i8>`×2가 반복(예: crates/bitvue-avc/src/overlay_extraction.rs:101-105), 블록 단위로 대량 인스턴스화됨.

---

### LAYOUT-006: enum variant 크기 불균형
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
pub enum SyntaxElement {
    Flag(bool),
    Ue(u32),
    Se(i32),
    ScalingList([u8; 64]),      // 64바이트짜리 variant 하나 때문에
    HrdParameters { cpb: Vec<CpbEntry>, nal_hrd: bool, vcl_hrd: bool }, // 얘도 큼
}
// size_of::<SyntaxElement>() 가 가장 큰 variant(ScalingList)에 맞춰짐
```

**문제**:
- Rust enum은 모든 variant 중 가장 큰 것에 맞춰 크기가 고정됨 — `ScalingList([u8; 64])` 하나 때문에 `Flag(bool)` 인스턴스도 64+바이트를 차지.
- 신택스 엘리먼트 파싱 결과를 `Vec<SyntaxElement>`로 수만 개 쌓는 구조라면, 실제로는 대부분 `Flag`/`Ue` 같은 작은 variant인데 메모리는 최대 variant 기준으로 낭비.
- 캐시라인 하나에 들어가는 엘리먼트 수가 줄어들어 순회 성능도 저하.

**발생 조건**:
- 범용 "신택스 엘리먼트" enum을 만들어 파싱 로그/디버그 트레이스용으로 대량 저장할 때(예: bitstream inspector의 element list).

**권장**:
```rust
// 큰 variant를 Box로 감싸 나머지 variant 크기에 영향 안 주게 분리
pub enum SyntaxElement {
    Flag(bool),
    Ue(u32),
    Se(i32),
    ScalingList(Box<[u8; 64]>),
    HrdParameters(Box<HrdParams>),
}
```
- 드물게 나오는 큰 variant만 `Box`로 힙에 위임하면 enum 자체는 작게 유지.
- 혹은 큰 variant를 별도 enum으로 분리(`SmallElement` / `LargeElement`)해 대부분의 순회가 작은 쪽만 다루게 설계.

**탐지 방법**:
- Static: `cargo +nightly rustc -- -Zprint-type-sizes`로 enum 크기와 variant별 기여도 확인.
- Static: clippy `large_enum_variant` lint 활성화.

**예외**:
- 모든 variant 크기가 비슷하거나, enum 인스턴스 수가 애초에 적다면(설정값 enum 등) 문제되지 않음.

**Bitvue 판정**: N/A — `SyntaxElement`류 범용 enum이나, 큰 고정 배열 variant가 작은 variant들과 섞인 사례를 찾지 못함; 배열을 가진 유일한 enum(`SeiData`, crates/bitvue-core/src/metadata.rs:452)은 variant 크기가 비교적 균형 있고 인스턴스 수도 적음.

---

### LAYOUT-007: u64가 필요 없는 필드까지 u64 사용
**분류**: LAYOUT · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
pub struct Block {
    pub x: u64,
    pub y: u64,
    pub width: u64,
    pub height: u64,
    pub qp: u64,       // QP 범위는 0~51 혹은 0~63
    pub skip: u64,     // 사실상 bool
}
```

**문제**:
- 프레임 좌표는 8K 기준으로도 `u16`(최대 65535)이면 충분한데 `u64`를 쓰면 필드당 6바이트 낭비.
- QP는 최대 6~7비트, skip은 1비트인데 각각 8바이트를 차지 — 구조체 하나가 48바이트로 부풀어 캐시라인 하나에 1.3개밖에 못 들어감.
- "일단 u64로 하면 오버플로 걱정 없다"는 방어적 습관이 대량 반복 구조체에 누적 비용으로 돌아옴.

**발생 조건**:
- 블록/CU 좌표, QP, 작은 카운터처럼 값의 실제 범위가 좁은 필드를 대량(프레임당 수만개) 생성하는 구조체.

**권장**:
```rust
pub struct Block {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub qp: u8,
    pub skip: bool,
}
// 48바이트 -> 10바이트(+패딩)로 축소
```
- 좌표는 `u16`(8K=7680 안에 충분히 들어감), 크기 값도 `u16`, QP는 `u8`, 플래그는 `bool`/비트필드.
- 실제 스펙 상 필드의 최대값을 문서화하고 그에 맞는 최소 정수 타입을 선택.

**탐지 방법**:
- Static: 코드 리뷰 체크리스트 — `u64`/`i64` 필드마다 "이 필드의 실제 최대값은?" 질문.
- Static: clippy 커스텀 lint 또는 grep으로 좌표/QP류 필드명 패턴(`_x`, `_y`, `qp`, `width`, `height`)에 대해 타입이 `u64`인 경우 플래그.

**예외**:
- 타임스탬프(PTS/DTS), 바이트 오프셋, 파일 크기처럼 실제로 64비트 범위가 필요한 필드는 `u64`가 맞음.

**Bitvue 판정**: N/A — 좌표/QP/skip류 필드에 `u64`/`i64`를 쓰는 사례를 찾지 못함(`x: u64` 등 grep 无결과); CU 좌표는 `u32`(HEVC: crates/bitvue-hevc/src/overlay_extraction.rs:77-78)로, 카탈로그가 지적하는 `u64`보다는 완화된 형태.

---

### LAYOUT-008: Frame 객체 안에 모든 분석 결과 포함
**분류**: LAYOUT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub struct Frame {
    pub poc: i32,
    pub pixels_y: Vec<u8>,
    pub pixels_u: Vec<u8>,
    pub pixels_v: Vec<u8>,
    pub mv_field: Vec<MotionVector>,
    pub qp_map: Vec<u8>,
    pub mb_type_map: Vec<MbType>,
    pub psnr: f64,
    pub ssim: f64,
    pub vmaf: Option<f64>,
    pub bitrate_breakdown: HashMap<String, u64>,
    pub decode_trace: Vec<TraceEvent>,
}
```

**문제**:
- 디코딩 hot path(픽셀 복원, 참조 프레임 접근)는 `pixels_*`만 필요한데, 구조체 전체가 크기 때문에 `Vec<Frame>` 순회나 `&Frame` 전달 시 불필요한 필드까지 캐시/레지스터 압력에 관여.
- 분석 전용 필드(`psnr`, `vmaf`, `decode_trace`)는 특정 단계에서만 채워지고 나머지 단계에서는 항상 기본값/None — cold data가 hot struct에 섞임(LAYOUT-009와 연결).
- 프레임 버퍼 풀링/재사용 시 구조체가 크면 memcpy/clear 비용도 커짐.

**발생 조건**:
- "일단 Frame에 다 넣고 나중에 채우자"는 식으로 분석 파이프라인이 확장되면서 필드가 계속 추가될 때.

**권장**:
```rust
pub struct FramePixels {
    pub poc: i32,
    pub pixels_y: Vec<u8>,
    pub pixels_u: Vec<u8>,
    pub pixels_v: Vec<u8>,
}

pub struct FrameAnalysis {
    pub poc: i32, // FramePixels와 조인 키
    pub mv_field: Vec<MotionVector>,
    pub qp_map: Vec<u8>,
    pub mb_type_map: Vec<MbType>,
}

pub struct FrameMetrics {
    pub poc: i32,
    pub psnr: f64,
    pub ssim: f64,
    pub vmaf: Option<f64>,
}
```
- 디코드 hot path는 `FramePixels`만 다루고, 오버레이/통계는 별도 구조체를 `poc`(또는 인덱스)로 조인.
- 각 구조체가 자신의 접근 패턴에 맞게 독립적으로 캐시 지역성을 가짐.

**탐지 방법**:
- Structural: 핵심 `Frame` 타입의 필드 수와 `size_of` 추적, PR마다 크기 증가를 CI에서 경고.
- Semantic: 필드별로 "어느 함수에서 읽는지" 매핑해 hot path 함수와 겹치지 않는 필드 식별.

**예외**:
- 소규모 툴/스크립트성 코드에서 프레임 수가 적고(<수십) 성능이 중요치 않다면 단일 구조체가 개발 편의상 낫다.

**Bitvue 판정**: N/A — 프레임 데이터가 이미 `FrameInfo`/`FrameAnalysis`/`FrameMetadata`/`FrameRgbData`/`FrameYuvData`로 분리되어 인덱스로 조인됨(crates/bitvue-core/src/stream_state.rs:326-860, `CachedFrame::from_components`) — 단일 god Frame 구조체 없음.

---

### LAYOUT-009: cold metadata와 hot loop 데이터를 같은 구조체에 배치
**분류**: LAYOUT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub struct Block {
    // hot: 매 픽셀 루프마다 접근
    pub residual: [i16; 16],
    pub pred_mode: u8,
    // cold: 디버그/로그/UI 툴팁에서만 가끔 접근
    pub source_file_offset: u64,
    pub parse_duration_ns: u64,
    pub debug_label: String,
    pub raw_bits_snapshot: Vec<u8>,
}
```

**문제**:
- `residual`/`pred_mode`를 순회하는 IDCT/역양자화 루프가, 실제로는 안 쓰는 `debug_label`, `raw_bits_snapshot`까지 같은 캐시라인/페이지에 끌고 다님.
- cold 필드(`String`, `Vec<u8>`)는 힙 포인터를 포함하므로 구조체 크기를 키우고, 배열로 순회할 때 stride가 커져 prefetch 효율이 떨어짐.
- "디버깅에 유용하니까 일단 넣어두자"는 필드가 프로덕션 hot path 성능을 갉아먹는 전형적 패턴.

**발생 조건**:
- 파서 개발 중 추가한 디버그 필드가 릴리스 빌드까지 구조체에 남아있는 경우, 특히 블록 단위로 수만 개 인스턴스가 생성될 때.

**권장**:
```rust
pub struct BlockHot {
    pub residual: [i16; 16],
    pub pred_mode: u8,
}

pub struct BlockDebugInfo {
    pub source_file_offset: u64,
    pub parse_duration_ns: u64,
    pub debug_label: String,
    pub raw_bits_snapshot: Vec<u8>,
}
// 디버그 정보는 별도 side table(예: HashMap<BlockId, BlockDebugInfo>)로,
// 디버그 빌드/verbose 모드에서만 채움
```
- hot/cold 분리 원칙: "이 필드가 초당 수백만 번 루프에서 읽히는가?"로 배치 결정.
- cold 데이터는 `#[cfg(debug_assertions)]` 또는 별도 옵트인 구조체로 분리해 릴리스 빌드에서 아예 제거 가능하게.

**탐지 방법**:
- Semantic: 구조체 필드별 읽기 빈도를 프로파일러(perf record + 소스라인 어노테이션)로 추정.
- Manual: 코드 리뷰에서 "hot loop 안에서 도는 구조체에 `String`/`Vec` 필드가 있는가"를 체크리스트화.

**예외**:
- 구조체 인스턴스 수가 적거나(프레임당 1개 수준의 헤더), hot loop 자체가 없다면 분리 이득이 작음.

**Bitvue 판정**: N/A — hot 루프에서 도는 블록/CU 구조체에 String/Vec<u8> 같은 cold 디버그 필드가 섞인 사례를 찾지 못함; CU/Macroblock 구조체는 숫자/enum 필드로만 구성.

---

### LAYOUT-010: 인접 프레임 데이터가 메모리상 분산
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct DecodedPictureBuffer {
    pub frames: HashMap<i32, Box<Frame>>, // poc -> Frame, 각각 별도 힙 할당
}

fn compute_temporal_diff(dpb: &DecodedPictureBuffer, poc_a: i32, poc_b: i32) -> f64 {
    let a = dpb.frames.get(&poc_a).unwrap();
    let b = dpb.frames.get(&poc_b).unwrap();
    // a, b가 힙 어디에 있는지 예측 불가 — 매번 포인터 역참조 + 캐시 미스
    diff_pixels(&a.pixels_y, &b.pixels_y)
}
```

**문제**:
- `HashMap<i32, Box<Frame>>`는 각 프레임을 독립적으로 힙에 할당하므로 인접 POC 프레임이라도 메모리 주소가 인접한다는 보장이 전혀 없음.
- 시간적 필터링(temporal denoise), 프레임 간 diff, GOP 단위 통계처럼 "여러 프레임을 순서대로 훑는" 연산에서 매 프레임 접근이 랜덤 액세스가 됨.
- 재생/스크러빙 시 순차 재생인데도 DPB 내부는 삽입 순서에 따라 흩어져 있어 OS 페이지 캐시/prefetcher 이득을 못 봄.

**발생 조건**:
- GOP 전체를 순회하는 배치 분석(예: 평균 QP 추이 그래프, 씬 컷 감지)이나 필름스트립 썸네일 생성처럼 다수 프레임을 순서대로 접근할 때.

**권장**:
```rust
pub struct DecodedPictureBuffer {
    // 링버퍼 형태로 연속 슬롯에 프레임을 배치, poc -> slot index만 별도 관리
    pub slots: Vec<Frame>,
    pub poc_to_slot: HashMap<i32, u32>,
    pub capacity: usize,
}
```
- 프레임 버퍼를 고정 크기 `Vec<Frame>` 슬롯 풀로 관리하고 재사용 — 슬롯 자체는 연속 메모리.
- 순서 접근이 필요한 배치 작업은 `slots`를 직접 순회하도록 API를 분리해 랜덤 HashMap 조회를 피함.

**탐지 방법**:
- Runtime: GOP 순회 벤치마크에서 `HashMap<_, Box<Frame>>` 버전과 slot pool 버전의 `cache-misses`/처리량 비교.
- Structural: DPB/프레임 캐시 구현이 `HashMap<K, Box<V>>` 패턴인지 검색.

**예외**:
- 프레임을 순서 없이 임의 접근하는 use case가 지배적이고(예: 사용자가 임의 프레임으로 점프하는 뷰어) 순차 스캔이 드물다면 HashMap 방식의 단순함이 더 유리.

**Bitvue 판정**: Suspected — 디코드 프레임 캐시가 `lru::LruCache<usize, CachedFrame>`(crates/bitvue-core/src/stream_state.rs:583-591)로 해시 기반이라 순차 지역성은 보장되지 않지만, 캡a 32의 seek용 캐시라는 용도 자체가 카탈로그가 명시한 '임의 프레임 점프 뷰어' 예외에 해당할 가능성이 높음.

---

### LAYOUT-011: 좌표를 객체 계층으로 표현
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct Point {
    pub x: Box<Coordinate>,
    pub y: Box<Coordinate>,
}
pub struct Coordinate {
    pub value: i32,
    pub unit: Unit,
}
pub struct Rect {
    pub top_left: Box<Point>,
    pub bottom_right: Box<Point>,
}
```

**문제**:
- 단순 2D 좌표 하나를 표현하는 데 힙 할당이 4~5번 발생(`Point` 2개 + `Coordinate` 2개 + 그 안의 `Box`).
- 블록 맵/오버레이에서 좌표가 수만 개 필요한데, 매번 포인터 역참조 체인(`rect.top_left.x.value`)을 타야 값 하나를 얻음.
- 값 타입이면 충분한 데이터(정수 좌표)를 참조 타입 계층으로 모델링한 전형적 과설계.

**발생 조건**:
- OOP 스타일에 익숙한 설계가 그대로 이식되어 "모든 개념은 별도 클래스/구조체"라는 원칙을 좌표에도 적용했을 때.
- 블록 맵, 바운딩 박스 오버레이처럼 좌표 인스턴스가 대량으로 생성되는 곳.

**권장**:
```rust
#[derive(Clone, Copy)]
pub struct Point { pub x: i32, pub y: i32 }

#[derive(Clone, Copy)]
pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }
```
- 좌표는 `Copy` 가능한 값 타입(POD)으로 스택/배열에 직접 저장.
- 단위(픽셀/블록/서브픽셀)가 섞여 혼동 우려가 있다면 newtype(`struct PixelCoord(i32)`)으로 타입 안전성만 확보하고, 힙 할당은 피함.

**탐지 방법**:
- Static: 좌표류 구조체 정의에서 필드 타입이 `Box<T>`/`Rc<T>`인지 검색.
- Structural: `Point`, `Rect`, `Coordinate` 등 이름 패턴을 가진 타입의 `size_of`와 내부 포인터 수 확인.

**예외**:
- 좌표에 방대한 메타데이터(단위 변환 히스토리, 좌표계 체인 등)가 실제로 붙어야 하는 특수 케이스라면 계층 구조가 정당화될 수 있음 — 다만 hot path와는 분리해야 함.

**Bitvue 판정**: N/A — `Point`/`Coordinate`/`Rect` 식의 좌표 객체 계층이 crates 어디에도 없음; 좌표는 항상 구조체에 직접 박힌 `x`/`y`/`width`/`height` 필드로 표현됨.

---

### LAYOUT-012: 그래프를 Rc<Node> 연결 구조로 표현
**분류**: LAYOUT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub struct RefFrameNode {
    pub poc: i32,
    pub referenced_by: RefCell<Vec<Rc<RefFrameNode>>>,
    pub references: RefCell<Vec<Weak<RefFrameNode>>>,
}
```

**문제**:
- 참조 프레임 그래프(누가 누구를 참조하는가, DPB 의존성 그래프)를 `Rc`/`Weak` 연결 리스트로 만들면 노드마다 별도 힙 할당 + 참조 카운트 오버헤드.
- 그래프 순회(예: "이 프레임을 참조하는 모든 프레임 찾기", GOP 구조 시각화)가 포인터를 따라가며 캐시 미스를 유발.
- `RefCell`의 런타임 borrow 체크 비용까지 hot path에 추가됨.
- 순환 참조 방지를 위해 `Weak`를 섞어야 해서 코드 복잡도도 증가.

**발생 조건**:
- GOP 구조 분석, B프레임 참조 체인 시각화처럼 그래프형 관계를 다루는 기능을 추가할 때 OOP 습관대로 노드-포인터 그래프를 만드는 경우.

**권장**:
```rust
pub struct RefFrameGraph {
    pub pocs: Vec<i32>,                 // node id = index
    pub edges: Vec<(u32, u32)>,         // (from_idx, to_idx) flat edge list
    // 또는 CSR(Compressed Sparse Row) 형태:
    pub edge_offsets: Vec<u32>,
    pub edge_targets: Vec<u32>,
}
```
- 그래프를 인접 리스트(CSR) 또는 edge list로 flat하게 저장 — 노드는 배열 인덱스로만 참조.
- 순회는 인덱스 기반 반복문으로 처리되어 캐시 친화적이고, 참조 카운트/borrow 체크 오버헤드가 없음.

**탐지 방법**:
- Structural: `Rc<`, `RefCell<`, `Weak<`가 그래프/트리형 타입 정의에 함께 등장하는지 검색.
- Runtime: 그래프 순회 벤치마크에서 CSR 버전과 Rc 체인 버전의 처리량 비교.

**예외**:
- 그래프 크기가 매우 작고(GOP 내 수십 개 프레임 수준) 순회 빈도가 낮다면 `Rc` 그래프도 실질적 문제가 되지 않을 수 있음.

**Bitvue 판정**: N/A — 참조 프레임 그래프가 이미 권장 패턴(플랫 edge list)으로 구현됨: `ReferenceEdge{from_idx,to_idx}` + `GraphNode`(crates/bitvue-core/src/reference_graph.rs:42-90); `Rc`/`RefCell`/`Weak`는 crates 전체에서 전혀 발견되지 않음.

---

### LAYOUT-013: index로 충분한 곳에 포인터 사용
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct Cu {
    pub qp: u8,
    pub neighbor_left: Option<*const Cu>,
    pub neighbor_above: Option<*const Cu>,
}
```

**문제**:
- CU/블록 이웃 참조를 원시 포인터로 저장하면, 배열이 재할당(`Vec` growth)될 때 포인터가 즉시 무효화됨 — use-after-free/댕글링 위험.
- 포인터는 8바이트, 대응하는 인덱스(`u32`)는 4바이트 — 구조체 크기가 불필요하게 커짐.
- 포인터 기반 이웃 참조는 직렬화/역직렬화(스냅샷, 리플레이)가 불가능하거나 매우 번거로움.

**발생 조건**:
- CU/PU 트리에서 좌/상단 이웃을 캐싱해 인트라 예측 등에 재사용하려는 최적화를 시도할 때.

**권장**:
```rust
pub struct Cu {
    pub qp: u8,
    pub neighbor_left: Option<u32>,   // Vec<Cu> 내 인덱스
    pub neighbor_above: Option<u32>,
}
// 또는 니치 최적화를 살리려면 sentinel 값 사용:
pub struct Cu {
    pub qp: u8,
    pub neighbor_left: u32,  // u32::MAX == "없음"
    pub neighbor_above: u32,
}
```
- 인덱스는 `Vec` 재할당에 안전(값이 그대로 유효), 크기도 작고, 직렬화가 자연스러움.
- "없음"을 별도 `Option`이 아니라 sentinel 값(`u32::MAX`)으로 표현하면 크기도 줄일 수 있음(LAYOUT-024와 연결).

**탐지 방법**:
- Static: `*const`/`*mut` 필드가 배열 인덱싱 용도로 쓰이는지 grep.
- Manual: unsafe 포인터 필드가 있는 구조체는 리뷰에서 "이거 인덱스로 대체 가능한가?" 질문.

**예외**:
- FFI 경계(디코더 네이티브 라이브러리와의 상호운용)에서는 실제 포인터가 필요할 수 있음 — 이 경우 unsafe 경계를 명확히 문서화.

**Bitvue 판정**: N/A — 발견된 원시 포인터 필드는 vvdec FFI 바인딩(crates/bitvue-decode/src/vvdec.rs)뿐이며 이는 카탈로그 자체가 예외로 인정하는 경우; CU 이웃 캐싱을 위한 내부 포인터 패턴은 없음.

---

### LAYOUT-014: 문자열 비교가 hot path에 존재
**분류**: LAYOUT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub fn classify_nal_unit(nal_type_name: &str) -> NalCategory {
    match nal_type_name {
        "IDR_W_RADL" => NalCategory::Idr,
        "IDR_N_LP" => NalCategory::Idr,
        "TRAIL_R" => NalCategory::Trail,
        "TRAIL_N" => NalCategory::Trail,
        // ... 수십 개 분기, NALU마다 호출됨
        _ => NalCategory::Unknown,
    }
}
```

**문제**:
- `&str` 비교는 길이 비교 + 바이트 단위 memcmp — NALU/블록마다 호출되면 정수 비교 대비 수배 느림.
- 문자열 리터럴이 여러 곳에 흩어져 있으면 오타로 인한 매칭 실패 버그도 유발하기 쉬움(타입 시스템의 보호를 못 받음).
- 이런 문자열이 구조체 필드로 저장되면(`String`/`&str`) 구조체 크기도 포인터+길이+캐패시티(24바이트, `String` 기준)로 커짐.

**발생 조건**:
- NAL unit 타입, 블록 모드, 신택스 엘리먼트 이름 등을 파싱 초기에는 enum으로 파싱해놓고, 이후 로직에서 다시 문자열로 왕복 변환해 비교하는 경우.

**권장**:
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NalType {
    IdrWRadl, IdrNLp, TrailR, TrailN, /* ... */ Unknown,
}

pub fn classify_nal_unit(nal_type: NalType) -> NalCategory {
    match nal_type {
        NalType::IdrWRadl | NalType::IdrNLp => NalCategory::Idr,
        NalType::TrailR | NalType::TrailN => NalCategory::Trail,
        _ => NalCategory::Unknown,
    }
}
```
- 파싱 시점에 스펙의 NAL type 값(정수)을 바로 enum으로 변환해 이후 전 구간에서 정수/enum 비교만 사용.
- 사람이 읽을 문자열 표현은 `Display`/`Debug` 구현으로 필요한 시점(로깅, UI)에만 생성.

**탐지 방법**:
- Structural: hot path 함수(파서 내부 루프, 프레임당 반복 호출) 시그니처에 `&str` 파라미터가 있고 `match`/`==`로 비교하는지 검색.
- Runtime: 문자열 매칭 함수에 대한 마이크로벤치마크 후 enum 매칭과 비교.

**예외**:
- 사용자 입력(CLI 옵션, 설정 파일 파싱)이나 호출 빈도가 낮은 API 경계에서는 문자열 비교가 자연스럽고 문제 없음.

**Bitvue 판정**: Suspected — `unit_type == "FRAME"` 문자열 비교가 존재하지만(src-tauri/src/services/frame_service.rs:132,149) UI 메타데이터 조립 시 유닛당 1회 호출이며 카탈로그가 말하는 블록/NALU 단위 hot loop은 아님; 다른 곳(crates/bitvue-hevc/src/nal.rs)의 NAL 타입 분류는 이미 enum 기반.

---

### LAYOUT-015: sparse 구조가 아닌데 HashMap 사용
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct QpMap {
    // 블록 좌표 (bx, by) -> qp, 하지만 실제로는 모든 블록에 값이 존재(dense)
    pub values: HashMap<(u32, u32), u8>,
}

fn average_qp(map: &QpMap, width_blocks: u32, height_blocks: u32) -> f64 {
    let mut sum = 0u64;
    for by in 0..height_blocks {
        for bx in 0..width_blocks {
            sum += *map.values.get(&(bx, by)).unwrap_or(&0) as u64;
        }
    }
    sum as f64 / (width_blocks * height_blocks) as f64
}
```

**문제**:
- 모든 블록에 항상 값이 존재하는 "dense" 데이터인데 `HashMap`을 쓰면 해시 계산 + 버킷 탐색 + 캐시 미스 비용을 매 조회마다 지불.
- 메모리 오버헤드도 큼 — `HashMap<(u32,u32), u8>` 엔트리 하나가 실제 유효 데이터(1바이트)에 비해 수십 바이트(키+해시+버킷 메타데이터)를 차지.
- 순차 순회(`for by, for bx`) 패턴인데도 해시 테이블은 순서를 보장하지 않아 메모리 지역성이 전혀 없음.

**발생 조건**:
- "혹시 나중에 값이 없는 블록이 생길 수도 있으니" 방어적으로 HashMap을 선택했지만, 실제 도메인(QP 맵, 스킵 맵, MV 맵)은 항상 dense인 경우.

**권장**:
```rust
pub struct QpMap {
    pub values: Vec<u8>, // row-major: values[by * width_blocks + bx]
    pub width_blocks: u32,
    pub height_blocks: u32,
}

fn average_qp(map: &QpMap) -> f64 {
    let sum: u64 = map.values.iter().map(|&q| q as u64).sum();
    sum as f64 / map.values.len() as f64
}
```
- Dense 2D 데이터는 row-major flat `Vec<T>`로 표현하고 `[by * width + bx]`로 인덱싱.
- 정말 sparse한 경우(예: 전체 블록의 5% 미만만 값이 있는 예외 맵)에만 `HashMap`이나 `BTreeMap`을 사용.

**탐지 방법**:
- Semantic: `HashMap<(u32,u32), _>` 또는 `HashMap<BlockCoord, _>` 타입을 검색하고, 채움 비율(fill ratio)이 실제로 100%에 가까운지 확인.
- Runtime: `HashMap` 버전과 `Vec` 버전의 전체 순회/평균 계산 벤치마크 비교.

**예외**:
- 실제로 희소한 오버레이(예: 사용자가 클릭해서 주석 단 특정 블록만 표시하는 annotation layer)라면 HashMap이 적절.

**Bitvue 판정**: Confirmed — `sb_index: HashMap<(u32, u32), Vec<usize>>`가 dense 그리드 블록마다(모든 슈퍼블록에 값 존재) 조회됨(crates/bitvue-av1-codec/src/overlay_extraction/partition.rs:684-705) — flat `Vec` 인덱싱으로 대체 가능한 구조.

---

### LAYOUT-016: 필드 순서가 접근 패턴별로 그룹화되지 않음
**분류**: LAYOUT · **심각도**: Low · **탐지**: Manual

**나쁜 예**:
```rust
pub struct DecoderState {
    pub current_qp: u8,          // hot: 매 블록마다 갱신
    pub sps_profile_idc: u8,     // cold: 시퀀스당 1회 설정 후 불변
    pub current_mv_x: i16,       // hot
    pub vui_timing_info: TimingInfo, // cold, 크고 거의 안 쓰임
    pub current_mv_y: i16,       // hot
    pub pps_id: u8,              // cold
}
```

**문제**:
- hot 필드(`current_qp`, `current_mv_x/y`)와 cold 필드(`sps_profile_idc`, `vui_timing_info`)가 선언 순서대로 뒤섞여 있어, 컴파일러의 기본 재배열에 맡기더라도 논리적 그룹핑이 코드상 드러나지 않음.
- hot 필드만 갱신하는 루프에서도 구조체 전체가 하나의 캐시라인 세트를 차지 — cold 필드가 중간에 끼어 있으면 hot 필드들이 서로 다른 캐시라인에 걸칠 수 있음.
- 유지보수자가 필드를 추가할 때 "어디에 넣어야 하는지" 기준이 없어 계속 무작위로 커짐.

**발생 조건**:
- 디코더/파서의 "현재 상태" 구조체가 오랜 기간에 걸쳐 필드가 추가되며 유기적으로 커진 경우.

**권장**:
```rust
pub struct DecoderState {
    // --- hot: per-block ---
    pub current_qp: u8,
    pub current_mv_x: i16,
    pub current_mv_y: i16,
    // --- cold: per-sequence, 거의 불변 ---
    pub sps_profile_idc: u8,
    pub pps_id: u8,
    pub vui_timing_info: TimingInfo,
}
```
- 필드를 "갱신 빈도" 기준으로 물리적으로 묶어 선언 — 주석으로 그룹 경계를 명시.
- 가능하면 LAYOUT-009처럼 아예 별도 구조체로 분리하는 것이 더 근본적 해법.

**탐지 방법**:
- Manual: 구조체 리뷰 시 필드별 "이 값은 얼마나 자주 바뀌는가"를 주석으로 달아보는 체크리스트.
- Static: `-Zprint-type-sizes`로 실제 컴파일러 재배열 결과를 확인해 의도와 다르게 배치됐는지 검증.

**예외**:
- 구조체가 작아서(캐시라인 1~2개 이내) 그룹핑 이득이 미미한 경우 실익이 적음.

**Bitvue 판정**: N/A — hot per-block 필드와 cold per-sequence 필드가 뒤섞인 'DecoderState'류 구조체를 찾지 못함; 비교 가능한 유일한 구조체(`ParserState`, crates/bitvue-codecs-parser/src/parser_strategy.rs:194-203)는 작고 파서 인스턴스당 1개.

---

### LAYOUT-017: repr(Rust)에 암묵적으로 의존
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
// FFI로 네이티브 디코더(C 라이브러리)와 프레임 메타데이터를 주고받는 구조체
pub struct FrameMeta {
    pub width: u32,
    pub height: u32,
    pub pix_fmt: u8,
    pub timestamp: i64,
}

extern "C" {
    fn native_decode_frame(meta: *mut FrameMeta) -> i32;
}
```

**문제**:
- `#[repr(Rust)]`(기본값)는 필드 순서/패딩/정렬을 컴파일러가 임의로 재배열할 수 있다고 명시적으로 보장하지 않음 — 실제로는 안정적으로 동작하는 것처럼 보여도 스펙상 미보장.
- FFI 경계나 파일 포맷 직렬화(바이너리 오버레이 캐시 저장 등)에서 이 구조체를 그대로 `memcpy`하면, Rust 컴파일러 버전/최적화 옵션이 바뀔 때 레이아웃이 달라질 위험이 있음.
- C 쪽 구조체 정의와 필드 순서가 맞는지 컴파일 타임에 검증할 방법이 없어짐.

**발생 조건**:
- 네이티브 디코더 라이브러리와 구조체를 직접 주고받는 FFI 코드, 또는 오버레이 캐시를 바이너리로 디스크에 저장/로드하는 기능에서.

**권장**:
```rust
#[repr(C)]
pub struct FrameMeta {
    pub width: u32,
    pub height: u32,
    pub pix_fmt: u8,
    pub timestamp: i64,
}
```
- FFI/직렬화 경계의 모든 구조체에 `#[repr(C)]`(또는 필요 시 `#[repr(C, packed)]`)를 명시적으로 부여.
- C 쪽 헤더와 필드 순서/타입이 일치하는지 `static_assertions::assert_eq_size!` 등으로 컴파일 타임 검증.
- 순수 내부 로직용 구조체(FFI/직렬화와 무관)까지 `repr(C)`를 강제할 필요는 없음 — 오히려 컴파일러 최적화 재배열을 막아 손해일 수 있음.

**탐지 방법**:
- Static: `extern "C"` 블록과 함께 쓰이는 구조체 중 `#[repr(C)]`가 없는 것을 grep.
- Static: `bindgen`/`cbindgen` 사용 시 자동 생성된 헤더와 Rust 정의 간 크기 불일치 검사를 CI에 포함.

**예외**:
- FFI/직렬화 경계와 무관한 순수 내부 구조체는 `#[repr(Rust)]` 기본값이 오히려 컴파일러 최적화(자동 필드 재배열)의 이점을 살릴 수 있어 더 낫다.

**Bitvue 판정**: N/A — 유일한 FFI 경계(crates/bitvue-decode/src/vvdec.rs)의 모든 교환 구조체가 `#[repr(C)]`로 명시적으로 선언되어 있음.

---

### LAYOUT-018: 블록/MV 배열이 SIMD를 위한 stride 정렬 안 됨
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct MvGrid {
    pub width_blocks: usize,   // 예: 121 (4x4 블록 기준 484px 폭)
    pub height_blocks: usize,
    pub data: Vec<(i16, i16)>, // width_blocks * height_blocks, stride == width_blocks
}

fn row_ptr(grid: &MvGrid, row: usize) -> &[(i16, i16)] {
    &grid.data[row * grid.width_blocks..(row + 1) * grid.width_blocks]
}
```

**문제**:
- 행(row) 길이가 SIMD 레인 폭(예: AVX2 8-wide, 16비트 값 기준 16-wide)의 배수가 아니면, 행 경계마다 남은 몇 개 원소를 스칼라 fallback으로 처리해야 함.
- 다음 행 시작 주소가 정렬 경계(16/32/64바이트)에 걸리지 않아 unaligned load/store가 강제되거나, 컴파일러가 자동 벡터화를 아예 포기함.
- 매 프레임 반복되는 MV 필드 스캔(옵티컬 플로우 비교, 히트맵 렌더링)에서 이 손실이 누적됨.

**발생 조건**:
- 프레임 폭이 SIMD 레인 폭의 배수가 아닌 해상도(예: 1920은 4x4 블록 기준 480, 8-wide로 나누면 60이라 괜찮지만 crop된 비표준 해상도는 안 나눠떨어지는 경우가 흔함).

**권장**:
```rust
pub struct MvGrid {
    pub width_blocks: usize,   // 논리적 폭
    pub stride: usize,         // width_blocks를 SIMD 레인 배수로 올림(padding 포함)
    pub height_blocks: usize,
    pub data: Vec<(i16, i16)>, // stride * height_blocks, 패딩 영역은 0으로 채움
}
```
- 논리적 폭과 별개로 `stride`를 SIMD 폭의 배수로 반올림(round up)해 각 행이 정렬 경계에서 시작하게 함.
- 패딩 영역은 안전한 sentinel 값으로 채워 SIMD 루프가 경계 체크 없이 전체 stride를 처리 가능하게.
- 버퍼 자체도 `#[repr(align(32))]` 또는 aligned allocator로 할당해 첫 행부터 정렬 보장.

**탐지 방법**:
- Runtime: `cargo asm`이나 `perf stat -e fp_arith_inst_retired` 등으로 벡터화 여부/스칼라 fallback 비율 확인.
- Static: SIMD 커널 코드에서 `width_blocks`를 stride로 그대로 쓰는지, 별도 정렬된 stride 필드가 있는지 검토.

**예외**:
- 해당 배열을 다루는 코드에 SIMD 최적화 계획이 전혀 없다면(단순 스칼라 순회로 충분한 저빈도 연산) stride 정렬 투자는 불필요한 복잡도.

**Bitvue 판정**: N/A — row-major MV/QP 그리드에 대해 stride 정렬을 고려하는 SIMD 코드가 없음; 기존 SIMD(crates/bitvue-metrics/src/simd.rs)는 row 경계 없는 flat 1차원 픽셀 버퍼만 처리.

---

### LAYOUT-019: enum variant마다 다른 payload를 AoS로 저장해 SoA 기회를 놓침
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub enum PredUnit {
    Intra { mode: u8, mpm_idx: u8 },
    Inter { mv: (i16, i16), ref_idx: u8 },
    Skip,
}

pub struct PuMap {
    pub units: Vec<PredUnit>, // 태그 + 최대 payload 크기로 매 원소가 고정 크기
}

fn count_intra(map: &PuMap) -> usize {
    map.units.iter().filter(|u| matches!(u, PredUnit::Intra { .. })).count()
}
```

**문제**:
- `PredUnit`은 가장 큰 variant(`Inter`)에 맞춰 discriminant + payload 크기가 고정됨 — `Skip`도 같은 크기를 차지.
- "인트라 블록 개수만 세기" 같은 단일 variant 스캔에서도 배열 전체를 원소 크기만큼 순회해야 하고, 태그 분기(`match`)가 매 원소마다 발생.
- 실제로는 variant별로 별도 배열(태그 배열 + 각 variant 전용 payload 배열)로 분리하면 태그 스캔은 1바이트 단위로, payload 접근은 필요할 때만 이루어짐.

**발생 조건**:
- 신택스 유닛 종류가 소수(3~6개)로 고정되어 있고, 특정 종류의 개수/비율 통계를 자주 계산하는 파이프라인(모드 분포 히스토그램 등).

**권장**:
```rust
pub struct PuMapSoA {
    pub tags: Vec<PuTag>,            // 1바이트 태그만 모은 배열 — 통계 스캔용
    pub intra: Vec<(u8, u8)>,        // (mode, mpm_idx), tags[i]==Intra인 원소만 대응 인덱스로 참조
    pub inter: Vec<((i16, i16), u8)>,
}
```
- 태그 배열만으로 분포 통계(각 모드 개수)를 빠르게 계산 가능.
- 특정 variant의 payload가 필요한 시점에만 해당 variant 전용 배열에 접근.

**탐지 방법**:
- Semantic: enum 배열을 순회하며 특정 variant만 필터링하는 함수가 반복적으로 나타나는지 코드 검색.
- Static: enum 크기(`size_of`)와 실제 필요한 최소 태그 크기(1바이트) 차이 확인.

**예외**:
- variant 종류가 항상 함께 처리되고(예: 렌더링 시 모든 PU를 한 번씩 그리기만 함) 필터링/통계 연산이 없다면 AoS enum이 더 단순하고 충분히 빠름.

**Bitvue 판정**: N/A — `PredUnit`류 payload-불균형 tagged enum을 대량 배열에 저장하고 필터링하는 패턴을 찾지 못함; 블록 타입 enum(MbType, PredMode)은 payload 없는 균일한 variant로 구성.

---

### LAYOUT-020: false sharing — 스레드별 카운터가 한 캐시라인에 packed
**분류**: LAYOUT · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct ParallelDecodeStats {
    pub tiles_done: [AtomicU32; 16], // 타일(스레드)별 완료 카운터, 16개가 연속 배치
}

// 스레드 i는 tiles_done[i]만 갱신하지만, 인접 원소가 같은 캐시라인(64B / 4B = 16개)에 들어감
fn worker(stats: &ParallelDecodeStats, tile_idx: usize) {
    // ... 타일 디코딩 ...
    stats.tiles_done[tile_idx].fetch_add(1, Ordering::Relaxed);
}
```

**문제**:
- `AtomicU32` 16개가 정확히 64바이트 캐시라인 하나에 들어가므로, 서로 다른 스레드가 서로 다른 인덱스를 갱신해도 물리적으로 같은 캐시라인을 두고 경쟁(false sharing).
- 코어 간 캐시라인 무효화(MESI 프로토콜의 invalidate/RFO)가 매 증가 연산마다 발생 — 논리적으로는 독립적인 카운터인데 하드웨어 레벨에서 직렬화됨.
- 타일 병렬 디코딩처럼 스레드 수가 많을수록 이 경합이 스레드 수에 비례해 악화되어, 병렬화 이득이 상쇄되거나 역전됨.

**발생 조건**:
- 멀티스레드 타일/슬라이스 병렬 디코딩에서 스레드별 진행 상황·통계 카운터를 배열 하나에 모아둘 때.

**권장**:
```rust
#[repr(align(64))]
pub struct PaddedCounter(pub AtomicU32);

pub struct ParallelDecodeStats {
    pub tiles_done: [PaddedCounter; 16], // 각 카운터가 독립 캐시라인 차지
}
```
- 각 카운터를 캐시라인 크기(보통 64바이트)로 정렬/패딩해 물리적으로 분리.
- 또는 스레드-로컬 카운터로 집계 후 마지막에 합산하는 방식으로 애초에 공유 캐시라인 경합을 없앰.

**탐지 방법**:
- Runtime: `perf c2c`(cache-to-cache) 분석으로 false sharing 핫스팟 식별.
- Structural: `[Atomic*; N]` 또는 `Mutex<[T; N]>` 같은 스레드별 카운터 배열 패턴을 grep, 패딩 유무 확인.

**예외**:
- 카운터 갱신 빈도가 낮거나(예: 프레임당 1회) 스레드 수가 1~2개뿐이라면 false sharing 영향이 미미해 패딩 투자가 불필요.

**Bitvue 판정**: N/A — 스레드/타일별 원자적 카운터 배열 패턴을 찾지 못함; 유일한 소형 원자 배열(`Arc<[AtomicU64; 2]>`, crates/bitvue-core/src/worker.rs:252)은 false sharing이 문제될 규모가 아니며 병렬 타일 디코드 카운터도 존재하지 않음.

---

### LAYOUT-021: 작고 흔한 컬렉션에 Vec 사용 (SmallVec 미적용)
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct Cu {
    pub ref_indices: Vec<u8>, // 대부분 0~2개, 최대 L0/L1 합쳐 4개 정도
    pub mvp_candidates: Vec<(i16, i16)>, // 보통 2~5개
}
```

**문제**:
- `Vec<T>`는 원소가 하나도 없어도 포인터+len+capacity(24바이트)를 차지하고, 원소 1개만 있어도 별도 힙 할당이 발생.
- 참조 인덱스 리스트, MVP 후보 리스트처럼 "거의 항상 아주 작은(≤4~8개)" 컬렉션이 CU/PU마다 생성되면, 힙 할당 횟수가 블록 수에 비례해 폭증.
- 이런 소형 `Vec`들이 흩어져 있으면 포인터를 따라가는 추가 역참조가 hot path마다 발생(LAYOUT-001과 유사한 문제가 컬렉션 필드 단위로 재발).

**발생 조건**:
- CU/PU 단위로 반복 생성되는 구조체 안에, 원소 개수 상한이 스펙상 작게 정해진 컬렉션 필드(참조 리스트, MVP 후보, 변환 계수 논제로 위치 등)가 있을 때.

**권장**:
```rust
use smallvec::SmallVec;

pub struct Cu {
    pub ref_indices: SmallVec<[u8; 4]>,
    pub mvp_candidates: SmallVec<[(i16, i16); 4]>,
}
```
- `SmallVec<[T; N]>`은 원소 수가 `N` 이하면 스택/인라인 버퍼에 저장하고, 초과할 때만 힙으로 스필오버.
- 스펙상 최대 개수(예: HEVC MVP는 최대 2개, 참조 리스트는 슬라이스 헤더에서 정해짐)를 `N`으로 설정하면 대부분의 경우 힙 할당이 완전히 사라짐.

**탐지 방법**:
- Structural: 블록/CU 단위 구조체 안의 `Vec<T>` 필드를 찾아 실제 관찰되는 최대 길이를 런타임 계측(히스토그램)으로 확인.
- Runtime: heaptrack/dhat으로 소형 `Vec` 할당 횟수와 평균 길이 측정, `SmallVec` 전환 전후 할당 횟수 비교.

**예외**:
- 원소 개수가 예측 불가능하게 크게 튈 수 있는 컬렉션(예: 매우 긴 CABAC 잔차 리스트)이라면 `SmallVec`의 인라인 버퍼가 오히려 구조체 크기만 키우는 역효과가 날 수 있음.

**Bitvue 판정**: N/A — `smallvec` 의존성이 없고, CU당 merge/AMVP 후보 리스트 같은 소형 컬렉션 필드도 없음 — Bitvue는 전체 모션보상 디코더가 아니라 오버레이 데이터 추출기.

---

### LAYOUT-022: Option<Vec<Option<T>>> 같은 중첩 nullable 레이어
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
pub struct SliceRefLists {
    // L0/L1 각각: 리스트 자체가 없을 수도(Option), 리스트가 있어도 각 항목이 없을 수도(Option)
    pub l0: Option<Vec<Option<RefPicEntry>>>,
    pub l1: Option<Vec<Option<RefPicEntry>>>,
}
```

**문제**:
- "없음"을 표현하는 계층이 3중(바깥 `Option`, `Vec` 자체의 empty, 안쪽 원소별 `Option`)으로 중복되어 실제 무엇이 "정상적으로 없는 것"인지 모호함.
- 안쪽 `Option<RefPicEntry>` 하나하나가 discriminant를 가지므로 LAYOUT-005와 같은 크기 낭비가 원소마다 반복됨.
- 이 데이터를 순회하는 코드가 `if let Some(list) = &lists.l0 { for item in list { if let Some(entry) = item { ... } } }`처럼 3중 중첩이 되어 가독성과 분기 예측 모두 나빠짐.

**발생 조건**:
- "혹시 몰라서" 방어적으로 Option을 겹겹이 감싸는 습관이 파서 리팩토링 과정에서 누적됐을 때, 특히 슬라이스 헤더의 참조 리스트처럼 옵셔널리티가 여러 단계로 존재하는 필드.

**권장**:
```rust
pub struct SliceRefLists {
    // 리스트가 없으면 빈 Vec으로 표현(빈 것과 없는 것을 구분할 실질적 의미가 없다면 통합)
    pub l0: Vec<RefPicEntry>,
    pub l1: Vec<RefPicEntry>,
}
// RefPicEntry 내부에 "이 항목이 유효한가"가 필요하다면 명시적 필드나 sentinel POC 값으로 표현
```
- "없음"의 의미를 하나로 통일: 리스트가 비어있으면 그냥 빈 `Vec`, 항목 자체가 무효라면 필드 안에 유효성 플래그를 명시적으로 둠.
- 정말 "리스트 존재 여부"와 "빈 리스트"를 구분해야 하는 스펙상 이유가 있다면 바깥 `Option` 하나만 유지하고 안쪽은 단순 `Vec<RefPicEntry>`로.

**탐지 방법**:
- Static: `Option<Vec<Option<` 또는 `Vec<Option<Option<` 같은 중첩 패턴을 타입 시그니처에서 grep.
- Manual: 코드 리뷰에서 "이 세 겹의 Option이 각각 다른 의미를 갖는가?"를 확인 — 대개 하나는 불필요.

**예외**:
- 세 계층이 실제로 서로 다른 의미(파싱 안 됨 / 파싱됐지만 빈 리스트 / 리스트는 있지만 개별 항목이 아직 결정 안 됨)를 가지며 모두 구분이 필요한 드문 경우.

**Bitvue 판정**: N/A — `Option<Vec<Option<T>>>` 중첩 패턴을 어디서도 찾지 못함; 참조 리스트류 구조체는 명시적 플래그와 함께 단순 `Vec<u8>`/`Vec<i16>` 사용(예: crates/bitvue-hevc/src/slice.rs:61-71).

---

### LAYOUT-023: 니블/비트필드로 packing 가능한 플래그를 개별 bool 필드로 분리
**분류**: LAYOUT · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
pub struct CuFlags {
    pub skip_flag: bool,
    pub merge_flag: bool,
    pub cu_transquant_bypass: bool,
    pub pcm_flag: bool,
    pub cu_qp_delta_present: bool,
    pub chroma_qp_offset_present: bool,
    pub prev_intra_luma_pred: bool,
    pub mpm_idx_present: bool,
}
// 8 bool = 8바이트, CU마다 8바이트를 플래그용으로만 소비
```

**문제**:
- bool 필드 8개가 각각 1바이트씩 차지 — 실제 정보량은 8비트(1바이트)면 충분한데 8배 낭비.
- CU/PU가 프레임당 수만 개 생성되는 구조에서 이 낭비가 곱연산으로 누적되어 전체 메모리 사용량과 캐시 점유율에 직접 영향.
- 플래그 8개를 각각 따로 읽고 쓰는 코드가 비트 연산 하나로 대체 가능한 경우가 많음에도 그대로 방치되는 경우가 흔함.

**발생 조건**:
- 스펙 문서의 신택스 엘리먼트를 그대로 1:1 bool 필드로 옮겨 적었을 때(특히 코덱 스펙에는 1비트 플래그가 수십 개씩 나열됨).

**권장**:
```rust
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct CuFlags: u8 {
        const SKIP                    = 1 << 0;
        const MERGE                   = 1 << 1;
        const TRANSQUANT_BYPASS       = 1 << 2;
        const PCM                     = 1 << 3;
        const CU_QP_DELTA_PRESENT     = 1 << 4;
        const CHROMA_QP_OFFSET        = 1 << 5;
        const PREV_INTRA_LUMA_PRED    = 1 << 6;
        const MPM_IDX_PRESENT         = 1 << 7;
    }
}
```
- `bitflags` 크레이트나 수동 비트마스크로 8개 bool을 `u8` 하나로 압축.
- 대량 배열(`Vec<CuFlags>`)로 저장 시 스캔 속도와 메모리 모두 8배 개선.

**탐지 방법**:
- Static: 구조체 안에 `bool` 필드가 4개 이상 연속 등장하는지 grep, `size_of`로 실제 절감 가능량 계산.
- Static: clippy에는 직접적 lint가 없으므로 커스텀 스크립트나 코드 리뷰 체크리스트로 보완.

**예외**:
- 플래그 각각이 서로 다른 시점에 독립적으로, 매우 드물게(구조체 인스턴스 수가 적게) 갱신된다면 bool 필드의 가독성이 비트 연산보다 유지보수에 유리할 수 있음.

**Bitvue 판정**: Suspected — LAYOUT-004와 동일 근거(bool 다수 구조체 존재, `bitflags` 크레이트 미사용)이지만 인스턴스가 시퀀스/프레임 단위라 카탈로그의 블록 단위 심각도 논리가 그대로 적용되지 않음.

---

### LAYOUT-024: 중첩 nullable 포인터 대신 sentinel/tagged 값이 나은 경우
**분류**: LAYOUT · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
pub struct MergeCandidate {
    pub mv: (i16, i16),
    pub ref_idx: Option<Box<u8>>, // "참조 인덱스가 없을 수도" 를 Box<u8>로 표현
}
```

**문제**:
- `u8` 하나를 감싸기 위해 `Option<Box<u8>>`을 쓰면 포인터(8바이트) + discriminant까지 붙어 원본 데이터(1바이트)보다 수십 배 큰 표현이 됨.
- 힙 할당이 병합 후보(merge candidate)마다 발생 — 병합 리스트는 블록당 최대 5개 후보를 프레임 전체에서 반복 생성하므로 할당 횟수가 상당함.
- `ref_idx`가 없을 수 있는 경우는 사실 스펙상 "유효하지 않음"을 뜻하는 특정 값(예: 255)으로 충분히 표현 가능한 경우가 대부분.

**발생 조건**:
- "옵셔널이니까 일단 Option으로 감싸고, 참조 타입이 필요할 것 같으니 Box도 씌우자"는 과도한 방어적 습관이 작은 값 타입에도 적용됐을 때.

**권장**:
```rust
pub struct MergeCandidate {
    pub mv: (i16, i16),
    pub ref_idx: u8, // 255 == "없음" (sentinel), 스펙상 유효 범위(0~15)와 겹치지 않음을 문서화
}
// 혹은 니치 최적화가 되는 타입을 활용:
pub struct MergeCandidate2 {
    pub mv: (i16, i16),
    pub ref_idx: Option<std::num::NonZeroU8>, // discriminant 없이 0 크기 오버헤드
}
```
- 값의 유효 범위가 좁고 "없음"을 나타낼 여유 비트 패턴이 있다면 sentinel 값이 가장 저렴.
- 니치 최적화가 가능한 `NonZero*` 계열을 쓰면 `Option`을 유지하면서도 크기 오버헤드 없이 안전성(sentinel 값과 실제 값의 혼동 방지)을 얻을 수 있음.

**탐지 방법**:
- Static: `Option<Box<`primitive`>>` 패턴을 grep — 대부분 과설계 신호.
- Manual: "이 필드의 유효 범위에 예약 가능한 값이 있는가?"를 리뷰에서 확인.

**예외**:
- 값 자체가 크고(예: 서브구조체) 복사 비용이 부담스러운 경우엔 `Box`가 정당화될 수 있음 — 다만 이 경우도 `Option<Box<T>>`이 니치 최적화로 포인터 크기 그대로 유지됨을 활용하면 됨.

**Bitvue 판정**: N/A — `Option<Box<primitive>>` 패턴을 찾지 못함; 오히려 sentinel 값을 이미 활용 중(`MISSING_MV`, crates/bitvue-core/src/mv_overlay.rs:11; `QPGrid.missing: i16`).

---

### LAYOUT-025: 컨테이너의 컨테이너의 컨테이너 (nested nested nested)
**분류**: LAYOUT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// 프레임 -> 타일 -> CTU row -> CU 리스트, 4중 Vec 중첩
pub struct FrameCuMap {
    pub tiles: Vec<Vec<Vec<Vec<Cu>>>>, // [tile][ctu_row][ctu_col][cu_in_ctu]
}

fn find_cu_at(map: &FrameCuMap, tile: usize, row: usize, col: usize, idx: usize) -> &Cu {
    &map.tiles[tile][row][col][idx]
}
```

**문제**:
- `Vec<Vec<Vec<Vec<T>>>>`는 각 레벨마다 별도 힙 할당 + 포인터 역참조 — 원소 하나에 접근하는 데 포인터를 4번 따라가야 함.
- 안쪽 `Vec`들이 서로 다른 시점에 다른 크기로 할당되므로 메모리상 전혀 인접하지 않음 — 인접 CU를 순회해도 사실상 랜덤 액세스.
- 각 레벨의 `Vec` 자체가 24바이트(포인터+len+cap) 오버헤드를 가지므로, 타일×row×col 개수만큼 이 오버헤드가 곱연산으로 늘어남(빈 타일이 많으면 특히 낭비).
- 경계 조건(타일 경계, CTU row 끝) 처리가 4중 인덱싱 코드 곳곳에 흩어져 버그 유발 지점이 많아짐.

**발생 조건**:
- 코덱의 계층적 파티셔닝(타일 → CTU → CU, 또는 슬라이스 → 매크로블록)을 그대로 중첩 컨테이너로 1:1 매핑했을 때.

**권장**:
```rust
pub struct FrameCuMap {
    pub cus: Vec<Cu>,                 // flat, 파싱 순서(래스터 순서)대로 저장
    pub tile_offsets: Vec<u32>,       // tile i의 CU 시작 인덱스 = tile_offsets[i]
    pub ctu_row_offsets: Vec<u32>,    // 필요하다면 CTU row 경계도 별도 오프셋 배열로
    pub width_ctus: u32,
    pub height_ctus: u32,
}
```
- 논리적 계층 구조는 "오프셋 배열"이나 좌표 계산식으로 표현하고, 실제 데이터는 단일 flat `Vec`에 저장.
- 계층 탐색이 필요하면 `tile_offsets[tile]..tile_offsets[tile+1]` 같은 슬라이스 범위로 얻어 그 안에서 순회 — 힙 할당은 전체에서 딱 1~2번.

**탐지 방법**:
- Structural: 타입 시그니처에서 `Vec<Vec<Vec<` 이상의 중첩을 grep으로 검색(3단계 이상이면 요주의).
- Runtime: 중첩 컨테이너 버전과 flat+offset 버전의 전체 프레임 순회 벤치마크 비교(할당 횟수는 heaptrack으로).

**예외**:
- 각 레벨의 크기가 극히 작고(예: 타일 수 2~4개, 인스턴스 수명도 짧은 일회성 임시 구조) 성능이 중요하지 않은 스크립트성 코드라면 중첩 Vec의 단순함이 더 나을 수 있음.

**Bitvue 판정**: N/A — 3단계 이상 중첩된 `Vec<Vec<Vec<...>>>`을 찾지 못함; 존재하는 `Vec<Vec<u8>>`은 가변 길이 NAL/샘플 페이로드 목록으로 정당한 용례이며, CU 컬렉션은 이미 flat `Vec<CodingUnit>` + 공간 인덱스로 구현됨.

---

### LAYOUT-026: 열(column) 접근이 필요한데 row 전체를 캐시에 올림
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct FrameBlockRecord {
    pub qp: u8,
    pub mb_type: MbType,
    pub mv: (i16, i16),
    pub ref_idx: i8,
    pub cbf: u8,
    pub tx_size: u8,
    pub intra_mode: u8,
    // ... 총 20바이트 안팎
}

pub struct FrameBlocks {
    pub records: Vec<FrameBlockRecord>, // width_blocks * height_blocks
}

// "QP 분포 히스토그램"만 계산하는데 record 전체(20바이트)를 다 읽음
fn qp_histogram(fb: &FrameBlocks) -> [u32; 64] {
    let mut hist = [0u32; 64];
    for r in &fb.records {
        hist[r.qp as usize] += 1;
    }
    hist
}
```

**문제**:
- LAYOUT-002/003과 근본 원인은 같지만, 여기서는 "record 하나가 이미 20바이트를 넘는 대형 구조체"라는 점이 다름 — 캐시라인(64B) 하나에 record가 3개 정도밖에 안 들어감.
- QP 히스토그램처럼 필드 하나(1바이트)만 읽는 연산에서 유효 대역폭이 1/20로 떨어짐 — row-major AoS의 전형적 실패 사례.
- UI에서 "레이어 토글" 방식으로 QP/MV/mb_type을 각각 따로 보여주는 뷰어일수록 이 열 단위 접근이 반복적으로 발생.

**발생 조건**:
- 통계/히트맵 계산처럼 "전체 블록에 대해 필드 하나만 훑는" 연산이 UI 상호작용(줌, 프레임 이동)마다 반복 실행될 때.

**권장**:
```rust
pub struct FrameBlocksSoA {
    pub qp: Vec<u8>,
    pub mb_type: Vec<MbType>,
    pub mv: Vec<(i16, i16)>,
    pub ref_idx: Vec<i8>,
    pub cbf: Vec<u8>,
    pub tx_size: Vec<u8>,
    pub intra_mode: Vec<u8>,
}
// qp_histogram은 fb.qp만 순회 — 캐시라인 하나에 QP 값 64개
```
- LAYOUT-003과 동일한 처방(SoA)이지만, record 크기가 클수록 AoS→SoA 전환의 이득이 더 커진다는 점을 강조 — 레코드가 크면 클수록 열 단위 스캔의 상대적 낭비가 커짐.
- 모든 필드를 동시에 쓰는 연산(예: "이 블록의 전체 정보를 툴팁에 표시")은 인덱스로 각 배열에서 값을 모아 조립.

**탐지 방법**:
- Runtime: 히스토그램/통계 계산 함수를 perf로 프로파일링, 구조체 크기 대비 유효 데이터 비율 계산.
- Semantic: record 크기(`size_of`)가 16바이트를 넘고 특정 필드만 읽는 순회 함수가 여러 개 있는지 검토.

**예외**:
- 통계/열 단위 스캔이 애초에 없고 항상 레코드 전체를 다루는 워크로드라면 AoS가 더 단순함.

**Bitvue 판정**: Confirmed — LAYOUT-002/003과 동일 근거: `Macroblock`/`CodingUnit` 레코드(Option 필드 포함 40바이트 이상 추정)를 그리드 추출마다 전부 순회하며 필드 하나만 읽음(crates/bitvue-avc/src/overlay_extraction.rs:176-181).

---

### LAYOUT-027: 정렬(alignment) 무시로 인한 misaligned 접근/불필요한 패딩
**분류**: LAYOUT · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
#[repr(C, packed)]  // FFI 크기 절약을 노렸지만
pub struct RawFrameHeader {
    pub flags: u8,
    pub timestamp: u64, // packed로 인해 8바이트 정렬이 깨짐
    pub width: u32,
}

fn read_timestamp(h: &RawFrameHeader) -> u64 {
    h.timestamp // packed 구조체에서 정렬 안 된 필드를 직접 참조 -> UB 위험/느린 접근
}
```

**문제**:
- `#[repr(C, packed)]`는 필드 간 패딩을 없애 구조체 크기는 줄이지만, 그 대가로 정렬이 깨진 필드에 대한 참조(`&h.timestamp`)를 만드는 것 자체가 미정의 동작(UB) 위험이 됨(Rust는 unaligned reference를 금지).
- 정렬이 깨진 필드를 안전하게 읽으려면 `read_unaligned`를 매번 써야 하는데, 이는 x86에서도 성능 페널티가 있고 일부 아키텍처(ARM 구세대 등)에서는 더 크게 느려짐.
- 반대로 크기를 줄이려는 목적이 없는데도 습관적으로 `packed`를 붙이거나, 반대로 정렬을 전혀 고려 않고 SIMD 대상 버퍼를 일반 `Vec::new()`로만 할당해 unaligned load가 강제되는 경우도 흔함.

**발생 조건**:
- 파일 포맷 헤더를 바이트 그대로 구조체에 매핑하려는 시도(`packed` 남용), 또는 SIMD 커널에 넘길 버퍼를 정렬 보장 없이 할당할 때.

**권장**:
```rust
#[repr(C)]
pub struct RawFrameHeader {
    pub timestamp: u64, // 큰 정렬 요구 필드를 앞으로 배치
    pub width: u32,
    pub flags: u8,
    // 컴파일러가 패딩을 자동 삽입, 정렬은 안전하게 보장됨
}

fn read_timestamp(h: &RawFrameHeader) -> u64 {
    h.timestamp // 안전한 정렬된 접근
}
```
- 바이트 스트림에서 파싱할 때는 `packed` 구조체에 직접 캐스팅하지 말고, 명시적으로 바이트를 읽어 필드별로 조립(`u64::from_le_bytes(...)`).
- SIMD 대상 버퍼는 `#[repr(align(32))]` 래퍼나 aligned allocator(`std::alloc::alloc`에 정렬 지정)로 할당.

**탐지 방법**:
- Static: `#[repr(packed)]` 사용처를 grep하고, 그 필드에 대한 참조(`&`)가 만들어지는지 clippy `unaligned_references` lint로 확인(최신 Rust는 컴파일 에러로 처리).
- Static: SIMD 관련 버퍼 할당 코드에서 정렬 보장 여부 리뷰.

**예외**:
- 파일 포맷 자체가 byte-packed이고, 필드 접근을 항상 `read_unaligned`/`from_le_bytes`로 안전하게 하는 규율이 코드베이스 전체에 일관되게 적용된 경우엔 `packed` 사용이 정당함.

**Bitvue 판정**: N/A — 코드베이스 전체에서 `#[repr(packed)]` 사용처를 찾지 못함.

---

### LAYOUT-028: 대형 구조체를 값으로 복사(Copy/Clone 남용)
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Clone, Copy)]
pub struct CtuSummary {
    pub qp_values: [u8; 64],       // 8x8 CU까지 세분화된 QP
    pub mb_types: [MbType; 64],
    pub mv_field: [(i16, i16); 64],
    // 총 300바이트 이상, 그런데도 #[derive(Copy)]
}

fn process_ctus(ctus: &[CtuSummary]) -> Vec<CtuSummary> {
    ctus.iter().map(|c| *c).filter(|c| has_skip(c)).collect() // 매 원소를 통째로 복사
}
```

**문제**:
- `Copy`를 파생시키면 함수 인자 전달, 클로저 캡처, 반복문 내 임시 변수 대입마다 암묵적으로 300바이트+ 복사가 발생 — 대형 구조체의 `Copy`는 "저렴한 복사"라는 `Copy`의 암묵적 계약을 위반.
- `*c`처럼 명시적 역참조 복사가 반복문 안에서 일어나면 스택/캐시 트래픽이 원소 크기에 비례해 커짐 — 필터링처럼 참조만으로 충분한 연산에서도 불필요한 복사가 강제됨.
- 함수 시그니처에 `CtuSummary`를 값으로 주고받는 습관이 퍼지면, 호출 스택 깊은 곳까지 대형 값이 계속 복사되며 전파됨.

**발생 조건**:
- 작은 구조체에 익숙해 습관적으로 `#[derive(Copy)]`를 붙였는데, 이후 필드가 추가되며 구조체가 커진 경우(CTU/CU 요약 정보처럼 고정 크기 배열을 필드로 가진 구조체에서 특히 흔함).

**권장**:
```rust
#[derive(Clone)] // Copy 제거, 필요한 곳만 명시적으로 clone()
pub struct CtuSummary {
    pub qp_values: [u8; 64],
    pub mb_types: [MbType; 64],
    pub mv_field: [(i16, i16); 64],
}

fn process_ctus(ctus: &[CtuSummary]) -> Vec<&CtuSummary> {
    ctus.iter().filter(|c| has_skip(c)).collect() // 참조만 필터링, 복사 없음
}
```
- 구조체 크기가 커지면(대략 캐시라인 2~3개, 128~256바이트 이상) `Copy` 파생을 재검토 — 참조(`&T`)나 인덱스 전달로 전환.
- 클리피의 `large_types_passed_by_value` lint를 활성화해 값으로 전달되는 대형 타입을 자동 검출.

**탐지 방법**:
- Static: `#[derive(Copy)]`가 붙은 타입 중 `size_of::<T>()`가 임계값(예: 64바이트)을 넘는 것을 CI에서 경고.
- Static: clippy `large_enum_variant`, `large_types_passed_by_value` lint.

**예외**:
- 구조체가 실제로 작고(≤32바이트 수준) 대입/전달 빈도가 매우 높다면 `Copy`가 참조 카운팅/수명 관리보다 오히려 저렴하고 단순함.

**Bitvue 판정**: N/A — 128바이트를 넘는 `#[derive(Copy)]` 구조체를 찾지 못함; 확인된 Copy 구조체(SummaryStats, WindowStats, CpbState)는 48~64바이트 수준의 작은 통계 집계형.

---

### LAYOUT-029: 오버레이 픽셀 배열에 색상/알파를 인터리브하지 않아 blend 시 캐시 미스
**분류**: LAYOUT · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct OverlayLayer {
    pub r: Vec<u8>,
    pub g: Vec<u8>,
    pub b: Vec<u8>,
    pub a: Vec<u8>, // 픽셀 수만큼, 4개의 별도 배열
}

fn blend_over_frame(layer: &OverlayLayer, frame_rgb: &mut [u8], width: usize, height: usize) {
    for i in 0..(width * height) {
        let alpha = layer.a[i] as u32;
        let idx = i * 3;
        frame_rgb[idx]   = blend(frame_rgb[idx],   layer.r[i], alpha);
        frame_rgb[idx+1] = blend(frame_rgb[idx+1], layer.g[i], alpha);
        frame_rgb[idx+2] = blend(frame_rgb[idx+2], layer.b[i], alpha);
        // 매 픽셀마다 4개의 서로 다른 배열(r,g,b,a)에 접근 -> 4개 캐시라인 스트림을 동시에 추적
    }
}
```

**문제**:
- LAYOUT-003과 반대 방향의 실수: 오버레이 블렌딩처럼 "픽셀 단위로 r,g,b,a를 항상 함께" 쓰는 연산에서는 SoA가 오히려 손해 — 픽셀 하나 처리에 4개의 독립된 스트림을 동시에 읽어야 해서 프리페처가 4배 더 많은 캐시라인을 추적해야 함.
- 목적지 `frame_rgb`(인터리브 RGB)와 소스(SoA RGBA)의 접근 패턴이 어긋나 있어, 결국 blend 루프가 두 가지 서로 다른 레이아웃 사이를 오가며 캐시 지역성을 모두 깨뜨림.
- "오버레이는 SoA가 무조건 좋다"는 규칙을 맹목적으로 적용하면, 실제로는 인터리브(AoS)가 더 나은 blend 같은 연산에서 역효과가 남.

**발생 조건**:
- 픽셀 단위로 여러 채널을 동시에 합성하는 연산(알파 블렌딩, 색상 변환)이 hot path일 때 — LAYOUT-003(필드 하나만 읽는 오버레이 스캔)과는 반대되는 접근 패턴.

**권장**:
```rust
pub struct OverlayLayer {
    pub rgba: Vec<[u8; 4]>, // 픽셀당 r,g,b,a를 인접 배치 (AoS)
}

fn blend_over_frame(layer: &OverlayLayer, frame_rgb: &mut [u8], width: usize, height: usize) {
    for (i, px) in layer.rgba.iter().enumerate() {
        let idx = i * 3;
        frame_rgb[idx]   = blend(frame_rgb[idx],   px[0], px[3] as u32);
        frame_rgb[idx+1] = blend(frame_rgb[idx+1], px[1], px[3] as u32);
        frame_rgb[idx+2] = blend(frame_rgb[idx+2], px[2], px[3] as u32);
    }
}
```
- 접근 패턴이 "필드 하나만 스캔"(SoA 유리, LAYOUT-003/026)인지 "레코드 전체를 항상 함께"(AoS 유리, 이 항목)인지 먼저 판별한 뒤 레이아웃을 선택.
- 같은 오버레이 데이터라도 "QP 히트맵 계산"과 "RGBA 합성"처럼 서로 다른 연산이 섞여 있다면, 용도별로 별도 표현을 두거나(예: QP는 SoA 별도 배열, 최종 합성용 색상만 AoS) 필요 시점에 변환하는 것도 방법.

**탐지 방법**:
- Runtime: blend/합성 루프의 캐시 미스율을 SoA(r,g,b,a 분리)와 AoS(인터리브) 버전으로 각각 측정해 비교.
- Manual: "이 데이터의 소비 연산이 필드 하나만 읽는가, 레코드 전체를 읽는가"를 설계 문서에 명시.

**예외**:
- 블렌딩 연산에도 SIMD 셔플/디인터리브 명령(예: `vld4`/AVX의 unpack 계열)을 활용해 SoA를 유지한 채 고성능을 낼 수 있는 경우 — 다만 구현 복잡도가 상당히 올라감.

**Bitvue 판정**: N/A — 오버레이 블렌딩은 프론트엔드 Canvas2D/WebGL에 위임됨(frontend/components/panels/OverlayRenderer/*); Rust나 TS 어디에도 채널별 r/g/b/a Vec을 수동으로 블렌딩하는 루프가 없음.

---

### LAYOUT-030: 트리 순회에서 부모 back-pointer를 항상 저장해 지역성 저하
**분류**: LAYOUT · **심각도**: Low · **탐지**: Structural

**나쁜 예**:
```rust
pub struct CuNode {
    pub qp: u8,
    pub children: Vec<Box<CuNode>>,
    pub parent: Option<*const CuNode>, // 상향 탐색을 위해 항상 저장
}
```

**문제**:
- 대부분의 순회(하향식 렌더링, 직렬화, 통계 집계)는 parent 포인터를 전혀 쓰지 않는데도, 모든 노드가 이 필드를 위한 8바이트를 상시 부담.
- parent 포인터는 자식 배열이 재할당(`Vec` growth, 또는 트리 재구성)될 때 댕글링되기 쉬워 unsafe 유지보수 부담을 늘림(LAYOUT-013과 유사한 함정).
- 상향 탐색이 정말 필요한 경우는 대개 특정 알고리즘(예: 쿼드트리 분할에서 이웃 CU 찾기)에 국한되는데, 이를 위해 모든 노드에 필드를 추가하는 것은 과도한 일반화.

**발생 조건**:
- "나중에 부모가 필요할 수도 있으니" 방어적으로 모든 트리 노드에 역방향 포인터를 미리 넣어두는 설계, CU/CTU 쿼드트리처럼 노드 수가 많은 트리에서.

**권장**:
```rust
pub struct CuNode {
    pub qp: u8,
    pub children: Vec<Box<CuNode>>, // parent 필드 제거
}

// 상향 탐색이 필요한 특정 알고리즘에서만 순회 중 스택으로 부모 경로를 구성
fn find_with_parent_stack(root: &CuNode, target_pred: impl Fn(&CuNode) -> bool)
    -> Option<Vec<&CuNode>> // 루트에서 타겟까지의 경로
{
    // DFS하며 Vec<&CuNode>를 스택처럼 push/pop — 필요한 순간에만 parent 정보 구성
    todo!()
}
```
- 상향 탐색이 필요한 알고리즘은 순회 시점에 임시 스택(경로)을 구성해 해결하고, 노드 자체에는 parent 필드를 두지 않음.
- 정말 빈번하게 상향 탐색이 필요하다면(예: 매 프레임 반복되는 특정 패턴) LAYOUT-001의 arena 표현으로 전환하고 부모 인덱스를 별도 배열(`parent_idx: Vec<u32>`)로 관리하는 편이 포인터보다 안전하고 저렴.

**탐지 방법**:
- Structural: 트리 노드 구조체에 `parent` 필드가 있는지, 그리고 실제로 이 필드를 읽는 코드가 얼마나 되는지(사용률) grep + 참조 카운트.
- Manual: "parent 필드를 쓰는 함수가 트리 관련 코드의 몇 %인가"를 조사해 낮으면 제거 후보로 표시.

**예외**:
- 상향 탐색이 트리 순회 코드의 대다수를 차지하는 워크로드(예: leaf-to-root 누적 통계를 프레임마다 반복 계산)라면 parent 포인터를 상시 유지하는 편이 매번 스택을 재구성하는 것보다 나을 수 있음.

**Bitvue 판정**: N/A — parent 링크가 있는 유일한 트리(`SyntaxNode.parent: Option<SyntaxNodeId>`, crates/bitvue-core/src/types.rs:528)는 원시 포인터가 아닌 String ID이고(댕글링 위험 없음), 실제로 hex↔syntax 역매핑이라는 실사용처가 있어 카탈로그가 인정하는 예외(상향 탐색이 실질적으로 필요한 경우)에 해당.
