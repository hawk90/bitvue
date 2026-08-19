# Anti-Pattern Catalog — BUILD: 빌드·Feature·Workspace 관리

Bitvue 안티패턴 카탈로그의 일부입니다 (전체 목록은 `docs/anti-patterns/INDEX.md` 참조). 이 문서는 Phase 4 웨이브로, ~20-crate Cargo 워크스페이스 + feature-gated native dependency(dav1d, libvmaf 등) + 워크스페이스 밖 `src-tauri` 앱이라는 Bitvue 특유의 빌드 토폴로지를 겨냥한 항목들을 다룹니다.

---

### BUILD-001: feature flag가 additive하지 않음
**분류**: BUILD · **심각도**: Critical · **탐지**: Structural|Runtime

**나쁜 예**:
```toml
# codec-av1/Cargo.toml
[features]
default = []
hardware-decode = []
software-decode = []

[dependencies]
# hardware-decode와 software-decode가 동시에 켜지면
# 서로 다른 디코더 struct를 같은 이름으로 재정의(#[cfg] 분기)
```
```rust
#[cfg(feature = "hardware-decode")]
pub struct Av1Decoder { /* GPU 경로 */ }

#[cfg(feature = "software-decode")]
pub struct Av1Decoder { /* dav1d 경로 */ }
// 둘 다 켜지면 컴파일 에러, 혹은 둘 다 꺼지면 타입이 사라짐
```

**문제**:
- Cargo의 feature unification 규칙상 워크스페이스 내 어떤 crate가 A를 요구하고 다른 crate가 B를 요구하면 최종 빌드는 A+B가 동시에 켜진 상태가 된다.
- feature가 "이것 아니면 저것"으로 설계되면 unification이 일어나는 순간 컴파일이 깨지거나, 더 나쁘게는 조용히 한쪽이 승리해 의도치 않은 코드가 링크된다.
- `cargo test --workspace --all-features`, `cargo build --workspace` 같은 표준 명령이 원천적으로 불가능해진다.

**발생 조건**:
- 여러 crate가 같은 feature-gated crate에 서로 다른 feature 조합으로 의존할 때.
- CI가 crate 단위로만 빌드하고 워크스페이스 전체 빌드/`--all-features`를 검증하지 않을 때 오래 숨어 있는다.

**권장**:
```toml
[features]
default = []
# mutually-exclusive 대신 additive + 런타임 선택으로 전환
decode = []
hw-accel = ["decode"]  # hw-accel은 decode의 상위집합, 배타적이지 않음

[dependencies]
```
```rust
// 컴파일 타임 배타적 struct 대신 런타임 dispatch
pub enum DecodeBackend { Hardware, Software }
pub struct Av1Decoder { backend: DecodeBackend }
```
- feature는 "코드를 추가"하는 용도로만 쓰고 "코드를 대체"하는 용도로 쓰지 않는다.
- 배타적 선택이 꼭 필요하면 컴파일 타임 feature가 아니라 런타임 config/enum으로 표현한다.

**탐지 방법**:
- CI에 `cargo hack check --feature-powerset` 또는 최소 `cargo check --workspace --all-features` 단계 추가.
- `cargo tree -e features -i <crate>`로 동일 crate에 대해 워크스페이스 내 다른 feature 조합 요구가 있는지 확인.

**예외**:
- feature가 순수히 optional 모듈 추가(예: `serde` 지원 추가)이고 서로 다른 feature가 동일 타입/함수를 재정의하지 않는다면 문제없다.

**Bitvue 판정**: N/A — 실제 feature(`ffmpeg`/`vvdec`/`vmaf`/`vmaf-cuda`/`parallel`)는 모두 additive이며 동일 타입 재정의 충돌 사례를 찾지 못함(`crates/bitvue-decode/src/lib.rs`, `crates/bitvue-metrics/src/lib.rs`). 다만 CI가 `--all-features`를 검증하는 job이 없어(docs job만 `cargo doc --all-features || true`로 실패를 무시, `.github/workflows/ci.yml:395`) 향후 충돌이 생겨도 못 잡을 위험은 남아있음.

---

### BUILD-002: default feature가 너무 무거움
**분류**: BUILD · **심각도**: High · **탐지**: Structural|Manual

**나쁜 예**:
```toml
# bitvue-core/Cargo.toml
[features]
default = ["av1", "hevc", "avc", "vp9", "vvc", "vmaf", "ssim", "gpu-accel", "cli-tools"]
```
```rust
// bitvue-core를 "파서 유틸만 필요해서" 의존하는 다운스트림 crate도
// libvmaf-sys, dav1d-sys, GPU 드라이버 바인딩까지 전부 컴파일해야 함
```

**문제**:
- 워크스페이스 내 다른 crate나 외부 소비자가 `bitvue-core = "0.1"`만 추가해도 필요 없는 native dependency(C 라이브러리, GPU SDK)까지 전부 빌드된다.
- 첫 clean build 시간이 codec 전체를 링크하는 시간으로 고정되어, 단순 파서 기능만 쓰는 CI job이나 로컬 iteration에서도 무거운 빌드를 감수해야 한다.
- "가벼운 用途"가 애초에 불가능해져 crate 분리의 이점이 사라진다.

**발생 조건**:
- 라이브러리를 처음 만들 때 "일단 다 켜두면 편하다"는 이유로 default를 채울 때.
- 다운스트림에서 `default-features = false`를 매번 명시해야 하는데 문서화가 안 되어 있을 때.

**권장**:
```toml
[features]
default = []  # 최소 기능만 — 파싱/구조 분석
av1 = ["dep:dav1d-sys"]
hevc = []
vmaf = ["dep:libvmaf-sys"]
full = ["av1", "hevc", "avc", "vp9", "vvc", "vmaf", "ssim"]
```
- `default = []` 또는 "구조 파싱만" 같은 최소 공통분모로 시작하고, 무거운 조합은 `full` 같은 명시적 opt-in feature로 묶는다.
- CLI/앱 바이너리 crate에서만 `full`을 켜고, 라이브러리 crate는 최소 default를 유지한다.

**탐지 방법**:
- `cargo tree --no-default-features -e features`와 default 포함 버전을 비교해 native dependency 개수 차이를 측정.
- clean build 시간을 `default-features = false` 여부로 비교(`cargo build --timings`).

**예외**:
- crate가 애초에 "올인원 CLI 배포판" 역할이라면(예: `bitvue-cli`) default가 풍부해도 목적에 맞는다 — 단, 이 경우 라이브러리 crate와 명확히 분리돼야 한다.

**Bitvue 판정**: N/A — 핵심 라이브러리 crate(`bitvue-core`, `bitvue-decode`, `bitvue-metrics`, `bitvue-av1-codec`)는 모두 `default = []`(각 Cargo.toml `[features]` 섹션 확인). 무거운 조합을 끌어안는 쪽은 `bitvue-cli`/`bitvue-codecs`인데 이는 문서가 인정하는 "올인원 배포판" 예외에 해당.

---

### BUILD-003: codec별 native dependency가 전체 빌드에 포함
**분류**: BUILD · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```toml
# bitvue-decode/Cargo.toml
[dependencies]
dav1d-sys = "0.5"      # AV1
libde265-sys = "0.3"   # HEVC
openh264-sys = "0.5"   # AVC
vpx-sys = "0.10"       # VP9
# feature gate 없이 무조건 링크
```

**문제**:
- 사용자가 AV1 파일만 열어보고 싶어도 HEVC/AVC/VP9 네이티브 디코더 4종을 전부 컴파일·링크해야 한다.
- 각 native dependency는 별도의 C 툴체인, 시스템 라이브러리, ABI 가정을 끌고 오므로 빌드 실패 지점이 4배로 늘어난다.
- 바이너리 크기와 공격 표면(attack surface)이 실제 사용 여부와 무관하게 최대치로 고정된다.

**발생 조건**:
- 초기 프로토타입에서 "일단 다 붙여보자"로 시작해 feature gate를 나중으로 미뤘을 때.
- 여러 codec crate가 공통 트레이트 하나에 의존하면서 트레이트 구현체 등록이 feature와 분리돼 있지 않을 때(→ BUILD-019와 연결).

**권장**:
```toml
[dependencies]
dav1d-sys = { version = "0.5", optional = true }
libde265-sys = { version = "0.3", optional = true }
openh264-sys = { version = "0.5", optional = true }
vpx-sys = { version = "0.10", optional = true }

[features]
av1 = ["dep:dav1d-sys"]
hevc = ["dep:libde265-sys"]
avc = ["dep:openh264-sys"]
vp9 = ["dep:vpx-sys"]
```
- codec 1개 = feature 1개 = optional native dependency 1개 원칙을 강제한다.
- CI 매트릭스에 codec 단일 feature 조합(`--features av1 --no-default-features`)을 각각 빌드하는 job을 추가해 격리를 검증한다.

**탐지 방법**:
- `cargo tree --no-default-features`에서 native `-sys` crate가 하나도 안 나와야 정상.
- `cargo build --features av1 --no-default-features -v`로 다른 codec의 C 컴파일러 호출이 발생하는지 로그 확인.

**예외**:
- 컨테이너 파싱(MP4/MKV demux)처럼 모든 codec 처리에 공통으로 필요한 최소 유틸은 예외 — 이건 애초에 codec-specific이 아니다.

