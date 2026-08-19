# Anti-Pattern Catalog — UIX_ERR: 오류 메시지와 진단 UX

이 문서는 Bitvue 안티패턴 카탈로그의 한 분류(UIX_ERR)이며, 전체 목록은 `docs/anti-patterns/INDEX.md`(별도 작성 중)를 참고한다. UI/UX+Tauri Phase 3 웨이브의 산출물로, Phase 1의 `ERR.md`(Rust 쪽 error 타입 설계·panic 전략)와 짝을 이룬다. `ERR.md`가 "에러를 어떤 타입으로 표현하고 어디서 잡을 것인가"를 다룬다면, 이 문서는 같은 에러가 "화면에 어떻게 보이고 사용자가 그것으로 무엇을 할 수 있는가"를 다룬다 — 하나의 에러가 backend에서 발생해 UI까지 도달하는 전체 여정 중 후반부다. 1단계(일반 참조 카탈로그) 산출물로, 아직 Bitvue 저장소를 감사하지 않은 상태에서 작성되었다.

아래 구조체는 이 카테고리 전체가 전제하는 기준 설계다. "사용자에게 보여줄 메시지"와 "개발자 진단 정보"를 타입 레벨에서 분리하는 것이 이 카테고리에서 다루는 거의 모든 안티패턴의 반례(反例)다.

```rust
struct UiError {
    title: String,
    summary: String,
    recovery: Vec<RecoveryAction>,
    context: ErrorContext,
    technical_details: Option<String>,
}
```

이 구조체는 "사용자 메시지 vs 개발자 진단 정보" 분리라는, 이 카테고리 전체를 관통하는 원칙을 그대로 구현한 것이다.

---

### UIX-ERR-001: 기술 오류 문자열을 그대로 노출

**분류**: 메시지 번역 계층 부재 · **심각도**: Critical · **탐지**: Code

**사용자 목표**:
무엇이 잘못됐는지 이해하고 다음 행동을 결정한다.

**증상**:
- 다이얼로그/toast에 `ParseError { kind: UnexpectedEof, offset: 0x4A2F1, .. }` 같은 Rust `Debug` 포맷 문자열이 그대로 출력된다.
- 스택 트레이스, 타입 이름, 내부 모듈 경로(`bitvue_core::parser::hevc::sps::parse`) 등이 사용자 화면에 노출된다.
- 같은 문제를 겪은 비개발자 사용자가 메시지를 그대로 캡처해 지원 요청을 보낸다.

**원인**:
UI 계층에 "사용자 메시지로 변환"하는 전용 단계가 없어서, backend에서 올라온 에러 값을 `to_string()`이나 `{:?}`로 그대로 문자열화해 화면에 바인딩한다. 개발자에게는 이 문자열이 이미 충분히 유익하기 때문에 별도 변환이 필요하다는 인식 자체가 생기지 않는다.

**구현 냄새**:
- `format!("{:?}", err)` 또는 `err.to_string()`이 dialog title/body에 직접 바인딩됨.
- `UiError`/`ErrorPresenter` 같은 중간 변환 타입이 존재하지 않음.
- 프론트엔드 컴포넌트가 backend 에러 enum의 variant 이름을 조건 없이 그대로 렌더.

**영향**:
비전문 사용자(QA, PM, 고객사, 현업 인코딩 엔지니어)가 메시지를 이해하지 못해 스스로 판단을 내릴 수 없고, 결과적으로 매번 지원 채널에 의존하게 되어 지원 비용이 증가하고 제품 신뢰도가 낮아진다.

**권장**:
- `UiError`처럼 `summary`(사용자용 문장)와 `technical_details`(원문)를 타입으로 분리하고, 화면은 기본적으로 `summary`만 렌더.
- "자세히 보기" 토글/접이식 섹션으로 `technical_details`를 원할 때만 노출.
- backend 에러 enum의 각 variant에 대해 도메인 용어로 쓰인 사용자 메시지를 필수로 매핑(누락 시 빌드/린트 실패하도록 강제하면 더 좋음).

**탐지**:
- 의도적으로 손상시킨 파일을 열어 다이얼로그 텍스트를 비개발자에게 보여주고 이해 여부를 확인하는 사용자 테스트.
- 에러 메시지 문자열에 Rust 타입 이름 패턴(CamelCase::snake_case, `0x` 16진수, `{ .. }` 구조체 리터럴)이 포함되는지 정적 검사.

**Bitvue 판정**: Confirmed — 이 항목은 프론트/백엔드 양쪽에서 확인됨. (1) 프론트: `frontend/hooks/useAppFileOperations.ts:197,242`의 `onError("Failed to Open File", toMessage(err))`가 `err.message`/`String(err)`를 그대로 다이얼로그 메시지로 바인딩. summary/details 분리를 구현한 전용 레이어 `frontend/errors/appError.ts`(`AppError`, `formatErrorForUI` 등)는 여전히 어떤 컴포넌트에서도 import되지 않는 죽은 코드(grep 결과 소비자 0곳). (2) 백엔드(더 심각): `crates/bitvue-sidecar/src/main.rs:1878-1880`의 `event_to_json`이 `Event::DiagnosticAdded`를 `format!("{diagnostic:?}")`로 직렬화 — `crates/bitvue-engine/src/event.rs`의 `Diagnostic` 구조체(severity/category/offset_bytes/frame_index/count/impact_score 필드 보유)를 Rust `Debug` 포맷 문자열 하나로 뭉개서 wire로 보냄. `frontend/services/electronBridgeService.ts:544-549`의 `openStream`이 이 문자열을 그대로 `error` 필드에 담아 UI까지 전달 — 이 카탈로그가 묘사한 정확한 패턴이 src-tauri 대신 새 sidecar 아키텍처에서 재현됨(`docs/anti-patterns/*`가 구 `src-tauri/src/commands/file.rs` 기준으로 작성됐던 원래 인용은 스테일 — 2026-08-08/11 Electron 마이그레이션으로 `src-tauri` 자체가 저장소에서 삭제됨, `git log -- src-tauri` 커밋 `e7194cc` 참고).

---

### UIX-ERR-002: 무엇을 열다가 실패했는지 표시하지 않음

**분류**: 에러 컨텍스트 소실 · **심각도**: High · **탐지**: Interaction

**사용자 목표**:
실패한 대상(파일/스트림/트랙)이 무엇인지 즉시 알고 재시도 여부를 판단한다.

**증상**:
- "파일을 열 수 없습니다"라는 문구만 뜨고 실제 파일명이나 경로가 어디에도 없다.
- 여러 파일을 동시에 열거나 배치 비교를 시작했을 때, 전체 중 어느 항목이 실패했는지 목록에서 구분되지 않는다.
- 동일한 제네릭 오류 문구가 완전히 다른 상황(파일 열기, 트랙 디코드, 프레임 seek)에서 재사용된다.

**원인**:
파일 경로·작업 종류 같은 컨텍스트가 호출 스택 상위로 전파되는 과정에서 소실된다. 공통 catch-all 에러 핸들러가 "무엇을 하다가"라는 정보 없이 결과만 성공/실패로 뭉갠다.

