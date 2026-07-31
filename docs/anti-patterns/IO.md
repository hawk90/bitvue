# Anti-Pattern Catalog — IO: 파일 I/O와 Zero-copy

이 문서는 Bitvue 안티패턴 카탈로그의 한 파트입니다. 전체 카탈로그 목록과 분류 체계는 `docs/anti-patterns/INDEX.md`(별도 작성)를 참고하세요. 아래 항목들은 memmap2/bytes/lru 기반으로 수 GB 크기의 비디오 파일을 다루는 Rust 코드베이스를 염두에 두고 작성된 일반 참조용 카탈로그이며, Bitvue 저장소 자체를 대상으로 한 감사는 2단계에서 별도로 진행됩니다.

---

### IO-001: std::fs::read로 전체 파일 로드
**분류**: IO · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
fn load_video(path: &Path) -> std::io::Result<Vec<u8>> {
    // 수 GB짜리 MKV/MP4 파일 전체를 힙에 올린다.
    let data = std::fs::read(path)?;
    parse_container(&data)
}
```

**문제**:
- 파일 크기만큼 즉시 힙 할당이 발생하며, 4K/8K RAW 소스나 장시간 캡처본은 수십 GB에 달해 OOM으로 직결된다.
- 첫 바이트를 읽기 전에 전체 파일을 디스크에서 읽어야 하므로 초기 로딩 지연(TTFB)이 파일 크기에 비례해 증가한다.
- 파싱이 끝난 뒤에도 `Vec<u8>`이 스코프에 남아있으면 이중으로 메모리를 낭비한다(원본 버퍼 + 파싱된 구조체).
- 컨테이너 포맷은 대부분 헤더/인덱스만 먼저 읽고 나머지는 필요할 때 접근하면 되는데, 이 패턴은 그 이점을 전혀 활용하지 못한다.

**발생 조건**:
- 사용자가 열려는 파일이 시스템 가용 메모리보다 크거나 비슷한 경우.
- CI/테스트 환경처럼 메모리 제한이 낮은 컨테이너에서 실행될 때 특히 치명적이다.
- "일단 동작하게" 빠르게 프로토타이핑한 뒤 정리하지 않고 남은 코드에서 흔함.

**권장**:
```rust
use memmap2::Mmap;
use std::fs::File;

