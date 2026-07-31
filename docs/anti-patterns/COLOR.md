# Anti-Pattern Catalog — COLOR: 색공간과 Bit Depth (VQ-Probe domain)

이 문서는 더 큰 안티패턴 카탈로그의 일부입니다(전체 목록은 별도로 작성 중인 `docs/anti-patterns/INDEX.md` 참고). COLOR는 VQ-Probe(듀얼 스트림 품질 비교) 도메인 절반(ALIGN/SPATIAL/METRIC/PIPE/HEAT/STAT와 함께)에 속하는 카테고리이며, Bitvue의 bitstream-analyzer 도메인과는 media-core 계층만 공유하는 별개 아키텍처다. Bitstream-analyzer 쪽에서 색공간 메타데이터는 §3.3에 정의된 대로 화면에 *표시*하는 대상일 뿐이지만, 여기 VQ-Probe에서는 색공간/bit-depth 처리가 잘못되면 *측정값 자체*가 바뀐다 — HDR PQ/BT.2020 레퍼런스와 SDR BT.709 인코드를 색공간을 무시한 채 plane 값으로 단순 차감하면, "품질 점수처럼 보이는 숫자"가 나오지만 실제로는 인코딩 손실이 아니라 색공간 불일치를 측정한 것이다. 이 카테고리는 그런 종류의, 숫자는 나오지만 의미가 없는(또는 의미가 반대인) 실패를 다룬다.

---

### COLOR-001: color primaries 무시
**분류**: 색공간 메타데이터 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_psnr(reference: &Yuv420Frame, distorted: &Yuv420Frame) -> f64 {
    // 두 스트림의 color_primaries(BT.709 / BT.2020 / DCI-P3 등)를 전혀 조회하지 않음
    let mse = mean_squared_error(&reference.y, &distorted.y);
    psnr_from_mse(mse, 255.0)
}

fn compare_streams(reference: &Stream, distorted: &Stream) -> QualityReport {
    // reference가 BT.2020(넓은 색역), distorted가 BT.709로 트랜스코딩된 스트림이어도
    // 동일한 YCbCr 좌표계라고 가정하고 그대로 plane 값을 diff한다
    let frame_scores: Vec<f64> = reference.frames.iter()
        .zip(distorted.frames.iter())
        .map(|(r, d)| compute_psnr(r, d))
        .collect();
    QualityReport::from_scores(frame_scores)
}
```

**문제**:
- 동일한 코드값(예: Y=200, Cb=140, Cr=110)이라도 primaries가 다르면 실제로 가리키는 색이 다르다. BT.709 색역의 초록과 BT.2020 색역의 초록은 같은 코드값에서도 물리적으로 다른 파장 분포를 의미한다.
- primaries가 다른 두 스트림을 plane 값으로 직접 비교하면, 인코더가 만들어낸 실제 손실과 "색역이 다른 두 그림을 겹쳐 뺀" 오차가 뒤섞여 분리 불가능한 숫자가 나온다.
- 특히 레퍼런스는 원본 마스터(BT.2020 wide gamut), 테스트 대상은 배포용으로 BT.709로 다운컨버전된 인코드인 워크플로에서, "인코더가 나쁘다"는 결론이 실제로는 "의도된 색역 변환"을 측정한 것일 수 있다.

**발생 조건**:
- 레퍼런스와 테스트 스트림이 서로 다른 마스터링 파이프라인(예: 극장용 DCI-P3 마스터 vs 방송용 BT.709 마스터)에서 파생된 경우.
- HDR 레퍼런스(BT.2020) 대 SDR 트랜스코드(BT.709) 비교처럼, 해상도/비트레이트 실험이 색역 변환과 동시에 일어나는 A/B 테스트.
- CICP(coding-independent code points) 파싱은 되어 있지만 비교 로직이 그 필드를 아예 읽지 않는 경우.

**권장**:
```rust
struct ColorMetadata {
    primaries: ColorPrimaries,
    transfer: TransferCharacteristics,
    matrix: MatrixCoefficients,
    range: ColorRange,
}

fn compare_streams(reference: &Stream, distorted: &Stream) -> Result<QualityReport, ColorMismatch> {
    if reference.color.primaries != distorted.color.primaries {
        // 묵인하지 않는다: 색역이 다르면 공통 색역(예: CIE XYZ)으로 정합 후 비교하거나
        // 명시적으로 "색역 불일치" 플래그를 결과에 남긴다
        return Err(ColorMismatch::PrimariesMismatch {
            reference: reference.color.primaries,
            distorted: distorted.color.primaries,
        });
    }
    let frame_scores = compute_scores_in_common_gamut(reference, distorted)?;
    Ok(QualityReport::from_scores(frame_scores))
}
```
- 비교 전에 두 스트림의 `color_primaries`를 반드시 조회하고, 다를 경우 자동으로 무시하지 않는다 — 최소한 리포트에 경고를, 이상적으로는 공통 색공간(CIE XYZ/Lab 등)으로 gamut mapping 후 비교한다.
- gamut mapping을 적용했다면 그 사실과 방법(clip / perceptual / relative colorimetric 등)을 리포트 메타데이터에 남겨, 최종 점수가 "인코딩 손실만"인지 "색역 변환 포함"인지 구분 가능하게 한다.

**탐지 방법**:
- Semantic: 품질 지표 계산 함수 시그니처가 `ColorMetadata`/`primaries`를 파라미터로 받는지 확인.
- Static: `compare_streams`류 진입점에서 `reference.color`와 `distorted.color`를 비교하는 코드가 존재하는지 grep.
- Manual: 동일 영상을 primaries만 다르게 태깅한 두 파일을 만들어 비교했을 때, "인코딩 손실 0"이 아닌 비정상적으로 낮은 점수가 나오는지 확인.

**예외**:
- 두 스트림이 의도적으로 서로 다른 색역이며, 비교 목적이 정확히 "이 색역 변환이 얼마나 지각적으로 차이나는가"를 측정하는 것이라면 gamut mapping 후 비교가 곧 목적이다. 이 경우도 원시 plane 값을 그대로 diff하는 것이 아니라 공통 색공간을 거쳐야 한다는 점은 동일하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-002: transfer characteristics 무시
**분류**: 색공간 메타데이터 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_ssim(reference: &Plane<u16>, distorted: &Plane<u16>, bit_depth: u8) -> f64 {
    // reference는 PQ(ST 2084) transfer, distorted는 BT.709 gamma로 인코딩되었을 수 있는데
    // 둘 다 "코드값"으로만 취급하고 transfer function을 전혀 고려하지 않는다
    let (mu_r, mu_d) = (mean(reference), mean(distorted));
    let (var_r, var_d) = (variance(reference, mu_r), variance(distorted, mu_d));
    let cov = covariance(reference, distorted, mu_r, mu_d);
    ssim_formula(mu_r, mu_d, var_r, var_d, cov, bit_depth)
}
```

**문제**:
- SSIM/PSNR 같은 지표는 원래 "코드값 공간에서의 구조/오차"를 재는데, 코드값과 지각적 밝기의 관계(transfer function)가 스트림마다 다르면 같은 코드값 차이가 전혀 다른 지각적 차이를 의미한다.
- PQ는 로그에 가까운 극도로 비선형인 곡선이라, 저휘도 영역의 코드값 1 차이와 고휘도 영역의 코드값 1 차이가 나타내는 실제 밝기 차이가 수백 배 다르다. BT.709 gamma와 같은 잣대로 비교하면 무의미하다.
- 레퍼런스(HDR/PQ)와 인코드(SDR/BT.709 gamma)의 transfer가 다른데 이를 무시하면, 지표가 "인코더가 형편없다"는 결론을 내지만 실제로는 두 곡선의 근본적 형태 차이를 측정한 것에 불과하다.

**발생 조건**:
- HDR10(PQ) 레퍼런스와 SDR(BT.709/sRGB) 인코드를 같은 파이프라인에서 비교할 때.
- HLG(Hybrid Log-Gamma) 콘텐츠와 PQ 콘텐츠를 서로 비교하는 방송 워크플로.
- 트랜스코더가 transfer function을 변환했는데(예: PQ→BT.709 톤매핑) 그 변환 이력이 비교 로직에 전달되지 않는 경우.

**권장**:
```rust
fn compute_ssim(
    reference: &Plane<u16>, ref_transfer: TransferCharacteristics,
    distorted: &Plane<u16>, dist_transfer: TransferCharacteristics,
    bit_depth: u8,
) -> Result<f64, TransferMismatch> {
    if ref_transfer != dist_transfer {
        return Err(TransferMismatch { reference: ref_transfer, distorted: dist_transfer });
    }
    // 필요하다면 공통 선형 공간으로 변환 후 지표 계산 (COLOR-008 참고: PQ와 linear를 혼동하지 않도록 주의)
    let linear_ref = eotf_to_linear(reference, ref_transfer, bit_depth);
    let linear_dist = eotf_to_linear(distorted, dist_transfer, bit_depth);
    Ok(ssim_on_linear(&linear_ref, &linear_dist))
}
```
- 지표 계산 전 두 스트림의 transfer characteristics를 명시적으로 확인하고, 다르면 공통 도메인(선형 광 또는 지각 균일 공간, 예: PU21)으로 변환 후 비교한다.
- HDR 지표(PU-PSNR, HDR-VDP 계열)를 쓸 때는 해당 지표가 요구하는 입력 도메인(선형 vs PQ 코드값)을 정확히 지켜야 한다.

**탐지 방법**:
- Semantic: 지표 함수가 `transfer` 파라미터 없이 원시 코드값만 받는지 시그니처 검사.
- Static: `TransferCharacteristics::Pq`와 `TransferCharacteristics::Bt709` 값이 서로 다른 스트림 쌍이 비교 파이프라인에 그대로 들어가는 경로 grep.
- Manual: 동일한 선형 밝기를 갖는 PQ 인코드와 BT.709 인코드 쌍을 만들어(수학적으로 등가) 비교했을 때 지표가 0에 가까운 오차를 보고하는지 검증.

