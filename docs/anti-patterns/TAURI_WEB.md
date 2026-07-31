# Anti-Pattern Catalog — TAURI_WEB: WebView와 Native 경계

이 문서는 Bitvue 안티패턴 카탈로그의 한 갈래이며, 전체 카탈로그의 색인은 별도로 작성 중인 `docs/anti-patterns/INDEX.md`를 참고한다. UI/UX + Tauri를 다루는 Phase 3 묶음에 속하며, 같은 묶음의 `TAURI_CMD.md`(커맨드 설계·IPC 계약), `TAURI_EVT.md`(이벤트·구독 생명주기)와 형제 문서다. 향후 작성될 수 있는 Phase 4 `PLAT.md`(파일시스템 대소문자 구분, OS별 프로세스/권한 모델 등 더 넓은 크로스플랫폼 데스크톱 이슈)와는 달리, 이 문서는 "WebView 프런트엔드가 native OS 기능(파일 시스템, 다이얼로그, 메뉴, 창, DPI)과 만나는 경계"에 국한한다.

---

### TAURI-WEB-001: OS 파일 경로를 UI 문자열처럼 직접 사용

**분류**: 파일 경로 처리 · **심각도**: High · **탐지**: Code

**나쁜 예**:
```typescript
// 프런트가 백엔드에서 받은 절대 경로를 그대로 문자열 조작
function getFileName(path: string): string {
  return path.split('/').pop() ?? path;
}

function getParentDir(path: string): string {
  const parts = path.split('/');
  parts.pop();
  return parts.join('/');
}

async function openSiblingFile(currentPath: string, name: string) {
  const dir = getParentDir(currentPath);
  await invoke('open_bitstream', { path: `${dir}/${name}` });
}
```

**문제**:
- `/` 구분자를 하드코딩하면 Windows 경로(`C:\Users\...\clip.hevc`)에서 파일명 추출과 부모 디렉터리 계산이 통째로 깨진다.
- 문자열 결합으로 경로를 재조립하면 중복 구분자, trailing slash, `..` 정규화 같은 경계 케이스가 플랫폼마다 다르게 실패한다.
- 드라이브 문자(`C:`), UNC 접두사(`\\server\share`), macOS `/Volumes/...` 마운트 경로 등 플랫폼 고유 표현을 프런트가 이해하지 못한 채 다룬다.
- 표시용 문자열과 실제 파일시스템 경로를 구분하지 않아, 표시를 위해 잘라낸 문자열을 다시 IPC 인자로 재사용하는 실수가 생긴다.

**발생 조건**:
- Windows에서 비트스트림 파일을 열거나, 최근 파일 목록에서 같은 폴더의 다른 파일(예: `.hevc` 옆의 `.json` 메타데이터)을 자동으로 찾을 때.
- 사용자가 UNC 경로(네트워크 드라이브)나 이동식 디스크에서 파일을 열 때.
- macOS/Linux에서 개발하고 테스트한 경로 처리 로직을 Windows CI 없이 그대로 배포할 때.

**권장**:
```rust
#[tauri::command]
fn get_path_info(path: String) -> Result<PathInfo, String> {
    let p = std::path::Path::new(&path);
    Ok(PathInfo {
        file_name: p.file_name().map(|s| s.to_string_lossy().to_string()),
        parent: p.parent().map(|s| s.to_string_lossy().to_string()),
        extension: p.extension().map(|s| s.to_string_lossy().to_string()),
    })
}

#[tauri::command]
fn resolve_sibling(path: String, name: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    let parent = p.parent().ok_or("no parent")?;
    Ok(parent.join(&name).to_string_lossy().to_string())
}
```
- 경로 파싱/조합은 항상 Rust `std::path::Path`/`PathBuf`로 native 측에서 수행하고, 프런트에는 이미 계산된 필드(파일명, 확장자, 표시용 축약 경로)만 전달한다.
- 프런트는 경로를 "불투명한 식별자"로 취급해 저장·재전달만 하고, 절대 자체적으로 잘라 붙이지 않는다.
- 표시용 경로(줄임표 처리 등)와 IPC용 원본 경로를 별도 필드로 분리해 혼동을 막는다.

**탐지 방법**:
- Code: 프런트엔드 코드에서 문자열 리터럴 `'/'`가 포함된 `.split()`/`.join()`/템플릿 리터럴 경로 조합 패턴을 grep.
- Structural: 파일 경로를 인자로 받는 IPC 커맨드 중 프런트에서 가공 없이 그대로 넘기는지, 아니면 문자열 연산을 거치는지 호출부 검토.

**예외**:
- 표시 전용으로 이미 native 측에서 정규화되어 내려온 "짧은 파일명" 필드를 UI에 그대로 렌더링하는 것은 문제없다(재분해하지 않는 한).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-002: Windows 경로·Unicode·UNC 처리 누락

**분류**: 파일 경로 처리 · **심각도**: High · **탐지**: Code / Domain review

**나쁜 예**:
```rust
#[tauri::command]
fn open_bitstream(path: String) -> Result<FileHandle, String> {
    std::fs::File::open(&path).map_err(|e| e.to_string())?;
    // path.len() > 260 이면 Windows에서 여기서 이미 실패했을 수 있음
    let name = path.rsplit('\\').next().unwrap_or(&path); // 유닉스 개발자가 뒤늦게 끼워넣은 임시 처리
    Ok(FileHandle { display_name: name.to_string() })
}
```

