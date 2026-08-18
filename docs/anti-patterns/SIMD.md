# Anti-Pattern Catalog — SIMD: SIMD/CPU 벡터화

이 문서는 더 큰 안티패턴 카탈로그의 한 카테고리입니다 (전체 인덱스는 `docs/anti-patterns/INDEX.md` 참고). Wave 5 추가 항목이며, 기존 49개 파일 중 어느 것도 벡터화(vectorization) 엔지니어링 자체의 리스크를 전담하지 않는다는 공백에서 작성되었습니다 — 가장 가까운 이웃인 `RPERF.md`(Wave 4)는 dyn dispatch·iterator chain·`Drop` timing 등 Rust 언어 추상화 비용을 다루지만 SIMD 인트린식 고유의 정합성·성능 함정(청크/tail 처리, cross-ISA 일관성, feature detection, 레인 관리)은 다루지 않습니다. 이 파일은 저장소에 실제로 존재하는 두 개의 손으로 작성된 SIMD 구현을 근거로 삼습니다: `crates/bitvue-metrics/src/simd.rs`(PSNR/SSIM, AVX2/AVX/SSE2/NEON/스칼라 5단 런타임 cascade)와 `crates/bitvue-decode/src/strategy/{avx2,neon,scalar}.rs`(YUV→RGB 색공간 변환, `StrategyRegistry`가 플랫폼별로 선택하는 AVX2/NEON/스칼라 3-전략 아키텍처). PSNR/SSIM의 지표 정확성 버그(하드코딩된 `255.0` max-value, 0 MSE에서 클램프 없이 반환되는 `f64::INFINITY` — `METRIC.md` PSNR 섹션 Confirmed)는 이미 다른 문서가 판정을 마쳤으므로 여기서는 재론하지 않고 "관련" 포인터로만 연결합니다. 이 문서가 다루는 층위는 그 버그들과 다릅니다: SIMD 코드를 "엔지니어링"하는 과정에서 반복적으로 발생하는 실수—feature detection 전략, 청크/tail 경계 처리, ISA 간 결과 일관성, 레지스터·레인 관리, 안전성 계약의 표현 방식—입니다. Phase 1(카탈로그 작성) 단계이며 저장소 전수 감사는 아니지만, 항목을 조사하는 과정에서 실제 코드를 읽고 grep으로 검증한 항목은 플레이스홀더 대신 실제 판정을 기록했습니다(구체 근거: SIMD-001/002/003/004/006/007/009/011/012/013/015/016/017 — Confirmed 또는 그에 준하는 판정, SIMD-008/014 — Suspected, SIMD-005/010만 미정으로 남김).

---

### SIMD-001: 런타임 feature detection 없이 컴파일타임 `#[cfg(target_arch)]`만으로 SIMD 분기
**분류**: SIMD · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
#[cfg(target_arch = "x86_64")]
fn compute_diff(a: &[u8], b: &[u8]) -> Vec<i16> {
    // "x86_64면 무조건 AVX2가 있다"고 가정 — 실제로는 Haswell(2013) 이전 x86_64
    // CPU, 일부 저사양 VM/컨테이너 vCPU 마스크는 AVX2를 지원하지 않는다.
    unsafe { compute_diff_avx2(a, b) } // target_feature 없이 호출하면 SIGILL
}
```

**문제**:
- `target_arch = "x86_64"`는 "이 CPU가 AVX2를 지원한다"를 보장하지 않는다 — 아키텍처와 명령어 확장 세트는 별개의 축이다.
- `#[target_feature(enable = "avx2")]`가 붙은 `unsafe fn`을 실제 AVX2 미지원 CPU에서 호출하면 컴파일은 통과하지만 실행 시 `SIGILL`(잘못된 명령어)로 즉시 크래시한다 — panic이 아니라 프로세스가 그냥 죽는다.
- 클라우드 VM(`-mno-avx2` 옵션의 최소 vCPU 마스크), 구형 데스크톱, 일부 저전력 ARM이 아닌 x86 SoC에서 특히 흔하다.

**발생 조건**:
- SIMD 함수를 아키텍처 `#[cfg]`로만 게이팅하고 `is_x86_feature_detected!`/`is_aarch64_feature_detected!` 런타임 검사를 생략했을 때.

**권장**:
```rust
fn compute_diff(a: &[u8], b: &[u8]) -> Vec<i16> {
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") {
        return unsafe { compute_diff_avx2(a, b) };
    }
    compute_diff_scalar(a, b)
}
```
- `#[cfg(target_arch = ...)]`는 "이 아키텍처용으로만 컴파일한다"는 컴파일타임 게이트로만 쓰고, 실제 호출 여부는 반드시 `is_x86_feature_detected!!`/`is_aarch64_feature_detected!`의 런타임 결과로 분기한다.

**탐지 방법**:
- Static: `#[target_feature(enable = ...)]` 함수의 모든 호출부에서 `is_x86_feature_detected!`/`is_aarch64_feature_detected!` 가드가 선행하는지 grep.
- Runtime: `RUSTFLAGS`로 타겟 feature를 낮춘 QEMU/오래된 CPU 환경에서 통합 테스트 실행.

**예외**: 빌드 자체를 `-C target-cpu=x86-64-v3`(AVX2 baseline) 등으로 고정해 배포하는 경우, 다만 Bitvue는 범용 데스크톱 앱이라 이 전제가 성립하지 않는다.

**Bitvue 판정**: N/A — 두 SIMD 서브시스템 모두 런타임 검사를 실제로 수행함. `crates/bitvue-metrics/src/simd.rs:48,50,57`(`compute_window_stats_simd`)와 `:518,520,522,529`(`psnr_simd`)는 `is_x86_feature_detected!("avx2"/"avx"/"sse2")`·`std::arch::is_aarch64_feature_detected!("neon")`로 분기 후에만 unsafe 함수를 호출한다. `crates/bitvue-decode/src/strategy/avx2.rs:32`(`Avx2Strategy::is_available`), `neon.rs:31`, `registry.rs:37,39`도 동일 매크로를 쓰며, `#[cfg(target_arch)]`는 오직 "이 플랫폼에서만 컴파일"이라는 컴파일타임 게이트로만 쓰인다(`strategy/mod.rs:9-13`).

---

### SIMD-002: 정렬되지 않은 버퍼에 정렬 전용 load/store 인트린식 사용
**분류**: SIMD · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
unsafe fn sum_avx2(data: &[u8]) -> i64 {
    // Vec<u8>는 32바이트 정렬을 보장하지 않는다 — 임의 offset에서
    // _mm256_load_si256(정렬 전용)을 쓰면 정렬되지 않은 주소에서 #GP fault.
    let v = _mm256_load_si256(data.as_ptr() as *const __m256i);
    /* ... */
    0
}
```

**문제**:
- x86 정렬 전용 load(`_mm256_load_si256`, `_mm_load_si128`)는 대상 주소가 각각 32/16바이트 경계에 있지 않으면 일반 보호 예외(`#GP`)로 크래시한다.
- `Vec<u8>`/슬라이스는 기본적으로 1바이트 정렬만 보장하므로, `start` 오프셋이 사용자가 지정한 윈도우 경계(예: SSIM 윈도우 시작점)처럼 임의 값일 때 정렬 전용 인트린식은 근본적으로 안전하지 않다.
- "대부분의 입력에서는 우연히 정렬이 맞아 통과하다가, 특정 프레임 크기/오프셋 조합에서만 크래시"하는 재현이 어려운 버그로 나타난다.

**발생 조건**: 비트스트림에서 유래한 임의 폭/오프셋의 프레임·플레인 버퍼에 SIMD 커널을 적용할 때(디코더 출력 버퍼가 항상 정렬 allocator를 쓰지 않는 한).

**권장**:
```rust
unsafe fn sum_avx2(data: &[u8]) -> i64 {
    // 정렬 여부를 가정하지 않는 loadu/storeu만 사용.
    let v = _mm256_loadu_si256(data.as_ptr() as *const __m256i);
    /* ... */
    0
}
```
- 입력 버퍼의 정렬을 제어할 수 없다면 `loadu`/`storeu`(NEON은 `vld1q_u8` 계열, 애초에 비정렬 접근을 허용) 계열만 사용한다.
- 정렬을 보장할 수 있는 내부 스크래치 버퍼에 한해서만 정렬 로드로 최적화하고, 반드시 `#[repr(align(32))]`/전용 allocator로 정렬을 실제로 강제한 뒤 적용한다.

**탐지 방법**: Static: `_mm256_load_si256`/`_mm_load_si128`/`_mm256_store_si256`(정렬 전용, `u`가 없는 변형) 호출을 grep. Runtime: 홀수 바이트 오프셋에서 시작하는 윈도우/버퍼로 fuzzing.

**예외**: 없음 — 정렬 로드가 필요한 성능 이득은 미미하고(Haswell 이후 loadu와 load의 처리량 차이는 사실상 0에 가까움) 크래시 리스크가 이를 압도한다.

**Bitvue 판정**: N/A — 정렬 전용 로드/스토어 인트린식(`_mm256_load_si256`, `_mm_load_si128`, non-`u` 변형) 사용처가 저장소 전체(`crates/bitvue-metrics/src/simd.rs`, `crates/bitvue-decode/src/strategy/{avx2,neon}.rs`)에서 grep 0건 — `loadu`/`storeu`(x86) 및 `vld1_u8`/`vld1q_u8`/`vst1_u8`(NEON, 애초에 비정렬 허용) 계열만 일관되게 사용됨.

---

