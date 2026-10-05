# Bitvue — Development Phases & Roadmap

> Phase 0-12 roadmap for closing the VQ Analyzer parity gap, plus the product/architecture decisions it rests on.
> **Status source of truth: `docs/specs/features.yaml`** — this doc keeps only decisions, rationale and ordering.
> Dated implementation logs (2026-08-08..09 sidecar/protocol/desktop/indexer rollout, Phase 4 AV1 entropy-decoder
> rewrite, Phase 7.5/7.6 session logs) moved verbatim to `docs/history/development-log.md`.
> See also: `docs/specs/features.yaml`, `VQA_PARITY_SPEC_V3.md`, `PARITY_CHECKLIST.md`,
> `COMPETITOR_FEATURE_MATRIX.md`, `UX_PARITY_MATRIX.md`.

---

## 우선순위 재검토 (2026-07-31)

경쟁사/UX 리서치 후 Phase 0-12 순서를 재검토한 결론:

- **Phase 1(F키 모드 분기) 최우선 유지** — Phase 2-6 코덱별 오버레이 전부가 이 인프라에 의존.
- **Phase 7.5(Dual-Stream Compare & VMAF) 위치 유지** — 기존 렌더/디코드 파이프라인을 재사용하는 독립 트랙이라 Phase 1-6을 밀어낼 근거가 아님.
- **Phase 7.6(Context Menu & Evidence Bundle) 신규 삽입** — `UX_PARITY_MATRIX.md` §9에서 P0/P1, Phase 7.5 Compare가 두 계약을 직접 요구하므로 Phase 11에서 7.5 직후로 승격.
- 그 외 신규 발견(CABAC 시각화, AVM 명명, 컨테이너 확장, HRD 그래프, bit-distribution 등)은 기존 phase에 흡수. Phase 8(Syntax 패널)은 범위가 커졌지만 "코덱 완성(3-6) → 신택스 심화(8)" 순서 논리는 그대로.

---

## 제품 아키텍처 확정 (2026-08-08, Tauri→Electron 전환)

> 결정 확정 및 구현 완료 — Tauri `src-tauri/`는 삭제됨(`e7194cc`), Electron 셸 + Rust sidecar가 현행 아키텍처.

### 제품 구조: bitvue-engine 공유, 3-제품 분기

```
                    bitvue-engine (공유 엔진)
      codec/parser · bitstream model · metrics · diagnostics · compare · indexing
                               │
             ┌─────────────────┼─────────────────┐
             ▼                 ▼                 ▼
      Bitvue Analyzer      Bitvue Probe       Bitvue CLI
       (Electron GUI)     (live monitoring)   (automation/CI)
```

| 제품 | 역할 | 사용 시점 |
|---|---|---|
| **Bitvue Analyzer** | Deep inspection / debugging — 개발 리소스 80-90% | 문제 원인 분석 |
| **Bitvue Probe** | Live/장시간 관측 (QoS: bitrate/fps/QP/GOP; 이벤트: corruption/discontinuity/decoder error) — Analyzer 안정화 후 착수 | 문제 탐지 |
| **Bitvue CLI** | 자동화/CI/스크립팅 (`crates/bitvue-cli`, Phase 9에서 확장) | 반복 분석, 회귀 게이트 |
| **bitvue-mcp** | 4번째 제품이 아니라 engine의 인터페이스 중 하나 (Rust API / CLI / IPC / MCP) | 에이전트/외부 툴 연동 |

**경계 (필수 준수):** Analyzer(파서 안전성·syntax 표현·codec state가 핵심 위험)와 Probe(프레임 정렬·색공간 정합성·중복 decode·메트릭 정확성이 핵심 위험)는 실패 모드가 다르다 — `UniversalFrameAnalysis` 같은 god-struct로 합치지 말 것 (Phase 7.5 원칙의 3-제품 확장).

**Multi-sync 범위:** `bitvue-engine/src/selection.rs`의 `SelectionState`(7 필드: `stream_id`/`temporal`/`cursor`/`unit`/`syntax_node`/`bit_range`/`source_view`)가 Syntax tree·Player·Timeline·Hex·QP heatmap을 커버. **Ref Graph 노드 / Metrics 샘플** 선택은 필드 자체가 없음 — sidecar가 아니라 engine 설계 작업. 이 struct에는 "God object refactoring note: intentionally cohesive"라는 방어적 주석이 있으니 필드 추가 전에 검토할 것.

**CLI가 증명한 것과 못 한 것:** `bitvue-cli`가 engine/formats/decode/codec/metrics 크레이트에만 의존한다는 사실은 **라이브러리 모듈성**만 증명한다. GUI가 필요로 하는 incremental/viewport-scoped/cancelable query(`get_syntax_range`, `get_hex_range` 등) 형태는 CLI(1회성 배치 호출)로 검증되지 않는다 — "core-UI 분리는 됐다, interactive query API 설계는 별개".

### 왜 Electron인가 — 결정 근거 (요약)

| Bitvue 관점 | Tauri v2 | Electron |
|---|---|---|
| macOS/Windows/Linux 렌더러 일관성 | 낮음 | 높음 (타겟이 Chromium 하나라 재현·추적이 쉽다는 의미 — "문제가 없다"는 아님, 아래 각주) |
| Linux WebGL/Canvas 지원 부담 | 높음 | 상대적으로 낮음 |
| Rust 코어 직접 호출 | 매우 좋음 | bridge 필요 |
| Rust crash 격리 | 별도 설계 필요 | **sidecar를 선택할 때만** 성립 (napi-rs면 이 행 무효) |
| 번들·idle 메모리 | 좋음 | 나쁨 (Bitvue는 이미 4K/8K 프레임 버퍼로 메모리를 많이 쓰는 도구라 실사용 환경에서 재확인 필요 — 아래 각주) |
| 대규모 시각화 생태계 | 보통 | 좋음 |
| DevTools/프로파일링 | 플랫폼별 차이 | 일관적 |
| Playwright/E2E 재현성 | 보통 | 좋음 |
| 업데이트·배포 사례 | 보통 | 풍부함 |
| 기본 보안 모델 | 좋음 | 명시적 hardening 필요 |
| 장기 플랫폼 QA 비용 | 높음 | 낮음 |
| 현재 Bitvue 코드 재사용 | 최대 | Rust 코어·React 재사용 가능 |
| 앱 전체 전환 비용 | 없음 | Tauri adapter 재작성 |