**문제**:
- Windows의 `MAX_PATH`(260자) 제약을 고려하지 않으면 깊은 폴더 구조에 저장된 대용량 캡처 파일을 열 때 이유를 알 수 없는 실패가 발생한다(`\\?\` 확장 경로 prefix 미사용).
- 파일명에 non-ASCII(한글, 이모지, 결합 문자)가 포함되면 OS별 정규화 형태(NFC vs NFD)가 달라 macOS에서 생성한 파일명을 Windows/Linux에서 바이트 단위로 비교하면 불일치한다.
- UNC 경로(`\\server\share\clip.ivf`)나 네트워크 드라이브는 `Path::parent()`/`exists()` 동작이 로컬 경로와 미묘하게 다르고, 지연이나 일시적 끊김에 대한 재시도 처리가 없으면 분석 도중 조용히 멈춘다.
- `path.rsplit('\\')`처럼 특정 OS 구분자를 하드코딩한 임시 처리가 다른 OS 코드 경로에 섞여 들어가면 크로스플랫폼 동작이 OS마다 달라진다.

**발생 조건**:
- Windows에서 딥 폴더 구조(예: 사내 공유 드라이브의 프로젝트/날짜/코덱별 하위 폴더)에 있는 캡처 파일을 열 때.
- 파일명에 한글/일본어 자모가 들어간 샘플을 macOS에서 만들어 Windows로 옮겨 열 때.
- 사내 NAS를 SMB로 마운트한 환경에서 대용량 원본 비트스트림을 직접 분석할 때.

**권장**:
```rust
use std::path::{Path, PathBuf};

fn normalize_for_platform(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    #[cfg(windows)]
    {
        // 긴 경로 지원이 필요하면 \\?\ prefix 부여 (canonicalize가 처리해줄 때도 있음)
        if let Ok(canon) = p.canonicalize() {
            return canon;
        }
    }
    p
}
```
- 경로 정규화는 `Path`/`PathBuf` API와 `dunce` 같은 크레이트(Windows에서 `\\?\` prefix를 표시용으로 안전하게 제거)를 활용해 OS 분기를 한 곳에 모은다.
- 파일명 비교가 필요하면 Unicode 정규화(NFC)를 명시적으로 거친 뒤 비교한다.
- CI에 Windows(딥 경로 픽스처 포함)와 non-ASCII 파일명 픽스처를 반드시 포함한다.

**탐지 방법**:
- Code: `\\` 또는 `/` 리터럴을 조건 없이 사용하는 경로 처리 코드 grep, `#[cfg(windows)]` 분기 유무 확인.
- Domain review: Windows/UNC/Unicode 파일명 픽스처로 "파일 열기 → 분석 → 저장" 전체 플로우를 수동 검증.

**예외**:
- 애초에 배포 대상이 단일 OS(예: 사내 리눅스 워크스테이션 전용 빌드)로 한정된 경우 우선순위를 낮출 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-003: 파일 drag-and-drop 중복 처리

**분류**: 파일 입력 · **심각도**: Medium · **탐지**: Interaction / Code

**나쁜 예**:
```typescript
// 브라우저 표준 드래그앤드롭 리스너
window.addEventListener('drop', async (e) => {
  e.preventDefault();
  const file = e.dataTransfer?.files[0];
  if (file) {
    await openBitstream(file.path); // File.path는 Tauri webview에서만 존재
  }
});

// 동시에 Tauri의 네이티브 file-drop 이벤트도 구독
listen('tauri://file-drop', (event) => {
  const [path] = event.payload as string[];
  openBitstream(path); // 같은 파일이 두 번 열림
});
```

**문제**:
- 웹 표준 `drop` 이벤트와 Tauri의 native `tauri://file-drop` 이벤트가 동시에 등록되면 같은 드롭 동작에 대해 두 핸들러가 모두 실행되어 파일이 중복으로 열리거나 로딩 상태가 두 번 토글된다.
- `File.path`는 표준 웹 API에 없는 Tauri 전용 확장 필드라서, 웹 코드와 데스크톱 코드를 같은 컴포넌트에서 분기 없이 섞으면 타입 안전성이 깨지고 브라우저 프리뷰(스토리북 등)에서 동작이 달라진다.
- 두 경로 모두 파일 크기/개수 제한, 지원 확장자 검증 로직을 각자 구현하면 검증 로직이 갈라져 한쪽만 패치되는 경우가 생긴다.
- 드롭 존이 여러 패널(파일 트리, 타임라인, hex view)에 걸쳐 있을 때 이벤트 버블링/캡처 순서에 따라 어느 패널이 "먼저" 처리했는지 예측 불가능해진다.

**발생 조건**:
- 웹 프로토타입 코드를 Tauri 앱으로 이식하면서 기존 `drop` 리스너를 제거하지 않고 native 리스너만 추가한 경우.
- 여러 패널이 각자 독립적으로 드롭을 처리하도록 구현되어 있고 전역 드롭 핸들러가 없는 경우.
- 대용량 파일 드롭 시 검증(확장자, 크기)이 비동기라서 두 핸들러의 응답 타이밍이 겹치는 경우.

**권장**:
```typescript
// 단일 진입점: Tauri native file-drop만 신뢰하고, 표준 drop은 preventDefault로 무력화
window.addEventListener('dragover', (e) => e.preventDefault());
window.addEventListener('drop', (e) => e.preventDefault()); // 브라우저 기본 동작(파일을 새 탭으로 여는 것 등)만 차단

const unlisten = await listen<string[]>('tauri://file-drop', async (event) => {
  const paths = event.payload;
  await handleDroppedFiles(paths); // 검증·중복 제거를 한 곳에서
});
```
- 파일 드롭 처리 경로를 하나로 통일하고(Tauri 앱이면 native 이벤트를 단일 소스로), 표준 `drop`은 브라우저 기본 동작 방지 용도로만 남긴다.
- 드롭 존이 여러 개라면 활성 드롭 존을 명시적 상태로 관리해 이벤트를 어디로 라우팅할지 결정한다.
- 확장자/크기 검증 로직을 공용 함수로 추출해 단일 지점에서만 호출한다.

**탐지 방법**:
- Code: `dataTransfer.files`와 `tauri://file-drop` 리스너가 같은 스코프/컴포넌트 트리에 동시 등록되는지 grep.
- Interaction: 파일을 드롭했을 때 로딩 인디케이터가 두 번 깜빡이거나 최근 파일 목록에 같은 항목이 중복 추가되는지 수동 확인.

**예외**:
- 웹 빌드와 데스크톱 빌드를 동시에 지원해야 하는 경우, 플랫폼 감지 후 조건부로 한쪽만 활성화하는 것은 정당한 분기다(중복 등록이 아니라면).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-004: native dialog와 web modal의 동작 불일치

**분류**: 다이얼로그 통합 · **심각도**: Medium · **탐지**: Interaction / Visual

**나쁜 예**:
```typescript
async function confirmCloseUnsavedSession() {
  // "저장 안 함" 확인은 커스텀 웹 모달
  const confirmed = await showWebConfirmModal({
    title: '저장하지 않은 분석 세션이 있습니다',
    message: '닫으시겠습니까?',
  });
  return confirmed;
}

async function pickExportPath() {
  // 내보내기 경로 선택은 native dialog
  return await dialog.save({ filters: [{ name: 'JSON', extensions: ['json'] }] });
}
```

**문제**:
- 같은 앱 안에서 어떤 확인창은 OS 네이티브 룩앤필(제목 표시줄, 포커스 링, 애니메이션)로 뜨고 어떤 확인창은 웹 스타일 모달로 뜨면 사용자가 "이게 진짜 경고인지" 일관되게 인지하기 어렵다.
- 웹 모달은 z-index/포커스 트랩을 앱이 직접 구현해야 하는데, native 다이얼로그가 이미 떠 있는 상태에서 웹 모달을 또 띄우면 포커스 순환이 어느 쪽에도 잡히지 않는 경우가 생긴다.
- 키보드 동작(Esc로 취소, Enter로 확인)이 native와 web 모달에서 다르게 구현되어 있으면(예: 하나는 Esc를 막고 하나는 허용) 같은 제스처에 다른 결과가 나온다.
- 스크린리더 사용자에게 native 다이얼로그는 OS 접근성 트리를 그대로 타지만, 커스텀 웹 모달은 `role="dialog"`/`aria-modal` 구현이 누락되면 완전히 다른(더 나쁜) 경험이 된다.

**발생 조건**:
- "파일을 선택하는 순간"에는 OS 다이얼로그가 필수(Tauri의 파일시스템 접근 정책상)이지만, 그 외 확인/경고는 개발 편의상 웹 모달로 만든 경우.
- 다크 모드에서 native 다이얼로그(OS 테마 추종)와 web 모달(앱 자체 테마)의 색상이 서로 다른 타이밍에 전환되는 경우.
- Esc 키 전역 핸들러가 web 모달 전용으로 등록되어 native 다이얼로그가 떠 있을 때는 반응하지 않는 경우.

**권장**:
- 파일 열기/저장처럼 OS가 강제하는 지점은 native dialog(`@tauri-apps/plugin-dialog`)를 쓰고, 그 외의 앱 내부 확인/경고는 하나의 web 모달 컴포넌트로 통일해 스타일과 키보드 동작을 일관되게 만든다.
- native dialog가 열려 있는 동안에는 web 모달을 큐잉하거나 비활성화해 동시 노출을 막는다(단일 "modal owner" 상태로 관리).
- web 모달에는 `role="alertdialog"`/`aria-modal="true"`와 포커스 트랩을 명시적으로 구현하고, Esc/Enter 동작을 native 관례와 맞춘다.
- 다크/라이트 테마 전환 시 web 모달이 OS 테마 변경 이벤트를 즉시 반영하도록 한다.

**탐지 방법**:
- Interaction: 파일 저장 도중 "저장하지 않은 변경사항" 경고를 트리거해 두 다이얼로그가 겹쳐 뜨는지 확인.
- Visual: 같은 세션에서 native/web 다이얼로그를 나란히 캡처해 테마·폰트·모서리 반경 차이를 비교.
- Code: `dialog.confirm`/`dialog.ask`(native) 호출부와 커스텀 모달 컴포넌트 호출부를 모두 인벤토리화해 사용 기준이 문서화되어 있는지 확인.

**예외**:
- 복잡한 폼 입력(내보내기 옵션 여러 개 선택 등)은 native dialog의 표현력이 부족하므로 web 모달이 정당하다. 이때는 최소한 진입 트리거와 키보드 동작 일관성만 지키면 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-005: native menu 상태와 frontend 상태 불일치

**분류**: 메뉴/상태 동기화 · **심각도**: High · **탐지**: Code / Interaction

**나쁜 예**:
```rust
// 앱 시작 시 한 번만 메뉴 구성, 이후 갱신 없음
fn build_menu() -> Menu {
    Menu::new()
        .add_item(CustomMenuItem::new("export_overlay", "Export Overlay Data"))
        .add_item(CustomMenuItem::new("toggle_mv", "Show Motion Vectors"))
}
```
```typescript
// 프런트에서 오버레이 토글은 별도 상태로만 관리, 메뉴에 반영 안 함
function toggleMotionVectors() {
  setShowMv((prev) => !prev); // 메뉴의 체크마크는 그대로 unchecked
}
```

**문제**:
- 프런트엔드 상태(예: MV 오버레이 on/off, 현재 열린 코덱 타입)가 바뀌어도 native 메뉴의 체크마크/활성화 상태가 갱신되지 않으면, 메뉴만 보고 실제 상태를 오판하게 된다.
- 비디오가 아직 로드되지 않았는데 "Export Overlay Data" 같은 메뉴 항목이 계속 활성화되어 있으면 클릭 후에야 실패 토스트를 보게 되는 나쁜 피드백 루프가 생긴다.
- 메뉴 클릭 이벤트(native → Rust → 프런트로 이벤트 전달)와 프런트 내부 버튼 클릭이 서로 다른 코드 경로로 같은 동작을 트리거하면, 한쪽만 수정하고 다른 쪽을 잊는 드리프트가 반복적으로 발생한다.
- 다중 창(예: 메인 뷰어 + 비교 뷰어)에서 전역 메뉴(특히 macOS의 앱 전체 메뉴바)를 공유할 때, 어느 창의 상태를 메뉴에 반영해야 하는지 기준이 없으면 포커스 전환 시 메뉴가 엉뚱한 창 기준으로 표시된다.

**발생 조건**:
- 키보드 단축키나 사이드바 버튼으로 오버레이를 토글할 수 있는 기능이 메뉴에도 동일 항목으로 노출되어 있는 경우.
- 파일이 열려 있는지 여부에 따라 활성화/비활성화되어야 하는 메뉴 항목(Export, Close, Reload)이 있는 경우.
- macOS에서 여러 창이 하나의 메뉴바를 공유하고 사용자가 Cmd+`(창 전환)으로 포커스를 바꾸는 경우.

**권장**:
```rust
#[tauri::command]
fn sync_menu_state(app: tauri::AppHandle, state: MenuStateDto) -> Result<(), String> {
    let menu_handle = app.get_window("main").unwrap().menu_handle();
    menu_handle.get_item("toggle_mv").set_selected(state.show_mv)?;
    menu_handle.get_item("export_overlay").set_enabled(state.has_open_file)?;
    Ok(())
}
```
```typescript
// 프런트 상태가 바뀔 때마다 메뉴 동기화 커맨드를 호출하는 단일 지점
useEffect(() => {
  invoke('sync_menu_state', { state: { showMv, hasOpenFile } });
}, [showMv, hasOpenFile]);
```
- 메뉴 상태를 프런트 상태의 "파생값"으로 취급해, 상태가 바뀔 때마다 단일 동기화 함수를 통해 native 메뉴를 갱신한다(역방향으로 메뉴 클릭도 같은 상태 변경 함수를 호출).
- 메뉴 클릭 이벤트와 UI 버튼 클릭이 동일한 액션 함수(예터: `toggleMotionVectors()`)를 호출하도록 통일해 로직 중복을 없앤다.
- 다중 창 환경에서는 "현재 포커스된 창"을 기준으로 메뉴 상태를 재계산하는 지점을 명확히 만든다(창 포커스 이벤트 훅).

**탐지 방법**:
- Code: 메뉴 아이템 정의(`CustomMenuItem`)와 프런트 상태 토글 함수가 서로를 참조하는지, 동기화 호출이 존재하는지 추적.
- Interaction: 단축키로 오버레이를 켜고 메뉴를 열어 체크마크가 즉시 반영되는지 확인. 파일을 닫은 후 Export 메뉴가 비활성화되는지 확인.

**예외**:
- 메뉴 항목이 순수 네비게이션(예: "About", "Documentation")이라 프런트 상태와 무관하다면 동기화가 필요 없다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-006: 창 닫기 시 미저장 작업 처리 불명확

**분류**: 창 생명주기 · **심각도**: High · **탐지**: Interaction / Code

**나쁜 예**:
```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window.on_window_event(|event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    // 아무 처리 없이 기본 동작(즉시 종료)에 맡김
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap();
}
```

**문제**:
- 사용자가 오버레이 주석이나 마커를 편집 중인데 `Cmd+Q`/`Alt+F4`/창 닫기 버튼을 누르면 확인 없이 즉시 프로세스가 종료되어 작업 내용이 사라진다.
- `CloseRequested` 이벤트는 비동기 확인(웹 모달 표시 후 사용자 응답 대기)이 필요한데, 이벤트 핸들러 안에서 `event.prevent_close()`를 호출하지 않으면 창은 이미 닫히는 방향으로 진행된 뒤라서 "닫기를 막았다"는 착각만 하고 실제로는 종료된다.
- 여러 창(메인 + 비교 뷰어)이 열려 있을 때 마지막 창을 닫는 것과 개별 창을 닫는 것을 구분하지 않으면, 다른 창에 미저장 작업이 남아 있어도 앱 전체가 종료될 수 있다.
- 분석 세션 저장이 디스크 I/O를 수반하는 경우, 확인 후 저장이 끝나기 전에 창이 먼저 닫히면 저장 파일이 손상되거나 잘릴 수 있다.

**발생 조건**:
- 긴 분석 세션(마커, 주석, 커스텀 오버레이 설정)이 자동 저장되지 않는 상태에서 사용자가 창을 닫으려 할 때.
- macOS에서 `Cmd+Q`로 앱 전체 종료를 시도할 때(개별 창 닫기와 다른 이벤트 경로).
- 다중 창 세션에서 부모/자식 관계 없이 창이 열려 있어 "마지막 창"의 의미가 불분명할 때.

**권장**:
```rust
window.on_window_event(move |event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close(); // 우선 닫기를 막고
        let window = window_handle.clone();
        tauri::async_runtime::spawn(async move {
            if confirm_and_maybe_save(&window).await {
                window.close().unwrap(); // 확인/저장이 끝난 뒤 명시적으로 닫음
            }
        });
    }
});
```
- `prevent_close()`로 기본 종료를 먼저 막고, 미저장 여부를 확인한 뒤 사용자 응답에 따라 명시적으로 `window.close()`를 호출하는 비동기 흐름을 사용한다.
- macOS `Cmd+Q`(앱 종료)와 개별 창 닫기를 같은 확인 로직으로 라우팅하되, "어떤 창에 미저장 작업이 있는가"를 전역 상태로 추적해 종료 전 전부 확인한다.
- 자동 저장(주기적 스냅샷)을 도입해 확인 다이얼로그 없이도 데이터 손실 위험을 줄이는 것을 병행 고려한다.

**탐지 방법**:
- Code: `on_window_event`/`CloseRequested` 핸들러에서 `prevent_close()` 호출 여부와 비동기 확인 흐름 존재 여부를 검토.
- Interaction: 마커를 추가한 뒤 저장하지 않고 창 닫기 버튼, `Cmd+Q`/`Alt+F4`, Dock/작업 표시줄에서 닫기를 각각 시도해 확인창이 매번 뜨는지 확인.

**예외**:
- 세션이 매 변경마다 즉시 디스크에 자동 저장되어 "미저장 상태"가 원천적으로 존재하지 않는 설계라면 확인 다이얼로그가 오히려 불필요한 마찰이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-007: 다중 창에서 backend state를 잘못 공유

**분류**: 다중 창 상태 · **심각도**: Critical · **탐지**: Code / Structural

**나쁜 예**:
```rust
struct AppState {
    decoder: Mutex<Decoder>,
    current_frame_index: Mutex<u32>, // 창이 여러 개여도 단 하나
}