**구현 냄새**:
- 에러 핸들러 함수가 파라미터 없이 고정 문자열을 반환.
- 배치 로드 로직이 개별 항목의 성공/실패를 집계하지 않고 전체를 하나의 boolean으로 축약.
- 에러 처리 코드에 파일 경로 변수가 스코프 안에 있는데도 메시지 조립 시 사용되지 않음.

**영향**:
다중 파일 비교·배치 분석 워크플로우에서 사용자가 실패 대상을 찾기 위해 각 항목을 하나씩 수동으로 재확인해야 하며, 대용량 배치일수록 비용이 커진다.

**권장**:
- `ErrorContext`에 실패 대상(파일 경로, URL, 트랙 인덱스, 작업 종류)을 필수 필드로 강제.
- 배치 작업은 항목별 성공/실패 상태를 리스트로 표시하고, 실패 항목만 필터링해 볼 수 있게 함.

**탐지**:
- 5개 파일 중 3번째만 손상된 세트로 다중 열기를 시도해, 에러 메시지가 정확히 그 파일명을 포함하는지 확인.

**Bitvue 판정**: Confirmed — `frontend/hooks/useAppFileOperations.ts:310-313` `handleOpenDependentFile`의 catch에서 `setOpenError(toMessage(err))`가 실패한 파일(pathB)을 메시지에 전혀 포함하지 않고, `App.tsx:672`에서 `WelcomeScreen`의 일반 `error` prop으로만 렌더(경로 정보 없음). 단일 파일 open 경로(`openFileAtPath`, line 189-193)는 `onError("Failed to Open File", result.error, selected)`로 path를 전달하긴 하나 `showErrorDialog`의 3번째 인자(`details`, 접이식 섹션)로만 들어가고 주 메시지엔 없음. 단, `handleOpenDependentFile` 자체는 `createWorkspace` 호출 대상 sidecar 커맨드(`create_compare_workspace`)가 현재 저장소에 미구현(`crates/bitvue-sidecar/src/main.rs`의 유일한 매칭 결과가 "unknown method returns Internal error" 테스트) — 이 경로는 사실상 항상 실패하는 죽은 기능. Bitvue는 진짜 "배치" 오픈 기능은 없어 원래 시나리오(대량 배치)는 해당 없음, 대신 2-스트림 비교의 동일 패턴으로 확인.

---

### UIX-ERR-003: frame·offset·track 정보를 숨김

**분류**: 진단 위치 정보 누락 · **심각도**: High · **탐지**: Domain review

**사용자 목표**:
손상 위치를 정확히 파악해 원인 분석이나 재인코딩 등 다음 조치를 취한다.

**증상**:
- "비트스트림 파싱 오류가 발생했습니다"라고만 뜨고 frame index, byte offset, NAL unit 타입, track ID 등이 어디에도 보이지 않는다.
- 로그 파일을 뒤져야만 실제 위치 정보를 알 수 있다.

**원인**:
파서 자체는 offset/frame 정보를 정확히 갖고 있지만, 에러가 UI 레이어로 매핑되는 과정에서 요약 문장(summary)만 사용되고 구조화된 위치 컨텍스트는 버려진다. "사용자에게는 자세한 정보가 필요 없다"는 암묵적 가정이 실제로는 정확히 이 도메인(비트스트림 분석 툴)의 핵심 사용자층에게는 틀린 가정이다.

**구현 냄새**:
- `ParseError`에 `offset`/`frame_index`/`nal_type` 필드가 존재하는데도 UI 컴포넌트는 `err.summary`만 렌더.
- 위치 정보가 로그에는 기록되지만 프론트엔드로 전달되는 IPC 페이로드에는 포함되지 않음.

**영향**:
인코더 개발자·QA·코덱 엔지니어가 어느 프레임/바이트를 재확인해야 하는지 알 수 없어, 로그를 뒤지는 별도 디버깅 세션이 필요해지고 진단 시간이 크게 늘어난다.

**권장**:
- context를 항상 화면에 노출한다 — 예: "Frame 142, offset 0x1A2F, NAL type 5 (IDR)".
- 위치 정보를 클릭 가능하게 만들어 해당 프레임/오프셋으로 즉시 이동할 수 있게 한다(UIX-ERR-009와 연결).

**탐지**:
- 알려진 오프셋에 바이트 손상을 주입한 테스트 파일을 열어, 에러 다이얼로그에 그 오프셋/프레임 번호가 실제로 나타나는지 확인.

**Bitvue 판정**: Confirmed — (구 인용 `src-tauri/src/commands/file.rs`는 스테일: `src-tauri`는 Electron 마이그레이션으로 저장소에서 삭제됨, 아래는 현재 sidecar 기준 재확인) `crates/bitvue-engine/src/event.rs:70-86`의 `Diagnostic`이 `offset_bytes`(MANDATORY 주석), `frame_index`, `category`, `severity`, `count`, `impact_score`를 전부 갖고 있지만, `crates/bitvue-sidecar/src/main.rs:1878-1880`의 `event_to_json`이 이를 `format!("{diagnostic:?}")` 통짜 문자열로만 wire에 실음 — 구조화된 필드 단위 전송이 아예 없음. 이 구조화 Diagnostic을 필드 단위(JSON)로 그대로 노출하는 sidecar 커맨드는 존재하지 않음(grep 결과 전무), 프론트도 이 문자열에서 offset/frame을 파싱해 쓰는 코드가 없음.

---

### UIX-ERR-004: 복구 가능한지 알려주지 않음

**분류**: 복구 경로 미제공 · **심각도**: High · **탐지**: Interaction

**사용자 목표**:
이 파일을 계속 쓸 수 있는지, 문제를 무시하고 진행해도 되는지 판단한다.

**증상**:
- fatal 오류와 부분 실패가 동일한 UI로 표시되며 "다시 시도" 외에 다른 선택지가 없다.
- 실제로는 파일의 90%가 정상 파싱됐는데도 "파싱 실패"로만 보고되고 그 부분 결과에 접근할 방법이 없다.

**원인**:
에러를 성공/실패 두 값으로만 모델링하는 얕은 타입(`Result<T, String>` 등)을 사용해, "복구 가능한가", "부분 결과가 있는가" 같은 중간 상태를 표현할 자리가 애초에 없다.

**구현 냄새**:
- 에러 타입에 `recoverable: bool`이나 이에 준하는 필드가 없음.
- 부분적으로 파싱된 결과를 담을 자료구조가 없어 실패 시 이미 계산된 값도 함께 버려짐.

**영향**:
사용자가 실제로는 활용 가능한 부분 결과가 있는 경우에도 파일 전체를 포기하게 되어, 특히 대용량 파일에서 재작업 비용이 커진다.

**권장**:
- `recovery: Vec<RecoveryAction>` 필드로 "부분 결과 보기", "해당 프레임 건너뛰고 계속", "엄격 모드를 끄고 재시도" 등 구체적 행동을 명시적으로 제공.
- 각 RecoveryAction은 실제로 다른 코드 경로를 실행해야 하며, 단순히 동일 작업 재실행이어서는 안 된다(UIX-ERR-014 참고).

**탐지**:
- 일부 프레임만 손상된 파일로 "부분 로드 후 계속 진행" 경로가 실제로 동작하는지 시나리오 테스트.