**Bitvue 판정**: Suspected — `crates/bitvue-decode/Cargo.toml`에서 `dav1d = { workspace = true }`가 `optional`이 아닌 필수 의존성(반면 `ffmpeg-next`는 `optional = true`로 올바르게 gate됨). AV1 디코드가 필요 없는 소비자도 native `libdav1d` 링크를 피할 수 없음. 다만 순수 파서 crate(`bitvue-avc`/`-hevc`/`-vp9`/`-vvc`)는 native dependency가 전혀 없어(구조상 확인) 문서가 우려하는 "4개 codec C 툴체인 전부" 시나리오보다는 범위가 훨씬 좁음.

---

### BUILD-004: debug/release 동작 차이
**분류**: BUILD · **심각도**: High · **탐지**: Runtime|Semantic

**나쁜 예**:
```rust
pub fn compute_frame_offset(base: u32, stride: u32, row: u32) -> u32 {
    base + stride * row  // debug 빌드: overflow 시 panic, release: wrapping
}

#[cfg(debug_assertions)]
fn validate_nal_length(len: usize) {
    assert!(len < MAX_NAL_SIZE, "NAL too large");
}
#[cfg(not(debug_assertions))]
fn validate_nal_length(_len: usize) {} // release에서는 검증 자체가 사라짐
```

**문제**:
- integer overflow checking은 dev 프로파일에서만 기본 활성화되므로, 손상된 비트스트림 입력에 대해 debug 빌드는 panic하고 release 빌드는 조용히 wrap-around된 잘못된 오프셋으로 메모리를 읽는다.
- `#[cfg(debug_assertions)]`로 감싼 검증 로직이 있으면 "QA에서는 항상 통과, 사용자 환경(release)에서만 실패"하는 재현 불가능한 버그가 생긴다.
- 릴리스에서만 발생하는 버그는 디버거 attach도 최적화로 인해 어려워진다.

**발생 조건**:
- 비트스트림 파서처럼 외부(신뢰할 수 없는) 입력을 다루는 코드에서 산술 연산에 `debug_assert!`만 걸어두고 release 경로에 명시적 bounds check가 없을 때.
- 성능 프로파일링을 debug 빌드로만 하고 release 빌드 회귀 테스트를 안 돌릴 때.

**권장**:
```rust
pub fn compute_frame_offset(base: u32, stride: u32, row: u32) -> Result<u32, OffsetError> {
    stride.checked_mul(row)
        .and_then(|v| v.checked_add(base))
        .ok_or(OffsetError::Overflow)
}

fn validate_nal_length(len: usize) -> Result<(), ParseError> {
    if len >= MAX_NAL_SIZE {
        return Err(ParseError::NalTooLarge(len));
    }
    Ok(())
}
```
```toml
[profile.release]
overflow-checks = true  # 신뢰 불가 입력을 다루는 파서 crate에서는 release에도 강제
```
- 신뢰할 수 없는 입력(비트스트림)을 다루는 산술은 `checked_*`/`Result`로 표현하고 `debug_assert`에 의존하지 않는다.
- 필요하면 파서 crate 프로파일에서 `overflow-checks = true`를 release에도 명시적으로 켠다(성능 영향 측정 후).

**탐지 방법**:
- fuzz 타깃을 debug와 release 양쪽 프로파일로 모두 돌려 결과 차이를 비교.
- clippy `#[cfg(debug_assertions)]` 사용처를 grep해 검증/보안 로직이 그 안에 있는지 수동 확인.

**예외**:
- 순수 성능 계측용 assertion(예: 캐시 히트율 sanity check)처럼 실패해도 안전에 영향이 없는 경우는 debug 전용이어도 무방하다.

**Bitvue 판정**: Confirmed — 루트 `Cargo.toml`의 `[profile.release]`에 `overflow-checks`가 명시돼 있지 않아 release는 Cargo 기본값(overflow-checks off)을 따름. 동시에 신뢰 불가 입력(비트스트림)을 다루는 엔트로피 디코더가 `debug_assert!`에 안전성 검증을 의존: `crates/bitvue-av1-codec/src/symbol/arithmetic.rs:314,337,354,397` (cnt 상하한 검증)와 `crates/bitvue-av1-codec/src/tile/partition.rs:455` (BlockSize 최소값 검증) — release 빌드에서는 이 체크들이 사라짐.

---

### BUILD-005: local system library에 암묵 의존
**분류**: BUILD · **심각도**: High · **탐지**: Structural|Manual

**나쁜 예**:
```rust
// libvmaf-sys/build.rs
fn main() {
    // 시스템에 이미 설치된 libvmaf를 pkg-config로 찾아 링크
    pkg_config::probe_library("libvmaf").unwrap();
}
```
```
$ cargo build
error: failed to run custom build command for `libvmaf-sys`
Package libvmaf was not found in the pkg-config search path.
```

**문제**:
- 개발자 A의 macOS는 `brew install libvmaf`로 이미 설치돼 있어 빌드가 되지만, CI 러너나 신규 기여자 머신에는 없어서 첫 `cargo build`부터 실패한다.
- 링크되는 libvmaf의 버전이 머신마다 달라 "내 컴퓨터에서는 되는데" 버그의 근원이 된다(예: VMAF 점수가 머신 간 미세하게 다름).
- README에 시스템 패키지 설치 안내가 없으면 온보딩 자체가 막힌다.

**발생 조건**:
- native C 라이브러리를 `-sys` crate로 감쌀 때 vendored 소스 대신 `pkg-config`/`find_library`로 시스템 설치를 가정할 때.
- Docker/CI 이미지에 해당 시스템 패키지 설치 단계가 누락됐을 때.

**권장**:
```toml
[dependencies]
libvmaf-sys = { version = "0.4", features = ["vendored"] }
```
```rust
// build.rs — vendored 소스를 기본으로, 시스템 라이브러리는 opt-in
fn main() {
    if cfg!(feature = "system-libvmaf") {
        pkg_config::probe_library("libvmaf").expect(
            "system-libvmaf feature requires libvmaf installed \
             (brew install libvmaf / apt install libvmaf-dev)"
        );
    } else {
        cc::Build::new().file("vendor/libvmaf/main.c").compile("vmaf");
    }
}
```
- 기본값은 vendored(소스 포함) 빌드로 두고, 시스템 라이브러리 사용은 명시적 opt-in feature로 분리한다.
- 시스템 의존이 불가피하면 버전 요구사항과 설치 명령을 build.rs 실패 메시지에 직접 출력한다.

**탐지 방법**:
- clean Docker 컨테이너(시스템 패키지 미설치)에서 `cargo build` 성공 여부를 CI로 검증.
- `build.rs`에서 `pkg_config`, `find_library`, `env::var("PATH")` 등 시스템 탐색 호출을 grep.

**예외**:
- 이미 시스템에 널리 배포된 라이브러리(예: OpenSSL)에 대해 vendored 옵션도 함께 제공하며, 시스템 라이브러리 사용이 문서화되고 CI에서 두 경로 모두 검증된다면 허용.

**Bitvue 판정**: Confirmed — `Cargo.lock`에서 `dav1d-sys`는 `system-deps`에, `libvmaf-sys`는 `pkg-config`에 의존(vendored 옵션 없음). `.github/workflows/ci.yml`이 이를 우회하려고 매 OS마다 수동 설치를 함: Linux `apt-get install libdav1d-dev`, macOS `brew install dav1d`, Windows `vcpkg install dav1d`(각 job에 반복). 시스템에 libdav1d/libvmaf가 없는 신규 기여자 머신은 `cargo build`부터 실패하는 구조.

---

### BUILD-006: bindgen을 매 빌드 실행
**분류**: BUILD · **심각도**: Medium · **탐지**: Runtime|Structural

**나쁜 예**:
```rust
// dav1d-sys/build.rs
fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .generate()
        .expect("bindgen failed");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap();
    // cargo:rerun-if-changed 지정 없음 → 매 빌드마다 무조건 재실행
}
```

**문제**:
- `rerun-if-changed`를 지정하지 않으면 Cargo는 해당 crate의 build script를 사실상 매번(혹은 최소 clean 빌드마다) 다시 실행하며, bindgen은 libclang을 초기화하고 C 헤더 전체를 파싱하는 무거운 작업이라 incremental build 시간을 크게 늘린다.
- CI에서 캐시(`~/.cargo`, `target/`)가 있어도 bindgen 출력이 캐시되지 않으면 매번 수 초~수십 초가 낭비된다.
- libclang 버전이 CI 이미지마다 달라 재현성 문제(BUILD-007)와 겹친다.

**발생 조건**:
- `-sys` crate가 여러 개이고 각각 bindgen을 호출할 때 빌드 시간이 누적된다.
- 워크스페이스 전체 `cargo build`를 자주 도는 iteration 루프에서 특히 체감된다.

**권장**:
```rust
fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=vendor/dav1d/include");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_path = out_path.join("bindings.rs");

    // 헤더가 안 바뀌었으면 커밋된 pregenerated 바인딩을 그대로 복사
    if cfg!(feature = "pregenerated-bindings") {
        std::fs::copy("src/bindings_pregenerated.rs", &bindings_path).unwrap();
        return;
    }

    bindgen::Builder::default()
        .header("wrapper.h")
        .generate()
        .expect("bindgen failed")
        .write_to_file(&bindings_path)
        .unwrap();
}
```
- 헤더/wrapper 변경 파일에 대해 `cargo:rerun-if-changed`를 명시해 불필요한 재실행을 막는다.
- 안정된 C API에 대해서는 bindgen 결과를 커밋해두고, `bindgen` feature는 헤더 업데이트 시에만 수동으로 켜는 방식도 고려한다(BUILD-015와 연동).

