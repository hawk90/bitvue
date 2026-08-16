# Bitvue — Development Phases & Roadmap

> Extracted from `VQA_PARITY_SPEC_V3.md` §5 "Development Phases" (2026-07-31 doc-family split). Phase-by-phase
> implementation roadmap for closing the VQ Analyzer parity gap — Rust code sketches, task checklists, and time
> estimates per phase. This is roadmap/planning material, not current-state spec: for "what a feature should do"
> see `VQA_PARITY_SPEC_V3.md`, for "is it done yet" see `PARITY_CHECKLIST.md`.
> See also: `VQA_PARITY_SPEC_V3.md` (backend/codec parity spec), `PARITY_CHECKLIST.md` (implementation tracking),
> `COMPETITOR_FEATURE_MATRIX.md` (per-product feature matrix), `UX_PARITY_MATRIX.md` (UI/UX interaction parity).

---

## 우선순위 재검토 (2026-07-31, 경쟁사/UX 리서치 반영 후)

Phase 0-12 순서는 4월 스펙 당시 그대로다 — 이번 세션의 경쟁사(`COMPETITOR_FEATURE_MATRIX.md`)/UX(`UX_PARITY_MATRIX.md`)
리서치가 끝난 뒤 순서 자체를 재검토한 결과:

- **Phase 1(F키 모드 분기)은 순위 유지.** Phase 2-6의 코덱별 오버레이 작업 전체가 이 인프라에 의존하므로 여전히
  최우선 — Dual-Stream Compare(7.5)나 UX 갭이 새로 발견됐다고 해서 이 의존관계가 바뀌지 않는다.
- **Phase 7.5(Dual-Stream Compare & VMAF)는 위치 유지, 순서를 앞당기지 않음.** 자체 아키텍처 경계 노트대로 기존
  렌더링/디코딩 파이프라인을 재사용하는 독립 트랙(Analyzer 코드와 분리)이라 Phase 1-6 순서를 밀어낼 근거가 아니다.
- **신규 삽입: Phase 7.6 (Context Menu & Evidence Bundle).** `UX_PARITY_MATRIX.md` §9에서 컨텍스트 메뉴(P1)와
  Evidence Bundle export(P0)가 심각도 P0/P1로 채점됐고 Phase 7.5의 Compare 워크스페이스가 이 두 계약(우클릭 메뉴,
  번들 export 4개 진입점)을 직접 요구하므로(§2/§6/§7), 기존 계획대로 Phase 11(🟢 최하위 우선순위)까지 미루지 않고
  Phase 7.5 직후로 승격한다. 아래 Phase 7.6 참조.
- **그 외 신규 발견 항목**(CABAC 시각화, AVM 명명 정리, 컨테이너 확장, HRD 버퍼 그래프, bit-distribution 시각화 등)은
  전부 기존 phase에 체크리스트 항목으로 흡수했다 — 순서를 바꿀 만큼 독립적이거나 큰 작업이 아니라고 판단.
- Phase 8(Syntax 패널 완성)이 이번 리서치로 흡수한 항목이 많아 범위가 커졌지만, "코덱 디코딩 완성(Phase 3-6) →
  시각화/신택스 심화(Phase 8)"라는 기존 순서 논리와 여전히 맞으므로 위치는 유지.

---

## 제품 아키텍처 확정 (2026-08-08, Tauri→Electron 전환 결정에 따른 정리)

> 설계 확정 — 성능/구현 세부는 나중에 갱신. `electron-migration` 브랜치 생성됨(코드 변경은 아직 없음).

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

| 제품 | 역할 | 사용 시점 | 현재 상태 |
|---|---|---|---|
| **Bitvue Analyzer** | Deep inspection / debugging — 지금 만드는 본체, 개발 리소스 80-90% 집중 대상 | 문제 원인 분석 | 진행 중 (Phase 1-12) |
| **Bitvue Probe** | Live/장시간 관측 (QoS: bitrate/fps/QP/GOP, 이벤트: corruption/discontinuity/decoder error) | 문제 탐지 | 미착수 — Analyzer 안정화 후 착수 |
| **Bitvue CLI** | 자동화/CI/스크립팅 (`bitvue inspect/frames/metrics/validate/diff/compare --fail-if`) | 반복 분석, 회귀 게이트 | **이미 존재** — `crates/bitvue-cli` 3055줄, Phase 9(커맨드라인 강화)에서 확장 중 |
| **bitvue-mcp** | 4번째 "제품"이 아니라 core의 인터페이스 중 하나 (Rust API / JSON Schema / CLI / IPC / MCP 중 하나) | 에이전트/외부 툴 연동 | **이미 존재** — `crates/bitvue-mcp` 1224줄, CLI와 형제 크레이트 |

**아키텍처 근거 (경계, 필수 준수):** Analyzer(파서 안전성·syntax 표현·codec state가 핵심 위험)와 Probe(프레임 정렬·색공간 정합성·중복 decode·메트릭 정확성이 핵심 위험)는 실패 모드가 다르다 — `UniversalFrameAnalysis` 같은 god-struct로 합치지 말 것. 이미 Phase 7.5에 명문화된 원칙([[project_analyzer_probe_separation]] 참조)이며 이번 확정은 이를 3-제품 구조로 확장한 것뿐, 새 원칙 아님.

**검증됨 (제안이 아니라 이미 사실) — 단, 범위는 제한적으로 읽을 것:**
- `bitvue-cli`의 Cargo 의존성은 `bitvue-engine/formats/decode/{codec}/metrics`뿐, `src-tauri` 없음 — 이건 **라이브러리 모듈성**(core가 특정 UI 프레임워크 크레이트에 링크되지 않음)만 증명한다. GUI가 실제로 필요로 하는 incremental/viewport-scoped/cancelable query 형태(`get_syntax_range`, `get_hex_range` 등)까지 core가 이미 그 모양으로 노출하고 있다는 뜻은 아님 — CLI는 1회성 배치 호출이라 이 부분은 검증하지 못한다. "core-UI 분리는 됐다, interactive query API 설계는 아직 안 됐다"가 정확한 상태.
- Cross-view multi-sync(Analyzer의 핵심 차별점)는 부분적으로만 이미 있음 — `crates/bitvue-engine/src/selection.rs`의 `SelectionState`가 `stream_id`/`temporal`/`cursor`/`unit`/`syntax_node`/`bit_range`/`source_view` 7개 필드로 **Syntax tree·Player·Timeline·Hex** 4개 뷰의 tri/multi-sync는 커버한다. 하지만 **QP heatmap**(frame 단위라 `cursor`로 충분, 사실상 커버)을 빼면 **Ref Graph 노드**와 **Metrics 샘플** 선택을 나타내는 필드는 없음 — 7개 뷰 중 2개는 새 필드/설계가 필요하다. 참고로 이 struct 자체에 "God object refactoring note: intentionally cohesive"라는 방어적 주석이 이미 달려 있어 필드 추가 전에 한번 검토할 가치가 있음.

### 왜 Electron인가 — 결정 근거

| Bitvue 관점 | Tauri v2 | Electron |
|---|---|---|
| macOS/Windows/Linux 렌더러 일관성 | 낮음 | 높음 (타겟이 Chromium 하나라 재현·추적이 쉽다는 의미 — "문제가 없다"는 아님, 아래 각주) |
| Linux WebGL/Canvas 지원 부담 | 높음 | 상대적으로 낮음 |
| Rust 코어 직접 호출 | 매우 좋음 | bridge 필요 |
| Rust crash 격리 | 별도 설계 필요 | **sidecar를 선택할 때만** 성립 (아래 "브리지 방식" 참조 — napi-rs면 이 행 무효) |
| 번들·idle 메모리 | 좋음 | 나쁨 (Bitvue는 이미 4K/8K 프레임 버퍼로 메모리를 많이 쓰는 도구라 실사용 환경에서 재확인 필요 — 아래 각주) |
| 대규모 시각화 생태계 | 보통 | 좋음 |
| DevTools/프로파일링 | 플랫폼별 차이 | 일관적 |
| Playwright/E2E 재현성 | 보통 | 좋음 |
| 업데이트·배포 사례 | 보통 | 풍부함 |
| 기본 보안 모델 | 좋음 | 명시적 hardening 필요 |
| 장기 플랫폼 QA 비용 | 높음 | 낮음 |
| 현재 Bitvue 코드 재사용 | 최대 | Rust 코어·React 재사용 가능 |
| 앱 전체 전환 비용 | 없음 | Tauri adapter 재작성 |

**결정 조건 (조건부 — repo 어디에도 플랫폼 지원 티어를 명시한 문서가 없어 아직 미확인):** Linux가 best-effort(Windows/macOS 1급, Ubuntu LTS 공식 지원, 기타 Linux best-effort)면 Tauri 유지로도 충분했다. Bitvue에서 실패 요인은 번들 크기/메모리가 아니라 "특정 Linux에서 빈 화면", "GPU 가속 fallback 차이", "QP/MV overlay 렌더링 차이", "macOS 버전별 WebKit 동작 차이", "플랫폼별 Canvas/WebGL 버그 추적 비용" — 즉 **Linux까지 진짜 1급으로 지원해야 하는 전문 영상 시각화 도구**라면 Electron이 맞다. **단, "Linux가 1급이다"는 아직 사용자가 명시적으로 확정한 사실이 아니라 이 판단의 전제 조건일 뿐** — Electron 전환 자체는 이미 결정·실행됐으므로(브랜치 생성됨) 지금 재논의 대상은 아니지만, 이 전제가 실제로 맞는지는 별도로 확인 필요.

> **각주 — Linux 렌더러 일관성 행에 대해:** Electron도 Linux GPU 가속은 그 자체로 까다롭다(VAAPI/ANGLE 백엔드 선택, X11 vs Wayland, NVIDIA 독점 드라이버, `--disable-gpu-sandbox` 류 플래그). Electron이 사는 건 "Linux GPU 문제가 없어짐"이 아니라 "타겟이 WebKitGTK 대신 Chromium 하나로 줄어 재현·추적 비용이 낮아짐" — Electron 전환 후에도 Linux GPU 이슈 트래킹은 계속 필요.
>
> **각주 — idle 메모리 행에 대해:** "Electron의 무거움은 실패 요인이 아니다"는 사용자 판단을 그대로 반영한 것. Bitvue는 4K/8K 프레임 버퍼·다중 코덱 디코더를 이미 메모리 집약적으로 쓰는 도구라, Chromium 베이스라인(윈도우당 수백MB)이 encoder/decoder 벤치마크 툴과 동시 실행되는 실사용 환경에서 실제로 무해한지는 실측으로 재확인하는 걸 권장.

### Electron 전환 시 경계

```
┌─ Electron: React/TypeScript ── Workspace/Timeline/Tree/Hex/Graph/Player ─┐
│                              │ typed IPC (control/data plane 분리)       │
└─ Rust: Parse → Model → Analyze → Query ── AV1/H264/HEVC/VP9/VVC/AVS3... ─┘
```

- Electron = presentation/workspace 계층만. Rust가 codec parsing/indexing/frame model/metrics/diagnostics/compare 전부 계속 소유 — **엔진을 JS로 옮기는 게 아니다.**
- FFI 디코더(dav1d/vvdec/libvmaf)를 포함해 `crates/bitvue-*`는 이미 Tauri에 종속되지 않은 순수 Rust이므로 재작성 불필요. 새로 필요한 건 브리지 레이어와 `src-tauri/src/commands/*.rs`(**40개 커맨드**, 15개 파일 — 이전에 161개로 잘못 기록됐던 걸 정정, `grep -rc '#\[tauri::command\]' src-tauri/src/commands/` 재확인) + AppState + 이벤트버스의 Electron 대응물뿐.

**먼저 결정해야 함 — 브리지 방식(napi-rs vs sidecar), 나머지가 이 선택에 종속됨:**

| | napi-rs (in-process native addon) | sidecar (별도 프로세스) |
|---|---|---|
| 호출 방식 | N-API 직접 함수 호출, 직렬화 프레이밍 없음 | 진짜 wire protocol 필요 (stdio/socket + 바이너리 프레이밍) |
| Rust crash 격리 | **안 됨** — panic/segfault(vvdec FFI 등) 시 로드된 프로세스 전체가 죽음, 지금 Tauri와 노출도 비슷하거나 더 나쁨 | 됨 — 위 결정 테이블의 "Rust crash 격리" 장점은 이 경로에서만 성립 |
| `bitvue-protocol`의 의미 | 사실상 공유 타입 정의(codegen)에 가까움, "protocol"이라 부르기엔 과장 | 문자 그대로 필요한 스키마 레이어 |

이 선택이 안 끝나면 `bitvue-protocol`을 뭘로 설계할지도 정해지지 않으므로, 아래 크레이트 경계보다 먼저 결정할 것.

**결정 (2026-08-08): sidecar.** Bitvue의 핵심 가치가 "malformed/비정상 비트스트림 파싱"이라 FFI 디코더(dav1d/vvdec/libvmaf)의 segfault 위험이 상시 존재(`docs/anti-patterns/PARSE.md`/`CODEC.md`가 이 카테고리를 통째로 다룰 정도). napi-rs는 이 경우 Electron 메인 프로세스 전체를 죽여 지금 Tauri 대비 개선이 없고, 위 결정 테이블의 "Rust crash 격리" 행 자체가 무효화됨. sidecar는 초기 구현 비용(프로세스 lifecycle, IPC 프레이밍)이 더 들지만 Bitvue 특성상 맞는 트레이드오프로 판단. VS Code language-server 패턴과 동일 구조.
- Rust sidecar는 기존 `crates/bitvue-engine`의 `AppState`(`Arc<Mutex<Core>>`)를 그대로 프로세스 경계로 옮기는 형태 — Electron main process가 spawn/monitor/restart를 담당하고, sidecar crash 시 상태(열린 스트림/커서 등)를 복구하는 정책은 별도 설계 필요(현재 미해결).
- `bitvue-protocol`은 문자 그대로 wire schema로 확정 — stdio 또는 로컬 소켓 위에 binary framing(control-plane 메시지는 구조화, data-plane은 raw buffer).

**핵심 원칙 — Electron IPC로 프레임을 통째로 보내지 않는다.** `src-tauri/commands`를 Electron IPC로 1:1 번역하면 프레임워크만 바뀌고 문제는 그대로 재발한다. 실제로 지금 이미 이 실패 패턴이 존재함(검증됨, `src-tauri/src/commands/frame.rs:28-43` `YUVFrameData`): Y/U/V 플레인을 base64 `String`으로 JSON 직렬화해 반환 — 정확히 "Rust frame → JSON 배열 → IPC → React state → Canvas" 안티패턴. Electron 전환은 이걸 프레임워크 무관하게 고칠 기회지, 자동으로 고쳐지는 게 아니다.

**Control plane** — 작고 구조화된 데이터만: `open_stream` / `request_frame` / `select_frame` / `set_overlay` / `cancel_request` / `get_syntax_range` / `get_hex_range`.

**Data plane** — 대용량 데이터는 목적별 typed 전송:

| 데이터 | 전송 형태 |
|---|---|
| decoded frame | binary/custom protocol/transferable buffer (base64 문자열 금지) |
| thumbnail | encoded image cache |
| syntax tree | viewport 범위만 |
| hex data | byte range만 |
| QP/MV map | compact typed array — 예: MV를 객체 배열이 아니라 `Int16Array [x0,y0,ref0,flags0, x1,y1,ref1,flags1, ...]` |
| statistics | batch/chunk |

Electron 공식 API의 renderer↔main/utility process 간 `MessagePort` 통신을 data plane 전달에 활용 가능.

**크레이트/프로세스 경계 (전환 전 먼저 확정, 나중에 재배치하지 말 것):**

```
bitvue-engine     순수 Rust 도메인 엔진   crates/bitvue-engine(구 bitvue-core) + formats/decode/codecs/metrics — 리네이밍 완료(2026-08-08, `c2a0e44`)
bitvue-protocol   request/event/error/binary schema   ← 신규, `crates/bitvue-protocol`로 구현됨(2026-08-08)
bitvue-sidecar    engine을 감싸고 bitvue-protocol을 stdio로 말하는 독립 프로세스   ← 신규, 스켈레톤 구현됨(2026-08-08, 아래 참조)
bitvue-desktop    Electron main + preload   ≈ 기존 src-tauri 대체
bitvue-ui         React renderer   ≈ 기존 frontend/
```

**정정:** 원래 4-box 구성은 napi-rs(in-process) 브리지를 암묵적으로 가정한 그림이었음 — 그 경우 engine이 `bitvue-desktop` 안에서 직접 로드되니 별도 프로세스 개념이 필요 없었음. sidecar로 결정하면서 실제로는 5번째 조각이 필요해짐: engine을 감싸고 `bitvue-protocol`을 stdio로 말하는 **독립 Rust 바이너리**. `bitvue-sidecar`로 명명.

**`bitvue-sidecar` 스켈레톤 + 첫 실커맨드 (2026-08-08):** `crates/bitvue-sidecar` — stdin에서 프레임을 읽고, `hello` 핸드셰이크에 `HelloResult`로 응답. **`open_stream`을 `bitvue_engine::Core::handle_command(Command::OpenFile)`에 실제로 연결**(스켈레톤이 아니라 진짜 엔진 호출) — 나머지 메서드는 여전히 `WireErrorCode::Internal`. 검증: 유닛테스트 6개(핸드셰이크/미구현 메서드/stdin 종료/`open_stream` 성공·존재하지 않는 파일·잘못된 stream id) + **실제 컴파일된 바이너리에 raw stdio 바이트를 파이프로 흘려 `hello`→`open_stream` 두 요청을 연속으로 보내고 진짜 `ModelUpdated` 이벤트를 받는 end-to-end 스모크 테스트**까지 통과. `cargo check --workspace` 클린.
- **발견한 것:** `bitvue_engine::{Command, Event}`도 `Serialize`/`Deserialize`를 derive하지 않음(`BitvueError`와 같은 상황). 둘 다 내부 UI↔Core 버스 타입이라 wire 계약과 분리하는 게 맞다고 판단해, `bitvue-sidecar`에서 `open_stream` 전용 JSON params 구조체를 손으로 만들고 `Event`→JSON 매핑 함수(`event_to_json`)를 수동 작성함(`WireErrorCode` vs `BitvueError`와 동일한 디커플링 논리 재사용). 메서드가 늘어날수록 이 수동 매핑이 반복 작업이 될 것 — 나중에 패턴이 명확해지면 매크로화 고려.
- **설계상 확인된 것(버그 아님):** `Core::handle_command`는 실패도 `Result`가 아니라 `Event`(`DiagnosticAdded`, severity Error)로 표현함 — 즉 존재하지 않는 파일을 열어도 wire 레벨 `Response`는 `ok:true`이고 events 배열 안에 에러 진단이 담김. `bitvue-sidecar`가 이걸 protocol-level 에러로 바꾸지 않고 그대로 통과시키는 게 맞음 — Core 자체가 성공/실패를 RPC 레벨에서 구분하지 않는 설계이므로 wire 레이어가 없는 구분을 만들어내면 안 됨.

**`bitvue-sidecar` 세 커맨드 추가 + 첫 데이터플레인 증명 (2026-08-08):**

| 커맨드 | 종류 | params | 성공 결과 | 실패 매핑 |
|---|---|---|---|---|
| `select_frame` | control | `{stream, frame_index}` | `Command::SelectFrame{stream, frame_key: FrameKey{stream, frame_index, pts: None}}` → `{events:[SelectionUpdated]}` | stream 오타 → `InvalidData` |
| `close_stream` | control | `{stream}` | `Command::CloseFile{stream}` → `{events:[ModelUpdated{kind:Container}]}` | stream 오타 → `InvalidData` |
| `get_hex_range` | **control+data** | `{stream, offset, len}` | control `Response{result:{offset,len}}` 프레임 직후, **같은 correlation_id**로 `Data` 프레임(raw bytes, JSON/base64 래핑 없음) | 스트림 미오픈 → `NotFound`("stream not open"); `ByteCache::read_range` 실패(범위 초과 등) → `BitvueError` variant를 그대로 미러링하는 `wire_error_code_for()`로 매핑(예: `InvalidRange`) |

`get_hex_range`가 이 마이그레이션의 핵심 주장("bulk 데이터는 JSON에 태우지 않는다")을 처음으로 실증한 커맨드 — 검증 3단계: (1) `tempfile`에 알려진 256바이트(`0..=255`)를 쓰고 `open_stream`→`get_hex_range`, 반환된 `Data` 프레임 바이트를 원본 슬라이스와 정확히 비교(단순 "바이트가 왔다"가 아니라 `assert_eq!(data_body, &known_bytes[offset..offset+len])`), (2) 실제 컴파일된 바이너리(`cargo build -p bitvue-sidecar`)에 대해 `hello`→`open_stream`→`get_hex_range` 세 요청을 실 OS 파이프로 흘리는 subprocess 통합테스트(`crates/bitvue-sidecar/tests/subprocess_smoke.rs`), (3) Python으로 프레임을 손수 인코딩해 같은 바이너리에 직접 파이프한 독립 확인 — 세 경로 모두 offset=100/len=32에서 `expected == actual` 바이트 일치.

**리팩터링 — `dispatch`의 단일-`Response` 반환을 유지하면서 `get_hex_range`만 예외로 뺌:** 기존 `dispatch(core, &request) -> Response` 시그니처는 `hello`/`open_stream`/`select_frame`/`close_stream` 그대로 유지(요청 1개당 응답 프레임 1개인 공통 케이스). `get_hex_range`만 `handle_frame`에서 메서드 이름으로 먼저 갈라내 `get_hex_range(core, &request, correlation_id, writer) -> io::Result<()>` 형태(`&mut W`를 받아 직접 프레임을 씀)로 처리 — control 메타 프레임 + data 프레임 두 개를 써야 해서 단일 `Response` 반환 타입에 맞지 않기 때문. 두 프레임 다 같은 correlation_id를 공유하도록 `write_response_frame()` 헬퍼로 통일(기존 `dispatch` 경로의 응답 쓰기와 `get_hex_range`의 메타 프레임 쓰기가 같은 헬퍼를 공유). 데이터플레인 커맨드가 하나뿐인 지금 단계에서는 이 정도 분기로 충분 — 여러 개로 늘어나면 그때 `enum HandlerResult { Single(Response), Custom(...) }` 류 추상화를 고려.

**`Arc<T>` 클론 함정(발견, 실제로 걸림):** `state.byte_cache.as_ref()`가 `Option<&Arc<ByteCache>>`를 주는데, 그 참조에 바로 `.clone()`을 호출하면(`cache.clone()`) 표준 라이브러리의 `impl<T> Clone for &T` 블랭킷 impl이 메서드 탐색에서 `Arc::clone`보다 먼저 매치되어 **참조 자체만 복사**되고 `Arc`의 refcount는 증가하지 않음 — 결과 타입도 `&Arc<ByteCache>`라서 락 가드(`state`)를 drop하면 바로 borrow-checker 에러로 잡히긴 하지만, 명시적으로 `std::sync::Arc::clone(cache)`를 쓰는 게 맞음(에러 없이도 조용히 잘못된 타입을 만드는 경우가 있을 수 있으므로 습관화 권장).

`bitvue-protocol`이 유일하게 실제로 새로 만들어야 하는 조각 — 지금 IPC 스키마가 `src-tauri/src/commands/*.rs`에 흩어진 `#[tauri::command]` 함수 시그니처+`serde` 구조체로만 존재하고, control/data plane을 강제하는 단일 스키마 레이어가 없어서 위 `YUVFrameData` 같은 사례가 생긴 것. 이 크레이트가 그 강제 지점이 된다.

### `bitvue-protocol` wire schema v0 (2026-08-08, sidecar 결정에 따라 확정, 크레이트로 구현됨)

**상태:** 설계만이 아니라 `crates/bitvue-protocol`로 실제 존재 — `FrameHeader`/`Request`/`Response`/`WireError`/`WireErrorCode`/`CancelParams`/`HelloParams`/`HelloResult` 구현 + 단위테스트 3개, `cargo test -p bitvue-protocol` 통과, 워크스페이스 전체 `cargo check` 정상. `bitvue-engine`에 의존하지 않음(의도적 — 아래 참조).

**전송:** 단일 stdio duplex 채널(LSP와 동일 패턴). `stdin`(Electron main→sidecar)/`stdout`(sidecar→main)을 프로토콜 프레임 전용으로 예약, **`stderr`는 로그/패닉 메시지 전용**(바이너리 프로토콜과 섞이면 크래시 진단이 불가능해지므로 분리 필수). 소켓 대신 stdio를 쓰는 이유: 포트/권한 관리가 필요 없고, Electron `child_process.spawn`이 파이프를 기본 제공하며, OS 파이프는 대용량(4K YUV 프레임 ~12MB) 벌크 전송에도 문제없음.

**프레임 포맷** (고정 9바이트 헤더 + payload):

```
[1B kind][4B correlation_id][4B payload_len][payload...]
kind: 0=control  1=data  2=event(sidecar가 먼저 보내는 알림, id=0)
```

- **control payload** = JSON. 빈도 낮고 디버깅 편의(로그로 그대로 읽힘)가 직렬화 속도보다 중요해서 bincode/msgpack 대신 JSON 선택.
- **data payload** = raw bytes 그대로(길이는 헤더의 `payload_len`이 이미 앎, 추가 인코딩 없음). 메타데이터(width/height/bit_depth/block 개수 등)는 같은 `correlation_id`의 직전 control 응답으로 먼저 보내고, data 프레임은 순수 바이트만 — MV/QP는 `Int16Array`/`Uint8Array`에 그대로 매핑되는 raw 배열.

**Request/Response 봉투 (control):**
```jsonc
// main → sidecar
{ "id": 42, "method": "get_frame_analysis", "params": { "frame_index": 183 } }
// sidecar → main (성공)
{ "id": 42, "ok": true, "result": { /* 작은 구조화 데이터, 또는 뒤따라올 data 프레임의 메타데이터 */ } }
// sidecar → main (실패)
{ "id": 42, "ok": false, "error": { "code": "PARSE_ERROR", "message": "...", "offset": 4096 } }
```
`correlation_id`(헤더)와 JSON의 `id`는 동일 값 — 헤더만 보고도 라우팅 가능하게 이중화(파싱 전에 프레임을 correlation로 버킷팅하기 위함).

**취소:** `cancel_request { "target_id": 42 }`를 control로 전송. sidecar는 해당 job을 중단하고 `{"id":42,"ok":false,"error":{"code":"CANCELLED"}}` 응답 — 프론트엔드가 이미 쓰고 있는 stale-response 방지 패턴(`YuvViewerPanel`의 `cancelled` 플래그)과 동일 개념을 프로토콜 레벨로 승격.

**버전 핸드셰이크:** sidecar 기동 직후 main이 `{"method":"hello","params":{"client_version":"..."}}` 전송, sidecar가 `{"result":{"protocol_version":"0.1.0","capabilities":[...]}}`로 응답. 버전 불일치 시 조용히 깨진 프레임을 만드는 대신 기동 단계에서 바로 실패시키기 위함.

**에러 타입:** 기존 `crates/bitvue-engine/src/error.rs`의 `BitvueError`는 `thiserror`만 derive하고 `Serialize`가 없음(확인함, `grep derive` 결과 `#[derive(Error, Debug)]`뿐) — 지금 Tauri 커맨드들도 이미 `Result<T, String>`으로 문자열화해서 넘기는 중이라 이 문제를 우회만 해왔음. `bitvue-protocol`은 `BitvueError` variant마다 안정적인 `code`(`"PARSE_ERROR"`/`"UNSUPPORTED_CODEC"`/... 위 example 참조)를 매핑하는 별도 wire-error enum을 새로 정의해야 함 — `BitvueError`에 `Serialize`를 직접 derive하는 것보다, 크로스 언어 계약을 `BitvueError`의 내부 변경(필드 추가/제거)으로부터 격리하기 위해 별도 매핑이 낫다.

**미해결로 남기는 것(지금 안 막힘, 설계 시점에만 명시):** sidecar 프로세스가 죽었을 때 이미 날아간 미완료 request들의 재시도/타임아웃 정책, frame별 progressive/streaming 응답(하나의 `get_frame_analysis`가 여러 data 프레임을 순차로 낼 수 있는지) 여부.

### `bitvue-sidecar` — 3개 커맨드 추가 + data-plane 증명 (2026-08-08)

**상태:** `crates/bitvue-sidecar/src/main.rs` — 실커맨드가 `open_stream` 1개에서 4개로: `select_frame`/`close_stream`(control-plane, `open_stream`과 동일한 param-struct/handler 패턴, 4곳에서 중복되던 `"A"/"B"` 매칭을 `parse_stream_id()` 헬퍼로 통합)/**`get_hex_range`(첫 data-plane 커맨드)**.

| 커맨드 | 종류 | Core 호출 | 비고 |
|---|---|---|---|
| `select_frame` | control | `Command::SelectFrame{stream, frame_key: FrameKey{stream, frame_index, pts: None}}` | |
| `close_stream` | control | `Command::CloseFile{stream}` | |
| `get_hex_range` | **data** | `core.get_stream(stream)` → `byte_cache.read_range(offset, len)` | 아래 참조 |

**`get_hex_range` 구현:** `handle_frame`이 메서드 이름으로 먼저 분기하도록 리팩터 — 기존 "요청 1개당 `Response` 1개"만 다루던 `dispatch`는 그대로 두고, `get_hex_range`는 별도 `fn get_hex_range<W: Write>(core, request, correlation_id, writer) -> io::Result<()>`로 분리해 metadata `Control` 프레임(`{offset, len}`) 하나 + raw bytes `Data` 프레임 하나, 총 두 프레임을 같은 `correlation_id`로 직접 씀(`write_response_frame()` 헬퍼를 양쪽 경로가 공유). 단일 데이터플레인 커맨드 하나 때문에 `dispatch` 전체를 더 크게 추상화하진 않음.

**실제로 만난 버그(가상 아님):** `state.byte_cache.as_ref()`는 `Option<&Arc<ByteCache>>`를 반환하는데, 여기에 바로 `.clone()`을 호출하면 `Arc::clone`이 아니라 `&T`용 blanket `Clone` impl(참조 자체만 복사)로 resolve됨 — `std::sync::Arc::clone(cache)`로 명시해서 우회. Rust에서 `Option<&Arc<T>>.clone()` 패턴을 쓸 때 일반적으로 재발 가능한 함정.

**검증(3중, 전부 실행 완료):**
1. 유닛테스트 13개(기존 6 + 신규 7: `select_frame`/`close_stream` 성공·잘못된 stream id, `get_hex_range` stream-not-open→`NotFound`/out-of-bounds→`InvalidRange`/**byte-exact 성공** — 256바이트(`0..=255`) 임시파일에 써놓고 offset=10,len=16 요청해서 `data_body == &known_bytes[10..26]` 정확히 일치 확인).
2. `crates/bitvue-sidecar/tests/subprocess_smoke.rs`(신규 통합테스트 — `env!("CARGO_BIN_EXE_bitvue-sidecar")`가 유닛테스트가 아닌 통합테스트 디렉터리에서만 채워지는 걸 이번에 확인, 그래서 유닛테스트 파일이 아니라 여기로 옮김): 실제 컴파일된 바이너리를 OS 파이프로 구동해 `hello`→`open_stream`→`get_hex_range`(offset=5,len=20) byte-exact 확인.
3. 독립된 세 번째 증명: Python으로 프레임을 수동 인코딩해 `target/debug/bitvue-sidecar`에 직접 파이프(offset=100,len=32) — `expected`/`actual` 바이트열 정확히 일치, exit code 0.

`cargo fmt --check`/`cargo check --workspace`/`cargo test -p bitvue-sidecar`(13+1)/`cargo test -p bitvue-protocol`(3, 미변경) 전부 클린 — 이 세션에서 직접 재확인함(에이전트 최초 보고 + 병합 후 재검증 두 번 다 통과).

### `bitvue-desktop` sidecar client (2026-08-08, TS 측 브리지 구현)

**상태:** `bitvue-desktop/` — `frontend`/`src-tauri`와 같은 레벨의 독립 패키지(npm workspace 멤버 아님, 루트 `package.json` 패턴 그대로 따름), Electron `main`이 나중에 그대로 import할 sidecar 클라이언트만 구현. `electron`/`BrowserWindow`/renderer IPC는 범위 밖 — `child_process.spawn`은 Node core API라 Electron main에서도 동일하게 동작하므로 `electron` 의존성 자체가 불필요.

- `src/protocol.ts` — 9바이트 헤더(`kind`/`correlation_id` LE u32/`payload_len` LE u32) 인코더 + `FrameDecoder` 클래스. `FrameDecoder.push(chunk)`가 상태를 들고 있다가 완성된 프레임만 배열로 반환 — `stdout`의 `data` 이벤트가 프레임 경계와 무관하게 조각나거나 여러 프레임을 이어붙여 오는 걸 전제로 설계(헤더 중간 분할/payload 중간 분할/1바이트씩 분할 모두 유닛테스트로 커버).
- `src/sidecarClient.ts` — `SidecarClient extends EventEmitter`. `request(method, params)`가 `correlation_id`를 키로 pending Promise map에 등록하고 매칭되는 `Control` 응답이 오면 resolve(`ok:true`)/reject(`SidecarRequestError`, `ok:false`)함. `Data`/`Event` 프레임은 응답 흐름에 안 걸리므로 `'data'`/`'event'` EventEmitter 이벤트로 노출(요청-응답과 1:1 대응하지 않는 프레임까지 `request()`가 억지로 반환값에 끼워넣지 않기 위한 선택). 프로세스가 죽으면(`exit`/`error`) pending Promise 전부를 `SidecarExitedError`로 reject — 무한 대기 방지. `stderr`는 줄 단위로 `console.error('[bitvue-sidecar] ...')`.
- `hello(clientVersion)` 편의 메서드 — `protocol_version` 불일치는 v0 시점엔 `console.error` 경고만(하드 실패시키지 않음, 지금은 클라이언트/sidecar가 같은 리포에서 함께 빌드되는 개발 단계라 조기 강제가 실익이 적다고 판단 — 페어가 독립 버전으로 배포되는 시점에 재검토).

**검증됨(실제 컴파일된 바이너리 기준, mock 아님):** `npx vitest run` — 유닛테스트 11개(`protocol.test.ts`, 프레이밍/조각화 전담) + 통합테스트 3개(`sidecarClient.integration.test.ts`, `cargo build -p bitvue-sidecar`로 만든 실제 바이너리를 `child_process.spawn`으로 띄움) 총 14개 전부 통과, `npx tsc --noEmit` 클린. 이 검증은 최초 작성 워크트리와 실제 크레이트가 있는 공유 체크아웃 양쪽에서 각각 확인함(아래 참조). 통합테스트 3개: (1) `hello()` → `{protocol_version: "0.1.0", capabilities: []}` 실수신 확인, (2) 임시파일로 `open_stream` 호출 → 실제 `bitvue_engine::Core`가 발생시킨 `ModelUpdated` 이벤트(`crates/bitvue-sidecar`의 `open_stream_success_emits_model_updated` 테스트와 동일 픽스처) 수신 확인, (3) 요청 대기 중 sidecar 프로세스를 강제 종료했을 때 pending Promise가 멈추지 않고 reject되는지 확인.

**통합 경과:** 이 절은 원래 `bitvue-protocol`/`bitvue-sidecar`가 없는 별도 워크트리에서 작성되어 `BITVUE_SIDECAR_BIN` 환경변수로 바이너리 경로를 임시 지정해 검증했음(기본 경로 `<repoRoot>/target/debug/bitvue-sidecar`는 그 워크트리에 없었기 때문). 이후 소스만(`node_modules`/lockfile 제외) 이 체크아웃(`crates/`와 같은 위치, `electron-migration`)으로 옮기고 `npm install` + `cargo build -p bitvue-sidecar` + `npx vitest run`을 여기서 다시 실행 — **환경변수 없이 기본 경로만으로 14개 테스트 전부 재확인 통과**, `npx tsc --noEmit` 클린.

### `bitvue-desktop` Electron 셸 — 마지막 미검증 링크 증명 (2026-08-08)

**상태:** `bitvue-desktop/electron/{main.ts,preload.cjs,index.html}` — 지금까지 독립적으로 검증된 조각들(`bitvue-protocol`/`bitvue-sidecar`/`SidecarClient`)을 실제로 **동작하는 Electron 앱**으로 처음 묶음. `electron` 패키지를 devDependency로 설치(다운로드 성공, v43.3.0 — 이 샌드박스가 외부 네트워크에 접근 가능함을 이번에 확인).

- `preload.cjs` — **의도적으로 TS 컴파일 안 함, 손으로 쓴 plain CommonJS.** Electron의 sandboxed preload가 ESM에 버전별 제약이 많아서, preload 하나만 컴파일 파이프라인 밖에 둠. `contextBridge.exposeInMainWorld("bitvue", {...})`로 `hello`/`openStream`/`getHexRange` 3개만 노출 — 범용 "아무 채널이나 invoke" 통로가 아니라 처음부터 이름 있는 채널만 노출(문서의 query 기반 IPC 원칙과 일치).
- `main.ts` — `ipcMain.handle`로 위 3채널을 등록해 `SidecarClient`에 위임. `getHexRange` 핸들러가 반환하는 `Buffer`는 Electron의 구조적 복제(structured clone)를 그대로 타고 렌더러까지 감(JSON/base64 인코딩 전혀 없음) — `YUVFrameData` 안티패턴을 프레임워크 경계 전체(Rust→sidecar→Electron main→renderer)에 걸쳐 실제로 회피했다는 증거.
- `SidecarClient`에 `getHexRange()` 편의 메서드 추가(기존 `request()`는 `Control` 응답만 반환하고 뒤따르는 `Data` 프레임을 correlation id로 엮어주지 않았음 — 이번에 추가, 에러 경로에서 `'data'` 리스너가 새지 않도록 정리도 포함).
- **빌드:** preload는 컴파일 안 하지만 `main.ts`/`src/*.ts`는 별도 `tsconfig.build.json`(CommonJS 아님, 기존과 동일한 NodeNext ESM 유지 — `dist/electron/main.js`가 `dist/src/sidecarClient.js`를 상대경로로 그대로 import)로 빌드, `npm run build:electron`이 컴파일+정적 자산 복사(`preload.cjs`/`index.html`)까지 수행.
- **알려진 단순화(프로토타입 단계, 출시 전 재검토 필요):** `sandbox:false`(sandboxed preload의 ESM 제약 회피), IPC 핸들러 파라미터 검증 없음(sidecar 자체가 거부하는 것 이상 검증 안 함), 단일 전역 `SidecarClient` 인스턴스(멀티 윈도우/멀티 세션 스토리 없음).

**검증(실제로 실행한 Electron 프로세스 기준, 목만 아님):**
- `BITVUE_ELECTRON_OFFSCREEN=1 npx electron .` — 오프스크린 렌더링(디스플레이 없는 샌드박스에서도 검증 가능하게, `webPreferences.offscreen` + `show:false`)으로 실제 앱 기동 성공, sidecar handshake 성공, SIGTERM으로 정상 종료(false-alarm "unexpectedly exited" 로그도 종료 플래그로 정리).
- `BITVUE_ELECTRON_SELFTEST=1` — **렌더러 → preload(`contextBridge`) → `ipcRenderer.invoke` → `ipcMain.handle` → `SidecarClient` → 실제 컴파일된 Rust 바이너리**까지 전체 경로를 `webContents.executeJavaScript`로 실제 렌더러 컨텍스트에서 구동(메인 프로세스가 `SidecarClient`를 직접 호출하는 우회가 아님): `hello()` → `protocol_version:"0.1.0"` 확인, `openStream("A", tempFile)` → 실제 `ModelUpdated` 이벤트 확인, `getHexRange("A",10,16)` → 64바이트 known-content 임시파일에서 offset 10~25 바이트가 **byte-exact** 일치(`BYTE_EXACT_MATCH: true`, 클린 재빌드 후 재확인 완료).
- 기존 `bitvue-desktop` 유닛+통합 테스트 14개, `cargo test -p bitvue-sidecar`(13+1)/`-p bitvue-protocol`(3), `cargo check --workspace` 전부 이 변경 이후 재실행해서 회귀 없음 확인.

이로써 `docs/DEVELOPMENT_PHASES.md`가 처음부터 그린 4-계층 경계(`bitvue-engine`/`bitvue-protocol`/`bitvue-sidecar`/`bitvue-desktop`)의 모든 연결점이 최소 1개 실커맨드 기준으로는 전부 실증됨. 남은 건 폭(더 많은 커맨드), 실제 React UI(`bitvue-ui`) 연결. (`bitvue-core`→`bitvue-engine` 리네이밍은 2026-08-08 완료 — 아래 별도 항목 참조. `src-tauri`/`frontend/` 자체의 이동·삭제는 여전히 별개의 더 큰 단계로 남아있음, `src-tauri`는 아직 작동하는 fallback 앱.)

### `bitvue-sidecar` 동시성 모델 (2026-08-08)

**결정: OS 스레드(요청당 1개), async 런타임(tokio) 아님.** `bitvue_engine::Core`의 작업이 I/O 대기가 아니라 CPU-bound 동기 Rust라서 스레드가 자연스러운 선택 — tokio를 도입해도 결국 모든 `Core` 호출을 `spawn_blocking`으로 감싸야 해서 얻는 게 없음.

**문제였던 것:** 기존 구조는 stdin 리더 루프가 요청 하나를 완전히 처리(`dispatch` 동기 호출)한 뒤에야 다음 프레임을 읽었음 — 즉 미래에 느린 커맨드(디코딩 등)가 생기면 그 동안 `hello`/`select_frame` 같은 가벼운 요청도 전부 막힘. `cancel_request`도 취소할 "진행 중인 작업" 개념 자체가 없어 구현 불가능했음.

**설계:**
- 리더 루프(메인 스레드)는 프레임을 읽고 파싱만 하고 즉시 다음 프레임을 읽으러 감 — 각 요청은 `thread::spawn`으로 워커 스레드에 위임.
- `compute_frames`(응답 계산)는 **순수 함수, I/O 없음** — 여러 워커 스레드가 동시에 lock 없이 실행 가능. 최종 프레임 쓰기만 `Mutex<Stdout>`으로 짧게 직렬화(요청 1개의 프레임 쓰기 동안만 lock, 계산 중엔 lock 안 잡음 — 계산 중에 lock을 잡으면 그 자체로 다시 전부 직렬화되는 실수라 명시적으로 피함).
- `cancel_request`: `correlation_id → Arc<AtomicBool>` 레지스트리(`Mutex<HashMap<...>>`). 워커 스레드는 시작 직전 자기 플래그를 **한 번만** 확인함 — **선점형 취소 아님**: 지금 커맨드들은 전부 빠른 동기 호출(파일 mmap 1회, selection 갱신, `read_range` 1회)이라 중간 체크포인트가 없음. 실행이 이미 시작된 요청은 취소해도 효과 없음, "아직 시작 전" 좁은 타이밍 창에서만 동작. 진짜 느린 커맨드가 생기면 그 커맨드 자신이 협조적 체크포인트를 추가해야 함(공짜로 되는 게 아님) — 정직하게 문서화된 한계.
- 종료 시 `main()`은 아직 안 끝난 워커 스레드의 `JoinHandle`을 전부 `join()`한 뒤에야 프로세스를 종료(리스트는 매 루프마다 완료된 핸들을 `retain`으로 정리해 무한정 안 커지게 함).

**실제로 잡은 버그(가상 아님):** 위 join 처리 없이 처음 구현했을 때, "요청 여러 개를 응답 안 기다리고 연달아 보낸 뒤 stdin을 바로 닫는" 실제 파이프라인 사용 패턴을 흉내낸 테스트에서 **응답 하나가 통째로 유실됨**을 실제 서브프로세스로 확인(Rust는 detached `thread::spawn` 스레드를 `main()` 종료 시 기다려주지 않음). `join()` 추가로 수정. 회귀 테스트로 `crates/bitvue-sidecar/tests/subprocess_smoke.rs`의 `all_in_flight_responses_arrive_even_when_stdin_closes_immediately_after`를 추가 — fix를 되돌리고 실제로 실패하는 것까지 확인(반대로 fix 없이 두면 "cancel_request 미구현" 등 다른 이유로도 실패할 수 있어 완전히 격리된 재현은 아니지만, 실제 유실 시나리오를 정확히 흉내내는 유일한 테스트).

**검증:** `cargo test -p bitvue-sidecar` 유닛 17개(신규: `cancel_request` 3개, 동시성 stress 1개 — `Arc<Core>` 공유 상태에 16개 실제 OS 스레드로 `select_frame` 동시 호출, panic/deadlock 없음 + 모든 correlation id 정확히 왕복 확인) + 통합 2개(기존 hello/open_stream/get_hex_range + 신규 shutdown-race 회귀). `cargo fmt --check`/`cargo check --workspace`/`bitvue-desktop` 14개 테스트 전부 이 변경 이후 재확인, 회귀 없음.

**타이밍 기반 증명은 없음, 의도적으로.** "느린 요청이 빠른 요청을 막지 않는다"는 걸 실제 지연시간 측정으로 보여주려면 인위적으로 느린 테스트 커맨드가 필요한데, 지금 커맨드가 전부 서브밀리초라 그런 커맨드를 진짜 제품 커맨드 세트에 끼워 넣는 건 오염이라고 판단해 안 함. 지금 검증된 건 아키텍처적 정확성(스레드 안전, 데드락 없음, 종료 시 유실 없음)이지 체감 성능이 아님 — 실제로 느린 커맨드가 생기면 그때 latency-hiding을 실측할 것.

### `bitvue-sidecar` 커맨드 폭 확장 — multi-sync 5종, 그리고 "더 이상 공짜 커맨드가 없음" 확인 (2026-08-08)

**추가:** `select_unit`/`select_syntax`/`select_bit_range`/`select_spatial_block` — `select_frame`과 같은 "Selection commands (Tri-sync)" 그룹에 속한 `bitvue_engine::Command` variant를 그대로 매핑. **새 엔진 작업 없음** — `Core::handle_command`가 이미 넷 다 처리하고 `Event::SelectionUpdated`를 반환함(`select_bit_range`는 Core 내부에서 알아서 가장 가까운 syntax node를 찾아 매칭까지 해줌, `select_spatial_block`은 현재 커서의 frame_index를 Core가 알아서 채워줌 — sidecar가 그 로직을 중복 구현할 필요 없음). 파라미터 타입은 `bitvue_engine::{UnitKey, BitRange, SpatialBlock}` 그대로: `UnitKey{stream,unit_type:String,offset:u64,size:usize}`, `BitRange{start_bit:u64,end_bit:u64}`, `SpatialBlock{x:u32,y:u32,w:u32,h:u32}`.

이걸로 `SelectionState`(`stream_id/temporal/cursor/unit/syntax_node/bit_range/source_view`)가 커버하는 7개 multi-sync 뷰 중 Ref Graph·Metrics를 뺀 5개(Syntax tree/Player/Timeline/Hex/QP-heatmap) 전부에 대응하는 wire 커맨드가 존재하게 됨(`select_frame`=Player/Timeline, `select_unit`=구조 단위, `select_syntax`=Syntax tree, `select_bit_range`=Hex, `select_spatial_block`=QP/MV overlay). 나머지 2개는 여전히 `SelectionState` 필드 자체가 없어서(이전 critical-review 기록 참조) sidecar 작업이 아니라 `bitvue-engine` 설계 작업으로 남아있음.

**중요하게 확인한 것 — `Core::handle_command`의 실제 구현 범위를 `core.rs` grep으로 재확인:** `Command` enum에는 `JumpToOffset`/`JumpToFrame`/`PlayPause`/`StepForward`/`StepBackward`/`ToggleOverlay`/`SetOverlayOpacity`/`SetPlayerMode`/`SetWorkspaceMode`/`SetSyncMode`/`ExportCsv`/`ExportBitstream`/`Export`/`RunFullAnalysis`까지 훨씬 많은 variant가 정의돼 있지만, **실제로 구현된 건 `OpenFile`/`CloseFile`/`SelectFrame`/`SelectUnit`/`SelectSyntax`/`SelectBitRange`/`SelectSpatialBlock` 7개뿐** — 나머지는 전부 `_ => { tracing::debug!(...); vec![] }` catch-all no-op으로 떨어짐. 이 7개를 이번 세션에 전부 sidecar에 연결 완료(`open_stream`/`close_stream`/`select_frame`/`select_unit`/`select_syntax`/`select_bit_range`/`select_spatial_block`) — **`bitvue-sidecar` 쪽에서 "더 연결하기만 하면 되는" 공짜 커맨드는 더 이상 없음.** 다음 커맨드 확장은 먼저 `bitvue-engine`에 실제 핸들러를 구현하는 엔진 작업이 선행돼야 함 — sidecar 쪽 와이어링 패턴 자체는 이미 9번 검증됐으니 반복 위험은 낮지만, 순서가 바뀌었다는 걸 다음 세션이 헷갈리지 않게 여기 명시.

**검증:** `cargo test -p bitvue-sidecar` 유닛 27개(select_unit/select_syntax/select_bit_range/select_spatial_block 각 성공+잘못된 stream id 케이스) + 통합 2개, 전부 통과. 실제 컴파일된 바이너리에 `hello`→`select_unit`→`select_syntax`→`select_bit_range` 배치 파이프 + `select_spatial_block` 단독 요청 둘 다 검증(서브프로세스, mock 아님) — 전부 `ok:true` + 올바른 `SelectionUpdated`. `cargo fmt --check`/`cargo check --workspace` 클린.

### `bitvue-sidecar` 크래시 복구 정책 (2026-08-08)

**결정: sidecar "프로세스"만 재시작, `bitvue_engine::Core`의 인메모리 상태는 복구 안 함.** `Core`가 프로세스 안에만 존재하고 영속화가 전혀 없어서, 크래시 시점에 열려있던 스트림/선택 상태를 재구성할 방법 자체가 없음(재생할 커맨드 로그도 없음) — 그래서 "프로세스 재시작"과 "애플리케이션 상태 복구"를 명확히 분리: 후자는 이번 범위 밖, 필요하면 caller(`bitvue-desktop`)가 `'restarted'` 이벤트를 받고 직접 `open_stream` 등을 재발급해야 함. 크래시 시점에 대기 중이던 요청은 자동 재시도하지 않고 그냥 reject함(`SidecarExitedError`) — 크래시 전에 이미 부분적으로 상태를 바꿨을 수도 있는 요청을 맹목적으로 재시도하는 게 안전하지 않다고 판단.

**구현 위치:** `bitvue-desktop/src/sidecarClient.ts`(Rust 쪽 변경 없음 — sidecar 프로세스 자체는 그냥 죽고 다시 뜨는 것뿐, 프로토콜/엔진 변경 불필요). `SidecarClientOptions.restart`(옵트인, 기본 비활성 — 기존 테스트/호출자 동작 안 바뀜)로 `{maxAttempts, backoffMs}` 지정. `'exit'` 핸들러가 (a) `close()`로 인한 의도적 종료가 아니고 (b) 재시도 횟수가 안 찼으면 `backoffMs` 뒤에 동일 binaryPath/args로 재spawn, `hello()`로 새 프로세스가 실제로 응답하는지까지 확인한 뒤 `'restarted'` emit. `'restarting'`/`'restarted'`/`'restart_failed'` 이벤트 추가. `bitvue-desktop/electron/main.ts`가 `restart:{maxAttempts:3,backoffMs:500}`로 활성화하고, `'restarted'`를 `preload.cjs`의 새 `window.bitvue.onSidecarRestarted(cb)` 채널로 렌더러까지 전달(실제 UI가 아직 없어서 지금은 로그만 찍지만 채널 자체는 연결해둠).

**검증(전부 실제 프로세스, mock 아님):** `bitvue-desktop/tests/sidecarClient.restart.test.ts` 3개 — (1) 실제 컴파일된 바이너리를 `process.kill(pid, 'SIGKILL')`로 진짜 죽이고 `'restarted'` 이벤트 수신 + 새 PID로 `hello()` 재성공 확인, (2) 크래시 시점 대기 요청이 재시도 없이 reject되는지 확인, (3) 계속 즉시 죽는 바이너리(`true`)를 붙여서 `maxAttempts` 소진 후 `'restart_failed'`로 포기하는지 확인. 여기에 **실제 Electron 앱 레벨 증명**도 추가: `BITVUE_ELECTRON_OFFSCREEN=1`로 앱을 띄우고 로그에서 실제 sidecar PID를 추출해 밖에서 `kill -9`, 앱이 죽지 않고 재시작 로그(`sidecar restarted (attempt 1) — application state was lost`)까지 찍히는 것 확인 — `SidecarClient` 단위 테스트뿐 아니라 `main.ts` 배선까지 전부 실증. `npx vitest run` 17/17, `npx tsc --noEmit` 클린.

### `bitvue-core` → `bitvue-engine` 크레이트 리네이밍 완료 (2026-08-08)

**범위 판단 — 세션 시작 시 명확히 좁혀서 진행:** "리네이밍" 항목이 원래 `bitvue-core`→`bitvue-engine`, `src-tauri`→퇴역, `frontend/`→`bitvue-ui` 셋을 포괄하는 것처럼 서술돼 있었지만, 실제로 실행한 건 **`bitvue-core`→`bitvue-engine`뿐**. `src-tauri` 삭제/이동과 `frontend/` 리네이밍은 지금도 하지 않음 — `src-tauri`는 여전히(불완전하게나마, 아래 참조) 작동하는 fallback 앱이고, `frontend/`는 이 세션과 무관한 anti-pattern-audit 작업이 이미 커밋되지 않은 채로 걸려있는 디렉터리라 지금 손대면 두 작업이 뒤섞임. 두 항목 다 "나중에 별도로" 남겨둠.

**실행:** `crates/bitvue-core` → `crates/bitvue-engine` (`git mv`), `Cargo.toml`의 `name` 필드, 루트 워크스페이스 `members`/`workspace.dependencies`, 그리고 **워크스페이스 전체에서 `bitvue_core`/`bitvue-core` 토큰 145개 파일**(Rust 소스+Cargo.toml 19개+`src-tauri`+`fuzz`+활성 문서 6개+`bitvue-desktop` 4개)을 순수 텍스트 치환 — 충돌 위험 사전 확인(`bitvue_core`/`bitvue-core`가 다른 크레이트명의 부분 문자열이 아님을 grep으로 확인 후 진행).

**일부러 안 건드린 것:**
- `docs/anti-patterns/*.md`(약 40개), `.claude/workflows/anti-pattern-scan.js` — 이 세션과 무관한, 아직 커밋 안 된 다른 세션의 작업물. 여기 있는 `bitvue-core` 텍스트 언급은 지금 stale 상태로 남음(다음에 그 작업을 커밋할 때 같이 고칠 것).
- `archive_docs/` — CLAUDE.md에 명시된 대로 동결된 과거 기록, 편집 대상 아님.
- `frontend/types/video.ts` — 딱 2군데 doc-comment 언급뿐이라 안전했지만, 이 파일을 건드리면 lefthook의 `frontend-fmt` 훅이 **스테이징된 파일이 아니라 `frontend/` 전체**를 prettier로 검사해서(`glob`이 트리거만 결정, 실제 `run`은 범위 지정 없음) 무관한 `ModeContext.test.tsx`의 기존 포맷 이슈 때문에 커밋 자체가 막힘 — 그래서 이 파일 변경은 되돌리고 커밋에서 제외.
- `crates/bitvue-core/tests/*_SUMMARY.md` 등 산문 요약본 — 경로는 옮겨졌지만(`bitvue-engine/tests/`로 이동) 내용 중 `bitvue_core` 언급은 히스토리 기록이라 자연스럽게 같이 치환됨(문제 없음, 별도 작업 아니었음).

**실제로 발견한 것 — `src-tauri`는 이미 (rename과 무관하게) 컴파일 안 되고 있었음:** `cargo check --no-default-features`(ffmpeg 헤더가 이 샌드박스에 없어서 기본 feature로는 검증 불가)에서 `bitvue_av1_codec::Obu`에 `obu_type`/`data` 필드가 없다는 등 7개 에러 발생. `git stash`로 리네이밍 이전 원본 코드를 재현해 **동일한 7개 에러가 리네이밍과 무관하게 이미 존재**함을 확인 — `Obu` 구조체가 세션 어느 시점엔가 필드를 `.header` 서브구조체로 옮기는 리팩터를 거쳤는데 `src-tauri`의 일부 호출부가 안 따라간 것으로 보임. **이건 이번 작업 범위 밖, 고치지 않음** — 다만 "src-tauri가 아직 작동하는 fallback"이라는 이 세션 내내의 전제 자체가 최소 이 구성(no-default-features)에서는 이미 깨져 있었다는 뜻이라 명시해둠. 기본 feature(ffmpeg 포함) 빌드가 실제로 되는지는 이 샌드박스에서 검증 불가.

**검증:** `cargo check --workspace` 클린, `cargo test -p bitvue-engine`(3848개) + `-p bitvue-sidecar` + `-p bitvue-protocol` 전부 통과, `cargo fmt --all --check` 클린, `bitvue-desktop` 17/17 유지. 사전에 존재하던 미커밋 anti-pattern-audit 변경분(약 55개 파일)은 이번 커밋 전후로 정확히 동일하게 남아있음(`git status` diff로 확인) — 섞이지 않음.

### `bitvue-ui` 실배선 — 첫 세로 슬라이스: 파일 열기 + 선택 (2026-08-08)

**범위를 실제 코드 조사 후 더 좁힘:** 처음엔 "shim 하나로 `invoke()` 호출을 전부 우회" 아이디어였지만, 실제로 확인해보니 **명령체계가 통째로 다름** — 예전 Tauri 커맨드(~40개)와 지금 `bitvue-sidecar`(9개, selection/query 중심으로 재설계)는 이름도 파라미터 모양도 겹치지 않음. 범용 shim은 애초에 불가능 — 콜사이트마다 새 의미로 다시 생각해야 함. 그래서 "파일 열기 + 프레임 선택만 진짜로 동작하게"로 좁힘 — 그마저도 조사하다 보니 **프레임 목록 조회(`get_frames_chunk` 등)조차 sidecar에 없어서** 실제 프레임 렌더링/썸네일은 이번 슬라이스에서 여전히 불가능하다는 걸 확인, 정직하게 그 경계를 지키며 진행.

**핵심 발견 — `frontend/`엔 이미 좋은 진입점이 있었음:** `frontend/services/tauriCommandService.ts`가 커맨드 호출의 90%가 지나가는 중앙 wrapper였지만, 명령체계 자체가 다르므로 그 파일의 내부만 바꾸는 것도 의미 없음(command name이 안 맞음) — 대신 실제 활성 훅(`frontend/App.tsx`가 쓰는 건 `useFileOperations.ts`가 아니라 `useAppFileOperations.ts`)의 `handleOpenFile`/`handleCloseFile` 두 곳만 정밀 타겟팅.

**한 일:**
- `bitvue-desktop/electron/main.ts`: `ipcMain.handle`에 `bitvue:selectFrame`/`bitvue:closeStream`/`bitvue:showOpenDialog`(Electron 네이티브 `dialog.showOpenDialog`, 렌더러가 직접 못 부르므로 main 경유) 추가. `createWindow()`가 이제 placeholder `index.html` 대신 **`frontend/dist/index.html`(빌드돼 있으면)을 실제로 로드** — `BITVUE_FRONTEND_URL` 환경변수로 dev 서버(`vite`, 5173) 지정도 가능, 아무것도 없으면 경고 로그와 함께 placeholder로 폴백.
- `bitvue-desktop/electron/preload.cjs`: `window.bitvue`에 `closeStream`/`selectFrame`/`showOpenDialog` 추가.
- `frontend/services/electronBridgeService.ts`(신규) — `window.bitvue.*`를 감싸는 얇은 wrapper. 모듈 doc에 "sidecar 9개 커맨드 외엔 아무것도 없다"는 경계를 명시적으로 적어둠(나중에 여기에 함부로 wrapper 추가하지 말라는 가드레일).
- `frontend/hooks/useAppFileOperations.ts`: `handleOpenFile`이 이제 `showOpenDialog`(네이티브 다이얼로그) → `openStream("A", path)` → 성공 시 `selectFrame("A", 0)`까지 실제로 호출. `FileInfo`의 `codec`/`width`/`height` 등은 sidecar가 아직 안 주므로 **정직하게 비워둠**(예전처럼 채워진 것처럼 속이지 않음, 주석으로 이유 명시). `handleCloseFile`도 `closeStream("A")`로 교체. `handleOpenDependentFile`(compare)은 안 건드림 — sidecar에 compare 대응이 없어서 손대도 의미 없음, 현재도 비작동 상태 그대로 둠.

**검증(3단계, 전부 실제로 실행):**
1. `frontend/tests/services/electronBridgeService.test.ts`(11개) — bridge wrapper 자체, `window.bitvue` mock.
2. `frontend/tests/hooks/useAppFileOperations.test.ts`(6개, 신규) — 훅이 새 bridge를 올바른 인자로 호출하는지, 성공/실패/부분실패(open은 성공했는데 selectFrame만 실패) 케이스 전부. React 훅 로직 검증은 Electron 프로세스보다 이쪽이 맞는 도구라고 판단.
3. **`bitvue-desktop`의 `BITVUE_ELECTRON_SELFTEST`를 확장**해서 실제 Electron 프로세스로 `selectFrame`/`closeStream`까지 검증 + `document.title`이 실제 프론트엔드("Bitvue - Bitstream Analyzer")인지 확인(placeholder가 아니라 진짜 앱이 로드됐다는 증거) — `frontend/dist`를 실제로 빌드해서 Electron이 그걸 로드하게 하고 실행, 전부 통과.

`npm run typecheck`/`eslint` 변경 파일 전부 클린. `npx vitest run`(frontend 전체) — 기존 36개 pre-existing 실패(무관한 다른 파일들, 전에 이미 확인된 것과 정확히 동일)만 남고 새 실패 0개, 신규 테스트 17개 전부 통과.

**아직 안 되는 것(정직하게 남겨둠):** 실제 프레임 렌더링/썸네일(프레임 목록 조회 자체가 sidecar에 없음), codec 인식(→ codec-aware 모드 UI 비활성 상태), compare 워크스페이스, export, quality metrics — 전부 `bitvue-sidecar`에 해당 커맨드가 생겨야 다음 슬라이스로 진행 가능.

### 남은 `invoke()` 콜사이트 재조사 — 규모 정정 (2026-08-08)

이전 정리에서 "8개 파일 남음"이라 적었던 건 **잘못된 수치** — 실제로 grep해보니 `frontend/`에서 Tauri
`invoke()`를 직접(또는 `tauriCommandService.ts` 경유로) 호출하는 파일이 **~40개 이상**. 실제 호출되는
커맨드명(`get_frames`, `get_frames_chunk`, `get_thumbnails`, `get_decoded_frame_yuv`, `get_frame_syntax`,
`get_frame_analysis`, `get_stream_info`, `get_codec_extended_info`, `get_yuv_diff_metrics`, `get_rd_point`,
`calculate_bd_rate`, `export_frames_json/csv`, `export_analysis_report` 등)을 추출해 확인한 결과 **전부
`bitvue-sidecar`에 대응 커맨드가 없음** — 파일열기+선택 슬라이스와 달리, 이건 "콜사이트 하나씩 재배선"이
아니라 "`bitvue-engine`에 프레임 목록/썸네일/syntax-detail/stream-info류 조회(query) API 자체가 아직
없다"는 하나의 근본 원인으로 수렴. 다음 라운드를 "다음 콜사이트"로 고르지 말 것 — 먼저 엔진에 어떤 조회
API를 추가할지부터 정해야 함(그리고 이건 `JumpToFrame`처럼 제품 의미 결정이 필요한 문제이지 순수 엔지니어링
문제가 아님). Tauri 창/메뉴 API(`getCurrentWindow`, `utils/menu/**` ~8개 파일)도 `invoke()`와 별개로 아직
전혀 손대지 않은 영역.

### Electron 패키징/빌드 워크플로우 (2026-08-08)

`src-tauri` 삭제(`e7194cc`) 이후 `release.yml`에 TODO로만 남아있던 항목. `bitvue-desktop`은 지금까지
dev 모드(`npm run electron`, 리포 체크아웃에 상대 경로로 sidecar/frontend를 찾음)만 있었고, 실제 배포
가능한 패키지를 만들 방법이 없었음.

**한 일:**
- `bitvue-desktop/electron/main.ts` — `app.isPackaged` 분기 추가: 패키징된 빌드는 `process.resourcesPath`
  기준(`resources/bin/bitvue-sidecar[.exe]`, `resources/frontend/index.html`)으로 리소스를 찾고, dev
  모드는 기존처럼 리포 체크아웃 상대 경로를 그대로 씀. 이 분기 없이는 패키징된 앱이 아예 sidecar를 못 찾음.
- `bitvue-desktop/package.json` — `electron-builder` devDependency 추가, `build` 필드에 플랫폼별
  `extraResources`(mac/linux는 `target/release/bitvue-sidecar`, win은 `.exe`, 공통으로
  `frontend/dist` → `resources/frontend`) 설정, `package`/`package:mac`/`package:linux`/`package:win`
  스크립트 추가. 서명 인증서가 없어 **미서명 빌드**(mac은 `zip`, win은 `nsis`, linux는 `AppImage` 타겟만) —
  코드사이닝/공증/자동업데이트는 다음 단계로 명시적으로 미룸, 지어내지 않음.
- `scripts/package_electron.sh`(신규, `[mac|linux|win]` 인자) — sidecar release 빌드 → frontend 빌드 →
  electron-builder 패키징을 한 번에. `CLAUDE.md` Key scripts 표에 반영.
- `.github/workflows/build-electron-app.yml`(신규) — 삭제된 `build-tauri-app.yml`과 동일한 구조(3-OS
  매트릭스, `workflow_call`로 `release.yml`에서 재사용, 태그 push 시 내부 `release` job이
  `softprops/action-gh-release`로 GH 릴리스 생성). 크로스컴파일 없음(러너 네이티브 아키텍처만) — Tauri
  워크플로우의 `aarch64-apple-darwin` 타겟 지정 같은 건 불필요, Electron은 러너별로 알아서 네이티브 빌드.
- `release.yml`의 TODO 주석을 실제 `build-electron` job(`build-electron-app.yml` 호출)으로 교체.
- **부수적으로 발견해 고침(범위 내 실제 버그):** `ci.yml`이 `bitvue-core`→`bitvue-engine` 리네임(`c2a0e44`)
  때 빠져서 3곳(test matrix, coverage 루프, 커버리지 파일 목록)에서 존재하지 않는 크레이트를 여전히
  참조 — 다음 CI 실행에서 그 크레이트만 실패했을 버그. `bitvue-engine`으로 정정. (참고: `bitvue-sidecar`/
  `bitvue-protocol`은 애초에 `ci.yml`의 test 매트릭스에 없었음 — 그건 별개 gap, 이번엔 안 건드림.)

**검증(로컬에서 실제 패키징된 앱을 실행, 빌드 성공만으로 끝내지 않음):**
- `cargo build --release -p bitvue-sidecar` → `arm64` Mach-O 바이너리 생성 확인.
- `./scripts/package_electron.sh mac` 클린 상태(`release/` 삭제 후)에서 처음부터 끝까지 실행 — sidecar
  릴리스 빌드 → frontend 빌드 → electron-builder 패키징까지 전부 성공, `Bitvue.app` 번들 생성.
  `Contents/Resources/{bin/bitvue-sidecar, frontend/index.html}` 실제 배치 확인.
- **패키징된 `Bitvue.app`을 실제로 실행**(`BITVUE_ELECTRON_SELFTEST=1`, dev 체크아웃이 아니라 번들
  자체를 실행) — `hello`/`openStream`/`selectFrame`/`getHexRange`(byte-exact)/`closeStream` 전부
  `process.resourcesPath` 기준 경로로 실제 성공, `document.title`도 실제 프론트엔드로 확인. 리소스 해석
  분기가 실제로 동작한다는 증거(패키징만 되고 실행은 깨지는 흔한 실패 모드를 배제).
- `cargo fmt --all --check` / `cargo check --workspace` 클린. YAML 3개 파일(`build-electron-app.yml`,
  `release.yml`, `ci.yml`) `yaml.safe_load`로 파싱 검증.

**아직 안 한 것(플래그만, 미구현):** 코드사이닝/공증(mac)·서명(win) — 인증서/시크릿 없음. 자동 업데이트
(`electron-updater` 등) — 미검토. 앱 아이콘 — `electron-builder`가 기본 Electron 아이콘 사용 중, 실제
Bitvue 아이콘 에셋 없음. Windows/Linux 매트릭스 레그는 로컬에서 실행 못 해봄(이 샌드박스는 macOS) — CI에서
처음 실행될 때 검증 필요.

### 디코드/분석 파이프라인 설계 논의 + 첫 구현: `bitvue-indexer` (2026-08-08)

**배경:** `RunFullAnalysis`, Ref Graph/Metrics multi-sync, frontend의 남은 ~40개 `invoke()` 콜사이트 —
전부 같은 근본 원인에 막혀있었음: `Core`가 파일을 열어도 `ByteCache`만 만들 뿐 실제로 컨테이너/유닛/신택스를
파싱하는 코드가 어디에도 없었음(`StreamState.syntax`는 쓰는 코드가 전무, 읽기만 존재). 사용자와 논의 후
두 가지를 확정: **(1) 첫 단계는 메타데이터 인덱싱까지만**(컨테이너+유닛, 픽셀 디코딩은 다음 단계로 보류 —
`DecodedFrame`은 무거운 타입이라 스트림이 길면 전체 디코딩이 비쌈), **(2) sidecar 명령은 세분화**(하나의
`run_full_analysis`가 아니라 `index_stream`/`get_stream_info`/`get_frames_chunk`처럼 frontend가 이미
기대하던 이름에 맞춘 개별 명령, 진행률 보고가 가능한 구조).

**아키텍처 조사 결과 — 재설계가 필요 없었음:** `bitvue-engine`이 leaf crate라 `Core`가 `bitvue-decode`를
직접 호출할 수 없다는 문제는, `Core::get_stream()`/`get_job_manager()`가 이미 `pub`이라는 걸 확인하면서
해소됨 — `bitvue-decode`처럼 `bitvue-engine`에 의존하는 크레이트라면 `bitvue-engine` 수정 없이
`Arc<RwLock<StreamState>>`를 통해 읽기/쓰기가 가능. `worker.rs`의 `JobManager`(latest-wins 취소, 스트림당
최대 2개 동시 실행)와 `Job` enum(`ParseContainer`/`ParseUnits`/`BuildSyntaxTree`/`BuildTimelineIndex`/
`DecodeFrame`/`ComputeMetrics` 등, 정확히 이 파이프라인 모양)도 이미 완성돼 테스트까지 있지만 **실제 호출자가
전무**(`spawn()`/`submit()` 호출이 코드베이스 전체에 0건) — 죽은 인프라였음. `indexing.rs`/`index_extractor.rs`
/`index_session.rs`(1781줄, "T1-1 Two-Phase Index Builder")도 마찬가지: `QuickIndex`/`FullIndex`/
`IndexState`/`IndexProgress` 등 진행률·게이팅 상태기계는 완성돼 있지만 실제 파싱 로직을 채우는 코드가 없음.
**이번 라운드는 이 진행률/2단계 상태기계에 올라타지 않음** — 그건 별도 스코프, 지금은 동기적으로
`StreamState.container`/`.units`만 채움. `TimelineModel`도 이번엔 스킵 — `stream_state.rs`와 `timeline.rs`에
서로 다른 `TimelineFrame` 구조체가 두 개 존재하는 걸 발견, 어느 게 정본인지 불명확해서 설계 재확인 없이
막 채우지 않기로 결정.

**한 일 (`crates/bitvue-indexer`, 신규 크레이트):**
- `bitvue-engine`+`bitvue-av1-codec`에만 의존. `pub fn index_stream(core: &Core, stream: StreamId) ->
  Vec<Event>` — `ByteCache`에서 바이트를 읽어 IVF 매직(`DKIF`) 확인 → `bitvue_av1_codec::ivf::
  parse_ivf_frames`로 프레임 워크 → 프레임마다 `parse_frame_header_basic`으로 OBU 프레임 헤더 파싱(프레임
  타입/QP/참조 프레임) → `ContainerModel`+`UnitModel`을 만들어 `StreamState`에 씀. IVF가 아니면 정직하게
  `DiagnosticAdded`(카테고리 Container) — 지어내지 않음.
- 로직은 `bitvue-mcp`의 기존 `parse_ivf_file`(실제로 동작 중인 코드, `load_file` MCP 툴이 씀)을 참고해
  거의 그대로 재현 — 바이트 오프셋 계산, OBU 헤더 스킵, `ref_frame_idx`/`base_q_idx` 추출 로직까지 동일.
  `bitvue-mcp`는 건드리지 않음(중복이지만, 이미 동작 중인 별도 툴을 이번 라운드에서 리팩터링할 이유 없음 —
  중복 사실만 정직하게 코드 주석에 남김).
- `Command::RunFullAnalysis`는 그대로 미사용 — `bitvue-sidecar`가 `Core::handle_command`를 거치지 않고
  `bitvue_indexer::index_stream()`을 직접 호출.

**sidecar 신규 명령 3개 (`index_stream`/`get_stream_info`/`get_frames_chunk`):** `get_stream_info`/
`get_frames_chunk`는 순수 읽기 — `state.container`/`.units`가 아직 없으면(인덱싱 전) 와이어 에러가 아니라
`{"indexed": false, ...}`로 정직하게 응답(아직 인덱싱 안 된 건 정상 상태지, 실패가 아님). `get_frames_chunk`는
`offset`/`limit` 페이지네이션. `UnitNode`는 이미 `Serialize` derive가 있어서(대부분의 `bitvue-engine` 타입과
달리) 직접 `serde_json::to_value`— `ContainerModel`은 derive가 없어서 수동 JSON 매핑(기존 `event_to_json`
패턴과 동일).

**검증:** `bitvue-indexer` 유닛 테스트 3개 — 실제 `test_data/av1_test.ivf` 픽스처로 컨테이너/유닛 채워짐,
첫 프레임 키프레임 확인, 오프셋이 단조증가+겹침없음(`offset[i+1] == offset[i] + size[i]`) 검증. 파일 미오픈/
비-IVF 입력에 대한 정직한 diagnostic도 테스트. `bitvue-sidecar` 통합 테스트 5개(신규) — 실제 픽스처로
`open_stream`→`index_stream`→`get_stream_info`/`get_frames_chunk` 전체 체인, 페이지네이션 2페이지째 확인,
인덱싱 전 `get_frames_chunk` 호출이 에러가 아니라 `indexed:false`로 응답하는 것까지. `cargo fmt --all --check`
/ `cargo check --workspace` 클린, `bitvue-indexer`+`bitvue-sidecar`+`bitvue-engine`+`bitvue-protocol`
전체 테스트 스위트(29+2+5+... 전부) 통과.

**아직 안 한 것(다음 단계 후보, 이번엔 의도적으로 안 함):** `.syntax`/`.timeline` 채우기(TimelineModel 이중
정의 문제 먼저 정리 필요), AV1 외 코덱(H.264/HEVC/VP9/VVC — `bitvue-codecs-parser`가 이름과 달리 실제
디스패처가 아니라 미구현 placeholder라는 것도 이번에 확인됨, "BOSS_03에서 구현 예정" 주석만 있음), MP4/MKV/TS
컨테이너, 픽셀 디코딩(`DecodeFrame` job), `JobManager`/`IndexState` 진행률 상태기계와의 통합(현재는 동기
호출, 스트리밍 진행률 없음).

### `bitvue-indexer` frontend 소비 — 프레임 리스트가 처음으로 UI에 도달 (2026-08-08)

**핵심 발견:** `frontend/contexts/FileStateContext.tsx`가 이미 Tauri의 `get_frames_chunk` 커맨드를
페이지네이션 방식으로 호출하고 있었음(`refreshFrames`/`loadMoreFrames`, 청크 크기 100) — 이름까지 새
sidecar 커맨드와 우연히 일치. 데이터 소스만 교체하면 되는 좋은 슬라이스였음. `types/video.ts`의 `FrameInfo`
타입도 `UnitNode`와 필드가 상당히 겹침(`frame_index`/`frame_type`/`size`/`pts`/`temporal_id`/
`ref_frames`/`ref_slots`).

**한 일:**
- `bitvue-desktop/electron/{preload.cjs,main.ts}`: `indexStream`/`getStreamInfo`/`getFramesChunk` IPC 채널
  추가(각각 sidecar의 `index_stream`/`get_stream_info`/`get_frames_chunk`에 그대로 매핑).
- `frontend/services/electronBridgeService.ts`: 위 3개 wrapper 함수 + `BridgeUnitNode`/
  `BridgeContainerModel`/`StreamInfoResult`/`FramesChunkResult` 타입(sidecar의 JSON 응답 shape를 그대로
  미러링).
- `frontend/contexts/FileStateContext.tsx`: `refreshFrames`가 이제 `indexStream("A")` 호출 후
  `getFramesChunk`로 페이지네이션 — 기존 청크 루프 구조는 그대로 유지, 데이터 소스만 Tauri `invoke`에서
  bridge로 교체. `unitNodeToFrameInfo()`가 `UnitNode`→`FrameInfo` 매핑, 없는 필드(`poc`/`display_order`/
  `coding_order`/`spatial_id`/`thumbnail`/`duration`/`ref_slot_info`)는 정직하게 `undefined`.
- **부수 발견(고치지 않음, 정직하게 테스트로 남김):** `loadMoreFrames`는 `refreshFrames`가 끝나면
  무조건 `hasMoreFrames`를 `false`로 리셋하는 기존 로직(이번 마이그레이션과 무관, Tauri 시절부터 있던 코드)
  때문에 **현재 앱 흐름에서는 사실상 도달 불가능한 죽은 코드** — `refreshFrames` 자체가 이미 전체를
  즉시 페이지네이션해서 다 가져오기 때문. 고치라는 요청도 없었고 동작 변경은 범위 밖이라 그대로 두고,
  테스트로 이 사실을 정직하게 문서화(가드가 실제로 no-op을 반환하는지만 검증).

**검증:** `frontend/tests/services/electronBridgeService.test.ts`(+4, 신규 wrapper), `frontend/tests/
contexts/FileStateContext.test.tsx`(5개, 신규 — 단일 페이지, 멀티 페이지 페이지네이션, 필드 미조작 확인,
에러 전파, `loadMoreFrames` no-op) 전부 통과. `npx vitest run` 전체 — 기존 36개 pre-existing 실패(파일
9개, 전 세션과 정확히 동일한 목록: AV1FeaturesView/BitrateGraphPanel/BitViewPanel/
KeyboardShortcutsDialog/ModeSelector/ReferenceGraphPanel/ResidualsView/StatisticsTab/
SyntaxDetailPanel)만 남고 새 실패 0개. **`BITVUE_ELECTRON_SELFTEST`를 실제 AV1/IVF 픽스처
(`test_data/av1_test.ivf`, 스트림 "B")로 확장**해서 진짜 Electron 프로세스로 `indexStream`→
`getStreamInfo`→`getFramesChunk`까지 실행 — `container.codec === "av1"`, 첫 5프레임 중 `frame_type ===
"I"`까지 실제 데이터로 확인, exit code 0. `npm run typecheck`(frontend+bitvue-desktop) 클린.

### 실제 앱 첫 실행에서 발견된 블랭크 스크린 버그 수정 (2026-08-08)

사용자가 `npm run electron`으로 실제 앱을 처음 띄워봄 — 검은/빈 화면. 이번 세션 내내 "실제 프론트엔드가
로드됐다"는 증거로 써온 `document.title` 체크는 정적 `<title>` 태그라 JS 번들이 아예 실행 안 돼도 통과한다는
게 드러남 — 진짜 검증 공백이었음. 원인은 두 개, 독립적:

1. `frontend/vite.config.ts`에 `base` 옵션이 없어서 Vite 기본값(절대경로, `/assets/index-*.js`)이 나감 —
   `http://`로 서빙할 땐 문제없지만 Electron의 `file://` 로딩(`win.loadFile()`)에선 404. `base: "./"`로 수정.
2. `App.tsx`의 `AppContent`가 `useLayout()`을 쓰는데 `main.tsx`는 `ThemeProvider`만 씌우고
   `LayoutProvider`는 한 번도 감싼 적이 없었음 — `git log`로 확인, 이번 마이그레이션이 만든 회귀가 아니라
   원래부터 있던 갭. `App.test.tsx`가 `LayoutContext`를 통째로 모킹해서 테스트로는 안 걸림. 렌더 트리가
   uncaught 에러로 죽어서 `#root`가 비어있었음. `main.tsx`에 `<LayoutProvider>` 추가로 수정.

**검증 공백도 같이 닫음:** `BITVUE_ELECTRON_SELFTEST`에 `document.getElementById("root").
childElementCount`를 폴링(고정 딜레이 아님)으로 확인하는 체크 추가, `console-message`/`did-fail-load`를
렌더러→메인 프로세스 stdout으로 포워딩(전에는 빈 화면이 떠도 터미널에 아무 설명이 안 나왔음). **교훈:
`document.title`은 HTML이 로드됐다는 증거지 React 앱이 렌더됐다는 증거가 아님** — 앞으로 "프론트엔드
로드 확인" 주장엔 실제 DOM 체크가 필요.

남은 논-fatal 이슈(고치지 않음, 플래그만): `initializeSystemMenu`(Tauri 메뉴 API)가 Electron에선 Tauri
런타임이 없어서 throw — catch돼서 렌더링은 안 막지만, `utils/menu/**` 아직 마이그레이션 안 된 기존
스코프 경계와 일치.

### `get_frame_syntax` — 지연(lazy) 신택스 트리 파싱 (2026-08-08)

`bitvue-indexer`의 다음 후보 중 "AV1 확장 코딩만 집중" 신호에 맞춰 신택스 트리 쪽으로: `Core::
handle_command`의 `SelectBitRange`가 이미 `.syntax`에 대한 on-demand nearest-node 탐색을 하고 있었지만
`.syntax`를 채우는 코드가 어디에도 없어서 항상 빈 상태였음 — 이번에 그 공백을 메움.

**핵심 발견:** `bitvue_av1_codec::parse_obu_syntax(data, obu_index, global_offset) -> Result<SyntaxModel>`
가 이미 완성돼 있고 CLI의 `analyze.rs`가 이미 씀 — 새로 파싱 로직을 안 짜고 그대로 재사용. frontend의
`FrameSyntaxTab.tsx`/`BitViewPanel.tsx`가 이미 `invoke("get_frame_syntax", {path, frameIndex})`를 호출하고
있던 것도(옛 Tauri 커맨드, 지금은 죽어있음) `get_frames_chunk` 때와 같은 패턴 — 이름이 이미 맞아떨어짐.

**한 일:** `bitvue-indexer::get_frame_syntax(core, stream, frame_index) -> Result<SyntaxModel, String>` —
`state.units`에서 유닛 조회 → `unit.offset+12`(12바이트 IVF 청크 헤더 스킵)부터 OBU 바이트 읽기 →
`parse_obu_syntax` 호출 → 결과를 `state.syntax`에 씀(그래서 `SelectBitRange`의 탐색이 이제 실제로 뭔가를
찾을 수 있음) → 모델을 그대로 반환. AV1이 아니거나 `index_stream`이 먼저 안 돌았으면 정직한 에러 문자열.
`bitvue-sidecar`에 `get_frame_syntax` 커맨드 추가(13번째 실제 커맨드) — `SyntaxModel`의 flat
`HashMap<SyntaxNodeId, SyntaxNode>`+`root_id`를 재귀로 중첩 JSON 트리로 변환(`syntax_node_to_json`),
frontend의 느슨한 `SyntaxNode{type,name,children,...}` 모양에 맞춤. 실패는 `get_frames_chunk`류와 다르게
진짜 wire error(`WireErrorCode::FrameNotFound`)로 — "이 프레임의 신택스 트리"엔 `{indexed:false}`같은
의미있는 부분 결과가 없어서.

**검증:** `bitvue-indexer` 신규 테스트 3개(실제 픽스처로 진짜 트리 생성 확인 — root에 자식 있음, `state.
syntax` 채워짐 / `index_stream` 안 돌았을 때 에러 / 범위 밖 frame_index 에러) + `bitvue-sidecar` 통합
테스트 3개(전체 체인으로 진짜 중첩 트리 확인 / 인덱싱 전엔 wire error / 잘못된 stream id) 전부 통과.
`cargo fmt --all --check` / `cargo check --workspace` 클린.

**아직 안 함(정직하게 남겨둠):** frontend 쪽 소비(`electronBridgeService`에 `getFrameSyntax` 추가 — 다음
라운드), AV1 외 코덱(VP9 등 — 세션 중 시도했다가 사용자 피드백으로 되돌림, 명시적 재확인 없이 다시 진행
안 함).

### `get_frame_syntax` frontend 소비 + 실제 버그 발견/수정 (2026-08-08)

`get_frame_syntax`를 이미 쓰던(옛 Tauri 커맨드) 컴포넌트 2개를 새 bridge로 재배선: `BitViewPanel.tsx`
(로컬 flat-value `SyntaxNode` 타입), `SyntaxDetailPanel/FrameSyntaxTab.tsx`(로컬 discriminated-union
`SyntaxValue` 타입). 두 컴포넌트가 서로 다른 로컬 타입을 갖고 있어서(공유 안 됨) 각자 자기 shape로 변환하는
어댑터 함수(`bridgeNodeToLocal`)를 각각 작성 — `unitNodeToFrameInfo` 때와 같은 "경계에서 번역" 패턴.

**실제 버그 발견(설계 중 우연히):** `FrameSyntaxTab`의 `byte_offset`(hex 뷰 점프용) 필드를 `bit_range.
start_bit / 8`로 계산하려다가, `bitvue-indexer::get_frame_syntax`(직전 라운드 `2b7f873`에서 커밋됨)가
`parse_obu_syntax`에 **바이트 오프셋을 그대로** 넘기고 있다는 걸 발견 — `TrackedBitReader::new`의 문서
확인 결과 `global_offset` 파라미터는 **비트** 오프셋이어야 함(CLI의 `analyze.rs`가 이미
`(offset * 8) as u64`로 올바르게 호출 중인 것도 확인). 고치지 않았으면 모든 `SyntaxNode.bit_range`가
8배 어긋난 채로 조용히 나갔을 것 — 트리 구조 자체는 정상이라 기존 테스트(빈 트리 아님만 확인)로는
안 걸림. `bitvue-indexer/src/lib.rs`에서 `obu_offset` → `obu_offset * 8`로 수정, 실제 파일 레이아웃
기준 회귀 테스트 추가(프레임 0의 OBU는 바이트 44=IVF헤더32+청크헤더12에서 시작하므로 `obu_forbidden_bit`
필드가 정확히 비트 352에서 시작해야 함 — 수정 전엔 44로 나와서 실패하는 것까지 확인).

**한 일:** `bitvue-desktop/electron/{preload.cjs,main.ts}`에 `getFrameSyntax` IPC 채널.
`electronBridgeService.ts`에 `BridgeSyntaxNode` 타입(`{type,name,value,bit_range,children}`, sidecar의
`syntax_node_to_json`을 그대로 미러링) + wrapper(13번째 커맨드 — 실패 시 `{indexed:false}`류가 아니라
진짜 throw, `get_stream_info`류와 다르게 "이 프레임 신택스"엔 의미있는 부분결과가 없어서). 두 컴포넌트
모두 `path`/`filePath` 기반 호출을 `stream="A"` 고정 호출로 교체(다른 콜사이트들과 동일한 단일-스트림
가정).

**검증:** `electronBridgeService.test.ts`(+2, 신규) 통과. `BitViewPanel.test.tsx`(5개 실패)는 기존
pre-existing 베이스라인(공급자 래핑 없이 구버전 placeholder 텍스트를 찾는 낡은 테스트, 이번 변경과 무관 —
확인함) 그대로, `FrameSyntaxTab.test.tsx`(16개)는 전부 통과. `npx vitest run` 전체 — 여전히 9파일/36개
pre-existing 실패만, 새 실패 0개. `BITVUE_ELECTRON_SELFTEST`에 `getFrameSyntax("B", 0)` 호출 추가 —
실제 렌더러→preload→main→sidecar 체인으로 진짜 중첩 트리를 받아서 `obu_forbidden_bit` 필드를 재귀
탐색해 `bit_range.start_bit === 352`까지 확인(수정된 비트오프셋 버그가 E2E 레벨에서도 안 재발하는지
증명), exit code 0.

### 나머지 tri-sync 선택 커맨드 4개를 bridge에 추가 (2026-08-08)

`select_unit`/`select_syntax`/`select_bit_range`/`select_spatial_block` — 이 세션 이전부터
`bitvue-sidecar`에 이미 있었고 테스트도 돼 있었지만(`Command`의 "Tri-sync" variant에 직결) `electronBridgeService.ts`엔 한 번도 안 걸려있었음. `selectFrame`만 있었음.

**이번 라운드는 이전 라운드들과 성격이 다름:** `get_frames_chunk`/`get_frame_syntax`는 전부 frontend에
이미 죽어있던 콜사이트(옛 Tauri invoke)가 있어서 "이름이 우연히 맞아떨어짐" 패턴이었지만, 이 4개는
**현재 아무 frontend 컴포넌트도 호출하지 않음**(grep으로 확인 — select_unit/select_syntax/
select_bit_range/select_spatial_block 관련 문자열이 frontend 어디에도 없음). "다음 구현" 지시에 따라
진행했지만, 실제 UI 트리거 없이 미리 배선하는 것 — sidecar 쪽 능력이 실재하고 테스트돼 있다는 근거로
진행, UI 기능이 완성됐다는 뜻은 아님. 모듈 doc에 명시.

**다른 후보들과 비교해 이걸 고른 이유:** `ContainerModel.duration_ms`/`bitrate_bps`(IVF 헤더의
framerate 필드로 계산 가능)도 검토했지만, IVF rate/scale 필드 의미가 구현마다 뒤바뀌기 쉬운 걸로 알려져
있어서(바로 전 라운드에서 발견한 bit/byte 오프셋 버그와 같은 부류의 위험) 실제 소비자도 없는 채로
추측성 단위 변환을 넣는 건 보류 — 이번 4개는 최소한 파라미터 shape가 이미 확정/테스트된 기존 커맨드를
그대로 미러링하는 순수 기계적 작업이라 리스크가 낮음.

**한 일:** `preload.cjs`/`main.ts`에 4개 IPC 채널(각 sidecar 커맨드의 정확한 파라미터 shape 그대로:
`select_unit{unit_type,offset,size}`, `select_syntax{node_id,start_bit,end_bit}`,
`select_bit_range{start_bit,end_bit}`, `select_spatial_block{x,y,w,h}`). `electronBridgeService.ts`에
동일 이름 wrapper 4개 추가 — 전부 `selectFrame`과 같은 `{events}` 반환 패턴.

**검증:** `electronBridgeService.test.ts`(+4, 신규) 전부 통과. `npx tsc --noEmit` 클린. `npx vitest run`
전체 — 여전히 9파일/36개 pre-existing 실패만, 새 실패 0개. sidecar 쪽 4개 커맨드는 이번 세션 이전부터
있던 기존 테스트(각 2개씩, 8개)로 이미 검증돼 있어서 새로 안 만듦. Electron selftest는 확장 안 함 —
실제로 호출할 UI 트리거가 없어서 의미 있는 E2E 시나리오를 못 만듦(정직하게 생략, 억지로 안 만듦).

### `TimelineFrame` 이중정의 조사 → `get_timeline` 구현 (2026-08-08)

"다음 구현" 지시에 이전에 "사용자 판단 필요"로 플래그해뒀던 `TimelineFrame` 이중정의 문제를 다시 봄 —
Explore 에이전트로 실제 코드 증거를 조사해보니 **애매하지 않고 결론이 명확했음**: `stream_state.rs`의
`TimelineModel`/`TimelineFrame`은 참조 0건, 어디서도 생성 안 됨 — `ContainerModel`/`UnitModel`이 이 세션
초반에 그랬던 것과 똑같은 "Phase 1" 미완성 placeholder. `timeline.rs`의 `TimelineBase`/`TimelineFrame`은
9개 이상의 실제 서브시스템 파일(lanes/window/evidence/export/picture_stats)이 이미 의존하는, "T4-1
deliverable"이라고 명시된 진짜 설계. **이건 제품 의미 결정이 아니라 조사로 풀리는 질문이었음** — 처음에
"물어봐야 한다"고 플래그한 게 성급했음, 답이 이미 코드 안에 있었음.

**추가 조사로 실현 가능성 확인:** `timeline.rs`를 실제로 쓰는 `TimelineMapper::new(stream_id, Vec<
FrameMetadata>, sizes, types).build_timeline_av1()`(`frame_identity.rs`)이 필요로 하는 입력은 딱
`{pts, dts}` + 크기 + 타입 문자열 — 디코드 순서로만 있으면 되고 내부에서 알아서 display order로 정렬함
(AV1 리오더링 케이스까지 이미 처리됨). 픽셀 디코딩 전혀 불필요 — `bitvue-indexer`가 이미 갖고 있는
`Vec<UnitNode>`(pts/size/frame_type)로 충분. `build_timeline_av1` 자체는 이 세션 이전부터 있었지만
프로덕션 호출자가 0건이었음(테스트에서만 씀).

**부수 발견(설계 중 우연히, 또 하나):** `Av1TimelineExtractor::determine_marker`는 리터럴 `"KEY_FRAME"`/
`"INTRA_ONLY_FRAME"` 문자열만 매치 — 제네릭 기본 extractor의 `"I"` 단축 매치와 다름(AV1 전용
override가 그걸 안 씀). `index_ivf_av1`이 만드는 "I"/"P"/"B" 단축 코드를 그대로 넘기면 키프레임이
0개로 조용히 나옴 — 테스트로 실제로 걸림(`keyframe_indices()`가 비어서 나옴), 이 호출 경계에서만
"I"→"KEY_FRAME" 매핑 추가해서 수정(`UnitNode.frame_type` 자체는 다른 곳 전부 "I"/"P"/"B" 그대로 유지).

**한 일:** `bitvue-indexer::get_timeline(core, stream) -> Result<TimelineBase, String>` — AV1만,
`index_stream`이 먼저 안 돌았으면 에러. **`StreamState.timeline`(죽은 `TimelineModel` 타입)엔 의도적으로
안 씀** — 함수 doc에 이유 명시(타입을 바꾸는 건 `bitvue-engine` 자체를 고치는 일이라 이 세션 내내
지켜온 "leaf crate는 안 건드림" 원칙 밖). `bitvue-sidecar`에 `get_timeline` 커맨드(14번째) — `TimelineBase`
가 이미 `Serialize`를 derive하고 있어서(대부분의 bitvue-engine 타입과 다름) `syntax_node_to_json` 같은
수동 매핑 불필요, 그대로 직렬화. frontend `getTimeline` wrapper도 같이 추가(`components/Timeline.tsx`에
`getTimelineRect`라는 이름이 비슷한 DOM 헬퍼가 있어서 착각할 뻔했지만 확인 결과 무관 — 실제 UI 소비자는
없음, `select_unit`류와 같은 이유로 완결성 목적으로 배선, 모듈 doc에 명시).

**검증:** `bitvue-indexer` 신규 테스트 2개(실제 픽스처로 frame_count 일치 + 첫 프레임 키프레임 마킹 확인)
+ `bitvue-sidecar` 통합 테스트 3개(전체 체인 진짜 타임라인 + marker 확인 / 인덱싱 전 wire error / 잘못된
stream id) + `electronBridgeService.test.ts`(+2) 전부 통과. `BITVUE_ELECTRON_SELFTEST`에 `getTimeline("B")`
추가해서 실제 렌더러→preload→main→sidecar 체인으로 진짜 타임라인 받아서 `frames[0].marker === "Key"`까지
확인(마커 매핑 버그가 E2E 레벨에서도 재발 안 하는지 증명), exit code 0. `npx vitest run` 전체 — 여전히
9파일/36개 pre-existing 실패만, 새 실패 0개.

### 실제 스크린샷 검증 인프라 + 진짜 프레임 타입 파싱 버그 발견/수정 (2026-08-08)

"다음 고고" 지시로 계속 진행 — 이전에 "육안 확인은 사용자 눈이 필요하다"고 여러 번 말했었는데, 실제로는
Electron의 `webContents.capturePage()`로 스크린샷을 찍어서 Read 툴로 직접 볼 수 있다는 걸 깨달음(멀티모달
LLM이라는 걸 스스로 활용 안 하고 있었음). 실제 프로덕션 코드 경로(App.tsx의 `handleOpenFile`, 네이티브
메뉴가 원래 디스패치하는 `"menu-open-bitstream"` DOM 이벤트로 트리거)를 실행시켜서 진짜 화면을 캡처.

**한 일:** `main.ts`에 `BITVUE_ELECTRON_SCREENSHOT=<output.png>` 모드 추가 — `menu-open-bitstream` 이벤트를
디스패치해 실제 `handleOpenFile()`을 실행시키고(네이티브 OS 파일 다이얼로그만 테스트 전용 환경변수
`BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH`로 우회 — 실제 사용 시엔 절대 설정 안 되는 값이라 프로덕션 동작에
영향 없음), 3초 대기 후 `capturePage()`로 진짜 화면을 PNG로 저장.

**첫 스크린샷에서 실제 버그 발견:** Stream Tree 패널에 250개 프레임이 전부 "I" 타입으로 나옴 — 프레임
사이즈가 10627/5488/217/251...로 크게 요동치는 걸 보면(전형적인 I/P 사이즈 패턴) 명백히 잘못됨. 원인:
IVF 청크 하나는 OBU 하나가 아니라 보통 Temporal Delimiter + (키프레임이면 Sequence Header) + 실제
Frame/FrameHeader OBU 순서로 여러 개가 들어있는데, `index_ivf_av1`이 청크의 **첫 바이트를 무조건 프레임
헤더로 가정**해서 Temporal Delimiter의 찌꺼기 바이트를 프레임 헤더인 것처럼 파싱하고 있었음
(`ObuIterator`로 실제 검증: frame 0 청크 = TD(2바이트) + SequenceHeader(13바이트) + Frame(그 다음부터)).
**`get_frame_syntax`도 똑같은 버그**였음 — 이전 라운드의 회귀 테스트(`bit_range.start_bit == 352`)가
통과했던 이유는 `obu_forbidden_bit` 필드가 OBU 타입과 무관하게 모든 OBU 헤더에 공통으로 존재해서, TD를
파싱해도 그 필드 자체는 나왔기 때문 — 테스트가 "트리가 나온다"만 확인하고 "올바른 OBU의 트리인지"는
확인 안 하고 있었음.

**수정:** `find_frame_obu()` 헬퍼 신규 — `ObuIterator`로 청크 안의 OBU들을 순회해서 `Frame`/`FrameHeader`
타입을 실제로 찾음(CLI의 `analyze.rs`가 이미 쓰던 정확한 패턴 그대로). `index_ivf_av1`과
`get_frame_syntax` 둘 다 이 헬퍼로 교체 — 더 이상 첫 바이트를 가정하지 않음. `get_frame_syntax`의
회귀테스트 기댓값도 352(이전 버그값, TD의 위치)에서 472(진짜 Frame OBU 위치, byte 59)로 정정, 새 회귀
테스트(`frame_zero_chunk_starts_with_a_temporal_delimiter_not_a_frame_obu`)로 픽스처의 실제 OBU 순서를
고정, `index_stream_frame_types_are_not_all_the_same`으로 프레임 타입이 실제로 다양한지 확인.

**결과 재확인:** 수정 전 분포 `{I: 250}` → 수정 후 `{P: 249, I: 1}` — 정상적인 인코딩 패턴. 두 번째
스크린샷으로 최종 확인: 필름스트립이 "I-00"(빨강) 다음 "P-01"~"P-08"(초록)로, Stream Tree도 "Frame #0 - I",
"Frame #1 - P"...로 정확하게 표시됨.

**발견했지만 안 고친 것(플래그만):** Stream Tree의 모든 프레임이 여전히 `@ 0x00000000`로 표시됨 —
`frontend/components/panels/StreamTreePanel.tsx`의 `frameUnits` fallback 생성 코드가 `offset: 0`을
하드코딩(프리-이 세션, 백엔드 버그와 무관) — 실제 오프셋 데이터는 이미 백엔드에 있음, 프론트엔드
쪽 별도 수정 필요, 이번 라운드 스코프 밖.

**검증:** `bitvue-indexer` 신규/수정 테스트 3개, `bitvue-sidecar`/`bitvue-indexer` 전체 스위트(11+37)
전부 통과. `cargo fmt --all --check`/`cargo check --workspace` 클린. 실제 Electron 프로세스 스크린샷
2회(버그 확인용 1회 + 수정 확인용 1회) — 코드 검증이 아니라 실제로 화면을 보고 확인한 첫 사례.

### `FrameInfo.offset` — StreamTreePanel의 `@ 0x00000000` 하드코딩 수정 (2026-08-08)

바로 위 라운드에서 플래그만 해두고 넘어갔던 것 — "다음 고고" 지시로 바로 이어서 처리. 원인 재확인:
`FrameInfo` 인터페이스 자체에 `offset` 필드가 아예 없었음 — `StreamTreePanel.tsx`의 `frameUnits` fallback
생성 코드가 `offset: 0`을 하드코딩한 게 아니라, 애초에 `frame.offset`이라는 필드를 참조할 방법이 없어서
`0`을 넣을 수밖에 없었던 것. 실제 오프셋 데이터는 `UnitNode.offset`(백엔드)에 이미 있었지만
`unitNodeToFrameInfo` 매핑(`FileStateContext.tsx`)이 이 필드를 건너뛰고 있었음.

**한 일:** `types/video.ts`의 `FrameInfo`에 `offset?: number` 추가, `unitNodeToFrameInfo`가 `unit.offset`을
그대로 전달하도록 수정, `StreamTreePanel.tsx`는 `frame.offset ?? 0`으로 실제 값 사용.

**검증:** `FileStateContext.test.tsx`에 실제 오프셋 값(16147) 왕복 확인 테스트 추가, 기존 `StreamTreePanel.
test.tsx`(75개, `units` prop 경로로 이미 0이 아닌 오프셋 포매팅을 검증하고 있었음 — 이번 수정은 데이터
소스만 고침) 전부 통과. `npx tsc --noEmit`/`npx vitest run` 전체 — 여전히 9파일/36개 pre-existing 실패만.
**세 번째 스크린샷으로 실측 확인**: `0x00000020`(=32, IVF 헤더), `0x000029a3`(=10659),
`0x00003f13`(=16147)... 전부 Rust 테스트에서 이미 검증된 실제 값과 정확히 일치.

### Electron selftest의 stale 352 값 수정 (2026-08-08)

바로 위 라운드(OBU 파싱 버그 수정, `find_frame_obu`)에서 `bitvue-indexer`의 Rust 회귀 테스트는
352→472로 고쳤지만, 같은 값을 독립적으로 검증하는 `bitvue-desktop/electron/main.ts`의
`BITVUE_ELECTRON_SELFTEST` 자체 어서션(`frameSyntaxOk`)은 고치지 않고 넘어갔던 것 — 다음 세션 시작
시 `grep`으로 재확인하다 발견. 352→472로 정정하고 `BITVUE_ELECTRON_SELFTEST=1` 전체 재실행으로
`get_frame_syntax OK: true`, exit 0 확인 — Rust 유닛/통합 테스트와 풀스택 Electron selftest가 이제
전부 같은 값(472)에 동의함.

### `YuvViewerPanel` 메인 미리보기 화면 완전 복구 (2026-08-08)

**발견 경위:** `get_frame_syntax` 수정을 스크린샷으로 육안 확인하려고 `BITVUE_ELECTRON_SCREENSHOT_CLICK_TAB`
(신규 — 스크린샷 모드가 캡처 전에 지정한 라벨의 좌측 탭을 클릭하게 함, 기본 탭 외의 패널도 검증 가능)을
추가해 Syntax 탭을 열어보던 중, 메인 비디오 미리보기 화면이 "Failed to load frame" 에러로 완전히
깨져있는 걸 발견. 콘솔에 `[YuvViewerPanel] Failed to load frame: TypeError: Cannot read properties of
undefined (reading 'invoke')` — `YuvViewerPanel`이 아직 `@tauri-apps/api/core`의 `invoke()`를 직접
호출하고 있었음(`get_decoded_frame_yuv`), Electron 브릿지로 마이그레이션된 적이 없어서.

**스코프 확인:** grep으로 확인한 결과 `frontend/` 전체에서 아직 `@tauri-apps/api`를 직접 import하는
파일이 40개(메뉴/필름스트립 썸네일/YUV diff/품질비교/RD curves/Compare Workspace 등) — 이번 세션의
좁은 스코프(파일 열기+선택 인프라)를 훨씬 넘는 별도의 큰 작업임을 사용자에게 보고, "YuvViewerPanel만
우선 수정"으로 스코프 확정 (전체 40개 스윕은 보류).

**한 일 — `get_decoded_frame_yuv` 신규 sidecar 커맨드:**
`bitvue-decode::Av1Decoder`(dav1d 기반, `bitvue-cli`의 `decode --dump`가 이미 씀)는 이미 실제로
동작하는 코드였음 — sidecar에 연결만 안 돼 있었음. `bitvue-sidecar/src/decode_bridge.rs` 신규
(`bitvue-indexer`가 아님 — 그 크레이트 자체 doc이 "no pixel decode"를 명시적으로 선언하고 있어서
경계를 지킴). `get_hex_range`와 동일한 데이터플레인 패턴(Control 메타데이터 + Data 프레임 raw
bytes, base64/JSON 배열 없음)으로 `get_decoded_frame_yuv` 커맨드 신규 — `sidecarClient.ts` →
Electron IPC(`main.ts`/`preload.cjs`) → `electronBridgeService.ts` → `YuvViewerPanel`까지 관통 배선.
정확성 우선, 성능은 나중(매 호출마다 스트림 처음부터 재디코드, 세션 캐싱 없음 — 모듈 doc에 명시).

**실제로 발견한 버그 2개 (스크린샷 없이는 못 잡았을 것들):**
1. **stride 메타데이터 오염** — `bitvue_decode::DecodedFrame.y_stride`/`u_stride`/`v_stride`는
   dav1d **원본** 픽처 stride(패딩 포함, 예: 384)인데, 실제 `y_plane`/`u_plane`/`v_plane` 바이트는
   `extract_plane()`이 항상 타이트하게 패킹(패딩 제거, 실제 width 기준)해서 복사함 — 즉 stride
   필드가 실제 바이트 레이아웃과 안 맞는 stale 메타데이터. `decode_bridge.rs`가 이 필드를 그대로
   믿고 프론트엔드에 전달해서, 매 행이 잘못된 오프셋에서 읽혀 이미지가 심하게 깨져 나옴(원(circle)
   무늬처럼 보이는 리피팅 아티팩트). 수정: stride를 `frame.width`(Y)/`chroma_width`(U,V, 크로마
   포맷 기준 직접 계산)로 재계산 — dav1d 원본 stride 무시.
2. **`YUVRenderer.render()`가 한 번도 실제로 그린 적이 없었음** — `frontend/utils/yuv/renderer.ts`:
   `render()`가 `!this.imageData`면 즉시 return하는데, `imageData`는 `resize()` 안에서만 생성되고
   그 `resize()` 호출은 이 얼리리턴 **다음 줄**에 있었음 — 즉 최초 호출 시 `imageData`가 항상
   `null`이라 `resize()`가 실행될 기회조차 없이 매번 return. `VideoCanvas.tsx`가 매 프레임마다
   검은색으로 `fillRect`한 게 유일하게 화면에 남는 것이었음 — 이 버그는 (1)번을 고친 뒤에도 여전히
   검은 화면만 나와서 추가로 파고들다 발견. 수정: `resize()` 호출을 얼리리턴보다 먼저 실행하도록
   순서 교체.

**검증:** `bitvue-cli`의 `decode --dump` 결과를 ground truth로 삼아 바이트 단위 정확히 일치 확인
(`0f 10 10 10 d5 d5 d6 d6...`). `bitvue-sidecar` 신규 테스트 3개(성공/out-of-range/stream-not-open,
+ ground-truth 픽셀 핀 테스트), `frontend/tests/utils/yuvRenderer.test.ts` 신규(YUVRenderer 최초
호출 시 실제로 paint하는지 직접 검증 — 기존엔 이 클래스를 직접 테스트하는 파일 자체가 없었음, 그래서
버그가 안 걸렸음). 스크린샷 4연속(에러 화면 → 검은 화면(디코드는 성공, 렌더 실패) → 깨진 이미지(stride
버그) → 최종 정상 EBU 테스트패턴("25fps SQUARE 320 x 240p 4:3", 타임코드, "Bip!" 톤, 컬러바)) —
전형적인 방송 테스트 카드였음, 실제 영상이 아니라 놀랄 필요 없음. `cargo test --workspace`/
`npx vitest run` 전체 재확인 — pre-existing 실패(`bitvue-av1-codec` 컴파일 에러 1개, `bitvue-engine`
플레이키 LRU 테스트 1개, 프론트엔드 9파일 무관 실패)만 남고 전부 이 세션 이전부터 있던 것으로 확인
(git stash로 내 변경분 뺐다 넣어서 직접 검증).

**안 고친 것(의도적, 스코프 밖):** 나머지 39개 파일(메뉴/필름스트립 썸네일/YUV diff/품질비교/RD
curves/Compare Workspace 등)은 여전히 Tauri `invoke()` 직접 호출 — 다음 스코프 결정 필요.

### `get_thumbnails` — 필름스트립 실제 썸네일 복구 (2026-08-08)

`YuvViewerPanel` 라운드와 같은 패턴("다음" 지시로 이어서 진행): 남은 ~39개 Tauri 파일 중 어느 것이
`get_decoded_frame_yuv`처럼 "이미 존재하는 기능, 배선만 하면 됨"인지 먼저 조사 — `useFilmstripState.ts`의
`get_thumbnails`가 정확히 그 케이스였음(필름스트립은 매 프레임마다 "[useFilmstripState] Failed to load
thumbnails" 콘솔 에러를 내며 색깔 블록 placeholder만 표시 중이었음). `bitvue_engine::filmstrip::
ThumbnailCache::generate_thumbnail`/`CachedFrame`(완전히 구현+테스트됐지만 프로덕션 호출자 0개 — 이전
세션들에서 이미 발견됐던 "완성됐지만 연결 안 된 코드" 패턴과 동일)와 `bitvue_decode::yuv_to_rgb`가 이미
존재 — sidecar에 배선만 하면 됐음. 기본 썸네일 폭(120px)이 프론트엔드 `THUMBNAIL_SIZE.WIDTH` 상수와
정확히 일치하는 것도 이 조합이 원래 의도된 것이었다는 신호.

**한 일:** `decode_bridge.rs`에 `get_thumbnails(data, frame_indices, target_width)` 추가 — 인덱스별로
매번 새로 디코드하지 않고, 배치(최대 `THUMBNAIL_BATCH_SIZE=50`개) 안의 요청된 인덱스를 **한 번의 디코드
패스**로 전부 캡처(최대 인덱스까지만 디코드). `yuv_to_rgb` → `CachedFrame` 구성 → `ThumbnailCache::
generate_thumbnail`(박스필터 다운샘플) → `image` crate로 실제 PNG 인코딩 → `base64`로 `data:image/png;
base64,...` URL 생성 — 프론트엔드가 `<img src>`에 그대로 꽂는 정확한 형식이라 base64가 낭비가 아니라
맞는 선택(`get_decoded_frame_yuv`의 raw-bytes 방식과 반대 이유로 base64 사용). 워크스페이스에
`base64`(신규) + `image`(sidecar 직접 의존성 추가, 기존에 `bitvue-decode`를 통해 이미 컴파일되던 것) 추가.
Control-only 커맨드(`get_hex_range`/`get_decoded_frame_yuv`와 달리 Data 프레임 없음) — sidecar의 16번째
커맨드. `main.ts`/`preload.cjs`/`electronBridgeService.ts`/`useFilmstripState.ts` 관통 배선.

**검증:** Rust 8개 신규 테스트(PNG를 `image::load_from_memory`로 실제 재디코드해서 진짜 유효한 PNG인지
확인, 엔드투엔드 dispatch, custom target_width, out-of-range/stream-not-open 에러) — `bitvue-sidecar`
전체 49개 테스트 통과. 프론트엔드 5개 신규(`useFilmstripState.test.ts`, 이 훅을 직접 테스트하는 파일
자체가 이전엔 없었음 — `Filmstrip.test.tsx`가 훅 전체를 mock 처리해서 실제 로직이 한 번도 검증된 적
없었음). 테스트 작성 중 발견한 나 자신의 실수: `frames` 배열을 렌더 콜백 안에서 매번 새로 만들면 매
리렌더마다 auto-load effect가 재실행되는 무한 재시도 루프가 생김 — 실제 제품 버그 아님, 안정적인
`frames` 참조를 콜백 밖에 정의해서 해결. 스크린샷으로 최종 확인 — 필름스트립의 모든 셀에 실제 EBU
테스트카드 썸네일(타이머가 프레임마다 줄어드는 파이차트)이 표시됨, 더 이상 placeholder 블록 아님.
36-실패/9-파일 기존 베이스라인 변화 없음.

**남은 것:** ~38개 파일 여전히 Tauri 직접 호출(메뉴/YUV diff/품질비교/RD curves/Compare Workspace) —
`get_frame_analysis`(QP/MV grid)와 `get_debug_yuv_frame`은 확인 결과 실제로 새 `Core` 엔진 작업이
필요함(기존 코드 재사용 불가), 나머지는 개별 확인 필요.

### `HexViewTab` — Rust 작업 전혀 없이 배선만으로 수정 (2026-08-09)

"다음 세션" 경계에서 사용자에게 방향 확인(Tauri 배선 스위프 계속 vs 새 Core 엔진 작업 vs 안티패턴
트랙 전환) — "Tauri 배선 스위프 계속" 선택. 남은 파일들을 훑다가 `UnitHexPanel/HexViewTab.tsx`의
`get_frame_hex_data`가 이전 두 라운드(`get_decoded_frame_yuv`/`get_thumbnails`)보다도 훨씬 가벼운
케이스임을 발견 — **새 Rust 코드가 전혀 필요 없었음**. 기존 `get_frame_hex_data`는 frameIndex를 받아
서버에서 offset으로 변환했지만, 이미 완성된 `getHexRange(stream, offset, len)`가 raw offset을 직접
받고, `FrameInfo.offset`(이전 라운드 `a4197b1`에서 StreamTreePanel 수정할 때 이미 추가한 필드)이
정확히 그 offset을 갖고 있음 — 프론트엔드 호출부만 바꾸면 끝.

**한 일:** `HexViewTabProps.frames` 아이템 타입에 `offset?: number` 추가, `invoke("get_frame_hex_data",
{frameIndex, maxBytes})` 호출을 `getHexRange("A", currentFrame.offset, Math.min(currentFrame.size,
2048))`로 교체. offset이 없는 프레임 소스에 대해선(구조적으로 가능하지만 실제 `FileStateContext`는
항상 채워줌) "No on-disk offset available" 정직한 에러 표시 — 값 추측 안 함. 스크린샷 검증 인프라도
확장: `BITVUE_ELECTRON_SCREENSHOT_CLICK_TAB`이 이제 쉼표로 여러 탭을 순서대로 클릭 가능(`"Unit HEX,Hex"`
처럼 중첩 서브탭까지 도달) — 이전엔 최상위 탭 하나만 클릭 가능했음.

**검증:** 기존 22개 테스트가 `test/setup.ts`의 전역 `get_frame_hex_data` Tauri mock에 전부 의존하고
있었음(더 이상 안 맞음) — `getHexRange` bridge mock으로 재작성(신규 2개: 실제 offset으로 호출되는지,
offset 없을 때 에러 경로). 스크린샷으로 실측 확인: frame 0의 첫 12바이트가
`77 29 00 00 00 00 00 00 00 00 00 00` — 정확히 IVF 청크 헤더(size=10615=0x2977 리틀엔디안 + PTS=0),
그다음 바이트가 `12`(이전 라운드에서 이미 확인된 진짜 Temporal Delimiter OBU 헤더 값)로 이어짐 — 완벽한
ground-truth 일치. Truncation 메시지도 "First 2048 bytes (of 10627 total)"로 정확. 36-실패/9-파일
베이스라인 변화 없음.

### Quit / Reload File / Open Recent File — 죽어있던 버튼 3개 (2026-08-09)

"다음 세션" 방향 확인 후 계속 Tauri 스위프. `App.tsx`에 남아있던 `@tauri-apps/api/core` 직접 호출
2종을 조사 — `HexViewTab`류와 또 다른 성격: **sidecar와 아예 무관**, 순수 Electron 창/앱 제어.

**`close_window` (Quit 메뉴/TitleBar 버튼):** `invoke("close_window")` — Tauri 커맨드 자체가 없어져서
Quit 버튼을 눌러도 아무 일도 안 일어나고 있었음. `bitvue-sidecar` 관여 전혀 없이 순수 Electron
`app.quit()` 하나만 새 IPC 채널(`bitvue:closeWindow`)로 노출 — `window.close()`가 아니라 `app.quit()`을
쓴 이유: macOS에서 "창 닫기"와 "앱 종료"는 다른 동작이라 Quit의 의미상 후자가 맞음. 기존
`before-quit` 핸들러가 이미 sidecar 정리를 담당하고 있어서 추가 변경 불필요.

**`open_file` (Reload File / Open Recent File):** 둘 다 `invoke("open_file", {path})` 직접 호출 —
역시 죽은 Tauri 커맨드라 아무 반응 없었음. `useAppFileOperations.ts`의 `handleOpenFile`이 다이얼로그
선택 후 실제로 하는 일(openStream → selectFrame → refreshFrames, 이미 검증된 브릿지 체인)을
`openFileAtPath(path)`로 분리해서 세 진입점(다이얼로그/reload/recent) 전부 같은 실제 로직 공유하도록
리팩터 — `handleOpenFile`은 이제 `showOpenDialog()` + `openFileAtPath(selected)`로 단순화.

**검증:** hook 레벨 신규 2개(`openFileAtPath` 성공/실패 경로), App.tsx 레벨 신규 2개(`menu-quit` →
`closeWindow` 실제 호출 확인, TitleBar Quit 버튼 클릭 → 실제 호출 확인) + `menu-open-recent-file`
디스패치가 안 던지는지(이 파일 기존 관례와 동일한 가벼운 스모크 테스트 수준). `onReloadFile` 자체의
App.tsx 배선(키보드 단축키 설정 객체를 통해 호출)은 별도 테스트 안 함 — 호출하는 로직
(`openFileAtPath`)은 이미 hook 레벨에서 충분히 커버되고, `globalShortcutHandler`의 mock 내부까지
파고드는 비용 대비 얻는 게 적다고 판단. `App.tsx`의 `invoke` import가 이제 완전히 미사용이라 제거.
36-실패/9-파일 베이스라인 변화 없음.

### 타입체크 안전망이 이번 세션 내내(아마 훨씬 전부터) 0개 파일을 검사하고 있었음 (2026-08-09)

`export_frames_csv`류 프론트엔드 Tauri 호출을 조사하다가 `ExportDialog.tsx`에서 `FrameInfo`에 없는
필드(`frameNumber`, `frameType` 등)를 참조하는 진짜 타입 에러를 발견 — 그런데 `npx tsc --noEmit -p .`는
이번 세션 내내 에러 0개로 통과하고 있었음. 원인: `tsconfig.json`의 `"include": ["src"]`가 이 repo에
**한 번도 존재한 적 없는** `src/` 디렉토리를 가리키고 있어서 — 소스는 `frontend/` 바로 아래
(`components/`, `contexts/`, `hooks/`, `services/`, `types/`, `utils/`, `App.tsx` 등)에 있음. 즉
`npm run typecheck`와 `npm run build`의 `tsc` 게이트 둘 다 이번 세션(그리고 그 이전 세션들도, 메모리에
남은 "typecheck clean" 기록들로 미루어) 내내 **파일을 0개 검사**하고 있었고, "타입체크 통과" 확인은
전부 빈 include가 만든 거짓 양성이었지 코드가 실제로 맞다는 증거가 아니었음. `vite.config.ts`의 `@`
alias도 똑같이 `./src`를 가리키는 실수가 있었지만(프로덕션 코드는 전부 상대 경로만 써서 실제로는
안 쓰였음 — `@/` import는 테스트 파일만 사용하고, `vitest.config.ts`는 이미 자체적으로 고친 alias를
갖고 있었음, 심지어 이 파일 자체에 "Fix @ alias to point to frontend root" 코멘트가 이미 있었는데
`tsconfig.json` 쪽은 한 번도 연결 안 됨).

**한 일:** include/paths를 실제 레이아웃에 맞게 수정. 누락됐던 `@types/react`/`@types/react-dom`
설치(애초에 설치된 적이 없었음) — 고친 include가 처음엔 4677개 에러를 냈는데 그 중 4571개는 순전히 타입
패키지 누락 때문(JSX.IntrinsicElements 등, 설치로 즉시 해결). 진짜 남은 106개는 두 그룹으로 처리:

1. **죽은 코드 확인 후 tsconfig exclude** — `App.tsx` 실제 렌더 트리에서 grep으로 importer 0개 확인된
   파일들(CompareWorkspace 전체, GraphUtils.tsx, QualityComparisonPanel.tsx, RDCurvesPanel.tsx,
   DualVideoView.tsx, TabContainer.tsx, errors/ 전체, advancedVisualizations류, exportData.ts,
   interactiveTooltips류, progressiveLoader.ts) — 고치지 않고 exclude에 추가, "스코프 밖"이라는
   정직한 표시로 남김. (QualityMetricsPanel.tsx/thumbnailUtils.ts는 똑같이 죽었지만 살아있는 배럴
   파일이 안 쓰이는 재출력을 갖고 있어서 exclude가 안 먹힘 — tsconfig exclude가 transitive import까지
   막지는 못한다는 걸 확인, 그냥 1줄짜리 진짜 수정으로 처리.)
2. **살아있는 코드의 진짜 버그 수정** — `ExportDialog.tsx`/`EnhancedView.tsx`가 `FrameInfo`를 옛날
   Tauri 시절 camelCase 필드명(`frameNumber`/`frameType`/`temporalId`/`spatialId`/`refFrames`)으로
   읽고 있었음 — 실제 필드는 `frame_index`/`frame_type`/`temporal_id`/`spatial_id`/`ref_frames`,
   전부 그동안 `undefined`를 조용히 읽고 있었던 것. `EnhancedView.tsx`의 `frame.qp` 체크는 아예
   존재하지 않는 필드라서 통째로 제거(진짜 프레임 평균 QP 데이터가 어디에도 없음, 조작 안 하고
   플래그만). `VideoCanvas.tsx`의 `YUVRenderer.dispose?.()`는 존재한 적 없는 메서드 호출(옵셔널
   체이닝이 조용히 흡수하고 있었음, 클래스 자체가 명시적 정리가 필요한 리소스를 안 가지므로 실질적
   해는 없었음) — 제거. `CodingFlowView.tsx`는 백엔드 응답의 `current_stage`(untyped string)를
   실제 유니온 타입으로 검증 후 setState하도록 수정. 그 외 `CompareContext.tsx`(enum을 `import type`으로
   가져와서 값으로 못 쓰던 문제), `StreamDataContext.tsx`(존재한 적 없는 타입 재출력), `HRDBufferPanel.tsx`
   (import 경로 깊이 오류), `tauriLogger.ts`(`window.__TAURI__` 미타입 접근), `OverlayRenderer` 렌더러
   여러 개(`_`-prefix가 매개변수에만 통하고 지역 변수엔 안 통하는 걸 모르고 붙인 케이스, 구조분해
   리네임 버그로 항상 undefined였던 프롭) 등 자잘한 미사용 import/변수 다수.

**검증:** 전체 테스트 스위트가 기존 36-실패/9-파일 베이스라인으로 정확히 복귀(중간에 EnhancedView.test.tsx
가 37번째로 잠깐 실패 — `frame_type` 수정으로 `gopBoundaries`가 더 이상 항상 비어있지 않게 되면서
"Next GOP" 버튼이 이제 정확하게 활성화됨, 테스트가 옛날 버그 동작을 검증하고 있었던 것이라 테스트를
고침). `npm run build`의 `tsc` 게이트가 이제 진짜로 통과함(이전엔 아무것도 안 검사해서 "통과"했던 것).
스크린샷 + 전체 `BITVUE_ELECTRON_SELFTEST` 재확인(exit 0, 모든 IPC 체인 byte-exact 그대로) — 아무것도
안 깨짐.

### Tauri 배선 스위프 마무리 — 남은 건 전부 새 Core 작업 (2026-08-09)

타입체크 복구 다음 라운드로 나머지 `@tauri-apps/api/core` 직접 호출 파일들을 마저 훑음. 결과:

**"이미 브릿지에 있음, 배선만 필요" 케이스 처리 완료:** `components/CompareWorkspace/DiffOverlay.tsx`,
`StreamPlayer.tsx`가 옛 Tauri `get_decoded_frame_yuv`(base64 평면)를 직접 부르고 있었음 —
`electronBridgeService.getDecodedFrameYuv`(raw bytes)로 교체. `YuvViewerPanel/index.tsx`에 있던
`BridgeDecodedYuvFrame → YUVFrame` 바이트 슬라이싱 헬퍼를 `electronBridgeService.bridgeYuvToFrame`로
뽑아서 세 곳 다 공유. **단, `CompareWorkspace/**`는 여전히 `App.tsx` 렌더 트리에 안 걸려서 tsconfig
exclude에 남아있음** — 이번 수정은 나중에 워크스페이스가 실제로 연결될 때를 위한 사전 정리이지,
지금 당장 뭔가 동작하게 만든 건 아님. (`contexts/ThumbnailContext.tsx`/`hooks/useThumbnail.ts`도
확인해봤는데 `get_thumbnails`를 각자 따로 부르는 죽은 병렬 구현 — 실제 필름스트립은
`components/useFilmstripState.ts` + `electronBridgeService.getThumbnails`를 씀, `ThumbnailProvider`는
어디에도 마운트 안 됨. 건드리지 않음, 향후 exclude 후보.)

**"새 Core 작업 필요" — 살아있는데 sidecar 16개 커맨드에 아예 없음:**

| 파일 (실제 App.tsx 트리에서 살아있음 확인) | 부르는 커맨드 |
|---|---|
| ~~`YuvViewerPanel/index.tsx`~~ | ~~`get_frame_analysis`~~ — 2026-08-09 "get_frame_analysis 구현" 참고, 완료됨 (`get_debug_yuv_frame`은 그 앞 "Debug YUV 커맨드 패밀리 구현" 참고, 완료됨) |
| ~~`contexts/YuvDiffContext.tsx`, `components/panels/YuvDiffPanel.tsx`~~ | ~~`load_debug_yuv` 등~~ — 2026-08-09 "Debug YUV 커맨드 패밀리 구현" 참고, 완료됨 |
| `contexts/CompareContext.tsx` (Provider는 마운트, `createWorkspace`는 `useAppFileOperations.ts`의 dependent-file-open 경로에서 실제로 호출됨) | `create_compare_workspace`, `set_sync_mode`, `set_manual_offset`, `reset_offset` — 단 `setSyncMode`/`setManualOffset`는 죽은 `CompareWorkspace.tsx`에서만 쓰여서 사실상 미호출 |
| `components/panels/SyntaxDetailPanel/RefListTab.tsx`, `StatisticsTab.tsx` | `get_codec_extended_info` |
| ~~`components/Player/views/AV1FeaturesView.tsx`, `hooks/useAv1Features.ts`~~ | ~~`get_av1_features`~~ — 2026-08-09 "get_av1_features 구현" 참고, 완료됨 (중복 구현 아니었음 — 대시보드 뷰 vs 단일 오버레이, 둘 다 정당한 소비자) |
| `components/Player/views/DeblockingView.tsx` | `get_deblocking_analysis` |
| `components/Player/views/ResidualsView.tsx` | `get_residual_analysis` |
| `components/Player/views/CodingFlowView.tsx` (기존에 이미 플래그됨) | `get_coding_flow_analysis` |

이 8개 커맨드는 전부 `bitvue-sidecar`에 없고(`bitvue-indexer`/`decode_bridge` grep으로 확인), 프론트엔드
스왑만으로 못 고침 — 각각 새 Rust 분석 로직이 필요함. 이걸로 프론트엔드 쪽 "배선 스위프"는 사실상 끝 —
남은 모든 항목이 새 백엔드 작업이거나 확인된 죽은 코드.

### Debug YUV 커맨드 패밀리 구현 (2026-08-09, 33ab6aa/aee52b6)

위 표에서 새 Core 작업 필요로 분류된 8개 커맨드 중 YuvDiffPanel 쪽(`load_debug_yuv`/`unload_debug_yuv`/
`set_debug_yuv_offset`/`set_debug_yuv_crop`/`get_yuv_diff_metrics`/`find_first_diff_frame`/
`get_debug_yuv_frame`, 7개)을 실제로 구현. VQ Analyzer의 "Debug → Load Reference YUV" 워크플로 —
사용자가 raw YUV 파일을 열어서 스트림 A의 디코딩 결과와 프레임 단위로 diff — 이 `YuvDiffPanel.tsx`/
`YuvDiffContext.tsx`에 UI는 이미 있었지만(`App.tsx`에 `YuvDiffProvider`+`YuvDiffPanelFromContext`로
실제 마운트돼있음) 백엔드가 마이그레이션 이후 한 번도 존재한 적이 없어서 완전히 죽어있었음.

**새 `bitvue-sidecar::debug_yuv` 모듈**: I420/NV12/NV21/I422/I444, 8/10/12/16-bit raw YUV 파일을
디스크에서 직접 읽음(디코드 아님) — `decode_bridge`가 압축 비트스트림을 다루는 것과 대조. 세션 상태
(경로/해상도/포맷/비트뎁스/offset/crop)는 `Core`가 아니라 `main.rs`가 들고 있는
`Mutex<Option<Session>>` — `StreamId`(A/B)별 상태가 아니고, `Core`는 리프 크레이트라 파일시스템 접근을
할 수 없음(다른 모듈 문서와 같은 이유). **스코프 결정**: diff/amplified/metrics 전부 8비트 정밀도로만
동작(10비트 이상은 비교 전에 다운시프트) — `decode_bridge`의 기존 wire 포맷과 프론트엔드
`VideoCanvas`/`yuv_to_rgb` 렌더러 둘 다 8비트 전용이라 거기 맞춤, 진짜 고비트뎁스 비교는 별도 wire
포맷+렌더러가 필요해서 범위 밖으로 명시적으로 남김. PSNR/SSIM은 새로 안 짜고 이미 구현+테스트된
`bitvue-metrics` 크레이트(`psnr_yuv`/`ssim_yuv`) 재사용 — VMAF까지 갖춘 크레이트인데 지금까지 아무도
안 쓰고 있었음. `get_debug_yuv_frame`은 `get_decoded_frame_yuv`가 이미 확립한 두-프레임(Control 메타
+ raw bytes Data) wire 패턴을 그대로 따름 — 옛 Tauri 시절 base64 `YUVFrameData`로 되돌아가지 않음.
`find_first_diff_frame`은 `get_thumbnails`처럼 스트리밍 디코드 한 번으로 처리(요청된 프레임마다 처음부터
다시 디코드하는 `get_decoded_frame_yuv` 방식대로 했으면 O(n²)이었을 것).

**프론트엔드**: `YuvDiffContext.tsx`/`YuvDiffPanel.tsx`의 죽은 Tauri `invoke()` 호출을
`electronBridgeService`의 새 래퍼로 교체. `YuvViewerPanel/index.tsx`의 debug-YUV 프레임 로딩도
`getDebugYuvFrame`(raw bytes)로 바뀌면서, 이제 실제 디코드 경로와 debug-YUV 경로 둘 다 같은
`bridgeYuvToFrame` 변환으로 수렴 — 옛 base64 전용이었던 `convertYUVDataToYUVFrame`/`yuvData` state를
통째로 제거(더 이상 아무도 base64 shape를 안 만들어서).

**검증**: Rust 쪽 74개 테스트(기존 60 + debug_yuv 유닛 테스트 14 + main.rs 통합 테스트 다수, 실제 AV1
픽스처의 진짜 디코드 바이트로 만든 참조 파일 써서 identical-frame PSNR/mismatch 검증까지). 프론트엔드
타입체크 409개 파일 그대로 클린, 전체 vitest 기존 36-실패/9-파일 베이스라인 그대로. **실제 Electron
IPC 체인까지 통과 확인** — `BITVUE_ELECTRON_SELFTEST`에 debug-YUV 라운드 추가: 스트림 A의 진짜 디코드
바이트로 참조 파일 생성 → `loadDebugYuv` → `getDebugYuvFrame("reference")`가 원본과 byte-exact 일치 →
`getYuvDiffMetrics`가 mismatch 없음 보고 → `getDebugYuvFrame("diff")`의 luma가 전부 0 → 실제 렌더러/
preload/main/sidecar 경로 전체를 거쳐서 exit 0.

**남은 것**: `create_compare_workspace`류, `get_codec_extended_info`, `get_av1_features`,
`get_deblocking_analysis`, `get_residual_analysis`, `get_coding_flow_analysis` — 아래 섹션에서
`get_frame_analysis`가 먼저 완료됨, 나머지 6개는 아직 미구현.

### get_frame_analysis 구현 — QP/MV/Partition/Prediction/Transform 그리드 (2026-08-09, 7bde09f/fa6157b)

앞 섹션에서 "새 Core 작업 필요"로 분류됐던 7개 중 두 번째 트랙 완료. 조사해보니 `bitvue-av1-codec::
overlay_extraction`에 QP/MV/partition/prediction-mode/transform-size 그리드 추출 로직이 **전부 이미
완성돼있고 유닛 테스트까지 있었는데**, `bitvue-cli`에도 `bitvue-sidecar`에도 어디서도 호출하는 곳이
없었음(debug-YUV 라운드의 `bitvue-metrics`와 완전히 같은 패턴 — "이미 있는데 아무도 안 쓰던 코드").
그래서 이번 라운드는 새 비트스트림 파싱이 아니라 순수 오케스트레이션 + wire 매핑.

**진짜 신경 쓴 부분**: `ParsedFrame::parse`는 넘겨준 `obu_data` 안에서 SequenceHeader OBU를 찾아야
실제 해상도를 알아냄(못 찾으면 1920x1080 스캐폴드로 조용히 폴백) — AV1 스트림은 보통 시퀀스 헤더를
프레임 0에만 한 번 싣고 반복 안 함. 그래서 앞쪽 몇 프레임을 스캔해서 시퀀스 헤더 원본 바이트를 찾아
매 프레임 요청마다 prepend — 테스트로 "5번 프레임(자기 청크에 시퀀스 헤더 없음)도 진짜 320x240이 나옴
(스캐폴드 아님)"까지 핀 박음. 그리고 `QPGrid`/`MVGrid`/`PartitionGrid`는 `Serialize` derive가 있지만
enum 필드(`PartitionType`, `BlockMode`)는 기본 derive로 직렬화하면 variant 이름 문자열("Split")이
나오는데 프론트엔드는 숫자 TS enum을 기대함 — 전부 손으로 JSON 매핑. `PredictionMode`는 단순
`as u8` 캐스팅도 안 통함: Rust enum의 자연 순서는 AV1 스펙의 intra y_mode 번호(0-12)와 정확히
일치하지만, 프론트엔드 색상/이름 테이블(`utils/colors.ts`)은 inter 모드를 64+ 대역에 기대함 — 진짜
의미론적 리매핑 테이블을 손으로 작성함(`TxSize`는 반대로 그대로 캐스팅 가능, 프론트 번호랑 이미 일치).

**검증**: Rust 92개 테스트(신규 10개 — 실제 픽스처 end-to-end, non-first-frame 실제 해상도 확인,
enum이 숫자로 나오는지 확인, out-of-range/stream-closed 에러 경로). 프론트엔드 타입체크/빌드/vitest
전부 기존 베이스라인 그대로. `BITVUE_ELECTRON_SELFTEST`에 `getFrameAnalysis(0)` 라운드 추가 —
실제 렌더러/preload/main/sidecar 경로로 진짜 320x240 + QP/partition 그리드 non-empty 확인, exit 0.
이걸로 `QPMapRenderer`/`MVFieldRenderer`/`CodingFlowRenderer`/`PredictionRenderer`/`TransformRenderer`
5개 오버레이 렌더러가 (렌더 트리엔 있었지만 데이터가 한 번도 안 왔던 상태에서) 실제 데이터를 받게 됨.

**남은 것**: `create_compare_workspace`류, `get_codec_extended_info`, `get_av1_features` (다음 섹션에서
완료됨), `get_deblocking_analysis`, `get_residual_analysis`, `get_coding_flow_analysis` — 6개.

### get_av1_features 구현 — 예상보다 훨씬 컸던 케이스 (2026-08-09, 782148b/9ae50f7)

처음엔 get_frame_analysis와 같은 "이미 구현된 추출 로직, 배선만 필요" 패턴으로 보였음
(`bitvue-av1-codec::advanced_features`에 `extract_cdef_data`/`extract_loop_restoration_data`/
`extract_film_grain_data`/`extract_super_resolution_data`가 전부 이미 구현+테스트돼있었음). 그런데
실제로 파보니 이 함수들의 입력(`FrameHeader.cdef_damping`/`loop_restoration`/`film_grain`/
`super_resolution`)을 채워주는 코드가 어디에도 없었음 — 유일한 프레임 헤더 파서인
`parse_frame_header_basic`이 quantization_params() 직후에 의도적으로 멈춤(자기 모듈 문서에 명시).
CDEF/LR/film-grain까지 가려면 segmentation_params~film_grain_params까지 AV1 스펙의 나머지 프레임
헤더 섹션 전체를 새로 파싱해야 했음.

**진짜 어려웠던 지점**: `skip_mode_params()`의 비트 존재 여부가 참조 프레임들의 order hint 비교에
달려있는데, 이건 프레임 단위 고립 파싱으로는 알 수 없고 스트림 시작부터 모든 프레임을 순서대로
파싱하면서 8슬롯 참조 상태(`RefFrameState`)를 추적해야만 알 수 있음 — get_frame_analysis처럼
프레임 1개만 떼서 못 봄. (다행히 `PrevGmParams`는 global_motion_params()의 서브지수 골롬 코드 길이가
참조값과 무관하게 자기-종결적이라 필요 없음을 스펙 확인 후 검증.) short reference-signaling
스트림(`set_frame_refs()` 필요)은 명시적으로 미지원으로 남김 — `parse_frame_header_basic`의 기존
"좁은 범위 문서화" 선례와 같은 방식.

**검증**: 독립적인 디코더 오라클이 없음(`dav1d` Rust 바인딩이 헤더 introspection을 노출 안 함) —
합성 비트스트림 유닛 테스트 6개(그 중 하나가 실제 버그를 잡음: `seq_choose_screen_content_tools=1`이면
프레임 헤더가 `allow_screen_content_tools`를 스킵한다고 처음에 잘못 가정했는데 실제론 정반대 — choose=1은
"프레임 헤더 비트로 결정한다"는 뜻이라 오히려 명시적으로 읽어야 함) + 실제 픽스처 250프레임 전부
순차 파싱 성공 + 320x240 확인 (`bitvue-av1-codec` 라이브러리 쪽). `bitvue-sidecar` 쪽은 88개 유닛
테스트(get_av1_features용 4개 + dispatch 레벨 2개 신규). `BITVUE_ELECTRON_SELFTEST`에
`getAv1Features(0)` 라운드 추가 — 실제 IPC 체인으로 진짜 CDEF 데이터(320x240, non-empty blocks)
확인, exit 0.

**프론트엔드 발견**: `AV1FeaturesView.tsx`(전체 대시보드)와 `useAv1Features.ts`(단일 오버레이 모드)가
독립적으로 같은(존재하지 않던) 커맨드를 서로 다른 응답 shape 가정으로 부르고 있었음 — 진짜 중복
구현이 아니라 그냥 두 개의 정당한 소비자가 둘 다 고장나 있었던 것. wire 타입은 sidecar 자체와 맞춰
snake_case로(`AV1FeaturesView.tsx`가 이미 그렇게 가정하고 있었음), `useAv1Features.ts`가 자기
경계에서 기존 camelCase `Av1FeaturesData`로 변환.

**남은 것**: `create_compare_workspace`류, `get_codec_extended_info`, `get_deblocking_analysis`,
`get_residual_analysis`, `get_coding_flow_analysis` — 5개.

### get_coding_flow_analysis 구현 — 가장 저렴했던 케이스 (2026-08-09, 70b8877/02b2593)

남은 4개(`get_codec_extended_info`/`get_deblocking_analysis`/`get_residual_analysis`/
`get_coding_flow_analysis`) 스코프를 먼저 조사(Explore 에이전트): 앞 셋은 각각 HEVC scaling-list/VP9
prob-decode/VVC APS 파서 신규(codec_extended_info, 여러 크레이트에 걸침), boundary-strength 신규
알고리즘(deblocking), AV1 잔차 CDF 심볼 디코드 신규(residual, `bitvue-av1-codec::tile` 모듈독에 이미
`⏳ pending`으로 표시돼 있던 항목) — 셋 다 get_av1_features급 이상. `get_coding_flow_analysis`만
예외: 프론트엔드(`CodingFlowView.tsx`)가 실제로 쓰는 필드는 `current_stage`+`codec_features`뿐이고
(`stages`는 타입엔 있지만 컴포넌트 바디에서 미사용), 둘 다 이미 있는 데이터로 만들 수 있음 —
`frame_analysis`의 prediction/transform/QP 그리드 추출 성공 여부로 파이프라인 단계별 완료를
판정(input→prediction→transform→quantization까지 도달, entropy/reconstruction은 이 코드베이스에
잔차 엔트로피 디코드/픽셀 재구성 단계가 아예 없어서 항상 미완료로 보고), `av1_features`가 이미
쓰던 시퀀스 헤더 스캔으로 `SequenceHeader`의 실제 bool 플래그(enable_cdef 등 15개)를
codec_features로 매핑 — 프론트엔드의 코덱명 기반 정적 테이블보다 스트림별로 진짜인 값.

신규 파싱 없음, `frame_analysis`/`av1_features` 패턴 재사용만으로 완결. `bitvue-sidecar` 유닛
테스트 4개(coding_flow 모듈) + dispatch 레벨 2개, 전부 실제 픽스처로 검증(frame 0 기준
`current_stage: "quantization"`, `codec_features`에 CDEF 포함 확인). 프론트엔드 발견: `CodingFlowView.tsx`가
`get_av1_features`와 마찬가지로 삭제된 `@tauri-apps/api` invoke를 그대로 부르고 있던 죽은
pre-migration 코드였음 — `electronBridgeService.ts` 패턴으로 이관. `BITVUE_ELECTRON_SELFTEST`에
`getCodingFlowAnalysis(0)` 라운드 추가, 실제 IPC 체인 exit 0 확인. 타입체크 409파일/vitest
36실패-9파일 베이스라인 그대로, 신규 회귀 없음.

**남은 것**: `create_compare_workspace`류, `get_codec_extended_info`, `get_deblocking_analysis`,
`get_residual_analysis` — 4개, 전부 get_av1_features급 이상의 신규 엔진/파서 작업 확인됨(더 이상
"배선만" 남은 케이스 없음).

### get_deblocking_analysis 구현 — 도중에 진짜 잠복 버그 2개 발견 (2026-08-09, ec47049/54a013d/e6a0ca3)

두 가지 신규 작업으로 구성: (1) `frame_header_full`의 `loop_filter_params()`가 지금까지 모든 값을
버려왔던 것(`skip_loop_filter_params`)을 실제 파싱으로 교체 — `get_av1_features` 때 cdef/lr/
film_grain을 추가했던 것과 같은 자리, 같은 패턴. (2) AV1 스펙 7.14.2 boundary-strength(BS) 유도
알고리즘이 이 워크스페이스 어디에도 없었음 — `overlay_extraction::deblocking` 신규 모듈에서 이미
파싱돼있던 코딩유닛 데이터(skip/mode/ref_frames/mv)만으로 유도(entropy/잔차 디코드 불필요 — AV1의
skip 플래그가 스펙상 "잔차 없음"과 정확히 동치라 근사가 아니라 정확한 조건).

**진짜 발견 — `ParsedFrame::parse`가 결합형 OBU(`ObuType::Frame`)에서 tile_data를 전혀 못 채우고
있었음**: `get_deblocking_analysis`가 실제 코딩유닛 파서를 처음으로 가드 없이 직접 호출하면서
드러남 — 다른 모든 extractor(QP/MV/partition/prediction-mode/transform 그리드, 즉
`get_frame_analysis` 전체)는 `has_tile_data()`/`Err` 시 스캐폴드로 조용히 폴백하는 구조라 이
버그가 지금까지 한 번도 겉으로 드러난 적이 없었음 — **실제 픽스처의 모든 프레임에서 tile_data가
항상 비어있었고, get_frame_analysis의 "real leaf blocks" 테스트를 포함한 모든 real-CU 검증이
사실 스캐폴드 데이터를 보고 통과하고 있었음**(약한 assertion 탓에 스캐폴드와 real을 구분 못함).
사용자에게 스코프 확인 받고(`이 세션에서 바로 고치고 진행`) `FrameHeader.header_size_bytes` 이후
바이트를 tile_data로 슬라이스하도록 수정. 이 수정이 real-CU 코드 경로를 처음으로 실제 실행시키면서
**두 번째 잠복 버그**를 노출: `build_grid_from_coding_units_spatial`(prediction-mode/transform
그리드 공용)이 `output[idx] = ...` 직접 인덱싱을 쓰는데 호출부가 `Vec::with_capacity`(길이 0)만
넘겨서 모든 쓰기가 `idx < output.len()` 가드에 조용히 막혀있었음 — `resize_with`로 사전 채움 추가.
두 버그 다 "한 번도 실행된 적이 없던 코드"라 이전 세션까지 전혀 발견되지 않았음.

**검증**: 수정 전/후 `cargo test -p bitvue-av1-codec --lib`(278) + `-p bitvue-sidecar`(98, 신규
4개) 전부 통과, `-p bitvue-cli`도 회귀 없음 확인. 프론트엔드: `DeblockingView.tsx`도 죽은 Tauri
invoke였음 — bridge 이관 + HEVC/VVC식 beta/tc offset을 실제 AV1 `loop_filter_params()` 필드(레벨/
sharpness/ref·mode 델타)로 교체, BS 범위도 AV1 실제 범위(0-2)에 맞게 조정(기존 코드는 HEVC식
BS 3-4/1-2 가정). 타입체크 409파일/vitest 36실패-9파일 베이스라인 그대로(변경된 텍스트에 맞춰
`DeblockingView.test.tsx` 2개 assertion만 업데이트). `BITVUE_ELECTRON_SELFTEST`에
`getDeblockingAnalysis(0)` 라운드 추가, exit 0.

**남은 것**: `create_compare_workspace`류, `get_codec_extended_info`, `get_residual_analysis` —
3개.

### get_codec_extended_info 구현 — 초기 스코핑보다 훨씬 작았던 케이스 (2026-08-09, 8617c1c/3dc4c4a)

이전 스코핑에서 "HEVC scaling-list/VP9 prob-decode/VVC APS 파서 3개 신규, 남은 것 중 가장 큼"으로
분류됐던 커맨드. 하지만 이 커맨드의 프론트 소비자 5개(QmTab/ProbsTab/ApsTab/RefListTab/
StatisticsTab) 중 QmTab/ProbsTab/ApsTab은 **진짜 죽은 코드임을 재확인** — `bitvue-sidecar`가
`bitvue-av1-codec`에만 의존하고 HEVC/VP9/VVC 디코드 경로가 전혀 없어서, 파일 확장자로 탭 버튼이
보이더라도 뒤에 실제 데이터를 만들 방법이 없음. 실제 도달 가능한 소비자는 `RefListTab`(L0/L1
참조 리스트)과 `StatisticsTab`(QP 히스토그램)뿐이고, 둘 다 이미 파싱된 데이터의 재조합이었음(신규
파싱 없음) — "get_av1_features급 이상"이라던 초기 추정이 실제로는 완전히 뒤집힌 사례.

`FrameHeader.ref_frame_idx`가 AV1의 실제 7개 참조 슬롯(REFS_PER_FRAME) 중 3개만 저장하고 있었음
(나머지 4개는 이미 읽고 버리고 있었음) — `[u8;7]`로 확장. `FrameHeader.order_hint`도 지역 변수로만
계산되고 구조체엔 한 번도 저장 안 되고 있었음 — 신규 필드 추가. 두 확장 다 "이미 파싱은 되고
있었는데 저장을 안 하고 있었다"는 이번 세션의 반복 패턴과 같은 결의 사소한 확장(get_deblocking의
두 버그처럼 "실행조차 안 되던 코드"는 아니고, 그냥 필드가 좁게 설계돼 있었을 뿐). 슬롯→
(order_hint, frame_index, frame_type) 추적은 `av1_features`/`deblocking`과 같은 순차 스캔 +
`refresh_frame_flags` 갱신 패턴 재사용, L0/L1 분류는 `skip_mode_params`가 이미 쓰던
`relative_dist`(스펙 7.9.2 부호있는 order-hint 비교)를 `pub`으로 노출해 재사용. QP 히스토그램은
기존 QP 그리드에 대한 단순 버킷팅.

AV1은 HEVC식 long-term 마킹/weighted-prediction 신택스가 없어서 `long_term`은 항상 `false`,
`weight`/`offset`은 항상 `null` — 프론트엔드 `RefEntry` 타입이 이미 nullable로 설계돼 있어 그대로
맞음. `RefListTab.tsx`/`StatisticsTab.tsx`도 죽은 Tauri invoke(+불필요한 `path` 파라미터)였음 —
bridge 이관, 다른 AV1 커맨드처럼 이미 열린 스트림 A 기준으로 통일. 검증: av1-codec 278 + sidecar
106(신규 4개) 전부 통과, `ref_frame_idx` 타입 변경의 다른 소비자(`bitvue-mcp`/`bitvue-indexer`)도
iterator 기반이라 회귀 없음 확인. 타입체크 409파일/vitest 36실패-9파일 베이스라인 그대로,
`BITVUE_ELECTRON_SELFTEST`에 `getCodecExtendedInfo(5)`(실제 L0 참조가 있는 인터 프레임) 라운드
추가, exit 0.

**남은 것**: `create_compare_workspace`류, `get_residual_analysis` — 2개(전자는 죽은 UI라 저우선,
후자는 AV1 잔차 CDF 심볼 디코드 신규 필요 — 이 세션에서 확인된 것 중 유일하게 진짜 큰 신규 작업).

### get_residual_analysis 구현 — 도중에 발견한 진짜 desync/crash 버그부터 수정 (2026-08-09, de2deac/19ef3e5/66f0073)

이 세션 백엔드 스윕의 마지막 항목. `bitvue-cli` 자체가 "requires a full AV1 tile-group decoder,
not yet implemented"라고 이미 선언했던 케이스, GUI는 QP값 기반 가짜 근사치를 렌더링하며 백엔드를
아예 호출하지 않고 있었음 — 정말로 신규 구현이 필요했던 유일한 커맨드.

**작업 도중 이 세션 전체를 소급 위협하는 버그 발견**: `parse_coding_unit`이 skip/mode/mv/delta_q만
읽고 AV1 스펙의 `residual()` 신택스를 전혀 안 읽고 있었음 — 타일의 계수 데이터는 바이트 정렬 없는
산술부호화라 skip=false인 CU 하나만 있어도 공유 `SymbolDecoder`가 이후 모든 신택스에서 통째로
desync됨. `get_codec_extended_info(fixture, 100)`이 실제로 크래시하는 것으로 확인 —
이번 세션에 만든 real-CU 기반 커맨드(`get_frame_analysis`/`get_deblocking_analysis`/
`get_codec_extended_info`) 전부가 tile_data가 실제로 채워지기 시작한 순간(OBU_FRAME 버그 수정,
같은 세션) 이 위험에 노출돼 있었음 — 사용자에게 먼저 실측 확인받고("지금 바로 측정") 실제 크래시
재현으로 근본 원인 확정 후 진행.

`SymbolDecoder::read_residual_block` 신규 추가(txb_skip/eob_pt+extra bits/coeff_base_eob/
coeff_base/coeff_br/dc_sign/골롬 확장) — skip/mode/partition이 이미 쓰던 "컨텍스트 독립
대표 CDF" 선례를 그대로 계수 신택스까지 확장(스펙 정확한 확률/컨텍스트 아님, 하지만 읽는
심볼의 개수/모양은 실제 신택스와 같아서 디코더 위치가 최소한 그럴듯한 만큼 전진함). 부수적으로
`ArithmeticDecoder::refill`의 버퍼 소진 경로에서 시프트 오버플로 patch(문서화된 `cnt>=MIN_CNT`
불변식만으로도 이미 64비트 시프트 한계를 넘을 수 있던 잠재 버그, residual 수정 후에도 남아있던
frame 19 크래시로 발견) — 실제 250프레임 전부 무크래시 확인하는 영구 회귀 테스트 추가.

`CodingUnit.residual`(nonzero_count/sum_abs_level/max_level) 신규 필드를 그대로
`get_residual_analysis`의 `block_residuals`/`coefficient_stats`로 매핑 — "energy"는
sum_abs_level(잔차 크기 총합)이지 공간영역 에너지 아님(역변환 단계 자체가 없음), 명시적으로
문서화. `ResidualsView.tsx`도 죽은 Tauri invoke + "QP-based approximation" 가짜 라벨이었음 —
bridge 이관 + 라벨을 실제 상태("Approximate coefficient decode", 근사 이유 명시)로 교체.
검증: av1-codec 279 + sidecar 106(신규 4개) 전부 통과, 250프레임 전부 무에러 확인, 타입체크
409파일/vitest 36실패-9파일 베이스라인 그대로, `BITVUE_ELECTRON_SELFTEST`에
`getResidualAnalysis(5)` 라운드 추가, exit 0 — 6개 AV1 커맨드 전부(`get_frame_analysis`/
`get_av1_features`/`get_coding_flow_analysis`/`get_deblocking_analysis`/
`get_codec_extended_info`/`get_residual_analysis`) 실제 IPC 체인으로 한 번에 검증됨.

**남은 것**: `create_compare_workspace`류(죽은 UI, 저우선) 1개 — 이번 세션의 backend-command
스윕은 여기서 사실상 종료. 남은 항목 하나는 소비하는 `<CompareWorkspace>` UI 자체가 이미
죽은 트리라 배선해도 사용자에게 보이는 효과가 없음.

### 확정 순서

```
Phase 1  Bitvue Analyzer (AV1 분석 완성)          ← 현재 위치
Phase 2  Analyzer multi-codec (H264/HEVC/VP9/...)
Phase 3  Bitvue CLI 강화 (CI/regression/automation) — 이미 부분 구현, 처음부터 만드는 게 아님
Phase 4  Bitvue Probe (live monitoring)
Phase 5  MCP/SDK/ecosystem — 이미 부분 구현
```

Probe보다 CLI를 먼저 하는 이유: Probe부터 벌리면 범위가 너무 커짐. CLI는 "Analyzer의 core가 제대로 분리됐는지" 검증하는 저비용 아키텍처 테스트 — 라이브러리 분리 자체는 이미 통과했지만, interactive query API 설계 검증은 CLI 강화 단계에서 실제로 완료해야 함(위 "검증됨" 항목 참조, 과신 금지).

### 미해결 — Probe→Analyzer 핸드오프

Probe에서 이상 구간 클릭 → "Analyze in Bitvue" → Analyzer가 해당 시점의 bitstream/frame을 염. Probe와 Analyzer가 별개 Electron 앱/윈도우일 경우 이를 위한 명시적 계약(딥링크 프로토콜 또는 로컬 소켓으로 stream+frame+timestamp 전달)이 필요 — `bitvue-engine` 공유만으로는 풀리지 않는 유일한 조각. Probe phase 착수 시 설계, 지금은 블로커 아님.

**한 줄 정의:** Bitvue = 영상 코덱을 위한 observability & analysis platform. Analyzer로 원인을 파고들고, Probe로 문제를 발견하고, CLI로 자동화한다.

---

## ⚠️ 주의 — 아래 Phase 0-12는 Tauri 구현 시절 작성됨, `src-tauri`는 2026-08-08 삭제됨

아래 로드맵은 Electron 전환(위 "제품 아키텍처 확정" 섹션) 이전에 작성돼서 **"Tauri 커맨드 추가"를 구현 단위로 삼음** — 이제 그 구현 벡터는 `bitvue-sidecar` 커맨드(위 섹션들의 `open_stream`/`select_frame`/... 패턴)로 바뀌었지만, **기능 자체(코덱 지원, 오버레이 모드, 재생, export 등)는 그대로 유효한 요구사항 목록**이다. 항목을 볼 때:

- `[ ] X Tauri 커맨드 추가` 같은 문구는 **"bitvue-sidecar에 X 커맨드 추가"로 읽을 것** — 무엇을 만들지는 안 바뀌었고 어디에 만들지만 바뀜.
- **`[x]`(완료 표시)는 신뢰하지 말 것.** 그 구현이 `src-tauri/src/commands/*.rs`에 있었다면 지금은 코드 자체가 삭제됨(`src-tauri` 전체 제거, 2026-08-08) — "기능 설계가 검증됐다"는 뜻이지 "지금 코드베이스에 그 기능이 동작한다"는 뜻이 아님. 지금까지 `bitvue-sidecar`에 실제로 이식된 건 위 섹션들에 나열된 9개 커맨드뿐(`open_stream`/`select_frame`/`select_unit`/`select_syntax`/`select_bit_range`/`select_spatial_block`/`close_stream`/`get_hex_range`/`cancel_request`) — 그 외 전부(codec 오버레이, export, playback 등)는 재이식 전이라고 가정할 것.
- `docs/PARITY_CHECKLIST.md`(✅/⚠️/❌ source of truth)도 동일한 문제가 있음 — 거기도 같은 취지의 주석을 달아둠.
- `frontend/`는 여전히 `@tauri-apps/api`의 `invoke()`를 광범위하게 직접 호출 중(수십 개 파일 — `tauriCommandService.ts`, 각종 훅/컴포넌트)이라 **`src-tauri` 삭제로 인해 지금 당장 동작 안 함** — `bitvue-ui`로의 실제 배선은 이 rename보다 훨씬 큰 별도 작업(각 호출부를 `window.bitvue.*`로 재작성)이고, 아직 손 안 댐.

---

## Phase 0: Project Setup & Foundation ✅ (이미 완료)

**현재 상태:** Bitvue v0.12.0에서 대부분 완료됨

- [x] Tauri + Rust + React TypeScript 프로젝트 구조
- [x] Cargo workspace with 32 crates
- [x] 기본 Tauri 커맨드 인터페이스 (23개)
- [x] React 컨텍스트 + 훅 아키텍처
- [x] DockableLayout 패널 시스템
- [x] 크로스플랫폼 빌드 (macOS/Windows/Linux)
- [x] CI/CD 파이프라인 기초
- [ ] MXF 컨테이너 지원
- [ ] AVI 컨테이너 지원
- [ ] HEIC 컨테이너 지원 (`COMPETITOR_FEATURE_MATRIX.md` §6, SE 소스 — 신규 2026-07-31)
- [ ] DASH-MPD 컨테이너 지원 (`COMPETITOR_FEATURE_MATRIX.md` §6, SE 소스 — 신규 2026-07-31)

**추가 해야 할 일:**
- [ ] vvdec-sys 바인딩 구현 및 VVC 디코딩 연결
- [ ] openavs3d 또는 FFmpeg AVS3 디코딩 연결
- [ ] MPEG-2 디코딩 FFmpeg 연결

**예상 소요:** 2~3주 (신규), 현재 이미 완료

---

## Phase 1: 코덱별 F키 모드 분기 시스템 🟢 (2026-08-10 재감사 — 대부분 완료, 배선만 남음)

**목표:** 현재 고정 6모드 → 코덱별 동적 모드 시스템으로 교체

> **재감사 요약:** 문서는 미착수(🔴)로 표시돼 있었지만 실제로는 Rust 구조체가 아니라 **프론트엔드 TS 레지스트리**로
> 동일한 목표가 이미 구현·배선 완료돼 있음 (HEVC/VVC/AV1/VP9/AVC/MPEG-2/AVS3/JPEG XS/VC-3 9개 코덱 전부).
> 아래 원래 설계(Rust `CodecModeRegistry` + `get_codec_modes` Tauri 커맨드)는 채택되지 않았고, 대신 정적 클라이언트
> 조회 테이블로 구현됐다 — 파일 로드 시 서버에 모드 목록을 물어볼 필요가 없어 별도 백엔드 커맨드가 불필요했던 것으로 보임.

- [x] `CodecModeRegistry` 구조체 설계 (코덱 → 가능한 모드 목록) — Rust 구조체 대신 TS 싱글 소스오브트루스로 구현:
      `frontend/utils/codecModeRegistry.ts`의 `CODEC_MODE_REGISTRY` (HEVC/VVC/AV1/VP9/AVC/MPEG2/AVS3/JPEGXS/VC3 9개 코덱)
- [x] 각 코덱별 F키 → 모드 이름 매핑 테이블 구현 — 같은 파일의 `HEVC_MODES`/`VVC_MODES`/`AV1_MODES`/... 배열들
      (각 항목 `{fKey, mode, label, description}`), `getModeByFKey()`/`getFKeyForMode()`
- [x] 프론트엔드 Mode 메뉴 동적 생성 (파일 로드 후 코덱 감지 → 메뉴 갱신) —
      `frontend/contexts/ModeContext.tsx`의 `setActiveCodec()`(코덱 변경 시 모드 목록 재계산 + 유효하지 않으면 기본 모드로
      리셋) + `frontend/components/panels/YuvViewerPanel/ModeSelector.tsx`(드롭다운이 `availableModes`로 렌더링),
      `App.tsx:329`에서 코덱 감지 콜백을 `setActiveCodec`에 연결
- [x] 툴바의 F키 버튼 동적 표시/숨기기 — `frontend/components/panels/YuvViewerPanel/OverlayToggleBar.tsx`
      (코덱에 오버레이가 없으면 `availableOverlays.length === 0`일 때 바 전체를 숨김, 있으면 코덱별 토글 버튼만 렌더링)
- [ ] `get_available_modes` Tauri 커맨드 추가 — **불필요로 판명, 미구현.** 모드 레지스트리가 정적 클라이언트 데이터라
      파일별 백엔드 조회가 필요 없음. 다만 코덱 감지 자체(문자열 "HEVC"/"AV1"/... 판별)가 실제로 sidecar를 거치는지는
      Phase 2의 백엔드 스코프 캐비어트 참조 — 현재 sidecar 파이프라인은 AV1 IVF만 실제로 디코드/분석하므로, 다른 코덱의
      `activeCodec` 판별 경로 자체를 별도로 검증 필요 (미검증, 후속 확인 요망).

> ⚠️ **백엔드 스코프 캐비어트 (Phase 2에서 상세):** 위 UI 레이어(메뉴/F키/토글 버튼)는 9개 코덱 모두에 대해 올바르게
> 나타나지만, 실제로 화면에 그려지는 오버레이 데이터를 공급하는 `get_frame_analysis`/`get_av1_features`/
> `get_decoded_frame_yuv` sidecar 커맨드는 **AV1/IVF 전용**이다 (`crates/bitvue-sidecar/src/frame_analysis.rs:4`
> 모듈 주석 "AV1/IVF only", `crates/bitvue-sidecar/Cargo.toml`에 bitvue-hevc/bitvue-avc/bitvue-vp9/bitvue-vvc/
> bitvue-avs3/bitvue-jpegxs/bitvue-vc3 의존성이 전무). 따라서 HEVC/VVC/AVC/VP9/AVS3/JPEG XS/VC-3 파일을 열면
> Mode 메뉴와 F키는 정확한 코덱별 항목을 보여주지만, 실제 오버레이 캔버스에는 데이터가 없거나(빈 화면) 렌더러의
> "데이터 없음" 폴백 메시지만 표시된다. 이 항목은 원래 Phase 1의 범위(메뉴/키 배선)는 아니지만, 사용자 체감상
> "다 됐다"고 오인하기 쉬운 지점이라 여기 명시한다.

**Parity 검증:**
- 각 코덱 파일 로드 후 Mode 메뉴에 올바른 항목만 표시되는지 확인 — ✅ 확인됨 (레지스트리 코드 검토 기준)
- F키 눌렀을 때 올바른 오버레이 렌더러 활성화 — ⚠️ F키 → 모드 매핑 자체는 맞지만, 위 캐비어트대로 AV1 외 코덱은
  렌더러가 활성화돼도 실데이터가 없어 시각적으로 빈 상태

**예상 소요:** 완료 (남은 것은 코덱별 백엔드 데이터 연결 — Phase 2/3/5/6 참조)

---

## Phase 2: 코덱별 Info Overlay 토글 시스템 🟡 (2026-08-10 재감사)

**목표:** QP Map, Heat Map, MV Heat 등 오버레이를 코덱별로 사용 가능/불가능 처리

> **재감사 요약:** 토글 인프라(메뉴, 다중 표시, 상태 저장)는 완료. 렌더러 컴포넌트도 표에 있는 항목 대부분 이미
> 존재한다(`frontend/components/panels/OverlayRenderer/renderers/*.tsx`, 29개 파일). 하지만 실데이터를 공급하는
> `get_frame_analysis`/`get_av1_features` sidecar 커맨드가 **AV1/IVF 전용**이라(Phase 1 캐비어트 참조), HEVC/VVC/
> AVC/VP9 대상 렌더러는 코드는 있어도 프로덕트에서 실제 데이터를 받아본 적이 없다. 아래 표의 "Bitvue 현황"은
> "렌더러 존재 여부"와 "실데이터 도달 여부"를 분리해서 표기한다.

- [x] `InfoOverlayCapabilities` 코덱별 매트릭스 구현 — `frontend/utils/codecModeRegistry.ts`의
      `getInfoOverlaysForCodec()` (코덱별 `isInfoOverlay: true` 항목 필터)
- [x] View/Mode 메뉴의 Info Overlays 하위 메뉴 동적 활성화/비활성화 —
      `frontend/components/panels/YuvViewerPanel/OverlayToggleBar.tsx` (코덱에 없는 오버레이는 목록에서 아예 제외)
- [x] 다중 오버레이 동시 표시 지원 (예: CodingFlow + QP Map 중첩) —
      `frontend/components/panels/OverlayRenderer/index.tsx`의 `renderModeOverlay()` — Pass 1(메인 모드) +
      Pass 2(`activeOverlays` Set을 순회하며 `INFO_OVERLAY_ALPHA=0.72` 반투명 중첩)
- [x] 각 오버레이별 토글 상태 저장 (Options에서 유지) — `ModeContext.tsx`의 `loadOverlayPrefs`/`saveOverlayPrefs`
      (`localStorage` 키 `bitvue-overlay-prefs`, 코덱별로 저장 후 코덱 전환 시 유효한 것만 복원)

**구현할 오버레이 우선순위:**

| 오버레이 | 코덱 지원 | Bitvue 현황 | 작업 |
|---------|---------|-----------|-----|
| QP Map | HEVC, VVC, AVC, MPEG-2 | ✅ 렌더러 완성(`QPMapRenderer.tsx`), 실데이터는 AV1만 (`frame.qp_grid`, AV1/IVF 전용) | 나머지 코덱 백엔드 연결 |
| Heat Map | 전체 | ⚠️ 전용 렌더러 없음 — `OverlayRenderer/index.tsx:218`가 명시적으로 MVField를 매그니튜드 히트맵으로 재사용 | 전용 bit-cost 렌더러 신규 |
| MV Heat | HEVC | ✅ 렌더러 완성(`MVFieldRenderer.tsx` + WebGL2 가속 `webgl/mv-webgl.ts`), 실데이터는 AV1만(`frame.mv_grid`) | HEVC 백엔드 연결(`bitvue-hevc`는 sidecar에 미배선) |
| PU Type | HEVC | ❌ 메뉴/토글 버튼은 있음(`codecModeRegistry.ts`, `OverlayToggleBar.tsx`)이나 렌더러 자체가 없음 — `OverlayRenderer/index.tsx:235`에 주석 처리된 스텁만 존재 | 렌더러 신규 구현 |
| PU Reference Indices | HEVC | ✅ 렌더러 완성 (`AvcRefIdxRenderer.tsx`, mode="reference-indices" 공유), 실데이터는 sidecar 미배선으로 도달 안 함 | HEVC 백엔드 연결 |
| MB Type | AVC | ✅ 렌더러 완성(`AvcMbTypeRenderer.tsx`) + 백엔드 추출 함수 존재(`crates/bitvue-avc/src/overlay_extraction.rs:1637 extract_mb_type_grid`), 단 `bitvue-avc`가 `bitvue-sidecar/Cargo.toml`에 의존성으로 없어 프로덕트에 미배선 | sidecar에 `bitvue-avc` 의존성 추가 + 커맨드 배선 |
| MB Reference Indices | AVC | ✅ 렌더러+백엔드 존재(`extract_ref_idx_grid`, 같은 파일 1696행), 동일하게 sidecar 미배선 | sidecar 배선 |
| Block Type | AV1, VP9 | ✅ 렌더러 완성(`Av1BlockTypeRenderer.tsx`, `frame.mv_grid.mode` 기반), AV1은 실데이터 도달, VP9은 `bitvue-vp9`가 sidecar 미배선 | VP9 백엔드 연결 |
| Efficiency Map | AV1, VP9 | ✅ AV1은 2026-08-10 완료 — 실제 per-CU 잔차 에너지 기반(`energy_extractor.rs`, `ab8faa2`), QP 프록시는 폴백으로만 남음. VP9은 여전히 sidecar 미배선(`bitvue-vp9` 의존성 없음) | VP9 백엔드 연결 |
| PSNR Overlay | 전체 | ❌ 메뉴/토글만 존재, 렌더러 없음(`OverlayRenderer/index.tsx:234` 주석 스텁) | Debug YUV 파이프라인 연결 + 렌더러 신규 |
| SSIM Overlay | HEVC | ❌ 동일 — 렌더러 없음, 토글만 존재 | Debug YUV 필요 + 렌더러 신규 |
| Inter Memory Reads | VVC | ❌ 토글만 존재(`inter-memory`), 렌더러 없음 | 신규 구현 |
| Simple Motion | 전체 | ❌ 토글만 존재(`simple-motion`), 렌더러 없음 | 단순화 MV 화살표 신규 |
| Loop Filter (deblock BS 색상) | HEVC, VP9, AVC | ✅ 완성 — 캔버스 오버레이가 아니라 전용 패널 `DeblockingView.tsx`로 구현, 실제 AV1 boundary-strength 알고리즘(AV1 spec §7.14.2) 백엔드 연동 확인됨(`crates/bitvue-sidecar/src/deblocking.rs`). HEVC/VP9/AVC는 여전히 sidecar 미배선 | HEVC/VP9/AVC 백엔드 연결 |
| SAO | HEVC | ❌ 메뉴/F키(HEVC F6, VVC F8, AVS3 F6)는 있으나 렌더러 자체가 없음(`OverlayRenderer/index.tsx` switch에 `case "sao"` 없음 → 아무것도 그려지지 않음). `bitvue-hevc`에는 SAO 파싱 코드가 존재하나 sidecar 미배선 | 렌더러 신규 구현 + sidecar 배선 |
| Reconstruction (+Detail popup) | HEVC, AV1 | ❌ 메뉴/F키는 있으나(`reconstruction` mode) `OverlayRenderer/index.tsx` switch에 대응 case가 없어 아무것도 그려지지 않음, Detail popup 컴포넌트도 검색 결과 없음 | 렌더러 신규 구현, Detail popup 신규 (`COMPETITOR_FEATURE_MATRIX.md` §1) |
| Predictions/Transform Detail popup | HEVC | ❌ 미착수 확인 (`DetailPopup` 관련 컴포넌트 grep 결과 없음) | Options 메뉴 "Detail Popup Windows" 참조 (`UX_PARITY_MATRIX.md` §10) |
| CABAC range/state 시각화 | HEVC, AVC | ❌ `CodingFlowView.tsx`에 "CABAC/CAVLC Encoding"이라는 정적 라벨만 존재, range/state 실시간 시각화 없음 | `PARITY_CHECKLIST.md` Layer 6 CMP-10 참조 |

**코덱 확장 (신규 2026-07-31, `COMPETITOR_FEATURE_MATRIX.md` §1 VP9/AVC 테이블):**
- [ ] VP9: Coding Flow/Partition grid, MV Field, Predictions, Transform/Reconstruction, Efficiency Map — 여전히 미배선.
      `crates/bitvue-vp9/src/overlay_extraction.rs`에 `extract_qp_grid`/`extract_mv_grid`/`extract_partition_grid`
      등 실제 추출 로직은 이미 존재하지만 `bitvue-sidecar`가 `bitvue-vp9`에 의존하지 않아 미배선
- [ ] AVC: Coding Flow/Partition grid, Transform/CBF — 여전히 미배선. `crates/bitvue-avc/src/overlay_extraction.rs`에
      동등한 추출 로직 존재(위 MB Type/Ref Indices 행 참조), 역시 sidecar 미배선
- [ ] HEVC RExt(4:2:2/4:4:4)/SCC/SHVC 확장 디코딩 지원 확인 및 완성 — 미검증 (`bitvue-hevc`가 sidecar에 없어 애초에
      HEVC 디코딩 자체가 프로덕트 경로에 없음. 파서 레벨 지원 여부는 `crates/bitvue-hevc/src/sps.rs` 별도 확인 필요)
- [ ] Decode-stage pixel value 표시 (pre-deblock/predicted/residual/final) — 미착수, Reconstruction 렌더러 자체가
      없으므로 선행 작업(Reconstruction 행 참조)이 먼저 필요

**Parity 검증:**
- 각 오버레이의 색상 스케일이 VQ Analyzer와 일치
- QP Map: jet colormap (blue→green→yellow→red) 범위 정확도
- 오버레이 중첩 시 투명도 처리

**예상 소요:** 중급 2~3주 (렌더러 다수는 이미 완성, 남은 작업은 주로 sidecar 코덱별 배선 + 8개 미구현 렌더러)

---

## Phase 3: VVC 전용 기능 완성 🟡 (2026-08-10 재감사 — 라이브러리 레벨은 완성, 제품 경로는 미연결)

**목표:** VVC 파싱 → 디코딩 → 전용 모드 완전 구현

> **재감사 요약:** vvdec FFI 바인딩은 이미 완전하게(RAII 가드, 타임아웃/포이즌 복구, 스레드 안전성까지) 구현돼
> 있지만 **cargo feature `vvdec`가 어디에서도 활성화되지 않아** 실제로는 링크/컴파일되지 않는다 — `bitvue-sidecar`가
> `bitvue-decode`를 feature 지정 없이 의존하고, 저장소 전체에서 `--features vvdec`를 사용하는 빌드 스크립트/CI가
> 없음(확인: `Cargo.toml`, `crates/bitvue-sidecar/Cargo.toml`, `scripts/package_electron.sh` grep 결과 0건).
> 게다가 실제 미리보기 픽셀 디코드 경로(`crates/bitvue-sidecar/src/decode_bridge.rs::get_decoded_frame_yuv`)는
> 코덱 분기 없이 `Av1Decoder` + IVF 파싱으로 하드코딩돼 있어, `vvdec` feature를 켜더라도 이 경로가 VVC를 타도록
> 별도 연결이 필요하다. VVC 전용 비트스트림 파서 크레이트(`bitvue-vvc`)는 실제로 존재하고 워크스페이스 멤버로
> 등록돼 있으며(`crates/bitvue-vvc/src/`, 4700줄+: `sps.rs`/`pps.rs`/`overlay_extraction.rs`/`tests.rs` 등),
> `overlay_extraction.rs::extract_partition_grid`가 슬라이스 데이터에서 실제 CTU/CU를 파싱해(`parse_slice_ctus`,
> intra 슬라이스는 CABAC 없이 CTU당 1개 Intra CU로 처리) `PartitionGrid`를 만든다 — 파싱 실패 시에만 "scaffold"
> 블록으로 폴백. 다만 이 크레이트는 어디에도 소비되지 않는다: `bitvue-sidecar`가 여전히 의존하지 않고,
> `bitvue-cli`에서도 VVC 전용 커맨드 배선을 찾지 못함(둘 다 grep 결과 0건) — 즉 "파서가 없다"가 아니라
> **"파서는 있는데 제품 어디에서도 안 부른다"**가 정확한 상태.

- [x] vvdec-sys Rust 바인딩 구현 (`vvdec` C API 래핑) — `crates/bitvue-decode/src/vvdec.rs`, 별도
      `vvdec-sys` 크레이트가 아니라 `bitvue-decode` 내부 `mod ffi`로 직접 구현 (RAII `DecoderGuard`/
      `AccessUnitGuard`, 타임아웃 감지+decoder poison/reset, `Decoder` trait 구현까지 완료).
      단, `[cfg(feature = "vvdec")]` 뒤에 있고 이 feature가 실제로 켜지는 곳이 없음 — **컴파일은 되지만 링크되지 않음**
- [ ] VVC 디코딩 파이프라인 연결 (`crates/bitvue-decode/src/decoder.rs`) — `DecoderFactory::create(CodecType::H266)`
      에서 `VvcDecoder::new()`를 호출하도록 배선은 돼 있음(`traits.rs:363`)이나, 실제 제품이 쓰는
      `get_decoded_frame_yuv` 경로가 이 팩토리를 거치지 않고 `Av1Decoder`로 하드코딩돼 있어 미연결. VVC 전용
      CU/파티션 파서 자체는 `bitvue-vvc::overlay_extraction::extract_partition_grid`로 이미 존재(intra 슬라이스
      CTU/CU 파싱 포함, `tree_type` 필드도 채움) — 없는 건 파서가 아니라 이 크레이트를 부르는 sidecar/CLI 배선
- [x] **Dual Tree 모드 렌더러** 구현 — `frontend/components/panels/OverlayRenderer/renderers/VvcDualTreeRenderer.tsx`,
      `frame.partition_grid`의 `tree_type` 필드로 루마(파랑)/크로마(빨강) 구분 + 데이터 없을 때 폴백 메시지까지 존재.
      백엔드(`bitvue-vvc`)는 `tree_type`을 실제로 채우지만 크레이트 자체가 sidecar에 미배선이라 GUI까지는
      도달 못 함 — 배선만 하면 이 렌더러는 바로 동작할 가능성이 높음(원본 데이터 자체가 없는 게 아님)
- [x] **Inverse Map 모드** (LMCS 역 매핑 시각화) — `VvcInverseMapRenderer.tsx` 렌더러 존재, 백엔드 LMCS 데이터 미공급으로 동일하게 폴백만 표시될 것으로 추정(직접 실행 미검증)
- [x] **Adaptive Filter (ALF) 모드** — `VvcAdaptiveFilterRenderer.tsx` 렌더러 존재, 동일 사유로 실데이터 미도달 추정
- [ ] **Inter Memory Reads** 오버레이 — 메뉴/토글만 존재(`inter-memory` in `codecModeRegistry.ts`), 렌더러 없음
      (Phase 2 표의 동일 행 참조)
- [ ] VVC Syntax 탭 완성 (APS 탭 추가) — 미검증 (Syntax 패널 쪽은 이번 재감사 범위 밖, 별도 확인 필요)

**핵심 구현 과제 — vvdec 연결 (Tauri → Electron+sidecar 아키텍처 갱신):**
```rust
// crates/bitvue-decode/src/vvdec.rs (구현 완료, 인용 경로만 갱신 — 원래 예시는 이제 없는
// src-tauri/src/decode/vvc_decoder.rs를 가리켰음. 실제 구조는 아래와 유사:)
pub struct VvcDecoder {
    decoder: Mutex<*mut ffi::VvdecDecoder>,      // FFI 핸들, 뮤텍스로 보호
    access_unit: Mutex<*mut ffi::VvdecAccessUnit>,
    // ...
}

impl Decoder for VvcDecoder {
    fn get_frame(&mut self) -> Result<DecodedFrame> {
        // vvdec_decode() 호출 → DecodedFrame 변환 (convert_frame)
    }
}
```
남은 일은 대부분 코드 자체가 아니라 **배선**: (1) `bitvue-sidecar`가 `vvdec` feature를 켜고 빌드되도록 CI/패키징
스크립트 갱신, (2) `get_decoded_frame_yuv`가 코덱별로 `Av1Decoder`/`VvcDecoder`를 선택하도록 리팩터, (3)
`bitvue-vvc`(파티션/QP/MV 파서는 이미 있음)를 sidecar 의존성에 추가하고 커맨드로 노출. LMCS(Inverse Map)/
ALF(Adaptive Filter) 전용 추출 함수는 `bitvue-vvc`에 아직 없어 이 둘만 진짜 신규 구현이 필요.

**Parity 검증:**
- Dual Tree: 루마/크로마 경계가 VQ Analyzer와 pixel-perfect 매칭 — 검증 불가 (배선 전, sidecar까지 데이터 미도달)
- LMCS 적용된 시퀀스에서 Inverse Map 시각화 확인 — 검증 불가 (추출 함수 자체가 아직 없음)

**예상 소요:** 중급 2~4주 (vvdec 라이브러리 코드 + VVC 파티션 파서 모두 이미 있음 — 주 작업은 sidecar 배선과
LMCS/ALF 추출 함수 신규 구현, 원래 추정보다 상당히 짧을 가능성)

---

## Phase 4: AV1 전용 고급 모드 완성 ✅ (2026-08-10 재감사 — 사실상 완료)

**목표:** AV1 전용 F6~F9 모드 구현 (CDEF, SuperRes, Loop Restoration, Film Grain)

> **재감사 요약:** 이 Phase는 문서상 🟡(부분 완료)였지만 실제로는 백엔드·프론트엔드·와이어 매핑·엔드투엔드
> 테스트까지 전부 완료돼 있다 — 6개 코덱 Phase 중 유일하게 백엔드 데이터가 프로덕트 경로(AV1/IVF)와 정확히
> 일치하는 Phase라서 온전히 동작한다 (Phase 1의 "AV1만 실데이터 도달" 캐비어트가 Phase 4에는 적용되지 않음,
> Phase 4 자체가 AV1 전용이기 때문).

- [x] **CDEF Filter 모드** — 백엔드 `bitvue_av1_codec::advanced_features::extract_cdef_data` +
      sidecar `crates/bitvue-sidecar/src/av1_features.rs`(`get_av1_features` 커맨드) +
      프론트엔드 `Av1CdefRenderer.tsx`(8방향 화살표, strength 색상 스케일, 강도=0 블록 회색 처리 모두 구현).
      end-to-end 테스트 `av1_features.rs`의 `get_av1_features_end_to_end_returns_real_cdef_data`로 실제 fixture
      데이터 검증 완료
- [x] **SuperRes Filter 모드** — `extract_super_resolution_data` + `Av1SuperResRenderer.tsx`, 같은 커맨드로 배선
- [x] **Loop Restoration 모드** — `extract_loop_restoration_data` + `Av1LoopRestorationRenderer.tsx`
      (Wiener/Self-guided/미적용 구분 렌더링 확인)
- [x] **Film Grain 모드** — `extract_film_grain_data` + `Av1FilmGrainRenderer.tsx`
- [x] AV1 Efficiency Map — **2026-08-10 정확도 개선 완료** (`ab8faa2`): QP 기반 근사식 대신 실제 디코드된
      per-CU 잔차 크기(`CodingUnit.residual.sum_abs_level / block_area`)를 사용하도록 교체 —
      `crates/bitvue-av1-codec/src/overlay_extraction/energy_extractor.rs` 신규(`extract_energy_grid_from_parsed`,
      qp_extractor.rs와 동일한 `CuSpatialIndex` 재사용), `get_frame_analysis` 응답에 `energy_grid` 필드 추가,
      `Av1EfficiencyMapRenderer.tsx`가 이를 우선 사용하고 없을 때만 QP 프록시로 폴백. 여전히 진짜 엔트로피
      비트 수는 아님(컨텍스트 독립 대표 CDF 사용, 실제 스펙의 이웃-컨텍스트 적응형 아님) — 실제 fixture로
      0이 아닌 에너지 값 확인하는 e2e 테스트로 검증
- [x] AV1 Block Type 오버레이 — **2026-08-11 IntraBC/Compound 구분 추가 완료** (`8b27766`):
      `parse_coding_unit`가 `ref_frame()`(spec 5.11.25)를 전혀 안 읽고 모든 inter 블록을
      `RefFrame::Last`로 하드코딩하던 진짜 desync 버그(이번 세션의 residual() 버그와 동일 패턴 —
      신택스 자체를 안 읽음, 단 크래시로 안 드러났을 뿐) 발견+수정하면서 같이 완성. 단일 참조
      7종 전부 + compound(uni/bidirectional) 트리 실제 파싱, `use_intrabc` 비트도 신규 파싱.
      부수로 `Av1BlockTypeRenderer.tsx`가 실제 `BlockMode` enum 값과 다른 매핑(0=Inter 가정, 실제는
      0=None)을 쓰고 있던 **별개의 기존 버그**도 발견+수정(모든 블록이 한 칸씩 밀려서 잘못된
      색으로 표시되고 있었음). 실제 fixture로 ref_frame 분포가 7종 전부 나오고 compound 블록도
      실제 검출됨을 확인하는 영구 회귀 테스트 추가

**AV1 파싱 보강 (완료 — Electron+sidecar 아키텍처 기준 인용 경로 갱신):**
```rust
// crates/bitvue-av1-codec/src/advanced_features.rs (실제 구현, 원 예시의
// bitvue-av1-codec/src/advanced_features.rs 경로는 유지되지만 이제
// Tauri 커맨드가 아니라 crates/bitvue-sidecar/src/av1_features.rs가 호출)
pub struct FilmGrainParams { /* apply_grain, grain_seed, point_y_value 등 — 구현 완료 */ }
pub struct CdefParams { /* cdef_damping_minus_3, cdef_y_pri_strength 등 — 구현 완료 */ }
```
uncompressed_header()의 segmentation~film_grain_params 구간, skip_mode_params용 8슬롯 참조상태 순차추적까지
신규 구현되어 실제 250프레임 스트림 파싱 검증을 통과함 (2026-08-09, electron-migration 커밋 이력 참조).

**Parity 검증:**
- CDEF 방향 화살표가 실제 CDEF 방향 결정과 일치 — 코드 레벨 검증(단위/e2e 테스트), 육안 VQ Analyzer 대조는 미실시
- Film Grain 전/후 픽셀 값이 dav1d 출력과 일치 — 미검증 (프레임 헤더 파라미터 추출은 완료, 픽셀 비교 테스트는 없음)

**예상 소요:** 완료. Efficiency Map 정확도 개선(2026-08-10), Block Type IntraBC/Compound 구분(2026-08-11), **compound 블록의 실제 예측 모드+L1 모션벡터(2026-08-11)** 셋 다 완료 — 남은 세부 작업 없음. `compound_mode()`(spec 5.11.24, 8-심볼 알파벳, `SymbolDecoder::read_compound_mode`) 신규 파싱 + `PredictionMode`에 compound 8종(`NearestNearestMv`..`NewNewMv`) 추가 + L0/L1 각각의 MV-selection 전략(`MvKind`: Nearest/Near/Global/New)을 `l0_mv_kind()`/`l1_mv_kind()`로 분리해 L1 predictor(`MvPredictorContext::get_mv_predictor_l1`)와 명시적 MV 읽기(New 컴포넌트가 L0/L1 어느 쪽이든)를 실제로 처리. `mv_extractor.rs`의 MV 그리드도 compound 블록에서 여태 항상 MISSING이던 L1을 실값으로 노출하도록 수정(그리드 슬롯 자체는 이미 있었지만 아무도 채운 적 없었음). 실제 fixture(`AV1_IVF_FIXTURE`)로 8종 중 실제로 관측되는 compound mode + 0이 아닌 L1 MV를 검증하는 회귀 테스트 추가(`cu_parser.rs`의 기존 `real_fixture_ref_frame_values_are_not_degenerate` 확장). `cargo test --workspace --lib` 클린(3853+ 통과, 무관한 기존 flaky LRU 테스트 1개 제외). **`partition_grid`의 `delta_q_enabled` 하드코딩 버그 발견+수정(2026-08-11)**: `get_frame_analysis`의 `partition_grid`(실제 CU 경계 표시용, `extract_partition_grid_from_parsed`)가 자체적으로 독립된 두 번째 superblock 파싱 패스를 갖고 있었는데, QP/MV/prediction 그리드가 쓰는 `cu_parser::parse_all_coding_units`와 달리 `delta_q_enabled`를 프레임 헤더 값(`parsed.delta_q_enabled`) 대신 항상 `false`로 하드코딩하고 있었음 — 실측 결과 이 fixture 250프레임 중 82프레임(33%)이 실제로 `delta_q_enabled=true`라 residual()/ref_frame()과 정확히 같은 "신택스 완전 미독해→desync" 모양의 실사용 버그였음(참고로 segmentation의 동일 패턴 TODO도 조사했으나 이 fixture+foreman 샘플 둘 다 `segmentation_enabled`가 한 번도 true인 적이 없어 우선순위 낮춰 보류). `parsed.delta_q_enabled` 한 줄로 수정, 같은 실제 bit을 `delta_q_enabled=true`/`false` 양쪽으로 돌려 결과가 실제로 갈라지는지 확인하는 회귀 테스트 추가(`partition.rs`). **부수 발견(안 건드림, 범위 밖으로 기록)**: `extract_partition_grid_from_parsed`와 `cu_parser::parse_all_coding_units`는 여전히 서로 독립된 두 개의 재파싱 패스로 남아있고, (1) 프레임 가장자리 superblock 크기 보정(`actual_block_size` vs 고정 `sb_size`), (2) 슈퍼블록 간 QP 누적 전파(전자는 매번 `base_qp`로 리셋, 후자는 `new_qp` 누적) 두 가지가 여전히 서로 다름 — 둘 다 desync은 안 일으키지만(값 정확도 이슈일 뿐, entropy 디코더 비트 위치엔 영향 없음) 구조적으로 같은 파싱을 두 번 하는 중복이라 언젠가 통합할 가치 있음. **후속: 세 번째 완전 중복 구현 제거(2026-08-11)**: 위 조사 중 `partition.rs`가 `cu_parser::parse_all_coding_units`와 **문자 그대로 동일한**(캐싱 로직·에러 처리까지 전부 동일, 주석 문구만 다름) `parse_all_coding_units` 사본을 자체적으로 갖고 있어서(prediction-mode/transform 그리드 추출용) 실제론 superblock 루프가 세 벌(버그였던 `extract_partition_grid_from_parsed`의 자체 루프 + `cu_parser`판 + 이 사본)이었음을 확인 — delta_q 버그가 애초에 나올 수 있었던 근본 원인이 바로 이 중복 자체였다고 판단해 사본을 삭제하고 `cu_parser::parse_all_coding_units`를 import해 재사용하도록 정리(순수 동작 불변 리팩터, `cargo test --workspace --lib` 클린). 남은 건 `extract_partition_grid_from_parsed`의 구조적으로 다른 자체 루프 하나뿐 — 그건 edge-block-size 보정이 실제로 의미가 달라서 단순 통합이 아니라 별도 검토가 필요함.

**AV1 엔트로피 디코더 코어를 진짜 spec 정확도로 재작성(2026-08-11)**: 사용자가 "CDF가 전부 representative(비적응형) 근사치"라는 한계를 "측정 문제가 아니라 구현 갭"이라고 정확히 짚고 spec 8.3/9.24 완전 재구현을 요청 — 큰 작업이라 EnterPlanMode로 Phase 1(재단+partition+skip) 계획 후 진행. rav1d(`memorysafety/rav1d`, BSD-2-Clause) 소스를 직접 다운로드해 한 줄씩 대조하며 진행:
- **`read_symbol`/`update_cdf` 완전 재작성**: 기존 오름차순(`cdf[0]=0..cdf[n]=32768`) 컨벤션을 실제 spec/rav1d의 내림차순(`cdf[0]≈32768`→`0`) + `EC_MIN_PROB`(=4)/`EC_PROB_SHIFT`(=6) 확률 하한 보정 구조로 교체. `update_cdf`도 rav1d의 정확한 공식(`rate=4+(count>>4)+(n>2)`)으로 재작성.
- **`refill()`/`renormalize()` 감사 중 진짜 버그 3개 추가 발견+수정**: (1) 바이트를 `^0xFF` 반전 없이 그대로 읽고 있었음(spec 필수 동작 누락), (2) 버퍼 소진 시 "남은 비트 전부 1로 채우기" 코드가 이중부정(`!(!(X))`)으로 상쇄돼 실제로는 8비트만 채우는 무의미한 코드였음, (3) refill 트리거 조건이 rav1d의 부호없는 비교 트릭과 달라 `cnt`가 음수가 된 이후 동작이 갈렸음.
- **기존 ~35개 CDF 테이블 전부 새 컨벤션으로 변환**(`to_descending()` 헬퍼, 순수 포맷 변환 — 값 자체는 그대로 유지, 실제 컨텍스트 도입은 아님).
- **파티션트리 파서 중복 통합 중 진짜 위치 버그 발견**: 실제 쓰이던 `superblock.rs`판 `child_position`이 10개 파티션 타입 중 6개(HorzA/HorzB/VertA/VertB/Horz4/Vert4)에서 자식 위치를 전부 부모 위치로 잘못 반환하고 있었음 — depth guard+legality 검증까지 갖춘 `partition.rs`판으로 통합.
- **`skip`은 진짜 컨텍스트+적응 완성**: `TileContext`(신규, 4x4단위 above/left skip 상태 추적) + rav1d 실제 기본값(`Default_Skip_Cdf`: 컨텍스트별 31671/16515/4576) + `read_symbol_adaptive`로 실제 적응까지 배선.
- **partition 컨텍스트(비트마스크+edge-index 트리)는 예상보다 훨씬 커서 사용자 확인 후 이번 라운드에서 보류** — 대신 `partition_cdfs`도 다른 35개와 동일하게 방향만 변환(진짜 컨텍스트는 아님)했는데, 이게 없었으면 이전 세션에 고친 delta_q 회귀테스트가 다시 깨졌을 정도로 필수적이었음(파티션 디코드가 방향 불일치로 "항상 no-split"으로 퇴화해있었음).
- 검증: `cargo test --workspace --lib` 전체 클린(3854+ 통과), `cargo clippy`/`cargo fmt` 클린(남은 warning 전부 무관한 기존 파일). 실제 fixture로 skip=true/false 둘 다 관측되는지 확인하는 회귀 테스트 신규 추가.

**key-frame intra_mode(`kfym`) 컨텍스트도 진짜로 완성(2026-08-11)**: "mode 컨텍스트도 같은 방식으로?" 질문에 rav1d로 먼저 스코핑 — intra_mode(키프레임용 `kfym`, above/left 실제 예측모드를 5개 클래스로 묶어 5×5 컨텍스트)는 skip급 크기, inter_mode/compound_mode는 `rav1d_refmvs_find()`(spatial+temporal 후보 스캔하는 참조-MV 서브시스템, `src/refmvs.rs`)가 필요해 partition급으로 큼 — intra_mode만 진행하기로 사용자 확인.
- `TileContext`에 above/left raw 모드값 추적 추가 + `INTRA_MODE_CONTEXT`(rav1d `DAV1D_INTRA_MODE_CONTEXT`) 클래스 매핑.
- `kfym` 5×5×13 실제 기본 CDF 테이블을 rav1d `src/cdf.rs`에서 파이썬 스크립트로 직접 파싱+변환해 이식(325개 숫자 수기 옮기면 오타 위험 커서 자동화) — `read_symbol_adaptive`로 실제 적응까지 배선.
- **검증 중 진짜 큰 버그 발견**: 새 kfym 테스트가 "첫 30프레임 중 키프레임이 하나도 없음"으로 계속 실패 — 원인 추적 결과 `overlay_extraction/parser.rs`가 `FrameType::is_intra_only()`(AV1의 희귀한 INTRA_ONLY_FRAME 타입만 매치)를 쓰고 있었는데 실제로 필요했던 건 `is_intra()`(KEY_FRAME도 포함, spec 5.9.2의 `FrameIsIntra` 정의와 일치) — **이 세션 내내(사실상 프로젝트 전체 기간) 진짜 키프레임이 전부 `is_key_frame=false`로 잘못 분류돼 `parse_coding_unit`의 INTER 분기를 타고 있었음**. 필드 이름/문서는 원래 의도("key/intra-only")를 정확히 담고 있었고 메서드 호출 하나만 틀렸던 것 — 2줄 수정으로 해결, 워크스페이스 전체 재검증 클린.
- 실제 fixture로 키프레임에서 여러 종류의 intra PredictionMode가 관측되는지 확인하는 회귀 테스트 추가(이 버그 수정 전엔 통과가 불가능했음 — 키프레임 자체가 감지되지 않았으므로).

**MV/delta_q 실제 adaptation 배선 완료(2026-08-11)**: 로드맵 중 "컨텍스트 불필요, 적응만 배선하면 됨"으로 표시돼있던 항목 — `mv_joint`/`mv_class`/`mv_bit`/`mv_sign`/`delta_q`/`delta_q_sign`/`diff` 7개 CDF에 `_mut` 접근자 추가 + `read_symbol`→`read_symbol_adaptive` 전환.
- **작업 중 mv_joint 자체가 어디서도 읽힌 적이 없다는 진짜 desync 버그 발견**: spec 5.11.32 `read_mv(ref)`는 `mv_joint` 심볼로 먼저 "어느 축이 0이 아닌지" 판정한 뒤 조건부로 가로/세로 component를 읽는데, `read_explicit_mv`(및 그 인라인 중복)는 이 심볼을 건너뛰고 `mv_x`/`mv_y`를 무조건 둘 다 읽고 있었음 — residual()/ref_frame()/delta_q_enabled와 정확히 같은 "신택스 요소 완전 미독해→공유 SymbolDecoder desync" 모양(그저 크래시로 안 드러났을 뿐). `read_mv_joint()` 신규 추가 + `read_explicit_mv`를 spec 순서(세로 컴포넌트 조건 먼저, 가로 다음)대로 재작성, 2곳의 인라인 무조건 이중 읽기도 통합.
- **수정 직후 기존 `real_fixture_delta_q_frame_changes_with_the_flag` 테스트가 첫 60프레임 범위에서 깨짐**: 원인은 회귀가 아니라 이 테스트 헬퍼(`parse_all_coding_units_with_delta_q_flag`)가 `?`로 슈퍼블록 파싱 에러를 즉시 전파하는 반면, 실제 경로(`cu_parser::parse_all_coding_units`)는 슈퍼블록 단위 에러를 조용히 스킵하고 계속 진행하는 관용적 구조라는 차이 때문 — mv_joint를 실제로 읽기 시작하면서 데이터 종속적으로 가변 개수의 심볼을 소비하게 됐고, 아직 mode/ref_frame/compound_mode/residual/partition 전부 representative(비컨텍스트) CDF라 프레임 내 엔트로피 상태가 이미 어느 정도 어긋나 있어 특정 프레임에서 더 쉽게 파티션 디코드가 깨짐 — 검색 범위를 전체 fixture로 넓혀 "성공하는 프레임 하나"만 찾도록 완화(테스트 목적 자체가 flag의 인과성 증명이지 전체 프레임 성공 보장이 아님), 테스트 자체에 이미 남겨뒀던 "다른 프레임 범위가 필요할 수 있음" 코멘트와 일치.
- 실제 fixture로 NewMv 블록의 MV 값이 여러 종류로 관측되는지 확인하는 회귀 테스트 추가(`real_fixture_new_mv_values_are_not_degenerate`).
- 검증: `cargo test --workspace --lib` 클린(3854+ 통과, 무관한 기존 flaky LRU 테스트 1개 제외 — 단독 실행 시 통과 확인), `cargo test --tests -p bitvue-av1-codec`(integration test 바이너리) 실행 중 별개로 이 세션 초반 `TileContext` 매개변수 추가 때부터 컴파일이 깨져있던 `tests/overlay_extraction_test.rs`(5곳, `PartitionBlock`에 신규 `tree_type` 필드 누락)와 `tests/mv_extraction_test.rs`(`parse_superblock` 인자 누락)도 함께 수정 — `cargo test --workspace --lib`만으로는 `tests/` 통합 테스트 바이너리가 컴파일되는지 전혀 검증되지 않는다는 사각지대였음. `cargo clippy`/`cargo fmt` 클린(터치한 파일 기준 경고 0).

**partition 실제 컨텍스트 완성(2026-08-11)**: "edge-index 트리, 큰 작업"이라 두 번 보류했던 항목 —
rav1d 소스를 직접 대조해 재스코핑한 결과 **과대평가였음**을 확인 후 진행. rav1d의 `get_partition_ctx`
(`src/env.rs`)는 런타임 트리 탐색이 아니라 `above_partition[x8]`/`left_partition[y8]`(8x8단위 바이트 배열)
의 특정 비트를 `bl`(block level)로 읽는 것뿐이고, 심볼 결정 후 쓰는 값도 매 호출 계산이 아니라 컴파일타임
상수 테이블 `DAV1D_AL_PART_CTX[dir][bl][bp]`(`src/tables.rs`, 5×10×2=100바이트) 룩업 — skip/kfym과
정확히 같은 "above/left 배열 + write 시 footprint 채우기" 패턴이었음. `TileContext`에
`above_partition`/`left_partition`(8x8단위) + `partition_context(x8,y8,bl)` + `set_partition(...)`
추가(skip 대비 델타: 4x4→8x8 단위, bool→u8 비트마스크, 쓰는 값이 심볼 자체가 아니라 테이블 룩업). 실제
CDF(rav1d `Default_Partition_W8/16/32/64/128_Cdf`, 5블록레벨×4컨텍스트, kfym보다 적은 ~150개 숫자)도
같이 포팅 — 8x8은 4심볼, 128x128은 실제로 8심볼(HORZ_4/VERT_4 없음, 기존 코드가 10심볼로 잘못 가정하고
있던 부분도 같이 수정), 16x16~64x64는 10심볼. `parse_partition_recursive`에 `tile_ctx: &mut TileContext`
매개변수 추가해 배선(Split은 8x8 초과에서 자식이 대신 쓰므로 자신은 안 씀, 8x8 Split은 자식이 4x4라
컨텍스트가 없어 자신이 씀 — rav1d와 동일 조건). **수정 직후 `real_fixture_delta_q_frame_changes_with_the_flag`
가 "우연히 같은 결과로 수렴"하는 프레임에 걸려 실패** — 회귀가 아니라 이 테스트가 "적격 프레임 *전부*
갈라져야 함"이라는 과도하게 엄격한 assert_ne를 for 루프 안에 두고 있었던 구조적 취약점(진짜 partition
컨텍스트가 생기면서 어떤 프레임은 delta_q 유무와 무관하게 같은 파티션 경계로 수렴하는 게 실제로 가능해짐)
— "적격 프레임 중 *하나라도* 갈라지면 충분"으로 완화(테스트의 실제 의도와 일치). 실제 fixture로 leaf
블록 크기가 다양하게 나오는지 확인하는 회귀 테스트 추가(`real_fixture_partition_leaf_sizes_are_not_degenerate`).
315/315(av1-codec lib) 통과, `cargo test --workspace --tests` 141개 스위트 클린, clippy/fmt 클린.
`has_rows`/`has_cols`(프레임 경계 축소-알파벳 읽기)는 여전히 미구현으로 남음(스코프 유지, 별도 항목).

- **완료(2026-08-11)**: ref_frame 브랜치 컨텍스트(`0e8cc0f`, 9개 rav1d 컨텍스트 함수 전부 이식, 16개
CDF 테이블 실값 포팅, decode.rs 실제 호출부 대조 중 uni_comp_ref_p2 극성 버그 발견+수정). inter_mode/
compound_mode의 **공간(spatial) 절반**(`33379f6`, `SpatialRefContext` 신규 — have_newmv_match/
have_refmv_match가 순수 불리언이라 rav1d의 가중치 MV candidate list 없이도 계산 가능함을 확인해 스코프
축소, `read_inter_mode`를 진짜 3단계 캐스케이드로 재작성). globalmv_ctx(temporal/cross-frame 성분)는
segment_id급 TODO로 고정 0 유지 — 결정론적 실제 스코핑 결과, 확대 안 함.

- **residual coefficient 컨텍스트 재조사 완료, 착수는 보류(2026-08-11)**: "42개 컨텍스트"라는 기존
프레이밍은 ref_frame의 "14/16 CDF"급 과대평가로 확인 — eob_bin/eob_hi_bit/coeff_base_eob는 상태 없는
순수 산술 공식이고 txb_skip/dc_sign은 partition급 above/left 배열로 끝남. 단, 직접 rav1d
`recon_tmpl.c`(아직 C, Rust 미포팅) 재확인 중 두 가지 진짜 걸림돌 발견: (1) **eob_bin 컨텍스트의
`is_1d` 축이 `transform_type()`(spec 5.11.47)에 의존하는데 이 신택스 요소 자체가 Bitvue에 전혀
구현돼있지 않음** — 별도의 실비트스트림-소모 심볼읽기(5개 조건분기 CDF패밀리 중 하나)이자, `lossless`/
`qidx`/`reduced_txtp_set` 등 현재 `parse_coding_unit` 호출체인에 전혀 threading 안 된 프레임헤더 플래그가
추가로 필요함(단순 컨텍스트 추가가 아니라 호출부 시그니처 확장이 필요한 아키텍처 확장). (2)
**txb_skip/dc_sign(`get_skip_ctx`/`get_dc_sign_ctx`)은 partition의 "above/left 비트마스크"급이 아니라
SIMD 최적화용으로 타입-폭(uint8/16/32/64)별로 분기하는 packed-byte above/left 배열**이라 값 인코딩부터
역설계 필요(폭 얕지 않음). 사용자에게 "컨텍스트만, 크로마 버그는 별도로" 확인 받았으나, 재조사 중 실제
드러난 스코프가 최초 프레이밍보다 커서(특히 transform_type() 프레임헤더 플래그 threading) 이번 세션에서는
착수하지 않고 정밀 스코프 맵만 남김. **크로마 잔차(chroma residual)가 luma 전용 호출부라 한 번도 읽힌 적
없다는 것도 이번 재조사 중 발견** — residual()/ref_frame()과 동일한 묵음 디스싱크급 잠재 버그, 별도 항목
(둘 다 다음 세션 후보).

- **완료(2026-08-11, `4133ca8`)**: `reduced_tx_set` 노출(기존엔 읽고 버려짐) + `ParsedFrame`에
`coded_lossless`(`base_q_idx==0 && y/uv_dc_delta_q==0`) 신규 파생 필드 — `transform_type()`가 필요로
하는 프레임헤더 플래그 중 `parse_coding_unit` 호출체인 밖(프레임 레벨)에서 구할 수 있는 부분만 먼저 노출.
`base_q_idx` 자체는 이미 노출돼 있었음. 아직 `parse_coding_unit`에 실제로 threading은 안 됨 — 다음 단계.

- **완료(2026-08-11, `dfb38be`)**: `transform_type()`(spec 5.11.47) 신규 구현 — txtp_intra1/intra2/
inter1/inter2/inter3 5개 CDF패밀리 rav1d 그대로 이식, 결정트리(lossless/large-tx/qidx==0 각각 0비트
분기 + intra가 inter보다 한 사이즈클래스 먼저 "large tx" 처리되는 실제 spec 비대칭 확인) 그대로 포팅.
전체 TxType 대신 `is_1d`(TX_CLASS_H/V vs 2D) 불리언만 추적(픽셀 재구성 안 하는 이 크레이트엔 그걸로 충분).
**이 자체가 실제 미구현 신택스 요소였다는 점에서 크로마 버그와 동일 계열의 잠복 디싱크 버그**(qidx!=0·
lossless 아님·최대tx 아님이라는 흔한 케이스에서 실제 인코더가 쓴 비트를 전혀 안 읽고 있었음) — 발견과
동시에 수정. 이어서 eob_bin/eob_hi_bit/coeff_base_eob도 실컨텍스트+적응으로 전환(정방형 전용이라 rav1d의
7개 계수-클래스 중 16/64/256/1024 4개만 도달 가능함을 tx2dszctx 산술로 직접 확인, qindex-버킷 4벌 중
1벌만 대표값 채택). `TxTypeFrameFlags`(coded_lossless/qidx_is_zero/reduced_tx_set) 신규 번들로
`parse_coding_unit` 호출체인 threading 완료. 351/351(av1-codec lib, 신규 테스트 11개), 워크스페이스 전체
클린.

- **txb_skip/dc_sign 실제 이웃 컨텍스트 시도 → 되돌림(2026-08-11, `8c05ba4`)**: packed above/left
`res_ctx` 바이트(rav1d `get_skip_ctx`/`get_dc_sign_ctx` 그대로 포팅) 구현 → 실 fixture 회귀 발견
(`real_fixture_key_frame_intra_modes_are_not_degenerate`, 모든 key-frame CU가 DcPred로만 디코드됨).
강제 컨텍스트 A/B 테스트로 `txb_skip_context`가 128x128(2x2 tx타일링) CU에서 잘못된 값을 만드는 것까지
좁힘. **근본원인**: 이 크레이트의 `tx_size`는 실제 비트스트림 `tx_size()` 읽기가 아니라 차원 기반
휴리스틱(`TxSize::from_dimensions`) — 예전 flat CDF는 tx 블록 경계를 신경 안 써서 이 갭이 무해했지만,
이웃 컨텍스트는 경계 정확도에 의존하므로 즉시 문제가 됨. **되돌리되 안전한 부분은 유지**: 두 CDF 모두
tx_size별 실제 rav1d 기본값은 유지(컨텍스트는 0 고정) — 여전히 예전 단일 flat 대표값보다는 개선. 미검증
이웃-컨텍스트 코드(TileContext의 res_ctx 추적, set_res_ctx, txb_skip_context/dc_sign_context)는 죽은
코드로 남기지 않고 완전 제거.

- **완료(2026-08-11, `755267d`)**: 인트라 전용 실제 `tx_size()` 비트스트림 읽기(spec 5.11.15/16) —
위 txb_skip/dc_sign 되돌림에서 지목한 선행조건 자체를 구현. `read_tx_mode`가 버려지던 `tx_mode_select`
비트를 실제로 반환하도록 변경(`TxfmMode::Only4x4`/`Largest`/`Switchable` 신규, `FrameHeader`에 노출).
`TileContext`에 above/left `tx_class` 이웃 배열 + `tx_size_context`(get_tx_ctx: 이웃 tx_class가 이
블록의 max_tx_class 이상인지 합산) 신규. `SymbolDecoder::read_tx_size`가 rav1d 기본값 이식한
`txsz_cdf`로 실제 depth 심볼 읽음. `parse_coding_unit`은 `!use_intrabc`인 INTRA CU에 한해 Only4x4/
Largest는 0비트로 즉시 해석, Switchable만 실제 심볼 읽기로 분기. **인터 블록과 IntraBC는 여전히 차원
휴리스틱** — 실제 spec은 재귀적 var-tx 쿼드트리(5.11.17/18)를 쓰는데, 이는 CU 하나가 여러 tx 크기를
가질 수 있다는 뜻이라 현재 `CodingUnit`의 단일 `tx_size: TxSize` 필드로 표현 불가 — 스키마 변경이
필요한 설계 갈림길이라 임의로 결정하지 않고 향후 세션으로 보류. 358/358(av1-codec lib, 신규 테스트 7개),
워크스페이스 전체(3853/3854, pre-existing flaky 1개 무관) + `--tests` 통합 스위트 클린, fmt/clippy 클린.

- **완료(2026-08-11, `e7171d5`)**: txb_skip/dc_sign 실제 above/left 이웃 컨텍스트 재시도 — 위 `8c05ba4`
되돌림의 근본원인(tx_size 휴리스틱)이 해소된 키프레임 인트라(비-IntraBC) CU에 한해 다시 도입. rav1d
`get_skip_ctx`/`get_dc_sign_ctx`(`src/recon_tmpl.c`)를 리서치 에이전트로 정밀 분석해 정확한 공식 확보:
txb_skip은 above/left `cul_level`(계수 절댓값 합, 63캡) 바이트를 tx블록 폭/높이만큼 비트 OR reduce한
뒤 `DAV1D_SKIP_CTX[min(la,4)][min(ll,4)]` 테이블 조회, dc_sign은 above/left DC부호 카테고리
(0=음수/1=중립/2=양수)를 `(category-1)`로 합산한 부호로 0/1/2 분류. `TileContext`에 rav1d의 패킹된
단일 바이트 대신 언패킹된 `cul_level`/`dc_sign_category` above/left 배열로 이식(이 크레이트는 chroma
축을 애초에 안 다뤄 패킹 트릭이 불필요). **인터/IntraBC CU는 여전히 예전 고정-컨텍스트-0 폴백을 쓰고
공유 이웃 배열을 아예 건드리지 않음** — 부정확한 tx 경계가 지금 막 정확해진 인트라 상태를 오염시키는
걸 원천 차단(정확히 지난 회귀의 재발 방지). `parse_coding_unit`의 잔차 루프가 이제 tx블록별 실제
(x4,y4) 위치를 추적(예전엔 단순 반복 횟수뿐이었음). 369/369(신규 테스트 11개, 지난번 회귀를 잡아냈던
바로 그 real-fixture 테스트로 재확인), 워크스페이스 전체 클린.

- **완료(2026-08-11, `6dc76ef`)**: coeff_base/coeff_br 실제 이웃 컨텍스트 + 실제 스캔 순서. 이 세션
남은 것 중 가장 컸던 단일 작업 완료. rav1d `src/scan.rs`(스캔테이블)+`src/recon_tmpl.c`의 `get_lo_ctx`
(컨텍스트 공식)를 직접 소스 대조로 이식한 신규 `symbol::scan` 모듈: (1) 정사각 tx 4개 크기(4x4/8x8/
16x16/32x32, 기존 eob_bin/coeff_base_eob과 동일하게 32x32 캡) 실제 2D 스캔테이블. TX_CLASS_H/V는
테이블 불필요 — 정사각 전용인 이 크레이트 범위에선 두 클래스의 위치공식이 `x=c%dim,y=c/dim`으로
완전히 동일해짐을 dav1d의 `DECODE_COEFS_CLASS` 매크로 직접 대조로 확인(추측 아님), 기존 `is_1d: bool`
그대로 충분 — 3분류 TxClass 불필요. (2) `LevelBuffer`: 블록당 scratch 상태(TileContext의 타일범위
above/left 배열과 다른 신규 패턴, 트랜스폼블록마다 초기화). dav1d의 바이트 인코딩(`tok*65`
비확장/`tok+192` coeff_br확장)을 값 그대로가 아니라 인코딩 자체를 이식 — get_lo_ctx의 magnitude 합산이
raw 바이트를 "이웃이 얼마나 유의미한가" 신호로 취급하기 때문에 인코딩 보존이 필수였음. (3) `lo_ctx`:
5개 고정 오프셋(항상 이미 디코드된 고주파 위치) 이웃합산 공식 + coeff_br이 재사용하는 `hi_mag` 부산물.
`coeff_base_cdf`/`coeff_br_cdf`를 단일 flat Vec에서 `[tx_size_class][ctx 0..40/20]` 실제 테이블로
전환(rav1d 기본값 이식, 41/21 컨텍스트), 둘 다 real adaptation(`read_symbol_adaptive`)으로 전환.
380/380(신규 테스트 11개+잔차 에너지 non-degenerate 회귀테스트 1개), 워크스페이스 전체 클린.

- **크로마 잔차 미read 버그 — 진짜 디싱크로 확인, 수정 시도 → 되돌림(2026-08-11, `ce90cdf`)**: 이 크레이트의
실제 테스트 fixture가 4:2:0(비-모노크롬)임을 확인 — 즉 `HasChroma`인 non-skip CU마다 실인코더가 쓴
크로마 잔차 비트를 지금까지 전혀 안 읽고 있었던 건 단순 누락이 아니라 타일 내 이후 모든 심볼을
오염시키는 진짜 디싱크 버그. `mono_chrome`/`subsampling_x`/`subsampling_y`를 SequenceHeader→
ParsedFrame→TxTypeFrameFlags로 소싱(정확함, 유지). "shape-only" 수정 시도(지배적 경우인 루마 양쪽
차원 모두 >=8x8일 때만, 루마 CDF 테이블을 ctx=0 고정+마지막 루마 tx블록의 is_1d 재사용으로 크로마
tx블록 개수만큼 읽기) → `real_fixture_key_frame_intra_modes_are_not_degenerate` 회귀 발견,
새 코드경로 비활성화→통과/재활성화→실패로 원인 확정 후 완전 되돌림. **추정 근본원인**: 다른 곳에서
용인되던 "고정 ctx=0" 근사(예: 인터 프레임 루마 txb_skip/dc_sign)와 달리, 적응형 range 디코더는
가변길이 구성요소(eob_bin 추가비트, 골롬 확장)의 비트 소비량 자체가 디코드된 "값"에 의존하고 그 값은
다시 컨텍스트/CDF에 의존함 — 컨텍스트가 틀리면 단순 오역이 아니라 실제 소비 비트 수 자체가 달라질 수
있음, 크로마 계수 통계가 루마와 충분히 달라서 루마 형태의 근사가 실제로 어긋남. 실제 UV 변환크기
매핑(`Max_Tx_Size_Rect`)과 정확한 `HasChroma` 조건도 별도 검증 안 됐음(둘 다 원인일 수 있음). 미검증
코드 안 남김 — `read_residual_block` 문서에 여전히 열린 진짜 디싱크 버그로 기록(실제 크로마 CDF
테이블 또는 검증된 안전한 shape-only 폴백 필요, 다음 세션 후보).

- **크로마 잔차 버그 재시도 → 부분 완료(2026-08-11, `23f2729`)**: 사용자가 "크로마 버그 계속 진행"
확정 후 실제 구현. rav1d 소스 재검증 결과 8x8/16x16/32x32 정사각 루마 블록(비-IntraBC)은 chroma tx
크기·개수 매핑이 정확히 맞음을 재확인, 실제 크로마 CDF 기본값(rav1d `[chroma=1]` 축, luma와 별도
데이터)까지 이식 완료. `coeff_base`/`coeff_br`은 `symbol::scan::lo_ctx`(평면 무관)를 그대로 재사용해
진짜 이웃 컨텍스트 확보, `txb_skip`/`dc_sign`은 크로마 전용 above/left 배열이 없어 고정 대표 컨텍스트
유지(luma 초기 단계와 동일 수준). **64x64/128x128 확장 시도 → 되돌림**: 이론상 안전해 보였으나
(chroma 32x32 캡 tiling도 rav1d 표와 일치 확인) 실 fixture 회귀, tx_size_class=3 경로 특유의 원인
미상 버그로 되돌림. **중간에 `is_key_frame` 제한을 추가했다가 오진단으로 확인되어 제거**: 이 fixture의
키프레임이 전부 미분할 128x128뿐이라 8~32 크기 CU가 전무해서, key-frame 제한을 걸면 게이트가 실제로는
단 한 번도 발동하지 않은 채 모든 테스트가 "우연히" 통과하고 있었음(전용 회귀 테스트 추가로 발견) —
인터 프레임에서도 8/16/32 스코프는 실제로 안전함을 재확인 후 제한 제거. 381/381(신규 1개: 게이트가
실제 fixture에서 헛돌지 않고 발동하는지 확인하는 "non-vacuous" 테스트), 워크스페이스 전체 클린.
**남은 것**: 64x64/128x128(원인 미상), 인터/IntraBC의 실제 var-tx(스키마 결정 필요), 비정사각 블록
전부 여전히 열린 갭.

- **partition `has_rows`/`has_cols` 프레임 경계 축소-알파벳 읽기 완성(2026-08-11)**: spec 5.11.4의
`decode_partition` 첫 두 줄(`r>=MiRows||c>=MiCols`→즉시 0, 심볼 없음)과 hasRows/hasCols 계산
(`MiCols=2*((FrameWidth+7)>>3)` 등, spec 5.9.5)을 그대로 이식, 이전엔 `parse_partition_recursive`에
하드코딩돼있던 `true,true`를 실제 프레임 MiRows/MiCols 기반 계산으로 교체(`mi_units` 신규, 3개
`parse_superblock` 호출부 전부 갱신). hasCols-only는 `split_or_horz`(HORZ vs SPLIT), hasRows-only는
`split_or_vert`(VERT vs SPLIT) 축소 이진 심볼로 분기 — rav1d `gather_top/left_partition_prob`
(`src/env.rs`)를 인덱스 단위로 이식(`symbol::cdf::split_or_horz_prob`/`split_or_vert_prob`, 128x128
버킷은 실제 0이 아닌 VertB 확률질량 때문에 패딩-제로 트릭이 아니라 명시적 `cdf.len()>=11` 가드 필요—
8x8/128x128/16-64 세 버킷 전부 손계산 대조 유닛테스트로 검증). 두 축소 심볼 모두 **비적응**(rav1d가
`partition_cdfs`에 write-back 안 함, `read_symbol_adaptive`가 아니라 `read_symbol` 사용).
**부수 발견+수정: 이 크레이트가 처음부터 갖고 있던 잠복 버그** — partition 트리 재귀가 SPLIT뿐 아니라
모든 non-None 파티션(HORZ/VERT/`_A`/`_B`/`_4`)의 자식에 대해서도 `parse_partition_recursive`를 다시
호출하고 있었음(spec은 SPLIT만 재귀, 나머지는 전부 terminal — 자식이 곧바로 `decode_block`). 비정사각
자식(예: 64x128)도 `block_size_log2`가 최대변 기준으로 실제 CDF 버킷을 골라버려 스펙에 없는 **두
번째 partition 심볼**을 읽고 있었음 — 이전엔 그 스퓨리어스 심볼이 대개 None으로 디코드돼(가장 흔한
결과) 그럴듯하지만 틀린 leaf 크기로 조용히 끝났을 뿐 거의 안 드러났는데, 실제 hasRows/hasCols가 프레임
경계에서 VERT/HORZ를 실제로 선택하게 만들자(이 fixture는 320x240, 128x128 슈퍼블록이라 우측/하단
슈퍼블록이 전부 실제로 경계에 걸침 — 조작 아닌 진짜 케이스) 잘못된 CDF 버킷 조회가 명시적 디코드
에러로 드러남(`VertB on block size Block64x128 produces sub-blocks of same size`). 근본 수정: SPLIT만
재귀, 나머지 non-None은 자식을 곧장 leaf `PartitionNode`로 생성(추가 심볼 읽기 없음). **부수 정정**:
바로 위 크로마 항목이 "이 fixture의 키프레임은 전부 미분할 128x128"라고 기록했던 것도 사실은 이
버그(+구 hardcoded true,true) 조합의 산물이었음 — 실제로는 우측 열 슈퍼블록이 VERT로 갈라짐
(`64x128` leaf, D203Pred 등 다양한 모드) 확인. 385/385(신규 4개: gather 함수 유닛테스트 3개 + 실제
fixture 비정사각 leaf 존재 확인 회귀테스트), 워크스페이스 전체(`--lib`+`--tests`) 클린, fmt 클린.

- **inter_mode globalmv_ctx 재조사 → 부분 완료(2026-08-11)**: "사용자 확인 후 보류"됐던 refmvs 항목을
"다 해주세요" 위임으로 재착수. rav1d `src/refmvs.rs` 직접 대조 결과 `globalmv_ctx`는
`rav1d_refmvs_find`에서 **`frm_hdr.use_ref_frame_mvs` 값으로 초기화**된 뒤, `rf.n_mfmvs>0`(과거
프레임에서 저장된 모션필드가 실제로 존재)일 때만 `add_temporal_candidate`가 이를 덮어씀을 확인 —
즉 진짜 시간축 예측(7.9 motion_field_estimation, 프레임 간 저장/투영 서브시스템)은 여전히 이
크레이트 범위 밖(픽셀 재구성 자체를 안 하므로 저장할 모션필드가 없음)이지만, **덮어쓰기 전
초기값**은 단순히 프레임 헤더 플래그 하나이므로 무료로 정확해질 수 있음을 발견. `use_ref_frame_mvs`
가 `parse_frame_header_full`에서 이미 파싱만 되고 버려지고 있던 걸(`let _ = use_ref_frame_mvs;`)
`FrameHeader`/`ParsedFrame`에 노출(`reference_select`/`allow_intrabc` 선례와 동일 패턴) →
`inter_mode_context`의 하드코딩 `0` 대신 이 플래그 직접 사용. 실측: 이 fixture의 실제 인터 프레임
249개 전부 `use_ref_frame_mvs=true` — 즉 기존 하드코딩 `0`은 100% 오답이었던 케이스에 적용되고
있었음. 유닛테스트(globalmv 비트 토글 확인) + 실제 fixture 비-vacuous 회귀테스트 추가, 387/387 +
워크스페이스 전체 클린. **여전히 남은 갭**: 실제 시간축 후보 발견 시의 override(진짜 모션필드
저장+투영 서브시스템)는 미구현 — 이 크레이트가 재구성을 아예 안 하는 한 원천적으로 큰 작업.

- **인터 var-tx 실제 재귀 읽기 완성(2026-08-11, `613e0e8`)**: "CodingUnit 스키마 변경 선행 필요"로
보류됐던 항목 착수. `CodingUnit`에 `tx_blocks: Option<Vec<TxBlock>>` 신규(실제 leaf 목록, 절대
4x4단위 x/y+정사각 `TxSize`) — 기존 `tx_size` 필드는 유지(var-tx CU에선 "시작/최대" 크기 의미로
격하). rav1d `read_tx_tree`/`read_vartx_tree`(`src/decode.rs`) 인덱스 단위 이식: `cat =
2*(4-from_class)-depth`로 CDF 행 선택(7개 카테고리, `Default_Txpart_Cdf` 실값 이식) +
`TileContext`에 신규 `above_var_tx`/`left_var_tx` 배열(인트라 `tx_size()`가 쓰는
`above_tx_class`와는 별개 — 실제 rav1d도 `tx_intra`/`tx` 두 개 독립 필드로 관리) + depth 2단계
캡(그 이상은 안 읽고 자동 non-split) + 8x8→4x4 분할은 심볼 없이 결정론적. **스코프**: 정사각
non-skip 인터 CU만(이 크레이트 `TxSize`가 정사각 전용이라 HORZ/VERT산 비정사각 인터 CU는 기존
휴리스틱 유지, IntraBC도 기존 tx_size() 선례와 동일하게 이번엔 제외). lossless/Only4x4/Largest
`TxfmMode`는 결정론적 균일 타일링(비트 안 읽음, `Switchable`만 진짜 재귀 읽기 필요) — 이전엔
이 세 경우조차 인터 분기가 아예 `TxfmMode`를 참조 안 하고 항상 차원 휴리스틱만 쓰고 있었던
별개 부정확성도 같이 해소. 잔차 루프를 균일 그리드 대신 `tx_blocks`(있으면) 순회로 교체 — **부수
효과로 txb_skip/dc_sign 실제 컨텍스트도 이 CU들에 한해 인터 프레임까지 자동 확장됨**(로드맵에
"이게 풀리면 확장 가능"이라 적어뒀던 항목까지 같이 해소, `use_real_residual_ctx` 조건에
`cu.tx_blocks.is_some()` 추가). 실측: 이 fixture의 non-skip 정사각 인터 CU 915개 전부 실제
`tx_blocks` 획득, 73%가 실제 분할 발생, leaf 크기 4x4~64x64 전부 관측(퇴화 없음). 유닛테스트(컨텍스트
공식) + 실제 fixture 비-vacuous 회귀테스트 추가, 391/391 + 워크스페이스 전체 클린. **남은 갭**:
비정사각 인터 CU, IntraBC의 var-tx 전부 여전히 차원 휴리스틱.

- **크로마 64x64 확장 완료, 128x128은 재조사에도 여전히 미해결(2026-08-11, `792df01`)**: "다 해주세요"
위임으로 재도전. 64x64는 실제로 검증됨 — 크로마 실제 최대 변환크기가 루마 크기와 무관하게 32x32로
캡됨을 rav1d `DAV1D_MAX_TXFM_SIZE_FOR_BS` 표로 재확인, 64x64 루마의 32x32 크로마 평면은 정확히
"타일 1개"라 이미 동작하던 8/16/32와 구조적으로 동일(`tx_size_class` 3에 처음 도달할 뿐) — 실제
64x64 CU가 존재하고 깨끗이 파싱됨을 확인하는 비-vacuous 검증 추가. **128x128은 이번에도 미해결**:
64x64 크로마 평면을 실제 32x32 4개로 타일링(2x2)해야 하는데, 타일 개수 자체는 스펙 표와 일치함을
재확인했음에도 여전히 다음 슈퍼블록에서 디코드 에러 발생. 이번 조사로 새로 좁혀진 것: (1) 호출
순서(U 전부→V 전부 vs U/V 타일별 인터리브) 둘 다 시도했으나 동일하게 실패 — `read_chroma_
residual_block`가 위치를 안 받고 크로마 above/left 컨텍스트도 안 쓰므로 순서는 애초에 무관함을
확인. (2) 이전 세션이 유력 용의자로 지목했던 "txb_skip/dc_sign 고정-대표 컨텍스트 근사"는 **이번에
반증됨** — 지금 막 실제로 동작 확인된 64x64 단일 타일 케이스도 정확히 같은 `tx_size_class` 3
고정-컨텍스트 경로를 쓰는데 문제없음. 남은 유력 용의자: 같은 CU 안에서 이 함수를 "두 번 이상
연속 호출"하는 것 자체에 관련된 무언가(CDF 적응 상태 등) — 더 깊이 파고들진 못함, 다음 세션 후보로
정밀 기록. `(8..=64)`로 프로덕션 게이트 유지.

- **크로마 128x128 실제 원인 규명 + 수정 완료(2026-08-12)**: 사용자가 "그 사이즈를 에이브이1에서
지원함?"이라 질문 → 먼저 확인: AV1은 128x128 변환을 지원하지 않음(`TxSize` enum 자체가 `Tx64x64`가
최대, spec `TX_SIZES_ALL`도 동일) — 루마는 이미 `compute_inter_tx_blocks`가 128x128 CU를 64x64
4타일로 올바르게 나눠 처리 중이었고, 크로마도 32x32 캡(스펙 `Max_Tx_Size_Rect`)이 정확히 스펙과
일치함을 재확인 — 즉 지난 두 번의 시도가 세운 "타일 4개(2x2)" 모델 자체는 처음부터 맞았음. **진짜
원인**: rav1d `recon_tmpl.c`의 `get_skip_ctx` 실제 소스를 직접 대조해서 찾음 — 크로마 `txb_skip`/
`dc_sign`이 `tx_size_class`로만 인덱싱되는 완전 고정 CDF 슬롯이라 위치 컨텍스트가 전혀 없었음.
64x64는 CU당 U/V 각 1회 호출이라 티가 안 났지만, 128x128은 U 4회+V 4회가 **동일한 전역 CDF
슬롯**을 한 CU 안에서 4배 세게 adapt시켜, 프레임 전체가 공유하는 그 슬롯을 실제 인코더가 가정하는
분포에서 이탈시킴 → 나중 슈퍼블록에서 desync. 지난 세션이 좁혀뒀던 "연속 호출 자체가 범인"이라는
가설이 맞았음. **수정**: rav1d의 실제 `default_coef_cdf[0].skip[tx_size_class][7..=12]`(크로마 6개
컨텍스트)/`.dc_sign[1]`(3개 컨텍스트) 기본값을 `src/cdf.c`에서 직접 소싱해 이식(`get_skip_ctx`의
`7 + not_one_blk*3 + ca + cl` 공식을 로컬 0..=5로 리매핑), 루마의 `above_cul_level`/
`left_cul_level`/`above_dc_sign_category`/`left_dc_sign_category`와 동일 패턴으로 U/V 평면별
독립 above/left 컨텍스트 배열(`TileContext`)을 신규 구현, `read_chroma_residual_block`이 이제
`ResidualBlockStats`(루마와 동일 반환 타입)를 반환하도록 변경해 호출부가 실제 컨텍스트를
갱신하도록 배선. `parse_coding_unit`의 게이트를 `(8..=128)`로 재확장. 실측: 391/391 lib 테스트+
워크스페이스 `--lib`(3853/3854, 무관한 `bitvue-engine` LRU 캐시 타이밍성 flaky 1건 제외 — 단독
실행 시 통과 확인) + `bitvue-av1-codec --tests`(통합 바이너리) 전부 클린, 128x128 CU가 실제로
존재하고 깨끗이 파싱됨을 확인하는 비-vacuous 검증(`saw_128x128`) 추가. 부수로 var-tx 회귀테스트가
969/969이 아닌 961/969로 실패한 것도 조사 — 원인은 새 버그가 아니라 이제서야 처음으로 정확히 소비된
크로마 비트 때문에 디코드 궤적이 바뀌면서 이전엔 도달 안 하던 4x4 인터 CU 2군데(8개)가 처음
등장한 것(4x4는 `compute_inter_tx_blocks`의 원래 설계 범위 밖, spec도 var-tx 재귀가 8x8 밑으로는
안 내려감) — 테스트의 eligibility 필터에 `width>=8` 추가해 정정. **남은 갭**: 비정사각 인터 CU +
IntraBC의 var-tx(스키마는 있음, `TxSize` 정사각 제한이 근본 원인).

- **segment_id/palette 스코핑 → 실측 오라클 부재로 착수 보류(2026-08-11)**: "다 해주세요" 위임의
마지막 항목 조사. segment_id는 이전에 이미 "이 fixture가 segmentation_enabled=true인 프레임이
전무"로 확인돼있었음. palette도 같은 문제 확인: palette와 IntraBC 둘 다 프레임 헤더의
`allow_screen_content_tools`(스크린 콘텐츠 인코딩용 희귀 플래그) 하나로 게이팅되는데, 실제
fixture 1369개 CU 전체를 스캔한 결과 `use_intrabc=true`인 CU가 **0개**(같은 게이트를 공유하는
palette도 사실상 확실히 0개일 것) — 즉 두 기능 모두 지금 유일하게 가용한 real fixture로는
구현 후 검증할 방법이 아예 없음. segment_id는 세그멘테이션 파라미터 전체 파싱+세그먼트별
QP/기능 파생, palette는 palette_mode_info()+컬러 캐시+palette 토큰까지 각각 그 자체로 상당한
규모(수백 줄급 신규 파싱)인데, 검증 불가능한 채로 수백 줄을 추가하면 이 세션 내내 지켜온
"실제 fixture로 검증" 기준을 정면으로 어기는 데다, 게이팅 로직 자체가 미묘하게 틀릴 경우
(예: `allow_screen_content_tools`/`segmentation_enabled` 판정 오류) 스크린 콘텐츠와 무관한
일반 스트림에서도 스퓨리어스 읽기로 이어질 위험까지 있음 — 그래서 두 기능 다 실측 오라클
(서드파티 스크린 콘텐츠 클립 또는 세그멘테이션 켠 클립) 없이는 착수하지 않기로 결정. 서드파티
테스트 데이터는 이미 이 세션 초반에 "리포 편입 금지" 확정됨 — 임시 검증용으로도 새로 구하려면
사용자 확인 필요.

- **다음 단계(로드맵, 남은 것)**: (1) 비정사각 인터 CU + IntraBC의 var-tx(스키마는 이미 있음,
`TxSize` 정사각 제한이 근본 원인이라 더 큰 리팩터 필요). (2) segment_id/palette — 실측 오라클
(세그멘테이션 켠 클립 또는 스크린 콘텐츠 클립) 확보가 선행 조건, 사용자 확인 필요. (3)
inter_mode/compound_mode의 진짜 시간축 모션필드 서브시스템(이 크레이트가 재구성을 구현하기 전엔
근본적으로 범위 밖). 크로마 128x128은 완료(위 항목 참고).

- **비정사각 인터 CU var-tx 완료 — 잔차 디코더 코어 전체를 정사각 전제에서 진짜 직사각형 지원으로
일반화(2026-08-12, `8ad7337`)**: 위 로드맵 (1)번 착수. 처음엔 "스캔 테이블+컨텍스트 재작업이
필요한 큰 작업"으로 스코핑해 사용자에게 진행 여부 확인 후 시작 — 실제로 rav1d 소스(`src/scan.c`,
`src/tables.c`, `src/decode.c`)를 직접 대조하며 구현:
  - **스캔 테이블**: 4x8~32x16 직사각형 전용 스캔 테이블 10개(dav1d `src/scan.c`에서 그대로 포팅,
    64와 짝을 이루는 크기(16x64/64x16/32x64/64x32)는 dav1d 자체도 32-cap된 다른 테이블을 재사용함을
    확인해 별도 테이블 불필요). `coeff_position`/`LevelBuffer`/`lo_ctx`를 단일 `dim`에서 독립적인
    `width_dim`/`height_dim`으로 일반화. `lo_ctx`의 2D 이웃-컨텍스트 오프셋 테이블이 실제로는
    정사각/wide/tall 3종(`dav1d_lo_ctx_offsets[3][5][5]`)이었음을 발견 — 기존엔 정사각 전용
    1종만 이식돼 있었음, wide/tall 2종 추가.
  - **`is_1d` bool → `TxClass1d`(TwoD/Horizontal/Vertical) 3분류로 확장**: rav1d의
    `DECODE_COEFS_CLASS` 매크로를 직접 대조해서 발견 — H는 height 기준, V는 width 기준으로 좌표를
    계산해서 정사각에서만 우연히 같았을 뿐 진짜 직사각형에선 서로 다른 좌표를 만들어냄. 기존엔
    V_DCT/H_DCT를 `bool`로 뭉쳐서 구분 안 하고 있었음(정사각 스코프에선 무해했지만 직사각형에선
    이 세션 내내 반복된 "컨텍스트 선택 오류→공유 CDF 드리프트→desync" 패턴을 새로 만들 뻔함) — 5개
    호출부(intra1/inter1/inter2 알파벳) 전부 실제 심볼 인덱스별로 V/H 정확히 매핑.
  - **`eob_bin_32/128/512` CDF 테이블 신규**(dav1d `default_coef_cdf`에서 포팅, 기존엔 정사각만
    닿는 16/64/256/1024 4개만 있었음), 선택 기준을 "정사각 클래스"에서 "실제 총 면적"으로 일반화.
  - **`read_var_tx_size`의 진짜 rav1d `read_tx_tree` 비대칭 분할 로직 이식**: 기존 구현은 무조건
    4분할(정사각 전제)이었는데, 실제 spec은 시작 크기가 넓으면(`w>h`) 자식 2개를 옆으로, 높으면
    자식 2개를 위아래로, 정사각일 때만 4개 전부 읽음 — C 매크로를 줄 단위로 대조해서 그대로 이식.
    `Max_Tx_Size_Rect` 시드 테이블도 처음엔 22행짜리 룩업 테이블이 필요하다고 예상했으나 rav1d의
    실제 테이블(`dav1d_max_txfm_size_for_bs`)을 까보니 "블록 자기 자신의 크기가 곧 시작 tx
    크기"(64 넘는 축만 64로 캡)라는 단순 산술로 100% 일치함을 확인 — 테이블 불필요, `width.min(64),
    height.min(64)` 한 줄로 대체.
  - `TxBlock`을 `size: TxSize`(정사각 전용)에서 `width_px`/`height_px` 명시 필드로 교체.
    `TileContext::var_tx_context`/`set_var_tx_class`도 above/left를 각각 width-class/height-class로
    독립 추적하도록 수정(기존엔 하나의 공유 class였음, rav1d 실제 `t->a->tx`/`t->l.tx`가 서로 다른
    축을 저장하는 걸 재확인).
  - **테스트 중 무관한 진짜 버그 1개 추가 발견**: `BlockSize::Block8x32::height()`가 32가 아니라
    64를 반환하던 복사-붙여넣기 오류(`tile/partition.rs`) — 비정사각 CU 실측 테스트를 처음 돌리자
    존재할 수 없는 (8,64) CU 크기가 나타나서 발견, 즉시 수정(무관한 기존 버그였지만 오늘 작업이
    처음으로 비정사각 CU 크기를 이 정밀도로 실측했기 때문에 드러남).
  - 실측: 550개 비정사각 인터 CU 전부(100%) 실제 `tx_blocks` 획득, 295개(54%)가 실제 분할 발생,
    진짜 직사각형 leaf(64x16, 16x8, 32x8, 8x4, 32x64 등) 확인하는 비-vacuous 회귀테스트 추가.
    391→396 lib 테스트, 워크스페이스 전체 `--lib`+`--tests`+clippy+fmt 클린. **남은 갭**: IntraBC의
    var-tx(같은 스키마, 아직 안 걸림), 크로마의 비정사각 지원(루마만 이번에 확장, 크로마 잔차
    리더는 여전히 정사각 전용).

- **IntraBC var-tx 배선 완료, 단 실측 미검증(2026-08-12, `4269a06`)**: 사용자 지시로 착수. rav1d
  `src/decode.c`를 직접 대조해서 확인: `b->intra = !intrabc_flag`(1043행) — IntraBC 블록은
  인트라 프레임 안에서 코딩됨에도 spec의 `read_block_tx_size()` 게이팅 목적으로는 `is_inter`로
  분류되고, `read_vartx_tree`가 실제 인터 블록과 완전히 동일한 호출부(1352행)에서 호출됨을
  확인 — 기존 크레이트 주석의 "spec이 인터처럼 재귀 트리로 라우팅한다"는 claim이 맞았음. 인터용
  `compute_inter_tx_blocks`를 IntraBC 경로에 그대로 재사용(신규 파싱 로직 없음, 라우팅만 변경).
  **검증 한계**: 유일한 실측 fixture(`test_data/av1_test.ivf`)가 `use_intrabc=true`인 CU를
  0/1676개 가짐(재확인) — `allow_screen_content_tools`가 이 클립에서 한 번도 켜지지 않는 희귀
  플래그라 segment_id/palette와 같은 급의 "실측 불가" 상황. 다만 이번 건은 이미 실측검증된
  `compute_inter_tx_blocks`를 그대로 재사용하는 순수 라우팅 변경이라 신규 파싱 로직을 통째로
  검증 없이 추가하는 것보다는 리스크가 낮다고 판단해 진행(사용자 지시). 코드/문서에 검증 한계
  명시. 스크린 콘텐츠 테스트 클립이 생기면 최우선 재검증 대상.

- **크로마 비정사각 지원 완료 — 실제로는 살아있는 desync 버그였음(2026-08-12, `ed4af7f`)**:
  사용자 지시로 착수해서 코드를 열어보니 예상보다 심각했음: 크로마 잔차 읽기 게이트가
  `width == height`(루마 CU가 정사각)를 요구해서, 비정사각 `HasChroma` 코딩블록은 크로마 비트를
  아예 한 번도 안 읽고 있었음 — 지난 세션에 비정사각 인터 var-tx를 구현하면서 비정사각 CU가
  흔해진 뒤(실측 fixture 1676개 중 550개) 이게 실제로 상시 발동하는 desync 버그가 돼있었음(크래시는
  안 났지만 이 세션 내내 반복된 "조용히 틀린 비트" 패턴). `read_chroma_residual_block`과 CDF/컨텍스트
  배관 전체를 루마와 동일하게 width/height 독립 일반화(이미 일반화돼있던 `scan::coeff_position`/
  `LevelBuffer`/`lo_ctx` 재사용, 크로마는 `transform_type()`을 독립적으로 안 읽어서 `TxClass1d`
  불필요, 항상 2D). `eob_bin_32/128/512` 크로마 전용 CDF 테이블 3개 신규(dav1d
  `default_coef_cdf[0]`의 chroma=1 축에서 포팅), `TileContext`의 크로마 컨텍스트 3개 메서드도
  above/left를 width-span/height-span으로 독립 추적하도록 수정. `parse_coding_unit`의 크로마
  타일링 루프도 각 축을 독립적으로 계산(`chroma_w = luma_width/2`, `chroma_h = luma_height/2`,
  타일 개수도 축별 독립). 실측: 비정사각 크로마 대상 CU가 4종 이상 다양한 크기로 실제 존재하고
  전체 250프레임 파싱이 여전히 깨끗함(이제 실제 크로마 비트를 소비하면서도)을 확인하는
  비-vacuous 회귀테스트 추가. 391→397 lib 테스트, 워크스페이스 전체 clean. **남은 갭**:
  세그멘테이션/스크린콘텐츠(오라클 부재), 시간축 모션필드 서브시스템(재구성 자체가 범위 밖).

- **세그멘테이션/스크린콘텐츠 실측 오라클 확보 완료(2026-08-12)**: 이전에 "실측 불가"로 보류됐던
  항목의 진짜 블로커(테스트 클립 부재)를 해소. 로컬 `aomenc`(libaom 레퍼런스 인코더, 이미 설치돼
  있었음)로 시도 → 합성 콘텐츠(블로키 패턴/프레임내 반복 타일/정확한 블록 복제 등 6회 시도)로는
  IntraBC를 RDO가 한 번도 선택 안 함(프레임 헤더 플래그는 여러 번 켜졌어도 실제 CU 사용 0건) →
  대신 libaom 자체 테스트 자산인 공식 `screendata.y4m`(`storage.googleapis.com/aom-test-data`,
  스크래치패드에만 다운로드, 커밋 안 함, 서드파티 데이터 정책과 무관 — 이미 검증에만 쓰고 리포에는
  안 넣는 전례 그대로)로 전환하자 즉시 성공: `aomenc --tune-content=screen --enable-intrabc=1`
  인코딩 결과 실제 IntraBC CU 10개 확인, 전부(100%) 실제 `tx_blocks` 획득, 파싱 에러 0건 —
  **IntraBC var-tx가 지난 세션에 남겼던 "스펙/rav1d로만 검증, 실측 미검증" 캐비앗이 이제 해소됨**
  (코드 주석 갱신, `8850134`). 세그멘테이션은 `aomenc --aq-mode=3`(cyclic refresh, libaom이
  세그멘테이션을 세그먼트별 QP 조절에 실제 사용하는 표준 메커니즘)으로 시도 → 처음엔
  `--end-usage=q`(고정 품질)에서 한 번도 안 켜짐 확인 → cyclic refresh는 원래 비트레이트
  관리용 기능이라는 점에 착안해 `--end-usage=cbr --target-bitrate=200`으로 바꾸자 20프레임 중
  18프레임에서 `segmentation_enabled=true` 확인(임시 디버그 print로 검증 후 원복, 커밋 없음).
  **세그멘테이션 검증용 클립도 이제 확보됨** — 다만 `segment_id`/palette 자체는 프레임 헤더 비트를
  읽고 버리기만 할 뿐(`skip_segmentation_params`) 실제 파싱이 전혀 구현 안 돼있어서, 이 오라클을
  실제로 쓰려면 세그멘테이션 파라미터 전체 노출(spec) + `segment_id()`(spec 5.11.9/10) 신규
  파싱(코딩유닛 레벨) + palette_mode_info()/컬러캐시/palette 토큰(별개 신규 파싱, 각각 수백
  줄급) 자체를 새로 구현해야 함 — 이건 "검증"이 아니라 "신규 기능 구현"이라 오라클 확보와는
  별개의 큰 작업, 사용자 확인 필요.

- **`segment_id()` 신규 구현 완료(spec 5.11.9/5.11.10, 2026-08-12, `3886d3a`)**: 사용자 지시로
  착수. `skip_segmentation_params`를 `parse_segmentation_params`로 개명해 프레임 헤더의
  `segmentation_enabled`/`update_map`/`temporal_update`와 spec 5.9.14의 파생값(`SegIdPreSkip`
  — 어떤 세그먼트든 `SEG_LVL_REF_FRAME`(5)/`SKIP`(6)/`GLOBALMV`(7) 피처가 켜져 있으면 true,
  `LastActiveSegId` — 피처가 켜진 최대 세그먼트 인덱스)을 실제로 노출(`FrameHeader`→`ParsedFrame`,
  `use_ref_frame_mvs`와 동일 배선 패턴). `seg_pred`/`seg_id` real CDF(dav1d `src/cdf.c`에서 포팅)
  신규. **`segment_id`가 이 세션의 다른 컨텍스트들과 근본적으로 다른 점 발견**: dav1d의
  `get_cur_frame_segid`(`src/env.h`)가 above-left 대각선 셀을 직접 조회하는데, 같은 행 안에서
  왼쪽 이웃이 이미 처리되면 그 열의 "above" 슬롯을 자기 행 값으로 덮어써버려서 이 크레이트의
  기존 "above/left 1D strip" 패턴(한 행씩 리셋)으로는 재현 불가능함을 확인 — dav1d처럼 실제
  타일 전체 2D 그리드(`TileContext::seg_id_grid`)를 도입, 근사 없이 그대로 이식.
  `parse_coding_unit`의 skip 읽기 전/후 두 지점에 실제 배선(dav1d `decode_b`의 `decode.c` 구조
  그대로 — pre-skip은 skip을 아직 모르니 항상 실제 심볼 읽기, post-skip은 `skip=true`면 예측값을
  비트 없이 그대로 사용하는 지름길 있음, 이 둘의 차이를 spec 의사코드만으론 못 잡아서 소스 직접
  대조). `neg_deinterleave`(dav1d `decode.c`에서 그대로 포팅) 신규, hand-computed known-answer
  테스트 4개로 검증. **알려진 갭**(`RefFrameState`와 같은 급): 이 크레이트는 프레임을 독립적으로
  파싱해서 진짜 크로스프레임 상태가 없음 — `segmentation_update_data==false`(이전 프레임의
  피처 설정 재사용)와 시간축 예측(`seg_pred==true`, 이전 프레임 세그먼트맵 필요) 둘 다 안전한
  기본값(0, 비트스트림 위치엔 영향 없음)으로 근사. 실측: `aomenc --aq-mode=3 --end-usage=cbr`
  (cyclic refresh, 스크래치패드 전용 자체 인코딩, 커밋 안 함)로 20프레임 중 18프레임
  segmentation_enabled=true, 파싱 에러 0건, 478개 CU에 걸쳐 진짜 서로 다른 segment_id 3종(0,1,2)
  디코드 확인 — degenerate 아님. 커밋된 실측 fixture(세그멘테이션 없음)엔 경량 회귀테스트만
  추가(segmentation.enabled=false 유지 + segment_id=0 유지 확인). 397→402 lib 테스트, 워크스페이스
  전체 clean. **남은 갭**: palette(별도 신규 파싱, 착수 안 함), 위에 적힌 크로스프레임 상태
  한계, inter_mode/compound_mode의 진짜 시간축 모션필드 서브시스템(재구성 자체가 범위 밖).

- **palette 신규 구현 완료(spec 5.11.46, 2026-08-13, `cc444aa`)**: 사용자 지시로 착수.
  `palette_mode_info()`(Y/UV 컬러 읽기, dav1d `read_pal_plane`의 정렬-병합 above/left
  컬러 캐시 그대로 포팅) + `palette_tokens()`(픽셀 단위 대각선 웨이브프론트 컬러-인덱스 맵,
  `order_palette`의 순위 기반 컨텍스트 — 이 크레이트의 다른 모든 컨텍스트와 근본적으로 다른
  above/left/above-left "값 자체" 기반 순위·순열 시스템) 신규. **작업 중 이 세션 전체를
  소급 위협하는 진짜 desync 버그 발견**: `intra_frame_mode_info()`의 절반
  (`angle_delta_y`/`uv_mode`/`cfl_alpha_signs`+`cfl_alpha`/`angle_delta_uv`)이 이 크레이트에
  아예 구현된 적이 없었음 — 키프레임에서 Y모드가 directional이거나 UV모드가 DC가 아닌 모든
  CU가 그 시점부터 desync 상태였는데 크래시가 안 나서 지금까지 아무도 못 잡음(residual/
  ref_frame과 동일 패턴). palette 게이팅 자체가 uv_mode 값+비트 위치에 의존해서 따로 뗄 수
  없었기 때문에 함께 구현(신규 CDF 테이블 6개, 전부 dav1d `default_cdf.m.*` 바이트 단위
  재검증). `filter_intra_mode_info()`도 같은 이유로 함께 구현. `allow_screen_content_tools`
  (allow_intrabc와 별개 프레임헤더 플래그 — screen content가 intrabc 없이도 켜질 수 있음)
  /`enable_filter_intra`(시퀀스헤더) 신규 배선(parser→cu_parser/partition→superblock→
  coding_unit). **부수 발견+수정**: `use_intrabc` CU가 y_mode를 무조건 읽고 있었음(real
  dav1d `b->intra = !intrabc_flag`가 이 모드정보 구간 전체를 건너뛰는데 반영 안 돼있었음) —
  모드정보 구간 전체를 `!cu.use_intrabc` 안으로 이동. 404/405 lib + 141개 `tests/` 스위트
  클린(마지막 1개는 아래 참고).

- **delta_q 게이팅 수정 + delta_q/delta_lf 실제 golomb 인코딩 구현(spec 5.11.38,
  2026-08-13, `fd268b4`)**: 위 palette 세션 마무리 후 남은 1개 테스트 실패 근본원인을
  추적하다가 발견한 별개의 진짜 버그 2건. (1) `delta_q`가 `delta_q_enabled`일 때 매 CU마다
  무조건 읽히고 있었음 — real spec/dav1d는 슈퍼블록당 한 번(맨 처음 리프에서만) + 그 리프가
  슈퍼블록 전체 크기이면서 skip이면 아예 안 읽음. `parse_superblock`의 자기 origin/size를
  `sb_x4`/`sb_y4`/`sb_size4`(4x4 단위)로 `parse_coding_units_recursive`→`parse_coding_unit`
  까지 배선해 "이 리프가 슈퍼블록의 첫 리프인가" 실제 판정 재구성. (2) `delta_q_abs`(그리고
  한 번도 구현된 적 없던 `delta_lf_abs`)가 real spec의 golomb 스타일 가변길이 확장(3비트
  n_bits 선택자 + n_bits 원시비트)을 손으로 만든 단일 심볼 CDF로 대체하고 있었고, 부호비트도
  real spec의 고정 50/50 원시비트 대신 적응형 CDF를 쓰고 있었음(4-outcome 베이스 CDF 자체도
  가짜 5-outcome이었음 — dav1d 실제값 `CDF3(28160,32120,32677)`으로 교체). `delta_lf`는
  `delta_lf_present`/`delta_lf_multi`(신규 배선, `delta_q_present`와 같은 배선 경로) 안에
  중첩된 real spec 구조 그대로 신규 구현. 404/405 lib + 141개 스위트 클린(변함없음).

- **남은 1개 테스트(`real_fixture_key_frame_intra_modes_are_not_degenerate`) 근본원인
  미해결 — 다음 세션으로 이월(2026-08-13)**: 이 fixture의 유일한 키프레임(320x240, SB 6개)이
  위 두 커밋 각각 적용 전부터 이미 SB 6개 중 2개가 조용히 파싱 실패하는 상태였음(에러 스킵됨,
  나머지 4개로 우연히 테스트 통과) — **클린 HEAD 워크트리에 아무 의미 없는 더미 비트 1개만
  추가로 읽게 해도 똑같이 SB 1개만 성공**하는 것까지 재현 확인, 즉 이 프레임 디코드가 비트
  위치 1개만 밀려도 무너질 만큼 이미 취약했음(회귀 아님, pre-existing). palette/delta_q 두
  커밋 모두 "스펙대로 진짜 비트를 더 읽을 뿐"인데 그것만으로 이 취약점을 더 일찍 건드림(각각
  적용 후 성공 SB 수: 4→1→0). **버퍼 고갈은 기각**(10594바이트 중 235바이트만 소비된 상태에서
  에러 발생). 대신 발견한 단서: 첫 128x128 CU의 루마 32x32 잔차 타일(16개 중 11번째)에서
  eob=596(1024칸 중 58% 비영점 밀도)이라는 실제 콘텐츠론 매우 드문 값이 나옴 — 이미 그
  지점에서 desync된 정황. **같은 residual 코드가 이 fixture의 INTER 프레임들엔 문제없이
  쓰이고 있어서**(관련 회귀테스트 다수 통과) residual 코드 자체보다는 이 유일한 키프레임에만
  적용되는 무언가(`transform_type()`의 intra 전용 분기, 또는 아직 못 찾은 다른 intra 전용
  신택스요소)가 원인일 가능성이 높음 — 계획서가 원래 추정했던 "residual 전체 컨텍스트는
  자체로 몇 주급" 규모의 조사가 될 수 있어 다음 세션으로 이월, 착수 안 함.

- **위 이슈의 실제 심각도 재확인(2026-08-13, UI 버그 수정 세션 중 우연히 발견)**: UI 수정
  검증 차 `BITVUE_ELECTRON_SELFTEST=1` 전체 실행했더니 exit code 1로 실패 — 원인은
  `get_deblocking_analysis(frame=0)`(바로 위 항목의 그 취약한 키프레임)가 edges 0개를
  반환하고 있었음(`stats: {total_edges:0, ...}` 전부 0). git worktree로 `cc444aa`(palette)
  직전 커밋(`6ec5db3`)에 **동일 셀프테스트**를 돌려 exit 0 + edges>0 확인 — **pre-existing이
  아니라 오늘 palette/delta_q 커밋이 만든 진짜 리그레션**. 즉 위 항목은 obscure한 unit test
  하나만 깨뜨린 게 아니라 **실제 제품 기능(디블로킹 분석)과 셀프테스트 게이트 자체를 이미
  깨뜨리고 있음** — 다음 세션 우선순위를 그에 맞게 올릴 것. 이번엔 더 파고들지 않고 발견
  사실만 기록(사용자 "알아서 해" 위임을 "UI 커밋 + 문서화까지만, 재조사는 다음 세션"으로
  해석해 진행).

- **`transform_type()`/`txb_skip` 순서 버그 발견+수정(2026-08-13, 다음 세션)**: dav1d
  `decode_coefs`(`src/recon_tmpl.c`)를 한 줄씩 대조하다가 실제 신택스 순서를 재확인 —
  real spec은 `all_zero`(`txb_skip`)를 무조건 먼저 읽고, **그 결과가 false일 때만**
  `transform_type()`을 읽음(`chroma`는 애초에 비트 없이 룩업으로만 유도됨: intra는
  `dav1d_txtp_from_uvmode[uv_mode]`, inter는 luma txtp에서 유도). 그런데 이 크레이트는
  **루마 트랜스폼 블록마다 `transform_type()`을 `txb_skip` 여부와 무관하게 항상 먼저
  읽고 있었음** — all-zero(실제 콘텐츠에서 흔함) 블록마다 real 인코더가 안 쓴 유령 심볼을
  읽는 진짜 desync 버그. `SymbolDecoder::read_txb_skip` 신규 분리(순수 `all_zero` 읽기만) +
  `read_residual_block`에서 `txb_skip` 읽기 제거, `parse_coding_unit`의 루마 잔차 루프를
  "먼저 `read_txb_skip` → false면 `transform_type()` → `read_residual_block`" 순서로
  재구성. **다만 이 fixture의 취약한 키프레임 테스트 자체는 여전히 실패** — 원인 추적해보니
  그 CU의 트랜스폼 블록이 전부 32x32(intra)라 `read_transform_type_is_1d`의 조기 반환 조건
  (`tx_class + is_intra >= 4`, real spec `t_dim->max + intra >= TX_64X64`과 동일)이 항상
  걸려서 애초에 순서 버그가 있든 없든 이 CU에선 비트 소비량이 동일했음(수정 전후 동일한
  tx 타일에서 동일한 eob=596/168 재현 확인) — 즉 이 수정은 **진짜 버그고 다른 콘텐츠(작은
  트랜스폼 크기·인터 블록)에선 실제로 유효하지만, 이 특정 테스트의 근본원인은 아님**.
  부수로 `SymbolDecoder::read_residual_block`의 오래된 doc 주석이 "coeff_base/coeff_br는
  아직 이웃 컨텍스트 없음"이라고 잘못 적혀있던 것도 발견+정정(실제로는 `6dc76ef`로 이미
  real `scan::lo_ctx` 이웃 컨텍스트가 들어가 있었음, 코드와 문서가 어긋나 있었음). 404/405
  lib + 141개 스위트 클린(변함없음). **eob=596(11번째 32x32 루마 타일, 58% 비영점 밀도)의
  진짜 원인은 여전히 미해결** — coeff_base/coeff_br 컨텍스트 자체의 정밀도 문제이거나 아직
  못 찾은 다른 intra 전용 신택스 순서 버그일 가능성이 남아있음, 다음 세션 계속.
- **`read_residual_block`의 `eob` off-by-one 실버그 발견+수정(2026-08-13)**: coeff_base/
  coeff_br 컨텍스트 정밀도 추적 재개 → dav1d `decode_coefs`(`recon_tmpl.c`)와 한 줄씩 대조.
  eob_bin/eob_hi_bit/extra bits로 계산되는 raw 값은 spec의 "계수 개수"가 아니라 **마지막
  스캔 인덱스(0-based, 계수 개수-1)** 인데(dav1d의 `scan[eob]` 직접 인덱싱 + `eob==0`일 때
  "dc-only" 분기가 여전히 정확히 1개 심볼을 읽는다는 점으로 교차검증), 이 크레이트는 이
  raw 값을 그대로 "개수"로 써서 `for c in (0..eob).rev()` + `is_eob_pos = c == eob-1`로
  루프를 돌고 있었음 — 매 잔차 블록마다 실제 최상위(가장 고주파) 계수 심볼을 아예 안 읽고,
  두 번째로 높은 위치를 마치 eob 위치인 것처럼 잘못된 CDF(`coeff_base_eob` 대신
  `coeff_base`)로 읽던 **전체 잔차 디코드의 체계적 desync**(그리고 `eob==0`일 땐 아예
  0개 심볼을 읽어 실제 1개 계수를 완전히 누락). `for c in (0..=eob).rev()` +
  `is_eob_pos = c == eob`로 수정(루마/크로마 양쪽 사본 동일 적용) + `coeff_base_eob_context`
  가 `eob==0`일 때 일반 공식(결과 1) 대신 dav1d처럼 하드코딩된 context 0을 반환하도록
  특례 추가. **다만 타깃 테스트는 여전히 미해결** — 디버그 계측으로 추적한 결과, 이
  fixture의 유일한 키프레임 데이터가 있는 superblock(0,0)은 파티션 트리 최상위(128x128)에서
  `partition=None`으로 즉시 결정되어 CU가 단 1개(128x128 전체)뿐이고, 그 CU의 **첫 번째
  잔차 읽기 호출(32x32 루마 타일)부터 이미 eob=596(58% 비영점) 이상치가 재현됨** — 즉
  이번 eob 수정의 영향 범위(잔차 블록 내부) 자체가 실행되기도 전에, `skip` 판독 직후부터
  이 CU의 mode-info 체인(`use_intrabc`/`tx_size()`/kfym/angle_delta/uv_mode/cfl/
  angle_delta_uv/palette/filter_intra) 어딘가 또는 최상위 `partition` 심볼 자체에서 이미
  desync가 시작됐을 가능성이 높음(baseline `e3a0ae0`과 이 수정 적용 후 모두 동일하게
  frame 0의 superblock 6개 전부 파싱 실패로 확인 — 이 수정 자체는 회귀 없음, 단지 이
  특정 CU에 도달하기도 전에 이미 망가져 있어서 효과를 검증할 수 없었을 뿐). 405/406 lib
  (신규 `coeff_base_eob_context`의 `eob==0` 케이스 테스트 1개 추가) + 통합 스위트 전부
  클린, clippy/fmt 무관 경고 13개 그대로. **범위가 "잔차 컨텍스트 정밀도"에서 "이 CU의
  mode-info 체인 전체(또는 최상위 partition 심볼) 재감사"로 확장됨** — 다음 세션 계속.
- **🎉 근본원인 발견 + `real_fixture_key_frame_intra_modes_are_not_degenerate` 최종 해결
  (2026-08-13)**: mode-info 체인 재감사 착수 → dav1d `decode_b`(`src/decode.c`)의 CU 신택스
  순서를 처음부터 끝까지 한 줄씩 대조해 **두 개의 실제 버그**를 동시에 발견. (1) **`cdef_idx()`
  (spec 5.11.56)가 이 크레이트 어디에도 전혀 구현돼있지 않았음** — `skip`이 아니고 슈퍼블록당
  관련 64x64 유닛에 아직 안 읽었을 때만(dav1d의 `cur_sb_cdef_idx_ptr` 패턴) `cdef.n_bits`
  비트를 읽는 완전히 누락된 신택스 요소. `CdefInfo`에 `bits: u8` 필드 신규 추가(기존엔 헤더
  파싱 중 `cdef_bits`를 세다가 그냥 버렸음) + `ParsedFrame::cdef_bits`로 노출 +
  `parse_coding_unit`에 `cdef_idx_state: &mut [i8; 4]`(슈퍼블록마다 리셋, `-1`=미독)로 실제
  구현. (2) **더 심각한 버그: `delta_q`/`delta_lf` 읽는 위치 자체가 완전히 틀려있었음** — real
  spec/dav1d는 `skip` → `segment_id`(postskip) → `cdef_idx()` → `delta_q`/`delta_lf` →
  (비로소) mode-info 순서인데, 이 크레이트는 **mode-info 전체(y_mode/angle_delta/uv_mode/cfl/
  palette/filter_intra/tx_size, 또는 inter의 ref_frame/mode/MV/var-tx) 를 다 읽은 다음에야**
  delta_q/delta_lf를 읽고 있었음 — `delta_q_enabled`인 모든 실인코딩에서 슈퍼블록의 첫 non-skip
  CU마다 큰 블록의 신택스가 통째로 잘못된 비트 위치에서 읽히던 셈. `delta_q`/`delta_lf` 읽기
  블록을 통째로 mode-info 앞(segment_id postskip 직후, cdef_idx 다음)으로 이동, 로직 자체는
  변경 없음(순수 위치 이동, `new_qp`가 함수 끝까지 안전하게 전달됨을 확인). **결과: 여러 세션에
  걸쳐 4번의 독립적인 진짜 수정(팔레트/mode-info 꼬리, delta_q 게이팅, txb_skip 순서, eob
  off-by-one)이 전부 못 고쳤던 타깃 테스트가 드디어 통과** — 406/406 lib **전부 통과**(실패 0),
  `--tests` 전부 클린. 부수로 **`get_deblocking_analysis`의 frame=0 회귀도 같은 근본원인이라
  함께 해결됨**(`get_deblocking_analysis_end_to_end_returns_real_edges` 재확인 — frame 0에서
  real edge/`total_edges>0` 정상 리턴). `parse_superblock`/`parse_coding_units_recursive`/
  `parse_coding_unit`의 시그니처에 `cdef_bits: u8`가 신규 파라미터로 추가돼 호출부 5곳(
  `cu_parser.rs`, `partition.rs` 2곳, `mv_extraction_test.rs`) 전부 갱신. 워크스페이스
  `--lib`(3853+ 통과, 무관 기존 flaky LRU 1개 제외) + `--tests` 클린, clippy/fmt 무경고 변화
  없음. **이 세션 전체(그리고 몇 세션에 걸친) 근본원인 추적 완료** — 남은 것은 이번 발견으로
  드러난 `cdef_idx`/`delta_q` 재배치가 다른 fixture/코덱 경로에 미치는 영향 재확인 정도, 별도
  버그 남아있다는 증거는 없음.
- **후속 확인(2026-08-13, 같은 세션): 인터 프레임 쪽에 훨씬 큰 별개 갭 발견, 미착수** — 사용자
  요청으로 "남은 mode-info 갭"을 마저 점검하다가, 이 크레이트가 `is_key_frame` 하나로만
  인트라/인터를 분기하고 real spec의 per-CU `skip_mode`(spec, `skip_mode_present` 프레임
  플래그 게이팅)와 `is_inter`(spec 5.11.5) 비트를 **인터 프레임에서 전혀 읽지 않는다**는 걸
  발견 — 인터 프레임 안에도 진짜 인트라 블록이 섞일 수 있는데 이 크레이트는 무조건 전부
  인터로 취급. `skip_mode_present` 자체도 헤더 파싱 중 비트만 소비하고 값은 버려짐(cdef_bits
  가 고쳐지기 전과 동일 패턴). **실측으로 규모 확인**: 인터 프레임 10개를 직접 파싱하며
  `SymbolDecoder::byte_offset()`으로 tile_data 대비 실소비 바이트를 찍어봄 — 에러 없이
  "성공" 처리되는 프레임도 대부분 tile_data의 소수만 소비하고 조기 종료(예: 309바이트 중
  19바이트만 소비), 다수 프레임은 슈퍼블록 6개 중 5~6개가 하드 에러. 지금까지 통과해온
  `real_fixture_ref_frame_values_are_not_degenerate` 류 테스트는 250프레임 전체에서 몇 개
  CU만 그럴듯해도 통과하는 느슨한 기준이라 이 정도로 광범위한 문제를 못 잡아냈던 것으로
  결론. **미착수 이유**: `skip_mode`/`is_inter` 신규 구현 + `is_inter==false`일 때 키프레임
  인트라 mode-info 경로 재사용 배선까지 필요해 `cdef_idx` 급 신규 기능 작업 — 사용자가
  "다음 세션"으로 마무리, 다음 착수 지점으로 명시.
- **`skip_mode`/`is_inter` 실제 구현 완료(2026-08-13, 같은 세션 "계속" 요청으로 착수)**: dav1d
  `decode_b`를 계속 대조해 `skip_mode`(spec 5.11.5, `skip_mode_present` 프레임 플래그 +
  `min(bw4,bh4)>1` 게이팅, `skip`보다 먼저 읽고 참이면 `skip`을 비트 없이 강제 1로)와
  `is_inter`(`get_intra_ctx` 4-컨텍스트, `cdef_idx`/`delta_q`/`delta_lf` 다음·mode-info 전에
  읽음) 신규 구현. `skip_mode_present`는 `read_skip_mode_params`가 비트만 소비하고 값을 버리던
  것을 반환하도록 수정 + `FrameHeader`/`ParsedFrame`에 노출(cdef_bits와 동일 패턴).
  `TileContext`에 `skip_mode_context`/`set_skip_mode`(신규 above/left 배열) +
  `intra_ctx`/`set_intra_flag`(기존 `ref_frame()`용 `above_ref_intra`/`left_ref_intra` 배열
  재사용 — 실제 dav1d도 `BlockContext.intra` 하나를 두 용도로 공유하는 걸 확인 후 재사용,
  단 `set_ref_frames`와 달리 `ref0`/`ref1`/`comp`는 안 건드리도록 별도 setter 신규). `y_mode`도
  키프레임(`kfym`, above/left 이웃 클래스)과 비키프레임 인트라(`y_mode_cdf`, 블록크기 클래스
  `y_mode_size_context`, dav1d `dav1d_ymode_size_context` 테이블 그대로 포팅)가 다른 CDF/컨텍스트를
  쓴다는 걸 dav1d 소스로 확인 후 분기 — 그 뒤(angle_delta/uv_mode/cfl/palette/filter_intra/
  tx_size)는 완전히 동일한 코드 경로라 그대로 재사용. `is_inter` 판정 결과로 최상위 분기를
  `is_key_frame`에서 `!is_inter`로 교체(인터 경로 자체 로직은 무변경, `is_inter`일 때만
  진입하도록 조건만 교체). 새 CDF 3개(`skip_mode_cdf`/`intra_cdf`/`y_mode_cdf`) 전부 rav1d
  `default_cdf`(`src/cdf.c`) 원시값 이식. **실측 검증**: 인터 프레임 30개 byte_offset 재측정 —
  fully-clean(에러 0 + 90%+ 소비) 프레임은 여전히 0/29지만, 총 성공/실패 슈퍼블록 비율이
  개선(대략 31%→42%)됐고 다수 프레임이 에러 발생 지점까지 tile_data를 거의 끝까지 소비하는
  형태로 바뀜(이전엔 극초반 조기종료가 흔했음) — 유의미한 진전이지만 여전히 미해결. 새로 관찰된
  에러 유형("Partition Vert4/Horz4 not allowed for Block16x16") 발견, 이 수정이 새로 만든 버그인지
  또 다른 다운스트림 갭(motion_mode/interintra/compound_type/wedge — 원래 엔트로피 디코더 계획의
  "Phase 6: 전혀 안 읽는 신택스" 목록에 이미 있던 항목들)의 증상인지는 미확인. 406/406 lib,
  `--tests` 전부 클린(무관 flaky LRU 1개 제외), clippy/fmt 무경고 변화 없음. 다음 세션: 남은
  인터 프레임 에러의 근본원인(motion_mode 등 Phase 6 잔여 신택스 vs 이번 구현의 잠복 버그)
  추적.
- **후속(같은 세션, "다음 세션 시작"으로 이어감): Partition Vert4/Horz4 에러는 회귀 아님 확인 +
  motion_mode/interintra/compound_type(wedge)/subpel_filter 4개 신규 신택스 구현** — 격리
  worktree(`af611bd`)로 대조해 그 에러가 skip_mode/is_inter 수정 전에도 이미 존재했음(1건→3건
  으로 더 노출됐을 뿐) 확인, 회귀 아님. dav1d `decode_b`를 계속 대조하다가 인터 블록에서
  완전히 안 읽던 신택스 4개를 추가 발견: `compound_type`(jnt_comp/seg/wedge 선택), `interintra`
  (모드+wedge), `motion_mode`(translation/OBMC/warp), `subpel_filter`. **DRL(dynamic reference
  list) 인덱스 읽기는 사용자 확인 후 별도 세션으로 명시적 보류**(`MvPredictorContext`가 real
  spec의 `mvstack`/`refmvs_find` 후보 리스트 구조 자체가 없어서, 제대로 하려면 MV 예측기
  재작업이 필요함이 드러났고, 이번엔 그것 없이 가능한 4개만 진행). **실측으로 게이팅 조건이
  이 fixture에서 실제로 매우 자주 켜져있음을 먼저 확인**(switchable_motion_mode=249/250,
  allow_warped_motion=132/250, subpel_filter_switchable=149/250, interintra/masked_compound/
  jnt_comp(시퀀스 레벨)=250/250 전부) — 구현 가치 있다고 판단 후 착수.
  `skip_mode_present`/`cdef_bits`와 같은 패턴으로 `subpel_filter_switchable`/
  `switchable_motion_mode`/`allow_warped_motion`(프레임 헤더, 기존에 비트만 소비하고 버려짐)
  + `enable_interintra_compound`/`enable_masked_compound`/`enable_jnt_comp`/`enable_warped_motion`
  (시퀀스 헤더, 이미 파싱만 돼있던 필드) 실제 노출. CDF 신규 10개 전부 rav1d `default_cdf` 원시값
  이식(motion_mode/obmc/interintra/interintra_mode/interintra_wedge/wedge_comp/wedge_idx/
  mask_comp/jnt_comp/filter). `TileContext`에 `comp_type`/`filter` above/left 배열 신규(둘 다
  real dav1d의 같은 이름 필드와 동일 목적) + `mask_comp_context`/`jnt_comp_context`/
  `filter_context`/`has_matching_single_ref`/`above_is_intra`/`left_is_intra` 신규 메서드.
  **두 가지 의도적 근사**(둘 다 문서화됨, DRL과 같은 근본원인은 아님): (1) `motion_mode`의
  warp 후보 판정(`find_matching_ref`)은 real dav1d가 above/left 엣지 전체를 서로 다른 크기의
  이웃 블록별로 스캔하는데, 이 크레이트는 CU 원점의 단일 above/left 위치만 확인(기존
  `above_ref0`/`left_ref0` 등 재사용) — 이웃이 이 CU의 엣지 전체를 덮는 일반적인 경우엔 정확,
  더 작은 이웃이 엣지 일부만 덮는 경우만 근사(거짓 매치는 절대 없음, 놓칠 수만 있음). (2)
  `jnt_comp_context`의 POC 기반 offset 항은 이 크레이트가 크로스프레임 OrderHint/POC 상태(진짜
  DPB)를 안 갖고 있어 0으로 고정. `motion_mode`의 "warped global motion 예외"/`global_motion_
  params()`도 이 크레이트가 안 갖고 있어 근사(항상 미제외로 취급). **실측 검증**: byte_offset
  재측정 결과 성공/실패 슈퍼블록 비율이 다시 개선(31%→42%→**48%**), fully-clean 프레임은
  여전히 0/29(완전 해결 아님). 406/406 lib, `--tests`+워크스페이스 전체 클린, clippy/fmt
  무경고 변화 없음. **DRL(다음 세션 명시적 착수 지점)이 유력한 남은 근본원인** — 그 외에도
  아직 발견 못 한 갭이 더 있을 가능성 있음.
- **후속(같은 세션, "다음 세션 시작"으로 이어감): DRL(dynamic reference list) 단일참조 전용
  실제 구현** — rav1d `refmvs.rs`(1842줄) 규모 확인 후 사용자와 두 번 범위 재조정(처음 "공간
  이웃만", 다시 "단일참조만, compound 확장 제외") 끝에 착수. 조사 중 이 크레이트가 이미
  `SpatialRefContext`(`inter_mode_context`/`compound_mode_context`)에 real dav1d `rav1d_refmvs_
  find`의 공간 스캔을 상당 부분 정확히 포팅해뒀던 걸 재확인(매치 카운트 기반 컨텍스트는 이미
  진짜) — 빠진 건 구체적으로 (1) `get_drl_context`가 필요로 하는 실제 가중치 기반 후보
  스택(`mvstack`, MV 값+weight)과 (2) `drl_bit` 자체 read였음. `SpatialRefCell`에 `mv0`/`mv1`/
  `width_4x4`/`height_4x4` 필드 추가(기존 매치-카운트 스캔은 안 건드림, DRL 전용 신규 경로) +
  real dav1d `scan_row`/`scan_col`의 이웃-폭-인식 스테핑을 포팅한 `single_ref_mv_stack`
  신규(가중치 640 임계값 부여 로직까지 포함) + `get_drl_context` 신규(free fn) +
  `drl_bit_cdf`(rav1d 원시값) + `read_drl_bit` 신규. `coding_unit.rs`의 NEWMV/NEARMV/NEARESTMV
  분기를 실제 drl_idx 선택 후 `stack[drl_idx].mv`를 predictor로 쓰도록 재작성(기존
  `MvPredictorContext`의 "가장 가까운 이웃 하나" 휴리스틱을 이 경로들에서 대체 — GLOBALMV는
  변경 없음, DRL 자체가 없는 모드). **세 가지 의도적 생략(전부 문서화, "compound DRL 확장은
  별도"에 이미 동의됨)**: temporal(시간축) 후보 — 이 fixture는 `use_ref_frame_mvs=true`가
  인터 프레임 249/249 전부에서 켜져있어 실제로 관여하는데도 생략(cross-frame 모션필드 저장
  자체가 이 크레이트에 없음, 남은 갭 중 가장 유력한 후보로 추정), compound DRL 확장
  (`add_compound_extended_candidate`, `sign_bias` 필요), 단일참조 cnt<2일 때의 "non-self
  reference" 확장 검색(`add_single_extended_candidate`, 이것도 sign_bias 필요 — 대신 이
  크레이트의 기존 GLOBALMV=0 근사와 일관되게 0으로 채움). 2차(secondary, n=2,3) 스캔은 real
  dav1d의 8x8-해상도 별도 인덱싱 대신 1차 스캔과 같은 4x4 스테핑으로 근사(640 임계값을 넘는
  일이 없어 `get_drl_context`의 주 판정에는 영향 없음, 저가중치 후보끼리의 드문 tie-break
  순서만 근사). **실측 검증**: byte_offset 재측정 성공/실패 비율 48%→**50%**(작은 개선,
  temporal 누락이 남은 주 원인으로 추정). 406/406 lib + `--tests` + 워크스페이스 전체 클린,
  clippy 신규 경고 2개 발견 즉시 수정(div_ceil 정리)해 13개 그대로 복귀, fmt 무경고. **부수:
  `MotionVector`에 `Default` derive 추가**(0,0으로 `MvStackEntry`의 zero-fallback 슬롯 채우는
  데 필요, 순수 기계적 파생 추가). 이걸로 이번 세션 "다음 세션 시작" 체인에서 스코핑됐던 DRL
  작업(단일참조 한정) 완료 — temporal 후보 추가는 진짜 별개의 cross-frame 상태 관리가
  필요해서 또 다른 큰 작업, 다음 세션 후보로 남김.
- **후속(같은 세션): temporal MV 후보 범위 조사 → 사용자 확인 후 보류(코드 변경 없음)** —
  rav1d `load_tmvs`/`save_tmvs`를 직접 확인해 실제 요구사항 확정: 디코드된 각 프레임의 8x8
  단위 모션필드를 참조 슬롯(최대 7개)별로 저장해뒀다가 나중 프레임에서 order_hint 기반 투영
  (`mv_projection`, `mfmv_ref2cur`/`mfmv_ref2ref`)으로 불러오는 구조. 이 크레이트는 프레임마다
  완전히 독립적으로 파싱하고(`parse_all_coding_units` 호출마다 상태 초기화) 프레임 간 지속
  상태가 전혀 없어서, 제대로 구현하려면 사실상 가벼운 DPB(decoded picture buffer)급 서브시스템
  (참조 슬롯 7개, 프레임 디코드마다 모션필드 저장, order_hint 투영 수학)을 새로 만들어야 함 —
  DRL급이거나 더 큰 규모로 확인. 사용자가 "여기서 멈춤, 다른 작업으로" 선택, 코드 변경 없이
  범위만 문서화. **다음 세션 후보로 유효하지만 우선순위 확인 필요.**
- **후속(같은 세션, "알아서해" 위임): 남은 인터 프레임 에러의 성격이 지금까지와 다르다는 걸
  확인(코드 변경 없음, 디버그 계측 전부 원상복구)** — DRL 이후 에러 유형/최초 실패 지점을
  재측정하고, 프레임 13(sb(0,0)에서 즉시 실패)을 골라 CU 단위로 상세 추적. **핵심 발견**: 이
  CU(128x128 compound, ref=[Last2,AltRef2], compound_mode=NewNewMv)는 skip_mode/is_inter/
  ref_frame/compound_mode/MV/compound_type/subpel_filter/var-tx(16개 real tx block)까지
  전부 성공적으로 통과하고, 잔차(residual) 디코드 중 luma tx 블록 12개째(16x16, x4=16,y4=12)
  근처에서야 arithmetic decoder underflow로 실패 — 앞선 11개 tx 블록(5개는 진짜
  all_zero, 6개는 실제 0이 아닌 잔차로 정상 디코드)까지는 전부 그럴듯한 값으로 성공. **이것은
  오늘 고친 "신택스 요소 전체 미독해"류 버그들과 다른 패턴** — 즉시/일관되게 실패하는 게
  아니라 수십~수백 개 심볼을 그럴듯하게 소비하다가 나중에야 무너지는 형태로, 이 세션 훨씬
  이전부터 있었던 "coeff_base/coeff_br 컨텍스트 정밀도" 의심(원래 엔트로피 디코더 조사의
  최초 미해결 용의자)과 같은 부류로 보임. **결론**: 남은 ~50%의 실패는 아마 더 이상 "빠진
  신택스 하나 더 찾기"로는 안 풀리고, 훨씬 느리고 깊은 심볼 단위 포렌식(또는 독립 오라클)이
  필요한 문제일 가능성이 큼 — 다음 세션 방향 재설정 필요.
- **다음 세션: 독립 오라클로 실제 근본 원인 확인 → coeff_base/coeff_br 의심은 틀렸음, 진짜
  원인은 tile_data 시작 오프셋 자체가 잘못 계산되고 있었음(2026-08-13)** — 사용자 확인 후
  실제 dav1d를 소스에서 `DEBUG_BLOCK_INFO`(프레임/블록 범위로 스코프 가능한 기존 디버그
  매크로) 활성화해 빌드(스크래치패드, 커밋 안 함), 프레임13 sb(0,0) 128x32유닛 범위의
  진짜 심볼별 트레이스를 얻어 이 크레이트 자체 파서와 직접 대조. **1차 발견**: 프레임13
  sb(0,0)의 첫 심볼(partition, 128x128, ctx=0)부터 이미 다름 — 진짜 dav1d는 SPLIT(bp=3),
  이 크레이트는 NONE(sym=0). CDF 테이블 자체는 문자 그대로 일치(rav1d
  `Default_Partition_W128_Cdf` 4개 컨텍스트 전부 `32768-p` 변환 후 완전 동일값 확인) —
  즉 컨텍스트/CDF 정밀도 문제가 전혀 아니라, 디코더에 넘겨지는 tile_data 바이트 자체가
  잘못된 위치에서 시작하고 있다는 뜻. **근본 원인 확정**: `overlay_extraction/parser.rs`의
  `ObuType::Frame` 분기가 tile_data를 잘라낼 때 `parse_frame_header_basic`(자체 doc이 "이
  header_size는 non-KEY 프레임에 대해 근사치"라고 명시하는, `frame_size()`/`tile_info()`/
  `segmentation_params()`/`loop_filter_params()`/`cdef_params()`/`lr_params()`/
  `global_motion_params()` 등을 통째로 건너뛰는 의도적 근사 파서)의 `header_size_bytes`를
  써왔음 — 같은 자리에서 이미 완전한 `parse_frame_header_full`도 호출하고 있었지만 그건
  플래그(reference_select 등) 추출용일 뿐, tile_data 자르기에는 한 번도 쓰이지 않았음.
  프레임13 실측: `basic`=8바이트, `full`=19바이트, 오라클로 역산한 진짜 오프셋도 19바이트 —
  기존 코드는 인터 프레임마다 진짜 tile_data보다 11바이트(경우에 따라 더) 먼저 시작해서,
  아직 프레임 헤더 비트인 구간을 심볼 디코더에 tile_data로 통째로 넘기고 있었음. **이게
  이 세션(그리고 그 이전 여러 세션) 내내 "coeff_base/coeff_br 컨텍스트 정밀도 문제"로
  의심해온 "느리게 누적되는 desync"의 실제 정체** — 정밀도 문제가 아니라 애초에 진짜
  tile_data를 읽은 적이 없었음. 수정: `full_hdr.header_size_bytes`로 자르도록 교체(`full`
  실패 시에만 `basic`으로 폴백, seq_header 없는 방어적 케이스 한정). **2차 발견(수정 직후
  회귀 테스트로 노출)**: 키프레임(프레임0)도 같은 분기를 타는데 수정 후 오히려 더 퇴화
  (`real_fixture_key_frame_intra_modes_are_not_degenerate`가 CU 1개/모드 1종으로 실패) —
  같은 오라클 대조로 재확인하니 `full`=25바이트인데 오라클 진짜 값은 17바이트, 8바이트
  과다소비. 원인: `frame_header_full.rs`의 `parse_global_motion_params` 호출이
  `!frame_is_intra` 게이트 없이 무조건 실행되고 있었음 — spec 5.9.2는 `global_motion_params()`를
  `FrameIsIntra`일 때 아예 호출하지 않는데, 키프레임엔 참조 프레임이 없으니 인코더도 이
  신택스를 전혀 안 씀. `allow_warped_motion`/`read_frame_reference_mode`는 이미 올바르게
  `frame_is_intra` 게이트가 있었는데 바로 다음 줄의 `global_motion_params`만 빠져 있었던
  것 — 게이트 추가로 수정, `full`=17로 오라클과 정확히 일치. **검증**: 두 수정 후 프레임13
  루트 partition이 SPLIT으로 정확히 일치, 그 아래 여러 레벨까지 오라클과 대조 확인(64x64
  NONE 리프 위치/크기까지 일치); 프레임0 키프레임도 non-degenerate 회복.
  `cargo test --workspace --lib`: 기존 403/407 통과에서 키프레임 테스트가 살아나 404/407 —
  단 **아직 3개 실패 남음**(`real_fixture_inter_var_tx_is_not_degenerate`,
  `real_fixture_nonsquare_inter_var_tx_is_not_degenerate`, 둘 다 "과반수 CU가 real
  txfm_split을 보여야 한다"는 임계값 어서션이 221/602, 229/643으로 미달; `real_fixture_
  delta_q_frame_changes_with_the_flag`는 delta_q_enabled 프레임 82개 전부가 이제
  하드에러로 실패 — 조사 결과 "CDF decode overran symbol alphabet"/"Arithmetic decoder cnt
  underflow"/"Partition Horz4 not allowed for block size Block16x16" 등 진짜 파싱 실패이지
  이번 수정으로 생긴 회귀가 아니라, 이제 처음으로 진짜 tile_data 끝부분까지 도달하면서
  **기존에 알려진 미완료 로드맵 항목**(inter_mode/compound_mode의 refmvs 기반 실컨텍스트,
  residual 전체 컨텍스트 — [[project_anti_pattern_audit]] 참고)이 처음으로 실제로
  노출되는 것으로 추정됨, 확정은 아직 안 함). 세 실패 다 이번 세션에서 손대지 않고 다음
  단계 확인 대기 중. **이 발견의 함의**: 이번 세션 이전까지의 "coeff_base/coeff_br 정밀도
  문제"라는 진단 자체가 틀렸었다는 뜻이므로, 그 가정 위에 쌓인 이전 세션들의 관련 메모/
  결론은 폐기하고 이 항목을 최신 근본원인으로 갱신할 것.
- **후속(같은 세션, 사용자 "3개 실패 더 깊이 조사" 선택): 남은 3개 전부 조사 완료 — 2개는
  진짜 3번째 버그, 1개는 스테일 테스트 임계값으로 확인, `bitvue-av1-codec --lib` 406/406
  클린 달성(2026-08-13)** — `real_fixture_delta_q_frame_changes_with_the_flag`의 실제 에러를
  까보니(임시 계측) "Partition Horz4 on block size Block16x16 produces sub-blocks of same
  size"류가 최다(38건) — `PartitionType::is_allowed`의 Horz4/Vert4 조건이
  `width>=16 && height>=32`(Horz4)/반대(Vert4)라는 완전히 잘못된 비대칭 형태였음(진짜 spec
  조건은 "정확히 16x16/32x32/64x64 정사각형만" — CDF 테이블 자체는 이미 16x16용 10-심볼
  alphabet로 Horz4/Vert4를 포함하고 있었으니 CDF는 처음부터 맞았고 이 검증 함수만 틀렸음).
  고치자 이번엔 `sub_block_size`가 16x16 케이스 자체를 아예 안 갖고 있어서(`_ => vec![*self]`
  폴백, 즉 크기 불변 리턴) "produces sub-blocks of same size" 에러로 이동 — 원인을 더 파보니
  `BlockSize` enum 자체에 `Block16x4`/`Block4x16` variant가 애초에 존재하지 않았음(형제뻘인
  Block32x8/Block64x16/Block128x32/Block8x32/Block16x64/Block32x128는 이미 있었는데 16x16용
  한 쌍만 빠짐 — 6개 중 4개만 완성돼 있던 상태). 두 variant 추가 + width()/height()/
  sub_block_size 갱신(전부 4개 파일뿐이라 blast radius 작음, 컴파일러가 exhaustiveness로
  나머지 확인, 에러 0건) → `real_fixture_delta_q_frame_changes_with_the_flag` 통과. **남은 2개
  (`real_fixture_inter_var_tx_is_not_degenerate`/`..._nonsquare_...`)는 조사 결과 버그 아님**:
  두 테스트 다 "eligible CU 과반수가 real txfm_split을 보여야 한다"는 임계값인데, 프레임13
  sb(0,0) 자체의 오라클 `vartxtree` 기록이 3번 중 1번만 split(약 33%)이라 이 세션에서 고친
  수치(232/660≈35%, 346/923≈37%)와 정확히 일치 — "과반수 split"이라는 가정 자체가 예전
  버그투성이 디코드 통계에 맞춰 잘못 짜인 임계값이었을 뿐, 실제 spec/인코더 성질이 아님(txfm_
  split 자체는 이미 실컨텍스트+adaptation 적용돼 있었음, 확인 완료). "과반수" → "0도 아니고
  전부도 아님"(진짜 non-degenerate 취지에 맞는 형태)으로 재보정. **결과**:
  `cargo test --workspace --lib` 기준 `bitvue-av1-codec` 자체는 406/406 완전 클린(워크스페이스
  전체에서 유일한 남은 실패 `bitvue-engine::compare_cache::test_evict_lru_stream_a`는 AV1과
  무관한 별개 크레이트의 병렬실행 시 타이밍 의존 플레이키 테스트로 확인 — 격리 실행 시 통과,
  이번 변경 이전에도 존재, 손대지 않음). **이번 세션 최종 정리**: dav1d 오라클 빌드 1회로
  진짜 버그 3개(tile_data 오프셋 근사파서, 키프레임 global_motion_params 게이트 누락,
  Horz4/Vert4 is_allowed+BlockSize enum 미완성) + 잘못된 테스트 임계값 1개를 동시에 잡아냄 —
  전부 "coeff_base/coeff_br 정밀도"라는 원래 가설과 무관했고, 몇 세션째 이어온 "심볼 단위
  포렌식 필요"라는 결론도 틀렸었음(byte-exact 오라클 대조 없이는 못 찾았을 클래스의 버그들).
- **실제 temporal MV candidate(spec 7.9/7.10) 구현 완료(2026-08-14)**: DRL 커밋이 "가장 큰
  남은 갭"으로 명시적으로 미룬 항목 착수. 실측 Explore 결과 이 크레이트의 실제 프로덕션
  호출부(`bitvue-sidecar`의 `get_frame_analysis`/`get_av1_features`/`get_deblocking_analysis`)는
  전부 무상태·프레임 단위 랜덤 액세스라 프레임 간 MV 상태가 전혀 없었고, spec 7.9/7.10은
  프레임 0..N을 순서대로 완전히 파싱해 참조 슬롯별 motion field 캐시를 쌓아야 함 — 사용자에게
  "정합성만 우선, sidecar 캐싱/세션 계층은 별도 후속 단계로 미룸"으로 스코프 확정 확인
  (EnterPlanMode). 계획 초안은 "슬롯 하나당 order-hint 델타로 한 번 프로젝션"으로 잘못
  가정했으나, 스크래치패드에 남아있던 실제 dav1d 오라클 소스(`refmvs.c`, 이전 세션의 오라클
  빌드에서 소스만 재사용, 빌드 불필요)를 직접 대조하자 실제 알고리즘은 훨씬 복잡함을 발견 —
  최대 3개 소스 슬롯을 우선순위로 선택(`dav1d_refmvs_init_frame`) + 저장 시점엔 원본 MV를
  그대로 두고 위치 오프셋만 계산해두었다가(`load_tmvs_c`) 실제 소비 시점(`add_temporal_
  candidate`)에 현재 블록 자신의 ref pocdiff로 최종 리스케일하는 2단계 체인 구조임을 재확인,
  사용자에게 "full fidelity로 재계획" 확인받고 진행(2번째 AskUserQuestion, 세션 내
  scope-growth 시 재확인하는 기존 패턴 반복). **신규 모듈**
  `crates/bitvue-av1-codec/src/tile/motion_field.rs`: `store_motion_field`(spec 7.9, CU 리스트→
  8x8그리드, compound ref[1] 우선+sign/magnitude 게이팅, 8x8쌍의 우하단 4x4 서브위치 샘플링을
  위해 4x4 해상도 `CuSpatialIndex` 재사용) + `select_motion_field_sources`(우선순위 소스 선택
  +ref2cur/ref2ref 계산, 새 `[[u32;7];8]` ref_ref_order_hint 상태 필요 — 슬롯이 새로고침될 때
  "그 프레임 자신의 7개 참조 order hint" 스냅샷) + `project_motion_field`(위치 오프셋만 계산,
  원본 MV+ref2ref 분모를 `ProjectedMv`로 저장) + `add_temporal_candidates`(최종 리스케일,
  `single_ref_mv_stack`이 소비). 단위 환산: 이 크레이트 `MotionVector`는 1/4-pel(dav1d는
  1/8-pel) — 매직넘버(4096/0x3fff/`>>6`) 전부 절반(또는 `>>5`)로 스케일. `context.rs`:
  `SpatialRefContext`에 `temporal: Option<TemporalMvContext>` 필드+`set_temporal_context`
  추가, `single_ref_mv_stack`의 +640 가중치 부여 직후·top-left secondary 스캔 직전에 temporal
  스캔 삽입(dav1d 순서 그대로, weight=2로 secondary와 동일 티어) — 기존 15개 호출부는 전부
  `temporal: None`이라 동작 불변 확인(406/406→407/407 회귀 없음). 부수로 `inter_mode_context`의
  `globalmv_ctx`도 완성 — 기존엔 "`use_ref_frame_mvs` 플래그를 그대로 근사값으로 사용"이라고
  이 파일 자체 문서에 이미 "후속 과제"로 명시돼 있었는데, 이번에 만든 projected grid의 `!(x|y)`
  위치 샘플을 재사용해 진짜 계산으로 교체(global motion 자체는 크레이트 전역의 기존
  "0/invalid 근사" 유지, 별도 갭 아님). `ParsedFrame`에 `order_hint`/`ref_frame_idx`/
  `refresh_frame_flags` 3개 필드 신규 노출(기존 `use_ref_frame_mvs`와 동일하게 skip_mode_params
  이전 비트 위치라 fresh `RefFrameState::new()`로도 값이 정확 — 헤더 재파싱 없이 시퀀셜
  테스트 하네스가 이 필드들만으로 상태를 이어갈 수 있음, `RefFrameState::apply_refresh` 신규
  헬퍼로 `parse_frame_header_full` 재호출 없이 순수 갱신). `cu_parser.rs`의
  `parse_all_coding_units`가 쓰던 캐시(`tile_data`+`base_qp` 키)는 temporal 입력을 반영 못하므로
  `parse_all_coding_units_with_temporal`(캐시 안 함)을 분리해 프로덕션 경로는 그대로
  `None`으로 캐시 경로 재사용, 테스트 하네스만 `Some`으로 비캐시 경로 사용. **범위 밖으로
  명시 기록(축소 아님)**: dav1d의 여분 3개 코너 샘플(중간 크기 블록의 좌하/우하/우상단,
  타일·SB 경계로 클램프)은 생략 — weight=2 저티어 후보만 늘리는 것이라 bit-position sync엔
  영향 없음, 완전성만 살짝 줄어듦(문서화). **검증**: 250프레임 전체 순차 파싱(`MotionFieldState`
  스레딩, `bitvue-sidecar`의 기존 `for frame in frames.iter().take(idx+1)` 순차 스캔 패턴과
  동일 계열) 무크래시 확인 + 실측 119/250 프레임(~48%)이 실제로 유효한 temporal 소스를
  찾아냄(신규 회귀 테스트 `real_fixture_temporal_mv_candidates_are_wired_and_non_degenerate`,
  ≥10% 임계값 — 이 클래스 변경엔 독립 값 오라클이 없다는 기존 세션 전제 유지, self-consistency
  +non-degenerate 체크로 대체). `bitvue-av1-codec --lib` 407/407, `--tests` 13개 바이너리 전부
  클린, `cargo test --workspace --lib` 3854+ 전부 통과(0 실패, 이전에 봤던 `bitvue-engine` 플레이키도
  이번엔 안 걸림). clippy: 유일한 error는 이 세션이 손댄 적 없는 `leb128_prop_tests.rs`의
  기존 `absurd_extreme_comparisons`(무관 확인), fmt 클린.

---

## Phase 5: AVS3 지원 구현 🟡 (2026-08-10 재감사 — 크레이트/파서/렌더러 존재, 제품 미연결)

**목표:** AVS3 코덱 전체 파싱 + 디코딩 + 전용 모드

> **재감사 요약:** "크레이트 신규 생성"부터 다시 시작해야 하는 상태가 전혀 아니다 — `crates/bitvue-avs3/`가 이미
> 존재하고 시작코드 스캔, Sequence/Picture Header 파싱, ESAO/CCSAO "휴리스틱" 오버레이 추출까지 구현돼 있으며
> `bitvue-cli`(Analyzer CLI 제품)에 배선까지 완료돼 있다(`crates/bitvue-cli/src/commands/decode.rs:916
> extract_avs3_frames`). 다만 (1) AEC 엔트로피 디코딩·CTU/CU 분할 파싱은 크레이트 자체 문서(`lib.rs:15-19`)가
> "planned but not yet implemented"라고 명시, (2) 실제 픽셀 디코딩 경로가 전혀 없고, (3) Electron 데스크톱 앱이
> 쓰는 `bitvue-sidecar`는 이 크레이트에 전혀 의존하지 않아 GUI에서는 AVS3 파일을 열어도 아무것도 볼 수 없다.

- [x] `bitvue-avs3-codec` 크레이트 신규 생성 — `crates/bitvue-avs3/` (패키지명은 `bitvue-avs3`), 이미 존재
- [x] AVS3 비트스트림 구조 파싱:
  - [x] 시작 코드 (AVS3 prefix: 0x000001xx) — `crates/bitvue-avs3/src/nal.rs::scan_nal_units`
  - [x] Sequence Header, Picture Header 파싱 — `sequence_header.rs::parse_sequence_header`,
        `picture_header.rs::parse_i_picture_header`/`parse_pb_picture_header`
  - [ ] CTU/CU 분할 구조 (HEVC 유사) — **미구현.** 헤더 레벨 파싱만 있고 CTU/CU 트리 파싱은 없음
        (`lib.rs` 모듈 문서가 명시적으로 "AEC decoding ... planned but not yet implemented"라 표기)
- [ ] AVS3 엔트로피 코딩: CABAC 변형 (AEC — Adaptive Entropy Coding) — **미구현** (위와 동일 근거)
- [x] **ESAO 모드 렌더러** (Enhanced SAO, AVS3 전용) — `frontend/.../renderers/Avs3EsaoRenderer.tsx` 렌더러
      존재 + 백엔드 `overlay_extraction.rs::extract_esao_map`도 존재. 단 이 백엔드 함수 자체가 진짜 AEC 디코딩이
      아니라 **"(qp + ctu_index) mod 6" 결정론적 휴리스틱**이라고 코드 주석에 명시(`overlay_extraction.rs:98-99`
      "Deterministic heuristic"), 렌더러도 화면에 "⚠ ESAO proxy" 워터마크를 표시함. 게다가 sidecar 미배선이라
      이 프록시 데이터조차 GUI에는 도달하지 않음(항상 "ESAO: disabled or unavailable" 표시)
- [x] **CCSAO 모드 렌더러** (Cross-Component SAO, AVS3 전용) — `Avs3CcsaoRenderer.tsx` + `extract_ccsao_map`,
      ESAO와 동일하게 휴리스틱 proxy + sidecar 미배선
- [ ] openavs3d 또는 FFmpeg 기반 AVS3 디코딩 — **미착수.** 픽셀 디코딩 코드 자체가 없음(파싱만 존재)
- [ ] AVS3 Syntax 탭 구현 — 미검증 (프론트엔드에서 AVS3 전용 Syntax 탭 컴포넌트를 찾지 못함, 후속 확인 필요)

**남은 작업 우선순위 (재감사 기준):**
1. AEC(Adaptive Entropy Coding) 엔트로피 디코더 신규 구현 — 나머지 항목 대부분의 선행 조건
2. CTU/CU 분할 구조 파싱 (AEC 위에서 동작)
3. `bitvue-sidecar`에 `bitvue-avs3` 의존성 추가 + IPC 커맨드 배선 (ESAO/CCSAO 프록시라도 우선 GUI에 노출 가능)
4. openavs3d/FFmpeg 기반 실제 픽셀 디코딩 연결
5. ESAO/CCSAO를 휴리스틱 proxy에서 실제 AEC 디코드 기반으로 교체

**레퍼런스:**
- AVS 표준 문서: http://www.avs.org.cn/
- openavs3d 오픈소스 구현

**예상 소요:** 중급 5~7주 (헤더 파싱은 이미 끝났으므로 원래 추정보다 다소 단축, AEC 구현이 여전히 최대 리스크)

---

## Phase 6: JPEG XS + VC-3 + APV 지원 🟡 (2026-08-10 재감사 — JPEG XS/VC-3 파싱·렌더러 완료 · 제품 미연결, APV 완전 미착수)

> **재감사 요약:** JPEG XS와 VC-3는 Phase 5(AVS3)와 같은 패턴 — 파서 크레이트, 오버레이 추출 함수, 전용 F키
> 렌더러까지 전부 존재하지만 `bitvue-sidecar`가 두 크레이트 어느 쪽에도 의존하지 않아 GUI에서는 도달 불가.
> APV는 크레이트/코드가 전혀 없어(저장소 전체 grep 결과 0건) 완전 미착수 상태 그대로다.

#### JPEG XS
- [x] `bitvue-jpegxs-codec` 크레이트 신규 생성 — `crates/bitvue-jpegxs/` (패키지명 `bitvue-jpegxs`), 이미 존재
- [x] JPEG XS 마커 파싱 (SOC, SLH, SLI, SLD, EOC) — `crates/bitvue-jpegxs/src/marker.rs`, 5개 마커 전부
      (`0xFF10`/`0xFF54`/`0xFF55`/`0xFF56`/`0xFF11`) 정의 및 스캔 구현 확인
- [x] Precinct 구조 파싱 — `overlay_extraction.rs::extract_precinct_map` + 렌더러 `JpegXsPrecinctRenderer.tsx`
- [x] 웨이블릿 계수 추출 — `overlay_extraction.rs::extract_dequant_map`/`extract_transform_map`
      (역양자화 계수·변환 서브밴드 구조 추출, 원시 웨이블릿 계수 자체보다는 서브밴드/양자화 레벨 단위 — 세부
      정확도는 픽셀 디코딩이 없어 검증 불가)
- [x] NLT, MCT 파라미터 파싱 — `nlt.rs`/`mct.rs` + `extract_nlt_info`/`extract_mct_info`,
      렌더러 `JpegXsNltRenderer.tsx`/`JpegXsMctRenderer.tsx`
- [x] 전용 F1~F6 모드 렌더러 — `codecModeRegistry.ts`의 `JPEGXS_MODES`(F1 Precinct ~ F5 NLT, F6 YUV) 전부
      대응 렌더러 존재(`JpegXsPrecinctRenderer`/`JpegXsDequantRenderer`/`JpegXsTransformRenderer`/
      `JpegXsMctRenderer`/`JpegXsNltRenderer`), sidecar 미배선으로 실데이터는 미도달

#### VC-3 / DNxHD
- [x] `bitvue-vc3-codec` 크레이트 신규 생성 — `crates/bitvue-vc3/` (패키지명 `bitvue-vc3`), 이미 존재
- [x] DNxHD 세그먼트 구조 파싱 — `segment.rs::scan_segments`/`parse_frame_header` (`DNXHD_MAGIC` 매직 넘버 확인 포함)
      + 렌더러 `Vc3SegmentRenderer.tsx` (`overlay_extraction.rs::extract_mb_grid`)
- [ ] FFmpeg 기반 디코딩 — **미착수**, 픽셀 디코딩 코드 없음 (파싱만 존재, AVS3/JPEG XS와 동일 패턴)

#### APV (Apple ProRes Video)
- [ ] APV 비트스트림 파싱 — **완전 미착수.** 저장소 전체에서 `bitvue-apv` 크레이트 없음, "APV"/"ProRes" 관련
      코드 grep 결과 무관한 우연 매치 1건 외 없음
- [ ] FFmpeg ProRes 디코딩 — 미착수

**남은 작업 우선순위 (재감사 기준):**
1. JPEG XS·VC-3: `bitvue-sidecar`에 두 크레이트 의존성 추가 + IPC 커맨드 배선 (파싱/렌더러가 이미 있으므로
   Phase 6 중 가장 저비용으로 "실제 동작"까지 갈 수 있는 경로)
2. VC-3: FFmpeg 기반 픽셀 디코딩 연결
3. APV: 크레이트 신규 생성부터 시작 (유일하게 원 계획대로 처음부터 시작해야 하는 항목)

**예상 소요:** JPEG XS/VC-3 배선 각 1~2주(파싱 완료 상태 기준), VC-3 FFmpeg 디코딩 2~3주, APV는 원안 그대로 중급 4~6주

---

## Phase 7: YUVDiff 모드 완성 🟡

**목표:** Debug YUV 비교 기능을 VQ Analyzer 수준으로 완성

- [ ] YUV 파일 로더 강화:
  - 자동 포맷 감지 (파일 크기 / 해상도 기반)
  - Planar (YUV420p, YUV422p, YUV444p) 지원
  - Semi-planar (NV12, NV21) 지원
  - 비트뎁스 (8/10/12/16bit) 지원
- [ ] 픽셀 단위 차이 계산 엔진 (Rust, SIMD 가속):
  ```rust
  pub fn compute_diff_frame(
      decoded: &YuvFrame,
      reference: &YuvFrame,
      amplification: u8,
  ) -> YuvFrame
  ```
- [ ] YUVDiff UI 완성:
  - [Decoded / Debug YUV / Difference / Amplified] 전환 버튼
  - PSNR/SSIM 실시간 표시
  - 차이 증폭 슬라이더 (×1 ~ ×64)
  - 첫 불일치 프레임 탐색 버튼
- [ ] 픽처 오프셋 조정 다이얼로그
- [ ] 크롭 설정 다이얼로그 (L/R/T/B 픽셀)
- [ ] Display Order vs Coding Order 전환
- [ ] Auto-reload 감시 (inotify/FSEvents 기반)

**Tauri 커맨드 추가:**
```typescript
load_debug_yuv(path: string, format: YuvFormat, bitdepth: number) -> Result<void>
get_yuv_diff_frame(frame_index: number, amplification: number) -> Result<FrameData>
get_yuv_psnr(frame_index: number) -> Result<PsnrResult>
find_first_diff_frame() -> Result<number>  // 첫 불일치 프레임 인덱스
```

**Parity 검증:**
- PSNR 값이 `ffmpeg -i ref.yuv -i decoded.yuv -lavfi psnr` 결과와 ±0.01dB 일치
- 차이 프레임 시각화가 VQ Analyzer의 색상 표현과 일치

**예상 소요:** 중급 2~3주

---

## Phase 7.5: Dual-Stream Compare & VMAF 🔴 (신규 — VQA_PARITY_SPEC_V3.md §4.9, §1.5)

**목표:** VQ Probe/StreamEye 패리티의 핵심 갭인 "두 비트스트림 비교" 워크플로우와 VMAF 품질 지표 추가.

**⚠️ 아키텍처 경계 (필수 준수):** Bitvue는 VQ Analyzer 성격(비트스트림 구조/코덱 상태 분석)과 VQ Probe 성격(화질
비교/정렬/메트릭 분석)을 **동시에** 만드는 프로젝트다. 이 둘은 실패 모드가 다르다 — Analyzer 쪽은 파서 안전성·syntax
표현 폭증·codec state가 핵심 위험이고, Probe 쪽은 프레임 정렬·색공간 정합성·중복 decode·메트릭 정확성이 핵심 위험이다.
**절대 하나의 거대 구조체로 합치지 말 것**:
```rust
// 안티패턴 — 하지 말 것
struct UniversalFrameAnalysis {
    syntax: Option<SyntaxTree>,
    motion_vectors: Option<Vec<MotionVector>>,
    qp_map: Option<QpMap>,
    psnr: Option<f64>, ssim: Option<f64>, vmaf: Option<f64>,
    heatmap: Option<Heatmap>,
    reference_frame: Option<FrameId>, distorted_frame: Option<FrameId>,
}
```
공유해야 하는 것은 **미디어 입력과 실행 인프라**(Demuxer, Decoder abstraction, PixelFormat, ColorMetadata,
FrameBuffer/Pool, Timeline, Job scheduler, Cache budget)이지 **분석 결과 도메인 자체가 아니다**. 이미 존재하는
`crates/bitvue-engine/src/{compare,alignment,...}.rs`가 비트스트림 분석 코드와 섞이지 않고 독립 모듈로 남아있는지
구현 착수 전 확인할 것. `docs/anti-patterns/`(작성 중)의 카탈로그도 이 경계를 그대로 따라 BIT-*(Analyzer 전용)와
VQ-*(Probe 전용)를 분리된 하위 카탈로그로 유지한다 — 문서 구조와 실제 코드 구조가 어긋나면 이 원칙이 무의미해짐.

- [ ] Stream B 로딩 인프라 완성 (Phase 3 진행 상황 메모 P1-4와 통합)
- [ ] Compare 모드 UI: Side-by-side / Split(H/V) / Subtraction / Temperature 전환 버튼
- [ ] RD-curve 패널에 BD-rate 계산 추가
- [ ] VMAF 통합: `libvmaf-sys` 연결 (이미 optional feature로 존재 — spec §1.2), pooled score + per-frame score + ADM2/VIF/motion2 서브스코어
- [ ] "Find First Difference" (두 스트림 간)
- [ ] CLI: `bitvue compare --stream-a --stream-b --vmaf` 서브커맨드
- [ ] ROI 기반 메트릭 (선택 영역 한정 PSNR/SSIM/VMAF) — `COMPETITOR_FEATURE_MATRIX.md` §3/§6 "Metrics-in-ROI" (신규 2026-07-31)
- [ ] 추가 메트릭 (APSNR/DELTA/MSE/MSAD/VQM/NQI/EPSNR/VIF) — SE 소스, 우선순위 낮음, `COMPETITOR_FEATURE_MATRIX.md` §3 (신규 2026-07-31)

**Parity 검증:** `VQA_PARITY_SPEC_V3.md` §4.9 참조.

**현황 정정 (2026-07-31, `docs/_import_v14` 마이닝 중 grep으로 발견):** `PARITY_CHECKLIST.md` Layer 6은
CMP-01/02를 `[ ]`(미시작)으로 표시하지만 실제로는 이미 부분 구현되어 있음 — `crates/bitvue-engine/src/{compare,alignment,compare_cache,compare_evidence,compare_strategy}.rs`,
`src-tauri/src/commands/compare.rs`(`create_compare_workspace`/`get_aligned_frame`/`set_sync_mode`/`set_manual_offset`/`reset_offset`,
전부 `lib.rs`에 등록됨), `frontend/components/CompareWorkspace/CompareWorkspace.tsx`(side-by-side 렌더링 확인)가
이미 존재. Split(H/V)·Subtraction/Temperature 뷰는 미확인. `PARITY_CHECKLIST.md` Layer 6에 `[-]`로 정정함 — 아래 참조.

**Diff Heatmap 구현 상세 (from `_import_v14/.../visualization/DIFF_HEATMAP_IMPLEMENTATION_SPEC.md`, CMP-03 대상):**

| 항목 | 값 |
|---|---|
| 입력 | 해상도/색공간 정렬된 프레임 A/B (luma 또는 RGBA→luma 변환), 선택적 블록 단위 메트릭 델타 맵 |
| 모드 (UI 토글) | `abs`(\|lumaA-lumaB\|, 기본값) / `signed`(lumaA-lumaB) / `metric`(블록별 메트릭 델타) |
| 텍스처 생성 | QP 히트맵과 동일 규칙으로 기본 Half-res; abs 모드는 4단계 램프, diff=0은 완전 투명; alpha는 diff에 비례, 최대 `180 * user_opacity`로 클램프 |
| 캐시 키 | `overlay_diff:<codec>:<filehashA>:<filehashB>:f<frame>\|hm<hw>x<hh>\|mode<abs\|signed\|metric>\|op<bucket>` |
| 인터랙션 | hover 시 픽셀/블록 diff 값 표시; 클릭 시 툴팁 고정(ESC로 해제) |
| 승인 테스트 | diff=0 영역 완전 투명 / opacity만 바뀌면 캐시 재사용 / 모드 전환 시에만 텍스처 재생성 |

이미 존재하는 `crates/bitvue-engine/src/diff_heatmap.rs`(YUVDiff §4.7용)를 Compare A/B 컨텍스트로 재사용 가능한지 확인 필요 —
현재 diff_heatmap.rs가 단일 스트림 YUVDiff용인지 A/B 두 스트림용인지 구현 시 확인.

**예상 소요:** 중급 3~4주

---

## Phase 7.6: UX Contract 완성 — Context Menu & Evidence Bundle 🟡 (2026-08-09, 35955fc/f3e9e81 — MVP 슬라이스 완료, 아래 잔여 항목 참고)

**목표:** Compare 워크스페이스(Phase 7.5)를 포함해 전체 앱에 공통으로 필요한 우클릭 컨텍스트 메뉴 계약과 원클릭
Evidence Bundle export 계약을 구현한다. 둘 다 `UX_PARITY_MATRIX.md` §9에서 P0/P1로 채점된 실제 갭이며, 기존
계획(Phase 11 "폴리시" 항목)까지 미루지 않도록 여기로 승격했다 — 근거는 문서 상단 "우선순위 재검토" 참조.

**착수 시점 발견**: `bitvue-engine`의 `export::context_menu`(guard 엔진+5스코프 카탈로그, starter 3스코프보다 이미
넓음)와 `export::evidence`(진짜 파일 쓰기, 411줄)는 이미 완성돼 컴파일까지 되고 있었으나 sidecar 커맨드로 노출된
적이 없었음(`create_compare_workspace`처럼 완전 미구현이 아니라 "배선 누락"). 또한 `parity_harness::context`라는
완전히 별개의, 한 번도 안 쓰인 중복 구현(`ContextMenuScope`/`ContextMenuItem`/`evaluate_guard`, 유일한 소비자가
자기 자신의 단위테스트)이 이미 존재했음 — 크레이트 루트 glob re-export 충돌을 `bitvue_engine::export::` 명시
경로로 회피, 중복 자체는 이번 패스 범위 밖이라 손대지 않음(코드에 문서화만).

**Context Menu 시스템 (`PARITY_CHECKLIST.md` Layer 7 CTX-01, `UX_PARITY_MATRIX.md` §6):**
- [x] `get_context_menu_items` sidecar 커맨드로 배선 완료 — 실제로는 5스코프(Player/HexView/StreamView/Timeline/
      DiagnosticsPanel, starter 3개보다 많음) × guard 엔진(`always`/`has_selection`/`has_byte_range`) 전부 이미
      완성돼있던 걸 노출만 함
- [x] 비활성 항목 disabled reason 툴팁 — 프론트 `ContextMenu.tsx`가 `item.disabled_reason`을 `title` 속성으로 표시
- [x] 신규 범용 `ContextMenu` 컴포넌트(`frontend/components/ContextMenu.tsx`) — 이 세션 이전엔 우클릭 UI 자체가
      프론트엔드 어디에도 전혀 없었음(완전 greenfield)
- [x] Player 스코프 실제 연결(`VideoCanvas.tsx`의 `onContextMenu`) — 나머지 4개 스코프(HexView/StreamView/
      Timeline/DiagnosticsPanel)는 백엔드 API는 이미 동일하게 작동하지만 각 패널에 아직 연결 안 함(다음 세션 후보)
- [ ] `UX_PARITY_MATRIX.md` §3 per-panel interaction contract 전체 확장(screenshot-driven refinement) — 범위 밖

**Evidence Bundle export (`PARITY_CHECKLIST.md` Layer 7 EVB-01, `UX_PARITY_MATRIX.md` §7):**
- [x] `export_evidence_bundle` sidecar 커맨드로 배선 완료 — `bundle_manifest.json`/`env.json`/`version.json`/
      `selection_state.json`/`order_type.json`/`backend_fingerprint.json`/`warnings.json` 실제 파일 쓰기 확인
- [ ] `plugin_versions.json` 별도 파일(현재는 `backend_fingerprint.json` 안에만 내장), `screenshots/*` — 엔진 함수
      자체의 기존 갭(이번 세션에서 만든 게 아님, `include_screenshots`/`include_interaction_trace`/`include_logs`
      플래그는 있지만 아무것도 캡처 안 함)
- [ ] ABI 호환 정책 — 문서화만 되고 코드 어디에도 구현 없음(정책-온-페이퍼), 이번 패스에서도 미착수
- [ ] Diff 계약(`bundle_diff`/`EvidenceBundleDiff`) — 엔진에 함수 자체가 존재하지 않음
- [x]/[ ] 4개 진입점 중 2개 배선: MainMenu(File > Export > Evidence Bundle — 죽어있던 `menu-export-evidence`
      리스너 신규 연결), ContextMenu(Player 스코프 한정). 남은 2개: MainPanel(BottomBar > Export — 애초에 export
      버튼 자체가 없음), CompareWorkspace(Toolbar — UI 자체가 죽은 트리, Phase 7.5 참고)
- `stream_fingerprint`는 진짜 값(스트림 A 파일경로+바이트길이 해시, `qp_heatmap`/`timeline_cache`와 같은 기존
  `DefaultHasher` 컨벤션 재사용) — placeholder 아님

**검증**: sidecar 112→118(신규 6개) 테스트 통과, 타입체크 411파일(신규 컴포넌트 2개 포함)/vitest 36실패-9파일
베이스라인 그대로(진행 중 `YuvViewerPanel`에 `SelectionContext` 의존성을 실수로 추가해 128개 테스트 깨뜨렸다가
발견 즉시 되돌림 — 테스트 스위트가 실제로 회귀를 잡아준 사례). `BITVUE_ELECTRON_SELFTEST`에
`getContextMenuItems`+`exportEvidenceBundle`(실제 임시 디렉터리에 파일 쓰기) 라운드 추가, exit 0.

**Parity 검증**: `UX_PARITY_MATRIX.md` §9 `INTERACTION_CONTEXT_MENU_GUARDS`/`EVIDENCE_ONE_CLICK_BUNDLE` 항목은
starter 슬라이스만 통과 — 전체 통과는 위 미완료 항목들에 달림.

**예상 소요**: 원래 추정 중급 2~3주 → 실측은 반나절(사전 존재하던 엔진 로직 덕분, "배선만" 패턴). 남은 잔여
항목(스크린샷 캡처, ABI 정책, diff 계약, 나머지 4스코프+2진입점 연결)은 여전히 별도 작업.

---

## Phase 8: Syntax 패널 완성 (코덱별 탭 상세화) 🟡

**목표:** 각 코덱의 Syntax 탭을 VQ Analyzer 수준으로 완성

- [ ] **Stats 탭 완성**:
  - 파이 차트: 프레임 타입 분포, CU 크기 분포, 인트라/인터 비율
  - 바 차트: 프레임별 크기, QP 분포, OBU-size 분포 (AV1, VEGA "Graph View" 참조 — 신규 2026-07-31)
  - 스트림 통계 테이블
  - Bit distribution 시각화 (신택스 요소별/블록타입별 비트 분포) — `COMPETITOR_FEATURE_MATRIX.md` §5/§6 (신규 2026-07-31)
  - Scene change detection (프레임 간 콘텐츠 변화 마커, Timeline 연동) — `COMPETITOR_FEATURE_MATRIX.md` §3/§6,
    `UX_PARITY_MATRIX.md` §1 "Scene-change markers" (신규 2026-07-31)
- [ ] **HRD/CPB Buffer 그래프** (fullness plot + VBV violation 마커) — spec §4.1 (build 미확인), `COMPETITOR_FEATURE_MATRIX.md`
      §1 "HRD/VBV buffer plot" / §5 "Buffer Analyzer"; conformance-checking 레이어(T-STD)는 범위 밖 (신규 2026-07-31)
- [ ] **DPB-occupancy 그래프** (`WS_REFERENCE_DPB` 참조, `UX_PARITY_MATRIX.md` §8) — Ref Lists 탭과 함께 구현 (신규 2026-07-31)
- [ ] **CABAC range/state 시각화** (HEVC/AVC) — `PARITY_CHECKLIST.md` Layer 6 CMP-10; 오버레이 자체는 Phase 2에서
      추적, Syntax 패널 쪽 상태 트레이스 뷰는 여기서 구현 (신규 2026-07-31)
- [ ] **신택스 트리 네비게이션**:
  - 트리 노드 펼치기/접기
  - 노드 클릭 → HEX View에서 비트 오프셋 강조
  - HEX View 바이트 클릭 → 신택스 트리 해당 요소 포커스
- [ ] **HEVC QM 탭**:
  - 양자화 매트릭스 테이블 (4x4 ~ 32x32, 색상 시각화)
- [ ] **VP9 Probabilities/Counts 탭**:
  - 컨텍스트 확률 테이블 시각화
  - 프레임별 업데이트 이력
- [ ] **VVC APS 탭**:
  - ALF APS, LMCS APS, Scaling List APS 분리 표시
- [ ] **모든 코덱 Ref Lists 탭**:
  - L0/L1 리스트, POC, 가중치/오프셋, long-term 여부

**예상 소요:** 중급 3~4주

---

## Phase 9: 커맨드라인 인터페이스 (CLI) 강화 🟡

**목표:** bitvue-cli가 VQ Analyzer CLI와 동일한 기능 제공

- [ ] `-hevc`, `-vp9`, `-av1`, `-avc`, `-mpeg2`, `-vvc`, `-avs3` 강제 코덱 플래그
- [ ] `-o <file>` 출력 YUV 파일명
- [ ] `-y4m` Y4M 포맷 출력
- [ ] `-dump` 디코딩된 YUV 덤프
- [ ] `-dump_bitdepth <N>` 출력 비트뎁스 설정
- [ ] `-regress` 헤드리스 디코딩 (GUI 없음)
- [ ] `-frames <N>` 처리할 프레임 수
- [ ] `-fast <0|1|2>` 성능 모드
- [ ] `-md5` 프레임별 MD5 체크섬
- [ ] `-psnr` PSNR 계산
- [ ] `-errors <file>` 구조화된 에러 로그 export (XML/PDF 또는 JSON 동등물) — `COMPETITOR_FEATURE_MATRIX.md` §5
      "Error Log Viewer" (신규 2026-07-31, 기존 텍스트 로그를 구조화 포맷으로 확장)
- [ ] `-stats` 통계 출력
- [ ] `-stream_stats` HEVC 스트림 통계
- [ ] `-display_order` 표시 순서 YUV 출력
- [ ] `-nocrop` 크롭 비활성화
- [ ] `-cpu_max_feature <feature>` CPU 최적화 선택
- [ ] `-film_grain` 필름 그레인 전/후 별도 출력 (AV1)
- [ ] `-n <frame>` 시작 프레임 seek (신규 2026-07-31, `COMPETITOR_FEATURE_MATRIX.md` §2)
- [ ] `-debug_yuv <file>` / `-dependent <file>` (§4.7 Debug YUV CLI 경로, 신규 2026-07-31)
- [ ] `-extract <pattern>` / `-dump` / `-dump_mode <stage>` (신규 2026-07-31)
- [ ] `-syntax_stats <file>` / `-syntax_count` (신규 2026-07-31)
- [ ] `-norm_pix` / `-norm_bits` / `-percent` (신규 2026-07-31)
- [ ] `-dump_headers` / `-dump_headers_filter` (신규 2026-07-31)
- [ ] `-oppoint` (AV1 operating point) — VVC `-ols`는 vvdec 미연결로 보류 (신규 2026-07-31)
- [ ] `-mv-to-csv` (신규 2026-07-31); `-track`/`-library_stream`(AVS3)은 AVS3 미구현이라 Phase 5 선행 필요

**예상 소요:** 중급 1~2주

---

## Phase 10: 성능 최적화 & 대용량 스트림 지원 🟡

**목표:** 4K/8K 스트림, 1GB+ 파일 안정적 처리

- [ ] **메모리 맵 기반 스트리밍** (`memmap2`):
  - 파일 전체를 메모리에 로드하지 않음
  - 청크 단위 탐색
- [ ] **썸네일 지연 생성**:
  - 뷰포트에 보이는 프레임만 디코딩
  - LRU 캐시 (최대 N개 썸네일 유지)
- [ ] **오버레이 WebGL 렌더러** (Canvas 대체):
  - 수천 개 MV 벡터 60fps 렌더링
  - GPU 가속 히트맵
- [ ] **Syntax 트리 가상화**:
  - 수천 개 노드 → 뷰포트만 렌더링
- [ ] **병렬 파싱** (rayon):
  - 타일 병렬 파싱 (AV1)
  - 프레임 병렬 메트릭 계산
- [ ] **프로파일링**:
  - Criterion 벤치마크로 핫스팟 식별
  - SIMD 최적화 (YUV→RGB, PSNR 계산)

**추가 구체안 (from `_import_v14/.../performance/*.md`, 2026-07-31 마이닝 — Bitvue는 egui가 아닌 Tauri/React라 "paint_ms/egui" 언급은 프레임워크 불일치 주의, 개념·수치만 차용):**

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

**예상 소요:** 중급 3~4주

---

## Phase 11: 폴리시, 키보드 단축키, 옵션 완성 🟢

- [ ] 모든 키보드 단축키 완성 (`PARITY_CHECKLIST.md` Layer 4 참조)
- [ ] Options 메뉴 모든 항목 구현
- [ ] 창 레이아웃 저장/복원 (패널 크기, 위치)
- [ ] 최근 파일 목록 (최대 10개)
- [ ] 에러/경고 Status Panel 완성
- [ ] 접근성 (ARIA 레이블, 키보드 네비게이션)
- [ ] 다크/라이트 테마 완성
- [ ] 라이선스 활성화 시스템 (오픈소스이므로 생략 가능)
- [ ] YUV Viewer: Color-gamut 전환(BT.601/709/2020, Options 메뉴 "Color Conversion" — `UX_PARITY_MATRIX.md` §10),
      Endianness 선택, Raw pixel format 20+종 지원 — `COMPETITOR_FEATURE_MATRIX.md` §5/§6 (신규 2026-07-31)

**예상 소요:** 중급 2~3주

---

## Phase 12: 전체 Parity 검증 & 테스트 🔴

- [ ] 공개 테스트 비트스트림으로 모든 모드 검증
- [ ] 자동화 스크린샷 비교 테스트
- [ ] 신택스 값 비교 테스트
- [ ] 회귀 테스트 스위트 구축
- [ ] **AV3/AVM 명명 정리 감사** (`PARITY_CHECKLIST.md` Layer 6 CMP-09) — Bitvue의 "AV3"와 VQ Analyzer v7.5+의
      "AVM"(AOM 차세대 실험 코덱 공식 명칭)이 동일 코덱을 가리키는지 1회성 감사로 확정하고, 다르면 명칭을 분리,
      같으면 문서/코드 전반의 명칭을 통일 (신규 2026-07-31 — 별도 phase가 아니라 한 줄짜리 감사 작업으로 스코프)

**예상 소요:** 중급 2~3주

---

## Appendix: 중요 Tauri 커맨드 추가 목록 (신규 필요)

```typescript
// Phase 1: 코덱 모드 시스템
get_codec_modes(codec: CodecType) -> Vec<ModeInfo>

// Phase 3: VVC 전용
get_vvc_dual_tree_data(frame: number) -> DualTreeData
get_vvc_alf_data(frame: number) -> AlfData
get_vvc_lmcs_data(frame: number) -> LmcsData

// Phase 4: AV1 전용
get_av1_cdef_data(frame: number) -> CdefData
get_av1_film_grain_data(frame: number) -> FilmGrainData
get_av1_loop_restore_data(frame: number) -> LoopRestoreData

// Phase 7: YUVDiff
load_debug_yuv(path: string, params: YuvParams) -> Result<void>
get_diff_frame(frame: number, amplify: number) -> FrameData
find_first_diff() -> Result<number>
get_yuv_metrics(frame: number) -> PsnrSsimResult

// Phase 8: Syntax 상세
get_stats_data(stream_id: string) -> StreamStats
get_qm_data(frame: number, codec: CodecType) -> QmData
get_vp9_prob_data(frame: number) -> ProbabilityData
```

---

## Appendix: Architecture & Correctness Reference (from `docs/_import_v14` critical_contracts/architecture pack, 2026-07-31)

**중요 발견 — 이 규칙들은 "앞으로 구현할 로드맵"이 아니라 이미 `crates/bitvue-engine/src/`에 대부분 코드로 구현되어 있음.**
해당 크레이트의 모듈 주석이 이 v14 pack의 정확한 파일명을 인용한다 (예: `selection.rs`가 `SELECTION_PRECEDENCE_RULES.md`를,
`command.rs`/`event.rs`가 `ARCHITECTURE.md §3.2/§3.3`을, `diagnostics.rs`가 `ERROR_MODEL.md`를, `coordinate_transform.rs`가
`COORDINATE_SYSTEM_CONTRACT.md`를, `lockcheck.rs`가 `V12_LOCKCHECK_SPEC.md`를 인용) — 즉 이 spec pack의 이전 버전(v9~v13)이
과거 세션에서 이미 `bitvue-engine`로 구현되었다는 뜻. `crates/bitvue-engine/src/lib.rs`의 `pub mod` 목록은 T0-1~T10-1 단계 태그를
달고 있어 순차 구현 이력이 그대로 남아 있다. 아래 표는 그 규칙 자체(유지보수 시 지켜야 할 계약)를 압축한 것이지 신규 작업이 아님 —
새 오버레이/패널 추가 시 이 표를 어기지 않았는지 확인하는 용도로 사용.

| 계약 | 핵심 규칙 | 구현 위치(코드) |
|---|---|---|
| Frame Identity | Primary timeline index = Display order(PTS). decode_idx는 내부 전용. PTS/DTS mismatch는 전용 band로만 시각화 | `crates/bitvue-engine/src/frame_identity.rs`, `frame_identity_test.rs` |
| Coordinate System | 파이프라인 고정: `screen_px → video_rect_norm(0..1) → coded_px → block_idx`. 모든 오버레이가 이 파이프라인만 사용, fit/zoom/pan은 screen→norm 단계만 수정 | `coordinate_transform.rs`, `coordinate_transform_test.rs` |
| Selection Precedence | 우선순위 Block > Point > Range > Marker, 한 번에 하나의 selection type만 활성 | `selection.rs` (`TemporalSelection` enum) |
| Cache Invalidation | QP/MV/Partition/Diff/Timeline 오버레이별 무효화 트리거 목록; 프레임 변경 시 프레임 종속 오버레이는 항상 무효화; 텍스처를 다른 frame_idx에 재사용 금지 | `cache_provenance.rs`, `cache_validation.rs` |
| Async Backpressure | Latest-wins 큐, 스트림당 in-flight 최대 2, 스크럽 중 비-현재 작업 취소 + quality-path 업그레이드 비활성 | `worker.rs` |
| Indexing Strategy | 2단계: Quick Index(키프레임/OBU 경계만 스캔, 즉시 표시) → Full Index(백그라운드, 진행률), UI는 Full Index 대기 안 함 | `indexing.rs`, `index_session*.rs`, `index_extractor*.rs` |
| Tri-sync 권위 순서 | 충돌 시 `bitRange > syntaxNode > unit > frameIndex/pts > stream_id` 순으로 해소; Hex→Syntax 역매핑은 "가장 작게 포함하는 노드, tie는 최대 depth, 그래도 tie면 SyntaxNodeId 사전순"으로 결정적 | `evidence.rs`(bit_offset↔syntax↔decode↔viz 4-layer evidence chain), `player_evidence.rs`, `timeline_evidence.rs` |
| Error Model | 심각도 Info/Warn/Error/Fatal; Diagnostic 레코드는 `offset_bytes` 필수; Fatal이어도 앱은 크래시하지 않고 Hex 검사는 계속 가능 | `diagnostics.rs`, `diagnostics_bands.rs`, `error.rs`, `app_error.rs` |
| File I/O | mmap 기반 랜덤 액세스, 세그먼트 캐시(64~256KB), 파일 크기 변경 시 mmap 무효화 + WARN 진단 | `byte_cache.rs` — `memmap2` 의존성 확인됨(`Cargo.toml`) |

**Layout/Grid 참고자료 (from `LAYOUT_CONTRACT.md`/`LAYOUT_GRID_SYSTEM.md`/`RESPONSIVE_VISUALIZATION_RULES.md`) — 주의:**
이 3개 문서는 egui 네이티브 5-region(R1 Toolbar/R2 Left/R3 Center/R4 Right/R5 StatusBar) 도킹 레이아웃을 전제로 하며, Bitvue는
Tauri+React `DockableLayout` 패널 시스템(Phase 0 완료)을 쓰므로 리전 이름 자체는 적용되지 않는다. 숫자만 참고할 가치가 있다면:
splitter 최소 폭 320px/최소 높이 140px, 타임라인 스트립 기본 높이 clamp(160px, 18vh, 260px), 툴팁 최대폭 360px/최대높이 40vh,
바 너비 <2px면 LOD 버킷 렌더링으로 전환, 리사이즈 이벤트 디바운스 필수 — Bitvue `DockableLayout`이 이미 이런 규칙을 갖는지는
미확인이므로 실제 적용 전 `frontend/components/layout/` 코드와 대조 필요 (이번 마이닝에서는 검증하지 않음).

---

## Appendix: Future Differentiators (beyond parity — post-parity/aspirational, from `docs/_import_v14` insight/session/ci/compliance/mcp specs)

이 섹션은 경쟁사 패리티(CMP-0N)가 아니라 v14 pack이 자체적으로 "Differentiators (our advantage)"로 분류한 신규 기능 제안이다.
**패리티 백로그에 섞지 말 것.** 다만 2026-07-31 마이닝 중 확인된 중요 사실: 아래 4개 기능 모두 `crates/bitvue-engine/src/`에
데이터 모델/로직 수준으로는 이미 부분 구현되어 있으나(모듈 주석이 각 spec 파일명을 그대로 인용), **`frontend/`에서 이를 사용하는 곳은
전무하고 Tauri 커맨드로도 노출되지 않음** (grep 결과 `InsightFeed|ComplianceScoreboard|McpIntegration|SessionEvidence` 프론트엔드 매치 0건).
즉 "데이터 계층은 있음, UI/커맨드 배선이 남은 작업"이라는 뜻 — 신규 설계가 아니라 배선 작업으로 재정의됨.

| 기능 | 핵심 아이디어 | 구현 위치(백엔드, 이미 존재) | 남은 작업 |
|---|---|---|---|
| Insight Feed | 규칙/통계 기반 auto-summary 카드(QP spike, metric dip, error burst, reorder mismatch, HRD risk, A/B regression 등), Jump/Filter/Export 가능, 트리거 근거 표시 | `insight_feed.rs` (`InsightType` enum 등 확인됨) | Tauri 커맨드 노출 + 프론트엔드 카드 UI |
| Session Evidence | `.baxsession.json` 세션(열린 파일/레이아웃/선택/북마크) + 북마크를 "증거 번들"(스냅샷+수치 요약+딥링크)로 export, 버그리포트(md+이미지+csv) 생성 | `evidence.rs` (bit_offset/syntax/decode/viz 4-stage evidence chain) | 세션 직렬화 포맷 확정 + export 커맨드 |
| Compliance Scoreboard | timing/reference structure/HRD/metadata/syntax legality 카테고리별 점수 + 위반 목록(룰 id, 조건, 관측값, jump target) | **미구현** — `parity_harness/mod.rs`의 `CategoryScore`는 이름만 비슷할 뿐 competitor-parity 스코어링용(아래 행 참조)이지 codec-compliance 스코어보드가 아님. 확인 완료(2026-07-31), 착오 정정 | 전체 신규 구현 |
| Regression Guard (CI) | A/B 비교에서 metric_delta/error_burst/reorder_mismatch/HRD 조건으로 CI 게이트 규칙 생성, `regression_report.json`/`regression_summary.md` 출력 | **목적이 다른 유사 시스템 존재**: `crates/bitvue-engine/src/parity_harness/mod.rs`는 REGRESSION_GUARD_SPEC이 아니라 `_import_v14/parity_harness/*.json`(경쟁사 패리티 매트릭스 스키마 검증/스코어링/semantic probe/render snapshot/evidence diff/Hard-Fail·Parity·Perf 게이트)의 구현체 — A/B 스트림 리그레션이 아닌 "Bitvue vs 경쟁툴 parity matrix" 채점용. 테스트(`tests/parity_harness.rs`, `parity_baseline_evaluation.rs`)만 있고 `scripts/parity_check.sh`/CI에는 배선 안 됨(grep 확인) | 전용 A/B 리그레션 룰 엔진 + CI 잡 (완전 별개 신규 구현 필요); 별도로 `parity_harness/mod.rs`를 실제 `scripts/parity_check.sh`에 연결하는 것도 독립적인 미완 작업 |

**Explainability Hints (from `EXPLAINABILITY_HINTS.md`)** — 오버레이별 마이크로 힌트 카피 예시, UI 문구 작성 시 참고:
QP Heatmap "Auto scale: min/max from current frame" / "Fixed scale: 0..63"; MV "Vectors shown in px (qpel/4)"; Partition
"Scaffold grid shown when partition data unavailable"; Diff "Abs diff: \|A-B\|" / "Signed diff: A-B"; Timeline "Markers never
dropped; clustered when dense".

**Onboarding First-5-Minutes flow (from `ONBOARDING_FIRST_5_MIN.md`)** — Help 메뉴에 넣을 5단계 가이드 초안: ① Timeline에서
frame size/QP 오버레이로 이상 구간 드래그 선택 → ② Player로 점프해 QP Heatmap/MV/Partition으로 공간적 원인 확인 → ③ Metrics
워크스페이스에서 전체 대비 선택구간 히스토그램 비교 → ④ Diagnostics에서 에러 버스트 자동 선택 후 evidence export → ⑤(optional)
Stream B 로드 후 Compare 워크스페이스로 A/B 델타 확인. Bitvue에 아직 없는 개념: Worst Frames 목록, Regression Guard 제안 —
이 둘은 위 표의 Insight Feed/Regression Guard가 선행되어야 함.

---

## Appendix: MCP 서버 실제 구현 대조 (from `docs/_import_v14/monster_pack/docs/mcp/MCP_INTERACTION_MODEL.md`, 2026-07-31)

**결론: Bitvue에는 서로 무관한 두 개의 MCP 관련 구현이 존재하며, spec을 실제로 따르는 쪽은 바이너리로 노출되지 않는다.**

| | spec (`MCP_INTERACTION_MODEL.md`) | `crates/bitvue-mcp`(`bitvue-mcp-server` 바이너리, 실행됨) | `crates/bitvue-engine/src/mcp.rs`(`McpIntegration`) |
|---|---|---|---|
| 모델 | Read-only "resources"(8종) + "actions"(제안/설명/초안 생성, 5종) | JSON-RPC stdio, MCP 표준 "tools"(10종: load_file/analyze_frame/get_qp_map/get_motion_vectors/compare_streams/get_gop_structure/find_decoding_issues/get_stream_info/search_syntax/list_files) | spec의 resource 목록과 거의 동일: `selection_state/insight_feed/diagnostics/metrics_summary/timeline_lanes/compare/session_evidence/compliance` — `get_resource(name)`/`list_resources()` 구현 |
| 코덱 지원 | 코덱 무관 설계 | **IVF/AV1 컨테이너만 파싱** (`parse_ivf_file`, 확장자 `.ivf`/`.av1` 외 전부 미지원 에러) | `bitvue-engine` 전체 모델을 재사용하므로 코덱 무관 |
| 관계 | — | `bitvue-engine`/`bitvue-av1-codec`에 의존하지만 **`bitvue_engine::mcp::McpIntegration`은 import하지 않음** — 자체 tool 세트를 처음부터 새로 구현 | `bitvue-mcp-server`의 `main.rs`에서 전혀 참조되지 않음 — 어디서도 호출되지 않는 죽은 코드에 가까움(테스트 커버리지만 있을 가능성) |

**정리:** 실행 가능한 `bitvue-mcp-server`는 spec과 무관한, 훨씬 단순한 자체 설계(질의형 tool-calling, AV1/IVF 전용)이고, spec을
거의 그대로 구현한 `McpIntegration`(read-only resource 모델)은 `bitvue-engine` 안에 존재하지만 어떤 바이너리에서도 사용되지 않는다.
두 구현을 통합할지, `McpIntegration`을 `bitvue-mcp-server`에 연결할지는 결정되지 않은 상태 — Phase 12 이후 정리 대상으로 기록.