**Bitvue 판정**: Confirmed — (구 인용 `src-tauri/src/error.rs`는 스테일, `src-tauri` 삭제됨) 현재 wire 계약인 `crates/bitvue-protocol/src/lib.rs:116-147`의 `WireError { code, message, offset }`/`WireErrorCode`(Io/Parse/UnexpectedEof/... /Internal)에 `recoverable` 필드나 부분 결과를 담는 타입이 없음. `openStream`(`electronBridgeService.ts:539-551`)도 `success: boolean` + `error?: string` 이진 모델뿐. `Diagnostic`의 `category` 주석(`crates/bitvue-engine/src/diagnostics.rs:25` "recoverable parsing issues")은 분류 텍스트일 뿐 실제 `recoverable: bool` 필드나 이를 소비하는 UI 분기는 없음.

---

### UIX-ERR-005: 오류 후 전체 프로젝트를 닫음

**분류**: 에러 격리 실패 · **심각도**: Critical · **탐지**: Interaction

**사용자 목표**:
세션 내 다른 파일·북마크·비교 작업 상태를 잃지 않고 이 파일 하나만 재확인한다.

**증상**:
- 손상된 파일 하나를 열었을 뿐인데 앱이 초기 화면으로 리셋되거나 완전히 크래시한다.
- 이미 열어둔 다른 탭/비교 세션의 미저장 상태(북마크, 주석, 필터)가 함께 사라진다.

**원인**:
error boundary가 애플리케이션 루트 한 곳에만 있고 파일/탭/패널 단위로 세분화되지 않았다. Rust 쪽에서는 워커 스레드 격리나 `catch_unwind` 경계 없이 파싱이 메인 프로세스에서 직접 실행되어, panic이 프로세스 전체를 데려간다.

**구현 냄새**:
- 프론트엔드: 전역 `<ErrorBoundary>` 하나가 전체 렌더 트리를 감쌈.
- 백엔드: `panic = "unwind"` 환경인데도 워커 스레드/`catch_unwind` 경계 없이 파싱 함수가 메인 스레드에서 직접 호출됨(Phase 1 `ERR.md`의 panic 전략 논의와 직결).

**영향**:
대형 프로젝트나 다중 탭 비교 세션에서 손상 파일 하나가 전체 작업을 무효화시켜, 신뢰할 수 없는 입력을 다루는 것이 본업인 도구로서는 치명적인 결함이 된다.

**권장**:
- 파일/탭/패널 단위 error boundary를 두어 실패가 해당 단위에만 국한되게 한다.
- 파싱은 워커 스레드 + `catch_unwind`로 격리해 backend panic이 있어도 IPC 계층에서 정상적인 `Err` 응답으로 변환되게 한다.

**탐지**:
- 정상 파일 3개를 열어둔 상태에서 4번째로 손상 파일을 열어, 나머지 3개 탭이 그대로 살아있는지 확인.

**Bitvue 판정**: Confirmed(카탈로그가 묘사한 것보다 더 심함) — 두 겹으로 확인됨. (1) `frontend/App.tsx:723,753` `<ErrorBoundary>` 하나가 앱 전체 `<div className="app">` 트리를 통째로 감쌈(파일/탭/패널 단위 분리 없음). (2) 더 심각한 백엔드 계층: `crates/bitvue-sidecar`(Rust)는 워커 스레드 격리나 `catch_unwind` 없이 실행되고(grep 결과 `catch_unwind`는 무관한 `compatibility_32bit_test.rs`에만 존재), sidecar 프로세스가 죽으면 `bitvue-desktop/electron/main.ts:1275-1308`가 이를 감지해 **프로세스 자체를 재시작**하되 주석이 명시하듯("application state was lost") `bitvue_engine::Core`의 인메모리 상태(열린 스트림/선택 상태 전부)가 통째로 사라짐. 이 이벤트는 `bitvue:sidecar-restarted` IPC로 렌더러까지 실제로 전달되고(`preload.cjs:99-103`, `electronBridgeService.ts:519` `onSidecarRestarted`) 배선 자체는 완결돼 있지만, **정작 소비자가 없음** — 실제 앱 코드(App.tsx 등) 어디서도 `onSidecarRestarted`를 호출하지 않고 테스트 목(`electronBridgeService.test.ts:66`)에만 존재(grep 결과 전무). 즉 손상 파일 하나가 sidecar를 크래시시키면 세션 전체가 조용히(사용자 통보 없이) 리셋됨 — 카탈로그가 말한 "에러 후 전체 프로젝트를 닫음"이 실제로 일어나는데, 그 사실조차 UI에 드러나지 않아 사용자는 원인을 알 길이 없음.

---

### UIX-ERR-006: 동일 오류 toast를 수백 번 표시

**분류**: 알림 폭주 · **심각도**: High · **탐지**: Performance

**사용자 목표**:
반복되는 동일 종류 오류를 한 번의 정보로 인지하고 나머지 작업을 계속한다.

**증상**:
- 프레임마다/패킷마다 발생하는 동일한 오류가 각각 개별 toast로 화면에 쌓여 UI를 뒤덮는다.
- toast 렌더 자체가 CPU를 잡아먹어 앱이 멈춘 것처럼 보인다.

**원인**:
에러 발생 지점에서 dedup이나 throttle 없이, 이벤트가 발생할 때마다 즉시 UI 알림을 호출한다. "에러 하나 = 알림 하나"라는 1:1 모델이 반복성 손상(예: 매 GOP마다 동일 필드 손상)에서 무너진다.

**구현 냄새**:
- 파싱 루프 내부에서 `emit_error()`를 직접 호출.
- rate limiting이나 알림 집계 레이어가 존재하지 않음.

**영향**:
UI 반응성이 저하되고, 진짜 유의미한 다른 오류가 반복 toast 더미 속에 묻혀 사용자가 놓친다.

**권장**:
- 동일 오류 유형은 카운트와 함께 하나로 집계("동일 오류 237회 발생 — 자세히 보기").
- rate-limited 알림 정책을 두고, 반복 오류는 별도 요약 패널로 유도.

**탐지**:
- 반복 손상 패턴(예: 매 GOP마다 동일 필드 손상)을 가진 파일로 실제 toast 개수와 UI 반응성을 측정.

**Bitvue 판정**: N/A — `showErrorDialog`/`onError`를 프레임·패킷 루프 안에서 호출하는 코드가 없음(grep 결과 호출부는 `App.tsx`/`useAppFileOperations.ts`/`useAppDialogs.ts` 등 전부 단발성 이벤트, 루프 내부 호출 없음). `ErrorToast` 컴포넌트(`ErrorDialog.tsx:140`)는 여전히 정의만 되어 있고 어디서도 import되지 않는 죽은 코드(grep 결과 소비자 0곳). `useErrorDialog.ts`도 배열이 아닌 단일 상태 객체라 여러 에러가 쌓이는 구조 자체가 없음(마지막 에러가 이전 것을 덮어쓰는 별개 문제는 있음). 흥미롭게도 백엔드 `Diagnostic` 구조체(`crates/bitvue-engine/src/event.rs:83`)엔 반복 감지용 `count: u32` 필드가 이미 있으나, 현재 wire 직렬화(UIX-ERR-001 참고)가 통짜 Debug 문자열이라 이 필드도 UI에 도달하지 않음 — dedup 인프라 절반만 존재.

