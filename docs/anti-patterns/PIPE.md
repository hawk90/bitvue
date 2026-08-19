# Anti-Pattern Catalog — PIPE: 프레임 재사용과 계산 공유 (VQ-Probe domain)

이 문서는 Bitvue 안티패턴 카탈로그의 한 분류이며, 전체 목록은 `docs/anti-patterns/INDEX.md`(별도 작성)를 참고한다. VQ-Probe(듀얼 스트림 화질 비교) 도메인 절반을 구성하는 ALIGN/SPATIAL/COLOR/METRIC/HEAT/STAT 카탈로그와 나란히 위치하며, PIPE는 그중에서도 "동일한 reference/distorted 프레임 쌍에 대해 여러 metric을 계산할 때 디코드·전처리·중간 표현을 어떻게 공유하는가"에 집중한다.

## 기준 파이프라인 형태

```
Decode + Align
      ↓
Normalized Frame Pair
      ↓
 ┌────┼────┬────┐
PSNR SSIM VMAF Heatmap
```

Reference/distorted 스트림을 각각 한 번만 디코드하고, 정렬(frame alignment) · 색공간 변환 · 리사이즈를 한 번만 거친 "정규화된 프레임 쌍"을 만든 뒤, 그 결과를 PSNR/SSIM/VMAF/heatmap 등 여러 metric 계산기에 팬아웃(fan-out)한다. 디코드·전처리 비용은 metric 개수와 무관하게 O(1)이고, metric별 비용만 O(N)으로 늘어난다.

이 문서에서 다루는 안티패턴은 대부분 이 형태가 다음과 같이 무너지는 경우다.

```
Decode+Align(PSNR용) → PSNR
Decode+Align(SSIM용) → SSIM
Decode+Align(VMAF용) → VMAF
Decode+Align(Heatmap용) → Heatmap
```

각 metric이 독립적으로 reference/distorted를 처음부터 디코드·정렬·변환하여, 디코드 비용이 metric 개수만큼 곱해진다. 4개 metric이면 디코드 비용이 4배가 되고, 여기에 스트림이 두 개(reference + distorted)이므로 실제로는 최대 8배까지 늘어날 수 있다.

---

### PIPE-001: metric마다 입력 영상을 별도로 decode
**분류**: PIPE · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
pub struct QualityReport {
    pub psnr: f64,
    pub ssim: f64,
    pub vmaf: f64,
}

pub fn compute_all_metrics(
    reference_path: &Path,
    distorted_path: &Path,
) -> QualityReport {
    // 각 metric 함수가 스스로 디코더를 열고 끝까지 디코드한다.
    let psnr = compute_psnr(reference_path, distorted_path);
    let ssim = compute_ssim(reference_path, distorted_path);
    let vmaf = compute_vmaf(reference_path, distorted_path);
    QualityReport { psnr, ssim, vmaf }
}

fn compute_psnr(reference_path: &Path, distorted_path: &Path) -> f64 {
    let mut ref_dec = MediaCoreDecoder::open(reference_path).unwrap();
    let mut dist_dec = MediaCoreDecoder::open(distorted_path).unwrap();
    let mut acc = PsnrAccumulator::new();
    while let (Some(r), Some(d)) = (ref_dec.next_frame(), dist_dec.next_frame()) {
        acc.add(&r, &d);
    }
    acc.finalize()
}

fn compute_ssim(reference_path: &Path, distorted_path: &Path) -> f64 {
    // 위와 동일한 두 파일을 처음부터 다시 연다.
    let mut ref_dec = MediaCoreDecoder::open(reference_path).unwrap();
    let mut dist_dec = MediaCoreDecoder::open(distorted_path).unwrap();
    // ...
    0.0
}
```

**문제**:
- 4K/8K 소스에서 소프트웨어 디코드는 metric 하나만 계산해도 실시간보다 느릴 수 있는데, metric N개면 디코드 비용이 N배가 된다.
- reference/distorted 두 스트림을 각 metric이 다시 열기 때문에 실제 배수는 "metric 개수 × 2"에 가깝다.
- 컨테이너 파싱, 디코더 초기화(SPS/PPS 파싱, 디코더 컨텍스트 할당), 파일 I/O가 모두 중복된다.
- 배치로 여러 클립을 비교하는 경우 이 배수가 클립 수만큼 다시 곱해져 전체 배치 처리 시간이 선형이 아니라 조합적으로 늘어난다.

**발생 조건**:
- 각 metric을 "독립적으로 테스트 가능한 순수 함수"로 설계하면서, 함수 시그니처를 `(reference_path, distorted_path) -> f64`처럼 파일 경로 기반으로 잡았을 때.
- 서로 다른 팀/시점에 PSNR, SSIM, VMAF를 각각 추가하면서 기존 디코드 파이프라인을 재사용하지 않고 매번 새 함수를 작성했을 때.

**권장**:
```rust
pub struct FramePair {
    pub reference: Arc<DecodedFrame>,
    pub distorted: Arc<DecodedFrame>,
    pub index: u64,
}

