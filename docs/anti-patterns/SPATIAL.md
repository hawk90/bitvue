# Anti-Pattern Catalog — SPATIAL: 해상도와 공간 정렬 (VQ-Probe domain)

이 문서는 Bitvue VQ Probe(듀얼 스트림 품질 비교 기능군, 비트스트림 분석기 도메인과 아키텍처상 분리되고 media-core 레이어만 공유) 안티패턴 카탈로그의 일부입니다. 전체 목록과 카테고리 구성은 `docs/anti-patterns/INDEX.md`(별도 작성 예정)를 참고하십시오. VQ-Probe 도메인 절반은 ALIGN/COLOR/METRIC/PIPE/HEAT/STAT와 함께 본 SPATIAL을 포함하며, 1단계(본 문서)는 일반적·도메인 특화 참조 카탈로그이고 실제 저장소에 대한 감사와 판정은 2단계에서 수행합니다.

---

### SPATIAL-001: 크기가 다르면 무조건 resize
**분류**: SPATIAL · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
fn align_for_metric(reference: &Frame, distorted: &Frame) -> (Frame, Frame) {
    if reference.width == distorted.width && reference.height == distorted.height {
        return (reference.clone(), distorted.clone());
    }
    // 크기가 다르면 이유를 묻지 않고 무조건 distorted를 reference 크기로 맞춘다
    let resized = resize(distorted, reference.width, reference.height, Filter::Bilinear);
    (reference.clone(), resized)
}
```

**문제**:
- 해상도 차이의 원인이 "의도된 다운스케일 인코딩"인지 "메타데이터 파싱 실패로 잘못된 크기를 읽은 버그"인지 구분하지 않고 동일하게 처리한다
- crop, letterbox, SAR(sample aspect ratio) 차이로 인한 크기 불일치까지 전부 단순 리사이즈로 뭉개버려 실제로는 안 맞는 정렬을 맞는 것처럼 보고한다
- resize는 필연적으로 저역통과 필터링 효과를 가지므로, 메트릭이 "화질 차이"가 아니라 "리샘플링으로 생긴 차이"를 측정하게 되는 경우가 생긴다
- 원인 분석 없는 자동 보정은 실제 버그(예: 컨테이너 파싱 오류로 SPS/VUI 크기를 잘못 읽음)를 조용히 숨긴다

**발생 조건**:
- ABR(adaptive bitrate) 인코딩 래더처럼 의도적으로 reference보다 낮은 해상도로 인코딩된 스트림을 비교할 때
- 파싱 버그로 실제 디코드 크기와 컨테이너 메타데이터 크기가 어긋났을 때도 같은 코드 경로를 타게 됨
- 사용자가 서로 다른 소스(4K reference vs 1080p 인코드본)를 실수로 비교 대상으로 지정했을 때

**권장**:
```rust
fn align_for_metric(reference: &Frame, distorted: &Frame) -> Result<(Frame, Frame), SpatialMismatch> {
    if reference.width == distorted.width && reference.height == distorted.height {
        return Ok((reference.clone(), distorted.clone()));
    }
    // 크기 차이의 원인을 분류해서 각기 다른 처리를 명시적으로 선택하게 한다
    match classify_resolution_mismatch(reference, distorted)? {
        ResolutionMismatch::IntendedDownscale { ratio } => {
            let resized = resize_with_policy(distorted, reference.size(), &upscale_policy(ratio));
            Ok((reference.clone(), resized))
        }
        ResolutionMismatch::Unexpected => Err(SpatialMismatch::RequiresUserConfirmation),
    }
}
```
- 크기 불일치를 감지하면 우선 "왜 다른가"를 사용자/호출자에게 보여주고, 자동 보정 여부를 명시적으로 선택하게 한다
- 자동 리사이즈를 적용하더라도 그 사실과 사용된 필터를 결과 메타데이터에 기록한다(SPATIAL-002, SPATIAL-015 참고)
- 예상 해상도 조합의 화이트리스트(예: 인코딩 래더 프리셋)가 있다면 그 범위를 벗어나는 조합은 오류로 처리

**탐지 방법**:
- Structural: `resize` 호출 지점을 grep해서 호출 직전에 크기 불일치 "원인"을 분기하는 조건문이 있는지 확인
- Runtime: 동일한 두 스트림에 대해 의도적으로 손상된 메타데이터를 주입해 리사이즈가 조용히 실행되는지 회귀 테스트
- 코드 리뷰: "이 resize가 실행되는 경우를 사용자가 알 수 있는가?"를 체크리스트 항목으로

**예외**:
- 명시적으로 "ABR 래더 비교 모드"로 동작하도록 설계된 기능에서, 문서화된 업스케일 정책 하에 자동 리사이즈하는 것은 정상 동작
- 프로토타입/디버그 도구에서 즉시성이 정확성보다 중요한 경우

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-002: resize filter를 기록하지 않음
**분류**: SPATIAL · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn resize(frame: &Frame, w: u32, h: u32) -> Frame {
    // 내부적으로 bilinear를 쓰는지 lanczos를 쓰는지 호출자는 알 수 없다
    image_ops::scale(frame, w, h)
}

fn compute_ssim_report(ref_frame: &Frame, dist_frame: &Frame) -> SsimReport {
    let aligned = resize(dist_frame, ref_frame.width, ref_frame.height);
    SsimReport { score: ssim(ref_frame, &aligned), resolution: (ref_frame.width, ref_frame.height) }
}
```

**문제**:
- 리샘플링 필터(nearest/bilinear/bicubic/lanczos/area 등)는 결과 메트릭 값에 직접 영향을 준다 — 같은 두 영상도 필터에 따라 SSIM/PSNR이 유의미하게 달라짐
- 리포트에 필터 정보가 없으면 재현이 불가능하고, 서로 다른 리포트 간 비교가 무의미해진다("작년 리포트는 몇 점이었지?" 질문에 답할 수 없음)
- 라이브러리 업그레이드로 기본 필터가 바뀌면 아무도 눈치채지 못한 채 과거 데이터와 비교 불가능한 값이 섞인다
- 회귀 테스트가 "필터가 바뀌어도 점수가 유지되는지"를 검증할 방법이 없다

**발생 조건**:
- 서드파티 이미지/비디오 스케일링 라이브러리(FFmpeg swscale, libyuv 등)를 기본값으로 호출할 때
- 여러 팀/모듈이 각자 resize 유틸을 감싸서 사용하며 필터 선택을 함수 내부에 하드코딩했을 때

**권장**:
```rust
#[derive(Clone, Copy, Debug, Serialize)]
enum ResizeFilter { NearestNeighbor, Bilinear, Bicubic, Lanczos3, Area }

fn resize(frame: &Frame, target: Size, filter: ResizeFilter) -> Frame {
    image_ops::scale_with(frame, target, filter.into())
}

struct SsimReport {
    score: f64,
    resolution: Size,
    preprocessing: PreprocessingRecord, // filter, 방향, 버전 등을 포함
}
```
- resize 함수는 필터를 필수 파라미터로 요구하고, 기본값을 암묵적으로 제공하지 않는다
- 사용된 필터(및 라이브러리 버전)를 결과 메타데이터/리포트에 항상 기록한다
- 필터 선택을 프로젝트 전역 상수 하나로 통일하고 변경 시 CHANGELOG에 남긴다

**탐지 방법**:
- Structural: resize 관련 API의 시그니처에 filter 파라미터가 없는지 검사
- Static: 메트릭 결과 구조체에 전처리 메타데이터 필드가 있는지 타입 검사로 강제
- Manual: 리포트 포맷 리뷰에서 "이 점수를 재현하려면 어떤 정보가 더 필요한가?" 질문