fn load_video(path: &Path) -> std::io::Result<Mmap> {
    let file = File::open(path)?;
    // 커널이 필요한 페이지만 로드하도록 위임한다.
    let mmap = unsafe { Mmap::map(&file)? };
    Ok(mmap)
}
```
- 컨테이너 헤더나 인덱스 박스(moov, EBML seek head 등)만 스트리밍으로 먼저 읽고 나머지는 mmap 또는 `Seek`+`Read`로 지연 접근한다.
- 전체 파일이 반드시 필요한 경우(예: 체크섬 계산)에도 청크 단위 스트리밍으로 처리해 피크 메모리를 상수로 유지한다.

**탐지 방법**:
- Static: `fs::read(` / `fs::read_to_string(` 호출부에서 인자가 사용자 지정 경로이고 결과가 대용량 파일일 가능성이 있는지 검토.
- Structural: 함수 시그니처가 `PathBuf -> Vec<u8>` 형태로 파일 전체를 반환하는 API를 리스트업.
- Runtime: 대용량 테스트 픽스처로 메모리 프로파일링(peak RSS)하여 파일 크기와 비례하는지 확인.

**예외**:
- 파일이 명백히 작다고 보장되는 경우(설정 파일, 프로젝트 메타데이터 JSON, 수 KB의 사이드카 파일).
- 전체 파일을 어차피 네트워크로 재전송해야 하는 등 지연 로딩이 의미 없는 워크로드.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-002: mmap 후 즉시 to_vec
**분류**: IO · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
fn read_nal_units(mmap: &Mmap) -> Vec<u8> {
    // zero-copy를 위해 mmap을 썼지만 바로 복사해버려서 의미가 없다.
    mmap[..].to_vec()
}

fn parse_frame(mmap: &Mmap, offset: usize, size: usize) -> Vec<u8> {
    mmap[offset..offset + size].to_vec()
}
```

**문제**:
- mmap을 사용하는 근본적인 이유(불필요한 복사 회피, 커널 페이지 캐시 공유)가 `to_vec()` 한 줄로 완전히 무효화된다.
- 프레임 단위로 `to_vec()`을 반복 호출하면 GOP 전체를 순회할 때 파일 크기만큼의 데이터를 다시 힙에 복사하는 셈이 된다.
- mmap의 장점인 "커널이 알아서 페이지를 evict/reload"하는 메모리 압박 대응 능력을 잃고, 애플리케이션이 직접 관리하는 `Vec` 더미로 되돌아간다.

**발생 조건**:
- mmap 슬라이스를 다른 함수에 넘기려는데 라이프타임이 얽혀서 "일단 복사해서 넘기자"는 선택을 할 때.
- `&[u8]`을 받는 기존 API와 `Vec<u8>`을 요구하는 새 API가 섞여 있을 때 타입을 맞추려고 무심코 삽입.

**권장**:
```rust
use bytes::Bytes;

// mmap을 Arc로 감싸고 Bytes로 슬라이스 뷰만 만든다 (참조 카운트만 증가, 데이터 복사 없음).
struct MappedFile {
    mmap: std::sync::Arc<Mmap>,
}

impl MappedFile {
    fn nal_view(&self, offset: usize, size: usize) -> &[u8] {
        &self.mmap[offset..offset + size]
    }

    // 소유권이 꼭 필요한 경우에만, bytes::Bytes로 zero-copy 참조 카운팅 복사.
    fn nal_bytes(self_mmap: std::sync::Arc<Mmap>, offset: usize, size: usize) -> Bytes {
        Bytes::from_owner(self_mmap).slice(offset..offset + size)
    }
}
```
- 가능한 한 `&[u8]` 슬라이스 뷰로 끝까지 처리하고, 소유권 이전이 꼭 필요할 때만 참조 카운팅 기반 zero-copy 타입(`bytes::Bytes`, `Arc<Mmap>` 슬라이스 래퍼)을 사용한다.
- "라이프타임이 복잡해서 복사한다"는 판단이 들면 우선 `Arc` 공유나 인덱스(offset/size) 전달로 해결 가능한지 검토한다.

**탐지 방법**:
- Structural: `Mmap`/`MmapMut` 타입 값에 대해 `.to_vec()`, `.to_owned()`, `Vec::from(&mmap[..])` 호출을 grep.
- Static: linter 규칙으로 "mmap 파생 슬라이스에 대한 owned 변환"을 경고.
- Manual: 코드리뷰에서 "이 mmap을 왜 여기서 읽었는가"를 추적해 즉시 복사되는 지점을 찾는다.

**예외**:
- 매우 작은 고정 크기 헤더(수십 바이트)를 파싱 편의를 위해 스택 배열이나 작은 `Vec`으로 복사하는 것은 성능에 영향이 없다.
- 복사된 버퍼가 mmap보다 훨씬 오래 살아야 하고(파일이 닫힌 뒤에도 캐시), 재파일오픈 비용이 복사 비용보다 클 때.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-003: Bytes::copy_from_slice 남용
**분류**: IO · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
use bytes::Bytes;

fn extract_payload(mmap: &[u8], offset: usize, len: usize) -> Bytes {
    // copy_from_slice는 항상 새 힙 버퍼를 할당하고 복사한다.
    Bytes::copy_from_slice(&mmap[offset..offset + len])
}

fn demux_packets(mmap: &[u8], packets: &[(usize, usize)]) -> Vec<Bytes> {
    packets
        .iter()
        .map(|&(off, len)| Bytes::copy_from_slice(&mmap[off..off + len]))
        .collect()
}
```

**문제**:
- `bytes` 크레이트를 도입한 목적은 참조 카운팅 기반 zero-copy 슬라이싱인데, `copy_from_slice`를 쓰면 매번 새 할당+복사가 일어나 `Vec<u8>`을 쓰는 것과 다를 바 없다.
- 패킷 수천~수만 개를 순회하며 이 함수를 호출하면 힙 할당기(allocator) 압박과 메모리 단편화가 누적된다.
- 원본 데이터가 이미 `Bytes`(또는 `Arc<Mmap>` 기반 뷰)라면 `.slice()`로 참조만 늘리면 되는데 이를 놓친 것.

**발생 조건**:
- 소스가 `&[u8]`(mmap 슬라이스, `&[u8]` 파라미터)이라 `Bytes`로 감싸려면 어차피 복사해야 한다고 오해할 때.
- 이미 `Bytes` 타입인 버퍼에서 부분 구간을 뽑아내면서 습관적으로 `copy_from_slice`를 사용할 때.

**권장**:
```rust
use bytes::Bytes;

// 소스가 이미 Bytes라면 슬라이스만 한다 (Arc 참조 카운트 증가, 복사 없음).
fn extract_payload(source: &Bytes, offset: usize, len: usize) -> Bytes {
    source.slice(offset..offset + len)
}

// mmap을 감싼 Bytes를 최상위에서 한 번만 만들고 이후에는 slice()로 재사용한다.
struct FileBackedBuffer {
    data: Bytes, // Bytes::from_owner(Arc<Mmap>) 등으로 최초 1회만 구성
}

impl FileBackedBuffer {
    fn packet(&self, off: usize, len: usize) -> Bytes {
        self.data.slice(off..off + len)
    }
}
```
- `Bytes::copy_from_slice`는 "이 데이터의 수명이 원본과 무관해야 하고, 복사 비용을 감수할 가치가 있다"는 것이 명확할 때만 사용한다.
- 원본이 이미 `Bytes`/`Arc` 기반이라면 `.slice()`, `.slice_ref()` 계열 API로 항상 참조 카운트 증가만 발생시킨다.

**탐지 방법**:
- Structural: `Bytes::copy_from_slice(` 호출 빈도를 카운트하고 루프/반복 파싱 경로에서의 호출을 별도 표시.
- Static: 동일 함수 내에서 `Bytes` 타입 변수가 이미 존재하는데 새로 `copy_from_slice`를 호출하는 패턴 탐지.
- Runtime: allocator 프로파일러(`dhat`, `heaptrack`)로 패킷 파싱 경로의 할당 횟수를 측정.

**예외**:
- FFI 경계를 넘어 C 라이브러리에 버퍼를 넘겨야 해서 어차피 소유권이 분리된 복사본이 필요한 경우.
- 원본 mmap이 곧 unmap될 예정이라 데이터를 반드시 독립시켜야 하는 짧은 순간(예: 파일 교체 직전 스냅샷).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-004: packet payload를 ownership 경계마다 복사
**분류**: IO · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct Demuxer;
struct Decoder;
struct OverlayRenderer;

impl Demuxer {
    fn next_packet(&mut self) -> Vec<u8> { /* mmap에서 읽어 Vec으로 복사 */ vec![] }
}

impl Decoder {
    // 함수 경계를 넘을 때마다 새 Vec으로 복사해서 저장
    fn decode(&mut self, payload: &[u8]) -> Vec<u8> {
        let owned = payload.to_vec();
        self.internal_buffer = owned.clone(); // 여기서 또 한 번 복사
        vec![] // 디코딩 결과
    }
}
```

**문제**:
- 하나의 패킷이 `demux -> decode -> overlay -> render` 파이프라인을 지나며 모듈 경계마다 "안전하게" 복사되어, 실질적으로 N단계 파이프라인이면 N배의 메모리 트래픽이 발생한다.
- 각 모듈이 독립적으로 개발되면서 "내 모듈은 입력을 소유해야 안전하다"는 방어적 설계가 습관화되어 성능 문제를 인지하지 못한 채 누적된다.
- 대형 프레임(4K raw, 무손실 인트라 프레임 등)에서는 이 복사 비용이 실제 디코딩 연산보다 커질 수 있다.

**발생 조건**:
- 여러 팀/모듈이 독립적으로 개발되어 서로의 라이프타임 계약을 신뢰하지 못하고 방어적으로 복사할 때.
- 비동기 태스크 경계(예: `tokio::spawn`)를 넘기기 위해 `'static` 소유권이 필요하다고 판단해 복사를 정당화할 때 — 실제로는 `Arc`/`Bytes`로 해결 가능한 경우가 많다.

**권장**:
```rust
use bytes::Bytes;

struct Demuxer;
struct Decoder;

impl Demuxer {
    fn next_packet(&mut self) -> Bytes { /* mmap 기반 Bytes 슬라이스, 복사 없음 */ Bytes::new() }
}

impl Decoder {
    // Bytes는 clone이 Arc 참조 카운트 증가일 뿐, 데이터 복사가 아니다.
    fn decode(&mut self, payload: Bytes) -> Bytes {
        self.internal_buffer = payload.clone();
        Bytes::new()
    }
}
```
- 파이프라인 전 구간에서 공통 zero-copy 타입(`Bytes`)을 표준으로 정하고, 모듈 경계에서 `.clone()`은 참조 카운트 증가만 의미하도록 설계한다.
- 비동기 태스크로 넘길 때도 `Bytes`/`Arc<[u8]>`는 `'static` + `Send`를 만족하므로 굳이 `Vec`으로 복사할 필요가 없다.

**탐지 방법**:
- Structural: 모듈 간 함수 시그니처를 스캔해 `&[u8] -> Vec<u8> -> &[u8] -> Vec<u8>` 형태로 타입이 계속 바뀌는 체인을 찾는다.
- Runtime: 프레임 하나를 파이프라인에 흘려보내며 각 단계의 메모리 할당량을 트레이싱(`tracing` + custom allocator hook).
- Manual: 파이프라인 설계 문서에서 "이 경계는 왜 소유권 이전이 필요한가"를 단계별로 질문.

**예외**:
- 파이프라인 단계 사이에 실제로 데이터 변환(디코딩, 색공간 변환 등)이 일어나 원본과 무관한 새 버퍼가 필요한 지점은 복사가 아니라 "생성"이므로 해당 없음.
- 별도 프로세스/스레드로 격리되어 메모리 공유가 안전상 금지된 플러그인 경계(FFI, sandbox).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-005: 작은 random read를 반복 수행
**분류**: IO · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn read_nal_header(file: &mut File, offset: u64) -> std::io::Result<[u8; 4]> {
    let mut buf = [0u8; 4];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut buf)?; // 4바이트를 위해 syscall 2회(seek+read)
    Ok(buf)
}

fn scan_all_nal_headers(file: &mut File, offsets: &[u64]) -> std::io::Result<Vec<[u8; 4]>> {
    offsets.iter().map(|&off| read_nal_header(file, off)).collect()
}
```

**문제**:
- NAL 헤더 하나(4바이트)를 읽기 위해 `seek` + `read` syscall 두 번을 호출하며, 수만 개 NAL을 스캔하면 syscall 오버헤드가 실제 유효 처리량을 압도한다.
- 커널 컨텍스트 스위치 비용은 요청 크기와 무관하게 고정 비용에 가까워서, 작은 read를 여러 번 하는 것이 큰 read 한 번보다 훨씬 비효율적이다.
- 로컬 SSD에서는 그나마 견딜 만하지만 네트워크 파일시스템(NFS, SMB) 위에서는 각 syscall이 왕복 지연(RTT)을 유발해 수십~수백 배 느려질 수 있다.

**발생 조건**:
- 인덱스(offset table)를 순회하며 각 엔트리마다 파일을 개별적으로 접근하는 스캔/검증 로직.
- 파일 포맷 파서가 "필요한 만큼만 정확히 읽는다"는 원칙을 과도하게 적용해 배치 읽기를 고려하지 않았을 때.

**권장**:
```rust
use memmap2::Mmap;

fn scan_all_nal_headers(mmap: &Mmap, offsets: &[u64]) -> Vec<[u8; 4]> {
    // 커널 페이지 캐시가 mmap을 통해 자연스럽게 재사용되고,
    // syscall 없이 메모리 접근만으로 헤더를 읽는다.
    offsets
        .iter()
        .map(|&off| {
            let off = off as usize;
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&mmap[off..off + 4]);
            buf
        })
        .collect()
}
```
- 무작위 소규모 접근이 반복되는 패턴에는 `File::read`보다 mmap이 압도적으로 유리하다(페이지 폴트는 있지만 syscall 왕복이 없다).
- mmap을 쓸 수 없는 상황(파일 디스크립터 제한, 원격 파일시스템)이라면 인접한 오프셋들을 정렬 후 하나의 큰 `read`로 묶어 요청 횟수를 줄인다(batching/coalescing).

**탐지 방법**:
- Runtime: `strace -c` / `dtruss`로 `seek`+`read` syscall 횟수를 측정하고 처리한 바이트 수 대비 syscall 수 비율을 확인.
- Structural: 루프 내부에서 매 반복마다 `seek` 후 소량(<4KB) `read`를 호출하는 패턴 탐지.
- Manual: 인덱스 기반 스캔 함수의 시간 복잡도를 offsets 개수 대비 측정해 선형 이상으로 나빠지는지 확인.

**예외**:
- 접근 빈도가 극히 낮은 경로(사용자가 한 번 클릭했을 때만 실행)에서는 syscall 오버헤드가 체감되지 않는다.
- 파일이 이미 정렬된 순차 접근 패턴이라 각 read가 사실상 연속적인 경우(=이 안티패턴에 해당하지 않음).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-006: seek와 read를 여러 worker가 공유
**분류**: IO · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::sync::RwLock;

// File 자체는 Sync지만, seek+read는 원자적이지 않다!
fn worker(file: Arc<RwLock<File>>, offset: u64, len: usize) -> Vec<u8> {
    let mut f = file.write().unwrap();
    f.seek(SeekFrom::Start(offset)).unwrap(); // (A) 다른 스레드가 여기서 끼어들 수 있다
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf).unwrap();           // (B) 잘못된 위치를 읽을 위험
    buf
}
```

**문제**:
- `RwLock<File>`로 감쌌더라도 `seek`와 뒤이은 `read`는 두 개의 개별 syscall이라 그 사이에 락을 놓치면(또는 lock 스코프 설계 실수) 다른 스레드가 커서를 옮겨버릴 수 있다.
- 위 예시처럼 write lock을 함수 전체에서 들고 있으면 사실상 모든 I/O가 직렬화되어 멀티스레드의 이점이 사라진다(IO-007과 연결되는 문제).
- 파일 커서 위치라는 전역 가변 상태를 여러 워커가 공유하는 설계 자체가 근본적으로 경쟁 조건에 취약하다.

**발생 조건**:
- 프레임 디코딩을 여러 스레드로 병렬화하면서 파일 핸들 하나를 공유 자원으로 잘못 설계했을 때.
- `Read + Seek`만 요구하는 기존 API를 그대로 멀티스레드 환경에 옮기면서 동시성 안전성을 재검토하지 않았을 때.

**권장**:
```rust
use std::fs::File;
use std::os::unix::fs::FileExt; // read_at: 커서 상태와 무관한 positioned read

fn worker(file: &File, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    // read_at은 내부적으로 pread(2)를 사용해 커서를 공유하지 않는다.
    file.read_at(&mut buf, offset)?;
    Ok(buf)
}

// 또는 mmap을 각 워커가 독립적으로 슬라이싱 (진짜 병렬, 락 불필요)
fn worker_mmap(mmap: &memmap2::Mmap, offset: usize, len: usize) -> &[u8] {
    &mmap[offset..offset + len]
}
```
- Unix에서는 `FileExt::read_at`(`pread`), Windows에서는 `FileExt::seek_read`처럼 커서 상태에 의존하지 않는 positioned I/O API를 사용한다.
- mmap 기반이라면 `Mmap`은 `Sync`이므로 워커마다 락 없이 서로 다른 구간을 동시에 읽을 수 있다 — 진짜 병렬 I/O가 가능해진다.

**탐지 방법**:
- Static: `Arc<Mutex<File>>` / `Arc<RwLock<File>>` 타입과 함께 `seek` 다음 줄에 `read`가 오는 패턴을 탐지.
- Structural: `File`을 여러 스레드/태스크에 공유하면서 `read_at`/`seek_read`를 쓰지 않는 코드를 찾는다.
- Runtime: 동시성 스트레스 테스트(여러 워커가 다른 오프셋을 동시에 요청)로 읽은 데이터의 오프셋 정합성을 검증(체크섬 비교).

**예외**:
- 단일 스레드에서만 파일에 접근하도록 아키텍처가 보장되어 있는 경우(예: 전용 I/O 스레드 + 채널로 요청을 직렬화).
- 플랫폼이 `pread`/`seek_read`를 지원하지 않는 특수 환경(드물지만 일부 임베디드 타깃)에서는 명시적 뮤텍스 직렬화가 유일한 대안일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-007: 하나의 Mutex<File>로 모든 I/O 직렬화
**분류**: IO · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct SharedFile {
    file: std::sync::Mutex<std::fs::File>,
}

// 프레임 디코딩, 썸네일 생성, hex view, export가 전부 이 하나의 락을 두고 경쟁
impl SharedFile {
    fn read_frame(&self, offset: u64, len: usize) -> Vec<u8> {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = self.file.lock().unwrap();
        f.seek(SeekFrom::Start(offset)).unwrap();
        let mut buf = vec![0u8; len];
        f.read_exact(&mut buf).unwrap();
        buf
    }
}
```

**문제**:
- 재생 스레드, 썸네일 프리로더, hex 뷰어, export 작업이 모두 같은 뮤텍스를 두고 경쟁하면서 실제로는 디스크가 병렬 처리를 감당할 수 있음에도 애플리케이션 레벨에서 순차화된다.
- UI 반응성이 백그라운드 export 작업 하나 때문에 막힐 수 있다 — 사용자가 스크러빙하는데 export의 큰 read가 락을 오래 붙잡고 있으면 재생이 끊긴다.
- 이 패턴은 "동시성 버그를 피하려고 안전한 걸 골랐다"는 판단에서 나오지만, 실제로는 IO-006에서 다룬 문제를 잘못된 방식(전면 직렬화)으로 해결한 것이다.

**발생 조건**:
- 여러 기능(재생/스크러빙/썸네일/hex view/export)이 동일한 파일 소스를 공유하는 멀티패널 UI(Bitvue와 같은 비디오 분석기의 전형적인 구조).
- 초기에는 단일 기능만 있어서 문제없다가, 기능이 늘어나며 공유 락이 병목이 되는 경우.

**권장**:
```rust
use std::sync::Arc;
use memmap2::Mmap;

// mmap은 불변 공유이므로 락이 필요 없다. 각 기능은 자신의 오프셋 구간만 읽는다.
struct SharedFile {
    mmap: Arc<Mmap>,
}

impl SharedFile {
    fn read_frame(&self, offset: u64, len: usize) -> &[u8] {
        &self.mmap[offset as usize..offset as usize + len]
    }
}

// 또는 파일 디스크립터 기반이라면 워커마다 독립 핸들 + pread로 락 없이 병렬화
```
- 읽기 전용 워크로드에는 mmap을 공유 상태로 채택해 락 자체를 제거한다.
- 파일 디스크립터 기반 I/O를 유지해야 한다면 기능별로 독립된 `File` 핸들을 열어(`File::open`을 여러 번, 커널이 동일 inode를 공유 캐시) 락 경합을 없앤다.
- 정말 직렬화가 필요한 자원(예: export의 쓰기 대상 파일)만 별도로 좁게 락을 건다.

**탐지 방법**:
- Structural: `Mutex<File>` / `RwLock<File>` 타입이 여러 개의 서로 다른 기능 모듈에서 참조되는지 의존성 그래프로 확인.
- Runtime: 락 획득 대기 시간을 계측(`tracing`의 span, 또는 `parking_lot`의 통계 기능)해 UI 스레드가 락 대기로 블로킹되는 비율 측정.
- Manual: "이 락이 보호하는 것이 진짜 공유 가변 상태인가, 아니면 단순히 파일 디스크립터 하나인가"를 코드리뷰에서 질문.

**예외**:
- 파일이 매우 작고 접근 빈도가 낮아 경합이 사실상 발생하지 않는 보조 파일(설정, 메타데이터 사이드카).
- 쓰기 작업이 대부분이라 어차피 직렬화가 정확성을 위해 필요한 경우(단일 export 파일에 대한 순차 append).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-008: mmap page fault가 UI thread에서 발생
**분류**: IO · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
// Tauri command 핸들러 — 기본적으로 호출 스레드(종종 메인/UI 관련 스레드)에서 실행될 수 있다.
#[tauri::command]
fn get_frame_bytes(state: tauri::State<AppState>, offset: usize, len: usize) -> Vec<u8> {
    let mmap = state.mmap.lock().unwrap();
    // 이 슬라이스 접근이 첫 접근이라면 major page fault가 발생해
    // 디스크(특히 네트워크 드라이브)에서 동기적으로 페이지를 읽어온다.
    mmap[offset..offset + len].to_vec()
}
```

**문제**:
- mmap 슬라이스에 처음 접근하는 순간 해당 페이지가 메모리에 없으면 커널이 디스크 I/O를 수행할 때까지 해당 스레드가 블로킹된다(major page fault) — 이는 명시적인 `await`나 `spawn_blocking` 없이도 발생하는 "숨은 블로킹 I/O"다.
- UI 이벤트 루프나 async 런타임의 워커 스레드에서 이 코드가 실행되면, 겉보기엔 순수 메모리 접근처럼 보이는 코드가 실제로는 수십~수백 ms의 디스크 지연을 유발해 프레임 드랍/입력 지연으로 나타난다.
- 특히 네트워크 드라이브나 외장 HDD 위의 파일이라면 page fault 하나가 수백 ms까지 걸릴 수 있어 체감 문제가 심각해진다.

**발생 조건**:
- Tauri command, GUI 콜백, async 태스크의 poll 함수 등 "블로킹하면 안 되는" 컨텍스트에서 mmap 슬라이스에 처음 접근할 때.
- 파일을 열자마자(콜드 캐시 상태) 사용자가 임의 프레임으로 즉시 점프(seek)할 때 특히 두드러진다.
- 스크러빙처럼 아직 페이지 캐시에 없는 새로운 파일 구간을 빠르게 순회할 때.

**권장**:
```rust
#[tauri::command]
async fn get_frame_bytes(state: tauri::State<'_, AppState>, offset: usize, len: usize) -> Result<Vec<u8>, String> {
    let mmap = state.mmap.clone();
    // 블로킹 가능한 mmap 접근을 전용 블로킹 스레드 풀로 위임한다.
    tauri::async_runtime::spawn_blocking(move || {
        mmap[offset..offset + len].to_vec()
    })
    .await
    .map_err(|e| e.to_string())
}
```
- mmap 최초 접근(콜드 페이지 가능성이 있는 모든 지점)은 `spawn_blocking`(Tokio) 또는 전용 I/O 스레드 풀에서 수행하고 결과만 async 경계로 넘긴다.
- 예측 가능한 접근 패턴(순차 재생)이라면 백그라운드에서 미리 페이지를 터치(prefetch, IO-012 참고)해 UI 스레드가 접근할 때는 이미 페이지 캐시에 적재된 상태로 만든다.

**탐지 방법**:
- Runtime: `perf`/`dtrace`로 major page fault를 UI 스레드/async 워커 스레드 기준으로 계측해 콜백 함수와 상관관계 확인.
- Structural: async 함수나 GUI 콜백 내부에서 mmap 슬라이스 인덱싱이 `spawn_blocking` 없이 직접 일어나는 지점을 탐지.
- Manual: Tauri command 핸들러 목록을 훑어 `async fn`이 아닌데 mmap에 접근하는 커맨드를 표시.

**예외**:
- 파일이 이미 애플리케이션 시작 시점에 전체 프리로드(`madvise(WILLNEED)` 등)되어 콜드 페이지가 사실상 없다고 보장되는 경우.
- 매우 작은 파일이라 page fault 비용이 프레임 예산(16ms) 내에서 무시 가능한 수준일 때.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-009: 파일 변경/truncate 상황 무시
**분류**: IO · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
struct VideoSource {
    mmap: memmap2::Mmap,
}

impl VideoSource {
    fn open(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Ok(Self { mmap }) // 이후 파일이 잘리거나 삭제되어도 아무런 대응이 없다
    }

    fn read_frame(&self, offset: usize, len: usize) -> &[u8] {
        &self.mmap[offset..offset + len] // 파일이 truncate되면 SIGBUS로 프로세스가 죽는다
    }
}
```

**문제**:
- mmap된 파일이 다른 프로세스(또는 사용자 자신이 export 도구로 덮어쓰기)에 의해 truncate되면, 이미 매핑된 구간을 접근할 때 `SIGBUS`가 발생해 프로세스 전체가 크래시한다 — Rust의 패닉/에러 처리로는 복구할 수 없는 하드웨어 트랩이다.
- 파일이 이동식 디스크나 네트워크 드라이브에 있고 그 사이 연결이 끊기면, 매핑이 여전히 유효한 것처럼 보이지만 실제 읽기 시점에 I/O 에러/SIGBUS가 발생한다.
- 애플리케이션이 오랫동안 파일을 열어두는(대용량 분석 세션) 특성상, "파일을 연 시점"과 "실제로 접근하는 시점" 사이의 간격이 커서 이 문제가 실제로 발생할 확률이 무시할 수 없다.

**발생 조건**:
- 사용자가 분석 중인 파일을 다른 애플리케이션(인코더, export 도구)으로 덮어쓰거나 삭제하는 경우.
- 네트워크 스토리지/외장 드라이브가 연결 해제되는 경우.
- 같은 프로세스 내에서 export 기능이 원본 파일을 in-place로 수정하는 설계상 결함과 결합될 때(IO-014와 연관).

**권장**:
```rust
use std::sync::atomic::{AtomicU64, Ordering};

struct VideoSource {
    mmap: memmap2::Mmap,
    expected_len: u64,
    expected_mtime: AtomicU64,
    path: std::path::PathBuf,
}

impl VideoSource {
    fn read_frame(&self, offset: usize, len: usize) -> Result<&[u8], VideoSourceError> {
        // 접근 전 메타데이터 검증으로 truncate/변경을 조기에 감지 (SIGBUS 자체는 막지 못하지만 위험도를 낮춘다)
        let meta = std::fs::metadata(&self.path).map_err(VideoSourceError::Io)?;
        if meta.len() < self.expected_len {
            return Err(VideoSourceError::FileTruncated);
        }
        if offset + len > self.mmap.len() {
            return Err(VideoSourceError::OutOfBounds);
        }
        Ok(&self.mmap[offset..offset + len])
    }
}

// 더 견고한 방법: 플랫폼별 SIGBUS 핸들러 설치(예: `region`/`signal-hook` crate)로
// 트랩을 잡아 panic으로 변환하거나, 애초에 mmap 대신 read_at으로 폴백.
```
- 파일 메타데이터(`len`, `mtime`)를 주기적/접근 전에 검증해 예상과 다르면 매핑을 무효화하고 사용자에게 알린다.
- 가능하면 exclusive lock(`flock`) 또는 OS 수준 파일 잠금으로 다른 프로세스의 수정 자체를 방지한다(협조적 잠금의 한계는 인지할 것).
- 정말 안전이 중요한 경로는 SIGBUS를 시그널 핸들러로 잡아 복구 가능한 에러로 변환하는 라이브러리를 도입하거나, 수정 가능성이 있는 파일에는 애초에 mmap 대신 `pread` 기반 I/O(에러를 정상적으로 반환)를 사용한다.

**탐지 방법**:
- Runtime: 파일을 연 상태에서 외부 프로세스로 truncate/삭제를 시뮬레이션하는 카오스 테스트로 크래시 여부 확인.
- Structural: mmap 필드를 장기 보관하는 구조체에 파일 메타데이터 검증/무효화 로직이 없는지 확인.
- Manual: export/편집 기능이 "현재 분석 중인 원본 파일"과 동일 경로를 쓰기 대상으로 삼는지 검토.

**예외**:
- 파일이 애플리케이션 자체 제어하에 있고 읽기 전용으로만 열리며, 삭제/수정 권한이 원천적으로 차단된 샌드박스 환경.
- 매핑 직후 즉시 처리하고 매핑을 짧게 유지하는 일회성 배치 작업(장시간 세션이 아닌 경우 위험도가 낮음).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-010: 32비트 주소 공간 고려 없음
**분류**: IO · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn map_file(path: &Path) -> std::io::Result<memmap2::Mmap> {
    let file = std::fs::File::open(path)?;
    // 파일 크기와 무관하게 항상 전체를 한 번에 매핑
    unsafe { memmap2::Mmap::map(&file) }
}
```

**문제**:
- 64비트 타깃에서는 가상 주소 공간이 사실상 무제한처럼 느껴져 문제가 드러나지 않지만, 32비트 빌드(임베디드 타깃, 일부 레거시 CI, WASM32 등)에서는 프로세스 가상 주소 공간이 2~4GB로 제한되어 대형 비디오 파일 하나를 통째로 매핑하는 것 자체가 실패한다.
- 여러 파일을 동시에 여는 멀티패널 UI(원본 비교, A/B 뷰)라면 32비트 환경에서 파일 하나만으로도 주소 공간을 소진할 수 있다.
- 코드가 "mmap이니까 메모리 문제 없다"고 안심하게 만들지만, mmap도 가상 주소 공간이라는 유한 자원을 소비한다는 사실을 놓친 것.

**발생 조건**:
- 크로스 컴파일 타깃에 32비트 아키텍처가 포함되어 있거나, 향후 포함될 가능성이 있는 프로젝트.
- 여러 개의 대형 파일을 동시에 열어야 하는 비교 분석 워크플로(레퍼런스 vs 인코딩 결과 비교 등).

**권장**:
```rust
fn map_file(path: &Path) -> std::io::Result<MappedRegion> {
    let file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();

    #[cfg(target_pointer_width = "32")]
    {
        // 32비트에서는 전체 매핑 대신 윈도우 단위로 필요한 구간만 매핑한다.
        return Ok(MappedRegion::Windowed(WindowedMapper::new(file, len, WINDOW_SIZE)));
    }
    #[cfg(target_pointer_width = "64")]
    {
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        return Ok(MappedRegion::Full(mmap));
    }
}
```
- `cfg(target_pointer_width = "32")`로 분기해 32비트 타깃에서는 파일 전체가 아닌 슬라이딩 윈도우(예: 256MB 단위)만 매핑하는 전략으로 전환한다.
- 64비트 전용으로 지원 범위를 명시적으로 좁힐 수도 있다 — 이 경우 빌드 타깃 문서에 명확히 기술하고 32비트 빌드를 CI에서 제외한다.
- 대형 파일 다중 비교 워크플로는 애초에 윈도우/청크 기반 접근을 기본 전략으로 채택해 32/64비트 모두에서 이점을 얻는다.

**탐지 방법**:
- Static: `Cargo.toml`/CI 매트릭스에 32비트 타깃(`i686-*`, `armv7-*`, `wasm32-*`)이 포함되는지 확인.
- Structural: mmap 매핑 크기 제한/윈도잉 로직 없이 `Mmap::map(&file)`을 파일 크기 검증 없이 호출하는 지점 탐지.
- Runtime: 32비트 빌드에서 대형 테스트 픽스처(수 GB)로 실제 매핑 성공 여부를 CI에서 검증.

**예외**:
- 프로젝트가 명시적으로 64비트 데스크톱(Tauri 데스크톱 앱 등)만 지원 대상으로 선언하고 32비트 빌드를 아예 지원하지 않는 경우, 이 항목은 낮은 우선순위로 강등 가능.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-011: 네트워크 파일과 로컬 파일 정책 동일
**분류**: IO · **심각도**: High · **탐지**: Manual

**나쁜 예**:
```rust
fn open_video(path: &Path) -> std::io::Result<VideoSource> {
    // 로컬 SSD든 SMB/NFS 마운트든 동일한 mmap + 동일한 random-access 전략을 적용
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    Ok(VideoSource { mmap })
}
```

**문제**:
- 로컬 NVMe SSD는 random access 지연이 마이크로초 단위지만, 네트워크 파일시스템(SMB, NFS, WebDAV 마운트, 클라우드 드라이브 동기화 폴더)은 밀리초~수십 밀리초 단위로 자릿수가 다르다 — 동일한 스크러빙 UX 전략(작은 조각을 자주 요청)이 로컬에서는 매끄럽지만 네트워크에서는 매우 버벅인다.
- mmap의 page fault는 네트워크 파일시스템에서 특히 예측 불가능한 지연을 유발하며, 일부 네트워크 FS는 mmap 자체를 완전히 지원하지 않거나 일관성 보장이 약하다(파일이 원격에서 변경되어도 로컬 캐시가 갱신되지 않는 등).
- "파일 경로만 다르고 나머지는 동일한 코드 경로"라는 가정이 네트워크 환경에서 사용자 경험을 크게 저하시키는데, 이를 감지할 계측이 없으면 버그 리포트가 "가끔 느리다"는 재현 어려운 형태로 들어온다.

**발생 조건**:
- 사용자가 NAS, 클라우드 동기화 폴더(iCloud Drive, OneDrive, Google Drive 데스크톱), 회사 네트워크 드라이브에 있는 영상 파일을 직접 여는 워크플로.
- 원격 마운트 여부를 사전에 감지하지 않고 로컬 파일과 동일 코드 경로를 강제하는 아키텍처.

**권장**:
```rust
enum StorageKind {
    Local,
    Network { estimated_rtt_ms: u32 },
}

fn detect_storage_kind(path: &Path) -> StorageKind {
    // 플랫폼별 API(statfs, GetVolumeInformation 등)나 마운트 테이블 조회로 판별
    // 간략화된 예시
    if is_network_mount(path) {
        StorageKind::Network { estimated_rtt_ms: probe_rtt(path) }
    } else {
        StorageKind::Local
    }
}

fn choose_io_strategy(kind: &StorageKind) -> IoStrategy {
    match kind {
        StorageKind::Local => IoStrategy::Mmap { window: None },
        StorageKind::Network { .. } => IoStrategy::Buffered {
            // 네트워크에서는 요청 횟수를 줄이는 것이 최우선: 큰 청크로 미리 당겨온다.
            chunk_size: 8 * 1024 * 1024,
            prefetch_depth: 4,
            local_cache_dir: Some(local_scratch_dir()),
        },
    }
}
```
- 열기 시점에 저장소 종류를 감지(마운트 테이블, `statfs` 등)하고 로컬/네트워크에 따라 서로 다른 I/O 전략(mmap vs 대형 청크 버퍼링 + 로컬 캐시)을 선택한다.
- 네트워크 파일은 접근 시 로컬 스크래치 디스크로 청크 단위 캐싱을 적극 활용해 반복 접근 시 재요청을 피한다.
- UI 레벨에서 "네트워크 드라이브의 파일입니다 — 성능이 느릴 수 있습니다"와 같은 사용자 피드백을 제공해 기대치를 관리한다.

**탐지 방법**:
- Manual: 파일 열기 경로에 저장소 종류 분기가 존재하는지 코드리뷰에서 확인.
- Runtime: 실제 NAS/클라우드 동기화 폴더에 있는 파일로 스크러빙 시나리오를 재현해 프레임 응답 지연을 측정.
- Structural: `File::open`/`Mmap::map` 호출부 근처에 storage-kind 분기가 전무한지 검토.

**예외**:
- 애플리케이션이 로컬 파일만 지원한다고 명시적으로 제한하고(오픈 다이얼로그에서 네트워크 경로를 사전 차단), 이를 문서화한 경우.
- 네트워크 스토리지가 사실상 로컬만큼 빠른 전용 고속 SAN 환경으로 배포 대상이 한정된 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-012: read-ahead 정책 부재
**분류**: IO · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn get_frame(source: &VideoSource, frame_index: usize) -> Vec<u8> {
    // 매 프레임 요청마다 그 프레임만 딱 읽고 끝 — 다음 프레임을 전혀 예측하지 않는다.
    let (offset, len) = source.index.frame_location(frame_index);
    source.read_at(offset, len)
}
```

**문제**:
- 순차 재생/스크러빙은 예측 가능한 접근 패턴(현재 프레임 다음은 거의 항상 다음 프레임)임에도 이를 활용하지 않아, 매 프레임마다 콜드 캐시 상태에서 I/O 지연을 그대로 감수한다.
- 커널의 기본 read-ahead 휴리스틱(순차 `read` 감지 시 자동 프리페치)은 mmap 기반 random-access 패턴에서는 제대로 작동하지 않는 경우가 많아, 애플리케이션이 명시적으로 힌트를 주지 않으면 이점을 얻지 못한다.
- 재생 프레임레이트를 유지하려면 프레임 예산(예: 33ms) 안에 디코딩+렌더링까지 끝나야 하는데, 콜드 I/O 지연이 그 예산을 잠식하면 프레임 드랍이 발생한다.

**발생 조건**:
- 순차 재생, 프레임 단위 스텝, 필름스트립 썸네일 스크롤처럼 다음 접근 위치를 상당히 정확하게 예측할 수 있는 UI 패턴.
- 네트워크/느린 스토리지 위 파일에서 특히 체감 효과가 크다(IO-011과 연동).

**권장**:
```rust
use std::sync::mpsc;

struct Prefetcher {
    request_tx: mpsc::Sender<usize>, // 다음에 필요할 것으로 예측되는 frame_index
}

impl Prefetcher {
    fn on_frame_shown(&self, current_index: usize, playback_direction: i32) {
        // 재생 방향을 기준으로 다음 N개 프레임을 백그라운드 스레드에 프리페치 요청
        for ahead in 1..=PREFETCH_DEPTH {
            let predicted = (current_index as i64 + ahead as i64 * playback_direction as i64).max(0) as usize;
            let _ = self.request_tx.send(predicted);
        }
    }
}

// 백그라운드 워커: mmap 페이지를 미리 터치하거나 madvise(WILLNEED) 힌트를 준다.
fn prefetch_worker(mmap: &memmap2::Mmap, rx: mpsc::Receiver<usize>, index: &FrameIndex) {
    for frame_index in rx {
        let (offset, len) = index.frame_location(frame_index);
        // 페이지를 터치해 major fault를 미리 흡수 (실제 사용 시점엔 캐시 히트)
        let _ = mmap[offset..offset + len].iter().step_by(4096).count();
    }
}
```
- 재생 방향과 현재 위치를 기반으로 다음 N프레임을 백그라운드 스레드에서 미리 터치(mmap) 또는 미리 읽어(버퍼드 I/O) 캐시를 데운다.
- 가능하면 `madvise(MADV_WILLNEED)` / `posix_fadvise(POSIX_FADV_WILLNEED)` 같은 커널 힌트를 활용해 애플리케이션이 직접 페이지를 순회하지 않고도 커널이 비동기로 프리페치하게 한다(IO-016 참고).
- 프리페치 깊이는 재생 프레임레이트와 스토리지 지연을 고려해 튜닝하고, 사용자가 방향을 바꾸면(되감기) 즉시 예측 방향도 갱신한다.

**탐지 방법**:
- Runtime: 순차 재생 시나리오에서 프레임별 I/O 대기 시간을 측정해 매 프레임이 유사한 콜드 지연을 보이는지 확인(프리페치가 있다면 첫 프레임 이후로는 지연이 사라져야 함).
- Structural: 프레임 접근 함수 근처에 "다음 프레임 예측" 관련 로직이나 백그라운드 워커가 전혀 없는지 확인.
- Manual: 재생/스크러빙 UX 코드에서 prefetch queue 개념이 설계에 존재하는지 검토.

**예외**:
- 사용자가 완전히 무작위로 프레임을 탐색하는 워크플로(예: 특정 조건을 만족하는 프레임을 검색하는 분석 도구)에서는 예측이 무의미해 prefetch 이득이 적다.
- 파일이 이미 전량 로컬 캐시에 적재되어(작은 파일, 반복 재생) 콜드 미스가 사실상 없는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-013: index 없이 처음부터 반복 스캔
**분류**: IO · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn find_frame_offset(mmap: &[u8], target_frame: usize) -> usize {
    let mut offset = 0;
    let mut current_frame = 0;
    // 목표 프레임을 찾을 때마다 항상 파일 처음부터 NAL을 순회한다.
    while current_frame < target_frame {
        let nal_size = parse_nal_size(&mmap[offset..]);
        offset += nal_size;
        current_frame += 1;
    }
    offset
}
```

**문제**:
- 프레임 N에 접근하는 비용이 O(N)이라, 사용자가 파일 뒷부분(예: 90% 지점)으로 이동하면 사실상 파일 전체를 매번 재파싱하는 것과 같다.
- 스크러빙처럼 짧은 시간에 여러 위치로 반복 점프하는 UI 패턴에서는 이 O(N) 스캔이 반복되어 체감 성능이 파일 길이에 비례해 선형으로 나빠진다.
- 컨테이너 포맷(MP4의 stss/stco, MKV의 Cues 등)이 이미 인덱스를 제공하는 경우가 많은데 이를 활용하지 않고 직접 NAL 단위로 재스캔하는 것은 이중 낭비다.

**발생 조건**:
- 컨테이너가 인덱스를 포함하지 않거나(raw Annex B 스트림, 일부 손상된 파일) 파서가 아직 인덱스 빌드 기능을 구현하지 않은 초기 개발 단계.
- "일단 정확성부터"라는 이유로 매번 처음부터 파싱하는 단순 구현을 최적화 없이 그대로 프로덕션에 남긴 경우.

**권장**:
```rust
struct FrameIndex {
    // frame_index -> (byte_offset, size) 매핑, 파일을 1회 스캔해 구축 후 캐시
    entries: Vec<(u64, u32)>,
}

impl FrameIndex {
    fn build(mmap: &[u8]) -> Self {
        let mut entries = Vec::new();
        let mut offset = 0usize;
        while offset < mmap.len() {
            let nal_size = parse_nal_size(&mmap[offset..]);
            entries.push((offset as u64, nal_size as u32));
            offset += nal_size;
        }
        Self { entries }
    }

    fn offset_of(&self, frame_index: usize) -> Option<(u64, u32)> {
        self.entries.get(frame_index).copied() // O(1)
    }
}
```
- 파일을 처음 열 때 한 번(또는 백그라운드에서 점진적으로) 인덱스를 구축해 `frame_index -> (offset, size)` 조회를 O(1)/O(log n)으로 만든다.
- 컨테이너가 자체 인덱스(MP4 `stss`/`stco`/`stsz`, MKV `Cues`)를 제공하면 이를 우선 활용하고, 없거나 손상된 경우에만 폴백으로 전체 스캔 후 인덱스를 캐시한다.
- 매우 큰 파일에서는 인덱스 구축 자체도 점진적(lazy, 백그라운드)으로 진행해 파일 열기 응답성을 해치지 않는다.

**탐지 방법**:
- Structural: 프레임/패킷 조회 함수가 루프 시작점을 항상 offset 0 또는 파일 시작으로 고정하는지 확인.
- Runtime: 프레임 인덱스를 다르게(초반/중반/후반) 준 조회 벤치마크를 돌려 지연이 인덱스에 비례해 증가하는지 측정.
- Manual: 코드에 `FrameIndex`, `SeekTable` 등 캐시 개념이 아예 존재하지 않는지 아키텍처 문서/구조체 목록에서 확인.

**예외**:
- 파일이 매우 작아(수백 프레임 이하) 선형 스캔 비용이 체감되지 않는 경우.
- 스트리밍 파싱이 목적이라 애초에 임의 접근이 요구사항에 없는 워크플로(순수 순차 처리 배치 도구).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-014: export 중 원본 파일 lock 장기 유지
**분류**: IO · **심각도**: High · **탐지**: Manual

**나쁜 예**:
```rust
fn export_clip(source: &Mutex<VideoSource>, start: usize, end: usize, out_path: &Path) -> std::io::Result<()> {
    // export 시작부터 끝날 때까지 원본 소스를 통째로 잠근다.
    let source = source.lock().unwrap();
    let mut out = std::fs::File::create(out_path)?;
    for frame_index in start..end {
        let data = source.read_frame(frame_index);
        out.write_all(&data)?; // 수천 프레임 export 동안 락을 계속 들고 있음
    }
    Ok(())
}
```

**문제**:
- 대형 구간(수천 프레임)을 export하는 동안 원본 소스 락을 계속 들고 있으면, 그 사이 사용자는 같은 파일을 재생/스크러빙/hex view로 열어볼 수 없다 — 애플리케이션이 export 도중 사실상 멈춘 것처럼 느껴진다.
- export가 디스크 쓰기 대역폭에 의해 병목되는데, 그 병목 시간 동안 읽기 전용 작업조차 막히는 것은 필요 이상의 제약이다(export는 쓰기 자원만 독점하면 되지, 읽기 소스까지 독점할 필요가 없다).
- export 중 오류(디스크 공간 부족, 사용자 취소)가 발생했을 때 락 해제가 지연되거나 누락되면 애플리케이션 전체가 응답 불능 상태로 보일 위험이 있다.

**발생 조건**:
- export 기능이 진행 상황 표시나 취소 기능 없이 동기적으로 전체 구간을 처리하도록 구현된 경우.
- 원본 소스에 대한 락 범위가 "export 함수 전체"로 설계되어 세밀한 락 스코프 분리가 되어 있지 않은 경우.

**권장**:
```rust
fn export_clip(source: &Arc<Mmap>, start: usize, end: usize, out_path: &Path, index: &FrameIndex) -> std::io::Result<()> {
    // mmap은 읽기 전용 공유 상태이므로 애초에 배타적 락이 필요 없다.
    let mut out = std::io::BufWriter::new(std::fs::File::create(out_path)?);
    for frame_index in start..end {
        let (offset, len) = index.offset_of(frame_index).expect("valid frame");
        out.write_all(&source[offset as usize..offset as usize + len as usize])?;
        // 취소 토큰 체크, progress callback 호출 등을 여기서 수행
    }
    Ok(())
}
```
- 읽기 전용 접근이라면 애초에 mmap 공유 참조(`Arc<Mmap>`)로 설계해 락 자체를 없앤다 — export와 재생이 서로 다른 오프셋을 동시에 읽어도 안전하다.
- 락이 꼭 필요한 자원(예: 쓰기 대상 출력 파일, 진행률 상태)만 최소 스코프로 잠그고, 원본 읽기 경로와는 분리한다.
- 대형 export는 취소 가능하도록 설계하고, 진행 중에도 다른 읽기 작업이 병행될 수 있음을 아키텍처 차원에서 보장한다.

**탐지 방법**:
- Manual: export 관련 함수의 락 획득 시점과 해제 시점을 추적해 락 보유 구간이 I/O 바운드 루프 전체를 감싸는지 확인.
- Runtime: export 실행 중 재생/스크러빙 조작이 응답하는지 수동/자동 UI 테스트로 검증.
- Structural: `lock()` 호출과 루프 블록 사이의 코드 라인 수/추정 실행 시간을 정적으로 근사해 "긴 락 보유" 후보를 표시.

**예외**:
- export 대상 구간이 매우 짧아(수 프레임) 락 보유 시간이 무시할 수 있는 수준인 경우.
- 애플리케이션이 애초에 "export 중에는 다른 작업 불가"를 의도된 UX로 명시하고 진행률 모달로 이를 사용자에게 알리는 설계라면 판단이 달라질 수 있다(단, 이 경우도 취소 가능성은 보장해야 한다).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-015: 임시 파일을 메모리 버퍼로 대체해 메모리 폭증
**분류**: IO · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn transcode_for_preview(source: &[u8]) -> Vec<u8> {
    let mut intermediate = Vec::new(); // 디스크 대신 메모리에 중간 산출물 전체를 보관
    for chunk in source.chunks(1024 * 1024) {
        let decoded = decode_chunk(chunk);
        intermediate.extend_from_slice(&decoded); // 원본 크기의 수 배로 불어날 수 있음
    }
    // intermediate 전체가 다음 단계로 전달될 때까지 메모리에 상주
    re_encode(&intermediate)
}
```

**문제**:
- "디스크 I/O가 느리니 메모리가 낫다"는 직관으로 임시 파일 대신 `Vec<u8>` 누적 버�퍼를 선택했지만, 디코딩된 중간 산출물(raw YUV 등)은 압축된 원본보다 수 배~수십 배 커질 수 있어 오히려 메모리 폭증을 유발한다.
- 여러 export/프리뷰 작업이 동시에 실행되면 각각의 메모리 버퍼가 누적되어 시스템 전체 메모리를 고갈시키고, 다른 정상 작업(재생, mmap 캐시)까지 OOM killer의 희생양이 될 수 있다.
- 디스크 기반 임시 파일은 커널 페이지 캐시를 통해 메모리 압박 시 자동으로 evict 가능하지만, 애플리케이션이 직접 들고 있는 `Vec<u8>`은 OS가 회수할 수 없는 고정 상주 메모리다.

**발생 조건**:
- 프리뷰 생성, 트랜스코딩, 필터 체인 적용처럼 원본보다 훨씬 큰 중간 표현(uncompressed frame buffer)을 다루는 파이프라인.
- "임시 파일 관리(생성/정리/에러 시 cleanup)가 번거롭다"는 이유로 메모리 버퍼가 더 간단해 보여서 선택한 경우.

**권장**:
```rust
use tempfile::NamedTempFile;
use std::io::{Write, BufWriter};

fn transcode_for_preview(source: &[u8]) -> std::io::Result<memmap2::Mmap> {
    let tmp = NamedTempFile::new()?; // 프로세스 종료/에러 시 자동 정리
    {
        let mut writer = BufWriter::new(tmp.reopen()?);
        for chunk in source.chunks(1024 * 1024) {
            let decoded = decode_chunk(chunk);
            writer.write_all(&decoded)?; // 커널 페이지 캐시를 경유, 메모리 압박 시 evict 가능
        }
        writer.flush()?;
    }
    // 이후 단계에서 필요하면 mmap으로 다시 zero-copy 접근
    let file = tmp.reopen()?;
    unsafe { memmap2::Mmap::map(&file) }
}
```
- 원본보다 크게 부풀어 오를 수 있는 중간 산출물은 `tempfile` 크레이트 등으로 디스크 임시 파일에 스트리밍하고, 필요 시 다시 mmap으로 zero-copy 접근한다.
- 임시 파일은 생명주기가 명확한 RAII 타입(`NamedTempFile`)으로 관리해 에러 경로에서도 정리가 보장되게 한다.
- 정말 작은 중간 결과(수십 MB 이하로 상한이 보장됨)만 메모리 버퍼로 유지하고, 상한을 넘을 가능성이 있는 경로는 처음부터 디스크 기반으로 설계한다.

**탐지 방법**:
- Structural: 디코딩/트랜스코딩 파이프라인에서 `Vec<u8>`이 청크 단위로 `extend_from_slice`되며 최종 크기가 입력 크기보다 몇 배 커질 수 있는 함수를 찾는다.
- Runtime: 프리뷰/export 기능 실행 중 RSS를 모니터링해 원본 파일 크기 대비 비정상적으로 큰 피크가 있는지 확인.
- Manual: "이 중간 버퍼의 최대 크기 상한이 코드로 보장되는가"를 각 파이프라인 단계별로 질문.

**예외**:
- 중간 산출물의 크기가 설계상 명확한 상한(예: 단일 프레임의 썸네일, 수백 KB)을 가지며 절대 커지지 않는 경우.
- 극도로 빠른 지연시간이 필요하고 임시 디스크 쓰기 자체가 병목이 되는 초저지연 경로(단, 이 경우 메모리 상한을 별도로 강제해야 한다).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-016: madvise/fadvise 접근 패턴 힌트 미사용
**분류**: IO · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn open_for_sequential_decode(path: &Path) -> std::io::Result<memmap2::Mmap> {
    let file = std::fs::File::open(path)?;
    // 순차 디코딩이 확실한데도 커널에 아무 힌트를 주지 않아
    // 기본 정책(범용 read-ahead 휴리스틱)에만 의존한다.
    unsafe { memmap2::Mmap::map(&file) }
}

fn open_for_random_scrub(path: &Path) -> std::io::Result<memmap2::Mmap> {
    let file = std::fs::File::open(path)?;
    // 랜덤 액세스가 확실한데도 동일한 매핑 정책을 사용해
    // 불필요한 read-ahead로 캐시가 낭비된다.
    unsafe { memmap2::Mmap::map(&file) }
}
```

**문제**:
- 애플리케이션은 "지금부터 순차 재생할 것"인지 "지금부터 랜덤 스크러빙할 것"인지 정확히 알고 있는데, 이 정보를 커널에 전달하지 않으면 커널은 범용 휴리스틱으로만 판단해 최적이 아닌 프리페치/캐시 결정을 내린다.
- 순차 접근인데 힌트가 없으면 read-ahead가 보수적으로 작동해 불필요한 지연이 남고, 랜덤 접근인데 커널이 순차로 오판하면 쓸모없는 프리페치가 캐시 오염(IO-018 참고)을 유발한다.
- 한 번 다 읽고 다시 볼 일 없는 대형 구간(예: 이미 export된 구간의 검증 스캔)에 대해 `MADV_DONTNEED`류 힌트를 주지 않으면 페이지 캐시가 불필요하게 오래 점유된다.

**발생 조건**:
- 재생(순차), 스크러빙(랜덤), 전체 검증 스캔(순차, 1회성)처럼 접근 패턴이 코드 상에서 명확히 구분되는데도 매핑/파일 열기 시점에 이를 커널에 알리지 않는 경우.
- 크로스플랫폼 지원 때문에 플랫폼별 API(`madvise` on Unix, `PrefetchVirtualMemory`/`SetFileValidData` on Windows) 도입을 미루다가 아예 누락된 경우.

**권장**:
```rust
#[cfg(unix)]
fn hint_sequential(mmap: &memmap2::Mmap) {
    let _ = mmap.advise(memmap2::Advice::Sequential);
}

#[cfg(unix)]
fn hint_random(mmap: &memmap2::Mmap) {
    let _ = mmap.advise(memmap2::Advice::Random);
}

#[cfg(unix)]
fn hint_wont_need(mmap: &memmap2::Mmap, offset: usize, len: usize) {
    // 이미 처리 끝난 구간은 캐시에서 밀어내라고 알려 메모리 압박을 줄인다.
    let _ = mmap.advise_range(memmap2::Advice::DontNeed, offset, len);
}

fn open_for_sequential_decode(path: &Path) -> std::io::Result<memmap2::Mmap> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    #[cfg(unix)]
    hint_sequential(&mmap);
    Ok(mmap)
}
```
- `memmap2::Mmap::advise` (Unix `madvise` 래퍼)로 `Sequential`/`Random`/`WillNeed`/`DontNeed`를 접근 패턴에 맞게 명시적으로 설정한다.
- 파일 디스크립터 기반 I/O에서는 `posix_fadvise` 동등 기능을 사용하고, Windows에서는 `PrefetchVirtualMemory` 등 플랫폼 API로 대응한다(크로스플랫폼 추상화 레이어를 두는 것을 권장).
- 접근 패턴이 상태(재생 중 vs 정지 중 vs 스크러빙 중)에 따라 바뀐다면, 상태 전환 시점마다 힌트를 재설정한다.

**탐지 방법**:
- Structural: `Mmap::map` 호출 이후 `advise` 계열 호출이 전혀 없는지 프로젝트 전체에서 확인.
- Manual: 재생/스크러빙/검증 스캔처럼 접근 패턴이 코드 로직상 명확히 구분되는 지점에서 힌트 설정 여부를 검토.
- Runtime: 힌트 적용 전/후로 동일 시나리오의 major/minor page fault 수를 비교.

**예외**:
- 파일이 매우 작아 어차피 전체가 캐시에 상주하는 경우 힌트의 실질적 효과가 없다.
- `madvise` 미지원 플랫폼/파일시스템(일부 FUSE 구현)에서는 호출이 무시되거나 에러를 반환할 수 있으므로 실패를 치명적으로 다루지 않아야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-017: 파일 핸들을 매 operation마다 재오픈
**분류**: IO · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn read_frame_bytes(path: &Path, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    // 프레임 하나 읽을 때마다 파일을 새로 연다.
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf)?;
    Ok(buf)
}
```

**문제**:
- `File::open`은 경로 탐색(디렉터리 엔트리 조회), 권한 검사, 파일 디스크립터 할당 등 여러 syscall과 커널 내부 작업을 수반하는 비교적 무거운 연산인데, 이를 프레임 단위로 반복하면 실제 데이터 전송보다 오픈/클로즈 오버헤드가 더 커질 수 있다.
- 동시에 여러 워커가 이 함수를 호출하면 파일 디스크립터 테이블에 대한 커널 락 경합이 늘어나고, 플랫폼별 파일 디스크립터 상한(`ulimit -n`)에 근접할 위험도 있다(짧은 순간 다수의 fd가 동시에 열릴 수 있음).
- 네트워크 파일시스템에서는 파일 열기 자체가 원격 핸들 협상을 수반해(NFS의 OPEN RPC 등) 로컬보다 훨씬 비싸므로, 이 패턴의 악영향이 몇 배로 증폭된다.

**발생 조건**:
- "상태를 최소화하자"는 의도로 함수를 순수 함수처럼 만들면서 파일 핸들을 인자로 받지 않고 매번 경로에서 새로 여는 설계를 택했을 때.
- 여러 짧은 수명의 async 태스크가 각자 독립적으로 파일을 여는 병렬 다운로드/스캔 로직.

**권장**:
```rust
struct FrameReader {
    file: std::fs::File, // 세션 동안 한 번만 열고 재사용
}

