# Anti-Pattern Catalog — OBS: 관측성·진단·프로파일링

Wave 4의 일부. 전체 카탈로그 구조는 `docs/anti-patterns/INDEX.md` 참조. 이 파일은 "영상을 분석하는 도구 자신도 분석 가능해야 한다"는 프레이밍 아래, 다른 카테고리 전반에 흩어진 계측 요구사항을 한데 묶는다 — 특히 OBS-003/004는 Wave 1 `PERF.md`의 측정 오류들과, OBS-009는 Wave 3 `UX_SCENARIO.md`의 계측 지점들과 직접 연결된다.

---

### OBS-001: 모든 로그가 문자열
**분류**: 구조화 부재 · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
fn decode_frame(session_id: &str, frame_idx: u32, nal_type: u8) -> Result<Frame, DecodeError> {
    log::info!(
        "decoding frame {} in session {} nal_type={} ...",
        frame_idx, session_id, nal_type
    );
    // ...
    log::error!("decode failed for frame {} : {:?}", frame_idx, err);
    Ok(frame)
}
```

**문제**:
- 로그가 사람이 읽기 위한 문장으로만 존재하고, 기계가 질의할 수 있는 필드로 존재하지 않는다.
- "이번 주 nal_type=5인 프레임 중 decode 실패율" 같은 질문에 답하려면 정규식으로 로그를 다시 파싱해야 한다.
- 문자열 포맷이 바뀌면(오타 수정, 메시지 개선) 기존 파싱 스크립트/대시보드가 조용히 깨진다.
- 로그 볼륨이 늘어나면 grep 기반 분석이 선형으로 느려지고, 필드 인덱싱이 불가능하다.

**발생 조건**:
- 사용자가 "특정 파일에서 재생이 멈춘다"고 신고했을 때, 로그에서 해당 파일의 세션만 필터링하려는 순간.
- 여러 코덱(AVC/HEVC/VP9/AV1)의 에러율을 코덱별로 집계하려는 순간.

**권장**:
```rust
use tracing::{info, error};

fn decode_frame(session_id: &str, frame_idx: u32, nal_type: u8) -> Result<Frame, DecodeError> {
    info!(session_id, frame_idx, nal_type, "decoding frame");
    // ...
    if let Err(ref err) = result {
        error!(session_id, frame_idx, error = %err, "decode failed");
    }
    result
}
```
- `tracing`/`slog` 등 구조화 로깅 라이브러리를 사용해 필드를 key-value로 남긴다.
- 로그를 JSON lines로 출력해 `jq`, DuckDB 등으로 바로 질의 가능하게 한다.
- 메시지 문자열은 사람을 위한 요약으로만 두고, 필터링/집계에 필요한 값은 항상 필드로 분리한다.

**탐지 방법**:
- Static: `log::info!("... {} ...", ...)` 형태로 3개 이상 인자를 문자열 보간하는 호출을 정적 검사로 탐지.
- Structural: `tracing`의 `%`/`?` 필드 문법 없이 `format!` 결과를 그대로 로깅하는 패턴 검색.

**예외**:
- 일회성 디버그 프린트(`dbg!`, 개발자 로컬 트러블슈팅)는 구조화할 필요 없음 — 커밋되지 않는 것이 전제.
- 로그 소비자가 영구적으로 사람뿐이고(예: 단발성 CLI 도구의 stderr) 자동 분석 요구가 전혀 없는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-002: request/frame/session ID 없음
**분류**: 상관관계 부재 · **심각도**: Critical · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
#[tauri::command]
async fn parse_bitstream(path: String) -> Result<ParseResult, String> {
    tracing::info!("parsing {}", path);
    let result = parser::parse(&path).await.map_err(|e| e.to_string())?;
    tracing::info!("parse done, {} frames", result.frame_count);
    Ok(result)
}
```

**문제**:
- 동일 파일을 여러 탭/패널에서 동시에 열면, 로그 상에서 어느 호출이 어느 결과로 이어졌는지 구분 불가능.
- 프론트엔드에서 발생한 사용자 액션(더블클릭, 드래그앤드롭)과 백엔드 로그를 이어 붙일 앵커가 없다.
- 취소된 요청과 완료된 요청의 로그가 뒤섞여, "왜 이 파싱이 두 번 로그에 나오는가"를 답할 수 없다.

**발생 조건**:
- 사용자가 큰 파일을 열자마자 실수로 다시 열기를 눌러 두 개의 파싱 요청이 겹칠 때.
- 여러 프레임 뷰어 패널이 동시에 같은 백엔드 워커 풀을 공유할 때 어떤 패널이 느려지는지 추적할 때.

**권장**:
```rust
use uuid::Uuid;

#[tauri::command]
async fn parse_bitstream(path: String) -> Result<ParseResult, String> {
    let request_id = Uuid::new_v4();
    let span = tracing::info_span!("parse_bitstream", %request_id, path = %path);
    let _enter = span.enter();

    tracing::info!("parsing started");
    let result = parser::parse(&path).await.map_err(|e| {
        tracing::error!(%request_id, error = %e, "parse failed");
        e.to_string()
    })?;
    tracing::info!(frame_count = result.frame_count, "parse done");
    Ok(result)
}
```
- 모든 IPC 커맨드 진입점에서 `request_id`(또는 `session_id`)를 생성하고, 프론트엔드로 반환해 UI 로그와 짝지을 수 있게 한다.
- 프레임 단위 작업에는 `frame_idx`를, 디코드 세션에는 `session_id`를 span 필드로 항상 부착한다.
- ID는 커맨드 경계뿐 아니라 그 안에서 spawn되는 하위 task/thread까지 전파한다.

**탐지 방법**:
- Structural: `#[tauri::command]` 함수 본문에서 `info_span!`/`request_id` 부재를 정적 검색.
- Runtime: 동일 타임스탬프대에 상관관계 필드 없이 로그 라인이 인터리빙되는지 실제 로그 샘플로 확인.

**예외**:
- 앱 시작 시 1회만 실행되는 초기화 로그(설정 로드 등)는 상관관계 ID가 불필요.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-003: decode·parse·metric 시간을 합쳐 기록
**분류**: 측정 뭉개기 · **심각도**: High · **탐지**: Structural/Semantic

**나쁜 예**:
```rust
let start = Instant::now();
let nal_units = parse_annexb(&buffer)?;
let frame = decode_slice(&nal_units)?;
let psnr = compute_psnr(&frame, &reference)?;
let elapsed = start.elapsed();
metrics::histogram!("frame_processing_ms").record(elapsed.as_millis() as f64);
```

**문제**:
- "왜 이 프레임이 느린가"에 답할 수 없다 — 파싱이 느린지, 디코드가 느린지, PSNR 계산이 느린지 뭉개져 있다.
- 코덱별로 파싱 비용과 디코드 비용의 비율이 완전히 다른데(AV1 파싱은 무겁고 VP9는 가볍다), 합산 지표로는 회귀가 어느 단계에서 왔는지 알 수 없다.
- 최적화 우선순위를 정할 근거 데이터가 없어, "감으로" 디코더부터 튜닝하다가 실제 병목(품질 지표 계산)을 놓친다.

**발생 조건**:
- 릴리스 후 "프레임 처리가 느려졌다"는 벤치마크 회귀 알림이 뜨는데, 원인 커밋 범위 내 파싱/디코드/메트릭 세 PR이 모두 섞여 있을 때.

**권장**:
```rust
let parse_start = Instant::now();
let nal_units = parse_annexb(&buffer)?;
metrics::histogram!("stage_ms", "stage" => "parse").record(parse_start.elapsed().as_millis() as f64);

let decode_start = Instant::now();
let frame = decode_slice(&nal_units)?;
metrics::histogram!("stage_ms", "stage" => "decode").record(decode_start.elapsed().as_millis() as f64);

let metric_start = Instant::now();
let psnr = compute_psnr(&frame, &reference)?;
metrics::histogram!("stage_ms", "stage" => "metric").record(metric_start.elapsed().as_millis() as f64);
```
- 파이프라인 단계마다 별도 타이머와 `stage` 레이블을 붙여 히스토그램을 분리한다.
- 합산 지표(`frame_processing_ms`)는 유지하되, 항상 분해 가능한 스팬 트리와 함께 남긴다(`tracing`의 nested span 활용).
- 단계별 p50/p95/p99를 대시보드에서 나란히 볼 수 있게 한다.