**예외**:
- 내부 디버그/스크래치 용도로 즉석에서 만든 일회성 비교 도구는 기록을 생략해도 무방(단, 결과를 정식 리포트로 승격하지 않는다는 전제)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-003: crop과 resize 순서가 불명확
**분류**: SPATIAL · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn preprocess(frame: &Frame, target: Size, crop: CropRect) -> Frame {
    // 어느 것을 먼저 해도 "동작은" 하기 때문에 순서를 대충 정함
    let resized = resize(frame, target, ResizeFilter::Bilinear);
    apply_crop(&resized, crop) // crop 좌표가 원본 해상도 기준인지 resize 후 기준인지 불명확
}
```

**문제**:
- crop 좌표계가 원본 해상도 기준인지 리사이즈 후 해상도 기준인지 호출부마다 암묵적으로 다르게 가정하기 쉽다
- 순서를 반대로 하면(crop 먼저 → resize) 결과 해상도는 같아도 실제로 비교되는 영역이 완전히 달라진다
- 두 스트림(reference/distorted)의 전처리 파이프라인이 서로 다른 순서를 쓰면, 같은 이름의 crop rect가 서로 다른 픽셀 영역을 가리키게 되어 메트릭이 조용히 틀어진다
- 파이프라인이 함수 합성으로만 표현되고 좌표계 불변식이 타입으로 강제되지 않으면, 리팩터링 중 순서가 뒤집혀도 컴파일 에러가 나지 않는다

**발생 조건**:
- letterbox 제거(crop) 후 리사이즈해서 메트릭 해상도를 표준화하는 파이프라인
- crop 파라미터가 설정 파일이나 API로 외부에서 주입되어 "어느 좌표계 기준인지"가 코드만 봐서는 알 수 없을 때
- reference/distorted 전처리 로직이 서로 다른 시점에 다른 작성자가 만들어 순서 컨벤션이 갈릴 때

**권장**:
```rust
/// 좌표계를 타입으로 구분해 순서 실수를 컴파일 타임에 막는다
struct CropInSourceSpace(CropRect);
struct CropInTargetSpace(CropRect);

fn preprocess(frame: &Frame, crop: CropInSourceSpace, target: Size, filter: ResizeFilter) -> Frame {
    // 정책을 명시적으로 고정: 항상 "원본 좌표계로 crop → 이후 resize"
    let cropped = apply_crop(frame, crop.0);
    resize(&cropped, target, filter)
}
```
- 파이프라인 순서를 프로젝트 전역 정책으로 문서화하고 하나의 헬퍼 함수로 강제(순서를 호출부마다 재구현하지 않기)
- crop 좌표계를 뉴타입으로 감싸 "원본 기준"과 "리사이즈 후 기준"을 타입 수준에서 섞이지 못하게 한다
- reference와 distorted 파이프라인이 동일한 헬퍼를 공유하는지 코드 리뷰/구조 테스트로 확인

**탐지 방법**:
- Structural: `apply_crop`와 `resize` 호출이 함수 하나로 감싸져 있는지, 혹은 호출부마다 순서가 제각각인지 grep
- Static: 좌표계 뉴타입을 쓰면 컴파일러가 잘못된 순서 조합을 거부
- Runtime: 알려진 crop+resize 조합에 대해 순서를 바꿔 실행한 두 결과가 다르다는 것을 보여주는 회귀 테스트

**예외**:
- crop 영역이 리사이즈 배율과 무관하게 항상 정수 배율 경계에 정렬되어 순서가 결과에 영향을 주지 않는 특수 케이스(드묾, 명시적으로 증명 가능할 때만)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-004: coded size와 display size 혼동
**분류**: SPATIAL · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn extract_luma_plane(decoded: &DecodedFrame) -> &[u8] {
    // 디코더가 매크로블록/CTU 경계에 맞춰 패딩한 coded size를 그대로 사용
    let stride = decoded.coded_width as usize;
    let rows = decoded.coded_height as usize;
    &decoded.luma_buffer[..stride * rows]
}
```

**문제**:
- 대부분의 코덱은 픽처를 16(AVC 매크로블록) 또는 64(HEVC/AV1 CTU/SB) 단위로 패딩해 디코드하고, 실제 표시 영역(display/conformance window)은 그보다 작다
- coded size 그대로 메트릭을 계산하면 우측/하단 가장자리의 패딩(엣지 확장 또는 미정의 값)이 실제 화질 데이터인 것처럼 섞여 들어간다
- reference와 distorted의 패딩량이 다르면(예: 해상도가 16의 배수가 아닌 1080 대신 1088로 패딩) 정렬 자체가 어긋난 채로 픽셀 단위 비교가 수행된다
- 이 오류는 특정 해상도(정확히 16/64의 배수)에서는 증상이 드러나지 않아 테스트에서 놓치기 쉽다

**발생 조건**:
- 세로/가로 해상도가 16 또는 64의 배수가 아닌 콘텐츠(예: 1080p = 1920x1080, 1080은 64의 배수가 아님)
- 디코더 SDK가 coded_width/coded_height와 display_width/display_height(또는 conformance window)를 별도 필드로 제공하는데 후자를 무시하고 전자만 사용할 때
- crop 없이 곧바로 디코더 출력 버퍼를 메트릭 계산에 넘기는 경로

**권장**:
```rust
fn extract_luma_plane(decoded: &DecodedFrame) -> Vec<u8> {
    let stride = decoded.coded_width as usize; // 버퍼 접근에는 coded size(stride)를 그대로 써야 함
    let win = decoded.conformance_window; // display size는 여기서만 가져온다
    let mut out = Vec::with_capacity(win.width as usize * win.height as usize);
    for row in win.y..win.y + win.height {
        let start = row as usize * stride + win.x as usize;
        out.extend_from_slice(&decoded.luma_buffer[start..start + win.width as usize]);
    }
    out
}
```
- 버퍼 인덱싱(stride)에는 coded size를, 메트릭 계산 영역 결정에는 display/conformance size를 사용한다는 원칙을 코드 전체에서 일관되게 유지
- `DecodedFrame` 타입에 coded_size와 display_size(또는 conformance_window)를 별개 필드로 명확히 구분해 보관
- crop 적용 여부를 프레임 메타데이터 플래그로 남겨 이후 파이프라인이 실수로 이중 crop하지 않게 한다

**탐지 방법**:
- Semantic: 디코더 바인딩 코드에서 `coded_width`/`coded_height`가 메트릭 계산 함수까지 그대로 전달되는 경로가 있는지 데이터 흐름 추적
- Runtime: 16/64의 배수가 아닌 해상도(1080p, 1088p 등 경계값)로 회귀 테스트해서 결과 차이 확인
- Manual: 코덱별 SPS/시퀀스 헤더 파싱 코드와 프레임 추출 코드 사이의 필드 매핑을 리뷰

**예외**:
- 해상도가 이미 16/64의 배수라 coded size == display size인 콘텐츠만 다루는 것이 문서로 보장된 도구(단, 향후 콘텐츠 확장 시 재검토 필요)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-005: black bar를 영상 품질 영역에 포함
**분류**: SPATIAL · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_frame_metric(reference: &Frame, distorted: &Frame) -> f64 {
    // letterbox/pillarbox의 검은 여백을 포함한 전체 프레임을 그대로 비교
    ssim(reference, distorted)
}
```

**문제**:
- letterbox(상하 검은 띠)나 pillarbox(좌우 검은 띠)는 콘텐츠가 아니라 여백이며, 인코더는 이 영역을 거의 무손실에 가깝게 압축한다(단색 평면)
- 여백 영역이 메트릭 계산에 포함되면 실제 콘텐츠 영역의 화질 저하가 "평균"에 의해 희석되어 점수가 실제보다 좋게 나온다(특히 여백 비율이 큰 세로 영상/시네마스코프 콘텐츠에서 심각)
- reference와 distorted의 여백 두께가 미세하게 다르면(인코더가 여백 경계에서 링잉/블록 아티팩트를 만드는 경우) 그 차이가 콘텐츠 화질 문제처럼 보고된다
- 서로 다른 여백 비율을 가진 두 콘텐츠의 점수를 나란히 비교하면 "여백이 적은 쪽이 유리해지는" 편향이 생긴다

**발생 조건**:
- 시네마스코프(2.39:1) 콘텐츠를 16:9 컨테이너에 letterbox로 넣은 스트림
- 세로 영상을 16:9 컨테이너에 pillarbox로 넣은 스트림(모바일 숏폼 콘텐츠 등)
- 자동 여백 감지 없이 프레임 전체 크기를 항상 "품질 측정 영역"으로 가정하는 파이프라인

**권장**:
```rust
struct ContentRegion { x: u32, y: u32, width: u32, height: u32 }

fn detect_letterbox(frame: &Frame, black_threshold: u8) -> ContentRegion {
    // 상하좌우에서 거의 균일한 어두운 행/열을 스캔해 콘텐츠 영역을 찾는다
    scan_uniform_borders(frame, black_threshold)
}