---

### UIX-ERR-007: warning과 fatal error의 시각적 차이가 없음

**분류**: 심각도 신호 부재 · **심각도**: High · **탐지**: Visual

**사용자 목표**:
무시해도 되는 문제와 즉시 대응해야 하는 문제를 한눈에 구분한다.

**증상**:
- 경고(warning)와 치명적 오류(fatal)가 동일한 아이콘·색상·toast 스타일로 표시된다.
- 사용자가 두 종류를 구분하려면 텍스트를 끝까지 읽어야 한다.

**원인**:
단일 알림 컴포넌트가 `severity` 파라미터 없이 하드코딩된 스타일 하나만 사용한다. "알림이 필요하다"는 인식은 있었지만 "심각도별로 달라야 한다"는 설계 요구가 반영되지 않았다.

**구현 냄새**:
- Toast 컴포넌트가 `message: string`만 받고 `level`/`severity` 인자가 없음.
- 색상·아이콘이 컴포넌트 내부에 상수로 고정.

**영향**:
사용자가 사소한 경고에도 작업을 중단하거나(과잉 반응), 반대로 치명 오류를 무시하고 잘못된 분석 결과를 신뢰하는(과소 반응) 양쪽 실패 모드가 모두 발생한다.

**권장**:
- 심각도별로 색·아이콘·지속성 정책을 분리한다: warning은 조용히 로그 패널에 누적, fatal은 모달 + 지속 배너.
- 심각도 체계(예: info/warning/error/fatal)를 문서화해 팀 전체가 일관되게 사용.

**탐지**:
- 동일 화면에서 warning 5건과 fatal 1건을 동시에 발생시켜 fatal이 즉시 시각적으로 구별되는지 확인.

**Bitvue 판정**: Confirmed — `frontend/hooks/useErrorDialog.ts`의 `ErrorDialogState`에 severity 필드 자체가 없고, `ErrorDialog` 컴포넌트는 원인에 관계없이 항상 동일한 빨간 error 아이콘만 렌더(`ErrorDialog.tsx:51-52`). 이건 실제 file-open 에러(`showErrorDialog`, App.tsx:334)가 지나가는 주 경로. `DiagnosticsPanel.tsx`(line 169-187, `getSeverityIcon`/`getSeverityColor`)는 severity별 아이콘·색을 실제로 구분하는 코드를 갖고 있지만, UIX-ERR-008에서 확인했듯 실제 backend `Diagnostic`(severity 필드 보유)이 연결되지 않고 mock 데이터 위주로 채워져 있어 이 severity 구분 UI 자체가 실질적으로 도달 불가 — "구현은 있지만 실데이터가 없다"는 점에서 오히려 더 근본적인 배선 문제.

---

### UIX-ERR-008: parser warning을 발견할 경로가 없음

**분류**: 경고 가시성 부재 · **심각도**: Critical · **탐지**: Domain review

**사용자 목표**:
파일이 스펙을 완전히 준수하지 않더라도 그 사실을 인지한 상태에서 분석 결과를 신뢰할지 판단한다.

**증상**:
- 파싱은 "성공"으로 끝나지만, 내부적으로는 비표준 필드나 추정치(fallback)를 사용했다는 사실이 어디에도 노출되지 않는다.
- warning이 로그 파일에만 기록되고 UI에는 집계 뷰조차 없다.

**원인**:
"성공/실패"라는 이진 상태만 사용자에게 전달되도록 설계되어, "성공했지만 완전히 스펙을 따르지는 않았다"는 중간 상태를 표현할 자리가 없다.

**구현 냄새**:
- 파서가 `Vec<Warning>`을 반환하지만 호출부에서 버려짐(`let _ = warnings;`).
- UI에 warning 배지나 전용 패널이 존재하지 않음.

**영향**:
사용자가 실제로는 비표준/추정 기반 결과를 스펙 준수 결과로 오인하게 되며, 이는 QA·컴플라이언스 검증 워크플로우에서 특히 치명적이다(잘못된 "통과" 판정으로 이어질 수 있음).

**권장**:
- 상태바나 파일 트리에 "N warnings" 배지를 상시 표시.
- warning 전용 패널(필터/정렬 가능)을 별도 탭으로 제공해 언제든 전체 목록을 확인할 수 있게 함.

**탐지**:
- 표준 위반이지만 대부분의 디코더가 관대하게 처리하는 파일(예: 잘못된 profile_idc)을 열어, UI 어디에도 경고가 노출되지 않는지 확인.

**Bitvue 판정**: Confirmed — (구 인용 `src-tauri/src/commands/file.rs`는 스테일, 삭제됨) 현재 sidecar 기준 재확인, 결론은 유지: `electronBridgeService.ts:539-551`의 `openStream`은 `events.find((e) => e.type === "DiagnosticAdded")`로 **첫 번째** DiagnosticAdded 이벤트 유무만으로 `success`를 이진 판정 — severity가 Warn/Info인 진단이 성공 경로에 섞여 나와도 이를 구분해 사용자에게 알리는 코드가 없음(severity 자체가 UIX-ERR-001에서 확인했듯 Debug 문자열에 묻혀 wire를 넘지도 못함). `DiagnosticsPanel.tsx`(line 40-105)도 실제 backend diagnostic을 구독하는 코드가 없음 — `propDiagnostics`를 넘기는 호출부가 저장소 전체에 0곳(`App.tsx:151`이 `<DiagnosticsPanel />`을 props 없이 렌더), 대신 `useFileState().error`(단일 문자열)를 "STREAM_ERROR" 한 건으로 감싸거나(line 56-66), 그마저 없으면 프레임 크기/참조 유무 기반 **가짜 mock 진단**(line 68-102, "Add mock diagnostics for demonstration")을 생성 — 실제 파서 warning은 UI 어디에도 도달하지 않음.

---

### UIX-ERR-009: 오류 메시지에서 관련 위치로 이동 불가

**분류**: 진단 내비게이션 부재 · **심각도**: Medium · **탐지**: Interaction

**사용자 목표**:
오류를 본 즉시 해당 프레임/노드/바이트로 이동해 맥락을 확인한다.

**증상**:
- 에러 다이얼로그 텍스트는 "Frame 142에서 실패"라고 언급하지만 클릭 가능한 링크나 버튼이 없다.
- 사용자가 타임라인/필름스트립을 수동으로 스크럽해 142번 프레임을 직접 찾아야 한다.

**원인**:
UI 에러 모델이 컨텍스트를 문자열 보간에만 사용하고, 네비게이션 가능한 참조(프레임 인덱스, 트리 노드 ID, 바이트 오프셋)로는 다루지 않는다. 정보는 있지만 "행동 가능한 정보"로 설계되지 않았다.

**구현 냄새**:
- 에러 메시지가 템플릿 문자열로만 조립됨(`format!("Frame {} failed", idx)`), 정작 `idx` 값이 클릭 핸들러로는 전달되지 않음.
- `ErrorContext`가 순수 문자열 필드만 갖고 구조화된 참조 타입(예: `FrameRef`, `NodeRef`)을 갖지 않음.

