# Anti-Pattern Catalog — SUPPLY: Software Supply-Chain & Licensing Risk

이 문서는 더 큰 안티패턴 카탈로그의 한 카테고리입니다 (전체 인덱스는 `docs/anti-patterns/INDEX.md` 참고). Wave 1-4(970개 항목, 47개 파일) 완료 후 Phase 2 감사 과정에서 발견된 공백을 메우는 **Wave 5** 추가 항목이며, 지금까지의 카탈로그가 다루지 않은 새 도메인입니다: Bitvue는 `Cargo.toml`의 `license = "AGPL-3.0-or-later"`와 저장소 루트의 `LICENSE-COMMERCIAL`(별도 유료 상용 라이선스 계약서)을 동시에 갖춘 "오픈코어(open-core)" 이중 라이선스 프로젝트인데, 이 사업 모델 자체가 지고 있는 법적·비즈니스 리스크는 Wave 1-4 어디에도 카테고리로 존재하지 않았다. `SEC.md`가 "신뢰할 수 없는 영상 파일을 여는 프로그램"의 위협 모델을 다루고 `FFI.md`가 dav1d/libvmaf FFI 경계의 메모리 안전성(UB/크래시)을 다루는 것과 달리, 이 문서는 그 경계 반대편에 있는 코드/라이브러리가 **어떤 라이선스로 배포되는지, 그 라이선스가 AGPL과 상용 라이선스를 동시에 파는 사업 모델과 실제로 양립하는지, 그리고 그 사실을 확인/기록/공지하는 자동화가 존재하는지**를 다룬다. 코드 정확성 문제가 아니라 "빌드는 성공하고 기능은 동작하지만, 그 결과물을 판매하거나 배포할 법적 근거가 조용히 무너져 있을 수 있다"는 별개 축의 리스크다. 이 카탈로그를 작성하며 저장소를 직접 확인한 결과, 일부 항목은 이미 상당히 잘 방어되어 있었다(`deny.toml` + `.github/workflows/license-check.yml`의 cargo-deny 기반 라이선스 게이트가 실제로 존재), 반면 컨트리뷰터 라이선스 동의, 서드파티 고지, 릴리스 바이너리 서명 등은 확인 결과 공백이 그대로 남아 있었다 — 항목별로 실제 확인한 근거를 그대로 남긴다.

---

### SUPPLY-001: 이중 라이선스 모델에 대한 문서화된 의존성-라이선스 정책 부재
**분류**: SUPPLY · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
# deny.toml — 어떤 라이선스가 왜 허용되는지 이유가 기록되어 있지 않음
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "AGPL-3.0-or-later", "..."]
```
```markdown
<!-- CONTRIBUTING.md — 의존성 추가 시 라이선스를 어떻게 검토해야 하는지 아무 언급이 없음 -->
## Coding Standards
- Follow Rust API Guidelines
- Use `cargo fmt`
```

**문제**:
- `deny.toml`의 allow-list는 "OSI 승인 라이선스 중 일반적으로 안전한 것들"의 범용 목록이지, "이 라이선스가 **AGPL 오픈소스판과 유료 상용판을 동시에 판매하는 이 프로젝트의 사업 모델**과 왜 양립하는가"라는 질문에 답하도록 설계된 목록이 아니다 — 예를 들어 LGPL 계열(동적 링크는 허용, 정적 링크는 재배포 시 소스 제공 의무 발생)처럼 "허용은 하되 정적 링크 방식에 조건이 붙는" 라이선스가 섞여 들어와도 이 목록만으로는 그 조건을 아무도 상기하지 못한다.
- 새 의존성을 추가하는 개발자 입장에서 "cargo-deny가 통과했으니 안전하다"는 결론은 "OSI 승인 라이선스다"까지만 보장하지, "이 코드를 상용 라이선스 고객에게도 재배포할 권리가 있다"는 것까지는 보장하지 않는다 — 이 둘의 차이를 설명하는 문서가 어디에도 없다.
- `CONTRIBUTING.md`(전체 236줄)에는 코딩 스타일, 테스트, PR 절차는 상세히 있지만 "의존성을 추가할 때 라이선스를 어떻게 확인하는지" 절이 전혀 없다 — 신규 기여자가 라이선스 위험을 판단할 기준 자체가 없다.

**발생 조건**:
- 오픈코어(AGPL + 상용 라이선스) 사업 모델을 채택한 프로젝트에서 의존성 추가/검토 절차를 일반적인 "라이선스 화이트리스트 통과 여부"로만 관리할 때.
- 여러 기여자가 각자의 판단으로 의존성을 추가하고, 그 판단 기준이 문서화되지 않아 리뷰어마다 다르게 적용될 때.

**권장**:
```markdown
## Dependency Licensing Policy (CONTRIBUTING.md에 추가)

Bitvue is dual-licensed (AGPL-3.0-or-later / commercial, see LICENSE-COMMERCIAL).
Any new dependency MUST be compatible with **both** tracks:
1. OSI-approved permissive (MIT/Apache-2.0/BSD-*) or a copyleft license we already
   ship under (AGPL-3.0) — see `deny.toml` for the enforced allow-list.
2. Must NOT require us to disclose Bitvue's own proprietary/commercial-tier
   modifications as a condition of use (rules out GPL-2.0/GPL-3.0-only, some
   source-available "BUSL"/"non-compete" licenses).
3. Native (non-Cargo) libraries linked via FFI (see SUPPLY-003) need the same
   review even though cargo-deny cannot see them automatically.
```
- `deny.toml`의 allow-list 각 항목 옆에 "왜 허용되는지" 한 줄 주석을 남긴다(이미 일부 clarify 항목은 이렇게 하고 있음 — `ring`, `epaint_default_fonts`).
- `CONTRIBUTING.md`에 위와 같은 명시적 "Dependency Licensing Policy" 절을 추가해 PR 리뷰 체크리스트에 편입한다.
- 신규 FFI/네이티브 의존성은 Cargo 의존성 추가와 별개로 라이선스 검토 단계를 필수로 거치게 한다(SUPPLY-003 참고).

**탐지 방법**:
- Structural: `CONTRIBUTING.md`, `deny.toml`, `docs/` 전체에서 "license policy"/"dependency licensing"/"라이선스 정책" 관련 절이 존재하는지 확인.
- Manual: 최근 병합된 PR 중 새 의존성을 추가한 것들을 골라, 라이선스 검토가 실제로 이루어졌다는 흔적(PR 코멘트, 체크리스트 항목)이 있는지 확인.

**예외**:
- 순수 오픈소스(단일 라이선스, 상용 판매 트랙이 없는) 프로젝트라면 이런 이중 검증 정책 자체가 불필요하다. Bitvue는 `LICENSE-COMMERCIAL`이 존재하는 순간부터 이 예외에 해당하지 않는다.

**Bitvue 판정**: Confirmed — `deny.toml`(루트)과 `CONTRIBUTING.md`를 전체 확인한 결과 이중 라이선스 사업 모델을 명시적으로 겨냥한 의존성 정책 문서는 어디에도 없다. `deny.toml`의 `[licenses] allow` 목록(28-46번째 줄)은 OSI 라이선스 화이트리스트이며 상용 재배포 관점의 주석은 없고, `CONTRIBUTING.md`의 라이선스 관련 언급은 236번째 줄 "By contributing, you agree that your contributions will be licensed under the AGPL-3.0" 한 줄이 전부(의존성 추가 시 검토 절차 언급 0건).

---

### SUPPLY-002: dav1d FFI 의존성 — 라이선스 호환 확인된 사례
**분류**: SUPPLY · **심각도**: Low · **탐지**: Structural

**나쁜 예**: (해당 없음 — 이 항목은 "이미 잘 처리된 사례"를 카탈로그에 기록해 대조군으로 남기기 위한 것)

**문제**:
- 이런 대조 항목이 없으면 카탈로그 전체가 "무언가 잘못되었을 가능성"만 나열하게 되어, 실제로 안전한 패턴이 무엇인지 판단할 기준점이 사라진다. dav1d는 Bitvue의 핵심 AV1 디코딩 경로(`crates/bitvue-decode`, 워크스페이스 의존성 `dav1d = "0.10"`)이므로 그 라이선스 상태를 정확히 기록해둘 가치가 있다.

**발생 조건**: (해당 없음)

**권장**:
- dav1d(VideoLAN)는 BSD-2-Clause 라이선스로 배포되는 순수 오픈소스 AV1 디코더이며, Rust 바인딩 `dav1d`/`dav1d-sys` 크레이트(Cargo.lock 확인: `dav1d 0.10.4`, `dav1d-sys 0.8.3`, 둘 다 crates.io 레지스트리)도 이를 그대로 반영한다. BSD-2-Clause는 `deny.toml`의 allow-list(30번째 줄)에 이미 포함되어 있어 cargo-deny가 정상적으로 통과시킨다.
- AV1 자체는 Alliance for Open Media(AOM) 회원사들의 로열티-프리 특허 라이선스 약정으로 보호되는 코덱이라, VVC(SUPPLY-004)와 달리 특허 라이선싱 이슈가 낮다 — 다만 이는 저작권 라이선스와 별개의 특허 라이선스 층위이므로 이 카탈로그의 확인 범위를 넘어서는 부분은 별도 법률 검토가 필요하다는 점은 동일하게 남는다.

**탐지 방법**:
- Static: `cargo deny check licenses`가 `dav1d`/`dav1d-sys` 노드에 대해 통과하는지 CI 로그로 확인(이미 `.github/workflows/license-check.yml`이 매 PR/weekly로 수행).

**예외**: (해당 없음)

**Bitvue 판정**: N/A — 라이선스 호환성 확인됨. dav1d/dav1d-sys는 BSD-2-Clause이고 `deny.toml`의 allow-list에 이미 포함되어 있어 별도 조치가 필요 없다. (이 크레이트의 FFI *메모리 안전성* 측면은 `docs/anti-patterns/FFI.md`가 별도로 다룬다 — 이 항목은 라이선스 층위만 다룬다.)

---

### SUPPLY-003: vvdec 네이티브 라이브러리가 Cargo 의존성 그래프 완전히 밖에 있어 라이선스 감사 도구의 사각지대
**분류**: SUPPLY · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// crates/bitvue-decode/src/vvdec.rs
//! The vvdec library must be installed on the system:
//! - macOS: `brew install vvdec`
//! - Linux: Build from source at https://github.com/fraunhoferhhi/vvdec
//! - Windows: Download prebuilt binaries from vvdec releases

mod ffi {
    // 손으로 작성한 extern "C" 바인딩 — Cargo 의존성이 아니라 시스템에
    // 이미 설치된 공유 라이브러리를 링크 타임/런타임에 찾아 연결한다.
    pub type VvdecDecoder = c_void;
    // ...
}
```
```toml
# crates/bitvue-decode/Cargo.toml
[features]
vvdec = []   # <- 이 feature에 대응하는 [dependencies] 항목이 아예 없음

[target.'cfg(target_os = "macos")'.dependencies]
# (비어 있음)
```

