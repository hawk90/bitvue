# Anti-Pattern Catalog — RPERF: Rust 성능 추상화

이 문서는 더 큰 안티패턴 카탈로그(Bitvue: Tauri + Rust + React 기반 AV1/HEVC/AVC/VP9/VVC/AV3 비트스트림 분석기)의 일부이며, Phase 4에 해당합니다. 전체 목록은 `docs/anti-patterns/INDEX.md`를 참고하십시오(Wave 1~3은 이미 작성됨, 재열람 불필요). Wave 1의 `MEM.md`/`LAYOUT.md`가 "메모리를 얼마나, 어떻게 배치해 쓰는가"(allocation, 데이터 레이아웃)를 다루고 `API_TYPE.md`가 API 표면 설계를 다뤘다면, 이 파일은 그보다 한 단계 아래—**Rust 언어 자체가 제공하는 추상화(trait object, iterator, `Drop`, 스마트 포인터, 표준 컬렉션 API)를 사용할 때 발생하는 "zero-cost가 아닌 비용"**에 초점을 맞춥니다. 즉 "얼마나 할당하는가"가 아니라 "얼마나 간접 호출·숨은 복사·해제 지연이 끼어드는가"의 문제입니다.

---

### RPERF-001: hot path에서 dyn Trait 호출
**분류**: RPERF · **심각도**: High · **탐지**: Static|Structural

**나쁜 예**:
```rust
trait MotionVectorPredictor {
    fn predict(&self, ctx: &MvContext) -> MotionVector;
}

struct HevcMvp;
struct Av1Mvp;
impl MotionVectorPredictor for HevcMvp { /* ... */
    fn predict(&self, ctx: &MvContext) -> MotionVector { todo!() }
}
impl MotionVectorPredictor for Av1Mvp { /* ... */
    fn predict(&self, ctx: &MvContext) -> MotionVector { todo!() }
}

struct MbDecoder {
    predictor: Box<dyn MotionVectorPredictor>,
}

impl MbDecoder {
    fn decode_frame(&self, mbs: &[MbContext]) -> Vec<MotionVector> {
        // 프레임당 수만 개 매크로블록, 매 블록마다 vtable 간접 호출
        mbs.iter().map(|ctx| self.predictor.predict(&ctx.mv_ctx)).collect()
    }
}
```

**문제**:
- `Box<dyn Trait>` 호출은 vtable을 거치므로 인라이닝이 불가능하고, 분기 예측기가 대상 함수를 학습하지 못해 매 호출마다 간접 점프 비용이 발생한다.
- 매크로블록/서브블록 단위로 초당 수백만 회 호출되는 루프에서는 이 간접 호출 자체가 누적되어 눈에 띄는 오버헤드가 된다.
- 컴파일러가 함수 본문을 볼 수 없으므로 루프 전체에 대한 벡터화·상수 전파 같은 최적화도 함께 막힌다.

**발생 조건**:
- 코덱별로 다른 예측/변환 로직을 "플러그인" 형태로 추상화하면서 trait object를 프레임/블록 루프 안쪽까지 끌고 들어갔을 때.
- 런타임에 코덱이 결정된다는 이유로, 실제로는 세션 시작 시 한 번만 정해지는 값을 매 블록마다 다시 디스패치할 때.

**권장**:
```rust
enum Predictor { Hevc(HevcMvp), Av1(Av1Mvp) }

impl Predictor {
    #[inline]
    fn predict(&self, ctx: &MvContext) -> MotionVector {
        match self {
            Predictor::Hevc(p) => p.predict_impl(ctx),
            Predictor::Av1(p) => p.predict_impl(ctx),
        }
    }
}

// 또는 제네릭으로 코덱 결정을 루프 바깥, 함수 경계로 끌어올린다.
fn decode_frame<P: MotionVectorPredictorImpl>(p: &P, mbs: &[MbContext]) -> Vec<MotionVector> {
    mbs.iter().map(|ctx| p.predict_impl(&ctx.mv_ctx)).collect()
}
```
- 코덱 종류처럼 값의 가짓수가 작고 고정적인 경우 `enum` + `match`로 정적 디스패치화한다(컴파일러가 인라인 가능).
- 정말 개방형 확장이 필요하면(플러그인 아키텍처) trait object는 프레임/GOP 단위 등 호출 빈도가 낮은 경계에만 두고, 그 안쪽 루프는 제네릭이나 enum으로 정적 디스패치한다.

**탐지 방법**:
- Static: `Box<dyn`, `&dyn`, `Arc<dyn` 선언이 `for`/`while` 루프 본문 안에서 매 반복 호출되는지 grep 후 호출 빈도 확인.
- Runtime: `perf record` 후 `perf annotate`에서 간접 호출(`call *%rax` 계열) 비중이 hot 함수에 몰려 있는지 확인. `cargo flamegraph`에서 vtable 경유 프레임이 넓게 나타나는지 확인.

**예외**:
- 프레임/패킷/파일 단위처럼 호출 빈도가 초당 수십~수백 회 이하인 경계에서는 dyn 디스패치 비용이 무시할 수준이며, 코드 단순성·바이너리 크기 이득이 더 크다.
- 코덱 종류가 런타임 플러그인으로 무한히 확장 가능해야 하는 설계 요구가 있는 경우.

**Bitvue 판정**: N/A — 발견된 모든 `Box<dyn`/`Arc<dyn` 사용(`parser_strategy.rs:804`, `overlay_factory.rs:275-287`, `index_extractor.rs:662`, `command_chain.rs`)은 파일/세션/팩토리 경계에서 1회성으로만 호출됨; 실제 per-block 처리부(`bitvue-av1-codec/src/tile/mv_prediction.rs`, `coding_unit.rs`)에는 dyn/trait object 자체가 없음(grep 0건).

---

### RPERF-002: iterator chain이 중간 allocation을 숨김
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Structural

**나쁜 예**:
```rust
fn collect_ref_frame_ids(gop: &[FrameHeader]) -> Vec<u32> {
    gop.iter()
        .map(|f| f.ref_indices.clone())          // Vec<u32> 복제
        .filter(|ids| !ids.is_empty())
        .flat_map(|ids| ids.into_iter().collect::<Vec<_>>()) // 다시 Vec으로
        .map(|id| id.to_string())                 // String 할당
        .map(|s| s.parse::<u32>().unwrap())        // 파싱 왕복
        .collect()
}
```

**문제**:
- 읽기 쉬운 `.map().filter().flat_map()` 체인 뒤에 `.clone()`, `String` 변환, 중간 `Vec` 수집이 숨어 있어 코드만 봐서는 할당 횟수를 가늠하기 어렵다.
- 각 단계가 lazy evaluation이라는 착각 때문에 "iterator니까 zero-cost"라고 방심하기 쉽지만, `clone()`이나 `collect::<Vec<_>>()`가 체인 중간에 섞이면 그 지점마다 실제 할당이 발생한다.
- `to_string()` → `parse()` 왕복처럼 원래 이미 갖고 있던 값을 문자열로 갔다가 되돌아오는 무의미한 변환이 리뷰에서 놓치기 쉽다.

**발생 조건**:
- 초기에는 단순했던 체인에 기능이 하나씩 추가되며 중간에 `.clone()`이나 포맷팅이 끼어들 때.
- 다른 타입 호환을 맞추기 위해 급하게 `.to_string()`/`.parse()`를 끼워 넣고 나중에 되돌리지 않았을 때.

**권장**:
```rust
fn collect_ref_frame_ids(gop: &[FrameHeader]) -> Vec<u32> {
    gop.iter()
        .flat_map(|f| f.ref_indices.iter().copied()) // clone 없이 값 복사(u32는 Copy)
        .collect()
}
```
- 체인 내부에서 `.clone()`, `.to_string()`, `.collect::<Vec<_>>()`가 등장하면 "왜 여기서 소유권이 필요한가"를 각각 질문한다.
- `Copy` 타입은 `.copied()`/`.cloned()` 대신 참조 반복 후 필요한 시점에만 복사하도록 체인을 재배치한다.
- 체인이 길어지면 `cargo expand`나 `#[inline(never)]` 벤치마크로 실제 어셈블리/할당 횟수를 검증한다.

**탐지 방법**:
- Static: iterator 체인 안의 `.clone()`, `.to_string()`, `.collect()` 호출 위치 grep.
- Runtime: `dhat`/`heaptrack`으로 해당 함수 호출 시 할당 횟수 프로파일링, 체인 길이 대비 할당 수가 비정상적으로 많은지 확인.

**예외**:
- GOP/프레임 헤더 목록처럼 호출 빈도가 낮고(파일 열기 시 1회 등) 데이터 크기도 작은 경로에서는 가독성이 우선이며 최적화가 불필요하다.

**Bitvue 판정**: Suspected — evidence 기록 코드에서 `.map(|x| x.field.clone())` 형태 체인 다수 확인(`core.rs:111`, `reference_graph_evidence.rs:180,196`, `index_session_evidence.rs:137,218`, `player_evidence.rs:330`); 다만 to_string()→parse() 왕복이나 다단 collect는 grep 0건이라 원문 예시만큼 심각하진 않고 호출 빈도(hot path 여부)도 미확인.

---

### RPERF-003: collect 후 다시 순회
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Structural

**나쁜 예**:
```rust
fn compute_frame_stats(nals: &[NalUnit]) -> FrameStats {
    let sizes: Vec<usize> = nals.iter().map(|n| n.payload.len()).collect();
    let total: usize = sizes.iter().sum();          // 재순회 1
    let max = sizes.iter().max().copied().unwrap_or(0); // 재순회 2
    let avg = total as f64 / sizes.len() as f64;
    FrameStats { total, max, avg }
}
```

**문제**:
- `collect()`로 중간 `Vec<usize>`를 만든 뒤 `sum()`, `max()`를 각각 별도로 다시 순회하므로, 원본을 한 번 훑으면 끝날 계산에 불필요한 힙 할당과 3회의 순회가 들어간다.
- 데이터가 클수록(예: 프레임당 NAL 수천 개) 캐시에서 이미 밀려난 `sizes` 벡터를 다시 스캔하는 비용이 누적된다.
- 통계 항목이 늘어날 때마다 재순회가 하나씩 늘어나는 패턴으로 굳어지기 쉽다.

**발생 조건**:
- "일단 다 모아놓고 나중에 여러 통계를 계산하자"는 순서로 코드를 짤 때.
- `sum()`/`max()`/`avg` 같은 여러 집계를 각각 별도 iterator 메서드로 구현하다가 매번 원본을 순회하게 됐을 때.