### SIMD-003: 실제 처리한 청크 수와 명목 청크 수를 혼동해 tail 데이터가 조용히 유실됨
**분류**: SIMD · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
let chunks = len / 32;                       // "이론상" 청크 수
let safe_chunks = ((safe_len - start) / 32).min(chunks); // 버퍼 길이로 캡한 실제 처리 가능 청크 수

for i in 0..safe_chunks { /* SIMD 처리 */ }

stats.count = chunks * 32;                    // 버그: safe_chunks가 아니라 chunks를 씀
for i in (start + stats.count)..end { /* 나머지 스칼라 처리 */ }
// safe_chunks < chunks인 경우, [start+safe_chunks*32, start+chunks*32) 구간은
// SIMD 루프(safe_chunks에서 멈춤)에도, 나머지 루프(chunks*32부터 시작)에도 걸리지 않고
// 통째로 통계에서 누락된다.
```

**문제**:
- 안전한 반복 횟수(`safe_chunks`, 실제 버퍼 길이로 캡)와 명목상 반복 횟수(`chunks`, 요청된 윈도우 길이만으로 계산)가 다른 변수로 분리되어 있으면, 두 값 중 하나만 "처리된 바이트 수"로 잘못 채택했을 때 그 차이만큼의 구간이 SIMD 루프에도 나머지(tail) 스칼라 루프에도 포함되지 않고 사라진다.
- 이는 크래시나 패닉을 일으키지 않아 조용히 넘어간다 — 통계 결과(합, 평균)가 미묘하게 낮게(또는 다르게) 나올 뿐이라 단위 테스트가 우연히 정확히 나누어떨어지는 길이만 쓰면 절대 드러나지 않는다.
- `reference`와 `distorted` 두 버퍼의 길이가 다를 수 있는 코드 경로(비교 대상 스트림 간 해상도/프레임 길이 불일치, 또는 방어적으로 넣은 `min()` 클램프 자체)에서만 `safe_chunks < chunks`가 성립하므로, 정상 경로에서는 재현되지 않고 엣지 케이스에서만 나타난다.

**발생 조건**: SIMD 커널이 "요청된 범위 길이로 계산한 청크 수"와 "실제 버퍼 길이로 캡한 안전 청크 수"를 별도 변수로 두고, 그중 나머지(tail) 처리의 시작점 계산에 잘못된 쪽을 사용할 때.

**권장**:
```rust
let safe_chunks = ((safe_len - start) / 32).min(chunks);

for i in 0..safe_chunks { /* SIMD 처리 */ }

stats.count = safe_chunks * 32;               // 실제로 처리한 만큼만 count에 반영
for i in (start + stats.count)..end { /* 나머지는 반드시 안전 청크 이후부터 */ }
```
- "몇 개를 처리했는가"를 나타내는 변수는 SIMD 루프가 실제로 반복한 횟수(`safe_chunks`)에서 단 한 번만 파생시키고, 나머지(tail) 루프의 시작 오프셋도 반드시 그 값을 재사용한다 — `chunks`처럼 이름이 비슷한 다른 변수를 나머지 계산에 실수로 섞어 쓰지 않는다.
- 동일 함수에 `chunks`(이론값)와 `safe_chunks`(실측값)처럼 이름이 유사한 두 변수를 동시에 두는 설계 자체가 이런 실수를 유발하므로, 가능하면 하나로 통일하거나 이름을 명확히 구분한다(`requested_chunks` vs `processed_chunks`).

**탐지 방법**: Static: SIMD 루프 종료 후 "처리된 바이트/픽셀 수"를 나타내는 변수가 실제 루프 반복 횟수 변수에서 파생되는지, 아니면 별도로 재계산되는지 대조. Runtime: `reference`/`distorted` 길이를 의도적으로 다르게 만든 프로퍼티 기반 테스트로 SIMD 결과와 순수 스칼라 결과를 모든 길이 조합에서 비교(현재 단위 테스트처럼 딱 떨어지는 길이만 쓰지 않는다).

**예외**: `safe_chunks`와 `chunks`가 항상 같음이 호출부 계약으로 보장되는 내부 전용 함수(예: 항상 같은 길이의 두 버퍼만 받는 것으로 타입/불변식이 강제된 경우)라면 위험이 사라지지만, 그 계약이 실제로 강제되는지는 별도 검증이 필요하다.

**Bitvue 판정**: Suspected (재검증으로 Confirmed에서 하향, 2026-08-05) — 버그 패턴 자체는 실재: `crates/bitvue-metrics/src/simd.rs`의 `compute_window_stats_avx2`(112-118행 `safe_chunks` 계산, 211행 `stats.count = chunks * 32` — `safe_chunks`가 아니라 `chunks` 사용, 214행부터 나머지 루프가 `start + stats.count`에서 시작), `compute_window_stats_sse2`(350행), `compute_window_stats_neon`(485행) 세 함수 모두 동일 패턴을 가진다. 다만 이 함수의 유일한 실제 호출부인 `compute_window_stats_simd`(같은 파일 40-64행)의 유일한 프로덕션 호출자 `ssim()`(`crates/bitvue-metrics/src/lib.rs:199`)이 호출 직전 `if end > reference.len() || end > distorted.len() { return Err(...) }`로 이미 검증하므로, `safe_len = min(ref.len(), dist.len()) >= end`가 항상 성립하고 이로부터 `safe_chunks == chunks`가 수학적으로 항상 보장된다(`reference.len() != distorted.len()`이어도 마찬가지 — 관건은 두 길이가 다른지가 아니라 `end`가 둘 중 하나를 넘는지이고, 호출자가 그걸 막는다). 즉 현재 저장소의 유일한 실제 호출 경로로는 이 버그가 트리거되지 않는 잠재 결함(latent, unreachable via the sole current caller)이다. 향후 이 함수가 pre-validation 없이 다른 곳에서 재사용되면 즉시 활성화되므로 수정은 여전히 권장되지만, "지금 프로덕션에서 조용히 값이 틀리게 나온다"는 원래 판정은 근거 부족이었다. 길이가 다른 두 버퍼로의 프로퍼티 테스트는 여전히 유효한 권장사항.

---

### SIMD-004: 같은 파일의 형제 커널마다 경계 초과 대응 철학이 다름(무음 손실 vs 하드 오류)
**분류**: SIMD · **심각도**: Medium · **탐지**: Static/Structural

**나쁜 예**: (동일 파일 내 두 함수가 같은 유형의 "버퍼가 예상보다 짧을 수 있음" 상황에 다르게 대응)
```rust
// 함수 A: 짧으면 그냥 있는 만큼만 조용히 처리하고 넘어감
let safe_chunks = ((safe_len - start) / 32).min(chunks);
for i in 0..safe_chunks { /* ... */ }

