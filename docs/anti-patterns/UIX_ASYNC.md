# Anti-Pattern Catalog — UIX_ASYNC: 비동기 상태와 피드백

이 문서는 Bitvue(Tauri + Rust + React 기반 비디오 비트스트림 분석기) 안티패턴 카탈로그의 일부입니다. 전체 카탈로그 목록은 `docs/anti-patterns/INDEX.md`(별도 작성 예정)를 참고하세요. 본 문서는 UI/UX+Tauri Phase 3 웨이브의 일부이며, 같은 웨이브의 `TAURI-CMD`(백엔드 커맨드 설계)·`TAURI-EVT`(백엔드 이벤트 설계) 카테고리와 밀접하게 연관됩니다 — 이 문서가 "화면에 어떻게 보여야 하는가"를 다룬다면, 그 두 문서는 "그 화면을 가능하게 하는 백엔드 신호를 어떻게 설계하는가"를 다룹니다. Tauri 앱에서 "UI가 고장난 것 같다"는 불만의 상당수는 프론트엔드 렌더링 버그가 아니라 백엔드 작업(디코드, 파싱, 메트릭 계산)의 상태·진행률·취소·에러가 프론트엔드에 제대로 전달되지 않아서 생긴다는 전제로 작성되었습니다.

아래는 이 카테고리 전체에 걸쳐 참고할 일반적인 상태 모델입니다. 모든 항목에 그대로 적용되는 규범은 아니며, 각 항목에서 관련되는 부분만 인용합니다.

```rust
enum AnalysisViewState<T> {
    Empty,
    Loading { request_id: u64, previous: Option<T> },
    Partial { request_id: u64, value: T, progress: Progress },
    Ready { request_id: u64, value: T },
    Failed { request_id: u64, previous: Option<T>, error: UiError },
    Cancelling { request_id: u64 },
}
```

이 모델의 핵심은 `request_id`로 "이 결과가 어느 요청에 대한 응답인가"를 항상 추적한다는 것과, `Loading`/`Failed` 상태가 `previous` 값을 들고 있어 "로딩 중 이전 데이터를 유지할지, 비울지, 부분 갱신할지"를 상태 자체가 표현할 수 있다는 점입니다. 다만 로딩 중 이전(stale) 결과를 흐리게 유지할지, 비울지, 부분적으로 갱신할지는 작업 종류별로 다른 결정이며, 이 문서 전체에 걸친 고정 규칙이 아니라 기능별 판단 사항입니다.

---

### UIX-ASYNC-001: 버튼 클릭 후 아무 변화가 없음

**분류**: UIX_ASYNC · **심각도**: High · **탐지**: Interaction

**사용자 목표**:
"분석 시작" 같은 버튼을 눌러 즉시 무언가 반응이 시작됐다는 확신을 얻고 싶다.

**증상**:
- 버튼을 클릭해도 커서 모양, 버튼 상태, 화면 어디에도 즉각적인 시각 변화가 없다.
- 사용자가 같은 버튼을 2~5회 연속 클릭한다(더블 클릭, 트리플 클릭 로그가 남는다).
- 실제로는 Tauri `invoke` 호출이 이미 백엔드에서 처리 중이었고, 몇 초 뒤 갑자기 결과나 에러가 나타난다.

**원인**:
`invoke()` 호출과 실제 UI 상태 갱신 사이에 "즉시 반영되는 pending 상태"가 없다. 버튼 클릭 핸들러가 `await` 완료 시점에만 상태를 바꾸고, 클릭 직후(요청 전송 시점)에는 아무 것도 하지 않는다.

**구현 냄새**:
- `onClick={() => invoke('start_analysis').then(setResult)}` 형태로, 클릭과 상태 업데이트 사이에 별도 "요청 시작" 디스패치가 없다.
- 버튼에 `disabled` 속성이나 로딩 인디케이터가 바인딩되어 있지 않다.
- 프론트엔드 상태 스토어에 `Loading` 같은 중간 상태가 아예 정의되어 있지 않다.

**영향**:
같은 분석 작업이 중복 트리거되어 백엔드에서 동일 요청이 여러 번 실행되고, 결과가 뒤섞이거나 리소스가 낭비된다. 사용자는 앱을 신뢰할 수 없다고 느끼고 강제 종료를 시도하기도 한다.

**권장**:
- 클릭 이벤트 핸들러의 첫 줄에서 동기적으로 `Loading` 상태로 전환하고, 버튼을 즉시 `disabled` 처리한다.
- `invoke` 호출 전에 낙관적 UI 반응(스피너, 텍스트 변경, progress bar 등장)을 먼저 커밋한다.
- 100ms 이내 반응이 없으면 "무언가 잘못됐다"고 간주하고 별도 QA 체크리스트 항목으로 관리한다.

**탐지**:
- 시나리오 테스트: 버튼을 클릭한 직후(0~50ms) 스크린샷을 찍어 이전 프레임과 픽셀 diff가 있는지 확인.
- Interaction 로그에서 동일 커맨드가 짧은 시간 내 중복 invoke 되는 빈도를 계측.

**Bitvue 판정**: Confirmed — (재검증: Tauri→Electron 전환으로 `src-tauri`는 저장소에서 완전히 삭제됐고 실제 경로는 `frontend/services/electronBridgeService.ts`의 sidecar 브리지다. 이전 판정이 인용한 `src-tauri`/`invoke("open_file")` 경로는 이제 존재하지 않으므로 아래는 현재 코드로 재확인한 내용.) `frontend/hooks/useAppFileOperations.ts`의 `handleOpenFile`(213-249행, 내부적으로 `openFileAtPath`→`openStream`/`selectFrame`/`getStreamInfo` bridge 호출 체인)은 이 파일 전체(328행)에 `isLoading`/`isOpening` 류 상태 자체가 없어 로딩 상태를 아예 반환하지 않는다. `frontend/App.tsx:692`의 `<button onClick={handleOpenFile}>Open Different File</button>`에도 `disabled`나 인디케이터가 없다. 클릭 즉시 아무 시각적 반응 없이 파일 다이얼로그→`openStream`→`selectFrame`→`refreshFrames()`가 순차 실행되고, `setFileInfo`는 전체 체인이 끝난 뒤에야 갱신된다.

---

### UIX-ASYNC-002: spinner 하나로 모든 상태 표현

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: Visual

**사용자 목표**:
지금 무엇이 일어나고 있는지(파일 읽는 중인지, 디코드 중인지, 메트릭 계산 중인지) 대략이라도 알고 싶다.

**증상**:
- 파일 열기, 프레임 디코드, VMAF 계산, 썸네일 생성 등 성격이 전혀 다른 작업이 모두 동일한 원형 spinner 하나로만 표시된다.
- 사용자가 "지금 몇 분 걸리는 게 정상인지" 판단할 근거가 화면에 없다.
- 짧게 끝나는 작업과 수십 초 걸리는 작업이 시각적으로 구별되지 않아, 짧은 작업에도 긴장하거나 긴 작업에도 방치한다.

**원인**:
프론트엔드가 백엔드 작업의 종류(kind)를 구분해서 받지 않고, "busy: bool" 같은 단일 불리언 플래그만으로 로딩 UI를 결정한다.

**구현 냄새**:
- 전역 `isLoading` 상태 하나가 여러 종류의 비동기 작업에서 공유된다.
- `<Spinner />` 컴포넌트가 라벨이나 아이콘 없이 앱 전역에서 재사용된다.
- 백엔드 이벤트 페이로드에 작업 종류를 나타내는 필드(`task_kind`, `stage`)가 없다.

**영향**:
사용자가 진행 상황을 예측할 수 없어 불안해하고, 실제로 멈춘 것(hang)과 정상적으로 오래 걸리는 것을 구분하지 못해 불필요한 재시작이나 강제 종료를 반복한다.

**권장**:
- 작업 종류별로 최소한의 라벨(예: "파일 읽는 중", "프레임 디코드 중", "VMAF 계산 중")을 붙인다.
- 예상 소요 시간대가 크게 다른 작업(수백ms vs 수십초)은 시각적 톤도 다르게 한다(인라인 spinner vs progress bar + 취소 버튼).
- `AnalysisViewState`의 `Loading`/`Partial`에 작업 종류를 실어 UI가 분기할 수 있게 한다.

