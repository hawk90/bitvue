# Anti-Pattern Catalog — PLAT: 크로스플랫폼 데스크톱

이 문서는 Bitvue 안티패턴 카탈로그의 한 갈래이며, 전체 색인은 `docs/anti-patterns/INDEX.md`를 참고한다. Phase 4 묶음에 속하며, Wave 3의 `TAURI_WEB.md`(WebView와 native OS 기능이 만나는 Tauri 런타임 경계 — 다이얼로그, 메뉴, 창 생명주기, IPC 상태 공유)와는 범위가 다르다. 이 문서는 Tauri라는 프레임워크와 무관하게 존재하는 더 넓은 OS 수준 문제 — 파일시스템 의미론, 경로 처리, 하드웨어 기능 감지, 배포 아키텍처 — 를 다룬다. PLAT-009(DPI)와 PLAT-014(드래그앤드롭)는 `TAURI_WEB.md`의 동명 항목과 주제가 겹쳐 보일 수 있으나, 여기서는 Tauri의 이벤트 처리 방식이 아니라 그 아래에 있는 OS별 원시 의미론(플랫폼마다 DPI 배율을 얻는 방식 자체가 다르다는 점, OS 드래그앤드롭 프로토콜이 전달하는 데이터 형식 자체가 다르다는 점)에 초점을 맞춘다.

---

### PLAT-001: path를 UTF-8로만 가정

**분류**: 파일 경로 인코딩 · **심각도**: High · **탐지**: Static / Runtime

**나쁜 예**:
```rust
fn display_name(path: &std::path::Path) -> String {
    // 경로가 항상 유효한 UTF-8이라고 가정하고 곧바로 unwrap
    path.to_str().unwrap().to_string()
}

#[derive(serde::Serialize)]
struct RecentFile {
    path: String, // OsString이 아니라 String으로 강제 저장
}

fn add_recent(p: std::path::PathBuf, list: &mut Vec<RecentFile>) {
    list.push(RecentFile { path: p.to_str().unwrap().to_owned() });
}
```

**문제**:
- Linux/macOS 파일시스템의 경로는 사실 임의의 바이트열이며, 유효한 UTF-8이라는 보장이 없다(레거시 EUC-KR로 이름 붙은 파일, 손상된 인코딩, 다른 OS에서 tar로 옮겨온 파일명).
- `to_str().unwrap()`은 non-UTF8 경로를 만나는 즉시 패닉하며, 이 패닉은 파일을 여는 순간이 아니라 "폴더를 스캔하다가" 갑자기 터지므로 원인 파악이 어렵다.
- Windows는 경로가 UTF-16(정확히는 잠재적으로 비쌍 서로게이트를 포함하는 WTF-16)이라 `String`으로의 손실 없는 왕복이 애초에 불가능한 경우가 있다.
- 최근 파일 목록 등을 JSON으로 직렬화할 때 `String` 필드로 강제하면, non-UTF8 경로를 가진 항목이 저장 시점에 조용히 깨지거나 저장 자체가 실패한다.

**발생 조건**:
- 오래된 인코딩(EUC-KR, Shift-JIS)으로 이름 붙은 캡처 파일을 사내 NAS에서 복사해 Linux 워크스테이션에서 열 때.
- 사용자가 Windows에서 이모지나 결합 문자가 포함된 파일명을 만들고 이를 macOS/Linux 빌드로 옮길 때.
- 최근 파일 목록·프로젝트 파일에 경로를 문자열로 직렬화해 저장할 때.

**권장**:
```rust
use std::ffi::OsString;
use std::path::{Path, PathBuf};

// 내부적으로는 항상 PathBuf/OsString을 유지한다.
fn add_recent(p: PathBuf, list: &mut Vec<PathBuf>) {
    list.push(p);
}

// 표시 전용으로만 손실 있는 변환을 명시적으로 허용한다.
fn display_name(path: &Path) -> String {
    path.to_string_lossy().into_owned() // 손실 가능함을 함수명/타입으로 드러냄
}

// 직렬화가 필요하면 손실 없는 인코딩(예: base64 of OsStr bytes)을 별도로 정의한다.
fn serialize_path_lossless(p: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        base64::encode(p.as_os_str().as_bytes())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<u16> = p.as_os_str().encode_wide().collect();
        base64::encode(bytemuck::cast_slice(&units))
    }
}
```
- 파일 경로는 프로세스 내부에서 끝까지 `PathBuf`/`OsString`으로 다루고, `String`으로의 변환은 "표시용"이라는 목적이 분명한 지점에서만 `to_string_lossy()`로 명시적으로 수행한다.
- 영속 저장(프로젝트 파일, 최근 목록)이 필요한 경로는 손실 없는 인코딩을 별도로 설계하거나, 최소한 non-UTF8 경로를 만났을 때의 저하 동작(항목 스킵 + 경고)을 명시적으로 정의한다.
- `unwrap()`이 아니라 `to_str()`의 `Option`을 항상 처리한다.

**탐지 방법**:
- Static: `.to_str().unwrap()` / `.to_str().expect(` 패턴을 경로 관련 코드에서 grep.
- Runtime: non-UTF8 바이트를 포함한 파일명 픽스처(Linux 컨테이너에서 생성)로 폴더 스캔·최근 파일 저장/복원 테스트.

**예외**:
- 배포 대상이 UTF-8 로케일을 강제하는 사내 리눅스 워크스테이션 전용이고, 파일명 생성 규칙 자체가 앱에 의해 통제된다면 우선순위를 낮출 수 있다.

**Bitvue 판정**: Confirmed — `src-tauri/src/commands/recent_files.rs:46` persists `RecentFileEntry { path: String, .. }` via tauri-plugin-store (String, not PathBuf/OsString); however other `to_str()` conversions (`file.rs`, `decode_service.rs:251`, `analysis/views.rs:49`) correctly use `.ok_or()` instead of `unwrap()`.

---

### PLAT-002: case-sensitive filesystem을 가정

**분류**: 파일시스템 대소문자 구분 · **심각도**: Medium · **탐지**: Static / Runtime

**나쁜 예**:
```rust
use std::collections::HashMap;

struct FileCache {
    // 경로 문자열을 키로 그대로 사용 — 대소문자 구분 비교
    entries: HashMap<String, DecodedFrames>,
}

impl FileCache {
    fn get(&self, path: &str) -> Option<&DecodedFrames> {
        self.entries.get(path) // "Clip.hevc"와 "clip.hevc"를 다른 키로 취급
    }
}

fn is_hevc(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("hevc") // 대문자 확장자 누락
}
```

**문제**:
- macOS 기본 APFS와 Windows NTFS는 대소문자를 구분하지 않지만 보존은 하는(case-insensitive, case-preserving) 파일시스템인 반면, Linux ext4는 완전히 대소문자를 구분한다. 같은 캐시 키 비교 로직이 플랫폼마다 다른 결과를 낸다.
- `"Clip.hevc"`와 `"clip.hevc"`를 Windows/macOS 사용자는 "같은 파일을 다르게 썼을 뿐"이라 여기지만, 앱의 캐시/중복 검사 로직은 두 개의 다른 항목으로 취급해 캐시 미스나 중복 항목이 생긴다.
- 확장자 매칭을 대소문자 구분으로 하면(`"hevc"` vs `"HEVC"`), 다른 도구에서 대문자 확장자로 내보낸 파일을 Bitvue가 인식하지 못하는 문제가 Linux/일부 Windows 환경에서만 재현된다.
- macOS의 경우 APFS를 case-sensitive로 포맷하는 것도 가능(주로 개발자용 볼륨)해서, "macOS는 항상 case-insensitive"라는 가정조차 100% 안전하지 않다.

**발생 조건**:
- Windows/macOS에서 대소문자만 다른 두 캡처 파일(`sample.hevc`, `Sample.HEVC`)을 같은 프로젝트에 추가하려 할 때 중복 경고가 뜨지 않음.
- Linux에서 대문자 확장자를 가진 파일이 파일 타입 자동 인식에서 누락됨.
- case-sensitive APFS 볼륨(드물지만 존재)에서 개발/테스트한 결과를 기본 case-insensitive APFS 사용자 환경에 그대로 배포.

**권장**:
```rust
use std::path::Path;

fn is_hevc(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("hevc"))
        .unwrap_or(false)
}

// 동일 파일 여부는 문자열 비교가 아니라 OS가 실제로 인식하는 동일성으로 판단한다.
fn same_file(a: &Path, b: &Path) -> std::io::Result<bool> {
    same_file::is_same_file(a, b) // canonicalize + inode/volume 비교
}
```
- 확장자·파일명 매칭은 항상 `eq_ignore_ascii_case`(또는 전체 유니코드 케이스폴딩이 필요하면 `unicase` crate)로 수행한다.
- "같은 파일인가"를 판단해야 하는 로직(중복 방지, 캐시 키)은 문자열 비교가 아니라 `same-file` crate 등으로 실제 파일시스템 동일성(정규화 경로 + inode/볼륨 시리얼)을 확인한다.
- 파일시스템의 대소문자 구분 여부를 가정하지 않고, 필요하면 런타임에 임시 파일로 실제 동작을 프로브해 확인한다.

**탐지 방법**:
- Static: 파일 경로/확장자 비교에서 `==`(대소문자 구분)를 그대로 쓰는 지점 grep.
- Runtime: 대소문자만 다른 파일명 쌍으로 각 OS(특히 Linux)에서 중복 검사·확장자 인식 테스트.

**예외**:
- 앱이 파일명 생성 자체를 완전히 통제하고 항상 소문자 규칙을 강제하는 내부 저장소(예: 자동 생성된 캐시 파일명)라면 해당 없음.

**Bitvue 판정**: Confirmed (partial) — extension matching is done correctly (`file.rs:564` uses `to_lowercase()`), but `recent_files.rs:112` dedups recent files via case-sensitive string compare (`e.path != sanitized_path`), not same-file/case-insensitive identity.

---

### PLAT-003: symlink와 shortcut을 동일하게 처리

**분류**: 심볼릭 링크/바로가기 처리 · **심각도**: High · **탐지**: Structural / Runtime

**나쁜 예**:
```rust
use std::path::Path;

fn scan_dir(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, out)?; // symlink 순환 감지 없이 무조건 재귀
        } else {
            out.push(path); // Windows .lnk 파일도 그냥 "파일"로 목록에 추가
        }
    }
    Ok(())
}
```

**문제**:
- `Path::is_dir()`은 symlink를 자동으로 따라가므로(`Metadata::follow`), 심볼릭 링크가 순환 구조(디렉터리 A 안의 링크가 상위 디렉터리를 다시 가리키는 경우)를 이루면 무한 재귀에 빠져 스택 오버플로 또는 무한 스캔이 발생한다.
- Windows의 `.lnk` 바로가기는 실제 symlink가 아니라 별도의 바이너리 포맷(OLE 구조화 저장소)이며, 단순 파일로 취급하면 "비트스트림 파일"로 잘못 인식되어 열기를 시도했다가 파싱 실패로 이어진다.
- macOS의 alias 파일(구식 Finder 별칭)도 symlink와 다른 메커니즘이며, 대상이 이동해도 추적이 가능하다는 점에서 오히려 symlink보다 더 신경 써서 다뤄야 하는데 이를 구분하지 않으면 깨진 경로로 오인된다.
- 하드링크와 symlink를 구분하지 않으면, 같은 파일을 가리키는 두 경로를 서로 다른 파일로 세거나(디스크 사용량 계산 오류) 반대로 잘못 병합하는 실수가 생긴다.

**발생 조건**:
- 캡처 파일 폴더 구조를 네트워크 심볼릭 링크로 구성한 워크스테이션에서 파일 트리를 재귀 스캔할 때.
- 사용자가 Windows 바탕화면의 `.lnk` 바로가기 파일을 드롭했을 때.
- 심볼릭 링크가 자기 자신이나 상위 디렉터리를 가리키도록(의도치 않게) 구성된 공유 폴더를 스캔할 때.

