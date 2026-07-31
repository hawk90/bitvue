# Anti-Pattern Catalog — STAT: 집계와 통계 (VQ-Probe domain)

이 문서는 더 큰 안티패턴 카탈로그의 일부이며(전체 색인은 `docs/anti-patterns/INDEX.md`, 별도 작성 예정), Bitvue의 두 도메인 중 VQ-Probe(듀얼 스트림 화질 비교) 도메인 절반 — ALIGN/SPATIAL/COLOR/METRIC/PIPE/HEAT/STAT — 의 마지막 카테고리다. Bitstream-Analyzer 도메인이 신택스/메타데이터를 "정확히 보여주는" 문제라면, VQ-Probe는 프레임 단위 점수를 어떻게 풀링(pool)하고, 이상치·드롭 프레임·scene cut을 어떻게 다루며, 숫자 하나의 산출 과정(provenance)이 그 숫자 자체와 함께 살아남는지를 다루는 문제다.

---

### STAT-001: 평균만 제공
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub struct SequenceReport {
    pub metric_name: String,
    pub mean_score: f64,
}

pub fn summarize(frame_scores: &[f64]) -> SequenceReport {
    let mean = frame_scores.iter().sum::<f64>() / frame_scores.len() as f64;
    SequenceReport {
        metric_name: "vmaf_like".into(),
        mean_score: mean, // 이게 리포트의 전부
    }
}
```

**문제**:
- 평균은 "고르게 나쁨"과 "대체로 좋지만 국소적으로 심각하게 나쁨"을 동일한 숫자로 만든다. 두 인코딩 결과가 평균이 같아도 사용자 체감 품질은 전혀 다를 수 있다.
- 분산/표준편차/최솟값이 없으면 "이 결과를 신뢰해도 되는가"를 판단할 근거가 없다.
- 리포트를 소비하는 하위 도구(회귀 감지, 랭킹)가 평균만 보고 의사결정을 내리게 되어, 실제로는 열화가 있는데도 "동일"로 분류되는 사례가 반복된다.

**발생 조건**:
- 초기 프로토타입에서 "일단 숫자 하나만 보여주자"로 시작한 리포트 구조가 그대로 프로덕션까지 이어질 때.
- UI 카드 한 칸에 숫자 하나만 넣으면 된다는 디자인 제약이 백엔드 데이터 모델까지 그대로 전파될 때.

**권장**:
```rust
pub struct SequenceReport {
    pub metric_name: String,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub p1: f64,   // 최악 1% 근방
    pub p5: f64,
    pub p95: f64,
    pub max: f64,
    pub sample_count: usize,
}
```
- 최소한 평균 + 표준편차 + min + 하위 percentile은 함께 저장하고, UI에는 요약 카드 + 상세 분포(히스토그램/박스플롯)를 함께 노출한다.
- 리포트 구조체를 설계할 때 "이 숫자 하나만으로 의사결정이 가능한가"를 기준으로 필드를 고른다.

**탐지 방법**:
- Structural: 리포트/DTO 타입에 `f64` 단일 필드만 있고 분포 관련 필드가 없는 패턴 검색.
- Manual: 리뷰 시 "이 평균값으로 pass/fail을 가른다면, 최악의 프레임이 몇 점인지 알 수 있는가?" 질문.

**예외**:
- 실시간 미리보기처럼 지연시간이 극히 중요하고, 상세 분포는 별도 백그라운드 계산으로 나중에 채워지는 구조라면 1차 응답에서 평균만 먼저 주는 것은 허용된다 — 단, 상세 분포가 "나중에 채워진다"는 사실 자체가 API 계약에 명시되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-002: 최악의 1% 구간을 숨김
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn summarize(frame_scores: &[f64]) -> SequenceReport {
    let mean = mean_of(frame_scores);
    let p50 = percentile(frame_scores, 0.50);
    // p1, p5는 계산은 되지만 UI 요약 카드에 표시되는 필드에 포함되지 않음
    SequenceReport { mean, median: p50 }
}
```

**문제**:
- 스트리밍/방송 화질에서 사용자가 실제로 체감하는 것은 평균이 아니라 "가장 나빴던 순간"인 경우가 많다 (버퍼링 직후, 급격한 모션, 트랜스코딩 아티팩트).
- 최악 1% 구간이 숨겨지면, 인코더 설정 A가 평균은 더 좋지만 최악의 순간에는 B보다 훨씬 나쁜 경우를 놓친다 — 이는 특히 저비트레이트 구간(scene 전환, 고모션)에서 흔하다.
- QA 프로세스가 "평균이 임계값을 넘었는가"만 게이트로 쓰면, 드문 심각한 열화가 릴리스를 통과한다.

**발생 조건**:
- 요약 카드/대시보드 공간이 제한적이어서 "대표값 하나"만 노출하도록 설계가 굳어질 때.
- percentile 계산 자체는 파이프라인에 존재하지만, 리포트 직렬화나 UI 바인딩 단계에서 필드가 누락될 때.

**권장**:
```rust
pub struct QualityDistribution {
    pub mean: f64,
    pub p50: f64,
    pub p5: f64,
    pub p1: f64,
    pub worst_frame: WorstFrameRef, // frame_index, score, timestamp
}

impl QualityDistribution {
    pub fn is_worst_case_acceptable(&self, floor: f64) -> bool {
        self.p1 >= floor // 게이트 기준을 평균이 아니라 최악 구간에 둔다
    }
}
```
- QA 게이트를 "평균 ≥ 임계값"이 아니라 "p1(또는 min) ≥ 최소 허용치" 기준으로 설계한다.
- 최악 프레임에는 반드시 frame_index/timestamp를 함께 저장해서 사용자가 실제로 재생 위치로 점프해 확인할 수 있게 한다.

**탐지 방법**:
- Semantic: 리포트 구조체에 percentile 필드가 있는지, 있다면 UI/API 응답 직렬화 경로에서 실제로 전달되는지 추적.
- Manual: "이 리포트만 보고 최악의 5초 구간이 어디인지 답할 수 있는가?" 질문.

**예외**:
- 표본 수가 매우 적은 경우(수십 프레임 이하) p1 percentile은 통계적으로 무의미할 수 있으므로, 이런 경우 min/max로 대체하고 표본 수가 부족하다는 사실을 명시한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-003: p95와 p5 방향을 metric 성격에 맞지 않게 해석
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// "higher-is-better" 지표(PSNR/SSIM/VMAF류)와
// "lower-is-better" 지표(왜곡/에러 지표)를 같은 함수로 처리
pub fn worst_case(scores: &[f64]) -> f64 {
    percentile(scores, 0.95) // 항상 p95를 "worst case"로 취급
}
```

**문제**:
- higher-is-better 지표(PSNR, SSIM, VMAF류)에서 "최악"은 분포의 하위 꼬리(p5, p1)인데, lower-is-better 지표(왜곡 에너지, 에러율)에서는 정반대로 상위 꼬리(p95, p99)가 최악이다. 하나의 `worst_case()` 함수가 항상 같은 방향의 percentile을 반환하면 절반의 metric에서는 결과가 뒤집힌다.
- 이 버그는 숫자가 "그럴듯하게" 나오기 때문에 코드 리뷰에서 걸러지지 않는다 — 특정 metric에서만 조용히 최선의 경우를 최악으로 보고한다.
- 여러 metric을 같은 대시보드에 늘어놓았을 때, 어떤 카드는 방향이 맞고 어떤 카드는 뒤집혀 있어 사용자가 잘못된 결론을 내린다.

**발생 조건**:
- metric 타입이 추가/확장될 때 (예: PSNR/SSIM만 다루다가 왜곡 기반 신규 metric을 추가), 기존 percentile 헬퍼를 그대로 재사용할 때.
- percentile 계산 유틸리티가 "worst"라는 이름으로 방향을 하드코딩하고 있을 때.

**권장**:
```rust
#[derive(Clone, Copy)]
pub enum MetricPolarity {
    HigherIsBetter,
    LowerIsBetter,
}

