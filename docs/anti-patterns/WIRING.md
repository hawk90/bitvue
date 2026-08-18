# Anti-Pattern Catalog — WIRING: 죽은 채로 남은 올바른 구현 (Correct-but-Disconnected Code)

이 문서는 더 큰 안티패턴 카탈로그의 한 카테고리입니다(전체 인덱스는 `docs/anti-patterns/INDEX.md` 참고). 다른 46개
파일과 달리 이 파일은 **Wave 5**로, Phase 2 감사(47개 파일 전체의 `Bitvue 판정` 채우기)가 끝난 뒤에 추가되었습니다.
감사를 진행하며 정확히 같은 모양의 메타 패턴이 계속 반복되는 것이 드러났는데, 그 어떤 기존 파일도 이 패턴 자체를
카테고리로 다루지 않았습니다: **어딘가에 올바르고 동작하는 구현이 이미 존재하는데, 실제 프로덕션 코드 경로가 그것을
전혀 호출하지 않아 버그·공백이 실무에서 그대로 남아있는 경우**입니다. 이는 "기능 자체가 없음"(다른 파일의 대다수
N/A 판정)과도, "있는 코드가 틀림"(대다수 Confirmed 판정)과도 구조적으로 다른 세 번째 범주입니다 — 코드는 이미
누군가 맞게 짜뒀는데, 배선(wiring)이 끊겨 있을 뿐입니다.

다른 파일들은 "일반적인 안티패턴을 먼저 정의하고, 그 다음 저장소를 감사해 Confirmed/Suspected/N/A를 채우는" 순서로
만들어졌습니다. 이 파일은 그 반대입니다 — 모든 항목이 이미 완료된 Phase 2 감사에서 실제로 발견된 교차 참조
증거로부터 역으로 구성되었으며, 그래서 처음부터 예외 없이 `Bitvue 판정: Confirmed`를 가지고 태어났습니다. 각 항목의
"나쁜 예" 코드는 다른 파일들처럼 가상의 예시가 아니라 — 그렇게 표시하지 않으면 오해할 수 있으므로 각 항목에서
명시합니다 — 실제 프로덕션 코드의 모양을 일반화해 재구성한 스케치입니다. `Bitvue 판정` 줄에는 이 메타패턴이 이미
다른 파일에서 어떤 ID로 문서화되어 있는지 교차 참조를 남겨, 원본 파일에서 더 자세한 문제/권장/탐지 방법을 찾아볼 수
있게 했습니다. 이 파일의 "예외" 절은 모든 항목에서 동일한 원칙 하나를 반복합니다: **다른 Confirmed 클러스터
(ALIGN/COLOR/METRIC/STAT/PIPE 등)에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지
먼저 grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.**

---

### WIRE-001: 프레임 정렬 엔진이 실제 품질 지표 파이프라인에 연결되지 않음
**분류**: WIRING · **심각도**: High · **탐지**: Structural|Semantic

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// crates/bitvue-core/src/alignment.rs — 이미 존재하고 올바르게 동작함
pub struct AlignmentEngine { /* PTS 기반 정렬, confidence(High/Medium/Low), gap_percentage() */ }

// src-tauri/src/commands/compare.rs — AlignmentEngine을 정상적으로 사용
fn sync_playback(engine: &AlignmentEngine, ui_pts: i64) -> FramePair {
    engine.resolve_pair(ui_pts) // confidence/gap 정보까지 함께 반환
}

// src-tauri/src/commands/quality.rs — 같은 AlignmentEngine을 import하지 않고
// 자체적으로 원시 인덱스 zip을 다시 구현
fn calculate_quality_metrics(ref_frames: &[Frame], dist_frames: &[Frame]) -> BatchQualityMetrics {
    let n = ref_frames.len().min(dist_frames.len()); // 드롭/오프셋 조용히 무시
    let scores: Vec<_> = (0..n)
        .map(|idx| compute_score(&ref_frames[idx], &dist_frames[idx]))
        .collect();
    BatchQualityMetrics::from_scores(scores) // confidence/gap 필드 자체가 없음
}
```

**문제**:
- `AlignmentEngine`은 PTS 기반 정렬, confidence 등급, `gap_percentage()`까지 이미 구현·검증되어 있고 UI 동기 재생
  경로(`compare.rs`)에서는 정상적으로 쓰인다 — 즉 "아무도 이 문제를 몰랐다"가 아니라 "한 소비자는 이미 올바르게
  연결했는데 다른 소비자는 그러지 않았다"는 케이스다.
- 정작 PSNR/SSIM 최종 수치를 만드는 `calculate_quality_metrics`는 이 엔진을 참조조차 하지 않고 `ref_count.min(dist_count)`
  인덱스 zip으로 되돌아가, 프레임 드롭이 있으면 있는지 없는지 사용자가 알 방법이 없는 수치를 만든다.
- 두 코드 경로가 같은 저장소, 같은 크레이트 계층에 공존하는데도 서로를 참조하지 않는다는 것은, 코드 리뷰나 통합
  테스트가 "이 값이 다른 곳에서도 이미 계산되고 있는가"를 확인하지 않았다는 신호다.

**발생 조건**:
- 동일한 도메인 문제(여기서는 두 프레임 시퀀스의 시간축 정렬)를 두 개 이상의 기능(재생 동기화, 품질 지표)이 각자
  구현할 때, 그 사이에 공유 계약이나 통합 테스트가 없을 때.

**권장**:
```rust
fn calculate_quality_metrics(
    engine: &AlignmentEngine,
    ref_frames: &[Frame],
    dist_frames: &[Frame],
) -> BatchQualityMetrics {
    let alignment = engine.align(ref_frames, dist_frames);
    let scores = alignment.pairs.iter()
        .map(|p| compute_score(&ref_frames[p.ref_idx], &dist_frames[p.dist_idx]))
        .collect();
    BatchQualityMetrics {
        scores,
        alignment_confidence: alignment.confidence,
        gap_percentage: alignment.gap_percentage(),
        ..Default::default()
    }
}
```
- 새 기능을 짜기 전에 "이 도메인 개념(정렬, 캐시, 페이지네이션 등)을 다루는 타입이 이 워크스페이스에 이미 있는가"를
  먼저 grep한다.
- 같은 도메인 개념을 다루는 모든 소비자가 동일한 타입을 import하는지 CI/코드 리뷰 체크리스트에 넣는다.

**탐지 방법**:
- Structural: 같은 크레이트/워크스페이스 안에서 같은 문제(정렬, 캐싱, 인덱스 매핑 등)를 다루는 타입이 두 곳 이상에서
  독립적으로 재구현되어 있는지 대조.
- Semantic: 한쪽 소비자의 출력 타입에는 있는 필드(confidence, gap 등)가 다른 소비자의 출력 타입에는 없는 비대칭을 검색.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `ALIGN-010`(`docs/anti-patterns/ALIGN.md`, "alignment confidence를 제공하지 않음")이
이미 이 정확한 사례를 문서화: `crates/bitvue-core/src/alignment.rs`의 `AlignmentEngine`은 confidence(34-42행)와
`gap_percentage()`(253-290행)를 계산하지만 `quality.rs`의 `BatchQualityMetrics`/`FrameQualityMetrics`(15-40행)는
이를 전혀 참조하지 않는다. `STAT-007`(`docs/anti-patterns/STAT.md`, "dropped frame을 집계에서 제외하고 알리지
않음")도 동일 근거로 Confirmed: `quality.rs:467-470`의 `frames_to_process`가 `ref_count.min(dist_count)`로 조용히
잘리며, 같은 저장소에 gap_count/confidence를 정식 추적하는 `AlignmentEngine`(`alignment.rs:24-79`)이 있음에도
이 파이프라인은 그것을 쓰지 않고 인덱스 zip과 동일하게 동작한다. 반대로 `compare.rs`(`AlignmentEngineData`)는
confidence/gap을 최종 응답까지 정상 전달해(STAT.md 621행 참고) 이 엔진의 "올바른 배선" 사례도 같은 저장소에
공존함을 보여준다 — 즉 재작성이 아니라 import 한 줄의 문제다.

---

### WIRE-002: mmap 기반 ByteCache가 구현되어 있지만 모든 hot path가 여전히 전체 파일을 읽음
**분류**: WIRING · **심각도**: Critical · **탐지**: Structural

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// crates/bitvue-core/src/byte_cache.rs — mmap 기반, read_range()로 부분 접근 가능하게 이미 설계됨
pub struct ByteCache { mmap: memmap2::Mmap /* ... */ }
impl ByteCache {
    pub fn read_range(&self, start: u64, len: u64) -> &[u8] { &self.mmap[start as usize..(start + len) as usize] }
}

// src-tauri/src/commands/file.rs, quality.rs, compare.rs, services/decode_service.rs
// — ByteCache를 열기만 하고(또는 아예 모르는 채로) 매번 전체 파일을 힙에 로드
fn open_file(path: &Path) -> Vec<u8> {
    let mut buf = Vec::new();
    std::fs::File::open(path).unwrap().read_to_end(&mut buf).unwrap(); // 8K 원본이면 수십 GB
    buf
}
```