**권장**:
```rust
use std::collections::HashSet;
use std::path::Path;

fn scan_dir(dir: &Path, visited: &mut HashSet<(u64, u64)>, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let meta = entry.metadata()?; // symlink를 따라가지 않는 symlink_metadata()가 필요하면 별도 사용
        if meta.is_symlink() {
            let target = std::fs::canonicalize(entry.path())?;
            let key = file_identity(&target)?; // (device, inode) 등
            if !visited.insert(key) {
                continue; // 이미 방문한 대상이면 순환으로 판단하고 스킵
            }
        }
        if meta.is_dir() {
            scan_dir(&entry.path(), visited, out)?;
        } else if is_shortcut_file(&entry.path()) {
            // .lnk/.alias는 별도 리졸버로 실제 대상 경로를 얻은 뒤에만 처리
            continue;
        } else {
            out.push(entry.path());
        }
    }
    Ok(())
}
```
- 재귀 스캔 시 방문한 (device, inode) 쌍을 집합으로 추적해 symlink 순환을 명시적으로 차단한다.
- `.lnk`(Windows)/alias(macOS) 여부를 확장자·매직 바이트로 먼저 식별하고, 지원하려면 전용 리졸버(Windows COM `IShellLink`, macOS `NSURL` bookmark)를 거치며, 지원하지 않으면 "바로가기는 지원하지 않습니다"로 명확히 거부한다.
- 심볼릭 링크를 아예 따라가지 않는 정책이 더 안전하다면 `read_dir` 순회 시 `symlink_metadata()`로 링크 자체를 감지해 스캔 대상에서 제외하는 옵션도 고려한다.

**탐지 방법**:
- Structural: 재귀 디렉터리 스캔 함수에서 방문 집합(visited set) 또는 순환 방지 로직이 존재하는지 코드 검토.
- Runtime: symlink 순환 픽스처(디렉터리 A → B → A)와 `.lnk` 파일 픽스처로 스캔 동작 테스트.

**예외**:
- 심볼릭 링크를 아예 지원하지 않는다고 명시하고, 발견 시 단순 거부만 하는 앱이라면 별도의 순환 감지 로직 없이 `symlink_metadata()`로 스킵하는 것만으로 충분하다.

**Bitvue 판정**: N/A — no recursive directory-tree scanning exists anywhere in `src-tauri` (`read_dir`/`WalkDir` grep is empty); the app only opens single files via `canonicalize()` (`file.rs:41`), which safely resolves one symlink hop with no cycle risk and no `.lnk`/alias handling need.

---

### PLAT-004: network drive latency 무시

**분류**: 네트워크 드라이브 I/O · **심각도**: High · **탐지**: Runtime / Static

**나쁜 예**:
```rust
#[tauri::command]
fn open_bitstream(path: String) -> Result<FileInfo, String> {
    // 메인/커맨드 스레드에서 동기 I/O — 로컬 SSD 속도를 암묵적으로 가정
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 4096];
    std::io::Read::read_exact(&mut file, &mut header).map_err(|e| e.to_string())?;
    Ok(FileInfo { size: meta.len() })
}
```

**문제**:
- SMB/NFS로 마운트된 네트워크 드라이브에서 `metadata()`/`open()`/`read()` 호출은 로컬 디스크보다 수백~수천 배 느릴 수 있고, VPN 상태에 따라 수 초에서 수십 초까지 걸릴 수 있다.
- 이 호출들이 커맨드 핸들러(사실상 UI 스레드와 직결)에서 동기적으로 실행되면, 네트워크 드라이브가 느리거나 일시적으로 응답하지 않을 때 앱 전체가 멈춘 것처럼 보인다.
- 타임아웃이나 취소 메커니즘이 없으면, 사용자가 "멈췄다"고 판단해 강제 종료하는 것 외에는 복구 방법이 없다.
- 네트워크 드라이브가 접근 도중 언마운트되거나 연결이 끊기면 후속 read가 무한정 블로킹되거나 예측 불가능한 에러 코드를 반환하는데, 이를 로컬 파일 없음 에러와 동일하게 처리하면 사용자에게 잘못된 안내가 나간다.

**발생 조건**:
- 사내 NAS(SMB/AFP/NFS 마운트)에 저장된 대용량 캡처 파일을 직접 열 때.
- VPN을 통한 원격 파일 서버 접근 중 연결이 불안정할 때.
- 외장 USB 드라이브의 슬립/스핀업 지연처럼 네트워크는 아니지만 유사한 지연 특성을 가진 저장 매체에서도 동일 증상이 재현될 때.

**권장**:
```rust
#[tauri::command]
async fn open_bitstream(path: String) -> Result<FileInfo, String> {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::task::spawn_blocking(move || -> std::io::Result<FileInfo> {
            let meta = std::fs::metadata(&path)?;
            Ok(FileInfo { size: meta.len() })
        }),
    )
    .await;

    match result {
        Ok(Ok(Ok(info))) => Ok(info),
        Ok(Ok(Err(e))) => Err(format!("파일 접근 실패: {e}")),
        Ok(Err(_)) => Err("내부 작업 실패".into()),
        Err(_) => Err("네트워크 드라이브 응답 없음(10초 초과) — 연결을 확인하세요".into()),
    }
}
```
- 파일 I/O는 `spawn_blocking`(또는 전용 워커 스레드)으로 옮기고, 명시적 타임아웃과 취소 가능한 진행 상태를 UI에 노출한다.
- 네트워크 드라이브 특유의 실패 모드(응답 없음, 연결 끊김)를 "파일 없음"과 구분되는 별도 에러 메시지로 안내한다.
- 대용량 파일을 여는 초기 단계에서 진행률 표시와 취소 버튼을 제공해, 느린 스토리지에서도 사용자가 앱이 살아있음을 인지하게 한다.

**탐지 방법**:
- Runtime: 네트워크 지연을 인위적으로 재현(`tc`/`Clumsy` 같은 도구로 마운트된 드라이브 처리량 제한)한 뒤 파일 열기 응답성 확인.
- Static: 커맨드 핸들러 안에서 `std::fs::*` 동기 호출이 `spawn_blocking` 없이 직접 쓰이는지 grep.

**예외**:
- 로컬 SSD 전용 워크스테이션 배포가 확정되어 있고 네트워크 드라이브 접근을 제품 정책상 지원하지 않는다면 우선순위를 낮출 수 있다.

**Bitvue 판정**: Confirmed — no `spawn_blocking` or `tokio::time::timeout` usage anywhere in `src-tauri/src`; `open_file`/`validate_and_open_file` (`file.rs:143,75`) and `get_file_data_arc` (`decode_service.rs:247`) run synchronous `std::fs` I/O directly inside `async fn` command handlers with no timeout or cancellation.

---

### PLAT-005: Windows file locking 미고려

**분류**: 파일 잠금 · **심각도**: Medium · **탐지**: Runtime / Manual

**나쁜 예**:
```rust
fn export_overlay_json(original_path: &std::path::Path, data: &OverlayData) -> std::io::Result<()> {
    // 원본을 읽기용으로 열어둔 핸들이 아직 살아있는 상태에서 같은 파일에 덮어쓰기 시도
    let export_path = original_path.with_extension("overlay.json");
    std::fs::write(&export_path, serde_json::to_vec(data)?)?;
    Ok(())
}
```

**문제**:
- Windows는 다른 프로세스(또는 자기 자신의 다른 핸들)가 파일을 열어 둔 상태에서 삭제·이름변경·독점 쓰기를 시도하면 `ERROR_SHARING_VIOLATION`으로 실패한다. Unix 계열은 열려 있는 파일도 `unlink`할 수 있으므로(디스크립터가 살아있는 한 데이터는 유지) 같은 코드가 Unix에서는 문제없이 동작한다가 Windows에서만 실패한다.
- 분석 중인 원본 파일을 같은 이름으로 다시 저장(내보내기 후 원본 갱신)하려는 흐름은 원본 read 핸들이 아직 열려 있으면 Windows에서 예외 없이 조용히 실패하거나 모호한 OS 에러 메시지만 준다.
- 백신 소프트웨어가 새로 생성/수정된 파일을 짧게 스캔하며 잠그는 동안 후속 쓰기가 실패하는 경우도 Windows에서 흔하지만 재현이 간헐적이라 원인 파악이 어렵다.
- 재시도 로직 없이 첫 실패를 곧바로 사용자 에러로 노출하면, 실제로는 수십~수백 밀리초 후 재시도하면 성공할 일시적 문제를 영구 실패처럼 보고하게 된다.

**발생 조건**:
- 분석 중인 파일을 같은 이름/인접 경로로 내보내기(export)하거나 프로젝트 저장 시 원본을 갱신하려 할 때.
- Windows Defender 등 백신 소프트웨어가 새로 쓰여진 파일을 실시간 스캔하는 환경.
- 같은 파일을 다른 창(비교 뷰어)에서 동시에 읽기 위해 열어 둔 상태에서 저장을 시도할 때.

**권장**:
```rust
use std::time::Duration;

fn write_with_retry(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut last_err = None;
    for attempt in 0..5 {
        match std::fs::write(&tmp, data).and_then(|_| std::fs::rename(&tmp, path)) {
            Ok(()) => return Ok(()),
            Err(e) if is_transient_lock(&e) => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(100 * (attempt + 1)));
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap())
}
```
- 파일 핸들의 생명주기를 명시적으로 관리해, 쓰기 전에 읽기용 핸들을 확실히 닫는다(스코프를 좁혀 drop 시점을 명확히 함).
- 쓰기는 임시 파일에 먼저 쓰고 `rename`으로 교체하는 원자적 패턴을 쓰되, 일시적 공유 위반(`ERROR_SHARING_VIOLATION`, `ERROR_ACCESS_DENIED`)에 한해 지수 백오프로 재시도한다.
- 재시도로도 해결되지 않으면 "다른 프로그램이 파일을 사용 중입니다" 같이 원인이 드러나는 메시지를 사용자에게 보여준다.

**탐지 방법**:
- Runtime: Windows CI에서 파일을 열어둔 채로 동시에 쓰기를 시도하는 경합 테스트.
- Manual: Windows에서 원본 파일을 다른 창/뷰어로 열어 둔 상태로 저장을 시도해 에러 메시지 품질 확인.

**예외**:
- 읽기 전용으로만 접근하고 쓰기/삭제/이름변경을 절대 수행하지 않는 기능에는 해당하지 않는다.

**Bitvue 판정**: Suspected — no atomic tmp+rename+retry write pattern exists anywhere (`export.rs:123,235,269` call `File::create` directly on the final path with no retry on transient lock errors); however no code path was found that writes back to the same file that still has an open read handle, so the specific "overwrite original while open" scenario is plausible but unconfirmed.

---

### PLAT-006: macOS sandbox 권한 유지 실패

**분류**: macOS 샌드박스 권한 · **심각도**: High · **탐지**: Runtime / Manual

**나쁜 예**:
```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct RecentEntry {
    path: String, // 단순 경로 문자열만 저장
}

fn save_recent(entries: &[RecentEntry]) -> std::io::Result<()> {
    let json = serde_json::to_vec(entries)?;
    std::fs::write(recent_file_path(), json)
}

fn reopen_recent(entry: &RecentEntry) -> std::io::Result<std::fs::File> {
    std::fs::File::open(&entry.path) // sandbox 재실행 후 접근 거부될 수 있음
}
```