**예외**:
- 두 스트림이 처음부터 동일한 transfer로 태깅되어 있음이 파이프라인 상에서 보장된 경우(예: 같은 마스터에서 파생된 A/B 인코딩 실험)라면 별도 변환 없이 코드값 비교가 정확하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-003: matrix coefficients 무시
**분류**: 색공간 메타데이터 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn diff_frame(reference: &YuvFrame, distorted: &YuvFrame) -> FrameDiff {
    // reference는 BT.2020 NCL 행렬로 만든 YCbCr, distorted는 BT.601 행렬로 재인코딩된 YCbCr
    // 둘 다 "Y, Cb, Cr"이라는 이름만 보고 같은 좌표계라고 가정
    FrameDiff {
        y_mse: mean_squared_error(&reference.y, &distorted.y),
        u_mse: mean_squared_error(&reference.u, &distorted.u),
        v_mse: mean_squared_error(&reference.v, &distorted.v),
    }
}
```

**문제**:
- RGB→YCbCr 변환 행렬(BT.601 / BT.709 / BT.2020 NCL / BT.2020 CL)이 다르면, 동일한 RGB 원본이라도 Y/Cb/Cr 코드값이 서로 다르게 나온다. 특히 크로마 채널(Cb/Cr)의 차이가 크다.
- matrix가 다른 두 YCbCr 스트림을 그대로 빼면, "인코더가 크로마를 훼손했다"는 결론이 실제로는 "행렬 계수가 다르다"는 사실을 측정한 것일 수 있다 — 특히 SD 콘텐츠(BT.601)를 HD 파이프라인(BT.709)에서 재인코딩하는 레거시 트랜스코딩 시나리오에서 흔하다.
- 이 문제는 PIXEL-008(디스플레이용 YUV→RGB 변환에서 matrix 무시)과 근본 원인은 같지만, VQ-Probe에서는 "화면이 이상해 보인다"가 아니라 "품질 점수가 조용히 틀린다"는 형태로 나타나 훨씬 늦게 발견된다.

**발생 조건**:
- 레퍼런스와 인코드가 서로 다른 트랜스코딩 세대를 거쳐, matrix coefficients가 명시적으로 다르게 시그널링된 경우.
- 레거시 콘텐츠(BT.601) 대 현대 인코드(BT.709) 비교, 또는 HDR 파이프라인의 BT.2020 NCL 대 실수로 BT.709 행렬을 쓴 트랜스코더 출력 비교.

**권장**:
```rust
fn diff_frame(reference: &YuvFrame, distorted: &YuvFrame) -> Result<FrameDiff, ColorMismatch> {
    if reference.matrix != distorted.matrix {
        // YCbCr 상태로는 직접 비교하지 않는다 — RGB(또는 XYZ)로 역변환 후 공통 공간에서 비교
        let rgb_ref = ycbcr_to_rgb(reference, reference.matrix, reference.range);
        let rgb_dist = ycbcr_to_rgb(distorted, distorted.matrix, distorted.range);
        return Ok(diff_in_rgb(&rgb_ref, &rgb_dist));
    }
    Ok(FrameDiff {
        y_mse: mean_squared_error(&reference.y, &distorted.y),
        u_mse: mean_squared_error(&reference.u, &distorted.u),
        v_mse: mean_squared_error(&reference.v, &distorted.v),
    })
}
```
- YCbCr 도메인에서 직접 비교하는 것은 두 스트림의 matrix coefficients가 동일함이 확인된 경우로 제한한다.
- 다를 경우 RGB(혹은 그 이상의 device-independent 공간)로 역변환한 뒤 비교하고, 어떤 matrix를 각각 사용했는지 리포트에 남긴다.

**탐지 방법**:
- Semantic: `diff_frame`류 함수가 `matrix` 필드를 읽거나 비교하는 코드가 있는지 확인.
- Static: YCbCr plane을 직접 빼는 코드 경로 앞에 matrix 동등성 검사가 없는 곳을 grep.
- Manual: 동일 RGB 소스를 BT.601과 BT.709 행렬로 각각 YCbCr 인코딩한 뒤 비교해, "무손실 재인코딩"인데도 지표가 0이 아닌 값을 보고하는지 확인.

**예외**:
- 파이프라인이 처음부터 단일 matrix만 지원하도록 설계되어 있고 입력 검증에서 이를 강제한다면(다른 matrix는 애초에 거부), 런타임 비교 단계에서 별도 확인은 생략할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-004: full/limited range 무시
**분류**: 색공간 메타데이터 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_mae(reference: &Plane<u8>, distorted: &Plane<u8>) -> f64 {
    // reference는 full range(0-255), distorted는 limited range(16-235)로 인코딩되었어도
    // 둘 다 그냥 u8 코드값으로 취급해 절대차를 낸다
    let sum: u64 = reference.data.iter().zip(distorted.data.iter())
        .map(|(&r, &d)| (r as i32 - d as i32).unsigned_abs() as u64)
        .sum();
    sum as f64 / reference.data.len() as f64
}
```

**문제**:
- limited range(16-235 for 8-bit luma)와 full range(0-255)는 같은 밝기를 나타내는 코드값 자체가 다르다. 순수한 range 변환만 거친(인코딩 손실이 없는) 두 스트림도 이 함수로는 큰 오차로 잡힌다.
- 이 오차는 프레임 전역에 걸쳐 체계적(systematic)이라서 — 어두운 곳도, 밝은 곳도 일관되게 오프셋 — MAE/MSE를 실제 인코딩 손실보다 훨씬 크게 부풀리고, 이 부풀림이 인코더 품질 비교(다른 CRF/bitrate 설정 비교)의 상대 순위까지 왜곡할 수 있다.
- range 불일치는 크래시나 시각적 이상으로 드러나지 않고 "이 인코더 설정이 저 설정보다 나쁘다"는 조용하지만 잘못된 결론으로 이어지기 때문에 특히 위험하다.

**발생 조건**:
- 레퍼런스는 캡처 장비(보통 full range)에서 온 원본, 인코드는 방송 표준 인코더(보통 limited range 강제)를 거친 경우.
- 컨테이너/코덱 레벨의 `color_range` 플래그가 스트림마다 다르게 시그널링되었거나, 트랜스코더가 range를 변환했지만 그 사실이 메타데이터에 반영되지 않은 경우(가장 위험한 케이스 — 플래그는 같은데 실제 값 범위는 다름).

**권장**:
```rust
fn compute_mae(
    reference: &Plane<u8>, ref_range: ColorRange,
    distorted: &Plane<u8>, dist_range: ColorRange,
) -> f64 {
    // 비교 전 공통 정규화 공간(예: [0.0, 1.0] 실제 신호 범위)으로 맞춘다
    let norm_ref = normalize_to_signal_range(reference, ref_range);
    let norm_dist = normalize_to_signal_range(distorted, dist_range);
    let sum: f64 = norm_ref.iter().zip(norm_dist.iter())
        .map(|(&r, &d)| (r - d).abs())
        .sum();
    sum / norm_ref.len() as f64
}

fn normalize_to_signal_range(plane: &Plane<u8>, range: ColorRange) -> Vec<f64> {
    match range {
        ColorRange::Full => plane.data.iter().map(|&v| v as f64 / 255.0).collect(),
        ColorRange::Limited => plane.data.iter()
            .map(|&v| (v as f64 - 16.0) / (235.0 - 16.0)) // luma 기준, chroma는 16-240 범위 사용
            .collect(),
    }
}
```
- 코드값을 직접 비교하지 않고, 항상 "실제 신호가 나타내는 정규화된 값"으로 변환한 뒤 비교한다.
- range 메타데이터가 신뢰할 수 없는 소스(예: 일부 캡처 장비, 컨슈머 카메라)일 경우, 실제 픽셀 값 분포(히스토그램이 16 미만/235 초과 값을 포함하는지)로 range를 추정하는 휴리스틱을 별도 검증 단계로 둔다.

**탐지 방법**:
- Static: 지표 계산 함수가 `ColorRange`를 파라미터로 받지 않으면서 코드값을 직접 빼는지 시그니처/본문 검사.
- Semantic: `16.0`, `235.0`, `240.0` 같은 limited-range 상수가 정규화 로직 어디에도 등장하지 않는 경우 의심.
- Manual: 동일 영상을 full range와 limited range로 각각 인코딩(무손실 range 변환만 적용)한 뒤 비교해, MAE가 0에 가까운지 확인.

**예외**:
- 두 스트림이 파이프라인 계약상 항상 동일 range로 강제된다면(예: 내부 인코딩 실험이 모두 full range 고정) 정규화를 생략해도 무방하지만, 이 가정이 깨지는 순간(외부 소스 도입 등) 조용히 틀린 값을 낼 위험이 있으므로 최소한 assert로 가정을 강제해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-005: 8-bit와 10-bit 값을 그대로 비교
**분류**: 비트 심도 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_psnr_cross_depth(reference: &Plane<u16>, distorted: &Plane<u8>) -> f64 {
    // reference는 10-bit(0-1023, u16 컨테이너), distorted는 8-bit(0-255)
    // 컨테이너 타입 변환(as f64)만 하고 스케일은 맞추지 않은 채 그대로 뺀다
    let mse: f64 = reference.data.iter().zip(distorted.data.iter())
        .map(|(&r, &d)| {
            let diff = r as f64 - d as f64; // 예: 512 - 128 = 384, 실제로는 거의 동일한 밝기인데
            diff * diff
        })
        .sum::<f64>() / reference.data.len() as f64;
    psnr_from_mse(mse, 1023.0) // MAX_I도 reference 기준으로만 고정
}
```

**문제**:
- 10-bit 코드값(0-1023)과 8-bit 코드값(0-255)은 스케일 자체가 다르다. 같은 밝기를 나타내는 10-bit 512와 8-bit 128을 그대로 빼면 384라는 거대한 가짜 오차가 나온다.
- 이 함수는 크래시하지 않고 그럴듯한 숫자(PSNR 값)를 반환하기 때문에, 검토자가 "10-bit 레퍼런스 대비 8-bit 인코드 품질이 심각하게 나쁘다"고 오판하기 쉽다 — 실제로는 단위 환산 버그다.
- MAX_I를 레퍼런스의 비트 심도(1023)로만 고정한 것도 별개의 오류: PSNR의 MAX_I는 "오차가 측정된 도메인의 최대값"이어야 하는데, 두 신호가 다른 스케일을 갖는 상태에서 이 값 자체가 무의미하다.

**발생 조건**:
- 레퍼런스가 마스터(10/12-bit)이고 테스트 대상이 8-bit 배포용 인코드인, VQ-Probe에서 가장 흔한 비교 시나리오.
- 서로 다른 인코더 설정(8-bit profile vs 10-bit profile)을 비교하는 A/B 테스트에서 두 출력을 정규화 없이 나란히 비교하는 경우.

**권장**:
```rust
fn compute_psnr_cross_depth(
    reference: &Plane<u16>, ref_bit_depth: u8,
    distorted: &Plane<u8>, dist_bit_depth: u8,
) -> f64 {
    // 항상 공통 정규화 스케일([0.0, 1.0])로 변환한 뒤 비교
    let ref_max = ((1u32 << ref_bit_depth) - 1) as f64;
    let dist_max = ((1u32 << dist_bit_depth) - 1) as f64;

    let mse: f64 = reference.data.iter().zip(distorted.data.iter())
        .map(|(&r, &d)| {
            let r_norm = r as f64 / ref_max;
            let d_norm = d as f64 / dist_max;
            (r_norm - d_norm).powi(2)
        })
        .sum::<f64>() / reference.data.len() as f64;

    psnr_from_mse(mse, 1.0) // 정규화 도메인에서는 MAX_I = 1.0
}
```
- 서로 다른 bit depth를 갖는 두 신호를 비교할 때는 반드시 공통 정규화 스케일(보통 [0, 1] 실수 도메인)로 변환한 뒤 오차를 계산한다.
- PSNR의 MAX_I는 정규화 도메인의 최대값(1.0)으로 고정하거나, 비교 목적에 맞는 명시적 기준(예: "8-bit 등가 PSNR"로 보고하려면 그 변환식을 문서화)으로 통일한다.

**탐지 방법**:
- Semantic: 두 개의 서로 다른 컨테이너 타입(`Plane<u8>`, `Plane<u16>`)을 입력으로 받는 비교 함수가 정규화 코드 없이 바로 산술 연산을 하는지 확인.
- Static: `as f64`/`as f32` 캐스팅 직후 정규화(나눗셈) 없이 바로 뺄셈이 오는 패턴 grep.
- Manual: 알려진 동일 밝기의 10-bit/8-bit 페어(예: 10-bit 512 ≈ 8-bit 128)로 단위 테스트를 작성해 정규화 후 오차가 0에 가까운지 검증.

**예외**:
- 두 신호가 이미 동일한 정규화 파이프라인을 거쳐 공통 스케일의 float 버퍼로 전달된 이후 단계라면, 추가 스케일 보정 없이 직접 비교가 맞다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-006: bit shift만으로 bit depth 정규화
**분류**: 비트 심도 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn normalize_to_10bit(sample: u16, source_bit_depth: u8) -> u16 {
    // 8-bit -> 10-bit을 단순 좌측 shift로 "정규화"
    match source_bit_depth {
        8 => sample << 2,
        10 => sample,
        12 => sample >> 2,
        _ => sample,
    }
}
```