impl FrameReader {
    fn open(path: &Path) -> std::io::Result<Self> {
        Ok(Self { file: std::fs::File::open(path)? })
    }

    fn read_frame_bytes(&self, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
        use std::os::unix::fs::FileExt;
        let mut buf = vec![0u8; len];
        self.file.read_at(&mut buf, offset)?; // 커서 공유 문제 없이 핸들 재사용 (IO-006 참고)
        Ok(buf)
    }
}
```
- 파일 핸들(또는 mmap)을 세션/소스 객체의 필드로 한 번만 열어 재사용하고, `read_at`/`pread` 같은 positioned I/O로 여러 호출자가 안전하게 공유하게 한다.
- 병렬 워커가 각자 자신의 핸들을 원한다면 세션 시작 시 필요한 개수만큼 미리 열어 풀링하고, 요청마다 새로 여는 것은 피한다.

**탐지 방법**:
- Structural: `File::open`/`fs::open` 호출이 루프 본문이나 자주 호출되는 함수 내부에 위치하는지 grep.
- Runtime: `strace -c -e open,openat`로 초당 open 호출 횟수를 측정해 프레임 처리율과 비교.
- Manual: 파일 접근 관련 함수 시그니처가 `&Path`를 받는지 `&File`/기존 핸들을 받는지 API 설계 리뷰.

**예외**:
- 접근 빈도가 극히 낮은 일회성 작업(파일 정보 조회 다이얼로그 등)에서는 오픈 오버헤드가 무의미한 수준.
- 파일 핸들을 장기 보관하는 것이 오히려 문제(다른 프로세스의 파일 교체를 감지해야 하는 워치 로직)인 특수한 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-018: 초대형 순차 스캔이 페이지 캐시를 오염시킴
**분류**: IO · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn verify_entire_file_checksum(path: &Path) -> std::io::Result<u64> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    // 수십 GB 파일을 한 번만 순회하며 체크섬을 계산 — 이후 다시 읽을 일이 거의 없다.
    // 그런데 이 접근이 기본 버퍼드 I/O/mmap 경로를 그대로 타면서
    // 페이지 캐시 전체를 이 데이터로 채워버려, 활발히 재사용되던
    // 다른 파일(썸네일, 인접 GOP)의 캐시 페이지를 밀어낸다.
    Ok(crc64(&mmap))
}
```