**문제**:
- App Sandbox(및 macOS의 file access 권한 모델) 아래에서는 사용자가 파일 다이얼로그로 명시적으로 선택한 파일 외에는 접근이 제한되며, 이 권한은 기본적으로 프로세스 생명주기에 묶여 있어 앱을 재시작하면 사라진다.
- 단순 경로 문자열만 저장해두면, 재실행 후 "최근 파일"을 다시 열려 할 때 sandbox가 그 경로에 대한 접근을 거부(`Operation not permitted`)하지만 에러 메시지만으로는 사용자가 원인(권한 재승인 필요)을 알기 어렵다.
- `startAccessingSecurityScopedResource`/`stopAccessingSecurityScopedResource` 짝을 맞추지 않으면(전자만 호출하고 후자를 누락), 리소스 접근 카운트가 누적되어 예기치 않은 시점에 접근이 막히거나 메모리 누수처럼 보이는 현상이 생긴다.
- 파일이 sandbox 컨테이너 밖(외장 드라이브, 다른 사용자 홈 디렉터리)에 있을 때 특히 취약하며, 사내 배포처럼 sandbox가 완화된 환경에서는 재현되지 않아 QA에서 놓치기 쉽다.

**발생 조건**:
- macOS App Store 배포 또는 hardened runtime + sandbox entitlement가 활성화된 빌드에서 "최근 파일 열기"를 앱 재실행 후 시도할 때.
- 파일이 외장 드라이브나 sandbox 컨테이너 바깥의 임의 위치에 있을 때.
- notarization은 되어 있지만 sandbox entitlement 설정이 누락되거나 잘못된 빌드.

**권장**:
```rust
// macOS 전용: 파일 다이얼로그로 얻은 URL을 security-scoped bookmark로 저장
#[cfg(target_os = "macos")]
fn save_bookmark(url: &objc2_foundation::NSURL) -> Result<Vec<u8>, String> {
    // NSURL bookmarkData(options: .withSecurityScope, ...) 로 생성한 bookmark를
    // 경로 문자열이 아니라 이 bookmark 데이터로 영속 저장한다.
    unimplemented!("실제 구현은 objc2/NSURL bookmark API를 통해 수행")
}

#[cfg(target_os = "macos")]
fn resolve_and_access(bookmark: &[u8]) -> Result<std::path::PathBuf, String> {
    // bookmark를 URL로 복원한 뒤 startAccessingSecurityScopedResource 호출,
    // 사용이 끝나면 반드시 stopAccessingSecurityScopedResource로 짝을 맞춘다.
    unimplemented!()
}
```
- 경로 문자열이 아니라 security-scoped bookmark(NSURL bookmark data)를 최근 파일 목록에 저장하고, 재실행 시 이를 복원해 `startAccessingSecurityScopedResource`로 접근 권한을 다시 획득한다.
- `startAccessing...`을 호출했다면 사용이 끝난 즉시(또는 RAII 가드 패턴으로) `stopAccessing...`을 반드시 짝지어 호출한다.
- sandbox가 적용된 빌드와 아닌 빌드를 구분해 최근 파일 복원 로직을 조건부로 분기하고, sandbox 빌드에서는 bookmark 해석 실패 시 "다시 선택해주세요" 안내로 자연스럽게 폴백한다.

**탐지 방법**:
- Runtime: sandboxed 빌드로 앱을 실행 → 파일 열기 → 완전 종료 → 재실행 → 최근 파일 클릭의 전체 흐름을 검증.
- Manual: Console.app에서 sandbox 거부(`sandboxd`) 로그가 발생하는지 확인.

**예외**:
- sandbox가 적용되지 않는 direct-distribution(비 App Store) notarized 빌드만 지원한다면 해당 없음(다만 향후 App Store 배포를 고려한다면 미리 대비할 가치가 있다).

**Bitvue 판정**: N/A — app is not sandboxed/App Store-distributed (no entitlements or sandbox config in `tauri.conf.json`); `recent_files.rs`'s plain path-string storage would need bookmarks only if App Store distribution is added later.

---

### PLAT-007: long path 미지원

**분류**: 긴 경로 지원 · **심각도**: Medium · **탐지**: Runtime / Static

**나쁜 예**:
```rust
fn validate_path_length(path: &str) -> Result<(), String> {
    if path.len() > 260 { // MAX_PATH를 앱 레벨에서 임의로 하드코딩해 사용자를 차단
        return Err("경로가 너무 깁니다".into());
    }
    Ok(())
}

fn open_capture(path: &str) -> std::io::Result<std::fs::File> {
    validate_path_length(path).map_err(std::io::Error::other)?;
    std::fs::File::open(path)
}
```

**문제**:
- Windows의 전통적인 `MAX_PATH`(260자) 제약은 앱 매니페스트에서 long path 지원을 선언하고 시스템 설정(`LongPathsEnabled`)이 켜져 있으면 우회할 수 있는데, 이를 모르고 앱 레벨에서 260자 제한을 하드코딩하면 실제로는 열 수 있는 파일도 앱이 스스로 차단한다.
- 반대로 long path를 선언하지 않은 상태에서 `\\?\` verbatim prefix 없이 깊은 경로에 접근하면 Windows API 자체가 실패하는데, 이 실패가 "앱의 임의 제한"인지 "OS 제약"인지 사용자에게 구분되지 않는 에러로 나온다.
- 회사 공유 드라이브처럼 연도/코덱/샘플/서브샘플 단위로 폴더가 깊게 중첩되는 구조에서는 절대경로가 260자를 쉽게 넘는데, 이런 환경에서만 재현되어 로컬 테스트에서 놓치기 쉽다.
- `\\?\` prefix가 붙은 경로를 표시용으로 그대로 노출하면 사용자에게 불필요하게 낯선 문자열이 보인다(별도 정규화 필요).

**발생 조건**:
- 사내 공유 드라이브의 깊은 폴더 구조(예: `\\nas\projects\2026\hevc\4k\regression\...`)에 있는 캡처 파일을 열 때.
- 프로젝트 내보내기 경로가 사용자 홈 디렉터리 깊숙한 곳(OneDrive 동기화 폴더 등)일 때.
- long path를 지원하지 않는 구형 Windows 10 빌드 또는 그룹 정책으로 `LongPathsEnabled`가 꺼진 사내 환경.

**권장**:
```xml
<!-- Windows 앱 매니페스트: long path 인식 선언 -->
<application xmlns="urn:schemas-microsoft-com:asm.v3">
  <windowsSettings>
    <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
  </windowsSettings>
</application>
```
```rust
fn open_capture(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    // Rust std는 Windows에서 필요 시 자동으로 \\?\ verbatim 처리를 시도하지만,
    // 표시용 경로는 dunce로 사람이 읽기 쉬운 형태로 정규화한다.
    std::fs::File::open(path)
}

fn display_path(path: &std::path::Path) -> String {
    dunce::canonicalize(path)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}
```
- Windows 매니페스트에 `longPathAware`를 선언해 OS가 지원하는 한 260자 제한을 앱 레벨에서 재차 강제하지 않는다.
- 사용자에게 보여주는 오류 메시지에서 "OS의 long path 설정이 꺼져 있을 수 있다"는 힌트를 제공한다(`LongPathsEnabled` 레지스트리 키 안내).
- 표시용 경로 정규화에는 `dunce` crate 등을 사용해 `\\?\` prefix가 그대로 UI에 노출되지 않게 한다.

**탐지 방법**:
- Runtime: Windows CI에 260자를 초과하는 깊은 경로 픽스처를 포함해 열기/저장/내보내기 전 과정을 검증.
- Static: 코드에서 `260`, `MAX_PATH` 같은 상수를 경로 검증 목적으로 하드코딩한 지점 grep.

**예외**:
- 항상 짧은 경로만 다루는 것이 조직 표준으로 강제된 사내 환경이라면 우선순위를 낮출 수 있다.

**Bitvue 판정**: Suspected — no hardcoded `MAX_PATH`/260 check exists in app code (`recent_files.rs:13`'s 4096 constant is unrelated DoS protection), but `tauri.conf.json`'s `bundle.windows` config is empty — no `longPathAware` manifest is declared, so deep Windows paths could still hit the OS-level 260 limit unmitigated.

---

### PLAT-008: native decoder/library 검색 경로 하드코딩

**분류**: 네이티브 라이브러리 경로 · **심각도**: High · **탐지**: Static / Runtime

**나쁜 예**:
```rust
#[cfg(target_os = "macos")]
const LIB_PATH: &str = "/usr/local/lib/libavcodec.61.dylib"; // Intel Homebrew 경로 하드코딩

#[cfg(target_os = "windows")]
const LIB_PATH: &str = r"C:\Program Files\Bitvue\avcodec-61.dll";

fn load_decoder() -> Result<libloading::Library, libloading::Error> {
    unsafe { libloading::Library::new(LIB_PATH) }
}
```

**문제**:
- macOS Homebrew는 Apple Silicon에서 `/opt/homebrew`, Intel에서 `/usr/local`을 사용하므로 하드코딩된 경로 하나로는 두 아키텍처를 동시에 지원할 수 없다.
- Linux는 배포판마다 라이브러리 검색 경로 관례(`/usr/lib/x86_64-linux-gnu`, `/usr/lib64`, distro별 멀티lib 배치)가 다르다.
- Windows는 사용자가 32비트 프로그램을 `Program Files (x86)`에 설치했는지, 시스템 로케일에 따라 경로 표기가 달라질 수 있는지 등 변수가 많고, 특정 사용자가 기본 설치 경로를 바꿨다면 즉시 실패한다.
- macOS는 임의 절대 경로의 dylib을 로드하는 동작이 Gatekeeper/hardened runtime 정책과 충돌할 수 있어(특히 서명되지 않았거나 quarantine 속성이 붙은 경우), 단순히 "경로가 존재하는지"만으로는 로드 성공을 보장할 수 없다.

**발생 조건**:
- 사용자가 시스템에 설치한 FFmpeg/코덱 공유 라이브러리를 동적으로 연동하려는 기능에서, Apple Silicon Mac과 Intel Mac이 서로 다른 Homebrew prefix를 쓸 때.
- 여러 Linux 배포판(Ubuntu, Fedora, Arch)에 동일 바이너리를 배포할 때.
- Windows에서 사용자가 기본 설치 경로가 아닌 곳에 의존 라이브러리를 둔 경우.

**권장**:
```rust
fn candidate_paths(lib_name: &str) -> Vec<std::path::PathBuf> {
    let mut candidates = vec![];
    #[cfg(target_os = "macos")]
    {
        candidates.push(format!("/opt/homebrew/lib/{lib_name}").into());
        candidates.push(format!("/usr/local/lib/{lib_name}").into());
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push(format!("/usr/lib/x86_64-linux-gnu/{lib_name}").into());
        candidates.push(format!("/usr/lib64/{lib_name}").into());
    }
    // 앱 번들에 동봉된 리소스 경로를 최우선으로 시도(가능하면 이쪽만 사용)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.insert(0, dir.join("lib").join(lib_name));
        }
    }
    candidates
}