**탐지 방법**:
- `cargo build --timings`로 bindgen 관련 build script 실행 시간 확인.
- `CARGO_LOG=cargo::core::compiler::fingerprint=trace cargo build`로 재빌드 트리거 원인 추적.

**예외**:
- 헤더가 자주 바뀌는 활발한 개발 단계(vendored 라이브러리를 자주 업데이트)에서는 항상 bindgen을 도는 편이 drift 방지에 낫다 — 이 경우 캐싱 대신 시간 단축(예: `--no-layout-tests`)에 집중.

**Bitvue 판정**: N/A — 저장소 내 자체 `build.rs`가 전혀 없음(`find . -name build.rs -not -path "*/target/*"` 결과 0건; 유일하게 있었던 `src-tauri/build.rs`도 2026-08-08 Electron 전환으로 `src-tauri` 자체가 retire되며 사라짐, `Cargo.toml:3` 주석 "src-tauri retired 2026-08-08"). bindgen은 외부 `-sys` crate(`dav1d-sys`, `libvmaf-sys`)에서만 transitively 쓰이며 우리가 고칠 수 있는 build.rs가 아님.

---

### BUILD-007: reproducible build 불가
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```rust
// build.rs
fn main() {
    let build_time = std::time::SystemTime::now();
    println!("cargo:rustc-env=BUILD_TIME={:?}", build_time);
    println!("cargo:rustc-env=BUILD_HOST={}", hostname::get().unwrap().to_string_lossy());
}
```
```toml
[dependencies]
dav1d-sys = "0.5"  # 상한 없는 caret 요구사항, Cargo.lock 미커밋
```

**문제**:
- 빌드마다 바뀌는 타임스탬프/호스트명을 바이너리에 내장하면 동일 소스 커밋에서도 바이트 단위로 다른 바이너리가 나와 캐시 공유(sccache, CI 아티팩트 재사용)나 바이너리 diff 기반 회귀 분석이 불가능해진다.
- `Cargo.lock`을 커밋하지 않으면 "어제 빌드"와 "오늘 빌드"가 서로 다른 transitive dependency 버전을 받아, 동일 태그를 다시 체크아웃해도 같은 결과를 재현할 수 없다.
- 보안 감사/공급망 검증(SBOM) 관점에서 "이 바이너리가 정확히 어떤 소스로 만들어졌는지" 증명할 수 없다.

**발생 조건**:
- 라이브러리 crate는 `Cargo.lock`을 커밋하지 않는 게 Rust 관례지만, 최종 바이너리(앱, CLI)를 배포하는 crate까지 `Cargo.lock`을 `.gitignore`에 넣었을 때.
- 디버깅 편의를 위해 빌드 메타데이터를 무분별하게 심을 때.

**권장**:
```rust
fn main() {
    // 빌드 시각 대신 소스 기반 식별자만 사용
    println!("cargo:rustc-env=GIT_SHA={}", git_sha());
    // SOURCE_DATE_EPOCH 관례를 따르면 재현 빌드 도구와 호환됨
    let epoch = env::var("SOURCE_DATE_EPOCH").unwrap_or_default();
    println!("cargo:rustc-env=BUILD_EPOCH={epoch}");
}
```
```
# 바이너리를 배포하는 앱/CLI crate는 Cargo.lock을 커밋
git add Cargo.lock  # src-tauri, cli 등 [[bin]] crate
```
- 최종 산출물(바이너리) crate는 `Cargo.lock`을 반드시 커밋한다. 순수 라이브러리 crate는 커밋하지 않는 것이 여전히 정상이다.
- 빌드 메타데이터가 필요하면 `SOURCE_DATE_EPOCH` 환경변수 관례를 따르거나 git SHA처럼 소스에서 결정론적으로 파생되는 값만 사용한다.

**탐지 방법**:
- 동일 커밋을 두 번 clean build해 바이너리 해시(`sha256sum target/release/bitvue`)를 비교.
- `git ls-files | grep Cargo.lock`으로 바이너리 crate의 lock 파일 커밋 여부 확인.

**예외**:
- 순수 라이브러리 crate(다운스트림이 자체 버전 범위를 결정해야 하는 경우)는 `Cargo.lock` 미커밋이 정상 관례다.

**Bitvue 판정**: Confirmed — `.gitignore:7`이 `Cargo.lock`을 전면 제외하고 있고, `git ls-files | grep Cargo.lock`은 0건. 그런데 `bitvue-sidecar`(Electron 데스크톱 앱의 백엔드 프로세스, `.github/workflows/build-electron-app.yml:83` `cargo build --release -p bitvue-sidecar`, `scripts/package_electron.sh:23`도 동일)와 `bitvue-cli`(`.github/workflows/publish-packages.yml:202` `cargo build --release -p bitvue-cli -p bitvue-gui`)는 실제 배포되는 바이너리 crate — lockfile 미커밋은 재현 불가능한 릴리스 빌드로 이어짐. (구 `src-tauri`도 동일 사례였으나 2026-08-08 Electron 전환으로 retire됨.)

---

### BUILD-008: crate version이 제각각 drift
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
# crates/parser-av1/Cargo.toml
version = "0.4.2"
# crates/parser-hevc/Cargo.toml
version = "0.3.7"
# crates/parser-avc/Cargo.toml
version = "0.5.1"
# crates/core/Cargo.toml
version = "0.2.9"
```

**문제**:
- 20개 crate가 각자 독립적인 버전 번호를 가지면 "이 릴리스에 어떤 조합의 crate 버전이 들어있는지"를 파악하기 위해 매번 20개의 Cargo.toml을 대조해야 한다.
- 릴리스 노트/CHANGELOG를 crate별로 따로 관리하면 사용자가 "bitvue 0.9 릴리스에 av1 파서 버그 수정이 포함됐는지" 알기 어렵다.
- semver 규칙상 서로 다른 crate가 서로 다른 속도로 major bump를 하면 워크스페이스 내부 의존 관계(`parser-core 0.2` vs `parser-av1`이 요구하는 `parser-core ^0.3`)가 어긋나기 쉽다.

**발생 조건**:
- crate마다 릴리스 담당자가 다르거나, "이 crate만 고쳤으니 이 crate만 버전 올리자"는 지역적 판단이 반복될 때.
- 워크스페이스 전체를 한 제품으로 배포하는데도 crate 버전 정책이 없을 때.

**권장**:
```toml
# 루트 Cargo.toml
[workspace.package]
version = "0.9.0"
edition = "2021"

# crates/parser-av1/Cargo.toml
[package]
name = "bitvue-parser-av1"
version.workspace = true
edition.workspace = true
```
- 워크스페이스 내부 crate가 함께 릴리스되는 "한 제품"이라면 `workspace.package.version`으로 lockstep 버전을 강제한다.
- 독립 배포되는 범용 라이브러리(예: 외부에도 공개하는 파서 crate)만 예외적으로 자체 버전을 유지하고, 그 이유를 문서화한다.

**탐지 방법**:
- `cargo metadata --no-deps --format-version 1 | jq '.packages[].version'`로 워크스페이스 내부 버전 분산도 확인.
- 릴리스 스크립트가 여러 버전 문자열을 개별적으로 bump하는지 CI 스크립트 검토.

**예외**:
- 진짜 독립적으로 versioning/배포되어야 하는 범용 유틸 crate(다른 프로젝트에서도 재사용)는 lockstep에서 제외하는 것이 맞다.

**Bitvue 판정**: N/A — 루트 `Cargo.toml`이 `[workspace.package] version = "0.12.0"`을 선언하고, 검사한 19개 crate 전부 `version.workspace = true`로 참조(예: `crates/bitvue-core/Cargo.toml`, `crates/bitvue-decode/Cargo.toml`). lockstep이 이미 강제되고 있음. 예외는 `bitvue-benchmarks`(`publish = false`, 독립 `0.1.0`)와 워크스페이스 밖 `src-tauri`(`0.10.0`, 별도 앱)뿐이며 둘 다 합리적 예외.

---

### BUILD-009: 동일 dependency 다중 버전
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```
$ cargo tree -d
thiserror v1.0.50
thiserror v2.0.3
bitflags v1.3.2
bitflags v2.4.1
```
```toml
# crates/parser-av1/Cargo.toml
thiserror = "1.0"
# crates/parser-hevc/Cargo.toml
thiserror = "2.0"
```

**문제**:
- 같은 dependency의 서로 다른 major 버전이 워크스페이스에 공존하면 컴파일 유닛이 중복되어 빌드 시간과 바이너리 크기가 늘어난다.
- `thiserror`처럼 매크로가 생성하는 타입이 버전마다 달라 crate 경계를 넘나드는 에러 타입 변환(`From` impl)이 미묘하게 깨질 수 있다.
- `cargo tree -d`가 항상 지저분한 상태로 남아 있으면 새로운 중복이 추가돼도 알아채기 어렵다("늘 이랬으니까" 무시).

**발생 조건**:
- crate마다 독립적으로 `cargo add`를 실행해 그 시점의 최신 버전을 받았을 때, 워크스페이스 차원의 버전 정렬이 없을 때(BUILD-010과 직결).
- 외부 의존성(예: `libvmaf-sys`가 예전 `bindgen 0.6x`를 요구, 우리 코드가 `bindgen 0.7x` 사용)까지 겹치면 자체적으로 해결이 어려운 경우도 있다.

**권장**:
```toml
# 루트 Cargo.toml
[workspace.dependencies]
thiserror = "2.0"
bitflags = "2.4"