**영향**:
텍스트로만 존재하는 정보를 사용자가 다시 수동으로 찾아가야 해 진단 시간이 불필요하게 늘어나고, 이는 특히 대용량 파일에서 체감 비용이 크다.

**권장**:
- `ErrorContext`에 navigable reference(frame_index, node_id, byte_offset)를 구조화된 필드로 포함.
- 메시지 안의 해당 부분을 클릭 가능한 링크/버튼으로 렌더해, 클릭 시 필름스트립·헥스뷰·신택스 트리가 해당 위치로 스크롤/포커스되게 함.

**탐지**:
- 에러 다이얼로그의 "해당 위치로 이동" 클릭이 실제로 필름스트립/헥스뷰를 그 프레임/오프셋으로 이동시키는지 확인.

**Bitvue 판정**: Confirmed — `frontend/components/panels/DiagnosticsPanel.tsx`: 진단 행 클릭은 `handleSelectDiagnostic`(line 131, onClick line 258)만 호출해 로컬 상세 패널을 열 뿐, frame index는 일반 텍스트/`<span>`으로만 표시되고(line 271-275 테이블 행, line 318-324 상세 패널) `setCurrentFrameIndex` 등 내비게이션 호출이 전혀 없음(해당 컴포넌트에서 `useCurrentFrame`은 읽기 전용 `currentFrameIndex`만 구독, setter는 import조차 안 함). `ErrorDialog`에도 위치 이동 어포던스가 없음. (참고로 UIX-ERR-008에서 확인했듯 실제 진단 데이터 자체가 대부분 mock이라, 이 항목은 실데이터가 흐르게 되더라도 여전히 막힐 자리.)
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-008 참고.

---

### UIX-ERR-010: 복사 가능한 상세 정보가 없음

**분류**: 진단 정보 전달 마찰 · **심각도**: Medium · **탐지**: Interaction

**사용자 목표**:
오류를 팀·버그 리포트·이슈 트래커에 정확하게 전달한다.

**증상**:
- 에러 다이얼로그 텍스트를 마우스로 드래그 선택할 수 없다.
- "복사" 버튼이 없어 사용자가 스크린샷으로만 오류를 공유할 수 있다.

**원인**:
에러 UI가 이미지/캔버스 기반으로 렌더되거나, 전역 CSS(`user-select: none`)가 텍스트 선택을 막는다. 설령 선택 가능해도 구조화된 `technical_details` 필드가 없어 복사해봐야 요약 문장 한 줄만 담긴다.

**구현 냄새**:
- 에러 모달이 `<canvas>`로 텍스트를 렌더.
- 전역 스타일시트가 모든 텍스트에 `user-select: none`을 적용해 예외 처리가 안 됨.

**영향**:
버그 리포트 품질이 낮아지고 재현에 필요한 offset·스택트레이스·버전 정보가 유실되어, 지원팀과 사용자 간 왕복 횟수가 늘어난다.

**권장**:
- "상세 정보 복사" 버튼으로 `technical_details`(스택, offset, 파일 해시, 앱/코덱 버전)를 구조화된 텍스트나 JSON으로 클립보드에 복사.
- 최소한 에러 다이얼로그 텍스트는 항상 선택 가능하게 유지.

**탐지**:
- 에러 발생 시 Cmd+C로 클립보드에 실제 유의미한 진단 텍스트가 들어가는지 확인.

**Bitvue 판정**: Confirmed(부분) — 재확인 결과 동일: `ErrorDialog.tsx`에 "상세 정보 복사" 버튼이 없음(footer는 Dismiss/Close 뿐, `ErrorDialog.tsx:75-90`). 다만 canvas 렌더링이나 전역 `user-select: none`은 아님(`ErrorDialog.css`에는 summary/button류에만 `user-select: none`, 본문 텍스트는 선택 가능) — 카탈로그가 말하는 최악의 형태(완전 선택 불가)까지는 아니고 "복사 버튼 부재"만 해당. 참고로 `DiagnosticsPanel.tsx:156-167`의 컨텍스트 메뉴엔 `Copy.Selection`(코드+메시지를 클립보드로)이 실제 구현돼 있어 — 진단 패널 쪽엔 이 안티패턴이 없지만 그 패널의 데이터 자체가 UIX-ERR-008 확인대로 대부분 mock이라 실효는 제한적.

---

### UIX-ERR-011: 사용자 취소를 오류 toast로 표시

**분류**: 상태 분류 오류 · **심각도**: Medium · **탐지**: Interaction

**사용자 목표**:
스스로 취소한 작업이 오류로 오인되지 않기를 원한다.

**증상**:
- 긴 분석 작업 중 "취소" 버튼을 눌렀는데 빨간 오류 toast("작업이 실패했습니다")가 뜬다.
- 취소와 실제 실패가 동일한 UI 언어와 색상으로 표현된다.

**원인**:
취소가 내부적으로 `Err(Cancelled)`처럼 동일한 에러 채널을 타고 올라오며, UI가 에러의 "종류"를 구분하지 않고 모든 `Err`를 동일한 실패 경로로 렌더한다.

**구현 냄새**:
- `CancellationToken` 발생 시 `Err` variant로 처리되고, catch 블록이 모든 `Err`를 동일한 `showErrorToast()`로 전달.
- `Cancelled`가 별도 상태 타입이 아니라 에러 enum의 variant 중 하나로 존재.

**영향**:
사용자가 의도적으로 취소했는데 "뭔가 잘못됐나?" 하는 불필요한 불안을 느끼고, 반복되면 실제 오류에 대한 민감도가 떨어지는 양치기 소년 효과가 생긴다.

**권장**:
- `Cancelled`를 명시적인 별도 상태로 분리해 에러 채널과 구분하고, 중립적인 톤(또는 무알림)으로 처리.
- 취소 확인은 진행 표시줄이 조용히 사라지는 정도로 충분하며, 별도 toast가 반드시 필요하지 않을 수 있음.

**탐지**:
- 장시간 파싱/분석 작업을 시작 직후 취소해, UI에 오류 색상 알림이 뜨지 않는지 확인.

**Bitvue 판정**: N/A(단, 재확인 결과 뉘앙스 있음) — 프로토콜 레벨의 취소 인프라는 실제로 존재함: `crates/bitvue-sidecar/src/main.rs:90-137`의 `cancel_request` 메서드가 요청별 `Arc<AtomicBool>` 레지스트리로 워커 스레드 취소를 지원하고, 완료 시 `WireErrorCode::Cancelled`(`bitvue-protocol/src/lib.rs:144`)라는 에러와 별개인 전용 코드까지 갖고 있음 — 즉 백엔드는 이미 이 항목의 "권장" 사항(Cancelled를 별도 채널로 분리)을 구현해 둔 상태. 하지만 이를 호출하는 프론트엔드 코드가 전혀 없음(grep 결과 `cancelRequest`/`cancel_request` 프론트/electron 쪽 소비자 0곳, `electronBridgeService.ts`에 래퍼조차 없음) — 사용자가 취소할 수 있는 "취소" 버튼이 UI 어디에도 없어(`useAppFileOperations.ts`의 "User cancelled"는 파일 선택 다이얼로그 취소일 뿐, 장시간 작업 취소 아님) 카탈로그가 묘사하는 실패 시나리오(취소를 오류로 표시) 자체가 발생할 수 없음. 즉 백엔드 인프라는 있으나 미배선이라 여전히 N/A.