#[tauri::command]
fn seek_frame(state: tauri::State<AppState>, index: u32) -> Result<(), String> {
    *state.current_frame_index.lock().unwrap() = index;
    Ok(())
}
```

**문제**:
- `AppState`가 `tauri::Manager`에 전역으로 등록된 단일 인스턴스이면, 비교 뷰어 창에서 프레임을 탐색할 때 메인 뷰어 창의 `current_frame_index`까지 같이 바뀌어 두 창이 서로의 조작에 영향을 준다.
- 어느 창이 어떤 파일을 열었는지 구분하는 개념이 없으면, 창 A에서 연 HEVC 스트림과 창 B에서 연 AV1 스트림이 같은 `decoder` 인스턴스를 두고 경합해 디코더 내부 상태가 오염된다.
- 창을 닫을 때 해당 창 전용 상태를 정리하는 로직이 없으면, 남은 창들이 참조하는 공유 `Mutex` 안에 이미 유효하지 않은 핸들(닫힌 파일, 해제된 버퍼)이 남아 다음 접근에서 패닉하거나 오작동한다.
- 이벤트 브로드캐스트(`app.emit_all`)로 상태 변경을 알릴 때 창 ID를 싣지 않으면, 모든 창이 다른 창의 이벤트까지 받아 처리하게 되어 TAURI_EVT.md의 이벤트 스코프 문제와 겹쳐 증상이 배가된다.

**발생 조건**:
- 사용자가 같은 비트스트림을 두 창(원본/필터 적용본)으로 나란히 비교할 때.
- 서로 다른 코덱 파일을 여러 창에서 동시에 분석할 때.
- 한 창을 닫고 다른 창에서 이어서 작업을 계속할 때(닫힌 창의 상태가 남아있는지 여부가 드러남).

**권장**:
```rust
struct AppState {
    // 창 label(또는 세션 id)을 키로 하는 맵으로 분리
    sessions: Mutex<HashMap<String, WindowSession>>,
}