pub fn worst_case(scores: &[f64], polarity: MetricPolarity) -> f64 {
    match polarity {
        MetricPolarity::HigherIsBetter => percentile(scores, 0.05),
        MetricPolarity::LowerIsBetter => percentile(scores, 0.95),
    }
}

pub struct MetricDefinition {
    pub name: &'static str,
    pub polarity: MetricPolarity,
}
```
- metric 카탈로그에 극성(polarity)을 명시적 타입으로 갖고 다니고, "worst"/"best" 계산은 항상 이 타입을 인자로 받게 한다.
- 문자열 이름(`"psnr"`)으로 if/else 분기하지 말고, MetricDefinition 레지스트리에서 조회한다.

**탐지 방법**:
- Static: 방향을 암시하는 이름(`worst_case`, `best_case`)을 가진 함수가 `MetricPolarity` 없이 percentile 값만 인자로 받는 시그니처 검색.
- Runtime: 알려진 lower-is-better 테스트 metric에 대해 worst_case() 결과가 실제 최악 프레임과 일치하는지 회귀 테스트.

**예외**:
- 단일 metric 전용으로 처음부터 끝까지 하드코딩된 내부 유틸(다른 metric에 절대 재사용되지 않음이 타입 시스템으로 보장된 경우)이라면 극성 파라미터 없이 방향을 고정해도 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-004: scene 길이를 무시한 scene 평균
**분류**: STAT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct SceneScore {
    pub scene_id: u32,
    pub mean_score: f64,
}

pub fn score_scene(frame_scores: &[f64], scene: &SceneBoundary) -> SceneScore {
    let frames = &frame_scores[scene.start_frame..scene.end_frame];
    SceneScore {
        scene_id: scene.id,
        mean_score: frames.iter().sum::<f64>() / frames.len() as f64,
        // scene의 프레임 수(scene.end_frame - scene.start_frame)가
        // SceneScore에 저장되지 않고 버려진다
    }
}
```

**문제**:
- `SceneScore`에 frame count가 없으면, 이후 어떤 소비자도 이 값을 다른 scene과 올바르게 가중 결합할 수 없다 — 결국 모두가 "scene당 동일 가중치"라는 잘못된 가정을 강제로 떠안게 된다.
- 통계 정보(표본 수)를 결과값 산출 시점에만 알고 버리면, 나중에 재현/검증하려는 사람이 원본 frame_scores를 다시 읽어야만 한다.
- 3초짜리 scene과 30초짜리 scene이 최종 집계에서 동일한 한 표로 취급되는 왜곡의 근본 원인이 된다.

**발생 조건**:
- scene 단위 리포트를 "요약 테이블 한 줄"로만 생각하고 설계할 때, 표본 크기를 부차적인 정보로 취급하고 누락시킬 때.

**권장**:
```rust
pub struct SceneScore {
    pub scene_id: u32,
    pub mean_score: f64,
    pub frame_count: usize,
    pub duration_ns: u64,
}

pub fn sequence_mean(scenes: &[SceneScore]) -> f64 {
    let total_frames: usize = scenes.iter().map(|s| s.frame_count).sum();
    scenes.iter()
        .map(|s| s.mean_score * s.frame_count as f64)
        .sum::<f64>() / total_frames as f64
}
```
- 집계 결과 타입은 항상 "값 + 표본 크기(또는 duration)"를 쌍으로 갖도록 설계한다. 값만 있고 가중치 정보가 없는 통계 타입은 그 자체로 코드 스멜이다.

**탐지 방법**:
- Structural: 집계 결과를 담는 struct 정의를 스캔해서 `_score`/`_mean` 필드는 있지만 `count`/`weight`/`duration` 필드가 없는 타입 검색.
- Manual: "이 SceneScore 두 개를 나중에 합치려면 무엇이 더 필요한가?" 질문.

**예외**:
- Scene 경계가 항상 고정 길이(예: 1초 GOP 단위로만 분할)로 설계가 보장된 시스템이라면 frame_count가 상수이므로 생략해도 무방하지만, 그 가정이 깨지는 순간(가변 GOP, scene-cut 기반 분할 도입) 이 필드가 반드시 필요해진다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-005: 짧은 scene과 긴 scene을 동일 가중치로 평균
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// SceneScore에 frame_count가 이미 정상적으로 저장되어 있다고 하더라도,
// 리포트 단계에서 "scene 평균들의 평균"을 또 한 번 단순 산술평균으로 낸다
pub fn sequence_summary(scene_scores: &[SceneScore]) -> f64 {
    scene_scores.iter().map(|s| s.mean_score).sum::<f64>()
        / scene_scores.len() as f64 // mean of means — 가중치 정보 무시
}
```

**문제**:
- "평균들의 평균"(mean of means)은 각 scene의 표본 크기가 다르면 전체 평균과 다르다 — 이는 STAT-004처럼 데이터가 누락돼서가 아니라, 데이터는 있는데 집계 함수가 그것을 쓰지 않아서 생기는, 더 흔하고 더 발견하기 어려운 실수다.
- 구체적 예: 2프레임짜리 문제 scene(점수 40)과 998프레임짜리 정상 scene(점수 95)이 있을 때, mean-of-means는 (40+95)/2=67.5를 보고하지만 실제 가중 평균은 (40*2+95*998)/1000≈94.9다. 사용자는 "심각한 문제"로 오인하거나 반대로 짧고 심각한 결함을 과대평가/과소평가한다.
- 짧고 극단적인 scene(예: 검은 화면 1프레임, 페이드 인/아웃)이 전체 점수를 비정상적으로 끌어내리거나 끌어올리는 왜곡이 반복적으로 재현된다.

**발생 조건**:
- 이미 scene별로 올바르게 집계된 결과 리스트를 "그냥 다시 평균내면 되겠지"라는 직관으로 2차 집계할 때 — 특히 대시보드/리포트 레이어를 만드는 사람이 1차 집계 로직을 작성한 사람과 다를 때 자주 발생한다.
- 통계 배경이 없는 엔지니어가 "평균의 평균 = 전체 평균"이라는 잘못된 직관을 그대로 코드로 옮길 때.

**권장**:
```rust
pub fn sequence_summary(scene_scores: &[SceneScore]) -> f64 {
    let total_weight: usize = scene_scores.iter().map(|s| s.frame_count).sum();
    scene_scores.iter()
        .map(|s| s.mean_score * s.frame_count as f64)
        .sum::<f64>() / total_weight as f64
}
```
- 값 리스트를 다시 평균낼 때마다 "이 값들이 서로 같은 가중치를 가지는가?"를 반드시 자문한다. 대부분의 pooling된 통계값은 그렇지 않다.
- 가능하면 애초에 "이미 집계된 값"을 재집계하지 말고, frame-level 원본에서 한 번에 가중 계산하는 경로를 표준으로 둔다.

**탐지 방법**:
- Semantic: 이미 `*_score`/`*_mean` 필드를 가진 리스트에 대해 다시 `.sum() / .len()`을 호출하는 코드 패턴 검색.
- Runtime: 표본 크기가 극단적으로 다른 합성 scene 세트로 mean-of-means와 가중 평균의 차이가 임계값을 넘는지 검증하는 회귀 테스트.

**예외**:
- 모든 scene의 frame_count가 사실상 동일하다는 것이 도메인 제약으로 보장된 경우(예: 고정 GOP 길이 스트림만 다루는 도구)라면 mean-of-means와 가중 평균이 수렴하므로 실질적 차이가 없다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-006: frame duration을 무시한 VFR 집계
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn temporal_mean(frames: &[FrameMetric]) -> f64 {
    // VFR(가변 프레임레이트) 스트림인데 프레임 개수로만 평균낸다
    frames.iter().map(|f| f.score).sum::<f64>() / frames.len() as f64
}

pub struct FrameMetric {
    pub frame_index: u32,
    pub score: f64,
    pub pts_ns: u64, // 존재하지만 집계에 쓰이지 않음
}
```

