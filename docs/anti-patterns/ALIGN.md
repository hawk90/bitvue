# Anti-Pattern Catalog — ALIGN: 프레임 정렬 (VQ-Probe domain)

이 문서는 더 큰 안티패턴 카탈로그의 일부입니다 (전체 색인은 별도로 작성 중인 `docs/anti-patterns/INDEX.md` 참조). ALIGN/SPATIAL/COLOR/METRIC/PIPE/HEAT/STAT는 VQ-Probe 도메인(듀얼 스트림 품질 비교) 카탈로그를 구성하며, Bitstream-Analyzer 도메인(OWN/MEM/LAYOUT/PARSE/CODEC 등)과는 아키텍처적으로 분리된 별개의 절반입니다.

## 기준 데이터 모델

정렬(alignment) 관련 결정은 묵시적으로 적용되어서는 안 되며, 다른 메트릭 실행 메타데이터와 마찬가지로 명시적으로 기록되어야 합니다. 아래 구조체는 이 카탈로그의 여러 항목이 참조하는 기준점입니다.

```rust
struct MetricRunMetadata {
    metric: MetricId,
    implementation_version: String,
    model_version: Option<String>,
    preprocessing: PreprocessConfig,
    alignment: AlignmentConfig,
    aggregation: AggregationConfig,
}
```

`alignment` 필드는 "어떤 정렬 방법을 썼는가, offset이 얼마인가, confidence는 얼마인가"를 메트릭 결과와 함께 영구히 보존해야 합니다 — 정렬을 조용히 적용하고 버리면 결과의 재현성과 신뢰성을 검증할 방법이 사라집니다.

---