---

### UIX-ERR-012: backend panic과 입력 오류를 동일하게 표시

**분류**: 원인 귀속(attribution) 부재 · **심각도**: Critical · **탐지**: Domain review

**사용자 목표**:
"내 파일이 이상한 것"과 "앱에 버그가 있는 것"을 구분해 대응 방법(파일 재확인 vs 버그 신고)을 다르게 선택한다.

**증상**:
- 잘못된 사용자 파일로 인한 정상적인 파싱 실패와, backend 내부 버그로 인한 panic이 동일한 "오류가 발생했습니다" 메시지로 뜬다.
- 두 경우 모두 동일한 CTA("다시 시도")만 제공된다.

**원인**:
`catch_unwind`로 잡은 panic과 정상 `Result::Err` 경로가 동일한 `UiError` 변환 함수를 거치면서, "이것이 panic이었다"는 구분 정보가 소실된다.

**구현 냄새**:
- panic hook이 잡은 메시지를 그대로 `ParseError::Other(String)` 같은 캐치올 variant에 욱여넣어, 이후 파이프라인에 둘을 구별할 typed 정보가 없음.

**영향**:
사용자가 재현 가능한 앱 버그를 "내 파일이 나쁨"으로 오인해 리포트하지 않거나, 반대로 실제 손상 파일을 "앱 버그"로 오인해 잘못된 채널로 문의하게 되어 양쪽 모두 잘못된 대응으로 이어진다.

**권장**:
- panic과 input error를 타입 레벨에서 분리한다(예: `UiError::Internal` vs `UiError::InputInvalid`).
- `Internal`은 "버그 신고" CTA와 함께 다른 톤/색으로 표시하고 자동으로 진단 번들을 첨부할 수 있게 함.

**탐지**:
- 의도적으로 panic을 유발하는 fuzz 케이스와 단순 truncated 파일을 각각 열어, 두 메시지의 title/CTA가 서로 다른지 비교.

**Bitvue 판정**: Confirmed(카탈로그가 묘사한 것보다 더 심함) — (구 인용 `src-tauri`는 스테일, 삭제됨) 현재 `crates/bitvue-sidecar`에도 `panic::set_hook`/`catch_unwind`가 전혀 없음(grep 결과 무관한 크레이트 테스트에만 존재). `WireErrorCode`(`bitvue-protocol/src/lib.rs:128-147`)에도 panic 전용 variant가 없어, `Internal`이 "엔진 에러 중 알려진 코드에 안 맞는 것들의 폴백"으로 진짜 panic과 평범한 미분류 에러를 똑같이 담음 — 구분 메커니즘 부재는 카탈로그 묘사와 동일. 다만 실제로는 이보다 심각함: `catch_unwind`가 없으므로 진짜 panic은 `WireError`로조차 변환되지 않고 **sidecar 프로세스 자체를 죽임** — UIX-ERR-005에서 확인한 대로 `bitvue-desktop/electron/main.ts`가 이를 감지해 프로세스를 재시작하지만 그 사실을 알리는 `bitvue:sidecar-restarted` 이벤트를 실제로 구독하는 프론트 코드가 없어(grep 결과 테스트 목 외 0곳), 사용자 입장에선 panic이 "버그 신고" CTA는커녕 아무 신호도 없이 이후 명령들이 알 수 없는 이유로 실패하는 것처럼만 보임 — attribution 부재가 아니라 attribution 자체가 완전히 침묵함.

---

### UIX-ERR-013: 일부 지원 codec을 완전 미지원처럼 표시

**분류**: 기능 지원 범위 표현 부재 · **심각도**: High · **탐지**: Domain review

**사용자 목표**:
"이 codec/profile을 아예 못 여는지, 일부 기능만 제한되는지"를 정확히 파악한다.

**증상**:
- 지원 profile 범위 밖의 파일(예: HEVC Main 10 이외 profile, AV1의 특정 tool 조합)을 열면 "지원하지 않는 형식입니다"만 뜨지만, 실제로는 헤더 파싱까지는 성공했고 특정 오버레이/필드 계산만 불가능한 상태다.
- "이 codec 지원한다더니 실제로는 안 되네"라는 인상을 준다.

**원인**:
partial/degraded support 경로가 별도로 모델링되지 않고, 첫 unsupported feature를 만나는 순간 전체를 실패로 취급하는 all-or-nothing 처리 방식을 쓴다.

**구현 냄새**:
- `match codec_profile { Supported(p) => ..., _ => return Err(Unsupported) }` 형태로 profile별 세분화 없이 이진 분기.
- 기능별(신택스 트리, MV 오버레이, QP 히트맵 등) 지원 여부를 별도로 추적하는 capability 모델이 없음.

**영향**:
실제로는 부분 분석(예: 신택스 트리는 보이지만 MV 오버레이만 계산 불가)이 가능한데도 사용자가 파일 전체를 포기하게 되고, "지원 codec" 목록에 대한 신뢰가 떨어진다.

**권장**:
- capability matrix를 UI에 노출: "이 파일 — 신택스 트리 지원, MV 오버레이 미지원 (이유: profile RExt 확장 미구현)".
- 기능별 degraded 상태를 개별 배지로 표시해 전체 미지원과 부분 미지원을 시각적으로 구분.

**탐지**:
- 지원 범위 경계에 있는 profile(예: HEVC RExt) 파일을 열어, 정확히 어떤 기능이 되고 안 되는지 메시지가 구분하는지 확인.

**Bitvue 판정**: Confirmed — (`bitvue-core`는 커밋 `c2a0e44`로 `bitvue-engine`로 개명됨, 구 인용 경로 스테일) 정확히 이 권장안을 구현한 `crates/bitvue-engine/src/disable_reason.rs`(`DisableReason` enum: MissingDependency/UnsupportedCodecFeature/InsufficientData 등 + `FeatureId` enum)가 존재하지만, 유일한 실제 소비자는 `crates/bitvue-engine/src/compare.rs`(compare-workspace 기능 게이팅)이고, 그 결과를 프론트가 받는 유일한 경로는 `frontend/components/CompareWorkspace/CompareWorkspace.tsx:168`의 `{workspace.disable_reason}`(단순 문자열 렌더)뿐인데 — 이 컴포넌트는 `App.tsx`에 마운트되지 않는 죽은 트리(grep 결과 import 0곳)이고, 그 배후 sidecar 커맨드 `create_compare_workspace`도 현재 `crates/bitvue-sidecar/src/main.rs`엔 존재하지 않음(유일한 매칭이 "unknown method → Internal error" 단위테스트). 즉 이 항목이 유일하게 구현한 degraded-support 모델은 사실상 이중으로 도달 불가능. 실제 코덱 지원 안내는 여전히 `useAppFileOperations.ts:38-41`의 `UNSUPPORTED_CODEC_EXTENSIONS` 하드코딩 맵(vvc/h266 2개 확장자만) 뿐이라, 그 외 코덱의 profile/feature 단위 부분 지원은 전혀 구분되지 않음.