// 함수 B (같은 파일): 짧으면 그 자리에서 Err 반환
if offset + 32 > reference.len() || offset + 32 > distorted.len() {
    return Err(BitvueError::InvalidData("SIMD buffer overflow".into()));
}
```

**문제**:
- 같은 파일, 같은 "버퍼 길이가 요청 범위보다 짧을 수 있다"는 위협 모델에 대해 한 함수는 조용히 있는 만큼만 처리(SIMD-003의 근본 원인)하고, 이웃 함수는 즉시 `Err`로 실패시킨다 — 호출자 입장에서 "이 크레이트의 SIMD 함수는 짧은 버퍼에 어떻게 반응하는가"를 예측할 수 없다.
- 두 전략 다 나름의 정당성은 있지만(전자는 "부분 통계라도 반환", 후자는 "잘못된 입력은 명시적으로 실패") 같은 파일 안에서 문서화되지 않은 채 혼재하면 리뷰어와 이후 유지보수자가 "이 함수는 방어적이니 저 함수도 그럴 것"이라고 잘못 유추하기 쉽다.
- 방어적인 쪽(`Err` 반환)이 오히려 더 안전한 기본값인데, 조용히 손실되는 쪽이 먼저 작성된 "레거시" 패턴으로 남아 새 코드가 그것을 복사해가는 방향으로 굳어질 위험이 있다.

**발생 조건**: 하나의 모듈/파일에 여러 SIMD 커널을 점진적으로 추가하면서 매번 그 시점의 판단으로 경계 처리 방식을 새로 결정할 때(공통 헬퍼로 통일하지 않을 때).

**권장**:
- 모듈 단위로 "짧은 버퍼에 대한 정책"을 하나로 정하고 문서화한다(예: "이 모듈의 모든 SIMD 커널은 실제 처리량을 반환값에 정직하게 반영하거나, 아니면 항상 Err로 실패한다 — 둘 중 하나만").
- 가능하면 경계 계산 자체를 공통 헬퍼 함수로 추출해 정책을 한 곳에서 강제한다.

**탐지 방법**: Structural: 같은 파일/모듈 내 SIMD 커널들을 나열해 "짧은 입력에 대한 반환값 형태"(부분 결과 vs Err)를 표로 비교.

**예외**: 함수마다 위협 모델이 실제로 다른 경우(하나는 신뢰된 내부 버퍼만 받고, 다른 하나는 사용자 입력에서 직접 파생된 길이를 받는 경우)라면 다른 정책이 오히려 타당할 수 있다 — 단, 그 이유를 코드 주석에 명시해야 한다.

**Bitvue 판정**: Confirmed — `crates/bitvue-metrics/src/simd.rs` 한 파일 안에서 `compute_window_stats_avx2/sse2/neon`(93-227, 230-366, 369-501행)은 `safe_chunks`로 조용히 부분 처리(SIMD-003의 근본 원인)하는 반면, `psnr_avx2`(679-773행, 특히 701-707행), `psnr_sse2`(788-877행, 809-815행), `psnr_neon`(883-974행, 902-908행)은 청크마다 `offset+32/16 > len`을 확인해 즉시 `Err(BitvueError::InvalidData(...))`로 실패한다. 두 정책 중 어느 쪽이 의도된 기본값인지 코드/주석에 명시되어 있지 않다.

---

### SIMD-005: 좁은 폭 곱셈(mullo) 결과가 부호 있는 오버플로 없이 정확하다는 가정이 modular 산술의 우연에 의존
**분류**: SIMD · **심각도**: Medium · **탐지**: Static/Manual

**나쁜 예**:
```rust
// diff는 i16 범위(-255..=255)로 부호 있는 뺄셈 결과.
// diff * diff는 최대 65025로, signed i16(최대 32767)를 초과한다.
let sq = _mm256_mullo_epi16(diff, diff); // "low 16-bit"만 취하는 truncating multiply
// sq는 이후 unsigned로 재해석되어 32비트로 zero-extend된다 —
// 65025 < 65536이므로 하위 16비트를 unsigned로 읽으면 우연히 값이 맞는다.
// 하지만 이 정합성은 "저장할 수 있는 최대값이 65536 미만"이라는 이 특정
// bit-depth/연산 조합에서만 성립하는 우연이지, 코드가 명시적으로 보장하는 성질이 아니다.
```

**문제**:
- `_mm*_mullo_epi16`은 두 16비트 값을 곱한 32비트 결과 중 하위 16비트만 반환한다(고정폭 truncating multiply). 피연산자를 부호 있는 값으로 다루면서 그 제곱을 부호 없는 값으로 재해석해 정합성을 맞추는 코드는, "제곱 결과가 항상 2^16 미만"이라는 값 범위 가정이 깨지는 순간(예: 10/12비트 처리 경로가 8비트 전용 이 코드 경로를 재사용하게 되는 리팩터링) 조용히 틀린 값을 낸다.
- 이런 종류의 정합성은 유닛 테스트로 잡히지 않는다 — 테스트가 같은 8비트 범위 내에서만 값을 넣는 한 항상 통과한다.
- 코드 어디에도 "이 값 범위를 벗어나면 틀린다"는 불변식이 assert나 타입으로 강제되어 있지 않으면, 다음 사람이 이 함수를 다른 비트심도에 재사용하기 쉽다.

**발생 조건**: 8비트 전용으로 설계된 SIMD 커널의 좁은 정수 폭 산술(mullo 등)을, 값 범위가 다른 비트심도(10/12비트) 처리에 그대로 재사용하거나 재사용을 시도할 때.

**권장**:
- 값 범위 가정을 코드에 명시적으로 문서화하고(`debug_assert!`로 8비트 범위를 강제하거나), 애초에 diff를 32비트 폭으로 승격한 뒤 곱해 truncation 자체를 없앤다(`_mm256_mullo_epi32`류 사용, 약간의 처리량 손해와 맞바꿈).
- 비트심도별로 다른 함수를 쓰는 경우 함수 이름/타입에 "8비트 전용"임을 드러내 실수로 재사용되지 않게 한다.

**탐지 방법**: Static: `mullo_epi16`류가 이후 unsigned 재해석·32비트 zero-extend로 이어지는 경로를 찾아, 곱셈 입력값의 최대 범위가 2^16을 넘지 않는다는 것이 코드/주석으로 보장되는지 확인. Manual: 값 범위 경계(diff=±255처럼 제곱이 2^16에 가까운 값)로 SIMD vs 스칼라 결과를 직접 비교.

**예외**: 입력이 항상 8비트 고정이고 그 사실이 함수 시그니처(타입)로 강제되는 경우 실질적 위험은 낮다 — 다만 "타입으로 강제"가 아니라 "현재 호출부가 우연히 8비트만 쓴다"는 것은 예외가 아니라 이 항목이 우려하는 상황 그 자체다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움. 관련: `METRIC.md`(PSNR 오버플로 항목, `crates/bitvue-metrics/src/simd.rs:749-753` 등 accumulation 단계의 정수 폭은 이미 N/A로 판정됨 — 이 항목은 accumulation이 아니라 `_mm256_mullo_epi16`/`_mm_mullo_epi16`(`simd.rs:155-160, 294-299, 724-725, 832-833` 등) 자체의 truncating-multiply 의미론이라는 별개 층위를 다룸). 현재 PSNR/SSIM은 8비트 `&[u8]` 버퍼만 다루므로(`METRIC.md:137` 판정 참고) diff² 최대값(65025)이 2^16을 넘지 않아 당장 관측 가능한 오류는 없을 것으로 보이나, 이 정합성이 코드에 assert로 강제되어 있지는 않음 — 10/12비트 PSNR/SSIM 경로가 이 함수를 재사용하게 될 경우 우선 재검토 필요.

---

### SIMD-006: saturating 클램프를 ISA마다 다른 메커니즘으로 구현하면서 교차검증이 없음
**분류**: SIMD · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**: (같은 "0-255로 클램프"를 ISA마다 다른 방식으로 구현하고 서로 비교하지 않음)
```rust
// AVX2 경로: 명시적 max/min으로 먼저 클램프한 뒤 pack
let clamped = _mm256_max_epi32(_mm256_min_epi32(v, _mm256_set1_epi32(255)), _mm256_setzero_si256());
let packed = _mm256_packus_epi16(_mm256_packs_epi32(clamped, clamped), ...);

// NEON 경로: 클램프 단계 없이 saturating narrow 인트린식 하나에 위임
let r_u8 = vqmovun_s16(r); // signed 16-bit -> unsigned 8-bit, saturating
```

**문제**:
- 두 경로가 "같은 결과"를 낸다는 것이 코드 상에서 자명하지 않다 — 하나는 명시적 min/max 비교 두 번, 다른 하나는 인트린식 하나의 내부 동작에 결과의 정확성을 위임한다. 경계값(정확히 0, 255, 또는 그 근처의 음수/큰 양수)에서 두 메커니즘이 실제로 항상 일치하는지는 코드를 읽는 것만으로 보장되지 않는다.
- ISA별 구현이 서로 다른 사람/시점에 작성되면, 한쪽만 버그가 수정되고 다른 쪽은 예전 동작 그대로 남는 drift가 발생하기 쉽다 — 특히 pack/narrow 계열 인트린식은 부호·포화 규칙이 미묘해 실수하기 쉽다.
- 이 불일치는 특정 CPU에서만 재현되므로("내 M1 Mac에서는 문제없는데 CI x86 러너에서는 색이 다르게 나온다"), 개발자가 흔히 쓰는 한 플랫폼에서는 발견되지 않는다.

**발생 조건**: 동일한 색공간 변환/클램프 로직을 AVX2/SSE2/NEON 각각에 대해 독립적으로 작성하고, 세 경로의 출력을 직접 비교하는 테스트가 없을 때.

**권장**:
- 클램프 경계값(0, 255, 그 바로 안팎)을 포함한 합성 입력으로 AVX2/NEON/스칼라 세 경로의 픽셀별 출력을 바이트 단위로 비교하는 테스트를 추가한다.
- 가능하면 클램프 로직을 "먼저 명시적으로 클램프 → 그 다음 narrow"처럼 ISA 간에 최대한 같은 순서의 연산으로 통일해, 인트린식의 암묵적 포화 동작에 결과 정확성을 맡기는 부분을 줄인다.

**탐지 방법**: Static: 동일 변환(YUV→RGB 등)의 AVX2/NEON 구현에서 클램프가 명시적 min/max인지 narrow 인트린식의 암묵적 saturate인지 대조. Runtime: 세 전략(`Scalar`/`Avx2`/`Neon`)에 동일한 경계값 프레임을 넣어 픽셀 단위 diff 테스트.

**예외**: 두 메커니즘이 정확히 동일한 결과를 낸다는 것을 이미 전수 비교 테스트로 검증해 둔 경우 위험이 사라진다.

**Bitvue 판정**: Confirmed — `crates/bitvue-decode/src/strategy/avx2.rs`의 `clamp_epi32_to_epu8`(287-298행)은 `_mm256_max_epi32`/`_mm256_min_epi32`로 명시적 클램프 후 pack하는 반면, `crates/bitvue-decode/src/strategy/neon.rs`의 `store_rgb_interleaved_neon` 직전 경로(예: 336-338행 `vqmovun_s16(r)`/`vqmovun_s16(g)`/`vqmovun_s16(b)`)는 명시적 min/max 없이 saturating narrow 인트린식에 클램프를 위임한다. 두 파일 모두 자체 단위 테스트만 있고(`avx2.rs:1077-1133`, `neon.rs:1130-1185`) 서로의 출력을 비교하는 테스트는 없음(SIMD-007과 동일 근거).

---

### SIMD-007: 교체 가능한 SIMD 전략들 간 골든 아웃풋 비교 테스트 부재
**분류**: SIMD · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
// Scalar/Avx2/Neon 세 구현이 같은 트레이트를 만족하고 런타임에 전략이
// 선택되지만("어느 전략이 골라질지는 실행 CPU에 달림"), 세 구현이
// 같은 입력에 대해 같은 픽셀을 내는지 검증하는 테스트가 어디에도 없음.
impl YuvConversionStrategy for Avx2Strategy { /* 자체 단위 테스트만 존재 */ }
impl YuvConversionStrategy for NeonStrategy { /* 자체 단위 테스트만 존재 */ }
impl YuvConversionStrategy for ScalarStrategy { /* 자체 단위 테스트만 존재 */ }
```