**문제**:
- 재사용 가능성이 낮은 "1회성 순차 스캔"(전체 체크섬 검증, 파일 무결성 검사, 백업 등)이 유한한 페이지 캐시 공간을 독점하면서, 실제로 반복 접근되는 "핫" 데이터(현재 재생 위치 근처, 자주 열람하는 썸네일)를 캐시에서 밀어낸다.
- 이 문제는 개별 함수의 버그가 아니라 "이 데이터가 재사용될 것인가"에 대한 정보를 커널에 전혀 전달하지 않아서 발생하는 시스템 레벨 부작용이라 발견이 어렵다 — 체크섬 계산 자체는 정상 동작하지만 그 여파로 다른 기능이 느려진다.
- 스왑이 활성화된 시스템에서는 캐시 오염이 실제 스왑 압박으로 이어져 전체 시스템 반응성이 저하될 수도 있다.

**발생 조건**:
- 무결성 검증, 백업, 전체 파일 해시 계산처럼 파일을 정확히 한 번(또는 드물게) 순회하는 배치성 작업이 활발한 상호작용형 기능(재생, 스크러빙)과 동시에 실행될 때.
- 대형 파일을 대상으로 하는 백그라운드 인덱싱/프리스캔 작업.

**권장**:
```rust
#[cfg(unix)]
fn verify_entire_file_checksum(path: &Path) -> std::io::Result<u64> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    // 순차 접근임을 알리고, 다 읽은 뒤에는 즉시 캐시에서 밀어내도 된다고 힌트를 준다.
    let _ = mmap.advise(memmap2::Advice::Sequential);

    let checksum = crc64(&mmap);

    // 재사용 가능성이 낮으므로 페이지 캐시를 즉시 반환하도록 요청.
    let _ = mmap.advise(memmap2::Advice::DontNeed);
    Ok(checksum)
}
```
- 1회성 대형 순차 스캔에는 `Advice::Sequential`로 프리페치를 공격적으로 하도록 하되, 완료 후 `Advice::DontNeed`로 즉시 캐시를 반환해 다른 핫 데이터가 밀려나는 시간을 최소화한다.
- 가능하면 이런 배치 작업은 활발한 상호작용 세션과 동시에 실행되지 않도록 스케줄링하거나(유휴 시간에 실행), 별도의 낮은 우선순위 I/O 클래스(`ionice` 상당, 플랫폼 지원 시)로 격리한다.
- 청크 단위로 처리 후 각 청크가 끝날 때마다 `DontNeed`를 호출해 캐시 점유 구간을 좁게 유지하는 것도 대안이다.

