# Anti-Pattern Catalog — METRIC: 품질 지표 구현 PSNR/SSIM/VMAF (VQ-Probe domain)

이 문서는 더 큰 안티패턴 카탈로그의 일부입니다(전체 목록은 별도로 작성 중인 `docs/anti-patterns/INDEX.md` 참고). 본 파일은 VQ-Probe 도메인(듀얼 스트림 품질 비교 기능군, ALIGN/SPATIAL/COLOR/PIPE/HEAT/STAT와 함께 Bitstream-Analyzer 도메인과는 아키텍처상 분리되고 media-core 레이어만 공유) 1단계(일반 참조 카탈로그)이며, 2단계에서 Bitvue 저장소를 실제로 감사하여 각 항목의 "Bitvue 판정"을 채웁니다.

---

## PSNR

### METRIC-PSNR-001: MSE가 0일 때 division by zero
**분류**: 수치 정확성 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
fn psnr(mse: f64, max_val: f64) -> f64 {
    // 두 프레임이 완전히 동일하면 mse == 0.0
    10.0 * (max_val * max_val / mse).log10()
}
```

**문제**:
- `mse`가 정확히 0이면 `max_val * max_val / mse`는 `f64`에서 `inf`가 되고 `log10()`은 `inf`를 반환한다. Rust는 이 경우 패닉하지 않지만, 이후 이 값을 정수형으로 캐스팅하거나 UI에 그대로 표시하거나 통계(평균/최댓값) 집계에 섞으면 `inf`가 전체 결과를 오염시킨다.
- 인코딩 손실이 전혀 없는 구간(예: 무손실 코덱 비교, 동일 파일 self-compare 테스트)에서 흔히 발생하며, "완벽한 화질"을 나타내는 정상적인 케이스인데도 예외적으로 처리하지 않으면 그래프가 깨지거나 정렬/필터링 로직이 오작동한다.
- `inf`를 그대로 JSON 직렬화하면 표준 JSON에는 `Infinity` 표현이 없어 직렬화 실패 또는 클라이언트에서 `null`로 깨지는 문제가 생긴다.

**발생 조건**:
- 무손실 인코딩 비교, 동일 소스 self-test, 완전히 검은 화면(모든 픽셀 0) 같은 정적 구간이 반복되는 스트림.
- 프레임 단위 PSNR을 실시간 그래프에 스트리밍할 때 `inf` 값이 축 스케일을 깨뜨림.

**권장**:
```rust
const PSNR_MAX_DB: f64 = 100.0; // 관례적 상한(라이브러리마다 값은 다를 수 있음, 문서화 필수)

fn psnr(mse: f64, max_val: f64) -> f64 {
    if mse <= 0.0 {
        return PSNR_MAX_DB;
    }
    10.0 * (max_val * max_val / mse).log10()
}
```
- MSE가 0(또는 부동소수점 오차 이내로 0에 가까움)일 때의 반환값을 명시적 상수로 정의하고, 왜 이 상수를 택했는지(예: VMAF/x264 관례 100dB) 코드 주석과 문서에 남긴다.
- 직렬화 경계(JSON/IPC)에서 `inf`/`NaN`이 그대로 나가지 않도록 타입 레벨에서 방지한다.

**탐지 방법**:
- Runtime: 동일 프레임 self-compare 테스트 케이스를 회귀 테스트에 포함시켜 `inf`/패닉 여부를 확인.
- Static: `.log10()` 또는 `/ mse` 패턴을 grep하여 0 가드 존재 여부 확인.

**예외**:
- 내부 계산 파이프라인에서 `f64::INFINITY`를 특수값으로 명시적으로 취급하고 이후 모든 소비자(집계/직렬화/UI)가 이를 안전하게 처리하도록 타입으로 보장했다면 상수 클램프 대신 `INFINITY`를 유지해도 된다.

**Bitvue 판정**: Confirmed — mse==0.0 시 f64::INFINITY를 그대로 반환(crates/bitvue-metrics/src/simd.rs:765,870,967)하며 라이브러리 자체엔 클램프 없음. 라이브 소비자 둘 중: crates/bitvue-cli/src/commands/quality.rs:171-177(avg 계산)는 is_finite 필터가 없어 self-compare 프레임이 섞이면 avg가 그대로 inf로 오염됨(실사용 가능한 회귀); 반면 crates/bitvue-sidecar/src/debug_yuv.rs:549-559(compute_frame_metrics)는 `PSNR_INFINITY_SENTINEL=100.0`으로 명시적 clamp를 이미 구현해 이 안티패턴을 정확히 회피 — 두 소비자 중 하나만 고쳐진 상태 (이전 판정은 마이그레이션으로 삭제된 src-tauri/src/commands/quality.rs를 인용한 오판정이었음, 재조사로 교체)

---

### METRIC-PSNR-002: maximum pixel value를 항상 255로 고정
**분류**: 비트 심도 처리 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_psnr(mse: f64) -> f64 {
    // 10bit/12bit HDR 소스에서도 8bit 기준 255를 그대로 사용
    let max_val = 255.0;
    10.0 * (max_val * max_val / mse).log10()
}
```

**문제**:
- 10bit 소스의 실제 최댓값은 1023, 12bit는 4095인데 255를 고정하면 PSNR이 수 dB 단위로 체계적으로 낮게 계산된다(peak가 작을수록 분자가 작아져 PSNR이 부풀려지는 것이 아니라, MSE 계산에 쓰인 픽셀 스케일과 max_val 스케일이 불일치하면 값 자체가 의미 없어진다).
- HDR/10bit 콘텐츠가 늘어나는 상황에서 이 버그는 조용히 잘못된 결과를 만들어내며, 8bit 콘텐츠에서만 테스트하면 절대 드러나지 않는다.
- 동일 코드베이스에서 8bit·10bit 스트림을 같은 비교 세션에 섞어 넣으면 두 결과를 나란히 비교하는 것 자체가 무의미해진다.

**발생 조건**:
- 10bit HEVC/AV1/VVC 소스, 특히 HDR10/HLG 콘텐츠를 다룰 때.
- bit_depth를 프레임 메타데이터에서 읽지 않고 상수로 하드코딩한 초기 프로토타입 코드가 그대로 프로덕션에 남는 경우.

**권장**:
```rust
fn compute_psnr(mse: f64, bit_depth: u8) -> f64 {
    if mse <= 0.0 {
        return 100.0;
    }
    let max_val = ((1u32 << bit_depth) - 1) as f64;
    10.0 * (max_val * max_val / mse).log10()
}
```
- `bit_depth`를 디코더가 보고하는 실제 값에서 가져오고, plane별로 다를 수 있는 경우(거의 없지만 일부 코덱 확장)까지 고려한다.
- 유닛 테스트에 8/10/12bit 케이스를 모두 포함시킨다.

**탐지 방법**:
- Static: `255.0` 또는 `255` 리터럴이 PSNR 계산 함수 내부에 하드코딩되어 있는지 grep.
- Structural: `compute_psnr` 계열 함수 시그니처에 `bit_depth` 파라미터가 없는지 확인.

**예외**:
- 8bit 전용으로 명시적으로 스코프를 제한한 레거시 경로(예: 썸네일 프리뷰용 근사치)라면, 함수명에 `_8bit` 접미사를 붙이고 10bit 입력이 들어오면 패닉/에러로 방어하는 조건 하에 허용 가능.

**Bitvue 판정**: Confirmed(API 레벨) — crates/bitvue-metrics/src/lib.rs:167, simd.rs:769,873,970 전부 max_value=255.0 하드코딩, bit_depth 파라미터 자체가 없음. 단 재조사 결과 현재 두 라이브 호출부는 진입 전 이미 8bit로 정규화해서 넘김: sidecar debug_yuv.rs의 read_reference_frame(256,270)·decoded_to_planes8(328,337) 둘 다 `(sample >> downshift) as u8`로 올바르게 다운시프트; CLI quality.rs의 decoded_frame_to_luma(237)는 `c[0]`(LE 16bit의 low byte)만 취해 다운샘플 — 이건 하위 8비트만 남기는 별개의 변환 버그(이 카탈로그 항목 범위 밖)이지만 결과적으로 여기서도 8bit 값이 psnr()에 들어감. 즉 max_value=255 하드코딩 자체는 두 라이브 경로 모두에서 현재 활성 버그로 관측되진 않으나, bit_depth를 안 받는 API 설계는 여전히 취약(향후 raw 10/12bit 버퍼를 직접 넘기는 새 호출자가 생기면 즉시 재현). tests/video_quality_metrics_test.rs:332-340의 test_bit_depth_scaling은 실제 psnr()과 무관한 로컬 헬퍼만 검증하는 형식적 테스트 (이전 판정의 src-tauri 인용은 삭제된 파일이라 오판정, 교체)

---

### METRIC-PSNR-003: u16 subtraction에서 underflow
**분류**: 정수 연산 안전성 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
fn pixel_diff_sq(a: u16, b: u16) -> u32 {
    // a < b인 경우 (a - b)가 underflow
    let diff = a - b;
    (diff as u32) * (diff as u32)
}
```

**문제**:
- `u16 - u16`에서 `a < b`이면 debug 빌드에서는 즉시 패닉하고, release 빌드(`overflow-checks = false`가 기본)에서는 wrapping되어 거대한 양수로 조용히 둔갑한다. 결과적으로 릴리즈 빌드의 PSNR/MSE가 조용히 터무니없는 값을 낸다.
- diff는 원래 부호가 없는 절대값이어야 하는데, 어느 프레임이 "더 큰지"는 픽셀마다 다르므로 단순 뺄셈으로는 항상 절반의 케이스에서 문제가 생긴다.
- CI가 debug 모드로만 테스트를 돌리면 이 버그가 패닉으로 조기에 드러나 "버그 없음"으로 오판하기 쉽지만, 정작 사용자에게 배포되는 release 빌드에서만 wrapping이 발생해 재현이 어렵다.

**발생 조건**:
- 10/12bit 소스에서 노이즈나 그레인이 강한 영역, 또는 레퍼런스와 왜곡 프레임의 밝기 차이가 큰 영역.
- release 프로파일(`overflow-checks = false`)로 빌드된 배포 바이너리.

**권장**:
```rust
fn pixel_diff_sq(a: u16, b: u16) -> u32 {
    let diff = a.abs_diff(b); // u16, 항상 절대값, overflow 불가
    (diff as u32) * (diff as u32)
}
```
- `abs_diff`(Rust 1.60+ 표준 라이브러리)를 사용해 부호 없는 절대 차이를 안전하게 구한다.
- `Cargo.toml`의 `[profile.release]`에 `overflow-checks = true`를 켜서 CI가 release 빌드에서도 이런 버그를 패닉으로 잡아내게 한다(성능 민감 경로가 아니라면).

**탐지 방법**:
- Static: `clippy::arithmetic_side_effects` 또는 수동 grep으로 부호 없는 정수 타입 간 `-` 연산자 사용처를 확인.
- Runtime: release 프로파일에 `overflow-checks = true`를 임시로 켜고 퍼징/랜덤 프레임 쌍으로 회귀 테스트.

**예외**:
- 두 값의 대소관계가 호출 시점에 이미 불변식으로 보장된 경우(예: 항상 `a >= b`임이 타입/구조로 증명됨)라면 일반 뺄셈이 허용될 수 있으나, 이런 불변식은 주석과 `debug_assert!`로 명시해야 한다.

**Bitvue 판정**: N/A — psnr/ssim 전체가 &[u8] 8비트 버퍼만 다루며(u16 경로 자체가 없음), diff 계산은 i32/i16 signed 캐스팅 후 수행(simd.rs 예: 751, AVX2의 _mm256_sub_epi16 등)되어 unsigned subtraction underflow가 구조적으로 발생하지 않음

---

### METRIC-PSNR-004: 차이를 정수형에서 제곱해 overflow
**분류**: 정수 연산 안전성 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn sum_sq_error(diff: u16) -> u16 {
    // 12bit 최대 diff(4095)의 제곱은 16,769,025로 u16 범위(65535)를 훌쩍 초과
    diff * diff
}
```

**문제**:
- `u16 * u16`은 결과 타입도 `u16`이므로, 8bit에서조차 diff가 256 이상이면 즉시 overflow하고(8bit면 diff 최대 255이므로 실제로는 255*255=65025로 아슬아슬하게 안 넘지만), 10/12bit에서는 확실히 overflow한다.
- 이런 좁은 중간 타입에서의 overflow는 debug에서 패닉, release에서 wrapping이라는 동일한 이중성 문제를 다시 낳는다.
- 누적합(sum of squared error)까지 좁은 타입으로 진행하면 프레임 하나의 총합만으로도 `u32` 범위조차 넘길 수 있어(4K 프레임 기준 픽셀 수 830만 개 × 최대 제곱값), 누산기 타입 선택도 함께 검토해야 한다.

**발생 조건**:
- 10bit 이상 소스, 특히 극단적인 화질 열화(디코딩 오류, 블록 손상)로 diff가 매우 커지는 구간.
- 프레임 전체를 순회하며 `u32`/`u16` 누산기를 그대로 사용하는 루프.

**권장**:
```rust
fn sum_sq_error(diff: u16) -> u64 {
    let d = diff as u64;
    d * d
}

fn frame_sse(reference: &[u16], distorted: &[u16]) -> u64 {
    reference
        .iter()
        .zip(distorted)
        .map(|(&r, &d)| {
            let diff = r.abs_diff(d) as u64;
            diff * diff
        })
        .sum() // u64 누산기: 4K 프레임 기준으로도 충분한 여유
}
```
- 제곱 연산 직전에 충분히 넓은 타입(`u64` 또는 `f64`)으로 캐스팅한다.
- 누산기 타입도 프레임 크기와 최대 bit depth를 곱한 최악의 경우를 계산해 여유 있게 선택한다.

**탐지 방법**:
- Static: 좁은 정수 타입끼리의 곱셈(`u8 * u8`, `u16 * u16`)이 diff 제곱 계산에 쓰이는지 grep/clippy로 확인.
- Runtime: 12bit 최댓값 근처의 합성 테스트 프레임(레퍼런스 0, 왜곡 4095)으로 overflow 재현.

**예외**:
- SIMD 커널에서 의도적으로 좁은 타입을 쓰고 별도의 오버플로우 방지 로직(saturating 연산, 중간 스케일 조정)을 갖춘 경우는 성능상 허용될 수 있으나, 그 경우도 정확성 회귀 테스트가 반드시 있어야 한다.

**Bitvue 판정**: N/A — 제곱합은 i64(원격 remainder, simd.rs:749-753)·i32 SIMD lane 후 u64로 합산되며, 8bit 전용 스코프(diff 최대 255, diff²=65025)에서 overflow 여유가 충분함 — 권장 패턴과 일치

---