struct WindowSession {
    decoder: Decoder,
    current_frame_index: u32,
}

#[tauri::command]
fn seek_frame(state: tauri::State<AppState>, window: tauri::Window, index: u32) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.get_mut(window.label()).ok_or("no session for window")?;
    session.current_frame_index = index;
    Ok(())
}
```
- 창별로 독립적인 세션 상태를 `window.label()`(또는 별도 발급한 세션 id)을 키로 하는 맵에 저장해 창 간 간섭을 원천 차단한다.
- 창 close 이벤트(`WindowEvent::Destroyed`)에서 해당 세션 항목을 명시적으로 제거해 리소스 누수와 오염된 참조를 방지한다.
- 정말로 전역이어야 하는 상태(예: 앱 설정, 최근 파일 목록)와 창별 상태(디코더 인스턴스, 현재 프레임)를 타입 수준에서 분리해 실수로 섞이지 않게 한다.

**탐지 방법**:
- Structural: `AppState` 필드 중 창 label/세션 id로 스코프되지 않은 `Mutex<T>` 필드를 인벤토리화해 "전역이어야 하는가"를 항목별로 검토.
- Code: 창을 2개 이상 열고 각각 다른 파일을 로드한 뒤, 한쪽 조작이 다른 쪽에 영향을 주는지 수동/통합 테스트.

**예외**:
- 앱 설정, 라이선스 상태, 전역 캐시(디코더 바이너리 자체 등)처럼 창과 무관하게 진짜 전역이어야 하는 상태는 공유가 맞다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-008: WebView reload 후 작업이 고아 상태가 됨

**분류**: WebView 생명주기 · **심각도**: Medium · **탐지**: Interaction / Code

**나쁜 예**:
```typescript
// 프런트 상태(현재 프레임, 오버레이 설정)는 React 메모리 상태에만 존재
const [currentFrame, setCurrentFrame] = useState(0);
const [overlaySettings, setOverlaySettings] = useState(defaultOverlay);