**탐지 방법**:
- Runtime: 대형 배치 스캔 실행 전후로 다른 파일(썸네일 등)의 접근 지연을 비교해 캐시 축출 영향을 측정.
- Structural: 파일 전체를 순회하는 함수(체크섬, 검증, 백업)에 `advise` 힌트가 없는지 확인.
- Manual: 백그라운드 배치 작업과 상호작용형 기능이 동시에 실행 가능한 아키텍처인지, 우선순위 격리가 있는지 검토.

**예외**:
- 시스템에 페이지 캐시로 쓸 여유 메모리가 넉넉하고(파일 크기 대비 RAM이 충분), 캐시 압박이 실질적으로 발생하지 않는 배포 환경.
- 배치 작업이 항상 유휴 시간(사용자가 다른 파일을 열지 않는 시점)에만 실행되도록 이미 스케줄링되어 있는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-019: 성장하거나 스트리밍 중인 파일에 대한 mmap 재매핑 누락
**분류**: IO · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
struct IngestSource {
    mmap: memmap2::Mmap, // 파일을 연 시점의 크기로 고정 매핑
}

impl IngestSource {
    fn open(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? }; // 현재까지 쓰인 만큼만 매핑됨
        Ok(Self { mmap })
    }

    fn read_frame(&self, offset: usize, len: usize) -> &[u8] {
        // 캡처/스트리밍 도구가 이어서 파일을 계속 append하고 있어도
        // 이 매핑은 open 시점 크기에 갇혀 새로 추가된 데이터에 접근 불가.
        &self.mmap[offset..offset + len]
    }
}
```

**문제**:
- 실시간 캡처 도구나 다운로드 중인 파일처럼 "쓰는 동안 읽는" 시나리오에서, mmap은 매핑 시점의 파일 크기에 고정되어 이후 append된 데이터에는 접근할 수 없다 — 사용자가 "최근 캡처된 프레임"을 보려 해도 오래된 스냅샷만 보인다.
- 파일 크기 변경을 감지하지 못한 채 매핑 범위를 넘어선 오프셋에 접근을 시도하면 범위를 벗어난 슬라이스 인덱싱으로 패닉이 발생한다.
- 재매핑을 아예 하지 않는 대신 "일단 파일을 통째로 다시 열고 다시 매핑"하는 식으로 매 프레임마다 처리하면 이번엔 반대로 극심한 오버헤드가 발생한다(이 역시 잘못된 대응).

**발생 조건**:
- 실시간 캡처(스크린 레코딩, 라이브 인코더 출력)를 진행하면서 동시에 분석 도구로 같은 파일을 여는 "tail -f" 유사 워크플로.
- 대형 파일을 네트워크로 다운로드하면서 부분적으로 도착한 데이터를 미리보기하는 기능.

**권장**:
```rust
use std::sync::RwLock;