**문제**:
- `cargo-deny`, `cargo-license`, SBOM 생성기 등 모든 Rust 생태계의 라이선스 감사 도구는 `Cargo.lock`/`cargo metadata`가 알고 있는 크레이트 그래프만 스캔한다. vvdec는 그 그래프에 노드로 존재하지 않는다 — Cargo 의존성이 아니라 사용자가 `brew`/소스빌드/프리빌트 바이너리로 **직접 설치한 시스템 공유 라이브러리**를 손으로 작성한 `extern "C"` 바인딩(`mod ffi`)으로 링크할 뿐이기 때문이다.
- 즉 `cargo deny check licenses`가 매 PR/weekly로 통과하더라도, 그 결과는 "Rust crates.io 의존성들의 라이선스가 문제없다"만 보장하며 vvdec의 라이선스 상태에 대해서는 아무것도 말해주지 않는다 — CI가 초록불이어도 이 특정 의존성의 라이선스 리스크는 전혀 검사되지 않은 상태로 남는다.
- 이런 "빌드 시스템에 조용히 묻어가는 네이티브 라이브러리"는 Rust 생태계 전반에서 흔한 맹점이다 — `pkg-config`나 수동 링크로 붙는 C/C++ 라이브러리는 대부분 이 사각지대에 들어간다.

**발생 조건**:
- FFI 바인딩을 Rust 크레이트가 아니라 시스템 설치 라이브러리에 대해 손으로 작성하고, `[dependencies]`에 대응 항목을 두지 않는 모든 코덱/네이티브 라이브러리 통합 지점.

**권장**:
```toml
# 최소한의 완화: 실제 -sys 크레이트가 없다면, 라이선스 검토 결과를
# deny.toml 옆에 별도 파일로 명시적으로 기록해 도구의 사각지대를 문서로 메운다.
```
```markdown
<!-- docs/THIRD_PARTY_NATIVE_LIBS.md (신규) -->
| Library | Linked from | Upstream License | Reviewed | Notes |
|---|---|---|---|---|
| vvdec  | crates/bitvue-decode/src/vvdec.rs (system-installed) | (검토 필요) | 미정 | Cargo 그래프 밖 — cargo-deny 스캔 안 됨 |
```
- 네이티브 라이브러리마다 "어디서 링크되는지, 업스트림 라이선스가 무엇인지, 검토 여부"를 기록하는 별도 문서/스크립트를 만들어 cargo-deny의 사각지대를 명시적으로 보완한다.
- 가능하다면 `-sys` 크레이트(vendored 또는 wrapper)로 감싸 최소한 Cargo 그래프에 노드로라도 등장하게 만든다 — 그러면 `deny.toml`의 `[[licenses.clarify]]` 메커니즘으로 라이선스를 수동 기록할 수 있다.
- CI에 "네이티브 라이브러리 목록과 위 문서의 목록이 일치하는지" 대조하는 간단한 스크립트를 추가해 새 네이티브 의존성이 조용히 추가되는 것을 막는다.

**탐지 방법**:
- Static: `extern "C"`, `mod ffi`, `#[link(name = ...)]` 패턴을 grep해 대응하는 Cargo `[dependencies]`/`-sys` 크레이트가 없는 것들을 찾는다.
- Structural: `cargo metadata`가 보고하는 전체 패키지 목록과, 저장소 소스에서 실제로 FFI 링크되는 라이브러리 목록을 수동으로 대조.

**예외**:
- 링크 대상이 OS 표준 라이브러리(libc, libm 등 라이선스 리스크가 사실상 없는 것)인 경우는 이 항목의 대상이 아니다.

**Bitvue 판정**: Confirmed — `crates/bitvue-decode/src/vvdec.rs`(1-80줄 확인)는 vvdec를 시스템 설치 라이브러리로 문서화하고(6-11줄) 손으로 작성한 `mod ffi` 바인딩으로 링크하며, `crates/bitvue-decode/Cargo.toml`의 `vvdec = []` feature(25번째 줄)에는 대응하는 `[dependencies]` 항목이 전혀 없다(27-32번째 줄의 플랫폼별 `[target...]` 섹션도 비어 있음). `Cargo.lock`/`deny.toml` 전체에서 "vvdec" 문자열은 이 두 위치(주석/문서) 외에 크레이트 노드로 등장하지 않아, cargo-deny 기반 라이선스 게이트(`.github/workflows/license-check.yml`)가 이 의존성을 원천적으로 스캔할 수 없다.

---

### SUPPLY-004: vvdec 라이선스 텍스트/버전이 저장소 어디에도 기록되어 있지 않음 (저작권 라이선스 vs VVC 특허 이슈 혼동 위험)
**분류**: SUPPLY · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
//! This module provides VVC decoding capabilities using the Fraunhofer vvdec library.
//! vvdec is an open-source VVC decoder optimized for performance.
// <- "open-source"라고만 적혀 있고 실제 라이선스 종류, 특허 관련 고지가 전혀 없음
```

**문제**:
- Fraunhofer HHI가 배포하는 vvdec/vvenc 계열 소프트웨어는 역사적으로 순수 3-clause BSD가 아니라 특허권 부여를 명시적으로 배제하는 조항이 포함된 라이선스 변형(업계에서 "Clear BSD" 계열로 불리는 유형)을 사용해온 것으로 알려져 있다 — 즉 "소스코드 저작권은 관대하게 쓰라고 허락하지만, 그렇다고 VVC 표준을 구현/판매/배포할 특허 실시권까지 주는 것은 아니다"라는 구조다. 이 문서 자체는 저장소 안에서 vvdec의 정확한 라이선스 텍스트나 버전을 확인할 수 있는 어떤 근거도 찾지 못했다(파일 없음) — 위 특징은 업스트림 프로젝트에 대한 일반적으로 알려진 사실이며 Bitvue 저장소가 실제로 어느 버전을 링크하는지, 그 버전의 정확한 라이선스 조항이 무엇인지는 이번 확인에서 검증되지 않았다.
- VVC(H.266)는 여러 특허 풀(예: Access Advance의 MC-VVC 풀 등 업계에 공개적으로 알려진 라이선싱 프로그램)이 존재하는 코덱이다. vvdec 소프트웨어 자체의 저작권 라이선스가 관대하더라도, Bitvue가 `LICENSE-COMMERCIAL`로 VVC 디코딩 기능을 포함한 상용 제품을 판매한다면 "소프트웨어 저작권 라이선스"와 "코덱 특허 라이선스"는 완전히 별개의 법적 트랙이라는 점을 프로젝트가 인지하고 있는지 이번 확인 범위에서는 확인할 수 없었다.
- 저장소 어디에도(README, LICENSE-COMMERCIAL, vvdec.rs 모듈 문서) vvdec의 정확한 라이선스나 이 특허 이슈에 대한 언급이 없다 — "open-source"라는 한 단어로만 기술되어 있어, 향후 상용 고객에게 VVC 지원을 판매할 때 이 구분이 실사(due diligence) 단계에서 처음 불거질 위험이 있다.

**발생 조건**:
- 특허 풀이 존재하는 것으로 업계에 알려진 코덱(VVC/H.266, HEVC/H.265 등)의 참조 구현을 링크하면서, 상용 라이선스 판매 모델을 함께 운영할 때.

**권장**:
```markdown
<!-- docs/THIRD_PARTY_NATIVE_LIBS.md 또는 별도 법무 검토 문서 -->
## vvdec (VVC/H.266 decoding)
- Upstream: https://github.com/fraunhoferhhi/vvdec
- Software copyright license: (정확한 버전과 라이선스 텍스트를 실제로 확인해 여기 기록)
- Patent licensing: VVC는 특허 풀이 존재하는 코덱 — Bitvue가 상용 라이선스로
  VVC 디코딩을 판매할 경우 별도 특허 라이선스 필요 여부를 법무 검토 대상으로 등록.