fn load_decoder(lib_name: &str) -> Result<libloading::Library, String> {
    for path in candidate_paths(lib_name) {
        if path.exists() {
            if let Ok(lib) = unsafe { libloading::Library::new(&path) } {
                return Ok(lib);
            }
        }
    }
    Err(format!("{lib_name}을(를) 찾을 수 없습니다. 설치 경로를 확인하세요."))
}
```
- 가능하면 native 라이브러리를 앱 번들/설치 패키지에 정적/동적으로 동봉해 시스템 설치 경로에 의존하지 않는다(macOS `@rpath`/`@loader_path`, Linux `RPATH`/`$ORIGIN`, Windows exe와 같은 디렉터리).
- 부득이 시스템 설치 라이브러리를 찾아야 한다면 아키텍처·배포판별 후보 경로 목록을 순회하고, 실패 시 사용자가 이해할 수 있는 명확한 에러(어떤 라이브러리를, 어디서 찾았는지)를 제공한다.
- macOS에서는 로드하는 dylib이 서명/notarization 요구사항을 만족하는지 별도로 검증한다.

**탐지 방법**:
- Static: 절대 경로 문자열 리터럴이 `libloading`/`dlopen`/`LoadLibrary` 호출에 직접 전달되는지 grep.
- Runtime: Apple Silicon Mac, Intel Mac(또는 Rosetta), 여러 Linux 배포판에서 라이브러리 로드 성공 여부를 CI 매트릭스로 검증.

**예외**:
- 모든 native 의존성을 정적 링크해 외부 라이브러리 검색 자체가 필요 없는 빌드라면 해당 없음.

**Bitvue 판정**: N/A — no `libloading`/`dlopen`/`LoadLibrary` runtime dynamic loading with hardcoded paths found; native codec deps (`dav1d`, `ffmpeg-next`, `libvmaf-rs`) are linked as Rust crates at build time, not loaded via runtime path strings.

---

### PLAT-009: DPI scaling에서 canvas 좌표 불일치

**분류**: DPI/좌표계 · **심각도**: High · **탐지**: Runtime / Manual

**나쁜 예**:
```rust
// 창 생성 시 OS의 DPI 인식 모델을 선언하지 않고 기본값에 맡김 (Windows)
fn create_window() -> WindowHandle {
    // 매니페스트에 dpiAwareness 선언이 없으면 Windows는 이 프로세스를
    // "DPI-unaware"로 취급해 항상 96 DPI(스케일 1.0)로 보고하고,
    // 실제 화면에는 OS가 비트맵을 확대해서 그린다(흐릿함의 원인).
    unimplemented!("windowing crate로 창 생성")
}

fn overlay_scale_factor(_monitor_id: u32) -> f64 {
    1.0 // 모든 플랫폼/모니터에서 배율이 항상 1.0이라고 가정
}
```

**문제**:
- Windows에서 프로세스가 DPI awareness(PerMonitorV2)를 명시적으로 선언하지 않으면 OS가 창을 자동으로 96 DPI 가정하에 렌더링한 뒤 실제 배율로 bitmap-stretch한다. 이 상태에서 배율을 "물어보면" 항상 1.0이 반환되므로, 코드가 정직하게 배율을 조회해도 실제 표시 배율과 다른 값을 얻는다 — 근본 원인은 좌표 계산 실수가 아니라 OS에 배율 인식 자체를 선언하지 않은 것이다.
- macOS는 `backingScaleFactor`가 창이 놓인 디스플레이별로 독립적으로 정의되며, 외장 4K 디스플레이(배율 2.0)와 내장 Retina(배율 2.0이지만 해상도가 다름) 사이를 창이 이동할 때 이 값이 실시간으로 바뀐다. Windows의 "전역 시스템 DPI" 개념과 근본적으로 다른 모델이라, 한 플랫폼에서 검증한 배율 갱신 타이밍 가정이 다른 플랫폼에 그대로 적용되지 않는다.
- Linux는 X11의 경우 사실상 전역 정수 배율(`Xft.dpi`)만 안정적으로 신뢰할 수 있고, fractional scaling(125%, 150%)은 데스크톱 환경마다 구현 방식이 다르며, Wayland는 출력(모니터)별로 정수 scale을 갖고 compositor가 자체적으로 반올림/합성하는 방식이 배포판마다 다르다. "배율을 얻는 API 자체가 다른 체계"라는 점이 핵심이며, 단순히 한 번 얻은 값을 재사용하면 값이 갱신되는 시점과 정밀도가 플랫폼마다 다르다.
- 이 배율 불일치를 무시하고 QP/MV 오버레이 격자를 그리면, 1px 단위 정밀도가 필요한 블록 경계선이 플랫폼에 따라 다른 두께/흐림으로 보이고, 클릭 히트테스트가 실제 프레임 좌표와 어긋난다.

**발생 조건**:
- Windows에서 DPI awareness 매니페스트 없이 빌드된 실행 파일을 125%/150%/200% 배율 모니터에서 실행할 때.
- macOS에서 배율이 다른 두 모니터 사이로 창을 드래그할 때.
- Linux fractional scaling(GNOME 125%, KDE 150%) 환경에서 정밀 오버레이를 그릴 때.

**권장**:
```xml
<!-- Windows 매니페스트: PerMonitorV2 DPI awareness 명시 -->
<application xmlns="urn:schemas-microsoft-com:asm.v3">
  <windowsSettings>
    <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
  </windowsSettings>
</application>
```
```rust
// native 레이어에서 OS별 API로 "정직한" 배율을 얻어 프런트에는
// 이미 정규화된 좌표 변환 계수만 전달한다.
#[cfg(target_os = "windows")]
fn monitor_scale(hwnd: HWND) -> f64 {
    unsafe { GetDpiForWindow(hwnd) as f64 / 96.0 }
}

#[cfg(target_os = "macos")]
fn monitor_scale(window: &NSWindow) -> f64 {
    window.backingScaleFactor() // 창이 위치한 디스플레이 기준
}
```
- Windows 매니페스트에 `PerMonitorV2` DPI awareness를 반드시 선언해, OS가 배율을 대신 처리(bitmap-stretch)하지 않고 앱이 정직한 값을 조회하도록 만든다.
- 각 OS의 네이티브 API(Windows `GetDpiForWindow`, macOS `backingScaleFactor`, Wayland `wl_output` scale)로 배율을 얻는 코드를 native 레이어 한 곳에 모으고, 모니터 변경/이동 이벤트를 구독해 갱신한다.
- 프런트에는 "CSS 픽셀 ↔ 물리 픽셀 ↔ 프레임 좌표"라는 이미 정규화된 변환 계수만 전달해, OS별 API 차이를 프런트 코드가 알 필요 없게 만든다.

**탐지 방법**:
- Runtime: Windows 매니페스트에 DPI awareness 선언 유무를 `mt.exe`/`sigcheck`로 확인, 125%/150%/200% 배율 각각에서 오버레이 격자 스크린샷 비교.
- Manual: macOS에서 배율이 다른 두 모니터 사이로 창을 이동한 직후 오버레이 클릭 정확도 확인.

**예외**:
- 배율 1.0(100%) 디스플레이만 공식 지원 대상으로 명시한 초기 프로토타입 단계라면 우선순위를 낮출 수 있다.

**Bitvue 판정**: Suspected — no DPI-awareness manifest declaration found in `tauri.conf.json`/`src-tauri`; `HRDBufferPanel.tsx:102` reads `devicePixelRatio` for canvas scaling but the main render path (`VideoCanvas.tsx`, `OverlayRenderer/webgl/mv-webgl.ts`) never references it, suggesting inconsistent HiDPI handling — plausible but no direct click-misalignment repro found.

---

### PLAT-010: endian은 고정이어도 alignment 차이 무시

**분류**: 메모리 정렬 · **심각도**: Critical · **탐지**: Static / Runtime

**나쁜 예**:
```rust
#[repr(C)]
struct SliceHeaderRaw {
    first_mb: u32,
    slice_type: u32,
    qp_delta: i32,
}

fn parse_slice_header(buf: &[u8], offset: usize) -> SliceHeaderRaw {
    unsafe {
        // 임의 바이트 오프셋에서 정렬을 보장하지 않은 채 포인터 캐스팅 후 역참조
        let ptr = buf.as_ptr().add(offset) as *const SliceHeaderRaw;
        std::ptr::read(ptr) // offset이 4의 배수가 아니면 unaligned read — UB
    }
}
```

**문제**:
- 비트스트림 파싱에서 NAL 유닛 페이로드 안의 임의 바이트 오프셋에 구조체를 직접 캐스팅해 읽는 코드는, 그 오프셋이 구조체의 정렬 요구사항(여기서는 4바이트)의 배수라는 보장이 없다.
- Rust에서 정렬되지 않은 포인터의 역참조(`*ptr` 또는 `ptr::read`을 정렬 요구가 있는 타입에 사용)는 언제나 미정의 동작(UB)이며, 이는 엄밀히 "모든 플랫폼에서" 문제이지만 x86/x86_64는 하드웨어가 unaligned access를 관대하게 허용(성능 저하만 발생)해 몇 년간 조용히 동작하다가, ARM64(Apple Silicon 등)에서 처음 실행되는 순간 SIGBUS로 크래시하거나 컴파일러 최적화 수준에 따라 잘못된 값을 읽는 형태로 드러난다.
- SIMD 최적화 코드(NEON/AVX)에서 aligned load 명령어(`_mm_load_*`, `vld1q_*` 계열 중 정렬을 요구하는 것)에 정렬되지 않은 슬라이스를 넘기면 마찬가지로 크래시하거나, 명령어에 따라서는 조용히 잘못된 데이터를 읽는다.
- 이런 버그는 x86 개발 머신에서 수년간 테스트를 통과하다가, Apple Silicon Mac 지원을 추가한 시점에야 처음 재현되는 경우가 많아 "왜 이제 와서 터지는가"를 둘러싼 디버깅 비용이 크다.

**발생 조건**:
- NAL unit의 raw byte 버퍼를 struct로 직접 캐스팅해 파싱하는 코드가 x86 개발/CI만 거치고 Apple Silicon(ARM64) 네이티브 빌드에서 처음 실행될 때.
- SIMD로 최적화된 픽셀/모션벡터 처리 루프에 정렬이 보장되지 않은 슬라이스(예: 임의 오프셋에서 슬라이싱한 프레임 버퍼 일부)를 전달할 때.
- 컴파일러 최적화 레벨이 바뀌면서(디버그 → 릴리즈) UB의 구체적 증상(조용한 오동작 → 크래시)이 달라질 때.

**권장**:
```rust
#[repr(C)]
struct SliceHeaderRaw {
    first_mb: u32,
    slice_type: u32,
    qp_delta: i32,
}

fn parse_slice_header(buf: &[u8], offset: usize) -> SliceHeaderRaw {
    // 정렬 요구 없이 바이트 단위로 안전하게 읽는다.
    SliceHeaderRaw {
        first_mb: u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()),
        slice_type: u32::from_le_bytes(buf[offset + 4..offset + 8].try_into().unwrap()),
        qp_delta: i32::from_le_bytes(buf[offset + 8..offset + 12].try_into().unwrap()),
    }
}

// 정렬되지 않은 포인터에서 값을 읽어야 하는 경우 read_unaligned를 명시적으로 사용
unsafe fn read_u32_unaligned(ptr: *const u8) -> u32 {
    (ptr as *const u32).read_unaligned()
}
```
- 바이트 버퍼에서 구조체를 읽을 때는 `from_le_bytes`/`from_be_bytes` 같은 정렬 무관 API나 `nom`/`bytes` 같은 파서 crate를 사용하고, raw 포인터 캐스팅 후 역참조를 지양한다.
- 정말 포인터 기반 접근이 필요하면 `read_unaligned`/`write_unaligned`를 명시적으로 사용해 정렬 가정을 코드에 드러낸다.
- SIMD 코드는 항상 unaligned load 명령어를 기본으로 사용하고, aligned load는 실제로 정렬이 보장된 버퍼(직접 할당해 정렬한 버퍼)에만 사용한다.

**탐지 방법**:
- Static: `unsafe`로 `*const T`/`*mut T` 캐스팅 후 역참조하는 지점을 grep, `clippy::cast_ptr_alignment` 린트 활성화.
- Runtime: ARM64 네이티브 CI 러너(Apple Silicon macOS, ARM64 Linux)에서 전체 파서 테스트 스위트 실행.

**예외**:
- 파싱 코드가 처음부터 바이트 단위 파서(nom, bytes crate)만 사용하고 raw 포인터 캐스팅이 코드베이스에 전혀 없다면 해당 없음.

**Bitvue 판정**: N/A — bitstream parsers (`bitvue-avc`/`bitvue-hevc`/`bitvue-av1-codec` `nal.rs`/`slice.rs`) use bit-level readers, not raw struct-pointer casts; all SIMD code (`bitvue-metrics/src/simd.rs`, `bitvue-decode/src/strategy/avx2.rs`) correctly uses unaligned `loadu`/`storeu` intrinsics, never the aligned variants.

---

### PLAT-011: AVX/NEON 지원 분기 부족

**분류**: SIMD 기능 분기 · **심각도**: High · **탐지**: Runtime / Static

**나쁜 예**:
```rust
#[cfg(target_arch = "x86_64")]
unsafe fn sum_abs_diff_avx2(a: &[u8], b: &[u8]) -> u32 {
    use std::arch::x86_64::*;
    // -C target-feature=+avx2로 컴파일해 두고 런타임 CPUID 체크 없이 곧바로 사용
    let va = _mm256_loadu_si256(a.as_ptr() as *const __m256i);
    let vb = _mm256_loadu_si256(b.as_ptr() as *const __m256i);
    let diff = _mm256_sad_epu8(va, vb);
    // ... 합산 생략
    0
}