> **각주 — Linux 렌더러 일관성:** Electron도 Linux GPU 가속은 까다롭다(VAAPI/ANGLE 백엔드 선택, X11 vs Wayland, NVIDIA 독점 드라이버,
> `--disable-gpu-sandbox` 류 플래그). Electron이 사는 건 "Linux GPU 문제가 없어짐"이 아니라 "타겟이 WebKitGTK 대신 Chromium 하나로 줄어
> 재현·추적 비용이 낮아짐"이다.
>
> **각주 — idle 메모리:** "Electron의 무거움은 실패 요인이 아니다"는 사용자 판단을 반영한 것. Chromium 베이스라인(윈도우당 수백MB)이
> encoder/decoder 벤치마크 툴과 동시 실행되는 실사용 환경에서 무해한지는 실측으로 재확인 권장.

**결정 조건 (미해결):** Linux가 best-effort(Windows/macOS 1급, Ubuntu LTS 공식 지원, 기타 Linux best-effort)였다면 Tauri 유지로도 충분했다. Electron이 맞는 건 "Linux까지 진짜 1급으로 지원하는 전문 영상 시각화 도구"라는 전제에서다 (실패 요인은 번들 크기가 아니라 특정 Linux 빈 화면, GPU fallback 차이, overlay 렌더링 차이, WebKit 버전차). **Linux 1급 여부는 사용자가 확정한 적 없음** — 플랫폼 지원 티어 문서화는 열린 항목. Electron에서도 Linux GPU(VAAPI/ANGLE, X11/Wayland, NVIDIA)는 계속 트래킹 필요.

### 브리지 방식 결정: sidecar (napi-rs 아님)

| | napi-rs (in-process) | sidecar (별도 프로세스) ✔ |
|---|---|---|
| 호출 | N-API 직접 호출 | stdio + 바이너리 프레이밍 wire protocol |
| Rust crash 격리 | 안 됨 — FFI segfault 시 Electron main 전체 사망 | 됨 |
| `bitvue-protocol` | 공유 타입 정의 수준 | 문자 그대로 wire schema |

**근거:** Bitvue의 핵심 가치가 malformed 비트스트림 파싱이라 FFI 디코더(dav1d/vvdec/libvmaf) segfault 위험이 상시 존재(`docs/anti-patterns/PARSE.md`/`CODEC.md`가 이 카테고리를 통째로 다룸). napi-rs는 이 경우 Electron main 전체를 죽여 Tauri 대비 개선이 없다. sidecar는 초기 비용(프로세스 lifecycle, IPC 프레이밍)이 더 들지만 Bitvue 특성상 맞는 트레이드오프. sidecar = VS Code language-server 패턴. Electron main이 spawn/monitor/restart 담당.
브리지 방식이 `bitvue-protocol`의 성격(공유 타입 vs 진짜 wire schema)을 정하므로 크레이트 경계보다 먼저 결정했다. 원래 4-box 구성(engine/protocol/desktop/ui)은 napi-rs(in-process)를 암묵적으로 가정했고, sidecar 결정으로 5번째 조각(engine을 감싸고 stdio로 protocol을 말하는 독립 바이너리 `bitvue-sidecar`)이 필요해졌다.

**Sidecar 운영 정책 (구현됨, 2026-10-05 코드 기준 — 상태는 `INFRA-002` 등 YAML):**
- **동시성:** 요청당 OS 스레드(tokio 아님 — Core 작업이 CPU-bound 동기 Rust), 실행은 `MAX_CONCURRENT_REQUESTS = 16` 세마포어로 제한(`request_dispatch.rs`). 리더 루프는 블록되지 않고, 응답 계산은 순수 함수, stdout 쓰기만 mutex로 직렬화. 종료 시 in-flight 워커 전부 join. 핸들러 panic은 `catch_unwind`로 잡아 해당 correlation_id에 응답.
- **취소:** `cancel_request{target_id}` → 요청별 `AtomicBool`; 오래 걸리는 핸들러(index/thumbnail/decode/first-diff)만 중간 체크포인트. 응답 `CANCELLED`.
- **크래시 복구:** 프로세스만 재시작(maxAttempts 3, backoff 500ms, 새 프로세스 `hello` 확인). Core 인메모리 상태는 복구하지 않음(영속화/커맨드 로그 없음) — 대기 요청은 재시도 없이 reject, 렌더러는 재시작 알림 후 사용자에게 파일 재오픈을 요청.
- **미해결:** 크래시 시 미완료 요청 타임아웃 정책 세분화, 하나의 요청이 여러 data 프레임을 순차로 내는 progressive 응답.

### 핵심 원칙 — 프레임을 control 메시지에 태우지 않는다

Tauri 시절 `YUVFrameData`(Y/U/V를 base64 문자열로 JSON 직렬화)가 정확히 "Rust frame → JSON → IPC → React state → Canvas" 안티패턴이었다. 커맨드를 1:1 번역하면 재발하므로 `bitvue-protocol`이 강제 지점이다.

- **Control plane** — 작고 구조화된 JSON만 (open/select/cancel/query 메타데이터).
- **Data plane** — 목적별 typed 전송:

| 데이터 | 전송 형태 |
|---|---|
| decoded frame | raw bytes data 프레임 → structured clone (base64 금지) |
| thumbnail | encoded image cache |
| syntax tree | viewport/프레임 단위 lazy |
| hex data | byte range만 |
| QP/MV map | compact typed array (예: MV `Int16Array [x,y,ref,flags,...]`) |
| statistics | batch/chunk |

Renderer↔main/utility process 간 data plane 전달에는 Electron `MessagePort`도 활용 가능(제안, 미채택).

