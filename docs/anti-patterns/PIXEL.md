# Anti-Pattern Catalog — PIXEL: Decode·Pixel·Image Pipeline

이 문서는 더 큰 안티패턴 카탈로그의 일부입니다(전체 목록은 별도로 작성 중인 `docs/anti-patterns/INDEX.md` 참고). 본 파일은 1단계(일반 참조 카탈로그)이며, 2단계에서 Bitvue 저장소를 실제로 감사하여 각 항목의 "Bitvue 판정"을 채웁니다.

---

### PIXEL-001: YUV→RGBA를 모든 분석에 선행
**분류**: 파이프라인 설계 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
fn analyze_frame(decoded: &DecodedFrame) -> FrameAnalysis {
    // PSNR, 히스토그램, MB 통계 등 모든 분석 전에 무조건 RGBA로 먼저 변환
    let rgba = yuv_to_rgba(&decoded.y, &decoded.u, &decoded.v, decoded.width, decoded.height);

    let psnr = compute_psnr_from_rgba(&rgba, &reference_rgba(decoded));
    let histogram = compute_histogram_from_rgba(&rgba);
    let mb_stats = compute_mb_stats_from_rgba(&rgba, &decoded.mb_info);

    FrameAnalysis { psnr, histogram, mb_stats, rgba }
}
```

**문제**:
- PSNR/SSIM 등 품질 지표는 YUV(특히 Y plane) 위에서 직접 계산하는 것이 정확하고 빠른데, RGBA 변환을 거치면 색공간 왕복 오차가 섞이고 불필요한 연산이 추가된다.
- RGBA는 화면 표시(디스플레이)에만 필요한 표현인데, 분석 로직이 이를 필수 전제로 삼으면 표시하지 않는 프레임(백그라운드 batch 분석 등)에서도 변환 비용을 강제로 지불한다.
- 변환 함수 하나에 모든 소비자가 의존하게 되어, 이후 병목 프로파일링 시 "분석이 느리다"와 "렌더링이 느리다"를 구분하기 어려워진다.

**발생 조건**:
- 4K/8K 프레임에서 프레임당 RGBA 변환이 수 ms 이상 걸리는 경우, 분석 파이프라인 전체에 이 비용이 누적된다.
- CLI/batch 모드로 다수 프레임을 순회하며 통계만 뽑아낼 때 특히 낭비가 크다.

**권장**:
```rust
fn analyze_frame(decoded: &DecodedFrame) -> FrameAnalysis {
    // 분석은 YUV plane에서 직접 수행
    let psnr = compute_psnr_yuv(&decoded.y, &reference_y(decoded), decoded.bit_depth);
    let histogram = compute_histogram_yuv(&decoded.y, decoded.bit_depth);
    let mb_stats = compute_mb_stats(&decoded.mb_info);

    FrameAnalysis { psnr, histogram, mb_stats }
}

// RGBA 변환은 실제로 화면에 그릴 필요가 있는 경로에서만 별도로 호출
fn prepare_for_display(decoded: &DecodedFrame) -> RgbaBuffer {
    yuv_to_rgba(&decoded.y, &decoded.u, &decoded.v, decoded.width, decoded.height)
}
```
- 분석 함수와 표시(디스플레이) 함수를 별도 경로로 분리하고, RGBA는 "지연 계산(lazy)"으로 만든다.
- 품질 지표는 YUV/원본 bit-depth에서 직접 계산하는 커널을 라이브러리화한다.

**탐지 방법**:
- Structural: 분석 함수 시그니처가 `RgbaBuffer`를 입력으로 받는지, 아니면 `YuvPlanes`/`&DecodedFrame`을 받는지 정적으로 확인.
- Static: `yuv_to_rgba` 호출 지점이 분석 모듈 내부에 있는지 grep으로 확인.

**예외**:
- 분석 로직이 정말로 색상 인지 지표(예: 사람 눈에 보이는 색차 기반 메트릭, CIEDE2000류)를 요구한다면 RGB/Lab 변환이 불가피할 수 있다. 이 경우에도 표시용 RGBA와는 별도의 변환 경로를 둔다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-002: 화면에 보이지 않는 프레임까지 RGBA 변환
**분류**: 파이프라인 설계 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn decode_all_frames(stream: &mut Av1Stream) -> Vec<RgbaBuffer> {
    let mut frames = Vec::new();
    while let Some(decoded) = stream.decode_next_frame() {
        // 타임라인 썸네일 바 하나만 그리는데도 모든 프레임을 RGBA로 미리 변환해 보관
        frames.push(yuv_to_rgba(&decoded.y, &decoded.u, &decoded.v, decoded.width, decoded.height));
    }
    frames
}
```

**문제**:
- 뷰포트에 실제로 표시되는 것은 현재 프레임 1장뿐인데, 스트림 전체를 미리 RGBA로 변환해 메모리에 쌓아두면 4K 영상 기준 프레임당 수십 MB가 즉시 낭비된다.
- GC/할당자 압박이 커지고, 실제로 스킵되거나 seek로 건너뛴 구간의 변환 비용은 100% 낭비다.
- UI 스레드가 아닌 디코드 스레드에서 불필요한 색공간 변환을 계속 돌리면 디코딩 자체의 처리량이 떨어진다.

**발생 조건**:
- 긴 스트림을 처음부터 끝까지 프리로드하는 구조, 또는 "다음 프레임 미리 디코드" 캐시가 RGBA까지 미리 만들어 두는 경우.
- 사용자가 재생 없이 bitstream 구조만 훑어보는(파싱 전용) 세션에서 특히 낭비.

**권장**:
```rust
struct FrameCache {
    decoded: LruCache<FrameIndex, DecodedFrame>, // YUV 상태로 캐시
}

impl FrameCache {
    // 실제로 화면에 그려야 할 때만 RGBA로 변환
    fn rgba_for_display(&mut self, idx: FrameIndex) -> RgbaBuffer {
        let decoded = self.decoded.get(&idx).expect("frame not decoded");
        yuv_to_rgba(&decoded.y, &decoded.u, &decoded.v, decoded.width, decoded.height)
    }
}
```
- 디코드 결과는 YUV 상태로 캐시하고, RGBA 변환은 "현재 뷰포트에 그려야 하는 프레임"에 한해 호출 시점에 수행한다.
- 타임라인 썸네일처럼 다량의 저해상도 미리보기가 필요하면 별도의 축소본 파이프라인(PIXEL-011 참고)을 쓴다.

**탐지 방법**:
- Runtime: 메모리 프로파일러로 `RgbaBuffer` 총 상주량이 "현재 화면에 보이는 프레임 수 × 프레임 크기"를 크게 초과하는지 확인.
- Structural: 디코드 루프와 RGBA 변환 호출이 같은 루프 안에 있는지(=선행 변환) 확인.

**예외**:
- 오프라인 트랜스코딩/썸네일 일괄 생성 배치 작업처럼 "모든 프레임을 결국 다 써야 하는" 경우는 예외. 다만 이 경우도 스트리밍 방식(생성 즉시 소비 후 해제)이 낫다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-003: frame마다 image buffer 재할당
**분류**: 메모리/할당 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn render_frame_to_rgba(decoded: &DecodedFrame) -> Vec<u8> {
    // 매 프레임마다 새 Vec을 힙에 할당
    let mut rgba = vec![0u8; decoded.width * decoded.height * 4];
    convert_yuv_to_rgba_into(decoded, &mut rgba);
    rgba
}
```

**문제**:
- 재생 중 초당 수십 회 호출되는 경로에서 매번 새 힙 버퍼를 할당/해제하면 allocator 락 경합과 페이지 폴트가 프레임 드랍의 주요 원인이 된다.
- 할당 크기가 매 프레임 동일한데도(해상도가 안 바뀌는 한) 이를 재사용하지 않는 것은 순수한 낭비다.
- 메모리 단편화가 누적되어 장시간 재생 시 RSS가 계속 증가하는 것처럼 보일 수 있다(실제 누수는 아니지만 단편화로 인한 증가).

**발생 조건**:
- 60fps 재생, 4K 이상 해상도에서 특히 체감된다.
- overlay 토글이나 zoom 변경으로 재변환이 빈번한 경로(PIXEL-010과 결합 시 악화).

**권장**:
```rust
struct RgbaFrameBuffer {
    data: Vec<u8>,
    width: usize,
    height: usize,
}

impl RgbaFrameBuffer {
    fn ensure_size(&mut self, width: usize, height: usize) {
        let needed = width * height * 4;
        if self.width != width || self.height != height {
            self.data.clear();
            self.data.resize(needed, 0);
            self.width = width;
            self.height = height;
        }
    }
}