**문제**:
- `ByteCache`는 정확히 "전체 파일을 힙에 올리지 않기 위해" 설계된 mmap 래퍼인데, 이를 실제로 써야 할 4개의 hot
  path(`open_file`, `quality.rs`, `compare.rs`, `decode_service.rs`) 전부가 이를 참조하지 않고 각자 독립적으로
  `read_to_end`/`fs::read`를 호출한다.
- "해결책이 이미 워크스페이스에 있다"는 사실이 오히려 문제를 더 오래 방치시킨다 — 누군가 이미 만들어뒀다는 착각
  때문에 새로 대용량 파일 이슈가 보고돼도 "그거 이미 mmap으로 처리하지 않나?"라고 잘못 넘겨짚기 쉽다.
- 4곳이 각자 같은 안티패턴을 독립적으로 반복한다는 것은, 파일 읽기 경로에 대한 단일 진입점(facade)이 없다는 구조적
  신호이기도 하다 — 배선 문제이면서 동시에 API 설계 문제(중복 진입점)다.

**발생 조건**:
- 성능/메모리 문제를 미리 예견하고 인프라(캐시, 정렬 엔진, 취소 토큰 등)를 먼저 만들었지만, 그 인프라를 실제로
  소비해야 할 기능 코드가 나중에 별도로(또는 먼저) 작성되어 서로 연결되지 않을 때.

**권장**:
```rust
fn open_file(cache: &ByteCache) -> Result<StreamHandle, Error> {
    let header = cache.read_range(0, HEADER_PROBE_SIZE); // 필요한 범위만 접근
    parse_header(header)
}
```
- 파일 접근이 필요한 모든 경로가 단일 `ByteCache`(또는 동등한 facade) 인스턴스를 주입받게 강제하고, `std::fs::read`/
  `read_to_end` 직접 호출을 그 facade 내부로만 한정한다(clippy 커스텀 린트 또는 코드 리뷰 체크리스트로 강제).
- 새 파일 I/O 경로를 추가할 때 "이미 있는 캐시/추상화를 쓰고 있는가"를 PR 체크리스트 항목으로 명시한다.

**탐지 방법**:
- Structural: `std::fs::read`/`read_to_end` 호출부와 `ByteCache::read_range` 호출부를 각각 grep해 겹치지 않는
  호출부 목록을 만든다(겹치지 않으면 배선 누락 후보).
