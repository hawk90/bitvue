# Anti-Pattern Catalog — PERF: 벤치마크·프로파일링·CI 안티패턴

이 문서는 더 큰 안티패턴 카탈로그의 일부입니다 (전체 목록은 `docs/anti-patterns/INDEX.md` 참고, 별도 작성 예정). 이 카테고리는 성능 버그 자체가 아니라 **성능을 측정하는 과정에서 스스로를 속이는 방식**을 다룹니다.

---

### PERF-001: release build가 아닌 결과 비교
**분류**: PERF · **심각도**: Critical · **탐지**: Structural|Manual

**나쁜 예**:
```bash
# "최적화했더니 파서가 3배 빨라졌어요!"
cargo run --bin bitstream_parser -- sample.hevc
# ...위 명령을 debug 프로파일(cargo build 기본값)로 실행하고
# 이전 버전과 시간을 비교해서 발표 자료에 붙여넣음
```
```rust
// 벤치마크 문서/README에는 이렇게만 적혀 있음:
// "AV1 OBU 파싱 성능: 12ms → 4ms (3x 개선)"
// 두 수치 모두 어떤 프로파일(debug/release)로 측정했는지 기록이 없음
```

**문제**:
- Rust debug 빌드는 오버플로 체크, 디버그 어서션, `#[inline]` 무시 등으로 release 대비 수 배~수십 배 느림. 최적화 효과가 debug/release 차이에 완전히 묻힘.
- 두 버전을 서로 다른 프로파일로 비교하면 부호(빨라졌는지 느려졌는지)조차 신뢰할 수 없음.
- CI에서 `cargo test`(기본 debug)로 나온 시간을 성능 회귀 판단에 쓰는 경우도 동일한 함정.

**발생 조건**:
- PR 설명에 "N배 빨라짐"을 급히 적을 때 (`cargo run`으로 빠르게 확인만 하고 끝냄).
- CI 로그에서 우연히 나온 타이밍 수치를 그대로 인용할 때.
- `cargo bench`가 아니라 `cargo test -- --nocapture`의 `Instant::now()` 출력을 벤치마크로 오인할 때.

**권장**:
- 모든 성능 비교는 `cargo bench --release`(Criterion은 기본적으로 release) 또는 명시적 `cargo build --release` 산출물로만 수행.
- 벤치마크 결과 리포트에 프로파일, `opt-level`, LTO 여부를 항상 명기.
- CI 성능 게이트는 debug 테스트 파이프라인과 완전히 분리된 release 전용 잡으로 구성.

**탐지 방법**:
- Manual: PR/이슈에 "빨라짐/느려짐" 수치가 있으면 어떤 빌드로 측정했는지 리뷰어가 질문.
- Structural: 성능 관련 CI 잡의 `cargo` 호출에 `--release` 또는 `cargo bench` 사용 여부 점검.

**예외**:
- 디버그 어서션 자체의 비용을 측정하려는 목적(예: "assert 오버헤드가 얼마나 되는가")일 때는 debug 빌드 비교가 의도된 것.