// 개발 중 F5/Cmd+R로 WebView를 새로고침하면 위 상태는 전부 초기화되지만
// Rust 쪽 decoder는 여전히 이전 파일을 물고 있는 채로 살아있음
```

**문제**:
- WebView가 reload(개발자 도구에서의 새로고침, 크래시 후 자동 복구, 또는 OS 레벨 GPU 프로세스 재시작에 따른 재로드)되면 프런트엔드 메모리 상태는 완전히 사라지지만, native 측 `AppState`(디코더, 열린 파일 핸들)는 그대로 살아있어 두 쪽이 어긋난다.
- reload 후 프런트가 "파일이 열려있지 않다"고 착각해 초기 화면을 그리는데, 실제로는 백엔드가 여전히 대용량 디코더 리소스를 점유하고 있어 사용자가 다시 파일을 열려고 하면 리소스 충돌이나 중복 로드가 발생할 수 있다.
- 진행 중이던 비동기 작업(백그라운드 프레임 사전 디코딩, 내보내기 진행률 스트림)의 구독이 reload로 끊기면, 해당 작업 자체는 native 쪽에서 계속 실행되며 이벤트를 emit하지만 아무도 듣지 않는 "고아 작업"이 된다.
- 사용자 입장에서는 원인 모를 새로고침(예: 메모리 부족으로 인한 WebView 프로세스 재시작) 한 번으로 분석 세션 전체를 잃는 것처럼 보인다.

**발생 조건**:
- 개발 중 HMR/수동 새로고침을 실제 앱 동작처럼 착각하고 상태 복구 로직을 만들지 않은 경우.
- 대용량 파일 처리로 WebView 프로세스 메모리 압박이 심해 OS가 WebView를 재시작시키는 경우(Windows WebView2에서 드물게 발생).
- 내보내기처럼 오래 걸리는 native 작업 도중 reload가 발생하는 경우.

**권장**:
```rust
#[tauri::command]
fn get_active_session(window: tauri::Window, state: tauri::State<AppState>) -> Option<SessionSnapshot> {
    // reload 직후 프런트가 부팅 시 호출: 이 창에 이미 열려있는 세션이 있으면 스냅샷 반환
    let sessions = state.sessions.lock().unwrap();
    sessions.get(window.label()).map(|s| s.snapshot())
}
```
```typescript
// 앱 부팅 시(reload 포함) native에 "이미 열려있는 세션이 있는지" 항상 확인
useEffect(() => {
  invoke<SessionSnapshot | null>('get_active_session').then((snapshot) => {
    if (snapshot) restoreFromSnapshot(snapshot); // 있으면 복구, 없으면 초기 화면
  });
}, []);
```
- native 상태를 "진실의 원천(source of truth)"으로 두고, 프런트는 부팅 시(reload 포함) 항상 native에 현재 세션 존재 여부를 물어 복구하는 패턴을 기본값으로 삼는다.
- 진행 중인 비동기 작업(내보내기 등)은 진행률을 native 상태에도 기록해두어, reload 후 재구독 시 마지막 진행률부터 이어 보여줄 수 있게 한다.
- 프로덕션 빌드에서는 개발자 도구 새로고침 단축키를 비활성화하거나 확인 다이얼로그를 거치게 해 실수 유발을 줄인다.

**탐지 방법**:
- Interaction: 파일을 연 상태에서 `Cmd+R`/F5로 WebView를 새로고침해 화면이 초기 상태로 돌아가는지, 그 상태에서 다시 파일을 열면 어떤 일이 벌어지는지 확인.
- Code: 프런트 부팅 시퀀스(`main.tsx`/앱 루트)에 native 세션 조회 호출이 존재하는지 확인.

**예외**:
- reload가 곧 "세션 초기화"를 의미하도록 의도적으로 설계했고 native 쪽도 reload 시점에 세션을 함께 정리하는 계약이 명시되어 있다면(즉 고아가 아니라 확실히 정리됨) 문제가 아니다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-009: DPI와 devicePixelRatio 차이 무시

**분류**: 렌더링/좌표계 · **심각도**: High · **탐지**: Visual / Code

**나쁜 예**:
```typescript
// 캔버스에 QP 오버레이를 그릴 때 CSS 픽셀 크기를 그대로 캔버스 픽셀로 사용
function drawOverlay(canvas: HTMLCanvasElement, blocks: Block[]) {
  const ctx = canvas.getContext('2d')!;
  canvas.width = canvas.clientWidth;   // devicePixelRatio 미고려
  canvas.height = canvas.clientHeight;
  for (const b of blocks) {
    ctx.strokeRect(b.x, b.y, b.width, b.height); // 프레임 좌표를 그대로 CSS 픽셀로 취급
  }
}