**`bitvue-protocol` wire schema v0:** 단일 stdio duplex(stdin/stdout = 프레임 전용, **stderr = 로그/패닉 전용**). 고정 9바이트 헤더 `[1B kind][4B LE correlation_id][4B LE payload_len]`, kind `0=control 1=data 2=event`. Control = JSON 봉투 `{id, method, params}` / `{id, ok, result|error{code,message,offset}}`, 헤더 correlation_id = JSON id. Data = raw bytes, 메타데이터는 같은 correlation_id의 직전 control 응답. 기동 직후 `hello` 핸드셰이크(`protocol_version` "0.1.0"). 에러 코드는 `BitvueError`와 분리된 `WireErrorCode`로 매핑(크로스 언어 계약 격리). Core의 실패는 `DiagnosticAdded` 이벤트로 표현되므로 wire 레이어가 RPC 에러를 지어내지 않는다.

### 크레이트/프로세스 경계

```
bitvue-engine     순수 Rust 도메인 엔진 (crates/bitvue-engine + formats/decode/codecs/metrics)
bitvue-protocol   request/event/error/binary frame schema           crates/bitvue-protocol
bitvue-indexer    컨테이너/유닛 메타데이터 인덱싱 (현재 IVF/AV1)        crates/bitvue-indexer
bitvue-sidecar    engine을 감싸고 bitvue-protocol을 stdio로 말하는 독립 바이너리   crates/bitvue-sidecar
bitvue-desktop    Electron main + preload + SidecarClient              bitvue-desktop/
React renderer    UI (bridge: frontend/services/electronBridgeService.ts)  frontend/
```

Electron = presentation/workspace 계층만; parsing/indexing/frame model/metrics/diagnostics/compare는 전부 Rust가 소유 — **엔진을 JS로 옮기지 않는다.** 렌더러는 이름 있는 preload 채널(`window.bitvue.*`)로만 sidecar에 접근(범용 invoke 통로 없음). `frontend/` → `bitvue-ui` 리네이밍은 하지 않았음.

> Feature/status items for this section live in `docs/specs/features.yaml` (area `infra` — protocol, sidecar,
> packaging, platform tier; area `qa` — typecheck gate, Electron selftest/screenshot harness; area `probe` — Bitvue Probe).
> Summary (2026-10-05 snapshot): protocol/sidecar/desktop shell done; sidecar analysis commands are AV1/IVF-only (`INFRA-001` is the
> cross-cutting non-AV1 wiring blocker); open: renderer sandbox hardening, code signing/auto-update,
> sidecar/desktop tests in CI, Linux tier decision, Bitvue Probe.

---

## 제품 단계 & 미해결 계약

### 확정 순서 (제품 단계)

```
Phase 1  Bitvue Analyzer (AV1 분석 완성)          ← 현재 위치 (2026-08-08 결정 시점)
Phase 2  Analyzer multi-codec (H264/HEVC/VP9/...)
Phase 3  Bitvue CLI 강화 (CI/regression/automation) — 이미 부분 구현
Phase 4  Bitvue Probe (live monitoring)
Phase 5  MCP/SDK/ecosystem — 이미 부분 구현
```

Probe보다 CLI를 먼저 하는 이유: Probe부터 벌리면 범위가 너무 커짐. CLI 강화는 "Analyzer core가 제대로 분리됐는지"를 검증하는 저비용 아키텍처 테스트 — 라이브러리 분리는 이미 통과했지만 interactive query API 설계 검증은 CLI 강화 단계에서 완료해야 함.

> 주의: 위 "제품 Phase 1-5"와 아래 "로드맵 Phase 0-12"는 번호 체계가 다르다. `features.yaml`의 `phase:`는 로드맵 번호("0".."12"), 제품 Phase는 `"product-N"`으로 표기.

### 미해결 — Probe→Analyzer 핸드오프

Probe에서 이상 구간 클릭 → "Analyze in Bitvue" → Analyzer가 해당 시점의 stream/frame을 염. Probe와 Analyzer가 별개 앱/윈도우면 명시적 계약(딥링크 프로토콜 또는 로컬 소켓으로 stream+frame+timestamp 전달)이 필요 — `bitvue-engine` 공유만으로는 풀리지 않는 유일한 조각. Probe phase 착수 시 설계, 지금은 블로커 아님. (Item: `PROBE-003`, `phase: product-4`)

**한 줄 정의:** Bitvue = 영상 코덱을 위한 observability & analysis platform. Analyzer로 원인을 파고들고, Probe로 문제를 발견하고, CLI로 자동화한다.

---

## Phase 현황 요약 (features.yaml, 2026-10-05)

| Phase | Items | done | partial | todo | dropped |
|---|---|---|---|---|---|
| 0 Setup & foundation | 11 | 5 | 3 | 3 | 0 |
| 1 F-key mode system | 3 | 1 | 1 | 0 | 1 |
| 2 Info overlays | 23 | 4 | 10 | 9 | 0 |
| 3 VVC | 4 | 0 | 3 | 1 | 0 |
| 4 AV1 advanced modes | 14 | 5 | 4 | 5 | 0 |
| 5 AVS3 | 5 | 0 | 2 | 3 | 0 |
| 6 JPEG XS / VC-3 / APV | 6 | 0 | 4 | 2 | 0 |
| 7 YUVDiff | 8 | 3 | 3 | 2 | 0 |
| 7.5 Dual-stream compare & VMAF | 11 | 5 | 2 | 4 | 0 |
| 7.6 Context menu & evidence bundle | 6 | 3 | 3 | 0 | 0 |
| 8 Syntax panels | 10 | 1 | 7 | 2 | 0 |
| 9 CLI | 17 | 5 | 8 | 4 | 0 |
| 10 Performance | 12 | 3 | 9 | 0 | 0 |
| 11 Polish / shortcuts / options | 10 | 3 | 6 | 0 | 1 |
| 12 Parity validation | 5 | 0 | 4 | 1 | 0 |
| (product phase `product-4`, Probe) | 1 | 0 | 0 | 1 | 0 |
| (no `phase:`) | 231 | 72 | 90 | 65 | 4 |

Computed with PyYAML: group `items[]` by the string field `phase` (missing → "no phase"), count `status`. Row totals sum to 377.

Query: `python3 -c "import yaml;[print(i['id'],i['status'],i['title']) for i in yaml.safe_load(open('docs/specs/features.yaml'))['items'] if i.get('phase')=='7.5']"`

## 로드맵 Phase 0-6 (Tauri 시절 원안, Electron+sidecar 기준으로 재해석)