**권장**:
```rust
fn compute_frame_stats(nals: &[NalUnit]) -> FrameStats {
    let (total, max, count) = nals.iter().fold((0usize, 0usize, 0usize), |(sum, max, n), nal| {
        let size = nal.payload.len();
        (sum + size, max.max(size), n + 1)
    });
    let avg = if count == 0 { 0.0 } else { total as f64 / count as f64 };
    FrameStats { total, max, avg }
}
```
- 여러 집계값이 필요하면 `fold` 한 번으로 튜플/구조체에 누적하거나, `itertools::multiunzip` 등으로 단일 패스에서 계산한다.
- 중간 결과를 정말 재사용해야 한다면(예: 정렬 후 median도 필요) `collect()`가 정당화되지만, 그렇지 않다면 단일 패스 fold를 기본으로 삼는다.

**탐지 방법**:
- Static: `let x: Vec<_> = ...collect();` 다음 줄들에서 `x.iter()`가 반복적으로 등장하는 패턴 grep.
- Structural: 같은 함수 내에서 동일 컬렉션에 대한 `.iter()` 호출이 2회 이상인지 린트로 검출.

**예외**:
- 정렬, 중복 제거, 인덱스 접근 등 컬렉션 형태 자체가 필요한 연산이 섞여 있다면 `collect()` 후 재순회가 불가피하며 적절하다.

**Bitvue 판정**: Confirmed — `crates/bitvue-core/src/block_metrics.rs:247-260` `BlockMetricsStatistics::from_grid`가 동일한 `values: Vec<f32>`(프레임당 블록 그리드)를 min/max/sum/variance(map+sum 재순회)/threshold-filter까지 5회 별도 순회. 단일 fold로 대체 가능한 정확히 이 패턴.

---

### RPERF-004: 작은 함수까지 과도한 generic으로 code bloat
**분류**: RPERF · **심각도**: Low · **탐지**: Static|Structural

**나쁜 예**:
```rust
fn clamp_sample<T: PartialOrd + Copy>(v: T, lo: T, hi: T) -> T {
    if v < lo { lo } else if v > hi { hi } else { v }
}

// 8bit/10bit/12bit/16bit 각 픽셀 포맷, i16/i32/u16/u32 조합으로
// 실제로는 4~5가지 타입에 대해서만 호출되지만 제네릭이라
// 호출 지점마다(모듈마다) monomorphization으로 별도 코드가 찍혀 나온다.
fn apply_lut<T: PartialOrd + Copy + Into<i64>>(samples: &mut [T], lut: &[T]) {
    for s in samples.iter_mut() {
        *s = lut[(*s).into() as usize];
    }
}
```

**문제**:
- 사소한 유틸 함수까지 제네릭화하면 호출되는 구체 타입 수만큼 코드가 복제(monomorphization)되어 바이너리 크기가 커지고, 명령어 캐시(I-cache) 압박이 커진다.
- 컴파일 시간도 타입 조합 수에 비례해 늘어나며, LTO 단계에서 동일 로직의 복제본을 병합하는 비용도 커진다.
- 코드 크기 증가가 icache miss로 이어지면 오히려 hot loop 성능이 제네릭화 이전보다 나빠질 수 있다 — "제네릭 = 항상 빠름"이라는 가정이 깨지는 지점.

**발생 조건**:
- 픽셀 포맷/비트 depth마다 다른 정수 타입(`u8`/`u16`/`i16`.../`u32`)을 다루는 저수준 함수를 습관적으로 제네릭화할 때.
- 실제 사용처가 2~3가지 타입뿐인데 "나중에 확장될 수도 있으니" 제네릭으로 미리 열어둘 때.

**권장**:
```rust
// 얇은 제네릭 wrapper는 구체 타입 함수로 즉시 위임 (monomorphized 코드는 최소화)
#[inline]
fn apply_lut_u16(samples: &mut [u16], lut: &[u16]) {
    for s in samples.iter_mut() {
        *s = lut[*s as usize];
    }
}
#[inline]
fn apply_lut_u8(samples: &mut [u8], lut: &[u8]) {
    for s in samples.iter_mut() {
        *s = lut[*s as usize];
    }
}
```
- 실제로 쓰이는 구체 타입이 소수(2~4개)라면 제네릭 대신 개별 함수 또는 매크로로 생성한 개별 함수를 사용해 각 함수가 독립적으로 최적화·인라인되게 한다.
- 정말 제네릭이 필요하면 `#[inline(never)]` 처리한 공통 core 함수를 두고, 얇은 제네릭 wrapper가 그것을 호출하도록 해 monomorphization 대상 코드량을 최소화한다.
- `cargo bloat --release`로 실제 코드 크기 기여도를 확인 후 결정한다.

**탐지 방법**:
- Static: `cargo bloat --release --crates`로 monomorphization 비중이 큰 제네릭 함수 식별.
- Structural: 픽셀/샘플 처리 유틸 중 제네릭 타입 파라미터 수 대비 실제 호출 사이트의 구체 타입 종류 수 비교.

**예외**:
- 호출 지점이 매우 많고 타입 종류도 많아 코드 중복이 오히려 유지보수 부담이 되는 경우, 또는 hot loop이 아닌 초기화/설정 코드에서는 제네릭의 유지보수 이점이 코드 크기 비용을 상회한다.

**Bitvue 판정**: N/A — 코덱 크레이트(`bitvue-hevc`/`bitvue-avc`/`bitvue-av1-codec`) 내에 픽셀/샘플 처리용 제네릭 유틸 함수(`fn ... <T: ... Copy>` 류) grep 0건. 저수준 샘플 처리는 구체 타입으로 작성되어 있어 이 패턴이 발생할 표면 자체가 아직 없음.

---

### RPERF-005: 모든 데이터에 serde derive
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Structural

**나쁜 예**:
```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MacroblockInfo {
    pub mb_type: u8,
    pub qp: i8,
    pub mv: [(i16, i16); 4],
    pub ref_idx: [i8; 4],
    pub coeffs: Vec<i16>, // 최대 수백 개
}

// 프레임당 수만 개 MacroblockInfo를 프론트엔드로 보내기 위해
// Tauri IPC 경계에서 매번 JSON 직렬화
#[tauri::command]
fn get_mb_overlay(frame_idx: usize) -> Vec<MacroblockInfo> {
    let mbs = decode_mb_grid(frame_idx); // 수만 개
    mbs // serde_json으로 통째 직렬화되어 프론트엔드로 전송
}
```

**문제**:
- `derive(Serialize, Deserialize)`를 습관적으로 모든 내부 구조체에 붙이면, 실제로는 IPC 경계를 넘지 않는 순수 내부 계산용 타입까지 직렬화 코드가 생성되어 코드 크기와 컴파일 시간이 늘어난다.
- 매크로블록처럼 프레임당 수만 개가 생성되는 고빈도 구조체를 JSON으로 직렬화하면, `serde_json`의 텍스트 포맷 오버헤드(숫자→문자열 변환, 필드명 반복 직렬화)가 실제 데이터 크기 대비 수 배로 부풀어 IPC 페이로드와 직렬화 시간을 모두 늘린다.
- "직렬화 가능하다"는 사실이 "직렬화해도 괜찮다"는 뜻으로 오인되어, 정말 필요한 요약/다운샘플 없이 원본 그레인 그대로 경계를 넘게 된다.

**발생 조건**:
- Tauri command 반환 타입을 만들 때 일단 `derive(Serialize)`부터 붙이고 시작할 때.
- 디버깅 목적으로 임시로 붙인 `Serialize`가 그대로 프로덕션 코드에 남아 hot path 구조체에 부착될 때.

**권장**:
```rust
// 내부 계산용 구조체에는 serde derive를 붙이지 않는다.
pub struct MacroblockInfo {
    pub mb_type: u8,
    pub qp: i8,
    pub mv: [(i16, i16); 4],
    pub ref_idx: [i8; 4],
    pub coeffs: Vec<i16>,
}

// IPC 경계 전용 뷰 타입을 별도로 두고, 바이너리 포맷 또는 다운샘플된 요약만 직렬화
#[derive(Serialize)]
pub struct MbOverlayRow {
    pub mb_type: u8,
    pub qp: i8,
}

#[tauri::command]
fn get_mb_overlay(frame_idx: usize) -> Vec<MbOverlayRow> {
    decode_mb_grid(frame_idx).iter().map(MbOverlayRow::from).collect()
}
```
- IPC로 넘어가는 타입과 내부 연산용 타입을 분리하고, `Serialize`는 실제로 경계를 넘는 "뷰" 타입에만 붙인다.
- 고빈도·대용량 데이터(오버레이 그리드 등)는 JSON 대신 typed array/바이너리 버퍼(`Uint8Array` 등)로 프론트엔드에 전달하는 것을 우선 검토한다(관련 상세는 IPC.md 참고).

**탐지 방법**:
- Static: `#[derive(...Serialize...)]`가 붙은 구조체 목록과, 그 구조체가 실제 `#[tauri::command]` 반환 경로에 등장하는지 교차 검증.
- Structural: 프레임/블록 단위로 생성 빈도가 높은 타입 중 `Serialize`가 붙은 것을 우선순위로 점검.

**예외**:
- 설정, 프로젝트 파일, 세션 메타데이터처럼 저빈도·소용량 데이터는 편의상 derive를 붙이는 것이 합리적이다.

**Bitvue 판정**: Confirmed (변형) — 원문의 "MacroblockInfo 대량 직렬화"는 grep상 없지만, 같은 근본 문제(대용량 바이너리를 JSON 경계로 강제 통과)는 존재: `src-tauri/src/commands/frame.rs:16-47`의 `DecodedFrameData`/`YUVFrameData`가 풀해상도 프레임/YUV 플레인 전체를 base64 `String` 필드로 감싸 `#[derive(Serialize)]`된 채 IPC로 전송(base64 오버헤드 + JSON 문자열 이스케이핑 비용 추가).

---

### RPERF-006: HashMap 기본 hasher를 hot path에서 사용
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Runtime

**나쁜 예**:
```rust
use std::collections::HashMap;

struct RefPicCache {
    frames: HashMap<u32, DecodedFrame>, // 기본 SipHash
}

impl RefPicCache {
    fn get(&self, poc: u32) -> Option<&DecodedFrame> {
        // 초당 수백~수천 회 조회되는 참조 프레임 캐시
        self.frames.get(&poc)
    }
}
```

**문제**:
- 표준 라이브러리 `HashMap`의 기본 해셔는 HashDoS 방어를 위한 SipHash 계열로, 암호학적 안전성 때문에 단순 정수 키에도 상대적으로 무거운 연산을 수행한다.
- `u32` POC(picture order count), NAL 타입, 프레임 인덱스처럼 신뢰할 수 있는 내부 키에는 이런 방어가 불필요한 비용이다.
- hot path(참조 프레임 조회, 캐시 lookup)에서 초당 수천~수만 회 호출되면 해시 계산 자체가 무시할 수 없는 비중을 차지한다.