fn compute_frame_metric(reference: &Frame, distorted: &Frame, region: ContentRegion) -> f64 {
    let r = crop_to_region(reference, &region);
    let d = crop_to_region(distorted, &region);
    ssim(&r, &d)
}
```
- 자동 letterbox/pillarbox 감지(여러 프레임에 걸쳐 안정적으로 검은 영역인지 확인)를 전처리 단계로 두고, 감지된 콘텐츠 영역만 메트릭 계산에 사용
- 자동 감지에 실패하거나 애매한 경우 사용자가 수동으로 콘텐츠 영역을 지정할 수 있게 한다
- 감지된 여백 크기와 방식을 리포트에 기록(감지가 틀렸을 때 재현/디버깅 가능하도록)

**탐지 방법**:
- Semantic: 메트릭 계산 직전 입력 프레임에 대해 여백 감지 단계가 파이프라인에 존재하는지 확인
- Runtime: 알려진 letterbox 콘텐츠(예: 2.39:1 in 16:9)로 골든 테스트를 만들어, 여백 포함/제외 시 점수 차이가 기대 범위인지 검증
- Manual: 히트맵(HEAT 카테고리와 연계) 시각화로 여백 영역이 메트릭에 기여하는지 육안 확인

**예외**:
- 여백을 포함한 "전체 프레임 압축 효율"을 의도적으로 평가하는 모드(예: 여백까지 포함한 실제 전송 비트 대비 품질 평가)에서는 여백 포함이 올바른 선택 — 단 이 경우 모드임을 리포트에 명시해야 한다

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-006: rotation metadata 무시
**분류**: SPATIAL · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn load_frame_for_comparison(path: &Path, frame_index: u64) -> Frame {
    // 컨테이너의 회전 메타데이터(예: MP4 'tkhd' matrix, HEVC display orientation SEI)를 읽지 않는다
    let decoded = decode_frame(path, frame_index);
    decoded.into_frame()
}
```

**문제**:
- 스마트폰으로 촬영된 영상은 실제 픽셀은 가로로 인코딩되고 회전 정보(90/180/270도)를 메타데이터로만 전달하는 경우가 흔하다(MP4 track matrix, HEVC/AVC display orientation SEI 등)
- reference와 distorted 중 하나만 회전 메타데이터를 적용해서 디코드/렌더링하면, 두 프레임의 픽셀이 90도 어긋난 채로 SSIM/PSNR을 계산하게 되어 점수가 사실상 무의미한 잡음이 된다
- 회전이 있는데 무시하면 가로세로 치수 자체가 바뀌어(예: 1080x1920 vs 1920x1080) 정렬 실패로 이어지거나, 우연히 치수가 같은 경우(정사각형에 가까운 비율)엔 조용히 잘못된 값을 낸다
- 재인코딩 파이프라인에서 한쪽은 메타데이터 회전을 픽셀에 베이크(bake)하고 다른 쪽은 메타데이터로만 유지하는 불일치가 자주 발생한다

**발생 조건**:
- 모바일 촬영 원본을 reference로, 트랜스코딩된 결과물을 distorted로 비교할 때(트랜스코더가 회전을 픽셀에 베이크하는 경우가 많음)
- 서로 다른 컨테이너(MP4 vs MKV)나 서로 다른 회전 신호 방식(track matrix vs SEI vs 없음)을 쓰는 두 소스를 비교할 때
- 디코더 래퍼가 회전 메타데이터 필드를 파싱은 하되 프레임 정규화 단계에 반영하지 않을 때

**권장**:
```rust
struct FrameOrientation { rotation_deg: u16, flip_horizontal: bool }

fn load_frame_for_comparison(path: &Path, frame_index: u64) -> Frame {
    let decoded = decode_frame(path, frame_index);
    let orientation = read_orientation_metadata(path); // track matrix / SEI / 없음(0도)을 통일된 타입으로
    normalize_orientation(decoded.into_frame(), orientation)
}
```
- 디코드 직후, 메트릭 계산 이전 단계에서 항상 정규화된(회전 0도) 좌표계로 통일한다
- reference/distorted 각각의 orientation을 독립적으로 읽고 정규화해서, 한쪽만 적용되는 사고를 구조적으로 막는다
- 정규화 후 치수가 예상과 다르면(회전으로 인해 W/H가 뒤바뀜) 다운스트림 crop/resize 로직이 이를 인지하도록 파이프라인 순서를 rotation → crop → resize로 고정

**탐지 방법**:
- Semantic: 컨테이너 파서가 회전 필드를 읽는 코드와, 프레임 정규화 코드 사이에 실제 데이터 연결이 있는지 추적
- Runtime: 90/180/270도 회전 메타데이터를 가진 샘플 파일로 회귀 테스트(정규화 후 두 이미지가 픽셀 단위로 정렬되는지 확인)
- Manual: 회전된 콘텐츠에 대해 나온 메트릭 점수가 비정상적으로 낮은지(정렬 실패의 전형적 징후) 리뷰

**예외**:
- 두 입력 모두 회전 메타데이터가 이미 픽셀에 베이크되어 있고 메타데이터 필드가 항상 0(또는 identity matrix)임이 파이프라인 계약으로 보장된 경우

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-007: sample aspect ratio 무시
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn resolutions_match(a: &StreamInfo, b: &StreamInfo) -> bool {
    // 픽셀 치수만 비교하고 SAR(sample aspect ratio)/PAR를 확인하지 않는다
    a.width == b.width && a.height == b.height
}
```

**문제**:
- 픽셀 치수(coded width/height)가 같아도 SAR(sample aspect ratio, 픽셀 자체의 가로세로 비율)가 다르면 실제 표시되는 화면 비율(DAR)이 다르다 — 예: anamorphic DVD/방송 콘텐츠는 720x480 픽셀에 SAR 8:9 같은 비정방 픽셀을 쓴다
- 픽셀 단위 비교(SSIM/PSNR)는 SAR와 무관하게 그대로 수행 가능하지만, DAR가 다른데 픽셀만 맞춰 비교하면 "같은 화면 비율"이라는 잘못된 전제로 리포트를 해석하게 된다
- 한쪽 스트림만 non-square 픽셀(SAR ≠ 1:1)이고 다른 쪽이 정방 픽셀로 재인코딩되었다면, 픽셀 치수가 같아도 실제 화면에 표시되는 형태(가로/세로 비율)가 달라 리사이즈 정렬이 필요한 상황을 "이미 정렬됨"으로 오판한다
- 화면 표시용 렌더링(오버레이, 프리뷰)에서 SAR를 무시하면 콘텐츠가 찌그러져 보이는 부가 버그로도 이어진다

**발생 조건**:
- 방송/DVD 유래 콘텐츠(anamorphic, SAR ≠ 1:1)를 웹 배포용으로 재인코딩(SAR 1:1로 정규화)한 결과물을 비교할 때
- 서로 다른 인코더/트랜스코더가 SAR를 다르게 시그널링하거나 아예 생략(기본값 1:1로 간주)할 때
- 컨테이너 레벨 SAR(예: MP4 pasp box)와 코덱 레벨 SAR(예: H.264 VUI aspect_ratio_info)가 서로 다른 값을 가질 때(어느 쪽이 우선인지 처리 안 함)

**권장**:
```rust
struct SampleAspectRatio { num: u32, den: u32 }

fn display_aspect_ratio(info: &StreamInfo) -> (u32, u32) {
    let dar_num = info.width as u64 * info.sar.num as u64;
    let dar_den = info.height as u64 * info.sar.den as u64;
    reduce_fraction(dar_num, dar_den)
}

fn resolutions_are_comparable(a: &StreamInfo, b: &StreamInfo) -> SpatialCompat {
    if a.width != b.width || a.height != b.height {
        return SpatialCompat::PixelSizeMismatch;
    }
    if display_aspect_ratio(a) != display_aspect_ratio(b) {
        return SpatialCompat::SarMismatch { a: a.sar, b: b.sar };
    }
    SpatialCompat::Compatible
}
```
- SAR를 StreamInfo의 1급 필드로 항상 파싱/보관하고, 명시되지 않으면 1:1로 기본값을 두되 그 사실을 기록
- 픽셀 치수 비교와 DAR 비교를 별개 체크로 분리해서, "픽셀은 같은데 SAR가 다른" 상황을 별도 경고로 표면화
- SAR가 다른 두 스트림을 비교해야 한다면, 비교 전 SAR를 정규화(정방 픽셀로 리샘플)할지 여부를 명시적 정책으로 결정

**탐지 방법**:
- Semantic: `width == b.width && height == b.height`만으로 "정렬됨"을 판단하는 코드 경로 검색
- Static: StreamInfo/Frame 타입에 SAR 필드가 있는지, 있다면 실제로 비교 로직에서 읽히는지 확인
- Manual: anamorphic SAR를 가진 실제 샘플(DVD/방송 유래 콘텐츠)로 파이프라인을 통과시켜 DAR 계산이 올바른지 검증

**예외**:
- 웹/모바일 전용 파이프라인으로 SAR가 항상 1:1임이 계약으로 보장된 콘텐츠만 다루는 경우(단, 입력 검증에서 이를 강제해야 함)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-008: chroma siting 차이 무시
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn upsample_chroma_to_444(frame: &Yuv420Frame) -> Yuv444Frame {
    // chroma sample location(siting) 정보를 확인하지 않고 항상 동일한 고정 필터로 업샘플
    let cb = bilinear_upsample_2x(&frame.cb);
    let cr = bilinear_upsample_2x(&frame.cr);
    Yuv444Frame { y: frame.y.clone(), cb, cr }
}
```