# crates/parser-av1/Cargo.toml
[dependencies]
thiserror.workspace = true
```
- `[workspace.dependencies]`로 버전을 한 곳에서 고정하고 각 crate는 `.workspace = true`로 참조한다(BUILD-010과 세트).
- 외부 crate가 강제하는 구버전 중복은 `cargo tree -d`에 예외 목록으로 남기고 upstream 이슈를 추적한다.

**탐지 방법**:
- CI에 `cargo tree -d --workspace` 결과가 비어있는지(또는 화이트리스트와 일치하는지) 검사하는 단계 추가.
- `cargo deny check bans`로 중복/차단 dependency를 정책화.

**예외**:
- 서로 다른 major 버전이 실제로 호환 불가능한 두 개의 독립적인 외부 라이브러리(예: `windows` crate의 세대 차이)를 각각 요구하고, 그 비용(중복 컴파일)이 마이그레이션 비용보다 낮다고 명시적으로 판단했다면 임시로 허용.

**Bitvue 판정**: Confirmed — `Cargo.lock` 분석 결과 28개 crate가 중복 major 버전으로 공존: `thiserror` 1.0.69 & 2.0.18, `bitflags` 1.3.2 & 2.10.0, `syn` 1.0.109 & 2.0.114, `rand` 0.8.5 & 0.9.2, `windows-sys` 0.59/0.60/0.61 등. 문서의 나쁜 예(`thiserror`/`bitflags` 이중 버전)와 정확히 일치하는 사례가 실제로 존재.

---

### BUILD-010: workspace dependency 정책 없음
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
# crates/parser-av1/Cargo.toml
serde = { version = "1.0.188", features = ["derive"] }
# crates/parser-hevc/Cargo.toml
serde = { version = "1", features = ["derive", "rc"] }
# crates/core/Cargo.toml
serde = "1.0"
```

**문제**:
- 같은 dependency에 대해 crate마다 버전 범위와 feature 조합이 제각각이면 unification 결과를 예측하기 어렵고, 어떤 crate가 어떤 feature를 필요로 해서 켜졌는지 추적이 안 된다.
- 새 crate를 추가하는 기여자가 "여기서는 어떤 버전을 써야 하는지" 기존 관례를 20개 파일에서 역추적해야 한다.
- 버전 업그레이드 시(예: `serde` 보안 패치) 20개 Cargo.toml을 모두 찾아 개별 수정해야 한다.

**발생 조건**:
- 워크스페이스가 성장하면서 초기에 `[workspace.dependencies]` 없이 crate를 하나씩 추가해왔을 때.
- "각 crate는 독립 배포될 수도 있으니 버전을 자유롭게 두자"는 원칙과 "한 제품으로 통합 배포한다"는 실제 운영이 충돌할 때.

**권장**:
```toml
# 루트 Cargo.toml
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "2.0"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

# crates/parser-av1/Cargo.toml
[dependencies]
serde.workspace = true
thiserror.workspace = true
```
- 모든 공통 외부 dependency를 루트 `[workspace.dependencies]`에 선언하고, 각 crate는 `dep.workspace = true`만 사용한다.
- 새 dependency 추가 PR에 "루트에 없는 외부 crate를 개별 crate Cargo.toml에 직접 버전 명시로 추가했는가"를 체크리스트로 넣는다.

**탐지 방법**:
- lint 스크립트: 각 crate `Cargo.toml`에서 `.workspace = true`가 아닌 버전 문자열이 있는 외부 dependency를 grep해 목록화.
- `cargo-machete` / `cargo-hakari` 같은 도구로 워크스페이스 dependency 위생 점검.

**예외**:
- 특정 crate 하나만 실험적으로 다른 버전을 검증해야 하는 과도기(마이그레이션 중)에는 일시적 예외를 허용하되 TODO/이슈로 추적한다.

**Bitvue 판정**: Confirmed — 루트 `[workspace.dependencies]`가 존재함에도 일부 crate가 이를 우회: `crates/bitvue-decode/Cargo.toml`의 `rayon = "1.10"`(workspace에 이미 선언된 값을 `.workspace = true` 대신 재입력), `crates/bitvue-av1-codec/Cargo.toml`의 `twox-hash = "1"`, 그리고 `crates/bitvue-mcp/Cargo.toml`은 `serde`/`serde_json`/`tokio`/`anyhow`/`tracing`/`tracing-subscriber`를 전부 workspace 참조 없이 직접 버전 명시 — 정책이 선언만 되고 강제되지 않음.

---

### BUILD-011: src-tauri가 workspace 밖이라 lint/test에서 제외
**분류**: BUILD · **심각도**: Critical · **탐지**: Structural|Runtime

**나쁜 예**:
```toml
# 루트 Cargo.toml
[workspace]
members = [
    "crates/parser-av1",
    "crates/parser-hevc",
    "crates/parser-avc",
    "crates/core",
    # ... 20개 crate
    # "src-tauri" 없음 — 별도 Cargo.toml/lock을 가진 독립 워크스페이스
]
```
```yaml
# .github/workflows/ci.yml
- run: cargo test --workspace
- run: cargo clippy --workspace -- -D warnings
# src-tauri는 위 두 명령 어디에도 포함되지 않음
```

**문제**:
- `cargo test --workspace`, `cargo clippy --workspace`는 정의상 `[workspace] members`에 등록된 crate만 대상으로 하므로, `src-tauri`가 별도 워크스페이스(자체 `Cargo.lock`)로 존재하면 이 crate의 테스트와 lint는 조용히 CI에서 완전히 건너뛰어진다.
- 신규 기여자나 CI 작성자는 "`cargo test --workspace`가 초록불이니 전체가 통과했다"고 착각하지만 실제로는 앱 레이어(Tauri command, IPC, state 관리)가 전혀 검증되지 않은 상태다.
- 이 문제는 crate가 하나씩 추가될 때마다 재발한다 — 새 crate에 테스트를 작성해도 그 crate가 CI 매트릭스/워크스페이스 멤버 목록에 등록되지 않으면 동일한 방식으로 조용히 스킵된다.

**발생 조건**:
- Tauri 프로젝트는 `src-tauri`가 프론트엔드 빌드 도구(`tauri.conf.json`, `beforeBuildCommand`)와 얽혀 있어 관례적으로 별도 워크스페이스로 두는 경우가 많다.
- CI 스크립트가 "워크스페이스 crate"와 "src-tauri"를 별개 단계로 취급하면서, src-tauri 쪽 `cargo test`/`cargo clippy` 실행 단계 자체를 잊어버렸을 때.
- 새 crate 추가 시 CI 매트릭스에 명시적으로 등록하지 않으면 `--workspace` 플래그를 쓰고 있어도 매트릭스 밖의 job에서는 여전히 빠질 수 있다.

**권장**:
```toml
# 루트 Cargo.toml — 가능하면 src-tauri를 실제 워크스페이스 멤버로 편입
[workspace]
members = [
    "crates/*",
    "src-tauri",
]
```
```yaml
# 편입이 어렵다면(빌드 도구 제약 등) CI에서 반드시 별도 명시적 단계로 추가
- name: Test workspace crates
  run: cargo test --workspace
- name: Test src-tauri (separate Cargo.lock)
  run: cargo test --manifest-path src-tauri/Cargo.toml
- name: Clippy src-tauri
  run: cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
- 가능하면 `src-tauri`를 워크스페이스 멤버로 편입해 `--workspace` 플래그 하나로 전부 커버되게 한다.
- 구조적 제약으로 분리가 불가피하면, "워크스페이스 밖 crate 목록"을 CI 설정 상단에 명시적으로 나열하고 각각에 대해 별도 test/clippy 단계를 강제한다. 새 crate 추가 시 이 목록 갱신을 PR 체크리스트에 포함한다.
- CI 매트릭스에 "이 워크스페이스/이 crate가 정말 커버되는가"를 검증하는 메타 테스트(예: `cargo metadata`로 멤버 목록을 뽑아 CI YAML의 job 목록과 diff)를 추가하는 것도 유효하다.

**탐지 방법**:
- `cargo metadata --no-deps | jq '.workspace_members'`로 실제 워크스페이스 멤버를 뽑고, CI가 커버하는 crate 목록과 diff.
- 별도 `Cargo.lock`을 가진 디렉터리(`find . -name Cargo.toml -not -path "*/target/*"`)를 모두 나열해 워크스페이스 루트 `members`와 대조.
- **Bitvue 실사례**: 2026-07-31 기준, `parity-regression` CI job이 새로 추가되면서 Phase 5/6에서 랜딩된 crate 중 실제 테스트를 보유한 4개 crate가 테스트 매트릭스에서 누락되어 있던 것이 발견·수정됨. 바로 이 항목(BUILD-011)이 묘사하는 실패 양상 — "워크스페이스/매트릭스 밖이라 조용히 스킵됨" — 이 실제로 발생했던 사례로, 이 클래스의 안티패턴이 이론이 아니라 실전에서 재발함을 보여주는 근거다.

**예외**:
- `src-tauri`가 정말로 얇은 wiring 레이어(Tauri command → 워크스페이스 crate 함수 호출 1줄)뿐이고 자체 로직/테스트가 없다면 커버리지 공백의 실질적 위험은 낮다 — 다만 이 경우에도 "테스트가 없음"이 의도된 것인지 CI 설정만 봐서는 구분이 안 되므로, 최소한 `cargo check --manifest-path src-tauri/Cargo.toml`은 CI에 있어야 한다.

**Bitvue 판정**: Confirmed — 재감사 결과 원래 근거(구 `src-tauri`가 `[workspace] exclude`돼 CI에서 스킵)는 stale함: `src-tauri`는 2026-08-08 Electron 전환으로 저장소에서 완전히 삭제됐고, 현재 `exclude = ["fuzz"]`뿐(`Cargo.toml:3`). 하지만 문서가 경고하는 **바로 그 패턴이 다른 대상으로 재발**해 있음 — `.github/workflows/ci.yml`의 `test` job matrix(`crate:` 목록, ci.yml:132 부근)에는 17개 crate만 나열돼 있는데, 루트 `Cargo.toml`의 `[workspace] members`에는 23개가 등록돼 있음. 매트릭스에서 빠진 6개 중 `bitvue-sidecar`(테스트 보유 12개 파일), `bitvue-protocol`(1개 파일), `bitvue-indexer`(2개 파일)는 실제 테스트 코드를 갖고 있는데도 `cargo test -p <crate>`가 CI에서 한 번도 실행되지 않음(`bitvue`/`bitvue-codecs`/`bitvue-benchmarks`는 테스트 자체가 없어 실질 영향 낮음). 특히 `bitvue-sidecar`는 Electron 앱의 실제 백엔드 프로세스(BUILD-007 참고)라 커버리지 공백의 실질 위험이 큼.

---

### BUILD-012: platform cfg가 코드 전체에 분산
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```rust
// src/hex_view/export.rs
#[cfg(target_os = "windows")]
let path = format!("{}\\export.bin", dir);
#[cfg(not(target_os = "windows"))]
let path = format!("{}/export.bin", dir);