**문제**:
- Strategy 패턴으로 설계된 SIMD 코드는 "어느 구현이 선택되든 같은 결과가 나와야 한다"는 것이 설계의 핵심 불변식인데, 그 불변식 자체를 검증하는 테스트가 없으면 회귀(regression)를 어떤 CI도 잡아내지 못한다.
- 사용자 A는 x86_64+AVX2에서 실행하고 사용자 B는 Apple Silicon(NEON)에서 실행하면, 동일한 영상 파일을 열었을 때 픽셀 단위로 미세하게 다른 렌더링 결과를 볼 수 있다 — 이는 "버그 재현이 플랫폼에 따라 다르다"는 매우 디버깅하기 어려운 사용자 리포트로 이어진다.
- 각 전략의 자체 테스트(`test_avx2_*`, `test_neon_*`)는 "이 전략이 크래시 없이 동작하는가"만 검증하고 "다른 전략과 같은 결과를 내는가"는 검증하지 않는다 — 서로 다른 질문이다.

**발생 조건**: 여러 SIMD 백엔드를 같은 트레이트로 추상화하되, 전략 간 출력 동등성을 검증하는 통합 테스트를 별도로 만들지 않았을 때.

**권장**:
```rust
#[test]
fn all_strategies_agree_on_output() {
    let (y, u, v) = synthetic_yuv420_frame(64, 64); // 경계값 포함 비자명 패턴
    let scalar = ScalarStrategy::new().convert_yuv420_to_rgb(&y, &u, &v, 64, 64, ..., 8).unwrap();
    if Avx2Strategy::new().is_available() {
        let avx2 = Avx2Strategy::new().convert_yuv420_to_rgb(&y, &u, &v, 64, 64, ..., 8).unwrap();
        assert_eq!(scalar, avx2, "AVX2 output diverges from scalar reference");
    }
}
```
- 모든 전략을 같은 입력에 대해 스칼라(신뢰된 기준선)와 바이트 단위로 비교하는 테스트를 별도 파일/모듈에 둔다.
- 입력은 상수값(0, 128 등 우연히 정합성이 맞기 쉬운 값)이 아니라 각 8픽셀/32바이트 블록 경계에 걸치는 비자명한 패턴을 포함해야 한다.

**탐지 방법**: Structural: 같은 트레이트를 구현하는 여러 SIMD 전략 파일들을 나열해, 전략 간 출력을 직접 비교하는 테스트가 그중 어디에도 없는지 확인.

**예외**: 전략이 하나뿐이거나(대체 구현이 없는 단일 SIMD 경로), 전략 선택이 빌드 타임에 고정되어 런타임에 여러 경로가 실제로 공존하지 않는 경우.

**Bitvue 판정**: Confirmed — `crates/bitvue-decode/src/strategy/{avx2,neon,scalar}.rs` 세 파일의 `#[cfg(test)] mod tests`(각각 1077-1133, 1130-1185, 284-428행)는 모두 자기 자신에 대한 단위 테스트만 포함(`vs_scalar`/`cross`/`golden`/`equivalence` 류 이름 grep 0건). 대조적으로 `crates/bitvue-metrics/src/simd.rs`는 `test_window_stats_vs_scalar`(598-616행)와 `test_psnr_simd_vs_scalar`(640-667행)로 SIMD/스칼라 정합성을 이미 검증하고 있어 — 같은 저장소 안에서도 두 SIMD 서브시스템(`bitvue-metrics` vs `bitvue-decode`) 간 테스트 성숙도 격차가 뚜렷함.

---

### SIMD-008: 소입력 윈도우에서 SIMD dispatch/설정 오버헤드가 실제 이득을 상쇄
**분류**: SIMD · **심각도**: Medium · **탐지**: Runtime/Manual

**나쁜 예**:
```rust
// 5개의 256비트 누산기를 0으로 초기화하고, 루프가 0번 도는 경우에도
// (윈도우 길이 < 32바이트) 그 5개를 메모리로 spill해 스칼라 합으로
// 변환하는 "추출" 코드를 그대로 실행한다.
pub fn compute_window_stats_simd(reference: &[u8], distorted: &[u8], start: usize, end: usize) -> WindowStats {
    if is_x86_feature_detected!("avx2") {
        return unsafe { compute_window_stats_avx2(reference, distorted, start, end) };
        // (end - start) < 32 -> safe_chunks == 0 -> SIMD 루프는 0회 실행,
        // 그래도 5개 누산기 zero-init + 5번의 _mm256_storeu_si256 추출은 실행됨.
    }
    /* ... */
}
```

**문제**:
- AVX2 경로는 32바이트 미만 입력에서는 SIMD 루프를 단 한 번도 돌지 못하고 전량이 "나머지(tail)" 스칼라 루프로 처리되는데, 그 전에 5개의 256비트 누산기를 초기화하고 이후 각각을 스택 배열로 추출(`_mm256_storeu_si256` 5회 + `.iter().sum()` 5회)하는 코드는 여전히 실행된다 — 이 부분이 실제로는 아무 실질 계산도 대체하지 못하면서 순수 오버헤드로만 남는다.
- 함수 진입 시 `is_x86_feature_detected!` 호출 자체는 내부적으로 캐시되어 저렴하지만, `unsafe` AVX2 함수로의 분기(인라인 불가능한 별도 심볼일 가능성)와 그 안의 레지스터 준비/추출 코드는 입력이 작을수록 상대적 비중이 커진다.
- SSIM류 슬라이딩 윈도우 계산처럼 "짧은 구간에 대해 매우 많은 횟수" 호출되는 사용 패턴에서는, 이 고정 오버헤드가 호출 횟수만큼 누적되어 스칼라 전용 구현보다 오히려 느려질 수 있다.

**발생 조건**: SIMD 커널이 호출 시점의 입력 길이를 보고 "이 길이는 SIMD 청크 하나도 못 채운다"는 사실을 사전에 확인하지 않고 무조건 SIMD 경로로 진입할 때, 특히 그 커널이 짧은 구간에 대해 매우 빈번히 호출되는 워크로드(윈도우 슬라이딩 등)에서.

**권장**:
```rust
pub fn compute_window_stats_simd(reference: &[u8], distorted: &[u8], start: usize, end: usize) -> WindowStats {
    const MIN_SIMD_LEN: usize = 32; // 최소 1개 AVX2 청크
    if end.saturating_sub(start) < MIN_SIMD_LEN {
        return compute_window_stats_scalar(reference, distorted, start, end);
    }
    /* 기존 feature-detection 분기 */
}
```
- 진입점에서 길이가 최소 청크 크기 미만이면 SIMD 분기 자체를 타지 않고 스칼라로 직행하는 크기 임계값(threshold)을 둔다.
- 실사용 호출 패턴(윈도우 크기 분포)을 벤치마크로 측정해 임계값을 정한다 — 추측이 아니라 실측 기반으로.

**탐지 방법**: Runtime: 실제 호출부의 전형적인 길이 분포를 계측(`PERF.md`의 프로파일링 원칙 참고)해, SIMD 청크 크기 미만으로 호출되는 비율을 확인. Manual: 짧은 길이 전용 마이크로벤치마크로 SIMD 경로와 스칼라 전용 경로의 처리량 비교.

**예외**: 호출 길이가 항상 SIMD 청크 크기를 크게 상회한다고 호출부 구조상 보장되는 경우(예: 프레임 전체를 한 번에 처리하는 PSNR 경로) 이 문제는 사실상 발생하지 않는다.

**Bitvue 판정**: Suspected — `METRIC.md`(725행 판정)에 따르면 `compute_window_stats_simd`의 실사용 SSIM 윈도우는 8x8=64픽셀로 고정되어 있다. 이 호출이 64바이트 연속 구간 하나로 이루어진다면 AVX2 청크(32바이트) 2개에 해당해 SIMD 이득이 있지만, SSIM은 2차원 윈도우이므로 실제로는 행(row) 단위로 8바이트씩 8회 호출되는 구조일 가능성이 있다 — 그 경우 매 호출이 `safe_chunks == 0`(8바이트 < 32바이트)이 되어 이 항목이 우려하는 순수 오버헤드 상황이 상시 발생한다. 정확한 호출 그래뉼래리티(한 번에 전체 윈도우를 넘기는지, 행 단위로 쪼개 넘기는지)는 `bitvue-metrics`의 SSIM 호출부(`ssim.rs` 등, 이번 조사 범위 밖)를 확인해야 확정 가능 — Phase 2 감사에서 호출 그래뉼래리티 확인 및 필요 시 최소 길이 임계값 도입 권장.

---

### SIMD-009: 수평 합산(horizontal reduction)을 store+스칼라 합으로 처리해 hadd 인트린식을 활용하지 않음
**분류**: SIMD · **심각도**: Low · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// 8-lane 누산기를 메모리로 store한 뒤, 스칼라 iterator로 합산.
let mut sums = [0i32; 8];
_mm256_storeu_si256(sums.as_mut_ptr() as *mut __m256i, sum_x_acc);
stats.sum_x = sums.iter().map(|&x| x as u64).sum();
```

**문제**:
- 벡터 누산기를 레지스터에서 곧바로 수평 합산하지 않고 메모리로 내보낸(store) 뒤 스칼라 루프로 다시 합치는 패턴은 store-to-load forwarding 지연과, 벡터 유닛에서 스칼라 유닛으로의 왕복 비용을 매번 지불한다.
- `_mm256_hadd_epi32` 계열이나 `_mm256_extracti128_si256` + `_mm_hadd_epi32` 트리, NEON의 `vaddvq_u32`(전체 레인 합을 스칼라 하나로) 같은 전용 수평 합산 인트린식을 쓰면 이 왕복을 없앨 수 있다.
- 누산기 하나당 store+iterator-sum을 반복하는 코드(예: 하나의 함수에서 5개 통계량 각각에 대해 이 패턴을 5번 반복)는 그만큼 비용이 5배로 누적된다.

**발생 조건**: 여러 개의 SIMD 누산기 최종값을 하나의 스칼라 합계로 축약해야 하는 모든 지점(윈도우 통계, MSE 누산 등) — 특히 그 축약이 루프 안이 아니라 루프 밖에서 한 번만 일어나 상대적 이득이 적어 보일 때 최적화가 누락되기 쉽다.

**권장**:
```rust
// x86: 128비트 반씩 더한 뒤 hadd로 트리 축약, 또는
let sum128 = _mm_add_epi32(_mm256_castsi256_si128(sum_x_acc), _mm256_extracti128_si256(sum_x_acc, 1));
let sum64 = _mm_hadd_epi32(sum128, sum128);
let total = _mm_cvtsi128_si32(_mm_hadd_epi32(sum64, sum64));

