# Anti-Pattern Catalog — UX_SCENARIO: 자동화 검사 + 시나리오 테스트

이 문서는 더 큰 안티패턴 카탈로그의 일부이자(전체 목차는 별도 작성 중인 `docs/anti-patterns/INDEX.md` 참고), UI/UX + Tauri Phase 3 웨이브(IA/SYNC/ASYNC/TIMELINE/TREE-HEX/VIZ/INPUT/LAYOUT/ERR/A11Y, TAURI-CMD/EVT/WEB, FRONT_REACT)의 마무리·검증 파일입니다. 다른 Phase 3 파일들은 각 안티패턴 항목에 "탐지" 필드를 갖지만, 실제로 "어떻게 잡아낼 것인가"는 대부분 이 문서의 정적 검사 항목, 계측 포인트, 또는 시나리오 테스트 중 하나로 귀결되도록 설계되어 있습니다. 즉 이 파일은 카탈로그가 아니라 카탈로그를 실행 가능하게 만드는 검증 레이어이며, 앱이 실제로 만들어진 뒤에는 3절의 시나리오들이 회귀 테스트 스위트로 전환되는 것을 전제로 합니다.

---

## 1. 정적 검사로 잡을 수 있는 것

빌드 타임/커밋 타임에 소스 코드만 보고 판정 가능한 것들. grep 기반의 저비용 검사부터 AST/eslint 커스텀 룰까지 난이도가 다양하지만, 전부 실행 없이 "패턴이 존재하는가"만으로 flag가 가능하다.

| # | 검사 항목 | 탐지 방법 | 잡아내는 패턴 | 관련 카테고리 |
|---|---|---|---|---|
| 1 | invoke 호출 수와 command granularity | AST: 이벤트 핸들러/effect 본문 내 `invoke(` 호출 수를 카운트, 동일 핸들러에서 N회 이상(예: 3회) 발생 시 flag | 하나의 사용자 액션이 여러 개의 잘게 쪼개진 command를 순차 호출하는 chatty IPC — 배치 커맨드로 합쳐야 할 후보 | TAURI_CMD |
| 2 | unlisten 누락 | AST: `listen(...)`/`event.listen(...)` 호출이 `useEffect` 내부에 있는데 반환된 Promise의 resolve 값(unlisten 함수)이 cleanup 함수에서 호출되지 않는 경우 flag | 컴포넌트 unmount/재실행 시 이벤트 리스너가 해제되지 않아 중복 등록되는 누수 | TAURI_EVT |
| 3 | loading boolean 남용 | grep/AST: 한 컴포넌트 내에 독립적인 `isLoading`/`pending`/`busy` 형태의 boolean state가 2개 이상 존재 | 상태 조합이 지수적으로 늘어나 "로딩도 아니고 에러도 아닌데 멈춰있는" 상태를 만드는 boolean soup — status enum/state machine으로 대체해야 할 후보 | UIX_ASYNC / FRONT_REACT |
| 4 | 대형 배열을 JSON DTO로 반환 | Rust AST/grep: `#[tauri::command]` 함수가 `Vec<T>`(T가 픽셀/샘플/좌표 등 수치형) 또는 대형 `serde_json::Value`를 반환하면서 청크/스트리밍/바이너리 채널 경유 표시가 없는 경우 flag | 프레임 전체 픽셀/전체 MV 배열을 매 요청마다 JSON 직렬화해 IPC로 넘기는 패턴 — 파일 크기에 비례해 payload가 무제한 증가 | Phase 1 IPC.md |
| 5 | 리스트 virtualizer 부재 | grep: timeline/tree/hex 뷰 컴포넌트 내 `.map(` 으로 JSX 리스트를 렌더링하면서 `react-window`/`react-virtual`/`@tanstack/virtual` 등 windowing 라이브러리 import가 없는 경우 flag | 수천~수백만 개 항목(프레임, syntax 노드, hex 바이트 행)을 DOM에 전부 마운트하는 패턴 | UIX_TIMELINE / UIX_TREE_HEX |
| 6 | pointermove에서 global state 갱신 | AST: `onPointerMove`/`onMouseMove` 핸들러 본문이 throttle/rAF/디바운스 래퍼 없이 직접 `setState`/store setter(`dispatch`, zustand `set` 등)를 호출하는 경우 flag | 마우스 이동마다 전역 리렌더를 유발해 스크럽/호버 중 프레임 드랍의 원인이 되는 패턴 | FRONT_REACT |
| 7 | 컴포넌트에서 backend command 연속 호출 | AST: 동일 함수 스코프에서 `await invoke(...)`가 `Promise.all`/배치 커맨드 없이 순차적으로 2회 이상 등장하는 경우 flag | 병렬화 가능한 왕복(round-trip)을 직렬로 처리해 지연시간이 합산되는 패턴 | TAURI_CMD |
| 8 | index를 React key로 사용 | eslint `react/no-array-index-key` 동등 규칙 또는 grep: `.map((\w+,\s*index|i)\s*=>` 뒤에 `key={index}`/`key={i}` 패턴 | 리스트 순서 변경/삽입 시 잘못된 DOM 재사용으로 상태 꼬임/애니메이션 깨짐을 유발하는 패턴, 특히 timeline/tree처럼 순서가 자주 바뀌는 리스트에서 치명적 | FRONT_REACT |
| 9 | 색상만 있는 범례 | 정적 스캔: 범례/Chip/Legend 컴포넌트 사용부에서 `backgroundColor`/`color` prop만 설정되고 `label`/`pattern`/`icon`/`aria-label` 중 아무것도 없는 경우 flag | 색약 사용자 또는 저채도 모니터에서 overlay 범례(MV 방향, MB 타입, QP 구간 등)를 구분할 수 없는 패턴 | UIX_A11Y / UIX_VIZ |
| 10 | 직접 작성된 modal·tooltip 중복 | grep: 공용 UI 킷 디렉터리(`components/ui/` 등) 밖에서 파일명/컴포넌트명이 `Modal`/`Dialog`/`Tooltip`/`Popover` 정규식에 매칭되는 독자 구현 정의 | 포커스 트랩, ESC 닫기, z-index, 위치 계산 등을 매번 새로 구현해 접근성/일관성이 파편화되는 패턴 | UIX_LAYOUT |