// src/pipeline/decode.rs (다른 파일)
#[cfg(target_os = "macos")]
fn preferred_thread_count() -> usize { num_cpus::get() - 2 }
#[cfg(not(target_os = "macos"))]
fn preferred_thread_count() -> usize { num_cpus::get() }

// src-tauri/src/commands/system.rs (또 다른 파일)
#[cfg(target_os = "windows")]
const MAX_OPEN_FILES: usize = 512;
#[cfg(not(target_os = "windows"))]
const MAX_OPEN_FILES: usize = 4096;
```

**문제**:
- 플랫폼별 분기가 파일 수십 개에 흩어져 있으면 "Windows에서 이 프로젝트가 실제로 어떻게 동작하는지"를 이해하려면 `grep -r 'target_os'` 전체를 훑어야 한다.
- 동일한 플랫폼 조건(`windows` vs `not(windows)`)이 매번 다른 방식으로 표현되면(`target_os = "windows"` vs `target_family = "windows"` vs `windows` cfg alias) 실수로 조건이 미묘하게 어긋나는 코드가 생긴다.
- 새 플랫폼(예: Linux 지원 추가)을 넣으려면 `#[cfg(not(target_os = "windows"))]`처럼 "windows가 아니면 전부 default"로 작성된 분기들이 의도치 않게 새 플랫폼에 잘못된 기본값을 적용한다.

**발생 조건**:
- 플랫폼 차이가 필요할 때마다 그 자리에서 `#[cfg(...)]`를 즉석으로 추가하는 습관이 누적됐을 때.
- 크로스 플랫폼 지원 범위(Windows/macOS/Linux)가 나중에 확장되면서 기존의 이항 분기(`windows` / `not(windows)`)가 세 갈래 이상이 되어야 할 때.

**권장**:
```rust
// src/platform/mod.rs — 플랫폼 차이를 한 곳에 모으는 추상화 계층
pub trait PlatformPaths {
    fn export_dir_separator() -> char;
    fn preferred_thread_count() -> usize;
    fn max_open_files() -> usize;
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

// 나머지 코드는 cfg 없이 trait만 사용
use platform::current::preferred_thread_count;
```
- `#[cfg(target_os = ...)]`는 `src/platform/{windows,macos,linux}.rs` 같은 전용 모듈 경계로만 국한하고, 그 바깥 코드는 플랫폼 무관 trait/함수를 호출한다.
- cfg alias(`#[cfg(windows)]`)와 `target_os`/`target_family`를 프로젝트 전체에서 하나의 관례로 통일한다(예: `.cargo/config.toml`의 `[target.'cfg(...)']` 문서화).

**탐지 방법**:
- `grep -rn 'cfg(target_os\|cfg(windows\|cfg(unix' --include=*.rs`로 분산도를 정량화하고 `src/platform/` 밖에서 나온 결과 수를 추적.
- 코드 리뷰 체크리스트에 "새 `#[cfg(target_os)]`가 `platform` 모듈 밖에 추가됐는가"를 포함.

**예외**:
- 파일 하나에서만 쓰이는 매우 지역적인 플랫폼 차이(예: 특정 OS 전용 API를 감싸는 wrapper 함수 내부)는 추상화 계층까지 만들 필요 없이 그 자리에 둬도 무방하다.

**Bitvue 판정**: Confirmed — `#[cfg(target_os = "macos")]`가 `crates/bitvue-decode/src/strategy/{mod.rs:15, registry.rs:40,133,197, metal.rs:105}`에 흩어져 있고(전체 저장소에서 이 패턴이 등장하는 유일한 프로덕션 코드 지점), `#[cfg(unix)]`/`#[cfg(windows)]`는 테스트 파일(`bitvue-decode/tests/edge_cases_test.rs`, `bitvue-formats/tests/container_edge_cases_test.rs`)에도 흩어져 있음. 저장소 전체에 `platform` 전용 모듈(`grep -r platform`/`find *platform*.rs`)이 존재하지 않아 중앙 추상화 계층이 없음. (구 `src-tauri/src/commands/file.rs`도 같은 패턴이었으나 2026-08-08 Electron 전환으로 파일 자체가 삭제됨.) 규모는 문서가 말하는 "파일 수십 개"보다 작은 5개 파일 수준.

---

### BUILD-013: target-specific dependency가 공통 API를 오염
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.52", features = ["Win32_Storage_FileSystem"] }
```
```rust
// src/io/file_watch.rs — 플랫폼 공통 모듈인데 시그니처에 windows crate 타입 노출
#[cfg(windows)]
pub fn watch_directory(path: &Path) -> windows::core::Result<FileWatcher> { ... }
#[cfg(not(windows))]
pub fn watch_directory(path: &Path) -> std::io::Result<FileWatcher> { ... }
// 호출부가 플랫폼별로 다른 Result 타입을 처리해야 함
```

**문제**:
- 플랫폼 전용 dependency의 타입(`windows::core::Result`, `windows::core::HRESULT` 등)이 공통 모듈의 public 시그니처에 그대로 새어 나오면, 호출하는 쪽 코드까지 `#[cfg(windows)]` 분기를 갖게 되어 오염이 전파된다.
- 크로스 플랫폼 crate를 표방하면서 실제로는 Windows에서 컴파일할 때와 그 외 플랫폼에서 컴파일할 때 public API 자체가 달라지므로, 문서/타입 시그니처가 플랫폼마다 달라 다운스트림이 혼란스럽다.
- non-Windows 환경에서 개발하는 기여자는 이 API를 아예 타입 체크할 수 없어(다른 플랫폼에서만 컴파일되는 코드 경로) 리뷰나 IDE 지원이 반쪽짜리가 된다.

**발생 조건**:
- `target.'cfg(windows)'.dependencies`처럼 플랫폼 조건부 dependency를 추가한 뒤, 그 라이브러리의 타입을 감싸지 않고 바로 공개 함수 시그니처에 노출했을 때.
- "지금 당장은 Windows만 쓰는 기능이니 괜찮다"는 판단이 나중에 크로스 플랫폼 확장 시점에 API 재설계 비용으로 돌아올 때.

**권장**:
```rust
// src/io/file_watch.rs — 공통 에러 타입으로 감싸 플랫폼 dependency를 격리
pub struct FileWatcher { inner: PlatformWatcher }

pub fn watch_directory(path: &Path) -> Result<FileWatcher, WatchError> {
    platform::watch_directory(path).map_err(WatchError::from)
}

// src/io/platform/windows.rs — windows crate 타입은 이 파일 밖으로 안 나감
#[cfg(windows)]
pub(super) fn watch_directory(path: &Path) -> windows::core::Result<PlatformWatcher> { ... }
```
```rust
#[derive(thiserror::Error, Debug)]
pub enum WatchError {
    #[error("failed to watch directory: {0}")]
    Io(#[from] std::io::Error),
}
#[cfg(windows)]
impl From<windows::core::Error> for WatchError { /* 변환 */ }
```
- 플랫폼 전용 crate의 타입은 그 플랫폼 모듈 내부(`pub(super)`/`pub(crate)`)에만 머물게 하고, 공개 API 경계에서 공통 에러/타입으로 변환한다.
- 플랫폼별로 public 시그니처가 달라지지 않도록 컴파일 타임에 강제하려면, 모든 플랫폼 모듈이 동일한 공통 trait을 구현하도록 설계한다.

**탐지 방법**:
- `cargo public-api` 같은 도구로 `--target x86_64-pc-windows-msvc`와 `--target x86_64-unknown-linux-gnu`의 공개 API diff를 비교 — 플랫폼별로 시그니처가 달라지면 오염 신호.
- public 함수 시그니처에 `windows::`, `libc::`, `nix::` 같은 플랫폼 crate 타입이 등장하는지 grep.

**예외**:
- 애초에 플랫폼 전용 기능임을 crate/모듈 이름으로 명시하고(예: `bitvue-windows-integration`), 다운스트림도 그 사실을 알고 조건부로만 사용한다면 오염이 아니라 의도된 설계다.