**문제**:
- VFR 스트림에서는 프레임 하나가 화면에 표시되는 시간(duration)이 서로 다르다 — 짧게 표시되는 프레임과 길게 표시되는 프레임을 동일하게 카운트하면, 실제 "사용자가 그 화질을 얼마나 오래 봤는가"와 통계가 어긋난다.
- 프레임 드롭/듀플리케이션이 있는 스트림에서 프레임 개수 기반 평균은 인지 품질과 괴리된 숫자를 만든다 — 예를 들어 저모션 구간에서 프레임이 오래 유지되면 그 구간의 화질이 실제로는 더 오래 노출됨에도 개수 기준 집계에서는 한 표로만 카운트된다.
- 두 스트림(reference/distorted)의 프레임레이트가 다른 비교 시나리오에서, duration 가중 없이 프레임 인덱스로만 정렬/평균하면 시간축이 어긋난 채로 비교하게 된다.

**발생 조건**:
- 소스가 CFR(고정 프레임레이트)이라고 암묵적으로 가정하고 작성된 집계 코드가, VFR 소스(스크린 레코딩, 일부 UGC, 가변 프레임레이트 인코딩 프로필)에 그대로 적용될 때.
- PTS 정보는 파이프라인 앞단(디먹싱/디코딩)에서는 정확히 추출되지만, metric 집계 단계로 전달되지 않거나 무시될 때.

**권장**:
```rust
pub fn temporal_mean(frames: &[FrameMetric]) -> f64 {
    let total_duration: u64 = frames.iter().map(|f| f.duration_ns).sum();
    frames.iter()
        .map(|f| f.score * f.duration_ns as f64)
        .sum::<f64>() / total_duration as f64
}

pub struct FrameMetric {
    pub frame_index: u32,
    pub score: f64,
    pub pts_ns: u64,
    pub duration_ns: u64, // 다음 프레임과의 PTS 간격, 명시적으로 계산되어 저장
}
```
- duration은 "다음 프레임 PTS - 현재 프레임 PTS"로 명시적으로 계산해 FrameMetric에 저장하고, 마지막 프레임은 스트림 전체 duration이나 평균 duration으로 보정한다.
- CFR/VFR 여부를 파이프라인 메타데이터에 기록하고, CFR이 확인된 경우에만 duration-가중과 개수-가중이 동일함을 근거로 개수 기반 경로를 최적화로 사용한다.

**탐지 방법**:
- Structural: `FrameMetric`류 타입에 duration/PTS 필드가 있는지, 집계 함수 시그니처가 그 필드를 실제로 사용하는지 대조.
- Runtime: 합성 VFR 픽스처(의도적으로 duration을 다르게 설정)에 대해 개수 기반 평균과 duration 기반 평균의 차이를 검증.

**예외**:
- 소스가 컨테이너 레벨에서 CFR임이 보장되고(고정 timebase, 프레임 드롭/듀플리케이션 없음) 그 보장이 파이프라인 계약에 명시되어 있다면, 개수 기반 집계와 duration 기반 집계는 수학적으로 동일하므로 개수 기반 구현을 유지해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-007: dropped frame을 집계에서 제외하고 알리지 않음
**분류**: STAT · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn align_and_score(reference: &[Frame], distorted: &[Frame]) -> Vec<f64> {
    let mut scores = Vec::new();
    for (r, d) in reference.iter().zip(distorted.iter()) {
        // distorted 스트림이 인코더/전송 과정에서 프레임을 드롭해
        // reference보다 짧아지면, zip()이 남은 reference 프레임을 조용히 버린다
        scores.push(compute_score(r, d));
    }
    scores // 원본 대비 몇 프레임이 비교에서 빠졌는지 아무 정보도 없다
}
```

**문제**:
- `zip()`은 더 짧은 쪽 길이에 맞춰 조용히 잘라내므로, distorted 스트림에서 프레임이 드롭되면 그 이후 reference 프레임들이 통째로 비교 대상에서 사라진다 — 이는 "품질이 좋아서 점수가 없는 것"이 아니라 "측정이 안 된 것"인데 결과물에서는 구분이 불가능하다.
- 프레임 드롭은 그 자체로 심각한 화질 결함(끊김, 프리징)인데, 드롭된 구간이 평균 계산에서 아예 빠지면 오히려 점수가 좋아 보이는 역설이 발생한다 — 드롭이 있는 스트림이 드롭이 없는 스트림보다 더 높은 평균 점수를 받을 수 있다.
- 최종 리포트 소비자는 "1000프레임 중 950프레임 평균 92점"과 "1000프레임 중 1000프레임 평균 92점"을 구분할 방법이 없어 신뢰도를 오판한다.

**발생 조건**:
- 인코더 설정에 따라 프레임 드롭이 실제로 발생하는 저비트레이트/저지연 프로필 비교 시.
- reference/distorted 정렬(alignment)이 프레임 인덱스 기반 naive zip으로 구현되어 있고, 별도의 alignment 단계(PTS 기반 매칭, 프레임 지문 매칭)가 없을 때.

**권장**:
```rust
pub struct AlignmentResult {
    pub matched: Vec<(FrameRef, FrameRef, f64)>,
    pub dropped_in_distorted: Vec<FrameRef>, // reference에는 있지만 distorted에 없음
    pub extra_in_distorted: Vec<FrameRef>,   // distorted에만 있는 프레임(듀플리케이션 등)
}

pub fn align_and_score(reference: &[Frame], distorted: &[Frame]) -> AlignmentResult {
    let alignment = temporal_align(reference, distorted); // PTS/지문 기반 정렬
    // 드롭된 프레임은 별도 리스트에 명시적으로 보관하고,
    // 리포트에는 dropped_frame_ratio가 top-level 필드로 노출된다
    alignment
}
```
- 드롭된 프레임은 통계에서 제외하더라도 "몇 개가, 어느 구간에서 제외됐는지"를 리포트 최상위 필드(`dropped_frame_count`, `dropped_frame_ratio`)로 반드시 노출한다.
- 드롭 자체를 품질 저하 이벤트로 취급해 별도 지표(freeze duration, drop rate)로 리포트에 포함하는 것을 고려한다.

**탐지 방법**:
- Static: 두 프레임 시퀀스에 대해 `zip()`을 직접 사용하는 정렬/비교 코드 검색.
- Semantic: 리포트 타입에 `dropped_frame_count`/`dropped_frame_ratio` 필드 존재 여부 확인.
- Runtime: reference/distorted 길이가 의도적으로 다른 합성 픽스처로 드롭 카운트가 정확히 보고되는지 검증.

**예외**:
- reference와 distorted가 프레임 단위로 1:1 대응이 프로토콜상 보장되는 파이프라인(예: 동일 인코더의 무손실 리사이즈 비교처럼 프레임 드롭이 원천적으로 불가능한 경우)이라면 별도 정렬 단계 없이 zip을 써도 되지만, 그 전제가 문서화되어 있어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-008: 무효 frame을 0점으로 넣음
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn score_frame(reference: &Frame, distorted: &Frame) -> f64 {
    match compute_ssim(reference, distorted) {
        Ok(score) => score,
        Err(_) => 0.0, // 계산 실패(크기 불일치, 디코딩 오류 등)를 "최악의 점수"로 취급
    }
}
```

**문제**:
- "측정 실패"와 "측정 결과 최악"은 완전히 다른 사건인데 같은 값(0.0)으로 합쳐지면, 이후 통계(평균, percentile, worst-frame 탐색)가 실패 원인과 실제 화질 저하를 구분하지 못한다.
- 디코딩 오류나 해상도 불일치로 실패한 프레임이 다수 섞이면, 실제로는 준수한 화질임에도 평균이 크게 낮아져 "인코더가 나쁘다"는 잘못된 결론에 도달한다.
- 0점이 실제로 관측 가능한 최저 점수 범위와 겹치는 metric(예: 0~1 정규화된 지표)이라면, 실패 프레임이 "정말로 최악이었던 프레임"과 통계적으로 구분 불가능해진다.