**발생 조건**:
- 참조 프레임 캐시, GOP 내 POC→프레임 매핑, NAL 타입별 통계 집계처럼 정수/작은 키를 hot path에서 반복 조회할 때.
- "일단 `HashMap::new()`로 시작"한 뒤 나중에 hasher를 바꾸는 것을 잊었을 때.

**권장**:
```rust
use rustc_hash::FxHashMap; // 또는 ahash::AHashMap

struct RefPicCache {
    frames: FxHashMap<u32, DecodedFrame>,
}

impl RefPicCache {
    fn get(&self, poc: u32) -> Option<&DecodedFrame> {
        self.frames.get(&poc)
    }
}

// 키 공간이 작고 조밀하면 HashMap 대신 Vec/슬라이스 인덱싱도 고려
struct RefPicCacheDense {
    frames: Vec<Option<DecodedFrame>>, // poc % window_size로 직접 인덱싱
}
```
- 외부 입력에 의해 키가 결정되지 않는(HashDoS 위협이 없는) 내부 hot path 캐시는 `rustc_hash::FxHashMap`이나 `ahash`처럼 비암호화 고속 해셔로 교체한다.
- 키 범위가 좁고 조밀하면 `HashMap` 자체를 걷어내고 배열/링버퍼 인덱싱으로 대체하는 것이 더 빠르고 단순하다.
- 외부(네트워크/파일 입력에서 직접 파생된) 키를 다루는 맵은 SipHash 기본값을 유지해 HashDoS에 노출되지 않도록 한다.

**탐지 방법**:
- Static: `HashMap::new()`/`HashMap::default()` 선언 위치와 호출 빈도(루프 내부 여부) 교차 확인.
- Runtime: `perf`로 hot 함수 중 `siphash`/`Hash::hash` 관련 심볼이 상위에 나타나는지 확인.

**예외**:
- 조회 빈도가 낮은 설정 맵, 사용자 입력 기반 키(외부 공격자가 키를 조작해 해시 충돌을 유발할 수 있는 경로)에는 기본 SipHash 유지가 안전하다.

**Bitvue 판정**: Confirmed — 코드베이스 전체에 `FxHashMap`/`ahash`/`rustc_hash` 의존성/사용 grep 0건. 재생/탐색 중 조회되는 내부 프레임 인덱스 맵(`player/frame_mapper.rs:76,79 display_to_decode/decode_to_display`, `player/h264_quirks.rs:33`, `player/av1_quirks.rs:102,280`)이 전부 표준 `HashMap`(SipHash) 사용. 다만 조회 빈도는 원문의 "초당 수천~수만"보다는 재생 프레임레이트(수십 Hz) 수준으로 낮음.

---

### RPERF-007: String format을 반복 생성
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Runtime

**나쁜 예**:
```rust
fn build_tree_labels(nodes: &[UnitNode]) -> Vec<String> {
    nodes.iter().map(|n| {
        // 트리 뷰 수만 개 노드마다 format! 호출 → 매번 String 할당
        format!("{} (offset=0x{:08x}, size={})", n.name, n.offset, n.size)
    }).collect()
}

fn log_every_nal(nal: &NalUnit) {
    // 로깅이 비활성화되어 있어도 format! 인자 평가와 String 생성이 항상 일어남
    log::debug!("{}", format!("NAL type={} size={}", nal.nal_type, nal.payload.len()));
}
```

**문제**:
- `format!`은 항상 새 `String` 힙 할당을 수반한다. 트리 뷰/hex 뷰처럼 수만 개 노드에 라벨을 붙이는 경로에서 매 노드마다 호출되면 할당 횟수가 그대로 노드 수에 비례한다.
- `log::debug!("{}", format!(...))`처럼 매크로 안에 `format!`을 다시 감싸면, 로그 레벨이 비활성화된 상태에서도(로깅 매크로가 인자를 lazy 평가하도록 설계되어 있음에도) 바깥의 `format!` 호출 자체는 무조건 실행되어 이점이 사라진다.
- UI 트리처럼 화면에 보이는 범위만 필요한 경우에도 전체 노드에 대해 미리 문자열을 만들어두면 스크롤하지 않는 부분까지 낭비가 발생한다.

**발생 조건**:
- 트리/hex 뷰 등 대량 노드에 표시용 라벨을 미리 전부 생성해둘 때.
- 로깅 매크로 사용법을 오해해 `format!`을 직접 인자로 넘길 때.
- 반복 루프 안에서 에러 메시지나 디버그 문자열을 미리 만들어 조건부로만 쓸 때.

**권장**:
```rust
use std::fmt;

struct NodeLabel<'a>(&'a UnitNode);
impl fmt::Display for NodeLabel<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (offset=0x{:08x}, size={})", self.0.name, self.0.offset, self.0.size)
    }
}

// 필요한 시점(렌더링 시, 화면에 보이는 범위만)에만 문자열화
fn render_visible(nodes: &[UnitNode], visible_range: std::ops::Range<usize>) -> Vec<String> {
    nodes[visible_range].iter().map(|n| NodeLabel(n).to_string()).collect()
}

// 로깅은 매크로 인자로 직접 값을 넘겨 lazy 평가를 살린다.
fn log_every_nal(nal: &NalUnit) {
    log::debug!("NAL type={} size={}", nal.nal_type, nal.payload.len());
}
```
- 표시용 라벨은 `Display` 구현으로 지연시키고, 실제로 화면에 그려지는(가상 스크롤 기준 visible) 범위에서만 문자열화한다.
- 로깅은 매크로에 포맷 인자를 직접 전달해 로그 레벨 비활성 시 포맷팅 자체가 스킵되도록 한다.
- 반복 호출되는 에러 경로는 `String` 대신 `&'static str` 또는 enum 에러 코드로 대체하고, 문자열화는 최종 출력 지점에서만 수행한다.

**탐지 방법**:
- Static: 루프 본문 내 `format!`, `to_string()` 호출과 `log::*!("{}", format!(...))` 패턴 grep.
- Runtime: `dhat`으로 `alloc::string::String` 할당 콜스택 상위 빈도 확인.

**예외**:
- 에러 경로, 파일 열기 시 1회 생성되는 메타데이터 요약처럼 호출 빈도가 낮은 곳에서는 가독성이 우선이다.

**Bitvue 판정**: Suspected — `player_evidence.rs:75,127,178,230,286`, `reference_graph_evidence.rs:105,138` 등에서 블록/노드 단위 `format!` 라벨 생성이 존재하나 호출 빈도(전체 블록 vs 화면 표시분만)를 확인하지 못함. `log::debug!("{}", format!(...))` 이중 래핑 패턴은 grep 0건으로 로깅 쪽 절반은 N/A.

---

### RPERF-008: enum match 안에서 대형 값 이동
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Structural

**나쁜 예**:
```rust
enum FrameHeader {
    Avc(AvcSliceHeader),   // 200+ 바이트
    Hevc(HevcSliceHeader), // 300+ 바이트
    Av1(Av1FrameHeader),   // 400+ 바이트, 내부에 배열 다수
}

fn process(headers: Vec<FrameHeader>) -> Vec<Summary> {
    headers.into_iter().map(|h| {
        match h {
            // 매 반복마다 가장 큰 variant 크기만큼 스택 이동이 발생
            FrameHeader::Avc(hdr) => summarize_avc(hdr),
            FrameHeader::Hevc(hdr) => summarize_hevc(hdr),
            FrameHeader::Av1(hdr) => summarize_av1(hdr),
        }
    }).collect()
}
```

**문제**:
- Rust enum의 크기는 가장 큰 variant + discriminant로 고정되므로, `Av1FrameHeader`처럼 무거운 variant가 하나만 있어도 `FrameHeader` 전체가 그 크기로 커진다.
- `match`로 값을 by-value 이동(consume)할 때마다 실제로 필요한 데이터(예: `AvcSliceHeader` 200바이트)만이 아니라 enum 전체 크기(400+ 바이트)만큼 `memcpy`가 발생할 수 있다.
- `Vec<FrameHeader>`처럼 이런 enum을 대량으로 담는 컬렉션은 각 원소가 최대 variant 크기로 패딩되어 실제 필요 메모리 대비 부풀어 있고, 순회 시 캐시 효율도 떨어진다(LAYOUT.md의 padding 이슈와 맞닿아 있으나, 여기서는 "매치 시 이동 비용"에 초점).

**발생 조건**:
- 코덱별 헤더처럼 variant 크기 편차가 큰 enum을 값으로(참조가 아니라) 대량 순회할 때.
- 헤더 파싱 결과를 담는 enum이 점점 필드가 늘어나며 특정 variant만 비대해질 때.

**권장**:
```rust
enum FrameHeader {
    Avc(Box<AvcSliceHeader>),
    Hevc(Box<HevcSliceHeader>),
    Av1(Box<Av1FrameHeader>),
}

fn process(headers: &[FrameHeader]) -> Vec<Summary> {
    headers.iter().map(|h| {
        match h {
            // 참조로 매치 + Box 간접 접근: enum 자체는 포인터 크기로 고정
            FrameHeader::Avc(hdr) => summarize_avc(hdr),
            FrameHeader::Hevc(hdr) => summarize_hevc(hdr),
            FrameHeader::Av1(hdr) => summarize_av1(hdr),
        }
    }).collect()
}
```
- variant 크기 편차가 크면 무거운 variant를 `Box`로 감싸 enum 전체 크기를 포인터 크기 수준으로 고정한다.
- 소유권 이동이 꼭 필요하지 않다면 `match &h` / `match h.as_ref()`처럼 참조 매치로 바꿔 불필요한 값 복사를 없앤다.
- `static_assertions::assert_eq_size!` 또는 `std::mem::size_of::<FrameHeader>()` 테스트로 enum 크기를 회귀 감시한다.

**탐지 방법**:
- Static: `size_of::<Enum>()`를 CI 테스트에 넣어 임계값(예: 64바이트) 초과 시 실패시키는 정적 가드.
- Structural: enum variant들의 필드 크기 총합 편차가 큰(예: 최대/최소 5배 이상) 타입 목록화.

**예외**:
- 컬렉션에 대량으로 담기지 않고 단발성으로만 쓰이는 enum(예: 함수 반환값 1개)은 크기 편차가 있어도 실질 영향이 없다.

**Bitvue 판정**: N/A — 유일하게 발견된 코덱-교차 enum `CodecMetadata`(`crates/bitvue-core/src/frame.rs:226`)는 variant 내부에 "simplified" `AvcSliceInfo`/`HevcSliceInfo`(String+소형 필드 1~2개)만 담아 크기 편차가 미미함. 실제 풀사이즈 슬라이스 헤더를 한 enum에 모아 대량 컬렉션에 담는 사례는 grep상 없음.

---

### RPERF-009: Option<Result<T>> 중첩으로 분기 복잡도 증가
**분류**: RPERF · **심각도**: Low · **탐지**: Static|Semantic