fn render_frame_to_rgba(decoded: &DecodedFrame, out: &mut RgbaFrameBuffer) {
    out.ensure_size(decoded.width, decoded.height);
    convert_yuv_to_rgba_into(decoded, &mut out.data);
}
```
- 해상도가 바뀌지 않는 한 버퍼를 재사용하는 "reusable scratch buffer" 패턴을 쓴다.
- 더블/트리플 버퍼링이 필요하면 고정 개수의 버퍼 풀을 두고 순환시킨다.

**탐지 방법**:
- Runtime: 힙 프로파일러(예: `heaptrack`, `dhat`)로 프레임 렌더 경로에서 반복되는 동일 크기 할당 패턴을 확인.
- Structural: 렌더 함수가 `Vec::with_capacity`/`vec![]`를 반환 타입으로 매번 생성하는지 시그니처로 확인.

**예외**:
- 해상도가 프레임마다 바뀌는(어댑티브 스트리밍 등) 극히 드문 경우는 재할당이 불가피하지만, 이때도 "이전 버퍼보다 크면만 재할당" 전략(capacity 재사용)은 여전히 유효하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-004: stride를 무시하고 packed plane 가정
**분류**: 메모리 레이아웃 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn copy_y_plane(decoded: &DecodedFrame, out: &mut [u8]) {
    // width * height 만큼만 연속으로 복사 — plane stride(row pitch)를 무시
    let len = decoded.width * decoded.height;
    out[..len].copy_from_slice(&decoded.y_data[..len]);
}
```

**문제**:
- dav1d를 비롯한 대부분의 디코더는 plane을 CPU 캐시라인/SIMD 정렬을 위해 `width`보다 넓은 stride(row pitch)로 반환한다. `width * height`만큼 연속으로 읽으면 각 행 끝의 패딩 바이트까지 데이터로 오인해 이후 행이 밀리는(skewed) 이미지가 만들어진다.
- 이 버그는 해상도가 정렬 경계(예: 64의 배수)와 우연히 일치하면 숨어 있다가, 특정 해상도(예: 1920처럼 정렬과 어긋나는 폭)에서만 터지는 재현이 까다로운 버그가 된다.
- 크롭 영역(visible rect)이 coded size보다 작은 경우도 같은 stride 문제의 변형이다(PIXEL-015 참고).

**발생 조건**:
- 홀수/비정렬 폭(예: 1920, 1366처럼 SIMD 정렬 배수가 아닌 해상도)의 영상.
- dav1d가 반환하는 `Picture`의 `stride[plane]`이 `width`와 다른 모든 경우 — 실무에서는 거의 항상 다르다.

**권장**:
```rust
fn copy_y_plane(decoded: &DecodedFrame, out: &mut [u8]) {
    let stride = decoded.y_stride; // 디코더가 알려주는 실제 row pitch(바이트 단위)
    let row_bytes = decoded.width; // 실제 유효 데이터 폭
    for row in 0..decoded.height {
        let src_start = row * stride;
        let dst_start = row * row_bytes;
        out[dst_start..dst_start + row_bytes]
            .copy_from_slice(&decoded.y_data[src_start..src_start + row_bytes]);
    }
}
```
- plane 접근은 항상 `(stride, width, height)` 세 값을 함께 다루고, "행 단위로 복사"를 기본 패턴으로 삼는다.
- 가능하면 `stride == width`를 가정하는 fast path와 일반 stride 처리 경로를 분리하되, fast path 진입 조건을 명시적으로 검증한다.

**탐지 방법**:
- Semantic: plane 복사/변환 함수가 `stride` 파라미터를 아예 받지 않는지 시그니처 검사.
- Manual: 비정렬 해상도(예: 폭이 64의 배수가 아닌) 샘플로 렌더링해 시각적으로 이미지가 기울어지는지(shearing) 확인.
- Runtime: dav1d picture의 `stride[0] != width`인 테스트 케이스를 회귀 테스트에 반드시 포함.