```
- vvdec를 실제로 링크하는 정확한 버전을 고정하고, 그 버전의 `LICENSE` 파일 전문을 저장소에 복사해두거나 최소한 링크와 요약을 문서로 남긴다.
- "이 소프트웨어의 저작권 라이선스가 관대함"과 "이 코덱을 상용으로 판매할 특허 권리가 있음"은 별개 질문이라는 점을 상용 라이선스 판매 프로세스에 명시적으로 반영한다(법무 검토 대상).

**탐지 방법**:
- Manual: 실제로 링크되는 vvdec 버전을 특정하고 해당 버전 저장소의 `LICENSE`/`NOTICE` 파일 전문을 직접 확인.
- Structural: `LICENSE-COMMERCIAL`, README, 마케팅 자료에 VVC 지원이 언급되는 모든 지점을 나열해 특허 관련 고지/면책 문구가 있는지 대조.

**예외**:
- VVC 디코딩 기능을 상용 라이선스 트랙에서 완전히 제외하고 AGPL 트랙에서만 제공한다면(순수 오픈소스 사용자 간 배포이므로) 특허 이슈의 상업적 노출은 크게 줄어든다 — 다만 이는 제품 결정 사항이며 현재 코드는 그런 분리를 하고 있지 않다(`vvdec` feature가 `bitvue-decode` 크레이트 자체에 속하며 상용/오픈소스 빌드를 구분하는 장치가 없음).

**Bitvue 판정**: 미정 — 2단계(저장소 감사, 특히 법무 검토)에서 채움. 확인된 사실은 "저장소 어디에도 vvdec의 정확한 라이선스 텍스트나 버전이 기록되어 있지 않다"(`crates/bitvue-decode/src/vvdec.rs`의 모듈 문서는 "open-source"라고만 기술, `grep -r "vvdec" LICENSE* docs/` 결과 라이선스 텍스트 없음)는 점뿐이며, 업스트림 vvdec의 실제 라이선스 조항 및 VVC 특허 풀과의 관계에 대한 구체적 법률 판단은 이번 문서 작성 세션에서 검증되지 않았다.

---

### SUPPLY-005: libvmaf-sys/libvmaf-rs 업스트림 라이선스가 deny.toml allow-list와 정확히 일치하는지 미검증
**분류**: SUPPLY · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```toml
# deny.toml
[licenses]
allow = [
    "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", # ...
    # "BSD-2-Clause-Patent" 항목이 없음
]
```

**문제**:
- Netflix가 배포하는 libvmaf(C 라이브러리)는 업계에 "BSD+Patent"(SPDX 식별자 `BSD-2-Clause-Patent`)로 알려진, 명시적 특허 부여 조항이 포함된 라이선스를 사용한다 — 이는 `deny.toml`의 allow-list에 있는 평범한 `BSD-2-Clause`/`BSD-3-Clause`와는 **다른 SPDX 식별자**다. cargo-deny는 SPDX 식별자를 정확히 매칭하므로, `libvmaf-sys`/`libvmaf-rs` 크레이트의 crates.io 메타데이터가 이 정확한 문자열로 라이선스를 선언하고 있다면 allow-list에 없는 라이선스로 분류되어 거부(deny) 대상이 될 수 있다.
- 이 크레이트 체인은 `Cargo.lock`에 이미 해석되어 존재한다(`libvmaf-rs 0.5.2` → `libvmaf-sys 0.4.4`, `crates/bitvue-metrics/Cargo.toml`의 `vmaf` feature를 통해 optional 의존성으로 선언). 실제로 `cargo deny check licenses`를 이 feature를 활성화한 상태로 로컬에서 실행해 통과/실패 여부를 확인해야 하는데, 이 문서 작성 세션에는 셸 접근이 없어 직접 실행해 검증하지 못했다.
- SUPPLY-006과 결합하면 상황이 더 나쁘다: CI가 이 feature를 애초에 스캔하지 않는다면(SUPPLY-006), 설령 실제로 라이선스가 allow-list와 불일치하더라도 CI는 계속 초록불을 보여줄 것이다 — "문제가 있는데 아무도 모른다"의 전형.

**발생 조건**:
- `-sys` 크레이트가 감싸는 C 라이브러리의 라이선스가 흔한 SPDX 식별자의 변형(`-Patent`, `-Clear` 접미사 등)일 때, 그 변형이 allow-list에 명시적으로 포함되어 있는지 확인하지 않을 때.

**권장**:
```toml
[licenses]
allow = [
    # ...,
    "BSD-2-Clause-Patent",   # libvmaf-sys/libvmaf-rs (Netflix libvmaf 업스트림 실제 확인 후 추가)
]
```
```bash
# 로컬/CI에서 vmaf feature를 명시적으로 켜고 라이선스 체크를 실행해 실제 결과 확인
cargo deny check licenses --features bitvue-metrics/vmaf
```
- 먼저 `cargo metadata --features bitvue-metrics/vmaf`로 실제 크레이트 메타데이터의 `license` 필드 원문을 확인하고, 그 정확한 SPDX 문자열을 allow-list에 명시적으로 추가하거나(문제 없다면) 대체 크레이트를 검토한다(문제가 있다면).
- 모든 optional feature를 대상으로 `cargo deny check --all-features`를 최소 한 번은 CI에 추가해 이런 사각지대를 근본적으로 없앤다(SUPPLY-006과 동일한 해결책).

**탐지 방법**:
- Static: `cargo metadata --features bitvue-metrics/vmaf --format-version 1 | jq '.packages[] | select(.name | test("vmaf")) | .license'`로 정확한 라이선스 필드 확인.
- Structural: `deny.toml` allow-list와 위에서 확인한 문자열을 문자 그대로 대조.

**예외**:
- `libvmaf-sys`/`libvmaf-rs`의 crates.io 메타데이터가 실제로는 평범한 `BSD-2-Clause`/`MIT`로 선언되어 있다면(바인딩 크레이트 자체의 라이선스와 감싸는 C 라이브러리의 라이선스가 다르게 선언되는 경우가 흔함) 이 위험은 발생하지 않는다 — 그래서 이 항목은 "확정된 문제"가 아니라 "확인이 필요한 위험"으로 분류된다.

**Bitvue 판정**: Suspected — 확인된 사실: `Cargo.lock`에 `libvmaf-rs 0.5.2`(1358-1371줄) → `libvmaf-sys 0.4.4`(1374줄~) 의존성 체인이 실제로 존재하고, `deny.toml`의 `[licenses] allow` 목록(28-46줄)에는 `BSD-2-Clause-Patent`라는 정확한 문자열이 포함되어 있지 않다. 다만 `libvmaf-sys`/`libvmaf-rs` 크레이트 자체가 crates.io에 실제로 어떤 `license` 필드 문자열로 선언되어 있는지, 그리고 `cargo deny check licenses`를 이 feature 활성화 상태로 실행했을 때 실제로 실패하는지는 셸 접근 없이 이번 세션에서 검증하지 못했다 — 2단계에서 `cargo metadata`/`cargo deny`를 직접 실행해 확정할 것.

---

### SUPPLY-006: cargo-deny CI가 non-default feature(vmaf 등) 의존성을 스캔 그래프에서 놓칠 수 있음
**분류**: SUPPLY · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```yaml
# .github/workflows/license-check.yml
- name: Run cargo-deny
  run: cargo deny check ${{ matrix.checks }}
  # --all-features나 --features가 전혀 지정되지 않음