---

### UIX-ERR-014: 재시도 버튼이 동일 실패를 그대로 반복

**분류**: 무의미한 복구 어포던스 · **심각도**: Medium · **탐지**: Interaction

**사용자 목표**:
"다시 시도"가 실제로 상황을 바꿀 가능성이 있을 때만 의미 있게 동작하기를 기대한다.

**증상**:
- 손상된 파일 파싱 실패 후 "다시 시도"를 클릭하면 동일한 입력으로 동일한 파싱 함수가 그대로 재실행되어 동일한 오류가 즉시 재현된다.
- 사용자가 여러 번 클릭해도 진행이 전혀 없다.

**원인**:
재시도 로직이 "같은 함수를 같은 인자로 다시 호출"하는 것 그 자체이며, 재시도가 의미를 가지려면 필요한 조건 변화(락 해제 대기, 네트워크 재연결, 더 관대한 파싱 모드로 전환 등)를 고려하지 않았다.

**구현 냄새**:
- `onRetry={() => sameParseCall(sameArgs)}` 형태로 재시도와 최초 호출이 코드상 완전히 동일.
- 에러 종류(일시적 vs 결정론적)를 구분하지 않고 모든 에러에 동일한 재시도 버튼을 노출.

**영향**:
사용자가 결정론적으로 실패할 작업을 반복 클릭하며 시간을 낭비하고, "다시 시도"라는 어포던스 자체에 대한 신뢰를 잃는다.

**권장**:
- 재시도가 실제로 의미 있는 에러 종류(일시적 IO 오류, lock 경합 등)에만 버튼을 노출.
- 결정론적 파싱 실패에는 "다시 시도" 대신 "관대한 모드로 재파싱", "이 구간 건너뛰고 계속" 등 실제로 다른 코드 경로를 타는 행동을 제공.

**탐지**:
- 항상 실패하는 손상 파일에서 재시도를 눌러도 진행이 없음을 확인하고, 애초에 그런 상황에서 "다시 시도" 버튼이 노출되지 않아야 함을 검증.

**Bitvue 판정**: Confirmed — 재확인 결과 동일: `frontend/components/ErrorBoundary.tsx`의 `DefaultErrorFallback`의 "Try Again" 버튼(line 79, `onClick={resetError}`, `resetError` 정의는 line 145-151)은 로컬 boundary 상태만 초기화(`hasError: false`)하고 실제로 다른 조건(입력, 모드 등)을 바꾸지 않은 채 동일 트리를 재렌더 — 결정론적 에러라면 즉시 재현됨. `ErrorDialog` 자체에는 재시도 버튼이 없어(Dismiss/Close만) 이 패턴은 ErrorBoundary 경로에 한정.

---

### UIX-ERR-015: 로그 파일 위치만 제시하고 핵심 문맥을 숨김

**분류**: 진단 정보 접근성 · **심각도**: Medium · **탐지**: Interaction

**사용자 목표**:
별도 파일 탐색기나 텍스트 에디터를 열지 않고도 오류의 핵심 맥락을 바로 파악한다.

**증상**:
- 에러 다이얼로그가 "자세한 내용은 로그를 확인하세요: `~/Library/Logs/bitvue/app.log`"만 보여준다.
- 사용자가 파일 탐색기와 텍스트 에디터를 열어 타임스탬프로 검색해야 원하는 항목을 찾을 수 있다.

**원인**:
UI가 `technical_details`를 인라인으로 표시하는 대신, 로그 파일이 존재한다는 사실만 언급하는 것으로 "상세 정보 제공"을 대체한다.

**구현 냄새**:
- 에러 dialog 컴포넌트에 `details` 필드 렌더 로직이 없고, "로그 확인" 안내 문자열만 하드코딩되어 있음.
- 앱 내 로그 뷰어/필터 UI가 존재하지 않아 로그 확인 자체가 별도 도구 의존적.

**영향**:
진단 마찰이 커진다 — 특히 비개발 사용자는 로그 파일을 어떻게 열어야 할지, 어느 줄을 봐야 할지조차 모른다.

**권장**:
- 핵심 `technical_details`(오프셋, 스택, 관련 필드 값)는 접이식 섹션으로 다이얼로그 안에 바로 표시.
- 로그 파일은 "더 많은 이력 보기"용 보조 수단으로만 남기고, 1차 진단은 다이얼로그만으로 완결되게 함.

**탐지**:
- 로그 파일에 접근할 권한/역량이 없는 사용자를 가정하고, 다이얼로그만으로 문제 파악이 가능한지 테스트.

**Bitvue 판정**: N/A — 재확인 결과 동일: `ErrorDialog.tsx:67-72`가 `details`를 `<details>`/"View Details" 접이식 섹션으로 다이얼로그 안에 이미 인라인 표시하고 있어, "로그 파일 확인하세요"류 안내로 대체하는 패턴 자체가 없음(grep 결과 그런 문자열이나 로그 파일 경로 안내 없음, sidecar/electron 쪽에도 없음). 앱 내 로그 뷰어는 없지만, 애초에 로그 파일로 유도하는 문구가 없어 이 항목이 묘사하는 실패 모드는 발생하지 않음.

---

### UIX-ERR-016: 에러 메시지 톤이 사용자를 탓하는 것처럼 들림

**분류**: UX 라이팅 · **심각도**: Medium · **탐지**: User test

**사용자 목표**:
실수를 했더라도 비난받는 느낌 없이 다음에 무엇을 하면 되는지 안내받는다.

**증상**:
- "잘못된 파일을 선택하셨습니다", "지원되지 않는 형식을 열려고 시도했습니다" 같은 수동 공격적 문구가 쓰인다.
- 실제 원인이 타사 인코더의 비표준 산출물인데도 마치 사용자 행동이 원인인 것처럼 서술된다.

**원인**:
에러 문자열이 개발자가 디버깅 로그를 작성하던 습관("사용자가 X를 시도함")을 그대로 UI에 노출하거나, 별도 UX 라이팅 검수 없이 내부 코드 주석의 톤이 그대로 번역·노출된다.

**구현 냄새**:
- 에러 메시지 작성에 UX 라이팅 가이드/체크리스트가 존재하지 않음.
- `invalid`, `illegal`, `bad` 같은 개발자 관점 단어가 그대로 번역되어 사용됨.

**영향**:
전문 도구에서 실제로는 입력 파일(타사 산출물)이 원인인데도 사용자가 마치 자신이 잘못한 것처럼 느끼게 되어, 반복 사용 의욕이 꺾이고 제품에 대한 정서적 신뢰가 낮아진다.

**권장**:
- 모든 에러 메시지를 "무엇이 일어났는지(중립적 사실) + 무엇을 할 수 있는지(행동)" 2단 구조로 작성.
- 사용자를 주어로 한 비난형 문장을 금지하는 UX 라이팅 체크리스트를 도입하고 리뷰 프로세스에 포함.

**탐지**:
- 모든 에러 메시지 문자열을 수집해 "당신이/사용자가 ~했습니다" 패턴과 비난성 형용사를 정적으로 검사.