#[cfg(not(target_arch = "x86_64"))]
fn sum_abs_diff_avx2(a: &[u8], b: &[u8]) -> u32 {
    // ARM64에는 전용 NEON 경로가 아예 없어 매우 느린 스칼라 fallback으로 조용히 강등
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).unsigned_abs()).sum()
}
```

**문제**:
- 빌드 시 `-C target-feature=+avx2`로 AVX2 명령어를 컴파일해 넣고 런타임 CPUID 체크(`is_x86_feature_detected!`) 없이 즉시 실행하면, AVX2를 지원하지 않는 CPU(구형 Intel, 일부 가상 머신, 저가형 Windows 노트북)에서 `SIGILL`로 즉시 크래시한다.
- ARM64(Apple Silicon) 전용 NEON intrinsics 경로를 아예 작성하지 않고 x86 분기가 없을 때 스칼라 fallback으로만 처리하면, 컴파일과 실행은 문제없이 되지만 hot loop(픽셀 비교, MV 히트맵 렌더링) 성능이 수 배~수십 배 저하되는데 이를 감지할 벤치마크/경고가 없으면 "그냥 느린 기능"으로 방치된다.
- 기능 감지(`is_x86_feature_detected!`)를 hot loop 내부에서 매번 호출하면(캐싱하지 않으면) 그 자체가 오버헤드가 되어 최적화 효과를 일부 상쇄한다.
- SIMD 최적화가 적용된 코드 경로와 스칼라 폴백 경로의 계산 결과가 부동소수점 반올림이나 포화 연산(saturating) 차이로 미세하게 달라질 수 있는데, 이를 검증하는 교차 테스트가 없으면 플랫폼마다 분석 결과(예: PSNR 수치)가 미묘하게 달라지는 문제로 이어질 수 있다.

**발생 조건**:
- 디코딩된 픽셀 비교, 모션벡터 히트맵 생성 같은 hot path를 SIMD로 최적화했는데 배포 CPU 매트릭스(구형 Intel/AMD, 저가형 노트북, VM, Apple Silicon)를 검증하지 않았을 때.
- AVX2 전제로 개발된 코드가 AVX2 미지원 CPU에서 처음 실행될 때(구매 후 오래된 회사 PC 등).
- Apple Silicon Mac 지원을 "일단 컴파일만 되면 됨"으로 취급하고 NEON 최적화 경로를 나중으로 미룬 채 출시했을 때.

**권장**:
```rust
use std::sync::OnceLock;

static HAS_AVX2: OnceLock<bool> = OnceLock::new();

fn sum_abs_diff(a: &[u8], b: &[u8]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        if *HAS_AVX2.get_or_init(|| is_x86_feature_detected!("avx2")) {
            return unsafe { sum_abs_diff_avx2(a, b) };
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return unsafe { sum_abs_diff_neon(a, b) };
        }
    }
    sum_abs_diff_scalar(a, b) // 항상 안전한 최종 폴백
}
```
- 런타임 기능 감지(`is_x86_feature_detected!`, `is_aarch64_feature_detected!`)는 필수이며, 결과를 `OnceLock`/`Lazy`로 캐싱해 hot loop 안에서 반복 호출하지 않는다.
- x86(AVX2 > SSE4.2 > scalar)과 ARM64(NEON > scalar) 양쪽에 대해 다단계 폴백을 마련하거나, `multiversion` 같은 crate로 함수 다중 버전 컴파일/디스패치를 자동화한다.
- CI에 ARM64 네이티브 러너(Apple Silicon)와 구형 CPU baseline(AVX2 비활성) 빌드의 벤치마크를 포함해 "느려졌는데 아무도 몰랐다"를 방지한다.
- SIMD 경로와 스칼라 경로의 출력이 허용 오차 내에서 일치하는지 교차 검증 테스트를 둔다.

**탐지 방법**:
- Static: `target_feature` 속성이 붙은 함수 중 런타임 감지(`is_*_feature_detected!`) 없이 직접 호출되는 지점 grep.
- Runtime: 다양한 CPU 매트릭스(AVX2 비활성 VM, ARM64 네이티브)에서 SIGILL 여부와 벤치마크 결과 비교.

**예외**:
- 앱 설치/실행 시점에 최소 CPU 요구사항(AVX2 필수)을 명시적으로 검증하고 미달 시 설치를 막는 정책이라면 hot loop 내 런타임 분기는 생략할 수 있으나, 이 경우도 설치 단계의 검증 자체는 반드시 필요하다.

**Bitvue 판정**: N/A — `bitvue-metrics/src/simd.rs:46-60` already implements the recommended pattern: runtime `is_x86_feature_detected!`/`is_aarch64_feature_detected!` checks with AVX2→SSE2 and NEON→scalar fallback chains, plus a dedicated `bitvue-decode/src/strategy/neon.rs`.

---

### PLAT-012: ARM64 macOS를 x86과 동일 취급

**분류**: ARM64/x86 아키텍처 · **심각도**: Medium · **탐지**: Static / Manual

**나쁜 예**:
```toml
# CI 빌드 설정: macOS 타겟을 x86_64 하나로만 고정
[build]
target = "x86_64-apple-darwin"
```
```rust
// 서드파티 네이티브 디코더 dylib를 x86_64 슬라이스만 번들에 포함
// (Apple Silicon Mac에서는 전체 프로세스가 Rosetta 2로 강제 전환됨)
```

**문제**:
- Rosetta 2로 번역된 x86_64 바이너리는 대부분의 경우 "동작은 하지만" SIMD 집약적 디코딩/픽셀 처리 성능이 네이티브 ARM64 대비 크게 저하되며, 사용자는 "Bitvue가 Mac에서 느리다"고 오인하게 된다.
- 앱 본체가 ARM64 네이티브로 빌드되어도, 연결된 서드파티 네이티브 라이브러리(코덱 디코더 dylib 등)가 x86_64 슬라이스만 가지고 있으면 그 라이브러리를 로드하는 순간 프로세스 전체가 Rosetta로 강제 전환되어 ARM64 네이티브 빌드의 이점이 전부 사라진다.
- 일부 하드웨어 가속 경로(VideoToolbox 등)는 네이티브 프로세스에서만 최적 동작하도록 되어 있어, Rosetta 하에서는 일부 가속 기능이 비활성화되거나 다른 코드 경로를 타는데 이 차이를 인지하지 못한 채 "Mac에서는 원래 이 정도"로 넘어가기 쉽다.
- CI가 x86_64만 빌드/검증하면, ARM64에서만 드러나는 문제(PLAT-010의 정렬 문제, PLAT-011의 NEON 누락, 심볼 링킹 차이)를 배포 전에 잡지 못한다.
- 사용자가 Apple Silicon Mac에서 실제로 Rosetta로 실행 중인지 여부를 알아채기 어렵다(작업관리자 "종류" 컬럼을 확인하는 습관이 없는 한).

**발생 조건**:
- CI/CD 파이프라인의 macOS 빌드 러너나 타겟 트리플이 `x86_64-apple-darwin`으로 고정되어 있고 `aarch64-apple-darwin`이 별도로 빌드/검증되지 않을 때.
- 서드파티 네이티브 의존성(FFI로 링크하는 코덱 라이브러리)이 universal2 또는 arm64 슬라이스를 아직 제공하지 않을 때.
- Apple Silicon Mac 사용자가 늘어난 뒤에도 성능 이슈 리포트를 "설정 문제"로 오판할 때.

**권장**:
```bash
# universal2 바이너리 빌드 (두 타겟을 각각 빌드 후 lipo로 결합)
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
lipo -create -output bitvue-universal \
  target/x86_64-apple-darwin/release/bitvue \
  target/aarch64-apple-darwin/release/bitvue

# 모든 native 의존성이 arm64 슬라이스를 포함하는지 검증
lipo -info target/aarch64-apple-darwin/release/deps/libavcodec.dylib
```
- CI 매트릭스에 Apple Silicon 네이티브 러너(`macos-14` 이상)를 포함해 `aarch64-apple-darwin` 타겟을 별도로 빌드·테스트한다.
- 배포 바이너리는 universal2(lipo로 결합)로 만들거나, 최소한 두 아키텍처용 빌드를 각각 명시적으로 제공한다.
- 모든 FFI 네이티브 의존성이 arm64 슬라이스를 포함하는지 `lipo -info`로 CI에서 자동 검증하고, 누락 시 빌드를 실패시킨다.
- 성능 회귀 리포트를 받으면 `arch` 명령이나 Activity Monitor의 "종류" 컬럼으로 실제 실행 아키텍처(Apple/Intel)를 먼저 확인하는 것을 지원 절차에 포함한다.

**탐지 방법**:
- Static: CI 설정 파일에서 macOS 타겟 트리플이 `x86_64-apple-darwin` 하나로만 고정되어 있는지 검토.
- Manual: Apple Silicon Mac에서 `arch` 명령 또는 Activity Monitor로 실제 실행 아키텍처 확인, `lipo -info`로 번들 내 모든 dylib의 슬라이스 구성 확인.

**예외**:
- 특정 서드파티 코덱 라이브러리가 x86 전용이라 당분간 Rosetta 경유가 의도된 임시 조치라면, 그 사실과 예상 성능 영향을 문서화한 뒤 허용할 수 있다.

**Bitvue 판정**: N/A — CI (`.github/workflows/build-tauri-app.yml:41`) builds the macOS release exclusively for `aarch64-apple-darwin` (native Apple Silicon), not an x86_64-only build that would force Rosetta.

---

### PLAT-013: Wayland/X11 차이 무시

**분류**: Linux 디스플레이 서버 · **심각도**: Medium · **탐지**: Runtime / Manual

**나쁜 예**:
```rust
#[cfg(target_os = "linux")]
fn save_window_position(x: i32, y: i32) {
    // X11의 절대 화면 좌표 개념을 그대로 가정
    // Wayland에서는 클라이언트가 자신의 절대 화면 위치를 알 방법이 없어
    // 이 값 자체가 애초에 의미 있게 얻어지지 않는다.
    config::set("window.x", x);
    config::set("window.y", y);
}
```

**문제**:
- Wayland의 보안 모델은 클라이언트 애플리케이션이 자신의 절대 화면 좌표를 알 수 없도록 의도적으로 제한한다. X11에서는 자연스럽던 "마지막 창 위치 복원" 기능이 Wayland에서는 API 자체가 없거나 항상 `(0, 0)` 같은 무의미한 값을 반환해, 이 값을 신뢰하면 창이 항상 같은 위치에서만 열리거나 조용히 기능이 무력화된다.
- 전체 화면 캡처 기반 기능(오버레이 diff를 위한 스크린샷 등)을 X11 전용 API(`XGetImage` 등)로 작성하면 Wayland compositor(GNOME/KDE)에서는 아예 동작하지 않거나, portal(xdg-desktop-portal) 기반의 별도 권한 요청 UI를 거쳐야 해서 사용자 경험이 크게 달라진다.
- 전역 단축키 등록도 X11의 grab 방식과 Wayland의 compositor별 프로토콜(각 데스크톱 환경마다 지원 여부와 API가 다름)이 근본적으로 달라, X11에서 동작하던 전역 단축키가 Wayland 세션에서는 등록 자체가 실패하거나 무반응일 수 있다.
- 클립보드·드래그앤드롭 프로토콜도 두 시스템 간 미묘한 차이가 있어, X11에서 검증한 상호작용이 Wayland compositor에 따라 다르게 동작할 수 있다.

**발생 조건**:
- Wayland가 기본 세션인 최신 Linux 배포판(Fedora, Ubuntu 최신 버전의 GNOME/KDE)에서 창 위치 저장/복원, 전체 화면 캡처, 전역 단축키 기능을 사용할 때.
- X11 전용 개발 환경에서만 테스트하고 Wayland 세션으로 검증 없이 배포할 때.
- 사용자가 X11과 Wayland 세션을 오가며 같은 설정 파일을 공유할 때(저장된 좌표가 세션 종류에 따라 무의미해짐).

**권장**:
```rust
fn session_type() -> SessionType {
    match std::env::var("XDG_SESSION_TYPE").as_deref() {
        Ok("wayland") => SessionType::Wayland,
        Ok("x11") => SessionType::X11,
        _ => SessionType::Unknown,
    }
}