// NEON: 전용 across-lane add
let total: u32 = vaddvq_u32(sum_x_acc);
```
- 최종 축약 지점에서만 인트린식을 바꾸면 되므로 루프 본문 변경 없이 적용 가능한 국소적 최적화다.
- 축약이 함수당 1회뿐이라 절대적 이득은 작을 수 있으나, 호출 빈도가 높은 함수(윈도우 통계처럼 짧은 구간에 반복 호출되는 함수, SIMD-008 참고)에서는 상대적 비중이 커진다.

**탐지 방법**: Static: SIMD 누산기 직후 `_mm*_storeu_*` → 배열 → `.iter().sum()`/`.map().sum()` 패턴을 grep. Runtime: `cargo flamegraph`에서 축약 코드가 hot path 상당 비중을 차지하는지 확인(SIMD-008과 함께 볼 것 — 호출 빈도가 낮으면 우선순위가 낮다).

**예외**: 축약이 함수 호출 전체 빈도에서 무시할 수준(프레임당 1회 등)이라면 가독성을 우선해 store+스칼라 합을 유지해도 실질적 손해가 없다.

**Bitvue 판정**: Confirmed(패턴 존재) — `crates/bitvue-metrics/src/simd.rs`의 `compute_window_stats_avx2`(195-209행, 5개 통계량 각각에 대해 `_mm256_storeu_si256` 후 `.iter().map(...).sum()`), `compute_window_stats_sse2`(333-348행), `compute_window_stats_neon`(468-483행), `psnr_avx2`(740-743, 759행), `psnr_sse2`(847-850, 865행), `psnr_neon`(947-950행) 모두 동일한 store+스칼라-합 패턴을 사용. 저장소 전체에서 `_mm256_hadd`/`_mm_hadd`/`vaddvq`/`vpaddq` 계열 인트린식 사용은 grep 0건 — 수평 합산 전용 인트린식이 단 한 곳에도 쓰이지 않음. 호출 그래뉼래리티가 SIMD-008과 같은 이유로 실측 필요(윈도우 단위로 자주 호출된다면 이 항목의 우선순위가 올라감).

---

### SIMD-010: AVX와 SSE 폭이 혼재하는 실행 경로에서 `vzeroupper` 처리 여부 미검증
**분류**: SIMD · **심각도**: Medium · **탐지**: Runtime/Manual

**나쁜 예**: (개념적 — 소스만으로는 실제 발생 여부를 판단할 수 없다)
```rust
#[target_feature(enable = "avx")]
unsafe fn psnr_avx(a: &[u8], b: &[u8], w: usize, h: usize) -> Result<f64> {
    // 이 함수 자신은 256비트 YMM을 쓰지 않더라도, 같은 바이너리/프로세스
    // 안에서 다른 시점에 AVX2(256비트) 코드가 실행된 뒤 legacy SSE(128비트)
    // 코드로 넘어가는 시점에 vzeroupper가 누락되면 하드웨어가 상위 128비트
    // 저장을 위해 수십~수백 사이클의 전환 페널티를 겪는다(Sandy/Ivy Bridge류
    // 구형 마이크로아키텍처에서 특히 심함, 최신 CPU는 영향이 작음).
    psnr_sse2(a, b, w, h)
}
```

**문제**:
- 256비트(YMM) AVX/AVX2 명령어 실행 후 상위 128비트를 명시적으로 클리어(`vzeroupper`)하지 않고 128비트(XMM) SSE 계열 명령어로 전환하면, 일부 마이크로아키텍처에서 상당한 전환 페널티가 발생한다.
- Rust/LLVM은 `#[target_feature(enable = "avx"/"avx2")]` 함수의 반환 지점에 일반적으로 `vzeroupper`를 자동 삽입하지만, 이는 컴파일러 최적화 수준·인라인 여부·함수 경계에 따라 항상 보장되는 동작이 아니며 소스 코드만 읽어서는 실제로 삽입되었는지 확인할 수 없다.
- 이 저장소처럼 런타임 feature detection으로 AVX2/AVX/SSE2 함수를 같은 프로세스 수명 안에서 번갈아 호출하는 구조(`is_x86_feature_detected!` cascade)는 정확히 이 전환이 반복적으로 일어날 수 있는 조건을 만든다.

**발생 조건**: 런타임에 선택된 여러 폭(256비트/128비트)의 `#[target_feature]` 함수가 같은 프로세스 안에서 번갈아 호출되는 모든 구조.

**권장**:
- 릴리스 빌드 바이너리를 `objdump -d`/`llvm-mca`로 확인해 AVX/AVX2 함수의 반환 직전에 `vzeroupper`가 실제로 존재하는지 검증한다.
- 의심스러우면 함수 끝에 `std::arch::x86_64::_mm256_zeroupper()`를 명시적으로 호출해 컴파일러 추론에 의존하지 않는다.
- `perf stat`으로 AVX/SSE 전환이 빈번한 hot path에서 실제 사이클 카운트 이상 유무를 측정한다(최신 Intel/AMD CPU는 페널티가 크게 줄었으므로 실측 없이 가정하지 않는다).

**탐지 방법**: Static은 이 문제를 확정할 수 없음(컴파일러 출력에 의존) — 반드시 Runtime(디스어셈블리 확인, `perf` 계측)로 검증.

**예외**: 대상 CPU가 모두 최신 세대(Skylake 이후 등, 전환 페널티가 사실상 사라진 마이크로아키텍처)로 한정된다고 문서화되어 있다면 우선순위가 크게 낮아진다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움. 소스 코드만으로는 컴파일러가 `vzeroupper`를 실제로 삽입하는지 확인할 수 없음 — `psnr_avx`가 `psnr_sse2`를 직접 호출하는 구조(SIMD-012 참고, `crates/bitvue-metrics/src/simd.rs:776-781`)와 `StrategyRegistry`가 런타임에 AVX2/스칼라를 오가며 선택하는 구조(`crates/bitvue-decode/src/strategy/registry.rs`)가 이 항목의 전제 조건(AVX/SSE 폭 혼재 실행)을 만족하는지는 릴리스 바이너리 디스어셈블리 확인이 필요.

---

### SIMD-011: unsafe `#[target_feature]` 함수를 수동 강제 선택 경로에서 가용성 재검증 없이 호출
**분류**: SIMD · **심각도**: Critical · **탐지**: Static/Structural

**나쁜 예**:
```rust
// "테스트/벤치마크용으로 특정 전략을 강제"하는 API가 가용성을 확인하지 않으면
// 사용자가 AVX2 없는 CPU에서 강제로 Avx2를 선택해 SIGILL을 유발할 수 있다.
pub fn set_strategy(strategy_type: StrategyType) {
    CURRENT.store(strategy_type); // 가용성 검사 없이 그대로 저장
}
```

**문제**:
- SIMD-001과 원인은 같지만(런타임 feature 없이 unsafe 함수 호출) 진입 경로가 다르다 — 자동 감지 로직은 올바르게 작성했더라도, "테스트/벤치마크를 위해 특정 전략을 수동으로 강제"하는 별도 API가 같은 검증을 생략하면 우회로가 생긴다.
- 이런 강제 선택 API는 보통 테스트/벤치마크 코드에서만 쓰이지만, CLI 플래그나 설정 파일로 노출되면 일반 사용자가 실수로 자신의 CPU가 지원하지 않는 전략을 강제할 수 있다.

**발생 조건**: 자동 감지와 별도로 전략을 수동 지정할 수 있는 API가 존재할 때.

**권장**:
```rust
pub fn set_strategy(strategy_type: StrategyType) -> Result<(), String> {
    if !strategy_type.is_available() {
        return Err(format!("{strategy_type} not available on this CPU"));
    }
    CURRENT.store(strategy_type);
    Ok(())
}
```
- 수동 강제 경로도 자동 감지 경로와 동일한 가용성 검사를 반드시 통과하게 하고, 실패 시 명확한 `Result::Err`로 알린다.

**탐지 방법**: Structural: 전략/구현을 수동으로 선택할 수 있는 모든 공개 API를 나열해 각각이 가용성 검사를 거치는지 확인.

**예외**: 강제 선택 API가 `#[cfg(test)]`로 완전히 제한되어 프로덕션 바이너리에 포함되지 않는 경우 위험이 사라진다.

**Bitvue 판정**: N/A — 이미 권장 패턴대로 방어됨. `crates/bitvue-decode/src/strategy/registry.rs`의 `StrategyRegistry::set_strategy`(87-123행)는 `StrategyType::Avx2`/`Neon`/`Metal` 각각에 대해 `if !StrategyType::X.is_available() { return Err(...) }`(101-103, 108-110, 115-117행)를 강제한 뒤에만 저장하며, 이 함수는 `#[cfg(test)]`가 아니라 공개 API(`pub fn set_strategy`, 234행)로 노출되어 있음에도 우회 경로가 없다.

---