```
```toml
# crates/bitvue-metrics/Cargo.toml
[features]
default = []          # <- vmaf feature가 기본값에 없음
vmaf = ["dep:libvmaf-rs"]
```

**문제**:
- `cargo-deny`는 내부적으로 `cargo metadata`로 의존성 그래프를 얻는데, 명시적으로 `--all-features`나 특정 `--features`를 지정하지 않으면 기본 feature 집합만 활성화된 상태로 그래프가 계산된다. `bitvue-metrics`의 `vmaf`(그리고 `vmaf-cuda`) feature는 `default = []`에 포함되어 있지 않으므로(24-25번째 줄), CI의 `cargo deny check licenses`가 이 feature 경로로 들어오는 `libvmaf-rs`/`libvmaf-sys`를 실제로 스캔하는지 확실하지 않다.
- 같은 패턴이 `bitvue-decode`의 `ffmpeg`(`ffmpeg-next` optional dep) feature에도 적용된다 — non-default feature 뒤에 있는 모든 의존성이 잠재적으로 동일한 사각지대에 들어간다.
- 이는 "라이선스 검사 도구가 아예 없다"보다 더 위험할 수 있는 패턴이다 — CI가 실제로 초록불을 보여주기 때문에 "우리는 이미 이 부분을 검사하고 있다"는 잘못된 확신을 주지만, 실제로는 optional feature 뒤의 의존성을 하나도 보지 못하고 있을 수 있다.

**발생 조건**:
- Cargo 워크스페이스에 non-default feature로 게이팅된 optional 의존성이 있고, 라이선스/보안 CI가 명시적으로 `--all-features`를 지정하지 않을 때(cargo-deny뿐 아니라 `cargo audit` 등 다른 metadata 기반 도구에도 동일하게 적용됨).

**권장**:
```yaml
- name: Run cargo-deny (all features)
  run: cargo deny check ${{ matrix.checks }} --all-features
```
- CI의 `cargo deny check` 호출에 `--all-features`를 추가해 optional feature 뒤의 모든 의존성이 스캔 대상에 포함되도록 한다.
- 스캔 대상이 넓어지면 새로운 advisory/라이선스 위반이 드러날 수 있으므로, 이 변경 자체를 별도 PR로 분리해 결과를 검토한다.

**탐지 방법**:
- Structural: CI 워크플로우의 `cargo deny`/`cargo metadata`/`cargo audit` 호출부에 `--all-features` 또는 관련 `--features` 플래그가 있는지 grep.
- Manual: `cargo deny check licenses --all-features`와 플래그 없이 실행한 결과를 비교해 스캔되는 패키지 수 차이를 직접 확인.

**예외**:
- 모든 optional feature가 이미 default에 포함되어 있어 "non-default feature"라는 개념 자체가 없는 워크스페이스는 해당 없음.

**Bitvue 판정**: Suspected — 확인된 사실: `.github/workflows/license-check.yml`(1-34줄 전체 확인)의 `cargo deny check ${{ matrix.checks }}` 호출에는 `--all-features`/`--features` 플래그가 없고, `crates/bitvue-metrics/Cargo.toml`의 `default = []`(25번째 줄)에는 `vmaf`가 포함되어 있지 않다. 다만 cargo-deny/cargo-metadata가 이 구체적인 워크스페이스 구성에서 실제로 어떻게 동작하는지(예: 다른 workspace 멤버가 우연히 `bitvue-metrics/vmaf`를 활성화하는 경로가 있는지)는 셸로 직접 실행해보지 않아 확정하지 못했다.

---

### SUPPLY-007: RustSec 보안 권고(advisories) 체크가 continue-on-error로 소프트 실패 처리됨
**분류**: SUPPLY · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```yaml
jobs:
  cargo-deny:
    strategy:
      matrix:
        checks: [advisories, "bans licenses sources"]
    continue-on-error: ${{ matrix.checks == 'advisories' }}
    # advisories 체크가 실패해도 워크플로우 전체는 성공(초록불)으로 표시됨
```

**문제**:
- `continue-on-error: true`가 걸린 job은 실패해도 GitHub Actions UI에서 전체 워크플로우를 "성공"으로 표시한다 — 새 RustSec 보안 권고(예: 사용 중인 의존성에 대한 신규 CVE)가 발표되어도 CI가 빨간불로 눈에 띄게 알려주지 않고, PR을 보는 사람이 job 상세 페이지까지 열어봐야만 실패를 발견할 수 있다.
- 이 설정 자체는 근거 있는 트레이드오프다(주석에 "Prevent sudden announcement of a new advisory from failing CI"라고 명시) — 신규 advisory 발표 하나로 무관한 PR들의 머지가 막히는 것을 막기 위함이다. 다만 그 대가로 "보안 권고가 실제로 확인되고 대응되는지"를 보장하는 별도 장치(주기적 사람 확인, 알림 등)가 필요한데, 이 워크플로우에는 그 후속 장치가 보이지 않는다.
- 라이선스/공급망 카탈로그 관점에서 이 항목이 중요한 이유는, "새 의존성이 알려진 취약점을 가진 버전을 가져왔다"는 신호가 조용히 무시될 수 있는 구조이기 때문이다 — 이는 라이선스 문제는 아니지만 동일한 CI 파이프라인이 다루는 인접한 공급망 리스크다.

**발생 조건**:
- `cargo update`나 신규 의존성 추가로 알려진 취약점이 있는 버전이 들어왔는데, 그 시점에 해당 advisory가 이미 RustSec DB에 있거나 이후 발표될 때.

**권장**:
- `continue-on-error`는 유지하되, advisories job이 실패했을 때 Slack/이메일 등으로 별도 알림이 가는 단계를 추가하거나, 주기적(weekly cron은 이미 있음)으로 실패 여부를 사람이 확인하는 절차를 문서화한다.
- 최소한 advisories job 실패 시 PR에 코멘트를 자동으로 남기는 액션을 추가해 "초록불이지만 확인이 필요한 항목이 있다"는 신호를 리뷰어에게 노출한다.

**탐지 방법**:
- Static: 워크플로우 파일에서 `continue-on-error` 뒤에 별도 알림/후속 조치 job이 있는지 확인.
- Manual: 과거 RustSec advisory 발표 시점 전후로 이 job이 실제로 실패했는지, 그 실패가 실제로 대응되었는지 GitHub Actions 히스토리에서 확인.

**예외**:
- weekly cron이 실패할 때마다 담당자가 수동으로 Actions 탭을 확인하는 절차가 이미 팀 관례로 확립되어 있다면(문서화되지 않았더라도) 실질 위험은 낮아진다 — 다만 그 확립 여부는 코드로 확인할 수 없다.

**Bitvue 판정**: Confirmed(설정 존재) / 미정(운영 절차 여부) — `.github/workflows/license-check.yml` 25번째 줄에 `continue-on-error: ${{ matrix.checks == 'advisories' }}`가 정확히 이 형태로 존재함을 확인했다. 이 소프트 실패에 대한 별도 알림/후속 확인 절차가 실제로 운영되고 있는지는 저장소 파일만으로는 확인할 수 없어 미정으로 남긴다.

---

### SUPPLY-008: 컨트리뷰터 라이선스 동의(CLA/DCO) 부재 — 외부 기여를 상용 라이선스 트랙에 재라이선싱할 법적 근거 없음
**분류**: SUPPLY · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```markdown
<!-- CONTRIBUTING.md -->
## License
By contributing, you agree that your contributions will be licensed under the AGPL-3.0.
<!-- 이것이 라이선스 관련 유일한 문장. CLA도, DCO(Signed-off-by 요구)도 없음 -->
```

**문제**:
- 이 한 문장은 외부 기여자의 코드가 **AGPL-3.0으로 배포되는 것에는 동의**를 받지만, Bitvue가 그 동일한 코드를 `LICENSE-COMMERCIAL`(유료 상용 라이선스) 트랙으로 **재라이선싱해서 판매할 권리**까지 부여하지는 않는다 — 저작권법상 이는 별개의 권리다. 오픈코어 모델(하나의 코드베이스를 AGPL과 상용 라이선스 두 트랙으로 동시에 배포)이 법적으로 성립하려면, 그 코드베이스의 모든 저작권자(=모든 컨트리뷰터 포함)로부터 "상용 트랙으로도 재라이선싱해도 좋다"는 명시적 동의(CLA, Contributor License Agreement)를 받거나, 최소한 컨트리뷰터 본인이 해당 기여의 저작권을 갖고 있다는 진술(DCO, Developer Certificate of Origin — `Signed-off-by` 커밋 트레일러로 표현)을 받아야 한다.
- 이 동의 없이 외부 PR을 병합하면, 그 PR에 포함된 코드는 이론상 AGPL로만 배포 가능하고 상용 라이선스 고객에게 판매할 수 없는 상태로 남는다 — 실무에서는 이 문제가 몇 년간 누적된 뒤에야(예: 대규모 컨트리뷰터 정리, 인수합병 실사, 상용 고객의 라이선스 감사 요구 시) 발견되어 훨씬 큰 비용으로 해결해야 하는 경우가 많다(전체 코드베이스에서 문제 있는 커밋들을 되짚어 각 저자에게 사후 동의를 구하거나, 코드를 다시 작성해야 함).
- CLA/DCO는 컨트리뷰터 입장에서 진입 장벽으로 작용할 수 있어(오픈소스 커뮤니티의 반발 대상이 되기도 함) 도입 여부와 방식(가벼운 DCO vs 무거운 CLA)은 신중한 사업적 판단이 필요하지만, "아무 장치도 없음"은 이중 라이선스 모델에서는 방치할 수 없는 공백이다.

**발생 조건**:
- 오픈코어(AGPL + 상용) 라이선스 모델을 채택한 프로젝트가 외부(회사 외부, 저작권 귀속 계약이 없는) 기여자로부터 PR을 받아들이기 시작하는 순간부터.

**권장**:
```markdown
<!-- CONTRIBUTING.md에 추가 -->
## Contributor License Agreement