struct IngestSource {
    file: std::fs::File,
    mmap: RwLock<memmap2::Mmap>,
    mapped_len: std::sync::atomic::AtomicU64,
}

impl IngestSource {
    fn refresh_if_grown(&self) -> std::io::Result<()> {
        let current_len = self.file.metadata()?.len();
        if current_len > self.mapped_len.load(std::sync::atomic::Ordering::Acquire) {
            let new_mmap = unsafe { memmap2::Mmap::map(&self.file)? };
            *self.mmap.write().unwrap() = new_mmap;
            self.mapped_len.store(current_len, std::sync::atomic::Ordering::Release);
        }
        Ok(())
    }

    fn read_frame(&self, offset: usize, len: usize) -> std::io::Result<Vec<u8>> {
        self.refresh_if_grown()?;
        let mmap = self.mmap.read().unwrap();
        Ok(mmap[offset..offset + len].to_vec())
    }
}
```
- 파일이 성장할 수 있는 시나리오(ingest, tail 유사 기능)에서는 접근 전 파일 크기를 확인하고, 매핑 범위를 벗어났다면 재매핑(`Mmap::map`을 다시 호출)한다.
- 재매핑 빈도를 매 접근마다가 아니라 "필요할 때만"(요청 오프셋이 현재 매핑 크기를 초과할 때)으로 제한해 오버헤드를 최소화한다.
- 정말 활발한 append-only 스트리밍이라면 mmap 대신 `read_at` 기반 스트리밍 리더 + 별도의 "새 데이터 도착" 알림(파일시스템 워처, 폴링)을 쓰는 편이 더 적합할 수 있다.

**탐지 방법**:
- Runtime: 파일에 데이터를 append하면서 동시에 읽기를 시도하는 통합 테스트로 최신 데이터 가시성을 검증.
- Structural: `Mmap` 필드를 가진 구조체 중 파일 크기 재확인/재매핑 로직이 없는 것을 찾는다.
- Manual: ingest/캡처/다운로드 관련 기능 목록을 살펴 mmap 기반 소스를 사용하는지, 성장 가능성을 고려했는지 검토.

**예외**:
- 파일이 열린 이후 절대 크기가 변하지 않는다고 보장되는 워크플로(사후 분석 전용, 캡처가 이미 완료된 파일만 다룸)에서는 이 문제가 아예 발생하지 않는다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### IO-020: 플랫폼별 mmap 동작 차이를 무시
**분류**: IO · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
fn map_file(path: &Path) -> memmap2::Mmap {
    let file = std::fs::File::open(path).unwrap();
    // Unix에서 개발/테스트하고, Windows에서의 동작 차이는 검증한 적이 없다.
    unsafe { memmap2::Mmap::map(&file).unwrap() }
}

fn export_over_source(path: &Path) {
    // Windows에서는 파일이 매핑되어 있으면 삭제/이름변경/일부 쓰기 모드 오픈이
    // 공유 위반(sharing violation)으로 실패할 수 있다는 사실을 고려하지 않았다.
    std::fs::remove_file(path).unwrap();
}
```