**탐지**:
- Visual 감사: 앱 내 모든 로딩 상태를 스크린샷으로 수집해 spinner 컴포넌트 재사용 빈도와 라벨 유무를 표로 정리.
- 코드에서 로딩 상태 타입을 grep해 boolean 단일 플래그 사용 빈도 확인.

**Bitvue 판정**: Suspected — (재검증, 현재 Electron+`bitvue-sidecar` 아키텍처 기준) 반쯤 뒤집힌 형태로 나타난다. 공유 `LoadingScreen`/`Spinner`(`frontend/components/Loading.tsx`, `title`/`message`/`progress` prop까지 갖춤)는 grep 결과 `vitest.config.ts` 외 앱 전체에서 단 한 곳도 사용되지 않는 죽은 코드이며, 대신 각 패널이 제각각 즉석 텍스트를 박아 넣는다(`YuvViewerPanel/index.tsx:579` "Loading frame N..." — 이건 실제 마운트되는 라이브 경로). `QualityMetricsPanel.tsx:193` "Calculating..."도 존재하지만 이 패널 자체가 `panels/index.ts` 배럴에서만 export되고 앱 어디서도 import/렌더링되지 않는 도달 불가 죽은 코드다(아래 008/009 판정 참고, 게다가 여전히 존재하지 않는 `@tauri-apps/api` invoke를 호출한다). 즉 "단일 spinner로 뭉개짐"은 아니지만, "작업 종류(`stage`)를 구조적으로 실어 나르는 채널이 없다"는 근본 원인은 라이브 경로 기준으로도 그대로다 — `bitvue-sidecar`의 wire 프로토콜에 `Event::WorkerProgress`(`crates/bitvue-engine/src/event.rs:28`, `crates/bitvue-sidecar/src/main.rs:1869`)라는 progress 이벤트 타입 자체는 정의돼 있지만, 이를 실제로 발행하는 커맨드 핸들러가 하나도 없다(자체 테스트에서만 구성됨, 아래 004 판정 참고).

---

### UIX-ASYNC-003: 시작 중·분석 중·취소 중·실패를 구분하지 않음

**분류**: UIX_ASYNC · **심각도**: High · **탐지**: Code

**사용자 목표**:
작업이 지금 "막 시작됐다", "한창 진행 중이다", "취소되고 있다", "실패로 끝났다" 중 어느 국면인지 구분해서 그에 맞게 행동하고 싶다(기다릴지, 취소 버튼을 다시 누를지, 재시도할지).

**증상**:
- 취소 버튼을 눌렀는데 화면이 여전히 "분석 중..."으로만 보여, 취소가 접수됐는지 알 수 없다.
- 실패했는데도 로딩 인디케이터가 계속 돌고 있거나, 반대로 아직 시작 단계인데 이미 "실패"로 잠깐 깜빡였다가 사라진다.
- 사용자가 같은 버튼(시작/취소)을 상태를 오인해 반복 클릭한다.

**원인**:
UI 상태 모델이 "idle vs busy" 2단계로만 축소되어 있어서, 시작·진행·취소 요청됨·취소 완료·실패라는 서로 다른 국면이 하나로 뭉개진다.

**구현 냄새**:
- 상태 변수가 `boolean isRunning` 하나뿐이고, 취소나 실패를 별도 enum 값으로 표현하지 않는다.
- 취소 요청이 프론트엔드 상태를 즉시 바꾸지 않고, 백엔드의 최종 종료 이벤트만 기다린다.
- `AnalysisViewState`의 `Cancelling` 같은 중간 상태가 코드베이스에 존재하지 않는다.

**영향**:
사용자가 취소가 먹혔는지 몰라 여러 번 클릭하고, 그 결과 취소 요청이 중복 전송되거나 취소와 재시작이 경합해 백엔드 상태가 꼬인다.

**권장**:
- 최소 5개 국면(Idle/Loading/Partial/Failed/Cancelling)을 명시적 enum으로 모델링하고, 각 국면마다 다른 버튼 배치·문구를 쓴다.
- 취소 버튼 클릭 시 즉시 `Cancelling`으로 전환하고, 버튼을 "취소 중..."으로 바꾸고 비활성화한다.
- 취소 완료 이벤트가 오면 `Empty` 또는 이전 상태로 명확히 되돌린다.

**탐지**:
- 시나리오 테스트: 시작 → 취소 클릭 → 백엔드 취소 완료까지 각 시점의 화면 상태를 캡처해 5개 국면이 실제로 구별되는지 확인.
- Code 리뷰: 상태 타입 정의에서 enum variant 개수를 카운트.