Bitvue is dual-licensed under AGPL-3.0-or-later and a commercial license
(see LICENSE-COMMERCIAL). To allow us to offer your contribution under both
tracks, all commits must include a `Signed-off-by` trailer (DCO) certifying
you have the right to submit the contribution under these terms:

    git commit -s -m "feat: ..."

For significant contributions, a full CLA may be required — contact
license@bitvue.dev.
```
```yaml
# .github/workflows/dco-check.yml (예시)
- uses: dcoapp/app@v2   # 또는 probot-dco 등 기존 액션 활용
```
- 최소 침습적 옵션으로 DCO(`Signed-off-by` 요구 + CI에서 검증하는 액션)부터 도입하고, 필요하면 이후 정식 CLA(개별 서명 또는 CLA-assistant 봇 연동)로 확장한다.
- 이미 병합된 과거 커밋들의 컨트리뷰터 목록을 정리해, 소급 동의가 필요한 대상이 있는지 별도로 감사한다(이번 카탈로그의 범위 밖 — 법무/메인테이너 판단 필요).

**탐지 방법**:
- Structural: `CONTRIBUTING.md`, `.github/` 전체에서 "CLA", "DCO", "Signed-off-by", "Developer Certificate of Origin" 키워드를 grep.
- Manual: 저장소의 외부 기여 PR 히스토리를 열어 실제로 CLA/DCO 서명을 요구하는 봇이나 체크가 동작했는지 확인.

**예외**:
- 지금까지 모든 코드가 단일 저작권자(예: 회사 소속 직원 전원, 고용 계약상 저작물 귀속이 이미 명확한 경우)에 의해서만 작성되었고 외부 PR을 아직 한 건도 받지 않았다면 이 위험은 아직 현실화되지 않았다 — 다만 외부 기여를 받기 **시작하기 전에** 도입해야 소급 비용을 피할 수 있다.

**Bitvue 판정**: Confirmed — 저장소 전체(`CONTRIBUTING.md`, `.github/` 하위 워크플로우 전부, `CODE_OF_CONDUCT.md`)에서 "CLA", "DCO", "Signed-off-by", "Developer Certificate of Origin" 문자열이 실제 컨트리뷰터 라이선싱 절차를 가리키는 용례로 단 한 건도 발견되지 않았다(grep으로 나온 다른 매치는 전부 "class"/"declare" 등 무관한 부분 문자열). `CONTRIBUTING.md` 236번째 줄의 "contributions will be licensed under the AGPL-3.0"이 유일한 라이선스 관련 문장이며, 이는 상용 트랙 재라이선싱 권리를 다루지 않는다.

---

### SUPPLY-009: 벤더링된/링크된 서드파티 코드에 대한 NOTICE·저작권고지 파일 부재
**분류**: SUPPLY · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```
bitvue/
├── LICENSE                  # AGPL-3.0 전문
├── LICENSE-COMMERCIAL        # 상용 라이선스 전문
└── (NOTICE 파일이 없음)
```

**문제**:
- Apache-2.0(`crates/vendor/abseil`가 이 라이선스로 벤더링됨, `crates/vendor/abseil/LICENSE`에서 확인)은 원본 배포물에 `NOTICE` 파일이 존재할 경우 그 내용을 재배포물에도 포함하도록 요구하는 조항(§4)을 갖고 있고, 실무 관례상 파생/재배포 프로젝트가 자체 `NOTICE` 파일에 "이 제품은 Apache-2.0으로 라이선스된 X를 포함합니다" 같은 고지를 남기는 것이 표준적인 컴플라이언스 관행이다.
- MIT/BSD 계열 라이선스(`deny.toml` allow-list의 대다수, 예: `MIT`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`)도 대부분 "저작권 고지와 라이선스 텍스트를 배포물에 유지하라"는 조건을 명시하고 있는데, 이 조건은 소스코드 저장소 안에서는 각 크레이트의 `Cargo.lock`/소스 자체로 만족되지만, **최종 사용자에게 배포되는 컴파일된 바이너리**(Tauri 앱 번들, `.dmg`/`.exe`/`.AppImage`)에는 그 고지가 어떤 형태로도 실려 나가지 않는다 — 소스 저장소에 라이선스가 존재하는 것과, 배포되는 바이너리 제품에 고지가 실제로 동봉되는 것은 다른 문제다.
- 이는 사소한 문서 누락이 아니라 각 서드파티 라이선스가 명시하는 "재배포 시 조건"을 실제로 이행하지 못하고 있다는 뜻이며, 상용 라이선스로 판매하는 제품일수록(더 많은 계약적 보증을 하는 입장이므로) 이 공백의 실무적 리스크가 커진다.

**발생 조건**:
- MIT/BSD/Apache-2.0 등 "고지 유지" 조건이 있는 라이선스의 의존성을 포함한 소프트웨어를 컴파일된 바이너리 형태로 최종 사용자에게 배포할 때, 그 바이너리 자체에 고지를 담는 메커니즘이 없을 때.

**권장**:
```bash
# cargo-about 등으로 실제 의존성 라이선스 텍스트를 모아 NOTICE/THIRD_PARTY_LICENSES 파일로 생성
cargo install cargo-about
cargo about generate about.hbs > THIRD_PARTY_LICENSES.html
```
```markdown
<!-- 루트 NOTICE 파일 (신규) -->
This product includes software developed by third parties:
- dav1d (BSD-2-Clause) — VideoLAN
- abseil-rust (Apache-2.0) — vendored at crates/vendor/abseil
- vvdec (see docs/THIRD_PARTY_NATIVE_LIBS.md — SUPPLY-004)
- libvmaf (optional, feature-gated — see SUPPLY-005)
(전체 목록은 `cargo about`/`THIRD_PARTY_LICENSES.html`로 자동 생성해 최신 상태 유지)
```
- `cargo-about`/`cargo-license` 같은 도구로 전체 Rust 의존성의 라이선스 텍스트를 모은 `THIRD_PARTY_LICENSES` 파일을 생성하고, 릴리스 빌드 아티팩트(앱 번들)에 실제로 포함시킨다.
- 이 생성 과정을 CI(릴리스 워크플로우)에 통합해, 의존성이 바뀔 때마다 고지 파일도 자동 갱신되게 한다(수동 관리는 곧 stale해짐).

**탐지 방법**:
- Static: 저장소 루트와 빌드 산출물(앱 번들 내부)에 `NOTICE`/`THIRD_PARTY_LICENSES`/`about.html` 유사 파일이 존재하는지 확인.
- Manual: 실제로 빌드된 `.app`/`.dmg`/`.exe`를 열어 그 안에 서드파티 라이선스 고지가 어떤 형태로든 포함되어 있는지 확인.

**예외**:
- 모든 의존성이 저작권고지 유지 조건이 없는 라이선스(예: Unlicense, 0BSD처럼 조건이 사실상 없는 것)로만 구성되어 있다면 이 항목의 실무적 필요성은 낮아진다 — Bitvue의 allow-list에는 MIT/BSD/Apache-2.0이 다수 포함되어 있어 이 예외에 해당하지 않는다.

**Bitvue 판정**: Confirmed — 저장소 루트 및 하위 전체에서 `NOTICE*` 파일을 glob 검색한 결과 0건. `cargo-about`/`cargo-license`/SBOM 생성 도구의 흔적도 저장소 전체에서 발견되지 않았다(`.github/workflows/*.yml`, `scripts/` 확인 결과 관련 스텝 없음). `crates/vendor/abseil`처럼 Apache-2.0으로 명시적으로 라이선스된 벤더 코드가 실제로 포함되어 있음에도(해당 크레이트 `Cargo.toml` 7번째 줄 `license = "Apache-2.0"` 확인) 최종 배포 산출물에 그 고지를 전달할 메커니즘이 없다.

---

### SUPPLY-010: About 다이얼로그가 구현되어 있지 않아 최종 사용자에게 라이선스/서드파티 고지가 노출될 경로가 없음
**분류**: SUPPLY · **심각도**: Low · **탐지**: Structural

**나쁜 예**:
```tsx
// frontend/components/TitleBar.tsx
{
  id: "help",
  items: [
    { id: "docs", label: "Documentation" },
    { id: "shortcuts", label: "Keyboard Shortcuts", action: onShowShortcuts },
    { id: "sep1", label: "", separator: true },
    { id: "about", label: "About Bitvue" },   // <- action 없음 = 이 컴포넌트 자체 관례상 자동 비활성화
  ],
}
```