### METRIC-PSNR-005: Y/U/V 평균을 단순 산술평균
**분류**: 색공간 가중치 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn frame_psnr(y_psnr: f64, u_psnr: f64, v_psnr: f64) -> f64 {
    // Y/U/V를 동일 가중치로 단순 평균
    (y_psnr + u_psnr + v_psnr) / 3.0
}
```

**문제**:
- 대부분의 4:2:0 소스에서 Y plane은 전체 픽셀 수의 절반, U/V는 각각 1/8에 불과한데 동일 가중치로 평균하면 크로마 왜곡이 과대평가되고 휘도(가장 지각적으로 중요한 성분) 왜곡이 과소평가된다.
- 업계 표준(예: x264/x265 로그, FFmpeg `psnr` 필터)은 흔히 `(6*Y + U + V) / 8` 같은 가중 평균이나, YUV를 하나의 결합 MSE로 합쳐 계산하는 방식을 쓴다. 단순 평균으로 낸 값은 이런 표준 도구의 출력과 숫자가 달라져 "레퍼런스 툴과 비교했더니 우리 수치가 이상하다"는 혼란의 근원이 된다.
- U/V PSNR이 유난히 낮은 특정 색상 왜곡 케이스에서 전체 점수가 실제 체감 화질보다 과도하게 나쁘게 나올 수 있다.

**발생 조건**:
- 4:2:0/4:2:2 크로마 서브샘플링 소스에서 채널별 PSNR을 따로 계산한 뒤 통합 지표를 낼 때.
- 레퍼런스 인코더(x264/x265) 로그나 다른 분석 툴과 수치를 나란히 비교하는 리포트 기능.

**권장**:
```rust
fn frame_psnr_weighted(y_psnr: f64, u_psnr: f64, v_psnr: f64) -> f64 {
    // x264/x265 관례를 따른 가중 평균(4:2:0 기준). 실제 가중치는 채택할 표준을 문서에 명시.
    (6.0 * y_psnr + u_psnr + v_psnr) / 8.0
}

// 또는 dB를 평균하는 대신 결합 MSE로부터 직접 계산(더 엄밀한 방식)
fn frame_psnr_from_combined_mse(y_mse: f64, u_mse: f64, v_mse: f64,
                                  y_pixels: usize, u_pixels: usize, v_pixels: usize,
                                  max_val: f64) -> f64 {
    let total_pixels = (y_pixels + u_pixels + v_pixels) as f64;
    let combined_mse = (y_mse * y_pixels as f64 + u_mse * u_pixels as f64 + v_mse * v_pixels as f64)
        / total_pixels;
    if combined_mse <= 0.0 { return 100.0; }
    10.0 * (max_val * max_val / combined_mse).log10()
}
```
- 어떤 가중치/공식을 채택했는지(단순 평균 vs 6:1:1 가중 vs 픽셀 수 가중 결합 MSE) 문서와 UI 레이블에 명시한다.
- 레퍼런스 툴(FFmpeg, x264/x265)과 비교 검증 테스트를 CI에 둔다.

**탐지 방법**:
- Semantic: `(y + u + v) / 3.0` 패턴을 코드 리뷰에서 특별히 주의 깊게 검토.
- Runtime: FFmpeg `psnr` 필터 출력과 자체 구현 출력을 동일 소스에 대해 비교하는 회귀 테스트.

**예외**:
- 4:4:4 소스(크로마 서브샘플링 없음)이고 애초에 픽셀 수가 동일하다면, 단순 평균과 픽셀 가중 평균의 차이가 작아질 수 있다(그래도 완전히 같지는 않음 — MSE를 평균하는지 dB를 평균하는지에 따라 다름).

**Bitvue 판정**: N/A — 이전 판정(quality.rs:270의 단순 1/3평균)은 Electron 이전 삭제된 src-tauri 코드 기준 오판정. 현재 라이브 경로 재확인: crates/bitvue-sidecar/src/debug_yuv.rs:559 `psnr_avg = (6.0*psnr_y+psnr_u+psnr_v)/8.0` — 나쁜 예가 아닌 '권장' 절의 6:1:1 가중평균을 이미 구현(주석에 근거 명시); crates/bitvue-cli/src/commands/quality.rs는 애초에 psnr()을 Y-plane에만 호출(U/V 자체를 계산하지 않아 평균 문제가 발생할 지점이 없음). 두 라이브 소비자 모두 나쁜 예의 균등(1/3) 평균 패턴을 쓰지 않음 — 단 6:1:1 고정 가중치의 크로마 포맷 비의존성 문제는 METRIC-PSNR-006 참고

---

### METRIC-PSNR-006: plane 크기 차이를 가중치에 반영하지 않음
**분류**: 색공간 가중치 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn combine_planes(y_mse: f64, u_mse: f64, v_mse: f64) -> f64 {
    // U/V plane이 Y plane의 1/4 픽셀 수(4:2:0)라는 사실을 무시하고 동일 취급
    (y_mse + u_mse + v_mse) / 3.0
}
```

**문제**:
- METRIC-PSNR-005와 밀접하지만 더 근본적인 문제: MSE를 결합할 때 각 plane의 실제 픽셀 개수(가중치)를 전혀 고려하지 않으면, 4:2:0/4:2:2/4:4:4 등 서브샘플링 포맷이 바뀔 때마다 "동일한 가중치 공식"이 실제로는 다른 의미를 갖게 된다.
- 하드코딩된 6:1:1 같은 상수 가중치조차 크로마 포맷이 4:2:2나 4:4:4로 바뀌면 더 이상 픽셀 수 비율과 일치하지 않아 부정확해진다.
- 특히 4:4:4 소스를 4:2:0 전용으로 튜닝된 가중치로 계산하면 크로마 기여도가 실제보다 과소평가된다.

**발생 조건**:
- 여러 크로마 서브샘플링 포맷(4:2:0/4:2:2/4:4:4)을 함께 지원하는 비교 도구.
- 크로마 포맷이 다른 두 스트림을 나란히 비교하는 세션(레퍼런스는 4:4:4, 인코딩 결과는 4:2:0인 경우 등).

**권장**:
```rust
fn combine_planes(y_mse: f64, u_mse: f64, v_mse: f64,
                   y_px: usize, u_px: usize, v_px: usize) -> f64 {
    let total = (y_px + u_px + v_px) as f64;
    (y_mse * y_px as f64 + u_mse * u_px as f64 + v_mse * v_px as f64) / total
}
```
- 가중치를 상수로 하드코딩하지 말고 실제 plane의 픽셀 수(width × height, 서브샘플링 반영)에서 매 호출마다 계산한다.
- 크로마 포맷이 섞인 비교 세션에서는 어느 쪽 포맷 기준으로 정렬/리샘플링했는지도 별도로 기록한다(COLOR 카테고리와 연관).

**탐지 방법**:
- Structural: MSE 결합 함수가 plane 픽셀 수를 파라미터로 받는지 확인.
- Manual: 4:2:0과 4:4:4 두 케이스에 대해 결과가 픽셀 수 비율을 반영해 달라지는지 코드 리뷰에서 검증.

**예외**:
- 항상 고정된 단일 크로마 포맷(예: 4:2:0)만 지원하기로 명시적으로 스코프를 제한한 초기 버전이라면 상수 가중치를 임시로 허용할 수 있으나, 다른 포맷 입력 시 명확히 에러를 내야 한다.

**Bitvue 판정**: Confirmed — crates/bitvue-sidecar/src/debug_yuv.rs:559의 `psnr_avg = (6.0*psnr_y+psnr_u+psnr_v)/8.0`는 session.format(YuvFormat: I420/NV12/NV21/I422/I444, chroma_ratio는 (2,2)/(2,1)/(1,1)로 전부 다름, debug_yuv.rs:29-45)과 무관하게 항상 고정 6:1:1 상수를 사용 — I422/I444처럼 실제 플레인 픽셀비가 6:1:1과 크게 다른 포맷에서도 동일 가중치가 그대로 적용됨, 픽셀 수 기반 재계산 로직 없음 (이전 판정의 src-tauri 인용은 오판정이었으나, 재조사로 debug_yuv.rs에서 동일 패턴을 실제로 확인)

---

### METRIC-PSNR-007: padding pixel 포함
**분류**: 영역 경계 처리 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn frame_mse(reference: &Plane, distorted: &Plane) -> f64 {
    // stride(coded width)를 그대로 순회 — 실제 표시 영역 밖의 padding까지 포함
    let mut sum = 0.0f64;
    for y in 0..reference.stride_height {
        for x in 0..reference.stride_width {
            let diff = (reference.at(x, y) as f64) - (distorted.at(x, y) as f64);
            sum += diff * diff;
        }
    }
    sum / (reference.stride_width * reference.stride_height) as f64
}
```

**문제**:
- 코덱은 매크로블록/CTU 정렬을 위해 실제 디스플레이 해상도보다 큰 coded 영역(stride)에 패딩을 넣는 경우가 흔하다(예: 1920×1080이 아니라 1920×1088로 디코딩). 이 padding 영역은 인코더/디코더 구현에 따라 임의의 값(반복 경계값, 0, 쓰레기값)을 가질 수 있다.
- padding을 MSE 계산에 포함시키면, 두 디코더가 padding을 다르게 채우는 경우 실제 화질과 무관하게 PSNR이 흔들린다.
- stride와 display 크기가 같은 해상도(예: 1920×1080이 8 정렬과 우연히 맞는 경우)에서는 버그가 드러나지 않다가, 1080p처럼 정렬이 안 맞는 흔한 해상도에서 갑자기 수치가 이상해지는 식으로 나타난다.

**발생 조건**:
- 8 또는 64 정렬 단위로 패딩되는 HEVC/AV1/VVC의 non-정렬 해상도(1080, 1088 배수가 아닌 높이 등).
- 디코더가 stride 버퍼를 그대로 노출하고, 별도의 crop 단계를 거치지 않고 분석 코드에 넘기는 아키텍처.

**권장**:
```rust
fn frame_mse(reference: &Plane, distorted: &Plane) -> f64 {
    let width = reference.display_width;   // stride_width가 아닌 실제 표시 폭
    let height = reference.display_height;
    let mut sum = 0.0f64;
    for y in 0..height {
        for x in 0..width {
            let diff = (reference.at(x, y) as f64) - (distorted.at(x, y) as f64);
            sum += diff * diff;
        }
    }
    sum / (width * height) as f64
}
```
- plane 순회는 항상 `display_width`/`display_height`(crop 후 크기)를 사용하고, `stride`는 메모리 레이아웃(row 간격)에만 사용한다.
- 두 값이 다른 케이스를 유닛 테스트에 반드시 포함시킨다(예: 1080×1920처럼 8 정렬이 안 맞는 해상도).

**탐지 방법**:
- Structural: MSE/SSE 계산 루프의 반복 범위가 `stride_*`인지 `display_*`(또는 `cropped_*`)인지 확인.
- Runtime: 높이가 8의 배수가 아닌 합성 테스트 소스(예: 1921×1081)로 결과가 안정적인지 검증.

**예외**:
- coded 크기와 display 크기가 항상 동일함이 컨테이너/코덱 레벨에서 보장되는 특수한 내부 포맷이라면 구분이 불필요할 수 있으나, 이런 가정은 매우 취약하므로 권장하지 않는다.

**Bitvue 판정**: N/A — crates/bitvue-decode/src/plane_utils.rs:293-302 extract_y_plane가 PlaneConfig(width,height,stride)로 stride 제거 후 정확히 width*height 크기 버퍼를 반환 — psnr()에 padding이 포함될 수 없는 구조

---

### METRIC-PSNR-008: crop 영역 불일치
**분류**: 영역 경계 처리 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compare_frames(reference: &DecodedFrame, distorted: &DecodedFrame) -> f64 {
    // 두 프레임의 crop 정보(conformance window 등)가 다를 수 있다는 걸 무시하고
    // 각자의 display 크기만 보고 (0,0)부터 min(width, height)만큼 비교
    let w = reference.display_width.min(distorted.display_width);
    let h = reference.display_height.min(distorted.display_height);
    frame_mse_region(reference, distorted, 0, 0, w, h)
}
```

**문제**:
- 레퍼런스와 왜곡 스트림의 conformance window(crop offset)가 다르면(예: 인코더가 좌우로 약간 다르게 크롭했거나, 리사이징/패딩 방식이 다른 경우) 원점을 그냥 (0,0)으로 맞추는 것은 실제로 서로 다른 콘텐츠 영역을 비교하는 것이 된다.
- crop offset이 몇 픽셀만 어긋나도 고주파 디테일이 많은 영상에서는 PSNR이 크게 흔들릴 수 있어, "인코더 품질이 나쁘다"는 잘못된 결론으로 이어질 수 있다.
- min(width, height)로만 맞추는 방식은 crop 시작 오프셋이 다른 경우를 전혀 감지하지 못하며 조용히 잘못된 정렬로 계산을 진행한다.

**발생 조건**:
- 서로 다른 인코더/툴체인으로 생성된 두 스트림을 비교할 때 (예: 레퍼런스 인코더 vs 하드웨어 인코더).
- 소스 필터링(letterbox 제거, 자동 crop) 도구를 거친 스트림과 원본을 비교할 때.

**권장**:
```rust
fn compare_frames(reference: &DecodedFrame, distorted: &DecodedFrame) -> Result<f64, MetricError> {
    if reference.crop_rect() != distorted.crop_rect() {
        // 자동으로 임의 정렬하지 말고, 명시적으로 실패하거나 사용자에게 경고
        return Err(MetricError::CropMismatch {
            reference: reference.crop_rect(),
            distorted: distorted.crop_rect(),
        });
    }
    let rect = reference.crop_rect();
    Ok(frame_mse_region(reference, distorted, rect.x, rect.y, rect.width, rect.height))
}
```
- crop 정보(conformance window / display rect)를 두 스트림에서 각각 명시적으로 비교하고, 불일치 시 자동으로 얼버무리지 말고 에러/경고로 사용자에게 알린다(ALIGN 카테고리와 접점).
- 의도적으로 서로 다른 crop을 비교해야 하는 경우(예: letterbox 제거 후 비교) UI에서 명시적으로 선택하게 한다.

**탐지 방법**:
- Semantic: 두 프레임 비교 함수가 각 프레임의 crop 오프셋을 개별적으로 조회하고 비교하는지 코드 리뷰.
- Runtime: 서로 다른 conformance window를 가진 합성 스트림 쌍으로 에러/경고가 발생하는지 테스트.

**예외**:
- 애플리케이션이 애초에 crop 정렬을 상위 레이어(ALIGN 카테고리의 정렬 파이프라인)에서 보장하고, 이 함수에 들어올 때는 이미 동일 crop_rect임이 타입/계약으로 보증된다면 내부 검증을 생략할 수 있다(단, 이 경우도 `debug_assert!`는 남기는 것이 안전).