**나쁜 예**:
```rust
fn find_and_parse_sps(nals: &[NalUnit]) -> Option<Result<Sps, ParseError>> {
    nals.iter()
        .find(|n| n.nal_type == NalType::Sps)
        .map(|n| parse_sps(&n.payload))
}

fn use_it(nals: &[NalUnit]) {
    match find_and_parse_sps(nals) {
        Some(Ok(sps)) => apply_sps(sps),
        Some(Err(e)) => log::warn!("SPS parse failed: {e}"),
        None => log::warn!("no SPS found"),
        // 호출부마다 이 3-way 분기를 반복해서 다뤄야 함
    }
}
```

**문제**:
- `Option<Result<T, E>>`는 "없음", "있지만 실패", "있고 성공" 세 가지 상태를 표현하지만, 이는 사실상 `Result<T, E>`에서 "찾지 못함"도 하나의 에러 variant로 흡수 가능한 경우가 많아 불필요하게 타입이 복잡해진다.
- 호출부마다 4가지 매치 팔(`Some(Ok)`, `Some(Err)`, `None`, 그리고 컴파일러가 강제하는 완전성)을 반복해서 다뤄야 하므로 분기 로직이 호출 지점마다 중복되고, 실수로 `None`과 `Some(Err)`를 같은 방식으로 처리(또는 반대로 다르게 처리해야 하는데 놓침)하는 버그가 생기기 쉽다.
- `?` 연산자를 자연스럽게 체이닝할 수 없어(양쪽 다 감싸야 함) 에러 전파 코드가 장황해지고, 컴파일러의 분기 예측/인라인 힌트에도 불리한 다단 매치 구조가 굳어진다.

**발생 조건**:
- "찾아서 파싱"처럼 두 단계 연산을 `Option::map`으로 이어붙이다가 자연스럽게 `Option<Result<_>>`가 만들어질 때.
- 여러 개발자가 각자 다른 관례로 "없음"과 "실패"를 구분하려다 타입이 뒤섞일 때.

**권장**:
```rust
#[derive(Debug)]
enum SpsError {
    NotFound,
    Parse(ParseError),
}

fn find_and_parse_sps(nals: &[NalUnit]) -> Result<Sps, SpsError> {
    let nal = nals.iter().find(|n| n.nal_type == NalType::Sps).ok_or(SpsError::NotFound)?;
    parse_sps(&nal.payload).map_err(SpsError::Parse)
}

fn use_it(nals: &[NalUnit]) -> Result<(), SpsError> {
    let sps = find_and_parse_sps(nals)?; // 단일 ? 로 전파
    apply_sps(sps);
    Ok(())
}
```
- "없음"도 실패의 한 형태로 볼 수 있다면 `Result<T, E>` 하나로 통합하고 `E`에 `NotFound` variant를 추가한다.
- "없음"과 "실패"를 정말 다른 축으로 다뤄야 한다면(예: "없음은 정상, 실패만 에러") `Option<T>`와 별개의 에러 채널을 분리하는 두 함수로 나누는 편이 `Option<Result<T,E>>` 하나보다 명확하다.
- `?` 연산자가 자연스럽게 체이닝되는 형태를 목표로 설계한다.

**탐지 방법**:
- Static: 함수 시그니처에서 `Option<Result<` 패턴 grep.
- Semantic: 코드 리뷰에서 "이 None과 Err는 호출부가 실제로 다르게 처리하는가"를 질문해 아니라면 병합 유도.

**예외**:
- "없음"과 "실패"가 호출부마다 명확히 다른 처리(재시도 vs 즉시 중단 등)를 요구하고, 이 구분이 도메인상 핵심 정보인 경우에는 유지가 타당하다.

**Bitvue 판정**: N/A — 유일한 사례 `bitvue-av1-codec/src/obu.rs:477 next_obu_with_offset`는 같은 파일의 `Iterator` impl(`Item = Result<Obu>`)과 동일한 관용적 시그니처(`Option`=스트림 끝, `Result`=파싱 실패)로, 표준 Rust Iterator 컨벤션과 일치 — 문서 자체가 명시한 예외("None/Err가 실제로 다르게 처리돼야 하는 경우")에 해당.

---

### RPERF-010: Drop 비용이 중요한 객체의 해제 시점 불명확
**분류**: RPERF · **심각도**: High · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
struct DecoderSession {
    handle: *mut ffi::DecoderHandle, // libdav1d 등 FFI 디코더 컨텍스트
}

impl Drop for DecoderSession {
    fn drop(&mut self) {
        unsafe { ffi::decoder_close(self.handle) }; // 수십~수백 ms 걸릴 수 있음
    }
}

fn switch_codec(state: &mut AppState, new_codec: CodecKind) {
    // 이전 세션이 어디서 드롭되는지 코드만 봐서는 불명확
    state.session = DecoderSession::new(new_codec);
    // 위 대입 순간, 이전 state.session의 Drop이 "그 자리에서" 즉시 실행됨
    // (동기 호출 스레드가 UI 이벤트 핸들러라면 그 스레드가 그대로 멈춘다)
}
```

**문제**:
- FFI 디코더 핸들 정리, 대형 캐시 해제, 파일 핸들 flush처럼 `Drop`이 실제로 무거운 작업을 수행하는 타입은, 그 값이 스코프를 벗어나는 "정확히 어느 지점"에서 그 비용이 지불되는지가 코드 상에서 암묵적이다.
- 대입(`state.session = new_session`), 컬렉션에서 제거, 함수 리턴 등 다양한 지점에서 예고 없이 무거운 `Drop`이 트리거될 수 있어, 그 지점이 우연히 UI 스레드의 이벤트 핸들러 한복판이면 즉시 프레임 드랍/입력 지연으로 이어진다.
- 리뷰어가 "이 대입문이 비싸다"는 것을 타입 정의(`impl Drop`)까지 따라가 보지 않으면 알 수 없어, 성능 문제의 원인 추적이 어렵다.

**발생 조건**:
- 코덱/세션 전환, 프로젝트 닫기처럼 무거운 리소스를 갖는 객체를 UI 스레드(메인 이벤트 루프)에서 직접 교체하거나 버릴 때.
- 컬렉션에서 `remove`/`retain`으로 다수의 무거운 `Drop` 타입 원소를 한 번에 제거할 때(→ RPERF-018과 연결).

**권장**:
```rust
fn switch_codec(state: &mut AppState, new_codec: CodecKind, disposer: &DisposerHandle) {
    let old_session = std::mem::replace(&mut state.session, DecoderSession::new(new_codec));
    // 무거운 Drop을 백그라운드 정리 스레드로 명시적으로 이관
    disposer.send(Box::new(old_session)); // 그 스레드에서 drop됨
}

// disposer 스레드: 채널로 받은 Box<dyn Any + Send>를 그냥 drop하는 역할만 수행
```
- `Drop` 비용이 큰 타입은 "UI/hot 스레드에서 절대 직접 drop되지 않는다"는 불변식을 명시적 API(전용 dispose 채널, arena)로 강제한다.
- `std::mem::replace`/`Option::take`로 소유권을 명시적으로 옮기고, 실제 drop은 별도 정리 스레드나 idle 콜백에서 수행한다.
- 타입 자체에 `// PERF: 이 Drop은 FFI 정리를 수행하며 O(수십ms) 소요` 같은 주석과, 가능하면 `#[must_use]`류 장치로 "무심코 버려지지 않도록" 힌트를 남긴다.

**탐지 방법**:
- Structural: `impl Drop`이 있는 타입 목록화 후, 그 타입이 UI 이벤트 핸들러/메인 스레드 코드 경로에서 대입·제거되는지 교차 확인.
- Runtime: UI 스레드 프레임 타임 스파이크와 해당 시점의 콜스택에 `<Type as Drop>::drop`이 등장하는지 프로파일러로 확인.

**예외**:
- `Drop` 비용이 마이크로초 이하로 무시 가능한 타입(단순 `Vec`/`String` 해제 등)은 이 패턴을 적용할 필요가 없다.

**Bitvue 판정**: Confirmed — `src-tauri/src/commands/file.rs:310` (`open_file`)에서 `stream_a.units = Some(UnitModel { units, ... })`가 `stream_a_lock.write()`(RwLock write guard)를 쥔 채로 직접 대입되어, 이미 파일이 열려 있는 상태에서 새 파일을 열면 이전 `UnitModel`(수천 개 unit 보유 가능)이 락 보유 중 그 자리에서 동기 drop됨 — RPERF-010/019가 지적하는 정확한 형태(`std::mem::replace` + 별도 스레드 이관 없음).

---

### RPERF-011: Arc strong count 증가가 hot path에 존재
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Runtime

**나쁜 예**:
```rust
struct Renderer {
    frame_cache: Arc<FrameCache>,
}

impl Renderer {
    fn render_tile(&self, tile_idx: usize) {
        // 타일마다(프레임당 수백 개) Arc를 clone → atomic fetch_add
        let cache = self.frame_cache.clone();
        let tile_data = cache.get_tile(tile_idx);
        draw(tile_data);
    }
}
```

**문제**:
- `Arc::clone`은 원자적(atomic) 증가 연산이며, 단일 스레드 컨텍스트에서도 CPU가 메모리 배리어를 강제하므로 일반 정수 증가보다 훨씬 비싸다.
- 타일/블록 단위로 초당 수백~수천 회 호출되는 렌더 루프 안에서 매번 `Arc::clone`을 수행하면, 참조 카운트 갱신 자체가 measurable한 오버헤드로 누적된다.
- 멀티코어에서 여러 스레드가 동시에 같은 `Arc`를 clone/drop하면 원자적 카운터를 두고 캐시라인 경합(false/true sharing)이 발생해 코어 수가 늘어날수록 오히려 느려지는 역설이 생길 수 있다.

**발생 조건**:
- `self`가 이미 `&self`(레퍼런스)로 살아있는 동안에도 습관적으로 내부 `Arc` 필드를 clone해서 넘길 때.
- 병렬 렌더 워커들이 같은 캐시의 `Arc`를 프레임마다 반복해서 clone/drop할 때.

**권장**:
```rust
impl Renderer {
    fn render_tile(&self, tile_idx: usize) {
        // & 참조만 전달: Arc 자체를 clone할 필요가 없다
        let tile_data = self.frame_cache.get_tile(tile_idx);
        draw(tile_data);
    }
}

// 클로저/스레드 경계를 실제로 넘어야 할 때만 Arc::clone은 그 경계 진입 "1회"로 제한
fn spawn_render_workers(cache: &Arc<FrameCache>, tiles: &[usize]) {
    for chunk in tiles.chunks(64) {
        let cache = Arc::clone(cache); // 워커 스레드당 1회
        let chunk = chunk.to_vec();
        std::thread::spawn(move || {
            for &t in &chunk {
                let tile_data = cache.get_tile(t); // 이후로는 참조만 사용
                draw(tile_data);
            }
        });
    }
}
```
- 소유권 이전/스레드 경계 통과가 필요한 지점에서만 `Arc::clone`을 수행하고, 그 안쪽 루프는 `&Arc<T>` 또는 `&T` 참조로 순회한다.
- `Arc::clone(&x)`처럼 명시적으로 표기해 "이 clone은 참조 카운트 증가"임을 리뷰에서 즉시 알아볼 수 있게 한다(암묵적 `.clone()`과 구분).
- 정말 매 반복 소유권이 필요한 게 아니라면, 함수 시그니처를 `&Arc<T>` 대신 `&T`로 낮춰 애초에 clone 유혹을 없앤다.