> 아래 각 Phase의 "현황/현실/남은 일" 문장은 2026-10-05 코드 대조 스냅샷이다(2026-08-10 재감사 요약을 압축).
> 현재 상태는 features.yaml 해당 `phase:` 항목을 본다.

원래 로드맵은 "Tauri 커맨드 추가"를 구현 단위로 삼았다(`src-tauri`는 2026-08-08 삭제). 이제 구현 벡터는 `bitvue-sidecar` 커맨드(`crates/bitvue-sidecar/src/request_dispatch.rs`)이고, 기능 요구사항 자체(코덱, 오버레이, 재생, export)는 유효하다. 옛 `[x]` 표시는 신뢰하지 말 것 — 상태는 `features.yaml`이 유일한 source of truth.

**공통 블로커:** `bitvue-sidecar`는 코덱 크레이트 중 `bitvue-av1-codec`에만 의존하고, 데스크톱 인덱싱(`index_stream`)은 IVF/AV1 전용이다. 따라서 HEVC/AVC/VP9/VVC/AVS3/JPEG XS/VC-3는 파서·추출기·렌더러가 있어도 GUI에서 실데이터가 나오지 않는다(codec 감지도 `get_stream_info` 경유라 non-AV1은 `activeCodec=null`). 프론트엔드 비테스트 소스 9개 파일이 아직 `@tauri-apps/*`를 import(죽은 경로, 2026-10-05; INFRA-006).

## Phase 0: Project Setup & Foundation

**목표:** 프로젝트 구조, 빌드, CI, 기본 컨테이너/디코더 기반 마련.

- 결정: Tauri → Electron + Rust sidecar(stdin/stdout framed protocol) 전환(2026-08-08). 프론트엔드는 `window.bitvue.*` 브리지 경유.
- 남은 범위: 추가 컨테이너(MXF/AVI demux/HEIC/DASH-MPD), MPEG-2 디코딩, 잔여 Tauri `invoke()` 제거.

Items: features.yaml `phase: "0"`

## Phase 1: 코덱별 F키 모드 분기 시스템

**목표:** 고정 6모드 → 코덱별 동적 모드 시스템으로 교체.

- 결정: Rust `CodecModeRegistry` + `get_available_modes` 백엔드 커맨드 대신 **정적 프론트엔드 레지스트리**(`frontend/utils/codecModeRegistry.ts`, 9개 코덱)로 구현 — 모드 목록이 파일별로 바뀌지 않아 백엔드 조회 불필요.
- 캐비어트: UI 배선은 9개 코덱 모두 존재하지만 codec 감지/오버레이 데이터는 AV1/IVF에서만 실제로 동작(위 공통 블로커).

Items: features.yaml `phase: "1"`

## Phase 2: 코덱별 Info Overlay 토글 시스템

**목표:** QP Map, Heat Map, MV Heat 등 오버레이를 코덱별로 사용 가능/불가능 처리.

- 토글 인프라(코덱별 매트릭스, 다중 중첩 α=0.72, 코덱별 상태 저장)는 완료. 렌더러는 `frontend/components/panels/OverlayRenderer/renderers/`에 다수 존재.
- 남은 작업의 대부분은 렌더러가 아니라 **sidecar 코덱별 배선**; 순수 신규는 PU Type, PSNR/SSIM, Simple Motion, Inter Memory Reads, SAO, Reconstruction(+Detail popup), CABAC 시각화, 전용 bit-cost Heat Map.
- 색상 스케일/투명도 parity 기준은 `docs/UX_PARITY_MATRIX.md` 참조.

Items: features.yaml `phase: "2"`

## Phase 3: VVC 전용 기능 완성

**목표:** VVC 파싱 → 디코딩 → 전용 모드(Dual Tree, Inverse Map, ALF, Inter Memory Reads) 완전 구현.

- 현황 요지: vvdec FFI(`crates/bitvue-decode/src/vvdec.rs`)와 VVC 파티션 파서(`bitvue-vvc`)는 있으나 "파서는 있는데 제품 어디에서도 안 부른다". `vvdec` feature는 어떤 빌드/CI에서도 켜지지 않고, `get_decoded_frame_yuv`는 `Av1Decoder`로 하드코딩.
- 남은 일: (1) sidecar를 `vvdec` feature로 빌드, (2) 디코드 경로 코덱별 디코더 선택, (3) `bitvue-vvc`를 sidecar에 배선, (4) LMCS/ALF 추출 함수 신규.
- Parity 검증 기준: Dual Tree 루마/크로마 경계가 VQ Analyzer와 pixel-perfect 매칭; LMCS 적용 시퀀스에서 Inverse Map 시각화 확인.

Items: features.yaml `phase: "3"`

## Phase 4: AV1 전용 고급 모드 완성

**목표:** AV1 전용 F6~F9 모드(CDEF, SuperRes, Loop Restoration, Film Grain) 구현.

- 유일하게 백엔드 데이터가 제품 경로(AV1/IVF)와 일치하는 Phase — 백엔드·sidecar(`get_av1_features`)·렌더러·e2e 테스트까지 완료.
- 2026-08-11~19에 AV1 엔트로피 디코더를 spec/rav1d/dav1d 대조로 재작성(적응형 CDF, 실제 컨텍스트, palette, segment_id, DRL, compound, temporal MV). 결정: 정합성 우선, byte-exact dav1d 오라클 대조로 검증; 서드파티 테스트 클립은 리포 편입 금지.
- Parity 검증 기준: CDEF 방향 화살표가 실제 CDEF 방향 결정과 일치(단위/e2e 테스트로 코드 레벨 검증, VQ Analyzer 육안 대조는 미실시); Film Grain 전/후 픽셀이 dav1d 출력과 일치(픽셀 비교 테스트 없음).
- 남은 갭: temporal MV가 프로덕션 경로에서 미사용(sidecar가 무상태·프레임 단위 랜덤 액세스 — 순차 디코드+캐시 재설계 필요, 보류), sign_bias 기반 확장 후보, gm_params 실값, 별도 V delta-q. 상세 개발 로그는 `docs/history/development-log.md`.

Items: features.yaml `phase: "4"`

## Phase 5: AVS3 지원 구현

