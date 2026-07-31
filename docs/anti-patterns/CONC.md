# Anti-Pattern Catalog — CONC: Async와 병렬 처리

이 문서는 Bitvue(Tauri + Rust + React 기반 비디오 비트스트림 분석기) 안티패턴 카탈로그의 일부입니다. 전체 카탈로그 목록은 `docs/anti-patterns/INDEX.md`(별도 작성 예정)를 참고하세요. 본 문서는 1단계(범용 참조 카탈로그)이며, 실제 저장소 코드 감사는 2단계에서 수행합니다.

Bitvue의 동시성 모델을 전제로 작성되었습니다: Tokio 멀티스레드 런타임(Tauri IPC/커맨드 처리) + Rayon(메트릭 병렬 계산) + FFI 디코더(dav1d, libvmaf 등, 자체 내부 스레드 보유)가 한 프로세스 안에서 공존합니다.

---

### CONC-001: async 함수 안에서 blocking parse
**분류**: CONC · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
#[tauri::command]
async fn parse_nal_units(path: String) -> Result<Vec<NalUnit>, String> {
    // 수백MB 파일을 동기적으로 읽고 파싱 — Tokio worker thread를 통째로 점유
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let units = bitstream::parse_annexb(&data); // CPU-bound, 수백ms~수초
    Ok(units)
}
```

**문제**:
- `async fn` 안이라고 해서 자동으로 non-blocking이 되는 것은 아니며, 동기 코드가 그대로 실행되면 해당 worker thread는 완전히 점유된다.
- Tokio 멀티스레드 런타임은 worker thread 수가 제한적(기본 = 논리 CPU 수)이라, 하나의 커맨드가 blocking parse로 thread를 점유하면 같은 런타임에서 스케줄된 다른 모든 async task(다른 Tauri 커맨드, IPC 응답, timer)가 지연된다.
- 특히 파일 크기가 클수록 parse 시간이 늘어나고, 사용자는 "앱이 멈췄다"고 느낀다.

**발생 조건**:
- 대용량 컨테이너(MKV, MP4) 또는 raw bitstream 파일을 열 때.
- 여러 Tauri 커맨드가 동시에 호출되는 상황(예: 파일 열기 중 UI가 다른 패널 상태를 조회).
- worker thread 수가 적게 설정된 환경(저사양 머신, CI 컨테이너)일수록 영향이 커진다.

**권장**:
```rust
#[tauri::command]
async fn parse_nal_units(path: String) -> Result<Vec<NalUnit>, String> {
    tokio::task::spawn_blocking(move || {
        let data = std::fs::read(&path)?;
        Ok(bitstream::parse_annexb(&data))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: std::io::Error| e.to_string())
}
```
- CPU-bound 또는 blocking I/O 작업은 `spawn_blocking`으로 분리해 별도의 blocking thread pool에서 실행한다.
- 파싱 작업이 반복적으로 발생한다면 스트리밍 파서로 전환해 청크 단위로 `.await` 지점을 만드는 것도 고려한다.

**탐지 방법**:
- Static: `async fn` 본문에서 `std::fs::*`, `std::thread::sleep`, 무거운 루프가 `.await` 없이 직접 호출되는지 검사(clippy `unused_async`와는 별개로, tokio 전용 lint인 `clippy::await_holding_lock` 계열 및 사내 커스텀 lint 필요).
- Runtime: Tokio Console(`tokio-console`)로 worker thread가 장시간 "busy" 상태인 task를 찾는다.
- Manual: 코드 리뷰 시 async 함수 안의 모든 non-`.await` 호출이 O(1)~O(작은 상수)인지 확인.

**예외**:
- 파일이 매우 작다고 보장되는 경우(예: 수 KB의 설정 파일)라면 blocking 호출을 async 함수 안에 그대로 두어도 실질적 영향이 없다.
- 애초에 single-threaded 런타임(`#[tokio::main(flavor = "current_thread")]`)을 의도적으로 쓰는 CLI 도구라면 다른 트레이드오프가 적용된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-002: Tokio worker에서 decode 직접 실행
**분류**: CONC · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
#[tauri::command]
async fn decode_frame(state: tauri::State<'_, AppState>, frame_idx: u64) -> Result<FrameBuf, String> {
    let decoder = state.decoder.lock().await;
    // dav1d_send_data / dav1d_get_picture는 수 ms~수십 ms가 걸리는 CPU-bound FFI 호출
    let pic = decoder.decode_frame_sync(frame_idx).map_err(|e| e.to_string())?;
    Ok(pic.into())
}
```

**문제**:
- dav1d/libvmaf 같은 FFI 디코더 호출은 근본적으로 동기적이고 CPU를 많이 소모하는데, 이를 Tokio worker thread에서 직접 실행하면 CONC-001과 동일한 방식으로 런타임 전체 처리량을 떨어뜨린다.
- 여러 패널(프리뷰, 필름스트립, 히스토그램)이 동시에 `decode_frame`을 호출하면 각 호출이 worker thread를 하나씩 점유하고, worker 수를 넘는 순간 나머지 요청은 큐에서 대기하며 마치 "직렬화된 디코더"처럼 동작한다.
- 디코드 자체는 병렬화 가능한 작업인데, Tokio async 모델 안에 억지로 넣으면 병렬성을 제대로 활용하지 못한다.

**발생 조건**:
- Filmstrip처럼 다수의 썸네일 프레임을 동시에 요청할 때.
- 재생 중 프레임레이트를 맞추기 위해 짧은 간격으로 연속 decode 요청이 들어올 때.

**권장**:
```rust
#[tauri::command]
async fn decode_frame(state: tauri::State<'_, AppState>, frame_idx: u64) -> Result<FrameBuf, String> {
    let decoder = state.decoder.clone(); // Arc<DecoderPool> 등 thread-safe 핸들
    tokio::task::spawn_blocking(move || {
        decoder.decode_frame_sync(frame_idx)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
```
- 디코드처럼 CPU-bound·blocking인 FFI 호출은 항상 `spawn_blocking` 또는 전용 Rayon/워커 스레드 풀로 위임한다.
- 디코더 인스턴스 자체가 스레드 안전하지 않다면(CONC-024 참고) 전용 decode worker thread + 요청 채널 구조로 격리한다.

**탐지 방법**:
- Structural: `async fn` 안에서 `unsafe extern "C"` FFI 호출이나 `*_sync` 네이밍 함수가 `.await`/`spawn_blocking` 없이 직접 불리는 패턴을 grep.
- Runtime: 부하 테스트 중 Tokio worker thread 점유율과 커맨드 응답 지연(p99) 상관관계 측정.

**예외**:
- 디코더가 이미 자체적으로 논블로킹 콜백 기반 API를 제공하고 Rust 바인딩이 `Future`를 반환하는 경우는 예외.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-003: Tokio와 Rayon 중첩 병렬화
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
async fn compute_all_metrics(state: tauri::State<'_, AppState>) -> Result<Vec<MetricResult>, String> {
    let frames = state.frames.clone();
    tokio::task::spawn_blocking(move || {
        // Rayon의 전역 pool을 그대로 사용
        frames.par_iter().map(|f| {
            // 이 클로저 안에서 다시 tokio::runtime::Handle::block_on을 호출한다면 CONC-027로 이어짐
            compute_vmaf_for_frame(f)
        }).collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())
}
```

**문제**:
- Tokio worker thread pool과 Rayon의 전역 thread pool이 각각 독립적으로 논리 CPU 수만큼 스레드를 만들면, 하나의 프로세스 안에 CPU 코어 수의 2배 이상 되는 스레드가 경쟁하게 되어 컨텍스트 스위칭 비용이 급증한다.
- `spawn_blocking` 안에서 Rayon을 호출하는 것 자체는 안전하지만, 그 반대로 Rayon 클로저 안에서 다시 async 작업을 `block_on`으로 기다리면 데드락 위험이 생긴다(CONC-027 참고).
- 여러 Tauri 커맨드가 동시에 각자 `par_iter`를 호출하면 Rayon 전역 pool 안에서 job이 경쟁하며 우선순위 제어가 불가능해진다.

**발생 조건**:
- 메트릭 계산(SSIM/VMAF/PSNR)이 프레임 단위로 Rayon 병렬화되어 있고, 동시에 여러 비디오/여러 패널에서 계산 요청이 들어올 때.
- 재생 중(디코드가 Tokio 쪽에서 진행 중)에 메트릭 배치 계산(Rayon)이 동시에 시작될 때.

**권장**:
```rust
// 전역 Rayon pool 크기를 명시적으로 제한하고, Tokio worker 수와의 합을 예산 안에 둔다.
static METRICS_POOL: once_cell::sync::Lazy<rayon::ThreadPool> = once_cell::sync::Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads((num_cpus::get() / 2).max(1))
        .build()
        .expect("failed to build metrics thread pool")
});

async fn compute_all_metrics(frames: Arc<Vec<Frame>>) -> Vec<MetricResult> {
    tokio::task::spawn_blocking(move || {
        METRICS_POOL.install(|| frames.par_iter().map(compute_vmaf_for_frame).collect())
    })
    .await
    .expect("metrics task panicked")
}
```
- Tokio worker thread 수, Rayon pool 크기, 디코더 내부 스레드 수를 합산해 전체 스레드 예산을 설계 단계에서 명시적으로 정한다(CONC-004, CONC-025 참고).
- Rayon 클로저 안에서 async runtime API(`block_on` 등)를 호출하지 않는다.

**탐지 방법**:
- Structural: `spawn_blocking` 클로저 내부에 `rayon::` 심볼이 등장하는지, 그 반대로 `par_iter`/`rayon::scope` 클로저 안에 `block_on`이 등장하는지 정적 검사.
- Runtime: `htop`/`sysinfo`로 프로세스의 총 스레드 수를 관찰하며 CPU 코어 수 대비 과다 여부 확인.

**예외**:
- Rayon pool 크기를 아주 작게(예: 2) 고정하고 사용 빈도가 낮다면 실질적 경쟁은 미미할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-004: decoder 내부 thread와 외부 thread pool 중첩
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// dav1d는 내부적으로 max_frame_delay만큼 스레드를 생성한다
let settings = Dav1dSettings {
    n_threads: num_cpus::get() as i32, // 예: 16
    ..Default::default()
};
let decoder = Dav1dDecoder::new(&settings)?;

// 동시에 애플리케이션도 프레임별 병렬 디코드를 시도
frame_indices.par_iter().for_each(|&idx| {
    decoder.decode_frame_sync(idx).unwrap(); // decoder 인스턴스마다 또 16 스레드
});
```

**문제**:
- dav1d/libvmaf 같은 FFI 라이브러리는 `n_threads` 설정으로 자체 내부 스레드풀을 생성한다. 이를 고려하지 않고 애플리케이션 레벨에서 또 다른 병렬 계층(Rayon `par_iter`)을 얹으면 스레드 수가 곱셈으로 폭증한다(`디코더 인스턴스 수 × n_threads`).
- 코어 수보다 훨씬 많은 스레드가 동시에 CPU를 다투면서 캐시 지역성이 깨지고, 오히려 단일 스레드 디코드보다 느려지는 역설적 상황이 발생할 수 있다.
- 디코더 내부 스레드가 이미 프레임 레벨/타일 레벨 병렬성을 활용하므로, 외부에서 프레임 단위로 또 병렬화하는 것은 대개 중복 투자다.

**발생 조건**:
- 여러 개의 디코더 인스턴스(비디오 A/B 비교 모드처럼)를 동시에 열고, 각각 다중 스레드로 설정했을 때.
- Filmstrip 썸네일 생성처럼 "여러 프레임을 동시에 디코드"하려는 시도가 디코더 내부 병렬성과 충돌할 때.

**권장**:
```rust
// 디코더 내부 병렬성만 사용하고, 외부에서는 순차적으로 요청을 넘긴다.
let settings = Dav1dSettings {
    n_threads: (num_cpus::get() / active_decoder_instances).max(1) as i32,
    ..Default::default()
};

// 여러 프레임 요청은 하나의 decode worker 스레드가 순차 처리(디코더가 내부적으로 병렬화)
for idx in frame_indices {
    let pic = decoder.decode_frame_sync(idx)?;
    tx.send(pic)?;
}
```
- 디코더 인스턴스별 `n_threads`를 "전체 예산 ÷ 동시에 열려 있는 디코더 인스턴스 수"로 명시적으로 나눈다.
- 프레임 단위 외부 병렬화는 디코더가 스스로 내부 병렬화를 제공하지 않는 경우(예: 단일 스레드 빌드된 라이브러리)에만 적용한다.

**탐지 방법**:
- Structural: 디코더 생성 코드에서 `n_threads`/`max_frame_delay` 등의 설정값과, 그 디코더를 감싸는 외부 `par_iter`/`spawn` 호출이 같은 모듈 안에 공존하는지 검사.
- Runtime: 디코드 벤치마크에서 코어 수를 늘려도 처리량이 개선되지 않거나 오히려 감소하는지 확인(오버서브스크립션 신호).

**예외**:
- 디코더가 단일 스레드 모드(`n_threads=1`)로 고정되어 있고, 외부 병렬화가 유일한 병렬성 원천이라면 문제가 없다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-005: unbounded channel
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<FrameEvent>();

// 디코더 worker: 소비 속도와 무관하게 계속 push
tokio::spawn(async move {
    loop {
        let frame = decode_next_frame().await;
        tx.send(FrameEvent::Decoded(frame)).ok(); // 절대 실패하지 않음, backpressure 없음
    }
});

// UI 이벤트 emit 쪽 소비자가 느리면 큐가 무한정 쌓인다
while let Some(event) = rx.recv().await {
    app_handle.emit_all("frame-decoded", &event).ok();
}
```

**문제**:
- `unbounded_channel`은 송신자가 수신자의 처리 속도를 기다리지 않으므로, 생산자가 소비자보다 빠르면 큐 안에 메시지(여기서는 디코드된 프레임 버퍼, 즉 큰 메모리 블록)가 무한히 쌓인다.
- 프레임 버퍼처럼 크기가 큰 데이터를 담는 채널에서는 수백 개만 쌓여도 수백 MB~GB 단위 메모리를 소모해 OOM으로 이어질 수 있다.
- Backpressure가 없으면 시스템 전체의 "가장 느린 소비자"가 병목이라는 신호가 생산자에게 전달되지 않아, 문제를 진단하기도 어렵다.

**발생 조건**:
- 재생 속도(디코드)가 렌더링/emit 속도보다 빠른 상황(고프레임레이트 비디오, 저사양 UI 스레드).
- 사용자가 빠르게 스크러빙해 다수의 decode 요청이 짧은 시간에 몰릴 때.

**권장**:
```rust
let (tx, mut rx) = tokio::sync::mpsc::channel::<FrameEvent>(16); // bounded

tokio::spawn(async move {
    loop {
        let frame = decode_next_frame().await;
        // 큐가 가득 차면 여기서 자연스럽게 대기(backpressure)
        if tx.send(FrameEvent::Decoded(frame)).await.is_err() {
            break; // 수신자가 사라짐 → 루프 종료
        }
    }
});
```
- 채널 용량을 명시적으로 정하고(`channel(N)`), 큐가 가득 찼을 때의 정책(대기 vs. drop-oldest vs. coalesce)을 의도적으로 선택한다.
- 프레임처럼 최신 값만 의미 있는 데이터는 채널 대신 `tokio::sync::watch`를 사용하는 편이 더 적합할 수 있다(CONC-006, CONC-007 참고).

**탐지 방법**:
- Static/Structural: 코드베이스에서 `unbounded_channel(` 사용처를 전수 검사하고, 각각에 대해 생산/소비 속도 불균형 가능성을 리뷰.
- Runtime: 채널 길이를 주기적으로 샘플링하는 메트릭을 추가해 큐 깊이가 계속 증가하는지 관찰.

**예외**:
- 메시지가 극히 작고(수 바이트) 발생 빈도가 낮음이 보장되는 제어 채널(예: 앱 종료 신호)이라면 unbounded도 실용적으로 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-006: 모든 이벤트에 FIFO 정책 사용
**분류**: CONC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
enum WorkItem {
    Scrub(u64),
    MetricsComplete(MetricResult),
    ExportProgress(f32),
}

let (tx, mut rx) = tokio::sync::mpsc::channel::<WorkItem>(256);

// 단일 FIFO 큐 — scrub 이벤트도, 배치 metrics 완료 이벤트도 순서대로만 처리
while let Some(item) = rx.recv().await {
    handle(item).await;
}
```

**문제**:
- 사용자가 스크러버를 빠르게 움직이며 발생시킨 `Scrub` 이벤트가 앞서 큐에 쌓인 `MetricsComplete`/`ExportProgress` 항목들 뒤에서 대기하게 되어, 사용자 입력에 대한 체감 반응성이 나빠진다.
- 모든 이벤트를 동일한 우선순위로 취급하면 "지금 화면에 보여줘야 하는 것"과 "나중에 처리해도 되는 것"을 구분하지 못한다.
- FIFO는 최신성이 중요한 이벤트(스크럽 위치)에도 오래된 항목을 그대로 처리하게 만들어 불필요한 작업(CONC-021의 stale 결과 문제)으로 이어지기 쉽다.

**발생 조건**:
- 하나의 큐/워커에 사용자 상호작용 이벤트와 백그라운드 배치 작업 이벤트가 섞여 들어올 때.
- Export나 메트릭 계산처럼 오래 걸리는 작업이 다수의 진행 이벤트를 생성하며 큐를 채울 때.

**권장**:
```rust
struct PriorityQueues {
    interactive: tokio::sync::mpsc::Sender<WorkItem>, // 우선 처리
    background: tokio::sync::mpsc::Sender<WorkItem>,
}

async fn dispatcher(mut hi: mpsc::Receiver<WorkItem>, mut lo: mpsc::Receiver<WorkItem>) {
    loop {
        tokio::select! {
            biased; // interactive를 우선 폴링
            Some(item) = hi.recv() => handle(item).await,
            Some(item) = lo.recv() => handle(item).await,
            else => break,
        }
    }
}
```
- 최소 2단계(interactive / background) 우선순위 큐로 분리하고, `select! { biased; ... }` 또는 우선순위 기반 스케줄러로 interactive 이벤트를 먼저 처리한다.
- 배경 작업 이벤트는 필요하면 CONC-010처럼 coalescing까지 함께 적용한다.

**탐지 방법**:
- Semantic: 이벤트 enum의 variant들을 "사용자 상호작용 결과물"과 "백그라운드 산출물"로 분류해보고, 하나의 채널/큐에 섞여 있는지 리뷰로 판단.
- Runtime: 스크럽 조작 중 UI 반응 지연을 측정해 백그라운드 작업 유무에 따른 편차 확인.

**예외**:
- 이벤트 종류가 하나뿐이거나, 모든 이벤트가 실제로 동일한 긴급도를 갖는 단순한 도구라면 우선순위 분리가 과설계일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-007: frame scrub 요청을 전부 처리
**분류**: CONC · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```typescript
// frontend: 슬라이더 onChange마다 매번 Tauri 커맨드 호출
function onScrub(frameIdx: number) {
  invoke('decode_frame', { frameIdx }); // debounce/throttle 없음
}
```
```rust
#[tauri::command]
async fn decode_frame(state: tauri::State<'_, AppState>, frame_idx: u64) -> Result<FrameBuf, String> {
    // 사용자가 100프레임을 드래그하면 100번의 decode_frame이 각각 끝까지 실행됨
    do_decode(frame_idx).await
}
```

**문제**:
- 사용자가 스크러버를 빠르게 드래그하면 초당 수십 번의 `decode_frame` 호출이 발생하고, 각 요청이 모두 끝까지 처리되면 실제로 화면에 보여야 할 "최종 정지 위치"보다 훨씬 뒤처진 프레임들이 계속 렌더링된다.
- 이미 지나간(사용자가 더 이상 보고 있지 않은) 프레임을 디코드하는 데 CPU 자원을 낭비하면서, 정작 최신 요청의 응답은 늦어진다.
- CONC-021(오래된 결과 적용)과 결합되면 최종적으로 화면에 잘못된 프레임이 표시될 위험도 있다.

**발생 조건**:
- 타임라인/필름스트립에서 빠른 마우스 드래그 스크러빙.
- 디코드 비용이 큰 코덱(고해상도 HEVC/AV1)일수록 체감 지연이 커진다.

**권장**:
```rust
// 최신 요청만 의미 있는 경우 watch 채널로 "최신값만 유지"
let (scrub_tx, mut scrub_rx) = tokio::sync::watch::channel::<u64>(0);

tokio::spawn(async move {
    loop {
        scrub_rx.changed().await.ok();
        let frame_idx = *scrub_rx.borrow();
        // 처리 중 새 값이 또 들어오면 다음 루프에서 최신값으로 자연스럽게 대체됨
        let _ = do_decode(frame_idx).await;
    }
});

#[tauri::command]
fn request_scrub(state: tauri::State<'_, AppState>, frame_idx: u64) {
    let _ = state.scrub_tx.send(frame_idx); // 이전 값을 덮어씀
}
```
- "최신 값만 중요한" 요청은 `watch` 채널이나 명시적 debounce/coalesce 레이어로 처리해 중간 요청을 자연스럽게 스킵한다.
- 프론트엔드에서도 debounce/throttle(예: `requestAnimationFrame` 단위로 1회만 invoke)을 적용해 IPC 호출 자체를 줄인다.

**탐지 방법**:
- Runtime: 스크럽 조작 시 백엔드 `decode_frame` 호출 횟수와 실제 최종 표시된 프레임 수를 비교(비율이 크면 낭비 신호).
- Manual: 프론트엔드 이벤트 핸들러에 debounce/throttle 유틸이 적용되어 있는지 리뷰.

**예외**:
- 정지 이미지 미세 탐색(방향키로 1프레임씩 이동)처럼 요청 빈도가 낮고 각 요청이 실제로 다 필요한 경우는 예외.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-008: 취소 토큰 없는 장시간 작업
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
async fn compute_vmaf_full_video(state: tauri::State<'_, AppState>) -> Result<Vec<f64>, String> {
    let frame_count = state.frame_count();
    let mut results = Vec::with_capacity(frame_count as usize);
    for idx in 0..frame_count {
        // 취소할 방법이 없음 — 사용자가 다른 비디오로 전환해도 끝까지 실행됨
        results.push(compute_vmaf_for_frame(idx).await?);
    }
    Ok(results)
}
```

**문제**:
- 수천 프레임짜리 비디오의 VMAF 전체 계산은 수 분이 걸릴 수 있는데, 취소 메커니즘이 없으면 사용자가 다른 비디오를 열거나 앱을 닫으려 해도 작업이 끝날 때까지 리소스(CPU, 디코더 세션)를 계속 점유한다.
- 이미 필요 없어진 작업이 유효한 작업과 CPU/스레드 풀 자원을 경쟁하면서 새 작업의 응답성까지 떨어뜨린다.
- 사용자가 "멈춰있다"고 인식하고 강제 종료할 경우 파일 핸들, 임시 파일 등이 정리되지 않을 위험도 있다.

**발생 조건**:
- 사용자가 메트릭 계산 도중 다른 비디오/다른 스트림으로 전환.
- Export/배치 분석처럼 명시적으로 "중단" 버튼이 UI에 있어야 하는 장시간 작업.

**권장**:
```rust
#[tauri::command]
async fn compute_vmaf_full_video(
    state: tauri::State<'_, AppState>,
    cancel: tauri::State<'_, CancellationToken>,
) -> Result<Vec<f64>, String> {
    let frame_count = state.frame_count();
    let mut results = Vec::with_capacity(frame_count as usize);
    for idx in 0..frame_count {
        if cancel.is_cancelled() {
            return Err("cancelled".into());
        }
        results.push(compute_vmaf_for_frame(idx).await?);
    }
    Ok(results)
}

#[tauri::command]
fn cancel_metrics(cancel: tauri::State<'_, CancellationToken>) {
    cancel.cancel();
}
```
- `tokio_util::sync::CancellationToken`(또는 자체 `AtomicBool`/generation ID)을 모든 장시간 작업 시작 시 발급하고, 루프의 매 반복마다 확인한다.
- 새 작업을 시작할 때 이전 동일 범주 작업의 토큰을 취소하는 정책(최신 요청 우선)을 함께 적용한다.

**탐지 방법**:
- Structural: `for`/`while` 루프가 여러 반복에 걸쳐 `.await`를 포함하면서 `CancellationToken`/`AtomicBool` 참조가 전혀 없는 async 함수를 검사.
- Manual: "중단" UI가 있는 기능마다 대응하는 백엔드 취소 로직이 실제로 존재하는지 매핑.

**예외**:
- 작업이 항상 수십 ms 이내로 끝난다고 보장되면(단일 프레임 디코드 등) 취소 토큰 없이도 실질적 문제가 없다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-009: 취소했지만 내부 loop가 확인하지 않음
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
async fn compute_vmaf_full_video(cancel: CancellationToken, frames: Vec<Frame>) -> Vec<f64> {
    frames.iter().map(|f| {
        // cancel 토큰이 함수에 전달되지 않음 — 취소돼도 이 클로저 내부에서는 알 수 없음
        compute_vmaf_for_frame_sync(f)
    }).collect()
}
```

**문제**:
- 상위 레벨에 `CancellationToken`이 존재하더라도, 실제 무거운 작업을 수행하는 내부 루프(특히 Rayon `par_iter`나 FFI 호출 루프)까지 토큰이 전달되지 않으면 취소 요청이 아무 효과가 없다.
- "취소 버튼을 눌렀는데 작업이 안 멈춘다"는 사용자 경험 문제로 이어지며, CONC-008에서 만든 인프라가 무의미해진다.
- 특히 Rayon `par_iter` 내부처럼 클로저가 외부 상태를 캡처하지 않고 순수 함수로 작성된 경우 토큰 전달이 누락되기 쉽다.

**발생 조건**:
- 취소 가능한 최상위 async 함수 안에서 실제 반복 작업이 별도 헬퍼 함수/Rayon 클로저로 위임될 때, 리팩터링 과정에서 토큰 전달이 빠질 때.
- 다단계 파이프라인(decode → filter → metric)에서 일부 단계만 토큰을 확인할 때.

**권장**:
```rust
async fn compute_vmaf_full_video(cancel: CancellationToken, frames: Vec<Frame>) -> Result<Vec<f64>, Cancelled> {
    tokio::task::spawn_blocking(move || {
        frames.par_iter()
            .map(|f| {
                if cancel.is_cancelled() {
                    return Err(Cancelled);
                }
                Ok(compute_vmaf_for_frame_sync(f))
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .unwrap()
}
```
- 취소 토큰은 함수 시그니처를 통해 실제 작업 최소 단위(프레임 하나 처리)까지 명시적으로 전달한다.
- Rayon처럼 병렬 반복 내부에서도 각 iteration 시작 시 토큰을 확인해 조기 종료(`try_fold`/`Result` 조합)한다.

**탐지 방법**:
- Structural: 취소 가능하다고 표시된(파라미터로 토큰을 받는) 함수가 내부에서 호출하는 헬퍼 함수들에도 토큰이 실제로 전달되는지 호출 그래프를 추적.
- Runtime: 취소 버튼을 누른 뒤 실제로 작업이 멈추기까지 걸리는 시간을 측정 — 예상보다 훨씬 길면 토큰 미확인 지점이 있다는 신호.

**예외**:
- 작업 단위가 이미 매우 작아(수 ms) 상위 레벨 취소만으로도 체감 지연이 무시할 수준이라면 내부 루프까지 토큰을 전달하지 않아도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-010: progress event 과다 전송
**분류**: CONC · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
for (idx, frame) in frames.iter().enumerate() {
    let result = compute_vmaf_for_frame(frame).await?;
    // 프레임마다(초당 수백~수천 회) Tauri 이벤트 emit
    app_handle.emit_all("metrics-progress", ProgressPayload {
        current: idx as u64,
        total: frames.len() as u64,
    }).ok();
}
```

**문제**:
- Tauri의 `emit_all`은 IPC를 통해 webview로 메시지를 전달하는데, 프레임 단위(초당 수백~수천 건)로 이벤트를 보내면 IPC 직렬화 비용과 webview 쪽 리스너의 렌더링 비용이 누적되어 오히려 전체 작업을 느리게 만들 수 있다.
- 프론트엔드가 매 이벤트마다 React 상태 업데이트/리렌더를 트리거하면 UI 자체가 버벅이는 역효과가 발생한다.
- 사용자에게는 어차피 초당 수 회 이상의 진행률 갱신은 시각적으로 의미가 없다.

**발생 조건**:
- 프레임 수가 많은 배치 메트릭 계산(전체 비디오 VMAF/SSIM).
- 진행률 콜백이 최적화 없이 계산 루프 안에 그대로 삽입되어 있을 때.

**권장**:
```rust
let mut last_emit = std::time::Instant::now();
for (idx, frame) in frames.iter().enumerate() {
    let result = compute_vmaf_for_frame(frame).await?;
    if last_emit.elapsed() >= std::time::Duration::from_millis(100) || idx == frames.len() - 1 {
        app_handle.emit_all("metrics-progress", ProgressPayload {
            current: idx as u64,
            total: frames.len() as u64,
        }).ok();
        last_emit = std::time::Instant::now();
    }
}
```
- 진행률 이벤트는 시간 기반(예: 100ms마다) 또는 퍼센트 기반(1% 단위) throttle을 적용해 전송 빈도를 제한한다.
- 마지막 이벤트(100% 완료)는 throttle과 무관하게 반드시 전송해 UI가 "멈춘 것처럼" 보이지 않게 한다.

**탐지 방법**:
- Runtime: 장시간 작업 중 `emit_all`/`emit` 호출 횟수를 계측해 초당 발생량이 비정상적으로 높은지 확인.
- Static: 루프 본문에서 `emit`/`emit_all` 호출에 throttle 가드(시간/카운터 조건)가 없는 패턴을 검사.

**예외**:
- 전체 작업이 짧아(예: 총 프레임 수 20개 이하) 이벤트 총량이 애초에 적다면 throttle이 불필요할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-011: 거대 Arc<Mutex<AppState>>
**분류**: CONC · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
struct AppState {
    current_video: Option<VideoHandle>,
    decoder: Option<DecoderHandle>,
    current_frame_idx: u64,
    metrics_cache: HashMap<u64, MetricResult>,
    ui_prefs: UiPreferences,
    export_jobs: Vec<ExportJob>,
}

// 앱 전체가 이 하나의 락을 공유
type SharedState = Arc<Mutex<AppState>>;

#[tauri::command]
async fn get_ui_prefs(state: tauri::State<'_, SharedState>) -> Result<UiPreferences, String> {
    let guard = state.lock().unwrap(); // decode_frame이 락을 쥐고 있으면 이 호출도 대기
    Ok(guard.ui_prefs.clone())
}
```

**문제**:
- 서로 관련 없는 데이터(디코더 상태, UI 설정, 메트릭 캐시, export 작업 목록)가 하나의 `Mutex`로 묶이면, 논리적으로 독립적인 작업들이 락 경쟁 때문에 인위적으로 직렬화된다.
- UI 설정 조회처럼 즉시 끝나야 할 가벼운 커맨드가, 디코드처럼 오래 걸리는 작업이 락을 쥔 동안 대기하게 된다.
- 상태가 커질수록 "이 필드는 어떤 락 규칙으로 보호되는가"를 파악하기 어려워지고, 락 순서 실수로 인한 데드락 위험도 커진다.

**발생 조건**:
- 프로젝트 초기에 상태 구조를 단순화하려고 모든 것을 한 구조체에 넣고 시작한 뒤, 기능이 늘어나면서 필드가 계속 추가될 때.
- 여러 Tauri 커맨드가 각기 다른 필드만 필요로 함에도 전체 상태를 잠글 때.

**권장**:
```rust
struct AppState {
    decoder: Arc<Mutex<Option<DecoderHandle>>>,
    playback: Arc<Mutex<PlaybackState>>, // current_video, current_frame_idx
    metrics_cache: Arc<RwLock<HashMap<u64, MetricResult>>>,
    ui_prefs: Arc<RwLock<UiPreferences>>,
    export_jobs: Arc<Mutex<Vec<ExportJob>>>,
}

#[tauri::command]
async fn get_ui_prefs(state: tauri::State<'_, AppState>) -> Result<UiPreferences, String> {
    let guard = state.ui_prefs.read().unwrap(); // decoder 락과 완전히 무관
    Ok(guard.clone())
}
```
- 상태를 도메인별로 분리하고, 각 부분에 독립적인 락(또는 락이 필요 없는 불변 값)을 부여한다.
- "자주 읽고 드물게 쓰는" 데이터는 `RwLock`으로, "짧고 자주 갱신되는" 데이터는 `Mutex` 또는 atomics로 구분한다.

**탐지 방법**:
- Structural: `struct AppState` 같은 최상위 상태 구조체의 필드 수와, 그 구조체를 감싸는 락의 개수(1개인지)를 검사. 필드가 5개 이상이고 락이 1개뿐이면 경고.
- Manual: 각 Tauri 커맨드가 실제로 접근하는 필드 목록을 표로 정리해, 서로 겹치지 않는 커맨드들이 같은 락을 공유하는지 확인.

**예외**:
- 애플리케이션 규모가 작고 상태 필드가 소수(2~3개)이며 접근 패턴이 거의 항상 전체 상태를 함께 다뤄야 한다면 단일 Mutex도 실용적일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-012: lock을 잡은 채 I/O
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
async fn export_frame_png(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let guard = state.lock().unwrap();
    let frame = guard.current_frame.clone();
    // 락을 쥔 채로 디스크 I/O 수행 — 이 시간 동안 다른 모든 커맨드가 state 접근 불가
    std::fs::write(&path, encode_png(&frame)).map_err(|e| e.to_string())?;
    Ok(())
}
```

**문제**:
- 디스크 쓰기는 밀리초~수십 밀리초가 걸릴 수 있는 상대적으로 느린 작업인데, 그 시간 동안 애플리케이션 전역 상태 락을 쥐고 있으면 다른 모든 상태 접근(현재 프레임 조회, 재생 위치 갱신 등)이 그동안 완전히 막힌다.
- 네트워크 I/O나 느린 디스크(외장 HDD, 네트워크 드라이브)에서는 이 지연이 수 초 단위로 늘어날 수 있어 체감 문제가 커진다.
- 락이 보호해야 할 것은 "공유 데이터의 일관성"이지 "I/O 작업 자체"가 아니라는 원칙이 깨진 사례다.

**발생 조건**:
- Export, 스크린샷 저장, 로그 기록 등 락으로 보호된 데이터를 읽은 뒤 즉시 파일에 쓰는 패턴.
- 네트워크 드라이브나 외장 저장 장치처럼 I/O 지연이 큰 환경.

**권장**:
```rust
#[tauri::command]
async fn export_frame_png(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let frame = {
        let guard = state.lock().unwrap();
        guard.current_frame.clone() // 필요한 데이터만 복제하고 즉시 락 해제
    };
    // 락 밖에서 I/O 수행
    tokio::task::spawn_blocking(move || std::fs::write(&path, encode_png(&frame)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
```
- 락을 쥐는 범위는 "공유 데이터를 읽거나 쓰는 최소 구간"으로 한정하고, 그 데이터를 복제/추출한 뒤 즉시 락을 해제한다.
- I/O는 락 밖에서, 필요하면 `spawn_blocking`으로 수행한다.

**탐지 방법**:
- Static/Structural: `lock()`/`.lock().await` 이후 `.unlock()` 또는 스코프 종료 전에 `std::fs::*`, `reqwest::*`, `tokio::fs::*` 등 I/O 호출이 등장하는 패턴을 검사(clippy에는 전용 lint가 없어 커스텀 스크립트 또는 세미그렙 규칙 필요).
- Manual: 락 스코프 안의 코드 줄 수와 호출 목록을 리뷰해 I/O성 호출이 섞여 있는지 확인.

**예외**:
- 메모리 매핑된 파일(`mmap`)에 대한 쓰기처럼 사실상 메모리 연산에 가깝고 지연이 무시할 수준이라면 예외로 볼 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-013: lock을 잡은 채 decode
**분류**: CONC · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
async fn decode_and_cache_frame(state: tauri::State<'_, AppState>, idx: u64) -> Result<FrameBuf, String> {
    let mut guard = state.lock().unwrap();
    // decode_frame_sync가 수십 ms 걸리는 동안 애플리케이션 전역 상태가 잠김
    let frame = guard.decoder.decode_frame_sync(idx).map_err(|e| e.to_string())?;
    guard.frame_cache.insert(idx, frame.clone());
    Ok(frame)
}
```

**문제**:
- CONC-012와 동일한 근본 원인이지만, 디코드는 I/O보다도 CPU 비용이 커서(수십 ms) 락 보유 시간이 더 길어지고 영향 범위도 크다.
- 이 패턴이 있으면 사실상 애플리케이션 전체가 "한 번에 하나의 프레임만 디코드 가능"한 구조가 되어, CONC-011의 거대 락 문제와 결합해 병렬성이 완전히 사라진다.
- 재생 중이라면 매 프레임마다 이 락이 전역 상태 접근을 막으므로, 재생과 무관한 UI 조작(설정 변경 등)까지 끊기게 된다.

**발생 조건**:
- `AppState`에 디코더와 캐시가 함께 들어있고, 두 필드를 "원자적으로" 갱신하려는 의도로 락을 필요 이상 넓게 잡을 때.
- 재생 루프처럼 반복적으로 디코드가 발생하는 상황.

**권장**:
```rust
#[tauri::command]
async fn decode_and_cache_frame(state: tauri::State<'_, AppState>, idx: u64) -> Result<FrameBuf, String> {
    // 디코더 자체는 별도 락(혹은 전용 worker) — 캐시 락과 분리
    let decoder = state.decoder.clone();
    let frame = tokio::task::spawn_blocking(move || decoder.decode_frame_sync(idx))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    // 캐시에 넣는 순간만 짧게 락
    state.frame_cache.lock().unwrap().insert(idx, frame.clone());
    Ok(frame)
}
```
- 디코드는 락 밖에서 `spawn_blocking`(CONC-002 참고)으로 수행하고, 그 결과만 짧게 락을 잡아 캐시에 반영한다.
- 디코더 자체에 대한 동시 접근 제어가 필요하다면, 범용 상태 락이 아니라 디코더 전용 락/전용 워커로 분리한다(CONC-011).

**탐지 방법**:
- Structural: 락 스코프(`lock()` ~ 스코프 종료) 안에 FFI 디코드 호출 또는 `*_sync` 함수 호출이 포함되는지 검사.
- Runtime: 재생 중 다른 UI 조작(설정 토글 등)의 응답 지연을 측정해, 프레임 디코드 주기와 상관관계가 있는지 확인.

**예외**:
- 디코더가 원천적으로 단일 스레드에서만 호출 가능하고 이미 그 사실을 명확히 문서화·격리했다면, "직렬화"는 버그가 아니라 설계다. 다만 이 경우에도 전역 앱 상태 락과는 분리되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-014: await 중 MutexGuard 유지
**분류**: CONC · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
async fn update_progress_and_notify(state: &Arc<tokio::sync::Mutex<AppState>>, idx: u64) {
    let mut guard = state.lock().await;
    guard.progress = idx;
    // 락을 쥔 채로 다른 async 작업을 기다림 — 그동안 이 Mutex는 계속 잠겨 있음
    notify_frontend(idx).await;
    guard.last_notified = idx;
}
```

**문제**:
- `tokio::sync::Mutex`의 `MutexGuard`를 쥔 채로 다른 `.await` 지점을(특히 IPC emit처럼 지연이 있을 수 있는 작업을) 통과하면, 그 `.await`가 완료될 때까지 락이 계속 유지된다.
- 이 시간 동안 같은 Mutex를 기다리는 다른 task는 전부 대기해야 하며, `notify_frontend`가 느려지면(예: webview가 바쁨) 대기 시간이 예측 불가능하게 늘어난다.
- 더 나쁜 경우, `notify_frontend` 내부가 다시 같은 락을 요청하는 코드 경로를 가지면 데드락으로 이어질 수 있다.

**발생 조건**:
- 상태 갱신과 알림(이벤트 emit, 채널 send)을 하나의 함수에서 순차적으로 처리하며 그 사이에 락을 해제하지 않을 때.
- 리팩터링 중 원래 동기적이던 코드에 `.await`가 추가되었지만 락 해제 시점을 재검토하지 않았을 때.

**권장**:
```rust
async fn update_progress_and_notify(state: &Arc<tokio::sync::Mutex<AppState>>, idx: u64) {
    {
        let mut guard = state.lock().await;
        guard.progress = idx; // 락은 여기까지만
    } // 스코프 종료 시 자동 해제

    notify_frontend(idx).await; // 락 밖에서 await

    let mut guard = state.lock().await;
    guard.last_notified = idx;
}
```
- 락을 쥐는 구간과 `.await`가 필요한 구간을 명확히 분리해, 락 스코프 안에는 동기적 연산만 남긴다.
- `clippy::await_holding_lock`(std::sync 대상) 및 유사 검사를 CI에 활성화하고, `tokio::sync::Mutex`에 대해서도 코드 리뷰 체크리스트로 동일 원칙을 적용한다.

**탐지 방법**:
- Static: `clippy::await_holding_lock` 활성화(주로 `std::sync::MutexGuard` 대상이며, `tokio::sync::Mutex`는 컴파일 에러로 잡히지 않으므로 별도 리뷰 필요).
- Manual: 락 스코프 안에 `.await` 키워드가 등장하는 모든 위치를 grep해 각각 정말 필요한지 검토.

**예외**:
- 없음에 가깝다 — 락을 쥔 채 await하는 것이 의도적으로 안전한 경우는 극히 드물며(예: 그 자체가 유일한 동시 접근자임이 구조적으로 보장될 때), 대부분은 리팩터링 대상이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-015: std::sync::Mutex와 async 혼용 오류
**분류**: CONC · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
struct AppState {
    cache: std::sync::Mutex<HashMap<u64, FrameBuf>>,
}

async fn get_or_decode(state: Arc<AppState>, idx: u64) -> FrameBuf {
    let mut guard = state.cache.lock().unwrap();
    if let Some(f) = guard.get(&idx) {
        return f.clone();
    }
    // std::sync::MutexGuard(Send 아님)를 쥔 채 await — 컴파일 에러 또는(운 나쁘면)
    // 우회 코드로 통과시켜 놓아 blocking-in-async 데드락 위험을 남김
    let frame = decode_frame_async(idx).await;
    guard.insert(idx, frame.clone());
    frame
}
```

**문제**:
- `std::sync::MutexGuard`는 `Send`가 아니므로, 이를 `.await` 지점 너머로 들고 가는 `async fn`은 대부분 컴파일되지 않는다. 하지만 개발자가 이를 억지로 우회(예: 락 해제/재획득을 잘못 배치, `unsafe`, 별도 스레드로 분리 등)하면 미묘한 버그로 이어진다.
- 컴파일이 통과하는 경우에도(예: `.await` 없이 잠깐 락을 잡는 것처럼 보이지만 실제로는 blocking 호출이 내부에 숨어 있는 경우) `std::sync::Mutex::lock()`은 blocking 호출이라, async 컨텍스트에서 잘못 사용하면 그 스레드의 다른 task 스케줄링을 막는다.
- `std::sync::Mutex`와 `tokio::sync::Mutex`를 같은 코드베이스에서 상황별로 섞어 쓰면, "이 락은 await 가능한가"를 매번 확인해야 하는 인지 부담이 커진다.

**발생 조건**:
- 원래 동기 코드였던 모듈을 async로 마이그레이션하면서 락 타입을 `tokio::sync::Mutex`로 바꾸는 것을 누락했을 때.
- 짧은 임계 구역이라는 이유로 `std::sync::Mutex`를 async 함수 안에서 사용하기로 "의도적으로" 선택했지만, 이후 그 임계 구역 안에 `.await`가 추가되었을 때.

**권장**:
```rust
struct AppState {
    // 임계 구역이 항상 동기적(빠른 CPU 연산)이라면 std::sync::Mutex 유지 + await 절대 금지
    cache: std::sync::Mutex<HashMap<u64, FrameBuf>>,
}

async fn get_or_decode(state: Arc<AppState>, idx: u64) -> FrameBuf {
    // 1) 락은 짧게, await 없이
    if let Some(f) = state.cache.lock().unwrap().get(&idx) {
        return f.clone();
    }
    // 2) await는 락 밖에서
    let frame = decode_frame_async(idx).await;
    // 3) 다시 짧게 락
    state.cache.lock().unwrap().insert(idx, frame.clone());
    frame
}
```
- 원칙을 명확히 한다: 임계 구역 안에 `.await`가 필요 없다면 `std::sync::Mutex`(더 가볍고 빠름)를 쓰고, 필요하다면 `tokio::sync::Mutex`(또는 락 자체를 없애는 설계)를 쓴다. 이 둘을 상황에 따라 바꿔가며 섞지 않는다.
- 팀 컨벤션 문서에 "이 상태는 std::sync, 저 상태는 tokio::sync"를 명시하고 타입 별칭으로 강제한다.

**탐지 방법**:
- Static: 컴파일러 자체가 `Send` 위반은 대부분 잡아주지만, `#[allow(...)]`로 우회된 코드나 `unsafe impl Send`가 붙은 래퍼는 별도로 grep.
- Structural: `std::sync::Mutex`/`std::sync::RwLock` 임포트가 `async fn`이 있는 모듈에 함께 존재하는 경우를 목록화해 리뷰.

**예외**:
- 임계 구역이 항상 순수 동기 연산이고 `.await`가 원천적으로 들어갈 수 없음이 타입 시스템으로 보장된다면(위 권장 예처럼), `std::sync::Mutex`를 async 코드베이스에서 쓰는 것 자체는 정상적이고 오히려 권장된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-016: RwLock이면 무조건 빠르다고 가정
**분류**: CONC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
struct PlaybackState {
    current_frame_idx: RwLock<u64>, // "읽기가 많으니 RwLock이 빠를 것"이라는 가정만으로 선택
}

// 재생 루프: 초당 30~60회 write
tokio::spawn(async move {
    loop {
        *playback.current_frame_idx.write().unwrap() += 1;
        tokio::time::sleep(frame_interval).await;
    }
});

// UI 폴링: 초당 수백 회 read (여러 패널이 각각 폴링)
for panel in panels {
    let idx = *playback.current_frame_idx.read().unwrap();
    panel.update(idx);
}
```

**문제**:
- `RwLock`은 "읽기가 압도적으로 많고 쓰기가 드문" 워크로드에서 유리하다는 것이 일반적 통념이지만, 실제로는 구현에 따라 writer starvation(쓰기 요청이 계속 밀리는 락 알고리즘)이 발생할 수 있고, 읽기/쓰기 전환 비용이 단순 `Mutex`보다 오히려 클 수도 있다.
- 재생 루프처럼 "쓰기가 규칙적으로 자주 일어나는" 워크로드에 RwLock을 적용하면, 다수의 reader가 짧은 read lock을 계속 획득/해제하는 사이 writer가 끼어들 타이밍을 못 찾아 지연되거나(또는 반대로 reader들이 writer를 계속 기다리는), 이론적 이점이 실제로는 실현되지 않는 경우가 흔하다.
- 이런 미세한 값(현재 프레임 인덱스 하나)은 애초에 락보다 원자적 타입(`AtomicU64`)이 훨씬 적합한데, "공유 상태 = 락"이라는 습관적 사고로 RwLock을 선택하는 경우가 많다.

**발생 조건**:
- 재생 중 프레임 인덱스처럼 자주 갱신되는 단일 값에 RwLock을 사용할 때.
- reader 수가 매우 많고 각 read가 매우 짧아, 락 획득/해제 오버헤드 자체가 실질 작업 시간보다 커질 때.

**권장**:
```rust
struct PlaybackState {
    current_frame_idx: std::sync::atomic::AtomicU64,
}

tokio::spawn(async move {
    loop {
        playback.current_frame_idx.fetch_add(1, Ordering::Relaxed);
        tokio::time::sleep(frame_interval).await;
    }
});

for panel in panels {
    let idx = playback.current_frame_idx.load(Ordering::Relaxed);
    panel.update(idx);
}
```
- 단일 정수/플래그 수준의 공유 상태는 `Atomic*` 타입으로 락 자체를 제거한다.
- RwLock 도입 여부는 실제 read/write 빈도와 임계 구역 크기를 벤치마크로 확인한 뒤 결정하고, "읽기가 많다"는 직관만으로 선택하지 않는다.

**탐지 방법**:
- Semantic: `RwLock<T>`에서 `T`가 원자 타입으로 대체 가능한 단순 값(정수, bool, 열거형)인 경우를 찾아 atomics 전환 후보로 표시.
- Runtime: writer 대기 시간(락 획득까지의 지연)을 계측해 RwLock 도입 전후 성능을 비교.

**예외**:
- 임계 구역이 실제로 복잡한 자료구조(해시맵, 벡터)를 다루고 read가 write보다 압도적으로 많으며 write 빈도가 낮다면(예: 설정값, 캐시 테이블), RwLock은 정확히 의도된 사용처다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-017: 작업 우선순위 부재
**분류**: CONC · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// 하나의 워커 풀이 프레임 디코드, 배치 메트릭, export, 썸네일 생성 요청을
// 들어온 순서대로만 처리
let pool = ThreadPool::new(num_cpus::get());

fn submit_work(pool: &ThreadPool, job: Job) {
    pool.execute(move || job.run()); // 모든 job이 동등하게 취급됨
}
```

**문제**:
- 사용자가 지금 보고 있는 프레임을 디코드하는 작업과, 백그라운드에서 진행 중인 전체 영상 export 작업이 동일한 큐/풀에서 경쟁하면, 먼저 제출된 export 작업들이 워커를 모두 점유해 사용자 상호작용이 멈춘 것처럼 보인다.
- "먼저 온 순서대로"는 배치 처리 시스템에는 공정하지만, 인터랙티브 애플리케이션에서는 사용자 체감 반응성과 직접적으로 배치된다.
- 우선순위 개념이 없으면 나중에 급한 작업(사용자가 방금 클릭한 프레임)이 이미 제출된 저우선순위 작업들 뒤에서 대기해야 한다.

**발생 조건**:
- 단일 스레드 풀/작업 큐로 모든 백그라운드 작업(디코드, 메트릭, export, 썸네일)을 처리할 때.
- 사용자 상호작용 중 백그라운드 배치 작업이 이미 진행 중일 때.

**권장**:
```rust
enum Priority { Interactive = 0, Background = 1 }

struct PriorityPool {
    interactive: rayon::ThreadPool, // 코어의 절반 이상을 항상 확보
    background: rayon::ThreadPool,  // 나머지, 필요시 yield
}

fn submit_work(pool: &PriorityPool, priority: Priority, job: Job) {
    match priority {
        Priority::Interactive => pool.interactive.spawn(move || job.run()),
        Priority::Background => pool.background.spawn(move || job.run()),
    }
}
```
- 최소한 "인터랙티브"와 "백그라운드" 두 종류의 워커 풀/큐를 물리적으로 분리해 인터랙티브 작업이 항상 실행 자원을 확보하도록 한다.
- 더 정교하게는 우선순위 큐(binary heap 기반)와 선점(preemption) 또는 협조적 yield 지점을 도입한다.

**탐지 방법**:
- Structural: 코드베이스에서 스레드 풀/작업 큐 인스턴스 수를 세어보고, 서로 다른 성격의 작업(사용자 응답 vs. 배치)이 동일 인스턴스를 공유하는지 확인.
- Runtime: 백그라운드 배치 작업이 실행 중일 때와 아닐 때의 인터랙티브 작업 지연(p50/p99)을 비교.

**예외**:
- 애플리케이션에 배치성 백그라운드 작업이 애초에 없거나(모든 작업이 사용자 트리거 즉시 처리) 작업량이 워커 수를 절대 넘지 않는다면 우선순위 분리가 불필요할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-018: thumbnail 작업이 현재 frame decode를 방해
**분류**: CONC · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
// Filmstrip이 화면에 보이는 즉시 전체 비디오의 썸네일을 순차/병렬 생성
async fn generate_filmstrip_thumbnails(decoder: Arc<DecoderHandle>, frame_count: u64) {
    for idx in (0..frame_count).step_by(30) {
        // 메인 뷰포트가 사용하는 것과 동일한 decoder 인스턴스/워커 풀을 공유
        let _thumb = decoder.decode_frame_sync(idx).await;
    }
}
```

**문제**:
- Filmstrip 썸네일 생성이 메인 뷰포트의 현재 프레임 디코드와 동일한 디코더 인스턴스 또는 동일한 워커 풀을 공유하면, 사용자가 실제로 보고 있는 프레임 요청이 썸네일 생성 대기열 뒤에 밀릴 수 있다.
- 썸네일은 본질적으로 저우선순위/저해상도로 처리해도 무방한데, 메인 재생과 동등한 우선순위로 자원을 점유하면 재생이 버벅이는(frame drop) 결과로 이어진다.
- 디코더가 상태를 갖는 경우(순차 디코드에 의존하는 코덱), 썸네일 요청이 디코더의 내부 상태(참조 프레임 캐시 등)를 어지럽혀 메인 재생 디코드를 더 느리게 만들 수도 있다.

**발생 조건**:
- 새 비디오를 열자마자 Filmstrip이 백그라운드에서 전체 썸네일을 생성하기 시작하는 동안 사용자가 바로 재생을 시작할 때.
- 디코더 인스턴스를 뷰포트와 필름스트립이 공유하도록 설계되어 있을 때.

**권장**:
```rust
struct DecoderPool {
    interactive: Arc<DecoderHandle>, // 메인 뷰포트 전용, 항상 최우선
    background: Arc<DecoderHandle>,  // 썸네일 전용, 별도 인스턴스/워커
}

async fn generate_filmstrip_thumbnails(pool: Arc<DecoderPool>, frame_count: u64, cancel: CancellationToken) {
    for idx in (0..frame_count).step_by(30) {
        if cancel.is_cancelled() { break; }
        let thumb = tokio::task::spawn_blocking({
            let decoder = pool.background.clone();
            move || decoder.decode_frame_sync(idx)
        }).await;
        // 저해상도 디코드(가능하다면) + 낮은 우선순위 스레드
    }
}
```
- 인터랙티브 경로(현재 재생 프레임)와 백그라운드 경로(썸네일)를 별도의 디코더 인스턴스/워커로 물리적으로 분리한다.
- 가능하다면 썸네일은 저해상도 디코드 경로(코덱이 지원하는 경우)를 사용해 애초에 작업량을 줄인다.
- 사용자가 다른 비디오로 전환하면 진행 중인 썸네일 생성은 취소한다(CONC-008/009와 연계).

**탐지 방법**:
- Structural: 썸네일 생성 코드와 메인 뷰포트 디코드 코드가 동일한 `DecoderHandle`/워커 풀 변수를 참조하는지 검사.
- Runtime: Filmstrip 로딩 중 재생 프레임레이트(frame drop 카운트)를 측정해 상관관계 확인.

**예외**:
- 비디오가 짧거나(수백 프레임 이하) 썸네일 생성이 순식간에 끝난다면 별도 격리 없이도 체감 문제가 없을 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-019: metrics 계산이 UI interaction을 방해
**분류**: CONC · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
#[tauri::command]
async fn compute_all_metrics(state: tauri::State<'_, AppState>) -> Result<Vec<MetricResult>, String> {
    let frames = state.frames.clone();
    tokio::task::spawn_blocking(move || {
        // 사용 가능한 모든 코어를 사용하는 전역 Rayon pool을 그대로 사용
        frames.par_iter().map(compute_vmaf_for_frame).collect::<Vec<_>>()
    }).await.map_err(|e| e.to_string())
}
```

**문제**:
- Rayon의 기본 전역 풀은 논리 CPU 수만큼 스레드를 생성해 모든 코어를 포화시키도록 설계되어 있다. VMAF/SSIM 같은 메트릭 계산이 이 풀을 사용하면 시스템의 모든 코어가 100%에 가깝게 점유된다.
- Tauri의 메인 이벤트 루프(webview 렌더링, IPC 처리, OS 이벤트 디스패치)도 결국 OS 스케줄러가 CPU 시간을 배분해야 하는데, 모든 코어가 포화 상태면 UI 스레드도 스케줄링 지연을 겪어 클릭/스크롤에 대한 반응이 끊기거나 프레임이 드롭된다.
- 특히 노트북처럼 코어 수가 적은(4~8코어) 환경에서 체감이 크다.

**발생 조건**:
- 사용자가 메트릭 계산을 시작한 직후 UI를 조작(다른 프레임 클릭, 패널 전환)할 때.
- 코어 수가 적은 머신, 또는 다른 애플리케이션과 CPU를 공유하는 상황.

**권장**:
```rust
static METRICS_POOL: once_cell::sync::Lazy<rayon::ThreadPool> = once_cell::sync::Lazy::new(|| {
    // 최소 1개 코어는 UI/IPC를 위해 남겨둔다
    let n = (num_cpus::get().saturating_sub(1)).max(1);
    rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap()
});

#[tauri::command]
async fn compute_all_metrics(state: tauri::State<'_, AppState>) -> Result<Vec<MetricResult>, String> {
    let frames = state.frames.clone();
    tokio::task::spawn_blocking(move || {
        METRICS_POOL.install(|| frames.par_iter().map(compute_vmaf_for_frame).collect::<Vec<_>>())
    }).await.map_err(|e| e.to_string())
}
```
- CPU 집약적 백그라운드 작업 전용 Rayon 풀을 만들고, 전체 코어 수보다 적게(예: `num_cpus - 1`) 설정해 UI/IPC 처리에 최소한의 여유를 남긴다.
- OS 스레드 우선순위 API(가능한 플랫폼에서)로 메트릭 워커 스레드의 우선순위를 낮추는 것도 고려한다.

**탐지 방법**:
- Runtime: 메트릭 계산 중 UI 입력 지연(클릭→반응 시간)을 측정해 계산 유무에 따른 차이를 비교.
- Structural: 전역 Rayon 풀(`rayon::current_num_threads()` 기본값 사용)을 그대로 쓰는 CPU 집약적 작업 호출부를 찾아 전용 풀 사용 여부 확인.

**예외**:
- 메트릭 계산이 "백그라운드 전용 배치 모드"로 명시되어 있고, 그동안 사용자가 앱을 조작하지 않을 것이 UX상 보장된다면(예: 진행 중 UI를 잠그고 진행률만 표시) 코어 전체를 사용하는 것이 합리적일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-020: 동일 frame을 여러 작업이 중복 decode
**분류**: CONC · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
async fn decode_frame(state: tauri::State<'_, AppState>, idx: u64) -> Result<FrameBuf, String> {
    // 캐시 확인 없이 매번 새로 디코드
    let decoder = state.decoder.clone();
    tokio::task::spawn_blocking(move || decoder.decode_frame_sync(idx))
        .await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}
```
```typescript
// frontend: 프리뷰 패널과 히스토그램 패널이 각각 독립적으로 같은 프레임을 요청
useEffect(() => { invoke('decode_frame', { idx }); }, [idx]); // Preview
useEffect(() => { invoke('decode_frame', { idx }); }, [idx]); // Histogram
```

**문제**:
- 여러 UI 패널(프리뷰, 히스토그램, 파형 분석)이 같은 프레임 인덱스를 각자 독립적으로 요청하면, 캐시나 in-flight 요청 중복 제거가 없을 경우 동일한 프레임을 여러 번 디코드하게 된다.
- 디코드는 비용이 큰 연산이므로, N개의 패널이 동시에 같은 프레임을 요청하면 CPU 자원을 N배로 낭비하는 셈이다.
- 이미 진행 중인 동일 요청이 있는데 새 요청이 또 디코드를 시작하는 "thundering herd" 패턴은 CONC-004(오버서브스크립션)와 결합되면 더 악화된다.

**발생 조건**:
- 여러 독립적인 UI 컴포넌트가 동일한 백엔드 커맨드를 각자 호출하는 아키텍처.
- 프레임 전환 시 여러 패널이 거의 동시에 리렌더/재요청될 때.

**권장**:
```rust
struct DecodeCoordinator {
    cache: RwLock<lru::LruCache<u64, FrameBuf>>,
    in_flight: Mutex<HashMap<u64, tokio::sync::watch::Receiver<Option<FrameBuf>>>>,
}

async fn decode_frame_coordinated(coord: Arc<DecodeCoordinator>, decoder: Arc<DecoderHandle>, idx: u64) -> FrameBuf {
    if let Some(f) = coord.cache.read().unwrap().peek(&idx) {
        return f.clone();
    }
    // 이미 같은 프레임을 디코드 중인 요청이 있으면 그 결과를 함께 기다림 (join, 중복 실행 없음)
    let mut in_flight = coord.in_flight.lock().unwrap();
    if let Some(rx) = in_flight.get(&idx) {
        let mut rx = rx.clone();
        drop(in_flight);
        rx.changed().await.ok();
        return rx.borrow().clone().expect("decode failed");
    }
    let (tx, rx) = tokio::sync::watch::channel(None);
    in_flight.insert(idx, rx);
    drop(in_flight);

    let frame = tokio::task::spawn_blocking(move || decoder.decode_frame_sync(idx)).await.unwrap().unwrap();
    coord.cache.write().unwrap().put(idx, frame.clone());
    coord.in_flight.lock().unwrap().remove(&idx);
    tx.send(Some(frame.clone())).ok();
    frame
}
```
- LRU 캐시로 최근 디코드된 프레임을 재사용하고, 동시에 들어온 동일 인덱스 요청은 "in-flight" 테이블로 병합(join)해 실제 디코드는 한 번만 수행한다.
- 프론트엔드 쪽에서도 여러 패널이 공용 데이터 소스(예: 전역 상태/캐시)를 구독하도록 해 애초에 중복 invoke를 줄인다.

**탐지 방법**:
- Structural: 백엔드 decode 커맨드에 캐시/in-flight 중복 제거 로직이 있는지 확인.
- Runtime: 프레임 전환 시 동일 `idx`에 대한 `decode_frame_sync` 호출 횟수를 로깅해 패널 수 대비 호출 배율을 측정.

**예외**:
- 디코드 비용이 매우 낮거나(캐시가 오히려 메모리 오버헤드를 정당화하지 못하는 초경량 코덱), 패널이 하나뿐인 애플리케이션이라면 중복 제거 인프라가 과설계일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-021: generation ID 없이 오래된 결과 적용
**분류**: CONC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
#[tauri::command]
async fn seek_and_decode(state: tauri::State<'_, AppState>, idx: u64) -> Result<(), String> {
    let decoder = state.decoder.clone();
    let app_handle = state.app_handle.clone();
    tokio::spawn(async move {
        let frame = tokio::task::spawn_blocking(move || decoder.decode_frame_sync(idx)).await.unwrap();
        // 이 emit이 실행되는 시점에는 사용자가 이미 다른 프레임으로 이동했을 수 있음
        app_handle.emit_all("frame-ready", frame.unwrap()).ok();
    });
    Ok(())
}
```

**문제**:
- 비동기 디코드 요청이 완료되는 순서는 요청이 시작된 순서와 다를 수 있다(느린 프레임이 나중에 끝날 수 있음). generation/sequence 식별자 없이 결과를 그대로 화면에 반영하면, 사용자가 이미 다른 프레임으로 이동한 뒤에 이전 요청의 결과가 뒤늦게 도착해 화면을 덮어쓸 수 있다.
- 특히 CONC-007(모든 스크럽 요청을 처리)과 결합되면, 여러 개의 decode 요청이 동시에 진행되다가 무작위 순서로 완료되어 최종적으로 화면에 어떤 프레임이 표시될지 예측할 수 없게 된다.
- 이 버그는 재현이 간헐적(타이밍에 의존)이라 디버깅이 특히 어렵다.

**발생 조건**:
- 빠른 스크러빙, 빠른 프레임 전환, 네트워크/디스크 I/O로 인해 디코드 시간이 요청마다 들쭉날쭉할 때.
- 여러 decode 요청이 동시에 진행 중(in-flight)일 수 있는 구조.

**권장**:
```rust
struct AppState {
    generation: AtomicU64,
}

#[tauri::command]
async fn seek_and_decode(state: tauri::State<'_, AppState>, idx: u64) -> Result<(), String> {
    let my_gen = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let decoder = state.decoder.clone();
    let app_handle = state.app_handle.clone();
    let generation = state.generation_handle(); // Arc<AtomicU64> clone

    tokio::spawn(async move {
        let frame = tokio::task::spawn_blocking(move || decoder.decode_frame_sync(idx)).await.unwrap();
        // 결과를 반영하기 직전, 자신의 generation이 여전히 최신인지 확인
        if generation.load(Ordering::SeqCst) == my_gen {
            app_handle.emit_all("frame-ready", frame.unwrap()).ok();
        } // 아니라면 조용히 폐기
    });
    Ok(())
}
```
- 모든 "최신 요청만 유효한" 종류의 비동기 작업에 단조 증가하는 generation/sequence ID를 부여하고, 결과를 적용하기 직전에 현재 generation과 비교해 최신이 아니면 폐기한다.
- 이 패턴은 decode뿐 아니라 검색, 필터링 등 "최신 입력에 대한 응답만 유효한" 모든 비동기 UI 갱신에 일반적으로 적용된다.

**탐지 방법**:
- Semantic: 비동기 완료 콜백/이벤트 핸들러가 결과를 상태에 반영하기 전에 "이 요청이 여전히 최신인가"를 확인하는 코드가 있는지 리뷰.
- Runtime: 빠른 스크러빙 후 정지했을 때 최종 표시된 프레임이 실제로 마지막 요청한 프레임과 일치하는지 자동화 테스트로 검증.

**예외**:
- 요청이 항상 순차적으로 하나씩만 진행되도록 이미 직렬화되어 있다면(다음 요청 시작 전 이전 요청 완료를 기다림) generation ID가 불필요할 수 있다. 다만 이 경우 응답성이 떨어질 수 있으니 트레이드오프를 인지해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-022: worker panic 후 pool 상태 불명확
**분류**: CONC · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
frames.par_iter().for_each(|f| {
    let mut guard = shared_cache.lock().unwrap();
    // decode_frame_sync가 잘못된 비트스트림에 대해 panic!을 던지면
    guard.insert(f.idx, decode_frame_sync(f)); // 이 스레드가 락을 쥔 채 panic
    // std::sync::Mutex는 이 경우 poisoned 상태가 되어 이후 모든 lock()이 Err를 반환
});
```

**문제**:
- FFI 디코더가 잘못된/손상된 비트스트림을 만나 panic(또는 Rust 레이어의 `unwrap()`이 실패)하면, 그 시점에 락을 쥐고 있던 스레드가 락을 정상적으로 해제하지 못하고 `std::sync::Mutex`가 "poisoned" 상태가 된다.
- Poisoned 상태에서는 이후 모든 `lock()` 호출이 기본적으로 `Err`를 반환하므로, 코드가 이를 `.unwrap()`으로 처리하고 있었다면 최초 panic 이후 연쇄적으로 모든 관련 기능이 panic하기 시작한다.
- Rayon의 경우 `par_iter` 내부에서 panic이 발생하면 전체 병렬 작업이 panic으로 전파되는데, 이미 완료된 다른 프레임들의 부분 결과를 보존할지, 전체를 실패로 처리할지에 대한 정책이 없으면 애매한 상태로 남는다.
- Tokio task 안에서 panic이 발생하면 해당 task는 종료되지만 프로세스 전체는 계속 실행되므로, "일부 작업만 조용히 사라진" 상태가 되어 사용자에게 아무 오류도 보이지 않을 수 있다.

**발생 조건**:
- 손상되었거나 스펙을 벗어난 비트스트림을 파싱/디코드할 때(퍼징, 잘못된 파일).
- FFI 경계에서 예외적인 반환값을 Rust 쪽이 `unwrap()`/`expect()`로 성급하게 처리할 때.

**권장**:
```rust
// 1) Poison 복구 전략을 명시
fn safe_lock<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::error!("mutex poisoned, recovering");
            poisoned.into_inner() // 데이터 일관성이 보장되는 경우에만 채택
        }
    }
}

// 2) FFI 경계에서 panic이 아니라 Result로 오류를 전파
let results: Vec<Result<MetricResult, DecodeError>> = frames.par_iter()
    .map(|f| std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decode_frame_sync(f)))
        .map_err(|_| DecodeError::PanicInDecoder(f.idx)))
    .collect();
```
- FFI 호출부는 가능하면 panic이 아니라 `Result`/에러 코드로 실패를 표현하도록 감싸고, 불가피하게 panic 가능성이 있다면 `catch_unwind`로 격리해 하나의 프레임 실패가 전체 배치를 무너뜨리지 않게 한다.
- `Mutex` poison에 대한 명시적 복구 정책(로그 남기고 재시작 vs. 데이터 무효화 후 재구축)을 정하고 `.unwrap()` 대신 이를 적용한다.
- 손상된 입력에 대한 방어적 검증(파싱 단계에서 사전 검증)을 추가해 panic이 애초에 발생하지 않도록 한다.

**탐지 방법**:
- Runtime: 손상된/퍼징된 비트스트림 샘플로 통합 테스트를 실행해 panic 발생 시 애플리케이션이 어떤 상태로 남는지 관찰.
- Static: `Mutex::lock().unwrap()` 패턴을 grep해 poison 처리 정책이 있는지 확인.

**예외**:
- 완전히 신뢰된 입력만 다루는 내부 도구이고 panic 발생 시 프로세스 전체를 재시작하는 것이 허용되는 배치 파이프라인이라면, poison 복구 로직 없이 "panic하면 프로세스가 죽고 상위 오케스트레이터가 재시작"하는 것도 유효한 전략이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-023: 종료 시 worker join 누락
**분류**: CONC · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn main() {
    let export_handle = std::thread::spawn(move || {
        export_video_to_file(&path); // 파일 쓰기 + flush + 임시 파일 rename
    });

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    // 앱 종료(윈도우 닫힘) 시 export_handle을 join하지 않고 프로세스가 그냥 끝남
}
```

**문제**:
- 사용자가 export 작업이 진행되는 도중 앱을 닫으면, 백그라운드 스레드가 아직 파일에 데이터를 쓰고 있는 상태에서 프로세스가 종료되어 출력 파일이 손상되거나 불완전하게 남을 수 있다.
- 임시 파일 → 최종 파일 rename 같은 원자적 커밋 패턴을 쓰고 있더라도, rename 직전에 프로세스가 죽으면 임시 파일만 남고 사용자는 작업이 실패했는지조차 알기 어렵다.
- Tokio 런타임 자체도 `Runtime` 드롭 시 진행 중인 task를 강제로 취소할 수 있는데, 이때 정리(cleanup) 로직이 실행되지 않으면 파일 핸들, 락 파일, 임시 디렉터리가 남는다.

**발생 조건**:
- Export, 배치 메트릭 계산 결과 저장 등 디스크에 쓰기를 수반하는 장시간 작업 중 사용자가 창을 닫거나 OS가 앱을 강제 종료할 때.
- Tauri 앱의 `on_window_event`에서 종료 이벤트를 가로채지 않고 기본 동작(즉시 종료)에 맡길 때.

**권장**:
```rust
fn main() {
    let export_state = Arc::new(ExportRegistry::new());

    tauri::Builder::default()
        .setup({
            let export_state = export_state.clone();
            move |app| {
                let app_handle = app.handle();
                app_handle.listen_global("tauri://close-requested", move |_| {
                    // 진행 중인 export가 있으면 사용자에게 확인/취소 옵션 제공,
                    // 또는 안전하게 중단하고 임시 파일 정리
                    export_state.cancel_all_and_cleanup();
                });
                Ok(())
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
- 장시간 백그라운드 작업의 핸들을 애플리케이션 상태에 등록해두고, 종료 이벤트(`on_window_event(CloseRequested)` 등)에서 명시적으로 취소·join하거나 사용자에게 확인을 받는다.
- 파일 쓰기는 항상 "임시 파일에 쓰고 완료 후 rename"하는 원자적 커밋 패턴을 사용해, 중간에 종료되어도 최종 파일은 손상되지 않게 한다.
- `std::thread::spawn`으로 만든 detached thread 대신, JoinHandle을 앱 상태에 보관해 종료 시퀀스에서 명시적으로 `join`(또는 timeout 후 강제 종료)한다.

**탐지 방법**:
- Structural: `std::thread::spawn`/`tokio::spawn`의 반환값(`JoinHandle`)이 버려지는(`let _ =` 없이 그냥 호출만 하는) 위치를 검사하고, 그중 파일 I/O를 포함하는 것이 있는지 확인.
- Manual: 종료 경로(창 닫기, Cmd+Q, 프로세스 kill)에서 진행 중인 백그라운드 작업이 어떻게 처리되는지 명시적으로 문서화되어 있는지 확인.

**예외**:
- 순수 메모리 연산만 수행하고 디스크 상태를 변경하지 않는 백그라운드 작업(예: 캐시 워밍업)은 종료 시 그냥 버려져도 데이터 손상 위험이 없다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-024: FFI 객체를 잘못 Send/Sync 처리
**분류**: CONC · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
struct Dav1dContext(*mut dav1d_sys::Dav1dContext);

// 컴파일러가 raw pointer를 이유로 Send/Sync를 자동 도출하지 못하니,
// "컴파일만 되면 된다"는 생각으로 안전성 검증 없이 그냥 unsafe impl을 붙임
unsafe impl Send for Dav1dContext {}
unsafe impl Sync for Dav1dContext {}

// 이후 여러 스레드가 동시에 같은 Dav1dContext를 통해 decode 호출
let ctx = Arc::new(Dav1dContext(raw_ptr));
for _ in 0..4 {
    let ctx = ctx.clone();
    std::thread::spawn(move || unsafe { dav1d_send_data(ctx.0, ...) }); // 실제 dav1d 컨텍스트는 동시 호출에 안전하지 않을 수 있음
}
```

**문제**:
- `unsafe impl Send`/`unsafe impl Sync`는 컴파일러에게 "이 타입은 실제로 스레드 간 이동/공유가 안전하다"는 약속을 하는 것인데, 이 약속이 실제 FFI 라이브러리의 스레드 안전성 보장과 일치하는지 검증하지 않고 붙이면 정의되지 않은 동작(UB)으로 이어진다.
- 많은 디코더/코덱 라이브러리는 "컨텍스트 하나는 단일 호출 스레드에서만 사용"하거나 "내부적으로 자체 락을 갖고 있어 스레드 안전" 등 서로 다른 스레드 안전성 모델을 갖는데, 이를 라이브러리 문서/헤더로 확인하지 않고 습관적으로 `unsafe impl Send + Sync`를 붙이는 경우가 흔하다.
- 이런 버그는 대부분의 실행에서는 우연히 문제없이 동작하다가, 특정 타이밍(동시 호출)에서만 크래시나 메모리 손상으로 나타나 재현이 매우 어렵다.

**발생 조건**:
- FFI 바인딩을 직접 작성하면서 raw pointer를 감싸는 wrapper 타입에 대해 Send/Sync를 수동으로 구현할 때.
- 여러 스레드/task가 동일한 디코더 컨텍스트를 공유하려는 설계(CONC-002, CONC-013과 연결)를 시도할 때.

**권장**:
```rust
// 라이브러리 문서를 확인해 정확한 안전성 모델을 반영한다.
// 예: dav1d 컨텍스트가 "동시 호출 불가, 하지만 다른 스레드로의 이동은 안전"하다면:
struct Dav1dContext(*mut dav1d_sys::Dav1dContext);
unsafe impl Send for Dav1dContext {} // 이동은 안전 — 문서/실험으로 확인됨
// Sync는 구현하지 않음 — 동시 공유 접근은 금지

// 동시 접근이 필요하다면 명시적 직렬화로 안전성을 프로그램적으로 보장
struct SafeDecoder {
    inner: std::sync::Mutex<Dav1dContext>, // Mutex가 동시 접근을 실제로 막아줌
}
```
- FFI wrapper에 `Send`/`Sync`를 구현하기 전, 해당 라이브러리의 스레드 안전성 문서(또는 소스)를 확인하고 근거를 코드 주석에 남긴다.
- 라이브러리가 동시 접근에 안전하지 않다면 `Sync`를 구현하지 말고, 대신 `Mutex`로 감싸 Rust의 타입 시스템이 실제로 직렬화를 강제하도록 만든다("`unsafe impl Sync`로 약속"하는 대신 "`Mutex`로 보장"하는 편이 항상 더 안전하다).
- 가능하면 FFI 안전성 검증을 Miri, ThreadSanitizer(`-Z sanitizer=thread`) 등으로 주기적으로 점검한다.

**탐지 방법**:
- Static: 코드베이스에서 `unsafe impl Send`/`unsafe impl Sync` 전수 목록화, 각각에 근거 주석이 있는지 확인.
- Runtime: ThreadSanitizer 빌드로 동시 FFI 호출 시나리오를 테스트해 data race 탐지.

**예외**:
- 라이브러리가 명시적으로 "컨텍스트는 스레드 안전(재진입 가능)"이라고 문서화했고, 이를 검증하는 테스트(TSan 등)를 통과했다면 `unsafe impl Sync`는 정당하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-025: thread 수를 logical CPU 수로 무조건 설정
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// 세 곳에서 각각 독립적으로 "논리 CPU 수만큼"을 최적이라고 가정
let tokio_rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(num_cpus::get()) // 기본값과 동일하지만 명시적으로 재확인 없이 사용
    .build().unwrap();

let rayon_pool = rayon::ThreadPoolBuilder::new()
    .num_threads(num_cpus::get())
    .build().unwrap();

let dav1d_settings = Dav1dSettings { n_threads: num_cpus::get() as i32, ..Default::default() };
```

**문제**:
- `num_cpus::get()`은 물리적으로 사용 가능한 논리 코어 수를 반환하지만, 컨테이너/cgroup으로 CPU가 제한된 환경(예: CI, Docker, 클라우드 VM의 CPU quota)에서는 이 값이 실제 할당량보다 훨씬 크게 나올 수 있다("호스트는 32코어지만 컨테이너에는 2코어만 할당됨").
- Tokio worker, Rayon pool, 디코더 내부 스레드가 각각 독립적으로 "논리 CPU 수"를 상한으로 사용하면, 컨테이너 환경에서 실제 할당량 대비 수 배~수십 배의 스레드가 생성되어 컨텍스트 스위칭 오버헤드로 성능이 오히려 급락한다.
- 데스크톱 환경에서도 세 계층이 각각 전체 코어 수를 기준으로 잡으면 CONC-003/CONC-004에서 설명한 오버서브스크립션이 필연적으로 발생한다.

**발생 조건**:
- Docker/CI 컨테이너처럼 cgroup으로 CPU가 제한된 환경에서 실행할 때(`num_cpus`가 cgroup quota를 인식하지 못하는 버전/설정일 때).
- 여러 병렬 계층(Tokio + Rayon + FFI 디코더)이 서로의 스레드 예산을 모르는 채 각자 최댓값을 잡을 때.

**권장**:
```rust
// 1) cgroup-aware 크레이트 사용(예: num_cpus 대신 available_parallelism 또는 cgroups 인식 크레이트)
let available = std::thread::available_parallelism()
    .map(|n| n.get())
    .unwrap_or(4);

// 2) 전체 스레드 예산을 애플리케이션 레벨에서 한 번만 계산하고 계층별로 분배
let total_budget = available.max(2);
let tokio_workers = (total_budget / 2).max(1);
let rayon_threads = (total_budget - tokio_workers).max(1);

let tokio_rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(tokio_workers)
    .build().unwrap();

let rayon_pool = rayon::ThreadPoolBuilder::new()
    .num_threads(rayon_threads)
    .build().unwrap();

// 디코더는 필요시 rayon_threads 예산 안에서 더 세분화
```
- 스레드 예산을 애플리케이션 시작 시 한 곳에서 계산하고, Tokio/Rayon/FFI 디코더 설정에 그 예산을 명시적으로 분배한다("각자 알아서 전체 코어 수를 쓴다"를 금지한다).
- 컨테이너 환경을 지원해야 한다면 cgroup v1/v2 quota를 인식하는 방식(`/sys/fs/cgroup/...` 파싱 또는 이를 지원하는 크레이트)을 사용하거나, 최소한 환경 변수로 스레드 수를 오버라이드할 수 있게 한다.

**탐지 방법**:
- Structural: `num_cpus::get()` 호출 지점을 전수 검색해, 서로 다른 병렬 계층(Tokio/Rayon/FFI)에서 각각 독립적으로 호출되는지 확인.
- Runtime: cgroup으로 CPU를 제한한 컨테이너(예: `docker run --cpus=2`)에서 실행해 실제 생성되는 총 스레드 수와 CPU 사용률(컨텍스트 스위치 비율)을 측정.

**예외**:
- 애플리케이션이 항상 단일 사용자의 데스크톱 환경에서만 실행되고 컨테이너/클라우드 배포 계획이 없다면, 위험은 CONC-003/004의 오버서브스크립션 문제로 국한되며 cgroup 인식은 불필요할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-026: UI 이벤트마다 새 Tokio task spawn (backpressure/coalescing 없음)
**분류**: CONC · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn on_zoom_change(state: tauri::State<'_, AppState>, zoom: f32) {
    let state = state.inner().clone();
    // 슬라이더가 매 픽셀 이동마다 이 커맨드를 호출하고, 그때마다 새 task가 뜬다
    tokio::spawn(async move {
        recompute_visible_range(&state, zoom).await;
    });
}
```

**문제**:
- 연속적인 UI 조작(줌 슬라이더 드래그, 마우스 휠)마다 새로운 `tokio::spawn`이 발생하면, 짧은 시간에 수십~수백 개의 task가 큐에 쌓이고 이들이 임의의 순서로 완료되어 CONC-021(오래된 결과 적용)과 동일한 문제로 이어질 수 있다.
- Task 생성 자체는 가볍지만, 각 task가 내부적으로 상태 락 획득, 재계산, emit을 수행한다면 그 누적 비용이 상당해지고, 뒤에 있는 task가 앞선 task의 작업을 그대로 덮어쓰는 낭비가 발생한다.
- Backpressure나 coalescing이 없으면 "이벤트 발생 빈도"가 그대로 "작업 부하"로 직결되어, 입력 장치 폴링 속도(수백 Hz)에 애플리케이션 처리량이 종속된다.

**발생 조건**:
- 슬라이더, 드래그, 휠 스크롤처럼 초당 수십~수백 회 발생 가능한 연속 입력 이벤트를 그대로 백엔드 커맨드에 매핑할 때.
- 각 이벤트 처리 비용이 이벤트 발생 간격보다 클 때(처리가 밀리기 시작).

**권장**:
```rust
struct DebouncedZoom {
    tx: tokio::sync::watch::Sender<f32>,
}

impl DebouncedZoom {
    fn new(state: Arc<AppState>) -> Self {
        let (tx, mut rx) = tokio::sync::watch::channel(1.0f32);
        tokio::spawn(async move {
            loop {
                rx.changed().await.ok();
                // 짧은 유예 시간 동안 추가 변경을 흡수(coalesce)
                tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                let zoom = *rx.borrow_and_update();
                recompute_visible_range(&state, zoom).await;
            }
        });
        Self { tx }
    }
    fn set(&self, zoom: f32) { let _ = self.tx.send(zoom); }
}

#[tauri::command]
fn on_zoom_change(state: tauri::State<'_, DebouncedZoom>, zoom: f32) {
    state.set(zoom); // task spawn 없음, 최신값만 유지
}
```
- 연속 이벤트는 `watch` 채널 + 단일 상시 워커 task로 coalescing해, 실제 무거운 재계산은 짧은 유예 시간 동안의 "최종 값"에 대해서만 수행한다.
- 이벤트마다 task를 새로 만드는 대신 하나의 장수(long-lived) 워커가 최신 상태를 폴링/구독하는 구조로 전환한다.

**탐지 방법**:
- Structural: 프론트엔드 이벤트 핸들러(`onChange`, `onMouseMove` 등)와 직접 연결된 `#[tauri::command]`가 내부에서 `tokio::spawn`을 호출하는지, 그리고 그 커맨드의 예상 호출 빈도가 높은지 교차 확인.
- Runtime: 슬라이더 드래그 같은 연속 조작 중 `tokio::spawn` 호출 횟수와 활성 task 수를 계측.

**예외**:
- 이벤트 발생 빈도가 본질적으로 낮은(버튼 클릭 등) 경우라면 매번 spawn해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-027: 이미 async 컨텍스트 안에서 block_on 호출
**분류**: CONC · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
#[tauri::command]
async fn export_with_metrics(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let handle = tokio::runtime::Handle::current();
    let frames = state.frames.clone();
    frames.par_iter().for_each(|f| {
        // Rayon 워커 스레드 안에서, 이미 실행 중인 Tokio 런타임에 대해 block_on 호출
        handle.block_on(async {
            compute_vmaf_for_frame_async(f).await;
        });
    });
    Ok(())
}
```

**문제**:
- Tokio 멀티스레드 런타임의 worker thread 안에서 같은 런타임에 대해 `block_on`을 호출하면 "Cannot start a runtime from within a runtime" panic이 발생한다. 위 예시처럼 Rayon 워커 스레드(Tokio worker가 아닌)에서 호출하면 panic은 피하지만, 그 Rayon 스레드가 이제 async task 완료를 동기적으로 기다리게 되어 Rayon pool 전체의 병렬성을 사실상 없애버린다.
- 더 위험한 경우는 Tokio worker thread 안에서 직접 `block_on`을 호출하는 패턴인데, 제한된 worker thread 풀 안에서 하나의 스레드가 다른 task의 완료를 동기 대기하면서 그 다른 task가 마침 같은(포화된) worker pool에서 스케줄되기를 기다리는 순환이 생기면 데드락으로 이어질 수 있다.
- "동기 함수 안에서 async 결과가 필요하다"는 요구 자체가 설계상 async/sync 경계가 잘못 그어졌다는 신호인 경우가 많다.

**발생 조건**:
- 동기 함수(Rayon 클로저, FFI 콜백, `Drop` 구현 등) 안에서 async 함수의 결과가 필요할 때 임시방편으로 `block_on`을 사용할 때.
- 라이브러리 경계(sync API를 요구하는 콜백)와 애플리케이션의 async 코드가 만나는 지점.

**권장**:
```rust
// 원칙: async 코드에서 필요한 것은 async 함수로, sync 코드에서 필요한 것은 sync 함수로 유지한다.
// 메트릭 계산 자체를 sync 함수로 만들고, spawn_blocking으로 그 전체를 감싼다.
#[tauri::command]
async fn export_with_metrics(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let frames = state.frames.clone();
    tokio::task::spawn_blocking(move || {
        frames.par_iter().for_each(|f| {
            compute_vmaf_for_frame_sync(f); // async가 아닌 순수 동기 버전
        });
    })
    .await
    .map_err(|e| e.to_string())
}
```
- async 함수와 sync 함수를 섞어야 하는 상황이라면, "async 함수 안에서 sync 블로킹 코드를 spawn_blocking으로 격리"하는 방향이 "sync 코드 안에서 block_on으로 async를 기다리는" 방향보다 항상 안전하다.
- 부득이하게 `block_on`이 필요하다면(예: FFI 콜백이 강제로 동기 반환을 요구), 별도의 전용 런타임(`tokio::runtime::Builder::new_current_thread()`)을 그 목적만을 위해 생성해 메인 애플리케이션 런타임과 완전히 분리한다.

**탐지 방법**:
- Static: `block_on(`/`Handle::block_on(` 호출부를 전수 검색하고, 호출 스택이 이미 async 컨텍스트(Tokio worker thread)에서 시작되는지 추적.
- Runtime: 통합 테스트에서 "Cannot start a runtime from within a runtime" panic 메시지 또는 응답 없는(hang) 상태를 감시.

**예외**:
- `main()` 같은 최상위 동기 진입점에서 런타임을 시작하기 위해 `Runtime::block_on`을 호출하는 것은 정상적인 용법이며 이 안티패턴과 무관하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-028: select! 루프에서 한쪽 branch가 굶주림
**분류**: CONC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
async fn decode_worker(mut frame_rx: mpsc::Receiver<u64>, mut shutdown_rx: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            Some(idx) = frame_rx.recv() => {
                decode_and_emit(idx).await;
                // frame_rx가 항상 즉시 준비되어 있는 상황(연속 재생 중)이면
                // 아래 shutdown 브랜치가 폴링될 기회를 계속 놓칠 수 있음
            }
            _ = shutdown_rx.changed() => {
                break;
            }
        }
    }
}
```

**문제**:
- `tokio::select!`는 기본적으로 각 반복마다 준비된 브랜치 중 하나를 유사 랜덤(pseudo-random)하게 고르지만, 한쪽 채널(`frame_rx`)이 항상 즉시 준비되어 있는 고빈도 상황에서는 통계적으로 다른 브랜치(`shutdown_rx`)가 선택될 확률이 낮아지고, 특히 `biased;`를 실수로 붙여 고빈도 브랜치를 먼저 배치하면 저빈도 브랜치가 사실상 무한정 굶주릴 수 있다.
- 종료 신호, 취소 토큰, 우선순위 이벤트처럼 "드물지만 반드시 적시에 처리되어야 하는" 브랜치가 굶주리면, 사용자가 취소/종료를 요청해도 즉시 반응하지 않는 문제로 나타난다(CONC-008/009와 연결).
- 이 문제는 부하가 낮을 때는 드러나지 않다가, 트래픽/이벤트 빈도가 높아지는 상황(빠른 연속 재생)에서만 나타나므로 테스트에서 놓치기 쉽다.

**발생 조건**:
- 하나의 브랜치가 지속적으로 즉시 준비 상태를 유지하는 고빈도 스트림(연속 프레임 디코드 요청)과, 드물게 발생하는 제어 신호(종료, 취소)가 같은 `select!`에 공존할 때.
- `biased;` 키워드를 고빈도 브랜치에 붙였거나, 공정성이 필요한 상황에서 아무 순서 힌트도 주지 않았을 때.

**권장**:
```rust
async fn decode_worker(mut frame_rx: mpsc::Receiver<u64>, mut shutdown_rx: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            biased;
            // 제어 신호를 항상 먼저 검사해 굶주림을 원천 차단
            _ = shutdown_rx.changed() => {
                break;
            }
            Some(idx) = frame_rx.recv() => {
                decode_and_emit(idx).await;
            }
        }
    }
}
```
- 우선순위가 높거나 드물지만 반드시 적시 처리가 필요한 브랜치(종료, 취소)는 `biased;`와 함께 `select!`의 맨 위에 배치해 매 반복마다 최우선으로 확인되도록 한다.
- 반대로 처리량이 중요한 고빈도 브랜치에는 명시적으로 처리 개수 제한(예: 한 번의 select 반복에서 최대 N개까지만 배치 처리 후 다시 loop top으로)을 두어 다른 브랜치에 기회를 준다.

**탐지 방법**:
- Semantic: `select!` 블록 안에 빈도가 크게 다른 브랜치(고빈도 데이터 채널 vs. 저빈도 제어 채널)가 섞여 있는지 리뷰하고, 우선순위/굶주림 방지 힌트가 있는지 확인.
- Runtime: 고빈도 스트림을 계속 흘려보내는 부하 테스트 중 종료/취소 신호에 대한 응답 지연을 측정.

**예외**:
- 모든 브랜치의 발생 빈도가 비슷하거나, 브랜치 하나가 지연되어도 기능적으로 문제가 없는(둘 다 저빈도이거나 둘 다 즉시 처리가 필수는 아닌) 경우라면 fairness 문제를 신경 쓰지 않아도 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-029: Drop에서 async 정리 작업 시도
**분류**: CONC · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
struct DecoderSession {
    ctx: Arc<Dav1dContext>,
    cleanup_tx: tokio::sync::mpsc::Sender<CleanupRequest>,
}

impl Drop for DecoderSession {
    fn drop(&mut self) {
        // Drop::drop은 동기 함수라 .await를 사용할 수 없음.
        // block_on으로 우회하면 CONC-027의 위험을 그대로 안게 됨.
        let ctx = self.ctx.clone();
        tokio::runtime::Handle::current().block_on(async move {
            flush_decoder_async(ctx).await; // 비동기 flush를 억지로 동기화
        });
    }
}
```

**문제**:
- Rust의 `Drop::drop`은 동기 함수로 정의되어 있어 `.await`를 사용할 수 없다. 디코더/파일 핸들을 정리하기 위해 비동기 flush나 네트워크 호출이 필요한 경우, `Drop` 안에서 이를 처리하려는 시도는 필연적으로 `block_on` 우회(CONC-027) 또는 정리 작업 자체를 생략하는 결과로 이어진다.
- `Drop` 시점에는 이미 값이 스코프를 벗어나는 중이라 에러 처리도 제한적이다(`drop`은 `Result`를 반환할 수 없다). 정리 작업이 실패해도 이를 호출자에게 알릴 방법이 없다.
- Tokio 런타임이 이미 종료 중이거나, `Drop`이 Tokio worker thread가 아닌 다른 컨텍스트(예: 메인 스레드, panic unwind 중)에서 호출되는 경우 `Handle::current()`가 panic하거나 예상과 다르게 동작할 수 있다.

**발생 조건**:
- 디코더 세션, 네트워크 연결, 임시 파일 핸들처럼 "정리에 비동기 작업이 필요한" 리소스를 RAII 패턴(`Drop`)으로 관리하려 할 때.
- 명시적인 `close()`/`shutdown()` async 메서드 호출을 잊고 값이 스코프를 벗어나도록 방치할 때.

**권장**:
```rust
struct DecoderSession {
    ctx: Option<Arc<Dav1dContext>>,
}

impl DecoderSession {
    // 명시적인 비동기 종료 메서드를 제공하고 호출을 강제(문서화 + 린트)
    async fn close(mut self) -> Result<(), DecodeError> {
        if let Some(ctx) = self.ctx.take() {
            flush_decoder_async(ctx).await?;
        }
        Ok(())
    }
}

impl Drop for DecoderSession {
    fn drop(&mut self) {
        // close()가 호출되지 않고 그냥 drop된 경우를 대비한 최소한의 동기 정리 + 경고 로그
        if self.ctx.is_some() {
            log::warn!("DecoderSession dropped without calling close(); resources may leak");
            // 여기서는 block_on을 쓰지 않고, 동기적으로 가능한 최소 정리(예: 파일 디스크립터 close)만 수행
        }
    }
}
```
- 비동기 정리가 필요한 리소스는 `Drop`에 의존하지 말고 명시적인 `async fn close(self)`/`shutdown(self)` 메서드를 제공해 호출자가 반드시 `.await`하도록 설계한다(가능하다면 `#[must_use]`로 호출 누락을 경고).
- `Drop`은 "정말 최후의 안전망"으로만 남기고, 그 안에서는 동기적으로 가능한 최소한의 정리(로그, 플래그 설정)만 수행하며 `block_on`을 사용하지 않는다.
- 구조화된 동시성 라이브러리(예: `async-dropper`류) 또는 명시적 소유권 이전 패턴을 검토해 "비동기 Drop이 필요하다"는 근본 문제를 설계 단계에서 해소한다.

**탐지 방법**:
- Static: `impl Drop`의 `drop` 함수 본문에서 `block_on`, `Handle::current()`, `futures::executor::block_on` 호출을 grep.
- Manual: 비동기 정리가 필요한 리소스 타입 목록을 만들어, 각각 명시적 `close()`/`shutdown()` API와 그 호출이 보장되는 경로(항상 호출됨을 리뷰나 타입 시스템으로 확인)가 있는지 점검.

**예외**:
- 정리 작업이 순수 동기적(메모리 해제, 동기 FFI 호출)이라면 일반적인 `Drop` 구현은 전혀 문제가 없으며 오히려 권장된다. 이 항목은 "비동기 작업이 필요한 정리"에 한정된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CONC-030: spawn_blocking 미사용으로 인한 executor 고갈
**분류**: CONC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
async fn analyze_frame_batch(state: tauri::State<'_, AppState>, indices: Vec<u64>) -> Result<Vec<Analysis>, String> {
    let mut results = Vec::new();
    for idx in indices {
        // spawn_blocking 없이 async 함수 안에서 직접 CPU-bound 루프 실행
        let decoded = decode_frame_cpu_heavy(idx); // 수십 ms, 동기 함수
        let analysis = run_heuristics(&decoded);    // 추가 CPU 작업
        results.push(analysis);
    }
    Ok(results)
}
```

**문제**:
- Tokio 멀티스레드 런타임의 worker thread 수는 기본적으로 논리 CPU 수로 제한되어 있는 "귀중한 자원"이다. CPU 집약적 동기 코드를 `spawn_blocking` 없이 이 worker thread에서 직접 실행하면, 그 thread는 코루틴처럼 다른 task로 양보(yield)하지 못하고 루프가 끝날 때까지 점유된다.
- 여러 개의 Tauri 커맨드가 동시에 이런 패턴으로 호출되면 worker thread 풀 전체가 CPU 작업으로 채워질 수 있고, 이 경우 다른 모든 async task(단순 IPC 응답, 타이머, 네트워크 콜백 등)가 스레드를 배정받지 못해 응답이 완전히 멈춘 것처럼 보이는 "executor starvation"이 발생한다.
- 이 문제는 CONC-001/CONC-002와 근본 원인이 같지만, 여기서는 "여러 커맨드가 누적되어 풀 전체가 고갈되는" 시스템 레벨 증상에 초점을 둔다 — 단일 호출은 짧아 보여도 동시 호출 수가 늘어나면 급격히 악화된다.

**발생 조건**:
- 여러 UI 패널이 동시에 각자 분석/디코드 커맨드를 호출해 짧은 시간에 다수의 async task가 동시에 CPU 작업을 수행할 때.
- Tokio worker thread 수가 적게 설정된 환경(CONC-025 참고)에서 특히 빠르게 고갈된다.

**권장**:
```rust
#[tauri::command]
async fn analyze_frame_batch(state: tauri::State<'_, AppState>, indices: Vec<u64>) -> Result<Vec<Analysis>, String> {
    tokio::task::spawn_blocking(move || {
        indices.into_iter().map(|idx| {
            let decoded = decode_frame_cpu_heavy(idx);
            run_heuristics(&decoded)
        }).collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())
}
```
- CPU 집약적이거나 blocking인 작업은 예외 없이 `spawn_blocking`(전용 blocking thread pool, 기본적으로 async worker pool과 분리되어 있고 필요시 동적으로 확장됨)으로 위임해 async worker thread를 보호한다.
- 팀 차원에서 "async fn 안의 모든 non-`.await` 호출은 O(마이크로초) 이하여야 한다"는 규칙을 정하고, 이를 벗어나는 호출은 반드시 `spawn_blocking`으로 감싸도록 린트/리뷰 체크리스트에 명시한다.

**탐지 방법**:
- Structural: `async fn` 본문에서 반복문(`for`/`while`)이 `.await` 없이 여러 번 순수 CPU 연산(디코드, 픽셀 처리, 파싱)을 수행하는 패턴을 검사.
- Runtime: `tokio-console`로 worker thread의 "busy" 시간 비율과 task 큐 대기 시간을 관찰해, 특정 시점에 모든 worker가 장시간 점유되는지 확인.

**예외**:
- 작업이 마이크로초 단위로 매우 짧고 반복 횟수도 적어(예: 몇 개의 헤더 필드 파싱) 실질적으로 executor에 영향이 없다면 `spawn_blocking`의 컨텍스트 전환 오버헤드가 오히려 손해일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