**문제**:
- `MenuItem` 타입의 주석(19번째 줄, "items without action are auto-disabled")에 따르면 `action`이 없는 메뉴 항목은 자동으로 비활성화된 껍데기다 — "About Bitvue" 메뉴 항목이 UI에 보이지만 실제로 클릭해도 아무 일도 일어나지 않는 스텁 상태다.
- SUPPLY-009에서 생성 방법을 다루는 서드파티 고지/라이선스 텍스트가 있더라도, 그것을 최종 사용자가 애플리케이션 안에서 실제로 볼 수 있는 화면(About 다이얼로그, 라이선스 뷰어 등)이 존재하지 않으면 그 고지는 사실상 저장소 안에만 존재하는 문서가 된다 — 사용자가 배포판을 통해 실제로 도달할 수 있는 경로가 없다.
- 버전 번호(`workspace.package.version = "0.12.0"`), 빌드 정보, 라이선스 종류(AGPL/상용 중 어느 빌드인지) 같은 정보도 About 다이얼로그가 없으면 사용자가 앱 내에서 확인할 방법이 없다 — 지원 요청이나 버그 리포트 시 버전 확인도 어려워진다는 부수적 UX 문제도 딸려온다.

**발생 조건**:
- 데스크톱 앱에서 "About" 메뉴 항목은 관례적으로 존재하지만 실제 구현(모달/다이얼로그 컴포넌트, 버전/라이선스 정보 표시)이 아직 붙지 않았을 때.

**권장**:
```tsx
{ id: "about", label: "About Bitvue", action: onShowAbout }
```
```tsx
// frontend/components/AboutDialog.tsx (신규)
export function AboutDialog({ open, onClose }: AboutDialogProps) {
  return (
    <Modal open={open} onClose={onClose}>
      <h2>Bitvue v{APP_VERSION}</h2>
      <p>Dual-licensed under AGPL-3.0-or-later or a commercial license.</p>
      <a href="#" onClick={showThirdPartyLicenses}>Third-party licenses</a>
    </Modal>
  );
}
```
- "About Bitvue" 메뉴 항목에 실제 `action`을 연결해 버전/빌드/라이선스 트랙 정보를 보여주는 최소한의 다이얼로그를 구현한다.
- SUPPLY-009에서 생성한 `THIRD_PARTY_LICENSES` 콘텐츠를 이 다이얼로그에서(또는 별도 링크로) 열람 가능하게 연결한다.

**탐지 방법**:
- Static: `TitleBar.tsx`의 `about` 메뉴 항목에 `action`이 연결되어 있는지, 대응하는 다이얼로그 컴포넌트가 실제로 존재하는지 확인.
- Manual: 실행 중인 앱에서 Help → About Bitvue를 클릭해 실제로 무언가 열리는지 확인.

**예외**:
- CLI 전용 배포(`bitvue-cli`)처럼 GUI 메뉴 자체가 없는 배포 형태는 `--version`/`--about` 같은 CLI 플래그로 동일한 정보를 제공하면 충분하며 이 항목의 UI 특정 지적은 적용되지 않는다.

**Bitvue 판정**: Confirmed — `frontend/components/TitleBar.tsx` 251번째 줄의 `{ id: "about", label: "About Bitvue" }` 항목에는 `action` 필드가 없다(같은 파일 19번째 줄 주석에 따라 이는 자동 비활성화를 의미). 저장소 전체에서 `AboutDialog`류 컴포넌트나 라이선스 뷰어 컴포넌트는 발견되지 않았다.

---

### SUPPLY-011: Dependabot 설정의 npm(frontend) 생태계 경로 오류로 프런트엔드 의존성 드리프트 감지 커버리지 0
**분류**: SUPPLY · **심각도**: High · **탐지**: Static

**나쁜 예**:
```yaml
# .github/dependabot.yml
- package-ecosystem: "npm"
  directory: "/src"   # <- 실제 package.json은 /frontend에 있음, /src는 존재하지 않는 경로
  schedule:
    interval: "weekly"
```

**문제**:
- 저장소의 실제 프런트엔드 프로젝트는 `frontend/package.json`에 있다(`frontend/` 하위에 `node_modules`, 테스트 등 프런트엔드 전체 구조 존재). `directory: "/src"`에 해당하는 `package.json`은 저장소 어디에도 없다(glob 검색 결과 `src/package.json` 0건).
- Dependabot은 설정된 `directory`에 매니페스트 파일이 없으면 조용히 아무 것도 하지 않는다(에러로 눈에 띄게 실패하지 않는 경우가 많다) — 즉 이 설정은 "작동하는 것처럼 보이지만 실제로는 아무 npm 패키지도 스캔하지 않는" 상태로 오랫동안 방치될 수 있다.
- 그 결과 프런트엔드(React/TypeScript, `frontend/package.json`)의 수백 개 npm 의존성 중 어떤 것도 자동 업데이트/취약점 알림/버전 드리프트 감지의 대상이 아니다 — Rust 쪽(`cargo` ecosystem, 올바르게 `directory: "/"`로 설정됨)과 GitHub Actions 쪽은 정상 커버되지만, npm 쪽만 사각지대다. npm 생태계는 transitive 의존성 수가 매우 많고 라이선스가 프로젝트마다 제각각인 것으로 알려져 있어(코드-레벨 사실은 아니지만 업계에 널리 알려진 특성), 자동 감지가 없다는 것은 이 문서가 다루는 "transitive 의존성 라이선스 드리프트"의 가장 큰 잠재 노출면이다.
- 부수적으로 `reviewers`/`assignees` 필드가 실제 사용자명이 아니라 플레이스홀더 문자열 `"your-username"`으로 남아 있다 — cargo/github-actions 항목은 경로 자체는 맞지만, 이 필드가 유효하지 않은 계정을 가리키고 있어 PR 생성 시 리뷰어 배정이 실패하거나 무시될 수 있다.

**발생 조건**:
- 모노레포에서 여러 `package-ecosystem` 항목을 설정하며 `directory` 경로를 실제 프로젝트 구조 변경(예: `src/` → `frontend/` 리네이밍) 이후 갱신하지 않을 때.

**권장**:
```yaml
- package-ecosystem: "npm"
  directory: "/frontend"     # 실제 package.json 위치로 수정
  schedule:
    interval: "weekly"
    day: "monday"
  open-pull-requests-limit: 5
  reviewers:
    - "<실제 GitHub 사용자명 또는 팀>"
  labels:
    - "dependencies"
    - "frontend"
```
- `directory`를 실제 `frontend/package.json` 경로로 즉시 수정한다 — 이는 코드 한 줄 수준의 수정이지만 지금까지 완전히 커버되지 않던 의존성 표면 전체를 되살리는 변경이다.
- `reviewers`/`assignees`의 플레이스홀더 값을 실제 유효한 계정/팀으로 교체한다.
- Dependabot 설정 변경 후 실제로 PR이 생성되는지(첫 주간 스케줄 또는 수동 트리거) 확인해 회귀를 방지한다.

**탐지 방법**:
- Static: `.github/dependabot.yml`의 각 `directory` 값이 실제로 해당 위치에 매니페스트 파일(`Cargo.toml`, `package.json` 등)을 갖고 있는지 대조.
- Manual: GitHub 저장소의 Insights → Dependency graph → Dependabot 탭에서 npm 생태계 관련 PR/알림이 실제로 생성된 이력이 있는지 확인(이 문서 작성 시점에는 GitHub API 접근 없이 저장소 파일만으로 확인).

**예외**:
- 없음 — 경로가 실제 존재하지 않는 이상 이 설정은 어떤 상황에서도 의도대로 동작할 수 없다.

**Bitvue 판정**: Confirmed — `.github/dependabot.yml` 36-47번째 줄의 `package-ecosystem: "npm"` 항목이 `directory: "/src"`로 설정되어 있으나, 저장소에는 `src/package.json`이 존재하지 않는다(glob 검색 결과 0건이며 실제 프런트엔드 매니페스트는 `frontend/package.json`). `reviewers: ["your-username"]`(10-11번째 줄), `assignees: ["your-username"]`(12-13번째 줄)도 플레이스홀더 값 그대로 남아 있다.

---

### SUPPLY-012: deny.toml의 관대한 `[bans]` 설정이 버전/의존성 드리프트를 넓게 허용
**분류**: SUPPLY · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```toml
# deny.toml
[bans]
multiple-versions = "warn"              # 중복 버전은 경고만, 빌드를 막지 않음
wildcards = "allow"                     # "*" 버전 와일드카드 의존성 허용
workspace-default-features = "allow"
external-default-features = "allow"
allow = []
deny = []                                # 명시적으로 금지된 크레이트가 하나도 없음
```