**문제**:
- 8-bit 값을 왼쪽으로 2비트 shift하면 하위 2비트가 항상 0으로 채워진다. 이는 진짜 10-bit 정밀도가 아니라 "8-bit 값을 10-bit 컨테이너에 옮겨 담았을 뿐"인데, 이후 비교 로직이 두 신호를 "동일한 유효 정밀도의 10-bit"로 착각하게 만든다.
- 진짜 10-bit 레퍼런스(하위 비트에 실제 정보가 있음)와 8-bit를 shift로 늘린 "가짜 10-bit"를 비교하면, 하위 2비트 영역에서 레퍼런스만 변화하고 distorted는 항상 0이므로 이 영역이 인코딩 손실로 잡힌다 — 실제로는 애초에 정밀도가 없던 소스의 특성일 뿐이다.
- 반대로 12-bit를 단순 shift(`>>2`)로 10-bit로 줄이는 것도 반올림 없는 절삭(truncation)이라 계통적 음의 바이어스를 만든다(COLOR-011의 rounding 문제와 결합).

**발생 조건**:
- 레퍼런스와 인코드의 원천 bit depth가 다른데(예: 8-bit 웹캠 캡처를 레퍼런스로, 10-bit profile로 인코딩한 결과물을 테스트 대상으로), 비교를 위해 한쪽을 다른 쪽 depth로 "맞추는" 전처리 단계.
- 여러 bit depth의 인코더 설정을 한 파이프라인에서 일괄 비교해야 하는 배치 벤치마크 도구.

**권장**:
```rust
fn normalize_to_10bit(sample: u16, source_bit_depth: u8) -> u16 {
    match source_bit_depth {
        8 => {
            // 단순 shift 대신 비례 스케일링(반올림 포함)으로 유효 범위를 정확히 매핑
            let scaled = (sample as f64) * 1023.0 / 255.0;
            scaled.round() as u16
        }
        10 => sample,
        12 => {
            let scaled = (sample as f64) * 1023.0 / 4095.0;
            scaled.round() as u16
        }
        _ => sample,
    }
}
// 더 중요한 원칙: 애초에 정규화된 정수 도메인으로 변환하지 말고,
// 비교 직전까지 원본 bit depth를 유지한 뒤 최종적으로 [0.0, 1.0] 실수로 정규화한다 (COLOR-005 권장 참고).
```
- bit depth 변환이 정말 필요하다면 단순 shift가 아니라 "유효 범위 대 유효 범위" 비례 스케일링 + 반올림을 사용한다.
- 가능하면 정수 도메인 변환 자체를 생략하고, 비교 직전에 실수 정규화 도메인으로 한 번에 변환하는 COLOR-005의 접근을 우선한다 — 중간 정수 변환 단계가 늘어날수록 절삭 오차가 누적된다.
- 낮은 bit depth 소스를 shift로 "업스케일"했다는 사실 자체를 메타데이터에 남겨, 하위 비트가 "진짜 정밀도"가 아님을 리포트 소비자가 알 수 있게 한다.

**탐지 방법**:
- Static: `<<`/`>>` 연산으로 bit depth 변환을 수행하는 함수를 grep(특히 정수 shift 뒤에 반올림 보정이 없는 경우).
- Semantic: bit depth 변환 함수의 반환값이 항상 하위 N비트가 0인지(shift-up의 특징) 단위 테스트로 확인.
- Manual: 8-bit 소스를 shift로 늘린 "가짜 10-bit"와 진짜 10-bit 레퍼런스를 비교했을 때, 하위 비트 영역에서만 국소적으로 오차가 몰리는 패턴이 나오는지 히스토그램으로 확인.

**예외**:
- 두 스트림 모두 shift 방식으로 동일하게 늘린 값이며, 비교 목적이 "정확한 절대 정밀도"가 아니라 "동일한 변환을 거친 두 결과의 상대적 차이"라면(예: 같은 8-bit 소스를 다른 인코더로 인코딩한 결과를 비교) shift 방식이 두 쪽 모두에 일관되게 적용되는 한 상대 비교는 유효할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-007: HDR을 SDR metric으로 직접 비교
**분류**: HDR · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_hdr_psnr(reference: &Plane<u16>, distorted: &Plane<u16>) -> f64 {
    // SDR용으로 설계된 표준 PSNR 공식을 PQ 코드값에 그대로 적용
    let mse = mean_squared_error_u16(reference, distorted);
    psnr_from_mse(mse, 1023.0) // 10-bit MAX_I만 맞추면 "HDR 지원"이라고 착각
}
```

**문제**:
- 표준 PSNR은 균등 양자화(코드값 1 차이가 어디서나 동일한 지각적 중요도를 갖는다는 암묵적 가정, 정확히는 SDR gamma 곡선에서 어느 정도 성립)를 전제로 설계됐다. PQ는 이 가정이 크게 깨지는 곡선이라, 동일한 코드값 오차가 어두운 영역과 밝은(수천 니트) 영역에서 전혀 다른 지각적 의미를 갖는다.
- HDR 콘텐츠에 SDR PSNR을 그대로 적용하면, 어두운 영역의 코드값 오차(사람 눈에 매우 잘 보임, PQ 저역은 조밀하게 양자화됨)와 밝은 하이라이트의 코드값 오차(덜 민감함)를 동일 가중치로 합산해 지각과 무관한 숫자를 만든다.
- 비트 심도(MAX_I)만 10-bit에 맞춘다고 "HDR 대응"이 되는 것이 아니다 — 이는 컨테이너 크기를 맞춘 것이지 지각 모델을 맞춘 게 아니다.

**발생 조건**:
- HDR10/HLG 콘텐츠 비교 파이프라인이 SDR 시절에 만든 PSNR/SSIM 구현을 bit depth 파라미터만 바꿔 재사용하는 경우(가장 흔한 발생 경로).
- HDR 전용 지표(HDR-VDP, PU-PSNR, PU-SSIM 등)를 도입하지 않고 기존 지표를 "10-bit 지원"이라는 이름으로만 확장한 경우.

**권장**:
```rust
fn compute_hdr_quality(
    reference: &Plane<u16>, distorted: &Plane<u16>,
    transfer: TransferCharacteristics, peak_nits: f64,
) -> HdrQualityScore {
    match transfer {
        TransferCharacteristics::Pq | TransferCharacteristics::Hlg => {
            // PU21 등 지각적으로 균등화된 인코딩으로 변환 후 지표 계산 (PSNR이 아니라 PU-PSNR)
            let pu_ref = pu21_encode(reference, transfer, peak_nits);
            let pu_dist = pu21_encode(distorted, transfer, peak_nits);
            HdrQualityScore::PuPsnr(psnr_from_mse(mean_squared_error(&pu_ref, &pu_dist), pu21_max()))
        }
        _ => HdrQualityScore::StandardPsnr(compute_psnr_sdr(reference, distorted)),
    }
}
```
- HDR 콘텐츠에는 지각적으로 균등화된 중간 표현(PU21/PU-PSNR, PU-SSIM 등)을 거친 HDR 전용 지표를 사용한다.
- 리포트에 어떤 지표(표준 PSNR인지 HDR 지표인지)를 사용했는지 명시해, SDR 지표와 HDR 지표의 숫자를 혼동해 나란히 비교하지 않도록 한다.

**탐지 방법**:
- Semantic: HDR 콘텐츠(transfer가 PQ/HLG) 경로가 SDR 경로와 동일한 `compute_psnr`/`compute_ssim` 함수를 호출하는지 호출 그래프 확인.
- Static: 코드베이스에 PU21, HDR-VDP, HDR 전용 SSIM 변형이 전혀 등장하지 않으면서 "HDR 지원"을 표방하는 모듈이 있는지 검색.
- Manual: 동일한 지각적 오차(사람이 보기에 동등하게 눈에 띄는 결함)를 저역과 고역 하이라이트에 각각 주입한 테스트 벡터로, 표준 PSNR이 두 경우에 크게 다른 페널티를 주는지 확인.

**예외**:
- 파이프라인이 명시적으로 "코드값 정확도 검증"(예: 무손실 전송 여부 확인, 비트 단위 재현성 테스트)을 목적으로 한다면 표준 PSNR/코드값 비교가 적절하다 — 이때는 "지각 품질"이 아니라 "비트 정확성"을 재는 것이라는 목적을 명확히 구분해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-008: PQ 값을 linear light로 오해
**분류**: HDR · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_linear_light_metric(reference: &Plane<u16>, distorted: &Plane<u16>, bit_depth: u8) -> f64 {
    // "linear light 기반 지표"라고 이름 붙였지만, PQ 코드값을 정규화만 하고 EOTF를 적용하지 않음
    let max_val = ((1u32 << bit_depth) - 1) as f64;
    let mse: f64 = reference.data.iter().zip(distorted.data.iter())
        .map(|(&r, &d)| {
            // 이 값은 여전히 PQ 코드값의 [0,1] 정규화일 뿐, 실제 선형 광량(nits)이 아니다
            let r_lin = r as f64 / max_val;
            let d_lin = d as f64 / max_val;
            (r_lin - d_lin).powi(2)
        })
        .sum::<f64>() / reference.data.len() as f64;
    mse // "linear" 도메인 지표라고 부르지만 실제로는 여전히 PQ 압축 도메인
}
```

**문제**:
- PQ(ST 2084)는 절대 휘도(nits)를 매우 비선형적으로 코드값에 매핑하는 EOTF다. 코드값을 [0,1]로 정규화하는 것과 "linear light(선형 광량)로 변환"하는 것은 전혀 다른 연산인데, EOTF 적용을 빼먹고 정규화만 하면 여전히 PQ 압축 도메인에 있는 값을 "선형"이라고 착각하게 된다.
- 이 오해는 코드가 잘 동작하는 것처럼 보인다는 점에서 특히 위험하다 — 값이 [0,1] 범위에 들어오고, 지표도 그럴듯한 숫자를 낸다. 그러나 저휘도 영역(PQ 코드값이 조밀하게 몰려 있는 영역)의 실제 광량 차이가 극도로 과소평가되고, 고휘도 영역(코드값이 성기게 퍼진 영역)의 차이는 과대평가된다.
- "linear light 기반이라 더 정확하다"는 이름의 지표가 실제로는 EOTF를 거치지 않아 PQ 곡선의 왜곡을 그대로 지표에 실어 나르는 경우, 검증 없이는 발견하기 매우 어렵다.