**문제**:
- YUV 4:2:0에서 chroma 샘플의 실제 위치(siting)는 표준/인코더마다 다르다(예: MPEG-2 스타일 co-sited horizontally, JPEG/H.264 기본 center-siting, left-siting 등) — H.273/VUI의 `chroma_sample_loc_type`이 이를 시그널링한다
- siting 가정이 reference와 distorted 사이에 다르면, 크로마 업샘플 결과가 반 픽셀 단위로 어긋나서 색상 경계(특히 채도가 높은 텍스트/그래픽 오버레이 근처)에서 색번짐처럼 보이는 차이가 생긴다
- 이 차이는 luma에는 영향을 주지 않으므로 PSNR/SSIM을 luma 채널로만 계산하면 드러나지 않다가, 색차 채널을 포함한 메트릭(예: 색차 가중 SSIM)에서만 원인 불명의 감점으로 나타난다
- siting 정보를 아예 시그널링하지 않는 스트림(레거시 콘텐츠)에서는 "기본값"을 무엇으로 가정할지 파이프라인마다 달라 재현성이 깨진다

**발생 조건**:
- 서로 다른 인코더/디코더 구현체가 만든 두 스트림을 비교하며 각각 다른 chroma siting 기본값을 쓸 때
- 4:2:0 콘텐츠를 4:4:4로 업샘플링해 색상 관련 메트릭(색차 SSIM, ΔE 등)을 계산하는 파이프라인
- 레거시(MPEG-2/DV) 유래 콘텐츠와 최신(H.264/HEVC) 인코딩 결과를 비교할 때(기본 siting 관례가 다름)

**권장**:
```rust
fn upsample_chroma_to_444(frame: &Yuv420Frame, siting: ChromaSiting) -> Yuv444Frame {
    let filter = upsample_filter_for_siting(siting); // siting별로 위상이 다른 필터 계수 사용
    let cb = upsample_2x_with_phase(&frame.cb, filter);
    let cr = upsample_2x_with_phase(&frame.cr, filter);
    Yuv444Frame { y: frame.y.clone(), cb, cr }
}

fn align_chroma_siting(reference: &Yuv420Frame, distorted: &Yuv420Frame) -> (ChromaSiting, ChromaSiting) {
    let r = reference.chroma_siting.unwrap_or(ChromaSiting::default_for(reference.origin_codec));
    let d = distorted.chroma_siting.unwrap_or(ChromaSiting::default_for(distorted.origin_codec));
    (r, d) // 다르면 호출자가 인지하고 정책 결정
}
```
- chroma siting을 VUI/시퀀스 헤더에서 파싱해 프레임 메타데이터로 보관하고, 업샘플 필터 선택에 반영
- siting 정보가 없는 경우의 기본값을 코덱/표준별로 문서화하고 하드코딩하지 않는다(COLOR 카테고리의 색공간 기본값 처리와 동일한 원칙)
- reference와 distorted의 siting이 다르면 경고를 발생시키고 리포트에 두 값을 모두 기록

**탐지 방법**:
- Semantic: 업샘플 필터 함수가 siting 파라미터를 받는지, 받는다면 실제로 파싱된 값이 전달되는지 데이터 흐름 확인
- Static: VUI 파서의 `chroma_sample_loc_type` 필드가 프레임 메타데이터까지 전파되는지 확인
- Manual: 알려진 siting 차이를 가진 두 샘플로 색차 채널 메트릭을 비교해 예상되는 편차 범위인지 검증

**예외**:
- luma 전용 메트릭만 계산하는 파이프라인(색차 채널을 아예 다루지 않는 경우)은 이 이슈의 영향을 받지 않음

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-009: reference만 resize하거나 distorted만 resize
**분류**: SPATIAL · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn align_pair(reference: &Frame, distorted: &Frame, target: Size) -> (Frame, Frame) {
    // reference는 원본 그대로 두고 distorted만 target 크기로 바꾼다
    let resized_distorted = resize(distorted, target, ResizeFilter::Bicubic);
    (reference.clone(), resized_distorted)
}
```

**문제**:
- reference를 리사이즈하지 않고 distorted만 리사이즈하면, distorted 쪽에는 리샘플링으로 인한 추가적인 블러/링잉이 생기지만 reference에는 없다 — 이는 인코더가 만든 화질 손상이 아니라 파이프라인이 만든 인공적 차이다
- "reference를 distorted 해상도로 낮춰서 비교"와 "distorted를 reference 해상도로 올려서 비교"는 서로 다른 메트릭 값을 낸다 — 어느 쪽을 택했는지가 결과 해석에 필수적인데 코드만 봐서는 정책이 드러나지 않는 경우가 많다
- 업스케일(낮은 해상도 → 높은 해상도)은 손실된 고주파 정보를 복원하지 못하므로, distorted만 업스케일해서 reference와 비교하면 "업스케일 알고리즘의 한계"가 "인코더 화질"로 오귀속된다
- 이 방향성 선택은 팀/모듈마다 암묵적으로 다르게 정해지기 쉬워 리포트 간 비교 가능성을 해친다

**발생 조건**:
- ABR 인코딩 래더 평가(distorted가 reference보다 낮은 해상도)에서 어느 방향으로 맞출지 정책이 없을 때
- 여러 모듈이 각자 필요에 따라 "reference 기준" 또는 "distorted 기준"으로 다르게 구현했을 때
- 업스케일러가 인코더 자체의 업스케일러(super-resolution 등)와 다른 알고리즘을 쓸 때(SPATIAL-018 참고)

**권장**:
```rust
enum AlignmentDirection {
    UpscaleDistortedToReference,
    DownscaleReferenceToDistorted,
    BothToCommonResolution(Size),
}

fn align_pair(reference: &Frame, distorted: &Frame, policy: AlignmentDirection, filter: ResizeFilter) -> (Frame, Frame) {
    match policy {
        AlignmentDirection::UpscaleDistortedToReference =>
            (reference.clone(), resize(distorted, reference.size(), filter)),
        AlignmentDirection::DownscaleReferenceToDistorted =>
            (resize(reference, distorted.size(), filter), distorted.clone()),
        AlignmentDirection::BothToCommonResolution(target) =>
            (resize(reference, target, filter), resize(distorted, target, filter)),
    }
}
```
- 리사이즈 방향을 명시적 정책 enum으로 표현하고, 리포트에 어떤 정책이 쓰였는지 기록
- 업스케일 방향을 쓸 경우, 가능하면 인코더가 실제로 사용한 업스케일러와 동일한 알고리즘을 참조 구현으로 사용(VMAF 등 일부 메트릭 표준이 권장하는 방식)
- 두 방향 모두 계산해서 리포트에 병기하는 옵션을 제공하면 정책 논쟁을 결과 해석 단계로 미룰 수 있다

**탐지 방법**:
- Structural: `resize` 호출이 reference/distorted 중 한쪽에만 있는 함수를 grep
- Runtime: 동일한 입력 쌍에 대해 두 방향 모두 계산해 점수 차이가 유의미한지 확인하는 진단 테스트
- Manual: 리포트 포맷에 정렬 방향 필드가 있는지, 없다면 추가를 요구

**예외**:
- reference가 항상 distorted와 동일 해상도임이 파이프라인 계약으로 보장되는 경우(리사이즈 자체가 발생하지 않음)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-010: metric마다 다른 전처리를 사용
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_psnr(reference: &Frame, distorted: &Frame) -> f64 {
    let aligned = resize(distorted, reference.size(), ResizeFilter::Bilinear);
    psnr(reference, &aligned)
}

fn compute_ssim(reference: &Frame, distorted: &Frame) -> f64 {
    // PSNR과 다른 필터, 다른 crop 로직을 독립적으로 구현
    let cropped = crop_letterbox(distorted);
    let aligned = resize(&cropped, reference.size(), ResizeFilter::Lanczos3);
    ssim(reference, &aligned)
}
```

**문제**:
- 같은 프레임 쌍에 대해 메트릭마다 다른 전처리(필터, crop 여부, 정렬 방향)를 쓰면 PSNR과 SSIM이 "같은 비교"를 하고 있다는 전제가 깨진다
- 메트릭 간 상관관계 분석이나 "PSNR은 낮은데 SSIM은 높다" 같은 진단이 실제 화질 특성 때문인지 전처리 불일치 때문인지 구분할 수 없게 된다
- 각 메트릭 함수가 독립적으로 전처리를 구현하면 한 곳만 버그를 고치고 다른 곳은 놓치는 일이 반복된다(SPATIAL-002~004의 개별 버그가 메트릭별로 따로 존재할 위험)
- 새 메트릭을 추가할 때마다 전처리를 처음부터 재구현하게 되어 유지보수 비용이 메트릭 수에 비례해 증가한다