**발생 조건**:
- 집계 함수가 `Result<f64, Error>`가 아니라 `f64`를 직접 반환하도록 설계되어 있어, 에러를 표현할 타입이 sentinel 값(0.0, -1.0, NaN 등)밖에 없을 때.
- "일단 파이프라인이 죽지 않게" 에러를 임의의 숫자로 뭉개고 넘어가는 방어적 코딩이 습관화되어 있을 때.

**권장**:
```rust
pub enum FrameScore {
    Valid(f64),
    Invalid { frame_index: u32, reason: InvalidReason },
}

pub enum InvalidReason {
    DecodeFailure,
    DimensionMismatch,
    AlignmentFailure,
}

pub fn aggregate(scores: &[FrameScore]) -> (f64, usize /* invalid_count */) {
    let valid: Vec<f64> = scores.iter()
        .filter_map(|s| match s { FrameScore::Valid(v) => Some(*v), _ => None })
        .collect();
    let invalid_count = scores.len() - valid.len();
    (mean_of(&valid), invalid_count)
}
```
- 실패는 값이 아니라 타입으로 표현하고(`FrameScore` enum), 집계 함수는 유효 표본만으로 계산하되 무효 표본 수를 함께 반환한다.
- 무효 프레임 비율이 임계값을 넘으면 리포트 자체에 경고 배지를 붙인다 (STAT-010, STAT-011과 연결).

**탐지 방법**:
- Static: 에러 처리 분기에서 magic number(`0.0`, `-1.0`, `100.0`)를 반환하는 패턴 grep.
- Runtime: 의도적으로 디코딩 실패를 유발하는 프레임을 포함한 픽스처로 평균이 왜곡되지 않는지 회귀 테스트.