**Bitvue 판정**: Confirmed — (재검증 결과 이전 판정의 "백엔드에 취소 개념 자체가 없다"는 결론은 틀렸다 — `src-tauri`는 삭제됐고, 현재 `bitvue-sidecar`에는 실제 `cancel_request` 와이어 커맨드가 있다: `crates/bitvue-sidecar/src/main.rs:137-150`(요청 파싱 즉시 특수 처리), `compute_cancel_response`(232행 이하)가 `correlation_id`→`AtomicBool` 레지스트리에 취소 플래그를 세팅한다. 그럼에도 이 항목은 여전히 Confirmed다: (1) `frontend/services/electronBridgeService.ts`에 `cancel_request`를 감싸는 wrapper가 전혀 없다 — 이 커맨드를 호출하는 프론트엔드 코드가 하나도 없어(grep 결과 export 함수 목록에 `cancel*` 없음) 사용자가 취소를 트리거할 UI 자체가 없다. (2) 프론트엔드는 예외 없이 `boolean isLoading`/`isCalculating` 패턴만 쓰고(`FileStateContext.tsx`의 `loading`, `QualityMetricsPanel.tsx:38`, `YuvDiffContext.tsx`의 `loading` 등) `Cancelling` 같은 중간 국면이나 5국면 enum이 어디에도 없다. 취소 프리미티브는 프로토콜 계층에 존재하지만 UI 표면까지 배선되지 않았다는 점이 핵심 결함(아래 006 판정도 참고 — 배선된다 해도 best-effort일 뿐).

---

### UIX-ASYNC-004: 진행률이 99%에서 오래 멈춤

**분류**: UIX_ASYNC · **심각도**: High · **탐지**: User test

**사용자 목표**:
진행률 바를 보고 "곧 끝난다"는 신뢰할 수 있는 신호를 얻고 싶다.

**증상**:
- 진행률이 0%에서 95%까지는 매끄럽게 올라가다가, 99%에서 수 초~수십 초간 멈춘다.
- 사용자가 "멈춘 건가?"를 판단하지 못해 앱을 재시작하거나 파일을 다시 연다.
- 실제로는 마지막 단계(예: 파일 flush, 인덱스 finalize, 메트릭 집계)가 나머지 전체보다 오래 걸리는데 진행률 계산에는 반영되지 않았다.

**원인**:
진행률이 "완료된 작업 개수 / 전체 작업 개수" 같은 균등 가중치로 계산되어, 실제 소요 시간이 균등하지 않은 마지막 단계(흔히 I/O flush나 finalize)의 비중을 과소평가한다.

**구현 냄새**:
- 진행률 계산이 `processed_frames / total_frames * 100` 처럼 프레임 수만 세고, finalize 단계(디스크 flush, 요약 통계 계산)는 progress 이벤트를 아예 보내지 않는다.
- 마지막 5% 구간에 해당하는 백엔드 코드가 동기적 blocking 작업(예: `File::sync_all()`)을 포함하지만 progress 콜백이 없다.

**영향**:
사용자가 실제로는 정상 진행 중인 작업을 hang으로 오인해 강제 종료 → 재시도 루프에 빠지고, 최악의 경우 실제로 finalize 중이던 파일이 손상된다.

**권장**:
- 시간 기반 프로파일링으로 각 단계의 실제 소요 비중을 측정해 진행률 가중치를 재산정한다(프레임 수가 아니라 벤치마크된 시간 비율로).
- finalize처럼 멈출 수 있는 구간은 "마무리하는 중입니다..." 같은 별도 sub-label과 indeterminate(불확정) 인디케이터로 전환한다.
- 진행률이 일정 시간(예: 3초) 이상 변하지 않으면 UI가 자동으로 indeterminate 모드로 전환하는 안전장치를 둔다.

**탐지**:
- User test: 실제 대용량 파일로 전체 파이프라인을 실행하며 진행률 변화 곡선을 기록, 마지막 10% 구간의 실제 경과 시간 비율을 측정.
- Performance: 각 단계별 progress 이벤트 발행 간격을 로깅해 "이벤트 공백 구간"을 자동 탐지.

**Bitvue 판정**: N/A — (재검증) 이 패턴이 성립하려면 먼저 숫자 진행률 바가 실제로 존재해야 하는데, 현재 배선된 프로덕션 경로(파일 열기 `useAppFileOperations.ts`, `electronBridgeService.ts`의 모든 sidecar 커맨드)에는 진행률 바 자체가 없다 — 있는 건 "Loading"/"Calculating..." 같은 정적 텍스트뿐이다(001, 002 판정 참고). `bitvue-engine`에 `Event::WorkerProgress`(`crates/bitvue-engine/src/event.rs:28`)와 이를 스케줄링할 수 있는 `AsyncJobManager`/`worker.rs`(last-wins 취소·최대 2-in-flight 모델까지 설계된 실제 잡 매니저)가 존재하지만, `grep -rln AsyncJobManager crates`로 확인한 결과 `worker.rs` 자신의 테스트 파일 외에는 `core.rs`나 `bitvue-sidecar`가 이를 전혀 호출하지 않는다 — 설계는 있지만 배선되지 않은 죽은 서브시스템이다. 단계별 progress 계산 로직을 담은 `frontend/utils/progressiveLoader.ts`도 여전히 어떤 컴포넌트에서도 호출되지 않는 죽은 코드라(005 판정), 99% 정체를 "관찰"할 화면 자체가 현재 없다.

---

### UIX-ASYNC-005: 진행률의 작업 단위가 중간에 바뀜

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: Code

**사용자 목표**:
진행률 숫자가 처음부터 끝까지 같은 기준으로 오르길 기대한다(예: "몇 % 남았다"가 계속 같은 의미이길).

**증상**:
- 진행률이 0→40%까지는 빠르게 오르다가, 40% 지점에서 갑자기 역행하거나 다시 0%부터 새로 카운트되는 것처럼 보인다.
- 예를 들어 "파일 스캔 단계(프레임 개수 기준)"에서 "디코드 단계(바이트 기준)"로 넘어가면서 진행률 산식이 바뀌는데, 화면의 progress bar는 하나로 이어져 있다.

**원인**:
파이프라인이 내부적으로 여러 하위 단계(스캔 → 파싱 → 디코드 → 메트릭)로 구성되고 각 단계가 서로 다른 단위(프레임 수, 바이트, 시간)로 진행률을 보고하는데, 프론트엔드가 이를 단일 progress bar에 그대로 이어 붙인다.

**구현 냄새**:
- 백엔드 progress 이벤트에 `stage` 필드 없이 `percent: f32` 하나만 전달된다.
- 프론트엔드가 여러 단계의 `percent`를 검증 없이 그대로 하나의 progress bar 값으로 매핑한다.
- 단계 전환 시점에 progress 값이 시간적으로 비단조(non-monotonic)인 케이스가 실측된다.

**영향**:
사용자가 진행률 숫자를 신뢰하지 않게 되고, 이후에는 progress bar를 아예 무시한 채 "그냥 될 때까지 기다리는" 습관이 생겨 UI의 존재 의미가 없어진다.

**권장**:
- 각 하위 단계마다 전체 파이프라인에서 차지하는 예상 비중을 사전 정의하고, 전체 진행률 = Σ(단계 비중 × 단계 내부 진행률)로 정규화해 단조 증가를 보장한다.
- 여러 단계를 하나의 progress bar로 뭉개지 말고, "2/4단계: 디코드 중 (63%)"처럼 현재 단계와 전체 위치를 함께 노출하는 것도 고려한다.
- 프론트엔드에서 진행률 값이 이전 값보다 작아지는 경우 방어적으로 클램프(clamp)한다.

**탐지**:
- Code 리뷰: 백엔드에서 progress 이벤트를 발행하는 모든 지점을 찾아 단위(percent 산식)가 통일되어 있는지 확인.
- 시나리오 테스트: 전체 파이프라인 실행 중 progress 값 시계열을 기록해 비단조 구간이 있는지 자동 검증.

**Bitvue 판정**: Suspected — (재검증, 여전히 유효) `frontend/utils/progressiveLoader.ts`의 `ProgressiveFileLoader`가 `stage: "parsing"`(103-149행 부근)과 `stage: "thumbnails"`(213-219행 부근)를 각각 독립적으로 0→100%로 계산해 `onProgress` 콜백에 보고하는 구조라, 두 콜백을 하나의 progress bar에 순서대로 이어 붙이면 정확히 이 항목이 설명하는 "100%→0%로 역행" 버그가 재현된다. 다만 grep 결과 `loadFramesProgressive`/`loadThumbnailsProgressive`/`ProgressiveFileLoader`를 호출하는 컴포넌트가 현재 하나도 없어(tsconfig.json 타입체크 대상 외 아무도 참조 안 함, 완전 죽은 코드) 사용자가 실제로 이 버그를 관찰할 화면은 없다 — Tauri→Electron 전환(2026-08-08) 이후로도 여전히 미배선 상태. 향후 이 유틸을 실제 UI에 연결할 때 재발할 잠재적 결함으로 플래그.

---

### UIX-ASYNC-006: 취소 버튼이 있지만 실제 작업은 계속됨

**분류**: UIX_ASYNC · **심각도**: Critical · **탐지**: Interaction

**사용자 목표**:
취소를 누르면 백엔드 작업이 실제로 멈추고 자원(CPU, 메모리, 디스크)이 즉시 해제되길 기대한다.

**증상**:
- 취소 버튼을 누르면 UI는 즉시 로딩 화면을 닫지만, 작업 관리자/Activity Monitor를 보면 CPU 사용률이 계속 높게 유지된다.
- 같은 파일을 다시 열려고 하면 "파일이 사용 중"이라거나 원인 불명의 지연이 발생한다.
- 취소 직후 재시작한 새 분석 요청이 아직 죽지 않은 이전 작업과 경합해 결과가 오염된다(UIX-ASYNC-007과 연관).

**원인**:
UI의 "취소"는 프론트엔드 상태만 초기화할 뿐, 백엔드 비동기 task/thread에 실제 취소 신호(cancellation token)를 전달하지 않는다. 특히 FFI 디코더 호출이나 `spawn_blocking`으로 넘어간 작업은 Rust의 `Future` drop만으로는 중단되지 않는다.

**구현 냄새**:
- Tauri 커맨드가 `tokio::select!` + `CancellationToken` 없이 단순히 `await`만 하고 있어, 프론트엔드가 응답을 버려도(promise 무시) 백엔드 task는 끝까지 실행된다.
- FFI 디코더(dav1d, libvmaf 등) 호출 루프 안에 취소 체크 포인트가 없다.
- "취소" 버튼의 핸들러가 로컬 상태만 리셋하고 별도 `cancel_*` 커맨드를 invoke하지 않는다.

**영향**:
사용자가 취소했다고 믿은 작업이 백그라운드에서 계속 CPU/메모리를 점유해 이후 작업 성능이 저하되고, 여러 번 취소·재시도를 반복하면 좀비 작업이 누적되어 결국 앱이 응답 불능 상태에 빠질 수 있다.

**권장**:
- 취소는 반드시 명시적 `cancel_*` 커맨드나 공유 `CancellationToken`으로 백엔드에 전달하고, 백엔드는 이를 실제로 관찰하는 체크포인트(루프마다, 청크마다)를 둔다.
- FFI 경계를 넘는 작업은 취소가 즉시 반영되지 않을 수 있음을 인정하고, `Cancelling` 상태를 UI에 유지하다가 백엔드의 실제 종료 확인 이벤트를 받은 뒤에야 `Empty`로 전환한다.
- 취소 후 일정 시간 내 종료 확인이 오지 않으면 사용자에게 "정리 중입니다"를 계속 보여주거나 강제 종료 옵션을 제공한다.

**탐지**:
- Interaction: 취소 클릭 후 시스템 리소스(CPU, 스레드 수)를 모니터링해 실제로 감소하는지 확인.
- 시나리오 테스트: 취소 직후 동일 파일로 새 작업을 시작해 결과가 오염되는지 검증.

**Bitvue 판정**: Confirmed — (재검증) 이번엔 최상급 증거가 나왔다: `crates/bitvue-sidecar/src/main.rs`의 모듈 문서(51-58행)가 **이 안티패턴을 스스로 정확히 자백**하고 있다 — "`cancel_request` sets a per-request `AtomicBool` flag... **This is best-effort, not preemption**: none of today's handlers have a cooperative checkpoint mid-execution... cancelling a request that has already started executing has no effect; it only works for the (currently narrow, timing-dependent) window before the worker thread's check runs." 즉 취소 플래그는 워커 스레드가 "실행 시작 직전 딱 한 번" 검사할 뿐(`spawn_request`, 196-212행), 실행 중인 `Core::handle_command`/디코드 루프 안에는 체크포인트가 전혀 없다 — `grep -rn "CancellationToken\|tokio::select!" crates/bitvue-sidecar/src crates/bitvue-engine/src` 결과도 0건. 게다가 `frontend/services/electronBridgeService.ts`에 `cancel_request` wrapper 자체가 없어(003 판정 참고) 이 반쪽짜리 취소조차 프론트엔드에서 트리거할 방법이 없다 — 사용자 관점에서는 "취소 버튼"이 아예 존재하지 않는 것과 같은 상태이면서, 백엔드 설계 문서 자체가 "취소해도 이미 시작된 작업은 안 멈춘다"를 명시하고 있는 이중 확인.

---

### UIX-ASYNC-007: 오래된 요청 결과가 최신 화면을 덮어씀

**분류**: UIX_ASYNC · **심각도**: Critical · **탐지**: Code

**사용자 목표**:
화면에 보이는 결과가 항상 "지금 선택한 것"에 대한 결과이길 기대한다.

**증상**:
- 사용자가 프레임 A를 선택했다가 빠르게 프레임 B로 옮겼는데, 화면에는 잠깐 B의 결과가 보이다가 뒤늦게 도착한 A의 결과로 다시 바뀐다.
- 특히 네트워크/디스크 I/O 편차가 큰 상황(예: 첫 프레임은 캐시 미스, 다음 프레임은 캐시 히트)에서 응답 순서가 요청 순서와 뒤바뀔 때 자주 발생한다.
- 사용자가 이미 다른 곳을 보고 있는데 화면이 "제멋대로" 바뀌는 것처럼 느껴진다.

**원인**:
비동기 요청의 완료 콜백이 "이 응답이 아직 유효한 최신 요청에 대한 것인가"를 검증하지 않고 도착 순서대로 그대로 상태에 반영한다. Race condition의 전형적인 형태.

**구현 냄새**:
- `useEffect`나 이벤트 리스너에서 `setResult(response)`를 호출할 때 요청 시점의 식별자와 비교하는 로직이 없다.
- 프론트엔드 상태에 `request_id`/`generation` 같은 필드가 없어 "이 응답이 최신 요청의 응답인지"를 판별할 수 없다.
- 이전 요청을 명시적으로 취소하거나 무시하는 처리(AbortController 상당 로직) 없이 매번 새 요청만 계속 발사한다.

**영향**:
사용자가 화면에 보이는 정보를 신뢰할 수 없게 되고, 특히 프레임 비교·메트릭 분석처럼 정확성이 핵심인 도구에서는 잘못된 프레임의 데이터를 실제 분석 결과로 오인할 위험이 있다.

**권장**:
- 모든 비동기 요청에 단조 증가하는 `request_id`를 부여하고, 응답 처리 시 "현재 최신 request_id와 일치하는가"를 반드시 검사한 뒤에만 상태를 갱신한다(`AnalysisViewState`의 `request_id` 필드가 이 역할).
- 가능하면 이전 요청을 실제로 취소(UIX-ASYNC-006 참고)해 불필요한 백엔드 작업 자체를 막는다.
- React 기준으로는 `useEffect` cleanup에서 stale closure를 무효화하는 flag/AbortController 패턴을 표준화한다.

**탐지**:
- Code 리뷰: 비동기 응답 핸들러마다 request_id 비교 로직이 있는지 grep으로 점검.
- 시나리오 테스트: 의도적으로 응답 지연을 역전시켜(첫 요청을 느리게, 두 번째를 빠르게 mock) 최종 화면이 최신 요청 결과와 일치하는지 자동 검증.

**Bitvue 판정**: Confirmed — (재검증, 이 항목은 프론트엔드 전용 패턴이라 Tauri→Electron 전환 영향이 적어 대부분 유효했다) `request_id`/`generation` 필드는 grep 결과 코드베이스 어디에도 없다(node_modules 제외). 구체적 사례: `frontend/contexts/YuvDiffContext.tsx`의 `fetchMetrics`(174-187행, 현재는 `getYuvDiffMetrics(frameIndex)` — `electronBridgeService.ts` bridge 함수, 옛 `invoke("get_yuv_diff_metrics")`가 아님)는 응답을 검증 없이 그대로 `setState((s) => ({ ...s, metrics }))`로 덮어쓴다. `frontend/components/panels/YuvDiffPanel.tsx`가 `currentFrameIndex`가 바뀔 때마다 가드 없이 재호출한다 — 빠른 프레임 이동 시 오래된 프레임의 응답이 늦게 도착하면 최신 프레임의 metrics를 덮어쓴다(단, `YuvDiffPanel.tsx:208`의 렌더링 쪽은 `metrics.frame_index === currentFrameIndex`로 화면 표시는 가드하므로 — 008 판정 참고 — 사용자가 "틀린 프레임 숫자"를 보는 것 자체는 막히지만, `state.metrics`엔 여전히 stale 값이 남아 다음 판정 로직 등에 영향을 줄 수 있다). 이 패턴이 전역적이진 않다: `YuvViewerPanel/index.tsx`, `SyntaxDetailPanel`의 각 탭(`ApsTab.tsx`/`QmTab.tsx`/`RefListTab.tsx`/`ProbsTab.tsx`), `UnitHexPanel/HexViewTab.tsx`는 effect-scope `let cancelled = false` 가드로 이 경합을 완화하고 있다(이번 재검증에서 라인 단위 재확인은 생략, 패턴 존재만 grep으로 재확인).

---

### UIX-ASYNC-008: 실패 후 이전 데이터가 최신 결과처럼 남음

**분류**: UIX_ASYNC · **심각도**: High · **탐지**: Interaction

**사용자 목표**:
분석이 실패했다면 "실패했다"는 사실과 함께, 지금 화면에 남아있는 숫자가 새 시도의 결과가 아니라 예전 결과라는 것을 명확히 알고 싶다.

**증상**:
- 새로고침/재분석을 시도했다가 실패했는데, 화면에는 이전 성공 결과(예: VMAF 점수, 메트릭 그래프)가 그대로 남아 있고 에러 메시지는 작은 토스트로 잠깐 스쳐 지나간다.
- 사용자가 에러를 못 보고 놓치면, 이전 파일/이전 설정의 결과를 현재 파일의 결과로 착각한 채 보고서를 작성하거나 의사결정을 내린다.

**원인**:
실패 시 상태 전이가 "에러 토스트 표시"만 하고 데이터 상태 자체는 건드리지 않아서, 화면에 남은 값과 그 값의 출처(어느 request_id, 어느 파일)가 사용자에게 구분되지 않는다.

**구현 냄새**:
- 에러 핸들링이 `catch(e) => toast.error(e.message)`로 끝나고 `previous` 데이터에 대한 "이건 오래된 값입니다" 표시가 전혀 없다.
- 실패 상태(`Failed`)가 `previous` 값을 UI 레벨에서 명확히 구분해서 렌더링하지 않고, 성공 상태와 동일한 컴포넌트를 그대로 재사용한다.

**영향**:
전문가용 분석 도구에서 특히 치명적이다 — 사용자가 잘못된(stale) 메트릭을 신뢰하고 QA 판정이나 인코딩 파라미터 결정을 내리면 실제 품질 문제를 놓칠 수 있다.

**권장**:
- `Failed { previous, error }` 상태에서는 반드시 이전 데이터에 시각적 마커(회색 처리, "이 값은 이전 분석 결과입니다" 배지, 워터마크성 오버레이)를 씌운다.
- 에러 메시지는 자동으로 사라지는 토스트가 아니라, 해당 데이터 영역에 지속적으로 표시되는 인라인 에러 배너로 노출한다.
- 실패한 요청의 대상(파일명, 프레임 번호 등)을 에러 메시지에 명시해 "무엇이 실패했는지" 오해의 소지를 없앤다.

**탐지**:
- Interaction: 성공 → 실패를 유발하는 시나리오(예: 파일을 분석 중 삭제)를 재현해 실패 후 화면 상태를 스크린샷으로 검증.
- User test: 실패 상태를 본 사용자에게 "지금 보이는 숫자가 최신 결과인지"를 질문해 오인률 측정.

**Bitvue 판정**: Confirmed — (재검증, 더 강한 라이브 사례로 교체) 원래 인용됐던 `QualityMetricsPanel.tsx`는 재확인 결과 `panels/index.ts` 배럴에서만 export되고 앱 어디서도 import/렌더링되지 않는 도달 불가 죽은 코드였다(게다가 존재하지 않는 `@tauri-apps/api` invoke와 `calculate_quality_metrics`라는, `bitvue-sidecar`에 없는 커맨드를 호출한다 — grep 결과 사이드카 커맨드 목록에 quality/vmaf/psnr/ssim 없음). 대신 실제로 마운트되는 `frontend/contexts/YuvDiffContext.tsx`의 `fetchMetrics`(174-187행)가 같은 패턴의 라이브 사례다: `getYuvDiffMetrics` 호출이 실패하면 `catch` 블록이 `logger.warn`만 하고 `return null`할 뿐 `state.metrics`를 지우거나 별도 실패 플래그를 세우지 않는다 — 실패 이전에 표시되던 PSNR/SSIM 값이 `frontend/components/panels/YuvDiffPanel.tsx:208-222`에 그대로 렌더링된 채 남고, 에러였다는 사실이 그 값에 부착되지 않는다(단, `metrics.frame_index === currentFrameIndex` 가드 덕에 "다른 프레임의 값"이 새 프레임인 척 보이는 최악의 경우는 막힘 — 007 판정 참고). `QualityMetricsPanel.tsx`(93-121행, `handleCalculate`)도 `catch`에서 `setError`만 하고 `metrics` state를 안 건드리는 동일 구조이나, 이쪽은 죽은 코드라 부차적 증거로만 남긴다.

---

### UIX-ASYNC-009: 부분 결과를 보여줄 수 있는데 끝까지 대기

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: Domain review

**사용자 목표**:
프레임 단위/구간 단위로 순차 계산되는 분석(예: 프레임별 VMAF, 프레임별 비트 사용량)에서, 이미 계산된 앞부분만이라도 먼저 보고 싶다.

**증상**:
- 1000프레임짜리 VMAF 계산이 프레임 1부터 순차적으로 진행되는데, 화면은 계산이 100% 끝날 때까지 빈 화면/spinner만 보여준다.
- 사용자가 앞쪽 50프레임만 봐도 "품질이 심각하게 나쁘다"는 걸 알 수 있는 상황에서도, 전체 계산(수 분)이 끝나야 아무 정보도 얻지 못한다.
- 작업을 중간에 취소하면 이미 계산된 부분 결과까지 통째로 버려진다.

**원인**:
백엔드 파이프라인이 스트리밍(청크 단위 emit) 구조가 아니라 "전체 계산 후 한 번에 반환"하는 batch 구조로 작성되어 있고, 프론트엔드도 최종 결과 하나만 받는 것으로 계약이 고정되어 있다.

**구현 냄새**:
- 백엔드 커맨드가 `Vec<FrameMetric>`을 전부 모은 뒤 함수 반환값으로 한 번에 돌려준다(중간 Tauri 이벤트 emit이 없음).
- 프론트엔드 상태 모델에 `Partial { value, progress }`에 해당하는 개념이 없어, 그래프 컴포넌트가 "완전한 배열"만 입력으로 받는다.
- 알고리즘 특성상 이미 순차적으로 계산되는데도(예: 프레임을 순서대로 디코드) 그 중간 산출물을 굳이 버퍼링만 하고 있다.

**영향**:
탐색적 분석(예: "이 구간에 문제가 있는지 빨리 훑어보고 싶다")의 워크플로가 크게 느려지고, 이미 계산된 데이터도 취소 시 함께 버려지므로 반복 작업 시간이 누적된다.

**권장**:
- 순차적으로 계산 가능한 파이프라인은 프레임/청크 단위로 Tauri 이벤트를 emit해 프론트엔드가 `Partial` 상태로 점진적으로 그래프를 채우게 한다.
- 이미 계산된 부분 결과는 취소 시에도 보존해 사용자가 "여기까지의 결과"를 계속 볼 수 있게 한다.
- 부분 결과 UI에는 "아직 진행 중"임을 나타내는 시각적 경계(예: 그래프의 오른쪽 끝에 흐림 효과)를 둔다.

**탐지**:
- Domain review: 순차 계산이 가능한 메트릭 파이프라인 목록을 만들고, 각각이 스트리밍 emit을 지원하는지 점검.
- Performance: "첫 유의미한 데이터가 화면에 나타나기까지의 시간(TTFB에 해당하는 지표)"을 측정해 전체 완료 시간과 비교.

**Bitvue 판정**: N/A — (재검증, 이전 Confirmed 판정을 뒤집음) 이전 판정이 인용한 `src-tauri/src/commands/quality.rs::calculate_quality_metrics`는 2026-08-08 Tauri 삭제와 함께 코드베이스에서 완전히 사라졌다. 현재 `bitvue-sidecar`의 실제 커맨드 표면(`crates/bitvue-sidecar/src/main.rs`의 method match — `open_stream`/`select_*`/`get_hex_range`/`get_thumbnails`/`get_yuv_diff_metrics`/`get_frame_analysis`/`get_av1_features`/`get_coding_flow_analysis`/`get_deblocking_analysis`/`get_codec_extended_info`/`get_residual_analysis` 등)에는 배치 VMAF/PSNR/SSIM 계산 커맨드가 아예 없다(grep 결과 quality/vmaf/psnr/ssim 문자열이 `main.rs`/`decode_bridge.rs` 등 어디에도 없음). `get_yuv_diff_metrics`는 프레임 1개당 한 번 계산하는 즉시 반환 호출이라 "순차 계산인데 끝까지 대기" 패턴이 성립할 만큼 길지 않다. 유일하게 이 패턴이 서술하는 배치 계산 코드는 `frontend/components/panels/QualityMetricsPanel.tsx`(93-121행)에 남아있지만, 이 패널은 (a) 앱 어디서도 렌더링되지 않는 도달 불가 죽은 코드이고 (b) 호출하는 `calculate_quality_metrics` 커맨드 자체가 백엔드에 없어 실행하면 즉시 실패한다 — 살아있는 사용자 경로에 이 패턴이 적용될 대상 기능이 현재 존재하지 않는다.

---

### UIX-ASYNC-010: 백그라운드 작업이 무엇인지 사용자가 모름

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: User test

**사용자 목표**:
현재 화면에서 직접 트리거하지 않은 작업(백그라운드 프리페치, 캐시 워밍, 자동 저장, 백그라운드 재인덱싱)이 지금 돌고 있다는 것과 그것이 무엇인지 알고 싶다.

**증상**:
- 앱이 갑자기 느려지거나 CPU 사용률이 튀는데, 화면에는 어떤 로딩 표시도 없다.
- 사용자가 명시적으로 시작한 적 없는 작업(예: 다음 프레임 미리 디코드, 인접 GOP 프리페치, 프로젝트 자동 저장)이 사용자가 다른 조작을 할 때 리소스를 놓고 경합한다.
- "지금 뭐 하고 있는 거지?"에 답할 UI 요소(상태 표시줄, 백그라운드 작업 목록)가 없다.

**원인**:
백그라운드 작업이 프론트엔드에 보고할 명분이 없다고 설계 단계에서 판단되어, 진행 상황을 사용자에게 노출하는 채널 자체가 없다.

**구현 냄새**:
- 프리페치/자동 저장/재인덱싱 등의 로직이 Rust 쪽 `tokio::spawn`으로 fire-and-forget 되고 Tauri 이벤트를 전혀 emit하지 않는다.
- 프론트엔드에 전역 "백그라운드 작업 상태" 패널/인디케이터가 존재하지 않는다.
- 사용자가 백그라운드 작업의 존재를 CPU 사용률이나 우연한 지연을 통해서만 추측할 수 있다.

**영향**:
사용자가 앱의 느려짐을 버그로 오인해 불필요한 문의/재시작을 하게 되고, 백그라운드 작업과 전경(foreground) 작업이 리소스를 두고 경합할 때 어느 쪽이 문제인지 진단하기 어려워진다.

**권장**:
- 상태 표시줄이나 알림 영역에 현재 실행 중인 백그라운드 작업 목록(종류, 대상, 진행률)을 최소한이라도 노출한다.
- 사용자가 원하면 백그라운드 작업을 일시정지하거나 우선순위를 낮출 수 있는 컨트롤을 제공한다(특히 전경 작업과 경합하는 경우).
- 최소한 "백그라운드에서 N개 작업 진행 중"이라는 요약 뱃지만이라도 상시 노출한다.

**탐지**:
- User test: 백그라운드 작업이 실행되는 동안 사용자에게 "지금 앱이 뭘 하고 있다고 생각하는지" 질문.
- Performance: 사용자 조작이 없는데도 CPU/디스크 I/O가 튀는 구간을 프로파일링해 그 시점에 대응하는 UI 신호가 있었는지 대조.

**Bitvue 판정**: N/A — (재검증) `crates/bitvue-sidecar/src`에서 유일한 `thread::spawn`은 `main.rs`의 `spawn_request`(196행)인데, 이건 사용자가 직접 트리거한 요청 하나당 워커 스레드 하나를 띄워 응답을 계산하는 정상적인 요청-처리 모델이지 프리페치/자동저장/재인덱싱류 fire-and-forget 백그라운드 작업이 아니다(요청을 보낸 프론트엔드가 응답을 기다리므로 "사용자가 트리거하지 않은 작업"이 아님). `grep -rln prefetch frontend crates`로 확인한 결과 프리페치 관련 코드는 벤더 abseil 크레이트의 무관한 매치 1건뿐이고, 이전에 존재했던 `get_or_decode_frame_with_prefetch`류 코드는 `src-tauri` 삭제와 함께 사라졌다. `crates/bitvue-engine/src/worker.rs`에 `Job::DecodeThumbnails`/`Job::BuildPlotLOD` 등 개념적으로 백그라운드성인 잡 타입이 정의돼 있지만(004/009 판정 참고), `AsyncJobManager`는 `core.rs`/`bitvue-sidecar`에서 전혀 호출되지 않는 미배선 죽은 서브시스템이라 실제로 실행되는 백그라운드 작업이 아니다. 신고할 대상 자체가 없다.

---

### UIX-ASYNC-011: 여러 작업의 progress를 하나로 합침

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: Code

**사용자 목표**:
여러 개의 독립적인 분석(예: 여러 메트릭을 동시에 계산, 또는 여러 파일을 배치로 분석)이 진행될 때, 각각이 얼마나 진행됐는지 개별적으로 알고 싶다.

**증상**:
- 하나의 progress bar가 "전체 작업 중 몇 %"만 보여주는데, 그 안에 서로 다른 3개 메트릭(PSNR, SSIM, VMAF)이 섞여 있어서 어느 것이 끝났고 어느 것이 아직인지 알 수 없다.
- 한 메트릭만 유독 느린 경우(예: VMAF가 PSNR보다 훨씬 느림), 전체 progress가 오랫동안 정체된 것처럼 보인다(UIX-ASYNC-004와 유사한 체감이지만 원인은 다중 작업 합산).
- 개별 작업 중 하나만 실패해도 전체 progress bar가 어떻게 반응해야 할지 정의되어 있지 않다.

**원인**:
프론트엔드가 여러 개의 독립적인 `request_id`/작업을 하나의 스칼라 progress 값으로 조기에 뭉쳐서(reduce) 표시하며, 그 과정에서 개별 작업의 정체성이 사라진다.

**구현 냄새**:
- 여러 백엔드 작업의 progress 이벤트를 단순 평균/합산해 단일 progress bar state로 합치는 reducer가 있다.
- UI 컴포넌트가 작업 목록이 아니라 "progress: number" 하나만 props로 받는다.
- 작업별 실패를 구분해서 표시할 수 있는 자료구조(예: `Map<TaskId, TaskState>`)가 없다.

**영향**:
여러 작업 중 어느 것이 병목인지, 어느 것이 실패했는지 사용자가 진단할 수 없어 "그냥 전체가 느리다/이상하다"는 막연한 불만으로 이어지고, 실제 문제(예: VMAF만 유독 느림)를 개선할 신호도 얻기 어렵다.

**권장**:
- 독립적인 작업은 개별 progress를 유지하고, UI에는 리스트/그리드 형태로 각 작업의 상태를 병렬 표시한다(전체 요약은 보조적으로만 제공).
- 전체 요약치가 필요하다면 "N개 중 M개 완료" 같이 합산 방식을 명확히 밝히고, 실패한 개별 작업은 요약에서도 구분되는 색/아이콘을 쓴다.
- `AnalysisViewState`를 작업당 하나씩 관리하는 컬렉션으로 모델링하고, 화면은 이 컬렉션을 렌더링한다.

**탐지**:
- Code 리뷰: progress 상태를 합산하는 reducer/selector를 찾아 개별 작업 식별자가 보존되는지 확인.
- 시나리오 테스트: 여러 작업 중 하나만 의도적으로 느리게/실패하게 만들어 UI가 이를 구분해 보여주는지 검증.

**Bitvue 판정**: Suspected — (재검증, 근거가 더 약해짐) 정확히 서술된 "여러 progress 값을 reduce하는 로직"은 없다(애초에 progress 값 자체가 없으므로, 004/009 판정 참고). 이전 판정이 유일한 증거로 삼았던 `QualityMetricsPanel.tsx`의 `handleCalculate`(PSNR+SSIM을 `calculate_quality_metrics` 단일 호출 뒤 `isCalculating` boolean 하나로 묶는 패턴)는 재확인 결과 앱 어디서도 렌더링되지 않는 도달 불가 죽은 코드이자, 호출 대상 커맨드 자체가 현재 `bitvue-sidecar`에 없다(009 판정 참고) — 라이브 경로에서의 증거는 아니다. 이번 재검증 범위에서 여러 독립 작업(예: 프레임 청크 로딩 + 썸네일 배치 로딩, `FileStateContext.tsx`의 `refreshFrames`)이 하나의 boolean/progress로 뭉개지는 라이브 사례를 새로 찾지는 못했다 — 다만 전체 프론트엔드를 전수조사한 것은 아니라 신호 부족으로 Suspected 유지, 확정도 배제도 하지 않는다.

---

### UIX-ASYNC-012: 재시도 중이라는 사실을 표시하지 않음

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: Code

**사용자 목표**:
일시적 오류(예: 파일 잠금 경합, 디코더 초기화 실패)로 백엔드가 내부적으로 재시도하고 있다면, 그 사실과 시도 횟수를 알고 싶다 — 계속 같은 로딩 화면만 보고 있으면 "그냥 오래 걸리는 것"과 "여러 번 실패하고 있는 것"을 구분할 수 없다.

**증상**:
- 백엔드가 exponential backoff로 3~5회 재시도하는 동안 프론트엔드는 처음과 동일한 단일 로딩 스피너만 보여준다.
- 재시도가 결국 성공하면 사용자는 지연 이유를 전혀 모른 채 "그냥 느렸다"고 생각하고, 재시도가 결국 실패하면 마치 "한 번의 시도가 실패한 것"처럼 보여 실제로는 훨씬 심각한 상황(계속 실패)임을 과소평가한다.

**원인**:
재시도 로직이 백엔드 내부에 캡슐화되어 있고, 그 사실을 프론트엔드로 전달하는 이벤트 채널이 설계되지 않았다.

**구현 냄새**:
- Rust 쪽 재시도 루프(`for attempt in 0..MAX_RETRIES`)가 로그(`tracing::warn!`)만 남기고 Tauri 이벤트를 emit하지 않는다.
- 프론트엔드 로딩 상태에 시도 횟수(`attempt: u32`), 마지막 실패 사유 같은 필드가 없다.

**영향**:
사용자가 실제 시스템 상태(예: 파일이 다른 프로세스에 의해 잠겨 반복 경합 중)를 인지하지 못해 문제를 스스로 해결할 기회(예: 다른 프로그램 종료)를 놓치고, 재시도 끝에 실패했을 때 받는 에러 메시지가 상황의 심각성과 맞지 않게 느껴진다.

**권장**:
- 재시도 시도마다 진행 상황 이벤트를 emit하고("N번째 시도 중..."), 프론트엔드는 이를 로딩 상태의 sub-label로 노출한다.
- 재시도 횟수가 일정 이상이면 UI 톤을 격상시켜(단순 스피너 → 경고성 문구) 사용자가 문제를 인지하게 한다.
- 최종 실패 시 에러 메시지에 "N회 재시도 후 실패"와 마지막 실패 사유를 포함한다.

**탐지**:
- Code 리뷰: 백엔드 재시도 루프를 찾아 progress/status 이벤트 emit 여부를 확인.
- 시나리오 테스트: 의도적으로 일시적 실패를 주입(예: 파일에 짧은 lock을 걸어둠)해 재시도 중 UI 변화를 관찰.

**Bitvue 판정**: N/A — (재검증) 이전 판정이 인용한 `src-tauri/src/error.rs::BitvueError::is_retryable()`는 `src-tauri` 삭제와 함께 사라졌다(현재 `is_retryable` 문자열은 워크스페이스 전체 grep 결과 0건). 대신 `crates/bitvue-engine/src/command_chain.rs`에 exponential-backoff 재시도 스캐폴드(`_retry_delay`(563행)/`_should_retry`(570행), `with_retry(max_retries, base_delay_ms)`(740행))가 실제로 존재한다는 새 사실을 확인했다 — 다만 메서드명이 `_` 접두사(Rust의 "미사용" 관례)이고, 코드 주석 자체가 "for future use", "In a real scenario, you'd need async/await for actual retry logic"라고 명시해 미완성 스텁임을 자백한다. `grep -rln with_retry crates`로 확인한 결과 `lib.rs`의 re-export 외에는 아무도 `with_retry`/`RetryHandler`를 호출하지 않는다 — `core.rs`/`bitvue-sidecar` 어디서도 실제로 재시도가 수행되지 않으므로, "재시도 중임을 숨긴다"는 문제가 성립할 실행 경로 자체가 없다.

---

### UIX-ASYNC-013: timeout을 일반 실패로 표시

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: Code

**사용자 목표**:
작업이 "명백히 잘못돼서(예: 파일 손상, 지원하지 않는 코덱) 실패"한 것인지, "너무 오래 걸려서 시간 제한에 걸려 중단"된 것인지 구분해서 알고 싶다 — 대응 방법이 완전히 다르기 때문이다.

**증상**:
- 매우 큰 파일이나 복잡한 비트스트림을 분석할 때 내부 timeout(예: 30초)에 걸려 중단됐는데, 에러 메시지는 "분석 실패: unknown error" 또는 일반적인 파싱 에러 메시지와 동일하게 뜬다.
- 사용자가 파일이 손상됐다고 오인하고 파일을 재인코딩하거나 다른 도구로 재검증하는 등 불필요한 작업을 한다.
- 실제로는 같은 파일을 timeout을 늘려 재시도하면 성공했을 상황이다.

**원인**:
에러 처리 계층이 timeout 에러를 별도 variant로 구분하지 않고 일반 `Err(String)`으로 뭉뚱그려 프론트엔드에 전달한다.

**구현 냄새**:
- Rust 쪽 에러 타입이 `enum AnalysisError { ParseError, DecodeError, ... }`처럼 세분화되어 있어도 `Timeout`이라는 variant가 없거나, 있어도 프론트엔드에서 별도 처리하지 않고 동일한 에러 배너로 렌더링한다.
- `tokio::time::timeout` 사용 지점에서 `Elapsed` 에러를 다른 에러와 동일하게 `map_err(|e| e.to_string())`로 문자열화해버려 타입 정보가 소실된다.

**영향**:
사용자가 잘못된 결론(파일 손상)에 도달해 시간을 낭비하고, 실제로 필요한 조치(더 긴 timeout으로 재시도, 더 작은 구간으로 나눠 분석)를 시도하지 못한다.

**권장**:
- 에러 타입에 `Timeout { elapsed: Duration, stage: String }` 같은 명시적 variant를 두고, 문자열화하기 전에 타입 정보를 프론트엔드까지 보존한다(`UiError`가 구조화된 discriminated union이어야 함).
- Timeout 에러 UI에는 일반 실패와 다른 액션(재시도, timeout 늘리기, 구간 분할 분석)을 제공한다.
- Timeout 임계값 자체가 파일 크기/코덱 복잡도에 비해 너무 짧게 고정되어 있지 않은지 별도로 검토한다.

**탐지**:
- Code 리뷰: 에러 처리 경로에서 `Elapsed`/`timeout` 관련 에러가 문자열화되기 전 어디서 타입이 소실되는지 추적.
- 시나리오 테스트: 의도적으로 매우 느린 입력(대형 파일, 인위적 지연 주입)으로 timeout을 유발해 사용자에게 노출되는 메시지를 검증.

**Bitvue 판정**: N/A — (재검증, 결론은 그대로) `src-tauri` 삭제 이후에도 여전히 유효: 워크스페이스 전체(`crates/`, `vendor` 제외)에 `tokio::time::timeout`/`time::Elapsed` 사용이 전혀 없다(grep 확인). 현재 프로토콜의 에러 표현인 `bitvue-protocol`의 `WireErrorCode` enum(`Io`/`Parse`/`InvalidObuType`/`UnexpectedEof`/`UnsupportedCodec`/`Decode`/`InsufficientData`/`InvalidData`/`InvalidFile`/`InvalidRange`/`FileModified`/`FrameNotFound`/`NotFound`/`Serialization`/`Cancelled`/`Internal`)에도 `Timeout` variant가 없다(참고로 `Cancelled`는 006/003 판정에서 확인한 `cancel_request` 전용 코드로 이미 존재함 — timeout과는 별개). 내부 timeout 메커니즘 자체가 아직 구현되어 있지 않으므로, 그것을 일반 에러로 뭉뚱그릴 코드 경로도 없다.

---

### UIX-ASYNC-014: 캐시 결과와 새 계산 결과를 구분하지 않음

**분류**: UIX_ASYNC · **심각도**: Low · **탐지**: User test

**사용자 목표**:
화면에 즉시 뜬 결과가 "방금 다시 계산된 신선한 값"인지 "이전에 계산해 캐시해둔 값"인지 알고 싶다 — 특히 캐시 무효화 로직을 완전히 신뢰하기 전까지는, 순간적으로 뜨는 결과에 대해 "혹시 오래된 캐시 아닌가?"라는 합리적 의심을 할 수 있어야 한다.

**증상**:
- 같은 프레임을 다시 선택했을 때 즉시(수 ms) 결과가 뜨는 경우와, 처음 계산할 때 수 초 걸리는 경우가 UI상 전혀 구별되지 않는다.
- 파일이나 설정이 바뀐 뒤에도 캐시가 잘못 무효화되지 않아 오래된 값이 그대로 "즉시" 뜨는 경우, 사용자는 그것이 새로 계산된 최신 값이라고 믿을 수밖에 없다(구분할 시각적 단서가 없으므로).

**원인**:
캐시 히트/미스가 프론트엔드에 노출되는 신호 없이 완전히 백엔드 내부 구현 디테일로 숨겨져 있다.

**구현 냄새**:
- 캐시 조회 결과와 신규 계산 결과가 동일한 Tauri 커맨드 반환 타입을 쓰며, 메타데이터(예: `from_cache: bool`, `computed_at: Timestamp`)가 없다.
- 캐시 무효화 조건(파일 변경, 설정 변경, 코덱 버전 변경 등)이 명시적으로 테스트되지 않아, 실제로 stale 캐시가 반환될 위험이 존재하는데도 사용자가 이를 검증할 UI 단서가 없다.

**영향**:
캐시 로직에 버그가 있을 경우(가장 흔한 캐시 무효화 실수) 사용자가 이를 알아챌 방법이 없어 문제가 오래 방치되고, 신뢰도가 중요한 분석 도구에서 "이 숫자를 믿어도 되나"라는 근본적 의심으로 이어질 수 있다.

**권장**:
- 결과에 `from_cache`와 `computed_at`(또는 캐시 생성 시각) 메타데이터를 실어 전달하고, UI에 작은 아이콘/툴팁으로 "캐시된 결과 (계산: 3분 전)"를 표시한다.
- 사용자가 명시적으로 "강제 재계산"할 수 있는 액션을 제공한다.
- 캐시 무효화 로직에 대한 회귀 테스트를 별도로 강화한다(이 항목 자체의 권장이라기보다 연관 CACHE 카테고리와 교차 참조).

**탐지**:
- User test: 캐시 히트/미스 상황을 각각 재현해 사용자가 그 차이를 인지할 수 있는지 질문.
- Code 리뷰: 캐시 조회 반환 타입에 출처 메타데이터가 있는지 확인.

**Bitvue 판정**: N/A — (재검증, 이전 Confirmed 판정을 뒤집음) 이전 판정이 인용한 `src-tauri/src/commands/thumbnails.rs`의 `cached: bool` 필드는 `src-tauri` 삭제와 함께 사라졌다. 현재 `bitvue-sidecar`의 `get_thumbnails`(`crates/bitvue-sidecar/src/main.rs:1479` → `decode_bridge::get_thumbnails`)를 직접 읽어보면, 이 함수는 호출될 때마다 `Av1Decoder::new()`로 완전히 새 디코더를 만들어 원본 바이트를 처음부터 다시 디코드한다(`decode_bridge.rs:160행 부근`) — 요청 간에 재사용되는 캐시 레이어 자체가 없다. 응답 JSON도 `frame_index`/`thumbnail_data`/`width`/`height`/`success`뿐, 캐시 출처를 나타내는 필드가 없다(main.rs:1533-1544). `getDecodedFrameYuv`도 `electronBridgeService.ts`의 모듈 문서가 "re-decodes from the stream start every call, no session caching yet"라고 명시한다. 즉 이 항목이 전제하는 "캐시 히트/미스를 구분 못 함"이라는 문제 자체가 성립하지 않는다 — 캐시 메커니즘이 아예 없으므로 모든 응답이 균일하게 "새로 계산됨"이고, 감출 캐시 출처 신호가 없다.

---

### UIX-ASYNC-015: loading 중 기존 데이터를 모두 비워 깜빡임 발생

**분류**: UIX_ASYNC · **심각도**: Medium · **탐지**: Visual

**사용자 목표**:
프레임을 하나씩 이동하거나 설정을 미세 조정할 때, 화면이 매번 완전히 빈 상태로 리셋됐다가 다시 채워지지 않고 이전 내용을 유지한 채 부드럽게 갱신되길 기대한다.

**증상**:
- 다음 프레임으로 이동할 때마다 그래프/썸네일/메트릭 패널이 순간적으로 완전히 빈 화면(또는 스피너)이 됐다가 새 데이터로 다시 그려진다.
- 빠르게 연속 탐색(화살표 키로 프레임 이동 등)하면 화면이 계속 깜빡여(flicker) 눈이 피로하고, 방금까지 보던 값이 순간적으로 사라져 비교가 어렵다.
- 특히 요청이 아주 빨리 끝나는 경우(수십 ms)에도 "빈 화면 → 채움" 사이클이 매번 강제로 발생해 실제로는 불필요한 깜빡임이다.

**원인**:
로딩 상태 전환 시 `Loading` 국면을 구현하면서 이전 데이터(`previous`)를 유지하지 않고 무조건 초기화(`data = null`)하는 방식으로 구현했다. `AnalysisViewState::Loading { previous }`처럼 이전 값을 들고 있을 수 있는 모델인데도 프론트엔드가 이를 활용하지 않는다.

**구현 냄새**:
- 요청 시작 시 `setData(null)` 또는 `setData(undefined)`를 먼저 호출한 뒤 요청 완료 시 새 값을 설정하는 패턴.
- 렌더링 컴포넌트가 `data === null`이면 무조건 스피너/빈 상태를 그리고, `previous` 값이라는 개념 자체가 상태 모델에 없다.
- 매우 빠르게 끝나는 요청에도 최소 표시 시간 없이 로딩 UI가 강제로 한 프레임이라도 끼어들어 깜빡임을 유발한다.

**영향**:
탐색/미세조정 워크플로(프레임 단위 이동, 파라미터 슬라이더 조정)의 체감 반응성이 크게 떨어지고, 매 전환마다 이전 값과 새 값을 시각적으로 비교하기 어려워져 분석 효율이 저하된다.

**권장**:
- 새 요청 시작 시 기존 데이터를 지우지 말고 `Loading { previous }`로 전환해 이전 값을 흐리게(opacity 감소) 유지한 채 새 데이터를 기다린다.
- 새 데이터가 도착하면 즉시 교체하고, 중간 로딩 인디케이터는 일정 시간(예: 150~300ms) 이상 걸릴 때만 표시해 빠른 응답에서는 깜빡임 자체가 없게 한다.
- 값이 실제로 바뀌는 부분만 시각적으로 강조(예: 짧은 하이라이트 트랜지션)해 "갱신됐다"는 신호는 주되 전체를 리셋하지는 않는다.

**탐지**:
- Visual 회귀 테스트: 연속 프레임 이동 시나리오를 녹화해 프레임 간 빈 화면(blank frame) 발생 여부를 자동 검출.
- 시나리오 테스트: 매우 빠른 응답(mock으로 0ms 지연)에서도 로딩 UI가 최소 1프레임이라도 나타나는지 확인.

**Bitvue 판정**: Suspected — (재검증, 두 근거 모두 현재도 라이브 코드에서 재확인됨) 엇갈린 근거가 있다. 메인 프레임 뷰어(`frontend/components/panels/YuvViewerPanel/index.tsx`)는 프레임 전환 시 기존 이미지를 지우지 않고 그 위에 "Loading frame N..." 오버레이만 얹는 방식(579행)이라 이 항목이 우려하는 완전 초기화형 깜빡임은 피하고 있다(단, 150~300ms 최소 표시 시간 같은 디바운스는 없어 매우 빠른 응답에서도 오버레이가 최소 1프레임 끼어들 수 있음). 반면 `frontend/contexts/FileStateContext.tsx`의 `refreshFrames`(87-94행, `useAppFileOperations.ts`의 `openFileAtPath`가 실제로 호출하는 라이브 경로)는 새 파일을 불러오기 시작할 때 93행에서 `setFrames([])`를 먼저 호출해 이전 프레임 목록을 즉시 비운다(88행 `setLoading(true)`, 94행 `setStreamInfo(null)`도 동시에) — 파일 재적재 시나리오에서는 이 항목이 설명하는 깜빡임이 실제로 발생할 수 있다.
</content>
</invoke>