**Bitvue 판정**: N/A — 전체 Cargo.toml/Cargo.lock을 확인했을 때 `windows`/`libc`/`nix`가 워크스페이스 crate의 **직접** 의존성으로는 어디에도 없음(`target.'cfg(windows)'.dependencies` 검색 결과 0건; `windows-sys`/`libc`는 `tokio`/`tempfile`/`rustix` 등을 통한 transitive 의존일 뿐). 플랫폼 분기가 존재하는 유일한 프로덕션 지점(`bitvue-decode/src/strategy`)도 공통 타입(`String` 에러, trait 기반 `StrategyType`)만 노출하며 플랫폼 전용 타입이 public 시그니처에 새어나오지 않음. (구 `src-tauri/src/commands/file.rs`도 같은 결론이었으나 2026-08-08 Electron 전환으로 파일 자체가 삭제됨.)

---

### BUILD-014: build.rs가 네트워크나 외부 도구에 의존
**분류**: BUILD · **심각도**: High · **탐지**: Runtime|Manual

**나쁜 예**:
```rust
// codec-vvc/build.rs
fn main() {
    // VVC 참조 SDK를 빌드 시점에 다운로드
    let status = std::process::Command::new("curl")
        .args(["-L", "-o", "vvc_sdk.tar.gz", "https://example.com/vvc-sdk-v3.2.tar.gz"])
        .status()
        .expect("curl not found");
    assert!(status.success());
    // 압축 해제 후 컴파일...
}
```

**문제**:
- 오프라인 환경(에어갭 CI, 비행기, 사내 방화벽)에서 빌드 자체가 불가능해진다.
- 원격 서버가 파일을 옮기거나 삭제하면(link rot) 과거에 성공했던 빌드가 어느 날 갑자기 실패하며, 이는 소스 코드 변경과 무관하게 발생해 디버깅이 매우 혼란스럽다.
- 다운로드되는 아카이브에 체크섬 검증이 없으면 공급망 공격(supply chain attack) 벡터가 된다 — MITM이나 서버 침해 시 임의 코드가 빌드에 섞여 들어갈 수 있다.
- CI 캐시가 없는 매 clean build마다 네트워크 왕복 지연이 빌드 시간에 추가된다.

**발생 조건**:
- 라이선스 문제 등으로 SDK를 vendoring(소스 트리에 포함)할 수 없어 "빌드 시점에 받아오자"는 임시방편을 택했을 때.
- `curl`, `wget`, `git clone` 같은 외부 CLI 도구의 존재를 build.rs가 암묵적으로 가정할 때(BUILD-005와 유사한 계열).

**권장**:
```toml
[dependencies]
vvc-sdk-sys = { version = "3.2", features = ["vendored"] }
# vendored 소스 tarball을 crates.io 패키지 자체에 포함(라이선스 허용 시)
# 또는 git submodule로 고정 커밋을 체크아웃해 vendor/ 디렉터리에 둠
```
```rust
fn main() {
    // 네트워크 접근 없이 vendor/ 디렉터리의 로컬 소스만 컴파일
    println!("cargo:rerun-if-changed=vendor/vvc_sdk");
    cc::Build::new().files(glob_c_files("vendor/vvc_sdk")).compile("vvc_sdk");
}
```
- 외부 SDK는 vendoring(소스 포함) 또는 git submodule로 고정 revision을 저장소에 둬 build.rs가 순수 오프라인으로 동작하게 한다.
- 정말 다운로드가 불가피하다면(라이선스상 재배포 금지) 체크섬을 검증하고, 실패 시 명확한 수동 설치 안내를 출력하며, `--offline` 모드에서는 명시적으로 스킵/에러 처리한다.

**탐지 방법**:
- `cargo build --offline`이 성공하는지 CI에서 검증(네트워크 네임스페이스 격리 또는 `--offline` 플래그).
- `build.rs`에서 `Command::new("curl"|"wget"|"git")`, `reqwest`, `ureq` 등 네트워크 관련 호출을 grep.

**예외**:
- 프록시/사내 아티팩트 서버에서 이미 검증된 내부 패키지 레지스트리를 통해 받아오는 경우(일반 `cargo`가 crates.io에서 받아오는 것과 동일한 신뢰 모델)는 예외로 볼 수 있다.

**Bitvue 판정**: N/A — 저장소 전체에 자체 `build.rs`가 하나도 없음(BUILD-006 참조; 유일하게 있었던 `src-tauri/build.rs`도 2026-08-08 Electron 전환으로 삭제됨). `curl`/`wget`/`git clone`/`reqwest` 등 네트워크 호출이 어떤 build.rs에도 없음.

---

### BUILD-015: generated code 변경을 추적하지 않음
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```
# .gitignore
/target
**/bindings_generated.rs   # bindgen 출력을 git에서 완전히 배제
```
```rust
// build.rs
bindings.write_to_file(out_path.join("bindings.rs")).unwrap();
// OUT_DIR 아래에만 생성되고 리뷰 대상이 되는 어떤 파일에도 diff가 남지 않음
```

**문제**:
- bindgen이 생성하는 FFI 시그니처가 C 헤더 업데이트로 미묘하게 바뀌어도(예: `int` → `long`으로 필드 타입 변경) 이 변화가 어떤 PR에도 diff로 나타나지 않는다 — `OUT_DIR`은 `.gitignore`되고 리뷰어 눈에 보이지 않는다.
- ABI가 깨지는 변경이 생겨도 코드 리뷰 시점에는 알 수 없고, 런타임 크래시나 미정의 동작으로만 드러난다.
- "이 바인딩이 마지막으로 어떤 vendored 헤더 버전에서 생성됐는지" 이력을 추적할 수 없어, 회귀 원인 분석 시 헤더 변경과 바인딩 변경을 연결 짓기 어렵다.

**발생 조건**:
- codegen 산출물이 `OUT_DIR`(빌드마다 재생성, git 추적 대상 아님)에만 존재하고 소스 트리에 커밋된 스냅샷이 없을 때.
- protobuf/FlatBuffers 스키마, bindgen 헤더, 상수 테이블 자동 생성기 등 "소스로부터 코드를 만드는" 모든 빌드 단계에 공통되는 문제.

**권장**:
```rust
// build.rs — 생성 결과를 OUT_DIR뿐 아니라 소스 트리에도 스냅샷으로 기록(옵션)
fn main() {
    let generated = bindgen::Builder::default()
        .header("wrapper.h")
        .generate()
        .expect("bindgen failed");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    generated.write_to_file(out_path.join("bindings.rs")).unwrap();

    // CI에서 `cargo xtask regen-bindings`로만 갱신, 평소 빌드는 커밋된 스냅샷 사용
}
```
```
src/sys/bindings_snapshot.rs   # git에 커밋, PR diff로 리뷰 가능
xtask/regen-bindings.rs        # 헤더 변경 시 수동 실행해 스냅샷 갱신
```
- 리뷰가 필요한 codegen 산출물(FFI 바인딩처럼 ABI에 직접 영향을 주는 것)은 소스 트리에 커밋하고, 평소 빌드는 이 스냅샷을 사용하며 "재생성"은 명시적 명령(`xtask regen-*`)으로만 수행한다.
- CI에 "vendored 헤더로 재생성한 바인딩이 커밋된 스냅샷과 동일한가"를 검증하는 drift-check job을 둔다.

**탐지 방법**:
- CI job: `cargo run -p xtask -- regen-bindings && git diff --exit-code`로 drift 탐지.
- `.gitignore`에 `*_generated.rs`, `bindings.rs` 같은 패턴이 있는지, 그리고 그 산출물이 ABI-critical인지 수동 검토.

**예외**:
- 매 빌드마다 결정론적으로 동일하게 재생성되고 ABI에 영향이 없는 순수 내부용 codegen(예: enum-to-string 매핑 테이블)은 커밋 없이 `OUT_DIR`에만 둬도 무방하다.

**Bitvue 판정**: N/A — BUILD-006과 동일 근거: 저장소 자체에 bindgen/codegen을 수행하는 build.rs가 전혀 없으므로(BUILD-006 참조) 추적해야 할 generated code 산출물 자체가 존재하지 않음.

---

### BUILD-016: LTO를 무조건 켜고 빌드 시간 문제 방치
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```toml
[profile.dev]
lto = "fat"       # 개발 중 incremental build마다 fat LTO 수행

[profile.release]
lto = "fat"
codegen-units = 1
```

**문제**:
- `dev` 프로파일에까지 `lto = "fat"`을 걸면 로컬 개발자의 매 `cargo build`/`cargo test` 반복이 링크 시간 폭증으로 고통스러워지고, incremental compilation의 이점이 대부분 상쇄된다.
- CI의 매 PR 빌드에서도 fat LTO + `codegen-units = 1` 조합은 릴리스 빌드 시간을 수 배로 늘려, "테스트만 빠르게 통과 여부를 보고 싶은" PR 피드백 루프를 느리게 만든다.
- LTO를 켜는 실제 이유(런타임 성능)를 정량적으로 측정하지 않은 채 "release니까 당연히 최적화 최대치"라는 관성으로 설정되는 경우가 많다.

**발생 조건**:
- 프로파일별 목적(로컬 iteration용 dev, PR 피드백용 CI release, 배포용 최종 release)을 구분하지 않고 단일 release 프로파일 설정을 모든 상황에 재사용할 때.
- 벤치마크 한 번으로 "LTO 켜니 빨라졌다"를 확인한 뒤, 그 설정을 모든 빌드 경로에 일괄 적용했을 때.