**Bitvue 판정**: N/A — 이전 판정은 삭제된 src-tauri 코드 인용(오판정), 재조사 결과 현재 두 라이브 경로 모두 이 시나리오 자체가 구조적으로 성립하지 않음. CLI quality.rs:85-91은 디코더가 보고하는 display width/height(conformance window 적용 후 값)만 비교해 불일치 시 skip; sidecar debug_yuv.rs의 compute_frame_metrics(514-518)도 동일하게 하드 에러(자동 정렬 없음, min(w,h)로 얼버무리지 않음). 다만 이 두 경로 모두 "압축 스트림 각자의 conformance window"를 비교하는 게 아니라, debug_yuv는 사용자가 지정한 단일 Crop을 참조/디코드 양쪽에 동일하게 적용하는 구조(session.crop, debug_yuv.rs:8)라 애초에 "서로 다른 두 conformance window"라는 개념 자체가 없음 — 나쁜 예가 가정하는 아키텍처(양쪽이 독립적 crop 메타데이터를 가짐)가 이 코드베이스에 존재하지 않음

---

### METRIC-PSNR-009: frame PSNR의 평균을 sequence PSNR로 사용
**분류**: 집계 방식 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn sequence_psnr(frame_psnrs: &[f64]) -> f64 {
    // 프레임별 PSNR(dB)을 그냥 산술 평균
    frame_psnrs.iter().sum::<f64>() / frame_psnrs.len() as f64
}
```

**문제**:
- 이 값은 흔히 "average PSNR" 또는 "PSNR average"라고 불리며, 각 프레임의 dB 값을 그대로 산술 평균한 것이다. 이는 "global PSNR"(전체 시퀀스의 모든 픽셀 오차 제곱합을 하나의 MSE로 합친 뒤 한 번에 dB로 변환한 값)과 **수학적으로 다른 값**이다.
- dB는 로그 스케일이므로 dB의 평균은 MSE의 평균(또는 총합)의 로그와 같지 않다. 즉 `mean(10*log10(a)) != 10*log10(mean(a))`이며, 이 두 지표는 각기 다른 것을 측정한다: "average PSNR"은 프레임 하나하나를 동등한 발언권으로 평균 내는 것이고("나쁜 프레임 하나가 전체를 크게 끌어내리지 않음"), "global PSNR"(때로 "overall PSNR"이라 불림)은 전체 오차 에너지를 하나의 값으로 합쳐 극단적으로 나쁜 프레임의 영향을 더 직접적으로 반영한다.
- 두 지표를 구분하지 않고 그냥 "PSNR"이라는 이름만 붙여 리포트에 내보내면, 다른 툴(예: FFmpeg는 기본적으로 average와 global을 모두 출력)과 비교할 때 사용자가 어느 쪽과 비교하고 있는지 알 수 없어 혼란을 겪는다.
- 특히 매우 나쁜 프레임(예: 디코딩 오류, 심한 프레임 드롭 은닉) 몇 개가 섞여 있을 때 두 값의 차이가 커지므로, 이 차이 자체가 "화질이 불균일하다"는 유의미한 신호인데 하나만 계산하면 이 신호를 잃는다.

**발생 조건**:
- 다수 프레임에 걸친 시퀀스 요약 통계를 낼 때 항상 해당.
- 프레임 품질 편차가 큰 콘텐츠(장면 전환, 일부 프레임 손상, VBR 인코딩의 품질 요동)에서 두 값의 차이가 특히 커진다.

**권장**:
```rust
struct SequencePsnr {
    average_db: f64, // 프레임별 dB의 산술 평균 ("average PSNR" / "PSNR average")
    global_db: f64,  // 전체 프레임의 총 SSE를 하나의 MSE로 합쳐 계산한 값 ("global PSNR" / "overall PSNR")
}

fn sequence_psnr(frame_mse: &[f64], frame_pixel_counts: &[usize], max_val: f64) -> SequencePsnr {
    let frame_psnrs: Vec<f64> = frame_mse.iter()
        .map(|&mse| if mse <= 0.0 { 100.0 } else { 10.0 * (max_val * max_val / mse).log10() })
        .collect();
    let average_db = frame_psnrs.iter().sum::<f64>() / frame_psnrs.len() as f64;

    let total_sse: f64 = frame_mse.iter().zip(frame_pixel_counts)
        .map(|(&mse, &n)| mse * n as f64)
        .sum();
    let total_pixels: usize = frame_pixel_counts.iter().sum();
    let global_mse = total_sse / total_pixels as f64;
    let global_db = if global_mse <= 0.0 { 100.0 } else { 10.0 * (max_val * max_val / global_mse).log10() };

    SequencePsnr { average_db, global_db }
}
```
- 두 값을 모두 계산하고 UI/리포트/API에서 `average_psnr_db`와 `global_psnr_db`처럼 명확히 다른 이름으로 노출한다.
- 문서(툴팁, API 스키마)에 두 값의 정의 차이를 명시해 사용자가 어떤 값을 인용하고 있는지 헷갈리지 않게 한다.

**탐지 방법**:
- Semantic: 코드 리뷰에서 "sequence PSNR" 또는 "overall PSNR"이라는 이름의 함수가 실제로는 dB 평균만 계산하는지, 아니면 두 정의를 모두 구현했는지 확인.
- Manual: FFmpeg `-psnr` 출력(average와 global을 모두 로그에 남김)과 자체 구현을 나란히 비교.

**예외**:
- 애플리케이션이 "average PSNR"만을 공식 지표로 채택하기로 명시적으로 결정하고 그 사실을 UI/문서에 일관되게 표기했다면 global PSNR을 생략해도 된다. 단, 이때도 내부 이름과 API 필드명에 반드시 `average`를 명시해 나중에 global과 혼동되지 않게 해야 한다.

**Bitvue 판정**: Confirmed — crates/bitvue-cli/src/commands/quality.rs:174-177 `let avg = psnr_vals.iter().sum::<f64>() / psnr_vals.len() as f64;`(프레임별 dB 단순 평균, "PSNR avg=...dB"로 출력)만 존재하고 global(전체 SSE 합산 후 한 번에 dB 변환) PSNR 계산 경로는 코드베이스 어디에도 없음 — 이 CLI 출력이 사실상 유일한 "sequence PSNR" 개념(이전 판정의 src-tauri 인용은 삭제된 파일이라 오판정이었으나 결론 자체는 재확인됨, 새 위치로 교체)

---

### METRIC-PSNR-010: sequence 전체 MSE와 frame PSNR 평균 혼동
**분류**: 집계 방식 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
struct SequenceReport {
    psnr_db: f64, // 이름만 보고는 average인지 global인지 알 수 없음
}

fn build_report(frame_psnrs: &[f64], frame_mse: &[f64]) -> SequenceReport {
    // 내부에서 두 계산을 상황에 따라 섞어 쓰다가, 한쪽 코드 경로에서는
    // average를 계산하고 다른 경로(예: 캐시 히트 시)에서는 global 값을 재사용
    let psnr_db = if let Some(cached) = get_cached_global_psnr() {
        cached // global 계산 결과
    } else {
        frame_psnrs.iter().sum::<f64>() / frame_psnrs.len() as f64 // average 계산 결과
    };
    SequenceReport { psnr_db }
}
```

**문제**:
- METRIC-PSNR-009가 "두 정의가 다르다"는 것을 다루는 항목이라면, 이 항목은 그 다음 단계의 실패 모드다: 코드베이스 안에서 두 계산 경로(프레임별 dB 평균 vs 전체 MSE 합산 후 dB 변환)가 **같은 필드/변수 이름으로 뒤섞여** 호출 조건(캐시 여부, 코드 경로, 리팩터링 시점)에 따라 다른 정의의 값이 나오는 것이다.
- 이런 혼동은 특히 리팩터링 중, 또는 성능 최적화를 위해 한쪽 계산 경로만 캐싱을 추가했다가 다른 경로와 의미가 어긋나는 형태로 흔히 발생한다.
- 결과적으로 동일한 스트림을 두 번 분석했는데 실행 경로(캐시 히트/미스, 병렬 처리 순서 등)에 따라 "PSNR"이라는 이름의 값이 미세하게 또는 크게 달라지는, 재현이 매우 어려운 버그가 된다.
- 회귀 테스트가 "PSNR 값이 이전 실행과 같은지"만 확인하고 "이 값이 average인지 global인지 정의를 검증"하지 않으면 이런 버그가 오래도록 숨어 있을 수 있다.

**발생 조건**:
- 캐시/증분 계산 경로와 풀 재계산 경로가 별도로 구현되어 있고 리팩터링 이력이 긴 코드베이스.
- 여러 개발자가 "PSNR"이라는 이름만 보고 정의를 재확인하지 않은 채 서로 다른 계산 로직을 이어 붙인 경우.

**권장**:
```rust
// 타입 자체에 정의를 새겨 넣어 이름 혼동을 원천 차단
#[derive(Clone, Copy, Debug)]
struct AveragePsnrDb(f64);
#[derive(Clone, Copy, Debug)]
struct GlobalPsnrDb(f64);

struct SequenceReport {
    average_psnr: AveragePsnrDb,
    global_psnr: GlobalPsnrDb,
}

fn build_report(frame_mse: &[f64], frame_pixel_counts: &[usize], max_val: f64) -> SequenceReport {
    // 캐시를 쓰더라도 캐시 키/값의 타입이 AveragePsnrDb/GlobalPsnrDb로 고정되어
    // 서로 다른 정의가 같은 필드에 섞여 들어갈 수 없다.
    SequenceReport {
        average_psnr: compute_average(frame_mse, max_val),
        global_psnr: compute_global(frame_mse, frame_pixel_counts, max_val),
    }
}
```
- 뉴타입(newtype) 패턴으로 두 정의를 타입 레벨에서 분리해, 컴파일러가 실수로 하나를 다른 곳에 대입하는 것을 막게 한다.
- 캐싱/증분 계산 경로를 추가할 때마다 "이 값이 어떤 정의를 캐싱하는지" 캐시 키에 명시한다.
- 회귀 테스트에 "동일 입력, 캐시 히트 vs 미스"를 모두 실행해 두 결과가 (같은 정의 기준으로) 일치하는지 검증하는 케이스를 추가한다.

**탐지 방법**:
- Semantic: `psnr_db`, `psnr` 같은 모호한 이름의 필드/변수가 여러 계산 경로에서 서로 다른 정의로 채워지는지 데이터 흐름 추적.
- Runtime: 캐시를 강제로 무효화한 실행과 캐시 히트 실행의 결과를 diff하는 회귀 테스트.

**예외**:
- 코드베이스 전체에서 오직 하나의 정의(예: average만)만 존재하도록 아키텍처 차원에서 강제하고 있다면(즉 global PSNR 계산 코드 자체가 존재하지 않는다면) 이런 혼동은 구조적으로 발생할 수 없다.

**Bitvue 판정**: N/A — average/global 두 정의가 애초에 공존하지 않고(PSNR-009 참조), CLI quality.rs::run()과 sidecar compute_frame_metrics 둘 다 캐시 없이 매 호출 전체 재계산(디코드부터 재수행)해 두 정의가 뒤섞일 코드 경로가 구조적으로 존재하지 않음 (이전 판정의 "calculate_quality_metrics" 인용은 삭제된 src-tauri 함수명이라 오판정, 결론은 재확인)

---

## SSIM / MS-SSIM

### METRIC-SSIM-001: window 경계 처리 불명확
**분류**: 알고리즘 정확성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn ssim_map(reference: &Plane, distorted: &Plane, window: usize) -> Vec<f64> {
    let half = window / 2;
    let mut map = Vec::new();
    // 경계 처리 방식이 코드만 봐서는 불명확: 클리핑? 제로 패딩? mirror?
    for y in half..(reference.height - half) {
        for x in half..(reference.width - half) {
            map.push(ssim_at(reference, distorted, x, y, window));
        }
    }
    map
}
```

**문제**:
- 경계에서 윈도우가 이미지 밖으로 나가는 픽셀들(가장자리 `half` 픽셀 폭)을 아예 건너뛰는 이 구현은 출력 SSIM map의 크기가 입력보다 작아지고, 그 결과 "전체 평균 SSIM"에서 경계 영역이 통째로 제외된다.
- 레퍼런스 구현(예: 원 논문의 MATLAB 코드, libvmaf 내부 SSIM)마다 경계를 제로 패딩, mirror(반사) 패딩, 클리핑(윈도우를 이미지 안으로 줄임) 등 다르게 처리하며, 이는 SSIM map의 크기와 경계 근처 값 모두에 영향을 준다.
- 어떤 정책을 썼는지 코드에 문서화되어 있지 않으면, 나중에 리팩터링하다가 실수로 정책이 바뀌어도 아무도 알아채지 못하고, 레퍼런스 툴과 비교 검증할 때도 원인을 특정하기 어렵다.
- 경계를 완전히 제외하는 방식은 특히 작은 해상도(썸네일, ROI 크롭)에서 유효 픽셀 비율을 크게 줄여 전체 평균에 왜곡을 준다.

**발생 조건**:
- 윈도우 크기가 크고(예: 11×11 Gaussian) 이미지/ROI 크기가 작을 때 경계 손실 비율이 커진다.
- 다른 SSIM 구현(FFmpeg, libvmaf, scikit-image)과 결과를 대조 검증할 때 경계 정책 차이가 전체 평균에 눈에 띄는 영향을 준다.

**권장**:
```rust
enum BorderPolicy {
    Exclude,       // 경계 윈도우는 계산에서 제외(현재 흔한 관례, VMAF의 float_ssim이 근접)
    ReplicatePad,  // 경계 픽셀을 반복해 윈도우를 채움
    MirrorPad,     // 이미지 경계를 기준으로 반사
}

fn ssim_map(reference: &Plane, distorted: &Plane, window: usize, policy: BorderPolicy) -> SsimMap {
    // 정책을 명시적 파라미터로 받고, 반환 타입에 사용된 정책과 유효 픽셀 수를 함께 기록
    match policy {
        BorderPolicy::Exclude => ssim_map_exclude_border(reference, distorted, window),
        BorderPolicy::ReplicatePad => ssim_map_replicate(reference, distorted, window),
        BorderPolicy::MirrorPad => ssim_map_mirror(reference, distorted, window),
    }
}
```
- 경계 정책을 타입/파라미터로 명시하고, 채택한 정책과 그 근거(어떤 레퍼런스 구현을 따랐는지)를 코드 주석과 문서에 남긴다.
- SSIM map의 크기가 입력과 같은지 다른지, 다르다면 얼마나 작은지를 반환 타입/메타데이터에 명시한다.

**탐지 방법**:
- Semantic: 경계 처리 로직이 함수 어딘가에 있는지, 있다면 그 정책이 문서화되어 있는지 코드 리뷰.
- Manual: 레퍼런스 구현(예: libvmaf, scikit-image `structural_similarity`)과 동일 입력에 대해 SSIM map 크기와 평균값을 비교.

**예외**:
- 윈도우가 이미지 대비 매우 작고(예: 8×8 윈도우, 4K 이미지) 경계 제외로 인한 오차가 무시할 수준(<0.01%)임을 실측으로 확인했다면, 단순 제외 정책을 성능상 이유로 채택해도 된다. 단, 이 경우도 정책은 명시해야 한다.

**Bitvue 판정**: Suspected — lib.rs의 ssim()은 8x8 비중첩 블록 타일링(step_by 8, lib.rs:175-179)이며 경계는 win_width=min(8,width-x)로 축소된 부분윈도우를 포함(스킵하지 않음)해 '나쁜 예'와 정확히 일치하진 않지만, 이 경계정책이 named policy/문서화 없이 암묵적으로 결정되어 있어 유사한 리스크가 있음

---

### METRIC-SSIM-002: sample variance와 population variance 혼동
**분류**: 알고리즘 정확성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn variance(samples: &[f64], mean: f64) -> f64 {
    // n으로 나누는지 n-1로 나누는지 명시 없이 하나를 고정
    let n = samples.len() as f64;
    samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0) // sample variance(불편 추정량)
}
```