**Bitvue 판정**: Suspected — (구 인용 `file.rs`는 스테일, `src-tauri` 삭제됨) 현재 sidecar/프론트 경로에서 재샘플링한 메시지도 결론은 동일: `crates/bitvue-sidecar/src/main.rs:369,386`의 `format!("method not implemented yet: {other}")`/`format!("unknown stream id: {other} (expected \"A\" or \"B\")")`, `useAppFileOperations.ts`의 "Failed to Open File"/"Frame Load Warning" 등은 "you/your" 식 비난형이 아닌 중립적 기술 문구. 다만 전체 코드베이스(특히 `crates/bitvue-*` 각 코덱 파서의 `error.rs`들, ~15개 크레이트)의 모든 사용자向 문자열을 전수 검사하지는 못해, 다른 곳에 비난형 문구가 있을 가능성을 배제할 수 없음.

---

### UIX-ERR-017: 장시간 파싱 중 진행 신호가 없어 응답 없음과 구분 불가

**분류**: liveness 신호 부재 · **심각도**: High · **탐지**: Performance

**사용자 목표**:
앱이 여전히 작업 중인지 멈춘(hang) 것인지 판단해 기다릴지 강제 종료할지 결정한다.

**증상**:
- 대용량 파일 인덱싱/파싱 중 progress bar가 없거나 indeterminate spinner로 고정된 채 몇 분간 아무 변화가 없다.
- 실제로는 내부적으로 deadlock이나 무한 루프에 빠졌는데도 UI는 계속 "처리 중"으로만 표시한다.

**원인**:
진행률 이벤트가 파일 단위처럼 coarse-grained하게만 backend에서 emit되어, "아직 살아있다"는 liveness 신호와 "실제로 진행되고 있다"는 progress 신호가 구분되지 않는다. 타임아웃이나 워치독이 없어 hang 자체를 감지해 사용자에게 알릴 방법이 없다.

**구현 냄새**:
- 프로그레스바가 결정론적 percentage 대신 indeterminate spinner로 고정.
- backend에 heartbeat/tick 이벤트가 없어 프론트엔드가 "마지막으로 언제 응답이 있었는지"를 알 방법이 없음.

**영향**:
사용자가 정상 진행 중인 대용량 작업을 hang으로 오인해 강제 종료(작업 유실)하거나, 반대로 실제 hang을 계속 기다리며 시간을 낭비한다.

**권장**:
- fine-grained progress(프레임/바이트 단위) 이벤트와 heartbeat를 함께 emit해 "마지막 응답 이후 N초"를 표시.
- 임계 시간을 초과하면 UI가 능동적으로 "응답이 없습니다 — 강제 종료하시겠습니까?"를 제안.

**탐지**:
- 의도적으로 backend를 deadlock시키는 테스트 케이스에서, UI가 일정 시간 후 hang을 능동적으로 알리는지 확인.

**Bitvue 판정**: Confirmed(카탈로그가 묘사한 것보다 더 심함) — (구 인용 `src-tauri`는 스테일, 삭제됨) 현재 아키텍처에서 재확인해도 동일 결론, 오히려 인프라 절반만 있고 죽어있음이 더 명확해짐: `Event::WorkerProgress { job_id, progress }`(`crates/bitvue-engine/src/event.rs:28-31`)이라는 progress 이벤트 타입 자체는 정의돼 있고 `crates/bitvue-sidecar/src/main.rs:1869-1870`의 `event_to_json`도 이를 올바르게 직렬화하지만, **저장소 전체에서 이 variant를 실제로 생성(`Event::WorkerProgress { ... }`)하는 코드가 단 한 곳도 없음**(grep 결과 `event.rs`의 정의와 `event_test.rs`의 단위테스트뿐 — 즉 테스트만 존재를 검증하고 아무도 emit하지 않음). 프론트 쪽도 이 이벤트를 구독하는 코드가 없음(`electronBridgeService.ts`/`App.tsx`에 `onProgress`류 없음). `LoadingScreen`의 결정론적 `progress` prop(`Loading.tsx`)도 여전히 호출부 없는 죽은 기능. `openFileAtPath`가 `openStream`을 기다리는 동안 App.tsx는 스피너조차 띄우지 않음(InlineLoading/Spinner/LoadingScreen 사용처 없음) — indeterminate spinner조차 없어 liveness 신호가 전무.

---

### UIX-ERR-018: 여러 오류 중 첫 번째만 보여주고 나머지를 삼킴

**분류**: 오류 집계 부재 · **심각도**: High · **탐지**: Domain review

**사용자 목표**:
파일에 존재하는 문제의 전체 규모를 파악해 "이 파일을 신뢰할 수 있는지" 판단한다.

**증상**:
- 실제로는 파일 전체에 걸쳐 수십 개의 서로 다른 파싱 오류가 있는데, UI는 가장 먼저 만난 오류 하나만 표시하고 나머지는 조용히 버린다.
- 사용자가 그 오류를 해결(혹은 확인)한 뒤 재시도하면 곧바로 다음 오류를 하나씩 순차적으로 마주치게 된다.

**원인**:
에러 집계 파이프라인이 `Result<T, E>`의 single-error 관용구를 그대로 사용해, 다중 오류 수집(`Vec<E>` 등) 없이 첫 실패 지점에서 파이프라인을 종료하고 그 하나만 UI로 전달한다.

**구현 냄새**:
- `?` 연산자로 첫 실패에서 즉시 반환하는 구조가, 배치/전체 스캔처럼 "가능한 만큼 계속 모아야 하는" 로직에도 그대로 사용됨.
- `ErrorCollector`/`Vec<Diagnostic>` 같은 누적 타입이 존재하지 않음.

**영향**:
사용자가 "오류 1건만 수정하면 끝"이라고 오인하고 반복적으로 재시도하며 다음 오류를 하나씩 순차적으로 마주쳐 왕복 비용이 커지고, 파일 전체의 손상도를 과소평가하게 된다.

**권장**:
- 비파괴적 스캔이 가능한 경로에서는 fail-fast 대신 오류를 모두 수집해 "총 N건 발견, 유형별 분포" 요약을 먼저 보여주고, 이후 개별 항목을 펼쳐볼 수 있게 함.

**탐지**:
- 서로 다른 위치에 여러 손상을 주입한 파일을 열어, UI가 "1건"이라고만 말하는지 아니면 전체 개수를 정확히 보여주는지 확인.

**Bitvue 판정**: Confirmed — (구 인용 `src-tauri/src/commands/file.rs`는 스테일, 삭제됨) 현재 sidecar 기준으로도 동일 패턴이 재현됨, 단 위치가 프론트로 이동: `frontend/services/electronBridgeService.ts:539-551`의 `openStream`이 `events.find((e) => e.type === "DiagnosticAdded")`로 **첫 번째** DiagnosticAdded만 취하고 나머지는 버림(누적 `Vec`/집계 없음, 구 버전의 "덮어쓰기"와 결과적으로 동일 — 마지막 대신 첫 번째만 남는다는 차이뿐). N개의 diagnostic이 발생해도 하나만 `error` 필드/UI에 도달하고, 총 개수·유형별 집계는 어디에도 없음 — `DiagnosticsPanel`도(UIX-ERR-008 참고) 이 이벤트들을 구독하지 않아 개수 집계 UI 자체가 없음.