**탐지 방법**:
- Static: 루프 본문 안의 `.clone()` 호출 대상이 `Arc<_>`/`Rc<_>` 타입인지 타입 추적.
- Runtime: `perf stat`으로 hot 함수의 atomic 연산(`lock xadd` 등) 카운트 확인, 코어 수 증가에 따라 처리량이 기대만큼 오르지 않는지 스케일링 테스트.

**예외**:
- 호출 빈도가 낮은 경로(세션 생성, 설정 변경)에서는 `Arc::clone` 비용이 무시할 수준이며 명확성이 더 중요하다.

**Bitvue 판정**: N/A — 발견된 `Arc::clone` 호출(`index_session.rs:134,281`)은 백그라운드 인덱싱 잡/스레드 스폰 경계에서 1회성으로만 발생. Rust 측에 per-tile/per-block 렌더 루프 자체가 없음(렌더링은 프론트엔드 Canvas가 IPC로 받은 YUV 데이터를 그리는 구조) — 이 패턴이 나타날 hot path가 현재 아키텍처에 없음.

---

### RPERF-012: RefCell/RwLock으로 설계 문제 은폐
**분류**: RPERF · **심각도**: High · **탐지**: Structural|Semantic

**나쁜 예**:
```rust
struct AnalysisState {
    // 소유권/차용 구조를 제대로 설계하지 못해 "일단 다 RefCell로 감싸서" 컴파일만 통과시킴
    frames: RefCell<Vec<DecodedFrame>>,
    stats: RefCell<HashMap<u32, FrameStats>>,
    cursor: RefCell<usize>,
}

fn process_frame(state: &AnalysisState, idx: usize) {
    let frames = state.frames.borrow(); // 런타임 borrow 체크
    let frame = &frames[idx];
    let mut stats = state.stats.borrow_mut(); // 여기서 이미 frames가 borrow 중이면 패닉 위험
    stats.insert(idx as u32, analyze(frame));
}
```

**문제**:
- 컴파일 타임 borrow checker를 만족시키지 못해 발생하는 설계 실패를 `RefCell`/`RwLock`으로 런타임 체크로 미루면, 그 비용(런타임 borrow 플래그 갱신, 락 획득/해제)이 정적으로 0이었을 문제에 대해 매번 지불된다.
- 더 심각하게는 `RefCell::borrow_mut()`이 이미 다른 `borrow()`가 살아있는 상태에서 호출되면 컴파일 에러가 아니라 **런타임 패닉**으로 나타나, 문제가 프로덕션까지 잠복했다가 특정 순서로만 터진다.
- 구조체 필드마다 개별 `RefCell`을 두는 "필드 단위 RefCell 남발"은 원자성(atomicity)도 보장하지 못한다 — 두 필드를 일관되게 갱신해야 하는데 그 사이에 다른 borrow가 끼어들 수 있다.
- 근본적으로 이는 성능 문제 이전에 소유권 설계가 실패했다는 신호이며, `RefCell`은 그 실패를 컴파일러가 아니라 런타임(그리고 결국 사용자)이 발견하게 만든다.

**발생 조건**:
- 여러 메서드가 `&self`로 서로 다른 필드를 읽고 쓰려는데 borrow checker가 막을 때, 구조를 재설계하는 대신 `RefCell`로 우회할 때.
- GUI 콜백처럼 호출 순서를 완전히 통제하기 어려운 코드에서 "일단 컴파일되게" `RefCell`을 습관적으로 쓸 때.

**권장**:
```rust
struct AnalysisState {
    frames: Vec<DecodedFrame>,
    stats: HashMap<u32, FrameStats>,
    cursor: usize,
}

// &mut self로 소유권/차용 관계를 컴파일 타임에 명확히 표현
fn process_frame(state: &mut AnalysisState, idx: usize) {
    let stat = analyze(&state.frames[idx]);
    state.stats.insert(idx as u32, stat);
}
```
- 가능하면 `&mut self` + 메서드 재구성으로 borrow checker를 만족시키도록 데이터 흐름 자체를 재설계한다(분리 가능한 필드는 struct를 쪼개 disjoint borrow를 활용).
- 정말 공유 가변 상태가 여러 소유자에게 필요한 경우(예: 콜백이 비동기로 상태를 갱신)에만 `RefCell`/`RwLock`을 쓰고, 그 이유를 주석으로 남긴다.
- `RefCell` 사용이 남더라도 borrow 범위를 최소화해 `borrow()`와 `borrow_mut()`가 동시에 살아있는 구간을 없앤다.

**탐지 방법**:
- Structural: 한 구조체 안에 `RefCell`/`Mutex`/`RwLock` 필드가 3개 이상이면 설계 재검토 대상으로 플래그.
- Semantic: 코드 리뷰에서 "이 RefCell은 왜 필요한가, `&mut self`로 바꿀 수 없는 이유는?"을 질문.
- Runtime: 테스트/퍼징으로 `already borrowed` 패닉 경로가 실행 가능한지 확인.

**예외**:
- 콜백 기반 UI 프레임워크(GTK/Tauri 이벤트 핸들러 등)와의 상호운용을 위해 어쩔 수 없이 공유 가변 상태가 필요한 경계 지점에서는 `RefCell`이 실용적 타협이다. 이 경우 borrow 범위를 최대한 좁게 유지한다.

**Bitvue 판정**: Suspected — `RefCell` 자체는 테스트 코드에서만 사용됨(프로덕션 grep 0건, N/A). 다만 탐지 기준으로 제시된 "필드 3개 이상 lock" 냄새는 `RwLock`/`Mutex`로 실제 존재: `core.rs:45,48,51`(`stream_a`/`stream_b`/`selection` 각각 별도 `Arc<RwLock<...>>`), `index_session.rs:65,68,71,74`(4개의 별도 `Arc<Mutex<...>>` 필드) — 원자성 버그가 확인된 건 아니지만 구조적으로 재검토 대상 패턴과 일치.

---

### RPERF-013: SmallVec를 무조건 최적화로 사용
**분류**: RPERF · **심각도**: Low · **탐지**: Static|Structural

**나쁜 예**:
```rust
use smallvec::SmallVec;

struct MbContext {
    // "작은 벡터니까 SmallVec이 빠르겠지"라는 추측만으로 도입
    neighbors: SmallVec<[MbId; 32]>,     // 실제로는 거의 항상 4~8개만 사용
    coeffs: SmallVec<[i16; 64]>,          // 4:2:0 8bit 기준 대부분 16개 이하
}
// MbContext 자체가 스택/인라인 배열 크기만큼 항상 부풀어 있어
// 이 구조체를 대량으로 담는 Vec<MbContext>의 총 메모리도 함께 커짐
```

**문제**:
- `SmallVec<[T; N]>`은 인라인 용량 `N`을 항상 구조체 크기에 포함시키므로, 실제 사용량이 그보다 훨씬 작다면 오히려 `Vec<T>`(포인터 3워드)보다 구조체가 커진다.
- 이 구조체를 대량(프레임당 수만 개 매크로블록)으로 담는 컬렉션에서는 그 "낭비된 인라인 용량"이 개별로는 작아 보여도 총합으로는 상당한 메모리 증가와 캐시 효율 저하로 이어진다.
- 인라인 용량 `N`을 실측 없이 "넉넉하게" 잡으면(위 예의 32, 64) 힙 스필을 피하려다 오히려 상시 메모리 사용량을 늘리는 역효과가 난다.
- "SmallVec = 항상 Vec보다 빠름"이라는 근거 없는 가정으로 프로파일링 없이 전면 도입하면, 실제 분포에 맞지 않을 때 개선은커녕 퇴보할 수 있다.

**발생 조건**:
- 실제 크기 분포를 측정하지 않고 "대부분 작을 것 같다"는 추측만으로 `SmallVec`을 채택할 때.
- 인라인 용량을 넉넉히 잡아 "힙 할당 회피"만 노리고 구조체 크기 증가는 고려하지 않을 때.

**권장**:
```rust
use smallvec::SmallVec;

// 실측: neighbors는 99%가 4개 이하, coeffs는 99%가 16개 이하였다고 가정
struct MbContext {
    neighbors: SmallVec<[MbId; 4]>,
    coeffs: SmallVec<[i16; 16]>,
}
```
- 도입 전 실제 워크로드에서 크기 분포를 히스토그램으로 측정하고(`p50`/`p99`), `N`을 p99~p100 근처로 타이트하게 설정한다.
- 도입 전후로 `size_of::<T>()`와 실제 벤치마크(할당 횟수, 처리량)를 비교해 정말 개선되는지 검증한다.
- 대량으로 컬렉션에 담기는 필드는 `SmallVec`보다 별도 arena(RPERF 밖 주제, LAYOUT.md 참고)나 offset 기반 인덱싱이 더 나을 수도 있음을 함께 검토한다.

**탐지 방법**:
- Static: `SmallVec<[_; N]>` 선언과 그 타입이 대량 컬렉션(`Vec<StructWithSmallVec>`)의 필드인지 확인.
- Runtime: 실측 크기 분포 로깅/히스토그램으로 `N` 값이 실제 분포와 맞는지 검증, 힙 스필 비율(`spilled()` 호출) 확인.

**예외**:
- 실측 결과 대부분(예: 99.9%)이 인라인 용량 이내이고, 해당 구조체가 대량 컬렉션에 담기지 않는(개별적으로만 존재하는) 경우에는 이 패턴이 실제로 유효한 최적화다.

**Bitvue 판정**: N/A — `SmallVec`/`smallvec` 크레이트 사용 grep 0건(의존성으로도 채택되지 않음). 이 패턴이 발생할 코드 자체가 없음.

---

### RPERF-014: Bytes를 쓰지만 실제로 매번 copy
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Structural

**나쁜 예**:
```rust
use bytes::Bytes;

fn slice_nal_payload(buf: &Bytes, start: usize, len: usize) -> Bytes {
    // Bytes::slice는 참조 카운트 공유로 O(1)이어야 하는데,
    // 아래처럼 쓰면 매번 새 버퍼로 복사된다
    let sub: Vec<u8> = buf[start..start + len].to_vec();
    Bytes::from(sub)
}

fn dedup_frame_payload(buf: &Bytes) -> Bytes {
    // "안전하게" 항상 복사해서 반환하는 습관이 Bytes 도입 취지를 무력화
    Bytes::copy_from_slice(&buf[..])
}
```