**목표:** AVS3 코덱 전체 파싱 + 디코딩 + 전용 모드(ESAO/CCSAO).

- 헤더 파싱(`crates/bitvue-avs3/`)은 완료되어 CLI `decode --avs3`에서 프레임 목록까지 동작. ESAO/CCSAO는 휴리스틱 proxy이며 sidecar 미배선.
- 우선순위: AEC 엔트로피 디코더 → CTU/CU 파싱 → sidecar 배선 → openavs3d/FFmpeg 픽셀 디코딩 → ESAO/CCSAO를 실제 데이터로 교체. AEC가 최대 리스크.
- 레퍼런스: AVS 표준 http://www.avs.org.cn/ , openavs3d.

Items: features.yaml `phase: "5"`

## Phase 6: JPEG XS + VC-3 + APV 지원

**목표:** JPEG XS, VC-3/DNxHD, APV 파싱·디코딩·전용 모드.

- JPEG XS/VC-3는 AVS3와 같은 패턴: 파서·추출기·렌더러 존재, CLI 프레임 목록 동작, sidecar 미배선, 픽셀 디코딩 없음. 가장 저비용 경로는 sidecar 배선.
- APV는 완전 미착수. 주의: 원 문서는 APV를 "Apple ProRes Video"로 적었지만 APV는 Advanced Professional Video(ProRes와 별개 코덱) — 범위 재확인 필요.

Items: features.yaml `phase: "6"`

---

## Phase 7: YUVDiff 모드 완성

**목표:** Debug YUV(디코더 출력 vs 레퍼런스 raw YUV) 비교를 VQ Analyzer 수준으로 완성.

- 백엔드는 sidecar `debug_yuv` 모듈(`load_debug_yuv`/`get_debug_yuv_frame`/`get_yuv_diff_metrics`/`find_first_diff_frame`/
  `set_debug_yuv_offset`/`set_debug_yuv_crop`), UI는 `YuvDiffPanel` + `LoadDebugYuvDialog` + `CropDialog`.
- **결정:** diff/amplified/metrics는 레퍼런스 bit depth와 무관하게 8-bit 정밀도로 비교 (wire format·렌더러가 8-bit 전용).
  10-bit+ 비교는 별도 wire/renderer 경로가 필요 — 실제 10-bit 테스트 자산이 생기면 재검토.
- 세션 상태(path/format/offset/crop)는 `Core`가 아닌 sidecar `DebugYuvSlot`에 둔다 — StreamId(A/B) 상태가 아니고, `Core`는
  파일시스템/디코드에 의존하지 않는 leaf crate이기 때문.
- Parity 기준: PSNR이 `ffmpeg -i ref.yuv -i decoded.yuv -lavfi psnr` 결과와 ±0.01 dB 일치; 차이 프레임 시각화가 VQ Analyzer 색상 표현과 일치.
- 디코드 측은 stream A(`decode_bridge`, AV1/IVF) 기준.

Items: features.yaml `phase: "7"`.

## Phase 7.5: Dual-Stream Compare & VMAF

**목표:** VQ Probe/StreamEye 패리티의 핵심 갭인 두 비트스트림 비교 워크플로우와 VMAF 지표.

**Analyzer/Probe 분리 원칙 (필수 준수):** Bitvue는 VQ Analyzer 성격(비트스트림 구조/코덱 상태)과 VQ Probe 성격(화질 비교/정렬/메트릭)을
동시에 만든다. 실패 모드가 다르다 — Analyzer는 파서 안전성·syntax 표현 폭증·codec state, Probe는 프레임 정렬·색공간 정합성·중복
decode·메트릭 정확성. **분석 결과를 하나의 거대 구조체(`UniversalFrameAnalysis{syntax, mv, qp, psnr, ssim, vmaf, ...}`)로 합치지 말 것.**
공유 대상은 미디어 입력·실행 인프라(Demuxer, Decoder abstraction, PixelFormat, ColorMetadata, FrameBuffer/Pool, Timeline, Job scheduler,
Cache budget)이지 분석 결과 도메인이 아니다. `bitvue-engine/src/{compare,alignment,compare_cache,compare_evidence,compare_strategy,diff_heatmap}.rs`는
비트스트림 분석 코드와 독립 모듈로 유지한다. `docs/anti-patterns/`도 BIT-*(Analyzer)와 VQ-*(Probe) 하위 카탈로그를 분리 유지.

- 구조: 엔진 compare 로직은 stream-agnostic, sidecar `compare.rs`(`create_compare_workspace`/`get_aligned_frame`/`set_sync_mode`/
  `set_manual_offset`/`reset_offset`/`get_diff_frame`/`find_first_diff_frame_ab`, `CompareSlot` 상태)는 얇은 배선.
  프레임 정렬은 `bitvue_indexer::build_frame_index_map` 재사용 → **A/B 비교는 indexer 제약을 그대로 상속해 AV1/IVF 전용.**
- `get_diff_frame`은 두 스트림 luma를 `destride_luma`로 stride 패딩 제거 후 `DiffHeatmapData::from_luma_planes` 호출; 해상도는
  exact-match 가드 (허용오차 "호환"이어도 assert 패닉 위험).
- Diff 모드 명명: Subtraction = Signed, Temperature = Abs. `metric` 모드는 엔진에 미구현이라 UI에서 제외.
- Split(H/V) 뷰는 픽셀 파이프라인 재사용(오프스크린 캔버스 2개 + `ctx.clip()` 합성), diff 컬럼은 side-by-side 전용.

**Diff Heatmap 레퍼런스 (from `_import_v14/.../visualization/DIFF_HEATMAP_IMPLEMENTATION_SPEC.md`, CMP-03):**

| 항목 | 값 |
|---|---|
| 입력 | 해상도/색공간 정렬된 프레임 A/B (luma 또는 RGBA→luma 변환), 선택적 블록 단위 메트릭 델타 맵 |
| 모드 (UI 토글) | `abs`(\|lumaA-lumaB\|, 기본값) / `signed`(lumaA-lumaB) / `metric`(블록별 메트릭 델타) |
| 텍스처 생성 | QP 히트맵과 동일 규칙으로 기본 Half-res; abs 모드는 4단계 램프, diff=0은 완전 투명; alpha는 diff에 비례, 최대 `180 * user_opacity`로 클램프 |
| 캐시 키 | `overlay_diff:<codec>:<filehashA>:<filehashB>:f<frame>\|hm<hw>x<hh>\|mode<abs\|signed\|metric>\|op<bucket>` |
| 인터랙션 | hover 시 픽셀/블록 diff 값 표시; 클릭 시 툴팁 고정(ESC로 해제) |
| 승인 테스트 | diff=0 영역 완전 투명 / opacity만 바뀌면 캐시 재사용 / 모드 전환 시에만 텍스처 재생성 |