**문제**:
- SSIM 공식의 표준 정의(Wang et al. 2004)는 Gaussian 가중 window 내에서의 **모집단(population) 분산/공분산**(가중치 합으로 나누는, 편향 보정 없는 버전)을 사용하는데, 구현에서 실수로 `n-1`로 나누는 불편 추정량(sample variance)을 쓰면 윈도우 크기가 작을수록(특히 8×8 이하) 값이 눈에 띄게 달라진다.
- 이 차이는 윈도우가 작을수록 커지므로(`n/(n-1)` 비율), 윈도우 크기를 8×8에서 11×11로 바꾸는 것만으로도 이 버그의 영향이 달라져 "윈도우 크기를 바꿨더니 결과가 이상하게 튄다"는 디버깅하기 어려운 증상으로 나타난다.
- Gaussian 가중 윈도우를 쓰는 경우 "표본 개수 n"의 정의 자체가 모호해지므로(가중치의 합을 n으로 볼지, 0이 아닌 가중치 개수를 n으로 볼지), 이 문제는 균일 윈도우보다 Gaussian 윈도우에서 더 미묘하게 틀리기 쉽다.

**발생 조건**:
- SSIM을 처음부터 직접 구현할 때(통계 교과서의 "불편 분산 추정량은 n-1로 나눈다"는 습관을 그대로 가져오는 경우).
- 작은 윈도우 크기(예: 8×8)를 사용하는 경량 구현.

**권장**:
```rust
fn weighted_variance(samples: &[f64], weights: &[f64], weighted_mean: f64) -> f64 {
    // SSIM 표준 정의를 따라 가중치 합(population 방식)으로 나눈다. n-1 보정을 하지 않는다.
    let weight_sum: f64 = weights.iter().sum();
    let weighted_sq_diff: f64 = samples.iter().zip(weights)
        .map(|(&x, &w)| w * (x - weighted_mean).powi(2))
        .sum();
    weighted_sq_diff / weight_sum
}
```
- SSIM 계산에서는 항상 population 방식(가중치 합/윈도우 픽셀 수로 나눔, `n-1` 보정 없음)을 쓰고, 이를 코드 주석에 "SSIM 표준 정의를 따름"이라고 명시한다.
- 통계 라이브러리의 범용 `variance()` 함수를 그대로 재사용하지 않는다 — 대부분의 범용 통계 라이브러리는 기본값이 sample variance(`n-1`)인 경우가 많다.

**탐지 방법**:
- Semantic: 분산/공분산 계산의 분모가 `n`인지 `n-1`인지, 그리고 SSIM 논문 공식과 일치하는지 코드 리뷰.
- Manual: 작은 윈도우(4×4, 8×8)로 레퍼런스 구현과 수치를 직접 비교해 미세한 차이를 검출.

**예외**:
- 없음 — SSIM 정의 자체가 population variance를 요구하므로, 다른 선택을 정당화할 도메인상의 이유는 없다. (다른 통계 목적의 분산 계산과 코드를 공유하다가 실수로 섞이지 않도록 별도 함수로 분리하는 것을 권장.)

**Bitvue 판정**: N/A — lib.rs:227-229 var_x=(sum_xx/n)-mean_x², cov_xy도 동일하게 n(population)으로 나눔 — n-1 보정 없이 이미 올바르게 population variance 사용

---

### METRIC-SSIM-003: Gaussian kernel 정규화 오류
**분류**: 알고리즘 정확성 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn gaussian_kernel(size: usize, sigma: f64) -> Vec<f64> {
    let half = (size / 2) as f64;
    (0..size)
        .map(|i| {
            let x = i as f64 - half;
            (-x * x / (2.0 * sigma * sigma)).exp()
        })
        .collect()
    // 정규화(합이 1이 되도록 나누기)를 빠뜨림
}
```

**문제**:
- 정규화하지 않은 Gaussian 커널을 가중 평균/가중 분산 계산에 그대로 쓰면, 커널 계수의 합이 1이 아니므로 가중 평균이 실제 평균보다 체계적으로 크거나 작게 나오고, 이는 luminance/contrast/structure 각 성분 계산 전체에 스케일 오차를 전파시킨다.
- 1D 커널을 만들고 2D로 외적(outer product)하는 구현에서는 1D 커널 각각을 정규화하지 않으면 2D 커널의 합이 1이 아니게 되는데, 1D 커널 각각이 개별적으로는 "거의 1처럼 보이는" 근사값을 가져 버그가 눈에 잘 띄지 않는다.
- 정규화를 아예 빠뜨리는 것 외에도, `size`가 작을 때(예: 5×5) tail을 충분히 포함하지 못해 시그마 대비 커널 크기가 부적절하면 정규화를 하더라도 레퍼런스 구현과 값이 어긋난다.

**발생 조건**:
- SSIM을 직접 구현하며 Gaussian 커널 생성 코드를 새로 작성할 때(라이브러리 함수를 재사용하지 않는 경우).
- 커널을 1D로 만들어 재사용하는 최적화 구현(분리 가능한 컨볼루션)에서 1D 커널 정규화를 빠뜨리는 경우.

**권장**:
```rust
fn gaussian_kernel(size: usize, sigma: f64) -> Vec<f64> {
    let half = (size / 2) as f64;
    let raw: Vec<f64> = (0..size)
        .map(|i| {
            let x = i as f64 - half;
            (-x * x / (2.0 * sigma * sigma)).exp()
        })
        .collect();
    let sum: f64 = raw.iter().sum();
    raw.into_iter().map(|v| v / sum).collect() // 합이 정확히 1이 되도록 정규화
}
```
- 커널 생성 직후 합이 1인지 확인하는 `debug_assert!((sum - 1.0).abs() < 1e-9)`를 추가한다.
- SSIM 표준 파라미터(11×11, sigma=1.5)를 기본값으로 두고, 이를 변경할 때는 레퍼런스 구현과의 수치 비교 테스트를 함께 갱신한다.

**탐지 방법**:
- Static: 커널 생성 함수에 정규화(합으로 나누기) 단계가 존재하는지 확인.
- Runtime: 생성된 커널의 합이 1.0(±1e-9)인지 검증하는 유닛 테스트.

**예외**:
- 없음 — 정규화되지 않은 커널을 의도적으로 쓸 이유는 SSIM 맥락에서는 없다.

**Bitvue 판정**: N/A — 코드베이스에 Gaussian 커널 생성 함수 자체가 없음(8x8 uniform block 평균만 사용) — 정규화 대상인 Gaussian 가중치 로직이 존재하지 않음

---

### METRIC-SSIM-004: integer accumulation overflow
**분류**: 정수 연산 안전성 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn windowed_sum_sq(window: &[u8]) -> u32 {
    // u8 픽셀 값(최대 255)의 제곱합을 누적하지만, 큰 윈도우에서 u32도 위험할 수 있고
    // 중간 단계에서 좁은 타입을 거치면 더 쉽게 문제
    window.iter().map(|&p| (p as u16).pow(2) as u32).sum()
}
```

**문제**:
- SSIM 계산은 luminance(평균), contrast(분산), structure(공분산) 각각에서 픽셀 값의 곱과 합을 반복적으로 구하는데, 이 중간 누적값들을 정수형으로 유지하면서 bit depth가 올라가거나 윈도우가 커지면 overflow 위험이 커진다.
- 특히 12bit(최대 4095) 픽셀의 곱(최대 약 1,677만)을 큰 윈도우(예: 11×11=121개)에 걸쳐 누적하면 `u32` 범위(약 43억)에 근접하거나, 다른 중간 연산(공분산의 교차곱 등)과 결합하면 쉽게 넘칠 수 있다.
- PSNR 계열 항목(METRIC-PSNR-003/004)과 동일한 근본 원인(좁은 정수 타입의 무음 wrapping)이지만, SSIM은 여러 통계량(평균/분산/공분산)을 동시에 누적하는 구조라 overflow 지점이 더 많고 발견하기 어렵다.

**발생 조건**:
- 정수 픽셀 데이터를 부동소수점으로 변환하지 않고 정수 연산으로 SSIM을 구현하는 성능 최적화 경로(SIMD 정수 커널 포함).
- 10/12bit 고비트심도 소스, 큰 윈도우 크기.

**권장**:
```rust
fn windowed_sum_sq(window: &[u16], bit_depth: u8) -> u64 {
    // 부동소수점으로 조기에 전환하거나, 정수 유지 시 충분히 넓은 타입(u64) 사용
    window.iter().map(|&p| (p as u64) * (p as u64)).sum()
}

// 또는 정확성이 최우선인 경로에서는 애초에 f64로 계산
fn windowed_sum_sq_f64(window: &[u16]) -> f64 {
    window.iter().map(|&p| (p as f64) * (p as f64)).sum()
}
```
- 정수 누산기를 쓸 경우 최악의 경우(최대 bit depth × 최대 윈도우 크기)를 계산해 충분히 넓은 타입(`u64` 이상)을 선택한다.
- 정확성이 성능보다 우선인 경로(레퍼런스 계산, 회귀 테스트 기준값 생성)에서는 처음부터 `f64`로 계산해 정수 overflow 클래스 전체를 제거한다.
- SIMD 정수 커널을 쓰는 고성능 경로는 별도로 overflow 안전성을 명시적으로 증명하고 문서화한다(METRIC-SSIM-009와 연관).

**탐지 방법**:
- Static: SSIM 통계량 누적에 쓰이는 정수 타입의 폭이 최대 bit depth × 윈도우 크기 최악값을 감당하는지 계산으로 확인.
- Runtime: release 프로파일에서 `overflow-checks = true`로 12bit 최댓값 근처 합성 데이터 테스트.

**예외**:
- 8bit 전용, 작은 윈도우(예: 4×4)로 스코프가 명확히 제한되어 있고 최악의 경우 값이 타입 범위 내에 안전하게 들어옴을 계산으로 증명했다면 좁은 타입을 유지해도 된다.

**Bitvue 판정**: N/A — compute_window_stats_simd(simd.rs)는 sum_xx/sum_yy/sum_xy를 u64로 누적(scalar: simd.rs:81-83, AVX2/SSE2/NEON도 u32 lane 후 u64 합산)하며, 실사용 윈도우가 8x8=64픽셀로 고정되어 있어 overflow 여유가 큼 — 권장 패턴과 일치

---

### METRIC-SSIM-005: bit depth에 따른 상수 C1/C2 미조정
**분류**: 비트 심도 처리 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
const C1: f64 = 6.5025;  // (0.01 * 255)^2, 8bit 전제
const C2: f64 = 58.5225; // (0.03 * 255)^2, 8bit 전제

fn ssim_component(mean_x: f64, mean_y: f64, var_x: f64, var_y: f64, cov_xy: f64) -> f64 {
    // 10bit 입력이 들어와도 8bit 기준 C1/C2를 그대로 사용
    let luminance = (2.0 * mean_x * mean_y + C1) / (mean_x * mean_x + mean_y * mean_y + C1);
    let contrast_structure = (2.0 * cov_xy + C2) / (var_x + var_y + C2);
    luminance * contrast_structure
}
```

**문제**:
- SSIM의 안정화 상수 `C1 = (K1*L)^2`, `C2 = (K2*L)^2`는 동적 범위 `L`(보통 `2^bit_depth - 1`)에 의존한다. 8bit 기준으로 계산한 `C1/C2`를 10bit(`L=1023`) 데이터에 그대로 쓰면, 분모/분자의 상대적 안정화 효과가 실제 데이터 스케일과 맞지 않아 SSIM 값이 왜곡된다.
- 특히 평균/분산이 작은 어두운/평탄한 영역에서 C1/C2의 상대적 크기가 결과에 큰 영향을 주므로, 이 오차는 均一(uniform)한 저대비 영역에서 두드러진다.
- METRIC-PSNR-002와 근본 원인이 같은 클래스의 버그(8bit 하드코딩)이지만, PSNR은 단순 스케일 오차로 끝나는 반면 SSIM은 비선형 안정화 항이라 영향이 더 미묘하고 발견하기 어렵다.

**발생 조건**:
- 10bit/12bit HEVC·AV1·VVC 소스, HDR 콘텐츠.
- 8bit 전용으로 작성된 SSIM 레퍼런스 코드(논문 부록, 온라인 예제)를 그대로 이식한 초기 구현.

**권장**:
```rust
fn ssim_constants(bit_depth: u8) -> (f64, f64) {
    let l = ((1u32 << bit_depth) - 1) as f64;
    let k1 = 0.01;
    let k2 = 0.03;
    ((k1 * l).powi(2), (k2 * l).powi(2))
}

fn ssim_component(mean_x: f64, mean_y: f64, var_x: f64, var_y: f64, cov_xy: f64, bit_depth: u8) -> f64 {
    let (c1, c2) = ssim_constants(bit_depth);
    let luminance = (2.0 * mean_x * mean_y + c1) / (mean_x * mean_x + mean_y * mean_y + c1);
    let contrast_structure = (2.0 * cov_xy + c2) / (var_x + var_y + c2);
    luminance * contrast_structure
}
```
- `C1`/`C2`를 상수가 아니라 `bit_depth`의 함수로 매 호출마다(또는 프레임/스트림 단위로 캐시하여) 계산한다.
- K1/K2 상수(0.01/0.03)는 논문 기본값을 유지하되, 필요 시 조정 가능하도록 노출은 하더라도 기본값은 표준을 따른다.

**탐지 방법**:
- Static: `C1`/`C2`가 `const`로 고정된 리터럴인지, 함수로 계산되는지 grep.
- Runtime: 8bit와 10bit로 동일한 상대적 콘텐츠(스케일만 다름)를 비교해 SSIM 값이 일관되는지 회귀 테스트.

**예외**:
- 애플리케이션이 8bit 콘텐츠만 지원하도록 명시적으로 스코프를 제한하고 다른 bit depth 입력을 사전에 거부한다면 상수 고정이 허용될 수 있다.

**Bitvue 판정**: Confirmed — lib.rs:166 `let l = 255.0; // Dynamic range for 8-bit`로 C1/C2가 bit_depth 무관 상수로 고정, bit_depth 파라미터 없음

---