**문제**:
- `bytes::Bytes`를 도입하는 핵심 이유는 `.slice()`/`.clone()`이 참조 카운트만 증가시키고 실제 바이트를 복사하지 않는 것인데, `to_vec()` → `Bytes::from()`이나 `Bytes::copy_from_slice()`를 쓰면 이 이점이 전부 사라지고 일반 `Vec<u8>` 복사와 동일한 비용을 지불한다.
- 코드베이스 전체를 `Bytes`로 마이그레이션했다는 사실만으로 "이제 zero-copy"라고 착각하기 쉽지만, 실제로 `.slice()` 계열 API를 쓰지 않으면 타입만 바뀌었을 뿐 복사 비용은 그대로 남는다.
- 이런 우회 복사가 hot path(NAL/OBU 페이로드 분할, 프레임 버퍼 서브뷰 추출)에 있으면 `Bytes` 도입 전과 성능 차이가 없거나, `Bytes`의 부가 오버헤드(참조 카운트 관리) 때문에 오히려 근소하게 느려질 수 있다.

**발생 조건**:
- `Bytes` API에 익숙하지 않아 `&[u8]` 슬라이싱 하듯 습관적으로 `to_vec()`을 거칠 때.
- 가변성이 필요하다는 이유로(오해로) 매번 새 버퍼를 만들어 복사할 때 — 실제로는 `BytesMut`이나 별도 소유 버퍼가 필요한 지점에서만 복사가 정당화됨.

**권장**:
```rust
use bytes::Bytes;

fn slice_nal_payload(buf: &Bytes, start: usize, len: usize) -> Bytes {
    buf.slice(start..start + len) // 참조 카운트 공유, O(1)
}

fn dedup_frame_payload(buf: &Bytes) -> Bytes {
    buf.clone() // Arc 유사 clone, 바이트 복사 없음
}
```
- `Bytes`를 쓰는 코드베이스에서는 `.slice()`/`.clone()`을 우선 사용하고, `to_vec()`/`copy_from_slice()`가 등장하면 "정말 소유권 분리(다른 수명/가변성)가 필요한가"를 확인한다.
- 진짜 새 버퍼가 필요한 지점(예: 여러 소스를 병합해 새 페이로드를 만드는 경우)에서만 명시적 복사를 허용하고, 그 이유를 주석으로 남긴다.
- `Bytes` 도입 효과를 벤치마크로 검증: 도입 전(`Vec<u8>` 복사) 대비 실제 할당/복사 횟수가 줄었는지 `dhat`으로 확인.

**탐지 방법**:
- Static: `Bytes`를 다루는 함수 내부에서 `to_vec()`, `copy_from_slice()`, `Vec::from(&buf[..])` 패턴 grep.
- Runtime: `dhat`/`heaptrack`으로 `Bytes` 관련 경로의 실제 `memcpy` 발생 여부 확인.

**예외**:
- 네트워크 경계, FFI 경계처럼 원본 버퍼의 수명을 보장할 수 없어 반드시 소유 복사가 필요한 지점은 정당한 예외다.

**Bitvue 판정**: N/A — `bytes::Bytes` 사용처는 `byte_cache.rs` 1곳뿐. `Bytes::copy_from_slice`가 등장하는 유일한 함수 `get_segment`(line 178-209)는 `#[allow(dead_code)]`로 미사용 상태이며, mmap→소유 버퍼 경계에서의 1회 복사는 정당한 예외에 해당. 캐시 히트 경로는 `bytes.clone()`(refcount만 증가)으로 올바르게 구현됨.

---

### RPERF-015: Box<dyn Error>로 오류 분류와 최적화 모두 상실
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Structural

**나쁜 예**:
```rust
use std::error::Error;

fn parse_obu(data: &[u8]) -> Result<Obu, Box<dyn Error>> {
    let header = parse_obu_header(data)?; // 내부에서 io::Error를 Box<dyn Error>로 변환
    let payload = extract_payload(data, &header)?; // 여기도 마찬가지, 서로 다른 에러 타입이 뒤섞임
    Ok(Obu { header, payload })
}

fn handle(result: Result<Obu, Box<dyn Error>>) {
    match result {
        Ok(obu) => process(obu),
        Err(e) => {
            // 실제 에러 종류를 알 수 없어 문자열 매칭에 의존
            if e.to_string().contains("truncated") {
                // ...
            }
        }
    }
}
```

**문제**:
- `Box<dyn Error>`는 힙 할당(에러 하나마다) + vtable 간접 호출을 수반하므로, 파싱 실패가 빈번하게 발생할 수 있는 hot path(예: 스트림 끝 tail 처리, 프로빙)에서 매 에러 생성마다 불필요한 비용이 든다.
- 더 근본적으로는 원래 에러의 구체 타입 정보가 트레이트 객체 뒤로 사라지므로, 호출부가 에러 종류에 따라 분기(재시도 가능 vs 치명적)하려면 `to_string()` 문자열 매칭 같은 취약한 방법에 의존하게 된다.
- 여러 하위 모듈의 서로 다른 에러 타입을 전부 `Box<dyn Error>`로 뭉뚱그리면, 타입 시스템이 제공하던 "이 함수가 낼 수 있는 에러의 종류"라는 정보(문서화 효과)도 함께 사라진다.

**발생 조건**:
- 여러 하위 모듈의 에러 타입(`io::Error`, `Utf8Error`, 커스텀 파싱 에러 등)을 빠르게 통합하려고 `?`가 자동으로 되는 `Box<dyn Error>`를 반환 타입으로 채택할 때.
- 프로토타입 단계에서 편의상 도입한 `Box<dyn Error>`가 정리되지 않고 hot path까지 남을 때.

**권장**:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
enum ObuParseError {
    #[error("truncated OBU header")]
    TruncatedHeader,
    #[error("invalid OBU type: {0}")]
    InvalidType(u8),
    #[error("payload extraction failed: {0}")]
    Payload(#[from] PayloadError),
}

fn parse_obu(data: &[u8]) -> Result<Obu, ObuParseError> {
    let header = parse_obu_header(data)?;
    let payload = extract_payload(data, &header)?;
    Ok(Obu { header, payload })
}

fn handle(result: Result<Obu, ObuParseError>) {
    match result {
        Ok(obu) => process(obu),
        Err(ObuParseError::TruncatedHeader) => request_more_data(),
        Err(e) => log::warn!("OBU parse failed: {e}"),
    }
}
```
- 모듈/크레이트 경계마다 `thiserror`로 구체적인 enum 에러 타입을 정의하고, 하위 에러는 `#[from]`으로 자동 변환한다.
- hot path(파서 내부, 프레임 디코드 루프)에서는 `Box<dyn Error>`를 피하고 스택에 올라가는 값 타입 enum 에러를 사용한다.
- 애플리케이션 최상위 경계(예: Tauri command의 최종 반환, CLI 진입점)처럼 다양한 에러를 한 곳에 모아 사용자에게 보여주기만 하면 되는 지점에서는 `anyhow::Error`/`Box<dyn Error>`가 실용적이다.

**탐지 방법**:
- Static: `Box<dyn Error>`/`Box<dyn std::error::Error>` 반환 타입이 파서/디코드 hot path 모듈에 있는지 grep.
- Structural: 에러 처리 코드에서 `.to_string().contains(...)` 같은 문자열 매칭 패턴 검출(타입 정보 손실의 방증).

**예외**:
- 애플리케이션 최상위/CLI 경계, 또는 에러 발생 빈도가 극히 낮은 초기화 경로에서는 `anyhow`/`Box<dyn Error>`의 단순함이 이점이 더 크다.

**Bitvue 판정**: N/A — 유일한 실사용처 `crates/bitvue-core/src/export/probes.rs:220 export_results`는 저빈도 최상위 export 호출(문서 자체가 명시한 예외)이고, `bitvue-metrics/src/lib.rs:41`은 doc-comment 예시일 뿐 실제 코드 아님. 파서/디코드 hot path에서의 사용 grep 0건.

---

### RPERF-016: clone_from_slice가 가능한데 재할당
**분류**: RPERF · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
struct FrameBuffer {
    plane_y: Vec<u8>,
}

impl FrameBuffer {
    fn update_plane_y(&mut self, new_data: &[u8]) {
        // 기존 버퍼(이미 올바른 크기로 할당되어 있음)를 버리고 새로 할당
        self.plane_y = new_data.to_vec();
    }
}
```

**문제**:
- `self.plane_y`가 이미 올바른 용량으로 할당되어 있는데도 `new_data.to_vec()`으로 새 `Vec`을 만들어 통째로 교체하면, 기존 할당을 버리고 재할당하는 비용(`malloc` + 기존 버퍼 `free`)이 매 갱신마다 반복된다.
- 프레임 재생/스크러빙처럼 같은 크기의 버퍼가 반복적으로(초당 수십 회) 갱신되는 경로에서는 이 재할당이 allocator 압박과 GC성 지연(실제로는 free list 관리 비용)으로 이어진다.
- `Vec::clone_from`이나 슬라이스의 `copy_from_slice`/`clone_from_slice`를 쓰면 기존 버퍼의 용량을 재사용해 memcpy만으로 끝낼 수 있는데, 이를 놓치는 경우가 흔하다.

**발생 조건**:
- 매 프레임/매 틱마다 같은 크기의 버퍼를 새 데이터로 "교체"하는 루틴을 작성할 때, 대입 연산자의 편리함 때문에 재할당 경로를 무심코 선택할 때.

**권장**:
```rust
impl FrameBuffer {
    fn update_plane_y(&mut self, new_data: &[u8]) {
        if self.plane_y.len() == new_data.len() {
            self.plane_y.copy_from_slice(new_data); // 기존 할당 재사용, memcpy만
        } else {
            self.plane_y.clear();
            self.plane_y.extend_from_slice(new_data); // 크기가 바뀔 때만 용량 조정
        }
    }
}
```
- 크기가 동일하게 유지되는 반복 갱신 경로는 `copy_from_slice`(바이트 등 `Copy` 타입)나 `clone_from_slice`(`Clone` 타입)로 기존 버퍼를 재사용한다.
- `Vec` 전체를 교체해야 하는 경우에도 `Vec::clone_from(&mut self.buf, &other)`을 쓰면 기존 용량을 최대한 재사용하며 갱신할 수 있다(`=` 대입은 이를 하지 않는다는 점에 유의).

**탐지 방법**:
- Static: 구조체 필드에 대해 `self.field = expr.to_vec()` / `self.field = Vec::from(...)` 형태의 대입이 반복 호출 경로(프레임 갱신 함수 등)에 있는지 grep.
- Runtime: `dhat`으로 같은 크기의 반복 할당/해제 패턴(할당 크기 히스토그램에서 동일 크기가 매우 높은 빈도로 반복)이 나타나는지 확인.

**예외**:
- 갱신 빈도가 낮거나 버퍼 크기가 매번 달라 어차피 재할당이 불가피한 경우에는 단순 대입이 더 명확하고 적절하다.

**Bitvue 판정**: Suspected — 디코드 경로에서 plane 버퍼 `.to_vec()` 호출 다수 확인(`bitvue-decode/src/ffmpeg.rs:286,290,294`, `decoder.rs:795`)이 매번 재할당하는 형태이나, 이 함수들이 재생 중 매 프레임(동일 크기 반복) 호출되는 hot path인지, 아니면 프레임당 1회성 변환인지는 확인하지 못함.

---

### RPERF-017: VecDeque를 random access 구조로 사용
**분류**: RPERF · **심각도**: Low · **탐지**: Static|Structural

**나쁜 예**:
```rust
use std::collections::VecDeque;

struct FrameRingBuffer {
    frames: VecDeque<DecodedFrame>,
}

impl FrameRingBuffer {
    fn frame_at(&self, idx: usize) -> &DecodedFrame {
        &self.frames[idx] // VecDeque Index: 내부적으로 ring 오프셋 계산 + 분기
    }