### SIMD-012: 상위 등급 ISA 라벨이 실제로는 하위 등급 구현으로 위장
**분류**: SIMD · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
/// AVX-optimized PSNR (Intel Sandy Bridge+, AMD Bulldozer+)
#[target_feature(enable = "avx")]
unsafe fn psnr_avx(reference: &[u8], distorted: &[u8], width: usize, height: usize) -> Result<f64> {
    // 실제로는 AVX(256비트 정수/부동소수점) 명령어를 전혀 쓰지 않고
    // SSE2(128비트) 구현으로 그대로 위임한다.
    psnr_sse2(reference, distorted, width, height)
}
```

**문제**:
- 함수명·doc comment·`#[target_feature(enable = "avx")]` 선언 모두 "이것은 AVX 최적화 구현"이라고 주장하지만 함수 본문은 SSE2 구현을 그대로 호출할 뿐이다 — 결과는 정확하지만(SSE2도 올바른 값을 낸다) "AVX면 SSE2보다 빨라야 한다"는 성능 기대와 실제 처리량 사이에 괴리가 생긴다.
- `is_x86_feature_detected!` cascade(AVX2 → AVX → SSE2)에서 "AVX2는 없지만 AVX는 있는" 드문 CPU(Sandy/Ivy Bridge 세대)가 이 경로를 타면, 이름과 문서가 약속한 "AVX 가속"을 전혀 받지 못한 채 SSE2와 동일한 처리량으로 실행된다.
- 이런 "라벨은 있지만 구현이 비어있는" 함수가 방치되면, 나중에 누군가 "AVX 경로는 이미 있으니 됐다"고 판단해 실제 AVX 256비트 구현을 영영 작성하지 않게 될 위험이 있다(WIRING.md의 배선 문제와 인접한 패턴 — 다만 여기서는 "안 쓰인다"가 아니라 "쓰이지만 라벨과 다른 것을 실행한다"는 점이 다르다).

**발생 조건**: 성능 계층 사다리(스칼라 < SSE2 < AVX < AVX2 등)를 미리 설계해 두고, 중간 단계 구현을 "일단 자리만 채워두고 나중에 채우자"는 의도로 하위 구현에 위임해 둔 채 방치할 때.

**권장**:
```rust
/// AVX 전용 256비트 부동소수점 구현이 아직 없어 SSE2로 폴백한다.
/// TODO(SIMD-012): 실제 AVX 256비트 정수/부동소수점 경로 구현.
#[target_feature(enable = "avx")]
unsafe fn psnr_avx(reference: &[u8], distorted: &[u8], width: usize, height: usize) -> Result<f64> {
    psnr_sse2(reference, distorted, width, height)
}
```
- 위임 사실을 doc comment와 함수 내부 주석에 명시하고, 원래 계획했던 등급의 구현이 아직 없다는 것을 TODO로 추적한다.
- 또는 애초에 "AVX 전용" 중간 단계를 만들지 않고 AVX2/SSE2 두 단계로만 cascade를 단순화해, 실제로 존재하지 않는 최적화 단계를 코드에 노출하지 않는다.

**탐지 방법**: Static: `#[target_feature(enable = "X")]` 함수의 본문이 다른(더 하위 등급의) `#[target_feature]` 함수를 그대로 호출하고 반환하는 패턴을 grep.

**예외**: 위임이 명시적으로 문서화되어 있고("아직 미구현, 폴백") 팀이 그 상태를 인지하고 있다면 심각도는 낮아진다 — 이 저장소의 경우도 코드 주석("For simplicity, fallback to SSE2 for now")이 위임 사실 자체는 밝히고 있어 완전히 숨겨진 상태는 아니다.

**Bitvue 판정**: Confirmed — `crates/bitvue-metrics/src/simd.rs:776-781`의 `psnr_avx` 함수가 정확히 이 패턴. doc comment(775행 `/// AVX-optimized PSNR (Intel Sandy Bridge+, AMD Bulldozer+)`)와 `#[target_feature(enable = "avx")]` 선언에도 불구하고 함수 본문(780행)은 `psnr_sse2(reference, distorted, width, height)`를 그대로 반환하며, 함수 내부 주석(779행 `// For simplicity, fallback to SSE2 for now`)이 위임 사실을 밝히고는 있음. `is_x86_feature_detected!` cascade(`simd.rs:518-524`)상 AVX2는 없지만 AVX는 있는 CPU에서만 이 경로가 실제로 선택됨 — 결과 정확성에는 영향 없으나 "AVX 가속"이라는 이름의 성능 약속은 지켜지지 않음.

---

### SIMD-013: 루프 불변 broadcast 상수를 루프 안에서 매 반복 재생성(형제 함수와 불일치)
**분류**: SIMD · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
for y in 0..height {
    for x in (0..width).step_by(8) {
        // BT.601 계수를 8픽셀마다(즉 매 반복마다) 새로 broadcast —
        // 값은 루프 전체에서 절대 바뀌지 않는데도 루프 안에 위치.
        let v_scaled = _mm256_mullo_epi32(v_vec, _mm256_set1_epi32(181));
        let u_scaled_g = _mm256_mullo_epi32(u_vec, _mm256_set1_epi32(44));
        /* ... */
    }
}
```

**문제**:
- `_mm256_set1_epi32(N)` 같은 broadcast 인트린식은 루프 변수에 의존하지 않는 순수 연산이므로 이론적으로는 컴파일러가 loop-invariant code motion으로 루프 밖으로 끌어올려 줄 수 있지만, 이는 최적화 수준·인라이닝·레지스터 압박 상황에 따라 항상 보장되지 않는다 — 소스에서 명시적으로 루프 밖에 두는 것이 유일하게 확실한 방법이다.
- 같은 파일 안의 "형제" 함수들이 동일한 상수를 루프 밖에서 한 번만 만드는 패턴을 쓰는데 특정 함수 하나만 다르게 작성되어 있으면, 그 파일을 읽는 사람이 "이 함수는 뭔가 다른 이유가 있어서 저렇게 했나?"라고 오해하게 만드는 불필요한 인지 부하를 유발한다 — 실제로는 그냥 놓친 것일 뿐이다.
- 8픽셀마다 5개의 broadcast를 반복하는 것이 최적화되지 않은 채 남으면(레지스터 압박이 높아 컴파일러가 끌어올리지 못하는 경우), 프레임 전체(수백만 픽셀)에 걸쳐 누적되는 실질적 명령어 수 증가로 이어진다.

**발생 조건**: 같은 파일에 유사한 SIMD 커널을 여러 개(포맷/비트심도 조합마다) 작성하면서, 어떤 것은 초기 버전에서 상수를 인라인으로 넣고 이후 리팩터링(상수 hoisting)이 다른 함수에만 적용되었을 때.

**권장**:
```rust
let v_coeff = _mm256_set1_epi32(181);
let u_g_coeff = _mm256_set1_epi32(44);
/* 나머지 계수도 루프 밖에서 한 번만 생성 */