Parity 검증: `VQA_PARITY_SPEC_V3.md` §4.9. Items: features.yaml `phase: "7.5"` (legacy CMP-01..06).

## Phase 7.6: UX Contract — Context Menu & Evidence Bundle

**목표:** 전체 앱 공통 우클릭 컨텍스트 메뉴 계약과 원클릭 Evidence Bundle export 계약.

**승격 근거:** 둘 다 `UX_PARITY_MATRIX.md` §9에서 P0/P1로 채점된 실제 갭이라 Phase 11 "폴리시"까지 미루지 않고 승격. 착수 시 엔진의
`export::context_menu`(guard 엔진 + 5스코프 카탈로그)와 `export::evidence`(실제 파일 쓰기)는 이미 완성돼 있었고 sidecar 노출만
빠져 있었다 — "배선 누락" 패턴(원래 추정 2~3주 → 실측 반나절).

- 컨텍스트 메뉴: `get_context_menu_items` + 범용 `ContextMenu` 컴포넌트, 5스코프(Player/HexView/StreamView/Timeline/DiagnosticsPanel),
  guard `always`/`has_selection`/`has_byte_range`, disabled reason은 툴팁.
- Evidence bundle: `export_evidence_bundle`, 진입점 4종(MainMenu / ContextMenu / 패널 / CompareWorkspace "Export Diff Bundle").
  `workspace`/`mode`는 자유 문자열이므로 Compare 진입점에 새 백엔드 스코프가 필요 없음. diff/ABI 검사는 CLI `bitvue evidence-diff A B [--strict]`.
- `parity_harness::context`는 사용되지 않는 중복 구현 — 코드에 문서화만, 제거는 별도 결정.
- 스크린샷 하네스는 `window.alert`를 스텁해야 클릭 시퀀스가 멈추지 않음(`bitvue-desktop/electron/main.ts`).

Items: features.yaml `phase: "7.6"` (legacy CTX-01, EVB-01).

## Phase 8: Syntax 패널 완성 (코덱별 탭 상세화)

**목표:** 각 코덱의 Syntax 탭을 VQ Analyzer 수준으로 완성 — Stats(분포 차트, bit distribution, scene change), HRD/CPB·DPB 그래프,
CABAC 트레이스, 트리↔Hex 동기화, 코덱별 탭(HEVC QM, VP9 Probs, VVC APS, Ref Lists).

- 현실 (2026-10-05 코드 기준): `get_codec_extended_info`가 AV1 Ref Lists/QP histogram만 공급. HEVC/VP9/VVC 탭(QM/Probs/APS)은 죽은 Tauri `invoke()` UI
  셸 — sidecar에 해당 코덱 경로 자체가 없음. HRD 패널은 프레임 크기 기반 추정치("(est.)")이며 시그널된 HRD 파라미터 아님.
- 범위 밖: T-STD 등 conformance-checking 레이어.

Items: features.yaml `phase: "8"`.

## Phase 9: CLI 강화

**목표:** `bitvue` CLI가 VQ Analyzer CLI와 동등한 기능 제공. VQA의 단일 대시 플래그(`-md5`, `-regress`…)는 clap 관례에 따라
`--md5`, `--regress`, `--stream-stats`, `--dump-bitdepth`, `--display-order`, `--no-crop`, `--film-grain`, `--errors`, `--cpu-max-feature`
등 kebab-case long flag로 구현(`crates/bitvue-cli/src/main.rs`가 정본). `info/frames/analyze/export/validate`의 입력은 `-f/--file`.

- 주의 (2026-10-05 코드 기준, 상태는 phase 9 항목): YUV 덤프·PSNR은 AV1 전용; `--md5`는 압축 프레임 바이트 해시(디코드 출력 아님); `--fast`/`--no-crop`/`--cpu-max-feature`는
  파싱만 되고 효과 없음; `--vvc`/`--mpeg2`는 "not yet implemented".
- VVC `-ols`는 vvdec 연동 전까지 보류.

Items: features.yaml `phase: "9"`.

## Phase 10: 성능 최적화 & 대용량 스트림

**목표:** 4K/8K, 1GB+ 파일 안정 처리.

레퍼런스 수치 (from `_import_v14/.../performance/*.md`, 2026-07-31 마이닝 — 원문은 egui 전제라 "paint_ms/egui" 언급은 프레임워크 불일치, 개념·수치만 차용):