    fn sort_by_poc(&mut self) {
        // make_contiguous()를 부르지 않고 정렬류 연산을 반복 호출하면
        // 매번 wrap-around 경계를 넘나드는 비교/스왑이 발생
        let mut v: Vec<_> = self.frames.iter().cloned().collect();
        v.sort_by_key(|f| f.poc);
        self.frames = v.into();
    }
}
```

**문제**:
- `VecDeque`의 `Index` 구현은 내부 ring buffer의 head 오프셋을 기준으로 modulo 연산에 준하는 처리를 거치므로, 순수 `Vec`/슬라이스 인덱싱보다 미세하게 무겁고 컴파일러의 자동 벡터화에도 불리하다(연속 슬라이스로 보장되지 않음).
- `frame_at`처럼 임의 인덱스 접근이 hot path(재생 중 프레임 탐색, 랜덤 시크)에서 빈번하다면,애초에 `VecDeque`를 선택한 이유(양끝 O(1) push/pop)를 활용하지 못하면서 인덱싱 비용만 추가로 지불하는 셈이다.
- 정렬처럼 컬렉션 전체를 훑는 연산을 `VecDeque`에 직접 반복 적용하면(위 예처럼 매번 `Vec`으로 변환하지 않는 안 좋은 버전) wrap-around 경계 처리 오버헤드가 반복된다.

**발생 조건**:
- 원래는 스트리밍 큐(디코드 순서대로 push, 표시 순서대로 pop)로 설계했던 구조를 나중에 "탐색/정렬 기능"이 필요해지면서 무분별하게 인덱스 접근을 늘려갈 때.
- "양 끝에서 넣고 뺀다"는 이유만으로 습관적으로 `VecDeque`를 기본 선택할 때, 실제 접근 패턴이 random access 위주인지 확인하지 않을 때.

**권장**:
```rust
use std::collections::VecDeque;

struct FrameRingBuffer {
    frames: VecDeque<DecodedFrame>,
}

impl FrameRingBuffer {
    fn sort_by_poc(&mut self) {
        // 정렬처럼 연속 슬라이스가 필요한 연산 전에 한 번만 contiguous화
        let slice = self.frames.make_contiguous();
        slice.sort_by_key(|f| f.poc);
    }
}

// random access가 주된 패턴이라면 애초에 Vec + 별도 head 인덱스로 설계
struct FrameWindow {
    frames: Vec<DecodedFrame>,
    start: usize, // 논리적 시작 오프셋
}
```
- 정렬/이진 탐색 등 연속 메모리가 필요한 연산 전에는 `make_contiguous()`를 명시적으로 호출해 이후 연산이 순수 슬라이스 위에서 동작하게 한다.
- 접근 패턴을 실측해 "양 끝 push/pop이 주(main)이고 random access는 드묾"이면 `VecDeque` 유지, "random access가 빈번"하면 `Vec`(+오프셋) 또는 다른 구조로 재설계한다.

**탐지 방법**:
- Static: `VecDeque` 필드에 대한 `[idx]`/`.get(idx)` 호출 빈도와, `.iter().collect::<Vec<_>>()`로 우회 변환하는 패턴 grep.
- Structural: 자료구조 선택 리뷰 체크리스트에 "이 컬렉션의 주 접근 패턴은 무엇인가(순차/양끝/랜덤)"를 포함.

**예외**:
- 재생 버퍼, 디코드 순서 큐처럼 실제로 양 끝 push/pop이 지배적인 워크로드에서는 `VecDeque`가 정확한 선택이며 가끔의 인덱스 접근은 무시할 수준이다.

**Bitvue 판정**: N/A — 발견된 모든 `VecDeque` 사용(`ffmpeg.rs:60 frame_buffer`, `timeline_window.rs:113 pending_loads`, `index_dev_hud_window.rs:77 recent_pattern`, `index_session_window.rs:94 lru_queue`)이 push_back/pop_front/retain 위주이며 랜덤 인덱스 접근 grep 0건. `ffmpeg.rs:131,376`은 "O(1) instead of O(n)" 주석까지 남기며 의도적으로 올바르게 사용됨.

---

### RPERF-018: retain/drain으로 대형 버퍼 반복 이동
**분류**: RPERF · **심각도**: Medium · **탐지**: Static|Runtime

**나쁜 예**:
```rust
struct DecodeQueue {
    pending: Vec<PendingFrame>, // 대형 페이로드를 갖는 원소, 수천 개까지 누적 가능
}

impl DecodeQueue {
    fn tick(&mut self, now: u64) {
        // 매 tick(초당 수십 회)마다 전체 버퍼를 재배치
        // retain은 남는 원소들을 앞으로 당기며 셔플(내부적으로 memmove 반복)
        self.pending.retain(|f| f.deadline > now);
    }