**문제**:
- Unix 계열은 매핑된 파일도 삭제/이름변경이 자유롭지만(inode가 유지되는 한), Windows는 파일이 매핑되어 있으면(특히 배타적 접근을 요구하는 핸들) 삭제나 특정 쓰기 작업이 공유 위반 오류로 실패하는 경우가 있어 동일한 코드가 플랫폼마다 다르게 동작한다.
- 페이지 크기가 플랫폼/아키텍처마다 다를 수 있어(대부분 4KB지만 일부 환경은 다름), 오프셋 정렬을 가정한 코드가 이식성 문제를 일으킬 수 있다.
- 빈 파일(0바이트)에 대한 매핑 시도는 플랫폼에 따라 에러를 반환하거나 동작이 달라질 수 있는데, 이를 별도로 처리하지 않으면 특정 플랫폼에서만 크래시/패닉이 발생하는 재현하기 어려운 버그가 된다.
- 개발/CI가 주로 한 플랫폼(예: macOS/Linux)에서 이루어지고 Windows는 릴리스 직전에만 확인하는 경우, 이런 차이가 뒤늦게 발견되어 릴리스 지연을 유발한다.

**발생 조건**:
- Tauri 앱처럼 Windows/macOS/Linux 모두를 타깃으로 하는 크로스플랫폼 프로젝트에서 mmap 관련 코드를 한 플랫폼에서만 집중적으로 테스트했을 때.
- export 기능이 "원본 파일을 새 결과로 교체"하는 것과 같이 열려 있는 파일에 대한 삭제/치환을 수행할 때.