| 항목 | 구체 값 |
|---|---|
| 캐시 레벨 (레퍼런스 예산, 스트림당) | Decode cache 64 frame LRU · Texture cache 256MB · QP heatmap texture 128MB · Diff heatmap texture 128MB(A/B) · MV visible-list 32MB · Grid line cache 16MB |
| 캐시 축출 정책 | 가중치 LRU(메모리 바이트 기준), 사용량 80% 초과 시 공격적 축출, 축출 이벤트 perf HUD 로깅 |
| Fast-path/Quality-path 2단계 프리뷰 | Fast: 파일 열기·스크럽 중 Quarter/Half-res, 오버레이도 저해상도 강제(QP half-res, Diff quarter/half-res, MV stride 샘플링) — 목표 첫 프레임 표시 ≤60ms(1080p 기준). Quality: 입력 200ms 없을 때 트리거, 고품질 디코드→RGBA 변환→고해상도 오버레이 순 업그레이드, 사용자 입력 시 즉시 중단 |
| 필수 프로파일링 타이머 | open_file_total_ms, io_read_ms, mmap_setup_ms, parse_ms, index_build_ms, decode_ms, convert_ms, overlay_build_ms(오버레이별), upload_texture_ms, paint_ms, cache_hit_rate — Dev HUD 토글 + 세션별 export 가능한 perf report로 노출 |
| 자동 성능 저하 규칙 | paint_ms 평균(60프레임 롤링) 16ms 초과 2초 지속 → LOD 1단계 상승 + MV stride 2 이상 강제 / 33ms 초과 → Diff 오버레이 자동 비활성(토스트) + heatmap Quarter-res로 하향 / 캐시 사용량 80% 초과 → diff→qp→mv→grid 순으로 오버레이부터 축출 / 스크럽 중 → Quality-path 비활성, Diff 기본 off, MV는 항상 샘플링 |
| LOD 정책 (Timeline/Chart, `LOD_PERF_CACHE_SPEC.md`) | LOD0 가시프레임 2,000 이하(전체 포인트) / LOD1 2,000~20,000(버킷당 min/max 엔벨로프) / LOD2 20,000~200,000(분위수 엔벨로프+희소 마커) / LOD3 200,000 초과(밀도 히트밴드+키 마커만); 버킷 수 = min(2048, ceil(N/target)), target=1,200(라인)/2,400(바); 마커는 절대 드롭하지 않고 밀도 1/6px 초과일 때만 클러스터링 |
| 캐시 키 정규 포맷 | `<kind>:<stream>:<codec>:<file_hash>:<params_hash>` — file_hash = SHA1(첫 4MB)+파일크기; 예: `overlay_tile:A:AV1:<hash>:qp_map frame120 tile12,8 scale2` |
| 성능 예산 (60fps 기준) | 프레임 전체 16.7ms 중 렌더링 6ms 이하, 인터랙션 처리 2ms 이하, 레이아웃 2ms 이하 |
| 세부 예산 (from `parity_harness/perf_budget_and_instrumentation.json`) | hit-test ≤1.5ms, tooltip 빌드 ≤0.8ms, selection 전파 ≤2.0ms; 예산 초과 시 단계적 저하: 라벨 비활성화 → 벡터 집계 → 히트맵 다운샘플 → 사유 표시 placeholder |

- 현실(2026-10-05): perf 계측/예산/저하/Fast-path 모델은 `bitvue-engine`(`performance.rs`, `indexing.rs`)에 있으나 sidecar·UI에 미배선.
  세부 예산 수치는 `bitvue-engine/src/parity_harness/gates.rs`의 `PerfBudgets::default()`에 하드코딩돼 있다(인용된 json 파일은 리포에 없음).

Items: features.yaml `phase: "10"`.

## Phase 11: 폴리시, 단축키, 옵션

**목표:** 단축키 전체(구 `PARITY_CHECKLIST.md` Layer 4 표 → `docs/history/parity-checklist-log.md`, 상태는 features.yaml UX-058 등), Options 메뉴, 레이아웃 저장/복원, 최근 파일(최대 10), Status 패널, 접근성,
다크/라이트 테마, YUV Viewer 색공간(BT.601/709/2020)·endianness·raw 포맷. 라이선스 활성화는 오픈소스라 제외.

Items: features.yaml `phase: "11"`.

## Phase 12: 전체 Parity 검증 & 테스트

**목표:** 공개 테스트 비트스트림으로 모든 모드 검증, 자동 스크린샷 비교, 신택스 값 비교, 회귀 스위트.

- `scripts/parity_check.sh [--local]` + CI `parity-regression` 잡이 현재 게이트(대부분 exit-code/출력 grep 스모크).
- AV3/AVM 명명 감사(CMP-09 → `CODEC-006`) 결론: 구 `bitvue-av3-codec`은 AOM AV2/AVM 공식 스펙(`av2.aomedia.org`)이 아닌 합성 OBU 구조(AV1형 목록 + 존재하지 않는 "OverheadInfo" 타입)였고 이후 크레이트 삭제됨. "AV3"는
  존재하지 않으며 AVS3(IEEE 1857.10, `bitvue-avs3`)와 혼동 금지.

Items: features.yaml `phase: "12"`.

---

## Appendix: 구 Tauri 커맨드 목록 (폐기)

Tauri 시대 신규 커맨드 제안 목록은 폐기. 현재 정본은 `crates/bitvue-sidecar/src/request_dispatch.rs`. 대응: YUVDiff →
`load_debug_yuv`/`get_debug_yuv_frame`/`find_first_diff_frame`/`get_yuv_diff_metrics`; AV1 CDEF/film grain/LR → `get_av1_features`;
Stats/Ref → `get_codec_extended_info`. VVC/QM/VP9-prob/codec-modes 커맨드는 sidecar에 없음(해당 phase 항목에서 추적).

## Appendix: Architecture & Correctness Reference

이 계약들은 로드맵이 아니라 이미 `crates/bitvue-engine/src/`에 구현된 규칙이다(모듈 주석이 v14 spec 파일명 인용, `lib.rs`의 T0-1~T10-1
태그가 구현 순서 이력). 새 오버레이/패널 추가 시 어기지 않았는지 확인하는 용도. 단, 일부 모듈(worker, indexing QuickIndex,
coordinate_transform, cache_provenance)은 엔진에만 있고 sidecar/UI 런타임 경로에서는 쓰이지 않는다 — 상태는 features.yaml 참조.

| 계약 | 핵심 규칙 | 구현 위치 |
|---|---|---|
| Frame Identity | 기본 타임라인 인덱스 = Display order(PTS); decode_idx는 내부 전용; PTS/DTS mismatch는 전용 band로만 | `frame_identity/` |
| Coordinate System | `screen_px → video_rect_norm(0..1) → coded_px → block_idx` 고정; fit/zoom/pan은 screen→norm만 수정 | `coordinate_transform.rs` |
| Selection Precedence | Block > Point > Range > Marker, 한 번에 한 타입만 활성 | `selection.rs` |
| Cache Invalidation | 오버레이별 무효화 트리거; 프레임 변경 시 프레임 종속 오버레이 항상 무효화; 다른 frame_idx에 텍스처 재사용 금지 | `cache_provenance.rs`, `cache_validation.rs` |
| Async Backpressure | Latest-wins, 스트림당 in-flight ≤2, 스크럽 중 비-현재 작업 취소 + quality-path off | `worker.rs` |
| Indexing Strategy | Quick Index(키프레임/OBU 경계, 즉시 표시) → Full Index(백그라운드, 진행률); UI는 Full 대기 안 함 | `indexing.rs`, `index_session*.rs` |
| Tri-sync 권위 | `bitRange > syntaxNode > unit > frameIndex/pts > stream_id`; Hex→Syntax = 최소 포함 노드, tie는 최대 depth, 그다음 SyntaxNodeId 사전순 | `evidence.rs`, `player_evidence.rs`, `timeline_evidence.rs` |
| Error Model | Info/Warn/Error/Fatal; Diagnostic은 `offset_bytes` 필수; Fatal이어도 크래시 없이 Hex 검사 가능 | `diagnostics.rs`, `error.rs`, `app_error.rs` |
| File I/O | mmap 랜덤 액세스, 세그먼트 캐시(64~256KB), 파일 크기 변경 시 mmap 무효화 + WARN | `byte_cache.rs` (memmap2) |