    fn flush_ready(&mut self) -> Vec<PendingFrame> {
        // drain(..)으로 전체를 뽑아내고 다시 채우는 패턴을 매 tick 반복
        let ready: Vec<_> = self.pending.drain(..).filter(|f| f.is_ready()).collect();
        ready
    }
}
```

**문제**:
- `Vec::retain`은 조건을 만족하지 않는 원소를 걸러내며 남은 원소들을 앞으로 당기는 in-place 압축을 수행하는데, 이는 제거 비율과 무관하게 전체 길이에 비례한 이동(최악 O(n))이 매 호출마다 발생한다. 이를 초당 수십 회 tick마다 대형 버퍼에 반복 적용하면 누적 비용이 커진다.
- `PendingFrame`이 큰 값 타입(포인터가 아니라 실제 데이터를 인라인으로 갖는 구조체)이라면, retain/drain 중 발생하는 이동이 단순 포인터 스왑이 아니라 구조체 전체의 `memmove`가 되어 비용이 더 커진다(RPERF-008과 유사한 맥락).
- `drain(..)`으로 전량을 뽑아 필터링 후 남은 것을 다시 넣는 패턴은, 애초에 필요했던 "일부만 꺼내기"를 위해 전체를 두 번(빼기+다시 채우기) 오가는 비효율을 낳는다.

**발생 조건**:
- 타이머/데드라인 기반으로 대기열을 정리하는 로직을 매 프레임 tick마다 전체 버퍼 스캔으로 구현할 때.
- 원소가 크고(포인터가 아니라 값 타입) 큐 길이가 수백~수천에 달할 때 이 비용이 두드러진다.

**권장**:
```rust
struct DecodeQueue {
    pending: Vec<Box<PendingFrame>>, // 원소를 Box로 얇게 만들어 이동 비용을 포인터 크기로 축소
    // 또는: 데드라인 정렬을 유지하는 BinaryHeap<Reverse<PendingFrame>>로 구조 자체를 바꿔
    // "만료분만 앞에서 pop"하도록 재설계
}

impl DecodeQueue {
    fn tick(&mut self, now: u64) {
        // 정렬된 구조라면 만료된 선두만 O(k) pop (k=이번에 만료된 개수)
        while matches!(self.pending.first(), Some(f) if f.deadline <= now) {
            self.pending.remove(0); // 예시 단순화; 실제로는 BinaryHeap/VecDeque 활용 권장
        }
    }
}
```
- 원소가 크면 `Box`로 감싸 이동 비용을 포인터 크기로 낮추거나, 애초에 정렬/우선순위 유지 구조(`BinaryHeap`, 데드라인 정렬된 `VecDeque`)로 바꿔 "매 tick 전체 스캔"이 아니라 "만료분만 O(k)로 꺼내기"가 되도록 재설계한다.
- `retain`/`drain` 호출 빈도 자체를 낮출 수 있는지(예: 매 tick이 아니라 배치로 모아 처리) 먼저 검토한다.
- 진짜 매 tick 전체 스캔이 필요하다면(만료 조건이 비순차적일 수 있는 경우) `retain`이 여전히 정답이지만, 원소 크기를 줄여 이동 비용을 낮추는 것이 우선이다.

**탐지 방법**:
- Static: hot loop(tick/frame 콜백)에서 `retain`, `drain`, `remove(0)` 호출과 그 대상 컬렉션의 원소 크기(`size_of`) 확인.
- Runtime: `perf`로 tick 함수 내 `memmove`/`ptr::copy` 관련 심볼 비중 확인.

**예외**:
- 큐 길이가 작게(수십 개 이하) 유지되도록 상한이 보장되어 있다면 `retain`의 O(n) 비용이 실질적으로 무시할 수준이다.

**Bitvue 판정**: Suspected — `.retain()` 호출 다수 확인(`index_session_window.rs:195,210,244,258 lru_queue`, `worker.rs:194,216 in_flight`, `compare_cache.rs:417`)하나 대상 컬렉션이 대부분 `usize`/job-id 등 작은 값이고 윈도우 크기로 상한이 있어 보여, 원문이 우려하는 "대형 페이로드 원소·수천개 누적" 시나리오보다는 영향이 작을 가능성이 높음 — 확정하려면 각 컬렉션의 실제 최대 길이 확인 필요.

---

### RPERF-019: 비싼 Drop이 UI thread에서 실행
**분류**: RPERF · **심각도**: High · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
struct ProjectState {
    all_frames: Vec<DecodedFrame>,   // 수천 프레임, 프레임당 수 MB
    overlay_cache: HashMap<u32, OverlayGrid>,
}

// Tauri command 핸들러 (프론트엔드 "새 파일 열기" 클릭 시 동기 호출)
#[tauri::command]
fn open_project(path: String, state: tauri::State<Mutex<Option<ProjectState>>>) {
    let mut guard = state.lock().unwrap();
    // 새 ProjectState 대입 순간, 이전 ProjectState(수천 프레임 보유)가
    // 그 자리에서 즉시 drop됨 — 호출한 이벤트 루프 스레드가 그동안 블록
    *guard = Some(ProjectState::load(&path));
}
```

**문제**:
- 대용량 컬렉션(수천 개 디코드 프레임, 오버레이 캐시)을 갖는 구조체를 UI 이벤트 처리 스레드(Tauri의 메인/커맨드 스레드 등)에서 직접 교체하면, 이전 값의 `Drop`(각 프레임 버퍼 `free` 수천 회)이 그 스레드를 동기적으로 점유한다.
- 사용자 입장에서는 "새 파일 열기"를 눌렀는데 몇백 ms 동안 UI가 완전히 얼어붙는(입력도 렌더링도 멈추는) 현상으로 나타나며, 원인이 "새 파일 로딩"이 아니라 "이전 파일 해제"라는 것을 프로파일링 없이는 오인하기 쉽다.
- `Mutex` 락을 잡은 채로 이 무거운 drop이 실행되면, 같은 락을 기다리는 다른 스레드(렌더 스레드 등)까지 함께 블록되어 문제가 전파된다.

**발생 조건**:
- "새 파일 열기", "프로젝트 닫기", "코덱 전환"처럼 대형 상태를 완전히 교체하는 커맨드 핸들러를 동기 함수로 작성했을 때.
- 락을 쥔 상태에서 대입/교체가 이뤄져 drop 비용과 락 경합이 겹칠 때.

**권장**:
```rust
#[tauri::command]
fn open_project(path: String, state: tauri::State<Mutex<Option<ProjectState>>>) {
    let new_state = ProjectState::load(&path); // 락 밖에서 미리 로드

    let old_state = {
        let mut guard = state.lock().unwrap();
        std::mem::replace(&mut *guard, Some(new_state)) // 락 보유 시간 최소화
    }; // 락 해제

    // 무거운 drop은 별도 스레드로 이관, UI/커맨드 스레드는 즉시 반환
    std::thread::spawn(move || drop(old_state));
}
```
- 대형 상태 교체는 `std::mem::replace`로 소유권만 옮기고, 실제 drop은 `std::thread::spawn` 또는 전용 정리 워커 스레드로 이관해 UI/커맨드 스레드를 즉시 반환시킨다(RPERF-010과 같은 원리를 UI 스레드 맥락에 적용).
- 락은 "포인터/핸들 교체"라는 짧은 임계구역에만 걸고, drop처럼 시간이 걸리는 작업은 락 바깥에서 수행한다.
- 가능하면 대형 컬렉션 자체를 `Arc`로 감싸 참조 카운트만 감소시키고, 마지막 참조가 정리 스레드에서 떨어지도록 구조화한다.

**탐지 방법**:
- Runtime: UI 프레임 타임/입력 지연 스파이크 시점과 `#[tauri::command]` 핸들러 내 대입문 타이밍을 트레이싱으로 교차 확인. `tracing` span으로 handler 실행 시간 측정.
- Structural: 대형 컬렉션을 필드로 갖는 상태 타입이 `Mutex` 락 보유 중에 직접 대입/드롭되는 커맨드 핸들러 목록화(TAURI_CMD.md의 "동기 커맨드에서 무거운 작업" 계열 항목과 연계 확인).

**예외**:
- 상태 크기가 작아(수십 KB 이하) drop 비용이 프레임 예산(예: 16ms) 대비 무시할 수준이면 별도 스레드 이관은 과설계다.

**Bitvue 판정**: Confirmed — RPERF-010과 동일 근거(`src-tauri/src/commands/file.rs:305-336 open_file`): `state.core.lock()` → `stream_a_lock.write()`를 쥔 채 이전 `UnitModel`을 직접 대입으로 교체·동기 drop. 다만 `open_file`이 `async fn`이라 엄밀한 "UI 스레드"는 아니고 Tauri 커맨드 스레드/tokio 워커 블로킹에 해당 — 원문의 "UI thread" 표현과는 약간 다르지만 본질적으로 같은 문제(락 보유 중 무거운 동기 작업).

---

### RPERF-020: debug assertion이 release correctness를 대신함
**분류**: RPERF · **심각도**: Critical · **탐지**: Static|Semantic

**나쁜 예**:
```rust
fn get_macroblock(&self, x: usize, y: usize) -> &Macroblock {
    debug_assert!(x < self.width && y < self.height, "MB coordinate out of range");
    // release 빌드에서는 위 assert가 완전히 사라짐 → 범위를 벗어나도 그냥 진행
    &self.mbs[y * self.width + x] // out-of-bounds면 인덱싱 자체 panic으로 잡히긴 하지만,
                                    // 의미상 잘못된 좌표(예: width 경계 넘어 다른 행 침범)는
                                    // 조용히 "잘못된 그러나 유효한 범위의" 값을 반환할 수 있음
}

fn parse_nal_length(data: &[u8], offset: usize) -> usize {
    let len = u32::from_be_bytes(data[offset..offset+4].try_into().unwrap()) as usize;
    debug_assert!(len <= data.len() - offset - 4, "NAL length exceeds buffer");
    // release에서는 이 불변식 검증이 사라진 채로 len이 그대로 다음 슬라이싱에 사용됨
    len
}
```

**문제**:
- `debug_assert!`는 `debug_assertions` cfg가 꺼지는 release 빌드에서 완전히 컴파일 아웃되므로, "이 조건이 항상 참"이라는 안전성 근거로 사용하면 release에서는 그 근거가 사라진 채 코드만 남는다.
- 특히 신뢰할 수 없는 입력(파일에서 읽은 length 필드, 사용자 지정 좌표)에 대한 검증을 `debug_assert!`로만 해두면, release 빌드(실사용자가 쓰는 빌드)에서는 malformed/적대적 비트스트림에 대해 검증 없이 그대로 진행되어 논리 오류, 잘못된 프레임 렌더링, 최악의 경우 별도 unsafe 코드와 결합 시 메모리 안전성 문제로 이어질 수 있다.
- "테스트에서는(debug 빌드로 돌리므로) 안 터졌으니 안전하다"는 잘못된 확신을 주기 쉽다 — CI가 debug 테스트 위주라면 이 구멍이 release에서만 드러나는 버그로 잠복한다.
- 반대로 성능이 실제로 중요한 hot path에서 `assert!`(release에도 남는 것)를 무분별하게 쓰면 그건 그것대로 실제 오버헤드가 되므로, "무엇을 debug_assert로, 무엇을 assert/명시적 에러 반환으로 할지"의 기준이 없는 것 자체가 문제의 핵심이다.

**발생 조건**:
- 외부 입력(파일 파싱 결과, IPC로 들어온 좌표/인덱스)의 유효성 검증을 성능 걱정 때문에 `debug_assert!`로 작성했을 때.
- "내부 불변식이니 release에서는 필요 없다"고 판단했지만, 실제로는 그 불변식이 신뢰할 수 없는 입력에 의존하고 있을 때.

**권장**:
```rust
fn get_macroblock(&self, x: usize, y: usize) -> Option<&Macroblock> {
    if x >= self.width || y >= self.height {
        return None; // release에도 남는 명시적 검증, 호출부가 처리
    }
    Some(&self.mbs[y * self.width + x])
}

fn parse_nal_length(data: &[u8], offset: usize) -> Result<usize, ParseError> {
    let len = u32::from_be_bytes(data[offset..offset+4].try_into().map_err(|_| ParseError::Truncated)?) as usize;
    if len > data.len().saturating_sub(offset + 4) {
        return Err(ParseError::LengthOutOfBounds { len, offset }); // release에도 유지되는 실제 검증
    }
    Ok(len)
}

// 반대로, "이미 위에서 명시적으로 검증된" 사실을 hot path 내부에서 재확인만 하고 싶을 때는
// debug_assert!가 적절 — 이때는 릴리즈에서 사라져도 안전성에 영향이 없어야 한다.
fn fast_inner_loop(mbs: &[Macroblock], idx: usize) -> &Macroblock {
    debug_assert!(idx < mbs.len(), "caller must pre-validate idx");
    unsafe { mbs.get_unchecked(idx) }
}
```
- **신뢰 경계를 넘어온 데이터(파일 입력, IPC, 사용자 지정 인덱스)에 대한 검증은 `debug_assert!`가 아니라 `Result`/`Option` 반환 또는 release에도 유지되는 `assert!`로 작성한다.**
- `debug_assert!`는 "호출부 계약상 이미 참이어야 하는 내부 불변식"을 개발 중 조기 발견하는 용도로만 쓰고, 그 불변식이 깨져도 release에서 안전한지(예: 이후 `unsafe get_unchecked` 없이 일반 인덱싱이면 최소 panic으로 그침) 항상 확인한다.
- 정말 성능이 중요해 release에서 검증을 생략해야 한다면, `unsafe fn`으로 계약을 타입 시그니처에 명시하고 문서화해 "검증 생략은 호출자 책임"임을 명확히 한다 — 검증 누락을 `debug_assert!` 뒤에 숨기지 않는다.

**탐지 방법**:
- Static: `debug_assert!`/`debug_assert_eq!` 호출 대상이 외부 입력 유래 값(파싱 결과, IPC 파라미터)인지 데이터 흐름 추적.
- Semantic: 코드 리뷰에서 "이 debug_assert가 release에서 사라지면 무슨 일이 일어나는가"를 각 항목마다 질문해 명시적으로 답할 수 없으면 재작성 대상.
- Structural: `cargo build --release`로 빌드한 바이너리에 대해 malformed 입력 퍼징을 수행해, debug 빌드에서는 assert로 잡히던 경로가 release에서 조용히 통과하는지 차등 테스트.

**예외**:
- hot path 내부에서, 이미 함수 진입 시점에 `Result`/`assert!`로 명시적으로 검증을 마친 값을 그 이후 내부 루프에서 다시 훑을 때 "이미 검증됨"을 문서화하는 용도의 `debug_assert!`는 정확히 이 패턴이 의도하는 올바른 사용이다.

**Bitvue 판정**: N/A — 파싱된 데이터에 관련된 실사용 `debug_assert!` 2건(`bitvue-av1-codec/src/types.rs:98,127 Qp::as_u8/From<u8>`, `bitvue-avs3/src/bitreader.rs:36 read_bits`)이 전부 문서가 권장하는 "이미 검증됨" 패턴에 해당: `Qp::new`(types.rs:52)가 0..=255 범위를 `Result`로 먼저 검증하고, `read_ue`(bitreader.rs:60-74)가 `leading_zeros<=31`을 `Err`로 먼저 걸러낸 뒤에만 `read_bits`를 호출함. 안티패턴이 우려하는 "신뢰 경계 데이터를 debug_assert로만 검증" 사례는 grep 범위 내에서 발견되지 않음.