**예외**:
- 해당 metric의 정의상 "계산 불가 = 최악"이 도메인적으로 참인 극히 드문 경우(예: "이 프레임이 존재하는가" 자체가 metric인 completeness 지표)라면 0점 처리가 정당화될 수 있다 — 단, 이 경우도 명시적으로 문서화되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-009: 무효 frame을 조용히 제거
**분류**: STAT · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn aggregate(raw_scores: &[f64]) -> f64 {
    let valid: Vec<f64> = raw_scores.iter()
        .copied()
        .filter(|s| s.is_finite()) // NaN/Inf를 그냥 걸러낸다
        .collect();
    valid.iter().sum::<f64>() / valid.len() as f64
    // 몇 개가 걸러졌는지, 왜 NaN/Inf가 나왔는지 어디에도 기록되지 않는다
}
```

**문제**:
- STAT-007(드롭 프레임)과 다른 실패 모드다 — 여기서는 두 스트림 모두 프레임이 "존재"하지만, metric 계산 자체가 NaN/Inf 같은 무효 값을 냈고(예: 완전히 검은 프레임에서 SSIM 분모가 0이 되는 edge case) 그것을 아무 흔적 없이 필터링한다.
- 필터링된 개수가 기록되지 않으면 분모(sample_count)가 원본 프레임 수와 조용히 달라지고, 이후 다른 metric과 결과를 나란히 비교할 때 "같은 N개 프레임을 봤다"는 암묵적 가정이 깨진다.
- NaN이 발생하는 원인(검은 프레임, 크롭 경계, 정렬 오류)이 반복적으로 특정 콘텐츠 유형에서 나타난다면, 그 자체가 파이프라인 버그의 신호인데 조용히 필터링되면 그 신호가 영구히 사라진다.

**발생 조건**:
- metric 구현이 특정 edge case(제로 분산 블록, 완전 단색 영역)에서 수학적으로 정의되지 않은 값을 낼 수 있는데, 이를 사전에 특수 처리하지 않고 사후 필터링으로만 방어할 때.

**권장**:
```rust
pub struct AggregationResult {
    pub mean: f64,
    pub valid_count: usize,
    pub filtered_count: usize,
    pub filtered_reasons: Vec<(u32 /* frame_index */, &'static str)>,
}

pub fn aggregate(raw_scores: &[(u32, f64)]) -> AggregationResult {
    let mut valid = Vec::new();
    let mut filtered_reasons = Vec::new();
    for &(idx, s) in raw_scores {
        if s.is_finite() {
            valid.push(s);
        } else {
            filtered_reasons.push((idx, if s.is_nan() { "nan" } else { "inf" }));
        }
    }
    AggregationResult {
        mean: mean_of(&valid),
        valid_count: valid.len(),
        filtered_count: filtered_reasons.len(),
        filtered_reasons,
    }
}
```
- 필터링된 프레임의 개수와 사유를 결과 구조체에 남기고, 필터링 비율이 임계값을 넘으면 로그/경고로 표면화한다.
- 가능하면 필터링보다 근본 원인(왜 NaN이 나오는가)을 metric 구현 단계에서 처리(예: 분모에 epsilon 추가, 특수 케이스 명시적 정의)하는 것을 우선한다.

**탐지 방법**:
- Static: `is_finite()`, `is_nan()` 필터가 카운트/로깅 없이 바로 `filter()`/`retain()`으로 이어지는 패턴 검색.
- Manual: "필터링된 개수를 어디서 확인할 수 있는가?"를 코드 리뷰 체크리스트에 포함.

**예외**:
- 프로토타입/디버그 전용 스크립트에서 빠르게 값을 확인하는 용도라면 조용한 필터링이 허용되지만, 리포트나 회귀 테스트 경로에는 절대 이 패턴을 두지 않는다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-010: metric별 유효 frame 수를 표시하지 않음
**분류**: STAT · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct MultiMetricReport {
    pub psnr_mean: f64,
    pub ssim_mean: f64,
    pub vmaf_like_mean: f64,
    // 각 metric이 몇 개의 유효 프레임으로 계산됐는지는 어디에도 없다
    // (metric마다 실패하는 프레임 집합이 다를 수 있음에도)
}
```

**문제**:
- 서로 다른 metric은 서로 다른 이유로 서로 다른 프레임에서 실패할 수 있다(예: 정렬 실패는 모든 metric에 영향을 주지만, 특정 색공간 변환 실패는 특정 metric에만 영향). 유효 개수를 표시하지 않으면 "세 metric이 정확히 같은 프레임 집합을 봤다"는 잘못된 전제가 암묵적으로 강요된다.
- N이 작은 metric(예: 300프레임 중 12개만 유효)의 결과가 N이 큰 metric(300개 모두 유효)과 시각적으로 동일한 신뢰도로 제시되면, 사용자는 표본이 부족한 숫자를 과신하게 된다.
- 버그(정렬 실패, 색공간 오판정)로 인해 특정 metric의 유효 표본이 급감해도 리포트 형태만으로는 이를 감지할 방법이 없다 — 회귀가 조용히 숨는다.

**발생 조건**:
- 여러 metric을 병렬로 계산하는 파이프라인에서 각 metric의 실패율이 서로 다를 수 있다는 사실을 리포트 스키마 설계 시 고려하지 않았을 때.

**권장**:
```rust
pub struct MetricSummary {
    pub name: String,
    pub mean: f64,
    pub valid_frame_count: usize,
    pub total_frame_count: usize,
}

pub struct MultiMetricReport {
    pub metrics: Vec<MetricSummary>,
}

impl MetricSummary {
    pub fn coverage_ratio(&self) -> f64 {
        self.valid_frame_count as f64 / self.total_frame_count as f64
    }
}
```
- 모든 metric 요약은 `valid_frame_count / total_frame_count` 쌍을 필수 필드로 갖는다.
- UI에서 coverage_ratio가 임계값(예: 95%) 미만이면 시각적 경고(낮은 채도, 주석 아이콘 등)를 표시한다.

**탐지 방법**:
- Structural: `MultiMetricReport`류 타입 정의에서 각 metric 필드 옆에 count 필드가 병렬로 존재하는지 대조.
- Manual: "이 세 metric이 정말 같은 프레임들을 대상으로 계산됐는가?"를 리뷰 질문으로 명시.

**예외**:
- 모든 metric이 동일한 사전 필터링된 프레임 집합에서만 계산되도록 파이프라인이 구조적으로 보장하는 경우(단일 valid_frame_mask를 모든 metric 계산 전에 공유), 개별 metric마다 카운트를 반복 표시할 필요는 없고 공유 마스크의 크기 하나만 표시해도 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-011: confidence나 alignment quality를 결과에서 제외
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub struct ComparisonResult {
    pub metric_scores: Vec<MetricSummary>,
    // temporal_align()이 내부적으로 계산하는 alignment confidence는
    // 이 구조체 어디에도 담기지 않는다
}

fn run_comparison(reference: &[Frame], distorted: &[Frame]) -> ComparisonResult {
    let alignment = temporal_align(reference, distorted);
    // alignment.confidence가 0.3(매우 낮음)이어도 그대로 metric을 계산해 반환
    let scores = compute_metrics(&alignment);
    ComparisonResult { metric_scores: scores }
}
```

**문제**:
- temporal alignment(두 스트림의 프레임을 시간축으로 정렬하는 단계)의 신뢰도가 낮으면, 그 위에서 계산된 모든 프레임별 metric은 "잘못 짝지어진 프레임끼리 비교"한 결과일 수 있다. 이 정보가 리포트에서 빠지면 사용자는 낮은 점수를 "화질이 나쁘다"로 오독하고, 실제로는 "정렬이 잘못됐다"는 진짜 원인을 놓친다.
- alignment confidence, 색공간 변환 신뢰도, crop/pad 추정치 등 파이프라인 중간 단계의 "이 결과를 얼마나 믿을 수 있는가"에 대한 신호가 최종 결과와 함께 보존되지 않으면, 숫자만 남고 그 숫자의 신뢰도는 영원히 사라진다.
- 동일한 낮은 점수라도 "정렬 신뢰도 99% + 실제 화질 저하"와 "정렬 신뢰도 40% + 정렬 오류로 인한 가짜 저하"는 완전히 다른 조치를 요구하는데, 리포트만 봐서는 구분할 수 없다.

**발생 조건**:
- alignment/전처리 모듈이 confidence를 내부적으로 계산은 하지만 그 값을 다음 단계로 전달하는 인터페이스가 없을 때 — 특히 모듈 간 경계가 "점수만 주고받는" 좁은 계약으로 설계됐을 때.

**권장**:
```rust
pub struct ComparisonResult {
    pub metric_scores: Vec<MetricSummary>,
    pub alignment_confidence: f64,
    pub alignment_method: AlignmentMethod,
    pub warnings: Vec<QualityWarning>,
}

fn run_comparison(reference: &[Frame], distorted: &[Frame]) -> ComparisonResult {
    let alignment = temporal_align(reference, distorted);
    let scores = compute_metrics(&alignment);
    let mut warnings = Vec::new();
    if alignment.confidence < 0.7 {
        warnings.push(QualityWarning::LowAlignmentConfidence(alignment.confidence));
    }
    ComparisonResult {
        metric_scores: scores,
        alignment_confidence: alignment.confidence,
        alignment_method: alignment.method,
        warnings,
    }
}
```
- 파이프라인의 각 전처리/정렬 단계가 만들어내는 신뢰도 신호는 최종 결과 타입까지 관통해서 전달한다.
- 신뢰도가 임계값 미만이면 결과를 숨기는 대신, 결과와 함께 명시적 경고를 붙여서 사용자가 판단할 수 있게 한다 (결과를 아예 숨기면 STAT-009와 같은 문제가 된다).

**탐지 방법**:
- Structural: alignment/전처리 단계 함수의 반환 타입에 confidence류 필드가 있는지, 그 필드가 최종 리포트 타입까지 전달되는 호출 경로를 추적.
- Manual: "이 metric 점수가 낮다면, 그것이 화질 문제인지 정렬 문제인지 리포트만으로 구분되는가?" 질문.

**예외**:
- alignment가 결정론적이고 confidence 개념 자체가 없는 방식(예: 컨테이너 타임스탬프가 완벽히 신뢰되는 controlled 환경에서 프레임 인덱스 직접 매핑)이라면 confidence 필드를 상수로 생략해도 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-012: scene cut 인접 프레임의 metric 하락을 동일하게 해석
**분류**: STAT · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn find_worst_frames(scores: &[FrameMetric], n: usize) -> Vec<FrameMetric> {
    let mut sorted = scores.to_vec();
    sorted.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    sorted.into_iter().take(n).collect()
    // scene cut 직후 프레임(인코더가 키프레임을 준비하며 일시적으로
    // 품질이 낮게 잡히는 구간)이 "진짜 화질 문제 프레임"과 뒤섞여 나온다
}
```

**문제**:
- scene cut 직후 1~2프레임은 인코더의 rate control ramp-up, GOP 구조, 그리고 metric 계산 자체의 특성(이전 프레임과 내용이 완전히 달라져 SSIM/시간적 지표가 구조적으로 낮게 나옴) 때문에 "정상적으로 예상되는" 하락을 보인다. 이를 실제 인코딩 결함과 같은 통계 버킷에 넣으면 worst-frame 리스트가 온통 scene cut 근방 프레임으로 도배된다.
- 사용자가 "worst 10 frames"를 검토했을 때 전부 scene cut이면, 정작 GOP 중간에 발생한 진짜 아티팩트(블록킹, 링잉)는 순위에서 밀려나 리포트에서 보이지 않게 된다.
- scene cut 인접 여부를 구분하지 않는 automated regression gate는 scene 구성이 조금만 바뀌어도(즉 화질과 무관하게 컷 위치가 바뀌면) worst-frame 통계가 요동쳐 false positive/negative를 반복한다.

**발생 조건**:
- scene 경계 정보(scene detection 결과)가 metric 파이프라인에 존재하지만, worst-frame 랭킹이나 이상치 탐지 로직이 그 정보를 참조하지 않고 순수 점수만으로 정렬할 때.

**권장**:
```rust
pub struct FrameMetric {
    pub frame_index: u32,
    pub score: f64,
    pub distance_from_scene_cut: Option<u32>, // None이면 scene cut 근처 아님
}

pub fn find_worst_frames(scores: &[FrameMetric], n: usize) -> WorstFramesReport {
    let (near_cut, steady_state): (Vec<_>, Vec<_>) = scores.iter()
        .partition(|f| f.distance_from_scene_cut.map_or(false, |d| d <= 2));

    WorstFramesReport {
        steady_state_worst: top_n_worst(&steady_state, n),
        scene_cut_adjacent_worst: top_n_worst(&near_cut, n),
    }
}
```
- worst-frame 분석은 scene cut 인접 구간과 steady-state 구간을 분리해서 별도로 랭킹을 매긴다.
- scene cut 근접 프레임의 하락은 별도 카테고리("예상된 전환 비용")로 리포트하고, steady-state 하락만 "진짜 인코딩 결함 후보"로 강조한다.

**탐지 방법**:
- Semantic: worst-frame/이상치 탐지 로직이 scene boundary 정보를 입력으로 받는지 함수 시그니처 검사.
- Runtime: scene cut을 다수 포함한 합성 시퀀스에서 worst-N 리스트가 scene cut 프레임에 편중되는지 통계적으로 검증.

**예외**:
- scene cut 자체의 화질(즉 전환이 얼마나 매끄러운지)을 평가하는 것이 분석 목적이라면, 오히려 scene cut 인접 프레임만 별도로 모아서 분석하는 것이 맞는 접근이다 — 이 경우 "동일하게 해석하지 않는다"는 원칙이 반대 방향으로 적용된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-013: 단일 worst frame만으로 인코더를 평가
**분류**: STAT · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn encoder_verdict(scores: &[f64], floor: f64) -> bool {
    let worst = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    worst >= floor // 단 하나의 최저 프레임 점수로 pass/fail 결정
}
```

**문제**:
- 단일 min 값은 노이즈에 극도로 취약하다 — metric 구현의 edge case(예: 완전 정지 화면에서의 수치적 불안정성), 정렬 오차 1프레임, 혹은 그저 통계적 이상치 하나가 전체 인코더 평가를 뒤집을 수 있다.
- "최악의 프레임"이 반복적으로 나타나는 구조적 문제(특정 콘텐츠 유형에서 지속적으로 낮은 품질)인지, 아니면 단발성 이상치인지 min 값 하나로는 전혀 구분되지 않는다.
- 인코더 A/B 비교에서 min만 기준으로 삼으면, 전반적으로 훨씬 우수하지만 단 1프레임에서 미세하게 낮은 인코더가 "실패"로 판정되고, 전반적으로 열등하지만 그 1프레임만 우연히 괜찮은 인코더가 "합격"하는 역전이 발생한다.

**발생 조건**:
- "최악의 경우를 보장한다"는 요구사항을 문자 그대로 min() 함수로 구현할 때 — 의도(안정성 보장)는 맞지만 구현(단일 표본 의존)이 그 의도를 배신한다.
- QA 게이트를 빠르게 만들어야 해서 percentile 계산 대신 min/max로 지름길을 택할 때.

**권장**:
```rust
pub struct RobustWorstCaseVerdict {
    pub p1_score: f64,        // 최악 1% 대표값 — 단일 이상치에 덜 민감
    pub worst_frame_count_below_floor: usize, // floor 미만 프레임이 몇 개인가
    pub single_min_score: f64, // 참고용으로는 유지하되 판정 기준으로 쓰지 않음
}

pub fn encoder_verdict(scores: &[FrameMetric], floor: f64) -> RobustWorstCaseVerdict {
    let below_floor = scores.iter().filter(|f| f.score < floor).count();
    RobustWorstCaseVerdict {
        p1_score: percentile_scores(scores, 0.01),
        worst_frame_count_below_floor: below_floor,
        single_min_score: scores.iter().map(|f| f.score).fold(f64::INFINITY, f64::min),
    }
}
```
- pass/fail 게이트는 percentile(p1 등) 또는 "임계값 미만 프레임의 개수/비율"을 기준으로 하고, 단일 min은 참고 정보로만 함께 보고한다.
- 최저 프레임이 반복 패턴인지 단발 이상치인지 구분하기 위해, "floor 미만 프레임의 개수"와 "그 프레임들의 시간적 분포(연속 구간인가 산발적인가)"를 함께 리포트한다.

**탐지 방법**:
- Static: QA/게이트 로직에서 `.min()`이나 `fold(f64::INFINITY, f64::min)`이 곧바로 pass/fail 조건에 쓰이는 패턴 검색.
- Manual: "이 게이트를 통과/실패시키는 데 프레임이 몇 개나 관여하는가?"를 리뷰에서 질문 — 답이 "1개"라면 경고.

**예외**:
- 안전 필수 도메인(예: 의료 영상, 특정 규제 준수 검증)에서 "단 한 프레임이라도 절대적 최저 기준을 위반하면 무조건 실패"가 명시적 요구사항인 경우, min 기반 하드 게이트가 의도적으로 맞는 설계다 — 단, 이 경우도 그 최저 프레임이 무엇인지 사용자가 검증할 수 있도록 frame_index/timestamp를 반드시 함께 제공해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-014: temporal pooling 방식을 기록하지 않음
**분류**: STAT · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub struct SequenceReport {
    pub metric_name: String,
    pub score: f64, // 92.3 — 이 숫자가 산술평균인지, 조화평균인지,
                     // p10인지, min인지 리포트 어디에도 없다
}

pub fn pool(frame_scores: &[f64]) -> f64 {
    // arithmetic mean, harmonic mean, min-pooling 중 하나를
    // 함수 내부에서 선택하지만 그 선택이 결과에 남지 않는다
    harmonic_mean(frame_scores)
}
```

**문제**:
- 동일한 프레임별 점수 배열이라도 pooling 방식(산술평균/조화평균/기하평균/percentile/min-pooling)에 따라 최종 숫자가 크게 달라진다. 어떤 방식이 쓰였는지 기록되지 않으면 두 리포트의 숫자를 비교하는 것 자체가 의미가 없어질 수 있다.
- pooling 방식이 시간이 지나 코드에서 변경돼도(예: 성능 최적화나 "더 정확한" 방식으로의 전환), 과거에 저장된 리포트와 새 리포트가 같은 스키마(`score: f64`)를 쓰기 때문에 겉보기엔 "같은 종류의 숫자"처럼 보이지만 실제로는 비교 불가능한 값이다.
- 버그 재현이나 감사(audit) 시 "이 92.3이라는 숫자가 어떻게 계산됐는가"를 코드 히스토리를 뒤져야만 알 수 있어, 재현성이 리포트 자체가 아니라 소스 코드 버전에 암묵적으로 의존하게 된다.

**발생 조건**:
- pooling 로직이 실험적으로 여러 번 바뀌는 초기 개발 단계에서, 리포트 스키마에 "어떻게 계산했는가"를 기록할 필드를 처음부터 만들어두지 않았을 때.
- pooling 함수가 여러 개(harmonic_mean, arithmetic_mean 등) 존재하고 호출부에서 상황에 따라 다른 것을 골라 쓰는데, 그 선택이 결과 값에 태그로 남지 않을 때.

**권장**:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PoolingMethod {
    ArithmeticMean,
    HarmonicMean,
    Percentile(u8),
    MinPooling,
}

pub struct SequenceReport {
    pub metric_name: String,
    pub score: f64,
    pub pooling_method: PoolingMethod,
    pub pooling_config_version: u32,
}

pub fn pool(frame_scores: &[f64], method: PoolingMethod) -> f64 {
    match method {
        PoolingMethod::ArithmeticMean => arithmetic_mean(frame_scores),
        PoolingMethod::HarmonicMean => harmonic_mean(frame_scores),
        PoolingMethod::Percentile(p) => percentile(frame_scores, p as f64 / 100.0),
        PoolingMethod::MinPooling => frame_scores.iter().cloned().fold(f64::INFINITY, f64::min),
    }
}
```
- pooling 방식은 함수 내부에 숨기지 말고 명시적 파라미터로 받고, 결과 리포트에 그대로 태그로 남긴다.
- 두 리포트를 비교하는 모든 코드 경로(차트, diff, 회귀 감지)는 `pooling_method`가 동일한지 먼저 확인하고, 다르면 비교를 거부하거나 명확히 경고한다.

**탐지 방법**:
- Structural: 결과 리포트 타입에 pooling 방식을 나타내는 필드가 있는지 확인.
- Manual: "이 score 필드 값 두 개를 나란히 비교해도 되는가?"를 리뷰에서 질문 — pooling 방식 기록이 없으면 대답할 수 없다는 것 자체가 문제 신호.

**예외**:
- 시스템 전체에서 pooling 방식이 단 하나로 고정되어 있고 그 사실이 시스템 설계 문서/버전 정책으로 강하게 보장되는 경우, 매 리포트마다 상수를 반복 기록하는 대신 스키마 버전 하나로 대체할 수 있다 — 단, 스키마 버전이 바뀌면 pooling 방식도 바뀔 수 있다는 점을 릴리스 노트에 명시해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-015: 서로 다른 버전 결과를 동일 차트에서 비교
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn render_trend_chart(history: &[SequenceReport]) -> ChartData {
    // metric 구현 버전, 모델 버전, 전처리 설정이 서로 다른 리포트가
    // 섞여 있어도 시간순으로 그냥 이어 그린다
    ChartData {
        points: history.iter().map(|r| (r.timestamp, r.score)).collect(),
    }
}
```

**문제**:
- metric 구현(예: SSIM 계산 라이브러리 버전, VMAF류 모델 버전)이나 전처리 설정(색공간 변환, 리사이즈 필터)이 바뀌면 동일한 콘텐츠에서도 점수 자체의 스케일과 분포가 달라질 수 있다. 이를 구분하지 않고 시계열 차트에 이어 붙이면, 실제로는 코덱/인코더 변경이 아니라 측정 도구 변경으로 인한 점프를 "품질 변화"로 오독하게 된다.
- 회귀 감지(regression detection) 자동화가 이 차이를 구분하지 못하면, 매 metric 버전 업그레이드마다 대량의 false positive 알림이 발생하거나, 반대로 실제 품질 회귀가 버전 변경의 노이즈에 묻혀 놓친다.
- 사용자가 "지난 분기 대비 품질이 좋아졌다"는 결론을 내릴 때, 그것이 인코더 개선 때문인지 metric 구현이 관대해졌기 때문인지 차트만 봐서는 절대 구분할 수 없다.

**발생 조건**:
- metric 라이브러리나 모델을 업그레이드하면서 과거 데이터를 재계산하지 않고, 새 버전으로 계산한 값을 기존 히스토리 테이블에 그냥 이어 쓸 때.
- 리포트 저장 스키마에 버전 정보 필드가 아예 없거나, 있어도 차트 렌더링 로직이 그 필드를 무시할 때.

**권장**:
```rust
pub struct SequenceReport {
    pub timestamp: DateTime<Utc>,
    pub score: f64,
    pub run_metadata: MetricRunMetadata,
}

pub struct MetricRunMetadata {
    pub metric_impl_version: String,
    pub model_version: Option<String>,
    pub preprocessing_config_hash: String,
    pub alignment_config_hash: String,
    pub aggregation_config: PoolingMethod,
}

pub fn render_trend_chart(history: &[SequenceReport]) -> Vec<ChartSeries> {
    // run_metadata가 다른 구간은 서로 다른 series(다른 색/스타일, 버전 경계 마커)로 분리
    group_by_run_metadata(history)
        .into_iter()
        .map(|(meta, points)| ChartSeries { meta, points })
        .collect()
}
```
- 버전이 다른 결과는 같은 series로 이어 그리지 않고, 최소한 버전 경계에 수직선 마커를 표시하거나 별도 series로 분리한다.
- 장기 트렌드 비교가 꼭 필요하다면, 과거 데이터를 새 버전으로 재계산(backfill)하는 파이프라인을 갖추고 "재계산됨" 여부를 메타데이터에 남긴다.

**탐지 방법**:
- Structural: 시계열/트렌드 차트 렌더링 함수가 `MetricRunMetadata`(또는 그에 준하는 버전 필드)를 그룹핑 키로 사용하는지 확인.
- Semantic: 히스토리 테이블 스키마에 버전 필드가 있는지, 있다면 실제로 서로 다른 값이 섞여 저장되어 있는지 데이터 감사.

**예외**:
- metric 구현/모델/설정이 이번 비교 목적상 변경되지 않았음이 배포 파이프라인 수준에서 보장되는 짧은 기간의 대시보드(예: 단일 CI 실행 내 A/B 비교)라면 버전 분리 로직 없이 단순 비교해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-016: 합성 점수의 가중치 근거를 기록하지 않음
**분류**: STAT · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn composite_score(psnr: f64, ssim: f64, vmaf_like: f64) -> f64 {
    // 왜 0.2/0.3/0.5인지, 언제 이 가중치가 정해졌는지 코드 어디에도 없다
    psnr * 0.2 + ssim * 0.3 + vmaf_like * 0.5
}
```

**문제**:
- 여러 metric을 하나의 종합 점수로 합성할 때 가중치는 사실상 "무엇이 중요한가"에 대한 제품 의사결정인데, 그것이 매직 넘버로 코드에 박혀 있으면 왜 이 가중치가 선택됐는지, 어떤 콘텐츠/유스케이스를 기준으로 튜닝됐는지 아무도 알 수 없다.
- metric 하나의 스케일이 바뀌면(예: 구현 버전업으로 vmaf_like의 분포 범위가 달라짐) 가중치가 더 이상 원래 의도한 균형을 반영하지 못하게 되는데, 가중치 자체가 별도로 검토/갱신되는 프로세스가 없으면 이 불일치가 영구히 방치된다.
- 서로 다른 유스케이스(스트리밍 vs 아카이빙 vs 실시간 통화)가 요구하는 metric 우선순위가 다를 수 있는데, 가중치가 하드코딩되어 있으면 유스케이스별로 다른 합성 점수를 제공할 수 없다.

**발생 조건**:
- "일단 그럴듯한 가중치로 시작하자"는 임시 결정이 리뷰나 문서화 없이 그대로 프로덕션 기본값으로 굳어질 때.
- 종합 점수가 대시보드 랭킹처럼 파급력이 큰 곳에 쓰이기 시작한 뒤에도 가중치 결정 근거를 소급 문서화하지 않을 때.

**권장**:
```rust
pub struct CompositeWeights {
    pub weights: Vec<(String /* metric name */, f64)>,
    pub rationale: &'static str,
    pub calibrated_against: &'static str, // 어떤 콘텐츠 세트로 튜닝됐는지
    pub version: u32,
}

pub fn composite_score(scores: &HashMap<String, f64>, weights: &CompositeWeights) -> f64 {
    weights.weights.iter()
        .map(|(name, w)| scores.get(name).copied().unwrap_or(0.0) * w)
        .sum()
}
```
- 가중치는 코드에 매직 넘버로 두지 말고, 근거(rationale)와 버전을 함께 갖는 설정 데이터로 분리한다.
- 종합 점수를 리포트할 때 어떤 `CompositeWeights.version`이 쓰였는지 함께 노출해, STAT-015의 버전 비교 문제가 여기에도 적용되게 한다.

**탐지 방법**:
- Static: 여러 f64 metric 값에 리터럴 가중치 상수를 곱해 더하는 패턴(`* 0.2`, `* 0.3` 등이 한 표현식에 여러 개) 검색.
- Manual: "이 가중치는 언제, 왜, 무엇을 기준으로 정해졌는가"를 리뷰에서 질문.

**예외**:
- 업계 표준으로 이미 고정되어 있고 임의 변경이 오히려 비교 가능성을 해치는 합성 지표(예: 특정 표준 기구가 정의한 공식 가중치)를 그대로 구현하는 경우, 가중치를 상수로 두되 출처(표준 문서/버전)를 주석으로 남기는 것으로 충분하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-017: 통계적 유의성 없이 두 스트림의 우열을 결론
**분류**: STAT · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn compare_encoders(a_scores: &[f64], b_scores: &[f64]) -> &'static str {
    let mean_a = mean_of(a_scores);
    let mean_b = mean_of(b_scores);
    if mean_a > mean_b { "Encoder A is better" } else { "Encoder B is better" }
    // 평균 차이가 0.05점이든 5점이든, 프레임별 분산이 얼마든 동일한 확신으로 결론
}
```

**문제**:
- 프레임 단위 분산이 두 평균의 차이보다 훨씬 크다면, 관측된 평균 차이는 우연(표본 변동)일 가능성이 높은데도 단순 부등호 비교는 항상 단정적인 결론을 낸다.
- 이 패턴은 특히 인코더 튜닝처럼 반복적으로 A/B 비교가 이뤄지는 워크플로에서, 매번 근소한 차이를 "개선" 또는 "회귀"로 잘못 해석해 튜닝 방향을 흔들리게 만든다.
- "Encoder A is better"라는 문자열 자체가 하위 소비자(자동 랭킹, 알림)에 의해 그대로 신뢰되어, 실제로는 노이즈 수준의 차이가 조직의 의사결정 체인을 타고 증폭된다.

**발생 조건**:
- 표본 수가 적거나(짧은 클립), 콘텐츠가 특히 고분산인 경우(고모션, 복잡한 텍스처 혼재)에 A/B 비교를 수행할 때.
- 두 인코더 설정 간 실제 차이가 미세 튜닝 수준(예: CRF 1 단위 차이)이라 평균 차이 자체가 원래 작을 때.

**권장**:
```rust
pub struct ComparisonVerdict {
    pub mean_diff: f64,
    pub std_err_of_diff: f64,
    pub is_significant: bool, // 예: paired t-test 또는 부트스트랩 신뢰구간 기반
    pub confidence_interval_95: (f64, f64),
}

pub fn compare_encoders(a_scores: &[f64], b_scores: &[f64]) -> ComparisonVerdict {
    let diffs: Vec<f64> = a_scores.iter().zip(b_scores).map(|(a, b)| a - b).collect();
    let mean_diff = mean_of(&diffs);
    let std_err = std_error_of_mean(&diffs);
    let ci = confidence_interval_95(&diffs);
    ComparisonVerdict {
        mean_diff,
        std_err_of_diff: std_err,
        is_significant: !(ci.0 <= 0.0 && ci.1 >= 0.0), // CI가 0을 포함하지 않을 때만 유의
        confidence_interval_95: ci,
    }
}
```
- A/B 비교 결과는 "A가 낫다/B가 낫다"는 단정 대신 평균 차이 + 신뢰구간(또는 유의성 검정 결과)을 함께 제시한다.
- 신뢰구간이 0을 포함하면 "유의한 차이 없음"으로 명시적으로 보고하고, 하위 자동화가 이를 "무승부"로 처리하도록 인터페이스를 설계한다.

**탐지 방법**:
- Semantic: 두 점수 집합을 비교해 문자열/bool 결론을 내는 함수가 평균만 사용하고 분산/표준오차를 전혀 참조하지 않는지 확인.
- Runtime: 동일 분포에서 뽑은 두 노이즈 표본(실제로는 차이가 없어야 함)에 대해 비교 함수가 얼마나 자주 "유의미한 차이"를 보고하는지 시뮬레이션으로 false positive율 측정.

**예외**:
- 표본 수가 충분히 크고(전체 프레임 수천 개 이상) 관측된 차이가 표준오차 대비 압도적으로 클 때는, 굳이 정식 유의성 검정을 리포트에 노출하지 않아도 실질적 결론이 흔들리지 않는다 — 다만 이 경우도 내부적으로는 검정을 수행해 조건을 만족하는지 확인하는 것이 안전하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### STAT-018: 리샘플링/보간된 프레임을 원본과 동일 가중치로 집계
**분류**: STAT · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn align_framerates(reference: &[Frame], distorted: &[Frame]) -> Vec<(Frame, Frame)> {
    // reference 30fps, distorted 24fps일 때 distorted를 30fps로
    // 보간(프레임 복제 또는 optical flow 보간)해 맞춘 뒤,
    // 보간된 프레임인지 여부를 표시하지 않고 그대로 반환
    let upsampled = interpolate_to_framerate(distorted, reference.len());
    reference.iter().cloned().zip(upsampled).collect()
}
```

**문제**:
- 프레임레이트가 다른 두 스트림을 비교하려고 한쪽을 리샘플링/보간하면, 보간된 프레임은 "실제로 인코더가 만들어낸 화질"이 아니라 "보간 알고리즘이 추정한 값"이다. 이를 실제 프레임과 구분 없이 집계하면 통계가 인코더 품질이 아니라 보간 알고리즘의 특성을 섞어서 반영하게 된다.
- 보간 프레임 비율이 높은 구간(프레임레이트 차이가 큰 콘텐츠)일수록 최종 점수에서 "진짜 측정"의 비중이 낮아지는데, 리포트는 이를 구분하지 않으므로 사용자는 표본 전체가 동등하게 신뢰할 만하다고 오인한다.
- 서로 다른 프레임레이트 조합(30→24, 60→30 등)마다 보간 비율이 달라 오염 정도가 다른데, 이 정보 없이는 어떤 비교 결과가 더 신뢰할 만한지 판단할 수 없다.

**발생 조건**:
- reference/distorted 프레임레이트가 다른 콘텐츠(적응형 스트리밍 프로필 비교, 서로 다른 소스 프레임레이트 비교)를 다룰 때.
- 프레임레이트 정합을 위한 보간 단계가 alignment 모듈 내부에 있고, 그 출력이 "보간됨" 표시 없이 순수 Frame 타입으로만 흘러나갈 때.

**권장**:
```rust
pub struct AlignedFrame {
    pub frame: Frame,
    pub provenance: FrameProvenance,
}

pub enum FrameProvenance {
    Original,
    Interpolated { method: InterpolationMethod },
    Duplicated,
}

pub fn align_framerates(reference: &[Frame], distorted: &[Frame]) -> Vec<(Frame, AlignedFrame)> {
    // 보간된 프레임에는 provenance 태그를 남긴다
    interpolate_to_framerate(distorted, reference.len())
}

pub fn aggregate_with_provenance(scores: &[(f64, FrameProvenance)]) -> (f64 /* original_only */, f64 /* all */) {
    let original_only: Vec<f64> = scores.iter()
        .filter(|(_, p)| matches!(p, FrameProvenance::Original))
        .map(|(s, _)| *s)
        .collect();
    let all: Vec<f64> = scores.iter().map(|(s, _)| *s).collect();
    (mean_of(&original_only), mean_of(&all))
}
```
- 보간/복제된 프레임은 `FrameProvenance`로 태그를 남기고, 최종 통계는 "원본 프레임만" 기준과 "보간 포함 전체" 기준을 함께 리포트한다.
- 보간 비율이 임계값을 넘는 비교는 리포트에 경고를 붙이거나, 애초에 프레임레이트가 크게 다른 스트림 간 직접 비교를 지양하고 공통 저프레임레이트로 양쪽을 다운샘플링하는 대안을 검토한다 (다운샘플링은 "추정치 생성"이 아니라 "정보 버림"이므로 보통 보간보다 오염이 적다).

**탐지 방법**:
- Structural: 프레임레이트 정합 함수의 출력 타입에 provenance/origin 정보가 있는지 확인.
- Semantic: 집계 함수가 원본 프레임과 보간 프레임을 구분해 별도 통계를 낼 수 있는 입력을 받는지 검사.

**예외**:
- 두 스트림이 사실상 동일 프레임레이트이고 아주 드문 지터(예: 타임스탬프 반올림 오차로 인한 1프레임 미만 보정)만 있는 경우, 그 정도의 미세 보간은 최종 통계에 미치는 영향이 무시할 만하므로 provenance 추적을 생략해도 실질적 문제가 없다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

## 총평

이 카테고리 전체를 관통하는 주제 하나: VQ-Probe의 최종 산출물은 "숫자"가 아니라 "숫자 + 그 숫자가 어떻게 만들어졌는가"다. 위 18개 항목은 각기 다른 실패 모드(평균의 함정, 가중치 누락, 드롭/무효 프레임 처리, scene 경계 왜곡, pooling 방식 실종, 버전 간 비교 불가)를 다루지만, 근본 원인은 하나로 수렴한다 — `MetricRunMetadata`(metric 구현 버전, 모델 버전, 전처리 설정, alignment 설정, aggregation/pooling 설정)가 결과 숫자와 함께 저장되지 않는 것. 이 provenance가 없으면, 아무리 계산 자체가 수치적으로 정확해도 그 결과는 신뢰할 수 없다 — 무엇과 비교 가능한 숫자인지, 어떤 조건에서 유효한 숫자인지 아무도 답할 수 없기 때문이다. STAT 카테고리의 모든 리뷰는 최종적으로 이 질문 하나로 되돌아가야 한다: "이 숫자를 나중에 다른 숫자와 비교하려는 사람에게, 이 숫자만으로 충분한 정보가 주어지는가?"