fn save_window_position(x: i32, y: i32) {
    if session_type() == SessionType::Wayland {
        return; // Wayland에서는 이 기능이 원천적으로 불가능함을 명시적으로 인정하고 스킵
    }
    config::set("window.x", x);
    config::set("window.y", y);
}
```
- 크로스플랫폼 창 라이브러리(`winit`/`tao` 등)가 제공하는 추상화를 우선 사용하고, 라이브러리가 지원하지 않는 기능은 Wayland에서 사용할 수 없다는 사실을 그대로 받아들인다.
- `XDG_SESSION_TYPE` 환경변수(또는 라이브러리가 제공하는 세션 감지 API)로 세션 종류를 확인해, 지원되지 않는 기능은 조용히 실패하는 대신 UI에서 "Wayland에서는 창 위치가 저장되지 않습니다" 같은 명시적 안내로 대체한다.
- 화면 캡처가 필요하면 `xdg-desktop-portal` 기반 API를 사용해 Wayland/X11 양쪽에서 동작하는 공식 경로를 따른다.

**탐지 방법**:
- Runtime: X11 세션과 Wayland 세션(GNOME, KDE 각각) 양쪽에서 창 위치 복원, 화면 캡처, 전역 단축키 기능을 수동 검증.
- Manual: `echo $XDG_SESSION_TYPE`으로 세션 종류를 확인한 뒤 관련 기능이 세션별로 다르게 동작/실패하는지 체크리스트로 검증.

**예외**:
- 배포 대상이 X11 전용으로 고정된 사내 Linux 워크스테이션이고 Wayland 지원 계획이 없다면 우선순위를 낮출 수 있다.

**Bitvue 판정**: N/A — no window-position persistence, global-shortcut plugin, or screen-capture feature exists (`src-tauri/Cargo.toml` has no such plugin); nothing yet exhibits Wayland/X11 divergence.

---

### PLAT-014: 파일 drag-drop URI decoding 오류

**분류**: 드래그앤드롭 URI 디코딩 · **심각도**: Medium · **탐지**: Static / Runtime

**나쁜 예**:
```rust
fn decode_dropped_uri(uri: &str) -> std::path::PathBuf {
    // "file://" 접두사만 제거하고, percent-encoding은 임시방편 replace 체인으로 처리
    let s = uri.strip_prefix("file://").unwrap_or(uri);
    let s = s.replace("%20", " "); // 공백 외 다른 percent 코드는 무시
    std::path::PathBuf::from(s)
}
```

**문제**:
- macOS/Linux(GTK `text/uri-list`)의 드롭 payload는 `file:///Users/.../클립 최종.hevc`처럼 공백과 non-ASCII 문자를 UTF-8 바이트 기준 percent-encoding으로 표현하지만, 이는 OS/툴킷이 규정한 표준 URI 인코딩 규칙(RFC 3986)을 따르는 것이지 "%20을 공백으로 바꾸면 되는" 임시방편으로 완전히 커버되지 않는다. 파일명에 실제로 `%`가 포함된 경우(드물지만 가능) `%2F` 같은 다른 percent 코드까지 있으면 단순 치환 체인은 잘못된 디코딩을 만든다.
- Windows Explorer의 드롭 payload는 애초에 URI 스킴이 아니라 별도의 클립보드 포맷(`CF_HDROP`, UTF-16 파일 경로 목록)으로 전달되는 경우가 있어, "드롭 데이터는 항상 `file://` URI"라는 전제 자체가 플랫폼에 따라 성립하지 않는다.
- Windows 드라이브 문자가 있는 절대경로가 `file:///C:/Users/...` 형태의 URI로 올 때, 스킴만 벗겨내고 선행 슬래시를 처리하지 않으면 `/C:/Users/...` 같은 유효하지 않은 경로 문자열이 만들어진다.
- macOS는 파일명이 파일시스템에 NFD(분해형) 유니코드 정규화로 저장되는 경우가 흔한데, URI를 디코딩해 얻은 문자열을 NFC로 저장된 프로젝트 메타데이터와 바이트 단위로 비교하면 "같은 파일인데 다른 항목"으로 인식되는 문제가 OS별로 다르게 나타난다.

**발생 조건**:
- 파일명에 공백·한글·이모지가 포함된 캡처 파일을 드래그앤드롭할 때.
- Windows에서 드라이브 문자를 포함한 경로가 URI 형태(`file:///C:/...`)로 전달될 때.
- macOS에서 만든 파일을 다른 OS의 경로 비교 로직으로 처리할 때(NFC/NFD 불일치).

**권장**:
```rust
fn decode_dropped_uri(uri: &str) -> Result<std::path::PathBuf, String> {
    let url = url::Url::parse(uri).map_err(|e| e.to_string())?;
    let path = url.to_file_path().map_err(|_| "file:// URI가 아닙니다".to_string())?;
    Ok(normalize_unicode(&path)) // NFC로 정규화해 프로젝트 메타데이터와 비교 가능하게 함
}

fn normalize_unicode(path: &std::path::Path) -> std::path::PathBuf {
    use unicode_normalization::UnicodeNormalization;
    let s: String = path.to_string_lossy().nfc().collect();
    std::path::PathBuf::from(s)
}
```
- URI 디코딩은 직접 구현한 percent-decoding 치환 체인이 아니라 `url` crate의 `Url::parse` + `to_file_path()`에 위임해, 스킴 처리·percent-decoding·OS별 경로 변환(드라이브 문자 포함)을 표준 구현에 맡긴다.
- Windows에서 `CF_HDROP` 계열 페이로드가 별도로 전달될 수 있다면 이를 URI가 아닌 별도 포맷으로 명시적으로 분기 처리한다.
- 디코딩 결과 경로 문자열은 `unicode-normalization` crate 등으로 NFC 정규화를 통일해, 저장된 메타데이터와의 비교가 OS에 관계없이 일관되게 한다.

**탐지 방법**:
- Static: `%XX` 문자열을 수동 `.replace()` 체인으로 디코딩하는 코드 grep.
- Runtime: 공백/한글/이모지/드라이브 문자를 포함한 파일명 드롭 픽스처로 각 OS에서 경로 복원 정확도 테스트.

**예외**:
- 앱이 드래그앤드롭을 지원하지 않고 항상 native 파일 다이얼로그로만 경로를 얻는다면 해당 없음.

**Bitvue 판정**: N/A — no drag-and-drop file handling is implemented anywhere in frontend or `src-tauri` (no `onDrop`/`file-drop` listeners found); feature doesn't exist yet.

---

### PLAT-015: OS별 video output/color management 차이 무시

**분류**: 색상 관리 · **심각도**: Medium · **탐지**: Manual / Static

**나쁜 예**:
```rust
fn render_frame_to_screen(rgb: &[u8], width: u32, height: u32) {
    // 디코딩된 RGB를 디스플레이가 항상 sRGB라고 가정하고 그대로 출력
    // 소스 색공간(BT.709/BT.2020/P3)이나 디스플레이 프로파일을 전혀 태깅하지 않음
    draw_to_canvas(rgb, width, height);
}
```

**문제**:
- macOS는 시스템 전체가 ColorSync로 디스플레이 프로파일을 자동 관리하며 wide-gamut(P3) 디스플레이가 흔한 반면, Windows는 애플리케이션이 명시적으로 WCS/ICC 프로파일을 다루지 않으면 렌더링 결과를 사실상 sRGB로 가정해 처리하므로, 같은 RGB 값이 두 OS에서 서로 다른 실제 색으로 보일 수 있다.
- Linux는 데스크톱 환경/컴포지터에 따라 색 관리 지원이 파편화되어 있고 대부분 기본값으로는 색 관리를 아예 하지 않아, "OS가 알아서 처리해줄 것"이라는 가정이 세 플랫폼 중 최소 하나에서는 성립하지 않는다.
- BT.2020/PQ 같은 HDR 색공간의 비트스트림을 톤매핑 없이 SDR 디스플레이에 그대로 출력하면 밝은 영역이 클리핑되어 원본과 다르게 보이는데, 이 문제는 색 관리가 되는 macOS 환경에서는 시스템이 일부 보정을 해줄 수 있어 눈에 덜 띄고 Windows/Linux에서 더 두드러지게 나타날 수 있다.
- 색상 정확도가 중요한 QC(품질 검수) 작업에서, 동일한 프레임이 플랫폼마다 미묘하게 다르게 보이면 "어느 쪽이 정답인지" 사용자가 혼란스러워하고, 리뷰 결과가 사용한 OS/모니터에 따라 갈릴 수 있다.

**발생 조건**:
- BT.2020/PQ HDR 비트스트림을 P3 Retina 디스플레이(macOS)와 표준 sRGB 모니터(Windows)에서 나란히 프리뷰 비교할 때.
- 색상 정확도가 중요한 QC 작업에서 같은 프레임을 여러 플랫폼의 워크스테이션에서 리뷰할 때.
- wide-gamut 외장 모니터와 기본 sRGB 노트북 화면 사이를 창이 오갈 때.

**권장**:
```rust
struct ColorTag {
    primaries: ColorPrimaries, // BT.709 / BT.2020 / P3 등, VUI에서 추출
    transfer: TransferFunction, // SDR / PQ / HLG
}

fn render_frame_to_screen(rgb: &[u8], tag: ColorTag, width: u32, height: u32) {
    // OS별 색 관리 API로 명시적 변환 후 렌더링하거나,
    // 변환이 불가능한 환경(Linux 기본)에서는 UI에 "색 관리되지 않음"을 명시한다.
    #[cfg(target_os = "macos")]
    let converted = convert_via_colorsync(rgb, &tag);
    #[cfg(target_os = "windows")]
    let converted = convert_via_wcs(rgb, &tag);
    #[cfg(target_os = "linux")]
    let converted = { mark_uncolor_managed(); rgb.to_vec() };

    draw_to_canvas(&converted, width, height);
}
```
- 렌더링 파이프라인에서 소스 색공간(비트스트림 VUI의 `colour_primaries`/`transfer_characteristics`)을 명시적으로 태깅하고, 가능한 OS별 색 관리 API(macOS ColorSync/CGColorSpace, Windows DXGI color space)를 통해 디스플레이 프로파일에 맞춰 변환한다.
- 색 관리를 지원하지 않는 환경(대부분의 Linux 기본 설정)에서는 이를 숨기지 않고 "이 프리뷰는 색 관리되지 않습니다"를 UI에 명시해 사용자가 화면 간 비교에 오해하지 않게 한다.
- 픽셀 값 자체를 수치로 비교하는 분석 기능(히스토그램, PSNR 계산 등)은 디스플레이 렌더링과 별개의 파이프라인임을 문서화해 혼동을 줄인다.