**탐지 방법**:
- Structural: 하나의 `Instant::now()`가 여러 이질적 함수 호출(파싱+디코드+메트릭)을 감싸는 패턴 검색.
- Semantic: 코드 리뷰에서 "이 타이머가 측정하는 단일 관심사는 무엇인가"를 명시적으로 질문.

**예외**:
- 정말 하나의 원자적 연산으로 취급해야 하는 경우(예: 서드파티 SDK 호출이 파싱+디코드를 블랙박스로 묶어 제공)는 합산이 불가피 — 이때는 최소한 블랙박스임을 지표 이름에 명시(`vendor_sdk_total_ms`).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-004: queue wait와 execution time을 구분하지 않음
**분류**: 측정 뭉개기 · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
async fn submit_decode_job(job: DecodeJob, pool: &WorkerPool) -> DecodeResult {
    let start = Instant::now();
    let result = pool.execute(job).await; // 큐 대기 + 실제 실행이 이 안에 섞여 있음
    metrics::histogram!("decode_job_ms").record(start.elapsed().as_millis() as f64);
    result
}
```

**문제**:
- 워커 풀이 포화 상태여서 대기 시간이 늘어난 것인지, 개별 작업 자체가 느려진 것인지 구분할 수 없다.
- "동시 재생 세션 수를 늘렸더니 느려졌다" 같은 스케일링 이슈를 진단할 때, 큐 길이 증가가 원인인지 CPU 경합이 원인인지 알 수 없다.
- 워커 풀 크기 튜닝(스레드 수 증설 여부 결정)에 필요한 핵심 신호가 없다.

**발생 조건**:
- 여러 패널에서 동시에 프레임을 요청할 때 응답 지연이 커지는데, 워커를 늘려야 할지 알고리즘을 최적화해야 할지 판단이 필요한 순간.

**권장**:
```rust
async fn submit_decode_job(job: DecodeJob, pool: &WorkerPool) -> DecodeResult {
    let enqueue_time = Instant::now();
    let (exec_start_tx, exec_start_rx) = oneshot::channel();

    let result = pool.execute_tracked(job, exec_start_tx).await;

    let exec_start = exec_start_rx.await.unwrap_or(enqueue_time);
    let queue_wait_ms = (exec_start - enqueue_time).as_millis() as f64;
    let exec_ms = exec_start.elapsed().as_millis() as f64;

    metrics::histogram!("decode_queue_wait_ms").record(queue_wait_ms);
    metrics::histogram!("decode_exec_ms").record(exec_ms);
    result
}
```
- 큐에 들어간 시각과 워커가 실제로 작업을 꺼낸 시각을 별도로 기록한다.
- 워커 풀 구현이 이를 지원하지 않으면, 작업 실행 콜백의 첫 라인에서 타임스탬프를 찍어 큐 대기시간을 역산한다.
- 큐 깊이(대기 중인 작업 수)도 게이지로 함께 노출한다.

**탐지 방법**:
- Structural: `pool.execute(...).await`를 감싼 단일 타이머 패턴 검색.
- Runtime: 부하 테스트 중 `queue_wait_ms`와 `exec_ms`를 분리 관찰할 수 있는지 확인 — 없으면 이 안티패턴.

**예외**:
- 워커 풀이 없는 단순 동기 호출(큐가 애초에 존재하지 않음)에는 해당 없음.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-005: cache hit만 있고 byte hit 없음
**분류**: 지표 불완전 · **심각도**: Medium · **탐지**: Structural/Semantic

**나쁜 예**:
```rust
struct FrameCache {
    entries: LruCache<FrameKey, Arc<DecodedFrame>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl FrameCache {
    fn get(&self, key: &FrameKey) -> Option<Arc<DecodedFrame>> {
        if let Some(frame) = self.entries.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(frame.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}
```

**문제**:
- hit count로는 "히트율 90%"처럼 보이지만, 히트한 항목이 대부분 작은 썸네일이고 미스가 대부분 큰 원본 프레임이면 실제 I/O·메모리 절감 효과는 훨씬 작다.
- 캐시 용량 산정(메모리 예산 대비 몇 %를 차지하는가)에 byte 단위 지표가 필수인데 개수만으로는 왜곡된다.
- 4K HEVC 프레임과 다운스케일 미리보기 프레임이 같은 캐시를 쓸 때, "히트율은 높은데 왜 메모리 절감 체감이 안 되는가" 질문에 답할 수 없다.

**발생 조건**:
- 캐시 크기(엔트리 수 제한)를 늘렸는데 체감 성능이 그대로일 때, 실제로는 큰 프레임이 계속 축출(evict)되고 있는지 확인이 필요한 순간.

**권장**:
```rust
struct FrameCache {
    entries: LruCache<FrameKey, Arc<DecodedFrame>>,
    hit_count: AtomicU64,
    hit_bytes: AtomicU64,
    miss_count: AtomicU64,
    miss_bytes: AtomicU64, // 미스 후 로드된 프레임 크기
}

impl FrameCache {
    fn get(&self, key: &FrameKey) -> Option<Arc<DecodedFrame>> {
        if let Some(frame) = self.entries.get(key) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            self.hit_bytes.fetch_add(frame.byte_size() as u64, Ordering::Relaxed);
            Some(frame.clone())
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}
```
- 카운트 기반 hit/miss와 별도로 byte 기반 hit/miss를 항상 병행 기록한다.
- 캐시 점유율도 "엔트리 수 / 최대치"가 아니라 "byte 사용량 / byte 예산"으로 노출한다.
- 프레임 크기가 균일하지 않은 캐시(썸네일+원본 혼재)는 byte hit 없이는 튜닝이 사실상 불가능하다는 점을 리뷰 체크리스트에 명시.

**탐지 방법**:
- Structural: 캐시 구현체에서 `hits`/`misses` 필드가 있는지, `bytes` 관련 필드가 함께 있는지 대조 검색.
- Semantic: 캐시 크기 조정 PR 리뷰 시 "byte 기준 근거가 있는가" 질문.

**예외**:
- 엔트리 크기가 완전히 균일한 캐시(예: 고정 크기 해시 결과만 캐싱)는 count만으로 충분.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-006: allocation peak를 기록하지 않음
**분류**: 지표 불완전 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
fn decode_gop(nal_units: &[NalUnit]) -> Result<Vec<Frame>, DecodeError> {
    let frames = nal_units.iter()
        .map(|nal| decode_frame(nal))
        .collect::<Result<Vec<_>, _>>()?;
    // 성공하면 아무 메모리 지표도 남기지 않음 — RSS는 "지금" 값만 대시보드에 노출
    Ok(frames)
}
```

**문제**:
- 8K HEVC GOP를 디코드하는 동안 순간적으로 메모리가 급증했다가 해제되면, 프로세스가 죽지 않는 한 그 피크는 어디에도 남지 않는다.
- OOM으로 프로세스가 강제 종료된 뒤 "무엇이 메모리를 다 썼는가"를 사후에 재구성할 방법이 없다.
- 평균/현재 RSS만 보는 대시보드는 짧고 뾰족한 피크(spike)를 완전히 숨긴다 — 샘플링 주기보다 짧은 피크는 절대 보이지 않는다.

**발생 조건**:
- 사용자가 "대용량 파일을 열면 가끔 앱이 죽는다"고 신고했는데, 재현이 안 되고 사후 로그에 메모리 정보가 전혀 없을 때.

**권장**:
```rust
struct PeakTracker {
    current: AtomicU64,
    peak: AtomicU64,
}

impl PeakTracker {
    fn add(&self, bytes: u64) {
        let new_val = self.current.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.peak.fetch_max(new_val, Ordering::Relaxed);
    }
    fn sub(&self, bytes: u64) {
        self.current.fetch_sub(bytes, Ordering::Relaxed);
    }
}

fn decode_gop(nal_units: &[NalUnit], tracker: &PeakTracker) -> Result<Vec<Frame>, DecodeError> {
    let frames = nal_units.iter()
        .map(|nal| {
            let frame = decode_frame(nal)?;
            tracker.add(frame.byte_size() as u64);
            Ok(frame)
        })
        .collect::<Result<Vec<_>, DecodeError>>()?;

    metrics::gauge!("gop_allocation_peak_bytes").set(tracker.peak.load(Ordering::Relaxed) as f64);
    Ok(frames)
}
```
- 커스텀 allocator 훅(`GlobalAlloc` 래퍼) 또는 주기적 RSS 샘플링(짧은 간격, 예: 10ms)으로 peak를 별도 추적한다.
- GOP/세션 단위로 "이 작업 동안의 peak"을 스코프화해 기록한다 — 프로세스 전역 peak만으로는 어느 작업이 원인인지 알 수 없다.
- 크래시 리포트에 마지막으로 관측된 peak 값을 포함시킨다(OBS-015와 연계).

**탐지 방법**:
- Runtime: 부하 테스트 중 `valgrind massif`/`heaptrack` 등으로 실측한 peak와 앱 자체 지표를 대조 — 자체 지표가 없으면 이 안티패턴.
- Static: allocator 훅이나 peak 게이지 코드가 저장소에 존재하는지 검색.

**예외**:
- 메모리 사용량이 입력 크기에 선형적이고 상한이 명확히 낮은 소규모 유틸리티(예: 헤더 몇 바이트만 읽는 프로브 커맨드)는 peak 추적이 과잉일 수 있음.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-007: dropped/stale/cancelled 작업 카운터 없음
**분류**: 지표 불완전 · **심각도**: Medium · **탐지**: Structural/Semantic

**나쁜 예**:
```rust
async fn on_seek(new_frame_idx: u32, state: Arc<PlayerState>) {
    let task = tokio::spawn(async move {
        let frame = decode_frame_at(new_frame_idx).await;
        state.set_current_frame(frame);
    });
    // 이전 seek로 인한 진행 중 task는 그냥 버려짐(drop) — 아무 기록도 없음
    state.replace_pending_task(task);
}
```

**문제**:
- 사용자가 타임라인을 빠르게 드래그하면 수십 개의 디코드 요청이 취소되는데, 이 취소가 정상 동작인지 버그인지 로그만으로 구분이 안 된다.
- "디코드 성공률 99%"라는 지표가 사실은 분모에서 취소된 작업을 빼고 계산된 것이라면, 실제 낭비된 작업량(CPU 소모)이 완전히 숨겨진다.
- stale 결과(이미 사용자가 다른 프레임으로 넘어간 뒤 도착하는 오래된 디코드 결과)를 조용히 버리는 코드가 있다면, 그 자체가 낭비인데 지표에 안 잡힌다.

**발생 조건**:
- CPU 사용률이 이상하게 높은데 "성공적으로 완료된 작업 수"는 적을 때 — 실제로는 취소된 작업이 CPU를 다 쓰고 버려지고 있는 상황.
- 빠른 스크러빙(scrubbing) UX 최적화를 검토할 때 현재 취소율이 얼마인지 기준선이 없어 개선 효과를 측정할 수 없음.

**권장**:
```rust
async fn on_seek(new_frame_idx: u32, state: Arc<PlayerState>) {
    if let Some(prev_task) = state.take_pending_task() {
        prev_task.abort();
        metrics::counter!("seek_task_cancelled_total").increment(1);
    }

    let task = tokio::spawn(async move {
        match decode_frame_at(new_frame_idx).await {
            Ok(frame) if state.is_current_target(new_frame_idx) => {
                state.set_current_frame(frame);
            }
            Ok(_) => {
                metrics::counter!("seek_result_stale_total").increment(1);
            }
            Err(_) => {
                metrics::counter!("seek_decode_failed_total").increment(1);
            }
        }
    });
    state.replace_pending_task(task);
}
```
- 취소(cancel), 만료(stale), 실패(failed), 성공(success) 네 상태를 명시적으로 구분해 카운터를 남긴다.
- "요청 대비 실제 사용된 결과 비율"을 대시보드 지표로 노출해 낭비 작업량을 가시화한다.
- abort된 task가 실제로 리소스를 즉시 반납하는지도 함께 확인(비동기 취소가 협조적이지 않으면 카운터만 늘고 실제 CPU는 계속 소모될 수 있음).

**탐지 방법**:
- Structural: `.abort()`, `drop(task)`, `if !is_current { return; }` 류의 조용한 폐기 패턴 주변에 카운터 증가가 없는지 검색.
- Semantic: 스크러빙/빠른 탐색 같은 "요청 폭주 후 대부분 버려지는" UX 흐름을 코드에서 식별하고 계측 여부 확인.

**예외**:
- 완전히 동기적이고 취소 개념 자체가 없는 짧은 연산(수 마이크로초 내 완료)에는 불필요.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-008: IPC payload size 측정 없음
**분류**: 지표 불완전 · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
#[tauri::command]
async fn get_frame_hex_data(session_id: String, offset: u64, size: u64) -> Result<Vec<u8>, String> {
    let data = read_bytes(&session_id, offset, size).map_err(|e| e.to_string())?;
    Ok(data) // 몇 바이트를 프론트엔드로 직렬화해 넘기는지 아무도 기록하지 않음
}
```

**문제**:
- Tauri IPC는 프론트엔드-백엔드 간 직렬화/역직렬화 비용이 있는데, payload가 커지면 (특히 hex viewer, 프레임 썸네일 등 바이너리 데이터) UI가 멈추는 원인이 되어도 원인 규명이 안 된다.
- "UI가 가끔 렉 걸린다"는 신고가 들어와도, 어떤 커맨드가 몇 MB를 얼마나 자주 실어 날랐는지 데이터가 없어 IPC가 원인인지 배제조차 못 한다.
- payload 크기 상한 정책(예: "1MB 넘으면 청크로 나눠라")을 세우려 해도 현재 분포를 모르면 임계값을 정할 근거가 없다.

**발생 조건**:
- 대형 4K/8K 프레임의 hex dump나 raw pixel 데이터를 IPC로 통째로 넘기다 UI 스레드가 얼어붙는 버그를 조사할 때.

**권장**:
```rust
#[tauri::command]
async fn get_frame_hex_data(session_id: String, offset: u64, size: u64) -> Result<Vec<u8>, String> {
    let data = read_bytes(&session_id, offset, size).map_err(|e| e.to_string())?;

    metrics::histogram!("ipc_payload_bytes", "command" => "get_frame_hex_data")
        .record(data.len() as f64);
    if data.len() > 1_000_000 {
        tracing::warn!(session_id, bytes = data.len(), "large IPC payload");
    }
    Ok(data)
}
```
- 모든 IPC 커맨드의 응답 크기를 히스토그램으로 기록하고, 커맨드 이름을 레이블로 분리한다.
- 임계값(예: 1MB)을 넘는 payload는 별도 경고 로그로 남겨 검색 가능하게 한다.
- 대형 데이터는 애초에 IPC 대신 파일 핸들/스트리밍(청크 전송)으로 우회하는 설계를 검토하되, 우회 전에 먼저 "얼마나 큰가"를 계측해 근거를 확보한다.

**탐지 방법**:
- Structural: `#[tauri::command]` 반환 타입이 `Vec<u8>`/대형 `String`/`serde_json::Value`인 함수에서 크기 로깅 부재를 검색.
- Runtime: 프론트엔드 devtools 네트워크/IPC 패널과 백엔드 지표를 대조해 누락된 커맨드 식별.

**예외**:
- 응답이 항상 몇 바이트 이내로 고정된 커맨드(단순 boolean/enum 반환)는 측정 불필요.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-009: UI long task와 backend latency 연결 불가
**분류**: 상관관계 부재 · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```typescript
// frontend: 백엔드 호출 시각과 UI freeze를 이어줄 식별자가 전혀 없음
async function loadFrame(idx: number) {
  const data = await invoke<FrameData>("get_frame", { idx });
  renderFrame(data); // 여기서 60ms+ 걸려도 아무 계측 없음
}
```
```rust
// backend: 프론트엔드가 어떤 렌더 프레임을 위해 호출했는지 알 방법이 없음
#[tauri::command]
async fn get_frame(idx: u32) -> Result<FrameData, String> {
    decode_and_return(idx).await.map_err(|e| e.to_string())
}
```

**문제**:
- 사용자가 체감하는 "버벅임"이 프론트엔드 렌더링(long task)에서 오는지, 백엔드 디코드 지연에서 오는지, IPC 왕복 자체에서 오는지 구분할 신호가 전혀 없다.
- Chrome DevTools Performance 탭에서 잡히는 long task와 Rust 쪽 `tracing` 스팬이 같은 타임라인 위에 놓이지 않아, 둘을 수동으로 시각 정렬해야 한다(사실상 불가능).
- 성능 회귀가 프론트엔드 PR 때문인지 백엔드 PR 때문인지 이분법적으로 판단할 근거가 없어 팀 간 책임 소재 논쟁으로 흐른다.

**발생 조건**:
- "재생 중 가끔 프레임이 뚝뚝 끊긴다"는 신고에 대해, 프론트엔드팀은 "백엔드가 느리다"고 하고 백엔드팀은 "우리 지표는 정상"이라고 서로 다른 데이터를 볼 때.

**권장**:
```typescript
async function loadFrame(idx: number) {
  const requestId = crypto.randomUUID();
  const t0 = performance.now();
  performance.mark(`frame-request-${requestId}-start`);

  const data = await invoke<FrameData>("get_frame", { idx, requestId });

  performance.mark(`frame-request-${requestId}-recv`);
  const ipcMs = performance.now() - t0;

  const renderStart = performance.now();
  renderFrame(data);
  const renderMs = performance.now() - renderStart;

  reportUiMetric({ requestId, ipcMs, renderMs, backendMs: data.backendElapsedMs });
}
```
```rust
#[tauri::command]
async fn get_frame(idx: u32, request_id: String) -> Result<FrameData, String> {
    let span = tracing::info_span!("get_frame", %request_id, idx);
    let _enter = span.enter();
    let start = Instant::now();
    let frame = decode_and_return(idx).await.map_err(|e| e.to_string())?;
    Ok(FrameData { backend_elapsed_ms: start.elapsed().as_millis() as u64, ..frame })
}
```
- 프론트엔드에서 생성한 `request_id`를 IPC 인자로 백엔드에 전달하고, 백엔드는 이를 `tracing` span에 부착한다.
- 백엔드 처리 시간을 응답 payload에 실어 프론트엔드로 되돌려주어, 클라이언트가 "IPC 왕복 - 백엔드 처리 = 순수 오버헤드"를 계산할 수 있게 한다.
- 프론트엔드 `performance.mark`/`measure`와 백엔드 span을 같은 `request_id`로 사후에 조인할 수 있는 로그 파이프라인을 구축한다(Wave 3 `UX_SCENARIO.md`의 계측 지점과 동일 원칙).

**탐지 방법**:
- Structural: `invoke()` 호출부에 request 상관관계 인자가 없는지, 백엔드 커맨드 시그니처에 대응 파라미터가 없는지 대조 검색.
- Runtime: 실제 성능 이슈 재현 시 프론트엔드 성능 마크와 백엔드 로그를 시간만으로 수동 매칭해봐서 안 맞으면 이 안티패턴이 원인.

**예외**:
- 완전히 동기적이고 항상 1ms 미만인 IPC 커맨드(단순 getter)는 상관관계 계측이 과잉.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-010: 오류가 발생한 codec state를 재현할 정보 없음
**분류**: 재현성 부재 · **심각도**: Critical · **탐지**: Structural/Semantic

**나쁜 예**:
```rust
fn parse_sps(bits: &mut BitReader) -> Result<Sps, ParseError> {
    let profile_idc = bits.read_u8(8)?;
    let level_idc = bits.read_u8(8)?;
    let seq_parameter_set_id = bits.read_ue()?;
    if seq_parameter_set_id > 31 {
        return Err(ParseError::InvalidSpsId); // 어떤 파일의 몇 번째 바이트에서, 어떤 이전 state였는지 정보 없음
    }
    // ...
}
```

**문제**:
- HEVC/AV1처럼 상태가 누적되는 파서(SPS/PPS 참조, 이전 프레임의 reference picture set 등)는 에러 시점의 파일 오프셋, 직전까지 파싱된 파라미터셋 상태가 없으면 재현이 사실상 불가능하다.
- 사용자가 보낸 파일로만 재현되고 개발자 로컬에서는 재현이 안 될 때(파일이 크거나 공유 불가), 에러 메시지 하나로는 어디서부터 디버깅을 시작해야 할지조차 모른다.
- "이 파일의 몇 번째 NAL 유닛에서 실패했는가"가 없으면, 동일 파일이라도 매번 처음부터 전체를 다시 파싱하며 눈으로 추적해야 한다.

**발생 조건**:
- 특정 인코더가 생성한(스펙 경계선의) 비표준 비트스트림을 열었을 때만 발생하는 파싱 실패를 조사할 때.
- 사용자가 파일을 공유할 수 없는 상황(기밀 콘텐츠)에서 원격으로 버그를 진단해야 할 때.

**권장**:
```rust
#[derive(Debug)]
struct ParseErrorContext {
    byte_offset: u64,
    nal_unit_index: u32,
    last_valid_sps_id: Option<u8>,
    last_valid_pps_id: Option<u8>,
    codec: CodecKind,
}

fn parse_sps(bits: &mut BitReader, ctx: &ParseState) -> Result<Sps, ParseError> {
    let profile_idc = bits.read_u8(8)?;
    let level_idc = bits.read_u8(8)?;
    let seq_parameter_set_id = bits.read_ue()?;
    if seq_parameter_set_id > 31 {
        let error_ctx = ParseErrorContext {
            byte_offset: bits.byte_offset(),
            nal_unit_index: ctx.nal_index,
            last_valid_sps_id: ctx.last_sps_id,
            last_valid_pps_id: ctx.last_pps_id,
            codec: ctx.codec,
        };
        tracing::error!(?error_ctx, "invalid sps_id");
        return Err(ParseError::InvalidSpsId(error_ctx));
    }
    // ...
}
```
- 파싱 에러 타입 자체에 재현에 필요한 컨텍스트(바이트 오프셋, NAL 인덱스, 직전 파라미터셋 상태)를 구조체로 담는다.
- 에러 발생 시 "이 지점까지의 파싱 상태를 재현하는 최소 바이트 범위"를 추출해 자동 첨부할 수 있게 설계한다(OBS-017의 진단 번들과 연결).
- 가능하면 에러 직전 N바이트를 hex와 함께 로그에 남겨(민감정보가 아닌 경우) 즉시 재현 스니펫을 만들 수 있게 한다.

**탐지 방법**:
- Structural: `ParseError` variant들이 payload 없이 unit-like enum인지 검사(`InvalidSpsId` vs `InvalidSpsId { offset, ... }`).
- Semantic: 코드 리뷰에서 "이 에러만 보고 재현 파일 없이 원인을 좁힐 수 있는가"를 체크리스트 질문으로 추가.

**예외**:
- 상태가 전혀 없는 무상태(stateless) 검증(예: 매직 넘버 체크)은 오프셋 정도만 있으면 충분, 파라미터셋 히스토리는 불필요.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-011: trace span이 async boundary에서 끊김
**분류**: 상관관계 부재 · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
async fn decode_pipeline(job: DecodeJob) -> Result<Frame, DecodeError> {
    let span = tracing::info_span!("decode_pipeline");
    let _enter = span.enter(); // async fn 안에서 guard를 await 너머로 들고 가면 트레이스가 끊기거나 패닉

    let parsed = tokio::spawn(async move { parse(job.data).await }).await??; // 새 task에 span 전파 안 됨
    let frame = decode(parsed).await?;
    Ok(frame)
}
```

**문제**:
- `tracing::Span`의 `Entered` guard는 `!Send`라 `.await` 지점을 넘겨 들고 있으면 컴파일 경고/런타임 혼선이 생기거나, 애초에 컴파일이 안 되어 개발자가 우회하다 span을 아예 빼먹는다.
- `tokio::spawn`으로 새 task를 띄우면 현재 span이 자동으로 전파되지 않아, 하위 task의 로그가 부모 트레이스와 완전히 단절된 별도 트리로 보인다.
- 결과적으로 하나의 논리적 요청(파일 파싱 → 디코드 → 메트릭 계산)이 트레이스 뷰어에서 3개의 서로 무관한 루트 span으로 흩어져, 종단간(end-to-end) 지연 분석이 불가능해진다.

**발생 조건**:
- Jaeger/Tempo 같은 트레이스 뷰어에서 하나의 요청을 검색했는데 파싱 span만 보이고 디코드 span은 완전히 별도 트레이스로 잡힐 때.
- `tokio::spawn`, `rayon::spawn`, 스레드 풀 제출 등 실행 컨텍스트가 바뀌는 모든 지점에서 반복 발생.

**권장**:
```rust
use tracing::Instrument;

async fn decode_pipeline(job: DecodeJob) -> Result<Frame, DecodeError> {
    async {
        let parsed = tokio::spawn(
            async move { parse(job.data).await }.in_current_span() // 명시적 span 전파
        ).await??;
        let frame = decode(parsed).await?;
        Ok(frame)
    }
    .instrument(tracing::info_span!("decode_pipeline"))
    .await
}
```
- `.enter()` guard를 async 블록에서 직접 들고 있지 말고, `.instrument(span)` 콤비네이터로 future 전체를 감싼다.
- `tokio::spawn`으로 새 task를 만들 때는 `.in_current_span()` 또는 span을 명시적으로 캡처해 전달한다.
- 스레드 풀(rayon 등) 경계를 넘을 때도 부모 span의 `Id`를 캡처해 새 스레드에서 `span.enter()`로 재진입한다.

**탐지 방법**:
- Structural: `tokio::spawn(async move { ... })` 호출부에 `.instrument(`/`.in_current_span()`이 없는 패턴 검색.
- Runtime: 실제 트레이스 백엔드에서 하나의 사용자 액션이 여러 개의 연결되지 않은 루트 span으로 나타나는지 확인.

**예외**:
- fire-and-forget으로 의도적으로 부모와 무관하게 실행되어야 하는 백그라운드 유지보수 task(예: 주기적 캐시 정리)는 독립 span이 오히려 맞음.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-012: 로그 레벨 변경을 위해 재시작 필요
**분류**: 운영성 부재 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn init_logging() {
    let level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_max_level(parse_level(&level))
        .init(); // 프로세스 시작 시 1회만 결정, 이후 변경 불가
}
```

**문제**:
- 프로덕션에서 문제가 재현 중일 때 `debug` 레벨로 잠깐 올려 관찰하고 싶어도, 재시작하면 재현 중이던 상태(디코더 세션, 캐시 상태)가 날아가 문제 자체가 사라진다.
- 데스크톱 앱(Bitvue처럼 Tauri 기반)에서 "재시작"은 사용자가 열어둔 파일/세션을 잃는 것을 의미해, 사용자에게 "로그 레벨 올리고 재현해달라"고 요청하기가 사실상 불가능하다.
- 상시 `debug` 레벨로 켜두면 OBS-013(성능 왜곡)이 발생하고, 상시 `info`로만 두면 필요한 순간에 정보가 부족한 딜레마에 빠진다.

**발생 조건**:
- 사용자가 특정 파일을 열고 특정 조작을 한 뒤에만 재현되는 버그를 진단할 때, 그 상태를 잃지 않고 로그만 상세하게 올리고 싶은 순간.

**권장**:
```rust
use tracing_subscriber::reload;

fn init_logging() -> reload::Handle<EnvFilter, Registry> {
    let filter = EnvFilter::new("info");
    let (filter, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
    reload_handle
}

#[tauri::command]
fn set_log_level(handle: tauri::State<reload::Handle<EnvFilter, Registry>>, level: String) -> Result<(), String> {
    handle.modify(|f| *f = EnvFilter::new(&level)).map_err(|e| e.to_string())
}
```
- `tracing_subscriber::reload`로 필터 핸들을 유지하고, 런타임에 레벨을 바꿀 수 있는 내부 커맨드/단축키를 제공한다.
- 모듈별 세분화된 필터(`bitvue_parser=debug,bitvue_ui=warn`)를 지원해, 전체를 debug로 올리지 않고도 필요한 부분만 상세화한다.
- 디버그 메뉴나 숨김 단축키(예: 개발자 모드 토글)로 사용자가 직접 레벨을 올려 "재현 중" 로그를 캡처하고 바로 진단 번들로 내보낼 수 있게 한다(OBS-017 연계).

**탐지 방법**:
- Static: `tracing_subscriber::fmt().init()` 같은 1회성 초기화 패턴을 검색하고, `reload::Layer` 사용 여부 확인.
- Structural: 로그 레벨을 바꿀 수 있는 IPC 커맨드나 설정 API가 저장소에 존재하는지 검색.

**예외**:
- 매우 짧게 실행되고 끝나는 CLI 유틸리티(1회 파싱 후 종료)는 재시작 비용이 사실상 0이라 불필요.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-013: verbose logging이 성능을 심하게 변경
**분류**: 관측 부작용 · **심각도**: High · **탐지**: Runtime/Semantic

**나쁜 예**:
```rust
fn decode_macroblock(mb: &Macroblock, ctx: &DecodeContext) -> Result<(), DecodeError> {
    tracing::debug!(
        mb_type = ?mb.mb_type,
        mv = ?mb.motion_vectors,       // Vec 전체를 매 매크로블록마다 Debug 포맷
        residuals = ?mb.residual_coeffs, // 수십~수백 계수 배열을 매번 포맷팅
        "decoding macroblock"
    );
    // ...
}
```

**문제**:
- 로그 레벨이 `debug` 이하로 필터링되어 실제로 출력되지 않아도, `?mb.motion_vectors`/`?mb.residual_coeffs` 같은 `Debug` 포맷팅 인자는 매크로 확장 방식에 따라 평가될 수 있어(특히 필터링이 런타임 체크인 구조에서) 프레임당 수만 번 호출되는 hot path에서 심각한 오버헤드를 유발한다.
- "디버그 로그를 켜서 재현해보니 버그가 사라졌다"는 하이젠버그(Heisenbug) 현상이 발생 — verbose 로깅 자체가 타이밍을 바꿔 레이스 컨디션을 은폐한다.
- 성능 벤치마크를 verbose 로깅이 켜진 빌드/환경에서 돌리면 실제 프로덕션 성능과 무관한 수치가 나와 잘못된 최적화 결정으로 이어진다.

**발생 조건**:
- 매크로블록/코딩 유닛 단위처럼 프레임당 수천 번 실행되는 루프 내부에 구조화 필드가 무거운 로그를 넣었을 때.
- "verbose 모드에서만 재현 안 되는" 버그 리포트를 받았을 때.

**권장**:
```rust
fn decode_macroblock(mb: &Macroblock, ctx: &DecodeContext) -> Result<(), DecodeError> {
    if tracing::event_enabled!(tracing::Level::TRACE) {
        tracing::trace!(
            mb_type = ?mb.mb_type,
            mv_count = mb.motion_vectors.len(),
            "decoding macroblock"
        );
    }
    // 상세 덤프는 명시적 요청 시에만, 별도 파일로
    if ctx.dump_requested_for(mb.index) {
        dump_macroblock_detail(mb, &ctx.dump_dir);
    }
    // ...
}
```
- `tracing::event_enabled!` 매크로로 실제 소비자가 있는지 먼저 체크한 뒤에만 무거운 포맷팅 인자를 구성한다(`tracing`은 대부분 이를 자동 처리하지만, 인자 계산 자체가 무거운 경우 명시적 가드가 안전).
- hot path에서는 매 반복 로깅 대신 샘플링(N개마다 1개) 또는 집계 후 요약 로깅으로 전환한다.
- verbose 로깅의 오버헤드 자체를 벤치마크로 측정해 "debug 레벨 활성화 시 처리량 X% 감소"를 문서화하고, 프로덕션 기본값과 분리된 전용 프로파일링 빌드를 고려한다.

**탐지 방법**:
- Runtime: 동일 워크로드를 `info`와 `debug`/`trace` 레벨로 각각 벤치마크해 처리량 차이를 측정 — 5% 이상 차이면 조사 대상.
- Static: hot path(루프 내부, 매크로블록/코딩 유닛 단위 함수)에서 `Debug`/`?` 포맷 필드를 가진 로그 매크로 검색.

**예외**:
- 이미 명확히 저빈도인 경로(세션 시작/종료, 파일 열기)의 verbose 로그는 오버헤드가 무시할 수준.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-014: 민감한 경로·파일명이 로그에 노출
**분류**: 보안/프라이버시 · **심각도**: High · **탐지**: Static/Semantic

**나쁜 예**:
```rust
tracing::info!("opening file: {}", path.display());
// path == "/Users/jsmith/Confidential/UnreleasedFilm_2027_ProjectRaven/reel3_final.mp4"
tracing::error!("failed to parse {}: {:?}", path.display(), err);
```

**문제**:
- 영상 분석 도구는 미공개 영화/광고/기밀 콘텐츠 파일을 다루는 경우가 많고, 파일명·경로 자체가 프로젝트명·클라이언트명·미공개 릴리즈 정보를 그대로 노출한다.
- 사용자가 버그 리포트에 로그를 첨부해 공개 이슈 트래커(GitHub 등)에 올리면, 경로에 담긴 정보가 그대로 유출된다.
- 원격 텔레메트리/크래시 리포팅 서비스로 로그가 전송되는 구조라면, 사용자 동의 범위를 넘어선 개인정보(사용자명이 포함된 홈 디렉터리 경로 등)가 서드파티 서버에 저장된다.

**발생 조건**:
- 사용자가 크래시 리포트를 자동 전송하거나, 진단 번들을 지원팀에 공유할 때.
- 로그가 CI/원격 저장소에 실수로 커밋되거나 공유될 때.

**권장**:
```rust
fn redact_path(path: &Path) -> String {
    // 파일명은 해시로, 디렉터리 구조는 깊이만 남김
    let hash = short_hash(path.to_string_lossy().as_bytes());
    format!("<file:{}:depth={}>", hash, path.components().count())
}

tracing::info!(file_ref = %redact_path(&path), "opening file");
tracing::error!(file_ref = %redact_path(&path), error = %err, "failed to parse");
```
- 파일 경로/이름을 로그에 남길 때는 원문 대신 세션 내에서 안정적인 해시나 별칭(`file_ref`)으로 치환한다.
- 로컬 전용 디버그 로그(사용자 기기에만 저장, 절대 전송되지 않음)와 원격 전송 로그를 별도 채널로 분리하고, 원격 채널에만 redaction을 강제한다.
- 진단 번들 생성 시(OBS-017) 경로 redaction을 기본값으로 하고, 사용자가 명시적으로 "전체 경로 포함"을 선택했을 때만 원문을 남긴다.

**탐지 방법**:
- Static: `path.display()`, `path.to_string_lossy()` 등이 로그 매크로 인자에 직접 들어가는 패턴을 grep.
- Semantic: 로그/크래시 리포트가 원격으로 전송되는 경로를 코드에서 식별하고, 해당 경로 상의 모든 필드에 redaction 여부 검토.

**예외**:
- 완전히 로컬에만 남고 절대 전송/공유되지 않는 것이 코드 수준에서 보장된 개발자 전용 디버그 빌드 로그는 원문 경로가 진단에 더 유용할 수 있음.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-015: crash 직전 작업 snapshot 없음
**분류**: 재현성 부재 · **심각도**: Critical · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("panic: {}", info); // 패닉 발생 순간의 어떤 작업 컨텍스트도 없음
    }));
    run_app();
}
```

**문제**:
- 패닉 메시지와 스택 트레이스만으로는 "그 순간 어떤 파일을, 어떤 프레임을, 어떤 코덱 경로로 처리 중이었는지" 전혀 알 수 없다.
- 사용자가 크래시를 신고해도 재현 조건(어떤 조작을 어떤 순서로 했는지)을 기억하지 못하면, 로그에 남은 정보가 재구성의 유일한 단서인데 그마저 없다.
- 멀티스레드 디코딩 환경에서는 패닉이 발생한 스레드 외 다른 스레드들이 그 순간 무엇을 하고 있었는지도 중요한데, 전혀 캡처되지 않는다.

**발생 조건**:
- 자동화 테스트에서는 재현되지 않고 특정 사용자의 특정 파일에서만 발생하는 크래시를 원격으로 진단해야 할 때.
- 크래시가 드물게(수백 세션에 1회) 발생해 로컬 재현 시도 자체가 비효율적일 때.

**권장**:
```rust
struct ActivitySnapshot {
    current_file: Option<String>, // redacted
    current_session_id: Option<Uuid>,
    current_frame_idx: Option<u32>,
    active_codec: Option<CodecKind>,
    last_ipc_commands: VecDeque<(String, Instant)>, // ring buffer, 최근 N개
}

fn install_panic_hook(activity: Arc<Mutex<ActivitySnapshot>>) {
    std::panic::set_hook(Box::new(move |info| {
        let snapshot = activity.lock().unwrap();
        tracing::error!(
            panic = %info,
            file_ref = ?snapshot.current_file,
            session_id = ?snapshot.current_session_id,
            frame_idx = ?snapshot.current_frame_idx,
            codec = ?snapshot.active_codec,
            recent_commands = ?snapshot.last_ipc_commands,
            "panic occurred"
        );
        write_crash_report(&snapshot); // 디스크에 즉시 flush, 다음 실행 시 업로드 제안
    }));
}
```
- 전역적으로 "현재 활성 작업 상태"를 저비용 스냅샷(원자적 필드 몇 개, ring buffer)으로 계속 갱신해두고, 패닉 훅에서 이를 읽어 크래시 리포트에 포함한다.
- 최근 IPC 커맨드/사용자 액션을 짧은 ring buffer로 유지해 "크래시 직전 무엇을 했는가"의 타임라인을 재구성한다.
- 크래시 리포트는 즉시 디스크에 flush하고(프로세스가 죽어도 남도록), 다음 실행 시 사용자에게 전송 여부를 물어본다.

**탐지 방법**:
- Structural: `panic::set_hook`이 있는지, 있다면 훅 본문이 활성 상태 컨텍스트를 참조하는지 검색.
- Runtime: 의도적으로 특정 시나리오(예: 특정 파일 열기 → 특정 프레임 탐색)에서 패닉을 주입해 리포트에 해당 컨텍스트가 담기는지 확인.

**예외**:
- 상태가 전혀 없는 순수 함수형 유틸리티 프로세스(입력 1개 받아 출력 1개 내는 CLI)는 스냅샷이 오버엔지니어링일 수 있음 — 인자 자체가 이미 재현 정보.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-016: perf counter가 release build에 제거
**분류**: 운영성 부재 · **심각도**: Medium · **탐지**: Static/Structural

**나쁜 예**:
```rust
#[cfg(debug_assertions)]
macro_rules! perf_counter {
    ($name:expr, $val:expr) => { metrics::histogram!($name).record($val) };
}
#[cfg(not(debug_assertions))]
macro_rules! perf_counter {
    ($name:expr, $val:expr) => {}; // release 빌드에서 완전히 사라짐
}
```

**문제**:
- 사용자가 실제로 겪는 성능 문제는 항상 release 빌드에서 발생하는데, 정작 그 빌드에는 원인을 진단할 지표가 하나도 없다.
- "내 로컬(debug 빌드)에서는 느리지 않다"는 흔한 함정에 빠지기 쉽다 — debug 빌드는 최적화가 꺼져 있어 애초에 성능 특성이 다른데, 지표까지 debug 전용이면 release 성능 문제를 debug 빌드로 재현하려는 잘못된 시도로 이어진다.
- 성능 회귀를 잡는 CI 벤치마크가 release 빌드로 도는 경우, 그 벤치마크 결과를 세분화해 어느 지점이 느려졌는지 볼 지표 자체가 없다.

**발생 조건**:
- 사용자로부터 "release 버전에서만 느리다"는 리포트를 받았는데, 정작 release 빌드에 아무 계측도 없어 추가 정보를 요청할 수밖에 없을 때.
- 성능 최적화 작업을 하면서 release 빌드 기준으로 전/후 비교를 하려는데 지표가 없어 wall-clock 감으로만 판단할 때.

**권장**:
```rust
// cfg(debug_assertions)로 게이팅하지 않는다. 대신 오버헤드가 낮은 지표는 항상 켜두고,
// 무거운 지표만 별도 feature flag로 옵트인한다.
macro_rules! perf_counter {
    ($name:expr, $val:expr) => { metrics::histogram!($name).record($val) };
}

#[cfg(feature = "heavy-instrumentation")]
macro_rules! perf_counter_detailed {
    ($name:expr, $val:expr) => { metrics::histogram!($name).record($val) };
}
#[cfg(not(feature = "heavy-instrumentation"))]
macro_rules! perf_counter_detailed {
    ($name:expr, $val:expr) => {};
}
```
- 저오버헤드 카운터(원자적 증가, 히스토그램 기록)는 `debug_assertions`가 아니라 항상 컴파일해 넣는다 — 이런 지표의 비용은 보통 무시할 수준이다.
- 정말 무거운 계측(상세 트레이스, 매 샘플 덤프)만 별도 Cargo feature(`heavy-instrumentation` 등)로 분리해, release 빌드에서도 "필요하면 켤 수 있는" 빌드를 별도로 만들 수 있게 한다.
- "release에서 지표 수집 여부"를 릴리스 체크리스트 항목으로 명시한다.

**탐지 방법**:
- Static: `#[cfg(debug_assertions)]`가 metrics/tracing 매크로 정의를 감싸는 패턴을 grep.
- Structural: release 빌드 산출물을 실제로 `perf`/지표 엔드포인트로 조회해 값이 나오는지 스모크 테스트.

**예외**:
- 컴파일러가 최적화로 제거하지 못할 만큼 무거운 계측(예: 프레임마다 전체 프레임 버퍼 체크섬 계산)은 release에서 기본 비활성이 합리적 — 다만 옵트인 스위치는 반드시 존재해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-017: 사용자용 진단 bundle 없음
**분류**: 운영성 부재 · **심각도**: High · **탐지**: Structural/Manual

**나쁜 예**:
```rust
// "버그가 있어요"라는 사용자에게 개발자가 매번 수동으로 요청:
// "로그 파일이 어디 있는지 아세요? OS 버전은요? 어떤 파일이었나요? 어느 프레임에서요?"
// -> 파일 위치를 아는 사용자가 드물고, 왕복 커뮤니케이션에 며칠씩 소요됨
```

**문제**:
- 버그 리포트를 받을 때마다 "로그 어디 있어요", "OS/GPU 정보 알려주세요", "재현 스텝 다시 알려주세요"를 수동으로 되묻는 왕복이 반복된다.
- 사용자 대부분은 로그 파일 위치, 앱 버전, 시스템 사양을 스스로 찾아낼 기술적 배경이 없어, 애초에 유용한 리포트 자체가 잘 안 들어온다.
- 진단에 필요한 정보(최근 로그, 활성 세션 상태, 시스템 사양, 최근 크래시)가 여러 파일/설정에 흩어져 있으면, 개발자가 받아도 조합하는 데 시간이 든다.

**발생 조건**:
- 일반 사용자(비개발자)가 "이상하게 느려요"/"파일이 안 열려요"라고 신고했을 때 후속 정보 수집이 필요한 모든 순간.

**권장**:
```rust
#[tauri::command]
async fn export_diagnostic_bundle(app: tauri::AppHandle) -> Result<PathBuf, String> {
    let bundle_dir = create_temp_dir()?;

    write_system_info(&bundle_dir)?;        // OS, GPU, 메모리, 앱 버전
    write_recent_logs(&bundle_dir, Duration::from_secs(600))?; // 최근 10분 로그
    write_redacted_session_state(&bundle_dir)?; // OBS-014 redaction 적용
    write_last_crash_report_if_any(&bundle_dir)?; // OBS-015 스냅샷
    write_active_config(&bundle_dir)?;

    let zip_path = zip_directory(&bundle_dir)?;
    tracing::info!(path = %zip_path.display(), "diagnostic bundle created");
    Ok(zip_path)
}
```
- "진단 정보 내보내기" 메뉴/버튼을 UI에 노출해, 사용자가 클릭 한 번으로 zip 파일을 생성하고 지원팀에 첨부할 수 있게 한다.
- 번들에는 시스템 정보, 최근 로그, redaction된 세션 상태, 마지막 크래시 리포트, 활성 설정을 포함한다.
- 번들 생성 자체를 계측해(생성 소요 시간, 크기) 번들 생성 과정이 또 다른 버그의 원인이 되지 않도록 확인한다.

**탐지 방법**:
- Structural: "diagnostic"/"export logs"/"support bundle" 관련 커맨드나 메뉴 항목이 저장소에 존재하는지 검색.
- Manual: 실제로 버그 리포트 프로세스를 처음부터 끝까지 밟아보고, 몇 번의 수동 왕복 커뮤니케이션이 필요한지 측정.

**예외**:
- 내부 개발팀만 사용하는 도구(외부 사용자가 없는 사내 전용 빌드)는 개발자가 직접 로그 위치를 알고 있으므로 우선순위가 낮음.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-018: timestamp clock source가 섞임
**분류**: 측정 왜곡 · **심각도**: Medium · **탐지**: Structural/Semantic

**나쁜 예**:
```rust
fn record_frame_latency(start: SystemTime, end: Instant) {
    // SystemTime(wall clock, NTP 보정 대상)과 Instant(monotonic)를 섞어 계산
    let elapsed = end.elapsed(); // end 자체가 Instant라 start(SystemTime)와 애초에 비교 불가한데
    // 실제 코드에서는 종종 SystemTime::now()끼리 빼거나, Instant와 UTC epoch를 섞어 로그에 같이 찍음
    let wall_elapsed = SystemTime::now().duration_since(start).unwrap_or_default();
    metrics::histogram!("frame_latency_ms").record(wall_elapsed.as_millis() as f64);
}
```

**문제**:
- `SystemTime`은 NTP 동기화, 사용자의 수동 시계 변경, 서머타임 전환 등으로 앞뒤로 튈 수 있어(non-monotonic), 지연시간 측정에 쓰면 음수 duration이나 비현실적인 스파이크가 발생한다.
- `Instant`(monotonic clock)와 `SystemTime`(wall clock)을 같은 계산식에 섞으면 컴파일 타임에는 안 걸리는 논리 버그가 생긴다 — 특히 한쪽은 이벤트 발생 시각 로깅용(wall clock 필요), 다른 쪽은 경과시간 측정용(monotonic 필요)인데 이 구분이 코드베이스 전체에서 일관되지 않으면 지표가 간헐적으로 말이 안 되는 값을 낸다.
- 분산된 로그(프론트엔드 JS `Date.now()`/`performance.now()`, 백엔드 Rust `SystemTime`/`Instant`)를 하나로 합쳐 타임라인을 그릴 때, clock source가 다르면 오차가 수십~수백 ms 단위로 어긋난다.

**발생 조건**:
- 지연시간 히스토그램에 가끔 음수나 비정상적으로 큰 값(수 시간)이 섞여 나올 때 — 대개 시스템 시계가 NTP 보정으로 튄 순간.
- 프론트엔드-백엔드 통합 타임라인(OBS-009)을 그릴 때 두 시스템의 시계 기준이 달라 순서가 뒤바뀌어 보일 때.

**권장**:
```rust
// 경과 시간(duration) 측정에는 항상 monotonic Instant만 사용
fn record_frame_latency(start: Instant) {
    let elapsed = start.elapsed(); // Instant::elapsed()는 항상 monotonic, 음수 불가
    metrics::histogram!("frame_latency_ms").record(elapsed.as_millis() as f64);
}

// "언제 발생했는가"를 로그에 남길 때만 wall clock(UTC) 사용, 절대 duration 계산에 쓰지 않음
fn log_event_timestamp() {
    let wall_clock_utc = chrono::Utc::now(); // 로깅/표시 전용
    tracing::info!(timestamp = %wall_clock_utc.to_rfc3339(), "event occurred");
}
```
- "얼마나 걸렸는가"는 항상 `Instant`(Rust)/`performance.now()`(JS)로 측정하고, "언제 발생했는가"는 항상 UTC wall clock으로 기록해 두 목적을 코드 레벨에서 타입으로 분리한다.
- 프론트엔드-백엔드 간 시각 상관관계가 필요하면(OBS-009), 절대 시각이 아니라 같은 `request_id` 기준 상대 오프셋으로 정렬한다.
- 코드 리뷰 체크리스트에 "이 duration 계산에 `SystemTime`이 관여하는가"를 명시적으로 넣는다.

**탐지 방법**:
- Static: `SystemTime::now().duration_since(...)`가 성능/지연시간 지표 계산에 쓰이는 패턴을 grep.
- Semantic: `Instant`와 `SystemTime`이 같은 함수/구조체 안에 섞여 있는지 타입 시그니처 검토.

**예외**:
- "이벤트가 실제로 몇 시 몇 분에 일어났는가"를 사람이 읽는 로그 타임스탬프로 남기는 용도는 wall clock이 정답 — duration 계산에만 쓰지 않으면 문제 없음.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-019: thread name이 없음
**분류**: 진단 편의성 부재 · **심각도**: Low · **탐지**: Static/Runtime

**나쁜 예**:
```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(8)
    .build()?; // 스레드 이름 없이 생성 -> "rayon-worker-3" 같은 기본값만 남음

std::thread::spawn(move || {
    decode_worker_loop();
}); // 이름 없는 스레드, 프로파일러/디버거에서 "Thread-142"로만 보임
```

**문제**:
- `perf`, `Instruments`(macOS), `VTune`, 또는 그냥 시스템 모니터에서 CPU를 많이 쓰는 스레드를 찾아도 "Thread-142"라는 이름으로는 그게 파서 스레드인지 디코더 스레드인지 UI 렌더 스레드인지 알 수 없다.
- 데드락/행(hang) 상황에서 스레드 덤프(`gdb`/`lldb` backtrace all)를 봐도, 이름이 없으면 어느 스레드가 무엇을 기다리는지 스택 프레임만으로 추론해야 해 진단 시간이 몇 배로 늘어난다.
- macOS Instruments나 Windows ETW 같은 OS 레벨 프로파일러의 타임라인 뷰에서 스레드별 색상/행 구분이 이름 없이는 무의미해진다.

**발생 조건**:
- 프로덕션 빌드에서 특정 스레드가 CPU를 100% 점유하는 문제를 시스템 프로파일러로 조사할 때.
- 여러 코덱(AVC/HEVC/VP9/AV1) 디코더가 각각 별도 스레드 풀을 쓰는 구조에서 "어느 코덱 처리가 느린가"를 스레드 목록만 보고 구분해야 할 때.

**권장**:
```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(8)
    .thread_name(|idx| format!("bitvue-decode-worker-{idx}"))
    .build()?;

std::thread::Builder::new()
    .name(format!("bitvue-parser-{codec:?}"))
    .spawn(move || {
        decode_worker_loop();
    })?;
```
- 애플리케이션이 생성하는 모든 스레드(수동 spawn, rayon/tokio 풀, 서드파티 라이브러리가 노출하는 빌더)에 목적을 알 수 있는 접두사(`bitvue-*`) + 역할(`decode-worker`, `parser`, `ui-render`)을 포함한 이름을 부여한다.
- tokio 런타임도 `Builder::new_multi_thread().thread_name("bitvue-async-worker")`로 이름을 지정한다.
- 이름 규칙을 문서화해 신규 스레드 생성 시 리뷰에서 누락을 잡는다.

**탐지 방법**:
- Static: `std::thread::spawn(`, `ThreadPoolBuilder::new()`, `Builder::new_multi_thread()` 호출부에 `.name(`/`.thread_name(`이 없는 패턴 grep.
- Runtime: 실행 중인 프로세스를 `ps -T`/Activity Monitor/Instruments로 열어 이름 없는 스레드(`Thread-N`) 비율 확인.

**예외**:
- 매우 짧게 존재하고 즉시 join되는 일회성 헬퍼 스레드(수 밀리초 내 종료)는 이름 없이도 진단 부담이 적음.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### OBS-020: profiler-friendly symbol 설정이 없음
**분류**: 진단 편의성 부재 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = true
strip = true      # 심볼을 완전히 제거 -> 프로파일러가 함수 이름 대신 주소만 표시
panic = "abort"
```

**문제**:
- `strip = true`로 심볼을 제거한 release 바이너리는 `perf record`/`Instruments`/`samply` 같은 프로파일러로 잡아도 함수 이름 대신 `0x7f8a3c1d2000` 같은 주소만 나와, 어떤 함수가 hot path인지 육안으로 알 수 없다.
- 사용자 환경에서 발생한 성능 문제를 원격으로 프로파일링해 받아도(예: `perf.data` 파일 전달), 심볼이 없으면 개발자 쪽에서 debug symbol을 따로 매칭해야 하는데 릴리스 시점의 정확한 심볼 파일을 보관해두지 않으면 영구히 분석 불가능해진다.
- LTO로 인라인이 공격적으로 일어나면 심볼이 있어도 "이 시간이 어느 원본 함수에서 왔는가"가 흐려지는데, 여기에 심볼까지 없으면 프로파일 데이터 자체가 쓸모없어진다.

**발생 조건**:
- 사용자가 "특정 시나리오에서 CPU 100%가 오래 지속된다"고 리포트했는데, 정작 그 사용자가 쓰는 release 빌드에서 뽑은 프로파일이 심볼 없이 주소만 나와 분석이 막힐 때.
- 자체 CI 성능 벤치마크에서 flamegraph를 뽑으려 했는데 release 바이너리가 strip되어 있어 무의미한 그래프만 나올 때.

**권장**:
```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = true
strip = false          # 심볼 유지, 별도 배포 바이너리에서만 strip
debug = 1               # 최소한의 라인 정보 포함 (완전한 debug=2보다 빌드 크기 절감)
panic = "unwind"        # 프로파일러/디버거가 스택을 완전히 풀 수 있도록

[profile.release-profiling]
inherits = "release"
debug = true
strip = false
```
- 사용자 배포용 바이너리는 별도 빌드 단계에서 strip하고, strip 전 심볼 파일(`.dSYM`/`.pdb`/`.debug`)을 버전과 매칭해 아카이브에 보관한다(`objcopy --only-keep-debug` 등으로 분리 저장).
- CI에 프로파일링 전용 프로파일(`release-profiling`)을 추가해 flamegraph/샘플링 프로파일러가 항상 유의미한 심볼을 볼 수 있게 한다.
- 크래시 리포트에 빌드 버전/커밋 해시를 포함시켜(OBS-015), 사후에 정확한 심볼 파일을 찾아 매칭할 수 있게 한다.

**탐지 방법**:
- Static: `Cargo.toml`의 `[profile.release]`에서 `strip = true` 또는 `strip = "symbols"` 여부, `debug` 필드 부재를 검사.
- Structural: CI에 flamegraph/perf 산출 단계가 있는지, 있다면 어떤 프로파일로 빌드하는지 확인.

**예외**:
- 바이너리 크기가 배포 채널(예: 매우 제한적인 다운로드 대역폭)의 하드 제약인 경우 최종 배포판은 strip하되, 반드시 심볼 파일을 별도 보관하는 것으로 절충.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
</content>