---

## 2. 계측(런타임 측정)으로 잡을 수 있는 것

정적 검사로는 드러나지 않고 실제로 앱을 조작해야만 관측되는 것들. 아래 항목들은 모두 "무엇을 계측할지"와 "왜 그것이 문제 신호인지"를 함께 정의해, 3절 시나리오들의 pass/fail 기준으로 재사용된다.

| # | 계측 항목 | 계측 방법 및 의미 |
|---|---|---|
| 1 | command latency p50/p95/p99 | `invoke()`를 얇은 타이밍 래퍼로 감싸 command 이름별 히스토그램을 기록. p95/p99가 SLA(예: UI 블로킹 커맨드 100ms, 무거운 파싱 1s)를 초과하면 파일 크기/해상도가 커질수록 조용히 느려지는 커맨드를 조기 발견 |
| 2 | serialized payload bytes | 각 IPC 응답의 직렬화 크기(JSON.stringify 길이 또는 실제 전송 바이트)를 프레임 수/해상도와 함께 기록. payload가 입력 크기에 선형 이상으로 증가하면 페이지네이션/스트리밍이 빠진 것 |
| 3 | frontend long task | `PerformanceObserver({entryTypes:['longtask']})`로 메인 스레드 50ms 초과 작업을 전량 캡처. 스크럽/호버 등 인터랙션 중 발생하면 체감 버벅임의 직접 증거 |
| 4 | interaction-to-next-paint (INP) | Web Vitals INP를 클릭/드래그/키다운 등 인터랙션 클러스터별로 수집해 릴리스 간 회귀를 추적 |
| 5 | frame scrub 중 dropped input | 스크럽 세션 동안 수신된 pointermove/keydown 이벤트 수 대비 실제로 렌더에 반영된 이벤트 수의 비율. 비율이 낮으면 이벤트 코얼레싱/백프레셔가 없다는 신호 |
| 6 | DOM node 수 | `document.querySelectorAll('*').length`를 tree/hex/timeline처럼 밀도가 높은 뷰에서 주기적으로 샘플링. 조작 반복 후에도 무한 증가하면 virtualization 부재 또는 언마운트 누락 |
| 7 | component render count | React Profiler API(또는 why-did-you-render)로 사용자 액션 1회당 컴포넌트별 렌더 횟수를 측정. 액션과 무관한 컴포넌트가 함께 리렌더되면 memoization/context 분리 실패 신호 |
| 8 | canvas redraw time | overlay/heatmap을 그리는 `requestAnimationFrame` 콜백의 실행 시간을 계측. 60fps 기준 예산(16.6ms)을 초과하면 캔버스 재계산 로직이 병목 |
| 9 | peak memory | 대용량 파일 워크플로우 동안 `performance.memory`(또는 Tauri 네이티브 메모리 조회 API)를 주기 샘플링. 세션이 길어질수록 baseline으로 회귀하지 않고 누적 증가하면 캐시/리스너 누수 |
| 10 | stale response discard 수 | 더 최신 요청에 의해 무효화되어 폐기된 비동기 응답의 수를 카운트. 이 카운터가 0인 채 빠른 연속 입력이 발생하면 race-condition 가드 자체가 없다는 뜻이고, 반대로 값이 지나치게 크면 불필요한 백엔드 작업이 낭비되고 있다는 뜻 |
| 11 | cancellation latency | 사용자가 취소를 누른 시점과 백엔드 작업이 실제로 종료되는 시점 사이의 지연을 측정. 프론트엔드 UI는 백엔드 정리를 기다리지 않고 즉시 unlock되어야 하며, 이 값은 그와 별개로 추적해 "취소했다는데 CPU는 계속 도는" 상황을 검출 |