**발생 조건**:
- 여러 메트릭(PSNR/SSIM/VMAF/사용자 정의)을 각각 다른 시점에 다른 작성자가 추가했을 때
- 성능 최적화를 이유로 특정 메트릭만 별도의 빠른 경로(다른 리사이즈 구현)를 탔을 때
- 서드파티 메트릭 라이브러리가 자체적으로 내부 전처리를 수행해서 프로젝트 공통 전처리와 이중으로 겹치거나 다르게 동작할 때

**권장**:
```rust
struct AlignedFramePair {
    reference: Frame,
    distorted: Frame,
    preprocessing: PreprocessingRecord,
}

fn align_once(reference: &Frame, distorted: &Frame, config: &SpatialConfig) -> AlignedFramePair {
    // 모든 메트릭이 공유하는 단일 정렬 단계
    let region = detect_letterbox(reference, config.black_threshold);
    let cropped = (crop_to_region(reference, &region), crop_to_region(distorted, &region));
    let aligned = align_pair(&cropped.0, &cropped.1, config.direction, config.filter);
    AlignedFramePair { reference: aligned.0, distorted: aligned.1, preprocessing: config.record() }
}

fn compute_all_metrics(pair: &AlignedFramePair) -> MetricSet {
    MetricSet {
        psnr: psnr(&pair.reference, &pair.distorted),
        ssim: ssim(&pair.reference, &pair.distorted),
        preprocessing: pair.preprocessing.clone(),
    }
}
```
- 정렬(crop/resize/rotation 정규화)을 메트릭 계산과 분리된 단일 단계로 만들고, 모든 메트릭이 그 출력을 공유하게 한다(PIPE 카테고리의 파이프라인 단계 분리 원칙과 연결)
- 특정 메트릭이 라이브러리 내부에서 자체 전처리를 수행한다면(예: 일부 VMAF 구현), 그 사실과 우리 쪽 전처리와의 관계(중복/충돌 여부)를 명시적으로 검증하고 문서화
- 전처리 설정을 코드베이스 전역 단일 소스(config)로 관리

**탐지 방법**:
- Structural: 서로 다른 메트릭 함수들이 각자 resize/crop을 호출하는지, 아니면 공통 정렬 결과를 인자로 받는지 시그니처 검사
- Static: 메트릭 함수 시그니처가 `Frame`을 직접 받는지 `AlignedFramePair`처럼 "이미 정렬됨"을 타입으로 보장하는 구조를 받는지
- Manual: 새 메트릭 추가 시 코드 리뷰 체크리스트에 "공통 정렬 단계를 재사용하는가" 포함

**예외**:
- 서로 다른 메트릭이 원천적으로 다른 해상도/색공간을 요구하도록 표준에 정의되어 있는 경우(예: 특정 메트릭이 항상 특정 해상도로 다운샘플링하도록 규격화됨) — 이 경우도 차이를 명시적으로 문서화해야 함

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-011: 정렬된 영역 밖의 pixel을 0으로 채워 비교
**분류**: SPATIAL · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn align_by_translation(frame: &Frame, dx: i32, dy: i32, canvas: Size) -> Frame {
    let mut out = Frame::filled(canvas, 0); // 검은색(0)으로 초기화한 캔버스
    let (sx, sy) = (dx.max(0) as u32, dy.max(0) as u32);
    blit(&mut out, frame, sx, sy);
    // 원본이 이동한 만큼 비어버린 가장자리는 0(검정)으로 남는다
    out
}
```

**문제**:
- 두 프레임을 서브픽셀/픽셀 단위로 평행이동시켜 정렬할 때, 이동으로 인해 한쪽에서 비게 되는 가장자리를 0(검정)으로 채우면 그 영역은 "완전히 다른 색"으로 취급되어 메트릭에 거짓 대형 오차를 유발한다
- 이 오차는 실제 이동량에 비례해 커지므로, 안정화(stabilization) 차이가 큰 프레임일수록 메트릭이 비정상적으로 나빠진다(SPATIAL-013과 연쇄)
- 검정으로 채우는 대신 반복(edge replication)이나 미러링으로 채우면 오차는 줄지만 이번엔 그 영역이 "완벽히 일치하는 것처럼" 과대평가되어 반대 방향으로 편향된다
- 어느 쪽이든, 채운 영역을 메트릭 계산에서 제외하지 않으면 "정렬로 인해 발생한 인공적 값"이 화질 점수에 섞여 들어간다

**발생 조건**:
- 손떨림 보정/모션 보상 정렬 후 두 프레임의 유효 겹침 영역이 원본 프레임보다 작아지는 경우
- 서브픽셀 정렬(SPATIAL-012)을 위해 한쪽 프레임을 이동시켜 재샘플링할 때
- 정렬 로직이 겹침 영역 계산 없이 항상 원본 캔버스 크기로 결과를 반환하도록 구현되었을 때

**권장**:
```rust
struct AlignedRegion { frame_a: Frame, frame_b: Frame, valid_area: Rect }

fn align_by_translation(a: &Frame, b: &Frame, dx: i32, dy: i32) -> AlignedRegion {
    let valid_area = overlap_rect_after_shift(a.size(), b.size(), dx, dy);
    AlignedRegion {
        frame_a: crop_to_region(a, &valid_area),
        frame_b: crop_to_region(&shift(b, dx, dy), &valid_area),
        valid_area,
    }
}

fn compute_metric(region: &AlignedRegion) -> f64 {
    // 겹치지 않는(패딩된) 영역은 아예 존재하지 않으므로 메트릭 계산에서 자동으로 제외됨
    ssim(&region.frame_a, &region.frame_b)
}
```
- 패딩값을 채워 넣는 대신, 두 프레임이 실제로 겹치는 유효 영역(valid_area)만 잘라내어 메트릭을 계산한다
- 유효 영역의 크기(원본 대비 비율)를 리포트에 함께 기록해서, 정렬로 인해 손실된 비교 영역이 얼마나 되는지 투명하게 드러낸다
- 유효 영역이 너무 작아지면(예: 원본의 90% 미만) 경고를 발생시켜 비정상적인 정렬 결과를 조기에 발견

**탐지 방법**:
- Runtime: 인위적으로 이동시킨 테스트 프레임 쌍으로 정렬 후 가장자리 픽셀 값을 검사해 0/미러링으로 채워졌는지 확인
- Static: 정렬 함수의 반환 타입이 "유효 영역" 정보를 포함하는지 검사
- Manual: HEAT 카테고리 히트맵에서 프레임 가장자리에 비정상적으로 균일한 고오차 띠가 나타나는지 확인

**예외**:
- 이동량이 서브픽셀 수준으로 매우 작아(1픽셀 미만) 가장자리 손실이 무시 가능한 수준이고, 그 사실이 정량적으로 검증된 경우

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-012: sub-pixel translation을 무시
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn align_frames(reference: &Frame, distorted: &Frame) -> (Frame, Frame) {
    // 정수 픽셀 단위로만 이동량을 추정하고 반올림해서 적용
    let (dx, dy) = estimate_shift_integer(reference, distorted);
    (reference.clone(), shift_integer_pixels(distorted, dx, dy))
}
```

**문제**:
- 실제 카메라 흔들림이나 재인코딩 파이프라인에서 발생하는 이동은 정수 픽셀 단위가 아닌 경우가 대부분이다(예: 0.3픽셀 이동)
- 이동량을 정수로 반올림해서 정렬하면 반올림 오차만큼 잔여 정렬 불일치가 남고, 이는 고주파(디테일이 많은 텍스처, 에지) 영역에서 메트릭을 실제보다 나쁘게 만든다
- 서브픽셀 이동을 무시한 채 정렬했다는 사실이 리포트에 드러나지 않으면, 낮은 메트릭 점수가 "압축 손상"인지 "정렬 잔차"인지 구분할 수 없다
- 특히 PSNR처럼 픽셀별 절대 차이에 민감한 메트릭은 서브픽셀 어긋남에 큰 폭으로 반응한다(SSIM보다 훨씬 민감)

**발생 조건**:
- 손떨림 보정, 리샘플링, 프레임레이트 변환 등을 거친 콘텐츠를 원본과 비교할 때
- 정렬 알고리즘이 정수 픽셀 상관관계(cross-correlation) 기반으로만 이동량을 추정하고 보간을 통한 서브픽셀 정밀화 단계가 없을 때
- 실시간성이 중요해 서브픽셀 정렬의 계산 비용을 의도적으로 생략했으나 그 트레이드오프가 문서화되지 않았을 때