**권장**:
```toml
[profile.dev]
lto = false
opt-level = 0

[profile.release]
lto = "thin"        # fat보다 빠르면서 대부분 이점 확보
codegen-units = 16

[profile.dist]       # 실제 배포 아티팩트만 최고 최적화
inherits = "release"
lto = "fat"
codegen-units = 1
strip = true
```
```
# CI: PR 검증은 release(thin LTO), 태그 릴리스만 dist(fat LTO) 사용
cargo build --profile release   # PR CI
cargo build --profile dist      # 태그 push 시 배포 빌드
```
- 목적별로 프로파일을 분리한다: `dev`(빠른 iteration), `release`(CI 검증용, thin LTO), `dist`(배포용, fat LTO) 3단계.
- LTO 도입 전후로 실제 벤치마크(디코딩 fps, 시작 시간 등)와 빌드 시간을 함께 측정해 트레이드오프를 문서화한다.

**탐지 방법**:
- `cargo build --timings`로 프로파일별 빌드 시간 비교.
- `[profile.dev]`/`[profile.test]`에 `lto` 키가 있는지 Cargo.toml 정적 검사.

**예외**:
- 워크스페이스가 작고(수 crate) 빌드 시간이 애초에 수 초 단위라 LTO 오버헤드가 체감되지 않는다면, 프로파일을 굳이 세분화하지 않아도 실용적 문제는 없다.

**Bitvue 판정**: N/A — 루트 `Cargo.toml`의 `[profile.release]`는 `lto = "thin"`(fat 아님) + `codegen-units = 1`만 설정하고, `[profile.dev]`는 별도 LTO 오버라이드가 전혀 없어 Cargo 기본값(lto=false)을 그대로 씀 — 문서가 권장하는 패턴과 이미 일치.

---

### BUILD-017: panic abort가 모든 바이너리에 강제
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural|Runtime

**나쁜 예**:
```toml
# 루트 Cargo.toml
[profile.release]
panic = "abort"
[profile.dev]
panic = "abort"
```

**문제**:
- `panic = "abort"`는 프로세스 단위 설정이라 워크스페이스 전체(라이브러리 crate 포함)에 강제되면, 이 crate들을 `cdylib`/`dylib`로 다른 언어(예: Tauri의 Node/WebView 브리지, 향후 Python 바인딩)에 노출할 때 unwind 기반 FFI 경계 처리가 불가능해진다.
- `cargo test`는 각 테스트를 별도 스레드에서 돌리고 `catch_unwind`로 개별 테스트 실패를 격리하는데, `panic = "abort"`가 걸리면 테스트 하나의 panic이 테스트 바이너리 전체를 즉시 종료시켜 나머지 테스트 결과를 전혀 알 수 없게 된다("테스트 17개 중 몇 개가 실패했는지" 대신 "바이너리가 죽었다"만 남는다).
- `proc-macro` crate나 `build.rs`처럼 워크스페이스 프로파일을 공유하는 다른 컴파일 유닛에도 의도치 않게 영향을 줄 수 있다.

**발생 조건**:
- 바이너리 크기 절감이나 unwind 테이블 제거로 인한 약간의 성능 이득을 위해 루트 프로파일에 일괄 적용했을 때.
- "release는 abort, dev는 unwind"까지는 흔한 관례지만, 실수로 `dev`/`test` 프로파일에까지 abort를 걸었을 때 위 테스트 격리 문제가 발생한다.

**권장**:
```toml
# 루트 Cargo.toml — 라이브러리는 unwind 유지, 최종 바이너리만 abort
[profile.dev]
panic = "unwind"     # 테스트 격리를 위해 항상 unwind

[profile.release]
panic = "unwind"      # 워크스페이스 기본은 unwind

# src-tauri/Cargo.toml — 최종 앱 바이너리에서만 abort 오버라이드 가능
[profile.release]
panic = "abort"
```
- `panic = "abort"`는 순수 최종 실행 바이너리(라이브러리로 재사용되지 않는 `[[bin]]` crate) 프로파일에만 적용하고, 워크스페이스 공유 라이브러리/테스트 대상에는 적용하지 않는다.
- FFI 경계(cdylib)를 가진 crate는 `panic = "unwind"`를 유지하고, 경계에서 `catch_unwind`로 명시적으로 panic을 흡수해 다른 언어 런타임으로 전파되지 않게 한다.

**탐지 방법**:
- `cargo test --workspace` 실행 시 테스트 하나 실패로 전체 바이너리가 죽는지(예상보다 적은 테스트 결과 출력) 확인.
- Cargo.toml에서 `panic = "abort"`가 걸린 프로파일과, 그 프로파일로 빌드되는 crate 중 `crate-type = ["cdylib", "lib"]`가 있는지 대조.

**예외**:
- 워크스페이스에 라이브러리 재사용/FFI 경계가 전혀 없고 오직 단일 최종 바이너리만 만드는 소규모 프로젝트라면 전역 `panic = "abort"`도 실질적 위험이 낮다.

**Bitvue 판정**: Confirmed(잠재적) — 루트 `Cargo.toml:131`의 `[profile.release] panic = "abort"`가 워크스페이스 전체(모든 라이브러리 crate 포함)에 적용됨 — 나쁜 예와 구조적으로 동일. 다만 현재는 실질 피해가 latent 상태: 워크스페이스 멤버 중 `cdylib`/`dylib` crate-type을 가진 crate가 없고(`crate-type` grep 결과 0건 — 구 `src-tauri`가 유일한 `cdylib`였으나 2026-08-08 Electron 전환으로 저장소에서 삭제됨), CI도 `cargo test --release`를 쓰지 않아(`ci.yml` test job은 release 플래그 없음) 테스트 격리 문제도 아직 발현되지 않음. 하지만 향후 워크스페이스 crate가 cdylib를 노출하거나 CI가 release 테스트를 추가하는 순간 문서가 설명하는 문제가 그대로 재현될 구조.

---

### BUILD-018: benchmark binary가 production feature와 다름
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural|Manual

**나쁜 예**:
```toml
[[bench]]
name = "decode_av1"
harness = false

[dev-dependencies]
criterion = "0.5"
# bench 대상 crate의 feature 기본값 그대로 사용
# default = [] 라서 SIMD, hw-accel 등 production에서 켜지는 feature가 꺼진 채로 벤치 실행
```
```bash
cargo bench  # default-features만으로 컴파일 → 실제 배포 바이너리와 다른 코드 경로 측정
```

**문제**:
- 프로덕션 바이너리는 `--release --features simd,hw-accel`로 빌드되는데 `cargo bench`는 별다른 feature 지정 없이 crate의 `default`만으로 컴파일되면, 벤치마크가 측정하는 코드 경로(스칼라 fallback)와 실제 사용자가 겪는 코드 경로(SIMD/HW 가속)가 완전히 다르다.
- "최적화했더니 20% 빨라졌다"는 벤치 결과가 실제로는 프로덕션에 전혀 반영되지 않는 코드 경로의 개선일 수 있어, 잘못된 결론으로 리소스를 낭비하게 된다.
- 회귀 감지용 벤치마크라면 더 위험하다 — 실제 배포 경로(SIMD)에 성능 회귀가 생겨도 벤치는 그 경로를 아예 컴파일하지 않으므로 회귀를 놓친다.

**발생 조건**:
- `cargo bench`를 feature 플래그 없이 습관적으로 실행할 때(특히 로컬 개발 중 빠른 확인 목적).
- CI 벤치마크 job이 `cargo build --release`(프로덕션 빌드)와 `cargo bench`(벤치 빌드)에 서로 다른 feature 셋을 사용하면서 이를 문서화하지 않았을 때.

**권장**:
```toml
[[bench]]
name = "decode_av1"
harness = false
required-features = ["simd", "hw-accel"]
```
```yaml
# CI — 벤치마크도 프로덕션과 동일한 feature 셋으로 명시 실행
- run: cargo bench --features simd,hw-accel --bench decode_av1
```
```
# xtask/bench.rs — 프로덕션 빌드 feature 목록을 단일 소스로 관리해 bench/build가 공유
const PRODUCTION_FEATURES: &[&str] = &["simd", "hw-accel"];
```
- 벤치마크 대상 feature 조합을 프로덕션 배포 빌드와 동일하게 명시하고, 가능하면 `required-features`로 컴파일 타임에 강제한다.
- "프로덕션 feature 목록"을 CI 스크립트/`xtask` 한 곳에서 정의해 빌드 단계와 벤치 단계가 동일한 상수를 참조하게 한다.

**탐지 방법**:
- `cargo bench` 실행 로그와 프로덕션 `cargo build --release` 로그에서 활성화된 feature 목록을 diff.
- 벤치마크 결과에 `SIMD` 관련 카운터(예: 명령어 통계, cpuid 체크)가 실제로 등장하는지 프로파일러로 확인.

**예외**:
- 의도적으로 "SIMD 없는 스칼라 경로만 따로 추적하고 싶다"는 별도 벤치 스위트(fallback 회귀 감시용)라면 이름과 목적을 명확히 구분해 `bench_scalar_fallback` 같은 이름으로 분리한다 — 이 경우는 의도된 것이므로 문제가 아니다.