---

## 3. 시나리오 테스트 (Acceptance Scenarios)

앱이 실제로 빌드된 뒤 회귀 테스트로 그대로 옮길 수 있도록, 각 시나리오는 액션과 측정 가능한 pass 기준을 짝지어 정의한다. 1·2절의 정적 검사/계측 항목이 여기서 실제 판정 도구로 쓰인다.

### UX-SCENARIO-001: 10GB 파일 열기

**action**:
10GB 크기의 컨테이너 파일(MP4/MKV)을 File > Open으로 로드한다.

**expect**:
- 파일 선택 직후 첫 UI 반응(로딩 인디케이터 표시)까지 100ms 미만
- 전체 파일을 메모리에 로드하지 않고 스트리밍/인덱싱 방식으로 처리 — peak memory가 파일 크기에 비례해 증가하지 않음(예: 파일 크기의 5% 이하로 유지)
- 인덱싱 진행률 표시가 실제 진행 상황을 반영하며, 진행 중에도 UI가 멈추지 않고 취소 버튼이 즉시 동작함
- 완료 후 첫 프레임이 화면에 렌더링되기까지 걸리는 총 시간이 파일 크기와 무관하게 상한선(수 초) 이내

**relates to**:
TAURI_CMD, UIX_ASYNC, Phase 1 IO/MEM

---

### UX-SCENARIO-002: timeline 1,000프레임 빠르게 scrub

**action**:
5초 동안 timeline을 연속 scrub한다.

**expect**:
- 입력 반응 p95 < 100ms
- 최종 선택 프레임만 정밀 분석
- 오래된 결과가 표시되지 않음
- pending 작업 수가 제한됨
- UI main thread long task가 200ms를 넘지 않음

**relates to**:
UIX_TIMELINE, UIX_ASYNC, FRONT_REACT

---

### UX-SCENARIO-003: syntax와 hex를 반복 이동

**action**:
syntax tree의 특정 노드를 선택해 대응하는 hex view 위치로 점프하고, 다시 syntax tree로 복귀하는 왕복을 20회 반복한다.

**expect**:
- 매 왕복마다 하이라이트 동기화 지연 50ms 미만
- hex view가 매번 해당 offset이 뷰포트 안에 보이도록 자동 스크롤됨(수동 재탐색 불필요)
- 반복 횟수가 늘어도 지연시간이 누적 증가하지 않음(리스너 중복 등록/메모리 누수 없음 — 등록된 리스너 수가 왕복 횟수에 비례해 늘지 않음)
- 20회 반복 후 DOM 노드 수가 초기 상태 대비 유의미하게 증가하지 않음