**Bitvue 판정**: Suspected — CI에 `cargo bench`/성능 비교 잡 자체가 없음(.github/workflows/*.yml grep 결과 bench 언급 0건, release.yml/publish-extended.yml은 `cargo build --release`만 수행), 즉 자동화된 debug/release 혼동 지점은 없지만 로컬 ad-hoc 비교를 막는 장치도 없음.

---

### PERF-002: warm cache만 측정
**분류**: PERF · **심각도**: High · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
fn bench_open_and_parse(c: &mut Criterion) {
    // 파일이 이미 OS page cache에 올라온 상태로 100회 반복
    c.bench_function("parse_mp4_container", |b| {
        b.iter(|| {
            let data = std::fs::read("fixtures/sample_4k.mp4").unwrap();
            black_box(parse_mp4(&data))
        })
    });
}
```

**문제**:
- 첫 실행 이후 파일이 OS page cache에 상주하므로 두 번째 반복부터는 디스크 I/O 비용이 사실상 0에 수렴.
- 실사용자는 콜드 상태(앱 첫 실행, 새 파일 열기)에서 겪는 지연을 벤치마크가 전혀 반영하지 못함.
- "파싱이 2ms밖에 안 걸린다"는 결론이 나오지만 실제 "파일 열기" 체감 시간은 수백 ms일 수 있음(디스크 I/O 지배).

**발생 조건**:
- 파일 열기/디코드 파이프라인 전체를 벤치마크하면서 파일 I/O도 포함했다고 착각할 때.
- 대용량 샘플(4K/8K 비트스트림)을 반복 로드하는 벤치마크에서 특히 문제가 커짐.

**권장**:
- I/O를 포함한 "파일 열기" 벤치마크와 순수 "파싱 로직" 벤치마크를 분리. 후자는 `iter_batched`로 메모리에 미리 로드한 데이터를 매 반복 재사용.
- 콜드 캐시 시나리오가 중요하면 별도로 PERF-003처럼 명시적으로 측정하고 두 수치를 나란히 보고.
- 벤치마크 이름에 `_warm`/`_cold`/`_parse_only` 등으로 범위를 명시.

```rust
c.bench_function("parse_mp4_container_only", |b| {
    b.iter_batched(
        || std::fs::read("fixtures/sample_4k.mp4").unwrap(), // setup, 측정 제외
        |data| black_box(parse_mp4(&data)),
        BatchSize::LargeInput,
    )
});
```

**탐지 방법**:
- Structural: 벤치마크 클로저 안에 `fs::read`/`File::open`이 포함되어 있는지 정적 검사.
- Manual: 벤치마크 결과와 실제 앱의 "파일 열기 체감 시간"을 사용자 테스트로 대조.

**예외**:
- 반복적으로 같은 파일을 여러 번 여는 워크플로(예: 타임라인 스크러빙 재파싱)가 실제 사용 패턴이라면 warm cache 측정이 오히려 대표성 있음 — 단, 그렇다고 명시해야 함.

**Bitvue 판정**: N/A — crates/bitvue-benchmarks/benches/*.rs (bitreader, export, magic_bytes, frame_parsing) 및 crates/bitvue-av1-codec/benches/overlay_extraction.rs 전부 `b.iter()` 밖에서 메모리 내 합성 데이터를 1회 생성하고 파일 I/O를 아예 하지 않음 — warm cache 왜곡이 발생할 파일 읽기 벤치마크 자체가 없음.

---

### PERF-003: cold cache만 측정
**분류**: PERF · **심각도**: Medium · **탐지**: Runtime|Manual

**나쁜 예**:
```bash
# "이 코덱 파서는 초당 1개 파일밖에 못 연다"
sync && sudo purge   # macOS: page cache 비우기
time ./target/release/bitvue_cli sample.ivf
# 매번 이 과정을 반복하며 "느리다"고 결론
```

**문제**:
- 반대로 매번 캐시를 비우고 측정하면 디스크 I/O 비용이 결과를 지배해서, 정작 최적화 대상인 파싱/디코드 알고리즘의 개선이 수치에 반영되지 않음.
- "파서 알고리즘을 개선했는데 왜 전체 시간이 그대로냐"는 잘못된 결론으로 이어져 실제로 유효한 최적화를 기각시킬 위험.
- SSD/HDD, 네트워크 드라이브 여부에 따라 결과가 크게 흔들려 재현성이 낮음.

**발생 조건**:
- "최악의 경우"를 보여주겠다는 의도로 매 반복 캐시를 비울 때.
- CI 러너가 매번 새 컨테이너/VM이라 사실상 항상 cold 상태인데 이를 인지하지 못하고 로컬(warm) 결과와 직접 비교할 때.

**권장**:
- I/O 바운드 구간과 CPU 바운드 구간(파싱/디코드)을 별도로 계측해서 어느 쪽을 최적화하려는지 명확히 구분.
- Cold-cache 수치를 낼 때는 재현 절차(캐시 드롭 방법, 스토리지 종류)를 명시하고 최소 5회 이상 반복해 분산을 함께 보고.
- CI 환경(항상 cold)과 로컬 개발 환경(보통 warm)의 결과를 같은 표에 섞지 않기.

**탐지 방법**:
- Manual: 벤치마크 방법론 문서에 캐시 상태 언급이 있는지 확인.
- Runtime: 동일 벤치마크를 warm/cold 두 조건으로 실행해 두 결과가 크게 다르면 어느 쪽이 보고되고 있는지 대조.

**예외**:
- "앱을 처음 켜고 첫 파일을 여는" 시나리오가 제품 요구사항의 핵심 지표(예: 첫 인상 latency SLA)라면 cold-cache 전용 측정이 정당함 — 단 CPU 최적화 검증용으로 재사용하면 안 됨.

**Bitvue 판정**: N/A — PERF-002와 동일한 이유로 벤치마크에 파일 I/O가 전혀 없어 cold-cache 시나리오도 구조적으로 발생하지 않음.

---

### PERF-004: 작은 샘플 하나로 결론
**분류**: PERF · **심각도**: High · **탐지**: Manual|Structural

**나쁜 예**:
```rust
// benches/hevc_bench.rs
fn bench_hevc(c: &mut Criterion) {
    let data = include_bytes!("../fixtures/small_320x240_10frames.hevc");
    c.bench_function("hevc_slice_parse", |b| {
        b.iter(|| black_box(parse_slice_header(data)))
    });
}
// 결론: "HEVC 슬라이스 헤더 파싱 최적화 완료, 40% 개선"
// -> 실제로는 4K 60fps, B-프레임 많은 실제 방송 스트림에서 회귀 발생
```

**문제**:
- 320x240·10프레임짜리 합성/축소 샘플은 실제 워크로드의 분기 분포(슬라이스 타입, MB/CU 크기 분포, reference 개수)를 대표하지 못함.
- 작은 입력은 캐시에 완전히 들어가므로 메모리 대역폭/캐시 미스 비용이 실제 대형 파일과 질적으로 다름.
- 단일 샘플에 대한 개선이 다른 프로파일(해상도, 프레임 수, 코덱 프로파일)에서는 반대로 회귀일 수 있음.

**발생 조건**:
- 테스트 픽스처를 저장소 용량 때문에 의도적으로 작게 유지했는데, 그 샘플을 성능 벤치마크에도 재사용할 때.
- "일단 하나 돌려보고" PR에 결과를 적을 때 — 벤치마크 스위트를 만들 시간이 없어서.

**권장**:
- 최소한 해상도(SD/HD/4K), 프레임 수(짧은/긴), 코덱 프로파일(Baseline/Main/High 등) 축으로 다양화한 샘플 세트를 벤치마크 그룹으로 구성.
- `criterion_group!`으로 여러 입력 크기를 파라미터화하고 `BenchmarkGroup`으로 함께 리포트.
- 결론을 쓸 때 "어떤 샘플 집합에서" 개선되었는지 항상 명시.

```rust
let mut group = c.benchmark_group("hevc_parse");
for name in ["sd_short", "hd_long", "4k_bframes_heavy"] {
    let data = load_fixture(name);
    group.bench_with_input(BenchmarkId::from_parameter(name), &data, |b, d| {
        b.iter(|| parse_bitstream(d))
    });
}
group.finish();
```

**탐지 방법**:
- Structural: `benches/` 디렉토리에서 사용되는 고유 fixture 파일 개수와 다양성(해상도/길이) 점검.
- Manual: 성능 개선 PR에 첨부된 샘플이 1개뿐인지 리뷰.

**예외**:
- 특정 버그(예: "이 한 파일에서만 크래시 직전까지 느려짐")를 재현하는 회귀 테스트라면 단일 샘플이 목적에 맞음 — 일반화된 성능 주장과는 구분해서 보고.

**Bitvue 판정**: Confirmed — crates/bitvue-av1-codec/benches/overlay_extraction.rs:16-30 `create_test_obu_data()`는 ~110바이트짜리 손으로 만든 단일 합성 OBU만 사용해 모든 grid 벤치마크에 재사용하고, frame_parsing.rs:64-72의 `bench_av1_obu_iterator`도 10바이트 단일 합성 블롭 하나뿐 — 해상도/길이/프로파일 다양화 없음.

---

### PERF-005: synthetic bitstream만 사용
**분류**: PERF · **심각도**: Medium · **탐지**: Manual|Structural

**나쁜 예**:
```rust
// 테스트 헬퍼로 만든 "적당히 그럴듯한" 비트스트림
fn make_synthetic_av1_obu(size: usize) -> Vec<u8> {
    let mut v = vec![0u8; size];
    v[0] = 0x0A; // OBU 헤더 흉내
    // 나머지는 그냥 0으로 채움 — 실제 인코더가 만든 엔트로피 분포와 무관
    v
}

fn bench_obu_parse(c: &mut Criterion) {
    let data = make_synthetic_av1_obu(1_000_000);
    c.bench_function("av1_obu_parse", |b| b.iter(|| parse_obu(&data)));
}
```

**문제**:
- 실제 인코더(x264/x265/aom/libvpx) 산출물은 엔트로피 코딩 분포, 분기 빈도(예: skip/merge MB 비율), 참조 프레임 패턴이 합성 데이터와 크게 다름.
- 0으로 채운/반복 패턴의 데이터는 CABAC/CDF 분기 예측기, 분기 예측(branch predictor), 데이터 의존적 캐시 접근 패턴을 비현실적으로 단순화시켜 실제보다 훨씬 빠른(또는 특정 분기에 편향된) 결과를 냄.
- 실제 스트림에서만 발생하는 엣지 케이스(긴 run, 드문 신택스 조합)를 성능 측정에서 완전히 놓침.

**발생 조건**:
- 실제 샘플 확보/라이선스 문제로 테스트용 합성 데이터를 급조했는데, 단위 테스트용 데이터를 성능 벤치마크에도 그대로 재사용할 때.
- 신택스 구조체 파싱만 특정해서 테스트하려다 상위 계층(엔트로피 디코딩)과의 상호작용을 놓칠 때.

**권장**:
- 성능 벤치마크는 반드시 실제 인코더로 생성했거나 실전에서 수집한(라이선스 확인된) 스트림 세트를 사용.
- 합성 데이터는 "파서 로직의 정확성"을 검증하는 단위 테스트 용도로만 한정하고, 벤치마크 스위트와 물리적으로 분리된 디렉토리/네이밍을 사용.
- 대표 코덱별로 공개 conformance 스트림 또는 오픈 라이선스 샘플(Xiph, 각 코덱 표준화 기구의 conformance set 등)을 벤치마크 코퍼스로 관리.

**탐지 방법**:
- Structural: 벤치마크가 참조하는 fixture가 `make_synthetic_*`/`vec![0u8; N]` 류의 생성 함수인지, 실제 파일 경로인지 확인.
- Manual: 벤치마크 코퍼스 출처(인코더, 실전 캡처 여부)를 문서화했는지 리뷰.

**예외**:
- 파서의 경계값 처리(최대 길이 필드, 특정 신택스 조합의 유무)를 의도적으로 스트레스 테스트하려는 fuzzing/edge-case 벤치마크는 합성 데이터가 적절 — 단 "일반적 성능"으로 일반화하지 않는다는 전제.

**Bitvue 판정**: Confirmed — frame_parsing.rs:64-72 및 overlay_extraction.rs:16-30의 `create_test_obu_data()`/`obu_data`는 실제 인코더 산출물이 아니라 0으로 채운/손으로 조립한 가짜 OBU 바이트(주석조차 "파싱 에러가 날 수 있지만 iteration만 벤치마크"라고 명시)를 사용.

---

### PERF-006: wall time만 측정
**분류**: PERF · **심각도**: Medium · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
let start = Instant::now();
let result = decode_frame(&packet);
println!("decode took: {:?}", start.elapsed());
// CPU 사용률, 스레드 수, GC/allocator 압박 등은 전혀 기록하지 않음
```

**문제**:
- Wall time(경과 시간)은 병렬 작업, 백그라운드 스레드, OS 스케줄링, 다른 프로세스와의 경합에 영향을 받아 "실제 코드가 한 일"과 "기다린 시간"을 구분하지 못함.
- 예: 4코어를 다 쓰는 병렬 디코드가 wall time은 짧아 보여도 CPU-time(코어·초)은 오히려 증가했을 수 있음 — 배터리/발열/멀티태스킹 환경에서는 나쁜 트레이드오프.
- 락 경합으로 인한 대기 시간이 "느려짐"으로 잘못 해석되거나, 반대로 다른 코어가 놀고 있어서 실제 처리 효율이 나쁜데도 wall time만 보면 문제없어 보임.

**발생 조건**:
- 멀티스레드 디코드/파싱 경로의 성능을 `Instant::now()` 하나로만 판단할 때.
- CI 러너가 다른 잡과 CPU를 공유하는 상황에서 wall time을 그대로 회귀 판정 기준으로 쓸 때.

**권장**:
- CPU 시간(`/usr/bin/time -v`의 user+sys, 또는 `getrusage`)과 wall time을 함께 기록.
- 멀티스레드 코드는 스레드별 활성 시간, 락 대기 시간을 tracing/`tokio-console`/`perf sched` 등으로 별도 계측.
- Criterion은 기본적으로 wall time 기준이므로, CPU-bound 비교가 중요한 경로는 `perf stat -e task-clock,cycles,instructions`로 보완.

**탐지 방법**:
- Runtime: 동일 벤치마크를 CPU 부하가 있는 환경과 없는 환경에서 각각 실행해 wall time 변동폭 확인.
- Structural: 성능 계측 코드에 `Instant::now()`만 있고 CPU/스케줄링 관련 계측이 전혀 없는지 검토.

**예외**:
- 사용자 체감 지표(예: "버튼을 눌러서 프레임이 뜨기까지 걸리는 시간")는 정의상 wall time이 맞는 지표 — 이 경우엔 wall time만으로 충분하며 CPU time은 부가 정보.

**Bitvue 판정**: Confirmed — crates/bitvue-engine/src/performance.rs(구 bitvue-core, 크레이트 리네임 반영해 경로 갱신)의 `PerfTracker`는 `Instant::now()`(L372, L384)만 사용하고 `MetricSummary`(L573-608)는 avg_ms/min/max/total_ms만 집계 — 저장소 전체에 CPU-time/getrusage 계측이 전무(grep 0건).

---

### PERF-007: allocation 수 미측정
**분류**: PERF · **심각도**: Medium · **탐지**: Runtime|Static

**나쁜 예**:
```rust
fn parse_nal_units(data: &[u8]) -> Vec<NalUnit> {
    let mut units = Vec::new(); // 용량 미지정, 반복적으로 재할당
    for chunk in split_by_start_code(data) {
        units.push(NalUnit::parse(chunk.to_vec())); // chunk마다 새 Vec 할당
    }
    units
}
// 벤치마크는 시간만 보고 "충분히 빠르다"고 판단, 할당 횟수는 확인 안 함
```

**문제**:
- 시간만 보면 통과하는 벤치마크도 할당 횟수가 과도하면 실제 대형 파일(수만 개 NAL 유닛)에서 allocator 경합, 메모리 단편화로 상황이 악화됨.
- 멀티스레드 환경에서 전역 allocator(락 기반) 경합이 심해져 코어 수를 늘려도 스케일링이 깨지는 원인이 될 수 있는데, wall-time 벤치마크만으로는 이를 발견하기 어려움.
- "할당 수를 줄였다"는 최적화 PR이 실제로 효과가 있었는지 확인할 방법이 없어 회귀를 감지하지 못함.

**발생 조건**:
- 파싱/디코드 핫패스에서 프레임/NAL 단위마다 `Vec::new()`, `to_vec()`, `String::from()`을 반복 호출할 때.
- "시간이 안 늘었으니 문제없다"고 판단하고 allocation 프로파일을 아예 보지 않을 때.

**권장**:
- `dhat`(dhat-rs) 또는 커스텀 카운팅 allocator를 벤치마크/테스트 빌드에 연결해 반복당 할당 횟수·총 바이트를 리포트.
- Criterion 벤치마크에 할당 카운터를 함께 출력하는 커스텀 measurement 또는 별도 assertion(예: "이 함수는 반복당 N회 이하만 할당해야 한다")을 CI 가드로 추가.
- 핫패스는 `Vec::with_capacity`, 버퍼 재사용(`clear()` 후 재사용), arena/pool allocator로 할당 횟수 자체를 구조적으로 줄이기.

```rust
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn parse_nal_units_allocation_budget() {
    let _profiler = dhat::Profiler::new_heap();
    let data = load_fixture("sample.h264");
    let stats_before = dhat::HeapStats::get();
    let _ = parse_nal_units(&data);
    let stats_after = dhat::HeapStats::get();
    assert!(stats_after.total_blocks - stats_before.total_blocks < 1000);
}
```

**탐지 방법**:
- Runtime: dhat/valgrind massif로 할당 프로파일을 주기적으로 수집해 추세 추적.
- Static: 핫패스로 알려진 함수 내부에서 루프마다 `Vec::new`/`to_vec`/`clone` 호출 여부를 린트/grep으로 점검.

**예외**:
- 초기화(setup) 단계나 드물게 호출되는 경로(파일 열기 1회)의 할당은 반복당 비용이 아니므로 엄격한 예산 적용 대상이 아님.

**Bitvue 판정**: Suspected — 저장소 전체에 dhat/heaptrack 등 할당 프로파일링 의존성이 전혀 없음(grep 0건)이라는 구조적 증거는 있으나, 특정 핫패스의 실제 과도한 할당을 직접 측정해 확인하지는 못함.

---

### PERF-008: peak RSS 미측정
**분류**: PERF · **심각도**: Medium · **탐지**: Runtime|Manual

**나쁜 예**:
```rust
// "8K HDR 스트림 디코드 최적화 완료, 프레임당 2ms"
// -> 실제로는 전체 프레임을 미리 다 메모리에 올려두는 방식으로 바꿔서
//    시간은 빨라졌지만 peak RSS가 300MB -> 4GB로 폭증
//    저사양 머신에서 OOM으로 앱이 죽음
```

**문제**:
- 시간(latency/throughput)만 최적화 지표로 삼으면 "메모리를 시간과 맞바꾸는" 변경(전량 캐싱, 프리페치, 메모이제이션)이 무제한으로 허용됨.
- Peak RSS는 특히 대형 프레임 버퍼를 다루는 비디오 분석기에서 OOM-killer, 스왑 발적 등 치명적 실패로 직결.
- CI 벤치마크 머신은 보통 메모리가 넉넉해 문제가 드러나지 않다가 실사용자의 저사양 환경에서만 터지는 경우가 많음.

**발생 조건**:
- 프레임 캐시, lookahead 버퍼, 디코드 파이프라인 깊이를 늘려 처리량을 높이는 최적화를 할 때.
- 대형 해상도(4K/8K) 스트림, 긴 GOP, 다중 레퍼런스 프레임을 다루는 코덱(HEVC/AV1/VVC) 벤치마크에서 특히 중요.

**권장**:
- 벤치마크/CI에 peak RSS 측정을 필수 지표로 추가 (`/usr/bin/time -v` "Maximum resident set size", macOS는 `/usr/bin/time -l`의 "maximum resident set size").
- 시간과 메모리를 함께 표로 리포트하고, 메모리 예산(예: "1080p 스트림당 peak RSS 500MB 이하")을 회귀 게이트로 설정.
- 메모리 사용량이 프레임 수/해상도에 비례해 무한정 느는지, 상한이 있는지(bounded buffer) 명시적으로 테스트.

**탐지 방법**:
- Runtime: CI에서 `/usr/bin/time -v` 또는 `heaptrack`/`massif`로 peak RSS를 매 실행 기록하고 추세 그래프화.
- Manual: "빨라졌다"고 보고된 PR에 메모리 사용량 변화가 함께 보고되었는지 리뷰 체크리스트에 포함.

**예외**:
- 메모리가 풍부하다고 명시적으로 가정된 데스크톱 전용 "고성능 모드" 옵션이 사용자 opt-in이라면 peak RSS 상한을 느슨하게 둘 수 있음 — 단 기본 모드와는 분리.

**Bitvue 판정**: Suspected — `/usr/bin/time`, heaptrack, massif 등 peak RSS 계측이 CI/scripts 어디에도 없음(grep 0건)이며 ByteCache(256MB 예산)와 달리 dav1d/ffmpeg/vvdec 디코드 경로의 프레임 버퍼 메모리 상한을 검증하는 테스트/게이트가 안 보임 — 다만 실제 OOM 사례를 직접 확인하지는 못함.

---

### PERF-009: UI latency 미측정
**분류**: PERF · **심각도**: High · **탐지**: Runtime|Manual

**나쁜 예**:
```rust
// Tauri 커맨드 자체는 벤치마크로 3ms까지 최적화됨
#[tauri::command]
fn get_frame_hex_data(offset: u64, size: u32) -> Vec<u8> {
    // ... 3ms 내외
}
```
```
// 그러나 프론트엔드에서 실제로 느껴지는 지연은 이렇게 측정된 적이 없음:
// 사용자 클릭 -> React 이벤트 -> invoke() -> IPC 직렬화 -> Rust 처리
//   -> IPC 역직렬화 -> React 리렌더 -> 브라우저 페인트
// Rust 커맨드 벤치마크 3ms만 보고 "충분히 빠르다"고 결론
```

**문제**:
- Rust 커맨드 함수 자체의 실행 시간과 사용자가 체감하는 "클릭부터 화면 갱신까지"의 시간은 완전히 다른 지표. IPC 직렬화/역직렬화, React 리렌더, 레이아웃/페인트가 총 지연의 대부분을 차지할 수 있음.
- 대용량 데이터(hex dump, overlay 좌표 배열 등)를 JSON으로 직렬화해 IPC로 넘기면 Rust 쪽 처리 시간보다 직렬화·전송 비용이 훨씬 클 수 있는데, 백엔드 벤치마크만으로는 이를 놓침.
- "백엔드는 빠른데 앱이 느리다"는 사용자 불만이 발생해도 원인을 못 찾는 상황으로 이어짐(측정 지점이 아예 없으므로).

**발생 조건**:
- Rust 크레이트 단위 Criterion 벤치마크만 성능 지표로 채택하고 프론트엔드/E2E 지연은 별도로 측정하지 않을 때.
- Overlay 렌더링, 프레임 스크러빙 등 사용자 상호작용이 잦은 UI 경로에서 특히 체감 차이가 큼.

**권장**:
- Playwright 등 E2E 도구로 "클릭 → 화면 갱신 완료"까지의 실측 latency를 별도 지표로 수집.
- 프론트엔드에서 `performance.mark`/`performance.measure`로 invoke 호출 전후, 리렌더 완료 시점을 계측하고 이를 CI 아티팩트로 축적.
- IPC 페이로드 크기와 (역)직렬화 비용을 백엔드 처리 시간과 분리해서 리포트(PERF-014와 연계).

**탐지 방법**:
- Runtime: E2E/브라우저 성능 API 기반 measurement가 CI 성능 스위트에 존재하는지 확인.
- Manual: "성능 개선"을 주장하는 PR이 Rust 벤치마크 수치만 인용하고 UI 체감 지표가 없는지 리뷰.

**예외**:
- 순수 백엔드 배치 작업(파일 인덱싱, 백그라운드 프리페치)처럼 사용자가 직접 기다리지 않는 경로는 UI latency보다 처리량이 더 적절한 지표.

**Bitvue 판정**: Confirmed — Playwright 등 E2E 프레임워크가 저장소에 전혀 없고(find 0건) CI에도 UI latency 게이트가 없음; frontend/services/tauriCommandService.ts:78-99(파일명은 Tauri 시절 이름 그대로지만 실제로는 Electron IPC 브리지 — 재확인함)의 `performance.now()` 기반 invoke latency 로깅은 존재하지만 콘솔 디버그 로그일 뿐 CI에 통합되거나 회귀 게이트로 쓰이지 않음.
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-011 참고.

---

### PERF-010: p50만 보고 p95/p99 무시
**분류**: PERF · **심각도**: High · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
c.bench_function("seek_to_frame", |b| {
    b.iter(|| seek_to_frame(&index, target_frame))
});
// Criterion 콘솔 출력의 "time: [1.2ms 1.3ms 1.4ms]" 중
// 가운데(평균 근사치)만 보고 "충분히 빠르다"고 커밋 메시지에 기록
// 실제로는 키프레임이 멀리 떨어진 위치로 seek할 때 p99가 80ms까지 튐
```

**문제**:
- 비디오 탐색(seek)처럼 입력(목표 프레임 위치, 키프레임과의 거리)에 따라 분산이 매우 큰 연산은 평균/중앙값이 최악의 경우를 완전히 숨김.
- 사용자는 "가끔 스크러빙이 버벅인다"고 체감하는데, p50 기준 벤치마크는 이 문제를 절대 재현하지 못하므로 회귀를 통과시킴.
- 꼬리 지연(tail latency)은 특히 인터랙티브 UI에서 "느리다"는 인상을 결정짓는 주된 요인인데 이를 아예 안 보는 셈.

**발생 조건**:
- Criterion 기본 출력의 요약 라인만 확인하고 전체 분포(히스토그램, HTML 리포트)를 열어보지 않을 때.
- Seek, 캐시 미스가 있는 조회, 네트워크/디스크 I/O가 섞인 연산처럼 분산이 큰 워크로드를 단일 대표값으로만 보고할 때.

**권장**:
- Criterion의 HTML 리포트(`--output-format html`) 또는 raw sample을 활용해 p50/p95/p99를 명시적으로 계산하고 함께 보고.
- Seek처럼 입력 특성에 따라 비용이 크게 갈리는 연산은 "키프레임 인접", "키프레임 원거리" 등 케이스를 나눠 각각 분포를 측정(PERF-004와 연계).
- CI 회귀 게이트를 p50이 아니라 p95/p99 기준으로 설정해 꼬리 악화를 조기에 잡기.

**탐지 방법**:
- Structural: 성능 리포트/대시보드가 단일 숫자(평균/중앙값)만 노출하는지, 분위수 브레이크다운이 있는지 확인.
- Runtime: 동일 벤치마크의 raw sample을 분위수로 재분석해 p50과 p99 격차가 큰데도 보고서엔 p50만 있는 경우를 탐지.

**예외**:
- 입력 분산이 거의 없는 순수 CPU 바운드 루프(예: 고정 크기 버퍼 CRC 계산)는 p50과 p99가 실질적으로 같으므로 단일 값 보고로 충분.

**Bitvue 판정**: Confirmed — crates/bitvue-engine/src/performance.rs(구 bitvue-core, 경로 갱신)의 `MetricSummary`(avg/min/max만, L587)와 frontend/services/tauriCommandService.ts의 `getLatencyStats()`(L209 기준 avg/min/max만) 둘 다 p50/p95/p99 등 분위수를 전혀 계산하지 않음.
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-011 참고.

---

### PERF-011: throughput과 latency 혼동
**분류**: PERF · **심각도**: Medium · **탐지**: Manual|Structural

**나쁜 예**:
```rust
// "디코더가 초당 240프레임 처리 가능 (throughput)"
// 이 수치를 그대로 "프레임 하나 디코드에 4.1ms 걸린다"(latency)로 홍보 자료에 인용
// 실제로는 8-way 파이프라인 병렬 디코드로 얻은 throughput이라
// 개별 프레임 하나의 latency는 30ms 이상일 수 있음
```

**문제**:
- Throughput(단위 시간당 처리량)은 파이프라이닝/배치/병렬화로 개선 가능하지만, 개별 요청의 latency(하나의 프레임이 요청부터 완료까지 걸리는 시간)는 오히려 늘어날 수 있음(버퍼링, 배치 대기).
- "초당 N프레임"을 "프레임당 1/N초"로 단순 역수 변환하면 파이프라인 깊이/배치 크기를 무시하게 되어 실시간 재생·인터랙티브 스크러빙처럼 latency가 중요한 시나리오의 실제 체감을 잘못 예측.
- 반대로 throughput을 올리려고 배치를 키우면 단일 프레임 seek/미리보기 같은 latency-critical 경로가 체감상 느려지는 트레이드오프를 놓치기 쉬움.

**발생 조건**:
- 배치/파이프라인 디코드 벤치마크 결과를 단일 프레임 응답성 지표로 재사용해 보고할 때.
- "초당 X" 형태의 숫자 하나로 성능을 요약하려는 압박이 있을 때(마케팅 자료, 릴리즈 노트).

**권장**:
- Throughput과 latency를 항상 별도 벤치마크·별도 숫자로 보고하고, 어떤 배치 크기/파이프라인 깊이에서 측정했는지 명시.
- 인터랙티브 경로(단일 프레임 요청, seek)는 배치 크기 1에서의 latency를 따로 측정.
- 배치 크기에 따른 throughput-latency 트레이드오프 곡선을 그려서 어느 지점을 택했는지 근거를 남기기.

**탐지 방법**:
- Manual: "초당 N" 수치가 latency 주장에 재사용되고 있는지 리뷰(단위 변환의 암묵적 가정 확인).
- Structural: 벤치마크 함수명/보고서에 throughput과 latency 벤치마크가 명확히 구분되어 있는지 점검.

**예외**:
- 배치 크기 1로 고정된 파이프라인(파이프라이닝이 없는 단순 순차 처리)이라면 throughput의 역수가 latency와 사실상 같으므로 혼용해도 무방.

**Bitvue 판정**: Suspected — bitreader.rs/export.rs는 `Throughput::Bytes`/`Throughput::Elements`로 처리량을 보고하지만 동일 연산에 대한 단일 아이템 latency 벤치마크가 별도로 없어 두 지표가 뒤섞여 보고될 위험이 구조적으로 존재하나, 실제로 처리량 수치가 latency 주장으로 재인용된 사례는 찾지 못함.

---

### PERF-012: 첫 프레임과 steady-state 혼동
**분류**: PERF · **심각도**: Medium · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
c.bench_function("decode_100_frames", |b| {
    b.iter(|| {
        let mut decoder = Decoder::new(&stream_config); // 매 반복 새로 생성!
        for _ in 0..100 {
            decoder.decode_next_frame();
        }
    })
});
// 매 반복마다 디코더 초기화(SPS/PPS 파싱, 테이블 구축, 워밍업)가 포함되어
// "프레임당 평균 시간"에 초기화 비용이 섞여 들어감
```

**문제**:
- 디코더/파서 초기화(구성 파싱, 룩업 테이블 생성, 스레드 풀 기동 등)는 1회성 비용인데 이를 매 반복 포함시키면 steady-state(안정 상태) 처리량이 실제보다 나빠 보임.
- 반대로 실제 사용자가 매번 새 스트림을 여는 시나리오(짧은 클립을 계속 갈아타는 워크플로)라면 초기화 비용을 배제한 steady-state 수치가 오히려 비현실적으로 낙관적.
- 두 경우를 구분하지 않고 "프레임당 X ms"라는 단일 숫자로 뭉뚱그리면 어느 쪽 최적화가 실제로 중요한지 판단할 수 없음.

**발생 조건**:
- Criterion `b.iter()` 클로저 안에 상태 초기화(디코더/파서 생성)와 반복 작업을 함께 넣을 때.
- 긴 GOP 스트림에서 "첫 프레임"과 "이후 프레임"의 비용 구조가 본질적으로 다른데(키프레임 vs P/B프레임) 이를 뭉쳐서 평균 낼 때.

**권장**:
- 초기화 비용은 `iter_batched`/`iter_with_setup`으로 측정 범위 밖에 두고, steady-state 루프만 측정.
- 초기화 비용 자체가 궁금하면 별도 벤치마크(`decoder_init`)로 분리해서 명시적으로 보고.
- 첫 프레임(키프레임, 초기화 포함)과 이후 프레임(steady-state)의 시간을 각각 리포트하고 합산하지 않기.

```rust
c.bench_function("decode_100_frames_steady_state", |b| {
    b.iter_batched(
        || Decoder::new(&stream_config), // 초기화는 setup에서, 측정 제외
        |mut decoder| {
            for _ in 0..100 {
                decoder.decode_next_frame();
            }
        },
        BatchSize::SmallInput,
    )
});
```

**탐지 방법**:
- Structural: `b.iter(|| { ... })` 클로저 내부에 `::new(`/생성자 호출과 반복 로직이 함께 있는지 점검.
- Runtime: 반복 횟수를 10 vs 1000으로 바꿔 프레임당 평균이 유의미하게 달라지면 초기화 비용 혼입 의심.

**예외**:
- "스트림 하나를 열어서 한 번만 처리하고 끝나는" 워크로드(예: 배치 변환 CLI에서 파일마다 새 프로세스)가 실사용 패턴이라면 초기화 비용을 포함한 end-to-end 측정이 오히려 정확함.

**Bitvue 판정**: N/A — 저장소에 다중 프레임을 반복 디코드하며 매 반복 `Decoder::new()`를 호출하는 형태의 벤치마크가 아예 없음(frame_parsing.rs/overlay_extraction.rs는 단일 헤더/단일 프레임 1회 파싱만 측정).

---

### PERF-013: decode 시간을 parser 시간에 포함
**분류**: PERF · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```rust
fn bench_parse_bitstream(c: &mut Criterion) {
    c.bench_function("parse_hevc_nal", |b| {
        b.iter(|| {
            let nal = parse_nal_header(&data);
            let slice = parse_slice_header(&nal);
            // 여기서부터는 사실 "파싱"이 아니라 픽셀 재구성(디코드)임
            let frame = reconstruct_frame(&slice, &reference_frames);
            black_box(frame)
        })
    });
}
// 벤치마크 이름은 "parse_hevc_nal"인데 실제로는 CABAC 디코딩 + 역변환 +
// 인루프 필터까지 포함되어 있어 "파서가 느리다"는 잘못된 결론으로 이어짐
```

**문제**:
- 신택스 파싱(구조체/필드 추출)과 실제 디코드(엔트로피 디코딩, 역양자화/역변환, 움직임 보상, 인루프 필터)는 계산 복잡도가 자릿수 단위로 다름. 둘을 한 벤치마크에 섞으면 "파서 최적화"가 전체 시간에 거의 영향을 못 주는데도 디코드 비용 때문에 개선이 안 보이는 것처럼 오판됨.
- 반대로 파서 계층에 버그로 인한 회귀가 생겨도 디코드 시간에 묻혀 CI가 못 잡아냄.
- Bitvue처럼 "파서/분석기"가 핵심 가치인 프로젝트에서 이 구분이 특히 중요 — 순수 파싱 성능이 제품의 핵심 지표인데 디코드 성능과 뭉쳐 측정하면 실제 병목을 오인함.

**발생 조건**:
- 파서 벤치마크를 만들 때 편의상 기존 디코드 파이프라인의 최상위 함수를 그대로 호출할 때.
- "파싱"과 "디코딩"의 경계가 코드 상 명확히 분리되어 있지 않을 때(같은 함수 안에 신택스 읽기와 픽셀 계산이 섞여 있는 구조).

**권장**:
- 신택스 파싱 전용 함수(구조체만 반환, 픽셀 데이터 미생성)와 픽셀 재구성 함수를 API 레벨에서 분리하고, 각각 별도 Criterion 벤치마크 그룹으로 관리.
- 벤치마크 함수명에 "parse_only"/"decode_full" 등으로 범위를 명시.
- 파서 전용 벤치마크는 신택스 트리/필드 값만 `black_box`로 소비하고 픽셀 버퍼를 만들지 않는지 코드 리뷰에서 확인.

**탐지 방법**:
- Structural: `parse_*` 이름의 벤치마크 함수가 픽셀/프레임 버퍼를 생성하는 함수(예: `reconstruct_frame`, `apply_loop_filter`)를 호출하는지 grep.
- Manual: 벤치마크 결과와 프로파일러(perf/flamegraph) 결과를 대조해 "parse" 벤치마크의 시간 대부분이 실제로 디코드 함수에서 소비되는지 확인.

**예외**:
- 프로젝트 목표가 애초에 "end-to-end 디코드 성능"이라면 파싱+디코드를 합쳐 측정하는 것이 맞음 — 이름을 `decode_full` 등으로 명확히 하면 문제없음.

**Bitvue 판정**: N/A — 현재 벤치마크(bitreader, frame_parsing, overlay_extraction)는 신택스 파싱(`ObuIterator`, `ParsedFrame::parse`, grid 추출)만 호출하고 픽셀 재구성/CABAC 디코드를 수행하는 bitvue-decode(dav1d/ffmpeg/vvdec)는 어떤 벤치마크에서도 호출되지 않음.

---

### PERF-014: IPC와 frontend 시간을 제외
**분류**: PERF · **심각도**: High · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
// src-tauri/src/commands/frame.rs
#[tauri::command]
fn get_frame_hex_data(offset: u64, size: u32) -> Result<Vec<u8>, String> {
    let start = Instant::now();
    let data = read_frame_bytes(offset, size)?;
    log::info!("get_frame_hex_data took {:?}", start.elapsed()); // 3ms
    Ok(data) // 여기서 Vec<u8> -> JSON 배열 직렬화 비용은 계측 안 됨
}
```
```
// "커맨드가 3ms" -> 릴리즈 노트에 "프레임 조회 3ms" 라고 기재
// 실측 사용자 체감(DevTools Network/Performance 탭 기준)은 45ms
// 원인: Vec<u8> 수십 KB를 JSON 배열로 직렬화 + IPC 전송 + JS 파싱이 40ms
```

**문제**:
- Tauri IPC는 기본적으로 JSON 직렬화를 거치므로, 바이너리에 가까운 데이터(hex dump, 픽셀 버퍼)를 `Vec<u8>`로 그대로 반환하면 직렬화 비용이 Rust 함수 본체보다 훨씬 커질 수 있음.
- Rust 쪽 `Instant::now()` 계측은 커맨드 함수 리턴 시점에서 끝나므로, 그 이후 Tauri 런타임의 직렬화·IPC 전송·프론트엔드 역직렬화 비용이 통째로 누락됨.
- "백엔드는 빠른데 앱은 느리다"는 상황이 반복되고, 최적화 노력이 병목이 아닌 곳(Rust 함수 내부)에 계속 투입됨.

**발생 조건**:
- 대용량 바이너리/배열 데이터를 Tauri command의 리턴값으로 그대로 전달할 때(특히 hex view, overlay 좌표 배열처럼 데이터량이 큰 기능).
- 성능 계측을 Rust 로그(`log::info!`, `tracing`)로만 하고 프론트엔드 쪽 계측이 없을 때.

**권장**:
- 대용량 바이너리 데이터는 JSON IPC 대신 Tauri의 raw byte response(`tauri::ipc::Response`) 또는 커스텀 프로토콜/스트리밍 방식을 사용해 직렬화 비용 자체를 없애기.
- 프론트엔드에서 `invoke()` 호출 전후로 `performance.now()`를 찍어 IPC 왕복 전체 시간을 별도 지표로 수집(PERF-009와 연계).
- 벤치마크/로그에 "Rust 함수 내부 시간"과 "IPC 왕복 총 시간"을 분리된 두 숫자로 항상 병기.

**탐지 방법**:
- Structural: `#[tauri::command]` 함수의 리턴 타입이 대용량 `Vec<u8>`/`Vec<T>`인데 raw response API를 안 쓰는 경우를 grep.
- Runtime: 브라우저 DevTools Performance 탭 또는 프론트엔드 `performance.measure`로 실측한 IPC 왕복 시간과 Rust 로그 시간의 차이를 비교.

**예외**:
- 리턴 데이터가 원래 작은 스칼라/짧은 구조체(수 바이트~수백 바이트)라면 직렬화 비용이 무시할 만한 수준이므로 Rust 내부 시간만으로도 충분.

**Bitvue 판정**: N/A(재확인, 이전 판정 폐기) — 원 인용 경로 `src-tauri/src/commands/frame.rs`는 Tauri→Electron sidecar 전환 완료로 저장소에서 완전히 삭제됨(src-tauri 디렉터리 자체가 없음). 후속 아키텍처(crates/bitvue-sidecar/src/main.rs)의 `get_hex_range`(L1304-1358)와 `get_decoded_frame_yuv`(L1376-)는 대용량 바이트 데이터를 "Control 메타 프레임 + Data 프레임(raw bytes, JSON 배열도 base64도 아님)"으로 명시적으로 분리 반환하도록 설계 주석(L9-10, L1373)에서 이 안티패턴을 의도적으로 피했다고 밝힘 — 즉 지적된 패턴이 현재 코드베이스에서 구조적으로 해소됨. 예외: `get_thumbnails`(L1479-)는 여전히 base64-in-JSON을 쓰지만 PNG 썸네일 크기가 작아 의도적 선택이라고 코드 주석(L1476 부근)에 명시됨 — 대용량 페이로드가 아니라 이 항목의 대상이 아님.

---

### PERF-015: thread 수 고정 없이 비교
**분류**: PERF · **심각도**: Medium · **탐지**: Runtime|Manual

**나쁜 예**:
```rust
// 로컬 개발 머신(8코어)에서 측정
c.bench_function("parallel_frame_decode_rayon", |b| {
    b.iter(|| {
        frames.par_iter().for_each(|f| decode_frame(f)); // 기본 thread pool = 논리 코어 수
    })
});
// 다음 날 CI 러너(2 vCPU)에서 같은 벤치마크를 돌리고
// "CI에서 회귀했다"고 알람 -> 실제로는 스레드 수 차이일 뿐
```

**문제**:
- Rayon 기본 스레드 풀은 논리 코어 수를 그대로 사용하므로, 서로 다른 머신(로컬 8코어 vs CI 2vCPU, 사용자 노트북 4코어)에서 결과가 코어 수 비율만큼 달라짐 — 알고리즘 변경과 무관한 차이가 "성능 변화"로 오인됨.
- 동일 머신이라도 다른 프로세스가 코어를 점유하고 있으면 실제 사용 가능한 병렬성이 달라져 재현성이 깨짐.
- "병렬화했더니 빨라졌다"는 주장이 실은 단순히 더 많은 코어를 썼을 뿐, 코어당 효율은 오히려 나빠졌을 수 있음(PERF-021과 연계).

**발생 조건**:
- 로컬 머신과 CI 러너의 코어 수가 다른데 같은 벤치마크 수치를 직접 비교할 때.
- Rayon/std::thread 기반 병렬 코드의 스레드 수를 명시적으로 고정하지 않고 기본값에 의존할 때.

**권장**:
- 벤치마크는 `RAYON_NUM_THREADS` 환경변수 또는 `ThreadPoolBuilder::num_threads()`로 스레드 수를 고정하고, 그 값을 결과에 항상 명기.
- 스레드 수를 축으로 한 스케일링 벤치마크(1, 2, 4, 8...)를 만들어 코어당 효율(speedup / thread 수)을 함께 보고 — 선형 스케일링과의 괴리를 드러냄.
- CI와 로컬의 코어 수가 다르면 절대 수치가 아니라 "동일 스레드 수 기준 상대 변화율"만 회귀 판정에 사용.

```rust
let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
c.bench_function("parallel_frame_decode_4threads", |b| {
    b.iter(|| pool.install(|| frames.par_iter().for_each(|f| decode_frame(f))))
});
```

**탐지 방법**:
- Manual: 성능 비교 리포트에 사용된 스레드 수/코어 수가 명시되어 있는지 확인.
- Runtime: 동일 벤치마크를 스레드 수를 바꿔가며 실행해 결과가 스레드 수에 비례해서만 바뀌는지(즉 알고리즘 변화 효과가 없는지) 검증.

**예외**:
- 단일 스레드 전용 코드 경로(의도적으로 병렬화하지 않은 로직)를 비교할 때는 스레드 수가 애초에 변수가 아니므로 해당 없음.

**Bitvue 판정**: Confirmed(재확인, src-tauri 인용 제거) — crates/bitvue-metrics/src/lib.rs:345-346,382, crates/bitvue-cli/src/commands/decode.rs:133, crates/bitvue-decode/src/decoder.rs:448 모두 `par_iter`/`rayon::join`을 사용하지만 저장소 전체에 `ThreadPoolBuilder`/`num_threads`/`RAYON_NUM_THREADS` 사용이 0건(재검색으로 확인) — 항상 기본(논리 코어 수) 풀에 의존. 이전 판정이 인용한 `src-tauri/src/commands/thumbnails.rs`는 Tauri→Electron 전환으로 삭제됐고, 현재 썸네일 경로(crates/bitvue-sidecar/src/decode_bridge.rs)는 par_iter를 쓰지 않아 해당 인용은 제거 — 나머지 3곳만으로도 판정은 그대로 유효.

---

### PERF-016: turbo/thermal 상태 무시
**분류**: PERF · **심각도**: Low · **탐지**: Runtime|Manual

**나쁜 예**:
```bash
# 노트북에서 벤치마크 A 실행 (방금 부팅, CPU 서늘함, 터보 부스트 최대)
cargo bench --bench decode_bench  # 결과: 4.2ms

# 30분 동안 다른 최적화 작업하며 동일 머신으로 컴파일 반복 (CPU 뜨거워짐)
cargo bench --bench decode_bench  # 결과: 6.8ms
# "방금 바꾼 코드 때문에 38% 느려졌다"고 커밋 되돌림
# 실제로는 열 스로틀링으로 클럭이 떨어진 것뿐
```

**문제**:
- 현대 CPU는 온도/전력 상태에 따라 부스트 클럭이 동적으로 변하므로(터보 부스트, 열 스로틀링), 같은 코드도 측정 시점의 열 상태에 따라 수십 % 차이가 남.
- 랩톱/팬리스 기기는 특히 연속 벤치마크 시 뒤로 갈수록 클럭이 떨어져 "나중에 측정한 게 항상 더 느려 보이는" 편향이 생김.
- 이 상태를 모르면 순서 효과(먼저 측정한 버전이 유리)로 인해 A/B 비교 자체가 무효화됨.

**발생 조건**:
- 노트북/개발용 데스크톱에서 장시간 컴파일 후 곧바로 벤치마크를 돌릴 때.
- 여러 버전을 순차적으로(A 먼저, B 나중) 같은 세션에서 연속 측정할 때 — B가 항상 열적으로 불리한 상태.
- 클라우드 CI의 공유 인스턴스에서 다른 테넌트의 부하로 인한 CPU 스로틀링(noisy neighbor)까지 포함.

**권장**:
- 벤치마크 전 충분한 idle/쿨다운 시간을 두거나, 각 측정 사이에 고정된 대기 시간을 삽입.
- A/B 비교 시 순서를 번갈아가며(A-B-A-B...) 측정해 열적 편향을 상쇄(interleaved measurement).
- 가능하면 전용 벤치마크 머신(고정 팬 커브, 충분한 냉각) 또는 클럭 고정 모드(`cpupower frequency-set --governor performance`, 코드 서명된 CI 러너)를 사용하고 CPU 온도/클럭 로그를 함께 남기기.

**탐지 방법**:
- Runtime: 동일 코드로 연속 10회 측정했을 때 뒤로 갈수록 단조롭게 느려지는 추세가 있으면 열 스로틀링 의심.
- Manual: A/B 비교가 항상 같은 순서(A 먼저)로만 수행되었는지 방법론 리뷰.

**예외**:
- 서버급 CI 러너(안정적 냉각, 일정한 부하)에서 짧은 벤치마크만 도는 경우는 열 편향의 영향이 미미할 수 있음 — 그래도 장시간 스위트에서는 재확인 권장.

**Bitvue 판정**: Suspected — 벤치마크 방법론 문서/interleaved A/B 절차, 열 상태 로깅이 전혀 없어 통제 장치가 없다는 것은 확인했으나, 실제 열 스로틀링으로 인한 잘못된 회귀 판정 사례를 직접 확인하지는 못함.

---

### PERF-017: Criterion benchmark 내부에서 파일 open 반복
**분류**: PERF · **심각도**: High · **탐지**: Static|Structural

**나쁜 예**:
```rust
fn bench_index_seek(c: &mut Criterion) {
    c.bench_function("index_seek_by_pts", |b| {
        b.iter(|| {
            // 매 반복(수백~수천 회)마다 파일을 새로 열고 인덱스를 재구축
            let file = File::open("fixtures/long_stream.mkv").unwrap();
            let index = build_index(&file).unwrap();
            black_box(index.seek_by_pts(1_500_000))
        })
    });
}
```

**문제**:
- `b.iter()`는 통계적으로 유의미한 샘플을 얻기 위해 클로저를 수백~수천 번 반복 실행하는데, 그 안에 파일 열기+인덱스 구축처럼 무거운 setup을 두면 측정 대상(`seek_by_pts`)의 비용이 setup 비용에 완전히 묻힘.
- 파일 디스크립터를 반복 open/close하면 OS 콜 오버헤드까지 측정에 섞여 "seek이 느리다"는 결론이 실제로는 "파일 열기가 느리다"는 결론일 수 있음.
- 벤치마크 실행 시간 자체도 불필요하게 길어져(매 반복 I/O) CI 파이프라인이 느려짐.

**발생 조건**:
- `c.bench_function`의 클로저 안에 무심코 setup 코드를 그대로 두었을 때(가장 흔한 Criterion 초보 실수).
- 측정하려는 연산(seek, lookup)이 사전 구축된 상태(인덱스, 캐시)에 의존하는데 그 상태 구축을 매번 반복할 때.

**권장**:
- Criterion의 `iter_batched`/`iter_batched_ref` 또는 클로저 밖에서 1회만 setup을 수행하고 `b.iter()`에는 순수 측정 대상만 남기기.
- 상태를 변경하는 연산(mutating)은 `iter_batched`로 매 반복 새 입력을 값싸게 공급하고, 읽기 전용 연산(seek처럼 상태를 안 바꾸는 경우)은 클로저 밖에서 한 번만 구축.

```rust
fn bench_index_seek(c: &mut Criterion) {
    let file = File::open("fixtures/long_stream.mkv").unwrap();
    let index = build_index(&file).unwrap(); // 클로저 밖, 1회만 실행
    c.bench_function("index_seek_by_pts", |b| {
        b.iter(|| black_box(index.seek_by_pts(1_500_000)))
    });
}
```

**탐지 방법**:
- Static/Structural: `benches/**/*.rs`에서 `b.iter(||` 클로저 본문에 `File::open`/`fs::read`/`build_index` 등 setup성 호출이 포함되어 있는지 grep으로 스캔.
- Manual: 새 벤치마크 추가 PR 리뷰 시 클로저 내부가 "측정 대상 1줄"에 가까운지 확인.

**예외**:
- 측정하려는 대상이 바로 "파일 열기 자체의 비용"이라면 클로저 안에 `File::open`이 있는 것이 맞음 — 이 경우 벤치마크 이름을 `file_open_cost` 등으로 명확히 구분.

**Bitvue 판정**: N/A — 5개 벤치 파일(bitreader/export/magic_bytes/frame_parsing/overlay_extraction) 전부 `File::open`/`fs::read`가 `b.iter()` 클로저 밖, setup 단계에서 1회만 호출되거나 애초에 파일 I/O가 없음 — 클로저 내부 setup 혼입 사례 없음.

---

### PERF-018: compiler version 차이 무시
**분류**: PERF · **심각도**: Low · **탐지**: Manual|Structural

**나쁜 예**:
```
# 3개월 전 벤치마크 (rustc 1.75, LTO=off)
parse_hevc_nal: 4.1 ms

# 오늘 벤치마크 (rustc 1.79, LTO=thin, 그 사이 rustup update 여러 번)
parse_hevc_nal: 3.2 ms

# 커밋 메시지: "NAL 파싱 22% 최적화" — 실제로는 코드 변경 0줄,
# 단지 컴파일러 버전과 빌드 설정이 그 사이 바뀐 것
```

**문제**:
- Rust 컴파일러(LLVM 백엔드 포함)는 마이너 버전마다 코드 생성/인라이닝/자동 벡터화 휴리스틱이 바뀌므로, 같은 소스가 컴파일러 버전만 달라져도 성능이 유의미하게 변함.
- 시간이 오래 지난 "이전 벤치마크 수치"와 "지금 수치"를 직접 비교하면 코드 변경 효과와 툴체인 변경 효과가 뒤섞여 어느 쪽이 원인인지 알 수 없음.
- Cargo.lock/의존성 버전이 그 사이 업데이트되었을 가능성도 함께 얽혀 재현 불가능한 비교가 됨.

**발생 조건**:
- 벤치마크 결과를 문서/이슈에 텍스트로만 기록하고 rustc 버전, 빌드 플래그, 의존성 lockfile 해시를 함께 기록하지 않을 때.
- "몇 달 전 수치"와 "지금 수치"를 롱텀 트렌드로 그래프에 이어 붙일 때 툴체인 변경 이벤트를 표시하지 않을 때.

**권장**:
- 모든 벤치마크 결과에 `rustc --version --verbose` 출력(commit hash 포함), `Cargo.lock` 해시, 관련 빌드 플래그(`RUSTFLAGS`, LTO, codegen-units)를 함께 기록.
- 회귀 판정은 항상 "같은 툴체인, 같은 lockfile"로 빌드한 baseline과 candidate를 나란히 재빌드해서 비교(A/B를 같은 세션에서).
- 장기 추세 그래프에는 rustc 버전이 바뀐 지점을 주석으로 표시해 코드 변경과 툴체인 변경을 시각적으로 구분.

**탐지 방법**:
- Structural: 벤치마크 CI 잡이 `rust-toolchain.toml`로 버전을 고정하는지, 아니면 "stable" 같은 부동 버전을 쓰는지 확인.
- Manual: 성능 비교가 실린 문서/PR에 두 시점의 rustc 버전이 다른데도 언급이 없는지 점검.

**예외**:
- 정확히 반대로 "이 컴파일러 업그레이드가 우리 코드에 어떤 성능 영향을 주는지" 알아보려는 목적이라면 컴파일러 버전 차이가 바로 측정 대상 — 이 경우 코드는 고정하고 툴체인만 바꿔야 함.

**Bitvue 판정**: Confirmed — 저장소에 `rust-toolchain.toml`/`rust-toolchain` 파일이 전혀 없고(find 0건) CI 워크플로도 특정 rustc 버전을 고정하지 않음 — 툴체인이 러너/개발 머신의 "stable"에 그대로 의존.

---

### PERF-019: feature 조합 차이 무시
**분류**: PERF · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```bash
# 로컬: 기본 feature만 켠 상태로 벤치마크
cargo bench -p bitvue-benchmarks

# CI: 전체 feature(simd, rayon-parallel, hardware-accel) 켠 상태
cargo bench -p bitvue-benchmarks --all-features

# 두 결과를 같은 표에 나란히 붙여 "CI에서 5배 빨라짐" 보고
# -> 코드는 동일, feature 플래그 조합만 다를 뿐
```

**문제**:
- `simd`, `rayon`, 하드웨어 가속 등 조건부 feature는 활성화 여부에 따라 완전히 다른 코드 경로를 실행하므로, feature 조합이 다른 두 빌드는 사실상 "다른 프로그램"을 비교하는 것과 같음.
- 사용자 대부분이 기본 feature로 빌드된 배포판을 쓰는데, 벤치마크는 전체 feature를 켠 "가장 빠른 구성"만 보고하면 실제 배포 성능을 과장하게 됨.
- Feature 플래그 조합이 배타적(mutually exclusive)이거나 상호작용이 있는 경우(예: `simd`가 `rayon-parallel` 없이는 효과가 적음), 조합을 명시하지 않으면 재현이 불가능.

**발생 조건**:
- 로컬 개발 환경과 CI/릴리즈 빌드의 `--features`/`--all-features`/`--no-default-features` 설정이 서로 다를 때.
- `bitvue-benchmarks` 크레이트가 여러 feature 조합을 지원하는데 벤치마크 실행 스크립트가 하나의 고정 조합만 쓰거나, 반대로 매번 다른 조합으로 우연히 실행될 때.

**권장**:
- 벤치마크 결과에는 항상 사용된 정확한 `--features` 플래그 목록(또는 `--no-default-features` 여부)을 함께 기록.
- 배포 기본값과 동일한 feature 조합을 "기준(baseline)" 벤치마크로 CI에 고정하고, 추가 feature 조합은 별도로 라벨링해 병기.
- 여러 feature 조합을 매트릭스로 CI에서 각각 벤치마크해 조합별 영향을 명시적으로 표로 관리(예: default / +simd / +rayon / all).

**탐지 방법**:
- Structural: 벤치마크 CI 잡과 로컬 개발 문서(README 등)에 명시된 `cargo bench` 플래그가 서로 일치하는지 diff.
- Manual: 성능 비교 리포트에 feature 플래그 목록이 누락되어 있는지 확인.

**예외**:
- 애초에 "feature가 성능에 미치는 영향"을 측정하는 것이 벤치마크의 목적이라면(예: "simd on/off 비교") feature 조합 차이가 바로 측정 대상이므로 문제 아님 — 이 경우 결과 표에 두 조합을 나란히 놓고 명시하면 됨.

**Bitvue 판정**: N/A — crates/bitvue-benchmarks/Cargo.toml에는 `[features]` 섹션 자체가 없고 4개 벤치는 bitvue-decode/bitvue-metrics의 `ffmpeg`/`vvdec`/`vmaf`/`parallel` feature로 게이트된 코드를 전혀 호출하지 않음 — feature 조합을 비교하는 벤치마크가 애초에 존재하지 않음.

---

### PERF-020: 최적화 후 정확성 검증 누락
**분류**: PERF · **심각도**: Critical · **탐지**: Runtime|Manual

**나쁜 예**:
```rust
// Before: 경계 체크 포함, 안전하지만 "느림"
fn get_pixel(frame: &[u8], x: u32, y: u32, stride: u32) -> u8 {
    let idx = (y * stride + x) as usize;
    frame.get(idx).copied().unwrap_or(0)
}

// After: "40% 빨라짐" — unsafe로 경계 체크 제거
fn get_pixel(frame: &[u8], x: u32, y: u32, stride: u32) -> u8 {
    unsafe { *frame.get_unchecked((y * stride + x) as usize) }
}
// 벤치마크는 통과(더 빨라짐), 그러나 conformance 테스트를 다시 안 돌려서
// 크롭된 프레임 경계 근처에서 OOB 읽기로 랜덤 픽셀값이 섞이는 회귀를 놓침
```

**문제**:
- 성능 벤치마크는 "얼마나 빠른가"만 확인할 뿐 "결과가 여전히 맞는가"는 전혀 검증하지 않음. 최적화(특히 `unsafe`, 근사 알고리즘, 캐싱, 병렬화로 인한 순서 변경)는 정확성을 깨뜨리기 쉬운 대표적 변경.
- 비디오 분석기에서 픽셀/신택스 값이 미묘하게 틀리면 화면상으로는 눈에 안 띄고 conformance 테스트가 없으면 CI도 통과해버려, 사용자가 잘못된 분석 결과를 신뢰하게 되는 심각한 결과로 이어짐.
- "빨라졌다"는 성과가 강조되는 PR일수록 리뷰어가 정확성보다 벤치마크 수치에 먼저 눈이 가서 회귀 검증이 느슨해지는 심리적 함정도 있음.

**발생 조건**:
- 경계 체크 제거, `unsafe` 블록 도입, 부동소수점 연산 순서 변경(병렬 reduce), 룩업 테이블/캐시 도입, SIMD 도입 등 "결과가 바이트 단위로 같아야 하는" 최적화를 할 때.
- 성능 CI와 정확성(conformance/regression) CI가 분리되어 있어 성능 PR이 정확성 스위트를 강제로 거치지 않을 때.

**권장**:
- 모든 성능 최적화 PR은 반드시 기존 conformance/reference 비교 테스트(비트 단위 또는 알려진 허용 오차 내 픽셀 비교)를 통과해야 머지 가능하도록 CI 게이트를 건다.
- `unsafe` 최적화는 debug 빌드에서 동일 로직의 안전한 버전과 결과를 diff하는 이중 구현 테스트(differential testing)를 추가.
- 벤치마크 PR 템플릿에 "정확성 테스트를 재실행했는가" 체크박스를 필수 항목으로 포함.

**탐지 방법**:
- Runtime: 최적화 전/후 산출물을 대량의 실전 샘플에 대해 바이트/픽셀 단위로 diff하는 회귀 스위트를 CI에서 자동 실행.
- Structural: `unsafe` 블록이 새로 추가된 PR에서 해당 함수의 정확성 테스트 커버리지가 함께 늘었는지 커버리지 diff로 확인.
- Manual: "N% 빨라짐" 문구가 있는 PR에 conformance 테스트 실행 로그가 첨부되어 있는지 리뷰 체크리스트로 강제.

**예외**:
- 정확성에 영향을 줄 수 없는 변경(예: 순수 메모리 레이아웃 재배치로 로직 동일, 컴파일러 힌트 추가)이라도 회귀 스위트가 이미 자동으로 도는 CI 구조라면 별도 수동 검증은 생략 가능 — 단, CI 자체가 반드시 존재해야 함.

**Bitvue 판정**: Suspected — dhat 등 할당 프로파일러나 별도 bitstream conformance 코퍼스는 못 찾았지만(문서상 parity 자료만 존재: docs/PARITY_CHECKLIST.md, scripts/parity_check.sh), crates/bitvue-metrics/src/simd.rs:599의 `test_window_stats_vs_scalar`처럼 unsafe SIMD 결과를 scalar와 비교하는 differential test가 실제로 존재 — 관행이 일부는 있으나 성능 PR 전반에 강제되는 게이트인지는 불명.

---

### PERF-021: "Rayon을 넣었으니 빨라졌다" (병렬화 = 개선이라는 착각)
**분류**: PERF · **심각도**: High · **탐지**: Runtime|Manual

**나쁜 예**:
```rust
// PR 설명: "프레임 디코드 루프를 Rayon으로 병렬화 -> 성능 개선"
frames.par_iter_mut().for_each(|frame| {
    decode_frame(frame); // 프레임 크기가 작아 함수 자체가 마이크로초 단위
});
// wall time만 재고 "빨라졌다"고 결론. 실제로는:
// - 스레드 스폰/스틸링 오버헤드가 작업 자체보다 커서 코어 4개 이상에서는 역효과
// - 모든 스레드가 같은 메모리 대역폭을 두고 경합해 8코어에서 2.3배밖에 안 남
```

**문제**:
- 병렬화는 "코어 수만큼 빨라진다"를 보장하지 않음. 작업 단위가 스레드 오버헤드보다 작으면(fine-grained task) 오히려 순차보다 느려질 수 있고, 메모리 대역폭이 병목인 워크로드(비디오 프레임 버퍼처럼 데이터가 큰 경우)는 코어를 늘려도 대역폭 천장(memory-bandwidth ceiling)에 곧 부딪혀 선형 스케일링이 절대 나오지 않음.
- "Rayon 도입 = 개선"이라는 결론은 실제 스케일링 계수(speedup / thread 수)를 측정하지 않은 채 wall time 한 번만 보고 내려지는 경우가 많음 — 이는 PERF-006(wall time만 측정), PERF-015(thread 수 고정 없이 비교)와 직결.
- 병렬화로 인해 CPU 총 사용량(코어·초)은 오히려 증가하는데, 이는 배터리/발열/다른 동시 작업(예: 같은 머신에서 돌아가는 UI 스레드)에 악영향을 줄 수 있음.

**발생 조건**:
- 루프를 `iter()` -> `par_iter()`로 바꾸는 것만으로 "최적화 완료"라고 표기할 때.
- 작업 단위가 매우 작거나(수 마이크로초), 반대로 공유 자원(참조 프레임 버퍼, 전역 캐시)에 대한 접근이 많아 락 경합이 심한 워크로드에 그대로 병렬화를 적용할 때.

**권장**:
- 병렬화 PR은 반드시 스레드 수 1, 2, 4, 8...에 따른 스케일링 곡선을 함께 제시하고, 선형 대비 효율(parallel efficiency = speedup / N)을 보고.
- 대역폭 바운드 워크로드인지 CPU 바운드 워크로드인지 `perf stat`(cache-misses, memory bandwidth 카운터)로 먼저 진단한 뒤 병렬화 적용 여부를 결정.
- 작업 단위가 작으면 `par_iter()`의 chunking(`with_min_len`)으로 태스크 크기를 키워 스폰 오버헤드를 줄이는 것부터 시도.

**탐지 방법**:
- Runtime: 스레드 수를 바꿔가며 실제로 측정한 speedup 곡선이 PR에 첨부되어 있는지 확인. 없으면 "빨라졌다"는 주장은 미검증.
- Manual: `par_iter`/`par_chunks` 도입 PR에 스케일링 근거(그래프/표)가 없는 경우를 리뷰에서 지적.

**예외**:
- 작업 단위가 명확히 크고(수 ms 이상) 독립적이며 공유 자원 접근이 없는 embarrassingly parallel 워크로드(예: 서로 다른 파일을 각각 파싱)는 병렬화 효과가 상식적으로 예측 가능 — 그래도 최소한 1회는 실측 검증 권장.

**Bitvue 판정**: Suspected — `par_iter()`가 bitvue-metrics(PSNR/SSIM), bitvue-decode(plane 추출), thumbnails.rs 등 프로덕션 핫패스에 쓰이지만 이들 중 어느 것도 bitvue-benchmarks의 5개 벤치에서 스레드 수별 스케일링으로 측정되지 않음 — 구조적으로 패턴에 부합하나 실제 "N배 빨라짐" 주장을 텍스트로 확인하지는 못함.

---

### PERF-022: "mmap을 썼으니 zero-copy다" (겉만 zero-copy)
**분류**: PERF · **심각도**: Medium · **탐지**: Structural|Semantic

**나쁜 예**:
```rust
// "mmap으로 zero-copy 파일 읽기 구현" 이라고 PR에 적혀 있음
let mmap = unsafe { Mmap::map(&file)? };

fn parse_nal_units(mmap: &Mmap) -> Vec<NalUnit> {
    let mut units = Vec::new();
    for range in find_start_codes(mmap) {
        // mmap에서 슬라이스를 얻자마자 즉시 to_vec()으로 복사 -> zero-copy 무효화
        units.push(NalUnit { data: mmap[range].to_vec(), ..Default::default() });
    }
    units
}
// 결과: to_vec()가 매 NAL 유닛마다 힙 복사를 발생시켜
// mmap의 이론적 이점(페이지 캐시 직접 참조)이 실질적으로 0
```

**문제**:
- `mmap`은 "파일 데이터를 커널이 관리하는 페이지에 매핑해 복사를 피할 수 있는 잠재력"을 제공할 뿐, 그 이후 코드가 실제로 복사를 피하지 않으면 아무 이득이 없음. `.to_vec()`, `copy_from_slice`, serde로 소유 타입에 직렬화하는 순간 zero-copy는 깨짐.
- "mmap을 썼다"는 사실 자체를 zero-copy 달성의 증거로 착각해 실제 할당/복사량을 측정하지 않으면, 오히려 일반 `fs::read` + 버퍼 재사용보다 못한 결과(mmap의 페이지 폴트 오버헤드만 추가)가 나올 수 있음.
- 수명(lifetime) 관리를 피하려고 데이터를 복사하는 경우가 흔한데, 이는 "안전하게 짜기 쉬운 코드"를 택하면서 "zero-copy"라는 성능 주장만 그대로 남겨두는 불일치.

**발생 조건**:
- `NalUnit`/`Obu` 같은 파싱 결과 구조체가 `&[u8]`(차용) 대신 `Vec<u8>`(소유)을 필드로 가지고 있는데 mmap에서 데이터를 채울 때.
- 파싱 결과를 나중에 IPC로 넘기거나 다른 스레드에 전달하기 위해 어차피 소유 타입으로 변환해야 하는 상황에서, 최초 읽기만 mmap으로 바꾸고 "zero-copy 달성"이라 주장할 때.

**권장**:
- Zero-copy를 주장하려면 파싱 결과 구조체가 원본 mmap 버퍼를 가리키는 `&'a [u8]` 차용 타입이어야 하며, 이를 dhat/allocation 카운터로 "이 경로에서 할당이 0에 가깝다"는 것을 실측으로 증명(PERF-007과 연계).
- Lifetime이 복잡해져 실용적이지 않다면 zero-copy를 포기하고 대신 "버퍼 재사용"(pool/arena) 전략으로 목표를 명확히 재정의.
- PR 설명에서 "mmap 사용"과 "zero-copy 달성"을 별개 주장으로 분리하고, 후자는 반드시 할당 카운트 벤치마크로 뒷받침.

**탐지 방법**:
- Structural: mmap 관련 코드 근처에서 `.to_vec()`, `.to_owned()`, `Vec::from(`, `copy_from_slice(` 호출을 grep.
- Semantic: 파싱 결과 타입이 `Cow<[u8]>`/`&[u8]` 차용인지 `Vec<u8>` 소유인지 타입 시그니처를 확인.
- Runtime: mmap 도입 전/후로 dhat 할당 카운트를 비교해 실제로 줄었는지 검증.

**예외**:
- 파싱 결과를 스레드 경계를 넘겨야 하거나 mmap의 수명보다 오래 살아야 하는 구조적 요구가 있다면 의도적으로 복사하는 것이 올바른 설계 — 이 경우 애초에 "zero-copy"를 목표/주장으로 내세우지 않으면 문제없음.

**Bitvue 판정**: N/A — crates/bitvue-formats/src/mp4.rs:314,323-347의 `extract_av1/avc/hevc_samples`는 실제로 `Cow::Borrowed`를 반환해 zero-copy 주장이 구현과 일치하며, ByteCache(crates/bitvue-engine/src/byte_cache.rs, 구 bitvue-core — 경로 갱신)의 `Bytes` 세그먼트 캐시는 LRU 중복제거를 위해 의도적으로 소유 복사본을 두는 설계로 zero-copy를 표방하지 않음.

---

### PERF-023: "release opt-level=3이니 충분하다" (컴파일러 플래그로 구조적 병목을 덮으려는 착각)
**분류**: PERF · **심각도**: Medium · **탐지**: Manual|Structural

**나쁜 예**:
```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
```
```rust
// "release 프로파일 최적화 플래그를 최대로 올렸으니 성능은 문제없다"
// 그러나 실제 구조는:
struct Frame {
    metadata: HashMap<String, String>, // 프레임마다 힙 할당된 문자열 맵
    pixels: Vec<Vec<u8>>,              // row마다 별도 Vec -> 캐시 비지역적
}
// opt-level=3/LTO는 이런 구조적 문제(할당 패턴, 메모리 레이아웃)를 해결해주지 못함
```

**문제**:
- `opt-level=3`, LTO, `codegen-units=1`은 인라이닝/벡터화/데드코드 제거 같은 "명령어 수준" 최적화만 담당하며, 구조적 문제(struct-of-arrays vs array-of-structs, 불필요한 힙 할당, IPC 직렬화 비용, 캐시 비지역적 접근 패턴, 알고리즘 복잡도)는 전혀 건드리지 못함.
- "컴파일러 플래그를 최대로 올렸다"는 사실이 심리적으로 "할 수 있는 최적화는 다 했다"는 착각을 주어, 실제로 훨씬 큰 이득을 볼 수 있는 구조적 리팩터링(레이아웃 변경, 할당 제거, IPC 배치)을 놓치게 만듦.
- LTO=fat + codegen-units=1은 빌드 시간을 크게 늘리는 비용을 치르면서도, 애초에 병목이 알고리즘/메모리 레이아웃에 있다면 실행 시간 개선은 미미할 수 있어 트레이드오프가 안 맞을 수 있음.

**발생 조건**:
- 프로파일러(perf/flamegraph)로 실제 핫스팟을 확인하지 않은 채 "release 최적화 설정을 올리는 것"을 성능 작업의 전부로 여길 때.
- "빌드 설정 튜닝"이 "코드 구조 튜닝"보다 변경 범위가 작고 리스크가 낮아 보여서 우선적으로 선택될 때(실제로는 효과가 훨씬 제한적인데도).

**권장**:
- 성능 작업의 출발점은 항상 프로파일링(`perf record`/`cargo flamegraph`/`samply`)으로 실제 시간이 어디서 소비되는지 확인하는 것 — 컴파일러 플래그 조정은 그 다음, 그리고 보통 마지막 단계.
- 구조적 병목 후보(할당 패턴, 데이터 레이아웃, IPC 직렬화, 캐시 미스율)를 `perf stat`의 캐시/분기 카운터로 먼저 점검.
- 빌드 플래그 변경 자체도 반드시 전/후 벤치마크로 실효를 검증 — "이론상 더 최적화됨"이 아니라 "측정상 몇 % 개선됨"으로 근거를 남기기.

**탐지 방법**:
- Manual: "성능 개선" PR이 `Cargo.toml` profile 설정 변경만 포함하고 코드 변경/프로파일링 근거가 없는지 확인.
- Structural: 플레임그래프/perf 리포트가 CI 아티팩트 또는 PR 첨부에 존재하는지 점검 — 없으면 병목을 실제로 진단했는지 의심.

**예외**:
- 프로파일링으로 이미 명령어 수준 병목(인라이닝 실패, 벡터화 실패)이 확인된 상태에서 opt-level/LTO 조정이 타겟 최적화로 적용되는 경우는 정당 — 이때는 "구조 문제를 덮으려는 착각"이 아니라 근거 있는 조치.

**Bitvue 판정**: Suspected — 워크스페이스 Cargo.toml이 opt-level=3/lto=thin/codegen-units=1을 이미 설정했고 flamegraph/perf/samply 등 프로파일링 도구·문서가 저장소 어디에도 없어(grep 0건) 구조적으로 패턴에 부합하나, 이를 대체 근거로 내세운 PR/커밋 텍스트는 직접 확인하지 못함.

---

### PERF-024: "async로 만들었으니 UI가 안 멈춘다" (블로킹을 async로 감쌌을 뿐)
**분류**: PERF · **심각도**: High · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
#[tauri::command]
async fn parse_large_container(path: String) -> Result<ContainerInfo, String> {
    // async fn으로 선언했지만 내부는 여전히 동기 블로킹 호출
    let data = std::fs::read(&path).map_err(|e| e.to_string())?; // 블로킹 I/O
    let info = parse_mp4_container(&data).map_err(|e| e.to_string())?; // CPU 바운드, 수백 ms
    Ok(info)
}
// "async로 바꿨으니 프론트엔드가 멈추지 않는다"고 가정
// 실제로는 Tokio 워커 스레드 하나를 수백 ms 동안 통째로 점유해
// 같은 워커에 스케줄된 다른 IPC 커맨드(예: UI 상태 폴링)까지 함께 멈춤
```

**문제**:
- `async fn`은 "이 함수가 non-blocking하다"를 보장하지 않음. 함수 본문이 `.await` 지점 없이 동기 블로킹 호출(`std::fs::read`, CPU 바운드 루프)을 실행하면, 그 태스크를 실행 중인 Tokio 워커 스레드는 완료될 때까지 다른 태스크를 전혀 스케줄링하지 못함 — 즉 async 런타임 전체가 부분적으로 정지(runtime worker starvation).
- Tauri 커맨드가 이런 식으로 워커를 오래 점유하면, 같은 워커 풀을 공유하는 다른 IPC 호출(예: 진행률 업데이트, 다른 패널의 데이터 요청)까지 지연되어 결국 UI가 멈춘 것처럼 체감됨 — "async니까 안 멈춘다"는 가정과 정반대 결과.
- 이 문제는 짧은 스모크 테스트(작은 파일)로는 드러나지 않고, 실제 대형 파일/느린 디스크에서만 체감되므로 PERF-004(작은 샘플)와 결합하면 더욱 놓치기 쉬움.

**발생 조건**:
- CPU 바운드 파싱/디코드 작업이나 동기 파일 I/O를 `async fn` 안에 `.await` 없이 그대로 넣을 때.
- Tokio 멀티스레드 런타임의 워커 스레드 수가 적은데(예: 저사양 기기, 코어 수 제한) 여러 무거운 커맨드가 동시에 호출될 때 문제가 더 뚜렷해짐.

**권장**:
- 블로킹 I/O는 `tokio::fs`(비동기 I/O) 또는 `tokio::task::spawn_blocking`으로 전용 블로킹 스레드 풀에 위임.
- CPU 바운드 파싱/디코드도 마찬가지로 `spawn_blocking`으로 async 워커 풀에서 분리하거나, 별도의 rayon 스레드 풀에서 실행하고 결과를 채널로 async 쪽에 전달.
- "async면 안전하다"를 검증하려면 무거운 커맨드 실행 중에 다른 가벼운 커맨드(UI 폴링 등)를 동시에 호출해 응답 지연이 없는지 실측하는 부하 테스트를 추가.

```rust
#[tauri::command]
async fn parse_large_container(path: String) -> Result<ContainerInfo, String> {
    tokio::task::spawn_blocking(move || {
        let data = std::fs::read(&path).map_err(|e| e.to_string())?;
        parse_mp4_container(&data).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

**탐지 방법**:
- Structural: `async fn` 내부에서 `.await` 없이 `std::fs::*`, `std::thread::sleep`, 무거운 동기 함수를 직접 호출하는 패턴을 grep(예: `async fn`과 `spawn_blocking` 부재를 함께 검사).
- Runtime: 무거운 커맨드 실행 중 동시에 가벼운 커맨드를 호출해 후자의 응답 시간이 급증하는지(워커 점유로 인한 헤드 오브 라인 블로킹) 부하 테스트로 확인.

**예외**:
- Tokio current-thread 런타임이 아니라 전용 블로킹 전용 스레드에서 실행되는 커맨드이거나, 작업 시간이 마이크로초 단위로 매우 짧아 워커 점유가 실질적으로 무해한 경우는 `spawn_blocking` 없이도 문제가 되지 않음 — 단, "짧다"는 것 자체를 실측해야 함.

**Bitvue 판정**: N/A(재확인, 이전 판정 폐기) — 원 인용 경로 `src-tauri/src/commands/compare.rs`는 Tauri→Electron sidecar 전환 완료로 저장소에서 완전히 삭제됨. 후속 아키텍처(crates/bitvue-sidecar/src/main.rs)는 애초에 async/Tokio를 쓰지 않음(`grep "async fn"` 0건) — 요청마다 `std::thread::spawn`으로 전용 OS 스레드를 띄우고 메인 리더 루프는 절대 블로킹하지 않는 설계를 모듈 문서(L28-37)에서 "Tokio 대신 OS 스레드를 의도적으로 선택 — Core의 작업은 I/O 대기가 아니라 CPU-바운드 동기 코드라 스레드가 더 단순한 fit"이라고 명시적으로 정당화함. 즉 이 항목이 겨냥하는 "async로 포장했지만 워커를 점유"하는 실패 모드 자체가 현재 아키텍처에 구조적으로 존재하지 않음.
</content>