**Bitvue 판정**: Suspected — `crates/bitvue-benchmarks/Cargo.toml`은 `bitvue-metrics`에 `parallel` feature를 요청하지 않는 반면, 프로덕션 빌드인 `src-tauri/Cargo.toml`은 `bitvue-metrics = { features = ["parallel"] }`로 명시 활성화. 실제로 `bitvue-metrics`를 쓰는 유일한 벤치(`frame_parsing.rs`)는 `parallel` 게이트가 걸린 `batch_psnr_parallel`이 아닌 비-병렬 `psnr` 함수만 호출 — 병렬 경로 회귀는 벤치에 안 잡힘. Confirmed로 올리지 않은 이유: CI 어디에도 `cargo bench` 실행 step이 없어(전체 워크플로 grep 결과 0건) 이 불일치가 실제로 "그릇된 확신"을 만들어내는 살아있는 파이프라인 자체가 없음.

---

### BUILD-019: optional codec을 제거해도 binary size가 줄지 않음
**분류**: BUILD · **심각도**: Medium · **탐지**: Structural|Runtime

**나쁜 예**:
```rust
// codec-registry/src/lib.rs
pub fn all_decoders() -> Vec<Box<dyn Decoder>> {
    vec![
        Box::new(parser_av1::Av1Decoder::default()),
        Box::new(parser_hevc::HevcDecoder::default()),
        Box::new(parser_avc::AvcDecoder::default()),
        Box::new(parser_vp9::Vp9Decoder::default()),
    ]
    // parser-hevc를 feature로 뺐다고 믿었지만, 이 함수가
    // codec-registry crate에 하드코딩되어 있어 항상 4개 dependency 전부 링크됨
}
```
```toml
[dependencies]
parser-av1 = { workspace = true }
parser-hevc = { workspace = true }   # optional 아님 — feature gate가 실제로는 없음
parser-avc = { workspace = true }
parser-vp9 = { workspace = true }
```

**문제**:
- feature flag(`hevc = []`)는 정의돼 있지만 실제 dependency가 `optional = true`로 선언되지 않았거나, registry 함수가 feature와 무관하게 모든 디코더를 하드코딩 등록하면 "feature를 꺼도 아무것도 빠지지 않는다."
- 사용자는 `--no-default-features --features av1`로 빌드했다고 믿지만 바이너리 크기와 링크되는 native dependency는 전혀 줄지 않아, feature가 사실상 장식(cosmetic)에 불과해진다.
- BUILD-003(codec dependency가 optional 아님)과 원인은 겹치지만, 이 항목은 특히 "trait object registry 패턴이 feature gate를 우회한다"는 더 미묘하고 흔한 실패 지점을 짚는다 — dependency는 optional로 잘 선언했는데 등록 코드만 놓치는 경우가 많다.

**발생 조건**:
- 플러그인 레지스트리/factory 패턴(`all_decoders()`, `register_all()`)이 feature 밖에서 전체 목록을 하드코딩할 때.
- feature gate 작업을 "Cargo.toml에 `optional = true` 추가"까지만 하고, 그 dependency를 실제로 사용하는 소스 코드의 `#[cfg(feature = ...)]` 누락을 놓쳤을 때.

**권장**:
```toml
[dependencies]
parser-av1 = { workspace = true, optional = true }
parser-hevc = { workspace = true, optional = true }

[features]
av1 = ["dep:parser-av1"]
hevc = ["dep:parser-hevc"]
```
```rust
pub fn all_decoders() -> Vec<Box<dyn Decoder>> {
    let mut v: Vec<Box<dyn Decoder>> = Vec::new();
    #[cfg(feature = "av1")]
    v.push(Box::new(parser_av1::Av1Decoder::default()));
    #[cfg(feature = "hevc")]
    v.push(Box::new(parser_hevc::HevcDecoder::default()));
    v
}
```
- registry/factory 함수의 각 항목을 대응하는 feature의 `#[cfg(feature = "...")]`로 명시적으로 감싼다.
- feature gate PR의 리뷰 체크리스트에 "Cargo.toml의 optional 선언"과 "소스 코드의 #[cfg(feature)] 사용"을 항상 쌍으로 확인하도록 명시한다.

**탐지 방법**:
- `cargo build --release --no-default-features --features av1`와 `--features av1,hevc,avc,vp9`로 각각 빌드해 바이너리 크기(`ls -la target/release/binary`)를 비교 — 유의미한 차이가 없으면 신호.
- `cargo bloat --features av1 --no-default-features`로 실제 링크된 심볼에 `hevc`/`avc` 관련 함수가 남아있는지 확인.

**예외**:
- 코드 크기가 애초에 목표가 아니고(예: 데스크톱 앱, 디스크 여유 충분) 오직 "컴파일 시간 단축"만이 feature gate의 목적이라면, 바이너리 크기 불변 자체는 실패 기준이 아닐 수 있다 — 다만 이 경우 목적을 문서에 명시해야 혼란이 없다.

**Bitvue 판정**: N/A — 저장소에서 발견된 유일한 codec 레지스트리 패턴인 `DecoderFactory`(`crates/bitvue-decode/src/traits.rs`)는 각 codec 등록/매치 분기마다 정확히 대응하는 `#[cfg(feature = "ffmpeg")]`/`#[cfg(feature = "vvdec")]`로 감싸져 있어(traits.rs:216-386) 권장 패턴과 일치, 장식용 feature가 아님. `bitvue-codecs`가 7개 codec parser crate를 feature 없이 통째로 재노출하는 것은 별개 사안(BUILD-002/003 영역)이며, 애초에 feature gate를 시도조차 안 한 것이라 "약속을 어긴" 이 항목의 패턴과는 다름.

---

### BUILD-020: MSRV를 명시했지만 CI에서 검증하지 않음
**분류**: BUILD · **심각도**: Low · **탐지**: Structural|Manual

**나쁜 예**:
```toml
# 루트 Cargo.toml
[workspace.package]
rust-version = "1.74"
```
```yaml
# .github/workflows/ci.yml
strategy:
  matrix:
    rust: [stable, nightly]   # 1.74가 목록에 없음 — MSRV는 문서 값일 뿐
```

**문제**:
- `rust-version = "1.74"`는 사용자와 dependency resolver(최신 Cargo는 MSRV-aware resolution 지원)에게 하는 약속인데, CI가 이를 실제로 검증하지 않으면 그 약속이 조용히 깨질 수 있다.
- 기여자가 stable 최신 버전에서만 개발하면서 최신 문법(예: `let-else`, 최근 안정화된 API)을 무심코 사용하면, MSRV 1.74 사용자의 빌드는 실패하는데 이는 릴리스 후에야 이슈 트래커를 통해 발견된다.
- "MSRV를 명시했다"는 사실 자체가 실제로는 검증되지 않는 거짓 안전감을 준다 — 명시가 없는 것보다 잘못 관리된 명시가 더 나쁠 수 있다(사용자가 그 값을 믿고 고정 버전 환경에 배포했다가 실패).

**발생 조건**:
- MSRV를 한 번 정하고 CI 매트릭스를 그 이후로 갱신하지 않았을 때(예: `rust-version`은 1.74인데 CI는 `stable`만 사용해 1.74 릴리스 이후 몇 달치 최신 문법이 이미 스며들었을 수 있음).
- MSRV 검증에 필요한 toolchain(`rustup toolchain install 1.74`)을 CI에서 설치하는 단계가 번거로워 생략됐을 때.

**권장**:
```yaml
strategy:
  matrix:
    rust: [stable, nightly, "1.74"]   # rust-version 값과 동기화
steps:
  - uses: dtolnay/rust-toolchain@master
    with:
      toolchain: ${{ matrix.rust }}
  - run: cargo +${{ matrix.rust }} check --workspace --all-features
```
```toml
# 추가로 cargo-msrv를 CI에 넣어 실제 최소값을 주기적으로 검증/제안
```
- CI 매트릭스에 `rust-version`과 정확히 동일한 버전을 별도 job으로 추가하고 `cargo check --workspace`(최소한 컴파일)를 그 버전으로 실행한다.
- `cargo-msrv` 같은 도구를 주기적(예: 월 1회 scheduled workflow)으로 돌려 선언된 MSRV가 여전히 정확한지 검증하거나 bump를 제안받는다.
- `rust-version`을 올릴 때는 CHANGELOG에 명시해 다운스트림이 대비할 수 있게 한다.

**탐지 방법**:
- CI YAML의 matrix 값과 `Cargo.toml`의 `rust-version` 문자열을 스크립트로 diff해 불일치 탐지.
- 로컬에서 `rustup toolchain install $(grep rust-version Cargo.toml | ...) && cargo +<version> check --workspace`를 실행해 실제 통과 여부 확인.

**예외**:
- 아직 MSRV 정책 자체를 공식화하지 않은 초기 프로젝트(0.x, 내부 전용)라면 `rust-version` 필드를 아예 생략하는 편이 "검증 안 되는 거짓 약속"보다 낫다 — 이 경우 이 항목은 해당하지 않는다.

**Bitvue 판정**: N/A — 재감사 결과 원래 근거(`src-tauri/Cargo.toml:9`의 `rust-version = "1.77.2"`)가 stale함: `src-tauri`는 2026-08-08 Electron 전환으로 저장소에서 완전히 삭제됨. `rust-version`/`workspace.rust-version` 선언은 저장소 전체(`grep -rn rust-version --include=Cargo.toml .`)에 현재 0건 — 루트 `[workspace.package]`에도 어떤 개별 crate에도 MSRV 선언이 없음. 위 예외 조항("MSRV 정책을 아예 공식화하지 않은 프로젝트는 `rust-version` 생략이 낫다")에 정확히 해당해 이 항목 자체가 적용되지 않음.