// 마우스 클릭 좌표로 블록을 찾을 때도 동일하게 스케일 무시
canvas.addEventListener('click', (e) => {
  const block = findBlockAt(e.offsetX, e.offsetY); // HiDPI에서 실제 프레임 좌표와 어긋남
});
```

**문제**:
- HiDPI 디스플레이(macOS Retina, Windows 150%/200% 스케일링)에서 `canvas.width`를 `clientWidth`로만 설정하면 캔버스가 흐릿하게 렌더링되고, 특히 1px 단위 QP/MV 오버레이 격자선이 뭉개져 보인다.
- 클릭 좌표(`offsetX/offsetY`, CSS 픽셀 기준)를 `devicePixelRatio`로 보정하지 않고 프레임의 실제 픽셀 좌표(예: 4K 프레임의 macroblock 위치)와 직접 비교하면, 사용자가 클릭한 블록과 실제 선택되는 블록이 어긋난다.
- 창을 스케일링이 다른 모니터(예: 4K 200% 외장 모니터 → 1080p 100% 내장 디스플레이)로 드래그하면 `devicePixelRatio`가 런타임에 바뀌는데, 이 변화를 리스닝하지 않으면 이동 후에도 이전 스케일 기준으로 계속 렌더링/히트테스트한다.
- Tauri 창 크기 API(`outerSize`, `innerSize`)가 반환하는 값이 논리적 픽셀인지 물리적 픽셀인지 헷갈려서 커스텀 타이틀바나 리사이즈 핸들 히트 영역 계산이 스케일링 배율에 따라 어긋난다.

**발생 조건**:
- HiDPI(Retina, Windows 배율 125%/150%/200%) 환경에서 QP 히트맵, MV 화살표, MB/CU 격자 오버레이를 정밀 좌표로 그릴 때.
- 다중 모니터 환경에서 서로 다른 배율의 디스플레이 사이로 창을 이동할 때.
- 픽셀 단위 정밀 클릭(특정 매크로블록 선택)이 필요한 상호작용에서.

**권장**:
```typescript
function setupCanvas(canvas: HTMLCanvasElement) {
  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  canvas.width = rect.width * dpr;
  canvas.height = rect.height * dpr;
  const ctx = canvas.getContext('2d')!;
  ctx.scale(dpr, dpr); // 이후 그리기 좌표는 CSS 픽셀 단위로 다룰 수 있음

  const mq = matchMedia(`(resolution: ${dpr}dppx)`);
  mq.addEventListener('change', () => setupCanvas(canvas), { once: true }); // dpr 변경 감지 후 재설정
}