**예외**:
- 디코더가 항상 tightly-packed 버퍼만 반환한다고 문서로 보장하는 경우는 없다고 봐야 한다(dav1d는 항상 정렬 stride를 반환). 예외는 사실상 없음 — 반드시 stride를 다뤄야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-005: bit depth를 u8로 강제 축소
**분류**: 비트 심도 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn get_y_plane_u8(decoded: &DecodedFrame) -> Vec<u8> {
    match decoded.bit_depth {
        8 => decoded.y_data_u8().to_vec(),
        10 | 12 => {
            // 상위 8비트만 취하고 하위 비트를 버림 — HDR/10bit 디테일 손실
            decoded.y_data_u16().iter().map(|&v| (v >> (decoded.bit_depth - 8)) as u8).collect()
        }
        _ => unreachable!(),
    }
}
```

**문제**:
- 10/12-bit 콘텐츠(HDR10, AV1 profile 2 등)를 분석 파이프라인 초입에서 8비트로 뭉개면, 이후의 모든 PSNR/히스토그램/QP 오버레이 상관 분석이 원본 정밀도를 잃는다.
- 특히 밴딩(banding) 아티팩트 탐지, HDR 톤매핑 검증처럼 애초에 높은 비트 심도가 필요해서 분석하는 경우, 이 축소가 분석 목적 자체를 무력화한다.
- "8비트로 통일하면 코드가 단순해진다"는 유혹이 크지만, 이는 비트 심도 정보 자체가 분석 대상인 비디오 분석 툴에서는 특히 치명적이다.

**발생 조건**:
- HDR10/HLG 콘텐츠, AV1/HEVC Main10 profile로 인코딩된 10-bit 스트림 분석 시.
- 12-bit profile(AV1 Professional profile 등)을 다루는 경우 손실이 더 커진다.

**권장**:
```rust
enum PlaneData<'a> {
    Bit8(&'a [u8]),
    Bit10Plus(&'a [u16]), // 10/12-bit 모두 u16 컨테이너에 실제 유효 비트만 사용
}

fn compute_histogram(plane: PlaneData, bit_depth: u8) -> Histogram {
    match plane {
        PlaneData::Bit8(data) => Histogram::from_u8(data),
        PlaneData::Bit10Plus(data) => Histogram::from_u16(data, bit_depth), // 원본 정밀도 유지
    }
}
```
- 분석 로직은 원본 bit depth를 그대로 유지하는 제네릭/enum 기반 경로를 갖춘다.
- 화면 표시를 위한 8-bit 변환(톤매핑 or 단순 shift)은 "표시 직전"에만, 그리고 명시적으로 수행한다.

**탐지 방법**:
- Semantic: 함수 시그니처에서 `bit_depth`를 입력받고도 반환 타입이 무조건 `u8`/`Vec<u8>`인 경우 의심.
- Manual: 10-bit 테스트 벡터로 PSNR을 계산해 8-bit 계산 결과와 비교, 부자연스럽게 낮은 정밀도가 나오는지 확인.

**예외**:
- 최종 화면 표시(SDR 디스플레이 대상 RGBA)는 8비트로 귀결되는 것이 정상이며, 이 경우는 축소가 아니라 "표시 변환"이다. 문제는 분석/중간 처리 단계에서의 조기 축소다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-006: 10/12-bit 데이터를 u16 전체 범위로 오해
**분류**: 비트 심도 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn normalize_sample(v: u16) -> f32 {
    // 10-bit 데이터인데 u16 최대값(65535) 기준으로 정규화
    v as f32 / 65535.0
}
```

**문제**:
- 10-bit 샘플은 실제로 0~1023 범위에 저장되지만 `u16` 컨테이너에 담겨 있을 뿐이다. 이를 65535 기준으로 정규화하면 값이 실제보다 훨씬 어둡게(값의 1/64 수준으로) 계산된다.
- 12-bit도 마찬가지로 실제 범위는 0~4095인데 65535로 나누면 1/16 스케일 오차가 발생한다.
- 이 오류는 히스토그램, 밝기 통계, PSNR의 MAX 값(peak signal) 계산 등 정규화가 들어가는 모든 곳에서 조용히 틀린 결과를 낸다 — 크래시가 없어 발견이 늦어진다.

**발생 조건**:
- 10/12-bit 콘텐츠에서 정규화된 float 값을 사용하는 모든 계산(PSNR의 MAX_I, 톤매핑, 밝기 히스토그램).
- bit_depth를 매개변수로 받지 않고 컨테이너 타입(`u16`)만 보고 범위를 추정하는 코드.

**권장**:
```rust
fn normalize_sample(v: u16, bit_depth: u8) -> f32 {
    let max_value = (1u32 << bit_depth) - 1; // 10-bit -> 1023, 12-bit -> 4095
    v as f32 / max_value as f32
}

fn psnr_max_i(bit_depth: u8) -> f64 {
    ((1u32 << bit_depth) - 1) as f64
}
```
- 정규화/최대값 계산은 항상 실제 `bit_depth`를 파라미터로 받아 `(1 << bit_depth) - 1`로 계산한다.
- 컨테이너 타입(u8/u16)과 유효 비트 심도(8/10/12)를 별개 개념으로 취급하는 타입/문서 규칙을 코드베이스 전체에 일관 적용한다.

**탐지 방법**:
- Static: 매직넘버 `65535.0`, `255.0` 등이 `bit_depth`와 무관하게 하드코딩된 곳을 grep.
- Manual: 10-bit 테스트 벡터의 알려진 밝기 값으로 정규화 결과를 검증(예: 중간 회색 512/1023 ≈ 0.5가 나오는지).

**예외**:
- 진짜 8-bit(0~255) 컨텐츠이거나, 이미 명시적으로 "full u16 range"로 재양자화(requantize)한 중간 표현이라면 65535 기준이 맞다 — 다만 이 경우 변수명/타입에 그 사실을 명확히 남겨야 한다.

**관련**: `UIX_VIZ.md` UIX-VIZ-002 참고 — bit-depth 의존 정규화 상수를 빠뜨리는 동일 패턴이나, 대상이 raw 픽셀 샘플 정규화(여기)와 QP 값의 legend 표시(UIX-VIZ-002)로 다름.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-007: chroma subsampling별 코드 중복
**분류**: 코드 구조 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn convert_yuv420_to_rgba(decoded: &DecodedFrame) -> Vec<u8> { /* 420 전용 upsampling 로직 300줄 */ }
fn convert_yuv422_to_rgba(decoded: &DecodedFrame) -> Vec<u8> { /* 422 전용 upsampling 로직 280줄, 420과 90% 동일 */ }
fn convert_yuv444_to_rgba(decoded: &DecodedFrame) -> Vec<u8> { /* 444 전용, upsampling 없음, 나머지 동일 */ }

fn convert(decoded: &DecodedFrame) -> Vec<u8> {
    match decoded.chroma_format {
        ChromaFormat::Yuv420 => convert_yuv420_to_rgba(decoded),
        ChromaFormat::Yuv422 => convert_yuv422_to_rgba(decoded),
        ChromaFormat::Yuv444 => convert_yuv444_to_rgba(decoded),
    }
}
```

**문제**:
- 세 함수가 색공간 변환 행렬, clamp, 출력 packing 로직을 거의 그대로 복제하고 있어, 색공간 계수 하나를 고치려면 세 곳을 동시에 고쳐야 한다(실무에서는 한 곳을 빠뜨리는 사고가 반드시 발생한다).
- subsampling에 따라 다른 것은 사실 "크로마 업샘플링 방식"뿐인데, 이를 위해 전체 파이프라인을 통째로 복제하는 것은 과도한 중복이다.
- 새로운 subsampling(예: 4:4:0, monochrome)을 추가할 때마다 또 하나의 거대 함수를 복붙하게 되는 구조적 부채가 쌓인다.

**발생 조건**:
- AV1/HEVC처럼 여러 subsampling을 지원하는 코덱을 다루면서 각 포맷을 순차적으로 추가 구현해온 코드베이스에서 흔히 발생.

**권장**:
```rust
trait ChromaUpsampler {
    fn sample_uv(&self, u_plane: &Plane, v_plane: &Plane, x: usize, y: usize) -> (i32, i32);
}

struct Upsample420;
struct Upsample422;
struct Upsample444;

impl ChromaUpsampler for Upsample420 { /* 2x2 -> 1 매핑만 담당 */ }
impl ChromaUpsampler for Upsample422 { /* 2x1 -> 1 매핑만 담당 */ }
impl ChromaUpsampler for Upsample444 { /* 1:1, 그대로 반환 */ }

fn convert(decoded: &DecodedFrame) -> Vec<u8> {
    let upsampler: &dyn ChromaUpsampler = match decoded.chroma_format {
        ChromaFormat::Yuv420 => &Upsample420,
        ChromaFormat::Yuv422 => &Upsample422,
        ChromaFormat::Yuv444 => &Upsample444,
    };
    convert_common(decoded, upsampler) // 공통 색공간 변환/packing 로직은 한 곳
}
```
- "달라지는 부분"(크로마 업샘플링)만 전략 패턴/제네릭으로 분리하고, 공통 변환 파이프라인은 단일화한다.
- 성능이 중요한 경우 제네릭 + 모노모픽화(monomorphization)로 컴파일 타임에 인라인시켜 trait object 오버헤드를 없앨 수 있다.

**탐지 방법**:
- Structural: 함수 이름이 `_420`/`_422`/`_444` 등 포맷별 접미사를 갖는 거의 동일 크기의 함수 세트가 존재하는지 검색.
- Static: 유사도 분석 도구(예: `similarity-rs`, 중복 코드 탐지기)로 90% 이상 유사한 함수 블록 탐지.

**예외**:
- monochrome(4:0:0)처럼 로직이 근본적으로 다른(크로마 자체가 없는) 케이스는 완전히 별도 경로로 두는 것이 오히려 명확할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-008: 색공간·range·transfer를 무시
**분류**: 색공간 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn yuv_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    // BT.601 계수를 하드코딩, full/limited range도 무시, transfer/primaries도 무시
    let r = y as f32 + 1.402 * (v as f32 - 128.0);
    let g = y as f32 - 0.344 * (u as f32 - 128.0) - 0.714 * (v as f32 - 128.0);
    let b = y as f32 + 1.772 * (u as f32 - 128.0);
    (r as u8, g as u8, b as u8)
}
```

**문제**:
- BT.601 계수를 모든 콘텐츠에 하드코딩하면 BT.709(HD)나 BT.2020(HDR/UHD) 콘텐츠에서 색상이 눈에 띄게 왜곡된다(특히 빨강/파랑 채널).
- limited range(16-235/16-240)와 full range(0-255)를 구분하지 않으면 명암 대비가 잘못되어 검은색이 회색으로, 흰색이 눌린 회색으로 보인다.
- HDR 콘텐츠(PQ/HLG transfer function)를 SDR gamma로 그대로 표시하면 지나치게 어둡거나 채도가 낮게 보이며, 이는 "디코딩이 잘못됐다"는 잘못된 버그 리포트로 이어지기 쉽다.
- 비디오 분석 툴에서 색공간을 무시하는 것은 단순 표시 버그가 아니라 "무엇을 분석하고 있는지 자체가 틀렸다"는 근본적 문제다.

**발생 조건**:
- BT.709/BT.2020 콘텐츠를 BT.601 계수로 잘못 변환하는 모든 경우.
- full-range로 인코딩된 스트림(webcam, 일부 게임 캡처)을 limited range로 가정하고 처리하는 경우.
- HDR(PQ/HLG) 콘텐츠를 SDR 표시 파이프라인에 그대로 흘려보내는 경우.

**권장**:
```rust
struct ColorInfo {
    matrix: ColorMatrix,        // BT601 / BT709 / BT2020NCL ...
    range: ColorRange,          // Limited / Full
    transfer: TransferFunction, // BT709 / PQ / HLG / SRGB ...
    primaries: ColorPrimaries,  // BT709 / BT2020 / ...
}

fn yuv_to_rgb(y: u16, u: u16, v: u16, bit_depth: u8, info: &ColorInfo) -> (f32, f32, f32) {
    let (y_n, u_n, v_n) = normalize_with_range(y, u, v, bit_depth, info.range);
    let rgb_linear = apply_matrix(y_n, u_n, v_n, info.matrix);
    apply_transfer_and_gamut(rgb_linear, info.transfer, info.primaries)
}
```
- 색공간 정보(matrix/range/transfer/primaries)는 비트스트림의 VUI/color_config에서 파싱해 `DecodedFrame`에 함께 실어 전달한다.
- 변환 함수는 이 메타데이터를 필수 입력으로 받도록 시그니처를 강제한다(디폴트값을 암묵적으로 쓰지 않는다).
- HDR 콘텐츠는 명시적인 톤매핑 단계를 거쳐 SDR 디스플레이에 표시하고, 원본 PQ/HLG 값 자체는 분석용으로 보존한다.

**탐지 방법**:
- Static: 색공간 변환 계수가 하드코딩된 상수로 박혀 있는지 grep(`1.402`, `0.344` 등 BT.601/709 특유의 매직넘버).
- Semantic: `yuv_to_rgb`류 함수가 `ColorInfo`/`matrix`/`range` 파라미터를 받는지 시그니처 검사.
- Manual: BT.2020 + limited range 테스트 벡터를 알려진 기준 렌더러(ffmpeg 등) 출력과 픽셀 비교.

**예외**:
- 코덱이 색공간 정보를 아예 시그널링하지 않는 레거시 스트림에서는 코덱별 관례적 기본값(예: SD는 BT.601, HD는 BT.709)으로 폴백하는 것이 합리적이다 — 다만 이는 명시적 "기본값 추정" 로직으로 문서화해야 하며, 하드코딩과는 구분된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-009: 영상 texture와 overlay를 합성한 뒤 전송
**분류**: 렌더링 파이프라인 · **심각도**: High · **탐지**: Structural

> 이 항목과 PIXEL-010은 "영상 texture와 overlay 분리" 원칙의 두 측면이다: 프레임 texture는 프레임이 바뀔 때만 갱신하고, QP/MV/파티션 등 오버레이는 모드 전환이나 줌 변경 시에만 갱신하며, 선택 영역 아웃라인 같은 즉각 반응이 필요한 요소는 별도 레이어로 즉시 렌더링해야 한다. 이 분리를 어기면 렌더링 계층 전체가 서로의 갱신 빈도에 종속되어 불필요한 재계산이 연쇄적으로 발생한다.

**나쁜 예**:
```rust
fn render_frame_with_overlay(decoded: &DecodedFrame, overlay: &QpOverlay) -> RgbaBuffer {
    let mut rgba = yuv_to_rgba(&decoded.y, &decoded.u, &decoded.v, decoded.width, decoded.height);
    // overlay를 픽셀 버퍼에 직접 합성한 뒤 하나의 텍스처로 GPU에 전송
    composite_qp_overlay_onto(&mut rgba, overlay);
    rgba
}
```

**문제**:
- 영상 프레임(초당 30~60회 갱신 가능)과 오버레이(사용자가 QP 오버레이를 켜거나 줌을 바꿀 때만 갱신)는 갱신 빈도가 완전히 다른데, 이를 CPU에서 미리 합성해 하나의 텍스처로 만들면 두 갱신 빈도가 강제로 동기화된다.
- 오버레이 토글/색상 변경/투명도 조절처럼 영상 자체는 그대로인 상황에서도 매번 전체 프레임을 다시 YUV→RGBA 변환하고 재합성해야 한다(PIXEL-010과 직결).
- GPU에서 별도 레이어로 합성(alpha blending)하면 되는 작업을 CPU에서 미리 픽셀 단위로 수행하는 것은 GPU 활용을 포기하는 것과 같다.

**발생 조건**:
- 오버레이가 여러 종류(QP, MV, 파티션, 참조 인덱스)이고 사용자가 토글을 빈번히 전환하는 UI.
- 줌/팬 인터랙션 중 오버레이만 다시 그려도 되는데 영상까지 재변환되는 경우 프레임 드랍으로 체감된다.

**권장**:
```rust
// 영상 텍스처: 프레임이 바뀔 때만 갱신
struct VideoTexture { rgba: RgbaBuffer, frame_index: FrameIndex }

// 오버레이 텍스처: 모드/줌/토글이 바뀔 때만 갱신, 영상과 별개의 레이어
struct OverlayTexture { rgba: RgbaBuffer, overlay_kind: OverlayKind, zoom: f32 }

fn render(video: &VideoTexture, overlay: Option<&OverlayTexture>, gpu: &mut GpuCompositor) {
    gpu.draw_layer(&video.rgba);
    if let Some(ov) = overlay {
        gpu.blend_layer(&ov.rgba, ov.alpha); // GPU 알파 블렌딩으로 합성
    }
}
```
- 영상 프레임과 오버레이를 각각 독립된 텍스처/레이어로 유지하고, 합성은 GPU 컴포지터(알파 블렌딩)에 위임한다.
- 선택 영역 아웃라인처럼 즉시 반응해야 하는 UI 요소는 다시 별도의 최상위 레이어로 두어, 영상/오버레이 재계산 없이 즉시 그린다.

**탐지 방법**:
- Structural: 렌더 함수가 `decoded frame`과 `overlay data`를 동시에 인자로 받아 하나의 `RgbaBuffer`를 반환하는지(=CPU 합성 신호) 시그니처 검사.
- Runtime: 오버레이만 토글했을 때 YUV→RGBA 변환 함수가 다시 호출되는지 프로파일러/트레이싱으로 확인.

**예외**:
- GPU 레이어 합성이 불가능한 환경(순수 CPU 렌더링, 서버사이드 프레임 덤프 생성 등)에서는 CPU 합성이 유일한 선택지일 수 있다. 이 경우도 "영상 캐시 + 오버레이만 재합성"으로 최소한 재변환은 피해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-010: overlay 변경마다 영상 재변환
**분류**: 렌더링 파이프라인 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn on_overlay_toggle(app: &mut AppState, kind: OverlayKind, enabled: bool) {
    app.overlay_settings.set(kind, enabled);
    // 오버레이만 바뀌었는데 현재 프레임 전체를 처음부터 다시 디코드+변환
    let decoded = app.decoder.decode_frame(app.current_frame_index);
    app.display_texture = yuv_to_rgba(&decoded.y, &decoded.u, &decoded.v, decoded.width, decoded.height);
    composite_overlay(&mut app.display_texture, &app.overlay_settings);
}
```

**문제**:
- 오버레이 체크박스 하나를 켜고 끄는 사용자 액션이, 영상 프레임 재디코드와 색공간 재변환이라는 무거운 연산을 매번 트리거한다 — 응답성이 프레임 크기에 비례해 나빠진다.
- 이는 PIXEL-009(합성 후 전송)의 직접적 결과다: 영상과 오버레이가 하나의 버퍼로 묶여 있으니, 오버레이만 바뀌어도 전체를 다시 만들 수밖에 없다.
- 줌 레벨 변경처럼 오버레이 좌표만 재계산하면 되는 경우도 영상 텍스처까지 다시 그리게 되어 인터랙션이 끊기는(janky) 느낌을 준다.

**발생 조건**:
- 여러 오버레이 종류를 빠르게 전환하며 비교하는 워크플로(코덱 분석 툴의 핵심 사용 패턴).
- 4K 이상 고해상도에서 재변환 비용이 커 UI 프레임레이트 저하가 눈에 띄게 체감된다.

**권장**:
```rust
fn on_overlay_toggle(app: &mut AppState, kind: OverlayKind, enabled: bool) {
    app.overlay_settings.set(kind, enabled);
    // 영상 텍스처는 그대로 재사용, 오버레이 레이어만 재생성
    app.overlay_texture = build_overlay_texture(&app.current_frame_metadata, &app.overlay_settings);
    // GPU 컴포지터가 다음 프레임에서 video_texture + overlay_texture를 블렌딩
}
```
- "무엇이 바뀌었는가"에 따라 갱신 범위를 최소화하는 dirty-flag/invalidation 전략을 명시적으로 둔다: 프레임 인덱스 변경 → 영상 텍스처 갱신, 오버레이/줌 변경 → 오버레이 텍스처만 갱신.
- 영상 텍스처는 프레임이 바뀌지 않는 한 캐시에서 재사용한다.

**탐지 방법**:
- Runtime: 오버레이 토글 이벤트 핸들러를 트레이싱해 `decode_frame`/`yuv_to_rgba` 호출 여부를 확인.
- Structural: 오버레이 설정 변경 이벤트 핸들러의 호출 그래프에 디코드/색공간 변환 함수가 포함되는지 정적 분석.

**예외**:
- 오버레이가 픽셀 값 자체에 의존하는 경우(예: "오버레이 색상을 픽셀 밝기에 따라 다르게" 같은 데이터 종속 오버레이)는 영상 데이터 접근이 필요하지만, 이 경우도 재디코드가 아니라 이미 디코드되어 캐시된 YUV를 재사용하면 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-011: thumbnail 생성에 원본 크기 RGBA 사용
**분류**: 파이프라인 설계 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn generate_thumbnail(decoded: &DecodedFrame, thumb_size: (u32, u32)) -> RgbaBuffer {
    // 원본 해상도로 전체 RGBA 변환 후에 축소
    let full_rgba = yuv_to_rgba(&decoded.y, &decoded.u, &decoded.v, decoded.width, decoded.height);
    resize_rgba(&full_rgba, thumb_size)
}
```

**문제**:
- 4K 프레임에서 128x72 썸네일 하나를 만들기 위해 8,294,400픽셀 전체를 RGBA로 변환한 뒤 99% 이상을 버리는 것은 순수 낭비다.
- 타임라인에 수십~수백 개의 썸네일을 그려야 하는 경우(스크러버 바), 이 낭비가 프레임 수만큼 누적되어 UI 반응성을 심각하게 저해한다.
- 원본 RGBA 버퍼가 일시적으로라도 메모리에 존재해야 하므로 피크 메모리 사용량도 불필요하게 증가한다.

**발생 조건**:
- 타임라인 스크러버, 필름스트립(filmstrip) UI처럼 다수의 저해상도 미리보기가 필요한 화면.
- 4K/8K 원본 소스에서 특히 낭비 비율이 커진다(다운스케일 비율이 클수록 버려지는 연산이 많음).

**권장**:
```rust
fn generate_thumbnail(decoded: &DecodedFrame, thumb_size: (u32, u32)) -> RgbaBuffer {
    // YUV 단계에서 먼저 다운샘플링(박스 필터 등) 후 축소된 크기로만 RGBA 변환
    let small_y = downsample_plane(&decoded.y, decoded.width, decoded.height, thumb_size);
    let small_u = downsample_plane(&decoded.u, decoded.chroma_width, decoded.chroma_height, thumb_chroma_size(thumb_size));
    let small_v = downsample_plane(&decoded.v, decoded.chroma_width, decoded.chroma_height, thumb_chroma_size(thumb_size));
    yuv_to_rgba(&small_y, &small_u, &small_v, thumb_size.0, thumb_size.1)
}
```
- 다운샘플링을 YUV(원본 저정밀도 표현) 단계에서 먼저 수행하고, RGBA 변환은 최종 축소 크기에서만 한다 — 변환할 픽셀 수 자체를 줄인다.
- 디코더가 저해상도 디코드(예: dav1d의 scale-down 옵션)를 지원한다면, 애초에 낮은 해상도로 디코드하는 것이 더 효율적이다.

**탐지 방법**:
- Structural: 썸네일 생성 함수 내부에서 `yuv_to_rgba`가 원본 `decoded.width/height`로 호출되는지, 그 뒤에 `resize`가 오는지 순서 확인.
- Runtime: 프로파일러로 썸네일 생성 경로의 픽셀 처리량이 목표 썸네일 크기 대비 비정상적으로 큰지 측정.

**예외**:
- 썸네일 크기가 원본과 큰 차이가 없는 경우(예: 2x 축소)는 다운샘플링 단계 분리의 이득이 작으므로 단순 구현이 허용될 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-012: SIMD alignment를 고려하지 않음
**분류**: SIMD/성능 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn convert_row_simd(y_row: &[u8], out_row: &mut [u8]) {
    // 버퍼 정렬을 확인하지 않고 무조건 정렬 load 명령을 사용
    unsafe {
        let ptr = y_row.as_ptr() as *const __m256i;
        let v = _mm256_load_si256(ptr); // 32바이트 정렬이 아니면 SIGSEGV
        // ...
    }
}
```

**문제**:
- 정렬 로드(`_mm256_load_si256` 등)는 입력 포인터가 요구 정렬(예: AVX2는 32바이트)을 만족하지 않으면 크래시(SIGSEGV/SIGBUS)를 일으킨다.
- 디코더가 반환하는 plane 버퍼나 `Vec<u8>` 기본 할당은 SIMD 정렬을 보장하지 않는 경우가 많아, 특정 프레임 크기/오프셋에서만 죽는 재현이 극히 까다로운 크래시로 나타난다.
- FFI 경계를 넘어온 버퍼(dav1d의 `Dav1dPicture` 등)는 디코더 내부의 정렬 정책을 따르며, 이를 호출부에서 임의로 가정하는 것은 위험하다.

**발생 조건**:
- 크롭된 영역, stride 오프셋이 정렬 경계와 어긋나는 특정 (width, height, crop) 조합.
- `Vec<u8>`을 기본 할당으로 만들고 SIMD 커널에 바로 넘기는 코드(기본 할당은 정렬을 보장하지 않음, 특히 offset이 더해진 슬라이스).

**권장**:
```rust
fn convert_row_simd(y_row: &[u8], out_row: &mut [u8]) {
    unsafe {
        let ptr = y_row.as_ptr();
        if ptr.align_offset(32) == 0 && y_row.len() % 32 == 0 {
            convert_row_avx2_aligned(y_row, out_row);
        } else {
            convert_row_avx2_unaligned(y_row, out_row); // _mm256_loadu_si256 사용
        }
    }
}
```
- 정렬을 보장할 수 없는 입력에는 항상 unaligned load(`loadu`)를 사용하는 것을 기본값으로 하고, 정렬이 보장된 경로에서만 aligned load로 최적화한다.
- 자체 버퍼를 할당할 때는 `#[repr(align(32))]` 또는 정렬 할당자를 사용해 애초에 정렬을 보장한다.
- 정렬 여부를 런타임에 확인하는 헬퍼(`ptr.align_offset`)를 SIMD 진입점에 항상 둔다.

**탐지 방법**:
- Runtime: ASan/UBSan 또는 실제 크래시 리포트에서 SIMD 로드 명령 근처의 SIGSEGV 패턴 확인.
- Static: `_mm256_load_si256`/`_mm_load_si128` 등 정렬 요구 intrinsic 호출부에 정렬 검증 코드가 선행하는지 검사.
- Manual: 다양한 (width, crop offset) 조합에 대해 fuzz 테스트를 수행해 정렬 크래시를 사전에 유발.

**예외**:
- 버퍼를 직접 정렬 할당자로 생성하고 오프셋 연산이 전혀 없는 내부 스크래치 버퍼라면 aligned load를 안전하게 상수처럼 사용할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-013: scalar fallback과 SIMD 결과 불일치
**분류**: SIMD/성능 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn yuv_to_rgb_scalar(y: i32, u: i32, v: i32) -> (i32, i32, i32) {
    // 정수 연산, 반올림 방식: 절삭(truncate)
    let r = y + (91881 * v >> 16);
    (r, 0, 0)
}

#[target_feature(enable = "avx2")]
unsafe fn yuv_to_rgb_avx2(y: __m256i, u: __m256i, v: __m256i) -> __m256i {
    // 부동소수점 근사 계수 사용, 반올림 방식: round-to-nearest
    // scalar와 계수/반올림 방식이 미묘하게 달라 결과가 ±1~2 LSB 어긋남
    todo!()
}
```

**문제**:
- SIMD 경로와 scalar fallback이 서로 다른 계수 근사치나 반올림 규칙을 쓰면, CPU에 따라(AVX2 지원 여부) 같은 입력에 대해 다른 출력이 나온다 — "내 컴퓨터에선 재현 안 되는" 버그의 전형적 원인이다.
- 품질 지표(PSNR/SSIM) 계산에 이 변환이 관여하면, 순수히 SIMD 유무 때문에 측정값이 달라지는 비결정성이 생겨 회귀 테스트의 신뢰도가 무너진다.
- golden/reference 이미지 비교 테스트가 특정 아키텍처에서만 실패하는 상황을 유발해 CI 디버깅 비용이 커진다.

**발생 조건**:
- AVX2/NEON 지원 CPU와 미지원 CPU(또는 CI 러너와 개발자 로컬 머신) 간에 동일 입력, 다른 출력이 나오는 모든 경우.
- 픽셀 단위 정확도가 중요한 골든 이미지 회귀 테스트, PSNR 계산 등에서 미세한 값 차이가 임계값을 넘나드는 경우.

**권장**:
```rust
// 공통 상수/반올림 규칙을 단일 소스로 정의하고 scalar/SIMD 양쪽에서 재사용
const YUV_TO_RGB_V_COEFF: i32 = 91881; // Q16 고정소수점, 두 경로가 동일 상수 참조

fn yuv_to_rgb_scalar(y: i32, u: i32, v: i32) -> (i32, i32, i32) {
    let r = y + ((YUV_TO_RGB_V_COEFF * v + (1 << 15)) >> 16); // round-to-nearest로 통일
    (r, 0, 0)
}

// AVX2 구현도 동일한 Q16 고정소수점 상수와 반올림(+1<<15 후 shift)을 사용하도록 맞춘다.
```
- SIMD와 scalar 경로가 "같은 정수 고정소수점 상수, 같은 반올림 규칙"을 공유하도록 상수를 단일 정의로 통일한다.
- CI에 SIMD 활성/비활성 두 빌드를 모두 돌려 동일 입력에 대한 출력 바이트가 정확히 일치하는지(bit-exact) 검증하는 테스트를 추가한다.

**탐지 방법**:
- Semantic: scalar와 SIMD 구현이 서로 다른 상수 테이블/부동소수점-정수 방식을 쓰는지 코드 비교.
- Runtime: `RUSTFLAGS="-C target-feature=-avx2"` 등으로 SIMD를 강제 비활성화한 빌드와 활성화 빌드의 출력을 동일 입력에 대해 diff.
- CI: bit-exact 비교를 회귀 테스트에 포함(허용 오차 없이 정확히 동일해야 함을 명시).

**예외**:
- 애초에 근사 연산(예: fast-path 미리보기용 저정밀 변환)임을 인지하고 허용 오차 범위를 명시적으로 문서화한 경우는 완전 일치를 요구하지 않아도 된다 — 단, 품질 지표 계산 경로에는 이 예외를 적용하면 안 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-014: frame plane을 Vec<Vec<u16>>로 저장
**분류**: 메모리 레이아웃 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct DecodedFrame {
    // 행마다 별도 힙 할당 — 캐시 지역성 파괴, 포인터 추적 오버헤드
    y_plane: Vec<Vec<u16>>,
    u_plane: Vec<Vec<u16>>,
    v_plane: Vec<Vec<u16>>,
}
```

**문제**:
- `Vec<Vec<u16>>`는 각 행이 독립된 힙 할당이라 메모리상에서 연속적이지 않다 — SIMD 벡터화, `memcpy` 기반 최적화, prefetch가 모두 불가능해진다.
- 프레임당 `height`번의 할당이 발생하므로(예: 2160행이면 2160번 할당), 할당자 오버헤드가 plane 크기에 비례해 커진다.
- 외부 라이브러리(FFI, image 크레이트, GPU 업로드 API)는 대부분 연속된 단일 버퍼 + stride를 기대하므로, 이 구조는 매 사용 시점마다 flatten 변환이 추가로 필요하다.

**발생 조건**:
- 프레임 크기가 클수록(4K/8K), 행 수가 많을수록 할당 오버헤드와 캐시 미스 비용이 커진다.
- SIMD 커널이나 GPU 업로드처럼 연속 메모리를 요구하는 모든 소비자와 결합할 때 특히 문제.

**권장**:
```rust
struct Plane {
    data: Vec<u16>,   // 단일 연속 버퍼
    stride: usize,    // 행 간 간격(요소 단위, 패딩 포함 가능)
    width: usize,
    height: usize,
}

impl Plane {
    fn row(&self, y: usize) -> &[u16] {
        let start = y * self.stride;
        &self.data[start..start + self.width]
    }
}

struct DecodedFrame {
    y_plane: Plane,
    u_plane: Plane,
    v_plane: Plane,
}
```
- plane은 항상 단일 `Vec<T>`(또는 정렬 할당 버퍼) + `stride`로 표현하고, 행 접근은 인덱스 계산으로 슬라이스를 얻는 헬퍼로 캡슐화한다(PIXEL-004의 stride 처리와 자연히 결합된다).
- FFI로부터 받은 버퍼는 가능하면 복사 없이 이 구조로 wrap(`&[u16]` 슬라이스 참조)한다.

**탐지 방법**:
- Structural: `Vec<Vec<_>>` 또는 `Vec<Box<[_]>>` 형태로 plane을 표현하는 구조체 정의를 grep.
- Runtime: 프레임 디코드 경로에서 힙 할당 횟수를 카운트해 `height`에 비례하는 할당이 발생하는지 확인(`dhat`/`heaptrack`).

**예외**:
- 프레임이 극도로 작거나(예: 아이콘 크기 분석), 성능이 전혀 중요하지 않은 프로토타입/테스트 코드에서는 단순함을 위해 허용될 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-015: crop과 coded size 혼동
**분류**: 기하 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn get_display_dimensions(decoded: &DecodedFrame) -> (u32, u32) {
    // coded size(매크로블록/CTU 정렬 크기)를 그대로 표시 크기로 사용
    (decoded.coded_width, decoded.coded_height)
}
```

**문제**:
- 비디오 코덱은 내부적으로 매크로블록(16px)이나 CTU(최대 64px) 단위로 정렬된 "coded size"로 인코딩하고, 실제 표시해야 할 "visible/display size"는 이보다 작거나 다를 수 있다(예: 1920x1080 영상이 1920x1088 coded size로 인코딩되는 경우가 흔하다).
- coded size를 그대로 표시하면 하단(또는 우측)에 인코더가 패딩으로 채운 쓰레기 픽셀 줄이 보이는 뚜렷한 시각적 결함이 생긴다.
- crop 정보를 무시하면 이후의 모든 좌표 기반 로직(오버레이 좌표, 클릭 좌표→픽셀 매핑, ROI 선택)이 동일한 오프셋만큼 어긋난다 — 단순 표시 버그를 넘어 좌표계 전체가 오염된다.

**발생 조건**:
- 폭/높이가 정렬 단위(HEVC는 CTU, AVC는 매크로블록)의 배수가 아닌 모든 해상도 — 실무 영상 대부분.
- crop 정보가 SPS/시퀀스 헤더의 `conformance_window`(HEVC), `frame_cropping`(AVC) 등에 별도로 시그널링되는 코덱.

**권장**:
```rust
struct FrameGeometry {
    coded_width: u32,
    coded_height: u32,
    crop_left: u32,
    crop_right: u32,
    crop_top: u32,
    crop_bottom: u32,
}

impl FrameGeometry {
    fn display_size(&self) -> (u32, u32) {
        (
            self.coded_width - self.crop_left - self.crop_right,
            self.coded_height - self.crop_top - self.crop_bottom,
        )
    }
}

fn get_display_dimensions(decoded: &DecodedFrame) -> (u32, u32) {
    decoded.geometry.display_size() // 항상 crop이 반영된 크기를 사용
}
```
- `coded_size`와 `display_size`(= visible rect)를 타입 레벨에서부터 구분하고, 화면 표시/좌표 매핑/ROI 계산은 반드시 `display_size` 기반으로만 수행한다.
- crop 오프셋(`crop_left`, `crop_top`)을 plane 접근 시 시작 오프셋에 반영해, "visible 영역의 (0,0)"이 실제 좌표계의 원점이 되도록 한다.

**탐지 방법**:
- Semantic: 표시/좌표 변환 함수가 `coded_width`/`coded_height`를 직접 참조하는지, `crop_*` 필드를 전혀 참조하지 않는지 검사.
- Manual: crop이 있는 실제 스트림(coded size ≠ display size)으로 렌더링해 우측/하단에 패딩 아티팩트가 보이는지 육안 확인.
- Runtime: 클릭 좌표→픽셀 좌표 매핑 테스트에서 프레임 경계 근처 좌표가 어긋나는지 회귀 테스트로 검증.

**예외**:
- crop이 0인(coded size == display size) 스트림에서는 두 값이 우연히 같으므로 버그가 드러나지 않는다 — 이는 예외가 아니라 "숨은 버그가 아직 발현되지 않은 상태"임에 유의해야 한다.

**관련**: `UIX_VIZ.md` UIX-VIZ-006 참고 — overlay 좌표가 실제 프레임 지오메트리와 어긋나는 문제이나, 이쪽은 crop/coded-size 오프셋, UIX-VIZ-006은 heatmap grid/CU 정합이 원인.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-016: visible rect 밖까지 metric 계산
**분류**: 메트릭 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn compute_psnr(decoded: &DecodedFrame, reference: &DecodedFrame) -> f64 {
    // coded size 전체(패딩 포함)로 PSNR을 계산 — crop 밖 패딩 픽셀까지 포함
    let mut sum_sq_err = 0.0f64;
    let n = (decoded.coded_width * decoded.coded_height) as f64;
    for i in 0..decoded.coded_width * decoded.coded_height {
        let diff = decoded.y_plane.data[i as usize] as f64 - reference.y_plane.data[i as usize] as f64;
        sum_sq_err += diff * diff;
    }
    10.0 * (255.0 * 255.0 / (sum_sq_err / n)).log10()
}
```

**문제**:
- PIXEL-015와 연결되는 문제로, coded size 전체로 PSNR/SSIM을 계산하면 사용자가 실제로 보지 않는 패딩 영역의 오차까지 지표에 섞인다.
- 패딩 영역은 인코더 구현에 따라 임의의 방식(가장자리 픽셀 복제, 0으로 채움 등)으로 채워지므로, 이 영역의 오차는 실제 화질과 무관한 노이즈다 — 인코더가 바뀌면 지표가 흔들리는 원인이 된다.
- 두 인코더/두 코덱을 비교하는 벤치마크에서 패딩 처리 방식 차이가 지표 차이로 오인될 수 있어, 비교의 공정성이 깨진다.

**발생 조건**:
- crop이 있는(coded size ≠ display size) 모든 콘텐츠에서 PSNR/SSIM/VMAF류 지표를 coded size 기준으로 계산하는 경우.
- 서로 다른 인코더로 인코딩된 스트림을 비교할 때 패딩 정책 차이가 특히 두드러진다.

**권장**:
```rust
fn compute_psnr(decoded: &DecodedFrame, reference: &DecodedFrame) -> f64 {
    let (w, h) = decoded.geometry.display_size();
    let (crop_left, crop_top) = (decoded.geometry.crop_left, decoded.geometry.crop_top);

    let mut sum_sq_err = 0.0f64;
    for y in 0..h {
        for x in 0..w {
            let idx = ((y + crop_top) * decoded.y_plane.stride as u32 + (x + crop_left)) as usize;
            let diff = decoded.y_plane.data[idx] as f64 - reference.y_plane.data[idx] as f64;
            sum_sq_err += diff * diff;
        }
    }
    let n = (w * h) as f64;
    10.0 * (255.0 * 255.0 / (sum_sq_err / n)).log10()
}
```
- 모든 품질 지표 계산은 `display_size`(visible rect) 범위로만 순회하도록 명시적으로 경계를 제한한다.
- 벤치마크/비교 리포트에는 사용된 영역(coded vs display)을 메타데이터로 함께 기록해 재현성을 보장한다.

**탐지 방법**:
- Semantic: 지표 계산 루프의 순회 범위가 `coded_width/height`인지 `display_size()`인지 확인.
- Runtime: crop이 있는 테스트 벡터에서 coded 기준 계산과 display 기준 계산 결과를 비교해 차이가 존재하는지(존재해야 정상) 검증하는 회귀 테스트.

**예외**:
- 인코더 패딩 정책 자체를 분석/검증하는 것이 목적인 도구(예: 패딩 아티팩트 탐지기)라면 의도적으로 coded size 전체를 다뤄야 하며, 이 경우는 예외로 명확히 문서화한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-017: PSNR 계산 전에 불필요한 format conversion
**분류**: 메트릭 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn compute_psnr_between(a: &DecodedFrame, b: &DecodedFrame) -> f64 {
    // YUV 원본이 있음에도 RGBA로 변환한 뒤 RGB 채널 기준으로 PSNR 계산
    let rgba_a = yuv_to_rgba(&a.y_plane, &a.u_plane, &a.v_plane, a.width, a.height);
    let rgba_b = yuv_to_rgba(&b.y_plane, &b.u_plane, &b.v_plane, b.width, b.height);
    psnr_rgba(&rgba_a, &rgba_b)
}
```

**문제**:
- PSNR/SSIM은 표준적으로 Y(휘도) 평면 또는 YUV 각 평면에서 직접 계산하는 지표인데, 이를 위해 RGBA 왕복 변환을 거치는 것은 불필요한 연산일 뿐 아니라 색공간 변환 과정에서 반올림 오차가 섞여 미세하게 다른 값을 만든다(PIXEL-001과 같은 근본 원인의 다른 발현).
- RGBA는 4채널(알파 포함)인데 PSNR 계산에 알파 채널까지 포함시키면(또는 항상 255로 무의미한 채널을 계산하면) 결과가 표준 정의(Y-PSNR)와 어긋나 다른 도구(ffmpeg, VMAF 등)와 비교 불가능한 수치가 나온다.
- 변환 비용 자체도 상당하다: PSNR 계산 하나를 위해 두 프레임을 각각 RGBA로 완전히 변환하는 것은 계산량을 몇 배로 늘린다.

**발생 조건**:
- 인코더 비교 벤치마크처럼 다수의 프레임 쌍에 대해 반복적으로 PSNR을 계산하는 워크로드에서 누적 비용이 크다.
- 표준 도구(ffmpeg의 `psnr` 필터 등)와 수치를 대조할 때, RGBA 경유 계산은 근본적으로 다른 정의를 계산하고 있어 비교 자체가 무의미해진다.

**권장**:
```rust
fn compute_psnr_between(a: &DecodedFrame, b: &DecodedFrame) -> PsnrResult {
    PsnrResult {
        y: psnr_plane(&a.y_plane, &b.y_plane, a.bit_depth),
        u: psnr_plane(&a.u_plane, &b.u_plane, a.bit_depth),
        v: psnr_plane(&a.v_plane, &b.v_plane, a.bit_depth),
    }
}

fn psnr_plane(a: &Plane, b: &Plane, bit_depth: u8) -> f64 {
    // display_size 범위에서 stride를 고려해 직접 순회 (PIXEL-004, PIXEL-016 참고)
    let max_val = ((1u32 << bit_depth) - 1) as f64;
    let mse = mean_squared_error(a, b);
    10.0 * (max_val * max_val / mse).log10()
}
```
- 품질 지표는 원본 표현(YUV plane, 원본 bit depth)에서 직접 계산하고, RGBA 변환은 이 계산 경로에서 완전히 배제한다.
- 표준 도구와 대조 가능하도록 Y/U/V 평면별 PSNR을 표준 정의(peak = `2^bit_depth - 1`)로 계산하고, 필요시 가중 평균(예: `(6*Y + U + V) / 8`) 같은 관례도 명시적으로 문서화한다.

**탐지 방법**:
- Structural: PSNR/SSIM 계산 함수 호출 그래프에 `yuv_to_rgba`가 선행되는지 확인.
- Runtime: PSNR 계산 함수를 벤치마크해 RGBA 변환 비용이 전체 시간의 유의미한 비율을 차지하는지 프로파일링.
- Manual: 동일 테스트 벡터에 대해 ffmpeg `psnr` 필터 결과와 비교해 수치가 합리적 오차 범위 내에서 일치하는지 검증.

**예외**:
- 지표 자체가 "화면에 실제로 표시되는 RGB 색상 차이"를 의도적으로 측정하려는 지각 기반 지표(예: RGB/Lab 공간에서의 색차 ΔE)라면 RGB 변환이 계산 정의의 일부이므로 예외다. 이 경우도 매 프레임 재변환이 아니라 필요한 만큼만 변환해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-018: metrics 병렬화로 memory bandwidth 포화
**분류**: 성능/동시성 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn compute_all_metrics(frames: &[DecodedFrame], references: &[DecodedFrame]) -> Vec<MetricResult> {
    use rayon::prelude::*;
    // 코어 수만큼 무제한 병렬화 — 각 스레드가 거대한 4K YUV 버퍼 두 개씩을 동시에 순회
    frames.par_iter().zip(references.par_iter())
        .map(|(f, r)| compute_full_metrics(f, r)) // PSNR + SSIM + VMAF 등 여러 지표를 한 번에
        .collect()
}
```

**문제**:
- PSNR/SSIM 계산은 연산 강도(compute-per-byte)가 낮고 메모리 접근 강도가 높은 memory-bound 작업이다. 코어 수만큼 무한정 병렬화하면 각 스레드가 서로의 메모리 대역폭을 잠식해 스레드 수를 늘려도 처리량이 늘지 않거나 오히려 캐시 스래싱으로 줄어드는 지점(diminishing/negative returns)에 빠르게 도달한다.
- 4K/8K YUV 버퍼(프레임당 수십 MB)를 다수 스레드가 동시에 순회하면 L3 캐시를 그 프레임들이 통째로 두고 경쟁하게 되어, 각 스레드의 유효 캐시 히트율이 급락한다.
- 여러 지표(PSNR, SSIM, VMAF)를 각각 별도로 전체 순회하면 같은 데이터를 캐시에서 몇 번이고 다시 읽어들이는 것도 대역폭 낭비를 가중시킨다.

**발생 조건**:
- 코어 수가 많은 머신(16코어 이상)에서 대형 프레임(4K/8K)을 대상으로 다중 지표를 한꺼번에 계산하는 배치 작업.
- NUMA 시스템에서는 스레드가 원격 메모리 노드의 버퍼에 접근하며 대역폭 문제가 더 심각해질 수 있다.

**권장**:
```rust
fn compute_all_metrics(frames: &[DecodedFrame], references: &[DecodedFrame]) -> Vec<MetricResult> {
    use rayon::prelude::*;
    // 병렬도를 물리 코어의 일부로 제한하고, 지표들을 한 번의 순회에서 함께 계산(fused pass)
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(measured_optimal_parallelism()) // 벤치마크로 산정한 값, 보통 물리 코어 수의 절반 내외
        .build()
        .unwrap();

    pool.install(|| {
        frames.par_iter().zip(references.par_iter())
            .map(|(f, r)| compute_fused_metrics(f, r)) // 한 번의 plane 순회에서 PSNR+SSIM 동시 누적
            .collect()
    })
}
```
- 병렬도를 논리 코어 수가 아니라 실측 벤치마크(스레드 수 대비 처리량 그래프)로 산정한 "메모리 대역폭이 포화되지 않는 지점"으로 제한한다.
- 여러 지표를 각각 별도 순회로 계산하지 않고, 가능하면 한 번의 plane 순회에서 여러 누적값(SSE, SSIM 윈도우 통계 등)을 동시에 계산하는 fused kernel로 캐시 재사용을 극대화한다.
- 프레임 단위 병렬화와 픽셀 단위 병렬화(SIMD) 중 하나를 우선하고 과도하게 중첩시키지 않는다.

**탐지 방법**:
- Runtime: 스레드 수를 늘려가며 처리량을 측정해 특정 지점 이후 처리량이 정체/역전되는지 확인(스케일링 벤치마크).
- Runtime: `perf stat`으로 메모리 대역폭 관련 카운터(예: LLC miss rate)가 병렬도 증가에 따라 포화되는지 관찰.

**예외**:
- 프레임 수가 적고(예: 단일 프레임 diff) 코어 수도 적은 워크로드에서는 병렬화 오버튜닝의 실익이 적어 단순 `par_iter()` 사용이 실용적일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-019: decoder-owned buffer lifetime 무시
**분류**: FFI/생명주기 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
fn get_current_frame_y_plane(decoder: &mut Dav1dDecoder) -> &[u8] {
    let picture = decoder.get_picture(); // dav1d가 소유한 버퍼를 가리키는 참조
    let slice = picture.plane_data(0);
    decoder.release_picture(picture); // 여기서 dav1d 내부적으로 버퍼가 재사용/해제될 수 있음
    slice // 이미 해제되었을 수도 있는 메모리를 가리키는 슬라이스를 반환
}
```

**문제**:
- dav1d 같은 디코더가 반환하는 picture 버퍼는 디코더의 내부 버퍼 풀에 속하며, `release`(또는 다음 프레임 디코드)가 호출되는 순간 그 메모리가 재사용되거나 해제될 수 있다.
- Rust의 라이프타임 시스템은 FFI 경계 너머의 C 라이브러리가 언제 버퍼를 재사용하는지 알지 못하므로, `unsafe` FFI 바인딩에서 라이프타임을 잘못 부여하면 컴파일러가 이 use-after-free를 잡아내지 못한다.
- 이 버그는 대개 즉시 크래시하지 않고 "다음 프레임의 데이터가 섞여 보인다"거나 "가끔 노이즈가 낀다"처럼 비결정적이고 미묘한 증상으로 나타나 원인 추적이 매우 어렵다.

**발생 조건**:
- picture/frame 참조를 유지한 채로 디코더의 다음 동작(다음 프레임 디코드, 명시적 release/unref)을 호출하는 모든 경로.
- 멀티스레드 환경에서 디코드 스레드와 소비(분석/렌더) 스레드가 분리되어 있어 버퍼 소유권 이전 시점이 명확하지 않은 구조.

**권장**:
```rust
struct OwnedFrame {
    // dav1d picture를 Drop에서 명시적으로 unref하는 RAII 래퍼가 소유권을 관리
    _picture_guard: Dav1dPictureGuard,
    y_data: *const u8,
    y_len: usize,
}

impl OwnedFrame {
    fn y_plane(&self) -> &[u8] {
        // _picture_guard가 살아있는 동안에만 이 슬라이스가 유효함을 타입으로 강제
        unsafe { std::slice::from_raw_parts(self.y_data, self.y_len) }
    }
}
// OwnedFrame이 스코프를 벗어나면 Drop이 dav1d_picture_unref를 호출해 소유권을 명확히 반환
```
- FFI에서 받은 프레임 버퍼는 RAII 가드(디코더의 unref/release를 `Drop`에서 호출)로 감싸, "가드가 살아있는 동안만 슬라이스가 유효하다"는 규칙을 Rust 타입 시스템으로 표현한다.
- 버퍼를 다른 스레드나 더 긴 생명주기로 넘겨야 한다면 PIXEL-020처럼 명시적으로 deep copy하거나, `Arc`로 참조 카운트를 공유해 release 시점을 늦춘다.
- 절대 `&[u8]`/`*const u8`을 가드 없이 원시 형태로 반환하지 않는다.

**탐지 방법**:
- Runtime: ASan/Miri로 FFI 버퍼 접근 경로에서 use-after-free를 탐지(가능하다면 FFI 부분을 안전한 래퍼로 감싼 뒤 Miri 적용).
- Structural: `unsafe` 블록에서 디코더 API가 반환한 포인터/슬라이스가 RAII 가드 없이 함수 반환값으로 그대로 노출되는지 검사.
- Manual: 디코드→릴리즈→사용 순서를 강제로 뒤바꾸는 스트레스 테스트(빠른 seek, 프레임 스킵)로 재현 시도.

**예외**:
- 없음에 가깝다 — FFI 버퍼 생명주기 관리는 항상 명시적이어야 하며, "보통은 안전하니 괜찮다"는 가정은 이 카테고리에서 특히 위험하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### PIXEL-020: FFI frame을 즉시 deep copy
**분류**: FFI/생명주기 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn get_current_frame(decoder: &mut Dav1dDecoder) -> DecodedFrame {
    let picture = decoder.get_picture();
    // "안전하게" 만들기 위해 매번 무조건 전체 plane을 deep copy
    let y_plane = picture.plane_data(0).to_vec();
    let u_plane = picture.plane_data(1).to_vec();
    let v_plane = picture.plane_data(2).to_vec();
    decoder.release_picture(picture);
    DecodedFrame { y_plane, u_plane, v_plane, /* ... */ }
}
```

**문제**:
- PIXEL-019의 use-after-free를 피하려는 의도는 옳지만, "모든 프레임을 무조건 즉시 deep copy"하는 것은 과잉 대응이다 — 4K 프레임 기준 프레임당 수십 MB 복사가 매 프레임(초당 수십 회) 발생해 메모리 대역폭과 할당 비용을 크게 낭비한다.
- 실제로는 짧은 스코프 내에서만 버퍼를 사용하는 경우(예: 이번 프레임을 즉시 렌더링하고 버리는 경우)라면 RAII 가드(PIXEL-019의 권장안)만으로 충분히 안전하며 복사가 전혀 필요 없다.
- "안전을 위해 항상 복사"라는 규칙이 코드베이스 전반에 퍼지면, 정말 복사가 필요한 지점(장기 캐시, 스레드 간 전달)과 불필요한 지점을 구분하지 못하게 되어 전체 파이프라인이 구조적으로 느려진다.

**발생 조건**:
- 매 프레임 반복 호출되는 hot path(재생 루프)에서 습관적으로 deep copy를 적용하는 경우 특히 손실이 크다.
- 4K/8K처럼 프레임 크기가 큰 콘텐츠일수록 불필요한 복사의 절대 비용이 커진다.

**권장**:
```rust
enum FrameRef<'a> {
    Borrowed(BorrowedFrame<'a>), // RAII 가드 기반, 짧은 스코프 내 사용(예: 즉시 렌더링)
    Owned(DecodedFrame),         // deep copy 완료, 캐시/스레드 전달 등 긴 생명주기용
}

fn get_current_frame_for_render(decoder: &mut Dav1dDecoder) -> BorrowedFrame<'_> {
    decoder.get_picture_guarded() // 복사 없이 가드만 반환, 즉시 사용 후 스코프 종료
}

fn get_current_frame_for_cache(decoder: &mut Dav1dDecoder) -> DecodedFrame {
    let borrowed = decoder.get_picture_guarded();
    borrowed.to_owned() // 캐시에 넣어야 하므로 여기서만 명시적으로 deep copy
}
```
- "짧은 스코프 내 즉시 소비"와 "장기 보관/스레드 간 전달"을 구분하고, 전자는 RAII 가드(PIXEL-019)로, 후자만 명시적 deep copy로 처리한다.
- 복사가 필요한 지점에서는 `to_owned()`/`into_owned()`처럼 의도가 드러나는 이름으로 호출부에서 "지금 복사가 발생한다"는 것을 명확히 표현한다.

**탐지 방법**:
- Runtime: 재생 루프(hot path)에서 프레임당 `to_vec()`/`clone()` 호출과 그로 인한 할당량을 프로파일링.
- Structural: `get_picture` 직후 무조건 `.to_vec()`이 뒤따르는 패턴이 hot path 전역에 퍼져 있는지 검색.

**예외**:
- 백그라운드 분석 워커로 프레임을 넘겨야 하거나, 여러 프레임을 동시에 비교해야 하는(PSNR 등) 경우처럼 디코더의 버퍼 재사용 시점보다 오래 데이터를 들고 있어야 한다면 deep copy가 정당하고 필요하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