**문제**:
- `wildcards = "allow"`는 어떤 의존성이 `version = "*"`처럼 상한 없는 버전 범위를 선언해도 통과시킨다 — 이런 의존성은 `cargo update` 한 번으로 라이선스가 바뀐 메이저 버전(실제로 발생한 사례가 업계에 존재하는 패턴 — 라이선스 재협상으로 이후 버전부터 라이선스 조건이 달라지는 경우)이 조용히 들어와도 이 규칙만으로는 막지 못한다.
- `multiple-versions = "warn"`은 동일 크레이트의 서로 다른 버전이 의존성 트리에 공존해도 CI를 실패시키지 않는다 — 오래된 버전이 이후 라이선스가 바뀐 최신 버전과 나란히 남아있는 상황을 방치할 수 있고, 무엇보다 "어느 버전의 라이선스가 실제로 적용되는지"를 사람이 판단하기 더 어렵게 만든다.
- `deny = []`(명시적으로 금지된 크레이트 목록이 비어 있음)는 "알려진 문제가 있는 특정 크레이트"(과거에 라이선스 재협상 이슈가 있었거나, 유지보수가 중단되어 보안 위험이 큰 것으로 알려진 크레이트 등)를 사전에 차단하는 메커니즘을 전혀 활용하고 있지 않다는 뜻이다.
- 이 설정들 각각은 licenses allow-list라는 1차 방어선이 이미 있는 상태에서의 2차적 완화 부재이며, "지금 당장 뭔가 잘못되었다"는 의미는 아니다 — 다만 향후 드리프트에 대한 여유(margin)가 다른 설정에 비해 좁다는 뜻이다.

**발생 조건**:
- `cargo update`로 의존성 버전이 갱신되는 모든 시점, 특히 `[bans]` 설정이 관대한 상태에서 새 메이저 버전이 라이선스 조건을 바꾸는 경우.

**권장**:
```toml
[bans]
multiple-versions = "deny"      # 최소한 신규 중복 버전 유입 시 CI가 눈에 띄게 실패하도록
wildcards = "deny"               # 버전 상한 없는 의존성 금지, 명시적 범위 요구
```
- `wildcards`를 `deny`로 바꿔 모든 의존성이 명시적 버전 상한을 갖도록 강제한다(이미 워크스페이스 의존성 대부분이 `"1.0"`, `"0.9"`처럼 구체적 버전을 쓰고 있어 전환 비용은 낮을 것으로 보임).
- `multiple-versions`를 최소 특정 크레이트군(예: 코덱/미디어 관련 핵심 의존성)에 한해 `deny`로 좁혀, 라이선스 판단이 중요한 의존성일수록 버전을 단일화한다.

**탐지 방법**:
- Static: `deny.toml`의 `[bans]` 섹션 설정값을 grep해 `"warn"`/`"allow"`로 되어 있는 항목을 나열.
- Runtime: `wildcards = "deny"`로 바꾼 뒤 CI를 실행해 실제로 위반하는 의존성이 있는지 확인.

**예외**:
- 의존성 개수가 적고 각 의존성의 버전/라이선스를 팀 전원이 암묵적으로 파악하고 있는 초기 단계 프로젝트라면 이 설정을 당장 엄격하게 바꿀 필요성은 낮다 — 다만 의존성 수가 늘어날수록(현재 워크스페이스는 이미 20개 이상의 멤버 크레이트를 가짐) 이 여유의 비용이 커진다.

**Bitvue 판정**: Confirmed — `deny.toml`의 `[bans]` 섹션(62-69번째 줄)이 `multiple-versions = "warn"`, `wildcards = "allow"`, `workspace-default-features = "allow"`, `external-default-features = "allow"`, `deny = []`로 정확히 위와 같이 설정되어 있음을 확인했다. 이 설정이 실제로 문제를 일으킨 구체적 사례가 있는지는 확인되지 않았으며(현재 시점의 위반 여부는 `cargo deny check bans`를 직접 실행해야 확정 가능), 이 항목은 "현재 위반"이 아니라 "여유가 좁은 설정"을 기록한 것이다.

---

### SUPPLY-013: SBOM(Software Bill of Materials) 생성/배포 파이프라인 부재
**분류**: SUPPLY · **심각도**: Medium · **탐지**: Structural

**나쁜 예**: (해당 없음 — 부재 자체가 문제)

**문제**:
- SBOM(예: CycloneDX, SPDX 포맷)은 "이 릴리스 바이너리가 정확히 어떤 버전의 어떤 의존성들로 구성되어 있는지"를 기계가 읽을 수 있는 형태로 제공한다. 이것이 없으면 (1) 신규 CVE가 발표되었을 때 "우리 제품이 영향받는지"를 확인하는 데 수동 조사가 필요하고, (2) 상용 라이선스 고객이 실사 과정에서 의존성 목록/라이선스 증빙을 요구할 때 즉시 제공할 자료가 없으며, (3) SUPPLY-009의 NOTICE 파일도 결국 SBOM 데이터를 기반으로 자동 생성하는 것이 정석인데 그 원천 데이터 자체가 없다.
- 이는 "지금 당장 무언가 잘못되었다"는 뜻이 아니라 "사고가 발생했을 때 대응 시간이 길어지고, 상용 판매 과정에서 반복적으로 수동 조사가 필요해진다"는 구조적 비용이다.

**발생 조건**:
- 상용 라이선스를 판매하는 오픈코어 프로젝트에서 릴리스 아티팩트에 대한 SBOM 생성/보관 절차가 릴리스 프로세스에 통합되어 있지 않을 때.

**권장**:
```yaml
# .github/workflows/release.yml에 추가할 수 있는 예시 스텝
- name: Generate SBOM
  run: |
    cargo install cargo-cyclonedx
    cargo cyclonedx --format json --output-cdx bitvue-sbom.json
- name: Attach SBOM to release
  uses: softprops/action-gh-release@v2
  with:
    files: bitvue-sbom.json
```
- 릴리스 워크플로우에 SBOM 생성 스텝을 추가하고 릴리스 아티팩트로 첨부한다.
- 이 SBOM을 SUPPLY-009의 NOTICE 파일 생성 및 SUPPLY-003의 네이티브 라이브러리 문서와 연결해 하나의 "이 릴리스가 무엇으로 만들어졌는가" 단일 소스로 통합하는 것을 장기 목표로 삼는다.

**탐지 방법**:
- Static: `.github/workflows/release.yml` 및 `scripts/`에서 SBOM 생성 도구(`cargo-cyclonedx`, `syft`, `cargo-sbom` 등) 호출이 있는지 grep.
- Structural: 최근 릴리스의 GitHub Release 페이지에 SBOM 파일이 첨부되어 있는지 확인.

**예외**:
- 아직 릴리스를 외부에 배포하지 않는 초기 개발 단계라면 우선순위는 낮다 — 다만 `LICENSE-COMMERCIAL`이 이미 존재하고 상용 판매를 전제하고 있어 이 예외의 적용 여지는 크지 않다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사, SBOM 생성 도구 도입 여부에 대한 팀 결정 포함)에서 채움. 확인된 사실은 "현재 저장소 어디에도 SBOM 생성 도구/스텝이 없다"(`cargo-cyclonedx`/`syft`/`spdx` 등 키워드 grep 결과 코드/워크플로우 어디에도 없음, `docs/anti-patterns/BUILD.md`가 개념적으로만 언급)는 것뿐이며, 이 카탈로그가 요구하는 수준의 도입 우선순위/설계는 별도 감사 대상이다.

---

### SUPPLY-014: 릴리스 바이너리 코드 서명/공증(macOS notarization, Windows Authenticode) 부재
**분류**: SUPPLY · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```yaml
# .github/workflows/release.yml
# codesign/notarize/Authenticode 관련 스텝, secret 사용이 전혀 없음
jobs:
  build:
    steps:
      - run: cargo tauri build
      - uses: softprops/action-gh-release@v2
        with:
          files: target/release/bundle/**
```

**문제**:
- 서명/공증되지 않은 macOS 앱은 Gatekeeper 경고("확인되지 않은 개발자")를 띄우고, 서명되지 않은 Windows 실행 파일은 SmartScreen이 "알 수 없는 게시자" 경고를 띄운다 — 사용자가 이 경고를 무시하고 실행하는 것에 익숙해지면, 그 경고가 실제 위협(악성 변조 바이너리)을 알리는 신호일 때도 무시하게 되는 학습 효과가 생긴다.
- 서명이 없으면 "이 바이너리가 실제로 Bitvue 메인테이너가 빌드한 것과 동일하다"는 것을 사용자나 OS가 검증할 방법이 없다 — 릴리스 파이프라인이나 GitHub Release 자산이 (예: 계정 탈취, CI 침해로) 변조되어도 이를 감지할 기술적 장치가 없다는 뜻이다. 이는 SEC-014(자동 업데이트 서명 검증)와 인접하지만 별개다 — SEC-014는 "자동 업데이트 기능 자체가 없어 해당 없음"으로 판정되었으나, 이 항목은 자동 업데이트 유무와 무관하게 **최초 배포되는 릴리스 바이너리 자체**의 서명/공증을 다룬다.
- 상용 라이선스 고객에게 판매하는 제품이 서명되지 않은 바이너리로 배포되는 것은 신뢰도/전문성 인상에도 영향을 준다 — 이는 순수 보안 이슈를 넘어 상용 트랙의 판매 신뢰성 문제이기도 하다.

**발생 조건**:
- macOS/Windows 데스크톱 앱을 GitHub Release 등으로 직접 배포하면서, 플랫폼 표준 서명 절차(Apple Developer ID + notarization, Windows Authenticode 인증서)를 릴리스 CI에 통합하지 않았을 때.