### METRIC-SSIM-006: RGB 채널 SSIM을 단순 평균
**분류**: 색공간 가중치 · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn rgb_ssim(r_ssim: f64, g_ssim: f64, b_ssim: f64) -> f64 {
    // R/G/B를 지각적 중요도 차이 없이 단순 평균
    (r_ssim + g_ssim + b_ssim) / 3.0
}
```

**문제**:
- 인간의 시각은 녹색 채널의 휘도 기여도가 가장 크고(ITU-R BT.601/709 휘도 가중치가 이를 반영), R/G/B를 동일 가중치로 평균하면 지각적으로 덜 중요한 채널(특히 블루)의 왜곡이 과대평가된다.
- 대부분의 표준 품질 지표(VMAF 포함)는 애초에 YUV(휘도/크로마 분리) 공간에서 계산하도록 설계되어 있는데, RGB SSIM을 단순 평균으로 합치는 구현은 이런 표준과 정합성이 없는 별개의 지표가 되어버려 다른 도구의 결과와 비교할 수 없다.
- RGB 입력 자체가 이미 감마 인코딩(sRGB 등)되어 있는지 선형 라이트인지도 명시하지 않으면, 결과의 재현성 자체가 흔들린다(이 부분은 COLOR 카테고리와도 접점이 있다).

**발생 조건**:
- 컨테이너/디코더가 RGB로 직접 출력하거나, 분석 파이프라인이 편의상 YUV→RGB 변환 이후 SSIM을 계산하는 경우.
- 원본이 YUV인데 잘못된 파이프라인 설계로 RGB 변환을 거친 뒤 지표를 계산하는 경우(PIXEL 카테고리의 "분석 전 RGBA 강제 변환" 안티패턴과 유사한 근본 원인).

**권장**:
```rust
fn luma_weighted_ssim(r_ssim: f64, g_ssim: f64, b_ssim: f64) -> f64 {
    // BT.709 휘도 가중치를 채널 SSIM에 적용(단, 이는 근사이며 YUV 직접 계산이 더 정확)
    0.2126 * r_ssim + 0.7152 * g_ssim + 0.0722 * b_ssim
}

// 더 나은 방법: 애초에 YUV(Y plane 우선)에서 SSIM을 계산
fn preferred_ssim(y_plane_ssim: f64) -> f64 {
    y_plane_ssim
}
```
- 가능하면 RGB 변환을 거치지 않고 원본 YUV의 Y plane(필요 시 U/V 포함)에서 직접 SSIM을 계산한다.
- RGB 경로가 불가피하다면 단순 평균 대신 표준 휘도 가중치를 적용하고, 어떤 가중치/색공간(sRGB vs linear)을 썼는지 문서화한다.

**탐지 방법**:
- Semantic: SSIM 계산 파이프라인이 RGB를 입력으로 받는지, YUV/Y plane을 직접 쓰는지 구조 확인.
- Manual: 표준 툴(FFmpeg SSIM 필터는 기본적으로 YUV planar에서 계산)과 채널별/통합 값 비교.

**예외**:
- 원본 콘텐츠가 애초에 RGB 네이티브(예: 스크린 캡처, 그래픽 원본)이고 YUV 변환이 불필요하거나 오히려 손실을 유발하는 경우, RGB 기반 계산이 더 적절할 수 있다. 이 경우도 단순 평균보다는 가중 평균을 권장한다.

**Bitvue 판정**: N/A — RGB 기반 SSIM 경로 자체가 코드베이스에 없음(YUV plane만 존재, ssim_yuv는 RGB가 아닌 Y/U/V)

---

### METRIC-SSIM-007: downsampling filter 불일치
**분류**: 알고리즘 정확성 · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
fn ms_ssim_downsample(plane: &Plane) -> Plane {
    // 최근접 이웃(nearest-neighbor) 다운샘플링으로 간단히 구현
    let mut out = Plane::new(plane.width / 2, plane.height / 2);
    for y in 0..out.height {
        for x in 0..out.width {
            out.set(x, y, plane.at(x * 2, y * 2));
        }
    }
    out
}
```

**문제**:
- MS-SSIM(Multi-Scale SSIM)은 여러 스케일에서 SSIM을 계산하기 위해 이미지를 반복적으로 다운샘플링하는데, 원 논문(Wang, Simoncelli, Bovik 2003)은 2×2 평균 필터 후 서브샘플링을 규정한다. nearest-neighbor나 다른 필터(bilinear, lanczos 등)를 쓰면 각 스케일의 이미지 콘텐츠 자체가 레퍼런스 구현과 달라져, 이후 모든 스케일의 SSIM 값이 어긋난다.
- 이 오차는 스케일이 진행될수록(다운샘플링을 반복할수록) 누적되어, 최종 MS-SSIM 값이 레퍼런스와 상당히 벌어질 수 있다.
- 특히 고주파 디테일이 많은 콘텐츠에서 다운샘플링 필터 차이가 aliasing 유무에 직접 영향을 주므로, 어떤 필터를 쓰는지에 따라 결과가 체계적으로 달라진다.

**발생 조건**:
- MS-SSIM을 직접 구현하며 다운샘플링 단계에서 "그냥 되는" 방법(nearest-neighbor, 짝수 인덱스만 취하기)을 임시로 썼다가 그대로 남기는 경우.
- 기존에 다른 목적(리사이징 UI, 썸네일 생성)으로 쓰던 범용 다운샘플러를 재사용하는 경우.

**권장**:
```rust
fn ms_ssim_downsample(plane: &Plane) -> Plane {
    // 논문 규정: 2x2 평균(박스) 필터 후 짝수 위치로 서브샘플링
    let mut out = Plane::new(plane.width / 2, plane.height / 2);
    for y in 0..out.height {
        for x in 0..out.width {
            let sum = plane.at(2*x, 2*y) as u32
                + plane.at(2*x+1, 2*y) as u32
                + plane.at(2*x, 2*y+1) as u32
                + plane.at(2*x+1, 2*y+1) as u32;
            out.set(x, y, ((sum + 2) / 4) as u16); // 반올림
        }
    }
    out
}
```
- MS-SSIM 다운샘플링은 논문이 규정한 2×2 평균 필터를 정확히 구현하고, 다른 리사이징 유틸리티와 절대 공유하지 않는다(용도가 다르면 필터도 다를 수 있으므로).
- 어떤 필터를 썼는지 함수명/주석에 명시(`box_downsample_2x2` 등)하여 향후 실수로 다른 필터로 교체되는 것을 방지한다.

**탐지 방법**:
- Manual: 다운샘플링 함수의 구현이 2×2 평균인지 코드 리뷰로 직접 확인.
- Manual: 레퍼런스 MS-SSIM 구현(예: libvmaf, 원 저자 MATLAB 코드)과 각 스케일별 중간 이미지를 비교.

**예외**:
- 성능이 극도로 중요한 근사 경로(예: 실시간 프리뷰용 "대략적인" MS-SSIM 근사치)에서 의도적으로 더 빠른 필터를 쓰고 "근사치"임을 명확히 라벨링한다면 허용 가능하나, 정밀 분석/리포트 경로에는 절대 사용하지 않는다.

**Bitvue 판정**: N/A — MS-SSIM(멀티스케일) 구현 자체가 코드베이스에 없음 — 단일 스케일 SSIM만 존재, 다운샘플링 단계 없음

---

### METRIC-SSIM-008: reference implementation과 다른 border policy
**분류**: 검증·정합성 · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
// 자체 구현이 어떤 레퍼런스(libvmaf? scikit-image? 원 논문?)를 따르는지
// 코드 어디에도 명시하지 않은 채 "SSIM을 구현했다"고만 알려짐
fn compute_ssim(reference: &Plane, distorted: &Plane) -> f64 {
    // ... 내부적으로 특정 경계 정책, 특정 window 함수, 특정 상수를 임의로 선택
    unimplemented!()
}
```

**문제**:
- SSIM은 이름은 하나지만 실제로는 구현마다 미묘하게 다른 여러 변종이 존재한다: 경계 처리(METRIC-SSIM-001), Gaussian vs uniform window, 8×8 vs 11×11 window, mean SSIM vs 각 스케일 가중 결합 방식(MS-SSIM) 등. "어떤 레퍼런스를 따랐는가"를 명시하지 않으면 두 SSIM 구현을 비교하는 것 자체가 무의미해진다.
- 사용자가 리포트에 찍힌 SSIM 값을 다른 도구(예: FFmpeg, libvmaf)의 값과 직접 비교하려 할 때, 값이 조금 다른 것이 "버그"인지 "설계상 다른 구현"인지 구분할 근거가 코드/문서 어디에도 없다.
- 검증(테스트) 관점에서도, "레퍼런스 구현과 비교"라는 검증 방법 자체가 어떤 레퍼런스인지 명시되지 않으면 재현 불가능한 검증이 되어버린다.

**발생 조건**:
- 항상 해당 — SSIM/MS-SSIM 구현을 처음 작성할 때부터 어떤 레퍼런스를 따를지 결정하지 않고 시작하는 경우.
- 여러 개발자가 각자 알고 있는 "SSIM 구현"을 참고해 조금씩 다른 버전을 만들어 합쳐진 코드베이스.

**권장**:
```rust
/// libvmaf의 float_ssim 구현을 레퍼런스로 채택함(2026-xx-xx 기준 버전 vX.Y.Z).
/// 차이점: 경계는 Exclude 정책, window는 11x11 Gaussian(sigma=1.5), K1=0.01, K2=0.03.
/// 검증: tests/ssim_reference_parity.rs 에서 libvmaf 출력과 대조.
fn compute_ssim(reference: &Plane, distorted: &Plane, params: &SsimParams) -> f64 {
    // ...
}
```
- 채택할 레퍼런스 구현(및 버전)을 하나 명시적으로 선택하고, 모듈/함수 docstring에 기록한다.
- 레퍼런스와의 수치 비교 테스트를 CI에 고정 케이스로 포함시켜, 향후 리팩터링이 정합성을 깨뜨리면 즉시 감지되게 한다.

**탐지 방법**:
- Manual: SSIM 관련 모듈에 "어떤 레퍼런스를 따랐는지" 명시하는 문서/주석이 있는지 확인.
- Runtime: 알려진 입력-출력 쌍(레퍼런스 툴로 생성한 골든 벡터)과의 오차가 허용 범위(예: 1e-4) 내인지 회귀 테스트.

**예외**:
- 표준 레퍼런스가 존재하지 않는 완전히 새로운 지표(SSIM의 변형이 아닌 독자 개발 지표)라면 이 항목은 적용되지 않는다.

**Bitvue 판정**: Confirmed — lib.rs의 ssim() 독스트링(112-138)에 공식만 적혀 있을 뿐 어떤 레퍼런스 구현(libvmaf/scikit-image/원논문)을 따르는지, 8x8 비중첩 블록 방식이 표준 11x11 슬라이딩 윈도우와 다르다는 점도 전혀 문서화되지 않음

---

### METRIC-SSIM-009: SIMD와 scalar 구현 결과 불일치
**분류**: 검증·정합성 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn ssim_row(reference: &[u16], distorted: &[u16]) -> f64 {
    if is_x86_feature_detected!("avx2") {
        unsafe { ssim_row_avx2(reference, distorted) } // 별도로 작성된 SIMD 커널
    } else {
        ssim_row_scalar(reference, distorted)
    }
    // 두 경로의 수치적 동등성을 검증하는 테스트가 없음
}
```

**문제**:
- SIMD 커널은 흔히 부동소수점 연산 순서(reduction 순서), 중간 정밀도(예: `f32` SIMD vs `f64` scalar), 근사 명령어(fast reciprocal/rsqrt 근사 등) 때문에 scalar 구현과 완전히 동일하지 않은 결과를 낸다. 차이가 작을 수 있지만, 대량의 픽셀/프레임에 걸쳐 누적되면 사용자가 인지할 수 있는 수준(소수점 몇 자리 차이가 리포트에 그대로 노출)까지 벌어질 수 있다.
- CPU 기능 감지에 따라 실행 경로가 자동으로 바뀌므로, 같은 소스 코드가 다른 하드웨어(AVX2 지원 여부)에서 서로 다른 SSIM 값을 낼 수 있어 "같은 빌드인데 머신마다 결과가 다르다"는 재현 불가능한 버그 리포트로 이어진다.
- 골든 테스트(기대값 고정 테스트)가 scalar 경로만 실행하는 CI 머신에서 통과했다가, AVX2를 지원하는 사용자 머신에서는 미세하게 다른 값이 나와 "테스트는 통과했는데 실제로는 다르다"는 신뢰 문제를 낳는다.

**발생 조건**:
- SIMD 최적화 커널을 도입한 직후, 또는 CI 머신과 배포 대상 머신의 CPU 세대/기능 집합이 다를 때.
- 부동소수점 reduction 순서에 민감한 대형 윈도우/대형 프레임.

**권장**:
```rust
#[cfg(test)]
mod parity_tests {
    use super::*;

    #[test]
    fn simd_matches_scalar_within_tolerance() {
        let (reference, distorted) = load_golden_test_frame();
        let scalar_result = ssim_row_scalar(&reference, &distorted);
        let simd_result = unsafe { ssim_row_avx2(&reference, &distorted) };
        // 완전히 동일할 필요는 없지만, 허용 오차 내여야 함(오차 크기는 실측 후 문서화)
        assert!((scalar_result - simd_result).abs() < 1e-6,
            "SIMD/scalar mismatch: {} vs {}", simd_result, scalar_result);
    }
}
```
- 모든 SIMD 경로에 대해 scalar 구현과의 동등성(허용 오차 내)을 검증하는 parity 테스트를 CI에 반드시 포함시킨다(가능하면 SIMD 지원 CI 러너에서 실제로 실행).
- 허용 오차의 근거(어떤 연산이 오차를 유발하는지, 그 오차가 최종 지표에 미치는 영향)를 문서화한다.
- 재현성이 절대적으로 중요한 경로(예: 공식 인증/벤치마크 리포트)에서는 SIMD를 비활성화하고 scalar 경로만 쓰는 "정밀 모드" 옵션을 제공하는 것도 고려한다.

**탐지 방법**:
- Runtime: CPU 기능이 다른 여러 머신(또는 기능 감지를 강제로 끈 빌드)에서 동일 입력에 대한 결과를 비교하는 CI 매트릭스.
- Static: SIMD 커널 추가 PR에 scalar 대조 테스트가 함께 포함되었는지 리뷰 체크리스트로 강제.

**예외**:
- SIMD와 scalar가 알고리즘적으로 완전히 동일한 연산 순서를 강제하도록(예: 명시적 순차 reduction) 작성되어 부동소수점 오차가 이론적으로도 발생하지 않음을 증명한 경우는 별도 허용 오차 없이 정확히 일치해야 하며, 이는 이 항목의 예외이자 이상적인 목표.

**Bitvue 판정**: N/A — simd.rs:598-616 test_window_stats_vs_scalar(정확 일치 assert)와 simd.rs:640-667 test_psnr_simd_vs_scalar(0.5dB 허용오차)로 SIMD/scalar parity 테스트가 이미 CI 테스트로 존재함