**발생 조건**:
- PQ EOTF(코드값 → 절대 니트 또는 상대 선형값) 변환 함수 호출이 파이프라인 어딘가에서 누락된 경우 — 특히 "10-bit 정규화"와 "PQ EOTF 적용"을 같은 단계로 착각해 하나로 합쳐 구현했을 때.
- 외부 라이브러리의 "linear" 옵션이 실제로는 다른 것(예: display-linear가 아니라 scene-linear, 또는 단순 정규화)을 의미하는데 이름만 보고 신뢰한 경우.

**권장**:
```rust
fn pq_eotf_to_nits(code_value_normalized: f64) -> f64 {
    // ST 2084 EOTF: 코드값 [0,1] -> 절대 휘도(nits, 0-10000)
    const M1: f64 = 2610.0 / 16384.0;
    const M2: f64 = 2523.0 / 4096.0 * 128.0;
    const C1: f64 = 3424.0 / 4096.0;
    const C2: f64 = 2413.0 / 4096.0 * 32.0;
    const C3: f64 = 2392.0 / 4096.0 * 32.0;

    let e_pow_1_m2 = code_value_normalized.powf(1.0 / M2);
    let numerator = (e_pow_1_m2 - C1).max(0.0);
    let denominator = C2 - C3 * e_pow_1_m2;
    10000.0 * (numerator / denominator).powf(1.0 / M1)
}

fn compute_linear_light_metric(reference: &Plane<u16>, distorted: &Plane<u16>, bit_depth: u8) -> f64 {
    let max_val = ((1u32 << bit_depth) - 1) as f64;
    let mse: f64 = reference.data.iter().zip(distorted.data.iter())
        .map(|(&r, &d)| {
            let r_nits = pq_eotf_to_nits(r as f64 / max_val); // 실제 EOTF 적용
            let d_nits = pq_eotf_to_nits(d as f64 / max_val);
            (r_nits - d_nits).powi(2)
        })
        .sum::<f64>() / reference.data.len() as f64;
    mse
}
```
- "linear light 도메인"이라고 주장하는 모든 지표 경로에 실제 EOTF(PQ는 ST 2084 공식, HLG는 별도 OOTF까지 포함) 적용 코드가 존재하는지 반드시 확인한다.
- 단순 정규화(`/ max_val`)와 EOTF 적용을 함수/타입 레벨에서 명확히 구분한다(예: `NormalizedCodeValue`와 `LinearLightNits`를 별개 타입으로).

**탐지 방법**:
- Semantic: "linear"라는 이름이 붙은 함수/변수가 실제로 `powf`/EOTF 상수(ST 2084의 m1/m2/c1/c2/c3)를 사용하는지 코드 검사.
- Static: 타입 시스템에 `NormalizedCodeValue` vs `LinearNits` 구분이 없고 둘 다 `f64`로만 표현되어 있어 실수로 섞일 여지가 있는지 확인.
- Manual: PQ 코드값 0.5(정규화 기준)를 EOTF에 통과시켰을 때 나오는 니트 값이 알려진 기준값(약 92 nits 부근, 정확한 공식 기준으로 검증)과 일치하는지 단위 테스트.

**예외**:
- 지표가 의도적으로 "코드값 압축 도메인에서의 오차"(즉 PQ 자체의 지각 균등화 특성을 활용하는 지표, 예: 단순 PQ 코드값 PSNR을 "근사 지각 지표"로 명시적으로 채택하는 경우)를 재는 것이 목적이라면 EOTF를 적용하지 않는 것이 오히려 의도된 설계다. 이 경우 지표 이름에 "linear"를 붙이지 않아야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-009: chroma plane에 luma와 동일 가중치 사용
**분류**: 채널 가중치 · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_overall_psnr(y_psnr: f64, u_psnr: f64, v_psnr: f64) -> f64 {
    // Y/U/V를 단순 산술 평균 — 채널별 지각적 중요도와 픽셀 수 차이를 모두 무시
    (y_psnr + u_psnr + v_psnr) / 3.0
}
```

**문제**:
- 사람의 시각은 밝기(luma) 변화에 훨씬 민감하고 색차(chroma)에는 덜 민감하다. Y/U/V를 동일 가중치로 평균하면 실제 지각 품질과 상관관계가 낮은 숫자가 나온다.
- 4:2:0처럼 크로마가 서브샘플링된 포맷에서는 U/V plane의 픽셀 수 자체가 Y의 1/4밖에 안 되는데, 이를 무시하고 "평균 PSNR"이라는 이름으로 단순 평균하면 척도상의 의미도 모호해진다.
- 서로 다른 chroma subsampling(4:2:0 vs 4:4:4)을 가진 두 스트림을 비교할 때, 동일 가중치 평균은 "크로마 서브샘플링 차이"와 "실제 인코딩 손실"을 더욱 뒤섞는다(COLOR-020과 결합해 문제가 커진다).

**발생 조건**:
- 전체 프레임 품질을 하나의 스칼라로 요약해서 리포트/대시보드에 표시해야 하는 모든 경우.
- 크로마 손실이 유독 심한 인코더 설정(저비트레이트에서 크로마를 더 공격적으로 압축하는 경우)을 비교할 때, 잘못된 가중치가 순위를 왜곡할 수 있다.

**권장**:
```rust
fn compute_overall_psnr(y_psnr: f64, u_psnr: f64, v_psnr: f64, chroma_format: ChromaFormat) -> f64 {
    // 업계 관행(예: 6:1:1) 가중치를 명시적으로 사용하고, 근거를 코드에 남긴다
    // MSE 기준으로 가중 평균하는 것이 PSNR 값을 직접 가중 평균하는 것보다 이론적으로 더 정확함에 유의
    const Y_WEIGHT: f64 = 6.0;
    const CHROMA_WEIGHT: f64 = 1.0;
    let total_weight = Y_WEIGHT + 2.0 * CHROMA_WEIGHT;
    (Y_WEIGHT * y_psnr + CHROMA_WEIGHT * u_psnr + CHROMA_WEIGHT * v_psnr) / total_weight
}

// 더 엄밀하게는 PSNR을 평균하지 않고 MSE 단계에서 픽셀 수 가중 평균 후 최종 PSNR을 한 번만 계산한다
fn compute_overall_psnr_from_mse(y_mse: f64, y_pixels: usize, u_mse: f64, u_pixels: usize, v_mse: f64, v_pixels: usize, max_i: f64) -> f64 {
    let total_pixels = (y_pixels + u_pixels + v_pixels) as f64;
    let weighted_mse = (y_mse * y_pixels as f64 + u_mse * u_pixels as f64 + v_mse * v_pixels as f64) / total_pixels;
    psnr_from_mse(weighted_mse, max_i)
}
```
- Y/U/V 가중치는 업계에서 통용되는 값(예: 6:1:1)을 명시적 상수로 두고, 왜 그 값을 택했는지 주석/문서로 남긴다.
- 가능하면 PSNR 값 자체를 평균하지 말고 MSE 단계에서 픽셀 수 가중 평균한 뒤 최종적으로 한 번만 로그 변환한다(PSNR은 로그 스케일이라 산술 평균이 통계적으로 왜곡되기 쉽다).

**탐지 방법**:
- Static: `(y + u + v) / 3.0`처럼 균등 평균 리터럴을 grep.
- Semantic: 전체 품질 스코어 계산 함수가 `chroma_format`/픽셀 수 정보를 파라미터로 받는지 확인.
- Manual: Y는 그대로 두고 U/V만 크게 훼손한 테스트 벡터와, U/V는 그대로 두고 Y만 훼손한 테스트 벡터를 만들어 "전체 점수"가 실제 지각 품질 순위와 일치하는지(Y 훼손 쪽이 더 낮은 점수) 확인.

**예외**:
- 리포트가 "채널별 점수"를 그대로 나열하고 단일 스칼라로 합치지 않는다면(예: `y_psnr`, `u_psnr`, `v_psnr`를 별도 필드로 노출) 이 문제는 발생하지 않는다 — 합산 자체를 피하는 것도 유효한 해법이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-010: RGB 변환 후 metric 계산을 무조건 신뢰
**분류**: 변환 신뢰성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_rgb_metric(reference: &YuvFrame, distorted: &YuvFrame) -> f64 {
    // 변환 자체가 정확하다는 가정 하에, 변환된 RGB로 계산한 결과를 그대로 최종 품질 점수로 사용
    let rgb_ref = yuv_to_rgb(reference);
    let rgb_dist = yuv_to_rgb(distorted);
    compute_ssim_rgb(&rgb_ref, &rgb_dist) // 변환 오차가 최종 점수에 섞여도 알 방법이 없다
}
```

**문제**:
- YUV→RGB 변환 자체가 반올림, 행렬 계수, clamp 정책에 따라 미세한 오차를 만든다(COLOR-011, COLOR-012 참고). 이 변환을 거친 결과를 "원본 신호"인 것처럼 신뢰하고 최종 지표로 삼으면, 변환 오차가 인코딩 손실과 뒤섞인다.
- 특히 레퍼런스와 distorted가 서로 다른 변환 경로(다른 라이브러리, 다른 정밀도)를 거쳤다면, 두 RGB 사이의 차이 중 일부는 순수하게 "변환 방식의 차이"이지 인코더의 손실이 아니다.
- 이 문제는 특히 검증되지 않은 채로 지표가 "그럴듯하게" 나오기 때문에, 변환 경로를 바꾸기만 해도(예: 라이브러리 업그레이드) 동일한 인코드에 대해 점수가 미세하게 달라지는 재현성 문제로 뒤늦게 발견된다.

**발생 조건**:
- YUV 네이티브 지표 대신 RGB 기반 지표(일부 SSIM 변형, 색차 기반 지표 CIEDE2000 등)를 써야 해서 변환이 불가피한 모든 경우.
- 레퍼런스와 distorted가 서로 다른 디코더/변환 라이브러리를 거쳐 RGB로 변환되는 파이프라인(예: 한쪽은 ffmpeg swscale, 다른 쪽은 자체 구현).

**권장**:
```rust
struct MetricResult {
    score: f64,
    conversion_error_estimate: f64, // 변환 자체가 만드는 오차의 상한 추정치
}

fn compute_rgb_metric(reference: &YuvFrame, distorted: &YuvFrame, converter: &dyn YuvToRgbConverter) -> MetricResult {
    let rgb_ref = converter.convert(reference);
    let rgb_dist = converter.convert(distorted);

    // 무손실 왕복(YUV -> RGB -> YUV)으로 변환 자체의 오차를 별도 측정
    let roundtrip_error = measure_roundtrip_error(reference, converter);

    MetricResult {
        score: compute_ssim_rgb(&rgb_ref, &rgb_dist),
        conversion_error_estimate: roundtrip_error,
    }
}
```
- 동일한 변환기(같은 라이브러리, 같은 정밀도, 같은 행렬/clamp 정책)를 레퍼런스와 distorted 양쪽에 반드시 동일하게 적용한다.
- 변환기 자체의 왕복(round-trip) 오차를 별도로 측정해 리포트에 "변환 오차 대비 측정된 오차"를 병기하면, 지표가 노이즈 바닥(noise floor) 이하를 재고 있는지 판단할 수 있다(COLOR-014와 직결).