**권장**:
```rust
fn align_frames(reference: &Frame, distorted: &Frame) -> (Frame, Frame, SubpixelShift) {
    let coarse = estimate_shift_integer(reference, distorted);
    let refined = refine_shift_subpixel(reference, distorted, coarse); // 예: 파라볼릭 피팅, 위상 상관
    let resampled = shift_subpixel(distorted, refined, ResizeFilter::Lanczos3);
    (reference.clone(), resampled, refined)
}
```
- 정수 픽셀 추정 이후, 보간 기반 서브픽셀 정밀화 단계(파라볼릭 피팅, 위상 상관 등)를 추가한다
- 추정된 서브픽셀 이동량을 리포트에 기록해서, 큰 잔여 이동이 감지되면 원인(정렬 실패 vs 실제 모션)을 조사할 수 있게 한다
- 서브픽셀 재샘플링에 쓰인 보간 필터도 SPATIAL-002와 동일하게 기록 대상에 포함

**탐지 방법**:
- Semantic: 이동량 추정 함수의 반환 타입이 정수(i32)인지 부동소수점(f64)인지 확인
- Runtime: 알려진 서브픽셀 이동량(예: 0.5픽셀)을 인위적으로 적용한 합성 테스트 쌍으로 정렬 후 잔여 오차 측정
- Manual: 정적 콘텐츠(움직임이 거의 없는 장면)에서 예상외로 낮은 메트릭이 나오는 프레임을 리뷰

**예외**:
- 두 스트림이 완전히 동일한 인코딩 파이프라인의 산출물로 서브픽셀 이동이 발생할 여지가 없음이 보장된 경우(예: 동일 소스의 서로 다른 비트레이트 인코딩만 비교하는 파이프라인)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-013: stabilization 차이를 압축 손상으로 해석
**분류**: SPATIAL · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_frame_scores(reference: &Frame, distorted: &Frame) -> f64 {
    // reference와 distorted 중 하나에만 손떨림 보정(stabilization)이 적용된 상태를
    // 고려하지 않고 그대로 프레임 단위 SSIM만 계산
    ssim(reference, distorted)
}
```

**문제**:
- 인코딩/트랜스코딩 파이프라인에 손떨림 보정이나 디지털 안정화가 포함되어 있으면, distorted 프레임은 reference 대비 프레임 전체가 미세하게(때로는 크게) 이동/워프되어 있다
- 이 전역적인 기하학적 차이를 SPATIAL-012처럼 단순 평행이동으로만 보정하려 하면, 안정화가 회전/스케일/원근 왜곡까지 포함하는 경우 남는 잔차가 여전히 크다
- 잔차가 큰 상태로 메트릭을 계산하면 점수가 크게 떨어지고, 이를 "인코더가 화질을 심각하게 손상시켰다"고 잘못 해석하게 된다 — 실제로는 인코딩 손상이 아니라 안정화로 인한 프레임 콘텐츠 자체의 차이
- 반대로, 안정화 차이를 완전히 흡수하는 강력한 전역 정렬을 무분별하게 적용하면 이번엔 실제 압축 손상(블러, 블록 아티팩트)까지 정렬 알고리즘이 "보정"해버려 점수가 과장되게 좋아질 수 있다

**발생 조건**:
- 모바일 카메라 원본(비안정화) reference와, 업로드/트랜스코딩 과정에서 안정화가 적용된 distorted를 비교할 때
- 인코더 파이프라인에 사전처리(pre-processing) 단계로 디지털 손떨림 보정이 포함되어 있는데 이 사실이 QC 파이프라인에 전달되지 않았을 때
- reference 자체가 이미 안정화된 상태로 캡처되어 두 소스의 안정화 "정도"가 다를 때(둘 다 안정화되었지만 강도가 다름)

**권장**:
```rust
fn compute_frame_scores(reference: &Frame, distorted: &Frame, expected_transform: TransformClass) -> ScoreWithDiagnostics {
    let global_motion = estimate_global_transform(reference, distorted); // affine/homography 추정
    if global_motion.magnitude() > expected_transform.tolerance() {
        // 안정화로 설명되지 않는 수준의 전역 이동이면 별도 플래그로 표시
        return ScoreWithDiagnostics::flagged(global_motion);
    }
    let compensated = warp(distorted, &global_motion.inverse());
    ScoreWithDiagnostics::scored(ssim(reference, &compensated), global_motion)
}
```
- 프레임 단위 평행이동을 넘어선 전역 기하 변환(affine/homography) 추정을 정렬 단계에 포함하되, 추정된 변환의 크기를 항상 리포트에 남긴다
- 파이프라인 메타데이터(어느 스트림에 안정화가 적용되었는지)를 QC 단계까지 전달해서, 전역 이동이 "예상된 안정화"인지 "비정상 정렬 실패"인지 구분한다
- 전역 변환 보정을 지역(local) 압축 아티팩트 평가와 분리 — 보정은 전역 기하에만 적용하고, 그 이후 지역적 차이만 화질 메트릭으로 취급

**탐지 방법**:
- Semantic: 파이프라인이 프레임 단위 평행이동 정렬만 수행하는지, 회전/스케일까지 다루는 전역 정렬을 수행하는지 확인
- Runtime: 안정화가 적용된 것으로 알려진 실제 콘텐츠 쌍으로 골든 테스트를 만들어 플래그가 올바르게 발생하는지 검증
- Manual: 비정상적으로 낮은 프레임 점수가 특정 장면(카메라 이동이 많은 구간)에 집중되는지 시계열로 리뷰(STAT/HEAT 카테고리와 연계)

**예외**:
- 두 스트림 모두 안정화가 적용되지 않았거나, 동일한 안정화 알고리즘/강도로 처리되었음이 파이프라인 계약으로 보장된 경우

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-014: frame별 crop 영역이 흔들림
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn crop_content_region(frame: &Frame, black_threshold: u8) -> Frame {
    // 매 프레임마다 독립적으로 letterbox 경계를 재탐지
    let region = detect_letterbox(frame, black_threshold);
    crop_to_region(frame, &region)
}
```

**문제**:
- letterbox/콘텐츠 영역 감지를 프레임마다 독립적으로 수행하면, 노이즈나 어두운 장면(암전, 페이드) 때문에 감지된 경계가 프레임마다 1~2픽셀씩 흔들릴 수 있다
- crop 영역이 프레임마다 다르면 결과적으로 각 프레임이 서로 다른 (미세하게 다른) 좌표계로 잘려 비교되는 셈이 되어, 프레임 간 메트릭 값의 일관성이 깨지고 시계열 그래프에 인공적인 노이즈가 낀다
- 특히 페이드 인/아웃 장면이나 순수 암전 프레임에서는 "콘텐츠 영역"과 "검은 여백"의 구분 자체가 모호해 감지기가 요동치기 쉽다
- 이 흔들림은 STAT 카테고리의 프레임별 통계 집계(평균/분산/이상치 탐지)를 왜곡시켜, 실제로는 안정적인 화질 구간을 "불안정하다"고 오판하게 만든다

**발생 조건**:
- 시네마스코프/방송 콘텐츠처럼 고정된 letterbox를 가진 영상에서 프레임별 재탐지를 수행하는 파이프라인
- 페이드 트랜지션, 암전 구간, 매우 어두운 장면이 포함된 콘텐츠
- 감지 임계값(black_threshold)이 노이즈에 민감하게 설정되어 있을 때

**권장**:
```rust
fn detect_stable_content_region(frames: &[Frame], black_threshold: u8, sample_count: usize) -> ContentRegion {
    // 시퀀스 시작 시 여러 프레임(암전/페이드 구간 제외)을 샘플링해 한 번만 결정
    let samples = sample_non_dark_frames(frames, sample_count);
    let candidates: Vec<ContentRegion> = samples.iter().map(|f| detect_letterbox(f, black_threshold)).collect();
    stabilize_region(&candidates) // 중앙값/최빈값 기반으로 단일 영역 확정
}

fn crop_content_region(frame: &Frame, region: &ContentRegion) -> Frame {
    // 시퀀스 전체에 걸쳐 동일한 region을 재사용
    crop_to_region(frame, region)
}
```
- letterbox/콘텐츠 영역은 시퀀스(GOP 또는 전체 클립) 단위로 한 번 안정적으로 결정하고, 이후 모든 프레임에 동일하게 적용한다
- 감지에 쓰는 샘플 프레임은 암전/페이드 구간을 제외하고 선택(밝기 히스토그램으로 필터링)
- 감지된 영역이 시퀀스 중간에 실제로 바뀌어야 하는 경우(예: 콘텐츠 자체가 종횡비를 바꾸는 편집물)를 위한 명시적 재탐지 트리거(장면 전환 감지와 연동)를 별도로 둔다