---

### METRIC-SSIM-010: 타일 병렬화 경계에서 window가 끊김
**분류**: 병렬화 정확성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn parallel_ssim(reference: &Plane, distorted: &Plane, window: usize, num_tiles: usize) -> f64 {
    let tile_height = reference.height / num_tiles;
    let partial_sums: Vec<f64> = (0..num_tiles)
        .into_par_iter()
        .map(|t| {
            let y_start = t * tile_height;
            let y_end = (y_start + tile_height).min(reference.height);
            // 각 타일을 독립적으로 처리하면서, 타일 경계에 걸친 window는
            // 이웃 타일의 픽셀에 접근하지 못해 잘못 계산되거나 건너뜀
            ssim_partial_sum(reference, distorted, 0, y_start, reference.width, y_end, window)
        })
        .collect();
    partial_sums.iter().sum::<f64>() / total_valid_windows(reference, window) as f64
}
```

**문제**:
- SSIM은 슬라이딩 윈도우 연산이라, 이미지를 타일로 나누어 병렬 처리할 때 타일 경계 근처의 윈도우는 이웃 타일의 픽셀까지 필요로 한다. 각 타일을 완전히 독립적으로(이웃 타일 데이터 접근 없이) 처리하면 경계 부근의 윈도우가 잘못된 값(경계 밖 데이터를 0/클리핑으로 대체하거나, 아예 건너뜀)을 만든다.
- 이 오차는 타일 수(스레드 수, 즉 실행 환경의 CPU 코어 수)에 따라 크기가 달라진다 — 타일이 많을수록(코어가 많을수록) 경계선이 많아져 오차가 커진다. 즉 같은 입력이 실행 환경(코어 수)에 따라 다른 SSIM 값을 낸다는, METRIC-SSIM-009와 유사하지만 원인이 다른 재현성 문제를 만든다.
- 단일 스레드 실행(타일 1개)에서는 이 버그가 전혀 드러나지 않으므로, 개발 중 단일 스레드로 검증하고 프로덕션에서만 멀티스레드로 실행하면 버그가 배포 후에야 발견된다.

**발생 조건**:
- CPU 코어 수가 많은 머신에서 이미지/plane을 행 단위 타일로 나눠 `rayon` 등으로 병렬화할 때.
- 타일 경계에 여유분(halo/overlap) 없이 정확히 분할선을 긋는 구현.

**권장**:
```rust
fn parallel_ssim(reference: &Plane, distorted: &Plane, window: usize, num_tiles: usize) -> f64 {
    let half = window / 2;
    let tile_height = reference.height / num_tiles;
    let partial_sums: Vec<f64> = (0..num_tiles)
        .into_par_iter()
        .map(|t| {
            let y_start = t * tile_height;
            let y_end = (y_start + tile_height).min(reference.height);
            // 각 타일이 윈도우 계산에 필요한 만큼(half) 이웃 타일 영역까지 halo로 포함해 읽되,
            // 결과를 기록하는 범위는 원래 타일 범위(y_start..y_end)로 제한
            ssim_partial_sum_with_halo(reference, distorted, y_start, y_end, half, window)
        })
        .collect();
    partial_sums.iter().sum::<f64>() / total_valid_windows(reference, window) as f64
}
```
- 타일 분할 시 윈도우 반경만큼의 halo(겹침 영역)를 포함해 읽되, 각 윈도우의 "소유자"(결과를 어느 타일이 책임지고 기록하는지)는 원래 분할선을 기준으로 명확히 구분한다.
- 타일 수(스레드 수)를 1, 2, 8 등으로 바꿔가며 결과가 완전히 동일한지(또는 정의된 허용 오차 내인지) 검증하는 회귀 테스트를 반드시 포함한다.

**탐지 방법**:
- Structural: 타일 병렬화 함수가 이웃 타일의 픽셀에 접근 가능한 구조(halo 포함)인지, 완전히 독립된 슬라이스만 받는지 확인.
- Runtime: 스레드/타일 수를 다르게 설정한 동일 입력 실행 결과를 diff하는 테스트(`RAYON_NUM_THREADS` 환경변수 등으로 제어).

**예외**:
- 윈도우 연산이 아닌 픽셀 단위 독립 연산(예: 단순 MSE)을 타일 병렬화하는 경우는 이 문제가 원천적으로 발생하지 않는다 — 이 항목은 슬라이딩 윈도우를 쓰는 SSIM류에 특화된 문제다.

**Bitvue 판정**: N/A — batch_ssim_parallel(lib.rs:364-387)은 프레임 단위로만 par_iter 병렬화하며 단일 프레임 내부의 타일 분할·halo 처리 로직 자체가 없음 — 프레임 내 슬라이딩 윈도우 타일 병렬화 코드가 존재하지 않아 해당 안티패턴이 발생할 여지가 없음

---

## VMAF

### METRIC-VMAF-001: 모델 이름과 버전을 저장하지 않음
**분류**: 결과 재현성 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
struct VmafResult {
    score: f64, // 어떤 모델로 계산했는지 결과에 기록되지 않음
}

fn compute_vmaf(reference: &Frame, distorted: &Frame) -> VmafResult {
    let score = run_libvmaf(reference, distorted, "vmaf_v0.6.1.json"); // 모델 경로가 호출부에만 존재
    VmafResult { score }
}
```

**문제**:
- VMAF 점수는 사용된 모델(기본 모델, phone 모델, 4K 모델, 커스텀 학습 모델 등)에 따라 절대값이 크게 달라지는데, 결과 구조체에 모델 식별자를 저장하지 않으면 나중에 이 점수가 어떤 조건에서 계산됐는지 알 수 없다.
- 리포트를 내보내거나(export), 세션을 저장했다가 나중에 다시 불러오거나, 다른 시점에 재계산해서 비교할 때 모델이 바뀌었는지(라이브러리 업데이트로 기본 모델 파일이 교체됐는지 등) 확인할 방법이 없어 "지난주엔 92점이었는데 오늘은 89점"이라는 혼란이 생겨도 원인을 특정할 수 없다.
- 서로 다른 모델로 계산된 두 점수를 같은 축의 그래프에 나란히 그리면(모델 불일치를 인지하지 못한 채) 의미 없는 비교가 된다.

**발생 조건**:
- 항상 해당 — VMAF를 조금이라도 저장/리포트/비교하는 모든 경로.
- 특히 사용자가 모델을 선택할 수 있는 UI가 있는 경우(기본/phone/4K/커스텀), 결과와 모델 선택이 분리되어 있으면 위험이 커진다.

**권장**:
```rust
struct VmafResult {
    score: f64,
    model_name: String,     // 예: "vmaf_v0.6.1", "vmaf_4k_v0.6.1"
    model_version: String,  // libvmaf 내부 모델 버전 문자열(모델 파일이 노출하는 경우)
    model_file_hash: String, // 모델 파일 내용의 해시(파일 경로가 아닌 내용 기준 — 같은 이름의 다른 파일 방지)
    libvmaf_version: String, // METRIC-VMAF-010과 연관
}

fn compute_vmaf(reference: &Frame, distorted: &Frame, model: &VmafModel) -> VmafResult {
    let score = run_libvmaf(reference, distorted, &model.path);
    VmafResult {
        score,
        model_name: model.name.clone(),
        model_version: model.version.clone(),
        model_file_hash: model.content_hash.clone(),
        libvmaf_version: libvmaf_library_version(),
    }
}
```
- VMAF 점수를 저장/직렬화할 때는 항상 모델 식별자(이름, 가능하면 파일 해시)와 라이브러리 버전을 함께 저장한다.
- UI에 점수를 표시할 때도 모델 이름을 함께(적어도 툴팁으로) 노출한다.

**탐지 방법**:
- Structural: `VmafResult`(또는 동등한 타입)에 모델 식별 필드가 있는지 타입 정의 확인.
- Static: 직렬화/리포트 생성 코드가 모델 정보를 함께 내보내는지 grep.

**예외**:
- 애플리케이션이 오직 단일 고정 모델만 영구적으로 지원하기로(다른 모델 선택 UI 자체가 없고, 향후에도 추가 계획이 없음을 아키텍처 문서에 명시) 결정했다면 필드를 생략하고 전역 상수/문서로만 명시해도 된다. 다만 이 가정은 매우 깨지기 쉬우므로 권장하지 않는다.

**Bitvue 판정**: Confirmed — compute_vmaf(crates/bitvue-metrics/src/vmaf.rs:130-212)는 f64 score만 반환하고 VmafConfig.model_path/버전/해시를 결과에 전혀 기록하지 않음(feature=vmaf일 때만 컴파일되며 기본 빌드/워크스페이스 어디서도 활성화되지 않음, Cargo.toml default=[])

---

### METRIC-VMAF-002: phone model과 기본 model 결과 혼용
**분류**: 결과 재현성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn get_default_vmaf_model(target_display: DisplayHint) -> &'static str {
    match target_display {
        DisplayHint::Phone => "vmaf_v0.6.1.json", // phone 전용 모델을 안 쓰고 기본 모델 사용
        DisplayHint::Tv => "vmaf_v0.6.1.json",
        _ => "vmaf_v0.6.1.json",
    }
    // 어떤 상황이든 동일 모델 파일을 반환하면서, 함수 이름은 "phone model 지원"을 암시
}
```

또는 반대 방향:
```rust
fn batch_compute(sessions: &[Session]) -> Vec<f64> {
    // 일부 세션은 phone 모델, 일부는 기본 모델로 계산된 채 하나의 배열에 섞임
    sessions.iter().map(|s| s.cached_vmaf_score).collect()
}
```

**문제**:
- VMAF phone 모델은 작은 화면에서의 시청 조건을 반영해 기본 모델보다 일반적으로 더 높은(관대한) 점수를 낸다. 두 모델의 점수는 척도가 다르므로 직접 비교하거나 평균 내는 것은 무의미하다.
- 세션마다 다른 모델로 계산된 점수가 하나의 데이터셋/그래프에 섞이면, 실제로는 동일한 인코딩 설정인데도 "phone 모델로 계산된 항목이 우연히 더 좋아 보이는" 식으로 왜곡된 순위가 나올 수 있다.
- 사용자가 "휴대폰에서 보는 것처럼 평가해줘"라는 의도로 phone 모델을 선택했는데 실제로는 기본 모델이 쓰이면(위 첫 번째 예시), 그 반대로 다른 조건을 요청했는데 항상 phone 모델이 쓰이면, 사용자의 기대와 실제 계산이 어긋난다.

**발생 조건**:
- 여러 세션/배치 작업 결과를 하나의 데이터셋으로 합산할 때 모델 선택이 세션마다 달랐던 경우.
- UI에 "phone model" 토글이 있지만 실제로 백엔드에 모델 선택이 제대로 전달되지 않는 배선 버그.

**권장**:
```rust
fn get_vmaf_model(target_display: DisplayHint) -> VmafModel {
    match target_display {
        DisplayHint::Phone => VmafModel::phone(),
        _ => VmafModel::default_hd(),
    }
}

fn batch_compute(sessions: &[Session]) -> Result<Vec<VmafResult>, MetricError> {
    // 배치 결과를 합산하기 전에 모두 동일 모델인지 검증
    let model_ids: HashSet<_> = sessions.iter().map(|s| &s.vmaf_result.model_name).collect();
    if model_ids.len() > 1 {
        return Err(MetricError::MixedModels(model_ids.into_iter().cloned().collect()));
    }
    Ok(sessions.iter().map(|s| s.vmaf_result.clone()).collect())
}
```
- 모델 선택 로직을 단위 테스트로 검증해 UI 선택이 실제 실행 인자로 정확히 전달되는지 확인한다.
- 여러 결과를 집계/비교하기 전에 모델 일관성을 검증하는 가드를 넣는다(METRIC-VMAF-001에서 저장한 모델 식별자를 활용).

**탐지 방법**:
- Semantic: 모델 선택 함수가 입력 조건에 따라 실제로 다른 모델을 반환하는지 로직 검토.
- Runtime: 배치/집계 함수에 서로 다른 모델의 결과를 섞어 넣었을 때 에러가 발생하는지 테스트.

**예외**:
- 애플리케이션이 phone 모델 지원을 아예 제공하지 않기로 결정했다면(단일 모델만 지원) 이 문제 자체가 발생하지 않는다.

**Bitvue 판정**: N/A — phone model 선택 로직 자체가 코드베이스에 없음(VmafModel::phone() 등 미존재, VmafConfig에 model_path 수동 지정만 존재)

---

### METRIC-VMAF-003: negative 결과 clipping 정책 불명확
**분류**: 수치 정확성 · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn report_vmaf(raw_score: f64) -> f64 {
    // libvmaf가 이론적으로 음수나 100 초과 값을 낼 수 있는데, 클리핑 여부가 불명확
    raw_score
}
```

**문제**:
- VMAF는 회귀 모델 기반 지표라, 극단적으로 나쁜 화질(심한 블록화, 완전한 노이즈)이나 반대로 특정 향상 처리(과도한 샤프닝 등)를 거친 프레임에서는 이론적 범위 0~100을 살짝 벗어난 음수 또는 100 초과 값이 나올 수 있다. 이를 클리핑할지 그대로 노출할지는 라이브러리/애플리케이션마다 정책이 다르다.
- 클리핑 정책을 명시하지 않으면, UI에서 "음수 VMAF 점수"라는 직관적으로 이상해 보이는 값이 그대로 노출되어 사용자가 버그로 오인하거나, 반대로 조용히 클리핑하면 극단적으로 나쁜 프레임들 사이의 상대적 차이 정보(클리핑 전에는 -20과 -5로 구분되던 것이 클리핑 후 둘 다 0)가 사라진다.
- 시퀀스 평균을 낼 때 클리핑 시점(프레임별로 먼저 클리핑 후 평균 vs 평균 낸 후 마지막에 한 번 클리핑)에 따라 결과가 달라질 수 있어 이 역시 명시가 필요하다.

**발생 조건**:
- 매우 낮은 품질(강한 압축, 전송 오류 은닉)의 프레임이나, VMAF 모델이 학습되지 않은 도메인 밖 콘텐츠(애니메이션, 스크린 콘텐츠 등)를 분석할 때.

**권장**:
```rust
#[derive(Clone, Copy)]
enum ClipPolicy {
    None,               // raw 점수를 그대로 노출(내부 분석/디버깅용)
    ClipToStandardRange, // [0, 100]으로 클리핑(사용자 대상 리포트 기본값)
}

fn report_vmaf(raw_score: f64, policy: ClipPolicy) -> f64 {
    match policy {
        ClipPolicy::None => raw_score,
        ClipPolicy::ClipToStandardRange => raw_score.clamp(0.0, 100.0),
    }
}

// 시퀀스 평균은 항상 raw 값으로 먼저 계산한 뒤, 최종 표시 단계에서만 클리핑
fn sequence_vmaf(raw_frame_scores: &[f64], policy: ClipPolicy) -> f64 {
    let raw_avg = raw_frame_scores.iter().sum::<f64>() / raw_frame_scores.len() as f64;
    report_vmaf(raw_avg, policy)
}
```
- 클리핑 정책을 명시적 파라미터/설정으로 노출하고, 기본값(보통 사용자 대상 리포트는 클리핑)을 문서화한다.
- 내부 raw 값은 클리핑 전 상태로 저장해두어, 필요 시 분석가가 raw 데이터를 다시 볼 수 있게 한다.
- 집계(평균)는 항상 클리핑 이전의 raw 값으로 수행하고, 클리핑은 최종 표시 단계에서만 한 번 적용한다.