pub fn compute_all_metrics(
    reference_path: &Path,
    distorted_path: &Path,
) -> QualityReport {
    let mut ref_dec = MediaCoreDecoder::open(reference_path).unwrap();
    let mut dist_dec = MediaCoreDecoder::open(distorted_path).unwrap();

    let mut psnr = PsnrAccumulator::new();
    let mut ssim = SsimAccumulator::new();
    let mut vmaf = VmafAccumulator::new();

    let mut index = 0;
    while let (Some(r), Some(d)) = (ref_dec.next_frame(), dist_dec.next_frame()) {
        let pair = FramePair { reference: Arc::new(r), distorted: Arc::new(d), index };
        // 한 번 디코드한 프레임 쌍을 세 metric에 그대로 팬아웃한다.
        psnr.add(&pair);
        ssim.add(&pair);
        vmaf.add(&pair);
        index += 1;
    }

    QualityReport { psnr: psnr.finalize(), ssim: ssim.finalize(), vmaf: vmaf.finalize() }
}
```
- metric 계산 함수는 파일 경로가 아니라 이미 디코드된 `FramePair`(또는 그 참조)를 입력으로 받도록 시그니처를 바꾼다.
- 디코드 루프는 하나만 존재하고, metric별 accumulator가 그 루프 안에서 `add()`만 호출한다.

**탐지 방법**:
- Structural: `MediaCoreDecoder::open(` 호출 횟수를 grep해 metric 함수 개수보다 많은지 확인. 함수 시그니처가 `&Path`를 받는 metric 계산 함수가 여러 개 있으면 의심.
- Runtime: 전체 파이프라인의 디코드 관련 flamegraph에서 동일 파일에 대한 `open`/`next_frame` 호출이 여러 클러스터로 나뉘어 나타나는지 확인.

**예외**:
- 서로 다른 metric이 서로 다른 프레임 서브샘플링(예: PSNR은 전체 프레임, VMAF는 1초 간격 샘플링)을 요구해 애초에 같은 디코드 순회를 공유할 수 없는 경우. 이때도 "디코드 자체"는 공유 캐시(PIPE-008 참고)를 통해 재사용을 시도하는 편이 낫다.

**Bitvue 판정**: N/A — (구 `src-tauri`는 2026-08-08 Electron 전환으로 삭제됨, 이하 전 항목 현재 sidecar/CLI 아키텍처 기준으로 재감사) `crates/bitvue-sidecar/src/debug_yuv.rs`의 `compute_frame_metrics`(498-543행)는 프레임 하나당 디코드를 한 번만 수행한 뒤(`decode_stream_a_frame`/`read_reference_frame`, 510-512행) 동일 `YuvFrame` 쌍에서 `psnr_yuv`와 `ssim_yuv`를 모두 계산한다(540-543행). CLI 경로(`crates/bitvue-cli/src/commands/quality.rs`)의 `compute_frame_metrics`(24-116행)도 reference/distorted를 `decode_ivf_frames`로 각각 한 번씩만 디코드하고(60-62행) 그 결과 `Vec`에서 프레임마다 `psnr()`/`ssim()`을 함께 호출한다(96-106행). metric마다 재디코드하는 경로는 발견되지 않음.

---

### PIPE-002: metric마다 color conversion 반복
**분류**: PIPE · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_psnr(pair: &FramePair) -> f64 {
    let ref_rgb = yuv_to_rgb(&pair.reference); // YUV420 → RGB, 매 프레임마다
    let dist_rgb = yuv_to_rgb(&pair.distorted);
    psnr_rgb(&ref_rgb, &dist_rgb)
}

fn compute_ssim(pair: &FramePair) -> f64 {
    let ref_rgb = yuv_to_rgb(&pair.reference); // 위와 동일한 변환을 다시 수행
    let dist_rgb = yuv_to_rgb(&pair.distorted);
    ssim_rgb(&ref_rgb, &dist_rgb)
}

fn compute_vmaf(pair: &FramePair) -> f64 {
    let ref_linear = yuv_to_rgb(&pair.reference); // 세 번째 중복 변환
    let dist_linear = yuv_to_rgb(&pair.distorted);
    vmaf_score(&ref_linear, &dist_linear)
}
```

**문제**:
- `yuv_to_rgb`는 프레임당 픽셀 수에 비례하는 O(W×H) 연산인데, metric 개수만큼 반복되어 전처리 비용이 선형으로 곱해진다.
- 대부분의 metric은 동일한 색공간(예: PSNR/SSIM은 8bit RGB, VMAF는 선형 라이트)만 다를 뿐 "YUV → 어떤 공통 중간 표현" 단계는 동일한데 이를 공유하지 않는다.
- SIMD로 최적화된 변환 루틴이라도 캐시에 올라간 프레임 데이터를 여러 번 훑기 때문에 메모리 대역폭이 병목이 되기 쉽다.

**발생 조건**:
- metric 구현체들이 서로 다른 개발자/시점에 추가되어 "이 metric은 RGB가 필요하다"는 가정을 각자 자기 함수 안에 캡슐화했을 때.
- 색공간 변환이 metric 모듈 내부에 숨어 있어 파이프라인 조립 시점에는 변환이 일어나는지조차 보이지 않을 때.

**권장**:
```rust
pub struct NormalizedFramePair {
    pub reference_rgb: Arc<RgbBuffer>,
    pub distorted_rgb: Arc<RgbBuffer>,
}

fn normalize(pair: &FramePair) -> NormalizedFramePair {
    NormalizedFramePair {
        reference_rgb: Arc::new(yuv_to_rgb(&pair.reference)),
        distorted_rgb: Arc::new(yuv_to_rgb(&pair.distorted)),
    }
}

fn compute_all(pair: &FramePair) -> (f64, f64) {
    let norm = normalize(pair); // 한 번만 변환
    let psnr = psnr_rgb(&norm.reference_rgb, &norm.distorted_rgb);
    let ssim = ssim_rgb(&norm.reference_rgb, &norm.distorted_rgb);
    (psnr, ssim)
}
```
- 색공간 변환을 "정규화" 단계로 파이프라인 최상단에 명시적으로 끌어올리고, metric 함수는 이미 변환된 버퍼만 받도록 시그니처를 제한한다.
- VMAF처럼 다른 색공간(선형 라이트)이 필요한 metric은 별도 정규화 단계를 두되, 그 단계도 metric 하나당 한 번만 실행되어야 한다(PIPE-007 참조).

**탐지 방법**:
- Structural: `yuv_to_rgb`(또는 유사 변환 함수) 호출 지점이 metric 계산 함수 내부에 흩어져 있는지 grep. 파이프라인 조립 코드에 정규화 단계가 별도로 없으면 의심.
- Runtime: 프레임 하나당 색변환 함수 호출 횟수를 카운터로 계측해 metric 개수와 일치하면 문제로 판정.

**예외**:
- metric마다 요구하는 정밀도/색공간이 근본적으로 다르고(예: 8bit sRGB vs 16bit 선형 라이트) 공유 가능한 중간 단계가 실질적으로 없는 경우. 이때도 "각 표현을 프레임당 한 번만" 만드는 것은 지켜야 한다(문제는 표현 공유가 아니라 반복 계산이다).

**Bitvue 판정**: N/A — `psnr_yuv`/`ssim_yuv`(crates/bitvue-metrics/src/lib.rs 267-303행)와 CLI가 쓰는 `psnr`/`ssim`(84-140행)은 디코드된 YUV/luma 평면을 그대로 받아 계산하며 색공간 변환이 없다. `yuv_to_rgb`(crates/bitvue-decode/src/yuv.rs)는 crates/bitvue-sidecar/src/debug_yuv.rs에서 실제로 호출되지 않고 모듈 최상단 doc 주석(16행, "프런트엔드 렌더링 파이프라인" 설명용)에만 등장한다 — quality/diff-metric 코드 경로와 교집합 없음(grep 확인).

---

### PIPE-003: metric마다 resize 반복
**분류**: PIPE · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_ssim(pair: &NormalizedFramePair) -> f64 {
    // SSIM 구현체가 내부적으로 8x8 블록 정렬을 위해 리사이즈
    let r = resize_to_multiple_of_8(&pair.reference_rgb);
    let d = resize_to_multiple_of_8(&pair.distorted_rgb);
    ssim_core(&r, &d)
}

fn compute_vmaf(pair: &NormalizedFramePair) -> f64 {
    // VMAF도 자기 내부 요구사항에 맞춰 독립적으로 리사이즈
    let r = resize_to_multiple_of_8(&pair.reference_rgb);
    let d = resize_to_multiple_of_8(&pair.distorted_rgb);
    vmaf_core(&r, &d)
}

fn compute_heatmap(pair: &NormalizedFramePair, display_size: (u32, u32)) -> HeatmapTile {
    // heatmap은 화면 표시용으로 또 다른 크기로 리사이즈
    let r = resize_to(&pair.reference_rgb, display_size);
    let d = resize_to(&pair.distorted_rgb, display_size);
    diff_heatmap(&r, &d)
}
```

**문제**:
- 여러 metric이 우연히 같은 목표 크기(8의 배수 정렬)를 요구하는데도 각자 리사이즈를 수행해 동일한 리샘플링 필터 연산이 중복된다.
- 리사이즈는 색공간 변환보다도 비용이 큰 경우가 많다(양방향 보간 필터는 커널 크기에 비례해 픽셀당 여러 샘플을 읽음). 4K 프레임에서 metric 3개가 각자 리사이즈하면 리사이즈 비용만 3배.
- 리사이즈 대상 크기가 metric마다 실제로 다르면(예: heatmap은 화면 표시 크기, VMAF는 원본 크기) 이 사실이 코드 표면에 드러나지 않아 "왜 이렇게 느린가" 진단이 어렵다.

**발생 조건**:
- 각 metric 라이브러리(또는 자체 구현체)가 "입력 크기 요구사항"을 내부에 캡슐화하고 있어 호출자가 이미 맞는 크기를 넘겨주는지 알 수 없을 때.
- heatmap처럼 UI 표시 목적의 리사이즈와 metric 계산 목적의 리사이즈가 같은 함수로 뭉뚱그려졌을 때.

**권장**:
```rust
pub struct MetricInputs {
    pub native: Arc<RgbBuffer>,          // metric 계산용, 8의 배수 정렬
    pub display: Option<Arc<RgbBuffer>>, // heatmap 등 표시용, lazy
}

fn build_inputs(pair: &NormalizedFramePair, display_size: Option<(u32, u32)>) -> MetricInputs {
    let native = Arc::new(resize_to_multiple_of_8(&pair.reference_rgb));
    let display = display_size.map(|sz| Arc::new(resize_to(&native, sz)));
    MetricInputs { native, display }
}
// SSIM과 VMAF는 동일한 `native` 버퍼를 공유하고,
// heatmap만 필요할 때 `display` 버퍼를 한 번 더 만든다(그것도 native에서 파생).
```
- "metric 계산용 정규 크기"와 "표시용 크기"를 명시적으로 분리하고, 각각 한 번씩만 리사이즈한다.
- 여러 metric이 같은 목표 크기를 요구한다면 리사이즈 결과를 캐시하거나 파이프라인 정규화 단계로 끌어올린다.

**탐지 방법**:
- Structural: `resize_to*` 계열 함수 호출이 metric 함수 개수만큼 존재하고 목표 크기가 동일한 리터럴/상수인 경우.
- Runtime: 프레임당 리사이즈 호출 횟수와 대상 해상도를 로깅해 중복 여부 확인.

**예외**:
- metric마다 목표 크기가 실제로 다르고 재사용 가능한 중간 크기가 없는 경우(예: VMAF는 원본 해상도, heatmap은 1/4 축소). 이때도 각 크기별로 "한 번만" 리사이즈하는 원칙은 유지해야 한다.

**관련**: `CACHE.md` CACHE-012 참고 — 이쪽은 metric마다 동일 목표 크기로 리사이즈를 반복 계산하는 문제이고, CACHE-012는 썸네일을 원본과 별도로 중복 저장하는 캐시 설계 문제다.

**Bitvue 판정**: N/A — quality 파이프라인에 리사이즈 단계가 아예 없다. CLI 경로는 reference/distorted 해상도가 다르면 리사이즈하지 않고 그 프레임 쌍을 건너뛴다(crates/bitvue-cli/src/commands/quality.rs 85-91행, 경고 로그 후 continue). sidecar `compute_frame_metrics`도 크기가 다르면 에러를 반환할 뿐 리사이즈하지 않는다(crates/bitvue-sidecar/src/debug_yuv.rs 514-520행). `resize_to*` 계열 함수 호출은 metric 경로 어디에도 없음.

---

### PIPE-004: metric마다 frame alignment 반복
**분류**: PIPE · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_psnr(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> f64 {
    // reference/distorted 프레임 수가 다를 수 있어 매번 정렬을 재계산
    let aligned = align_frames_by_pts(reference, distorted); // O(N log N) 매칭
    psnr_over(&aligned)
}

fn compute_ssim(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> f64 {
    let aligned = align_frames_by_pts(reference, distorted); // 동일 정렬을 다시 계산
    ssim_over(&aligned)
}

fn compute_vmaf(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> f64 {
    let aligned = align_frames_by_pts(reference, distorted); // 세 번째 중복 계산
    vmaf_over(&aligned)
}
```

**문제**:
- 프레임 드롭·가변 프레임레이트가 있는 두 스트림을 PTS 기준으로 매칭하는 정렬 알고리즘은 metric 계산 자체보다 오래 걸릴 수 있는데(예: DTW 기반 정렬), 이를 metric마다 반복한다.
- 정렬 결과(어느 reference 프레임이 어느 distorted 프레임과 짝지어지는지)는 metric과 무관한 순수 시간축 데이터인데도 metric 코드 내부에 숨어 있어 재사용 경로가 없다.
- 정렬 알고리즘에 휴리스틱이나 임계값이 있는 경우, 반복 계산 과정에서 부동소수점 비결정성으로 metric 간 정렬 결과가 미세하게 달라질 위험도 있다(같은 프레임 쌍이라고 믿었던 것이 metric마다 실제로는 다를 수 있음).

**발생 조건**:
- frame alignment 로직이 공용 모듈로 분리되지 않고 각 metric 구현체에 "전처리 단계"로 인라인되어 있을 때.
- ALIGN 카탈로그(별도 문서)에서 다루는 정렬 알고리즘 자체의 문제와 별개로, "정렬을 어디서 한 번 계산해 공유하는가"라는 파이프라인 구조 문제가 방치되었을 때.

**권장**:
```rust
pub struct AlignedTimeline {
    pub pairs: Vec<(usize, usize)>, // (reference_idx, distorted_idx)
}

pub fn build_pipeline(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> QualityReport {
    let timeline = align_frames_by_pts(reference, distorted); // 단 한 번만 계산
    let mut psnr = PsnrAccumulator::new();
    let mut ssim = SsimAccumulator::new();
    let mut vmaf = VmafAccumulator::new();
    for &(ri, di) in &timeline.pairs {
        let pair = FramePair::new(&reference[ri], &distorted[di]);
        psnr.add(&pair);
        ssim.add(&pair);
        vmaf.add(&pair);
    }
    QualityReport { psnr: psnr.finalize(), ssim: ssim.finalize(), vmaf: vmaf.finalize() }
}
```
- 정렬 결과(`AlignedTimeline`)를 파이프라인 조립 시점에 한 번 계산해 모든 metric이 동일한 `(ref_idx, dist_idx)` 쌍 목록을 공유하게 한다.
- 이렇게 하면 "PSNR과 SSIM이 실제로 같은 프레임 쌍을 비교했는가"를 구조적으로 보장할 수 있다(정확성 이점도 있음).

**탐지 방법**:
- Structural: 정렬 함수(`align_frames_by_pts` 등) 호출이 metric 함수 개수만큼 존재하는지 grep.
- Semantic: metric 함수 시그니처가 정렬 전 원시 프레임 슬라이스를 받는지, 이미 정렬된 타임라인을 받는지 검토.

**예외**:
- 없음에 가깝다. 프레임 정렬은 metric과 무관한 순수 시간축 연산이므로 공유하지 않을 이유가 거의 없다. 굳이 예외를 든다면 metric마다 정렬 허용 오차(tolerance)가 근본적으로 다른 특수 경우 정도다.

**Bitvue 판정**: N/A — `AlignmentEngine::new`(crates/bitvue-engine/src/alignment.rs 44행)는 `CompareWorkspace::new`(crates/bitvue-engine/src/compare.rs 61행)에서 워크스페이스당 단 한 번만 호출되어 정렬을 계산한다; metric마다 재계산하는 구조가 아니다. 다만 PSNR/SSIM을 실제로 계산하는 두 경로(CLI quality.rs, sidecar debug_yuv.rs) 모두 이 `AlignmentEngine`을 전혀 사용하지 않고 원시 인덱스(+ debug_yuv.rs의 수동 `picture_offset`)로만 프레임을 매칭한다 — "정렬이 metric 계산 경로에 통합되지 않았다"는 별개 이슈이며, PIPE-004가 지적하는 "metric마다 정렬 반복 계산"과는 다른 문제.

---

### PIPE-005: 하나의 느린 metric이 전체 pipeline을 막음
**분류**: PIPE · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn compute_all_metrics_sequential(pair: &FramePair) -> QualityReport {
    let psnr = compute_psnr(pair);   // ~0.1ms
    let ssim = compute_ssim(pair);   // ~1ms
    let vmaf = compute_vmaf(pair);   // ~50ms — 신경망 기반 feature 추출 포함
    QualityReport { psnr, ssim, vmaf }
}

pub fn analyze_stream(pairs: &[FramePair]) -> Vec<QualityReport> {
    pairs.iter().map(compute_all_metrics_sequential).collect()
    // PSNR/SSIM은 이미 끝났는데도 VMAF가 끝날 때까지 다음 프레임으로 못 넘어간다.
}
```

**문제**:
- VMAF처럼 비용이 큰 metric이 파이프라인의 단일 스레드 순차 경로에 있으면, 계산량이 적은 PSNR/SSIM까지 그 지연에 발이 묶인다.
- 전체 처리량이 "가장 느린 metric의 처리량"으로 수렴하는데, 이는 병렬화 가능한 자원(멀티코어)을 활용하지 못하는 것이다.
- 실시간 미리보기(스크러빙 중 즉석 PSNR 표시 등)처럼 빠른 metric만 우선 필요한 사용 사례에서도 느린 metric이 응답성을 깎아먹는다.

**발생 조건**:
- metric들을 하나의 함수에서 순서대로 호출하도록 작성했을 때(코드는 단순하지만 병렬성이 전혀 없음).
- VMAF 등 무거운 metric을 나중에 추가하면서 기존 PSNR/SSIM 파이프라인 구조를 그대로 재사용했을 때.

**권장**:
```rust
pub fn compute_all_metrics_parallel(pair: &FramePair) -> QualityReport {
    // rayon::join 또는 별도 스레드 풀로 metric을 독립 실행
    let (psnr, (ssim, vmaf)) = rayon::join(
        || compute_psnr(pair),
        || rayon::join(|| compute_ssim(pair), || compute_vmaf(pair)),
    );
    QualityReport { psnr, ssim, vmaf }
}
```
또는 metric마다 별도 스테이지로 분리해 느린 metric이 빠른 metric의 완료·보고를 막지 않게 한다(PIPE-006 참조).
- CPU 바운드 metric은 `rayon`/스레드 풀로 병렬 실행하고, 결과는 각 metric의 완료 시점에 독립적으로 보고한다.
- metric별 예상 비용(cheap: PSNR/SSIM, expensive: VMAF)을 스케줄링 힌트로 활용해 무거운 metric을 별도 큐/우선순위로 분리하는 것도 고려한다.

**탐지 방법**:
- Runtime: 프레임당 각 metric 소요 시간을 계측해 전체 프레임 처리 시간이 metric 시간의 합과 근접하면(병렬화가 없다는 뜻) 문제로 판정.
- Structural: metric 호출이 `rayon::join`/스레드 spawn 없이 한 함수 안에서 순차적으로 나열되어 있는지 확인.

**예외**:
- 단일 코어 환경이거나, metric 간 데이터 의존성이 있어 순서 실행이 불가피한 경우(예: heatmap이 SSIM의 중간 맵을 입력으로 요구 — PIPE-017 참조). 이때는 병렬화 대신 의존성 그래프에 따른 파이프라이닝을 고려한다.

**Bitvue 판정**: Confirmed — crates/bitvue-cli/src/commands/quality.rs의 프레임 루프(77-113행)는 각 프레임에서 `psnr()`(96-97행) 다음 `ssim()`(102-103행)을 순차 호출하며 `rayon`/`par_iter` 없이 완전 순차다. crates/bitvue-metrics/src/lib.rs의 `batch_psnr_parallel`/`batch_ssim_parallel`(329-387행)이 rayon 기반 병렬 처리를 제공하지만 opt-in Cargo feature `parallel` 뒤에 있고(crates/bitvue-metrics/Cargo.toml `default = []`, 26행), repo 전체에서 호출부가 전무하다(grep 확인) — 병렬 처리 수단은 존재하나 실제 파이프라인에 연결되어 있지 않다. VMAF는 `vmaf` feature 자체가 기본 비활성이고 sidecar/CLI 어디에도 배선되지 않아(grep 결과 없음) "느린 VMAF가 PSNR/SSIM을 막는" 구체적 시나리오는 아직 실현되지 않지만, 구조 자체(순차 나열, 병렬화 없음)는 이 안티패턴과 일치한다.

---

### PIPE-006: 모든 metric이 끝날 때까지 부분 결과를 제공하지 않음
**분류**: PIPE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct QualityReport {
    pub psnr: f64,
    pub ssim: f64,
    pub vmaf: f64,
}

pub fn analyze(pairs: &[FramePair]) -> QualityReport {
    let mut psnr_acc = PsnrAccumulator::new();
    let mut ssim_acc = SsimAccumulator::new();
    let mut vmaf_acc = VmafAccumulator::new();
    for pair in pairs {
        psnr_acc.add(pair);
        ssim_acc.add(pair);
        vmaf_acc.add(pair); // 전체 스트림 처리 중 가장 느린 단계
    }
    // 세 metric이 모두 끝나야만 함수가 반환된다 — UI는 그 전까지 아무것도 못 그린다.
    QualityReport { psnr: psnr_acc.finalize(), ssim: ssim_acc.finalize(), vmaf: vmaf_acc.finalize() }
}
```

**문제**:
- UI(프리뷰 패널, 그래프)가 PSNR처럼 이미 값이 확정된 metric조차 VMAF 전체 완료 전까지 표시할 수 없다.
- 사용자가 긴 클립을 분석할 때 "지금까지 몇 프레임 처리됐고 PSNR은 이 정도다"라는 중간 피드백이 전혀 없어 진행 상황을 알 수 없다.
- 파이프라인 중간에 취소하고 싶어도(예: 처음 몇 프레임만 보고 이 클립은 스킵) 이미 계산된 부분 결과를 활용할 방법이 없다.

**발생 조건**:
- 반환 타입이 `QualityReport`처럼 "모든 metric이 채워진 완전한 구조체" 하나뿐이고, 함수가 그 값을 만들어 반환하는 형태로만 설계되었을 때.
- 프론트엔드와의 통신이 단일 IPC 응답(요청-응답)으로만 이루어져 스트리밍 업데이트 채널이 없을 때.

**권장**:
```rust
pub enum MetricUpdate {
    Partial { metric: MetricKind, frame_index: u64, running_value: f64 },
    Finalized { metric: MetricKind, value: f64 },
}

pub fn analyze_streaming(pairs: &[FramePair], sink: &dyn Fn(MetricUpdate)) {
    let mut psnr_acc = PsnrAccumulator::new();
    let mut ssim_acc = SsimAccumulator::new();
    let mut vmaf_acc = VmafAccumulator::new();
    for pair in pairs {
        psnr_acc.add(pair);
        sink(MetricUpdate::Partial { metric: MetricKind::Psnr, frame_index: pair.index, running_value: psnr_acc.running() });
        ssim_acc.add(pair);
        sink(MetricUpdate::Partial { metric: MetricKind::Ssim, frame_index: pair.index, running_value: ssim_acc.running() });
        vmaf_acc.add(pair);
        sink(MetricUpdate::Partial { metric: MetricKind::Vmaf, frame_index: pair.index, running_value: vmaf_acc.running() });
    }
    sink(MetricUpdate::Finalized { metric: MetricKind::Psnr, value: psnr_acc.finalize() });
    sink(MetricUpdate::Finalized { metric: MetricKind::Ssim, value: ssim_acc.finalize() });
    sink(MetricUpdate::Finalized { metric: MetricKind::Vmaf, value: vmaf_acc.finalize() });
}
```
- metric별로 "진행 중 값"과 "최종 확정 값"을 구분해 콜백/채널로 즉시 내보낸다.
- IPC 계층(Tauri 이벤트 등)에서 이 업데이트를 스트리밍으로 프런트엔드에 전달해 프레임 단위 진행률 UI를 지원한다.

**탐지 방법**:
- Structural: 분석 함수의 반환 타입이 모든 metric 필드를 가진 단일 struct이고, 콜백/채널 파라미터가 없는지 확인.
- Manual: UI 쪽에서 "분석 중" 스피너만 있고 프레임 단위 진행률 표시가 없다면 백엔드 구조를 의심.

**예외**:
- 배치 오프라인 분석(사용자가 결과를 기다리지 않고 나중에 리포트를 확인하는 경우)처럼 실시간 피드백이 불필요한 워크로드는 스트리밍 없이도 무방하다.

**Bitvue 판정**: N/A — 현재 UI가 실제로 도달하는 metric 엔드포인트인 sidecar `get_yuv_diff_metrics`(→`debug_yuv::compute_frame_metrics`, crates/bitvue-sidecar/src/debug_yuv.rs 498-578행)는 프레임 하나당 IPC 요청/응답 하나라서 애초에 "모든 프레임이 끝나야 반환"하는 구조가 아니다(프런트엔드가 스크러빙하며 프레임별로 호출). 옛 `calculate_quality_metrics`(`BatchQualityMetrics` 반환)에 대응하는 sidecar 커맨드는 grep 결과 존재하지 않는다 — 이를 호출하던 frontend/components/panels/QualityMetricsPanel.tsx·QualityComparisonPanel.tsx는 여전히 `@tauri-apps/api/core`의 `invoke`를 쓰는 죽은 Tauri 잔재이고(101-102행), App.tsx 등 어디에도 마운트되지 않아(grep 확인) 실도달 불가능하다 — 이 항목이 겨냥하는 "블로킹 배치 응답"이 라이브 코드에는 없다. CLI(`crates/bitvue-cli/src/commands/quality.rs` `run()`)는 요청된 프레임을 모두 계산한 뒤 한 번에 출력하지만(118-190행) 오프라인 배치 워크플로우로 이 항목의 예외 조항에 해당.

---

### PIPE-007: metric별 요구 format이 다른데 중간 표현 하나로 강제
**분류**: PIPE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct NormalizedFramePair {
    pub reference: Arc<RgbBuffer>, // 8bit sRGB로 고정
    pub distorted: Arc<RgbBuffer>,
}

fn normalize(pair: &FramePair) -> NormalizedFramePair {
    NormalizedFramePair {
        reference: Arc::new(yuv_to_rgb8_srgb(&pair.reference)),
        distorted: Arc::new(yuv_to_rgb8_srgb(&pair.distorted)),
    }
}

fn compute_vmaf(norm: &NormalizedFramePair) -> f64 {
    // VMAF는 선형 라이트, 고정밀 입력을 요구하는데 8bit sRGB를 받아 내부에서 역감마 보정
    let ref_linear = srgb_to_linear(&norm.reference); // 이미 8bit로 양자화된 뒤라 정밀도 손실
    let dist_linear = srgb_to_linear(&norm.distorted);
    vmaf_core(&ref_linear, &dist_linear)
}
```

**문제**:
- "정규화는 한 곳에서"라는 PIPE-002의 교훈을 잘못 적용해 모든 metric에게 동일한 8bit sRGB 중간 표현을 강제하면, 더 높은 정밀도가 필요한 metric(VMAF의 선형 라이트 변환 등)이 이미 양자화된 데이터에서 재변환해야 해 정밀도가 손실된다.
- 반대로 모든 metric을 만족시키기 위해 중간 표현을 가장 높은 정밀도(예: 32bit float 선형 라이트)로 통일하면, 단순 8bit 비교로 충분한 PSNR 같은 metric까지 불필요하게 큰 메모리와 변환 비용을 지불한다.
- "공유 가능한 것만 공유"가 아니라 "무조건 하나로 통일"을 목표로 삼으면 오히려 정확성이나 성능 중 하나를 희생하게 된다.

**발생 조건**:
- PIPE-001~003을 해결하는 과정에서 "정규화 단계를 하나만 두자"는 목표를 지나치게 문자 그대로 적용했을 때.
- metric 라이브러리(특히 외부 VMAF 바인딩)의 입력 정밀도 요구사항을 파이프라인 설계 초기에 조사하지 않았을 때.

**권장**:
```rust
pub struct NormalizedFramePair {
    pub reference_8bit: Arc<RgbBuffer>,      // PSNR/SSIM/heatmap용
    pub distorted_8bit: Arc<RgbBuffer>,
    pub reference_linear: OnceLock<Arc<LinearBuffer>>, // VMAF용, lazy 생성
    pub distorted_linear: OnceLock<Arc<LinearBuffer>>,
}

impl NormalizedFramePair {
    pub fn linear_reference(&self, source: &DecodedFrame) -> Arc<LinearBuffer> {
        self.reference_linear
            .get_or_init(|| Arc::new(yuv_to_linear_rgb(source)))
            .clone()
    }
}
```
- 원본 디코드 프레임에서 "필요한 만큼만" 파생되는 여러 중간 표현을 허용하되, 각 표현은 프레임당 한 번만 생성되도록 `OnceLock`/캐시로 감싼다.
- 어떤 metric이 어떤 표현을 필요로 하는지 표를 만들어 정규화 단계 설계 시점에 명시적으로 정리한다(예: PSNR/SSIM→8bit RGB, VMAF→선형 라이트, heatmap→8bit RGB + 표시 크기).

**탐지 방법**:
- Structural: 단일 `NormalizedFramePair` 타입에 필드가 하나뿐인데 여러 metric이 그 필드를 서로 다른 방식으로 추가 변환하고 있는지 확인.
- Manual: metric 라이브러리 문서에서 요구하는 입력 정밀도/색공간을 정규화 단계 구현과 대조.

**예외**:
- 실제로 모든 metric이 동일한 정밀도 요구사항을 갖는다면(예: 모두 8bit RGB로 충분) 단일 표현으로 통일하는 것이 맞다. 이 항목은 "무조건 여러 표현을 만들라"가 아니라 "필요 이상으로 강제 통일하지 말라"는 뜻이다.

**관련**: `CACHE.md` CACHE-002 참고 — 이쪽은 metric별 정밀도 요구에 맞춰 중간 표현을 분리하는 문제이고, CACHE-002는 재취득 비용이 다른 데이터를 캐시 티어로 분리하는 문제다.

**Bitvue 판정**: N/A — 강제된 단일 중간 표현 자체가 없다. PSNR/SSIM은 디코드된 YUV/luma 평면을 그대로 사용하고(PIPE-002 판정 참고), VMAF는 `VmafFrame`(crates/bitvue-metrics/src/vmaf.rs)이라는 별도 타입을 받아 `to_vmaf_picture()`(57행)에서 필요한 형식으로만 변환한다(71/82/93행 `write_plane`) — 다만 VMAF 자체가 어느 파이프라인에도 배선되지 않았음(PIPE-005 판정 참고). "모든 metric에게 8bit sRGB를 강제"하는 정규화 단계가 없으므로 이 안티패턴의 전제 자체가 성립하지 않음.

---

### PIPE-008: 중간 frame을 무제한 queue에 보관
**분류**: PIPE · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn spawn_pipeline() -> (Sender<FramePair>, JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel::<FramePair>(); // unbounded
    let handle = thread::spawn(move || {
        for pair in rx {
            run_all_metrics(&pair);
        }
    });
    (tx, handle)
}

fn decode_loop(tx: Sender<FramePair>, reference_path: &Path, distorted_path: &Path) {
    let mut ref_dec = MediaCoreDecoder::open(reference_path).unwrap();
    let mut dist_dec = MediaCoreDecoder::open(distorted_path).unwrap();
    let mut index = 0;
    while let (Some(r), Some(d)) = (ref_dec.next_frame(), dist_dec.next_frame()) {
        // 디코드는 빠른데 metric 소비가 느리면 여기서 무한정 쌓인다.
        tx.send(FramePair { reference: Arc::new(r), distorted: Arc::new(d), index }).unwrap();
        index += 1;
    }
}
```

**문제**:
- `std::sync::mpsc::channel`은 unbounded이므로 디코드 스레드가 metric 스레드보다 빠르면 큐에 `FramePair`가 무한정 쌓인다.
- 4K 프레임 하나가 reference+distorted 합쳐 수십 MB인데, 이런 프레임이 수백 개 쌓이면 GB 단위 메모리를 순식간에 소비한다(PIPE-013, PIPE-014와 직결).
- 메모리 압박이 심해지면 OS 페이징이 발생해 오히려 전체 파이프라인이 더 느려지는 역설적인 상황이 생긴다.

**발생 조건**:
- 디코드 스레드와 metric 계산 스레드를 채널로 분리하면서 채널 용량을 명시적으로 고민하지 않고 기본(unbounded) 채널을 사용했을 때.
- 개발/테스트 환경에서는 짧은 클립만 사용해 문제가 드러나지 않다가, 실사용에서 긴 클립이나 느린 metric(VMAF)과 만나 누적됐을 때.

**권장**:
```rust
pub fn spawn_pipeline(capacity: usize) -> (SyncSender<FramePair>, JoinHandle<()>) {
    // bounded 채널 — 큐가 capacity에 도달하면 send()가 block되어 자연스럽게 backpressure가 걸린다.
    let (tx, rx) = std::sync::mpsc::sync_channel::<FramePair>(capacity);
    let handle = thread::spawn(move || {
        for pair in rx {
            run_all_metrics(&pair);
        }
    });
    (tx, handle)
}
```
- bounded 채널(`sync_channel`, `crossbeam_channel::bounded`)을 사용해 큐 크기를 명시적으로 제한한다.
- capacity는 "동시에 메모리에 떠 있어도 되는 프레임 쌍 개수"로 산정하고, 프레임 크기(해상도)에 따라 동적으로 조정하는 것도 고려한다(PIPE-014 참조).

**탐지 방법**:
- Structural: `std::sync::mpsc::channel(` (bounded 아닌 형태) 또는 용량 인자가 없는 채널 생성자를 grep.
- Runtime: 파이프라인 실행 중 채널 큐 길이(또는 대기 중인 `Arc<FramePair>` 강한 참조 수)를 모니터링해 지속적으로 증가하는지 확인.

**예외**:
- 프레임 쌍이 아니라 이미 스칼라로 축약된 결과(metric 점수 하나, 수십 바이트)를 전달하는 채널이라면 unbounded여도 메모리 위험이 낮다.

**관련**: `CACHE.md` CACHE-001 참고 — 이쪽은 디코드/metric 파이프라인 큐의 backpressure(bounded channel) 문제이고, CACHE-001은 프레임 캐시의 byte-budget 축출 정책 문제다.

**Bitvue 판정**: N/A — 디코드↔metric을 잇는 채널/스레드 기반 producer-consumer 구조 자체가 존재하지 않는다. `rg "mpsc::channel|sync_channel|crossbeam_channel"`을 crates/bitvue-metrics, crates/bitvue-engine, crates/bitvue-sidecar, crates/bitvue-cli에 돌려도 매치 없음. CLI 경로는 `decode_ivf_frames`(quality.rs 192-229행)로 스트림 전체를 `Vec`에 미리 다 디코드해두는 방식이라 무제한 큐가 쌓일 여지 자체가 없다(다만 이는 "클립 전체를 한 번에 메모리에 올린다"는 별개 성격의 메모리 문제로 이어질 수 있음 — PIPE-013 판정 참고).

---

### PIPE-009: reference frame을 metric worker마다 복사
**분류**: PIPE · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn dispatch_to_workers(pair: &FramePair, workers: &[Sender<FramePair>]) {
    for tx in workers {
        // FramePair가 Clone이고, 내부 필드가 Arc가 아니라 Vec<u8>라면
        // 여기서 매번 프레임 전체를 deep copy한다.
        tx.send(pair.clone()).unwrap();
    }
}

#[derive(Clone)]
pub struct FramePair {
    pub reference: DecodedFrame,  // DecodedFrame이 Vec<u8> 버퍼를 직접 소유
    pub distorted: DecodedFrame,
}
```

**문제**:
- worker(PSNR용, SSIM용, VMAF용 스레드)마다 동일한 프레임 데이터를 힙에 복제해, worker 개수만큼 메모리 사용량과 복사 비용이 곱해진다.
- 4K RGBA 프레임 하나가 약 33MB라면 reference+distorted 쌍 하나가 66MB이고, worker 4개면 프레임 하나당 264MB를 순간적으로 할당·복사해야 한다.
- 복사 자체가 memcpy 대역폭을 소모해 CPU 캐시를 오염시키고, metric 계산에 쓰일 대역폭을 빼앗는다.

**발생 조건**:
- `FramePair`(또는 `DecodedFrame`) 타입이 `Arc` 없이 소유 버퍼(`Vec<u8>`, `Box<[u8]>`)를 직접 담고 있고, 여러 worker에 "보내기 위해" `Clone`을 구현했을 때.
- 채널 기반 팬아웃 구조를 도입하면서 "각 worker가 독립적인 데이터를 가져야 한다"는 오해로 deep clone을 선택했을 때.

**권장**:
```rust
#[derive(Clone)]
pub struct FramePair {
    pub reference: Arc<DecodedFrame>, // Arc clone은 참조 카운트 증가만 수행 (O(1))
    pub distorted: Arc<DecodedFrame>,
}

fn dispatch_to_workers(pair: &FramePair, workers: &[Sender<FramePair>]) {
    for tx in workers {
        tx.send(pair.clone()).unwrap(); // 이제 이 clone은 저렴하다
    }
}
```
- 프레임 버퍼 자체를 `Arc<DecodedFrame>`으로 감싸 여러 worker가 소유권이 아니라 참조를 공유하게 한다.
- worker가 프레임을 변형(in-place mutation)해야 한다면 `Arc::make_mut` 또는 명시적 clone-on-write로 처리하되, 이는 실제로 변형이 필요한 worker에 한해서만 비용을 지불하게 한다(PIPE-018 참조).

**탐지 방법**:
- Structural: `FramePair`/`DecodedFrame` 타입 정의에서 필드가 `Arc<T>`가 아니라 `T`(소유 버퍼)인지 확인하고, 그 타입에 `#[derive(Clone)]`가 있으면서 여러 worker에 `.clone()`으로 전달되는 지점을 grep.
- Runtime: worker 개수를 늘렸을 때 프레임당 메모리 할당량이 비례해서 늘어나는지 프로파일링.

**예외**:
- worker가 정말로 독립적인 가변 버퍼가 필요하고(예: in-place로 다운샘플링해도 되는 스크래치), 그 비용이 실측상 무시할 수준(작은 해상도, 적은 worker 수)이라면 deep copy도 허용 가능하다.

**Bitvue 판정**: N/A — `DecodedFrame`의 평면 필드가 `Arc<[u8]>`로 설계되어 있다(`y_plane`/`u_plane`/`v_plane`, crates/bitvue-decode/src/decoder.rs 41/45/49행) — 이미 권장 예제 형태. CLI의 `decoded_frame_to_luma`(quality.rs 232행)가 `Arc<[u8]>`를 `Vec<u8>`로 한 번 복사해 풀지만, worker별 fan-out 없이 단일 스레드 순차 루프(PIPE-005 판정 참고) 안에서 프레임당 한 번만 일어나므로 이 안티패턴이 지적하는 "worker 개수만큼 복제"에 해당하지 않는다.

---

### PIPE-010: frame pair 수명 관리가 Arc clone에만 의존
**분류**: PIPE · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
pub struct MetricPipeline {
    // 각 metric worker가 자기가 처리 중인 프레임 쌍을 계속 들고 있음
    pending: Vec<Arc<FramePair>>,
}

impl MetricPipeline {
    pub fn submit(&mut self, pair: Arc<FramePair>) {
        // "언젠가 정리하겠지"라는 가정 하에 Arc clone만 늘어난다.
        self.pending.push(pair.clone());
        self.psnr_worker.submit(pair.clone());
        self.ssim_worker.submit(pair.clone());
        self.vmaf_worker.submit(pair);
        // pending에서 언제 제거하는지에 대한 명시적 로직이 없음 —
        // 각 worker가 처리를 끝내도 pending 벡터의 clone은 그대로 살아있어
        // 실제 free 시점이 "누군가 pending을 청소할 때"까지 미뤄진다.
    }
}
```

**문제**:
- `Arc`는 "마지막 참조가 사라지면 자동으로 해제된다"는 편리함을 주지만, 그 편리함을 파이프라인 전체의 수명 관리 전략으로 삼으면 "언제 마지막 참조가 사라지는지"를 아무도 설계하지 않게 된다.
- worker들이 처리를 끝낸 뒤에도 `pending` 같은 부수적인 컬렉션이 참조를 계속 들고 있으면, 실제 free 시점이 프로그래머의 의도보다 훨씬 늦게(또는 영영 오지 않게) 밀린다(PIPE-013으로 직결).
- 참조 카운트 기반 해제는 "언제"를 예측할 수 없어, 메모리 사용량을 프레임 처리 진행률과 연관 지어 추론하기 어렵다. 디버깅 시 "이 프레임이 왜 아직 메모리에 있는가"를 추적하려면 모든 `Arc` clone 지점을 찾아야 한다.

**발생 조건**:
- 파이프라인 단계마다 `Arc::clone`을 넘기기만 하고, 각 단계가 처리를 끝낸 뒤 명시적으로 참조를 drop하는 지점을 설계하지 않았을 때.
- 진행 상황 추적, 재시도, 취소 등의 부가 기능을 위해 여기저기 `Arc<FramePair>`를 보관하는 컬렉션이 늘어났을 때.

**권장**:
```rust
pub struct MetricPipeline {
    // pending은 인덱스/메타데이터만 보관하고, 실제 프레임 데이터는 보관하지 않는다.
    pending: HashMap<u64, PendingState>,
}

struct PendingState {
    remaining_metrics: HashSet<MetricKind>, // 이 프레임 쌍을 아직 기다리는 metric 목록
}

impl MetricPipeline {
    pub fn submit(&mut self, pair: Arc<FramePair>) {
        self.pending.insert(pair.index, PendingState {
            remaining_metrics: [MetricKind::Psnr, MetricKind::Ssim, MetricKind::Vmaf].into(),
        });
        // worker에는 Arc를 넘기지만, submit 함수 자신은 pair를 보관하지 않는다 —
        // pair의 마지막 소유자는 오직 각 worker의 처리 큐뿐이다.
        self.psnr_worker.submit(Arc::clone(&pair));
        self.ssim_worker.submit(Arc::clone(&pair));
        self.vmaf_worker.submit(pair);
    }

    pub fn on_metric_done(&mut self, frame_index: u64, metric: MetricKind) {
        if let Some(state) = self.pending.get_mut(&frame_index) {
            state.remaining_metrics.remove(&metric);
            if state.remaining_metrics.is_empty() {
                self.pending.remove(&frame_index); // 명시적으로 추적 종료
            }
        }
    }
}
```
- 어떤 구조체가 `FramePair`의 "진짜 소유자"인지 설계 단계에서 명시하고, 그 외의 컬렉션은 인덱스/메타데이터만 보관한다.
- 각 metric이 처리를 끝냈을 때 명시적으로 "이 프레임에 대한 이 metric의 참조는 끝났다"를 알리는 콜백을 두어, 마지막 참조가 사라지는 시점을 추론 가능하게 만든다.

**탐지 방법**:
- Semantic: `Arc<FramePair>`(또는 유사 타입)를 필드로 보관하는 구조체를 모두 나열하고, 각각에 대해 "이 참조를 언제 제거하는가"에 대한 코드가 존재하는지 확인. 삽입 로직은 있는데 제거 로직이 없으면 의심.
- Runtime: 파이프라인을 오래 실행했을 때 `Arc::strong_count`가 예상보다 크게 유지되는 프레임이 있는지 계측.

**예외**:
- 파이프라인이 극히 짧게 살고(예: 단발성 CLI 배치 실행 후 프로세스 종료) 명시적 해제 없이도 프로세스 종료 시 전체가 정리되는 워크로드라면 엄격한 수명 관리가 필요 없을 수 있다.

**관련**: `CACHE.md` CACHE-009 참고 — 이쪽은 Arc clone을 소유권 설계 없이 여기저기 보관해 해제 시점이 불명확해지는 문제이고, CACHE-009는 강한 참조 순환으로 인한 명시적 메모리 누수다.

**Bitvue 판정**: N/A — metric 계산 경로(CLI quality.rs, sidecar debug_yuv.rs)에 `Arc<FramePair>`를 여러 컬렉션이 장기 보관하는 구조가 없다. crates/bitvue-engine/src/alignment.rs의 `FramePair`류 타입은 인덱스/PTS 델타 같은 가벼운 메타데이터만 담고 픽셀 버퍼가 없으며, CLI의 `Vec<(Vec<u8>, usize, usize)>`(`decode_ivf_frames`)는 함수 로컬 변수로 반환 시 drop된다 — "pending" 류의 별도 보관 컬렉션이 없음.

---

### PIPE-011: metric 계산 순서가 cache locality를 악화
**분류**: PIPE · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn analyze(pairs: &[FramePair]) -> QualityReport {
    // metric 단위로 전체 스트림을 한 바퀴씩 순회 — "metric-major" 순서
    let mut psnr_acc = PsnrAccumulator::new();
    for pair in pairs {
        psnr_acc.add(pair); // 프레임 0..N을 순회하며 psnr만 계산
    }
    let mut ssim_acc = SsimAccumulator::new();
    for pair in pairs {
        ssim_acc.add(pair); // 다시 프레임 0..N을 순회하며 ssim만 계산
    }
    let mut vmaf_acc = VmafAccumulator::new();
    for pair in pairs {
        vmaf_acc.add(pair); // 세 번째로 프레임 0..N을 순회
    }
    QualityReport { psnr: psnr_acc.finalize(), ssim: ssim_acc.finalize(), vmaf: vmaf_acc.finalize() }
}
```

**문제**:
- 프레임 쌍 전체가 메모리(디코드 캐시)에 이미 올라가 있다고 가정할 때, "frame-major"(프레임 하나에 대해 모든 metric을 계산한 뒤 다음 프레임으로) 순서가 아니라 "metric-major"(metric 하나에 대해 모든 프레임을 순회한 뒤 다음 metric으로) 순서를 쓰면 같은 프레임 데이터를 여러 번 다른 시점에 캐시로 불러와야 한다.
- 특히 프레임 전체를 미리 디코드해 벡터에 들고 있는 구조라면, metric-major 순회는 각 metric 패스마다 L2/L3 캐시를 처음부터 다시 채워야 해 캐시 미스가 metric 개수만큼 반복된다.
- PIPE-001~004를 이미 해결해 디코드/전처리를 공유하더라도, 이 순서 문제는 별도로 남아있을 수 있다(디코드는 공유했지만 순회 패턴이 캐시에 불리한 경우).

**발생 조건**:
- accumulator를 metric별로 독립적인 함수로 작성한 뒤, 파이프라인 조립 코드에서 "간단하니까" 순서대로 각 accumulator의 전체 루프를 따로 호출했을 때.
- 병렬화(PIPE-005)를 위해 metric마다 별도 스레드에서 전체 프레임 배열을 처음부터 순회하도록 만들었을 때(이 경우 병렬성과 캐시 지역성이 트레이드오프 관계에 놓인다).

**권장**:
```rust
pub fn analyze(pairs: &[FramePair]) -> QualityReport {
    // frame-major 순서 — 프레임 하나를 캐시에 올린 김에 모든 metric을 계산
    let mut psnr_acc = PsnrAccumulator::new();
    let mut ssim_acc = SsimAccumulator::new();
    let mut vmaf_acc = VmafAccumulator::new();
    for pair in pairs {
        psnr_acc.add(pair);
        ssim_acc.add(pair);
        vmaf_acc.add(pair);
    }
    QualityReport { psnr: psnr_acc.finalize(), ssim: ssim_acc.finalize(), vmaf: vmaf_acc.finalize() }
}
```
- 단일 스레드 구조에서는 frame-major 순서(프레임당 모든 metric 계산 후 다음 프레임)가 캐시 지역성에 유리하다.
- 병렬화가 필요하다면 "metric마다 전체 스트림을 순회하는 스레드"보다 "프레임 청크마다 여러 metric을 동시에 계산하는 스레드"로 나누는 편이 지역성과 병렬성을 동시에 챙길 수 있다.

**탐지 방법**:
- Runtime: `perf stat`(또는 플랫폼 동등 도구)로 캐시 미스율을 metric-major/frame-major 두 순서에서 비교.
- Structural: 파이프라인 조립 코드에서 `for pair in pairs { ... }` 루프가 metric 개수만큼 반복해서 나타나는지 grep.

**예외**:
- 전체 프레임 세트가 CPU 캐시에 애초에 다 들어가지 않을 만큼 크다면(대용량 배치) metric-major/frame-major 차이가 캐시 지역성보다 디스크/디코드 I/O 패턴에 더 크게 좌우될 수 있어 이 문제의 우선순위가 낮아진다.

**Bitvue 판정**: N/A — 실제 루프는 이미 frame-major다. crates/bitvue-cli/src/commands/quality.rs의 프레임 루프(77-113행)는 프레임 하나마다 psnr/ssim을 그 자리에서 모두 계산한 뒤 다음 프레임으로 넘어가고, sidecar `compute_frame_metrics`도 요청 하나당 프레임 하나를 통째로 처리한다 — "나쁜 예"에서 지적하는 metric별 전체 스트림 재순회 패턴은 없음.

---

### PIPE-012: 여러 metric이 동일 scratch memory를 lock으로 공유
**분류**: PIPE · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct SharedScratch {
    // 여러 metric이 임시 계산 버퍼(가우시안 블러 중간값 등)를 공유
    buffer: Mutex<Vec<f32>>,
}

fn compute_ssim(pair: &FramePair, scratch: &SharedScratch) -> f64 {
    let mut buf = scratch.buffer.lock().unwrap(); // SSIM이 락을 잡고
    gaussian_blur_into(&pair.reference, &mut buf);
    // ... buf를 이용한 계산 ...
    0.0
} // 락 해제

fn compute_vmaf(pair: &FramePair, scratch: &SharedScratch) -> f64 {
    let mut buf = scratch.buffer.lock().unwrap(); // VMAF도 같은 버퍼를 위해 대기
    // SSIM과 VMAF를 병렬로 돌리려 했지만 사실상 이 락 때문에 직렬화된다.
    0.0
}
```

**문제**:
- "메모리 할당을 아끼자"는 의도로 scratch buffer 하나를 여러 metric이 공유하게 만들면, 병렬 실행을 위해 나눈 스레드들이 이 락에서 다시 직렬화되어 PIPE-005에서 얻으려던 병렬성 이득이 사라진다.
- 락 경합이 심해지면 스레드 컨텍스트 스위칭 비용까지 추가되어, 애초에 스크래치 버퍼를 각자 할당했을 때보다 더 느려질 수 있다.
- 버그 유입 경로도 된다: 한 metric이 scratch를 다 쓰기 전에 다른 metric이 같은 버퍼를 다른 목적으로 덮어쓰면(락 스코프 실수 등) 조용히 잘못된 계산 결과가 나올 수 있다.

**발생 조건**:
- 메모리 할당/해제 비용을 줄이려는 최적화 시도로 스레드 간 공유 스크래치 풀을 도입했는데, 그 풀의 자료구조가 스레드별 슬롯이 아니라 단일 `Mutex<Buffer>`였을 때.
- 병렬화를 나중에 추가하면서 원래 단일 스레드용으로 설계된 scratch buffer 재사용 코드를 그대로 멀티스레드 환경에 옮겼을 때.

**권장**:
```rust
thread_local! {
    // 스레드마다 독립적인 스크래치 버퍼 — 락 없이 재사용
    static SCRATCH: RefCell<Vec<f32>> = RefCell::new(Vec::new());
}

fn compute_ssim(pair: &FramePair) -> f64 {
    SCRATCH.with(|buf| {
        let mut buf = buf.borrow_mut();
        gaussian_blur_into(&pair.reference, &mut buf);
        // ...
        0.0
    })
}
```
또는 스레드 풀 크기만큼 스크래치 버퍼를 미리 만들어 워커별로 고정 할당(rayon의 스레드 로컬 풀, `object_pool` 크레이트 등)한다.
- 스크래치 메모리 재사용의 이득(할당 비용 절감)은 유지하면서, 공유 대상을 "여러 metric"이 아니라 "한 스레드 안에서의 반복 호출"로 좁힌다.
- 정말 스레드 간 공유가 필요하다면(메모리가 극도로 제한적인 환경) 락 대신 lock-free 슬롯 풀이나 라운드로빈 배정을 고려한다.

**탐지 방법**:
- Runtime: 병렬 metric 실행 중 `Mutex`/`RwLock` 대기 시간을 계측(예: `parking_lot`의 통계 기능, 또는 `perf lock`)해 병렬 스레드 수 대비 유효 동시성이 1에 가까우면 의심.
- Structural: 여러 metric 함수가 동일한 `Mutex<T>`/`RwLock<T>` 필드를 인자로 받는지 확인.

**예외**:
- 스크래치 버퍼가 애초에 병렬 실행되지 않는(항상 순차 실행되는) 경로에서만 쓰인다면 락 오버헤드가 무시할 수준일 수 있다. 다만 이 경우 애초에 락 자체가 불필요하다.

**Bitvue 판정**: N/A — crates/bitvue-metrics/src, crates/bitvue-cli/src/commands/quality.rs, crates/bitvue-sidecar/src/debug_yuv.rs 어디에도 `Mutex<`/`thread_local!`/`RefCell<` 패턴이 없다(grep 결과 전무). crates/bitvue-engine/src에 `Mutex` 사용처가 여럿 있지만(event_observer.rs, index_session.rs, worker.rs 등) 전부 인덱싱/이벤트버스/작업큐용이고 PSNR/SSIM/VMAF가 공유하는 scratch 버퍼가 아니다.

---

### PIPE-013: frame pair가 해제되지 않아 pipeline memory 증가
**분류**: PIPE · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct AnalysisSession {
    // 리포트/디버깅 목적으로 처리한 모든 프레임 쌍을 계속 보관
    history: Vec<Arc<FramePair>>,
}

impl AnalysisSession {
    pub fn process(&mut self, pair: Arc<FramePair>) -> QualityReport {
        let report = compute_all_metrics(&pair);
        self.history.push(pair); // "혹시 나중에 다시 볼 수도 있으니" 계속 쌓인다.
        report
    }
}
```

**문제**:
- 긴 스트림(수만 프레임)을 분석하면 `history`가 모든 프레임 쌍의 `Arc`를 영구히 들고 있어, 처리 진행률에 정비례해 메모리 사용량이 계속 증가한다.
- 각 metric 계산이 끝난 프레임 쌍은 (재생/스크러빙 등 명시적 요구가 없는 한) 더 이상 필요하지 않은데도 "혹시 몰라서" 보관하는 습관이 실질적인 메모리 누수와 동일한 효과를 낸다.
- 장시간 실행되는 분석 세션(예: 백그라운드에서 여러 클립을 순차 비교)에서는 결국 OOM으로 이어지거나, OS가 스왑을 시작해 전체 시스템이 느려진다.

**발생 조건**:
- "디버깅 편의를 위해 처리 기록을 남기자"는 요구가 명시적인 보관 정책(개수 제한, TTL, 다운샘플링) 없이 구현되었을 때.
- undo/재분석 기능을 위해 프레임 쌍 자체를 캐시하려 했지만, 캐시 크기 상한을 두지 않았을 때(CACHE-001과 동일한 근본 원인).

**권장**:
```rust
pub struct AnalysisSession {
    // 프레임 쌍이 아니라 이미 계산된 스칼라 결과만 기록에 남긴다.
    history: Vec<QualityReport>,
    // 최근 N개 프레임 쌍만 재생/스크러빙용으로 짧게 보관 (바이트 예산 기반, CACHE-001 참조)
    recent_frames: ByteBudgetCache,
}

impl AnalysisSession {
    pub fn process(&mut self, pair: Arc<FramePair>) -> QualityReport {
        let report = compute_all_metrics(&pair);
        self.history.push(report.clone()); // 수십 바이트 — 무제한 보관해도 안전한 크기
        self.recent_frames.put(pair.index, pair); // 용량 제한된 캐시, 오래된 것은 자동 축출
        report
    }
}
```
- "결과"(가벼움)와 "원본 프레임 데이터"(무거움)를 분리해서 보관 정책을 각각 다르게 적용한다.
- 프레임 데이터를 정말 보관해야 한다면 CACHE-001의 byte-budget 캐시처럼 명시적 상한이 있는 자료구조를 사용한다.

**탐지 방법**:
- Runtime: 긴 스트림 처리 중 RSS를 시간에 따라 그래프로 그려 처리 진행률에 선형 비례해서 계속 증가하는지 확인(정상이라면 어느 시점에서 안정화되어야 한다).
- Structural: `Vec<Arc<FramePair>>`(또는 유사 타입)를 필드로 갖는 구조체 중 push만 있고 상한/축출 로직이 없는 곳을 grep.

**예외**:
- 짧은 클립(수십~수백 프레임)만 다루는 워크로드로 전체 프레임을 메모리에 유지해도 실측상 문제가 없다고 확인된 경우. 이 경우에도 상한을 두는 것이 안전하지만 심각도는 낮아진다.

**Bitvue 판정**: N/A — metric 계산 경로에 세션 전체에 걸쳐 프레임을 누적 보관하는 `history`류 구조가 없다. CLI의 `Vec<(Vec<u8>, usize, usize)>`(`ref_decoded`/`dist_decoded`, quality.rs 60-62행)는 `run()`/`compute_frame_metrics` 호출 하나에 스코프된 로컬 변수로 반환 시 drop된다. 참고로 UI 비교(diff overlay) 캐시인 `CompareCacheManager`(crates/bitvue-engine/src/compare_cache.rs 43행)는 이미 `evict_lru_stream`/`evict_lru_diff`(385/402행) 기반 bounded LRU 축출을 갖추고 있다 — CACHE-001과 같은 byte-budget 정책이 다른 서브시스템엔 이미 존재. (다만 CLI가 클립 전체를 한 번에 `Vec`로 미리 다 디코드해두는 것 자체는 PIPE-008에서 언급한 별개의 "무제한 사전 로드" 성격 이슈.)

---

### PIPE-014: backpressure가 없어 decode가 metric보다 앞서감
**분류**: PIPE · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn run_pipeline(reference_path: &Path, distorted_path: &Path) {
    let (tx, rx) = std::sync::mpsc::channel(); // unbounded, PIPE-008과 동일 근본 원인
    let decode_handle = thread::spawn(move || {
        let mut ref_dec = MediaCoreDecoder::open(reference_path).unwrap();
        let mut dist_dec = MediaCoreDecoder::open(distorted_path).unwrap();
        // 디코드는 GPU 가속으로 매우 빠름 (예: 초당 500프레임)
        while let (Some(r), Some(d)) = (ref_dec.next_frame(), dist_dec.next_frame()) {
            tx.send(FramePair::new(r, d)).unwrap(); // metric 소비 속도와 무관하게 계속 보냄
        }
    });
    let metric_handle = thread::spawn(move || {
        for pair in rx {
            // VMAF는 초당 10프레임 수준으로 느림 — 디코드 대비 50배 느림
            compute_all_metrics(&pair);
        }
    });
    decode_handle.join().unwrap();
    metric_handle.join().unwrap();
}
```

**문제**:
- 디코드가 GPU 가속 등으로 metric 계산보다 훨씬 빠르면, 소비되지 않은 `FramePair`가 채널에 무제한으로 쌓인다(PIPE-008의 직접적 원인이자 결과).
- 이 문제는 채널 용량을 bounded로 바꾸는 것만으로 근본 해결되지 않을 수 있다 — bounded 채널이라도 producer가 "얼마나 빨리 생산해도 되는지"를 스스로 조절하는 로직이 없다면, producer 스레드가 `send()`에서 계속 블록되며 CPU를 낭비하거나, 디코더가 내부적으로 자체 버퍼를 계속 채우는 방식이라면 그쪽에서 메모리가 쌓인다.
- 여러 스트림을 동시에 분석하는 배치 시나리오에서는 각 스트림 파이프라인이 독립적으로 이 문제를 겪어 전체 메모리 사용량이 스트림 수만큼 곱해진다.

**발생 조건**:
- 디코드와 metric 계산의 처리 속도 차이가 큰 코덱/metric 조합(예: 하드웨어 디코드 + VMAF)을 사용할 때.
- 파이프라인을 "생산자-소비자" 구조로 설계하면서 소비자의 처리 속도를 생산자에게 피드백하는 메커니즘(bounded 채널, 세마포어, 명시적 credit 시스템)을 두지 않았을 때.

**권장**:
```rust
fn run_pipeline(reference_path: &Path, distorted_path: &Path, in_flight_budget: usize) {
    let (tx, rx) = std::sync::mpsc::sync_channel(in_flight_budget); // bounded — send()가 자연스럽게 대기
    let decode_handle = thread::spawn(move || {
        let mut ref_dec = MediaCoreDecoder::open(reference_path).unwrap();
        let mut dist_dec = MediaCoreDecoder::open(distorted_path).unwrap();
        while let (Some(r), Some(d)) = (ref_dec.next_frame(), dist_dec.next_frame()) {
            // 큐가 가득 차면 여기서 block되어 디코드 스레드가 자동으로 느려진다.
            tx.send(FramePair::new(r, d)).unwrap();
        }
    });
    let metric_handle = thread::spawn(move || {
        for pair in rx {
            compute_all_metrics(&pair);
        }
    });
    decode_handle.join().unwrap();
    metric_handle.join().unwrap();
}
```
- bounded 채널을 backpressure 메커니즘으로 명시적으로 채택하고, `in_flight_budget`을 사용 가능한 메모리와 프레임 크기로부터 산정한다.
- 디코더 자체가 내부 버퍼를 갖는 경우(하드웨어 디코더의 출력 큐 등) 그 버퍼 크기도 함께 제한해야 채널 앞단에서 다시 쌓이는 것을 막을 수 있다.

**탐지 방법**:
- Runtime: 디코드 스레드와 metric 스레드의 처리량(프레임/초)을 각각 계측해 지속적으로 차이가 나는지, 그 차이만큼 큐/메모리가 누적되는지 확인.
- Structural: producer-consumer 스레드 쌍이 unbounded 채널로 연결되어 있는지 grep(PIPE-008과 함께 검토).

**예외**:
- 디코드와 metric 처리 속도가 실측상 비슷하거나 metric이 더 빠른 조합(예: 저해상도 + PSNR만)에서는 backpressure의 실질적 이득이 작을 수 있다. 다만 코덱/metric 조합이 바뀌면 다시 문제가 될 수 있으므로 구조적으로는 갖춰두는 것이 안전하다.

**관련**: `CACHE.md` CACHE-023 참고 — 이쪽은 파이프라인 큐의 in-flight budget을 프레임 크기 기반으로 산정하는 backpressure 문제이고, CACHE-023은 프레임 캐시 예산을 해상도에 비례해 산정하는 문제다.

**Bitvue 판정**: N/A — PIPE-008과 동일한 근거: 디코드와 metric 계산을 잇는 별도 스레드/채널이 없어 "디코드가 metric보다 앞서가는" 생산자-소비자 구조 자체가 존재하지 않는다(CLI는 디코드 완료 후 동기적으로 metric 계산, sidecar는 프레임 1개당 IPC 요청 1개). backpressure 메커니즘이 필요할 producer/consumer 분리가 아직 도입되지 않았다는 뜻이지, 이미 있는데 배압이 빠졌다는 뜻은 아니다.

---

### PIPE-015: metric 실패 시 전체 batch를 중단
**분류**: PIPE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub fn analyze_batch(clips: &[ClipPair]) -> Result<Vec<QualityReport>, AnalysisError> {
    let mut reports = Vec::new();
    for clip in clips {
        // 클립 하나에서 VMAF가 실패(예: 특정 프레임에서 신경망 추론 오류)하면
        // `?`가 전체 배치 함수를 즉시 반환시켜 나머지 클립은 아예 처리되지 않는다.
        let report = compute_all_metrics(clip)?;
        reports.push(report);
    }
    Ok(reports)
}
```

**문제**:
- 배치에 클립 수십~수백 개가 있을 때, 그중 하나의 metric 계산이 실패(디코드 오류, 극단적 해상도로 인한 VMAF 모델 불일치 등)하면 이미 성공적으로 계산된 다른 클립들의 결과까지 모두 버려진다.
- 사용자는 "왜 실패했는지"뿐 아니라 "실패하지 않은 나머지 결과가 어디 갔는지"도 알 수 없어 배치 재실행에 큰 비용을 치른다(이미 성공한 클립까지 처음부터 다시 계산).
- 하나의 metric(예: VMAF)만 실패했을 뿐 같은 클립의 PSNR/SSIM은 이미 정상 계산되었을 수 있는데, 이마저도 버려진다(metric 단위 부분 실패를 클립 단위, 배치 단위로 전파시키는 문제).

**발생 조건**:
- `Result<T, E>`와 `?` 연산자를 사용한 초기 구현에서 "일단 동작하게" 만드는 데 집중해 부분 실패 처리를 뒤로 미뤘을 때.
- 에러 타입이 세분화되지 않아(ERR 카탈로그 참고) "이 에러가 이 클립만의 문제인지, 파이프라인 전체가 잘못된 것인지" 구분할 수 없을 때.

**권장**:
```rust
pub struct BatchReport {
    pub succeeded: Vec<(ClipId, QualityReport)>,
    pub failed: Vec<(ClipId, AnalysisError)>,
}

pub fn analyze_batch(clips: &[ClipPair]) -> BatchReport {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for clip in clips {
        match compute_all_metrics(clip) {
            Ok(report) => succeeded.push((clip.id, report)),
            Err(e) => failed.push((clip.id, e)), // 이 클립만 건너뛰고 배치는 계속 진행
        }
    }
    BatchReport { succeeded, failed }
}

// metric 단위 부분 실패도 같은 원칙으로 클립 단위 결과에 반영한다.
pub struct QualityReport {
    pub psnr: Result<f64, MetricError>,
    pub ssim: Result<f64, MetricError>,
    pub vmaf: Result<f64, MetricError>, // VMAF만 실패해도 psnr/ssim은 값을 유지
}
```
- 배치 처리 함수는 "전부 성공 아니면 실패"가 아니라 성공/실패를 클립 단위로 모아 반환하는 구조로 설계한다.
- 클립 내부에서도 metric 단위로 성공/실패를 독립적으로 표현해, 하나의 metric 실패가 같은 클립의 다른 metric 결과까지 무효화하지 않게 한다.

**탐지 방법**:
- Structural: 배치 처리 루프 안에서 `?` 연산자로 개별 항목의 에러를 즉시 상위로 전파하는 패턴을 grep.
- Manual: 에러 처리 정책 문서/PR에서 "부분 실패", "partial failure", "skip and continue" 같은 표현이 있는지, 실제 구현이 이를 따르는지 대조.

**예외**:
- 배치 내 클립들이 서로 강하게 의존적인 경우(예: 이전 클립의 결과를 다음 클립 정규화에 사용하는 파이프라인)라면 조기 중단이 오히려 올바른 동작일 수 있다. 이때는 "부분 실패 허용"이 아니라 "의존성 있는 단계에서의 명확한 실패 전파"가 목표가 되어야 한다.

**Bitvue 판정**: N/A — 여러 클립을 한 번에 도는 배치 기능 자체가 코드베이스에 없다(현재 "batch"는 quality.rs에서 단일 파일 쌍의 "여러 프레임" 배치만 지칭). 오히려 단일 파이프라인 내부에서는 이미 이 항목의 권장안과 같은 원칙을 프레임 단위로 따른다 — 해상도가 안 맞는 프레임은 경고 후 skip하고(crates/bitvue-cli/src/commands/quality.rs 85-91행) 그 프레임의 다른 metric이나 이후 프레임 처리를 막지 않는다.

---

### PIPE-016: metric마다 별도 스레드 풀을 생성해 CPU를 오버서브스크립션
**분류**: PIPE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct MetricEngine {
    psnr_pool: rayon::ThreadPool,
    ssim_pool: rayon::ThreadPool,
    vmaf_pool: rayon::ThreadPool,
}

impl MetricEngine {
    pub fn new() -> Self {
        // 각 metric이 "자기 것"이라는 생각으로 num_cpus만큼의 풀을 각자 만든다.
        let n = num_cpus::get();
        Self {
            psnr_pool: rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap(),
            ssim_pool: rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap(),
            vmaf_pool: rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap(),
        }
        // 8코어 머신이라면 24개 워커 스레드가 8개 코어를 두고 경쟁하게 된다.
    }
}
```

**문제**:
- 물리 코어 수를 기준으로 스레드 풀을 만드는 것이 정석이지만, metric마다 독립적으로 "코어 수만큼" 풀을 만들면 metric 개수 × 코어 수만큼의 워커 스레드가 동시에 존재해 실제 코어 수를 몇 배 초과(oversubscription)한다.
- 스레드 수가 코어 수를 넘으면 OS 스케줄러가 컨텍스트 스위칭을 빈번히 수행해야 하고, CPU 캐시(특히 L1/L2)가 스레드 전환마다 무효화되어 전체 처리량이 오히려 떨어진다.
- 각 metric 풀이 서로의 존재를 모르기 때문에, 어떤 metric도 "지금 시스템이 이미 바쁘다"는 사실을 반영해 작업을 양보하지 않는다.

**발생 조건**:
- metric 구현체들을 서로 다른 크레이트/모듈로 나누면서 각 모듈이 자기 완결적으로 동작하도록(외부 스레드 풀에 의존하지 않도록) 설계했을 때.
- `rayon::ThreadPoolBuilder`나 수동 `thread::spawn` 루프를 라이브러리 초기화 코드에 넣고, 그 라이브러리를 여러 metric 모듈이 각자 임포트해 사용할 때.

**권장**:
```rust
pub struct MetricEngine {
    // 전체 파이프라인이 공유하는 단일 스레드 풀
    shared_pool: Arc<rayon::ThreadPool>,
}

impl MetricEngine {
    pub fn new() -> Self {
        let n = num_cpus::get();
        Self { shared_pool: Arc::new(rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap()) }
    }

    pub fn compute_all(&self, pair: &FramePair) -> QualityReport {
        self.shared_pool.install(|| {
            let (psnr, (ssim, vmaf)) = rayon::join(
                || compute_psnr(pair),
                || rayon::join(|| compute_ssim(pair), || compute_vmaf(pair)),
            );
            QualityReport { psnr, ssim, vmaf }
        })
    }
}
```
- 파이프라인 전체가 스레드 풀 하나를 공유하게 하고, metric 모듈은 풀을 직접 만들지 않고 주입(dependency injection)받는다.
- rayon처럼 work-stealing 스케줄러를 가진 라이브러리를 쓰면 metric 간 부하 불균형(예: VMAF가 오래 걸리는 동안 PSNR/SSIM 워커가 남는 상황)도 같은 풀 안에서 자연스럽게 재분배된다.

**탐지 방법**:
- Structural: `ThreadPoolBuilder::new()`(또는 `thread::spawn` 루프로 만든 수동 풀) 생성 호출이 metric 모듈 개수만큼 존재하는지 grep.
- Runtime: 파이프라인 실행 중 살아있는 OS 스레드 수를 확인해 물리 코어 수를 몇 배 초과하는지 확인.

**예외**:
- metric마다 우선순위/QoS가 근본적으로 다르고 이를 OS 스케줄링 클래스나 cgroup으로 분리해야 하는 특수 배포 환경이라면 의도적으로 별도 풀을 둘 수 있다. 이 경우에도 풀별 스레드 수의 합이 코어 수를 크게 넘지 않도록 조정해야 한다.

**Bitvue 판정**: N/A — `ThreadPoolBuilder`/수동 스레드 풀 생성 코드가 crates/bitvue-metrics, crates/bitvue-engine, crates/bitvue-sidecar, crates/bitvue-cli 어디에도 없다(grep 결과 전무). 유일한 rayon 사용처(`batch_psnr_parallel`/`batch_ssim_parallel`의 `par_iter`, crates/bitvue-metrics/src/lib.rs 329-387행)는 rayon 전역 공유 풀에 의존하고, opt-in feature 뒤에 있으며 호출부가 없는 dead code다(PIPE-005 판정 참고) — 풀 오버서브스크립션을 일으킬 여러 풀이 아예 만들어지지 않는다.

---

### PIPE-017: SSIM/heatmap이 같은 sliding-window 통계를 독립적으로 재계산
**분류**: PIPE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_ssim(pair: &NormalizedFramePair) -> f64 {
    // 지역 평균/분산/공분산을 위해 슬라이딩 윈도우(가우시안 커널)를 프레임 전체에 적용
    let local_mean_ref = sliding_gaussian_mean(&pair.reference_rgb, WINDOW);
    let local_mean_dist = sliding_gaussian_mean(&pair.distorted_rgb, WINDOW);
    let local_var_ref = sliding_gaussian_variance(&pair.reference_rgb, WINDOW);
    let local_var_dist = sliding_gaussian_variance(&pair.distorted_rgb, WINDOW);
    let local_cov = sliding_gaussian_covariance(&pair.reference_rgb, &pair.distorted_rgb, WINDOW);
    ssim_from_stats(&local_mean_ref, &local_mean_dist, &local_var_ref, &local_var_dist, &local_cov)
}

fn compute_heatmap(pair: &NormalizedFramePair) -> HeatmapTile {
    // heatmap도 "지역적으로 어디가 다른가"를 보여주기 위해 같은 슬라이딩 윈도우 통계가 필요한데
    // SSIM이 이미 계산한 결과를 재사용하지 않고 처음부터 다시 계산한다.
    let local_mean_ref = sliding_gaussian_mean(&pair.reference_rgb, WINDOW);
    let local_mean_dist = sliding_gaussian_mean(&pair.distorted_rgb, WINDOW);
    let local_var_ref = sliding_gaussian_variance(&pair.reference_rgb, WINDOW);
    let local_var_dist = sliding_gaussian_variance(&pair.distorted_rgb, WINDOW);
    let local_cov = sliding_gaussian_covariance(&pair.reference_rgb, &pair.distorted_rgb, WINDOW);
    heatmap_from_stats(&local_mean_ref, &local_mean_dist, &local_var_ref, &local_var_dist, &local_cov)
}
```

**문제**:
- SSIM의 "지역 SSIM 맵"(픽셀/블록별 SSIM 값)은 사실상 heatmap이 표시하려는 데이터와 동일하거나 거의 동일한데, 이를 공유할 파이프라인 단계가 없어 슬라이딩 윈도우 통계(평균·분산·공분산) 전체를 두 번 계산한다.
- 슬라이딩 윈도우 연산은 프레임 크기에 비례하는 컨볼루션이라 색공간 변환보다도 비쌀 수 있어(가우시안 커널 크기에 따라), 이 중복은 PIPE-002/003과 비슷한 배수의 비용을 유발한다.
- 이런 "metric 결과 자체가 다른 metric/시각화의 입력이 될 수 있다"는 의존 관계는 파이프라인을 병렬 실행(PIPE-005)하려는 시도와 충돌할 수 있어, 설계 초기에 명시적으로 고려하지 않으면 나중에 병렬화를 되돌려야 하는 상황이 생긴다.

**발생 조건**:
- heatmap 기능이 SSIM보다 나중에 추가되면서, SSIM 모듈의 중간 결과(local SSIM map)를 외부에 노출하는 API가 없어 heatmap 모듈이 별도로 통계를 재구현했을 때.
- metric과 시각화(heatmap)가 서로 다른 팀/모듈 경계에 있어 "이미 계산된 것을 재사용하자"는 논의 자체가 이루어지지 않았을 때.

**권장**:
```rust
pub struct SsimResult {
    pub score: f64,
    pub local_map: Arc<Vec<f32>>, // 픽셀/블록별 지역 SSIM 값 — heatmap이 그대로 재사용 가능
}

fn compute_ssim(pair: &NormalizedFramePair) -> SsimResult {
    let local_mean_ref = sliding_gaussian_mean(&pair.reference_rgb, WINDOW);
    let local_mean_dist = sliding_gaussian_mean(&pair.distorted_rgb, WINDOW);
    let local_var_ref = sliding_gaussian_variance(&pair.reference_rgb, WINDOW);
    let local_var_dist = sliding_gaussian_variance(&pair.distorted_rgb, WINDOW);
    let local_cov = sliding_gaussian_covariance(&pair.reference_rgb, &pair.distorted_rgb, WINDOW);
    let local_map = Arc::new(ssim_map_from_stats(&local_mean_ref, &local_mean_dist, &local_var_ref, &local_var_dist, &local_cov));
    SsimResult { score: average(&local_map), local_map }
}

fn compute_heatmap(ssim: &SsimResult) -> HeatmapTile {
    // 이미 계산된 local_map을 색상으로 매핑만 하면 된다 — 슬라이딩 윈도우 재계산 없음
    HeatmapTile::from_local_map(&ssim.local_map)
}
```
- metric의 중간 표현(지역 통계, local map)을 내부 구현 세부사항으로 숨기지 말고, 다른 단계가 재사용할 수 있는 명시적 출력으로 노출한다.
- heatmap처럼 시각화 목적의 산출물은 가능한 한 "metric의 부산물"로 설계해 별도 연산 경로를 만들지 않는다.

**탐지 방법**:
- Structural: 서로 다른 두 함수(예: `compute_ssim`, `compute_heatmap`)가 동일한 시그니처의 슬라이딩 윈도우 함수(`sliding_gaussian_*`)를 동일한 인자로 호출하는지 grep.
- Manual: 코드 리뷰에서 "SSIM local map"과 "heatmap 데이터"가 개념적으로 같은 것을 가리키는지 도메인 지식으로 확인.

**예외**:
- heatmap이 SSIM이 아니라 단순 픽셀 차분(절대값 차이)만 시각화하는 등, 실제로 슬라이딩 윈도우 통계를 요구하지 않는 경우라면 이 항목은 해당하지 않는다.

**Bitvue 판정**: N/A — 정확히 위 예외 조항에 해당한다. `DiffHeatmapData::from_luma_planes`(crates/bitvue-engine/src/diff_heatmap.rs 103행)는 SSIM 슬라이딩 윈도우 통계가 아니라 단순 픽셀 차분만 계산한다. `DiffMode::Metric`은 "실제 metric 델타"용으로 설계돼 있지만 구현이 없어 signed diff로 fallback하는 스텁이다(144행 주석 "Fallback to signed if no metric"). `bitvue_metrics::ssim()`(crates/bitvue-metrics/src/lib.rs 140행)은 local SSIM map을 반환/노출하지 않으므로(f64 스칼라만 반환) heatmap이 재사용할 대상 자체가 없다 — 공유할 계산이 존재하지 않는 상태.

---

### PIPE-018: 공유 프레임 버퍼를 metric이 in-place로 변형해 다음 metric을 오염
**분류**: PIPE · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_vmaf(pair: &FramePair) -> f64 {
    // VMAF 내부 구현이 편의상 입력 버퍼를 in-place로 감마 보정 후 재사용
    let buf = pair.reference.pixels_mut(); // &mut 접근 — Arc<DecodedFrame>이지만 내부 Mutex/UnsafeCell로 변형 허용
    apply_gamma_correction_in_place(buf);
    vmaf_core_on(buf)
}

fn compute_ssim(pair: &FramePair) -> f64 {
    // VMAF가 먼저 실행된 뒤라면, 이 시점의 pair.reference는
    // 이미 감마 보정이 적용된 상태 — SSIM은 원본을 기대했는데 오염된 데이터를 받는다.
    ssim_core_on(pair.reference.pixels())
}

pub fn compute_all_metrics(pair: &FramePair) -> QualityReport {
    let vmaf = compute_vmaf(pair); // 순서에 따라 결과가 달라지는 숨은 의존성
    let ssim = compute_ssim(pair); // 이 값은 이제 잘못됐다 — 하지만 컴파일도 되고 패닉도 안 남
    QualityReport { psnr: 0.0, ssim, vmaf }
}
```

**문제**:
- `Arc<DecodedFrame>`으로 프레임을 공유하는 것(PIPE-009의 권장 사항)은 "읽기 전용 공유"를 전제로 한 최적화인데, 어떤 metric이 내부적으로 그 버퍼를 in-place로 변형하면 이 전제가 조용히 깨진다.
- 컴파일러가 이 문제를 잡아주지 못하는 경우가 흔하다 — 변형이 `unsafe`, `UnsafeCell`, 또는 FFI(외부 VMAF 라이브러리가 입력 포인터를 non-const로 요구하는 경우 등)를 통해 이루어지면 Rust의 대여 검사기가 개입할 지점이 없다.
- 가장 위험한 특징은 "조용히 틀린 결과"라는 점이다. 크래시하거나 컴파일 에러가 나는 게 아니라 SSIM/PSNR 점수가 실행 순서에 따라 달라지는 미묘한 버그가 되어, 회귀 테스트에서조차 발견하기 어렵다(같은 입력, 같은 코드인데 metric 실행 순서만 바뀌어도 결과가 달라짐).
- PIPE-005/PIPE-016에서 권장한 병렬 실행과 정면으로 충돌한다 — 병렬로 실행하면 어떤 metric이 먼저 변형을 가할지조차 비결정적이 되어 결과 재현성이 완전히 무너진다.

**발생 조건**:
- 외부 metric 라이브러리(특히 C/C++ FFI로 감싼 VMAF 등)가 입력 버퍼를 "작업 공간"처럼 취급해 non-const 포인터를 요구하고, 이를 감싸는 Rust 바인딩이 안전하지 않은 방식으로 `Arc` 내부 데이터에 가변 접근을 허용했을 때.
- 성능을 위해 "여기서 한 번 감마 보정한 김에 원본 버퍼에 바로 써버리자"는 최적화를 별도 출력 버퍼 없이 구현했을 때.
- 코드 리뷰에서 각 metric 함수의 부수 효과(side effect)를 시그니처만으로는 파악할 수 없어(입력이 `&FramePair`로 보이지만 내부에서 변형이 일어남) 놓쳤을 때.

**권장**:
```rust
fn compute_vmaf(pair: &FramePair) -> f64 {
    // in-place 변형 대신 별도 출력 버퍼에 감마 보정 결과를 쓴다.
    let mut gamma_corrected = pair.reference.pixels().to_owned(); // 필요한 경우에만 복사
    apply_gamma_correction_in_place(&mut gamma_corrected);
    vmaf_core_on(&gamma_corrected)
}

fn compute_ssim(pair: &FramePair) -> f64 {
    // pair.reference는 어떤 metric이 실행되었든 항상 원본 그대로다.
    ssim_core_on(pair.reference.pixels())
}
```
- `FramePair`가 가리키는 버퍼는 파이프라인 전체에서 불변(immutable) 계약으로 취급하고, 이를 타입 시스템으로 강제한다(`Arc<DecodedFrame>`의 픽셀 접근자를 `&self` 전용으로만 노출하고 `unsafe`/`UnsafeCell` 우회를 금지).
- FFI로 non-const 포인터를 요구하는 외부 라이브러리를 감쌀 때는 항상 복사본을 넘기거나, 라이브러리가 실제로 입력을 변형하지 않는다는 것을 문서/테스트로 검증한 뒤에만 원본을 공유한다.
- metric 함수 시그니처에서 "이 함수가 입력을 변형하지 않는다"는 것이 코드만 보고 명확해야 한다 — 필요하다면 `&FramePair`가 아니라 `&ImmutableFramePair`처럼 타입으로 계약을 드러낸다.

**탐지 방법**:
- Semantic: metric 함수 내부에서 공유 버퍼에 대해 `unsafe`, `as_mut_ptr`, FFI로의 non-const 포인터 전달이 있는지 코드 리뷰로 확인. 자동 탐지가 어려워 반드시 사람이 각 metric의 입력 계약을 검증해야 한다.
- Runtime: 동일 입력에 대해 metric 실행 순서를 바꿔가며(예: VMAF→SSIM vs SSIM→VMAF) 결과를 비교하는 회귀 테스트를 추가해 순서에 따라 값이 달라지면 이 안티패턴이 존재한다는 강력한 신호로 삼는다.
- Manual: 외부 metric 라이브러리(특히 C FFI 바인딩) 도입 시 "입력 버퍼가 const인가"를 체크리스트 항목으로 명시.

**예외**:
- metric이 자신만을 위해 새로 할당한 버퍼(다른 metric과 공유되지 않는 로컬 복사본)를 변형하는 것은 문제가 아니다. 이 항목은 어디까지나 "여러 metric이 공유하는 버퍼"에 대한 변형에 한정된다.
- 파이프라인이 애초에 metric을 항상 고정된 순서로만 순차 실행하고, 그 순서가 의도적으로 "VMAF의 감마 보정 결과를 이후 metric의 입력으로 삼는다"고 설계 문서에 명시된 경우라면 in-place 변형이 아니라 의도된 단계적 변환일 수 있다. 이 경우 PIPE-007의 원칙(중간 표현을 명시적으로 다루기)을 따르는 것으로 재분류한다.

**Bitvue 판정**: N/A — PSNR/SSIM/VMAF 모두 공유 프레임 데이터를 불변 참조로만 다룬다. `psnr`/`ssim`(crates/bitvue-metrics/src/lib.rs)과 SIMD 구현(simd.rs)은 전부 `&[u8]` 불변 슬라이스를 받고, simd.rs의 `unsafe` 블록들(49/93/232/372행 등)도 CPU-feature 인트린식용일 뿐 버퍼를 쓰기 변형하지 않는다. `VmafFrame::to_vmaf_picture`(vmaf.rs 57행)는 `&self`를 받아 새 `VmafPicture`에 `write_plane`으로 복사할 뿐(71/82/93행) self를 변형하지 않는다. 공유 버퍼에 대한 `as_mut_ptr`/non-const FFI 전달 패턴은 발견되지 않음.