**탐지 방법**:
- Manual: 레퍼런스 모니터 대조 — 같은 HDR 프레임을 macOS와 Windows에서 나란히 캡처해 밝기/색조 클리핑 차이를 비교.
- Static: 렌더링 코드 경로에서 색공간 변환 단계가 존재하는지, 소스 색공간 태그가 렌더러까지 전달되는지 검토.

**예외**:
- 픽셀 값의 수치 비교(히스토그램, PSNR/SSIM 등)만 수행하고 화면 표시 자체는 참고용이라고 명시된 기능은 디스플레이 색 관리와 무관하므로 예외로 둘 수 있다.

**Bitvue 판정**: Confirmed — `bitvue-decode/src/yuv.rs:409` hardcodes fixed BT.601 conversion coefficients regardless of the stream's actual VUI `color_primaries`/`transfer_characteristics` (which `metadata.rs` does parse and expose), and no OS ColorSync/WCS integration exists anywhere; HDR/BT.2020 content is never tone-mapped for SDR display.

---

### PLAT-016: 임시 디렉터리 수명 오해

**분류**: 임시 디렉터리 수명 · **심각도**: Medium · **탐지**: Static / Runtime

**나쁜 예**:
```rust
fn cache_decoded_frames(session_id: &str, frames: &[u8]) -> std::io::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join("bitvue").join(session_id);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("frames.cache");
    std::fs::write(&path, frames)?;
    Ok(path) // 이 경로를 프로젝트 파일에 저장해 다음 실행에서 재사용할 계획
}
```

**문제**:
- Windows의 `%TEMP%`는 사용자가 디스크 정리 도구로 수동 정리하거나, 관리 정책에 따라 로그인 세션이 바뀌면 경로 자체가 달라질 수 있어 "다음에도 거기 있을 것"이라는 가정이 위험하다.
- macOS는 `std::env::temp_dir()`이 프로세스별로 무작위 컨테이너가 배정되는 `/var/folders/.../T/` 경로를 반환하는 경우가 흔하며(`TMPDIR` 환경변수 기반), 앱을 재실행하면 다른 경로가 배정될 수 있고 OS가 주기적으로(대개 며칠 단위) 자동 정리한다.
- Linux는 `systemd-tmpfiles`가 기본적으로 `/tmp`를 주기적으로 청소하도록 설정되어 있는 배포판이 많아(기본값 10일), 장기 세션에서 저장해 둔 캐시가 다음 방문 시 사라져 있을 수 있다.
- 이런 특성을 모르고 캐시 파일 경로만 프로젝트 파일에 저장해 두면, 며칠 뒤 세션을 이어서 열었을 때 "파일 없음" 에러를 만나게 되고 사용자는 원인을 알 수 없다.

**발생 조건**:
- 대용량 파일 디코딩 결과를 임시 디렉터리에 캐시하고, 이를 다음 앱 실행에서도 재사용하려고 경로를 영속 저장할 때.
- 며칠에서 몇 주에 걸친 장기 분석 세션에서 중간에 앱을 재시작할 때.
- OS가 자동으로 임시 파일을 정리하는 주기(수 일)보다 세션 재개 간격이 긴 사용 패턴.

**권장**:
```rust
fn cache_dir_for_reuse(app_id: &str) -> std::io::Result<std::path::PathBuf> {
    // 재시작 간 지속되어야 하는 캐시는 OS별 앱 전용 캐시 디렉터리를 사용한다.
    let base = directories::ProjectDirs::from("com", "bitvue", app_id)
        .ok_or_else(|| std::io::Error::other("cache dir 확인 실패"))?;
    let dir = base.cache_dir().to_path_buf();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn scratch_dir_for_session() -> std::path::PathBuf {
    // 세션 내에서만 필요하고 앱 종료 시 정리해도 되는 순수 휘발성 파일은
    // std::env::temp_dir()이 정확한 선택이다.
    std::env::temp_dir().join(format!("bitvue-scratch-{}", std::process::id()))
}
```
- 재시작 후에도 필요한 캐시는 `directories` crate의 `cache_dir()`(macOS `~/Library/Caches/<bundle-id>`, Windows `%LOCALAPPDATA%`, Linux `~/.cache`)처럼 OS가 자동 정리하지 않는 앱 전용 위치에 저장한다.
- 진짜 세션 내 휘발성 스크래치 파일만 `temp_dir()`을 사용하고, 사용 후에는 앱이 직접 정리해 OS 정리 정책에 의존하지 않는다.
- 캐시가 사라져 있을 가능성을 항상 전제하고, 캐시 미스 시 원본에서 재생성하는 경로를 항상 함께 구현한다(캐시를 "있으면 좋고 없어도 되는" 것으로 설계).

**탐지 방법**:
- Static: `temp_dir()` 사용처 중 반환된 경로가 프로젝트 파일 등에 영속 저장되어 재시작 간 재사용을 기대하는 지점 코드 리뷰.
- Runtime: 임시 디렉터리를 강제로 비운 뒤(또는 며칠 경과를 시뮬레이션) 캐시 참조 기능이 정상적으로 재생성 경로를 타는지 확인.

**예외**:
- 앱 세션 내에서만 사용하고 앱 종료 시 명시적으로 정리하는 순수 스크래치 파일에는 `temp_dir()`이 올바른 선택이다.

**Bitvue 판정**: N/A — no decoded-frame disk cache is written to `std::env::temp_dir()` and persisted for reuse across restarts; the only `tempfile::TempDir` usage found is in test fixtures (auto-cleaned on drop).

---

### PLAT-017: separator를 문자열 replace로 처리

**분류**: 경로 구분자 처리 · **심각도**: Medium · **탐지**: Static / Structural

**나쁜 예**:
```rust
fn normalize_display_path(path: &str) -> String {
    // "보기 좋게" 통일한답시고 전역 문자열 치환으로 구분자를 바꿔치기
    path.replace('\\', "/")
}

fn to_windows_style(path: &str) -> String {
    path.replace('/', "\\")
}
```

**문제**:
- UNC 경로(`\\server\share\clip.hevc`)의 선행 이중 백슬래시를 단순 replace로 바꾸면 `//server/share/clip.hevc`가 되어, 일부 API에서는 "로컬 루트 아래의 server 디렉터리"처럼 완전히 다른 의미로 해석될 위험이 있다.
- Windows extended-length prefix(`\\?\C:\...`)가 붙은 경로를 전역 치환하면 prefix 구조 자체가 깨져 경로가 무효화된다.
- 드라이브 문자(`C:`) 뒤의 백슬래시까지 모두 바뀌면, 결과 문자열(`C:/Users/...`)이 일부 Rust API에서는 동작하지만 외부 프로세스 호출(`cmd.exe`, 일부 레거시 Win32 API)에는 전달할 수 없는 형태가 되어 그 지점에서만 실패한다.
- 이런 실수는 "구분자를 통일하면 크로스플랫폼이 되겠지"라는 생각에서 나오지만, 실제로는 경로의 구조적 의미(스킴, 드라이브, UNC prefix)를 텍스트로 취급해 깨뜨리는 결과를 낳는다.

**발생 조건**:
- Windows 경로를 로그나 UI에 "예쁘게" 통일해서 보여주려고 전역 치환을 적용했는데, 그 경로에 UNC 접두사나 `\\?\` verbatim prefix가 섞여 있을 때.
- 크로스플랫폼 설정 파일에 저장된 경로를 다른 OS에서 읽을 때 구분자를 바꿔서 재사용하려 할 때.

**권장**:
```rust
use std::path::{Component, Path, PathBuf};

// 경로 조합/분해는 항상 Path/PathBuf API로 수행한다.
fn join_relative(base: &Path, rel: &str) -> PathBuf {
    base.join(rel) // 구분자는 OS에 맞게 std가 알아서 처리
}

// 표시 전용 축약 경로가 필요하면 컴포넌트 단위로 다룬다.
fn display_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}
```
- 경로 조합·분해·정규화는 항상 `Path`/`PathBuf`의 `join()`, `components()`, `parent()` API로 수행하고, 구분자 문자를 문자열로 직접 다루지 않는다.
- 표시용 문자열이 필요하면 이미 파싱된 `Path`를 기반으로 별도 포맷 함수를 만들고, 구분자 치환이 아니라 컴포넌트 재조합으로 처리한다.
- UNC/verbatim prefix가 있는 경로는 표시 전에 `dunce` crate 등으로 사람이 읽기 쉬운 형태로 안전하게 정규화한다.

**탐지 방법**:
- Static: `.replace("\\", ...)` / `.replace('/', ...)` 패턴을 경로 관련 코드에서 grep.
- Structural: 경로를 다루는 함수의 시그니처가 `&str`/`String`인지 `&Path`/`PathBuf`인지 검토 — 전자가 많을수록 구분자 문자열 조작이 섞여 있을 가능성이 높다.

**예외**:
- 완전한 파일시스템 경로가 아니라 순수 표시용 짧은 레이블 문자열(경로가 아닌 단순 이름)을 다루는 경우는 해당하지 않는다.

**Bitvue 판정**: N/A — zero `.replace(` calls on path strings anywhere in `src-tauri/src`; no separator-swap logic exists.

---

### PLAT-018: app update 중 프로젝트 파일 손상

**분류**: 앱 업데이트/파일 무결성 · **심각도**: Critical · **탐지**: Runtime / Manual

**나쁜 예**:
```rust
fn save_project(path: &std::path::Path, project: &Project) -> std::io::Result<()> {
    // 직접 원본 경로에 write — 도중에 프로세스가 죽으면 파일이 반쯤 쓰인 채로 남음
    let json = serde_json::to_vec_pretty(project)?;
    std::fs::write(path, json)
}

fn on_update_available(app: &AppHandle) {
    // 미저장 세션 여부를 확인하지 않고 곧바로 재시작을 트리거
    app.restart();
}
```

**문제**:
- 프로젝트 파일 저장이 원자적이지 않은 상태(임시 파일 없이 원본에 직접 `write`)에서, 자동 업데이트가 백그라운드에서 앱 재시작을 트리거하면 저장이 완료되기 전에 프로세스가 종료되어 파일이 잘린 채로 남을 수 있다.
- 업데이트 확인 없이 곧바로 재시작하면, 사용자가 큰 분석 세션(마커, 주석, 오버레이 설정)을 저장하지 않은 상태에서 작업 내용이 통째로 사라진다 — 이는 `TAURI_WEB.md`의 창 닫기 미확인 문제(TAURI-WEB-006)와 같은 근본 원인이 업데이트 트리거에도 존재하는 경우다.
- 프로젝트 파일 포맷이 버전 간 바뀌었는데, 새 버전이 구 포맷을 마이그레이션 없이 그대로 열려다 파싱에 실패하면 "업데이트했더니 프로젝트가 깨졌다"는 인상을 준다.
- 업데이트 적용 도중 크래시가 나면 실행 바이너리와 동봉된 네이티브 라이브러리의 버전이 서로 어긋난 "반쯤 업데이트된" 상태가 될 수 있어, 이후 파일 저장/로드 동작 자체가 예측 불가능해진다.

**발생 조건**:
- 자동 업데이트가 활성화된 상태에서 사용자가 큰 세션을 저장하지 않고 방치해두다 업데이트 알림의 "지금 재시작"을 누를 때.
- 프로젝트 파일 포맷이 버전 간 변경되었는데 마이그레이션 로직이 업데이트 순서와 맞물리지 않을 때.
- 업데이트 다운로드/적용 도중 네트워크 단절이나 디스크 공간 부족으로 프로세스가 비정상 종료될 때.

**권장**:
```rust
fn save_project_atomic(path: &std::path::Path, project: &Project) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let json = serde_json::to_vec_pretty(project)?;
    let mut f = std::fs::File::create(&tmp)?;
    std::io::Write::write_all(&mut f, &json)?;
    f.sync_all()?; // fsync로 디스크 반영을 보장한 뒤에만 교체
    std::fs::rename(&tmp, path)?; // 원자적 교체
    Ok(())
}