**탐지 방법**:
- Semantic: VMAF 점수를 다루는 코드 경로 전체에서 클리핑이 어느 시점에 적용되는지(계산 직후 vs 표시 직전 vs 전혀 없음) 추적.
- Manual: 음수 raw 점수를 낼 수 있는 극단적 합성 테스트(완전 랜덤 노이즈 프레임)로 클리핑 동작 확인.

**예외**:
- libvmaf 자체가 옵션으로 내부 클리핑을 제공하고 애플리케이션이 이를 그대로 신뢰하기로 결정했다면, 상위 레이어에서 별도 클리핑 로직을 두지 않아도 된다 — 단, 이 경우도 "libvmaf가 클리핑한다"는 사실과 조건을 문서화해야 한다.

**Bitvue 판정**: Confirmed — compute_vmaf(vmaf.rs:207-211)가 vmaf.score()의 raw 값을 clamp 없이 그대로 반환 — ClipPolicy 등 클리핑 로직 부재

---

### METRIC-VMAF-004: 4K model과 일반 model 혼용
**분류**: 결과 재현성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn select_vmaf_model(width: u32, height: u32) -> &'static str {
    // 해상도와 무관하게 항상 HD 기준 기본 모델을 사용
    "vmaf_v0.6.1.json"
}
```

**문제**:
- VMAF 4K 모델(`vmaf_4k_v0.6.1.json`)은 4K 해상도·정상 시청 거리 조건에 맞춰 별도로 학습된 모델로, HD용 기본 모델과 점수 척도가 다르다. 4K 콘텐츠에 기본 모델을 그대로 적용하면 시청 거리/해상도 가정이 맞지 않아 점수가 실제 지각 품질을 반영하지 못한다.
- 반대로 HD 콘텐츠에 4K 모델을 잘못 적용해도 마찬가지로 부정확하다. 즉 이 문제는 "모델을 안 바꾸는 것"과 "무조건 4K 모델을 쓰는 것" 양방향 모두 해당한다.
- 소스 해상도만으로 모델을 자동 선택하는 로직이 있더라도, 업스케일/다운스케일된 콘텐츠(예: 4K로 업스케일된 HD 소스)에서는 "해상도"가 실제 콘텐츠의 원본 디테일 수준을 반영하지 않으므로 자동 선택 자체가 오판할 수 있다는 점도 함께 고려해야 한다.

**발생 조건**:
- 4K/UHD 콘텐츠 비교 세션에서 모델 선택 로직이 해상도를 아예 고려하지 않거나, 잘못된 임계값을 쓰는 경우.
- 여러 해상도가 섞인 배치 분석에서 모델이 세션마다 일관되게 선택되지 않는 경우.

**권장**:
```rust
fn select_vmaf_model(width: u32, height: u32, user_override: Option<VmafModelKind>) -> VmafModel {
    if let Some(kind) = user_override {
        return VmafModel::for_kind(kind); // 사용자가 명시적으로 선택했다면 그 선택을 존중
    }
    // 자동 선택은 어디까지나 기본값(default)이며, 그 기준을 문서화
    if width >= 3840 || height >= 2160 {
        VmafModel::uhd_4k()
    } else {
        VmafModel::default_hd()
    }
}
```
- 해상도 기반 자동 모델 선택 로직을 두되, 그 임계값과 근거를 문서화하고 사용자가 override할 수 있게 한다.
- 선택된 모델은 METRIC-VMAF-001의 결과 필드에 반드시 기록해, 나중에 "이 세션엔 어떤 모델이 쓰였는지" 추적 가능하게 한다.

**탐지 방법**:
- Structural: 모델 선택 함수가 `width`/`height` 파라미터를 실제로 분기에 사용하는지 확인.
- Runtime: 4K 해상도 합성 입력으로 호출했을 때 실제로 4K 모델 파일 경로가 선택되는지 테스트.

**예외**:
- 애플리케이션이 HD 콘텐츠만 지원 범위로 명시하고 4K 입력을 사전에 거부/경고한다면 자동 선택 로직 자체가 불필요할 수 있다.

**Bitvue 판정**: N/A — 해상도 기반 자동 모델 선택 로직 자체가 없음(select_vmaf_model 류 함수 미존재) — VmafConfig.model_path는 항상 수동/None

---

### METRIC-VMAF-005: subsampling 설정 누락
**분류**: 성능·정확성 트레이드오프 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_sequence_vmaf(frames: &[FramePair]) -> f64 {
    // n_subsample 설정을 명시하지 않고 라이브러리 기본값(1 = 모든 프레임)을 암묵적으로 사용
    // 하다가, 성능 문제로 누군가 나중에 "임시로" 5로 바꿔놓고 그 사실이 리포트에 드러나지 않음
    run_libvmaf_sequence(frames, VmafConfig::default())
}
```

**문제**:
- libvmaf는 `n_subsample` 옵션으로 매 N번째 프레임만 계산하고 나머지는 보간/생략할 수 있는데, 이는 계산 속도를 크게 높이지만 정확도를 낮추는 트레이드오프다. 이 설정이 코드 안에 암묵적으로 박혀 있고 결과에 기록되지 않으면, "빠른 프리뷰용으로 5프레임마다 계산한 근사치"와 "전체 프레임을 정밀 계산한 값"이 똑같이 "VMAF 점수"로만 보고되어 구분할 수 없다.
- 서브샘플링된 점수를 정밀 값처럼 취급해 세밀한 프레임 단위 그래프(예: 특정 프레임에서 급격한 화질 저하)를 그리면, 실제로 존재하는 프레임별 요동을 놓치거나 보간으로 왜곡된 그래프를 보여주게 된다.
- 서브샘플링 설정이 세션마다 다르면(예: 미리보기는 5, 최종 리포트는 1) 두 결과를 같은 자리에서 비교하는 것이 무의미해진다.

**발생 조건**:
- 대용량/고해상도 시퀀스에서 계산 시간을 줄이기 위해 서브샘플링을 켜는 성능 최적화 경로.
- "빠른 프리뷰" 모드와 "정밀 분석" 모드가 함께 존재하는 UI.

**권장**:
```rust
struct VmafConfig {
    n_subsample: u32, // 1 = 모든 프레임(정밀), N>1 = 매 N프레임마다 계산(근사)
}

struct VmafResult {
    score: f64,
    n_subsample: u32, // 결과에도 서브샘플링 설정을 함께 기록
    // ... 모델 정보 등(METRIC-VMAF-001)
}

fn compute_sequence_vmaf(frames: &[FramePair], config: &VmafConfig) -> VmafResult {
    let score = run_libvmaf_sequence(frames, config);
    VmafResult { score, n_subsample: config.n_subsample }
}
```
- 서브샘플링 설정을 명시적 파라미터로 노출하고 기본값을 문서화(정밀 분석 기본값은 1을 권장).
- 결과에 서브샘플링 설정을 함께 저장해, UI에서 "이 점수는 근사치입니다(매 N프레임)"라는 안내를 표시할 수 있게 한다.

**탐지 방법**:
- Structural: `VmafConfig`(또는 동등 구조)에 `n_subsample` 필드가 명시적으로 노출되어 있는지, 결과 타입에도 기록되는지 확인.
- Static: `run_libvmaf*` 호출부에서 서브샘플링 관련 옵션이 하드코딩되어 있는지 grep.

**예외**:
- 애플리케이션이 서브샘플링 기능을 아예 지원하지 않고 항상 전체 프레임을 계산하기로 결정했다면, 이 설정을 노출할 필요 없이 상수 1로 고정해도 된다(단, 이 결정도 문서화 권장).

**Bitvue 판정**: Confirmed — VmafConfig(vmaf.rs:13-22)에 n_subsample 필드가 존재하지 않아 서브샘플링 설정 자체를 노출할 수 없는 구조

---

### METRIC-VMAF-006: libvmaf 내부 thread와 외부 병렬화 중첩
**분류**: 동시성·성능 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn analyze_all_sessions(sessions: &[Session]) -> Vec<f64> {
    // 세션마다 libvmaf를 n_threads=8로 호출하면서,
    // 동시에 이 함수 자체도 rayon으로 세션 단위 병렬화
    sessions.par_iter().map(|s| {
        run_libvmaf(&s.reference, &s.distorted, VmafConfig { n_threads: 8, ..Default::default() })
    }).collect()
}
```

**문제**:
- libvmaf는 내부적으로 프레임/블록 단위 스레드 풀을 가질 수 있는데(`n_threads` 옵션), 이를 외부의 세션 단위 병렬화(rayon 등)와 곱셈으로 중첩시키면 (세션 수 × libvmaf 내부 스레드 수)만큼의 스레드가 동시에 CPU 코어를 놓고 경쟁하게 된다.
- 이 경우 실제 사용 가능한 코어 수를 훨씬 초과하는 스레드가 컨텍스트 스위칭 오버헤드만 유발하며, 오히려 순차 실행보다 느려지는 역설적인 성능 저하가 발생할 수 있다.
- 메모리 사용량도 병렬 실행 중인 세션 수만큼 배가되어(각 세션이 프레임 버퍼, 모델 데이터 등을 독립적으로 들고 있으므로), 대량 배치 처리 시 OOM 위험이 커진다.

**발생 조건**:
- 배치 분석(여러 세션/스트림 쌍을 한 번에 처리)에서 외부 병렬화와 libvmaf의 내부 병렬화 옵션을 모두 기본값 또는 모두 최대치로 켜둔 경우.
- 코어 수가 제한된 환경(CI 러너, 컨테이너 cgroup 제한)에서 특히 두드러진다.

**권장**:
```rust
fn analyze_all_sessions(sessions: &[Session], total_cores: usize) -> Vec<f64> {
    // 총 코어 예산을 세션 병렬도와 libvmaf 내부 스레드 수로 명시적으로 분배
    let session_parallelism = (total_cores / 2).max(1);
    let vmaf_threads_per_session = (total_cores / session_parallelism).max(1);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(session_parallelism)
        .build()
        .expect("thread pool");

    pool.install(|| {
        sessions.par_iter().map(|s| {
            run_libvmaf(&s.reference, &s.distorted,
                VmafConfig { n_threads: vmaf_threads_per_session, ..Default::default() })
        }).collect()
    })
}
```
- 외부 병렬도와 libvmaf 내부 `n_threads`를 곱한 총합이 사용 가능한 논리 코어 수를 넘지 않도록 명시적으로 예산을 나눈다.
- 배치 크기와 개별 세션 크기(프레임 수, 해상도)에 따라 최적 분배가 달라질 수 있으므로, 벤치마크로 실측해 기본값을 정한다.

**탐지 방법**:
- Runtime: CPU 사용률/코어 경합을 모니터링하면서 배치 처리 시간이 세션 수에 따라 선형보다 나쁘게 증가하는지 벤치마크.
- Structural: 세션 단위 병렬화 코드와 `VmafConfig::n_threads` 설정이 서로 독립적으로(코어 예산 조율 없이) 정해지는지 확인.

**예외**:
- 배치 크기가 항상 1(세션을 순차적으로만 처리)이라면 이 문제는 발생하지 않으며, libvmaf의 `n_threads`를 최대치로 설정해도 안전하다.

**Bitvue 판정**: N/A — VmafConfig.n_threads 필드가 정의만 되어 있고 compute_vmaf 본문에서 실제 Vmaf 인스턴스에 전달되지 않으며(vmaf.rs:155-168 어디에도 config.n_threads 미사용), 세션 단위 rayon par_iter로 compute_vmaf를 호출하는 코드도 코드베이스에 없어 중첩이 발생할 경로가 없음

---

### METRIC-VMAF-007: frame feature를 모두 메모리에 보존
**분류**: 메모리 관리 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
struct VmafSession {
    // 프레임별 중간 feature(ADM, VIF, motion 등 서브 피처 전체)를
    // 시퀀스 끝까지 전부 Vec에 쌓아둔 채 세션이 끝날 때까지 보존
    all_frame_features: Vec<FrameFeatures>,
    final_score: Option<f64>,
}

impl VmafSession {
    fn push_frame(&mut self, features: FrameFeatures) {
        self.all_frame_features.push(features);
    }
}
```

**문제**:
- VMAF 계산 과정에서 파생되는 프레임별 서브 피처(ADM, VIF 여러 스케일, motion 등)는 최종 점수 산출과 프레임별 리포트에 필요한 요약값만 남기면 충분한데, 원본 피처 전체를 세션이 끝날 때까지 계속 누적하면 긴 시퀀스(수만 프레임)에서 메모리 사용량이 선형으로 계속 증가한다.
- 특히 4K/8K처럼 프레임당 피처 데이터 자체도 큰 해상도에서는, 짧은 스트림이라도 메모리 사용량이 예상보다 훨씬 커질 수 있다.
- 이 문제는 스트리밍/장시간 분석 세션에서 특히 치명적이다 — 스트림 앞부분에서는 문제없이 동작하다가, 뒷부분에 갈수록 메모리 사용량이 누적되어 결국 OOM으로 프로세스가 죽는 "서서히 나빠지는" 패턴을 보인다.

**발생 조건**:
- 긴 시퀀스(수 분~수 시간)를 스트리밍 방식으로 분석하는 배치/CLI 도구.
- 프레임별 상세 피처를 나중에 재계산 없이 다시 보여주기 위해 "일단 다 저장해두자"는 안이한 설계.

**권장**:
```rust
struct VmafSession {
    running_sum: f64,
    running_sum_sq: f64,     // 표준편차 등 요약 통계 계산용
    frame_count: u64,
    frame_scores: Vec<f32>,  // 프레임별 최종 점수(요약값)만 유지 — 서브 피처는 버림
    // 서브 피처가 정말 필요하면 디스크로 스트리밍 기록(옵션)하고 메모리엔 남기지 않음
}

impl VmafSession {
    fn push_frame(&mut self, features: FrameFeatures) {
        let score = features.final_score();
        self.running_sum += score;
        self.running_sum_sq += score * score;
        self.frame_count += 1;
        self.frame_scores.push(score as f32);
        // features(서브 피처 전체)는 여기서 drop됨
    }
}
```
- 서브 피처는 프레임 처리 직후 최종 점수만 추출하고 즉시 버린다(요약 통계는 running sum 방식으로 O(1) 메모리 유지).
- 프레임별 상세 피처를 정말 UI/디버깅에서 다시 봐야 한다면, 메모리 대신 디스크(임시 파일, 압축 로그)로 스트리밍 기록하고 필요 시 다시 읽는다.
- 장시간 세션에 대한 메모리 사용량 벤치마크(프레임 수 대비 RSS)를 CI에 포함시켜 회귀를 조기에 잡는다.