**relates to**:
UIX_TREE_HEX, FRONT_REACT

---

### UX-SCENARIO-004: 분석 도중 다른 파일 열기

**action**:
대용량 분석 작업(예: 전체 프레임 QP heatmap 계산)이 진행 중인 상태에서 File > Open으로 새 파일을 연다.

**expect**:
- 이전 분석 작업이 명시적으로 취소되거나, 새 파일 로드와 독립적으로 안전하게 백그라운드에서 계속/폐기됨(크래시 없음)
- 새 파일의 UI가 이전 분석 결과로 오염되지 않음(stale data 노출 0건)
- 이전 작업에서 진행 중이던 invoke 응답이 새 파일 컨텍스트의 상태를 갱신하지 않음(race condition 없음)
- "이전 분석이 취소/중단되었다"는 피드백이 사용자에게 노출됨

**relates to**:
TAURI_CMD, TAURI_EVT, UIX_ASYNC, UIX_ERR

---

### UX-SCENARIO-005: 작업 중 취소 후 재시작

**action**:
장시간 분석 작업(예: 전체 GOP 모션벡터 추출)을 시작해 50% 지점에서 취소하고, 즉시 동일 작업을 재시작한다.

**expect**:
- 취소 버튼 클릭 후 UI가 즉시(100ms 미만) idle 상태로 복귀함(백엔드 완료를 기다리지 않음)
- 재시작 시 이전 작업의 부분 결과가 새 작업 결과와 섞이지 않음
- 취소 후 재시작을 5회 연속 수행해도 메모리/스레드가 누적되지 않음(peak memory가 반복 횟수에 비례해 증가하지 않음)
- cancellation latency 계측치로 확인했을 때, 취소 신호가 실제로 백엔드에 전달되어 무의미한 계산이 계속되지 않음

**relates to**:
TAURI_CMD, UIX_ASYNC, Phase 1 CONC

---

### UX-SCENARIO-006: 손상된 파일 열기

**action**:
헤더가 잘리거나 중간 NALU가 손상된 파일을 연다.

**expect**:
- 애플리케이션이 크래시하거나 무한 로딩에 빠지지 않음
- 파싱 가능한 부분까지는 정상 표시되고, 손상 지점부터는 명확한 에러가 표시됨(어느 offset/구조에서 실패했는지 포함)
- 에러 메시지가 사용자가 다음 행동을 취할 수 있는 정보를 포함(단순 "Error" 텍스트가 아님)
- 손상된 파일을 닫고 정상 파일을 다시 여는 데 문제가 없음(앱 전역 상태가 오염되지 않음)

**relates to**:
UIX_ERR, Phase 1 PARSE

---

### UX-SCENARIO-007: 4K/8K 프레임 overlay 전환

**action**:
8K 프레임에서 MV overlay → MB type overlay → QP heatmap overlay를 각 전환 사이 500ms 이내로 빠르게 순차 전환한다.

**expect**:
- 각 overlay 전환의 canvas redraw time이 프레임 예산(16.6ms) 내이거나, 최소한 육안으로 버벅임이 느껴지지 않음
- 전환 도중 이전 overlay와 새 overlay가 겹쳐 그려지는 flicker/artifact가 없음
- 연속 전환 5회 동안 dropped input/long task가 발생하지 않음
- 표시되는 overlay 데이터가 항상 현재 프레임과 일치함(frame index mismatch 없음)

**relates to**:
UIX_VIZ, Phase 1 PIXEL/PERF

---

### UX-SCENARIO-008: VFR 파일 탐색

**action**:
Variable Frame Rate 파일에서 timeline을 프레임 단위로 이동하며 PTS/DTS 순서가 뒤섞인 구간(B-프레임 재정렬 구간)을 탐색한다.

**expect**:
- timeline 상의 프레임 위치가 실제 presentation 시간과 일관되게 매핑됨(프레임 간격이 왜곡되어 표시된다면 그 사실이 명확히 시각화됨)
- 탐색 중 표시되는 프레임 번호/타임스탬프가 실제로 디코딩되어 화면에 그려진 프레임과 항상 일치함
- 재정렬 구간에서 순방향/역방향 탐색 모두 의도한 정확한 프레임에 도달함