async fn on_update_available(app: &AppHandle, has_unsaved: bool) {
    if has_unsaved && !confirm_discard_or_save().await {
        return; // 사용자가 저장/취소를 선택할 때까지 업데이트 재시작을 보류
    }
    app.restart(); // 확인이 끝난 뒤에만 재시작
}
```
- 프로젝트 파일 저장은 항상 임시 파일 쓰기 + `fsync` + `rename`(원자적 교체) 패턴을 쓴다.
- 업데이트 적용 전에는 창 닫기와 동일한 미저장 확인 흐름을 재사용해, 사용자 확인 없이 재시작이 트리거되지 않게 한다.
- 업데이터 자체는 새 바이너리를 별도 경로에 내려받아 두고 실제 교체는 다음 실행 시점에만 수행하는 방식(플랫폼 표준 업데이터가 흔히 쓰는 패턴)을 사용해, 실행 중인 바이너리를 직접 덮어쓰지 않는다.
- 프로젝트 파일에 포맷 버전 필드를 두고, 앱 시작 시 버전 체크 후 필요하면 명시적 마이그레이션을 거치게 한다.

**탐지 방법**:
- Runtime: 저장 도중(또는 업데이트 적용 도중) 프로세스를 강제 종료(`kill -9`)한 뒤 프로젝트 파일 무결성과 재실행 시 복구 가능 여부를 확인.
- Manual: 미저장 변경 사항이 있는 상태에서 업데이트 알림의 "지금 재시작"을 눌러 확인 다이얼로그가 뜨는지 검증.

**예외**:
- 프로젝트가 매 변경마다 즉시 원자적으로 자동 저장되어 "미저장 상태"가 설계상 존재하지 않고, 업데이트 재시작도 항상 확인 절차를 거친다면 위험이 낮다.

**Bitvue 판정**: N/A — no auto-updater plugin is configured (`src-tauri/Cargo.toml` has no `tauri-plugin-updater`) and no persistent project-file save/load feature exists yet to corrupt.

---

### PLAT-019: crash dump 위치가 플랫폼마다 다름

**분류**: 크래시 덤프 위치 · **심각도**: Low · **탐지**: Manual / Static

**나쁜 예**:
```rust
fn crash_report_hint() -> &'static str {
    // 문서/에러 메시지에 macOS 경로만 하드코딩해 안내
    "크래시 로그는 ~/Library/Logs/DiagnosticReports 에서 확인할 수 있습니다"
}
```

**문제**:
- macOS는 크래시 리포트가 `~/Library/Logs/DiagnosticReports/`(또는 통합 크래시 리포터)에 자동 생성되지만, Windows는 WER(Windows Error Reporting)이 `%LOCALAPPDATA%\CrashDumps`나 이벤트 뷰어에 기록하며 기본 설정으로는 전체 메모리 덤프가 남지 않을 수 있어 macOS 안내를 그대로 옮겨 적용할 수 없다.
- Linux는 배포판에 따라 core dump가 기본적으로 비활성화(`ulimit -c 0`)되어 있거나, `systemd-coredump`/ABRT 등으로 저장 위치와 형식이 배포판마다 제각각이다.
- 자체 크래시 핸들러(breakpad, sentry-native 등)를 도입해도 minidump 기본 저장 경로가 라이브러리·플랫폼마다 다르며, 이를 앱이 명시적으로 재정의하지 않으면 지원 문서와 실제 경로가 어긋난다.
- 사용자가 크래시를 겪고 "로그를 첨부해달라"는 안내를 받았을 때, 문서가 한 OS 경로만 언급하면 다른 OS 사용자는 아예 파일을 찾지 못해 지원 프로세스가 막힌다.

**발생 조건**:
- 사용자가 크래시를 겪고 지원 채널에 로그 첨부를 요청받았는데, 문서/에러 메시지가 한 플랫폼의 경로만 안내할 때.
- 자동 크래시 리포트 업로드 기능이 플랫폼별 기본 덤프 경로 차이로 일부 OS에서만 파일을 찾지 못해 조용히 업로드에 실패할 때.

**권장**:
```rust
fn crash_dump_dir() -> std::path::PathBuf {
    // 앱이 직접 크래시 핸들러(breakpad/sentry-native 등)를 등록해
    // 플랫폼에 관계없이 항상 동일한 앱 전용 경로를 사용하게 한다.
    let base = directories::ProjectDirs::from("com", "bitvue", "Bitvue")
        .expect("directories 확인 실패");
    base.data_local_dir().join("crashes")
}

fn crash_report_hint() -> String {
    format!(
        "진단 정보는 앱 메뉴의 '진단 정보 보내기'로 자동 전송하거나, {} 에서 직접 확인할 수 있습니다.\n\
         (자체 수집 실패 시 OS 기본 위치: macOS `~/Library/Logs/DiagnosticReports`, \
         Windows `%LOCALAPPDATA%\\CrashDumps`, Linux `journalctl`/`coredumpctl`)",
        crash_dump_dir().display()
    )
}
```
- 크로스플랫폼 크래시 핸들러(breakpad, sentry-native 등)를 앱이 직접 등록해, 덤프 저장 경로를 OS 기본값에 맡기지 않고 `directories` crate의 `data_local_dir()` 하위 고정 폴더로 앱이 직접 지정한다.
- "진단 정보 보내기" 같은 인앱 기능을 제공해 사용자가 경로를 몰라도 되게 하고, 자동 수집이 실패했을 때를 대비해 OS별 폴백 경로를 지원 문서에 모두 명시한다.

**탐지 방법**:
- Manual: 각 OS에서 의도적으로 크래시를 유발(테스트 빌드에서 panic 트리거)한 뒤 실제 덤프/로그 생성 위치를 확인.
- Static: 크래시 핸들러 등록 코드에서 저장 경로가 명시적으로 설정되어 있는지, 지원 문서가 플랫폼별 경로를 모두 언급하는지 검토.

**예외**:
- 크래시 리포팅을 자동화하지 않고 사용자에게 재현 스텝만 받는 지원 정책이라면 덤프 위치 문제의 우선순위는 낮다.

**Bitvue 판정**: N/A — no crash-reporting/dump handling code (breakpad/sentry-native or similar) exists in the codebase; feature not yet built.

---

### PLAT-020: native dialog 결과의 canonicalization 차이 무시

**분류**: 다이얼로그 경로 정규화 · **심각도**: Medium · **탐지**: Static / Runtime

**나쁜 예**:
```rust
fn add_to_recent(dialog_result_path: String, recent: &mut std::collections::HashMap<String, ()>) {
    // 다이얼로그가 반환한 문자열을 정규화 없이 그대로 식별자(dedup 키)로 사용
    recent.insert(dialog_result_path, ());
}
```

**문제**:
- macOS 파일 다이얼로그는 `/var/...`가 실제로는 `/private/var/...`의 심볼릭 링크인 것처럼, 표시 경로와 실제 canonical 경로가 다른 문자열을 반환할 수 있어 같은 파일을 다른 세션에서 열어도 문자열 자체가 달라 "다른 파일"로 중복 등록된다.
- Windows 다이얼로그 경로는 매핑된 네트워크 드라이브 문자(`Z:\project\clip.hevc`)와 그 드라이브가 가리키는 UNC 경로(`\\nas\share\project\clip.hevc`)가 같은 파일을 가리키더라도, 사용자가 드라이브를 다시 매핑하거나 다른 컴퓨터에서 열면 서로 다른 문자열로 반환되어 동일성 판단이 깨진다.
- 케이스 보존 파일시스템(NTFS, APFS 기본 설정)에서는 실제 디스크의 표기와 다이얼로그가 사용자 입력 시점에 반환하는 대소문자 표기가 다를 수 있어(`Clip.hevc` vs `clip.hevc`), 단순 문자열 dedup이 대소문자 차이로 실패한다(PLAT-002와 연결되는 지점).
- 이 모든 경우 "다이얼로그가 반환한 문자열은 항상 안정적인 식별자"라는 가정이 깨지며, 그 결과 최근 파일 목록에 같은 파일이 중복으로 쌓이거나 반대로 다른 파일을 같은 항목으로 잘못 병합하는 실수로 이어질 수 있다.

**발생 조건**:
- 같은 파일을 여러 세션에서 반복해 열어 최근 파일 목록에 중복 항목이 쌓일 때.
- 네트워크 드라이브와 그에 대응하는 UNC 경로를 오가며 같은 공유 폴더의 파일을 열 때.
- 심볼릭 링크가 낀 홈 디렉터리 구조(macOS `/Users` → `/System/Volumes/Data/Users`)를 가진 환경에서 다이얼로그로 파일을 반복해 열 때.

**권장**:
```rust
fn add_to_recent(dialog_result_path: &std::path::Path, recent: &mut Vec<RecentEntry>) -> std::io::Result<()> {
    // Windows에서 \\?\ verbatim prefix 부작용 없이 정규화하려면 dunce 사용
    let canonical = dunce::canonicalize(dialog_result_path)?;

    if let Some(existing) = recent.iter_mut().find(|e| e.canonical == canonical) {
        existing.last_opened = now(); // 이미 있는 항목이면 시간만 갱신
        return Ok(());
    }
    recent.push(RecentEntry {
        canonical,
        display: dialog_result_path.to_path_buf(), // 표시용 원본은 별도 보관
        last_opened: now(),
    });
    Ok(())
}
```
- 다이얼로그가 반환한 경로는 사용 전에 즉시 `canonicalize`(Windows는 `dunce::canonicalize`로 verbatim prefix 부작용 없이)로 정규화한 뒤 식별자로 사용한다.
- 원본 표시 문자열(사용자가 다이얼로그에서 실제로 본 경로)은 별도 필드로 보관해, 정규화된 식별자와 표시용 문자열의 역할을 분리한다.
- 더 엄밀한 동일성 판정이 필요하면 canonical 경로 비교에 더해 OS 레벨 파일 식별자(inode/볼륨 시리얼 등)까지 비교하는 `same-file` crate 사용을 고려한다.

**탐지 방법**:
- Static: 다이얼로그(`dialog.open`/`FileDialog` 등) 반환값을 canonicalize 없이 곧바로 `HashMap`/`Vec`의 dedup 키로 사용하는 지점 grep.
- Runtime: 심볼릭 링크가 낀 경로와 매핑된 네트워크 드라이브를 통해 같은 파일을 서로 다른 경로 표현으로 두 번 열어 최근 파일 목록에 중복이 생기는지 확인.

**예외**:
- 다이얼로그 결과를 그 세션 내에서 1회성으로만 사용하고 저장·비교하지 않는다면 canonicalization 부담 없이 그대로 사용해도 무방하다.

**Bitvue 판정**: Confirmed — `recent_files.rs:112` uses the raw dialog-returned path string as the dedup key (`e.path != sanitized_path`) with no `canonicalize()`/same-file check, matching the anti-pattern exactly; contrast with `file.rs:41,79` which does canonicalize, but only for security validation, not identity/dedup.
</content>