**탐지 방법**:
- Runtime: 긴 합성 시퀀스(예: 10,000 프레임)로 메모리 프로파일링(RSS가 프레임 수에 비례해 계속 증가하는지).
- Structural: 세션 상태 구조체에 프레임별 전체 피처를 담는 `Vec<FrameFeatures>` 같은 필드가 있는지 확인.

**예외**:
- 세션 길이가 항상 짧음(예: 수 초, 수백 프레임 이내)이 애플리케이션 설계로 보장되고 메모리 예산이 충분하다면, 편의를 위해 전체 피처를 보존해도 실질적 위험이 낮을 수 있다. 이 경우도 상한을 두는 것이 안전하다.

**Bitvue 판정**: Suspected — compute_vmaf(vmaf.rs)가 &[VmafFrame] 전체를 파라미터로 요구해 호출자가 시퀀스 전체를 미리 메모리에 들고 있어야 하는 구조이며, libvmaf 내부 서브피처 보존 여부는 FFI 경계 너머라 직접 확인 불가 — 세션 상태에 프레임별 서브피처를 쌓는 자체 구현 코드는 없음

---

### METRIC-VMAF-008: 모델 파일 로딩을 매 작업 반복
**분류**: 성능 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn compute_vmaf_for_frame_pair(reference: &Frame, distorted: &Frame) -> f64 {
    // 프레임 하나를 계산할 때마다 모델 파일을 디스크에서 다시 읽고 파싱
    let model = VmafModel::load_from_file("models/vmaf_v0.6.1.json")
        .expect("failed to load VMAF model");
    run_libvmaf_single_frame(reference, distorted, &model)
}
```

**문제**:
- VMAF 모델 파일(JSON 기반 SVM 회귀 계수 등)은 프레임마다 바뀌지 않는 정적 데이터인데, 프레임 단위 함수 안에서 매번 디스크 I/O와 파싱을 반복하면 순수 계산 비용보다 모델 로딩 비용이 더 커지는 경우가 생긴다.
- 특히 수천~수만 프레임을 순회하는 배치 분석에서는 이 반복 로딩이 누적되어 전체 처리 시간의 상당 부분을 차지하게 되고, 디스크 I/O 경합(다른 프로세스와의 경쟁, 네트워크 파일시스템 지연)이 있으면 더욱 악화된다.
- 병렬 처리 환경에서는 여러 스레드가 동시에 같은 모델 파일을 반복적으로 여는 것 자체가 불필요한 락 경합/파일 핸들 낭비를 유발할 수 있다.

**발생 조건**:
- 프레임 단위로 세분화된 함수 시그니처를 설계하면서 "모델은 어디서 오는가"를 고려하지 않고 함수 내부에서 즉석 로딩하는 구현.
- 대량의 짧은 클립을 배치 처리하는 워크플로우(클립마다 세션을 새로 만들며 매번 모델을 새로 로딩).

**권장**:
```rust
struct VmafEngine {
    model: Arc<VmafModel>, // 한 번 로딩해 세션/프로세스 생애주기 동안 재사용
}

impl VmafEngine {
    fn new(model_path: &str) -> Result<Self, MetricError> {
        let model = Arc::new(VmafModel::load_from_file(model_path)?);
        Ok(Self { model })
    }

    fn compute_for_frame_pair(&self, reference: &Frame, distorted: &Frame) -> f64 {
        run_libvmaf_single_frame(reference, distorted, &self.model)
    }
}
```
- 모델을 프로세스/세션 시작 시 한 번 로딩해 `Arc`로 공유하고, 프레임 단위 함수는 이미 로딩된 모델을 참조만 받는다.
- 병렬 워커 간에도 동일 모델 인스턴스를 `Arc`로 공유해 중복 로딩을 방지한다.
- 여러 다른 모델을 함께 다뤄야 하는 경우, 모델별로 캐시(LRU 등)를 두어 동일 모델의 반복 로딩만 방지한다.

**탐지 방법**:
- Runtime: 프레임 처리 함수를 프로파일링해 모델 로딩(파일 I/O + 파싱) 비용이 전체 시간에서 차지하는 비율 측정.
- Static: 모델 로딩 함수 호출 지점이 프레임 단위 루프 내부인지 세션/엔진 초기화 지점인지 grep으로 확인.

**예외**:
- 프레임당 한 번만 호출되는 극히 짧은 CLI 유틸리티(예: "이미지 두 장 비교"용 일회성 도구)라면 매번 로딩해도 실질적 성능 영향이 없다.

**Bitvue 판정**: N/A — compute_vmaf/compute_vmaf_per_frame(vmaf.rs) 둘 다 모델을 루프 진입 전 1회만 로딩(vmaf.rs:159-168, 241-250)하여 프레임마다 반복 로딩하지 않음 — 이 항목이 지적하는 안티패턴이 코드에 없음. 다만 이 함수를 반복 호출하는 상위 caller 자체가 존재하지 않아(feature 미사용) 실사용 시나리오 검증은 불가

---

### METRIC-VMAF-009: VMAF 실패 시 PSNR로 조용히 대체
**분류**: 에러 처리 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_quality_score(reference: &Frame, distorted: &Frame) -> f64 {
    match run_libvmaf(reference, distorted) {
        Ok(score) => score,
        Err(_) => {
            // VMAF 계산 실패(모델 로딩 실패, 라이브러리 없음, 해상도 불일치 등)를
            // 사용자에게 알리지 않고 PSNR로 조용히 대체 — 반환값의 척도/의미가 완전히 다름
            compute_psnr(reference, distorted)
        }
    }
}
```

**문제**:
- VMAF(0~100 스케일, 지각 품질 기반)와 PSNR(dB 스케일, 픽셀 오차 기반)은 척도도 의미도 완전히 다른 지표인데, 실패 시 폴백으로 조용히 바꿔치기하면 호출자는 반환된 숫자가 어느 지표인지 알 방법이 없다.
- 예를 들어 "VMAF 95"와 "PSNR 95"(이론상 나오기 힘들지만 극단적으로)는 전혀 다른 의미인데, 함수 시그니처가 `f64` 하나만 반환하면 이 구분이 완전히 소실된다. UI에 "품질 점수: 42.3"이라고만 표시되면, 사용자는 이것이 VMAF인지 PSNR인지, 심지어 폴백이 발생했다는 사실조차 알 수 없다.
- libvmaf 자체가 없는 환경(선택적 의존성이 빌드에서 빠진 경우), 모델 파일 경로가 잘못된 경우, 해상도 불일치로 libvmaf가 에러를 반환하는 경우 등 다양한 실패 원인이 모두 동일하게 "조용한 대체"로 뭉개져서, 실제로 무엇이 잘못됐는지 디버깅할 단서도 사라진다.
- 이런 폴백이 배치 작업 중간에 발생하면, 같은 리포트/그래프 안에 VMAF 값과 PSNR 값이 라벨 구분 없이 섞여 들어가는 최악의 경우로 이어질 수 있다(METRIC-VMAF-002와 유사한 혼용 문제).

**발생 조건**:
- `libvmaf-sys`가 선택적(optional) 의존성으로 빌드에서 빠질 수 있는 구성(빌드 플래그, feature gate로 VMAF 지원이 꺼진 바이너리).
- 모델 파일 누락, 해상도/포맷 불일치 등으로 libvmaf 호출이 런타임에 실패하는 경우.

**권장**:
```rust
#[derive(Debug)]
enum QualityScore {
    Vmaf { score: f64, model_name: String },
    Psnr { score_db: f64 },
}

#[derive(Debug)]
enum QualityScoreError {
    VmafUnavailable(String), // 라이브러리 미포함 등 구조적 사유
    VmafComputationFailed(String), // 런타임 계산 실패(모델 로딩, 해상도 불일치 등)
}

fn compute_quality_score(reference: &Frame, distorted: &Frame) -> Result<QualityScore, QualityScoreError> {
    run_libvmaf(reference, distorted)
        .map(|score| QualityScore::Vmaf { score, model_name: "vmaf_v0.6.1".into() })
        .map_err(|e| QualityScoreError::VmafComputationFailed(e.to_string()))
    // PSNR로 자동 대체하지 않는다. 대체가 필요하다면 호출자가 명시적으로 선택하게 한다.
}

fn compute_quality_score_with_explicit_fallback(
    reference: &Frame, distorted: &Frame,
) -> QualityScore {
    match run_libvmaf(reference, distorted) {
        Ok(score) => QualityScore::Vmaf { score, model_name: "vmaf_v0.6.1".into() },
        Err(e) => {
            log::warn!("VMAF unavailable ({e}), falling back to PSNR — results are NOT comparable to VMAF scores");
            QualityScore::Psnr { score_db: compute_psnr(reference, distorted) }
        }
    }
}
```
- 반환 타입 자체에 "어떤 지표인지"를 태그로 새겨(enum 등) 호출자가 실수로 서로 다른 지표를 같은 것으로 취급할 수 없게 만든다.
- 자동 폴백이 정말 필요한 제품 요구사항이라면, 반드시 명시적인 함수명(`_with_explicit_fallback`)과 사용자에게 보이는 경고(로그, UI 배지)를 동반해야 한다.
- 실패 원인(라이브러리 부재 vs 런타임 계산 실패)을 구분해 각기 다른 처리(빌드 설정 안내 vs 재시도/모델 경로 확인 안내)를 할 수 있게 한다.

**탐지 방법**:
- Semantic: VMAF 계산 실패 처리 분기에서 다른 지표로 조용히 폴백하는 코드가 있는지 코드 리뷰로 추적.
- Structural: 품질 점수 반환 타입이 단일 `f64`인지, 어떤 지표인지 구분 가능한 타입인지 확인.

**예외**:
- 폴백이 발생했다는 사실이 반환 타입/로그/UI 모두에 명확히 드러나고, 호출자가 이를 의도적으로 요청한 경우(예: "VMAF 우선, 불가하면 PSNR이라도" 옵션을 사용자가 명시적으로 켠 경우)라면 허용 가능하다. 핵심은 "조용히"가 아니라 "명시적으로"다.

**Bitvue 판정**: N/A — crates/bitvue-cli/src/main.rs:192는 `-m`(metrics) 도움말에 "psnr, ssim, vmaf"라고 vmaf를 언급하지만, crates/bitvue-cli/src/commands/quality.rs::run()(119-123)은 `metrics.contains("psnr")`/`"ssim"`만 검사하고 vmaf를 요청하면 조용히 PSNR로 대체하는 게 아니라 `anyhow::bail!("Unknown metrics ...")`로 명시적으로 에러 처리 — VMAF 계산 시도 자체가 없고 조용한 대체 로직도 없음(이전 판정의 src-tauri 인용은 삭제된 파일이라 오판정, 결론은 재확인 후 정확한 위치로 교체)

---

### METRIC-VMAF-010: library version이 달라도 동일 결과로 간주
**분류**: 결과 재현성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn cache_key(reference_hash: &str, distorted_hash: &str, model_name: &str) -> String {
    // 캐시 키에 libvmaf 라이브러리 버전이 빠져 있어,
    // 라이브러리를 업그레이드해도 예전 캐시된 점수를 그대로 재사용
    format!("{reference_hash}:{distorted_hash}:{model_name}")
}
```

**문제**:
- libvmaf는 버전 간에 내부 알고리즘 세부사항(특징 추출 정밀도, 회귀 계수 반올림, 기본 옵션값 등)이 조정되는 경우가 있어, 동일 입력·동일 모델이라도 라이브러리 버전이 다르면 점수가 미세하게(때로는 눈에 띄게) 달라질 수 있다. 이는 버그가 아니라 업스트림의 정상적인 버전 간 변화다.
- 캐시 키나 결과 메타데이터에 라이브러리 버전이 빠져 있으면, 라이브러리를 업그레이드한 뒤에도 예전 버전으로 계산된 캐시 값을 "동일한 결과"로 재사용하게 되어, 실제로는 재현 불가능한 값이 재현 가능한 것처럼 보이는 착시가 생긴다.
- 장기간에 걸친 회귀 추적(예: "이번 릴리즈가 지난 릴리즈보다 화질이 좋아졌다")을 할 때, 결과 차이가 실제 인코더 변경 때문인지 단순히 VMAF 라이브러리 버전이 바뀌어서인지 구분할 수 없게 된다.

**발생 조건**:
- `libvmaf-sys` 등 의존성을 업그레이드하는 모든 시점.
- 결과를 디스크/DB에 영속 캐시하고 장기간에 걸쳐 재사용하는 아키텍처.
- 서로 다른 머신/CI 러너가 서로 다른 libvmaf 버전을 링크하고 있는 경우(정적 링크 vs 시스템 라이브러리 혼용 등).

**권장**:
```rust
fn cache_key(reference_hash: &str, distorted_hash: &str, model_name: &str) -> String {
    let libvmaf_version = libvmaf_library_version(); // 런타임에 조회 가능한 버전 문자열
    format!("{reference_hash}:{distorted_hash}:{model_name}:{libvmaf_version}")
}

struct VmafResult {
    score: f64,
    model_name: String,
    libvmaf_version: String, // METRIC-VMAF-001의 필드와 동일 — 캐시 키에도 반드시 포함
}
```
- 캐시 키와 결과 메타데이터 모두에 라이브러리 버전을 포함시켜, 버전이 바뀌면 캐시가 자동으로 무효화되게 한다.
- CI/배포 파이프라인에서 사용 중인 libvmaf 버전을 고정(pin)하고, 버전을 올릴 때는 알려진 골든 벡터에 대한 점수 변화를 명시적으로 검토·기록하는 절차를 둔다.
- 장기 추적용 리포트에는 각 데이터 포인트에 라이브러리 버전을 함께 표시해, 버전 변경 시점을 그래프상에서 구분할 수 있게 한다.

**탐지 방법**:
- Structural: 캐시 키 생성 함수가 라이브러리 버전을 입력으로 포함하는지 확인.
- Manual: 의존성 업그레이드 PR에서 골든 벡터 점수 diff를 리뷰 체크리스트 항목으로 강제.

**예외**:
- 라이브러리 버전이 빌드 시점에 완전히 고정되어(vendored, 정적 링크, 버전 핀 고정) 애플리케이션 생애주기 동안 절대 바뀌지 않음이 빌드 시스템으로 보증된다면, 버전을 캐시 키에 매번 포함하지 않아도 실질적 위험은 낮다. 다만 결과 메타데이터에는 여전히 기록해두는 것이 감사(audit) 목적에서 안전하다.

**Bitvue 판정**: N/A — VMAF 결과에 대한 캐시 키 생성 로직 자체가 코드베이스에 존재하지 않음(캐싱 미구현)