**탐지 방법**:
- Structural: 레퍼런스와 distorted가 서로 다른 변환 함수/라이브러리를 호출하는 코드 경로가 있는지 확인.
- Semantic: 변환 함수의 반환값을 검증 없이 곧바로 "최종 품질 점수" 계산에 사용하는지, 왕복 오차 측정 코드가 파이프라인에 존재하는지 확인.
- Manual: 동일한 YUV 소스를 두 번(같은 변환기로) 변환해 RGB 결과가 bit-exact한지, 변환기를 바꿔가며 동일 소스의 지표 점수가 얼마나 흔들리는지 측정.

**예외**:
- 변환기가 표준 규격(예: 특정 SIMD 명령어 집합에 대해 IEEE 754 정확도가 보증된 구현)이고 두 스트림에 항상 동일하게 적용됨이 빌드 시스템 레벨에서 보증된다면, 별도의 왕복 오차 측정 없이도 신뢰할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-011: conversion library마다 다른 rounding 무시
**분류**: 변환 신뢰성 · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
// 레퍼런스는 라이브러리 A(round-to-nearest)로 변환
fn convert_reference(frame: &YuvFrame) -> RgbFrame {
    library_a::yuv_to_rgb_round_nearest(frame)
}

// distorted는 라이브러리 B(truncate)로 변환 — 파이프라인 히스토리상 우연히 다른 경로를 탐
fn convert_distorted(frame: &YuvFrame) -> RgbFrame {
    library_b::yuv_to_rgb_truncate(frame)
}
```

**문제**:
- 반올림(round-to-nearest) vs 절삭(truncate)은 변환 결과에 최대 ±1 코드값(8-bit 기준) 차이를 체계적으로 만든다. 이 차이는 무작위 노이즈가 아니라 절삭 쪽이 항상 음의 방향으로 편향되는 바이어스다.
- 두 스트림이 서로 다른 rounding 정책의 변환기를 거치면, 이 바이어스가 "인코더가 약간 더 어둡게 만든다"처럼 보이는 가짜 신호를 만들 수 있다 — 특히 저손실/고비트레이트 비교처럼 실제 인코딩 오차가 작을 때 이 바이어스가 상대적으로 더 크게 드러난다.
- 두 변환기가 우연히 다른 라이브러리 버전을 쓰게 되는 경로(레퍼런스 로더는 오래된 유틸리티, distorted 로더는 새로 추가된 최적화 경로 등)는 코드 리뷰에서 놓치기 쉽다.

**발생 조건**:
- 레퍼런스 로딩 경로와 distorted 로딩 경로가 서로 다른 시점에 다른 팀/PR에 의해 구현되어 서로 다른 변환 유틸리티를 참조하게 된 경우.
- 서드파티 라이브러리를 부분적으로만 업그레이드해, 한쪽 경로만 새 rounding 정책을 쓰게 된 경우.

**권장**:
```rust
// 단일 변환기를 레퍼런스/distorted 양쪽에서 공유
struct SharedYuvToRgbConverter {
    rounding: RoundingMode, // 코드베이스 전체에서 단 하나의 정책만 존재
}

impl SharedYuvToRgbConverter {
    fn convert(&self, frame: &YuvFrame) -> RgbFrame {
        yuv_to_rgb_with_rounding(frame, self.rounding)
    }
}

fn build_comparator() -> Comparator {
    let converter = SharedYuvToRgbConverter { rounding: RoundingMode::RoundToNearestEven };
    Comparator::new(converter) // 레퍼런스/distorted 로더가 동일 인스턴스를 주입받는다
}
```
- 변환기 인스턴스를 하나만 만들고 레퍼런스/distorted 로딩 경로에 의존성 주입으로 공유해, "서로 다른 변환기를 쓸 수 없는" 구조로 만든다.
- CI에 "동일 변환기 강제" 린트/테스트를 추가해, 새 코드가 별도의 변환 유틸리티를 도입하면 실패하게 한다.

**탐지 방법**:
- Structural: 레퍼런스 로딩과 distorted 로딩 코드 경로에서 호출하는 변환 함수/라이브러리가 동일한지 호출 그래프 비교.
- Static: 코드베이스에 `round`, `truncate`, `floor`, `as u8` 캐스팅(암묵적 절삭) 등 서로 다른 반올림 방식이 여러 변환 함수에 흩어져 있는지 grep.
- Manual: 동일한 YUV 입력을 두 로딩 경로 각각에 흘려 변환 결과가 bit-exact한지 골든 테스트.

**예외**:
- 두 변환기의 rounding 차이가 지표의 유효숫자보다 훨씬 작다는 것이 정량적으로 증명된 경우(예: 지표가 애초에 ±0.5dB 이상의 노이즈를 갖는 성긴 비교 용도)라면 엄격한 통일이 실익이 적을 수 있다 — 다만 이 판단은 반드시 문서화되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-012: float 변환 과정의 clamp 누락
**분류**: 수치 안정성 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn tone_curve_to_linear(pq_code_value: f64) -> f64 {
    // EOTF 계산 결과가 이론적으로 항상 [0, 10000] nits 안에 들어온다고 가정하고 clamp를 생략
    // 입력이 아주 살짝 범위를 벗어나거나(부동소수점 오차) 톤매핑 후 오버슛이 있으면 음수/초과값이 그대로 전파
    let numerator = pq_code_value.powf(1.0 / 78.84) - 0.8359;
    let denominator = 18.8516 - 18.6875 * pq_code_value.powf(1.0 / 78.84);
    10000.0 * (numerator / denominator).powf(1.0 / 0.1593)
}
```

**문제**:
- 톤매핑, 필터링, 리샘플링 등 중간 연산은 이론적 범위를 살짝 벗어나는 값(예: -0.0003, 1.0002)을 흔히 만들어낸다. 이를 clamp 없이 다음 단계(특히 `powf`, `log` 같은 함수)로 넘기면 NaN/음수/허수 성격의 결과가 조용히 발생한다.
- `numerator`가 음수인데 분수 지수(`powf(1.0/0.1593)`처럼 정수가 아닌 지수)를 적용하면 Rust의 `f64::powf`는 NaN을 반환한다 — 크래시 없이 NaN이 파이프라인 전체로 퍼져나간다(COLOR-013과 직결).
- 이런 종류의 버그는 "거의 항상" 정상 값이 나오다가 특정 극단적인 픽셀(순수 흰색, 순수 검정, 톤매핑 오버슛이 발생하는 하이라이트)에서만 터지기 때문에, 일반적인 테스트 벡터로는 재현되지 않는 경우가 많다.

**발생 조건**:
- HDR 톤매핑, gamut mapping, 선형 변환 등 여러 단계의 부동소수점 연산이 연쇄되는 파이프라인.
- 극단적인 코드값(0.0, 최대값)이나 오버슛이 흔한 필터(샤프닝, 언샤프 마스크 후처리)를 거친 프레임.

**권장**:
```rust
fn tone_curve_to_linear(pq_code_value: f64) -> f64 {
    let cv = pq_code_value.clamp(0.0, 1.0); // 입력을 이론적 유효 범위로 명시적으로 강제
    let e_pow = cv.powf(1.0 / 78.84);
    let numerator = (e_pow - 0.8359).max(0.0); // 음수 방지
    let denominator = (18.8516 - 18.6875 * e_pow).max(f64::EPSILON); // 0 나눗셈 방지
    let nits = 10000.0 * (numerator / denominator).powf(1.0 / 0.1593);
    nits.clamp(0.0, 10000.0) // 출력도 유효 범위로 강제
}
```
- 모든 부동소수점 변환의 입력과 출력에 이론적 유효 범위에 대한 명시적 `clamp`를 둔다 — "이론상 항상 범위 안"이라는 가정에 의존하지 않는다.
- 분모가 0 또는 음수가 될 수 있는 모든 나눗셈에 최소값 하한을 둔다.
- 이런 clamp 지점들을 파이프라인 곳곳에 산발적으로 두기보다, 색공간 변환 유틸리티 함수 경계마다 일관된 계약(contract)으로 강제한다.

**탐지 방법**:
- Runtime: 대량의 실제/합성 프레임으로 파이프라인을 돌려 NaN/Inf 발생 빈도를 카운트하는 회귀 테스트.
- Static: `powf`, `ln`, `sqrt`, 나눗셈 연산 직전에 clamp/max/min 가드가 없는 색공간 변환 함수를 grep.
- Manual: 순수 검정(0)과 순수 흰색(최대값), 그리고 톤매핑 오버슛을 유발하는 인위적 극단값으로 각 변환 함수를 단위 테스트.

**예외**:
- 함수가 내부 스크래치 계산에서만 쓰이고, 호출부가 이미 입력 범위를 엄격히 보증하며 그 보증이 타입 시스템(예: `NormalizedF64` 뉴타입)으로 강제된다면 매 호출마다 clamp를 반복할 필요는 없다 — 다만 그 타입의 생성자 자체에는 clamp/검증이 있어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-013: NaN/Inf 발생을 정상 값으로 집계
**분류**: 수치 안정성 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
fn aggregate_frame_scores(frame_scores: &[f64]) -> f64 {
    // PSNR이 MSE=0일 때 Inf가 되거나, 색공간 변환에서 NaN이 섞여 들어와도 그대로 합산
    let sum: f64 = frame_scores.iter().sum();
    sum / frame_scores.len() as f64 // NaN이나 Inf가 하나라도 있으면 전체 평균이 오염됨
}

fn frame_psnr(mse: f64) -> f64 {
    if mse == 0.0 {
        return f64::INFINITY; // 완전 일치 프레임 — 이 자체는 정상이지만 집계 시 처리가 안 됨
    }
    psnr_from_mse(mse, 255.0)
}
```

**문제**:
- `f64::INFINITY`가 평균 계산에 섞이면 전체 평균이 `Inf`가 되어 다른 모든 프레임의 정상적인 점수를 완전히 가려버린다. 반대로 NaN이 섞이면 `Inf + NaN = NaN`이 되어 전체가 NaN으로 오염된다.
- 완전히 동일한(MSE=0) 프레임에서 PSNR이 Inf가 되는 것 자체는 수학적으로 정상이다 — 문제는 이를 "특별 케이스"로 인지하고 별도 처리하는 로직이 집계 단계에 없다는 것이다.
- NaN은 특히 위험한데, 비교 연산(`NaN < x`, `NaN == NaN`)이 항상 false를 반환하므로, "이상치 필터링" 같은 방어 코드가 있어도 NaN은 그 필터를 통과해버리는 경우가 흔하다(Rust는 `Ord`가 아니라 `PartialOrd`로 이를 어느 정도 강제하지만, `.unwrap()`이나 안전하지 않은 정렬 키 사용 시 여전히 뚫린다).
- 리포트에 조용히 이상한 숫자(NaN을 0.0으로 fallback 처리하는 경우 등)가 나오면, "이 설정의 평균 품질이 매우 낮다"는 완전히 틀린 결론으로 이어질 수 있다.

**발생 조건**:
- 완전히 동일한 프레임(정지 화면, 검은 화면, 인트로 카드 등)이 스트림에 포함되어 MSE=0이 나오는 실제 콘텐츠.
- COLOR-012처럼 색공간 변환 과정에서 clamp 누락으로 NaN이 생성되고 그대로 집계 단계까지 전파되는 경우.
- SSIM 계산에서 분산이 0인 완전 평탄한 영역(단색 배경)에서 0/0 형태의 NaN이 발생하는 경우.

**권장**:
```rust
#[derive(Debug)]
enum FrameScore {
    Finite(f64),
    PerfectMatch,      // MSE=0 -> Inf를 명시적 케이스로 분리
    Invalid(String),   // NaN 등 계산 자체가 실패한 경우, 원인을 함께 기록
}