**탐지 방법**:
- Runtime: 동일 letterbox 콘텐츠의 연속 프레임들에 대해 감지된 crop 영역의 프레임 간 분산을 측정 — 0에 가까워야 정상
- Static: `detect_letterbox`가 프레임 루프 내부에서 매번 호출되는지, 시퀀스 레벨에서 한 번만 호출되는지 구조 검사
- Manual: 시계열 메트릭 그래프에서 콘텐츠에 실제 변화가 없는데도 프레임 간 점수가 미세하게 요동치는 구간을 리뷰

**예외**:
- 콘텐츠 자체가 편집으로 종횡비/여백을 프레임마다 바꾸는 특수한 경우(뮤직비디오 등)에는 프레임별(또는 장면별) 재탐지가 실제로 필요 — 이 경우 재탐지 경계를 장면 전환 지점으로 한정해 흔들림을 최소화해야 한다

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-015: 전처리 이력을 결과에 저장하지 않음
**분류**: SPATIAL · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct ComparisonReport {
    frame_scores: Vec<f64>,
    overall_score: f64,
    // resize 여부, 필터, crop 영역, 회전 보정, 정렬 방향 등 어떤 전처리도 기록되지 않는다
}
```

**문제**:
- SPATIAL-001~014에서 다룬 모든 결정(리사이즈 여부/필터, crop 순서/영역, coded/display 구분, letterbox 처리, 회전/SAR/chroma siting 보정, 정렬 방향, 서브픽셀 보정, 안정화 보정)은 최종 점수에 직접 영향을 준다
- 이 이력이 리포트에 없으면, 같은 두 영상을 나중에 다시 비교했을 때 다른 점수가 나와도 원인을 추적할 방법이 없다(파이프라인 버전이 바뀌었는지, 우연한 편차인지 구분 불가)
- 리포트를 받은 사람(엔지니어, 이해관계자)이 "이 점수를 신뢰해도 되는가"를 판단할 근거가 없다 — 예를 들어 큰 crop이 적용되었다면 비교 영역이 작아 통계적 신뢰도가 낮을 수 있는데 그 정보가 없다
- A/B 리포트 비교, 회귀 테스트, 감사(audit) 모두 전처리 이력이 없으면 "블랙박스 숫자"에 불과해진다

**발생 조건**:
- 리포트 스키마를 초기에 "점수만" 담도록 설계하고 이후 전처리 로직이 늘어나면서 스키마를 갱신하지 않았을 때
- 여러 팀이 각자 리포트 포맷을 확장하면서 전처리 필드 표준화 없이 임시방편으로 로그에만 남길 때
- 성능/저장공간을 이유로 "필요 없어 보이는" 메타데이터를 생략했을 때

**권장**:
```rust
#[derive(Serialize)]
struct PreprocessingRecord {
    coded_size: Size,
    display_window: Rect,
    letterbox_region: Option<ContentRegion>,
    rotation_deg: u16,
    sar: SampleAspectRatio,
    chroma_siting: Option<ChromaSiting>,
    resize_filter: Option<ResizeFilter>,
    alignment_direction: Option<AlignmentDirection>,
    subpixel_shift: Option<SubpixelShift>,
    global_transform: Option<TransformEstimate>,
    pipeline_version: String,
}

struct ComparisonReport {
    frame_scores: Vec<f64>,
    overall_score: f64,
    preprocessing: PreprocessingRecord, // 결과와 항상 함께 다닌다
}
```
- 전처리 관련 모든 결정을 하나의 구조체로 모아 리포트에 필수 필드로 포함시킨다(옵셔널이 아니라 "적용 안 됨"도 명시적으로 표현)
- 파이프라인 버전(코드 버전 또는 정책 버전)을 기록해서, 향후 로직 변경 시 과거 리포트와의 비교 가능 여부를 판단할 수 있게 한다
- 리포트 직렬화 스키마에 이 필드를 추가할 때 하위 호환성(과거 리포트 로딩)을 고려

**탐지 방법**:
- Structural: 리포트/결과 구조체 정의에 전처리 관련 필드가 있는지 타입 검사
- Static: 리포트 생성 함수가 정렬 단계에서 나온 `PreprocessingRecord`를 실제로 전달받는지 시그니처 확인
- Manual: "이 점수만 보고 재현 가능한가?"를 리포트 포맷 리뷰의 표준 질문으로 포함

**예외**:
- 완전히 동일한 해상도/색공간/방향의 두 스트림만 다루도록 입력이 사전 검증되어 전처리가 원천적으로 발생하지 않는 파이프라인(그래도 "전처리 없음"이라는 사실 자체는 기록하는 편이 안전)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-016: 필드 순서(interlaced) 불일치로 인한 수직 정렬 오류
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn deinterlace_and_compare(reference: &InterlacedFrame, distorted: &InterlacedFrame) -> f64 {
    // 두 스트림의 field order(top-field-first vs bottom-field-first)를 확인하지 않고
    // 항상 동일한 순서로 필드를 분리/병합
    let r = weave_fields(&reference.top, &reference.bottom);
    let d = weave_fields(&distorted.top, &distorted.bottom);
    ssim(&r, &d)
}
```

**문제**:
- 인터레이스 콘텐츠는 top-field-first(TFF)와 bottom-field-first(BFF) 두 가지 필드 순서가 있으며, 이는 각 필드가 표현하는 수직 위치(짝수/홀수 라인)를 결정한다
- reference와 distorted의 필드 순서 가정이 다르면, 디인터레이스나 필드 weave 결과가 수직으로 반 라인(various 필드 간격)만큼 어긋난 상태로 비교된다 — 이는 SPATIAL-012의 서브픽셀 이동과 유사하지만 수직 방향으로 구조적으로 고정된 오차다
- 특히 모션이 있는 장면에서는 필드 순서 오류가 "빗질(combing)" 아티팩트처럼 보이는 큰 오차를 만들어, 실제로는 정렬 문제인데 압축/디인터레이스 손상으로 오인하기 쉽다
- 트랜스코딩 과정에서 필드 순서가 뒤바뀌는(swap) 버그는 드물지 않으며, 픽셀 치수만으로는 감지되지 않는다

**발생 조건**:
- 방송/레거시 인터레이스 콘텐츠(576i/480i, 일부 1080i)를 다루는 파이프라인
- 서로 다른 트랜스코더/디인터레이서가 필드 순서 메타데이터를 다르게 해석하거나 무시할 때
- 인터레이스 → 프로그레시브 변환(deinterlace) 알고리즘이 reference/distorted에 서로 다르게 적용될 때

**권장**:
```rust
fn deinterlace_and_compare(reference: &InterlacedFrame, distorted: &InterlacedFrame) -> f64 {
    let r_order = reference.field_order.expect("field order must be signaled or configured");
    let d_order = distorted.field_order.expect("field order must be signaled or configured");
    let r = weave_fields_with_order(&reference.top, &reference.bottom, r_order);
    let d = weave_fields_with_order(&distorted.top, &distorted.bottom, d_order);
    if r_order != d_order {
        log::warn!("field order mismatch: reference={:?} distorted={:?}", r_order, d_order);
    }
    ssim(&r, &d)
}
```
- 필드 순서를 메타데이터에서 명시적으로 읽어 각 스트림에 독립적으로 적용하고, 불일치 시 경고를 발생시킨다
- 디인터레이스 알고리즘 자체도 SPATIAL-002처럼 어떤 알고리즘/설정을 썼는지 리포트에 기록
- 가능하면 인터레이스 콘텐츠는 필드 단위로 별도 비교하는 옵션을 제공(weave 후 프로그레시브 비교보다 필드별 비교가 필드 순서 오류에 덜 민감)

**탐지 방법**:
- Semantic: field order를 읽는 파싱 코드와 weave/deinterlace 함수 사이의 데이터 연결 확인
- Runtime: TFF/BFF를 인위적으로 뒤바꾼 합성 테스트 쌍으로 회귀 테스트
- Manual: 인터레이스 콘텐츠에서 모션이 많은 장면에 국한해 비정상적으로 낮은 점수가 나오는지 리뷰