- Runtime: 대용량 픽스처로 각 경로의 peak RSS를 측정해 파일 크기에 비례하는지 확인.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `IO-001`(`docs/anti-patterns/IO.md`, "std::fs::read로 전체 파일 로드")이 이미 이
사례를 Confirmed로 문서화: `src-tauri/src/commands/file.rs`의 `open_file`이 `read_to_end`로 전체 파일을 `Vec<u8>`에
로드(257-262행)하며 `quality.rs:95,1006`, `compare.rs:161,230`, `services/decode_service.rs:256`,
`commands/analysis/mod.rs:319`도 동일 패턴이다. mmap 기반 `ByteCache`(`crates/bitvue-core/src/byte_cache.rs`)는
열리기만 하고 `read_range()`가 프로덕션에서 전혀 호출되지 않는다. `MEM-001`(`docs/anti-patterns/MEM.md`, "파일 전체
read_to_end")도 동일 파일들을 근거로 독립적으로 Confirmed 판정하며 "mmap 기반 ByteCache가 별도로 존재하지만 이
경로들에는 쓰이지 않음"이라고 명시한다.

---

### WIRE-003: Rayon 병렬 PSNR/SSIM 배치 함수가 존재하지만 실제 지표 루프는 완전히 순차적
**분류**: WIRING · **심각도**: Medium · **탐지**: Structural|Runtime

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// crates/bitvue-metrics/src/lib.rs — 이미 구현된 Rayon 병렬 배치 함수 (약 327-387행)
pub fn batch_psnr_parallel(refs: &[Frame], dists: &[Frame]) -> Vec<f64> {
    refs.par_iter().zip(dists.par_iter()).map(|(r, d)| psnr(r, d)).collect()
}
pub fn batch_ssim_parallel(refs: &[Frame], dists: &[Frame]) -> Vec<f64> { /* 동일 패턴 */ }

// src-tauri/src/commands/quality.rs — 위 함수들을 호출하지 않고 프레임마다 순차 루프
fn calculate_quality_metrics(ref_frames: &[Frame], dist_frames: &[Frame]) -> BatchQualityMetrics {
    let mut scores = Vec::with_capacity(ref_frames.len());
    for (r, d) in ref_frames.iter().zip(dist_frames.iter()) { // 346-535행대, 코어 1개만 사용
        scores.push(calculate_single_frame_metrics(r, d));
    }
    BatchQualityMetrics::from_scores(scores)
}
```

**문제**:
- `batch_psnr_parallel`/`batch_ssim_parallel`은 이미 작성되어 컴파일되고 있는 코드인데(dead code 경고조차 없을
  정도로 크레이트 공개 API로 노출돼 있음), 실제 사용자가 "품질 지표 계산"을 트리거했을 때 실행되는 코드는 이를
  전혀 부르지 않고 프레임 하나씩 순차 처리한다.
- 대용량 스트림(수천 프레임) 비교 시 병렬화 유무가 체감 대기 시간을 코어 수 배만큼 좌우하는데, 그 병렬화 코드가
  이미 존재함에도 사용자는 순차 처리의 대기 시간을 그대로 겪는다 — "성능 최적화를 못 했다"가 아니라 "이미 한
  최적화를 안 쓰고 있다".
- 이런 대칭(병렬 버전은 라이브러리에, 순차 버전은 커맨드에)은 두 함수가 서로 다른 시점/다른 사람에 의해 작성됐고
  하나가 다른 하나를 대체하지 못한 채 둘 다 남아있다는 신호다.

**발생 조건**:
- 성능 개선을 목적으로 병렬 버전 함수를 별도로 추가했지만, 기존 호출부(Tauri 커맨드)를 그 병렬 버전으로 교체하는
  후속 작업이 누락되었을 때.

**권장**:
```rust
fn calculate_quality_metrics(ref_frames: &[Frame], dist_frames: &[Frame]) -> BatchQualityMetrics {
    let psnr_scores = bitvue_metrics::batch_psnr_parallel(ref_frames, dist_frames);
    let ssim_scores = bitvue_metrics::batch_ssim_parallel(ref_frames, dist_frames);
    BatchQualityMetrics::from_scores(psnr_scores, ssim_scores)
}
```
- 병렬 버전을 추가하는 PR은 반드시 "이 병렬 버전을 실제로 호출하도록 기존 호출부를 교체했는가"를 완료 조건에
  포함시킨다 — 병렬 함수 추가 자체를 "끝"으로 보지 않는다.
- CI에 "정의는 됐지만 어떤 프로덕션 코드에서도 참조되지 않는 pub 함수"를 찾는 정적 검사(예: `cargo udeps` 또는
  커스텀 grep 스크립트)를 추가해 이런 배선 누락을 자동 감지한다.

**탐지 방법**:
- Structural: `pub fn` 정의부와 워크스페이스 전체의 호출부를 대조해, 정의만 있고 `src-tauri`에서 참조되지 않는
  성능 관련 함수를 찾는다.
- Runtime: `calculate_quality_metrics` 실행 중 CPU 코어 사용률을 모니터링해 단일 코어에 근접하는지 확인.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `PIXEL-018`(`docs/anti-patterns/PIXEL.md`, "metrics 병렬화로 memory bandwidth 포화")이
이미 `batch_psnr_parallel`(`crates/bitvue-metrics/src/lib.rs:328-353`)과 `batch_ssim_parallel`(`:365-387`)의 존재와
구현 방식(전역 Rayon pool, PSNR/SSIM 각각 별도 전체 순회)을 Confirmed로 기록했다. 반면 `CONC-019`
(`docs/anti-patterns/CONC.md`, "metrics 계산이 UI interaction을 방해")는 실제 사용자 경로인
`calculate_quality_metrics`(`quality.rs:441-500대`)가 "동기적으로" 프레임을 디코드·채점한다고 Confirmed로 명시해,
두 항목을 겹쳐 읽으면 "병렬 함수는 라이브러리에 존재 확인됨" + "실사용 경로는 순차 확인됨"이 대조된다 — 이 파일이
그 대조 자체를 배선 문제로 명명한다.

---

### WIRE-004: 통계 분포/최악-프레임 타입이 구현되어 있지만 어떤 커맨드에서도 호출되지 않음
**분류**: WIRING · **심각도**: High · **탐지**: Structural|Semantic

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// crates/bitvue-core/src/metrics_distribution.rs — 이미 구현됨
pub struct SummaryStats { pub mean: f64, pub std_dev: f64, pub min: f64, pub p1: f64, pub p5: f64, pub p95: f64, pub max: f64 }
pub struct WorstFrames { /* frame_index, score를 함께 보관하는 최악 N개 추적 */ }

// src-tauri/src/commands/quality.rs — 위 타입을 쓰지 않고 평균 하나만 반환
pub struct BatchQualityMetrics {
    pub average_psnr: f64,
    pub average_ssim: f64,
    pub average_vmaf: f64,
    // SummaryStats/WorstFrames 필드 없음
}
```

**문제**:
- `SummaryStats`(분산, percentile, 표본 수)와 `WorstFrames`(최악 프레임의 frame_index/timestamp까지 보관)는 이미
  올바르게 설계·구현되어 있는데, 사용자가 실제로 받는 `BatchQualityMetrics`는 `average_*` 필드 3개뿐이다.
- 평균만으로는 "1000프레임 중 950프레임 평균 92점"과 "1000프레임 중 1000프레임 평균 92점"을 구분할 수 없고,
  최악 5초 구간이 어디인지도 알 수 없다 — 그런데 그 정보를 만들어낼 코드는 이미 존재한다.
- 두 죽은 타입이 서로 다른 두 문제(분포 요약, 최악 프레임 추적)를 각각 정확히 해결하도록 설계돼 있다는 점은,
  이 기능이 "생각을 안 해봤다"가 아니라 "설계·구현까지 끝냈는데 마지막 연결 단계만 빠졌다"는 것을 보여준다.

**발생 조건**:
- 리포트/DTO 타입을 두 단계(도메인 계산 계층, IPC 응답 계층)로 나눠 설계했을 때, 도메인 계층에서 더 풍부한 타입을
  만들고도 IPC 응답 계층 DTO를 갱신하는 후속 작업이 빠질 때.

**권장**:
```rust
pub struct BatchQualityMetrics {
    pub psnr_stats: SummaryStats,
    pub ssim_stats: SummaryStats,
    pub worst_frames: WorstFrames,
}

fn calculate_quality_metrics(/* .. */) -> BatchQualityMetrics {
    let psnr_scores = /* .. */;
    BatchQualityMetrics {
        psnr_stats: SummaryStats::from_scores(&psnr_scores),
        worst_frames: WorstFrames::new(&psnr_scores, 10),
        ..Default::default()
    }
}
```
- 도메인 계층에 더 풍부한 타입을 추가하는 PR은 IPC 응답 DTO와 프론트엔드 소비 컴포넌트까지의 전체 배선을 완료
  조건에 포함시킨다.
- "이 도메인 타입이 정의는 됐는데 Tauri 커맨드 반환 경로 어디에서도 쓰이지 않는다"를 찾는 정적 검사를 CI에 둔다.

**탐지 방법**:
- Structural: 리포트/DTO 타입에 `f64` 단일 필드만 있고, 같은 크레이트 안에 더 풍부한 분포 타입이 별도로 존재하는지
  대조.
- Semantic: "이 평균값으로 pass/fail을 가른다면 최악의 프레임이 몇 점인지 알 수 있는가?"를 리뷰 질문으로 사용.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `STAT-001`(`docs/anti-patterns/STAT.md`, "평균만 제공")이 Confirmed: `BatchQualityMetrics`
(`quality.rs:31-37`)에는 `average_psnr`/`average_ssim`/`average_vmaf`만 있고, 분포를 포함한 `SummaryStats`
(`crates/bitvue-core/src/metrics_distribution.rs:118-176`)는 존재하지만 어떤 Tauri 커맨드에서도 호출되지 않는 죽은
코드다. `STAT-002`(같은 파일, "최악의 1% 구간을 숨김")도 Confirmed: `WorstFrames`(`metrics_distribution.rs:279-320`)가
구현돼 있지만 어떤 커맨드에서도 호출되지 않아(`MetricsDistributionPanel`/`WorstFrames` 외부 참조 0건) 최악 구간이
사용자에게 전달되지 않는다. 다만 `STAT-003`(같은 파일)이 Suspected로 경고하듯, 이 타입을 나중에 실제로 배선할 때는
`WorstFrames::new`가 "오름차순 정렬 = worst"를 하드코딩하고 있어(`MetricPolarity` 같은 극성 타입 부재) 배선 자체
에서 새로운 버그를 만들지 않도록 주의가 필요하다.

---

### WIRE-005: 구조화된 에러 타입이 정의되어 있지만 모든 Tauri 커맨드가 `Result<T, String>`으로 통일됨
**분류**: WIRING · **심각도**: High · **탐지**: Structural

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// src-tauri/src/error.rs — "raw String error message 대신 쓰기 위해" 이미 작성된 구조화 타입
#[derive(Debug, thiserror::Error, Serialize)]
pub enum BitvueError {
    PathValidation(String), FileIo(String), Parse(String),
    Decode(String), InvalidParameter(String), ResourceExhausted(String),
    RateLimited, Unsupported(String),
}
impl BitvueError { pub fn is_retryable(&self) -> bool { matches!(self, Self::RateLimited) } }

// src-tauri/src/lib.rs — 재export 한 곳 외에는 아무도 쓰지 않음
pub use error::BitvueError;

// src-tauri/src/commands/*.rs — 161개 커맨드 전부 이 패턴
#[tauri::command]
async fn get_stream_info(path: String) -> Result<StreamInfo, String> {
    parse(&path).map_err(|e| e.to_string()) // BitvueError를 거치지 않고 바로 문자열화
}
```
```typescript
// frontend/services/tauriCommandService.ts — 구조화 타입이 없으니 문자열 패턴 매칭으로 재시도 여부 판단
function isNonRetriableError(message: string): boolean {
    return message.toLowerCase().includes("validation") || message.toLowerCase().includes("unauthorized");
}
```

**문제**:
- `BitvueError`는 "String 에러 메시지 대신 쓰기 위해"라는 목적 그대로 설계·구현되어 있는데, 정작 그 목적을 실현할
  대상인 161개 `#[tauri::command]` 시그니처 전부가 여전히 `Result<T, String>`을 쓴다 — 구조화 타입이 실제로
  가로채는 에러가 0건이다.
- 그 결과 프런트엔드는 에러 종류(파일 손상 vs 미구현 vs 내부 버그vs 권한 문제)를 구분할 방법이 없어, 카탈로그
  전반이 경고하는 "문자열 부분 일치로 에러를 분류"하는 패턴(`isNonRetriableError`)을 그대로 구현하게 됐다 — 이는
  구조화 타입이 있었다면 애초에 필요 없었을 우회다.
- `BitvueError::is_retryable()`처럼 구조화 타입 위에 이미 만들어진 유용한 메서드(재시도 가능 여부 판단)조차 실제
  재시도 로직이 없어 호출되지 않는다 — 배선 누락이 한 겹이 아니라 여러 겹으로 겹쳐 있다.

**발생 조건**:
- 에러 처리 개선을 "타입을 먼저 설계"하는 방식으로 시작했지만, 기존 커맨드 161개를 새 타입으로 마이그레이션하는
  작업의 규모가 커서 후속 PR로 미뤄진 채 방치될 때.

**권장**:
```rust
#[tauri::command]
async fn get_stream_info(path: String) -> Result<StreamInfo, BitvueError> {
    parse(&path).map_err(BitvueError::from) // 구조화 타입을 실제로 반환
}
```
```typescript
function isNonRetriableError(err: BitvueErrorDto): boolean {
    return err.kind !== "RateLimited"; // 문자열 매칭이 아니라 태그 기반 분기
}
```
- 마이그레이션을 한 번에 하지 못하더라도, 새로 작성되는 커맨드부터 `BitvueError`를 강제하는 린트/코드 리뷰
  규칙을 두고, 기존 161개는 "부채"로 명시적으로 추적한다(전부 한 번에 바꾸는 대신 점진적 마이그레이션 체크리스트).
- 프런트엔드 에러 분류 로직(`isNonRetriableError` 등)이 문자열 패턴 매칭을 하고 있다는 사실 자체를, 백엔드 구조화
  타입이 배선되지 않았다는 신호로 코드 리뷰에서 인지한다.

**탐지 방법**:
- Structural: `#[tauri::command]` 시그니처 중 `Result<_, String>` 비율과, `BitvueError`를 실제로 반환하는 커맨드
  수를 대조.
- Structural: 프런트엔드에서 에러 문자열을 `includes`/정규식으로 분기하는 코드가 있는지 검색 — 있다면 백엔드
  구조화 타입 미배선의 강한 신호.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `ERR-003`(`docs/anti-patterns/ERR.md`, "anyhow::Error로 전 계층 평탄화")이 Confirmed:
`src-tauri/src/commands/` 전체에서 `Result<.., String>` 시그니처가 161곳 쓰이고, "raw String error messages 대신
쓰기 위해" 이미 작성된 `BitvueError`(`src-tauri/src/error.rs`)는 `src-tauri/src/lib.rs:8`의 re-export 1곳 외에는
어디서도 참조되지 않는 죽은 코드다. 프런트엔드 `tauriCommandService.ts:158-172`의 `isNonRetriableError`는 정확히
문자열 부분 일치(`"validation"|"invalid"|"unauthorized"` 등)로 재시도 여부를 판단한다. `ERR-018`(같은 파일, "오류
타입이 손상된 파일 vs 미구현 기능 vs Bitvue 버그를 구분하지 못함")도 동일 근거로 Confirmed: `BitvueError::{Parse,
Decode, Unsupported}`가 실제로 존재하는데도 Tauri 커맨드 계층 전체가 이를 쓰지 않는다. `IPC-018`
(`docs/anti-patterns/IPC.md`, "오류를 단일 문자열로 변환")도 88건의 `Result<_, String>` 시그니처(예: `file.rs:407`)를
독립적으로 확인했다. `UIX-ASYNC`의 두 항목(444, 479행)은 `is_retryable()`/`Timeout` variant 부재도 함께 확인했다.

---

### WIRE-006: 페이지네이션 커맨드가 구현되어 있지만 실제 프런트엔드 호출부 5곳이 여전히 구버전 전체-반환 커맨드를 씀
**분류**: WIRING · **심각도**: Medium · **탐지**: Structural

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// src-tauri/src/commands/file.rs:498-540 — offset/limit을 받는 페이지네이션 버전, 이미 구현됨
#[tauri::command]
fn get_frames_chunk(offset: usize, limit: usize) -> Vec<FrameData> { /* ... */ }

// src-tauri/src/commands/file.rs:453-490 — 전체를 한 번에 반환하는 구버전, 여전히 살아있음
#[tauri::command]
fn get_frames() -> Vec<FrameData> { /* offset/limit 없음 */ }
```
```typescript
// frontend/contexts/LegacyStreamDataContext.tsx:57, utils/progressiveLoader.ts:128,
// utils/exportData.ts:286, ReferenceGraphPanel.tsx:57, BitrateGraphPanel.tsx:45
const frames = await invoke("get_frames", {}); // 5곳 모두 신버전이 아니라 구버전을 호출
```

**문제**:
- `get_frames_chunk`는 정확히 이 문제(전체 프레임을 한 번에 IPC로 보내는 비용)를 해결하기 위해 이미 구현돼 있는데,
  실제 5개 프런트엔드 호출부는 여전히 페이지네이션이 없는 `get_frames`를 부른다 — 해결책이 반쪽만 배선됐다.
- 신버전 커맨드를 아는 개발자가 새 기능(progressive loading 등)을 만들 때는 신버전을 쓰지만, 기존 5개 호출부를
  마이그레이션하는 청소 작업은 별도 후속 작업으로 남아 무기한 미뤄지기 쉬운 전형적인 패턴이다.
- 두 커맨드가 동시에 살아있으면 유지보수 비용도 2배가 된다 — 백엔드 로직 변경 시 두 경로 모두 갱신해야 하고,
  하나만 갱신하면 두 경로의 동작이 미묘하게 갈라지는 회귀가 생기기 쉽다.

**발생 조건**:
- 성능 개선을 위해 새 API(페이지네이션, 커서 기반 등)를 추가했지만, 기존 호출부 전체를 새 API로 옮기는 마이그레이션
  작업이 "나중에" 로 미뤄질 때, 특히 호출부가 여러 파일에 흩어져 있을 때.

**권장**:
```typescript
// LegacyStreamDataContext.tsx 등 5개 호출부를 모두 아래로 교체
const { frames, totalCount } = await invoke("get_frames_chunk", { offset, limit: PAGE_SIZE });
```
- 신버전 커맨드를 추가하는 PR에서 구버전 커맨드를 `#[deprecated]`로 표시하고, 컴파일 경고를 마이그레이션 진행률의
  강제 신호로 활용한다.
- 구버전 커맨드의 모든 호출부를 마이그레이션 완료할 때까지 이슈/체크리스트로 명시적으로 추적한다 — "신버전을
  만들었다"와 "마이그레이션을 끝냈다"를 별개의 완료 조건으로 취급한다.

**탐지 방법**:
- Structural: 페이지네이션 파라미터(offset/limit/cursor)가 있는 커맨드와 없는 "같은 데이터를 반환하는" 커맨드
  쌍이 공존하는지 grep, 그리고 각각의 프런트엔드 호출부 수를 비교.
- Runtime: 대형 스트림 로드 시 실제로 어느 커맨드가 호출되는지 IPC 트래픽을 관찰.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `IPC-007`(`docs/anti-patterns/IPC.md`, "pagination 부재")이 Confirmed: `get_frames_chunk`
(`src-tauri/src/commands/file.rs:498-540`, offset/limit)로 페이지네이션을 구현해뒀음에도, 페이지네이션이 없는
`get_frames`(`file.rs:453-490`, 전체 `Vec<FrameData>` 반환)가 여전히 존재하며
`frontend/contexts/LegacyStreamDataContext.tsx:57`, `frontend/utils/progressiveLoader.ts:128`,
`frontend/utils/exportData.ts:286`, `ReferenceGraphPanel.tsx:57`, `BitrateGraphPanel.tsx:45`에서 그대로 호출된다.

---

### WIRE-007: 캐시 출처 추적/프레임 매핑 로직이 존재하지만 src-tauri에서 전혀 import되지 않음
**분류**: WIRING · **심각도**: Medium · **탐지**: Structural

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// crates/bitvue-core/src/cache_provenance.rs — 옵션별 CacheKey variant까지 설계된 캐시 추적 로직
pub struct CacheProvenanceTracker { /* .. */ }
impl CacheProvenanceTracker { pub fn age(&self) -> Duration { /* .. */ } }

// crates/bitvue-core/src/player/frame_mapper.rs — display_idx(PRIMARY) vs decode_idx(internal-only) 계약
pub struct FrameMapEntry { pub display_idx: u32, pub decode_idx: u32 }
pub fn display_to_decode(map: &[FrameMapEntry], display_idx: u32) -> Option<u32> { /* .. */ }

// src-tauri/src/commands/frame.rs — 위 두 모듈을 import하지 않고 컨테이너 샘플 순서를 그대로 frame_index로 사용
fn decode_ivf_frame_generic(container_sample_order: u32) -> DecodedFrame {
    decode_at(container_sample_order) // display/decode order 구분 없이 그대로 사용
}
```

**문제**:
- 두 모듈 모두 정확한 도메인 문제(캐시 키에 영향을 주는 옵션 추적, B-frame이 있는 스트림에서 display order와
  decode order의 구분)를 겨냥해 설계되어 있는데, `src-tauri` 어디에서도 이들을 import하지 않는다(grep 0건).
- `frame_mapper.rs`가 다루는 display/decode order 구분은 B-frame이 포함된 실제 스트림에서 조용한 오정합(잘못된
  프레임이 캐시되거나 표시됨)으로 이어질 수 있는 문제인데, 이를 막을 코드가 이미 있음에도 실제 디코드 경로
  (`decode_ivf_frame_generic` 등)는 이를 거치지 않는다.
- 두 모듈이 각각 다른 크레이트/모듈 경로에 위치해 있다는 것은, "이 문제를 해결하는 코드가 이미 있다"는 사실을
  찾기가 grep 없이는 쉽지 않다는 뜻이기도 하다 — 배선 누락이 발견되기 어려운 이유 중 하나가 발견 가능성
  (discoverability) 자체의 부재다.

**발생 조건**:
- 코어 크레이트(`bitvue-core`)에서 도메인 로직을 먼저 설계·구현하고, 그 소비자가 될 애플리케이션 크레이트
  (`src-tauri`)의 실제 커맨드 구현이 별도 시점에 별도 방식으로 작성되어 서로 연결되지 않을 때.

**권장**:
```rust
use bitvue_core::player::frame_mapper::display_to_decode;

fn decode_ivf_frame_generic(frame_map: &[FrameMapEntry], display_idx: u32) -> DecodedFrame {
    let decode_idx = display_to_decode(frame_map, display_idx).expect("frame map must be complete");
    decode_at(decode_idx)
}
```
- 코어 크레이트에 새 도메인 타입/모듈을 추가할 때, 그 모듈을 실제로 소비해야 할 애플리케이션 크레이트 쪽 이슈를
  같은 PR 또는 연결된 후속 PR로 함께 연다.
- `cargo udeps` 또는 워크스페이스 전체의 "정의된 pub 모듈 vs 실제 import" 대조 스크립트를 CI에 추가해, 코어
  크레이트의 모듈이 애플리케이션 크레이트에서 전혀 참조되지 않는 경우를 자동 감지한다.

**탐지 방법**:
- Structural: `crates/bitvue-core/src`의 각 pub 모듈에 대해 `src-tauri/src` 전체에서 import 여부를 grep.
- Runtime: B-frame이 포함된 실제 스트림으로 재생/캐시 동작을 확인해 display/decode order 오정합이 실제로
  발생하는지 검증(이 항목은 그 검증이 아직 수행되지 않아 Suspected로 남은 부분도 있다).

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `CACHE-003`(`docs/anti-patterns/CACHE.md`, "cache key에 analysis option 누락")이
Confirmed 텍스트 안에서 명시: "`bitvue-core`의 `cache_provenance.rs`(옵션별 `CacheKey` variant 설계)는 이 문제를
인지하고 설계됐지만 src-tauri에서 미사용." `CACHE-008`(같은 파일, "frame index와 display index 혼용")은 Suspected로
기록: `crates/bitvue-core/src/player/frame_mapper.rs:1-40`에 `FRAME_IDENTITY_CONTRACT`(display_idx PRIMARY vs
decode_idx internal-only)를 구현한 모듈이 존재하지만 `src-tauri/src` 어디에서도 `player::frame_mapper`/
`FrameMapEntry`를 import하지 않는다(grep 0건) — 실제 프로덕션 디코드 경로(`frame.rs`의 `decode_ivf_frame_generic`
등)는 이 매핑을 거치지 않고 컨테이너 샘플 순서를 그대로 `frame_index`로 쓴다. B-frame이 포함된 실제 스트림으로
화면 확인을 하지 않아 실제 오정합 여부는 미검증이라 Suspected로 남아있다.

---

### WIRE-008: 진단/빈 상태 컴포넌트가 존재하지만 실제 패널 트리에 마운트되지 않음
**분류**: WIRING · **심각도**: High · **탐지**: Visual|Structural

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```tsx
// frontend/components/panels/DiagnosticsPanel.tsx — 심각도별 아이콘/색상까지 이미 구현됨
export function DiagnosticsPanel({ diagnostics }: Props) { /* severity-aware rendering */ }

// frontend/components/common/EmptyState.tsx — 로딩/빈 데이터/에러를 구분하는 사전 구성 변형까지 있음
export function EmptyState({ variant }: { variant: "loading" | "empty" | "error" }) { /* .. */ }

// App.tsx / DockableLayout.tsx — 두 컴포넌트 모두 패널 목록에 등록되어 있지 않음
function App() {
  return <DockableLayout panels={[SyntaxPanel, HexPanel, PlayerPanel /* DiagnosticsPanel 없음 */]} />;
}
// 다른 패널들도 로딩/빈 상태를 자체 스피너/빈 div로 처리 — EmptyState import 0건
```

**문제**:
- `DiagnosticsPanel`은 심각도별 시각 구분까지 이미 만들어져 있는데, 실행 중인 앱의 `StatusBar`는 파일 경로/프레임
  수만 보여줄 뿐 심각도 구분 표시 자체가 없다 — 사용자는 fatal 에러와 warning을 시각적으로 구분할 수 없다.
- `EmptyState`는 로딩/빈 데이터/에러를 구분하는 사전 구성 변형까지 포함해 정확히 이 문제를 풀기 위해 만들어졌는데,
  자기 자신의 정의/테스트 파일 외 어디에서도 import되지 않아, 실제 패널들은 빈 화면을 구분 없이 그대로 노출한다.
- 두 컴포넌트 모두 "만들어졌다"와 "화면에 나타난다"가 분리되어 있다는 점에서, 컴포넌트 구현 완료를 곧 기능 완료로
  착각하기 쉬운 프런트엔드 버전의 배선 누락이다.

**발생 조건**:
- 재사용 가능한 UI 컴포넌트를 먼저 만들고, 그 컴포넌트를 실제 화면(레이아웃 트리, 각 패널의 상태 분기)에 끼워
  넣는 후속 작업이 별도 PR/작업으로 미뤄질 때.

**권장**:
```tsx
function App() {
  return (
    <DockableLayout panels={[SyntaxPanel, HexPanel, PlayerPanel, DiagnosticsPanel]} />
  );
}

function SomePanel({ data, isLoading, error }: Props) {
  if (isLoading) return <EmptyState variant="loading" />;
  if (error) return <EmptyState variant="error" />;
  if (!data) return <EmptyState variant="empty" />;
  return <PanelContent data={data} />;
}
```
- 재사용 컴포넌트를 만드는 PR은 "최소 1곳 이상의 실제 화면에 마운트했는가"를 완료 조건에 포함시킨다.
- Storybook 등에만 등장하고 실제 앱 트리에는 import되지 않는 컴포넌트를 찾는 정적 검사(빌드 시 unused export
  경고 강화)를 CI에 둔다.

**탐지 방법**:
- Structural: 컴포넌트 정의 파일 외에 `import.*ComponentName`이 실제 앱 트리(`App.tsx`, 레이아웃 컴포넌트 등)에
  존재하는지 grep.
- Visual: 다양한 코덱/파일 조합으로 열어보며 빈 패널·에러 상태가 실제로 시각적으로 구분되는지 확인.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `UIX-IA-006`(`docs/anti-patterns/UIX_IA.md`, "중요한 상태와 부가 정보를 같은 시각적
무게로 표시")이 Confirmed: 실제로 마운트되는 `StatusBar`(`frontend/components/StatusBar.tsx`)는 파일 경로/프레임
수만 보여주고 심각도 스타일이 전혀 없으며, 심각도별 아이콘·색상을 실제로 구현한 `DiagnosticsPanel`
(`frontend/components/panels/DiagnosticsPanel.tsx:14,129-146`)은 `App.tsx`/`DockableLayout`의 어떤 패널 목록에도
포함되지 않아 실행 중인 앱에는 심각도 구분 표시 자체가 없다. `UIX-IA-014`(같은 파일, "빈 패널을 계속 노출")도
Confirmed: 로딩/빈 데이터/에러를 구분하도록 만들어진 재사용 `EmptyState` 컴포넌트(`frontend/components/common/
EmptyState.tsx`)가 존재하지만, 자기 자신의 정의/테스트 파일 외 어디에서도 import되지 않는다(`import.*EmptyState`
검색 결과 0건). `UIX-ERR-009`(`docs/anti-patterns/UIX_ERR.md`, "오류 메시지에서 관련 위치로 이동 불가")도 관련:
`DiagnosticsPanel.tsx`의 진단 행 클릭은 로컬 상세 패널만 열 뿐 `setCurrentFrameIndex` 등 내비게이션 호출이 없다.

---

### WIRE-009: Compare 워크스페이스 백엔드/컨텍스트 상태가 실제 화면에 마운트되지 않음
**분류**: WIRING · **심각도**: Critical · **탐지**: Visual|Structural

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```tsx
// frontend/contexts/CompareContext.tsx — pathA/pathB/currentFrameA/currentFrameB까지 추적하는
// "dependent bitstream" 비교 워크스페이스 상태가 이미 구현됨
export function CompareProvider({ children }: Props) {
  const [state, setState] = useState<CompareState>({ pathA: null, pathB: null, /* .. */ });
  return <CompareContext.Provider value={state}>{children}</CompareContext.Provider>;
}
export function useCompare() { return useContext(CompareContext); }

// frontend/hooks/useAppFileOperations.ts:175-234 — 비교 워크스페이스를 실제로 생성함
function handleOpenDependentFile(path: string) {
  compareContext.setPathB(path); // 상태는 정상적으로 갱신됨
}

// App.tsx — useCompare()를 호출하는 마운트된 컴포넌트가 어디에도 없음
function App() {
  return <DockableLayout panels={[SyntaxPanel, HexPanel, PlayerPanel /* CompareWorkspace 없음 */]} />;
}
// frontend/components/CompareWorkspace/* 전체가 고아 코드 — import하는 곳이 없음
```

**문제**:
- `CompareContext`는 좌/우 파일·프레임을 추적하는 실제 상태를 갖고 있고, `handleOpenDependentFile`이 그 상태를
  정상적으로 채우기까지 하는데, 그 상태를 렌더링해야 할 `CompareWorkspace/*` 컴포넌트 트리 전체를 아무도 화면에
  마운트하지 않는다 — 사용자가 "두 번째 파일 열기"를 실행해도 화면에는 좌/우 비교가 전혀 나타나지 않는다.
- 이는 "헷갈린다" 수준의 UX 문제가 아니라 기능이 데이터 레벨에서는 동작하고 화면 레벨에서는 존재하지 않는,
  가장 심각한 형태의 배선 누락이다 — 데이터가 만들어지는데 그걸 보여줄 뷰가 트리에 없다.
- 화질 비교가 Bitvue의 핵심 차별화 기능(VQ-Probe 도메인) 중 하나라는 점에서, 이 배선 누락은 단순 버그가 아니라
  기능 전체가 "구현됐지만 도달 불가능"한 상태임을 의미한다.

**발생 조건**:
- Context/상태 관리 계층과 그 상태를 그리는 뷰 계층을 서로 다른 시점에 작업했고, 뷰 계층을 실제 앱 레이아웃
  트리에 등록하는 마지막 단계가 별도 PR로 누락되었을 때.

**권장**:
```tsx
function App() {
  const { pathB } = useCompare();
  return (
    <DockableLayout
      panels={pathB ? [SyntaxPanel, CompareWorkspace, PlayerPanel] : [SyntaxPanel, HexPanel, PlayerPanel]}
    />
  );
}
```
- Context를 추가하는 PR은 "이 Context를 구독하는 컴포넌트가 실제 앱 트리에 최소 1곳 마운트되어 있는가"를 완료
  조건에 포함시킨다.
- `useCompare()`처럼 상태를 만드는 훅과 그 훅을 소비하는 컴포넌트가 실제로 렌더링 트리에 연결되는지 확인하는
  통합 테스트(예: 두 번째 파일 열기 → 비교 패널이 실제로 DOM에 나타나는지)를 추가한다.

**탐지 방법**:
- Structural: `useXxx()` 커스텀 훅 정의부와 그 훅을 호출하는 컴포넌트 목록을 대조해, 훅은 있는데 소비자가
  `App.tsx`/레이아웃 트리에 마운트되지 않은 경우를 찾는다.
- Interaction: 두 번째(비교 대상) 파일을 실제로 열어 화면에 좌/우 비교 UI가 나타나는지 수동 확인.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed (가장 심각) — `UIX-IA-013`(`docs/anti-patterns/UIX_IA.md`, "파일·트랙·프레임 선택 계층이
표현되지 않음")이 Confirmed: `FileStateContext`는 `filePath: string | null` 단일 파일만 모델링하며
(`frontend/contexts/FileStateContext.tsx:34,56`), `CompareContext`는 pathA/pathB/currentFrameA/currentFrameB를
추적하는 비교 워크스페이스 상태를 갖고 있지만(`frontend/contexts/CompareContext.tsx`), `useCompare()`를 호출하는
마운트된 컴포넌트가 `App.tsx`를 포함해 어디에도 없고 `frontend/components/CompareWorkspace/*` 전체가 고아 코드임이
확인됐다. `handleOpenDependentFile`(`frontend/hooks/useAppFileOperations.ts:175-234`)이 비교 워크스페이스를
생성해도 실제 화면에는 좌/우 파일 표시가 전혀 나타나지 않는다 — "헷갈린다" 수준이 아니라 표시 자체가 없다.
`UX_SCENARIO.md`(242행)도 관련 확인: dual-stream compare 기능 자체는 실재하나(`compare.rs` 419줄, `set_sync_mode`
등) `CompareWorkspace`/`StreamPlayer` 내부의 메모리/오버레이 교차오염 여부는 별도 조사가 필요해 Suspected로 남는다.

---

### WIRE-010: LayoutContext에 영속 저장 로직이 있지만 실제 스플리터 이벤트와 연결되지 않음
**분류**: WIRING · **심각도**: Medium · **탐지**: Interaction

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```tsx
// frontend/contexts/LayoutContext.tsx — debounce 저장/복원 로직이 이미 구현됨
export function LayoutProvider({ children }: Props) {
  const [layoutState, setLayoutState] = useState(DEFAULT_LAYOUT);
  const updateLeftPanel = useCallback((size: number) => {
    setLayoutState((s) => ({ ...s, left: size }));
    debouncedSaveToLocalStorage(layoutState); // 저장 인프라는 존재
  }, [layoutState]);
  // updateLeftPanel/updateTopPanel/updateBottomPanel: 정의는 있으나 호출하는 곳이 없음
  return <LayoutContext.Provider value={{ layoutState }}>{children}</LayoutContext.Provider>;
}

// DockableLayout.tsx — 실제 스플리터는 react-resizable-panels를 그대로 사용
function DockableLayout() {
  return (
    <PanelGroup> {/* autoSaveId 없음, onResize가 LayoutContext에 연결되지 않음 */}
      <Panel>{/* .. */}</Panel>
      <PanelResizeHandle />
      <Panel>{/* .. */}</Panel>
    </PanelGroup>
  );
}
```

**문제**:
- `LayoutContext`는 debounce 저장/복원 로직까지 갖추고 있어 겉보기엔 "구현된 기능"처럼 보이지만,
  `updateLeftPanel`/`updateTopPanel`/`updateBottomPanel`은 정의부 외에 호출되는 곳이 없다.
- 실제 스플리터 컴포넌트(`react-resizable-panels`의 `Panel`/`PanelGroup`)는 `autoSaveId`도 없고 `onResize` 콜백이
  `LayoutContext`에 연결되지도 않아, 사용자가 드래그한 크기는 `layoutState`에 절대 반영되지 않고 앱 재시작 시
  유실된다.
- "저장 인프라만 있고 배선이 끊긴 죽은 코드"라는 정확한 표현이 이미 기존 감사에서 나올 정도로, 이 사례는 이
  카탈로그가 다루는 메타패턴 자체를 원본 파일이 먼저 정확히 짚어낸 경우다.

**발생 조건**:
- 상태 관리 로직(Context, 저장/복원)을 먼저 만들고, 그 상태를 실제로 갱신해야 할 UI 이벤트 핸들러(드래그 종료,
  resize 콜백)를 나중에 별도 라이브러리로 구현하면서 둘을 연결하는 단계를 빠뜨렸을 때.

**권장**:
```tsx
function DockableLayout() {
  const { layoutState, updateLeftPanel } = useLayout();
  return (
    <PanelGroup autoSaveId="bitvue-main-layout" onLayout={(sizes) => updateLeftPanel(sizes[0])}>
      <Panel defaultSize={layoutState.left}>{/* .. */}</Panel>
      <PanelResizeHandle />
      <Panel>{/* .. */}</Panel>
    </PanelGroup>
  );
}
```
- Context에 상태 갱신 함수(`updateLeftPanel` 등)를 추가하는 PR은 "이 함수를 실제로 호출하는 이벤트 핸들러가
  존재하는가"를 완료 조건에 포함시킨다.
- 정의는 됐지만 호출되지 않는 함수를 찾는 정적 검사(unused export 경고, 커스텀 grep)를 CI에 둔다.

**탐지 방법**:
- Structural: Context가 노출하는 갱신 함수(`updateXxx`)의 정의부와 실제 호출부를 전체 grep으로 대조.
- Interaction: 스플리터를 드래그해 위치를 바꾼 뒤 앱을 재시작해 위치가 유지되는지 회귀 테스트.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `UIX-LAYOUT-003`(`docs/anti-patterns/UIX_LAYOUT.md`, "splitter 위치를 저장하지 않음")이
Confirmed: `LayoutContext`(`frontend/contexts/LayoutContext.tsx`)에 debounce 저장/복원 로직이 있어 겉보기엔
구현된 듯 보이지만, `updateLeftPanel`/`updateTopPanel`/`updateBottomPanel`은 정의만 되어 있을 뿐 앱 어디에서도
호출되지 않는다(전체 grep 결과 정의부 외 호출 0건). 실제 스플리터(`DockableLayout.tsx`의 `Group`/`Panel`/
`Separator`, `react-resizable-panels`)는 `autoSaveId`도 없고 `onResize` 콜백도 `LayoutContext`에 연결되어 있지
않아, 사용자가 드래그한 크기는 `layoutState`에 절대 반영되지 않고 재시작 시 유실된다 — 원문 그대로 "저장
인프라만 있고 배선이 끊긴 죽은 코드".

---

### WIRE-011: IPC 지연시간 계측이 존재하지만 console.debug에 그쳐 CI 회귀 게이트로 이어지지 않음
**분류**: WIRING · **심각도**: Low · **탐지**: Structural

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```typescript
// frontend/services/tauriCommandService.ts:78-99 — performance.now() 기반 계측이 이미 구현됨
async function invokeWithTiming<T>(cmd: string, args: unknown): Promise<T> {
  const start = performance.now();
  const result = await invoke<T>(cmd, args);
  const elapsed = performance.now() - start;
  console.debug(`[invoke] ${cmd} took ${elapsed.toFixed(1)}ms`); // 콘솔에만 남고 끝
  return result;
}
// getLatencyStats()도 avg/min/max만 계산 — p95/p99 없음, CI로 export되지 않음
```

**문제**:
- `invokeWithTiming`은 모든 IPC 호출의 지연시간을 이미 정확히 계측하고 있는데, 그 결과는 `console.debug` 한 줄로
  끝나고 어떤 CI 파이프라인에도, 어떤 회귀 게이트에도 연결되지 않는다.
- 그 결과 "이번 PR이 특정 커맨드의 응답 시간을 2배로 느리게 만들었다"는 회귀가 발생해도, 계측 코드 자체는 이미
  그 정보를 갖고 있었음에도 아무도 자동으로 알아채지 못한다 — 계측이 있다는 사실이 오히려 "이미 감시되고 있다"는
  잘못된 안도감을 줄 수 있다.
- `getLatencyStats()`가 avg/min/max만 계산하고 p95/p99를 계산하지 않는 것도 같은 배선 문제의 연장선이다 — 이미
  샘플을 모으고 있는데 그 샘플로 만들 수 있는 분위수를 만들지 않는다.

**발생 조건**:
- 계측/로깅 코드를 개발 편의(디버깅)를 위해 먼저 추가했지만, 그 계측 결과를 CI 회귀 게이트나 성능 대시보드로
  승격시키는 후속 작업이 "나중에"로 남아있을 때.

**권장**:
```typescript
async function invokeWithTiming<T>(cmd: string, args: unknown): Promise<T> {
  const start = performance.now();
  const result = await invoke<T>(cmd, args);
  const elapsed = performance.now() - start;
  latencyRecorder.record(cmd, elapsed); // 회귀 게이트가 읽을 수 있는 곳에 축적
  return result;
}
// CI: latencyRecorder의 p95가 기준선 대비 20% 이상 느려지면 실패
```
- 디버그 로그로 시작한 계측이라도, "이 숫자가 실제로 어딘가에서 소비되고 있는가"를 주기적으로 재검토한다.
- 회귀 게이트로 승격시키는 작업을 명시적 후속 이슈로 만들어 추적하고, "계측 코드 존재"와 "회귀 게이트 존재"를
  같은 것으로 착각하지 않는다.

**탐지 방법**:
- Structural: 성능 계측 코드(`performance.now()`, `Instant::now()` 등)의 결과가 로그 출력 외에 CI 스크립트나
  대시보드로 이어지는지 추적.
- Manual: "성능 회귀를 자동으로 잡을 수 있는가?"를 계측 코드 존재 여부와 별개로 확인.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `PERF-009`(`docs/anti-patterns/PERF.md`, "UI latency 미측정")가 Confirmed: Playwright
등 E2E 프레임워크가 저장소에 전혀 없고 CI에도 UI latency 게이트가 없으며, `frontend/services/
tauriCommandService.ts:78-99`의 `performance.now()` 기반 invoke latency 로깅은 존재하지만 콘솔 디버그 로그일 뿐
CI에 통합되거나 회귀 게이트로 쓰이지 않는다. `PERF-010`(같은 파일, "p50만 보고 p95/p99 무시")도 Confirmed:
`crates/bitvue-core/src/performance.rs`의 `MetricSummary`(avg/min/max만)와 `tauriCommandService.ts`의
`getLatencyStats()`(L207-221, avg/min/max만) 둘 다 분위수를 전혀 계산하지 않는다.

---

### WIRE-012: 취소 토큰 인프라가 존재하지만 어떤 장시간 Tauri 커맨드에도 연결되지 않음
**분류**: WIRING · **심각도**: High · **탐지**: Structural|Runtime

**나쁜 예** (실제 프로덕션 코드의 모양을 일반화한 스케치입니다 — 가상 예시가 아닙니다):
```rust
// crates/bitvue-core/src/index_session.rs:77 — 취소 인프라가 이미 구현됨
pub struct IndexSession {
    pub should_cancel: std::sync::atomic::AtomicBool,
    // 스트리밍/취소 가능한 인덱싱을 염두에 두고 설계됨
}
impl IndexSession {
    pub fn cancel(&self) { self.should_cancel.store(true, Ordering::SeqCst); }
    pub fn is_cancelled(&self) -> bool { self.should_cancel.load(Ordering::SeqCst) }
}