fn frame_psnr(mse: f64) -> FrameScore {
    if mse == 0.0 {
        FrameScore::PerfectMatch
    } else if mse.is_nan() {
        FrameScore::Invalid("MSE computation produced NaN".to_string())
    } else {
        FrameScore::Finite(psnr_from_mse(mse, 255.0))
    }
}

fn aggregate_frame_scores(scores: &[FrameScore]) -> AggregateReport {
    let mut finite_sum = 0.0;
    let mut finite_count = 0usize;
    let mut perfect_count = 0usize;
    let mut invalid_frames = Vec::new();

    for (idx, score) in scores.iter().enumerate() {
        match score {
            FrameScore::Finite(v) => { finite_sum += v; finite_count += 1; }
            FrameScore::PerfectMatch => perfect_count += 1,
            FrameScore::Invalid(reason) => invalid_frames.push((idx, reason.clone())),
        }
    }

    AggregateReport {
        mean_finite_psnr: if finite_count > 0 { Some(finite_sum / finite_count as f64) } else { None },
        perfect_match_frames: perfect_count,
        invalid_frames, // 리포트 소비자가 반드시 인지해야 하는 실패 목록
    }
}
```
- Inf(완전 일치)와 NaN(계산 실패)을 별개의 명시적 케이스로 분리하고, 평균 등 집계 연산에 그대로 흘려보내지 않는다.
- NaN/Inf가 발생한 프레임 인덱스와 원인을 리포트에 명시적으로 남겨, "평균 어딘가에 숨겨서 조용히 사라지는" 대신 사용자가 그 존재를 알 수 있게 한다.

**탐지 방법**:
- Runtime: 집계 함수 출력에 `is_nan()`/`is_infinite()` 검사를 추가한 회귀 테스트, 그리고 대량 실전 데이터셋에서 이 케이스의 발생 빈도를 모니터링.
- Static: `.sum()`, 평균 계산 직후 NaN/Inf 필터링 코드가 없는 집계 함수를 grep.
- Semantic: `f64::INFINITY`를 반환하는 지표 함수와 그 소비자(집계 함수) 사이에 특수 케이스 처리가 있는지 데이터 흐름 확인.

**예외**:
- 애초에 완전 일치 프레임을 시퀀스에서 제외하고 집계하는 것이 파이프라인의 명시적 정책이라면(예: "인코딩 손실이 있는 프레임만 채점"), PerfectMatch를 finite 평균에서 제외하는 것 자체가 의도된 동작이다 — 다만 이 정책은 문서화되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-014: pixel format 변환이 metric보다 더 큰 오차를 생성
**분류**: 변환 신뢰성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compare_streams(reference: &Stream, distorted: &Stream) -> f64 {
    // 두 스트림이 서로 다른 pixel format(4:2:0 10bit vs 4:4:4 8bit)이므로
    // 공통 포맷(4:4:4 8bit RGB)으로 강제 변환 후 비교 — 이 변환 자체의 오차 규모를 검증하지 않음
    let common_ref = convert_to_common_format(reference, PixelFormat::Rgb444_8bit);
    let common_dist = convert_to_common_format(distorted, PixelFormat::Rgb444_8bit);
    compute_psnr(&common_ref, &common_dist)
}
```

**문제**:
- 4:2:0에서 4:4:4로 크로마를 업샘플링하고, 10-bit에서 8-bit로 다운샘플링해 RGB로 변환하는 과정 자체가 상당한 정보 손실/보간 오차를 만든다. 이 변환 오차가 실제로 측정하려는 "인코더가 만든 손실"보다 크면, 지표는 사실상 인코딩 손실이 아니라 변환 파이프라인의 특성을 측정하게 된다.
- 특히 고비트레이트/저손실 인코딩 비교(원본과 거의 구분 안 되는 인코드들을 순위 매기는 경우)에서, 변환 오차가 신호보다 큰 노이즈 바닥(noise floor)을 형성하면 인코더 간의 실제 미세한 품질 차이가 이 노이즈에 완전히 묻혀버린다.
- "공통 포맷으로 맞췄으니 공정한 비교"라는 직관은, 그 변환 자체가 무손실이 아니라는 사실을 간과할 때 위험한 가정이 된다.

**발생 조건**:
- 서로 다른 chroma subsampling/bit depth를 가진 두 스트림을 하나의 공통 포맷으로 강제 정규화하는 모든 비교 파이프라인.
- 인코더 A/B 테스트에서 두 인코더의 실제 품질 차이가 매우 작은(0.1dB 수준) 고품질 영역 비교.

**권장**:
```rust
struct ComparisonPlan {
    conversion_error_estimate: f64, // 변환 자체가 만드는 오차를 사전에 추정
    measured_error: f64,
}

fn compare_streams(reference: &Stream, distorted: &Stream) -> Result<ComparisonPlan, LowSignalToNoiseRatio> {
    let common_format = choose_minimal_loss_common_format(reference.format, distorted.format);

    // 변환 자체의 오차를 별도로 추정: 레퍼런스를 왕복 변환(원본포맷 -> 공통포맷 -> 원본포맷)해 측정
    let conversion_error = estimate_conversion_roundtrip_error(reference, common_format);

    let common_ref = convert_to_common_format(reference, common_format);
    let common_dist = convert_to_common_format(distorted, common_format);
    let measured_error = compute_mse(&common_ref, &common_dist);

    if measured_error < conversion_error * 2.0 {
        // 측정된 오차가 변환 노이즈 바닥과 같은 자릿수 — 결과를 신뢰할 수 없음을 명시
        return Err(LowSignalToNoiseRatio { measured_error, conversion_error });
    }

    Ok(ComparisonPlan { conversion_error_estimate: conversion_error, measured_error })
}
```
- 공통 포맷으로 변환하기 전에, 그 변환이 만드는 오차 규모를 (왕복 변환 등으로) 사전에 추정한다.
- 측정된 오차가 변환 자체의 노이즈 바닥과 비슷하거나 작으면 결과를 "신뢰할 수 없음"으로 명시적으로 플래그한다.
- 가능하면 정보 손실이 적은 방향으로 공통 포맷을 선택한다(예: 8-bit로 다운캐스트하지 말고 둘 다 10-bit 이상으로 업캐스트, 4:2:0을 4:4:4로 올리기보다 가능하면 원본 subsampling에 가까운 지표를 각각 계산).

**탐지 방법**:
- Semantic: 공통 포맷 변환 함수 호출 뒤에 변환 오차 추정/검증 코드가 있는지 확인.
- Runtime: 동일한 스트림을 자기 자신과 비교(reference == distorted)했을 때 0이 아닌 유의미한 오차가 나오는지 — 나온다면 그 값이 순수 변환 노이즈 바닥이다.
- Manual: 알려진 무손실 변환 쌍(원본 4:4:4 10bit를 4:2:0 8bit로 변환 후 다시 4:4:4 10bit로 되돌린 것)의 왕복 오차를 측정해 비교 파이프라인의 노이즈 바닥을 문서화.