### ALIGN-001: frame index만으로 원본·왜곡 영상 정렬
**분류**: ALIGN · **심각도**: Critical · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
fn compare_streams(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> Vec<FrameScore> {
    reference.iter()
        .zip(distorted.iter())
        .map(|(r, d)| compute_metric(r, d))
        .collect()
}
```

**문제**:
- 두 스트림의 프레임 개수와 순서가 정확히 같다는 근거 없는 가정에 의존한다.
- 인코더 지연, 프레임 드롭/중복, B-frame 재정렬이 하나라도 있으면 index N이 서로 다른 시각의 프레임을 가리키게 된다.
- 오정렬된 프레임 쌍을 비교하면 메트릭 점수가 완전히 무의미해지지만, 파이프라인은 아무 오류 없이 "정상적으로" 결과를 낸다.

**발생 조건**:
- 원본과 왜곡본이 서로 다른 인코더/설정으로 생성된 경우.
- 프레임 레이트 변환, VFR 소스, 스트리밍 중 패킷 손실로 인한 드롭.

**권장**:
```rust
fn compare_streams(
    reference: &[DecodedFrame],
    distorted: &[DecodedFrame],
    alignment: &AlignmentConfig,
) -> Result<Vec<FrameScore>, AlignError> {
    let mapping = align_frames(reference, distorted, alignment)?; // PTS/컨텐츠 기반
    mapping.pairs.iter()
        .map(|p| Ok(compute_metric(&reference[p.ref_idx], &distorted[p.dist_idx])))
        .collect()
}
```
- PTS를 공통 timebase로 환산하거나 컨텐츠 기반(해시/상관도) 매핑을 우선 사용한다.
- 정렬 결과(mapping)를 `AlignmentConfig`/메타데이터로 보존한다.

**탐지 방법**:
- Structural: `zip`/`enumerate`로 두 프레임 시퀀스를 직접 짝짓는 코드 검색.
- Runtime: 의도적으로 프레임을 드롭/중복시킨 fixture로 회귀 테스트.

**예외**:
- 동일 인코더·동일 설정으로 생성되어 프레임 수·순서가 bit 단위로 보장된 lossless 파이프라인 테스트.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-002: PTS를 float 초로 변환한 뒤 비교
**분류**: ALIGN · **심각도**: High · **탐지**: Static|Runtime

**나쁜 예**:
```rust
fn pts_matches(a_pts: i64, a_tb: (i32, i32), b_pts: i64, b_tb: (i32, i32)) -> bool {
    let a_sec = a_pts as f64 * a_tb.0 as f64 / a_tb.1 as f64;
    let b_sec = b_pts as f64 * b_tb.0 as f64 / b_tb.1 as f64;
    (a_sec - b_sec).abs() < 0.001 // "적당히" 잡은 epsilon
}
```

**문제**:
- f64는 큰 PTS 값(장시간 스트림)에서 유효 정밀도를 잃어 인접 타임스탬프가 같은 값으로 뭉개질 수 있다.
- epsilon을 경험적으로 고른 상수로 두면, timebase가 다른 스트림 쌍에서는 너무 느슨하거나 너무 엄격해진다.
- 프레임마다 반올림 오차가 누적되어 스트림 뒷부분으로 갈수록 체계적으로 드리프트한다.

**발생 조건**:
- 장시간(수시간) 스트림, timebase 분모가 큰 컨테이너(예: 1/90000).
- PTS 경계값 근처에서 float 반올림 방향이 뒤집히는 경우.

**권장**:
```rust
fn pts_matches(a_pts: i64, a_tb: (i64, i64), b_pts: i64, b_tb: (i64, i64), tol_ticks: i64) -> bool {
    // 정수 교차곱으로 유리수 비교 (float 변환 없음)
    let lhs = a_pts * a_tb.0 * b_tb.1;
    let rhs = b_pts * b_tb.0 * a_tb.1;
    let scaled_tol = tol_ticks * a_tb.0 * b_tb.1; // 허용 오차도 동일 스케일로
    (lhs - rhs).abs() <= scaled_tol
}
```
- 가능하면 끝까지 정수/유리수(분수) 표현을 유지하고, 비교 시점에만 필요하면 정수 교차곱을 쓴다.
- 허용 오차는 timebase 단위(tick) 기준으로 정의한다.

**탐지 방법**:
- Static: PTS 근처에서 `as f64` 캐스팅 검색.
- Runtime: 장시간(수 시간) 합성 스트림으로 차등 테스트, 누적 오차 관찰.

**예외**:
- 수 초 내외의 짧은 클립을 다루는 디버깅/프로토타입 도구로, 스코어링에는 쓰이지 않는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-003: time base 변환에서 정수 오차 발생
**분류**: ALIGN · **심각도**: High · **탐지**: Static|Runtime

**나쁜 예**:
```rust
fn rescale_pts(pts: i64, from_tb: (i64, i64), to_tb: (i64, i64)) -> i64 {
    // 절단(truncation) 방향이 항상 같아 편향된 오차가 누적된다
    pts * from_tb.0 * to_tb.1 / (from_tb.1 * to_tb.0)
}
```

**문제**:
- 정수 나눗셈은 항상 0을 향해(또는 항상 아래로) 절단하므로, 무작위 오차가 아니라 방향성 있는 편향(bias)이 누적된다.
- 두 스트림의 timebase가 다르면(예: 1/25 vs 1/90000) 매 프레임마다 조금씩 어긋나 장시간에 걸쳐 프레임 하나 이상의 드리프트로 커진다.
- 오차가 "일관되게 같은 방향"이라 짧은 테스트로는 문제가 드러나지 않고 실제 장시간 스트림에서만 발견된다.

**발생 조건**:
- 원본·왜곡 스트림의 timebase가 서로 다를 때(다른 컨테이너/먹서 사용).
- 긴 시퀀스, 특히 짝수/홀수 timebase 조합처럼 나눗셈이 딱 떨어지지 않는 경우.

**권장**:
```rust
enum RoundMode { NearInf, Down, Up }

fn rescale_pts_rnd(pts: i64, from_tb: (i64, i64), to_tb: (i64, i64), mode: RoundMode) -> i64 {
    let num = pts as i128 * from_tb.0 as i128 * to_tb.1 as i128;
    let den = from_tb.1 as i128 * to_tb.0 as i128;
    let q = match mode {
        RoundMode::NearInf => (num + den / 2) / den, // ffmpeg av_rescale_rnd 유사
        RoundMode::Down => num.div_euclid(den),
        RoundMode::Up => (num + den - 1).div_euclid(den),
    };
    q as i64
}
```
- 반올림 모드를 명시적으로 선택하고(가능하면 nearest), 전체 파이프라인에서 일관되게 적용한다.
- 가능하면 애초에 rescale을 최소화하도록 공통 timebase 하나로 통일해 처리한다.

**탐지 방법**:
- Static: rescale/timebase 변환 함수에서 반올림 없는 정수 나눗셈 검색.
- Runtime: 알려진 유리수 변환 쌍에 대한 단위 테스트로 기대값과 비교.

**예외**:
- 두 스트림이 이미 동일한 timebase를 공유하여 rescale이 아예 필요 없는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-004: duplicate frame을 무조건 정상 프레임으로 비교
**분류**: ALIGN · **심각도**: Medium · **탐지**: Runtime|Semantic

**나쁜 예**:
```rust
fn score_all(pairs: &[(DecodedFrame, DecodedFrame)]) -> Vec<f64> {
    pairs.iter().map(|(r, d)| psnr(r, d)).collect()
    // 텔레시네/VFR->CFR 패딩으로 생긴 중복 프레임을 구분하지 않음
}
```

**문제**:
- 텔레시네(pulldown), VFR→CFR 변환 시 프레임 반복 삽입 등으로 인해 "같은 내용이 여러 번" 나타날 수 있는데 이를 구분하지 않는다.
- 중복 프레임을 서로 다른 실제 프레임과 짝지으면 점수가 왜곡되고, 반대로 중복끼리 짝지어지면 비정상적으로 완벽한 점수가 통계를 오염시킨다.
- 평균/집계 단계에서 이 왜곡이 희석되어 최종 리포트만 봐서는 원인을 알 수 없다.

**발생 조건**:
- 텔레시네/풀다운 소스, 화면 캡처(정지 구간에서 프레임 반복), VFR을 CFR 컨테이너에 담기 위한 프레임 복제.

**권장**:
```rust
fn tag_duplicates(frames: &[DecodedFrame]) -> Vec<bool> {
    frames.windows(2)
        .map(|w| frame_hash(&w[0]) == frame_hash(&w[1]))
        .chain(std::iter::once(false)) // 첫 프레임 처리
        .collect()
}

fn score_all(pairs: &[(DecodedFrame, DecodedFrame)], dup_flags: &[bool]) -> Vec<FrameScore> {
    pairs.iter().zip(dup_flags).map(|((r, d), &is_dup)| {
        FrameScore { value: psnr(r, d), is_duplicate: is_dup }
    }).collect()
}
```
- 프레임 해시/근사 비교로 중복을 탐지하고 태그를 메타데이터에 남긴다.
- 집계 시 중복 프레임을 제외하거나 별도 가중치로 다루는 정책을 명시적으로 선택한다.

**탐지 방법**:
- Runtime: 프레임 해시 기반 중복 탐지를 실제 스트림에 대해 실행.
- Semantic: 점수 분포에서 비정상적으로 동일한 값이 반복되는 패턴 통계 검사.

**예외**:
- 프리즈 프레임처럼 두 스트림 모두에서 의도적으로 동일 프레임이 반복되는 컨텐츠를 검증하는 테스트.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-005: dropped frame 이후 전체 alignment가 밀림
**분류**: ALIGN · **심각도**: Critical · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
fn align_sequential(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> Vec<(usize, usize)> {
    // 한 번 정해진 global offset을 끝까지 그대로 사용
    let offset = find_initial_offset(reference, distorted);
    (0..distorted.len())
        .map(|i| (i + offset, i))
        .collect()
}
```

**문제**:
- 드롭이 한 번이라도 발생하면, 그 지점 이후 모든 프레임 쌍이 하나씩 밀린 채로 끝까지 비교된다.
- 드롭 지점 이전 구간의 점수는 정상이지만 이후 구간은 전부 무효인데, 파이프라인은 이를 구분하지 않고 하나의 리포트로 합산한다.
- 재동기화 로직이 없으면 드롭 하나가 스트림 전체 분석 결과를 오염시킨다.

**발생 조건**:
- 손실 있는 트랜스코딩 파이프라인(과부하/오류 시 프레임 드롭), 패킷 손실이 있는 스트리밍 경로.

**권장**:
```rust
fn align_with_resync(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> Vec<AlignedPair> {
    let mut pairs = Vec::new();
    let mut r_cursor = 0;
    let mut d_cursor = 0;
    while r_cursor < reference.len() && d_cursor < distorted.len() {
        let window_offset = local_cross_correlate(
            &reference[r_cursor..], &distorted[d_cursor..], SEARCH_WINDOW,
        );
        if window_offset.confidence < RESYNC_THRESHOLD {
            log_drop_event(r_cursor, d_cursor, &window_offset);
        }
        pairs.push(AlignedPair { ref_idx: r_cursor + window_offset.ref_delta, dist_idx: d_cursor, confidence: window_offset.confidence });
        r_cursor += window_offset.ref_delta + 1;
        d_cursor += 1;
    }
    pairs
}
```
- 전역 offset 하나 대신 롤링 윈도우 기반 로컬 재동기화를 사용한다.
- 드롭/재동기화 이벤트를 로그로 남기고 confidence를 함께 기록한다(ALIGN-010 참고).

**탐지 방법**:
- Runtime: 메트릭 점수 시계열에서 갑작스러운 절벽/진동 패턴은 미정렬의 전형적 신호.
- Structural: 정렬 코드에 재동기화 경로가 전혀 없는지 검토.

**예외**:
- 동일 소스의 무손실 리먹스처럼 드롭이 구조적으로 불가능하다고 증명된 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-006: encoder delay 무시
**분류**: ALIGN · **심각도**: High · **탐지**: Manual|Semantic

**나쁜 예**:
```rust
fn initial_alignment(reference_first_pts: i64, distorted_first_pts: i64) -> i64 {
    // 첫 디코딩 프레임의 PTS를 그대로 "재생 시작점 0"으로 취급
    distorted_first_pts - reference_first_pts
}
```

**문제**:
- B-frame lookahead를 쓰는 인코더(x264/x265 등)는 첫 몇 프레임에 걸쳐 인코더 지연(delay)이 존재하며, 이는 dts/pts 오프셋으로만 드러난다.
- 이 지연을 무시하면 스트림 시작부의 N개 프레임이 상수 오프셋만큼 통째로 어긋나고, 이 오프셋이 이후 전체 정렬 기준에 그대로 전파된다.
- 인코더/설정마다 delay 크기가 달라 하드코딩된 상수로는 해결할 수 없다.

**발생 조건**:
- B-frame lookahead가 있는 인코더, 계층적 GOP(hierarchical B) 구조, 시작부 dts != pts인 컨테이너.

**권장**:
```rust
fn measure_encoder_delay(stream: &StreamInfo) -> i64 {
    // 컨테이너의 CTTS/edit-list 또는 첫 유효 dts-pts 차이로부터 delay를 유도
    stream.first_pts - stream.first_dts.min(stream.first_pts)
}

fn initial_alignment(reference: &StreamInfo, distorted: &StreamInfo) -> AlignmentOffset {
    let ref_delay = measure_encoder_delay(reference);
    let dist_delay = measure_encoder_delay(distorted);
    AlignmentOffset { ticks: (distorted.first_pts - dist_delay) - (reference.first_pts - ref_delay) }
}
```
- 스트림별로 encoder delay를 실측하여 `AlignmentConfig`에 반영한다.

**탐지 방법**:
- Manual/Semantic: 스트림 시작 부분 프레임을 시각적으로 비교하여 정렬 여부 확인.
- Structural: delay 계산 코드가 아예 존재하는지 검토.

**예외**:
- B-frame이 전혀 없는 all-I 인코딩처럼 delay가 0으로 보장되는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-007: B-frame decode order를 display order로 오해
**분류**: ALIGN · **심각도**: Critical · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
fn collect_frames(decoder: &mut Decoder) -> Vec<DecodedFrame> {
    let mut frames = Vec::new();
    while let Some(f) = decoder.decode_next_packet() {
        frames.push(f); // 디코더가 뱉어내는 순서를 그대로 신뢰
    }
    frames
}
```

**문제**:
- B-frame이 있는 스트림에서는 디코드 순서와 표시(display) 순서가 다르며, 디코더가 프레임을 내놓는 순서를 그대로 시퀀스로 쓰면 순서가 뒤섞인다.
- 뒤섞인 순서로 다른 스트림과 짝을 지으면 시간적으로 전혀 다른 두 프레임을 비교하게 된다.
- 이 오류는 조용히 발생하며, 메트릭 값이 "그럴듯하게 나쁜" 수치로 나와 실제 화질 열화로 오인되기 쉽다.

**발생 조건**:
- B-frame이 있는 모든 스트림(H.264/HEVC 메인 프로파일 이상), 특히 계층적 B 피라미드 구조.

**권장**:
```rust
fn collect_frames_in_display_order(decoder: &mut Decoder) -> Vec<DecodedFrame> {
    let mut frames: Vec<DecodedFrame> = Vec::new();
    while let Some(f) = decoder.decode_next_packet() {
        frames.push(f);
    }
    frames.sort_by_key(|f| f.pts); // 디코드 호출 순서가 아니라 PTS로 재정렬
    frames
}
```
- 디코더 API가 "이미 표시 순서로 반환한다"고 명시적으로 보장하지 않는 한, 항상 PTS 기준으로 재정렬한다.

**탐지 방법**:
- Structural: 비교기가 디코드 루프에서 바로 프레임을 꺼내 쓰는지 검토.
- Runtime: 알려진 B-frame GOP fixture로 순서 검증 테스트.

**예외**:
- all-I 인코딩, 또는 디코더 API가 표시 순서를 명시적으로 보장함을 검증한 경우(가정이 아니라 검증 필요).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-008: VFR 영상을 CFR로 가정
**분류**: ALIGN · **심각도**: High · **탐지**: Static|Runtime

**나쁜 예**:
```rust
fn expected_pts(frame_idx: u64, nominal_fps: f64, time_base: (i64, i64)) -> i64 {
    // 컨테이너 명목 fps로부터 매 프레임 pts를 "계산"
    ((frame_idx as f64 / nominal_fps) * time_base.1 as f64) as i64
}
```

**문제**:
- VFR 소스(화면 녹화, 일부 카메라, 가변 프레임률 마스터)는 프레임 간 실제 간격이 일정하지 않은데, 이를 명목 fps로 역산하면 처음부터 틀린 값이 나온다.
- index 기반으로 pts를 "계산"해서 정렬 기준으로 쓰면, 실제 pts와의 오차가 프레임마다 다르게 누적되어 예측 불가능하게 어긋난다.
- `r_frame_rate`(명목값)와 `avg_frame_rate`(실측 평균)가 다른 스트림에서 특히 잘 드러나지 않게 실패한다.

**발생 조건**:
- VFR로 플래그된 컨테이너, 화면 캡처/게임 녹화 영상, r_frame_rate != avg_frame_rate인 스트림.

**권장**:
```rust
fn detect_vfr(pts_deltas: &[i64]) -> bool {
    let mean = pts_deltas.iter().sum::<i64>() as f64 / pts_deltas.len() as f64;
    let variance = pts_deltas.iter()
        .map(|&d| (d as f64 - mean).powi(2))
        .sum::<f64>() / pts_deltas.len() as f64;
    variance.sqrt() / mean > VFR_VARIATION_THRESHOLD
}

fn actual_pts(frame_idx: usize, demuxed_frames: &[DemuxedFrame]) -> i64 {
    demuxed_frames[frame_idx].pts // 항상 실측 pts를 그대로 사용
}
```
- 항상 디먹서가 보고하는 실제 프레임별 pts를 읽어서 쓰고, 명목 fps로부터 역산하지 않는다.
- pts 델타의 분산을 측정해 VFR 여부를 탐지하고 정렬 전략을 분기한다.

**탐지 방법**:
- Static: fps 기반으로 pts를 유도하는 코드 검색.
- Runtime: VFR fixture를 투입해 정렬 오차 측정.

**예외**:
- 캡처 장비/인코더 스펙상 엄격한 CFR임이 확인되고, 실측 pts 델타 분산도 이를 검증한 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-009: seek 결과가 정확한 target frame이라고 가정
**분류**: ALIGN · **심각도**: Medium · **탐지**: Runtime|Manual

**나쁜 예**:
```rust
fn sample_at(target_pts: i64, demuxer: &mut Demuxer, decoder: &mut Decoder) -> DecodedFrame {
    demuxer.seek(target_pts);
    decoder.decode_next_packet().expect("frame at target")
    // seek 후 첫 프레임이 target_pts라고 가정
}
```

**문제**:
- 대부분의 seek 구현은 키프레임 단위로만 이동하므로, target 이전의 가장 가까운 키프레임에 도달할 뿐이다.
- 실제 target에 도달하려면 그 이후 여러 프레임을 디코드하며 버려야 하는데(디코더 프라이밍), 이를 생략하면 요청한 지점과 전혀 다른 프레임을 얻는다.
- 키프레임 간격이 넓거나 B-frame이 있는 스트림일수록 오차가 커진다.

**발생 조건**:
- 부분 분석/스팟 체크를 위한 seek 기반 샘플링, 키프레임 간격이 넓은 스트림, B-frame이 있는 스트림에서의 seek.

**권장**:
```rust
fn sample_at(target_pts: i64, demuxer: &mut Demuxer, decoder: &mut Decoder, tolerance: i64) -> Result<DecodedFrame, SeekError> {
    demuxer.seek(target_pts);
    loop {
        let frame = decoder.decode_next_packet().ok_or(SeekError::Eof)?;
        if frame.pts >= target_pts {
            if (frame.pts - target_pts).abs() > tolerance {
                return Err(SeekError::ImpreciseSeek { requested: target_pts, actual: frame.pts });
            }
            return Ok(frame);
        }
        // target 이전 프레임은 디코더 상태 워밍업을 위해 버림
    }
}
```
- seek 후 실제 도달한 pts를 target과 비교·검증하고, 허용 오차를 벗어나면 명시적으로 실패시킨다.
- 실제 달성된 pts를 정렬 메타데이터에 기록한다.

**탐지 방법**:
- Runtime: 요청 pts와 실제 디코드 pts를 비교하는 단언(assertion) 테스트.
- Manual: seek+compare 경로 코드 리뷰.

**예외**:
- 프레임 인덱스 테이블이 있는 무손실/all-I 포맷처럼 프레임 정확 seek이 보장되고 검증된 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-010: alignment confidence를 제공하지 않음
**분류**: ALIGN · **심각도**: High · **탐지**: Structural|Semantic

**나쁜 예**:
```rust
fn align(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> Vec<(usize, usize)> {
    cross_correlate(reference, distorted) // (ref_idx, dist_idx) 쌍만 반환
}
```

**문제**:
- 정렬 결과에 신뢰도(confidence)가 없으면, 저모션 구간에서의 확실한 매칭과 거의 무작위에 가까운 애매한 매칭을 구분할 방법이 없다.
- 예를 들어 검은 화면/정지 화면이 연속되면 상관도 피크가 모호해지는데, 이런 경우도 "정상 정렬"처럼 취급되어 다운스트림 메트릭 파이프라인에 그대로 전달된다.
- 실패가 조용히 전파되어 최종 리포트의 점수를 신뢰할 수 없게 만들지만, 사용자는 이를 알아챌 방법이 없다.

**발생 조건**:
- 자동(비수동) 정렬 방법(상관도, 해시, ML 기반) 전반 — 모든 방법에는 실패 모드가 있다.

**권장**:
```rust
struct AlignmentResult {
    pairs: Vec<AlignedPair>,
    confidence: f64,       // 예: 상관도 피크 sharpness
    ambiguity_ratio: f64,  // 2위 후보와의 근접도
}

fn align(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> AlignmentResult {
    let (pairs, peak, second_peak) = cross_correlate_with_diagnostics(reference, distorted);
    AlignmentResult {
        pairs,
        confidence: peak.score,
        ambiguity_ratio: second_peak.score / peak.score,
    }
}
```
- confidence/ambiguity 지표를 `AlignmentConfig`/`MetricRunMetadata`에 함께 저장하고, 임계값 미만이면 결과에 플래그를 남기거나 실패시킨다.

**탐지 방법**:
- Structural: 정렬 함수의 반환 타입에 confidence 필드가 없는지 검토.
- Semantic: 최종 리포트에 정렬 신뢰도가 노출되는지 확인.

**예외**:
- 완전 수동으로 사람이 검증한 정렬(암묵적 confidence = 1.0이지만, 이 경우에도 생략보다는 명시적으로 기록하는 편이 낫다).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-011: scene cut을 정렬 기준에 활용하지 않음
**분류**: ALIGN · **심각도**: Medium · **탐지**: Manual|Semantic

**나쁜 예**:
```rust
fn align(reference: &StreamInfo, distorted: &StreamInfo) -> AlignmentOffset {
    // 신뢰할 수 없는 타임스탬프 메타데이터에만 의존
    AlignmentOffset { ticks: distorted.first_pts - reference.first_pts }
}
```

**문제**:
- 두 스트림이 별도로 캡처되었거나 타임스탬프 기준이 서로 무관한 경우, PTS/index만으로는 애초에 신뢰할 근거가 없다.
- Scene cut(장면 전환)은 컨텐츠에 내재된, 타이밍 메타데이터와 무관한 고신뢰 동기화 지점인데 이를 활용하지 않으면 저렴하게 얻을 수 있는 검증 수단을 낭비하는 것이다.
- 다세대 트랜스코드나 제3자 인코딩 비교처럼 타임라인 공유가 불확실한 상황에서 특히 취약하다.

**발생 조건**:
- 상대적 타이밍 오프셋이 알려지지 않았거나 신뢰할 수 없는 경우, 다세대 트랜스코드, 공유 타임라인이 없는 제3자 인코딩과의 비교.

**권장**:
```rust
fn detect_scene_cuts(frames: &[DecodedFrame]) -> Vec<usize> {
    frames.windows(2).enumerate()
        .filter(|(_, w)| histogram_diff(&w[0], &w[1]) > SCENE_CUT_THRESHOLD)
        .map(|(i, _)| i)
        .collect()
}

fn align_via_scene_cuts(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> AlignmentOffset {
    let ref_cuts = detect_scene_cuts(reference);
    let dist_cuts = detect_scene_cuts(distorted);
    match_cut_sequences(&ref_cuts, &dist_cuts) // 컨텐츠 기반 앵커로 대략 정렬
}
```
- Scene cut 시퀀스를 앵커로 우선 사용해 대략적인 정렬을 잡고, 이후 구간 내에서 상관도 기반으로 정밀화한다.

**탐지 방법**:
- Manual/Semantic: 정렬 모듈에 컨텐츠 기반 폴백이 있는지 검토.
- Runtime: 타임스탬프를 의도적으로 뒤섞은 fixture로 테스트.

**예외**:
- 단일 소스에서 나온 인코딩 작업처럼 상대 PTS 오프셋이 이미 고신뢰로 알려진 경우 — 이때 scene-cut 앵커링은 불필요한 연산이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-012: audio 또는 container timing과 video timing 혼용
**분류**: ALIGN · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```rust
fn video_offset(container: &ContainerInfo) -> i64 {
    // 오디오 트랙의 start_time을 비디오 정렬 오프셋으로 그대로 사용
    container.audio_track.start_time - container.reference_audio_start_time
}
```

**문제**:
- 컨테이너 내 오디오/비디오 트랙은 각자 독립적인 `start_time`/pts 오프셋을 가질 수 있는데, 두 트랙이 동일 타임라인을 공유한다고 가정하면 틀린 값을 얻는다.
- 업스트림 인코더에서 발생한 A/V 드리프트가 있으면 오디오 기준 오프셋이 비디오 프레임 정렬에는 맞지 않는다.
- 트랙별 timescale이 다른 경우 변환 과정에서 추가 오차(ALIGN-003)까지 겹친다.

**발생 조건**:
- 트랙마다 독립적인 start_time/timescale을 갖는 컨테이너, 약간의 A/V 드리프트가 있는 녹화물.

**권장**:
```rust
fn video_offset(reference: &StreamInfo, distorted: &StreamInfo) -> AlignmentOffset {
    // 비디오 정렬은 반드시 비디오 트랙 자신의 pts 시퀀스로부터 유도
    AlignmentOffset { ticks: distorted.video_track.first_pts - reference.video_track.first_pts }
}

// 오디오 핑거프린팅은 "대략적인 사전 정렬" 힌트로만 쓰고, 반드시 비디오 컨텐츠 앵커로 검증
fn coarse_prealign_from_audio(reference: &StreamInfo, distorted: &StreamInfo) -> AlignmentOffset {
    let audio_hint = audio_fingerprint_offset(reference, distorted);
    validate_against_scene_cuts(audio_hint, reference, distorted)
}
```
- 비디오 정렬은 항상 비디오 트랙 자신의 pts에서 유도한다.
- 오디오 기반 사전 힌트를 쓰더라도 반드시 비디오 컨텐츠 앵커(scene cut 등)로 검증한다.

**탐지 방법**:
- Structural: 정렬 코드가 오디오 트랙 필드를 참조하는지 검색.
- Manual: 멀티트랙 디먹싱 로직 리뷰.

**예외**:
- 오디오 기반 대략적 사전 정렬을 의도적으로 사용하되, 이후 비디오 컨텐츠로 명시적으로 검증하는 설계.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-013: frame 수가 다르면 즉시 분석 실패
**분류**: ALIGN · **심각도**: Medium · **탐지**: Static|Structural

**나쁜 예**:
```rust
fn run_comparison(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> Result<Report, CompareError> {
    if reference.len() != distorted.len() {
        return Err(CompareError::FrameCountMismatch);
    }
    // 정렬 시도 없이 바로 실패
    Ok(compare_paired(reference, distorted))
}
```

**문제**:
- 실제 트랜스코딩 비교에서는 인코더 지연, 드롭, VFR 등으로 인해 프레임 수가 정확히 같은 경우가 오히려 드물다.
- 개수 불일치를 즉시 오류로 처리하면, 정렬을 통해 충분히 해결 가능한 정상적인 비교 요청까지 차단한다.
- 결과적으로 사용자는 실제로는 유효한 비교를 "실패"로만 인지하게 된다.

**발생 조건**:
- 사실상 모든 실전 트랜스코드 비교 — 개수가 정확히 일치하는 경우가 오히려 예외적이다.

**권장**:
```rust
fn run_comparison(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> Result<Report, CompareError> {
    let alignment = align_frames(reference, distorted, &AlignmentConfig::default())?;
    if alignment.confidence < MIN_CONFIDENCE {
        return Err(CompareError::AlignmentLowConfidence(alignment.confidence));
    }
    Ok(compare_aligned(reference, distorted, &alignment))
}
```
- 개수 불일치는 정렬을 호출하는 신호로 취급하고, 정렬 시도 후 confidence가 임계값 미만일 때만 실패시킨다.

**탐지 방법**:
- Static: 정렬 호출 이전에 있는 길이 동등성 가드(guard) 검색.
- Structural: 파이프라인 순서(정렬 → 비교) 검토.

**예외**:
- bit-exact 리먹스 검증처럼 설계상 프레임 수 일치가 하드 요구사항인 파이프라인.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-014: 짧은 offset 탐색에 전체 brute force 사용
**분류**: ALIGN · **심각도**: Low · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
fn find_best_offset(reference: &[DecodedFrame], distorted: &[DecodedFrame], search_range: Range<i64>) -> i64 {
    search_range
        .map(|offset| (offset, full_stream_ssim(reference, distorted, offset)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap().0
    // 후보 offset마다 전체 스트림에 대해 풀 정밀도 SSIM 계산
}
```

**문제**:
- 복잡도가 O(offset 후보 수 × 프레임 수 × 픽셀 비용)으로, 탐색 범위가 넓고 스트림이 길수록 연산량이 폭발한다.
- offset 탐색 자체는 대략적인 신호로도 충분한데, 최종 메트릭과 동일한 고비용 연산을 반복 사용하는 것은 불필요한 낭비다.
- 인터랙티브 UI 도구에서는 이 비용이 응답성을 크게 해친다.

**발생 조건**:
- 긴 영상, 인코더 지연 등으로 탐색 범위가 넓은 경우, 빠른 피드백이 필요한 대화형 도구.

**권장**:
```rust
fn find_best_offset(reference: &[DecodedFrame], distorted: &[DecodedFrame], search_range: Range<i64>) -> AlignmentResult {
    // 1단계: 저비용 프록시(다운샘플 휘도 히스토그램/해시)로 후보 좁히기
    let candidates = search_range
        .map(|offset| (offset, cheap_histogram_correlation(reference, distorted, offset)))
        .sorted_by_top_k(TOP_K_CANDIDATES);

    // 2단계: 상위 후보만 작은 윈도우에서 풀 정밀도로 검증
    candidates.into_iter()
        .map(|(offset, _)| (offset, windowed_full_metric(reference, distorted, offset, VERIFY_WINDOW)))
        .max_by(|a, b| a.1.confidence.partial_cmp(&b.1.confidence).unwrap())
        .unwrap().1
}
```
- 탐색은 저비용 프록시 신호로 후보를 좁히고, 최종 검증만 작은 윈도우에서 풀 정밀도 메트릭을 사용한다.

**탐지 방법**:
- Runtime: 프로파일링으로 정렬 탐색 단계가 핫스팟인지 확인.
- Structural: 탐색 알고리즘의 복잡도 검토.

**예외**:
- 수 초 이내의 짧은 클립처럼 brute force의 비용이 무시할 만하고 단순함이 더 가치 있는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-015: spatial alignment 없이 pixel metric 실행
**분류**: ALIGN · **심각도**: High · **탐지**: Manual|Runtime

**나쁜 예**:
```rust
fn compare_aligned_frames(reference: &DecodedFrame, distorted: &DecodedFrame) -> f64 {
    // 시간축만 맞춘 뒤 곧바로 픽셀 메트릭 실행, 해상도/크롭 차이는 확인하지 않음
    psnr(reference, distorted)
}
```

**문제**:
- 시간적으로 완벽히 정렬된 프레임이라도 해상도, 크롭, 패딩, 픽셀 종횡비가 다르면 픽셀 메트릭은 무의미한 값을 낸다.
- 인코더가 추가하는 크롭/패딩 메타데이터, 레터박스, 리사이즈 필터 등이 컨텐츠 위치를 이동시킨다.
- 이 오류는 스트림 전체에 걸쳐 균일하게 낮거나 잡음 섞인 점수로 나타나 마치 "화질이 나쁜 것"처럼 오인되기 쉽다.

**발생 조건**:
- 서로 다른 해상도/종횡비의 스트림, 크롭/패딩 메타데이터를 추가하는 인코더, 다운스케일 후 비교하는 파이프라인, 픽셀 종횡비 시그널링이 다른 경우.

**권장**:
```rust
fn compare_aligned_frames(
    reference: &DecodedFrame,
    distorted: &DecodedFrame,
    spatial: &SpatialAlignmentConfig,
) -> Result<f64, AlignError> {
    let (r_registered, d_registered) = register_spatially(reference, distorted, spatial)?;
    Ok(psnr(&r_registered, &d_registered))
}

fn register_spatially(
    reference: &DecodedFrame, distorted: &DecodedFrame, cfg: &SpatialAlignmentConfig,
) -> Result<(DecodedFrame, DecodedFrame), AlignError> {
    let r = apply_crop_detect(reference)?;
    let d = normalize_resolution(distorted, r.dimensions())?;
    Ok((r, d))
}
```
- 픽셀 메트릭 이전에 크롭 검출, 해상도 정규화, 필요시 서브픽셀 정합을 수행한다.
- 공간 정렬 파라미터도 `AlignmentConfig`에 시간 오프셋과 함께 기록한다.

**탐지 방법**:
- Manual: 시각적 diff 리뷰.
- Runtime: 전 프레임에 걸쳐 균일하게 낮거나 잡음 섞인 점수 패턴 탐지.

**예외**:
- 동일 해상도, 크롭 없음, 프레이밍이 픽셀 단위로 동일함이 검증된 파이프라인(예: 기하 변화 없는 내부 무손실 포맷 래더).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-016: 스트림 중간 timestamp discontinuity(스플라이스/광고 삽입)를 단일 offset으로 처리
**분류**: ALIGN · **심각도**: High · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
fn align_whole_stream(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> AlignmentOffset {
    // 스트림 시작부에서만 offset을 구해 끝까지 재사용
    AlignmentOffset { ticks: find_initial_offset(reference, distorted) }
}
```

**문제**:
- 라이브-to-VOD, 광고 삽입(SCTE-35), 여러 세그먼트를 이어붙인 자산은 스트림 중간에 PTS 불연속점을 가질 수 있다.
- 전역 offset 하나는 다음 스플라이스 지점 전까지만 유효한데, 스칼라 값 하나로는 이 사실이 전혀 드러나지 않는다.
- 스플라이스 이후 구간은 임의로 어긋난 채 비교되지만 파이프라인은 이를 정상 결과처럼 보고한다.

**발생 조건**:
- 라이브 스트림의 VOD화, 광고 삽입, 여러 세그먼트를 연결한 자산, edit list가 변경된 스트림 복사본.

**권장**:
```rust
struct SegmentedAlignment {
    segments: Vec<(FrameRange, AlignmentOffset)>,
}

fn detect_discontinuities(pts_sequence: &[i64], expected_delta: i64, tolerance: i64) -> Vec<usize> {
    pts_sequence.windows(2).enumerate()
        .filter(|(_, w)| (w[1] - w[0] - expected_delta).abs() > tolerance)
        .map(|(i, _)| i + 1)
        .collect()
}

fn align_segmented(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> SegmentedAlignment {
    let cuts = detect_discontinuities(&extract_pts(distorted), NOMINAL_DELTA, DELTA_TOLERANCE);
    SegmentedAlignment {
        segments: split_into_segments(reference, distorted, &cuts)
            .map(|(range, r_seg, d_seg)| (range, find_initial_offset(r_seg, d_seg)))
            .collect(),
    }
}
```
- PTS 델타 시퀀스에서 불연속(급격한 점프)을 탐지하고, 각 연속 구간을 독립적인 정렬 문제로 다룬다.
- `AlignmentConfig`가 단일 offset이 아니라 세그먼트별 정렬을 표현할 수 있어야 한다.

**탐지 방법**:
- Runtime: pts 델타 시퀀스 모니터링으로 점프 탐지.
- Structural: `AlignmentConfig`가 단일 스칼라 offset만 지원하는지 검토.

**예외**:
- 스플라이싱 없이 한 번에 인코딩된 단일 연속 VOD 자산.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-017: alignment을 1회 계산 후 drift 재검증 없이 전체 구간에 고정 적용
**분류**: ALIGN · **심각도**: Medium · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
fn align_once_and_reuse(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> AlignmentOffset {
    // 스트림 시작 몇 초만 보고 계산한 offset을 수 시간짜리 스트림 전체에 재사용
    find_initial_offset(&reference[..INITIAL_SAMPLE], &distorted[..INITIAL_SAMPLE])
}
```

**문제**:
- 독립적으로 타임스탬프가 매겨진 두 스트림 사이에는 클럭 드리프트가 존재할 수 있고, VFR 불규칙성이나 반올림 누적(ALIGN-003)도 시간이 지날수록 실제 offset을 서서히 이동시킨다.
- 초기 정렬이 정확했더라도, 이 드리프트를 재검증하지 않으면 스트림 후반부로 갈수록 조용히 어긋난다.
- 장시간 라이브 스트림일수록 이 문제가 누적되어 심각해진다.

**발생 조건**:
- 장시간 스트림, 공유 마스터 클록 없이 독립적으로 캡처/인코딩된 스트림, 라이브 스트림.

**권장**:
```rust
fn align_with_periodic_revalidation(
    reference: &[DecodedFrame], distorted: &[DecodedFrame], recheck_interval: Duration,
) -> Vec<(FrameRange, AlignmentOffset)> {
    let mut offsets = Vec::new();
    let mut current_offset = find_initial_offset(reference, distorted);
    for window in time_windows(reference, distorted, recheck_interval) {
        let measured = find_initial_offset(window.reference, window.distorted);
        if (measured.ticks - current_offset.ticks).abs() > DRIFT_TOLERANCE {
            log_drift_correction(window.range, current_offset, measured);
            current_offset = measured;
        }
        offsets.push((window.range, current_offset));
    }
    offsets
}
```
- 주기적으로 경량 상관도 검사를 재실행해 드리프트를 감지하고, 허용치를 넘으면 매핑을 보정하며 이벤트를 로그로 남긴다.

**탐지 방법**:
- Runtime: 스트림 진행에 따라 프레임 쌍 유사도가 점진적으로 저하되는 추세 분석.
- Structural: 주기적 재검증 로직 부재 여부 검토.

**예외**:
- 짧은 클립, 또는 단일 하드웨어 클록/마스터 타임라인을 공유함이 검증되어 드리프트 가능성이 없는 스트림.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### ALIGN-018: GOP 구조 차이(오픈 GOP vs 클로즈드 GOP, 상이한 GOP 길이)를 정렬에서 무시
**분류**: ALIGN · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```rust
fn align_via_keyframes(reference: &StreamInfo, distorted: &StreamInfo) -> AlignmentOffset {
    // 두 스트림의 키프레임 위치가 대응한다고 가정
    let r_kf = reference.keyframe_positions[0];
    let d_kf = distorted.keyframe_positions[0];
    AlignmentOffset { ticks: d_kf - r_kf }
}
```

**문제**:
- 원본과 재인코딩본이 서로 다른 인코더/설정을 쓰면 GOP 길이와 오픈/클로즈드 GOP 구조가 달라 키프레임 위치가 시간적으로 대응하지 않는다.
- 키프레임 위치를 정렬 앵커나 seek 기반 샘플링의 기준으로 삼으면, 대응 관계가 틀렸는데도 조용히 잘못된 짝을 만든다.
- 적응형 비트레이트 래더의 서로 다른 렌디션 간 비교, 오픈 GOP HEVC처럼 GOP 경계를 넘나드는 참조가 있는 경우 특히 취약하다.

**발생 조건**:
- 원본과 재인코딩본을 비교하는 모든 파이프라인(다른 인코더/설정), 적응형 비트레이트 래더 렌디션 비교, 오픈 GOP HEVC 스트림.

**권장**:
```rust
fn align_content_based(reference: &[DecodedFrame], distorted: &[DecodedFrame]) -> AlignmentOffset {
    // 키프레임 위치가 아니라 컨텐츠(픽셀/해시) 기반 앵커 사용
    let ref_anchors = detect_scene_cuts(reference);
    let dist_anchors = detect_scene_cuts(distorted);
    match_cut_sequences(&ref_anchors, &dist_anchors)
}

// GOP 구조 메타데이터는 참고용으로만 사용하고 정렬 대응관계로 신뢰하지 않는다
fn gop_structure_as_metadata(stream: &StreamInfo) -> GopInfo {
    GopInfo { length: stream.avg_gop_length, is_open: stream.has_open_gop }
}
```
- 두 스트림의 GOP 구조가 같다고 가정하지 말고, 키프레임 위치와 무관한 컨텐츠 기반(픽셀/해시) 앵커로 정렬한다.
- GOP 구조 메타데이터는 진단/참고 정보로만 취급한다.

**탐지 방법**:
- Structural: 정렬 로직이 두 스트림의 키프레임 인덱스가 대응한다고 가정하고 사용하는지 검토.
- Manual: GOP 구조가 다른 스트림 쌍 fixture로 리뷰.

**예외**:
- 동일 인코더/설정/GOP 구조가 보장된 경우(예: 재인코딩 없는 리먹스 전용 파이프라인).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