Layout 참고 수치(from `LAYOUT_CONTRACT.md`/`LAYOUT_GRID_SYSTEM.md`/`RESPONSIVE_VISUALIZATION_RULES.md` — egui 5-region(R1 Toolbar/R2 Left/R3 Center/R4 Right/R5 StatusBar) 전제 문서라 리전 이름은 비적용, 미검증): splitter 최소 폭 320px/높이 140px, 타임라인 스트립
clamp(160px, 18vh, 260px), 툴팁 최대 360px/40vh, 바 너비 <2px면 LOD 버킷 렌더, 리사이즈 디바운스 필수. 적용 전
`frontend/components/panels/DockableLayout.tsx`와 대조할 것.

Checkable contracts: features.yaml (no phase; area infra/ux/perf, "contract" in title).

## Appendix: Future Differentiators (post-parity)

경쟁사 패리티가 아닌 v14 pack 자체 "Differentiators" 제안 — **패리티 백로그에 섞지 말 것.** 대부분 엔진에 데이터 모델은 있으나
sidecar 커맨드·UI가 없어 "신규 설계가 아니라 배선 작업"으로 정의된다.

| 기능 | 핵심 아이디어 | 엔진 쪽 기반 (2026-07-31 확인) | 남은 작업 |
|---|---|---|---|
| Insight Feed | 규칙/통계 기반 요약 카드(QP spike, metric dip, error burst, reorder mismatch, HRD risk, A/B regression), Jump/Filter/Export, 트리거 근거 표시 | `insight_feed.rs` (`InsightType` 등) | sidecar 커맨드 노출 + 카드 UI |
| Session Evidence | `.baxsession.json`(파일/레이아웃/선택/북마크) + 북마크를 증거 번들(스냅샷+수치 요약+딥링크)로 export, 버그리포트(md+이미지+csv) | `evidence.rs` (bit_offset/syntax/decode/viz 4-stage evidence chain) | 세션 직렬화 포맷 확정 + export 커맨드 |
| Compliance Scoreboard | timing/ref structure/HRD/metadata/syntax legality 카테고리별 점수 + 위반 목록(룰 id, 조건, 관측값, jump target) | 없음 — `parity_harness`의 `CategoryScore`는 경쟁사 패리티 채점용으로 무관 | 전체 신규 |
| Regression Guard (CI) | A/B metric_delta/error_burst/reorder_mismatch/HRD 조건으로 CI 게이트 규칙 → `regression_report.json`/`regression_summary.md` | 없음 — `parity_harness/`는 "Bitvue vs 경쟁툴 parity matrix" 채점기(스키마 검증/semantic probe/render snapshot/Hard-Fail·Parity·Perf 게이트)로 목적이 다름 | 전용 A/B 룰 엔진 + CI 잡; `parity_harness`를 `parity_check.sh`에 연결하는 것도 별개 작업 |

Explainability Hints (UI 문구 참고): QP "Auto scale: min/max from current frame" / "Fixed scale: 0..63"; MV "Vectors shown in px (qpel/4)";
Partition "Scaffold grid shown when partition data unavailable"; Diff "Abs diff: |A-B|" / "Signed diff: A-B"; Timeline "Markers never dropped; clustered when dense".

Onboarding First-5-Minutes (Help 메뉴 가이드 초안): ① Timeline에서 frame size/QP로 이상 구간 선택 → ② Player에서 QP/MV/Partition으로
공간적 원인 확인 → ③ Metrics 워크스페이스에서 구간 히스토그램 비교 → ④ Diagnostics 에러 버스트 선택 후 evidence export →
⑤(선택) Stream B 로드 후 Compare로 A/B 델타. Worst Frames/Regression Guard 제안은 Insight Feed/Regression Guard 선행 필요.

Items: features.yaml area `probe` (+ qa/ux/metrics), priority P3.

## Appendix: MCP 서버 실제 구현 대조

Bitvue에는 서로 무관한 두 MCP 구현이 있다.

| | spec (`MCP_INTERACTION_MODEL.md`) | `crates/bitvue-mcp` (`bitvue-mcp-server`) | `bitvue-engine/src/mcp.rs` (`McpIntegration`) |
|---|---|---|---|
| 모델 | Read-only resources 8종 + actions(제안/설명/초안 생성) 5종 | JSON-RPC stdio, MCP tools 10종: load_file/analyze_frame/get_qp_map/get_motion_vectors/compare_streams/get_gop_structure/find_decoding_issues/get_stream_info/search_syntax/list_files | spec resource 목록과 거의 동일(`selection_state/insight_feed/diagnostics/metrics_summary/timeline_lanes/compare/session_evidence/compliance`), `get_resource`/`list_resources` |
| 코덱 | 코덱 무관 | IVF/AV1만 (`parse_ivf_file`, `.ivf`/`.av1` 외 미지원 에러) | 엔진 모델 재사용(코덱 무관) |
| 관계 | — | `McpIntegration` 미사용, 자체 tool 세트 | 어떤 바이너리에서도 미사용(테스트만) |

통합 여부(또는 `McpIntegration`을 서버에 연결)는 미결 — Phase 12 이후 정리 대상. Tool별 상태: features.yaml area `mcp`.

---

See also: `docs/specs/features.yaml` (status source of truth) · `docs/history/development-log.md` (dated implementation logs).