**권장**:
```yaml
# tauri-apps/tauri-action 사용 시 secrets로 서명 자격 증명을 주입하는 예시
- uses: tauri-apps/tauri-action@v0
  env:
    APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
    APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
```
- macOS: Apple Developer ID 인증서로 서명하고 `notarytool`(또는 Tauri의 내장 notarization 지원)로 공증을 받는다.
- Windows: Authenticode 인증서(EV 코드서명이 SmartScreen 평판을 즉시 확보하는 데 유리)로 실행 파일/설치 프로그램에 서명한다.
- 서명 인증서/비밀키는 CI secret으로 격리하고, SEC-014의 업데이터 서명 키 관리 원칙(회전 절차, 노출 시 대응)을 동일하게 적용한다.

**탐지 방법**:
- Static: `.github/workflows/release.yml`, `.github/workflows/build-tauri-app.yml`에서 `codesign`, `notarize`, `signtool`, `APPLE_ID`, `APPLE_CERTIFICATE` 등 서명 관련 키워드를 grep.
- Manual: 실제로 배포된 릴리스 바이너리를 다운로드해 macOS Gatekeeper/Windows SmartScreen 경고가 뜨는지 확인.

**예외**:
- 사용자가 반드시 소스에서 직접 빌드하도록 강제하는 배포 모델(프리빌트 바이너리를 아예 제공하지 않음)이라면 이 항목은 적용되지 않는다 — 다만 `.github/workflows/release.yml`/`build-tauri-app.yml`이 존재한다는 것 자체가 프리빌트 바이너리 배포를 전제한다.

**Bitvue 판정**: Confirmed — `.github/workflows/release.yml` 전체에서 `sign`/`notariz`/`codesign`/`APPLE_ID` 등 서명 관련 키워드 grep 결과 0건. 릴리스 빌드 산출물에 대한 코드 서명/공증 스텝이 현재 파이프라인에 존재하지 않는다.

---

### SUPPLY-015: 재현 가능한 빌드(reproducible build) 검증 부재
**분류**: SUPPLY · **심각도**: Low · **탐지**: Structural

**나쁜 예**: (해당 없음 — 부재 자체가 문제)

**문제**:
- 동일한 소스 커밋에서 두 번 빌드했을 때 바이트 단위로 동일한 산출물이 나온다는 것(재현 가능한 빌드)을 검증하는 절차가 없으면, "GitHub Release에 올라온 바이너리가 정말 공개된 소스 그대로에서 빌드된 것인가"를 외부에서 독립적으로 검증할 방법이 없다 — SUPPLY-014(서명)가 "누가 서명했는가"를 보장한다면, 재현 가능한 빌드는 "서명한 그 조직이 실제로 공개 소스 그대로를 빌드했는가"라는 한 단계 더 깊은 신뢰를 보장한다.
- Rust 생태계에서 완전한 재현성은 컴파일러 버전, 타임스탬프, 경로 임베딩 등 여러 요인에 의해 쉽지 않다고 알려져 있어, 이 항목은 "당장 구현 가능한 낮은 우선순위"로 분류하는 것이 합리적이다.

**발생 조건**: (일반적 배경 — 특정 트리거 없음, 프로젝트가 신뢰도를 높이려는 성숙 단계에서 고려)

**권장**:
- 단기적으로는 우선순위를 낮게 두고, 장기적으로 `cargo build --locked`와 고정된 툴체인 버전(`rust-toolchain.toml`)을 전제로 재현성 실험을 하는 정도로 시작한다.

**탐지 방법**:
- Structural: 릴리스 프로세스에 재현성 검증 스텝이나 관련 문서가 있는지 확인(없으면 이 항목 해당).

**예외**:
- 대부분의 오픈소스 프로젝트가 아직 이 수준의 검증을 갖추지 않은 것이 현재 업계 평균이므로, 이 항목의 부재 자체를 다른 항목들과 동일한 긴급도로 다룰 필요는 없다.

**Bitvue 판정**: 미정 — 2단계(우선순위 판단 포함한 별도 감사)에서 채움. 현재 저장소에 재현 가능한 빌드 검증 절차가 없다는 사실은 확인했으나(관련 스텝/문서 grep 결과 없음), 이 카탈로그 작성 세션에서 그 이상의 구체적 조사는 수행하지 않았다.

---

### SUPPLY-016: 시스템 설치 네이티브 라이브러리(vvdec 등)의 버전/해시 핀 고정 부재
**분류**: SUPPLY · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
//! The vvdec library must be installed on the system:
//! - macOS: `brew install vvdec`
//! - Linux: Build from source at https://github.com/fraunhoferhhi/vvdec
//! - Windows: Download prebuilt binaries from vvdec releases
// <- 어떤 버전을 설치해야 하는지 명시되어 있지 않음
```

**문제**:
- `Cargo.lock`은 Rust crates.io 의존성의 정확한 버전을 고정해, 서로 다른 개발자/CI 러너가 같은 커밋에서 항상 같은 버전의 의존성으로 빌드되게 보장한다. vvdec는 Cargo 의존성이 아니므로(SUPPLY-003) 이 보장 메커니즘 밖에 있고, `brew install vvdec`/"build from source"/"download prebuilt binaries" 중 무엇을 택하든 **어떤 버전을 설치해야 하는지가 문서 어디에도 명시되어 있지 않다** — 개발자 A는 오늘 `brew`가 제공하는 최신 버전을, 개발자 B는 몇 달 전 소스에서 빌드한 버전을, CI는 또 다른 시점의 버전을 링크하게 될 수 있다.
- 이는 라이선스 관점에서도 실질적 리스크다: vvdec 업스트림이 향후 라이선스 조건을 변경한 새 버전을 릴리스하면(SUPPLY-004에서 언급한 특허 조항처럼, 오픈소스 프로젝트가 버전 간 라이선스를 바꾸는 사례는 드물지 않다), Bitvue의 어떤 빌드가 어떤 라이선스 조건의 vvdec를 실제로 링크했는지조차 사후에 재구성하기 어렵다 — Cargo 의존성이라면 `Cargo.lock` 히스토리로 정확히 답할 수 있는 질문이 여기서는 답할 수 없는 질문이 된다.
- 보안 관점에서도 동일한 문제가 있다(오래된 버전에 알려진 취약점이 있어도 아무도 그 사실을 추적하지 못함) — 다만 이 카탈로그는 라이선싱 초점이므로 이 측면은 부수적으로만 언급한다.

**발생 조건**:
- 네이티브 라이브러리를 Cargo 의존성 그래프 밖에서, 버전을 명시하지 않고 "시스템에 설치되어 있는 것을 사용" 방식으로 링크할 때 전반.

**권장**:
```rust
//! Requires vvdec >= 2.1.0, < 3.0.0 (tested against 2.1.3).
//! Verify with: `pkg-config --modversion vvdec`
```
```yaml
# CI에서 정확한 버전을 고정 설치
- name: Install vvdec (pinned version)
  run: |
    curl -L https://github.com/fraunhoferhhi/vvdec/releases/download/v2.1.3/vvdec-2.1.3-linux.tar.gz -o vvdec.tar.gz
    echo "<expected-sha256>  vvdec.tar.gz" | sha256sum -c -
    tar xzf vvdec.tar.gz
```
- 모듈 문서(`vvdec.rs`)와 `CONTRIBUTING.md`에 요구되는 최소/최대 버전 범위를 명시하고, 가능하면 빌드 스크립트에서 `pkg-config --modversion`으로 실제 링크되는 버전을 확인해 범위를 벗어나면 빌드를 실패시킨다.
- CI에서는 특정 버전을 체크섬과 함께 고정 설치해(위 예시), 로컬 개발 환경과 별개로 최소한 CI 빌드/릴리스 산출물만큼은 어떤 버전의 vvdec로 만들어졌는지 항상 정확히 알 수 있게 한다.

**탐지 방법**:
- Static: 모듈 문서, `CONTRIBUTING.md`, CI 워크플로우에서 vvdec(및 다른 시스템 설치 네이티브 라이브러리)의 버전 범위/체크섬이 명시되어 있는지 확인.
- Structural: CI 빌드 로그에서 실제로 설치되는 vvdec 버전을 출력하는 스텝이 있는지 확인.

**예외**:
- 네이티브 라이브러리가 ABI/API뿐 아니라 라이선스까지 버전 간 완전히 안정적이라고 업스트림이 명시적으로 보증하는 경우는 위험이 낮아지지만, 이런 보증을 하는 프로젝트는 드물다.

**Bitvue 판정**: Confirmed — `crates/bitvue-decode/src/vvdec.rs` 모듈 문서(6-11번째 줄)의 세 가지 설치 방법(`brew install vvdec`, 소스 빌드, 프리빌트 바이너리 다운로드) 어디에도 구체적인 버전 번호나 버전 범위가 명시되어 있지 않다. `CONTRIBUTING.md`의 시스템 의존성 절(15-18번째 줄)도 dav1d 설치 방법만 다루고 vvdec는 언급하지 않는다. CI 워크플로우(`.github/workflows/*.yml`)에서도 vvdec 버전을 고정 설치하는 스텝은 확인되지 않았다.