for y in 0..height {
    for x in (0..width).step_by(8) {
        let v_scaled = _mm256_mullo_epi32(v_vec, v_coeff);
        let u_scaled_g = _mm256_mullo_epi32(u_vec, u_g_coeff);
        /* ... */
    }
}
```
- 값이 루프 변수에 의존하지 않는 모든 broadcast/상수 생성은 반드시 가장 바깥 루프 이전으로 끌어올린다.
- 같은 파일에 유사 커널이 여러 개 있다면 리팹터링 시 전체를 일관되게 수정한다 — 하나만 고치고 넘어가지 않는다.

**탐지 방법**: Static: `_mm256_set1_*`/`_mm_set1_*`/`vdupq_n_*` 호출이 `for`/`while` 루프 본문 안에 있는지, 같은 파일의 다른 함수와 위치가 일관되는지 grep 후 대조.

**예외**: 컴파일러가 실제로 항상 끌어올려 준다는 것을 디스어셈블리로 확인했고, 소스 가독성상 루프 안에 두는 것이 의도된 경우(드묾) — 다만 이 경우도 파일 내 일관성은 유지해야 한다.

**Bitvue 판정**: Confirmed — `crates/bitvue-decode/src/strategy/avx2.rs`의 `yuv420_to_rgb_avx2_impl`(8비트, 161-285행)만 `_mm256_set1_epi32(181/44/91/227/128)` 호출을 최내부 x-루프 본문 안(238-253행 부근)에 인라인으로 둔다. 같은 파일의 `yuv422_to_rgb_avx2_impl`(431-528행)은 442-445행에서, `yuv444_to_rgb_avx2_impl`(537-630행)은 546-550행에서, 16비트 계열 `yuv420/422/444_to_rgb_avx2_inner`(640-767, 789-906, 928-1035행)도 각각 652-656/801-805/938-942행에서 동일 계수를 함수 최상단(루프 밖)에 명시적으로 hoisting한다 — `yuv420_to_rgb_avx2_impl` 하나만 이 파일의 나머지 5개 형제 함수와 다른 패턴. 실제 어셈블리에서 LLVM이 CSE로 끌어올렸는지는 미검증(소스 레벨 불일치만 확인됨).

---

### SIMD-014: 256비트 벡터의 128비트 레인 분할/재조합 로직이 실제 필요한 데이터 범위와 불일치할 위험
**분류**: SIMD · **심각도**: Critical · **탐지**: Runtime/Manual

**나쁜 예**: (실제 저장소 패턴을 단순화)
```rust
// 32바이트(32픽셀)를 로드했지만 이번 반복에 필요한 것은 앞의 8픽셀뿐.
let y_vec = _mm256_loadu_si256(y_plane.as_ptr().add(y_idx) as *const __m256i);
let y_low = _mm256_castsi256_si128(y_vec);              // bytes[0..16]
let y_high = _mm256_extracti128_si256(y_vec, 1);         // bytes[16..32]
// cvtepu8_epi32는 128비트 입력의 "하위 8바이트만" 32비트로 zero-extend한다.
let y_low_i32 = _mm256_cvtepu8_epi32(y_low);   // 실제로 필요한 8픽셀: bytes[0..8]
let y_high_i32 = _mm256_cvtepu8_epi32(y_high); // 이번 반복과 무관한 bytes[16..24]
// 서로 다른(겹치지 않는) 두 픽셀 그룹을 비트 OR로 합친다 — 값이 아니라
// 비트가 겹치지 않는다는 우연에 기대지 않는 한 결과는 둘 다 아닌 값이 된다.
let y_i32 = _mm256_or_si256(y_low_i32, y_high_i32);
```

**문제**:
- `_mm256_cvtepu8_epi32`는 256비트 결과를 내지만 입력으로 받는 것은 128비트뿐이며, 그마저 하위 8바이트만 사용한다 — 32바이트를 로드해 두 개의 128비트 절반으로 나눈 뒤 각각에 이 인트린식을 적용하면, 두 결과는 원본 32바이트 중 `[0..8)`과 `[16..24)`라는 서로 인접하지 않은 두 구간에서 나온 값이 된다.
- 이번 반복(8픽셀 단위)에서 실제로 필요한 것은 `[0..8)` 하나뿐인데, 관련 없는 `[16..24)` 구간에서 얻은 값을 비트 OR로 섞으면 각 픽셀 값이 0이 아닌 한(거의 항상 0이 아님) 결과는 두 원본 값 중 어느 것도 아닌 임의의 합성값이 된다 — 이는 룩업이 아니라 데이터 손상이다.
- 픽셀값이 모두 0이거나(검은 프레임) 두 그룹 중 하나가 우연히 0인 테스트 데이터로는 이 문제가 절대 드러나지 않는다 — 실제 다양한 밝기의 영상에서만 나타나는 렌더링 오염(눈에 띄는 색 번짐/노이즈)으로 관측될 가능성이 있다.
- 유사한 레인 분할 재조합 복잡성은 RGB 인터리빙 저장 단계(저 128비트만 반복 사용해 상위 절반 픽셀 데이터가 실질적으로 버려지거나 잘못된 소스에서 재구성되는 것으로 보이는 코드)에도 나타난다 — 인접한 두 함수(위젠 단계와 저장 단계)가 같은 종류의 실수를 공유할 가능성을 시사한다.

**발생 조건**: 256비트 벡터를 두 개의 128비트 절반으로 나눠 각각에 "하위 N바이트만 쓰는" 폭 변환 인트린식(`cvtepu8_epi32`, `cvtepu16_epi32` 등)을 적용할 때, 그 결과로 얻는 두 그룹이 원래 로드한 32바이트 중 실제로 필요한 연속 구간과 일치하는지 주석/테스트로 검증하지 않은 채 작성할 때.

**권장**:
- 처리 단위(이번 예: 8픽셀)에 정확히 맞는 만큼만 로드한다 — 필요 이상으로 32바이트를 통째로 로드한 뒤 그중 일부만 쓰는 대신, 애초에 `_mm_loadu_si128`(16바이트)이나 그보다 좁은 로드로 필요한 데이터만 가져온다.
- 레인 분할이 꼭 필요하다면, 분할된 각 절반이 정확히 어느 원본 바이트 구간에 대응하는지 주석으로 명시하고, 그 구간들을 합칠 때 OR/ADD 등 비트 연산의 의미론이 실제로 올바른지(겹치지 않는 값의 OR는 일반적으로 틀린 결합 방식) 재검토한다.
- 비자명한 픽셀 패턴(그라디언트, 체커보드 등 모든 바이트가 서로 다른 값)으로 SIMD 결과와 스칼라 결과를 픽셀 단위로 비교하는 차분 테스트를 추가한다(SIMD-007).

**탐지 방법**: Static: `_mm256_cvtepu8_epi32`/`_mm256_cvtepu16_epi32`처럼 "하위 절반만 쓰는" 폭 변환 인트린식이 `castsi256_si128`/`extracti128_si256`로 얻은 두 절반 모두에 적용된 뒤 그 결과가 `_mm256_or_si256`으로 합쳐지는 패턴을 grep해, 두 결과가 실제로 필요한 연속 구간에 대응하는지 개별 확인. Runtime: 0이 아닌 비자명한 값으로 채운 8배수가 아닌 폭의 프레임을 AVX2 경로와 스칼라 경로 양쪽으로 렌더링해 픽셀 단위로 diff.

**예외**: 두 절반이 실제로 이번 반복에서 필요한 정확히 같은 논리적 데이터를 (다른 정렬로) 담고 있는 것이 증명된 경우(예: 필요한 데이터를 애초에 두 번 복제해서 로드한 설계) — 이 경우 OR 결합이 의도된 동작일 수 있으나, 그 사실이 코드에 명시되어 있어야 한다.

**Bitvue 판정**: Suspected — `crates/bitvue-decode/src/strategy/avx2.rs`의 `yuv420_to_rgb_avx2_impl`(224-229행 `y_low_i32`/`y_high_i32`/`_mm256_or_si256`), `yuv422_to_rgb_avx2_impl`(478-483행), `yuv444_to_rgb_avx2_impl`(569-574행) 모두 동일 패턴. 인트린식 의미론(`_mm256_cvtepu8_epi32`가 128비트 입력의 하위 8바이트만 사용)에 대한 분석 결과 이번 반복에 필요한 8픽셀(`y_low_i32`, bytes[0..8])과 무관한 다음다음 블록의 데이터(`y_high_i32`, bytes[16..24])를 OR로 합치는 것으로 보이나, 최종 사용처(`store_rgb_interleaved`, 311-360행)가 `r`/`g`/`b`의 저(low) 128비트만 재사용하는 방식(313-315행 `_mm256_castsi256_si128`, 352행에서 `rg_high`와 함께 `b_low`를 재사용)과 맞물려 상쇄되거나 의도치 않게 "정답과 우연히 같아지는" 경로가 있을 가능성을 배제할 수 없어 Confirmed가 아닌 Suspected로 표기함. 기존 테스트(`avx2.rs:1077-1133`)는 상수 픽셀값(`vec![0; 100]` 등)만 사용해 이 문제가 드러나지 않는 입력만 exercise함 — Phase 2 감사에서 8배수 아닌 폭 + 그라디언트 패턴으로 AVX2 vs 스칼라 픽셀 단위 차분 테스트를 최우선 권장.

---

### SIMD-015: SIMD 용량 메타데이터(speedup_factor)가 실측이 아닌 하드코딩 상수
**분류**: SIMD · **심각도**: Low · **탐지**: Static/Structural

**나쁜 예**:
```rust
pub const fn avx2() -> Self {
    Self {
        speedup_factor: 4.5, // 어떤 CPU, 어떤 해상도, 어떤 비트심도 기준인지 불명
        /* ... */
    }
}
```

**문제**:
- `speedup_factor`가 상수로 박혀 있으면, 실제 실행 중인 CPU 세대·워크로드(해상도, 비트심도, YUV 포맷)에 따라 실제 배율이 크게 다를 수 있음에도 UI/로그에는 항상 같은 숫자가 노출된다.
- 이 값이 사용자에게 노출되는 진단 정보(`available_strategies()`가 "~4.5x speedup" 문자열을 만드는 데 사용)라면, 실측과 다른 숫자가 사용자를 오도할 수 있다.
- 벤치마크가 이 상수를 갱신하는 절차와 연결되어 있지 않으면, 코드가 바뀌어 실제 배율이 달라져도(SIMD-013처럼 최적화가 누락/추가되어도) 이 상수는 영원히 그대로 남는다.

**발생 조건**: SIMD 전략의 "예상 성능"을 코드 상수로 미리 박아두고, 이를 갱신하는 벤치마크/CI 파이프라인이 없을 때.

**권장**:
- 상수 대신 "측정 방법과 측정 시점"을 doc comment에 명시하거나(예: "2026-08 벤치마크, 1920x1080 8bit YUV420, M1 Pro 기준"), 가능하면 벤치마크 결과를 빌드 시점에 주입한다.
- 사용자에게 노출하는 배율은 "예상치"임을 UI 문구로 분명히 하고, 가능하면 실행 중 실측치(자체 A/B 타이밍)로 대체한다.

**탐지 방법**: Structural: 성능 배율을 나타내는 상수가 어떤 실측 근거(벤치마크 커밋, 측정 조건)와 연결되어 있는지 추적.

**예외**: 순수 내부 로깅/디버깅 용도로만 쓰이고 사용자에게 노출되지 않는다면 우선순위는 낮다.

**Bitvue 판정**: Confirmed(사실 관계만) — `crates/bitvue-decode/src/strategy/mod.rs`의 `StrategyCapabilities::scalar/avx2/neon/metal`(53-100행)이 각각 `speedup_factor: 1.0/4.5/3.5/9.0`을 상수로 선언하며, `metal()`의 9.0은 Metal 전략 자체가 아직 미구현(`registry.rs:114-117` `is_available()`이 항상 `false`)인데도 "예상 배율"로 존재한다. 이 값들이 어떤 측정에서 유래했는지 소스에 근거가 없으며, `registry.rs:176, 190, 204`의 `available_strategies()`가 이 상수를 그대로 `"~4.5x speedup"` 등 사용자向 문자열로 노출한다. 관련: `PERF.md`(벤치마크/측정 안티패턴을 다루는 인접 파일, 이 항목은 그중 "SIMD 용량 메타데이터"라는 좁은 사례).

---

### SIMD-016: unsafe 함수의 버퍼 크기 계약이 타입이 아닌 주석에만 존재
**분류**: SIMD · **심각도**: Medium · **탐지**: Static/Structural

**나쁜 예**:
```rust
/// Precondition: offset + 24 <= rgb.len() (caller must validate)
#[inline]
unsafe fn store_rgb_interleaved(rgb: &mut [u8], offset: usize, r: __m256i, g: __m256i, b: __m256i) {
    let dst = rgb.as_mut_ptr().add(offset);
    // 이후 dst.add(0)..dst.add(23)까지 경계 검사 없이 직접 포인터 쓰기.
    // offset이 이 불변식을 어긴 채로 넘어오면 컴파일러도 런타임도 막지 못한다.
}
```

**문제**:
- "호출자가 검증해야 한다"는 계약이 함수 시그니처(타입)가 아니라 doc comment 한 줄에만 존재하면, 이 계약은 Rust 컴파일러가 강제하는 것이 아니라 사람이 매 호출부에서 산술을 정확히 다시 검산해야만 유지된다.
- 호출부가 여러 곳(8비트/10비트/12비트 × YUV420/422/444 조합)이고 각각이 서로 다른 루프 구조·경계 검사 변수(`y_safe`, `uv_safe` 등 이름은 같지만 계산식이 조금씩 다른)로 이 불변식을 만족시키려 하면, 그중 하나가 리팩터링 중 계산식을 미묘하게 바꾸면서 불변식을 깨도 컴파일 타임에는 아무 신호가 없다.
- 이런 함수는 `#[inline]`이 붙어 있어 크래시가 발생해도 호출 스택이 인라인되어 정확한 원인 지점을 디버거에서 특정하기 어려운 경우가 많다.