**relates to**:
UIX_TIMELINE, Phase 1 CODEC

---

### UX-SCENARIO-009: 창 크기 최소화·복원

**action**:
분석 작업이 진행 중인 상태에서 앱 창을 최소화하고 30초 후 복원한다.

**expect**:
- 최소화 상태에서도 백엔드 작업이 계속 진행되거나, 의도적으로 일시정지되어 복원 시 재개됨 — 둘 중 하나가 명시적으로 보장됨(어중간하게 멈춰 있지 않음)
- 복원 시 canvas/webview가 올바르게 재렌더링됨(빈 화면이나 깨진 렌더가 없음)
- 최소화 중 도착한 이벤트/진행률 업데이트가 유실되지 않고 복원 시점에 최신 상태로 반영됨

**relates to**:
TAURI_EVT, UIX_ASYNC

---

### UX-SCENARIO-010: backend 작업 중 WebView reload

**action**:
분석 작업이 진행 중인 상태에서 WebView를 강제 새로고침(reload)한다.

**expect**:
- 백엔드 Rust 프로세스는 크래시하지 않고 계속 살아있음
- 프론트엔드가 재초기화된 뒤 진행 중이던 백엔드 작업의 상태를 다시 조회할 수 있는 경로가 존재하거나, 조회가 불가능하면 명확히 실패로 처리되고 재시작을 안내함
- 재초기화 과정에서 invoke/이벤트 리스너가 중복 등록되어 메모리 누수나 이벤트 중복 수신을 일으키지 않음

**relates to**:
TAURI_CMD, TAURI_EVT, UIX_ERR

---

### UX-SCENARIO-011: dual-stream compare 워크플로우

**action**:
같은 소스를 서로 다른 비트레이트/코덱으로 인코딩한 두 스트림을 동시에 로드해 synchronized scrub으로 비교한다.

**expect**:
- 두 스트림의 timeline이 동일한 사용자 입력에 대해 동기화되어 이동함(프레임 매핑 오차가 없거나, 매핑이 불가능한 경우 그 사실이 명시적으로 표시됨)
- 한쪽 스트림의 로딩/분석 지연이 다른 쪽 스트림의 반응성을 블로킹하지 않음
- 두 스트림 각각의 overlay/heatmap이 독립적으로 정확하게 렌더링됨(교차 오염 없음)
- 비교 뷰 전체의 메모리 사용량이 단일 스트림 대비 선형적으로 증가함(비정상적인 배수 증가가 없음)

**relates to**:
UIX_TIMELINE, UIX_VIZ, Phase 1 MEM

---

### UX-SCENARIO-012: 키보드만으로 전체 워크플로우 수행

**action**:
마우스 없이 키보드만으로 파일 열기 → timeline 탐색 → syntax tree 노드 선택 → overlay 전환 → 분석 결과 확인까지 전체 워크플로우를 수행한다.

**expect**:
- 모든 인터랙티브 요소가 Tab/화살표 키로 접근 가능하고 포커스 순서가 논리적임
- 현재 포커스 위치가 항상 시각적으로 명확히 표시됨(focus indicator 누락 없음)
- 프레임 단위 이동, overlay 토글 등 핵심 기능에 문서화된 단축키가 존재함
- 스크린 리더 사용 시 상태 변경(로딩/완료/에러)이 `aria-live` 등으로 공지됨

**relates to**:
UIX_A11Y, UIX_INPUT

---

### UX-SCENARIO-013: 장시간 세션 메모리 누수 감지

**action**:
8시간 근무 세션을 시뮬레이션하는 스크립트로, 파일 열기/닫기·overlay 전환·timeline scrub을 200회 반복 수행한다.

**expect**:
- peak memory가 세션 시간에 비례해 단조 증가하지 않음(파일을 닫을 때마다 baseline으로 회귀)
- DOM 노드 수가 반복 횟수에 비례해 누적되지 않음
- 등록된 이벤트 리스너 수가 반복 횟수에 비례해 누적되지 않음(unlisten이 정상 동작함을 확인)
- 200회 반복 후에도 command latency p95가 첫 10회 대비 유의미하게 저하되지 않음

**relates to**:
TAURI_EVT, FRONT_REACT, Phase 1 MEM