function toFrameCoords(offsetX: number, offsetY: number, viewScale: number): FramePoint {
  return { x: offsetX / viewScale, y: offsetY / viewScale }; // 캔버스 CSS 좌표 → 프레임 좌표 별도 변환
}
```
- `devicePixelRatio`를 캔버스 초기화 시 반영하고, `matchMedia('(resolution: ...)')` 또는 `ResizeObserver` + dpr 재확인으로 모니터 이동 시 재설정한다.
- "CSS 픽셀 좌표 ↔ 물리 픽셀 ↔ 프레임/블록 좌표"를 서로 다른 좌표계로 명시하고 변환 함수를 한 곳에 모아, 클릭 히트테스트와 렌더링이 항상 같은 변환을 거치게 한다.
- Tauri 창 크기 API 반환값이 논리/물리 픽셀 중 무엇인지 문서에서 확인하고, 스케일 배율과 곱/나눗셈하는 지점에 주석으로 단위를 명시한다.

**탐지 방법**:
- Visual: Retina/HiDPI 화면과 100% 배율 화면에서 같은 오버레이 스크린샷을 비교해 격자선 흐림/두께 차이 확인.
- Code: `devicePixelRatio`를 참조하지 않는 `canvas.width = clientWidth` 패턴, `offsetX/offsetY`를 프레임 좌표로 직접 사용하는 클릭 핸들러 grep.
- Interaction: 배율이 다른 두 모니터 사이로 창을 드래그한 뒤 오버레이 클릭 정확도 확인.

**예외**:
- 순수 텍스트/DOM 기반 UI(캔버스 없이 CSS로만 그려지는 패널)는 브라우저가 DPI를 자동 처리하므로 해당하지 않는다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TAURI-WEB-010: OS별 shortcut·menu conventions 무시

**분류**: 단축키/메뉴 관례 · **심각도**: Medium · **탐지**: Domain review / Interaction

**나쁜 예**:
```typescript
// 모든 플랫폼에서 동일한 키 조합을 하드코딩
document.addEventListener('keydown', (e) => {
  if (e.ctrlKey && e.key === 's') { // macOS 사용자는 Cmd+S를 기대함
    e.preventDefault();
    saveSession();
  }
  if (e.key === 'Delete') { // macOS 키보드에는 물리적으로 다른 두 개의 삭제 키가 있음(Backspace/Forward Delete)
    deleteMarker();
  }
});
```

**문제**:
- macOS 사용자는 `Cmd` 계열 단축키에 익숙한데 `Ctrl`만 처리하면, 시스템 단축키(예: macOS의 다른 `Ctrl+`조합)와 충돌하거나 아예 반응하지 않는 것처럼 느껴진다.
- 메뉴 표시 형식도 플랫폼 관례를 따라야 한다 — macOS는 앱 메뉴(좌상단 앱 이름 메뉴)에 About/Preferences/Quit이 위치하는 관례가 있고, Windows/Linux는 File 메뉴 안에 Exit, Edit 메뉴 안에 Preferences가 들어가는 관례가 있다. 이를 무시하고 단일 메뉴 구조를 모든 OS에 그대로 배포하면 각 플랫폼 사용자에게 낯설게 느껴진다.
- 삭제 키 하나(`Delete`/`Backspace`)만 처리하면 macOS의 Forward Delete와 일반 Backspace 중 한쪽 키보드 레이아웃에서만 동작한다.
- 창 관리 단축키(macOS `Cmd+W`로 탭/창 닫기, `Cmd+`로 창 전환)를 구현하지 않으면 macOS 사용자가 기대하는 기본적인 창 조작이 먹통이라고 느낀다.
- 접근성 단축키 표기(메뉴 아이템 옆 단축키 힌트)를 플랫폼에 맞는 기호(⌘⇧ vs Ctrl+Shift)로 표시하지 않으면 사용자가 실제 키 조합을 유추하기 어렵다.

**발생 조건**:
- Windows에서 먼저 개발한 뒤 macOS 지원을 나중에 추가하면서 키 매핑을 이벤트 리스너 레벨에서만 패치한 경우.
- 메뉴 구조를 플랫폼별로 분기하지 않고 Tauri `Menu` 빌더를 한 군데서 공용으로 사용한 경우.
- 커스텀 단축키 설정 UI가 있는데 플랫폼 기본값(예: macOS `Cmd+,` = Preferences)을 사용자가 임의로 덮어써도 경고가 없는 경우.

**권장**:
```typescript
const isMac = navigator.platform.toLowerCase().includes('mac');
const mod = isMac ? 'metaKey' : 'ctrlKey';