// src-tauri/src/commands/quality.rs — IndexSession을 import하지 않고 취소 확인 없이 최대 10,000프레임 루프
#[tauri::command]
async fn calculate_quality_metrics(/* .. */) -> Result<BatchQualityMetrics, String> {
    for idx in 0..frame_count.min(10_000) { // 취소 확인 없음
        scores.push(calculate_single_frame_metrics(idx));
    }
    Ok(BatchQualityMetrics::from_scores(scores))
}
```

**문제**:
- `IndexSession::should_cancel`은 정확히 "장시간 작업을 중간에 멈출 수 있게"라는 목적으로 설계된 `AtomicBool`
  취소 플래그인데, 최대 10,000프레임까지 루프를 도는 `calculate_quality_metrics`는 이를 참조하지 않는다.
- 사용자가 "중단" UI를 눌러도(만약 그런 UI가 있다면) 실제로 멈출 방법이 백엔드에 없다 — 인프라가 있다는 사실이
  "이미 취소가 된다"는 오해를 낳기 쉬운, 이 카탈로그가 다루는 배선 문제의 전형이다.
- 취소 인프라가 인덱싱(`IndexSession`)을 염두에 두고 설계됐지만 실제로 취소가 필요한 다른 장시간 작업(품질 지표
  계산)에도 재사용 가능한 형태인데, 그 재사용 자체가 이뤄지지 않았다 — "이 문제를 처음 겪는 게 아니다"라는
  사실이 드러나는 지점이다.

**발생 조건**:
- 한 기능(인덱싱)을 위해 취소 인프라를 설계했지만, 나중에 추가된 다른 장시간 작업(품질 지표 계산 등)이 그
  인프라를 재사용하지 않고 별도로 구현되거나 아예 취소 로직 없이 구현될 때.

**권장**:
```rust
#[tauri::command]
async fn calculate_quality_metrics(
    session: tauri::State<'_, IndexSession>,
    /* .. */
) -> Result<BatchQualityMetrics, String> {
    for idx in 0..frame_count.min(10_000) {
        if session.is_cancelled() {
            return Err("cancelled".into());
        }
        scores.push(calculate_single_frame_metrics(idx));
    }
    Ok(BatchQualityMetrics::from_scores(scores))
}
```
- 취소 인프라를 설계할 때 "이 타입/플래그를 다른 장시간 작업에서도 재사용할 수 있는가"를 이름/위치 선정 단계에서
  고려한다(예: 인덱싱 전용 이름 대신 범용 `CancellationToken`으로 분리).
- 프레임 수 기반 루프를 도는 모든 async 커맨드 목록을 만들어, 각각이 취소 토큰을 확인하는지 체크리스트로 감사한다.

**탐지 방법**:
- Structural: 여러 반복에 걸쳐 `.await`를 포함하는 `for`/`while` 루프를 가진 async 커맨드 중, 취소 토큰 참조가
  없는 것을 grep.
- Manual: "중단" UI가 있는 기능마다 대응하는 백엔드 취소 로직이 실제로 연결되어 있는지 매핑.

**예외**:
- 다른 Confirmed 클러스터에서 새 수정 코드를 작성하기 전에, 워크스페이스 안에 이미 올바른 구현이 있는지 먼저
  grep하라 — 배선이 새로 짜는 것보다 대개 더 싸다.

**Bitvue 판정**: Confirmed — `CONC-008`(`docs/anti-patterns/CONC.md`, "취소 토큰 없는 장시간 작업")이 Confirmed:
`calculate_quality_metrics`가 최대 10,000프레임 루프를 취소 확인 없이 실행하며(`quality.rs:455-478`),
`bitvue-core`에 취소 토큰 인프라(`should_cancel: AtomicBool`, `index_session.rs:77`)가 있지만 어떤 Tauri 커맨드에도
연결되어 있지 않다.

---

## 이 파일과 다른 파일들의 관계

이 파일의 12개 항목은 모두 다른 Wave 1/2/3의 기존 Confirmed 판정을 인용한 것이며, 새 항목을 원본 파일에 추가하지
않았다(그 파일들의 갱신은 이 작업의 범위 밖). 아래는 이 파일이 인용한 원본 ID의 요약이다:

| WIRE ID | 인용한 원본 ID |
|---|---|
| WIRE-001 | ALIGN-010, STAT-007 |
| WIRE-002 | IO-001, MEM-001 |
| WIRE-003 | PIXEL-018, CONC-019 |
| WIRE-004 | STAT-001, STAT-002, STAT-003 |
| WIRE-005 | ERR-003, ERR-018, IPC-018, UIX-ASYNC(444, 479행) |
| WIRE-006 | IPC-007 |
| WIRE-007 | CACHE-003, CACHE-008 |
| WIRE-008 | UIX-IA-006, UIX-IA-014, UIX-ERR-009 |
| WIRE-009 | UIX-IA-013, UX_SCENARIO(242행) |
| WIRE-010 | UIX-LAYOUT-003 |
| WIRE-011 | PERF-009, PERF-010 |
| WIRE-012 | CONC-008 |