**예외**:
- 두 스트림이 원래 같은 pixel format이라 변환이 아예 필요 없는 경우, 또는 변환 오차가 목표 지표 해상도(예: 0.1dB 단위)보다 최소 한 자릿수 이상 작다는 것이 사전에 검증된 안정된 파이프라인이라면 매번 재검증할 필요는 없다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-015: color metadata 누락 시 임의 기본값 사용
**분류**: 색공간 메타데이터 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn load_color_metadata(stream: &Stream) -> ColorMetadata {
    // VUI/CICP 파라미터가 시그널링되지 않았으면 조용히 BT.709를 기본값으로 사용
    stream.vui.as_ref().map(|vui| ColorMetadata::from_vui(vui))
        .unwrap_or(ColorMetadata {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Bt709,
            matrix: MatrixCoefficients::Bt709,
            range: ColorRange::Limited,
        })
}
```

**문제**:
- 색공간 메타데이터가 시그널링되지 않은 스트림에 임의의(비록 흔한 관행이라도) 기본값을 조용히 채워 넣으면, 레퍼런스는 실제로 BT.2020인데 메타데이터 누락으로 BT.709로 오인되는 경우가 생긴다 — 이는 COLOR-001~004의 모든 문제를 "메타데이터가 있는데 무시하는" 것보다 더 은밀한 방식으로 재현한다: 코드는 색공간을 "존중"하는 것처럼 보이지만 애초에 잘못된 값을 존중한다.
- 레퍼런스와 distorted가 둘 다 메타데이터 누락으로 같은 기본값을 받으면 우연히 문제가 가려지지만, 한쪽만 메타데이터가 있고 다른 쪽이 없는 비대칭적인 경우(레퍼런스는 잘 태깅된 마스터, distorted는 메타데이터를 보존하지 않는 트랜스코더를 거친 결과) 특히 위험하다.
- "기본값을 쓴다"는 결정 자체가 코드 어디에도 로그/경고로 남지 않으면, 사후에 "왜 이 비교 결과가 이상한가"를 추적할 방법이 없다.

**발생 조건**:
- 오래된 인코더/레거시 컨테이너가 VUI/CICP를 아예 시그널링하지 않는 스트림.
- 트랜스코딩 파이프라인 중 일부 단계가 색공간 메타데이터를 보존하지 않고 드롭하는 경우(FFmpeg 필터체인에서 메타데이터 전달이 누락되는 등 흔한 실무 사고).

**권장**:
```rust
enum ColorMetadataSource {
    Signaled(ColorMetadata),
    InferredDefault { assumed: ColorMetadata, reason: &'static str },
}

fn load_color_metadata(stream: &Stream) -> ColorMetadataSource {
    match stream.vui.as_ref() {
        Some(vui) => ColorMetadataSource::Signaled(ColorMetadata::from_vui(vui)),
        None => ColorMetadataSource::InferredDefault {
            // 기본값을 쓰더라도 "추정"이라는 사실 자체를 데이터로 남긴다
            assumed: ColorMetadata::conventional_default_for(stream.resolution),
            reason: "no VUI/CICP signaled; assumed by resolution convention",
        },
    }
}

fn compare_streams(reference: &Stream, distorted: &Stream) -> Result<QualityReport, ColorMetadataWarning> {
    let ref_color = load_color_metadata(reference);
    let dist_color = load_color_metadata(distorted);

    if matches!(ref_color, ColorMetadataSource::InferredDefault { .. })
        || matches!(dist_color, ColorMetadataSource::InferredDefault { .. }) {
        // 리포트에 반드시 경고로 남긴다 — 조용히 진행하지 않는다
        log::warn!("color metadata inferred, not signaled: ref={:?} dist={:?}", ref_color, dist_color);
    }
    // ...
    Ok(QualityReport::default())
}
```
- 메타데이터가 없어 기본값을 쓰는 경우, 그 사실을 타입(예: `InferredDefault` variant)으로 구분해 조용히 사라지지 않게 한다.
- 최종 리포트에 "색공간 메타데이터가 추정값임"을 경고로 명시해, 결과 소비자가 그 신뢰도를 판단할 수 있게 한다.
- 가능하면 해상도/코덱 관례(SD→BT.601, HD→BT.709 등)에 기반한 추정 로직을 명시적으로 문서화해, "그냥 아무 값"이 아니라 "업계 관례 기반 추정"임을 분명히 한다.

**탐지 방법**:
- Semantic: `.unwrap_or(default_color())`류 패턴이 로그/경고 없이 색공간 메타데이터 로딩에 쓰이는지 grep.
- Structural: 메타데이터 로딩 함수의 반환 타입이 "시그널링됨"과 "추정됨"을 구분하는지 타입 검사.
- Manual: VUI가 없는 스트림과 있는 스트림을 각각 레퍼런스/distorted로 조합해 비교했을 때, 리포트에 경고가 남는지 확인.

**예외**:
- 파이프라인이 다루는 콘텐츠 소스가 항상 단일하고 검증된 소스(예: 사내에서 항상 BT.709로 캡처/마스터링하는 스튜디오 전용 도구)로 제한되어 있어 "메타데이터 누락 = 항상 BT.709가 맞다"는 것이 운영상 보증된다면, 경고 없이 기본값을 써도 실무적으로 안전할 수 있다 — 다만 이 가정이 깨지는 외부 소스가 유입되는 순간 위험해진다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-016: HDR peak luminance metadata 무시
**분류**: HDR · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_hdr_score(reference: &HdrFrame, distorted: &HdrFrame) -> f64 {
    // 두 스트림의 MaxCLL(Maximum Content Light Level)/MaxFALL이 다를 수 있는데
    // PQ 코드값만 비교하고 실제 마스터링 peak luminance 차이를 전혀 고려하지 않음
    compute_pu_psnr(&reference.plane, &distorted.plane, TransferCharacteristics::Pq)
}
```

**문제**:
- PQ는 절대 휘도 체계라 코드값이 동일해도, 두 스트림의 실제 마스터링 peak luminance(예: 1000 nits 마스터 vs 4000 nits 마스터)가 다르면 같은 코드값이 서로 다른 절대 밝기를 의미하지 않는다(PQ 자체는 절대 코드북이지만, 콘텐츠가 실제로 사용하는 코드값 범위가 마스터링 peak에 따라 다르다).
- MaxCLL/MaxFALL이 크게 다른 두 스트림을 비교하면서 이 메타데이터를 무시하면, distorted 쪽이 재그레이딩(regrading)되어 peak가 낮아진 것뿐인데 이를 "하이라이트 디테일 손실"로 오판할 수 있다.
- HDR 스트림을 다른 peak luminance 디스플레이용으로 재타겟팅(display-referred retargeting)한 경우, 이는 인코딩 손실이 아니라 의도된 편집이므로 별도로 식별되어야 한다.

**발생 조건**:
- 동일 마스터에서 서로 다른 peak luminance 타겟(예: 1000 nits용, 4000 nits용)으로 각각 그레이딩된 HDR 배포본을 비교하는 경우.
- 트랜스코딩 과정에서 dynamic metadata(HDR10+, Dolby Vision 등)가 재계산되어 MaxCLL/MaxFALL이 변경된 스트림.

**권장**:
```rust
struct HdrComparisonContext {
    ref_max_cll: f64,
    dist_max_cll: f64,
    ref_max_fall: f64,
    dist_max_fall: f64,
}

fn compute_hdr_score(reference: &HdrFrame, distorted: &HdrFrame, ctx: &HdrComparisonContext) -> HdrQualityReport {
    let peak_mismatch_ratio = ctx.dist_max_cll / ctx.ref_max_cll;
    let score = compute_pu_psnr(&reference.plane, &distorted.plane, TransferCharacteristics::Pq);

    HdrQualityReport {
        score,
        // peak luminance 차이가 유의미하면 결과 해석에 반드시 참고해야 할 컨텍스트로 노출
        peak_luminance_mismatch: if (peak_mismatch_ratio - 1.0).abs() > 0.1 {
            Some(peak_mismatch_ratio)
        } else {
            None
        },
    }
}
```
- HDR 스트림 비교 시 MaxCLL/MaxFALL(및 가능하면 마스터링 디스플레이의 min/max luminance)을 함께 로드하고, 둘 사이의 차이를 결과 리포트에 명시적으로 노출한다.
- peak luminance가 크게 다르면 코드값을 직접 비교하기 전에 공통 타겟으로 재정규화(display-referred → scene-referred 등)하는 것을 고려한다.

**탐지 방법**:
- Semantic: HDR 지표 계산 함수가 `MaxCLL`/`MaxFALL`/마스터링 디스플레이 메타데이터를 파라미터로 받는지 확인.
- Static: HDR 메타데이터 파싱 코드(SEI mastering display colour volume, content light level 등)는 존재하지만 비교 로직에서 참조되지 않는(dead-end) 경로 grep.
- Manual: 동일 마스터를 서로 다른 peak luminance로 재그레이딩한 두 배포본을 비교해, peak 차이가 지표에 미치는 영향을 정량화.

**예외**:
- 두 스트림이 동일한 마스터링 파라미터(같은 MaxCLL/MaxFALL)로 태깅되어 있음이 사전에 보증된 A/B 인코더 비교(같은 HDR 마스터를 다른 인코더 설정으로만 인코딩)라면 이 검증은 생략 가능하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-017: mastering display metadata를 결과에서 제거
**분류**: HDR · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct QualityReport {
    psnr: f64,
    ssim: f64,
    // SMPTE ST 2086 마스터링 디스플레이 메타데이터(색역 좌표, min/max luminance)를
    // 파싱 단계에서는 읽었지만 최종 리포트 구조체에 필드가 아예 없어 버려짐
}

fn build_report(reference: &Stream, distorted: &Stream) -> QualityReport {
    let psnr = compute_psnr(reference, distorted);
    let ssim = compute_ssim(reference, distorted);
    QualityReport { psnr, ssim } // mastering_display_metadata는 여기서 조용히 사라진다
}
```

**문제**:
- 마스터링 디스플레이 메타데이터(색역 좌표, min/max luminance)는 "이 콘텐츠가 어떤 디스플레이를 기준으로 그레이딩되었는가"를 알려주는 핵심 컨텍스트다. 이것이 리포트에서 빠지면, 나중에 "왜 이 PSNR 수치가 이 정도로 나왔는가"를 재구성할 방법이 없다.
- 같은 숫자(예: PSNR 42dB)라도, 1000-nits 마스터링 기준 그레이딩과 4000-nits 마스터링 기준 그레이딩에서는 의미가 다를 수 있다. 이 컨텍스트 없이 숫자만 남으면, 서로 다른 프로젝트의 리포트를 나중에 비교하려는 사람이 사과와 오렌지를 비교하게 된다.
- 파싱 단계에서는 메타데이터를 읽어놓고 최종 리포트 구조체에만 반영하지 않는 것은, "데이터는 있지만 UI/저장 스키마가 못 따라간" 전형적인 설계 부채다.

**발생 조건**:
- 리포트 스키마가 SDR 시절(마스터링 디스플레이 개념이 없던)에 설계되어 필드가 추가되지 않은 채 HDR 지원이 나중에 얹힌 경우.
- 리포트를 JSON/DB로 직렬화할 때 스키마 마이그레이션 없이 필드가 누락되는 경우.

**권장**:
```rust
struct QualityReport {
    psnr: f64,
    ssim: f64,
    reference_mastering_display: Option<MasteringDisplayMetadata>,
    distorted_mastering_display: Option<MasteringDisplayMetadata>,
}

struct MasteringDisplayMetadata {
    primaries: [(f64, f64); 3], // R/G/B 색역 좌표
    white_point: (f64, f64),
    max_luminance_nits: f64,
    min_luminance_nits: f64,
}

fn build_report(reference: &Stream, distorted: &Stream) -> QualityReport {
    QualityReport {
        psnr: compute_psnr(reference, distorted),
        ssim: compute_ssim(reference, distorted),
        reference_mastering_display: reference.mastering_display_metadata.clone(),
        distorted_mastering_display: distorted.mastering_display_metadata.clone(),
    }
}
```
- 리포트 스키마에 마스터링 디스플레이 메타데이터를 1급 필드로 포함시켜, 지표 숫자와 함께 항상 보존한다.
- 메타데이터가 없는 경우는 `None`으로 명시하고, "존재하지 않음"과 "저장을 안 함"을 구분한다.

**탐지 방법**:
- Structural: 파싱 계층에는 mastering display metadata 타입이 존재하는데 리포트/출력 계층의 구조체에는 대응 필드가 없는지 타입 정의 비교.
- Static: 리포트 직렬화(JSON/protobuf 스키마)에 색공간/HDR 관련 필드가 있는지 grep.
- Manual: HDR 스트림으로 생성한 리포트를 열어, 원본 스트림에 SEI로 존재하는 마스터링 디스플레이 정보가 리포트에도 나타나는지 수동 대조.

**예외**:
- 리포트가 순수 SDR 콘텐츠 전용으로 스코프가 명확히 제한되어 있고, HDR 지원 계획이 없는 도구라면 이 필드 자체가 불필요할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-018: tone mapping 후 결과를 원본 HDR과 직접 비교
**분류**: HDR · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compare_hdr_via_tonemap(reference_hdr: &HdrFrame, distorted_hdr: &HdrFrame) -> f64 {
    // distorted만 미리보기용으로 SDR 톤매핑을 거쳤는데, 그 톤매핑된 결과를
    // 톤매핑되지 않은 원본 HDR 레퍼런스와 그대로 비교한다
    let distorted_tonemapped = tonemap_to_sdr(distorted_hdr);
    compute_psnr(&reference_hdr.plane, &distorted_tonemapped.plane) // 서로 다른 도메인끼리 비교
}
```

**문제**:
- 톤매핑은 HDR의 넓은 다이내믹 레인지를 SDR의 좁은 레인지로 압축하는 비가역적, 의도적인 변형이다. 톤매핑된 SDR 프레임과 톤매핑되지 않은 HDR 프레임을 직접 비교하는 것은, 서로 다른 다이내믹 레인지·transfer function을 가진 두 신호를 비교하는 것(COLOR-002, COLOR-007과 근본적으로 같은 오류)이다.
- 이 경우 지표가 보고하는 "손실"의 대부분은 인코더의 손실이 아니라 톤매핑 알고리즘이 의도적으로 압축한 하이라이트/섀도우 디테일이다 — 즉 톤매핑 함수 자체를 평가하고 있는 것이지, 인코더를 평가하고 있는 게 아니다.
- 흔히 미리보기/썸네일 생성 경로에서 만들어진 톤매핑된 버퍼를 (재변환 비용을 아끼려고) 실수로 품질 비교 경로에 재사용할 때 발생한다 — PIXEL-009/010에서 다루는 "표시용과 분석용을 분리하지 않는" 문제의 색공간 버전이다.

**발생 조건**:
- HDR 콘텐츠의 SDR 미리보기/썸네일 생성 로직과 품질 비교 로직이 같은 중간 버퍼를 공유하도록 (실수로) 설계된 경우.
- distorted 스트림 자체가 실제로 SDR로 다운컨버전된 배포본이라서 톤매핑이 불가피한 경우(이때는 레퍼런스도 동일한 톤매핑을 거쳐야 공정하다).

**권장**:
```rust
enum ComparisonDomain {
    NativeHdr, // 양쪽 다 HDR 코드값 그대로
    TonemappedSdr { tonemap_operator: TonemapOperator }, // 양쪽 다 동일 연산자로 톤매핑
}

fn compare_hdr(reference_hdr: &HdrFrame, distorted_hdr: &HdrFrame, domain: ComparisonDomain) -> f64 {
    match domain {
        ComparisonDomain::NativeHdr => {
            compute_pu_psnr(&reference_hdr.plane, &distorted_hdr.plane, TransferCharacteristics::Pq)
        }
        ComparisonDomain::TonemappedSdr { tonemap_operator } => {
            // 레퍼런스도 반드시 "동일한" 톤매핑 연산자를 거친다 — 한쪽만 거치지 않는다
            let ref_sdr = apply_tonemap(reference_hdr, tonemap_operator);
            let dist_sdr = apply_tonemap(distorted_hdr, tonemap_operator);
            compute_psnr(&ref_sdr.plane, &dist_sdr.plane)
        }
    }
}
```
- 비교는 항상 "같은 도메인"에서 이루어져야 한다: 네이티브 HDR끼리 비교하거나, 동일한 톤매핑 연산자를 양쪽에 동일하게 적용한 뒤 SDR끼리 비교한다.
- 어떤 도메인에서 비교했는지(`ComparisonDomain`)를 타입으로 강제해, 실수로 두 도메인을 섞을 수 없게 한다.

**탐지 방법**:
- Semantic: 비교 함수의 두 입력 중 하나만 톤매핑 함수를 거쳤는지 데이터 흐름(dataflow) 분석.
- Structural: 미리보기/썸네일 생성 모듈에서 만든 버퍼가 품질 비교 모듈로 그대로 전달되는 경로가 있는지 모듈 의존성 확인.
- Manual: 동일한 HDR 프레임을 자기 자신과 비교(reference == distorted)하되 한쪽만 톤매핑을 적용해, 0이어야 할 오차가 크게 나오는지 확인(회귀 테스트로 고정).

**예외**:
- 비교 목적이 정확히 "이 톤매핑 연산자가 원본 HDR 대비 얼마나 정보를 압축하는가"를 재는 것이라면(즉 톤매핑 자체가 평가 대상), HDR과 톤매핑된 SDR을 나란히 두고 차이를 보고하는 것이 목적에 부합한다 — 다만 이 경우도 결과를 "인코딩 손실"이 아니라 "톤매핑 손실"로 명확히 라벨링해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-019: alpha plane 처리 정책 없음
**분류**: 채널 처리 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_rgba_metric(reference: &RgbaFrame, distorted: &RgbaFrame) -> f64 {
    // alpha 채널의 존재/의미(premultiplied vs straight, 완전 투명 영역 처리)를 전혀 고려하지 않고
    // RGB만 비교하거나, 혹은 RGBA 4채널을 무차별 평균
    let rgb_mse = mean_squared_error_rgb(&reference.rgb(), &distorted.rgb());
    rgb_mse
}
```

**문제**:
- alpha=0(완전 투명)인 영역의 RGB 값은 대개 "의미 없는" 쓰레기 값(인코더/디코더가 최적화를 위해 임의로 채운 값)일 수 있는데, 이를 그대로 RGB 비교에 포함시키면 실제로 보이지 않는 영역의 차이가 지표를 오염시킨다.
- premultiplied alpha(RGB가 이미 alpha로 곱해진 상태)와 straight alpha(RGB와 alpha가 독립)를 구분하지 않고 비교하면, 두 스트림이 서로 다른 alpha 컨벤션을 쓸 때 RGB 값 자체가 다른 의미 체계에 있는 상태로 비교된다.
- alpha 채널 자체의 품질(예: 알파 매트의 경계 정확도)이 평가 대상인 콘텐츠(그린스크린 합성, UI 오버레이 등)에서 alpha를 아예 무시하면 가장 중요한 품질 차원을 놓친다.

**발생 조건**:
- 알파 채널을 지원하는 코덱 프로파일(VP9 profile with alpha, AV1 alpha 확장, ProRes 4444 등)로 인코딩된 콘텐츠를 비교할 때.
- 합성/그래픽 오버레이가 포함된 콘텐츠, 또는 알파 매트 품질 자체가 평가 목적인 워크플로.

**권장**:
```rust
struct AlphaAwareComparisonConfig {
    alpha_convention: AlphaConvention, // Premultiplied | Straight
    transparent_region_policy: TransparentRegionPolicy, // Exclude | IncludeWithWeight(f64) | CompareAlphaOnly
}

fn compute_rgba_metric(reference: &RgbaFrame, distorted: &RgbaFrame, config: &AlphaAwareComparisonConfig) -> RgbaQualityReport {
    let alpha_mse = mean_squared_error(&reference.alpha(), &distorted.alpha());

    let rgb_mse = match config.transparent_region_policy {
        TransparentRegionPolicy::Exclude => {
            // alpha=0인 픽셀은 RGB 비교 대상에서 제외
            mean_squared_error_masked(&reference.rgb(), &distorted.rgb(), &reference.alpha())
        }
        TransparentRegionPolicy::IncludeWithWeight(w) => {
            mean_squared_error_weighted_by_alpha(&reference.rgb(), &distorted.rgb(), &reference.alpha(), w)
        }
        TransparentRegionPolicy::CompareAlphaOnly => 0.0,
    };

    RgbaQualityReport { rgb_mse, alpha_mse, convention: config.alpha_convention }
}
```
- alpha 채널이 존재하는 콘텐츠 비교에는 반드시 명시적 정책(투명 영역 제외/가중치/알파 전용 비교)을 두고, 그 정책을 리포트에 남긴다.
- premultiplied/straight 컨벤션이 다를 수 있는 소스를 비교할 때는 비교 전에 하나의 컨벤션으로 통일한다.
- alpha 자체의 품질(예: 알파 MSE/SSIM)을 RGB와 별개의 지표로 리포트에 포함한다.

**탐지 방법**:
- Structural: RGBA 프레임을 다루는 비교 함수가 alpha 채널을 파라미터/필드로 아예 참조하지 않는지 확인.
- Semantic: 투명 영역(alpha=0) 마스킹 로직의 존재 여부를 코드에서 검색.
- Manual: 동일 RGB에 alpha만 다르게(완전 불투명 vs 부분 투명) 만든 테스트 프레임 쌍으로, 투명 영역의 쓰레기 RGB 값이 지표에 영향을 주는지 확인.

**예외**:
- 파이프라인이 다루는 콘텐츠가 전부 알파 없는 포맷(YUV 4:2:0 등)으로 제한되어 있다면 이 항목 자체가 적용되지 않는다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### COLOR-020: 4:2:0과 4:4:4를 단순 nearest resize로 맞춤
**분류**: 크로마 서브샘플링 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn align_chroma_resolution(u_plane: &Plane<u8>, target_width: usize, target_height: usize) -> Plane<u8> {
    // 4:2:0 크로마(1/2 해상도)를 4:4:4(원본 해상도)에 맞추기 위해 nearest-neighbor로 단순 확대
    let mut out = Plane::new(target_width, target_height);
    for y in 0..target_height {
        for x in 0..target_width {
            let src_x = x * u_plane.width / target_width;
            let src_y = y * u_plane.height / target_height;
            out.set(x, y, u_plane.get(src_x, src_y)); // 보간 없이 가장 가까운 샘플을 그대로 복제
        }
    }
    out
}
```

**문제**:
- nearest-neighbor 업샘플링은 블록 형태의 계단 아티팩트를 만든다. 4:2:0 레퍼런스와 4:4:4 distorted(또는 그 반대)를 비교하기 위해 한쪽을 이 방식으로 맞추면, 이 계단 아티팩트 자체가 "오차"로 측정되어 실제 인코딩 손실과 뒤섞인다.
- 크로마 서브샘플링 변환에는 이미 정해진 표준적인 필터(예: 4:2:0→4:4:4는 bilinear 또는 더 정교한 필터, 인코더들이 실제로 쓰는 방식과 일치하는 필터)가 있는데, nearest-neighbor는 그중 가장 저품질인 방식이다.
- 특히 크로마 서브샘플링이 다른 두 인코더 설정(예: 4:2:0 인코더 대 4:4:4 인코더)을 "공정하게" 비교하려는 목적이라면, 정렬 방법 자체의 아티팩트가 비교 결과의 공정성을 해친다 — 어느 인코더가 더 나은가가 아니라 "정렬 필터가 어느 포맷에 더 유리한가"를 재는 꼴이 된다.

**발생 조건**:
- 레퍼런스와 distorted가 서로 다른 chroma subsampling(4:2:0 vs 4:2:2 vs 4:4:4)으로 인코딩된 모든 비교 시나리오.
- 성능을 위해 "가장 빠른" 리샘플링 방법을 무심코 선택한 경우(nearest-neighbor가 구현이 가장 단순하고 빠르기 때문에 초기 프로토타입에서 흔히 선택됨).

**권장**:
```rust
fn align_chroma_resolution(u_plane: &Plane<u8>, target_width: usize, target_height: usize, filter: ResampleFilter) -> Plane<u8> {
    match filter {
        // 표준 인코더/디코더들이 실제로 쓰는 것과 동일한 계열의 필터를 사용
        ResampleFilter::Bilinear => bilinear_resample(u_plane, target_width, target_height),
        ResampleFilter::Lanczos3 => lanczos_resample(u_plane, target_width, target_height, 3),
        ResampleFilter::NearestNeighbor => nearest_resample(u_plane, target_width, target_height), // 명시적 opt-in만 허용
    }
}

fn align_for_comparison(reference: &Plane<u8>, distorted: &Plane<u8>) -> (Plane<u8>, Plane<u8>) {
    // 기본값은 항상 고품질 필터 — nearest는 호출부가 명시적으로 요청할 때만
    let common_size = choose_common_chroma_resolution(reference, distorted);
    (
        resample_to(reference, common_size, ResampleFilter::Lanczos3),
        resample_to(distorted, common_size, ResampleFilter::Lanczos3),
    )
}
```
- 크로마 해상도 정렬에는 기본값으로 bilinear 이상의 품질을 갖는 필터(가능하면 실제 코덱 표준 크로마 필터와 일치하는 것)를 사용하고, nearest-neighbor는 성능이 절대적으로 중요한 미리보기 용도로만 명시적 옵트인하게 한다.
- 어떤 필터로 정렬했는지 리포트 메타데이터에 남겨, 필터 선택이 결과에 미친 영향을 나중에 검증할 수 있게 한다.

**탐지 방법**:
- Semantic: 크로마 리샘플링 함수 이름/구현이 nearest-neighbor(단순 인덱스 매핑, 보간 가중치 계산 없음)인지 코드 검사.
- Static: `resample`/`upsample` 함수 내부에 가중 평균/보간 커널 계산이 없는지 grep.
- Manual: 동일한 4:4:4 소스를 4:2:0으로 다운샘플했다가 다시 4:4:4로 복원(nearest vs bilinear/lanczos)했을 때, 왕복 오차가 필터에 따라 크게 다른지 측정해 필터 선택의 영향력을 정량화.

**예외**:
- 실시간 미리보기처럼 지표 정확도보다 속도가 절대적으로 중요한 경로에서는 nearest-neighbor가 허용될 수 있다 — 다만 이 결과를 정식 품질 비교 리포트에 재사용해서는 안 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