document.addEventListener('keydown', (e) => {
  if (e[mod] && e.key.toLowerCase() === 's') {
    e.preventDefault();
    saveSession();
  }
});
```
```rust
#[cfg(target_os = "macos")]
fn build_menu() -> Menu {
    Menu::new()
        .add_submenu(Submenu::new("Bitvue", Menu::new()
            .add_native_item(MenuItem::About("Bitvue".into(), Default::default()))
            .add_native_item(MenuItem::Separator)
            .add_item(CustomMenuItem::new("preferences", "Preferences...").accelerator("Cmd+,"))
            .add_native_item(MenuItem::Quit)))
        // ...
}

#[cfg(not(target_os = "macos"))]
fn build_menu() -> Menu {
    Menu::new()
        .add_submenu(Submenu::new("File", Menu::new()
            .add_item(CustomMenuItem::new("preferences", "Preferences").accelerator("Ctrl+,"))
            .add_native_item(MenuItem::Quit)))
        // ...
}
```
- 키보드 modifier는 `metaKey`(macOS `Cmd`)와 `ctrlKey`(Windows/Linux `Ctrl`)를 플랫폼 감지로 분기하거나, Tauri의 accelerator 문자열(`CmdOrCtrl+S`)처럼 플랫폼 중립 표현을 우선 사용한다.
- 메뉴 구조 자체를 `#[cfg(target_os = ...)]`로 분기해 각 OS의 관례(앱 메뉴 위치, Preferences/Quit 배치)를 따른다.
- 삭제류 동작은 `Delete`와 `Backspace` 둘 다 리스닝하고 문맥(선택된 항목이 텍스트 입력 중인지)에 따라 처리한다.
- 단축키 힌트 UI는 플랫폼별 기호(⌘/⌥/⇧ vs Ctrl/Alt/Shift)로 렌더링한다.

**탐지 방법**:
- Domain review: macOS/Windows/Linux 각각에서 저장, 삭제, 창 닫기, Preferences 열기 등 핵심 단축키를 실제로 눌러 기대한 관례대로 동작하는지 체크리스트로 검증.
- Code: `e.ctrlKey`만 검사하고 `e.metaKey` 분기가 없는 키보드 핸들러, OS 분기 없이 단일 `Menu` 빌더만 존재하는지 grep.

**예외**:
- 사내 전용 도구처럼 배포 대상 OS가 하나로 고정되어 있고 앞으로도 확장 계획이 없다면 해당 OS 관례만 따르는 것으로 충분하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