**권장**:
```rust
fn map_file(path: &Path) -> std::io::Result<Option<memmap2::Mmap>> {
    let file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    if len == 0 {
        // 빈 파일은 플랫폼별로 매핑 동작이 다르므로 명시적으로 분기 처리.
        return Ok(None);
    }
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    Ok(Some(mmap))
}

fn replace_source_with_export(original: &Path, exported: &Path, mmap_guard: &mut Option<memmap2::Mmap>) -> std::io::Result<()> {
    // Windows 공유 위반을 피하기 위해 교체 전 매핑을 명시적으로 해제(drop)한다.
    *mmap_guard = None;
    // 원자적 교체를 위해 rename 사용 (플랫폼 별 원자성 보장 방식이 다르므로 문서 확인 필요)
    std::fs::rename(exported, original)
}
```
- 빈 파일, 파일 삭제/교체, 페이지 정렬처럼 플랫폼별로 동작이 갈리는 지점을 명시적으로 처리하고 주석으로 근거를 남긴다.
- CI 매트릭스에 Windows/macOS/Linux를 모두 포함해 mmap 관련 테스트(빈 파일 열기, 매핑 중 삭제/rename, export 후 교체)를 각 플랫폼에서 실행한다.
- 매핑을 해제해야 하는 시점(파일 교체/삭제 전)에는 `Option<Mmap>`처럼 명시적으로 drop 가능한 형태로 관리해 어느 플랫폼에서든 안전하게 해제 순서를 보장한다.

**탐지 방법**:
- Manual: mmap 관련 코드에 플랫폼별 분기(`#[cfg(windows)]`, `#[cfg(unix)]`)가 전혀 없는지, 혹은 필요한 곳에 없는지 검토.
- Runtime: Windows CI 러너에서 "매핑된 파일 삭제/rename" 시나리오를 별도 테스트로 실행해 에러 처리 경로를 검증.
- Structural: 빈 파일(0바이트) 입력에 대한 매핑 경로에 별도 처리가 있는지 테스트 케이스 존재 여부로 확인.

**예외**:
- 배포 대상이 단일 플랫폼으로 확정되어 있고 이를 명시적으로 문서화한 프로젝트(예: 리눅스 서버 전용 백엔드 도구)라면 우선순위가 낮아질 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