**발생 조건**: 여러 호출부에서 재사용되는 `unsafe` 저수준 SIMD 헬퍼 함수가, 안전성 불변식을 타입(예: 길이가 보장된 슬라이스 래퍼, `try_into`로 검증된 인덱스)이 아니라 주석으로만 표현할 때.

**권장**:
```rust
/// # Safety
/// `dst` must have at least 24 valid bytes for writing.
#[inline]
unsafe fn store_rgb_interleaved_raw(dst: *mut u8, r: __m256i, g: __m256i, b: __m256i) {
    /* ... */
}

#[inline]
fn store_rgb_interleaved(rgb: &mut [u8], offset: usize, r: __m256i, g: __m256i, b: __m256i) -> Result<(), ConversionError> {
    let slice = rgb.get_mut(offset..offset + 24).ok_or(ConversionError::PlaneSizeMismatch { .. })?;
    unsafe { store_rgb_interleaved_raw(slice.as_mut_ptr(), r, g, b) }
    Ok(())
}
```
- 가능하면 `unsafe` 경계를 가장 안쪽(순수 포인터 연산)으로 좁히고, 그 바로 바깥에 `slice::get_mut`/`checked_add` 등으로 안전하게 길이를 검증하는 safe wrapper를 둔다 — 불변식 검증이 "주석을 믿는 것"이 아니라 "타입이 반환하는 `Option`/`Result`를 처리하는 것"이 되게 한다.
- 불가피하게 순수 `unsafe fn`으로 남긴다면, `debug_assert!(offset + 24 <= rgb.len())`을 함수 최상단에 추가해 최소한 디버그 빌드/테스트에서는 계약 위반이 패닉으로 드러나게 한다.

**탐지 방법**: Static: `unsafe fn`의 doc comment에 "Precondition"/"Caller must ensure" 문구가 있으면서 함수 본문에 그 불변식을 재확인하는 `assert!`/`debug_assert!`가 없는지 grep.

**예외**: 호출부가 단 하나뿐이고 그 호출부가 같은 파일 안에서 함께 리뷰되는 경우, 또는 이미 `debug_assert!`로 계약이 이중화되어 있는 경우 위험이 줄어든다.

**Bitvue 판정**: Confirmed(패턴 존재, 문제 여부는 Phase 2 판단) — `crates/bitvue-decode/src/strategy/avx2.rs`의 `store_rgb_interleaved`(300-302행 `/// Precondition: offset + 24 <= rgb.len() (caller must validate)`)와 `crates/bitvue-decode/src/strategy/neon.rs`의 `store_rgb_interleaved_neon`(988-990행 동일 문구)이 모두 이 패턴 — `debug_assert!`로 이 불변식을 재확인하는 코드는 두 함수 어디에도 없음(grep 0건). 각 호출부(`yuv420/422/444_to_rgb_avx2_impl` 등 6곳, `yuv420/422/444_to_rgb_neon_impl` 등 6곳)는 `if x + 8 <= width && y_safe && uv_safe`류 조건으로 계약을 만족시키려 하나, 이 조건식들이 함수마다 조금씩 다르게 작성되어 있어(예: 8비트 경로는 `y_idx + 8 <= y_plane.len()`, 16비트 경로는 `y_idx.checked_mul(2).and_then(|v| v.checked_add(16))...`) 계약이 실제로 항상 유지되는지는 코드 검토만으로 확정하기 어려움.

---

### SIMD-017: 거의 동일한 SIMD 커널이 포맷×비트심도 조합마다 복제되어 유지보수 리스크가 누적됨
**분류**: SIMD · **심각도**: Medium · **탐지**: Structural

**나쁜 예**: (구조적 패턴 — 코드 자체는 정상 동작)
```rust
// YUV420/422/444 × 8bit/16bit = 6개의 거의 동일한 함수가
// BT.601 계수·클램프·저장 로직을 각각 독립적으로 반복.
unsafe fn yuv420_to_rgb_avx2_impl(...) { /* ~120줄, BT.601 계수 인라인 */ }
unsafe fn yuv422_to_rgb_avx2_impl(...) { /* ~100줄, 거의 동일한 BT.601 계수/클램프 */ }
unsafe fn yuv444_to_rgb_avx2_impl(...) { /* ~95줄, 또 거의 동일 */ }
unsafe fn yuv420_to_rgb_avx2_inner(...) { /* 16bit 버전, 또 거의 동일 */ }
// ... NEON에도 같은 6개 조합이 별도로 존재
```

**문제**:
- SIMD-013(루프 불변 상수 hoisting 누락)처럼 "형제 함수 중 하나만 다르게 작성된" 버그가 발생하는 근본 원인이 바로 이 복제 구조다 — 로직을 한 곳에서 고치면 되는 게 아니라 ISA(AVX2/NEON) × 포맷(420/422/444) × 비트심도(8/10/12) 조합의 수만큼 각각 따로 고쳐야 한다.
- SIMD-014(레인 분할 재조합 버그)도 세 개의 8비트 AVX2 함수(420/422/444)에 동일하게 나타난다 — 하나의 잠재 버그가 복제로 인해 여러 곳에 동시에 존재하게 된 사례다.
- 코드량이 커질수록(이 두 파일만 각각 1,000줄 이상) 리뷰 비용과 다음 포맷/ISA 추가 시의 복제 압박(NEON 담당자가 AVX2 코드를 보고 그대로 흉내내 같은 실수를 반복할 위험)이 함께 커진다.

**발생 조건**: 색공간 변환처럼 "핵심 수식은 하나지만 포맷/비트심도 조합이 여러 개"인 SIMD 코드를, 조합마다 완전히 독립된 함수로 처음부터 다시 쓸 때(공통 커널 + 포맷별 로드/스토어 어댑터로 분리하지 않을 때).

**권장**:
- BT.601 계수 계산·클램프처럼 포맷에 무관한 "산술 커널"과, 포맷마다 다른 "로드(서브샘플링 방식)/스토어(인터리브 방식)" 부분을 분리한다 — 산술 커널을 제네릭 함수나 매크로로 한 번만 작성하고, 포맷별 코드는 그 커널을 호출하는 얇은 어댑터로 줄인다.
- 비트심도별 반복(`match bit_depth { 10 => ..., 12 => ... }`처럼 이미 부분적으로 쓰이는 패턴)을 인트린식 레벨까지 일관되게 확장한다.
- 매크로(`macro_rules!`) 또는 제네릭 + 트레이트로 포맷 파라미터를 추상화하는 것을 고려하되, 매크로 전개 결과가 디버깅하기 어려워지는 트레이드오프도 함께 평가한다.

**탐지 방법**: Structural: 파일 내 함수들을 diff/유사도 비교 도구로 대조해 복제 비율을 정량화. 신규 포맷/ISA 추가 시 걸리는 코드량(줄 수)을 추적해 복제 부담이 늘어나는 추세인지 확인.

**예외**: 조합의 수가 적고(2-3개) 각 조합의 로직이 실제로 상당히 다르다면(진짜 다른 알고리즘) 억지로 통합하는 것이 오히려 가독성을 해칠 수 있다.

**Bitvue 판정**: Confirmed(사실 관계) — `crates/bitvue-decode/src/strategy/avx2.rs`(1,134행)는 `yuv420/422/444_to_rgb_avx2_impl`(8비트, 3개)과 `yuv420/422/444_to_rgb_avx2_inner`(16비트, 3개) 총 6개의 변환 커널을 포함하며 각각 BT.601 계수(181/44/91/227) 선언과 클램프 로직을 독립적으로 반복(SIMD-013이 지적한 hoisting 불일치, SIMD-014가 지적한 레인 재조합 위험이 모두 이 6개 중 여러 곳에 동일 패턴으로 나타남). `crates/bitvue-decode/src/strategy/neon.rs`(1,186행)도 동일한 6-커널 구조를 별도로(코드 공유 없이) 구현. 두 파일 사이, 그리고 각 파일 내부 6개 커널 사이에 공통 산술 로직을 추출한 공유 함수/매크로는 없음(각 커널이 계수 선언부터 클램프까지 완전히 독립).