**예외**:
- 프로그레시브 콘텐츠만 다루는 파이프라인은 이 항목과 무관(입력 검증에서 인터레이스 콘텐츠를 명시적으로 거부하는 것이 안전)

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-017: crop 후 홀수 크기로 인한 chroma plane 정렬 붕괴
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn crop_to_region(frame: &Yuv420Frame, region: &ContentRegion) -> Yuv420Frame {
    // luma 좌표를 그대로 반으로 나눠 chroma crop 좌표를 계산 — 반올림/정렬을 고려하지 않음
    let luma = crop_plane(&frame.y, region);
    let chroma_region = ContentRegion {
        x: region.x / 2, y: region.y / 2,
        width: region.width / 2, height: region.height / 2,
    };
    let cb = crop_plane(&frame.cb, &chroma_region);
    let cr = crop_plane(&frame.cr, &chroma_region);
    Yuv420Frame { y: luma, cb, cr }
}
```

**문제**:
- letterbox/사용자 지정 crop 영역의 x/y/width/height가 홀수(odd)이면, 4:2:0/4:2:2처럼 서브샘플링된 크로마 평면에서 좌표를 2로 나눌 때 반올림 방식에 따라 실제로는 존재하지 않는 반 픽셀 위치를 참조하게 된다
- crop 시작 좌표가 홀수면 크로마 서브샘플링의 위상(어느 luma 픽셀 쌍이 하나의 크로마 샘플에 대응하는지)이 완전히 뒤바뀌어, 크로마 평면이 원래 대응해야 할 luma 위치와 다른 색 정보를 갖게 된다
- 이 오류는 luma 채널 메트릭에는 드러나지 않고 색차 채널에서만 나타나므로 luma 전용 PSNR/SSIM으로는 절대 감지되지 않는다
- reference와 distorted의 crop 좌표 반올림 방식이 미묘하게 다르면(예: 한쪽은 floor, 다른 쪽은 round) 색상 정렬이 위상 어긋난 채로 비교되어 색번짐처럼 보이는 인공적 차이가 생긴다

**발생 조건**:
- 자동 letterbox 감지(SPATIAL-005)나 사용자 지정 crop 영역의 좌표가 짝수 정렬을 보장하지 않을 때
- 4:2:0/4:2:2 콘텐츠에 대해 crop 로직이 luma 좌표계로만 작성되고 크로마 서브샘플링 위상을 별도로 검증하지 않을 때
- 여러 crop 연산이 체이닝되며(letterbox crop → 사용자 지정 crop) 매번 반올림이 누적될 때

**권장**:
```rust
fn crop_to_region(frame: &Yuv420Frame, region: &ContentRegion) -> Result<Yuv420Frame, CropAlignmentError> {
    // 4:2:0 서브샘플링에 안전하려면 crop 좌표/크기가 짝수여야 함을 강제
    let aligned = region.align_to_chroma_grid(SubsamplingFormat::Yuv420)?;
    let luma = crop_plane(&frame.y, &aligned);
    let chroma_region = ContentRegion {
        x: aligned.x / 2, y: aligned.y / 2,
        width: aligned.width / 2, height: aligned.height / 2,
    };
    let cb = crop_plane(&frame.cb, &chroma_region);
    let cr = crop_plane(&frame.cr, &chroma_region);
    Ok(Yuv420Frame { y: luma, cb, cr })
}
```
- crop 좌표/크기를 서브샘플링 포맷(4:2:0은 2, 4:2:2는 수평만 2 등)에 맞춰 그리드에 정렬하도록 강제하는 함수를 crop 진입점에 둔다(정렬 불가능하면 명시적 오류 반환)
- 정렬을 위해 좌표를 조정해야 한다면(예: 홀수 → 짝수로 1픽셀 확장/축소) 그 조정량을 로그/리포트에 남긴다
- reference와 distorted에 동일한 반올림 규칙(항상 floor 등)을 하나의 공유 함수로 강제

**탐지 방법**:
- Runtime: 의도적으로 홀수 좌표의 crop 영역을 주입해 색차 채널 메트릭이 비정상적으로 나빠지는지 회귀 테스트
- Static: crop 함수가 서브샘플링 포맷을 인자로 받아 정렬을 검증하는지, 아니면 무조건 좌표를 2로 나누기만 하는지 코드 검사
- Manual: 크로마 채널만 별도로 시각화(색차 히트맵)해서 luma 대비 부자연스러운 오프셋이 있는지 확인

**예외**:
- crop 영역이 항상 짝수 그리드에 정렬되도록 상류(letterbox 감지, UI 입력 검증)에서 이미 보장하는 경우

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### SPATIAL-018: ABR 래더 비교 시 업스케일 알고리즘이 인코더와 불일치
**분류**: SPATIAL · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn compare_ladder_rung(reference_4k: &Frame, distorted_1080p: &Frame) -> f64 {
    // distorted를 reference 해상도로 올릴 때 항상 bicubic만 사용
    // 실제 재생 환경(플레이어/디스플레이)의 업스케일러와는 무관한 임의의 선택
    let upscaled = resize(distorted_1080p, reference_4k.size(), ResizeFilter::Bicubic);
    ssim(reference_4k, &upscaled)
}
```

**문제**:
- ABR 래더(여러 해상도로 인코딩된 스트림 집합)를 평가할 때, 낮은 해상도로 인코딩된 rung을 원본 해상도로 올려 reference와 비교하는 것이 표준 관행(예: VMAF의 권장 방식)인데, 이때 쓰는 업스케일 알고리즘 선택이 결과에 큰 영향을 준다
- 평가에 사용하는 업스케일러가 실제 재생 환경(TV, 플레이어, 브라우저)에서 쓰이는 업스케일러와 다르면, 점수가 "실제 시청자가 보는 화질"과 괴리된다 — 특정 인코더가 저해상도에서 더 선명하게 압축했더라도 평가용 업스케일러가 그 이점을 살리지 못하면 점수에 반영되지 않는다
- 서로 다른 rung(720p, 1080p, 4K)을 같은 업스케일러로 비교하면 "어느 rung이 최종 화질에 유리한가"라는 래더 설계 질문에 답할 때, 업스케일러의 특성(고주파 보존/억제 성향)이 결과를 왜곡시킨다
- 인코더 자체에 내장된 super-resolution/업스케일 후처리가 있는 경우(일부 최신 코덱 확장), 평가 파이프라인이 이를 반영하지 않으면 그 인코더만 부당하게 낮은 점수를 받는다

**발생 조건**:
- ABR 스트리밍 서비스의 인코딩 래더 설계/튜닝을 위해 여러 rung을 비교 평가할 때
- 평가 기준을 표준 메트릭 도구(예: VMAF)의 기본 업스케일 정책에 의존하면서, 그 정책이 실제 배포 환경과 다르다는 점을 인지하지 못했을 때
- 서로 다른 인코더 벤더를 비교하는데 한쪽만 인코더 내장 업스케일러 정보를 제공할 때

**권장**:
```rust
enum UpscaleReference {
    StandardFilter(ResizeFilter),          // 업계 표준 평가 필터(예: VMAF 기본값)로 고정
    PlaybackEnvironment(PlayerUpscaler),   // 실제 배포 대상 플레이어의 업스케일러 재현
    EncoderNative,                          // 인코더가 자체 제공하는 업스케일 결과를 그대로 사용
}

fn compare_ladder_rung(reference: &Frame, distorted: &Frame, upscale: &UpscaleReference) -> ScoreWithContext {
    let upscaled = apply_upscale_reference(distorted, reference.size(), upscale);
    ScoreWithContext { score: ssim(reference, &upscaled), upscale_policy: upscale.describe() }
}
```
- 업스케일 정책을 평가 목적(표준화된 비교 vs 실제 시청 경험 재현)에 맞춰 명시적으로 선택하고 리포트에 기록
- 여러 정책으로 동일 rung을 평가해 정책 민감도를 함께 리포트하면, 특정 업스케일러 선택에 결과가 과도하게 의존하는지 드러낼 수 있다
- 인코더가 자체 업스케일 후처리를 제공하는 경우, 평가 파이프라인이 그 출력을 인식하고 사용할 수 있는 훅을 마련

**탐지 방법**:
- Semantic: 래더 비교 코드에서 업스케일 필터가 하드코딩되어 있는지, 정책으로 추상화되어 있는지 확인
- Runtime: 동일 rung에 대해 여러 업스케일 필터로 점수를 계산해 편차가 얼마나 큰지 정량화하는 진단 테스트
- Manual: 래더 설계 리포트를 받는 이해관계자에게 "이 점수가 어떤 재생 환경을 가정하는가"를 설명할 수 있는지 리뷰

**예외**:
- 업계 표준 벤치마크(예: 특정 표준 기구의 고정 평가 프로토콜)를 그대로 재현하는 것이 목적인 경우, 표준이 지정한 필터를 그대로 쓰는 것이 올바르다 — 단 그 표준을 명시해야 한다

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
