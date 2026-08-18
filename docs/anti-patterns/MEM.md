# Anti-Pattern Catalog — MEM: 메모리와 Allocation

이 문서는 Bitvue류(Tauri + Rust + React 기반, AV1/HEVC/AVC/VP9/VVC/AV3/MPEG-2 파서, mmap 파일 I/O, dav1d/libvmaf FFI를 포함하는) 비디오 비트스트림 분석기를 위한 안티패턴 카탈로그의 일부입니다. 전체 목차는 `docs/anti-patterns/INDEX.md`(별도 작성 예정)를 참고하십시오. 이 파일은 1단계(일반 카탈로그 작성) 산출물이며, 2단계에서 실제 Bitvue 저장소를 이 기준으로 감사합니다.

---

### MEM-001: 파일 전체 read_to_end
**분류**: MEM · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
use std::fs::File;
use std::io::Read;

fn load_bitstream(path: &str) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?; // 파일 전체를 힙에 로드
    Ok(buf)
}

fn analyze(path: &str) {
    let data = load_bitstream(path).unwrap();
    // 8K HDR 원본 파일이면 수십 GB가 즉시 상주 메모리에 올라감
    parse_annexb(&data);
}
```

**문제**:
- 4K/8K raw 또는 고비트레이트 원본 스트림은 수 GB~수십 GB에 달하며, `read_to_end`는 그 전체를 한 번에 힙에 상주시킨다.
- 파일 크기가 가용 물리 메모리를 초과하면 즉시 OOM 또는 스와핑으로 UI가 멈춘다.
- 분석기는 보통 파일 전체를 동시에 필요로 하지 않고 특정 NAL/OBU/패킷 구간만 순차적으로 필요로 하므로, 전체 로드는 필요량 대비 과도한 낭비다.
- 실패 시(`Result` unwrap) 큰 파일일수록 실패까지 걸리는 시간과 메모리 낭비가 커진다.

**발생 조건**:
- 사용자가 "파일 열기"로 대형 컨테이너(MP4/MKV/IVF)를 로드할 때.
- CI에서 소형 테스트 벡터로만 검증하고 실사용 대형 파일로는 테스트하지 않았을 때 잠복해 있다가 실사용에서 터진다.

**권장**:
```rust
use memmap2::Mmap;
use std::fs::File;

fn load_bitstream(path: &str) -> std::io::Result<Mmap> {
    let file = File::open(path)?;
    // SAFETY: 분석 도중 파일이 외부에서 변경되지 않는다고 가정
    let mmap = unsafe { Mmap::map(&file)? };
    Ok(mmap)
}

fn analyze(path: &str) -> std::io::Result<()> {
    let mmap = load_bitstream(path)?;
    parse_annexb(&mmap[..]); // OS 페이지 캐시가 필요한 부분만 로드
    Ok(())
}
```
- 파일 I/O는 mmap 기반으로 통일하고, 순차 접근이 확실한 구간은 `BufReader` + 청크 단위 스트리밍을 사용한다.
- 파일 크기를 사전에 확인해 임계값 이상이면 강제로 스트리밍 경로를 taken하도록 가드를 둔다.

**탐지 방법**:
- `grep -rn "read_to_end\|read_to_string" --include=*.rs`로 파일 I/O 경로를 전수 조사.
- 코드 리뷰 체크리스트에 "새 파일 로딩 경로는 mmap 사용 여부"를 항목화.

**예외**:
- 파일 크기가 항상 작다고 보장되는 메타데이터/사이드카 파일(JSON 설정, 인덱스 파일 등)은 `read_to_end`가 적절하다.
- 테스트 픽스처처럼 크기가 통제된 경우도 허용.

**Bitvue 판정**: Confirmed — src-tauri/src/commands/file.rs:258-262(`open_file`)가 파일 전체를 `read_to_end`로 Vec에 로드; crates/bitvue-decode/src/decoder.rs:625-631의 AnnexB 폴백 경로도 동일. mmap 기반 ByteCache(crates/bitvue-core/src/byte_cache.rs)가 별도로 존재하지만 이 경로들에는 쓰이지 않음.
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-002 참고.

---

### MEM-002: 패킷마다 Vec 생성
**분류**: MEM · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
fn extract_packets(mmap: &[u8], index: &[PacketIndex]) -> Vec<Vec<u8>> {
    index.iter()
        .map(|idx| {
            let mut packet = Vec::new(); // 패킷마다 새 힙 할당
            packet.extend_from_slice(&mmap[idx.offset..idx.offset + idx.size]);
            packet
        })
        .collect()
}
```

**문제**:
- 수만~수십만 개 패킷을 순회하는 동안 매번 `malloc`/`free`가 발생해 allocator 경합과 단편화를 유발한다.
- mmap이 이미 원본 바이트를 메모리에 갖고 있는데 이를 복사해 별도 `Vec`으로 만드는 것은 이중 보관이다.
- GOP 단위 배치 분석에서 패킷 수 × 평균 크기만큼 순간적으로 메모리가 두 배로 증가한다.

**발생 조건**:
- 컨테이너 디먹싱 후 각 NAL 유닛/OBU/패킷을 개별 버퍼로 만들어 파서에 넘기는 구조에서 흔하다.
- 프레임 단위가 아니라 서브샘플(슬라이스, 타일) 단위까지 세분화된 반복 처리에서 특히 심하다.

**권장**:
```rust
struct PacketView<'a> {
    data: &'a [u8],
}

fn extract_packets<'a>(mmap: &'a [u8], index: &[PacketIndex]) -> Vec<PacketView<'a>> {
    index.iter()
        .map(|idx| PacketView { data: &mmap[idx.offset..idx.offset + idx.size] })
        .collect()
}
```
- 소유권 이전이 정말 필요한 경우(비동기 큐로 전달 등)에만 복사하고, 그 외에는 mmap을 가리키는 슬라이스/`Bytes`(참조 카운트 기반)를 사용한다.
- 배치 처리가 필요하면 파서에 반복자(iterator)를 넘겨 lazy하게 순회하도록 한다.

**탐지 방법**:
- 루프 본문에서 `Vec::new()` 또는 `.to_vec()`이 인덱스/오프셋 순회와 함께 나타나는 패턴을 grep.
- 프로파일러(heaptrack, dhat)로 짧은 시간 내 수만 건의 동일 크기대 할당이 몰리는지 확인(Runtime 탐지).

**예외**:
- 패킷을 다른 스레드/프로세스 경계로 넘겨야 해서 라이프타임을 끊어야 하는 경우 복사가 불가피하다. 이때는 `Bytes`(참조 카운트) 사용을 우선 고려.

**Bitvue 판정**: Confirmed — crates/bitvue-hevc/src/nal.rs:438-447 `parse_nal_units`가 NAL마다 `raw_payload = nal_data[2..].to_vec()`와 `payload = remove_emulation_prevention_bytes(...)`로 이중 Vec 복사(동일 패턴이 bitvue-avc/bitvue-vvc의 nal.rs에도 반복).

---

### MEM-003: 필드마다 String 생성
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
struct NalUnitInfo {
    nal_type_name: String,   // "IDR_W_RADL" 등 고정된 몇 가지 값
    profile_name: String,    // "Main", "High", ...
    chroma_format: String,   // "4:2:0", "4:2:2", "4:4:4"
}

fn describe(nal_type: u8) -> NalUnitInfo {
    NalUnitInfo {
        nal_type_name: format!("{:?}", nal_type_to_enum(nal_type)),
        profile_name: "High".to_string(),
        chroma_format: "4:2:0".to_string(),
    }
}
```

**문제**:
- 값의 종류가 유한(enum으로 표현 가능)한데도 매번 힙 할당되는 `String`을 만들어 트리 노드/패킷 정보 하나당 3~4개의 추가 할당이 발생한다.
- 수십만 개의 syntax element/NAL 노드에 이 구조체가 반복되면 String 오버헤드(포인터+길이+capacity 24바이트 + 힙 버퍼)가 누적되어 전체 트리 메모리가 실제 정보량 대비 몇 배로 부풀어 오른다.
- 문자열 비교/직렬화가 매번 문자열 파싱을 거치게 되어 CPU 비용도 함께 증가한다.

**발생 조건**:
- 파서가 UI 표시용 "사람이 읽는 이름"을 파싱 단계에서 미리 만들어 트리에 박아 넣을 때.
- syntax tree를 JSON으로 직렬화하기 편하게 하려고 모든 필드를 문자열화했을 때.

**권장**:
```rust
#[derive(Clone, Copy, Debug)]
enum ChromaFormat { Yuv420, Yuv422, Yuv444 }

impl ChromaFormat {
    fn as_str(self) -> &'static str {
        match self {
            ChromaFormat::Yuv420 => "4:2:0",
            ChromaFormat::Yuv422 => "4:2:2",
            ChromaFormat::Yuv444 => "4:4:4",
        }
    }
}

struct NalUnitInfo {
    nal_type: NalUnitType,     // enum, Copy
    profile: Profile,          // enum, Copy
    chroma_format: ChromaFormat,
}
```
- 표시용 문자열은 UI 레이어에서 필요한 순간에만 `&'static str`로 변환한다(트리에는 저장하지 않음).
- 정말 가변 텍스트(파일 경로, 사용자 주석 등)만 `String`으로 남긴다.

**탐지 방법**:
- 구조체 정의에서 `String` 필드가 실제로는 닫힌 집합(finite set) 값만 갖는지 리뷰 체크리스트로 확인.
- `#[derive(Debug)]` 출력이나 `format!("{:?}", enum)`으로 생성한 문자열을 그대로 필드에 저장하는 패턴을 grep(`format!\(` 뒤에 `.to_string()`/`to_owned()` 없이 구조체 필드 대입).

**예외**:
- 파일명, 사용자 정의 메타데이터 등 실제로 임의 텍스트인 필드는 `String`이 맞다.
- 값의 집합이 아직 코덱 표준에서 확정되지 않아 문자열로 남겨야 하는 실험적 필드(벤더 확장 등)는 예외로 허용 가능.

**Bitvue 판정**: Confirmed — crates/bitvue-core/src/types.rs:521-526 `SyntaxNode.field_name: String`, `value: Option<String>`이 파싱되는 모든 syntax element(수십만 개까지 가능)마다 힙 String을 생성(개별 `profile_name() -> &'static str`류 헬퍼는 이미 좋은 패턴이지만 트리 자체는 전면 문자열화).

---

### MEM-004: syntax node마다 children Vec 생성
**분류**: MEM · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
struct SyntaxNode {
    name: &'static str,
    value: i64,
    children: Vec<SyntaxNode>, // 리프 노드도 항상 Vec을 가짐
}

fn make_leaf(name: &'static str, value: i64) -> SyntaxNode {
    SyntaxNode { name, value, children: Vec::new() } // 빈 Vec도 24바이트 + 잠재적 힙 포인터
}
```

**문제**:
- HEVC/AV1 syntax tree는 리프 노드(대부분의 flag, ue(v) 값 등)가 압도적으로 많은데, 모든 노드가 `Vec`을 갖는 구조라면 리프 노드조차 `Vec`의 (ptr, len, cap) 24바이트를 지고 다닌다.
- `Vec::new()`는 힙 할당을 하지 않지만, 이후 `push` 한 번이라도 발생하면 재할당이 생기고, 트리 전체로 보면 "빈 Vec 오버헤드 × 리프 노드 수"가 상당한 상수 배율로 누적된다.
- 트리 순회/직렬화 시 매 노드마다 children 유무를 확인하는 분기 비용도 추가된다.

**발생 조건**:
- 슬라이스 헤더, SPS/PPS의 세부 필드처럼 자식이 없는 노드가 트리 전체 노드 수의 80% 이상을 차지하는 전형적인 비트스트림 트리에서 두드러진다.
- 트리 뷰(UI)에서 "모든 노드가 동일한 타입"이어야 코드가 단순해진다는 이유로 이런 설계를 택하기 쉽다.

**권장**:
```rust
enum SyntaxNode {
    Leaf { name: &'static str, value: i64 },
    Branch { name: &'static str, children: Vec<SyntaxNode> },
}
```
- 리프와 브랜치를 enum으로 분리해 리프 노드에는 아예 `Vec` 필드가 존재하지 않도록 한다.
- 트리가 매우 크면(수백만 노드) 별도로 `MEM-005`(Box 기반 재귀 트리) 대안인 arena/인덱스 기반 트리도 함께 검토한다.

**탐지 방법**:
- `struct` 정의에서 재귀적 트리 노드 타입에 `Vec<Self>` 필드가 무조건 존재하는지 확인.
- `std::mem::size_of::<SyntaxNode>()`를 단위 테스트로 출력해 리프/브랜치 통합 크기가 예상보다 큰지 회귀 감시.

**예외**:
- 트리 크기가 원래 작은 컨테이너 레벨 구조(box tree 등, 수백~수천 노드)라면 통합 타입으로 단순화하는 것이 유지보수상 더 나을 수 있다.

**Bitvue 판정**: Confirmed — crates/bitvue-core/src/types.rs:514-536 `SyntaxNode`는 리프/브랜치 구분 없이 모든 노드가 `children: Vec<SyntaxNodeId>` 필드를 가짐(리프가 대다수인 HEVC/AV1 syntax tree에 그대로 적용).

---

### MEM-005: Box 기반 재귀 트리
**분류**: MEM · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
enum QuadTreeNode {
    Leaf(CodingUnit),
    Split {
        tl: Box<QuadTreeNode>,
        tr: Box<QuadTreeNode>,
        bl: Box<QuadTreeNode>,
        br: Box<QuadTreeNode>,
    },
}

fn build_cu_tree(depth: u32) -> QuadTreeNode {
    if depth == 0 {
        QuadTreeNode::Leaf(CodingUnit::default())
    } else {
        QuadTreeNode::Split {
            tl: Box::new(build_cu_tree(depth - 1)),
            tr: Box::new(build_cu_tree(depth - 1)),
            bl: Box::new(build_cu_tree(depth - 1)),
            br: Box::new(build_cu_tree(depth - 1)),
        }
    }
}
```

**문제**:
- HEVC/VVC의 CTU 쿼드트리/멀티타입 트리는 프레임당 수천~수만 개 노드로 분해되며, 각 `Box::new` 호출이 개별 힙 할당이 되어 노드 수만큼 malloc 호출이 발생한다.
- 노드들이 힙 여기저기 흩어져 캐시 지역성이 나쁘고, 트리 순회(디코드 시각화, MV 오버레이 계산 등)마다 포인터 체이싱 비용이 든다.
- 트리 삭제 시 재귀적 `Drop`이 호출되어 깊은 트리에서는 스택 오버플로 위험도 존재한다(특히 8x8까지 내려가는 깊은 분할).
- 노드 하나하나가 별도 할당이라 해제 시점도 제각각이라 메모리 단편화가 누적된다.

**발생 조건**:
- 프레임마다 CU/PU/TU 트리를 새로 만들고 버리는 구조에서, 프레임 처리량이 초당 수십 프레임에 달하면 할당기 부하가 병목이 된다.
- 시각화를 위해 트리 전체를 유지해야 하는 다중 프레임 캐시(필름스트립, 오버레이 프리페치)에서 노드 수가 배가된다.

**권장**:
```rust
struct CuTreeArena {
    nodes: Vec<CuNode>, // 모든 노드를 하나의 연속 버퍼에 저장
}

struct CuNode {
    payload: CodingUnit,
    children: Option<[u32; 4]>, // arena 내 인덱스, Box 대신 u32 인덱스
}
```
- arena(슬롯 배열) + 인덱스 기반 트리로 전환하면 할당 횟수가 프레임당 1회(또는 재사용 시 0회)로 줄고 캐시 지역성도 개선된다.
- 트리 전체를 한 번에 `nodes.clear()`로 해제할 수 있어 재귀 `Drop` 비용도 사라진다.

**탐지 방법**:
- 재귀 enum/struct 정의에서 `Box<Self>` 또는 `Box<[Self; N]>` 패턴을 grep.
- 프레임당 트리 생성 함수에 대해 heaptrack/dhat으로 프레임당 할당 횟수를 측정(Runtime).

**예외**:
- 트리 크기가 작고(파일 박스 트리, SPS/PPS 파라미터 셋 목록 등 수십~수백 노드) 생성 빈도가 낮다면 `Box` 기반이 코드 가독성 면에서 더 낫다.

**Bitvue 판정**: N/A — SyntaxTree는 `Box<Self>` 재귀가 아니라 `HashMap<SyntaxNodeId, SyntaxNode>`(id 기반 arena형) 구조(crates/bitvue-core/src/types.rs:572)를 사용. Box 기반 재귀 트리 정의는 발견되지 않음(다만 String id 기반이라 MEM-018 문제로 대체됨).

---

### MEM-006: Vec<Vec<T>> 영상 평면
**분류**: MEM · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
struct YuvPlane {
    rows: Vec<Vec<u8>>, // 행마다 별도 힙 버퍼
}

fn alloc_plane(width: usize, height: usize) -> YuvPlane {
    YuvPlane {
        rows: (0..height).map(|_| vec![0u8; width]).collect(), // height번의 개별 할당
    }
}
```

**문제**:
- `Vec<Vec<u8>>`는 행(row) 수만큼 독립된 힙 버퍼를 만들어 4K 프레임(2160행) 기준 매 평면당 2000회 이상의 할당이 발생한다.
- 각 행 버퍼가 메모리상 인접하다는 보장이 없어 캐시 미스가 증가하고, dav1d/libvmaf 같은 FFI에 넘길 때 연속된 단일 버퍼를 요구하는 C API와 맞지 않아 결국 복사(flatten)가 한 번 더 필요하다.
- 프레임마다 이 구조를 새로 만들면 할당/해제가 프레임 수 × 행 수만큼 누적되어 픽셀 처리 자체보다 allocator 오버헤드가 커질 수 있다.

**발생 조건**:
- YUV 평면, RGBA 프레임버퍼, VMAF 스코어맵처럼 2차원 데이터를 "그냥 자연스럽게" `Vec<Vec<T>>`로 모델링했을 때.
- SIMD 최적화나 stride 기반 접근이 필요한 디코드 후처리 경로에서 특히 성능 저하가 두드러진다.

**권장**:
```rust
struct YuvPlane {
    data: Vec<u8>,   // 단일 연속 버퍼
    width: usize,
    height: usize,
    stride: usize,   // 정렬을 위해 width보다 클 수 있음
}

impl YuvPlane {
    fn row(&self, y: usize) -> &[u8] {
        let start = y * self.stride;
        &self.data[start..start + self.width]
    }
}
```
- 단일 `Vec<u8>` + `stride`로 평면을 표현하면 할당이 프레임당 1회로 줄고, FFI(dav1d, libvmaf)에도 포인터 하나로 그대로 전달 가능하다.
- SIMD 정렬이 필요하면 `aligned-vec` 등 정렬 보장 버퍼를 사용한다.

**탐지 방법**:
- `Vec<Vec<` 패턴을 grep하고, 영상/평면/버퍼 관련 타입에 해당하는지 확인.
- 벤치마크에서 `alloc_plane`류 함수의 호출당 malloc 횟수를 dhat으로 측정.

**예외**:
- 각 행의 길이가 실제로 가변적인 데이터(예: 가변 길이 엔트로피 코딩 중간 산출물을 행 단위로 저장하는 디버그 구조)라면 `Vec<Vec<T>>`가 정당할 수 있다.

**Bitvue 판정**: N/A — 실제 픽셀 평면은 `Arc<[u8]>`/`Vec<u8>` + width/height/stride의 연속 버퍼로 설계됨(crates/bitvue-decode/src/decoder.rs:33-58 `DecodedFrame`, crates/bitvue-decode/src/plane_utils.rs). `Vec<Vec<u8>>`는 컨테이너 샘플 목록(가변 길이 패킷 모음, 예: bitvue-formats/src/mkv.rs:181)에만 쓰이며 이는 카탈로그의 정당한 예외 사례에 해당.

---

### MEM-007: HashMap 남용
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
use std::collections::HashMap;

struct FrameMetadata {
    fields: HashMap<String, i64>, // "poc", "qp", "frame_type" 등 고정 키
}

fn parse_slice_header() -> FrameMetadata {
    let mut fields = HashMap::new();
    fields.insert("poc".to_string(), 42);
    fields.insert("qp".to_string(), 28);
    fields.insert("frame_type".to_string(), 1);
    FrameMetadata { fields }
}
```

**문제**:
- `HashMap<String, _>`은 버킷 배열 할당 + 각 키 String 힙 할당이 겹쳐 고정된 소수의 필드를 담는 데는 과도한 오버헤드다.
- 해시 계산(기본 SipHash) 비용이 단순 구조체 필드 접근보다 훨씬 비싸며, 프레임마다 생성/조회가 반복되면 무시할 수 없는 CPU 비용이 된다.
- 키 오타(`"pos"` vs `"poc"`)가 컴파일 타임에 잡히지 않아 런타임 버그로 이어지기 쉽다.
- 필드 순서/존재 여부가 타입 시스템에 드러나지 않아 IDE 자동완성, 리팩터링 안전성이 사라진다.

**발생 조건**:
- 파서 초기 프로토타입에서 "필드가 자꾸 늘어나니 유연하게" 하려고 map으로 시작했다가 정식 구조체로 전환되지 않은 채 굳어졌을 때.
- 코덱마다 필드 집합이 달라 공통 구조체를 만들기 귀찮아서 map으로 우회했을 때.

**권장**:
```rust
struct FrameMetadata {
    poc: i64,
    qp: i32,
    frame_type: FrameType,
}
```
- 필드 집합이 정적으로 알려져 있으면 구조체로 정의한다.
- 정말 동적인 키-값(벤더별 확장 메타데이터, 사용자 정의 태그)만 `HashMap<Cow<'static, str>, Value>` 형태로 남긴다.
- 성능이 중요한 경로에서 map이 꼭 필요하다면 `FxHashMap`/`AHashMap` 같은 non-cryptographic 해셔로 교체한다.

**탐지 방법**:
- `HashMap<String,` 패턴을 grep하고, struct 필드로 대체 가능한지 리뷰.
- clippy의 `disallowed_types` lint로 특정 모듈에서 `HashMap` 사용을 금지/경고하도록 설정 가능.

**예외**:
- 사이드카 메타데이터, 사용자 주석, 플러그인 확장 필드처럼 스키마가 컴파일 타임에 고정될 수 없는 경우는 map이 적절하다.

**Bitvue 판정**: Suspected — 대부분의 `HashMap<String,String>`은 metadata/context/evidence용으로 카탈로그가 인정하는 예외(사이드카 메타데이터)에 해당하지만, SyntaxTree의 `HashMap<SyntaxNodeId(=String), SyntaxNode>`(types.rs:572)는 정적으로 알려진 대량 구조를 문자열 키 map으로 다루는 더 심한 사례(MEM-018과 중복 근거).

---

### MEM-008: 작은 객체의 개별 heap allocation
**분류**: MEM · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct MotionVector {
    x: i16,
    y: i16,
}

fn collect_mvs(block_count: usize) -> Vec<Box<MotionVector>> {
    (0..block_count)
        .map(|_| Box::new(MotionVector { x: 0, y: 0 })) // 4바이트 값을 위해 개별 힙 할당
        .collect()
}
```

**문제**:
- `MotionVector`는 4바이트에 불과한데 `Box`로 감싸면 매 블록마다 별도 힙 할당(대개 최소 16바이트 이상의 allocator 오버헤드 포함)이 발생한다.
- 4K 프레임에서 8x8 블록 단위 MV만 해도 수만 개이므로, 프레임당 수만 회의 malloc/free가 MV 저장만으로 발생한다.
- `Vec<Box<T>>`는 포인터 배열이라 순회 시 각 원소가 힙 여기저기 흩어져 있어 캐시 미스가 `Vec<T>` 대비 크게 증가한다.

**발생 조건**:
- "다형성을 대비해서" 또는 습관적으로 작은 값 타입까지 `Box`로 감쌌을 때.
- 다른 언어(Java/C#)의 참조 타입 사고방식을 그대로 Rust에 옮겼을 때 자주 나타난다.

**권장**:
```rust
struct MotionVector { x: i16, y: i16 } // Copy 가능한 값 타입

fn collect_mvs(block_count: usize) -> Vec<MotionVector> {
    vec![MotionVector { x: 0, y: 0 }; block_count] // 단일 연속 버퍼
}
```
- 값이 작고 `Copy` 가능하면 `Box` 없이 그대로 `Vec<T>`에 담는다.
- 다형성이 정말 필요한 경우에만(트레이트 객체 등) `Box<dyn Trait>`을 사용한다.

**탐지 방법**:
- `Vec<Box<` 패턴을 grep하고, 감싸인 타입의 `size_of`가 작은지(포인터 크기 이하) 확인.
- clippy `clippy::box_collection`, `clippy::boxed_local` 등의 lint를 활성화.

**예외**:
- 크기가 크고 가변 길이인 variant를 다루는 enum(예: MEM-019)에서 특정 variant만 `Box`로 감싸 enum 전체 크기를 줄이는 것은 반대로 권장되는 패턴이다.

**Bitvue 판정**: N/A — 발견된 `Vec<Box<dyn Trait>>` 사례(event_observer.rs:771, validation_strategy.rs:331, command_chain.rs:600, overlay_factory.rs:340)는 모두 트레이트 객체 다형성 목적으로, 카탈로그가 명시한 예외에 해당. 작은 값 타입을 개별 Box로 감싸는 패턴은 발견되지 않음.

---

### MEM-009: capacity 예약 없이 반복 push
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn collect_nal_units(data: &[u8]) -> Vec<NalUnit> {
    let mut units = Vec::new(); // capacity 0으로 시작
    let mut cursor = 0;
    while cursor < data.len() {
        let nal = parse_next_nal(data, &mut cursor);
        units.push(nal); // growth마다 재할당 + 기존 데이터 복사
    }
    units
}
```

**문제**:
- `Vec::new()`는 첫 `push`부터 시작해 용량이 부족할 때마다 2배씩 재할당하며, 재할당 때마다 기존 원소를 새 버퍼로 복사한다.
- NAL 개수가 수만 개에 달하는 긴 스트림에서는 로그(log2(N)) 번의 재할당이 발생하고, 마지막 재할당은 이미 절반 정도 찬 대형 버퍼를 통째로 복사하므로 순간 피크 메모리가 최종 크기의 최대 1.5~2배까지 치솟는다.
- 파일 크기나 평균 NAL 크기로 대략적인 개수를 미리 추정할 수 있는데도 이 정보를 버리는 것.

**발생 조건**:
- 컨테이너 인덱스(샘플 테이블, cues 등)를 이미 파싱해서 패킷 개수를 알고 있는데도 결과 `Vec`을 `with_capacity` 없이 만들 때 특히 아깝다.
- 실시간성이 중요한 프리뷰/시크 경로에서 재할당으로 인한 지연이 체감되는 끊김으로 나타날 수 있다.

**권장**:
```rust
fn collect_nal_units(data: &[u8], estimated_count: usize) -> Vec<NalUnit> {
    let mut units = Vec::with_capacity(estimated_count);
    let mut cursor = 0;
    while cursor < data.len() {
        units.push(parse_next_nal(data, &mut cursor));
    }
    units
}
```
- 컨테이너 인덱스, 이전 프레임 통계, 평균 NAL 크기 등으로 상한/추정치를 계산해 `with_capacity`를 사용한다.
- 정확한 개수를 모르면 "약간 과대 추정"이 "재할당 여러 번"보다 대체로 유리하다.

**탐지 방법**:
- 루프 안에서 `.push(`가 있는데 루프 바깥에 `Vec::new()`만 있고 `with_capacity`/`reserve`가 없는 패턴을 grep.
- clippy `clippy::vec_init_then_push`가 일부 케이스를 잡아준다.

**예외**:
- 결과 개수를 전혀 예측할 수 없고 데이터도 작은(수십 개 이하) 경우에는 굳이 예약할 필요가 없다.

**Bitvue 판정**: Confirmed — crates/bitvue-avc/src/frames.rs:457-462(및 bitvue-hevc/src/frames.rs 동일 패턴)에서 `nal_data = Vec::new()`에 `extend_from_slice`를 반복 호출해 프레임을 조립하면서도 총 크기를 사전 계산해 `with_capacity`를 쓰지 않음(다른 곳, 예: nal.rs의 `parse_nal_units`는 이미 `with_capacity` 적용).

---

### MEM-010: oversized capacity 장기 보존
**분류**: MEM · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
struct StreamAnalyzer {
    frame_buffer: Vec<u8>,
}

impl StreamAnalyzer {
    fn analyze_keyframe(&mut self, frame: &[u8]) {
        self.frame_buffer.clear();
        self.frame_buffer.extend_from_slice(frame); // 8K IDR: ~50MB
        // ... 분석 ...
    }

    fn analyze_pframe(&mut self, frame: &[u8]) {
        self.frame_buffer.clear();
        self.frame_buffer.extend_from_slice(frame); // 이후 P프레임: 몇 KB
        // capacity는 여전히 50MB 그대로 유지됨
    }
}
```

**문제**:
- 한 번 큰 프레임(키프레임, I-slice, 고해상도 레이어)을 처리하면서 확보된 capacity가 이후 훨씬 작은 프레임을 처리하는 내내 해제되지 않고 유지된다.
- `StreamAnalyzer` 인스턴스가 세션 내내 살아있는 장수 객체라면, 최댓값 기준 capacity가 사실상 "영구 예약 메모리"가 되어 여러 스트림/탭을 동시에 열 때 메모리 사용량이 실제 필요량보다 훨씬 크게 누적된다.
- 사용자 입장에서는 "작은 파일을 열었는데도 메모리 사용량이 안 줄어든다"는 형태로 체감된다.

**발생 조건**:
- 가변 비트레이트 스트림에서 씬 전환 직후의 큰 I-프레임을 처리한 뒤 정적 장면의 작은 P-프레임이 이어질 때.
- 여러 스트림 탭을 순차적으로 열었다 닫을 때 동일한 analyzer 인스턴스/버퍼 풀을 재사용하는 구조라면 문제가 더 커진다.

**권장**:
```rust
impl StreamAnalyzer {
    const CAPACITY_SHRINK_THRESHOLD: usize = 4; // capacity가 실사용의 4배 넘으면 축소

    fn analyze_frame(&mut self, frame: &[u8]) {
        self.frame_buffer.clear();
        self.frame_buffer.extend_from_slice(frame);
        if self.frame_buffer.capacity() > frame.len() * Self::CAPACITY_SHRINK_THRESHOLD {
            self.frame_buffer.shrink_to(frame.len() * 2); // 여유는 남기되 과도한 예약은 줄임
        }
    }
}
```
- 주기적으로(또는 임계값 기반으로) `shrink_to`/`shrink_to_fit`을 호출해 실제 사용량에 맞춰 capacity를 되돌린다.
- 애초에 프레임 크기 편차가 크다면 크기별 버퍼 풀(예: 소/중/대 세 종류)을 두는 것도 방법이다.

**탐지 방법**:
- 장수 객체의 버퍼 필드에 대해 세션 진행 중 `capacity()`를 주기적으로 로깅해 최댓값 이후 줄어드는지 관찰(Runtime).
- 메모리 프로파일러에서 "한 번 스파이크 후 고평탄(plateau)" 패턴이 보이면 이 안티패턴의 신호.

**예외**:
- 버퍼가 매우 짧게 살고 곧 drop된다면(함수 로컬 변수 등) 굳이 shrink할 필요가 없다 — 어차피 함수 종료 시 전체 해제된다.

**Bitvue 판정**: Suspected — `shrink_to`/`shrink_to_fit`이 Bitvue 자체 코드에서 전혀 쓰이지 않음(vendor/abseil의 미사용 유틸 함수 제외). ByteCache/CachedFrame처럼 가변 크기 프레임을 반복 처리하는 장수 구조체가 있어 구조적으로 발생 가능하나, capacity 누적을 직접 관측하는 런타임 근거는 확보하지 못함.

---

### MEM-011: clear 후 대형 capacity가 계속 유지됨
**분류**: MEM · **심각도**: Low · **탐지**: Structural

**나쁜 예**:
```rust
struct SyntaxTreeBuilder {
    scratch: Vec<SyntaxNode>,
}

impl SyntaxTreeBuilder {
    fn build_for_frame(&mut self, frame_idx: usize) -> Vec<SyntaxNode> {
        self.scratch.clear(); // 원소는 지워지지만 capacity는 그대로
        populate_nodes(&mut self.scratch, frame_idx);
        self.scratch.clone() // capacity가 큰 채로 매번 clone까지 발생
    }
}
```

**문제**:
- `Vec::clear()`는 길이만 0으로 만들 뿐 capacity를 해제하지 않는다는 것 자체는 종종 의도된 최적화(재사용)이지만, 이 패턴이 반환값을 `clone()`하는 코드와 결합되면 "큰 capacity를 가진 채로 매번 복제"라는 이중 낭비가 된다.
- MEM-010과 유사하지만 여기서는 "clear를 최적화라고 착각해 무분별하게 여기저기 적용"하다가 실제로는 불필요한 clone/이관 경로에 큰 capacity가 계속 실려 다니는 것이 핵심 문제다.
- 여러 `SyntaxTreeBuilder` 인스턴스(예: 여러 스트림 동시 분석)가 각자 최댓값 capacity를 들고 있으면 인스턴스 수만큼 낭비가 배가된다.

**발생 조건**:
- "reuse buffer는 무조건 좋다"는 규칙을 기계적으로 적용해 반환 시점에 소유권을 그대로 넘기지 않고 매번 clone하는 API 설계에서 나타난다.
- 최대 크기 프레임 처리 이후 장기간 유휴 상태(사용자가 재생을 멈추고 다른 작업 중)로 들어갈 때 해제되지 않은 capacity가 그대로 남는다.

**권장**:
```rust
impl SyntaxTreeBuilder {
    fn build_for_frame(&mut self, frame_idx: usize) -> Vec<SyntaxNode> {
        self.scratch.clear();
        populate_nodes(&mut self.scratch, frame_idx);
        std::mem::take(&mut self.scratch) // 소유권 이전, scratch는 빈 Vec(capacity 0)으로 리셋
    }
}
```
- `clone()` 대신 `std::mem::take`/`std::mem::replace`로 소유권을 이전하면 불필요한 복제가 사라진다.
- 재사용이 필요하면 호출자가 사용한 버퍼를 다시 `builder`에 돌려주는 "버퍼 반환" API(`return_buffer(vec)`)를 설계해 capacity를 실제로 재활용한다.

**탐지 방법**:
- `.clear()` 직후 같은 필드에 대해 `.clone()`이 호출되는 패턴을 grep.
- 코드 리뷰에서 "scratch 버퍼를 반환할 때 소유권 이전인지 복제인지" 확인.

**예외**:
- clone 비용이 무시할 만큼 작은 소규모 트리(수십 노드 이하)라면 문제 삼을 필요는 없다.

**Bitvue 판정**: Suspected — `.clear()` 이후 동일 필드를 `.clone()`해 반환하는 scratch-buffer 패턴은 grep으로 명확히 특정되지 않음. 판단에 충분한 근거를 찾지 못해 단정하지 않음.

---

### MEM-012: scratch buffer를 매 호출 새로 생성
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn compute_residual_block(coeffs: &[i32]) -> Vec<i32> {
    let mut temp = vec![0i32; 64]; // 매 블록 호출마다 새 힙 할당
    inverse_transform(coeffs, &mut temp);
    temp
}

fn decode_frame(cus: &[CodingUnit]) {
    for cu in cus {
        let residual = compute_residual_block(&cu.coeffs); // 프레임당 수만 번 호출
        apply_residual(cu, &residual);
    }
}
```

**문제**:
- 역변환 등 블록 단위 연산에서 매번 새 `temp` 버퍼를 할당하면, 4K 프레임 기준 수만 개 블록 × 프레임 수만큼 malloc/free가 반복되어 순수 연산 시간보다 allocator 오버헤드가 더 커질 수 있다.
- scratch buffer는 본질적으로 함수 호출 간 재사용 가능한 임시 공간인데, 매번 새로 만들면 그 이점을 전혀 살리지 못한다.
- 캐시 관점에서도 매번 새로 할당된 메모리는 콜드 상태라 접근 지연이 더 크다.

**발생 조건**:
- 코덱의 역변환/역양자화/인루프 필터처럼 블록 단위로 반복 호출되는 hot path 함수에서 흔하다.
- 함수형 스타일을 선호해 "입력을 받아 새 값을 반환"하는 순수 함수로 작성하다 보니 자연스럽게 매번 할당하게 된 경우.

**권장**:
```rust
struct TransformScratch {
    buf: [i32; 64], // 스택에 고정 크기로 유지(64는 최대 TU 크기 기준)
}

fn compute_residual_block(coeffs: &[i32], scratch: &mut TransformScratch) {
    inverse_transform(coeffs, &mut scratch.buf);
}

fn decode_frame(cus: &[CodingUnit]) {
    let mut scratch = TransformScratch { buf: [0; 64] };
    for cu in cus {
        compute_residual_block(&cu.coeffs, &mut scratch);
        apply_residual(cu, &scratch.buf);
    }
}
```
- 크기가 컴파일 타임에 고정되어 있으면(최대 TU 크기 등) 스택 배열로 대체해 힙 할당 자체를 없앤다.
- 크기가 가변적이면 호출자(디코드 루프)가 scratch buffer를 한 번만 만들어 `&mut` 참조로 넘긴다.

**탐지 방법**:
- 함수 시그니처가 `Vec`/`Box` 등을 반환하면서 함수 본문 첫 줄이 `let mut x = vec![...]`/`Vec::with_capacity`인 패턴을 grep하고, 해당 함수가 hot loop에서 호출되는지 호출 그래프로 확인(Structural).
- perf/flamegraph에서 `malloc`/`je_malloc` 심볼이 디코드 경로 상위에 뜨는지 확인(Runtime).

**예외**:
- 호출 빈도가 낮은 경로(파일 열기 시 1회, 설정 변경 시 등)라면 매번 할당해도 체감 비용이 없다.

**Bitvue 판정**: N/A — Bitvue는 자체 픽셀 역변환/역양자화를 구현하지 않고 디코딩을 dav1d/vvdec 등 외부 FFI에 위임하는 비트스트림 분석기이므로, 블록 단위 hot-path에서 scratch 버퍼를 매번 새로 할당하는 코드 자체가 발견되지 않음.

---

### MEM-013: frame마다 RGBA buffer 재할당
**분류**: MEM · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
fn yuv_to_rgba(plane: &YuvPlane) -> Vec<u8> {
    let mut rgba = vec![0u8; plane.width * plane.height * 4]; // 매 프레임 새 할당
    convert_yuv420_to_rgba(plane, &mut rgba);
    rgba
}

fn render_loop(frames: impl Iterator<Item = YuvPlane>) {
    for plane in frames {
        let rgba = yuv_to_rgba(&plane); // 재생 중 초당 수십 회 반복
        upload_to_gpu_texture(&rgba);
    }
}
```

**문제**:
- 재생/스크러빙 중 매 프레임마다 4K 기준 약 33MB(3840×2160×4바이트)에 달하는 RGBA 버퍼를 새로 할당하고 곧바로 버리는 패턴이 초당 수십 회 반복된다.
- 이는 UI 렌더링 경로의 hot loop이므로 allocator 부하가 프레임 드랍(끊김)으로 직접 체감된다.
- GPU 업로드 전에 CPU 버퍼가 매번 새 주소에 있으므로, 드라이버/브릿지 레이어의 캐싱 최적화 여지도 줄어든다.

**발생 조건**:
- Filmstrip, YuvViewerPanel 등 프레임을 연속으로 그려야 하는 미리보기/재생 UI 경로에서 전형적으로 나타난다.
- 프레임 크기가 재생 중 바뀌지 않는데도(같은 해상도 스트림) 버퍼를 매번 새로 만들 이유가 없는 경우가 대부분이다.

**권장**:
```rust
struct RgbaConverter {
    buf: Vec<u8>,
}

impl RgbaConverter {
    fn convert(&mut self, plane: &YuvPlane) -> &[u8] {
        let needed = plane.width * plane.height * 4;
        if self.buf.len() != needed {
            self.buf.resize(needed, 0); // 해상도 변경 시에만 재할당
        }
        convert_yuv420_to_rgba(plane, &mut self.buf);
        &self.buf
    }
}
```
- 컨버터를 재생 세션 동안 유지되는 상태 객체로 만들어 버퍼를 재사용하고, 해상도가 실제로 바뀔 때만 `resize`한다.
- GPU 업로드까지 고려한다면 더블/트리플 버퍼링(MEM-028 참고)과 결합해 CPU 쓰기와 GPU 읽기가 겹치지 않게 한다.

**탐지 방법**:
- 렌더 루프/프레임 콜백 함수 안에서 `vec![0u8; ...]` 또는 `Vec::with_capacity`가 매 호출 등장하는지 grep.
- 재생 중 dhat/heaptrack으로 "프레임 레이트와 동일한 주기로 동일 크기 할당이 반복"되는 패턴을 확인.

**예외**:
- 해상도가 자주 바뀌는(멀티 스트림 비교 뷰 등) 경우라면 재할당 자체는 불가피하지만, 그래도 "매 프레임"이 아니라 "해상도 변경 시"로 빈도를 낮추는 것이 핵심이다.

**Bitvue 판정**: Confirmed(frontend) — frontend/utils/yuv/renderer.ts:66 `yuvToImageData()`가 호출마다 `new ImageData(width,height)`를 새로 할당하고, 클래스형 `YUVRenderer.render()`(204행 `this.imageData` 재사용)조차 내부적으로 `yuvToImageData()`를 호출한 뒤 `this.imageData.data.set(...)`로 복사해 사실상 매 프레임 이중 할당. YUVCache가 일부 완화하지만 캐시 미스/신규 프레임마다 재현됨.

---

### MEM-014: decode 결과와 UI 결과 중복 보관
**분류**: MEM · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct FrameState {
    decoded_yuv: YuvPlane,      // dav1d에서 받은 원본 디코드 결과
    display_rgba: Vec<u8>,      // UI 렌더용 변환 결과
    overlay_snapshot: Vec<u8>,  // 오버레이(MV 화살표 등)까지 합성한 또 다른 복사본
    thumbnail: Vec<u8>,         // 필름스트립용 축소본
}

// 네 버전이 모두 FrameState 하나에 동시에 상주
```

**문제**:
- 같은 프레임의 픽셀 데이터가 원본 디코드 결과, RGBA 변환본, 오버레이 합성본, 썸네일까지 네 가지 형태로 동시에 메모리에 존재하면 프레임 하나당 메모리 사용량이 원본 크기의 몇 배로 불어난다.
- 이 구조가 프레임 캐시(필름스트립, undo 히스토리 등)에 여러 프레임 분량으로 곱해지면 메모리 사용량이 기하급수적으로 증가한다.
- 각 표현이 서로 다른 시점에 무효화되어야 하는데(디코드 결과가 바뀌면 나머지 세 개도 다시 계산해야 함) 이를 동기화하는 로직 버그가 stale한 오버레이/썸네일을 화면에 남기는 별도 버그로 이어지기 쉽다.

**발생 조건**:
- 오버레이 렌더링(MB 타입, 참조 인덱스, MV 화살표 등)을 별도 레이어로 합성하면서 원본을 보존하기 위해 매번 새 버퍼에 합성 결과를 만들 때.
- 필름스트립 썸네일을 프레임 디코드와 같은 시점에 미리 계산해 캐시에 함께 넣어둘 때.

**권장**:
```rust
struct FrameState {
    decoded_yuv: YuvPlane,              // 유일한 진실 원본(source of truth)
    display_cache: Option<RgbaCache>,   // lazy, 필요할 때만 생성, LRU로 축출 가능
}

struct RgbaCache {
    rgba: Vec<u8>,
    overlay_flags: OverlaySettings, // 이 설정으로 생성됐음을 기록해 무효화 판단에 사용
}
```
- 디코드 결과만 "원본(source of truth)"으로 보관하고, RGBA/오버레이/썸네일은 화면에 표시되는 순간에만 lazy하게 생성한 뒤 짧은 TTL의 캐시로 관리한다.
- 오버레이는 가능하면 별도 버퍼에 "합성"하지 않고 GPU 컴포지팅(레이어를 겹쳐 그리기)으로 대체해 CPU 측 복사본 자체를 없앤다.
- 썸네일은 전체 해상도 디코드와 별개로 저해상도 디코드 경로(MEM-025 참고)로 생성한다.

**탐지 방법**:
- 프레임 관련 상태 구조체에서 픽셀 버퍼 필드가 몇 개인지 세어보고, 그중 몇 개가 "다른 필드로부터 항상 재계산 가능한" 파생 데이터인지 리뷰.
- 메모리 프로파일러에서 프레임 하나당 총 상주 바이트를 원본 YUV 크기와 비교(원본의 2배 이상이면 의심).

**예외**:
- 오버레이 합성 결과를 디스크로 내보내야 하는(스크린샷/익스포트) 기능이라면 그 순간에 한해 합성 버퍼를 만드는 것은 정당하다 — 문제는 "상시 보관"이지 "일시적 생성"이 아니다.

**Bitvue 판정**: Confirmed — crates/bitvue-core/src/stream_state.rs:658-696 `CachedFrame`이 `rgb_data: Vec<u8>`(RGB 변환본)과 `y_plane/u_plane/v_plane`(원본 YUV, Arc)을 동시에 보관하며, 기본 32프레임 LRU(FrameModel, 585행)에 곱해져 프레임당 원본+RGB 두 형태가 상시 상주.

---

### MEM-015: 동일 payload의 Vec/Bytes/Arc 복제본 공존
**분류**: MEM · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct ParsedNal {
    raw: Vec<u8>,        // 원본 바이트
}

struct DecodedFrame {
    source_nal: Vec<u8>, // ParsedNal.raw를 clone해서 다시 보관
}

fn to_decoded(nal: &ParsedNal) -> DecodedFrame {
    DecodedFrame { source_nal: nal.raw.clone() } // 동일 payload의 두 번째 사본
}
```

**문제**:
- 같은 바이트 시퀀스가 파싱 단계 구조체와 디코드 단계 구조체에 각각 독립적인 `Vec<u8>`로 존재해, 실질적으로 하나여야 할 데이터가 두 배의 메모리를 차지한다.
- 파이프라인 단계가 늘어날수록(파서 → 디코더 → 오버레이 → 익스포트) 동일 payload가 단계마다 복제되어 총 메모리가 단계 수에 비례해 불어난다.
- 원본을 수정할 일이 없는데도(불변 데이터) 매 단계에서 소유권을 명확히 하겠다는 이유로 clone을 습관적으로 사용하는 경우가 많다.

**발생 조건**:
- 여러 모듈이 서로 다른 타입(`Vec<u8>`, `bytes::Bytes`, `Arc<[u8]>`)을 섞어 쓰면서 경계마다 변환 겸 복사가 발생할 때.
- 파이프라인 각 단계가 "이전 단계 결과를 참조가 아니라 소유"해야 한다는 설계 관행이 굳어져 있을 때.

**권장**:
```rust
use std::sync::Arc;

struct ParsedNal {
    raw: Arc<[u8]>,
}

struct DecodedFrame {
    source_nal: Arc<[u8]>, // 같은 Arc를 clone → 참조 카운트만 증가, 데이터 복사 없음
}

fn to_decoded(nal: &ParsedNal) -> DecodedFrame {
    DecodedFrame { source_nal: Arc::clone(&nal.raw) }
}
```
- 불변 payload는 `Arc<[u8]>` 또는 `bytes::Bytes`(내부적으로 참조 카운트 기반 슬라이싱 지원)로 통일해 파이프라인 전 구간에서 동일 타입을 공유한다.
- 타입을 통일할 수 없는 외부 경계(FFI 등)에서만 불가피하게 복사하고, 그 지점을 명시적으로 문서화한다.

**탐지 방법**:
- 파이프라인 인접 단계의 구조체를 나란히 놓고 동일한 바이트 데이터를 가리키는 필드가 각각 독립 `Vec<u8>`인지 확인(Structural, 코드 리뷰).
- `.clone()`이 `Vec<u8>`/대형 버�터 타입에 대해 호출되는 지점을 grep해 정말 필요한 복제인지 하나씩 검토.

**예외**:
- 디코더가 원본 바이트를 변형(비트 반전, in-place 파싱 등)해야 하는 경우처럼 실제로 가변 소유권이 필요한 지점은 복사가 정당하다.

**Bitvue 판정**: Suspected — 디코드 단계 평면은 이미 `Arc<[u8]>`/`Arc<Vec<u8>>`로 공유되어 있어(decoder.rs DecodedFrame, stream_state.rs CachedFrame) 좋은 패턴이지만, NalUnit의 `payload`/`raw_payload`(MEM-002 참고)처럼 파싱 단계 바이트가 별도 Vec으로 복제되는 지점이 있어 파이프라인 전체가 일관되게 공유 타입을 쓰는 것은 아님.

---

### MEM-016: 캐시 크기를 entry 수로만 제한
**분류**: MEM · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct FrameCache {
    entries: lru::LruCache<u64, YuvPlane>, // 최대 200개 엔트리
}

impl FrameCache {
    fn new() -> Self {
        FrameCache { entries: lru::LruCache::new(200) } // 프레임 크기 고려 없음
    }
}
```

**문제**:
- `200`이라는 엔트리 개수 제한은 프레임 하나가 SD 해상도(수백 KB)인지 8K HDR(수십 MB)인지에 따라 실제 메모리 사용량이 100배 이상 차이 날 수 있다는 사실을 반영하지 못한다.
- 사용자가 고해상도 파일을 열면 캐시가 의도치 않게 수 GB까지 자라 다른 애플리케이션과 메모리를 다투거나 OOM killer의 대상이 된다.
- 엔트리 수 기반 제한은 "캐시가 안전하다"는 잘못된 안도감을 주어 실제 메모리 압박 상황에서도 아무 조치가 취해지지 않는다.

**발생 조건**:
- 여러 해상도의 스트림을 오가며 분석하는 사용자 워크플로(같은 세션에서 SD 프록시와 원본 4K를 번갈아 열람)에서 특히 위험하다.
- 캐시 라이브러리(`lru` crate 등)의 기본 API가 엔트리 수 기반이라 별다른 고민 없이 그대로 사용했을 때.

**권장**:
```rust
struct FrameCache {
    entries: lru::LruCache<u64, YuvPlane>,
    max_bytes: usize,
    current_bytes: usize,
}

impl FrameCache {
    fn insert(&mut self, key: u64, frame: YuvPlane) {
        let frame_bytes = frame.data.len();
        while self.current_bytes + frame_bytes > self.max_bytes {
            if let Some((_, evicted)) = self.entries.pop_lru() {
                self.current_bytes -= evicted.data.len();
            } else {
                break;
            }
        }
        self.current_bytes += frame_bytes;
        self.entries.put(key, frame);
    }
}
```
- 엔트리 개수가 아니라 실제 바이트 크기(해상도 × 채널 수 × 비트 심도)를 기준으로 축출(evict)하도록 캐시를 재설계한다(MEM-017과 직결).
- `max_bytes`는 가용 시스템 메모리의 일정 비율(예: 10~20%)로 동적으로 설정하는 것도 고려한다.

**탐지 방법**:
- 캐시 생성 코드에서 크기 파라미터가 "개수"인지 "바이트"인지 이름/타입으로 확인(`LruCache::new(200)`처럼 정수 하나만 넘기면 의심).
- 서로 다른 해상도 파일 두 개를 열어 RSS(Resident Set Size)를 비교하는 회귀 테스트를 runtime에 추가.

**예외**:
- 캐시에 들어가는 항목이 항상 고정 크기임이 보장되는 경우(예: 고정 해상도 썸네일 캐시)라면 개수 기반 제한도 충분하다.

**Bitvue 판정**: Confirmed — crates/bitvue-core/src/stream_state.rs:585,594 `FrameModel`의 `lru::LruCache<usize, CachedFrame>`가 해상도와 무관하게 고정 32개 엔트리로 제한(바이트 예산 없음). 대조적으로 ByteCache(byte_cache.rs)는 세그먼트가 고정 크기라 바이트 기준 캐시로 정당함. filmstrip.rs의 `ThumbnailCache.max_cache_size`도 개수 기준.

---

### MEM-017: 실제 메모리 비용을 계산하지 않는 LRU
**분류**: MEM · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct SyntaxTreeCache {
    cache: lru::LruCache<u64, Arc<SyntaxNode>>,
}

fn cache_cost(node: &SyntaxNode) -> usize {
    std::mem::size_of::<SyntaxNode>() // 트리 전체가 아니라 루트 노드 하나의 크기만 계산
}
```

**문제**:
- `size_of::<SyntaxNode>()`는 최상위 노드 자체의 스택 크기만 알려줄 뿐, 그 노드가 재귀적으로 소유한 자식 트리 전체의 힙 사용량은 전혀 반영하지 못한다.
- 결과적으로 "노드 하나=고정 비용"이라는 잘못된 가정 아래 캐시가 실제로는 트리 크기(수만 노드)에 비례하는 메모리를 쓰면서도 스스로는 자신을 "가볍다"고 착각한다.
- MEM-016과 함께 발생하면 문제가 배가된다: entry 수 제한 + 잘못된 cost 계산이 겹치면 캐시 크기 예측이 완전히 어긋난다.

**발생 조건**:
- 캐시 라이브러리가 요구하는 "cost 함수"를 형식적으로만 구현하고 실제 재귀 구조를 순회하는 비용 계산 함수를 만들지 않았을 때.
- 트리/그래프처럼 소유 관계가 깊은 자료구조를 캐시에 넣을 때 흔히 발생한다.

**권장**:
```rust
fn tree_memory_footprint(node: &SyntaxNode) -> usize {
    let mut total = std::mem::size_of::<SyntaxNode>();
    if let SyntaxNode::Branch { children, .. } = node {
        total += children.capacity() * std::mem::size_of::<SyntaxNode>();
        for child in children {
            total += tree_memory_footprint(child); // 재귀적으로 실제 비용 합산
        }
    }
    total
}
```
- cost 함수는 재귀 구조 전체를 순회해 실제 바이트 수를 근사하도록 구현한다(정확한 값이 부담스러우면 노드 수 × 평균 노드 크기로 근사해도 개수 기반보다는 훨씬 낫다).
- 비용 계산 자체가 비싸다면(트리가 매우 클 때) 트리 생성 시점에 크기를 계산해 노드에 캐싱해두고 캐시는 그 캐싱된 값을 읽기만 하도록 한다.

**탐지 방법**:
- 캐시의 cost/weight 함수 구현을 찾아 재귀 자료구조에 대해 `size_of::<T>()` 한 줄로 끝나는지 확인(고정값 반환은 경고 신호).
- 캐시에 큰 트리를 넣고 작은 트리를 넣었을 때 실제 RSS 변화가 cost 함수가 보고하는 값과 비례하는지 runtime에서 검증.

**예외**:
- 트리 크기가 애초에 좁은 범위로 고정되어 있다면(예: 항상 고정 깊이의 컨테이너 박스 트리) 고정 비용 근사가 실용적으로 충분할 수 있다.

**Bitvue 판정**: N/A — 재귀 구조에 대해 `size_of::<T>()`만으로 비용을 오판하는 잘못된 cost 함수 자체가 발견되지 않음(코드베이스가 애초에 비용 기반 축출을 시도하지 않고 단순 개수 제한만 사용 — 이는 MEM-016 사례).

---

### MEM-018: String key 기반 대규모 syntax table
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
use std::collections::HashMap;

fn build_syntax_table(elements: &[(String, i64)]) -> HashMap<String, i64> {
    elements.iter().cloned().collect() // "sps_seq_parameter_set_id" 같은 긴 문자열 키가 수천 번 반복
}
```

**문제**:
- HEVC/VVC SPS/PPS/슬라이스 헤더는 `sps_video_parameter_set_id`, `log2_max_pic_order_cnt_lsb_minus4`처럼 이름이 긴 syntax element가 수백 개에 달하며, 이를 프레임/슬라이스마다 문자열 키 map으로 만들면 동일한 문자열이 반복적으로 힙에 재할당된다.
- 문자열 비교 기반 조회(`table.get("log2_max_pic_order_cnt_lsb_minus4")`)는 정수/enum 키 조회보다 훨씬 느리며, 해시 계산 비용도 문자열 길이에 비례해 커진다.
- 오탈자가 컴파일 타임에 걸러지지 않아 존재하지 않는 키를 조회해도 조용히 `None`이 반환되는 버그 온상이 된다.

**발생 조건**:
- 여러 코덱의 syntax element를 통합 테이블로 다루려다 보니 "이름이 다 다르니 문자열로 통일하자"는 결정을 내렸을 때.
- 디버그/로깅 편의를 위해 만든 임시 테이블이 그대로 정식 데이터 경로로 승격되었을 때.

**권장**:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SyntaxElementId {
    SpsSeqParameterSetId,
    Log2MaxPicOrderCntLsbMinus4,
    // ...
}

fn build_syntax_table(elements: &[(SyntaxElementId, i64)]) -> HashMap<SyntaxElementId, i64> {
    elements.iter().copied().collect() // 키가 정수 크기의 enum, 힙 할당 없음
}
```
- syntax element 이름은 컴파일 타임에 닫힌 집합이므로 enum(필요하면 `#[repr(u16)]`)으로 표현하고, 사람이 읽을 이름은 `as_str()` 매핑 함수로 UI 레이어에서만 얻는다.
- 표준 문서의 element 목록이 방대하다면 코드 생성(build.rs, 매크로)으로 enum과 이름 매핑을 자동 생성해 수작업 오류를 줄인다.

**탐지 방법**:
- `HashMap<String,` 또는 `BTreeMap<String,` 선언이 syntax/파싱 관련 모듈에 있는지 grep.
- 테이블에 삽입되는 키 리터럴 수가 수십 개를 넘는지, 그리고 값의 타입 집합이 정적으로 고정되어 있는지 리뷰.

**예외**:
- 벤더 확장이나 실험적 SEI 메시지처럼 표준화되지 않아 키 집합이 사전에 닫혀 있지 않은 경우는 문자열 키가 불가피하다.

**Bitvue 판정**: Confirmed — crates/bitvue-core/src/types.rs:505 `type SyntaxNodeId = String;`와 572행 `nodes: HashMap<SyntaxNodeId, SyntaxNode>`로, 프레임당 수만~수십만 개까지 늘어날 수 있는 syntax tree 노드 전체가 힙 String 키의 HashMap으로 관리됨.

---

### MEM-019: enum 내부 대형 variant로 전체 크기 팽창
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
enum SeiMessage {
    BufferingPeriod { cpb_delays: [u32; 4] },      // 16바이트
    PicTiming { flags: u8 },                        // 1바이트
    UserDataUnregistered { payload: [u8; 4096] },   // 4096바이트 — 드물게만 나타남
    RecoveryPoint { poc: i32 },                      // 4바이트
}

fn collect_sei(messages: Vec<SeiMessage>) {
    // 모든 variant가 4096바이트 이상 크기를 갖게 됨(가장 큰 variant + 태그)
}
```

**문제**:
- Rust의 enum 크기는 가장 큰 variant에 discriminant를 더한 값으로 고정되므로, `UserDataUnregistered`처럼 드물게 등장하는 대형 variant 하나 때문에 `PicTiming`, `RecoveryPoint`처럼 흔하고 작은 variant까지 전부 4KB 이상을 차지하게 된다.
- 이 enum을 `Vec<SeiMessage>`로 대량 보관(SEI 메시지가 프레임마다 여러 개씩 누적되는 로그/타임라인 뷰)하면 실제 정보량 대비 메모리 사용량이 수백~수천 배로 부풀어 오른다.
- 이런 크기 팽창은 컴파일러 경고 없이 조용히 발생하기 때문에 프로파일링 전까지 눈치채기 어렵다.

**발생 조건**:
- SEI 페이로드처럼 가변 길이/드문 대형 데이터를 포함하는 variant와, 항상 나타나는 소형 variant가 같은 enum에 섞여 있을 때.
- `Box`를 쓰면 간접 참조가 생겨 "성능이 걱정된다"는 이유로 인라인 배열을 그대로 둔 경우.

**권장**:
```rust
enum SeiMessage {
    BufferingPeriod { cpb_delays: [u32; 4] },
    PicTiming { flags: u8 },
    UserDataUnregistered { payload: Box<[u8]> }, // 힙에 위임, enum 자체는 포인터+길이 크기만 차지
    RecoveryPoint { poc: i32 },
}
```
- 크기가 크거나 가변적인 variant의 필드는 `Box<[u8]>`/`Vec<u8>`/`Box<T>`로 감싸 간접화하여 enum 전체 크기를 나머지 variant들의 자연스러운 크기 수준으로 되돌린다.
- `std::mem::size_of::<SeiMessage>()`를 컴파일 타임 단위 테스트(`const _: () = assert!(...)` 또는 `static_assertions` crate)로 감시해 회귀를 방지한다.

**탐지 방법**:
- `clippy::large_enum_variant` lint(기본 활성화되어 있지 않다면 명시적으로 enable)가 이 패턴을 정확히 잡아낸다.
- CI에 `cargo clippy -- -W clippy::large_enum_variant`를 게이트로 추가.

**예외**:
- enum이 애초에 소수(수십 개 이하)만 생성/보관되고 핫패스와 무관하다면(예: 설정 파싱 결과) 약간의 크기 낭비는 실질적 영향이 없다.

**Bitvue 판정**: N/A — CI(`.github/workflows/ci.yml:120`)가 `cargo clippy --workspace --lib -- -D warnings`로 기본 warn 레벨인 `clippy::large_enum_variant`를 오류로 취급해 구조적으로 차단. 코드 내 큰 배열을 인라인으로 갖는 SEI류 enum variant도 발견되지 않음.

---

### MEM-020: alignment와 padding을 무시한 구조체
**분류**: MEM · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
struct MacroblockInfo {
    mb_type: u8,        // 1바이트
    qp: i32,             // 4바이트 (4바이트 정렬 필요 → mb_type 뒤 3바이트 패딩)
    skip_flag: bool,     // 1바이트
    ref_idx: [i8; 2],    // 2바이트
    mv: [i16; 2],        // 4바이트 (2바이트 정렬)
    intra_pred_mode: u8, // 1바이트
    // 필드 순서가 뒤섞여 있어 컴파일러가 곳곳에 패딩을 삽입
}
// 실제 유효 데이터는 13바이트지만 size_of는 정렬 규칙에 따라 그보다 커짐
```

**문제**:
- 필드를 크기가 뒤섞인 순서로 선언하면 Rust(C와 동일한 기본 정렬 규칙 하에서)가 각 필드 사이에 정렬을 맞추기 위한 패딩 바이트를 끼워 넣어, 구조체 크기가 필드 실제 합보다 커진다.
- 이 구조체가 매크로블록/CU 단위로 프레임당 수만 개씩 배열로 보관되면(`Vec<MacroblockInfo>`), 패딩으로 인한 손실이 원소 수만큼 곱해져 무시할 수 없는 크기가 된다.
- 캐시 라인 활용도도 떨어져 순회 성능에도 간접적으로 영향을 준다.

**발생 조건**:
- 구조체 필드를 "논리적 그룹" 순서(비트스트림 syntax 순서 그대로)로 선언하다 보니 크기가 큰 필드와 작은 필드가 교대로 나타날 때.
- 필드 수가 많고 크기가 1~4바이트로 다양한 저수준 파싱 결과 구조체에서 흔하다.

**권장**:
```rust
#[repr(C)] // 또는 기본 repr(Rust)도 필드 재배치를 하지만 명시적으로 큰 것부터 정렬하면 의도가 분명해짐
struct MacroblockInfo {
    mv: [i16; 2],        // 4바이트
    qp: i32,              // 4바이트
    ref_idx: [i8; 2],     // 2바이트
    mb_type: u8,           // 1바이트
    skip_flag: bool,        // 1바이트
    intra_pred_mode: u8,   // 1바이트
    // 큰 필드부터 작은 필드 순으로 배치해 패딩 최소화
}
```
- 필드를 크기 내림차순으로 배치해 패딩을 최소화한다(Rust 기본 `repr(Rust)`는 이미 컴파일러가 자동으로 재배치하지만, `#[repr(C)]`가 필요한 FFI 경계 구조체는 수동으로 순서를 관리해야 한다).
- 대량 배열로 쓰이는 구조체는 `std::mem::size_of`를 테스트로 고정해 리팩터링 중 실수로 필드를 추가해 크기가 커지는 것을 감지한다.

**탐지 방법**:
- FFI 경계(`#[repr(C)]`)에 있는 대량 배열용 구조체를 대상으로 필드 순서와 예상 크기를 리뷰.
- `cargo` 확장 도구(`cargo-show-asm`, 혹은 직접 `size_of`/`align_of` 출력 테스트)로 구조체 크기를 계측.

**예외**:
- `repr(Rust)`(기본값)를 쓰는 대부분의 내부 구조체는 컴파일러가 알아서 필드를 재배치해 패딩을 최소화하므로, 원소 수가 적거나(수십 개 이하) 대량 배열로 쓰이지 않는다면 수동 최적화는 불필요하다.

**Bitvue 판정**: N/A — Bitvue 자체 구조체는 기본 `repr(Rust)`를 사용해 컴파일러가 필드를 자동 재배치하며(카탈로그가 명시한 예외 케이스), 블록 단위 대량 배열에 `#[repr(C)]`를 강제하는 구조체가 발견되지 않음(예: crates/bitvue-avc/src/overlay_extraction.rs:87 `Macroblock`도 repr 지정 없음).

---

### MEM-021: hot path에서 format!() 기반 로깅 문자열 할당
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn decode_cu(cu: &CodingUnit, log_level: LogLevel) {
    log::trace!("decoding CU at ({}, {}) size={}x{} qp={}",
        cu.x, cu.y, cu.width, cu.height, cu.qp); // trace 비활성이어도 format! 인자 평가는 발생 여지
    // ...
}

fn debug_dump(cu: &CodingUnit) -> String {
    format!("CU[{},{}] {}x{} qp={} mv=({},{})", // 매 CU마다 새 String 할당
        cu.x, cu.y, cu.width, cu.height, cu.qp, cu.mv.x, cu.mv.y)
}

fn decode_frame(cus: &[CodingUnit]) {
    for cu in cus {
        let msg = debug_dump(cu); // 항상 호출되고 항상 버려짐
        if cfg!(debug_assertions) { eprintln!("{msg}"); }
    }
}
```

**문제**:
- `debug_dump`처럼 로그 문자열을 미리 만들어 반환하는 함수는 로그가 실제로 출력되지 않는 상황(release 빌드, 낮은 로그 레벨)에서도 `format!`이 항상 실행되어 CU/블록 수만큼 `String` 힙 할당이 발생한다.
- `log::trace!` 매크로 자체는 레벨 필터링을 지원하지만, 인자로 넘기는 표현식이 무거운 연산(예: 별도 함수 호출로 문자열을 미리 만드는 경우)이면 필터링 이전에 이미 비용이 발생해버린다.
- 디코드 hot path(CU/블록/샘플 단위)에서 초당 수백만 번 호출될 수 있는 코드에 로깅 할당이 섞이면, 로깅을 끈 상태에서도 상당한 오버헤드가 남는다.

**발생 조건**:
- 디버깅 편의를 위해 넣은 상세 로그가 릴리즈 빌드 경로에도 그대로 컴파일되어 남아있을 때.
- 로그 매크로가 아니라 별도 "문자열을 만들어 반환하는 헬퍼 함수"를 만들어 로그 프레임워크의 지연 평가(lazy evaluation) 이점을 스스로 무력화했을 때.

**권장**:
```rust
fn decode_cu(cu: &CodingUnit) {
    // log 매크로의 인자는 해당 레벨이 활성화된 경우에만 평가됨(지연 평가) — 이 자체는 괜찮음
    log::trace!("decoding CU at ({}, {}) size={}x{} qp={}", cu.x, cu.y, cu.width, cu.height, cu.qp);
}

fn decode_frame(cus: &[CodingUnit]) {
    for cu in cus {
        // 별도 함수로 문자열을 미리 만들지 말고, 필요한 시점/레벨에서만 매크로가 직접 포맷하게 한다
        #[cfg(debug_assertions)]
        log::trace!("CU[{},{}] {}x{} qp={} mv=({},{})", cu.x, cu.y, cu.width, cu.height, cu.qp, cu.mv.x, cu.mv.y);
    }
}
```
- 로그는 `log`/`tracing` 매크로에 직접 포맷 인자를 전달해 프레임워크의 지연 평가에 맡기고, 별도로 `String`을 미리 만드는 헬퍼 함수를 hot path에 두지 않는다.
- 정말 필요한 상세 덤프는 `#[cfg(debug_assertions)]` 또는 별도 feature 플래그로 릴리즈 빌드에서 완전히 컴파일 제외한다.
- `tracing`을 쓴다면 `tracing::trace_span!`/구조화 필드를 사용해 문자열 조합 자체를 로깅 백엔드가 활성화된 경우로 미룬다.

**탐지 방법**:
- hot path(디코드 루프, 픽셀/블록 단위 함수) 안에서 `format!`이 로깅 목적으로 호출되는지 grep(`format!\(` 뒤에 `log::`/`eprintln!`/`println!`이 뒤따르는 패턴).
- 릴리즈 빌드 프로파일에서 `alloc::fmt` 관련 심볼이 상위에 나타나는지 flamegraph로 확인.

**예외**:
- 로그 호출 빈도가 애초에 낮은 경로(파일 열기, 세션 시작/종료 등)에서는 문제가 되지 않는다.

**Bitvue 판정**: Suspected — hot path에서 로그 문자열을 미리 만들어 반환하는 `debug_dump`류 헬퍼는 grep으로 확인되지 않았으나, 코드베이스 전반의 로깅 매크로 사용 전수 조사는 하지 못해 확정하기 어려움.

---

### MEM-022: 프레임 단위 짧은 수명 할당에 arena/bump allocator 미사용
**분류**: MEM · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn parse_frame(data: &[u8]) -> FrameSyntax {
    let mut cus = Vec::new();
    let mut sub_blocks = Vec::new();
    let mut residuals = Vec::new();
    // 파싱 도중 수천 개의 작은 임시 객체가 개별적으로 할당되고,
    // 프레임 파싱이 끝나면 전부 한꺼번에 버려짐
    parse_ctus(data, &mut cus, &mut sub_blocks, &mut residuals);
    FrameSyntax { cus, sub_blocks, residuals }
}
```

**문제**:
- 프레임 파싱 도중 생성되는 CU/서브블록/잔차 등 임시 객체들은 수명이 거의 동일(프레임 파싱 시작~끝)한데도 각자 독립적으로 `malloc`되고 독립적으로 `free`된다.
- 이런 "일괄 생성, 일괄 폐기" 패턴은 일반 malloc/free 대신 arena(bump) allocator를 쓰면 할당은 포인터 증가 한 번, 해제는 arena 통째 리셋 한 번으로 끝나 오버헤드가 극적으로 줄어드는 전형적인 사례다.
- 일반 allocator를 계속 쓰면 프레임마다 수천 번의 malloc/free가 반복되어 allocator 내부 락(멀티스레드 디코딩 시)이나 프리리스트 관리 비용이 파싱 성능의 병목이 될 수 있다.

**발생 조건**:
- 여러 스레드가 각자 다른 프레임을 병렬로 파싱하는 구조에서 전역 allocator 경합이 심할 때 특히 효과가 크다.
- 파싱 단계에서 생성한 중간 객체 대부분이 최종 `FrameSyntax`로 압축/변환되고 원본 임시 객체는 버려지는 워크플로에서 적합하다.

**권장**:
```rust
use bumpalo::Bump;

fn parse_frame<'arena>(data: &[u8], arena: &'arena Bump) -> FrameSyntax<'arena> {
    let cus = bumpalo::collections::Vec::new_in(arena);
    // arena에서 할당된 임시 객체들은 arena.reset() 한 번으로 전부 해제됨
    parse_ctus_in_arena(data, arena, cus)
}

fn decode_stream(frames_data: &[&[u8]]) {
    let mut arena = Bump::new();
    for data in frames_data {
        let frame = parse_frame(data, &arena);
        finalize_frame(frame);
        arena.reset(); // 다음 프레임을 위해 재사용, 실제 free 시스템콜 없음
    }
}
```
- `bumpalo` 등 arena crate를 프레임 단위 파싱 임시 객체에 도입하고, 프레임 처리가 끝나면 `reset()`으로 일괄 해제한다.
- 최종적으로 프레임 경계를 넘어 유지되어야 하는 결과물(디코드된 픽셀, 요약 메타데이터)만 arena 밖의 일반 힙(`Vec`/`Box`)으로 복사해 남긴다.

**탐지 방법**:
- 파싱 함수 내부에서 생성되는 임시 객체 수와 그 수명(함수 스코프 내에서만 쓰이고 끝나는지)을 리뷰해 arena 적용 후보를 선별(Structural).
- 프레임 파싱 벤치마크에서 dhat/heaptrack으로 프레임당 malloc 호출 횟수를 측정하고, arena 도입 전후를 비교.

**예외**:
- 파싱 결과 대부분이 그대로 장기 보관되어야 해서(임시성이 낮아) arena reset 시점에 대부분의 데이터를 다시 복사해야 한다면 arena 도입 이득이 상쇄될 수 있다.

**Bitvue 판정**: Suspected — 워크스페이스 어디에도 `bumpalo` 등 arena/bump allocator 의존성이 없음. MEM-003/004/018에서 확인된 대로 syntax tree 파싱이 노드마다 String/Vec을 다수 생성하는 구조라 arena 도입 시 이득이 클 것으로 구조적으로 보이나, 실측 프로파일링 근거는 없음.

---

### MEM-023: hot loop에서 Vec::from_iter로 크기 힌트 없이 수집
**분류**: MEM · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
fn extract_qp_values(cus: &[CodingUnit]) -> Vec<i32> {
    Vec::from_iter(cus.iter().filter(|cu| !cu.skip_flag).map(|cu| cu.qp))
    // 이터레이터가 size_hint를 정확히 제공하지 못하면(filter 사용) 내부적으로 재할당 반복 가능
}
```

**문제**:
- `filter`가 섞인 이터레이터 체인은 `size_hint()`의 상한만 알 수 있고 정확한 개수를 모르므로, `Vec::from_iter`/`collect`가 내부적으로 여러 차례 재할당하며 성장할 수 있다(구현에 따라 다르지만 최악의 경우 MEM-009와 동일한 문제로 귀결).
- 이 패턴이 프레임마다 반복 호출되는 통계 집계(QP 히스토그램, MV 분포 등) 함수에 있으면, 필터링 비율이 높을수록(대부분 스킵되고 일부만 남는 경우) 과대 추정 대비 실제 사용량 격차가 커져 낭비 또는 재할당이 반복된다.
- 개별 사례로는 사소하지만, 이런 통계 집계 함수가 여러 개 존재하고 프레임마다 반복 호출되면 누적 비용이 무시할 수 없어진다.

**발생 조건**:
- 프레임/슬라이스 통계를 UI 사이드 패널(히스토그램, 그래프)에 표시하기 위해 매 프레임 재계산하는 함수에서 흔하다.
- `filter`, `flat_map`처럼 결과 개수를 컴파일러가 알 수 없는 어댑터를 체인 중간에 사용할 때 특히 그렇다.

**권장**:
```rust
fn extract_qp_values(cus: &[CodingUnit]) -> Vec<i32> {
    let mut values = Vec::with_capacity(cus.len()); // 상한으로 예약(필터링돼도 과할당이 재할당보다 저렴)
    values.extend(cus.iter().filter(|cu| !cu.skip_flag).map(|cu| cu.qp));
    values.shrink_to_fit(); // 결과를 오래 보관한다면 과대 capacity 정리
    values
}
```
- 필터링 비율을 대략 알 수 있다면(예: 대부분의 CU가 non-skip) 입력 길이를 상한으로 `with_capacity`를 예약한 뒤 `extend`하는 편이 무작정 `collect`하는 것보다 재할당 횟수를 줄인다.
- 결과를 오래 보관해야 한다면 `shrink_to_fit`으로 과대 예약을 정리한다(단발성 반환값이면 생략 가능).

**탐지 방법**:
- 통계/집계 함수에서 `filter`/`flat_map` 뒤에 바로 `.collect()`/`Vec::from_iter`가 오는 패턴을 grep하고, 호출 빈도(프레임당 1회 이상)를 확인.
- clippy `clippy::extend_with_drain` 등 관련 lint를 참고.

**예외**:
- 결과 크기가 원래 작거나(수십 개 이하) 호출 빈도가 낮다면 재할당 비용은 무시할 수준이라 최적화가 불필요하다.

**Bitvue 판정**: Suspected — `filter`/`flat_map` 체인 뒤 크기 힌트 없이 `collect()`하는 통계 집계 함수가 있는지 전수 확인하지 못함. 판단에 충분한 근거를 찾지 못해 단정하지 않음.

---

### MEM-024: 디코드된 프레임 버퍼를 필요 이상 오래 보유
**분류**: MEM · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct PlaybackSession {
    all_decoded_frames: Vec<YuvPlane>, // 재생한 프레임을 전부 누적 보관
}

impl PlaybackSession {
    fn on_frame_decoded(&mut self, frame: YuvPlane) {
        self.all_decoded_frames.push(frame); // 되감기를 대비해 계속 쌓아둠
        render(self.all_decoded_frames.last().unwrap());
    }
}
```

**문제**:
- "되감기 시 다시 디코드하지 않아도 되게" 같은 편의를 위해 재생된 모든 프레임을 무제한으로 누적하면, 긴 스트림을 끝까지 재생했을 때 전체 압축 해제된 픽셀 데이터가 메모리에 모두 상주하게 된다(원본 압축 파일 크기의 수십~수백 배에 달할 수 있음).
- reference frame 관리처럼 디코더 내부에서 잠깐 필요한 것과, 재생/탐색 UI가 "혹시 몰라서" 오래 들고 있는 것은 성격이 다른데 하나의 무제한 벡터로 뭉뚱그려지면 메모리 상한이 사실상 사라진다.
- 명시적인 `drop`이나 버퍼 재사용 지점이 코드에 없으면, 컴파일러/런타임이 대신 판단해주지 않으므로 이 문제는 스스로 자라기만 하고 절대 줄지 않는다.

**발생 조건**:
- 되감기/탐색(seek back) UX를 단순하게 구현하려고 "이미 디코드한 건 버리지 말자"는 결정을 내렸을 때.
- 세션이 길게 유지되는 장시간 분석 워크플로(수 시간 로그/스트림 리뷰)에서 문제가 누적된다.

**권장**:
```rust
struct PlaybackSession {
    recent_frames: VecDeque<YuvPlane>, // 최근 N개만 유지(되감기 버퍼)
    max_buffered: usize,
}

impl PlaybackSession {
    fn on_frame_decoded(&mut self, frame: YuvPlane) {
        if self.recent_frames.len() >= self.max_buffered {
            self.recent_frames.pop_front(); // 오래된 프레임은 명시적으로 드롭 → 메모리 회수
        }
        self.recent_frames.push_back(frame);
    }
}
```
- "재생 중 최근 N프레임"처럼 되감기 요구를 유한한 윈도우로 한정하고, 그 이상 과거로 가려면 원본에서 재디코드하는 것을 기본 전략으로 삼는다.
- 원본 파일에서의 재탐색(seek)이 빠르다면(키프레임 인덱스가 있다면) 프레임을 오래 들고 있는 것보다 재디코드가 전체 메모리 관점에서 더 합리적인 경우가 많다.

**탐지 방법**:
- 세션/플레이백 상태 구조체에서 프레임 버퍼가 상한 없는 `Vec`/`VecDeque`인지, `max_buffered` 같은 명시적 상한이 있는지 리뷰.
- 긴 파일을 끝까지 재생시키는 runtime 테스트에서 RSS가 파일 길이에 비례해 무한정 증가하는지 확인.

**예외**:
- 짧은 클립(수 초, 수십 프레임) 전용 분석 도구라면 전체를 메모리에 두는 것이 오히려 단순하고 안전하다.

**Bitvue 판정**: Confirmed — src-tauri/src/commands/quality.rs의 `decode_samples_subset`(약 833-884행)과 `decode_all_frames`이 요청된 인덱스와 무관하게 `0..=max_idx`(또는 파일 전체)를 전부 디코드해 `Vec<Option<DecodedFrame>>`에 동시 보관한 뒤에야 필요한 프레임만 추려냄 — 늦은 프레임 하나만 요청해도 그 이전 전체가 순간적으로 메모리에 상주.

---

### MEM-025: 썸네일 생성 시 풀해상도 중간 버퍼 할당
**분류**: MEM · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn generate_thumbnail(full_frame: &YuvPlane, target_w: usize, target_h: usize) -> Vec<u8> {
    let full_rgba = yuv_to_rgba(full_frame); // 8K 프레임이면 ~133MB 중간 버퍼
    downscale_rgba(&full_rgba, full_frame.width, full_frame.height, target_w, target_h)
    // 결과 썸네일은 수십 KB인데, 그것을 위해 133MB를 거쳐감
}
```

**문제**:
- 필름스트립처럼 작은 썸네일(예: 160×90) 수백 개를 만드는 과정에서 매번 풀해상도 RGBA 변환을 거친다면, 최종 결과물 대비 수백~수천 배 큰 중간 버퍼를 반복적으로 할당/해제하게 된다.
- 필름스트립 UI가 스크롤될 때마다 보이는 범위의 썸네일을 새로 생성한다면, 이 낭비가 스크롤 프레임률에 직접적인 병목으로 나타난다.
- 풀해상도 YUV→RGBA 변환 자체도 CPU 비용이 크므로 메모리 낭비와 CPU 낭비가 겹친다.

**발생 조건**:
- 디코더가 이미 풀해상도로 디코드한 프레임을 재사용하는 것이 "당연히 효율적"이라고 가정하고, 축소 경로를 별도로 설계하지 않았을 때.
- 다양한 코덱/컨테이너에 대해 공통 썸네일 파이프라인을 만들면서 "일단 RGBA로 바꾸고 나서 처리하자"는 단순한 설계를 택했을 때.

**권장**:
```rust
fn generate_thumbnail(full_frame: &YuvPlane, target_w: usize, target_h: usize) -> Vec<u8> {
    // YUV 평면 단계에서 먼저 다운샘플링(정수 배율 박스 필터 등)한 뒤 RGBA 변환
    let small_yuv = downscale_yuv(full_frame, target_w, target_h);
    yuv_to_rgba(&small_yuv) // 중간 버퍼가 이미 썸네일 크기
}
```
- 다운스케일을 RGBA 변환 "이전" 단계(YUV 평면)에서 수행해 중간 버퍼 크기를 처음부터 작게 유지한다.
- 디코더가 지원한다면(dav1d의 저해상도 출력 옵션 등) 애초에 낮은 해상도로 디코드하는 경로를 별도로 사용하는 것도 검토한다.
- 필름스트립처럼 다수의 썸네일을 반복 생성하는 UI는 생성된 썸네일 자체를 캐시(MEM-016/017 참고)해 재생성을 피한다.

**탐지 방법**:
- 썸네일/프리뷰 생성 함수에서 `yuv_to_rgba`(풀해상도) 호출이 다운스케일 호출보다 먼저 나오는지 코드 순서를 리뷰.
- 스크롤/필름스트립 조작 중 dhat으로 순간 피크 메모리를 관찰(풀해상도 크기의 스파이크가 반복되면 신호).

**예외**:
- 정확한 픽셀 품질 비교(원본 대비 축소 알고리즘 차이 검증)가 목적인 디버그/QA 도구에서는 풀해상도를 거치는 것이 의도적으로 필요할 수 있다.

**Bitvue 판정**: Suspected — crates/bitvue-core/src/filmstrip.rs:54 `generate_thumbnail`이 이미 RGB로 변환된 `CachedFrame.rgb_data`(풀해상도)에서 다운샘플링하지만, 이 RGB 버퍼가 썸네일 전용으로 새로 만들어지는지 아니면 다른 표시 목적과 공유되는지는 확인하지 못해 낭비 여부가 명확하지 않음.

---

### MEM-026: 루프 내부에서 호이스트 가능한 할당을 반복 수행
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn compute_frame_diffs(frames: &[YuvPlane]) -> Vec<f64> {
    let mut diffs = Vec::with_capacity(frames.len());
    for i in 1..frames.len() {
        let mut abs_diff = vec![0u8; frames[i].data.len()]; // 매 반복마다 재할당(크기는 항상 동일)
        for (d, (a, b)) in abs_diff.iter_mut().zip(frames[i].data.iter().zip(frames[i-1].data.iter())) {
            *d = a.abs_diff(*b);
        }
        diffs.push(abs_diff.iter().map(|&v| v as f64).sum::<f64>() / abs_diff.len() as f64);
    }
    diffs
}
```

**문제**:
- 프레임 크기가 루프 전체에서 동일함에도 `abs_diff` 버퍼를 매 반복마다 새로 `vec![0u8; ...]`로 할당해, 프레임 수만큼 불필요한 힙 할당/해제가 반복된다.
- 이 버퍼는 루프 밖으로 호이스트(hoist)해 재사용해도 로직상 아무 문제가 없는데, 반복문 안에 선언되어 있다는 이유만으로 매번 새로 만들어진다.
- 씬 전환 감지, 프레임 간 차이 기반 통계(품질 지표 사전 계산 등)처럼 프레임 수만큼 반복되는 분석 루프에서 이런 패턴이 누적되면 분석 자체보다 할당 비용이 더 커질 수 있다.

**발생 조건**:
- 함수를 작성할 때 "이 값은 이 반복에서만 쓰인다"는 이유로 습관적으로 루프 안에 선언했지만, 실제로는 크기/타입이 반복마다 불변인 경우.
- 리뷰 시 "루프 안의 할당"이라는 표면적 형태만 보고 실제 불변성 여부를 확인하지 않았을 때 놓치기 쉽다.

**권장**:
```rust
fn compute_frame_diffs(frames: &[YuvPlane]) -> Vec<f64> {
    let mut diffs = Vec::with_capacity(frames.len());
    let mut abs_diff = vec![0u8; frames.get(0).map_or(0, |f| f.data.len())]; // 루프 밖에서 한 번만 할당
    for i in 1..frames.len() {
        for (d, (a, b)) in abs_diff.iter_mut().zip(frames[i].data.iter().zip(frames[i-1].data.iter())) {
            *d = a.abs_diff(*b);
        }
        diffs.push(abs_diff.iter().map(|&v| v as f64).sum::<f64>() / abs_diff.len() as f64);
    }
    diffs
}
```
- 크기/형태가 반복 전체에서 불변인 버퍼는 루프 밖에서 한 번만 할당하고, 루프 안에서는 내용만 덮어쓴다(`fill`, 슬라이스 재사용 등).
- 이런 최적화는 작아 보여도 코드 리뷰에서 "이 할당이 정말 반복마다 달라지는가?"를 습관적으로 질문하는 것만으로 상당수 발견 가능하다.

**탐지 방법**:
- clippy의 일부 lint(`clippy::same_item_push` 등)가 유사 패턴을 잡기도 하지만, 이 항목은 대체로 코드 리뷰/manual inspection이 가장 효과적이다.
- 루프 본문에서 `vec![`/`Vec::with_capacity`가 나타나면 해당 크기 표현식이 루프 변수에 의존하는지 확인하는 체크리스트 항목을 둔다.

**예외**:
- 버퍼 크기가 실제로 반복마다 달라진다면(가변 해상도 프레임 비교 등) 호이스트가 불가능하므로 이 패턴이 정당하다.

**Bitvue 판정**: N/A — 프레임 간 diff/씬 전환 감지처럼 루프마다 동일 크기 버퍼를 반복 재할당할 만한 기능(scene detection, frame diff) 자체가 코드베이스에서 발견되지 않음.

---

### MEM-027: 고정 크기 배열이어야 할 곳에 Vec<u8> 사용
**분류**: MEM · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
struct MotionVectorPredictor {
    candidates: Vec<[i16; 2]>, // 항상 최대 5개(HEVC MVP candidate list)뿐인데 Vec 사용
}

fn build_mvp_list() -> MotionVectorPredictor {
    let mut candidates = Vec::new(); // 힙 할당 + 포인터 간접 참조
    candidates.push([0, 0]);
    candidates.push([1, 1]);
    MotionVectorPredictor { candidates }
}
```

**문제**:
- HEVC MVP 후보 리스트, VP9 참조 프레임 슬롯(최대 8개), AV1 CDF 컨텍스트 등 코덱 표준상 상한이 명확히 정해진 소규모 컬렉션을 `Vec`으로 표현하면, 매번 별도 힙 할당이 필요해지고 데이터가 힙에 있어 스택/인라인 배열 대비 캐시 지역성이 나쁘다.
- 이런 구조체가 CU/블록 단위로 수만 번 생성되는 hot path에 있으면 MEM-002/MEM-008과 동일한 방식으로 할당 오버헤드가 누적된다.
- `Vec`은 상한 검사가 런타임에만 이루어지는 반면, 고정 크기 배열이나 `ArrayVec` 같은 타입은 상한을 타입 레벨에서 어느 정도 드러내 실수를 줄여준다.

**발생 조건**:
- 표준 문서상 "최대 N개"로 정의된 리스트(참조 픽처 리스트, MVP 후보, CDF 테이블 등)를 별 생각 없이 범용 `Vec`으로 모델링했을 때.
- 초기 프로토타입에서 상한이 아직 확정되지 않아 `Vec`을 썼다가, 표준이 확정된 이후에도 리팩터링되지 않았을 때.

**권장**:
```rust
use arrayvec::ArrayVec;

struct MotionVectorPredictor {
    candidates: ArrayVec<[i16; 2], 5>, // 스택에 고정, 힙 할당 없음, 상한이 타입에 드러남
}
```
- 상한이 코덱 표준에 의해 고정된 컬렉션은 `[T; N]`(항상 꽉 참) 또는 `arrayvec::ArrayVec<T, N>`(가변 길이지만 상한 고정)으로 표현해 힙 할당을 제거한다.
- 상한을 표준 문서의 상수로 명명(`const MAX_MVP_CANDIDATES: usize = 5`)해 매직 넘버를 피하고 표준 개정 시 한 곳만 수정하면 되게 한다.

**탐지 방법**:
- 구조체 필드가 `Vec<T>`인데 push되는 최대 횟수가 코드 전체에서 상수로 제한되어 있는지(즉 사실상 고정 상한인지) grep 및 리뷰.
- 표준 스펙 문서(각 코덱 사양)와 대조해 "최대 N" 문구가 있는 리스트들을 목록화해 점검.

**예외**:
- 상한이 매우 크거나(수백 개 이상) 프로파일/레벨에 따라 크게 달라지는 경우는 `Vec`이 더 적절하다.

**Bitvue 판정**: Suspected — crates/bitvue-hevc/src/slice.rs:63-72 `RefPicListModification.list_entry_l0/l1: Vec<u8>`처럼 표준상 상한이 있는 소규모 리스트가 `Vec`으로 표현된 사례가 있으나, 슬라이스당 1회 생성이라 빈도가 낮아 카탈로그가 우려하는 블록당(수만 회) hot path와는 심각도 차이가 큼. `ArrayVec` 등 고정 상한 컨테이너 의존성은 없음.

---

### MEM-028: 더블 버퍼링 미구현으로 매 프레임 스왑마다 재할당
**분류**: MEM · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct VideoRenderer {
    current_frame: Option<Vec<u8>>,
}

impl VideoRenderer {
    fn present(&mut self, new_frame: Vec<u8>) {
        self.current_frame = Some(new_frame); // 이전 프레임 버퍼는 drop, 다음 프레임은 새로 할당된 채 전달됨
    }
}

fn decode_and_present(renderer: &mut VideoRenderer, plane: &YuvPlane) {
    let rgba = yuv_to_rgba(plane); // 매번 새 Vec<u8> 생성해서 renderer로 이동
    renderer.present(rgba);
}
```

**문제**:
- GPU 업로드/화면 표시(present) 파이프라인에서 이전 프레임 버퍼를 재사용하지 못하고 매번 새 버퍼를 만들어 넘기면, CPU 측에서 생성-이동-폐기가 프레임마다 반복되어 MEM-013과 유사한 낭비가 발생한다.
- 더블/트리플 버퍼링이 없으면 GPU가 이전 프레임을 아직 읽는 중(디스플레이 동기화 대기)일 때 CPU가 같은 버퍼를 덮어써도 되는지 판단할 수 없어, 안전하게 매번 새 버퍼를 만드는 보수적인 설계로 흐르기 쉽다 — 이것이 바로 이 안티패턴의 근본 원인이다.
- 결과적으로 프레임 표시 파이프라인이 "안전을 위해 항상 새로 할당"하는 상태에 고착되어 렌더링 hot path의 할당 부담이 구조적으로 해소되지 않는다.

**발생 조건**:
- 재생/스크러빙처럼 프레임을 연속적으로 GPU에 올려야 하는 경로에서, "버퍼 소유권을 렌더러에 넘기고 끝"이라는 단순한 API로 시작해 재사용 메커니즘을 나중에 추가하지 못했을 때.
- vsync/프레임 페이싱 로직이 아직 없어 CPU와 GPU 진행 속도를 조율할 방법이 없을 때 더블 버퍼링 도입이 뒤로 밀린다.

**권장**:
```rust
struct VideoRenderer {
    buffers: [Vec<u8>; 2], // 더블 버퍼
    write_idx: usize,
}

impl VideoRenderer {
    fn write_buffer(&mut self) -> &mut Vec<u8> {
        &mut self.buffers[self.write_idx]
    }

    fn present(&mut self) {
        upload_to_gpu(&self.buffers[self.write_idx]); // GPU가 읽는 동안
        self.write_idx = 1 - self.write_idx;           // 다음 프레임은 반대편 버퍼에 씀
    }
}
```
- 두 개(또는 세 개)의 버퍼를 미리 할당해두고 매 프레임 인덱스만 교대로 전환하는 더블/트리플 버퍼링을 도입해, present 경로에서 힙 할당을 완전히 제거한다.
- GPU 업로드가 비동기라면 fence/세마포어로 "이 버퍼를 다시 써도 되는 시점"을 추적해 조기 덮어쓰기로 인한 티어링을 방지한다.

**탐지 방법**:
- 렌더/present 경로 함수 시그니처가 매번 새 `Vec<u8>`을 인자로 받는지(소유권 이전 방식인지) 확인하고, 내부에 고정 버퍼 풀이 있는지 리뷰.
- 재생 중 dhat/heaptrack으로 프레임 레이트와 동일한 주기의 대형 할당이 반복되는지 확인(MEM-013과 동일한 신호를 present 레이어에서 재확인).

**예외**:
- 정지 이미지 뷰어처럼 초당 프레임 표시 빈도가 매우 낮은(사용자 조작에 의해서만 갱신되는) 경로라면 더블 버퍼링 도입 이득이 작다.

**Bitvue 판정**: N/A — Rust/Tauri 백엔드에 자체 GPU present 루프나 프레임버퍼 스왑 코드가 없고, 화면 렌더링은 브라우저 `<canvas>`(frontend/utils/yuv/renderer.ts)에 위임되어 더블 버퍼링은 브라우저 컴포지터가 담당. 이 안티패턴이 겨냥하는 Rust 측 커스텀 렌더 파이프라인 자체가 존재하지 않음.

---

### MEM-029: 대형 일회성 파싱 후 메모리가 OS로 반환되지 않음
**분류**: MEM · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn parse_full_index(path: &str) -> ContainerIndex {
    let mmap = load_bitstream(path).unwrap();
    let mut scratch = Vec::with_capacity(500_000_000); // 인덱싱 중 500MB 스크래치 버퍼
    let index = build_index(&mmap, &mut scratch); // 인덱싱 완료 후 scratch는 더 이상 불필요
    index // scratch는 함수 종료 시 drop되지만, allocator가 OS에 페이지를 반환하지 않을 수 있음
}
// 이후 세션 내내 프로세스 RSS가 500MB 근처에서 내려오지 않음
```

**문제**:
- 파일 열기 시 한 번만 필요한 대형 스크래치 버퍼(전체 인덱싱, 전체 무결성 검사 등)를 사용하고 `drop`하더라도, 많은 시스템 allocator(glibc malloc, 일부 jemalloc 설정)는 해제된 큰 메모리 블록을 즉시 OS에 반환(`munmap`/`madvise`)하지 않고 향후 재사용을 위해 보유한다.
- 그 결과 실제로는 더 이상 쓰지 않는 메모리인데도 프로세스의 RSS/작업 관리자상 메모리 사용량이 그 시점 이후 내내 높게 유지되어, 사용자가 "이 앱이 메모리를 계속 많이 쓴다"고 오인하게 만든다.
- 여러 파일을 연달아 열고 닫는 워크플로에서는 이런 "반환되지 않은" 피크가 파일마다 새로 쌓이지는 않더라도(allocator가 재사용은 함), 시스템 전체적으로 다른 프로세스가 쓸 수 있는 메모리가 줄어드는 효과는 남는다.

**발생 조건**:
- 파일을 열 때 한 번 큰 스크래치를 쓰고 그 이후로는 훨씬 작은 메모리만 필요한 워크플로(초기 인덱싱 → 이후 순차 재생)에서 전형적으로 나타난다.
- glibc 기본 allocator를 그대로 쓰는 Linux 빌드에서 특히 두드러지며, macOS/Windows의 기본 allocator는 동작이 다를 수 있다.

**권장**:
```rust
fn parse_full_index(path: &str) -> ContainerIndex {
    let mmap = load_bitstream(path).unwrap();
    let index = {
        let mut scratch = Vec::with_capacity(500_000_000);
        build_index(&mmap, &mut scratch)
    }; // scratch가 스코프 종료로 drop
    index
}

// 애플리케이션 시작 시점에 jemalloc/mimalloc 등 반환 정책이 더 적극적인 allocator로 교체
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```
- 큰 스크래치 버퍼는 최대한 좁은 스코프에 가둬 사용 즉시 drop되도록 하고(이미 예시처럼 되어 있다면), 그래도 RSS가 안 내려온다면 allocator 자체를 `jemalloc`/`mimalloc`처럼 메모리 반환 정책을 세밀히 제어할 수 있는 것으로 교체한다.
- 정말 중요한 지점(파일 닫기 직후 등)에서는 플랫폼별 API(`malloc_trim` on glibc)를 명시적으로 호출하는 것도 고려할 수 있으나 이식성이 떨어지므로 최후의 수단으로 취급한다.

**탐지 방법**:
- 대형 파일을 열었다 닫은 뒤 RSS가 파일을 열기 전 수준으로 돌아오는지 runtime에서 측정(플랫폼별 메모리 도구: Activity Monitor, `/proc/self/status`의 VmRSS 등).
- allocator를 바꿔가며(system vs jemalloc vs mimalloc) 동일 시나리오의 RSS 곡선을 비교하는 벤치마크를 CI 또는 수동 점검 루틴에 추가.

**예외**:
- 애플리케이션이 짧게 실행되고 곧 종료되는 CLI 도구(1회성 배치 분석기)라면 프로세스 종료 시 OS가 전체 메모리를 회수하므로 이 문제는 무시해도 된다.

**Bitvue 판정**: Suspected — 워크스페이스에 `jemalloc`/`mimalloc` 등 커스텀 글로벌 allocator 설정이 없어(`global_allocator` 미사용) 기본 시스템 allocator에 의존. ByteCache가 최대 256MB 세그먼트 캐시를 예약하는 등 대형 버퍼가 존재하나, 실제로 열기/닫기 후 RSS가 회수되지 않는지는 런타임 측정 없이는 확정할 수 없음.

---

### MEM-030: Rc<RefCell<>> 남용으로 인한 참조 카운트 오버헤드와 순환 참조 누수
**분류**: MEM · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
use std::rc::Rc;
use std::cell::RefCell;

struct TreeNode {
    value: SyntaxValue,
    parent: Option<Rc<RefCell<TreeNode>>>, // 부모를 강한 참조로 보관
    children: Vec<Rc<RefCell<TreeNode>>>,
}

fn link(parent: &Rc<RefCell<TreeNode>>, child: Rc<RefCell<TreeNode>>) {
    child.borrow_mut().parent = Some(Rc::clone(parent)); // parent -> child -> parent 순환 참조
    parent.borrow_mut().children.push(child);
}
```

**문제**:
- 부모와 자식이 서로를 `Rc`(강한 참조)로 참조하면 참조 카운트가 절대 0이 되지 않는 순환 참조가 만들어져, 트리 전체가 명시적으로 해제되기 전까지(혹은 영원히) 메모리에서 회수되지 않는다.
- 자식이 없는 리프 노드까지 포함해 모든 노드가 `Rc<RefCell<_>>`로 감싸지면, 원소마다 참조 카운트(strong/weak 2개의 카운터) 오버헤드와 `RefCell`의 런타임 borrow 체크 비용이 추가된다.
- 대량의 syntax tree 노드에 이 패턴을 적용하면 MEM-004/MEM-005의 문제(개별 힙 할당, 캐시 지역성 저하)에 더해 순환 참조로 인한 메모리 누수까지 겹친다.
- `RefCell`의 borrow 규칙 위반은 컴파일 타임이 아니라 런타임 패닉으로만 드러나, 디버깅 난이도가 올라간다.

**발생 조건**:
- 다른 언어(GC가 있는 Java/C#/Python)의 부모-자식 양방향 참조 패턴을 그대로 Rust에 옮기려 할 때.
- 트리 순회 중 "부모로 거슬러 올라가야 하는" 요구사항(예: 오버레이 렌더링 시 상위 CU 컨텍스트 참조)을 안일하게 강한 참조로 해결했을 때.

**권장**:
```rust
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct TreeNode {
    value: SyntaxValue,
    parent: Option<Weak<RefCell<TreeNode>>>, // 부모는 약한 참조로 순환 방지
    children: Vec<Rc<RefCell<TreeNode>>>,
}
```
- 트리처럼 소유 방향이 명확한 구조에서는 자식→부모 방향을 `Weak`로 바꿔 순환 참조를 끊는다.
- 근본적으로는 MEM-005/MEM-022에서 권장한 arena + 인덱스 기반 트리로 전환하면 `Rc`/`RefCell` 자체가 필요 없어져 참조 카운트 오버헤드와 순환 참조 위험이 동시에 사라진다.
- 멀티스레드 환경이 필요 없다면 `Arc<Mutex<_>>` 대신 `Rc<RefCell<_>>`를 쓰는 것 자체는 맞지만, 가능하면 공유 가변 상태 자체를 줄이는 설계(단일 소유자 + 인덱스 참조)를 우선 검토한다.

**탐지 방법**:
- `Rc<RefCell<` 패턴이 재귀적 트리/그래프 구조에 쓰이면서 부모-자식 양방향 링크가 모두 `Rc`인지 grep 및 리뷰(Structural).
- `valgrind --leak-check`나 장시간 세션에서 특정 트리 타입의 인스턴스 수가 세션 종료 후에도 줄지 않는지 확인(Runtime, 순환 참조 누수 탐지에는 제한적이지만 참고 가능).

**예외**:
- 노드 수가 적고(수십~수백 개) 생명주기가 매우 명확한(생성 즉시 해제) 소규모 구조라면 순환 참조로 인한 실질적 누수 영향이 미미할 수 있다.

**Bitvue 판정**: N/A — Bitvue 자체 코드에서 `Rc<RefCell<>>` 재귀 트리는 발견되지 않음(유일한 매치는 vendor/abseil의 무관한 유틸리티). 트리는 대신 MEM-018에서 지적된 String-키 HashMap arena 방식을 사용.

---

### MEM-031: mmap된 파일을 Vec으로 복사해서 사용
**분류**: MEM · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
fn open_for_parsing(path: &str) -> std::io::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    Ok(mmap.to_vec()) // mmap된 페이지를 그대로 다시 힙에 복사
}
```

**문제**:
- 이미 mmap을 통해 파일 전체를 가상 메모리 공간에 매핑해 lazy-loading(OS 페이지 캐시 기반)의 이점을 얻어놓고도, `.to_vec()`으로 즉시 전체를 힙에 복사하면 MEM-001(전체 read_to_end)과 동일한 문제로 되돌아간다 — mmap을 쓴 의미가 사라진다.
- 이런 코드는 대개 "다른 함수가 `&[u8]`이 아니라 `Vec<u8>`을 요구해서" 같은 타입 불일치를 급하게 봉합하려다 생기며, 원래 mmap을 도입한 이유(대형 파일의 메모리 절약)를 무효화한다.
- mmap 페이지는 필요한 부분만 물리 메모리에 올라오는데, 강제로 전체를 `Vec`에 복사하면 실제로 쓰지 않는 파일 영역까지 전부 물리 메모리에 적재된다.

**발생 조건**:
- 함수 시그니처가 `&[u8]`을 받도록 일관되게 설계되지 않고 일부는 `Vec<u8>` 소유권을 요구하는 구식 API로 남아있을 때, 호출부에서 편의상 변환해버리는 경우.
- FFI 경계(dav1d 등)에 넘기기 전 "안전하게 소유권 있는 버퍼로 만들자"는 과도한 방어적 코딩.

**권장**:
```rust
fn open_for_parsing(path: &str) -> std::io::Result<memmap2::Mmap> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    Ok(mmap) // 소유권째 반환, Deref<Target=[u8]>로 슬라이스처럼 사용
}

fn parse(mmap: &memmap2::Mmap) {
    parse_annexb(&mmap[..]); // &[u8]를 받는 API와 자연스럽게 호환
}
```
- 파싱/분석 API는 가능한 한 `&[u8]`(또는 그 위의 얇은 뷰 타입)을 받도록 통일해, 호출부가 mmap이든 `Vec`이든 그대로 넘길 수 있게 한다.
- 정말 소유권 있는 복사본이 필요한 지점(예: 파일이 닫힌 뒤에도 데이터를 보관해야 하는 경우)에서만, 그것도 필요한 구간만 최소로 복사한다.

**탐지 방법**:
- `Mmap`/`mmap2::Mmap` 뒤에 `.to_vec()`이 바로 이어지는 패턴을 grep.
- 함수 시그니처 감사에서 `Vec<u8>`을 요구하는 파싱 계열 API가 있다면 `&[u8]`로 완화 가능한지 리뷰.

**예외**:
- 파일이 매우 작고(수 KB 이하) 짧게 쓰이는 경우라면 mmap 자체가 과할 수 있고, 이 경우 `.to_vec()`으로 인한 실질적 낭비도 미미하다.

**Bitvue 판정**: N/A — `Mmap...to_vec()`류 전체 복사 패턴은 grep으로 발견되지 않음. ByteCache(byte_cache.rs)는 mmap을 유지한 채 필요한 세그먼트만 LRU에 캐시하는 올바른 설계.

---

### MEM-032: Clone 남용 — Arc/Rc 대신 deep clone
**분류**: MEM · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
#[derive(Clone)]
struct StreamSummary {
    frame_stats: Vec<FrameStat>,      // 프레임 수만큼의 통계, 수만 개일 수 있음
    gop_structure: Vec<GopInfo>,
    bitrate_curve: Vec<f64>,
}

fn notify_ui(summary: &StreamSummary, tx: &std::sync::mpsc::Sender<StreamSummary>) {
    tx.send(summary.clone()).unwrap(); // 채널로 보낼 때마다 전체 deep clone
}
```

**문제**:
- `#[derive(Clone)]`이 붙은 구조체에 대형 `Vec` 필드가 여러 개 있으면, 단순히 다른 스레드/컴포넌트에 "알려주기" 위한 호출 하나가 수만 개 원소의 deep clone으로 이어져 예상치 못한 큰 비용을 유발한다.
- 이런 clone은 호출부만 봐서는 비용이 드러나지 않아(`summary.clone()` 한 줄), 코드 리뷰에서 놓치기 쉬운 "숨겨진 비용(hidden cost)"의 전형적인 사례다.
- UI 이벤트가 자주 발생하는 구조(재생 중 매 프레임 통계 업데이트)에서 이 패턴이 hot path에 있으면 채널 전송 자체가 병목이 된다.

**발생 조건**:
- Tauri의 프런트엔드-백엔드 이벤트 브리지처럼 "상태를 스냅샷으로 보내야 하는" 구조에서, 스냅샷 생성 비용을 고려하지 않고 통째로 clone할 때.
- 여러 구독자(멀티 패널 UI)에게 동일한 요약 정보를 브로드캐스트하면서 구독자 수만큼 clone이 반복될 때 특히 심하다.

**권장**:
```rust
use std::sync::Arc;

fn notify_ui(summary: &Arc<StreamSummary>, tx: &std::sync::mpsc::Sender<Arc<StreamSummary>>) {
    tx.send(Arc::clone(summary)).unwrap(); // 참조 카운트만 증가, 데이터 복사 없음
}
```
- 여러 소비자에게 같은 스냅샷을 공유해야 한다면 `Arc<T>`로 감싸 clone 비용을 포인터 복사 수준으로 낮춘다.
- 정말 각 소비자가 독립적으로 값을 변경해야 한다면(공유가 아니라 소유가 목적이라면) deep clone이 맞지만, 그 경우에도 변경이 필요한 부분만 별도로 분리해(copy-on-write 패턴 등) 전체 구조체 clone을 피할 수 있는지 검토한다.

**탐지 방법**:
- 대형 `Vec`/`HashMap` 필드를 가진 구조체에 `#[derive(Clone)]`이 붙어 있고, 그 타입에 대해 `.clone()`이 채널 전송/이벤트 발행 경로에서 호출되는지 grep.
- 프로파일러에서 UI 이벤트 발행 함수가 예상외로 CPU/메모리 비용이 큰지 확인.

**예외**:
- 구조체가 작거나(수 필드, 스칼라 위주) clone 빈도가 매우 낮다면 `Arc`로 감싸는 것이 오히려 불필요한 간접 참조 복잡도를 더할 수 있다.

**Bitvue 판정**: Confirmed(부분) — crates/bitvue-core/src/stream_state.rs의 `impl Clone for FrameModel`(약 638-646행)이 LRU의 모든 `CachedFrame`을 순회하며 `v.clone()`을 호출하는데, `y/u/v_plane`은 `Arc`라 저렴하지만 `rgb_data: Vec<u8>`는 Arc로 감싸여 있지 않아 캐시 전체 clone 시 프레임마다 풀 RGB 버퍼가 deep-copy됨.

---

### MEM-033: 파싱 시 &str 대신 String으로 소유권 복사
**분류**: MEM · **심각도**: Low · **탐지**: Static

**나쁜 예**:
```rust
struct BoxHeader {
    box_type: String, // "moov", "trak", "mdat" 등 4바이트 FourCC
}

fn parse_box_header(data: &[u8]) -> BoxHeader {
    let box_type = String::from_utf8_lossy(&data[4..8]).into_owned(); // 4바이트를 위해 힙 할당
    BoxHeader { box_type }
}
```

**문제**:
- MP4/MKV 박스 파싱처럼 원본 바이트 슬라이스(`&[u8]`, mmap 기반)가 파싱 대상 데이터 전체보다 오래 살아있는 상황에서, 4바이트짜리 FourCC 하나를 위해 매번 `String`(힙 할당)을 만드는 것은 불필요한 복사다.
- 박스가 컨테이너 안에 수백~수천 개 존재하는 경우(프래그먼트된 MP4의 다수 `moof`/`traf` 등), 이 작은 복사가 누적되어 무시할 수 없는 할당 횟수가 된다.
- FourCC는 정확히 4바이트 고정 길이이므로애초에 동적 크기 문자열일 필요가 없다.

**발생 조건**:
- 원본 데이터의 라이프타임을 구조체에 전파하는 것(`&'a str`)이 번거로워 보여서, 일단 소유권 있는 `String`으로 복사해 라이프타임 문제를 회피했을 때.
- 파서 초기 버전에서 라이프타임 설계를 미루고 임시로 `String`을 썼다가 정착된 경우.

**권장**:
```rust
struct BoxHeader {
    box_type: [u8; 4], // FourCC는 고정 4바이트, 힙 할당 전혀 없음
}

impl BoxHeader {
    fn type_str(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.box_type) // 표시가 필요한 순간에만, 그마저도 Cow로 복사 최소화
    }
}

fn parse_box_header(data: &[u8]) -> BoxHeader {
    let mut box_type = [0u8; 4];
    box_type.copy_from_slice(&data[4..8]);
    BoxHeader { box_type }
}
```
- FourCC처럼 크기가 고정된 값은 `[u8; 4]`로 표현해 할당 자체를 없앤다.
- 가변 길이지만 원본 버퍼보다 수명이 짧아도 되는 문자열은 `&'a str`로 참조를 유지하고, 라이프타임이 정말 부담스러운 경우에만 `String`으로 복사한다(그 경우도 `Cow<str>`로 필요한 경우에만 복사가 일어나게 할 수 있다).

**탐지 방법**:
- 파서 구조체에서 고정 길이가 명확한 필드(FourCC, GUID, 짧은 태그)가 `String`으로 선언되어 있는지 grep 및 리뷰.
- clippy `clippy::string_lit_as_bytes`류는 직접 해당하지 않지만, 구조체 필드 타입 리뷰를 코드 리뷰 체크리스트에 포함.

**예외**:
- 정말 가변 길이이고 UTF-8 검증/정규화가 필요한 텍스트 필드(메타데이터 태그 값 등)는 `String`이 적절하다.

**Bitvue 판정**: N/A — crates/bitvue-formats/src/mp4.rs:71 `BoxHeader.box_type: [u8; 4]`로 FourCC가 이미 고정 배열로 표현되고, 사람이 읽는 문자열은 `box_type_str()`에서 필요 시점에만 생성(권장 패턴과 일치). mkv/ts 등 다른 컨테이너 파서는 개별 확인하지 못함.

---

### MEM-034: 대형 버퍼를 캡처한 클로저가 채널에 쌓임
**분류**: MEM · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn spawn_export_tasks(frames: Vec<YuvPlane>, tx: std::sync::mpsc::Sender<Box<dyn FnOnce() + Send>>) {
    for frame in frames {
        let task = move || {
            let rgba = yuv_to_rgba(&frame); // frame(수십 MB) 전체가 클로저에 캡처됨
            write_png(&rgba);
        };
        tx.send(Box::new(task)).unwrap(); // 워커가 처리하기 전까지 채널 큐에 프레임이 통째로 쌓임
    }
}
```

**문제**:
- 클로저가 `frame`(대형 YUV 버퍼) 전체를 캡처(`move`)한 채 채널로 전송되면, 워커 스레드가 소비하기 전까지 큐에 쌓인 태스크 개수만큼 대형 버퍼가 동시에 메모리에 상주한다.
- 프로듀서(메인 스레드에서 export 태스크 생성)가 컨슈머(워커 풀)보다 빠르면 채널이 무한정 쌓이면서 백프레셔(backpressure) 없이 메모리가 폭증할 수 있다.
- 이 문제는 클로저 캡처가 "값처럼 보이지 않아서"(코드상 `frame`이라는 변수 하나만 보임) 리뷰에서 실제 캡처 크기를 놓치기 쉽다.

**발생 조건**:
- 익스포트/배치 처리(PNG 시퀀스 저장, 다중 프레임 품질 지표 계산 등)를 위해 프레임 단위 작업을 클로저로 만들어 워커 풀/채널에 던지는 구조에서 전형적으로 나타난다.
- 워커 수가 제한적인데 생산 속도가 그보다 빠른 배치 작업(대량 프레임 일괄 익스포트)에서 큐 적체가 심해진다.

**권장**:
```rust
use std::sync::mpsc::sync_channel;

fn spawn_export_tasks(frames: Vec<YuvPlane>) {
    let (tx, rx) = sync_channel::<YuvPlane>(4); // bounded 채널로 백프레셔 확보(최대 4개까지만 대기)
    let worker = std::thread::spawn(move || {
        while let Ok(frame) = rx.recv() {
            let rgba = yuv_to_rgba(&frame);
            write_png(&rgba);
        }
    });
    for frame in frames {
        tx.send(frame).unwrap(); // 큐가 4개 차면 send가 블록되어 프로듀서 속도를 자동 조절
    }
    drop(tx);
    worker.join().unwrap();
}
```
- 무제한 `mpsc::channel` 대신 `sync_channel(N)`처럼 용량이 제한된(bounded) 채널을 사용해, 컨슈머가 못 따라가면 프로듀서가 자동으로 대기하도록 백프레셔를 건다.
- 클로저에 대형 값을 캡처하는 대신, 값 자체(또는 그 핸들)를 채널 메시지로 직접 보내고 워커가 처리 로직을 갖도록 구조를 분리하면 캡처 크기를 명시적으로 통제하기 쉬워진다.

**탐지 방법**:
- `Box<dyn FnOnce() + Send>` 같은 타입 소거된 클로저를 채널로 전송하는 코드에서, 클로저가 `move`로 어떤 크기의 값을 캡처하는지 리뷰(Structural).
- 무제한 `mpsc::channel`(bounded가 아닌)이 대형 페이로드와 함께 쓰이는 지점을 grep하고, 프로듀서/컨슈머 속도 불균형 가능성을 점검.
- 배치 익스포트 실행 중 RSS가 처리된 프레임 수가 아니라 "아직 큐에 남은" 프레임 수에 비례해 증가하는지 runtime에서 관찰.

**예외**:
- 태스크 수가 적고(수십 개 이하) 페이로드가 작다면 무제한 채널이라도 실질적 위험이 없다.

**Bitvue 판정**: N/A — 워크스페이스 전체에서 `mpsc`/`crossbeam channel`/`tokio::sync::mpsc` 등 채널 사용 자체가 전혀 발견되지 않아, 클로저를 채널에 태워 워커 풀로 넘기는 구조(배치 익스포트 등)가 아직 존재하지 않음.

---

### MEM-035: 에러 경로에서 대형 버퍼가 해제 시점을 넘겨 유지됨
**분류**: MEM · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
struct DecodeContext {
    scratch: Vec<u8>,           // 디코드 중간 계산용 대형 버퍼
    last_error_frame: Option<YuvPlane>, // 디버깅을 위해 에러 발생 시 프레임 보관
}

fn decode_frame(ctx: &mut DecodeContext, packet: &[u8]) -> Result<YuvPlane, DecodeError> {
    let frame = run_decoder(&packet, &mut ctx.scratch)?;
    if let Err(e) = validate_frame(&frame) {
        ctx.last_error_frame = Some(frame.clone()); // 에러마다 프레임 스냅샷을 누적 보관
        return Err(DecodeError::Validation(e));
    }
    Ok(frame)
}
```

**문제**:
- "디버깅에 도움이 되도록" 에러 발생 시 프레임을 스냅샷으로 남기는 습관이 있으면, 에러가 반복되는 손상된 스트림을 분석할 때 `last_error_frame`이 매번 새 프레임으로 덮어써지긴 하지만 그 직전 값이 즉시 drop되지 않고 다음 에러까지 살아있어(특히 에러가 연속으로 발생하는 구간에서) 순간적으로 두 개의 대형 프레임이 동시에 상주할 수 있다.
- 더 심각한 경우는 `last_error_frame`을 덮어쓰지 않고 `Vec<YuvPlane>`처럼 계속 누적하는 실수를 함께 저지르는 경우로, 손상된 스트림을 분석하는 동안(에러가 수백 번 반복될 수 있음) 메모리가 무한정 증가한다.
- 에러 경로는 정상 경로보다 테스트가 부족한 경우가 많아, 이런 누적/과다 보관이 코드 리뷰나 일반 테스트에서 발견되지 않고 "손상된 실제 파일을 열었을 때만" 드러난다.

**발생 조건**:
- 손상되었거나 표준을 벗어난(비표준 인코더가 만든) 실제 파일을 분석할 때, 정상 스트림에서는 절대 타지 않는 에러 경로가 대량으로 반복 실행되면서 문제가 드러난다.
- 에러 리포팅/재현을 위해 "실패 시점의 상태를 최대한 많이 남기자"는 방어적 디버깅 습관이 상한 없이 적용되었을 때.

**권장**:
```rust
struct DecodeContext {
    scratch: Vec<u8>,
    last_error_summary: Option<ErrorFrameSummary>, // 전체 프레임이 아니라 요약 정보만 보관
}

struct ErrorFrameSummary {
    frame_index: u64,
    error_kind: String,
    checksum: u32, // 필요하면 프레임의 checksum/hash 정도만 남겨 재현에 활용
}

fn decode_frame(ctx: &mut DecodeContext, packet: &[u8]) -> Result<YuvPlane, DecodeError> {
    let frame = run_decoder(packet, &mut ctx.scratch)?;
    if let Err(e) = validate_frame(&frame) {
        ctx.last_error_summary = Some(ErrorFrameSummary {
            frame_index: frame.index,
            error_kind: format!("{e:?}"),
            checksum: compute_checksum(&frame),
        });
        return Err(DecodeError::Validation(e));
    }
    Ok(frame)
}
```
- 에러 시 "전체 프레임"이 아니라 재현/디버깅에 실질적으로 필요한 최소 정보(인덱스, 체크섬, 에러 종류)만 남긴다.
- 정말 전체 프레임 덤프가 필요하다면(심층 디버그 모드) 개수 상한을 명시적으로 두고(예: 최근 3개까지만), 그마저도 opt-in 플래그로만 활성화한다.
- 에러 경로도 정상 경로와 동일한 수준으로 메모리 동작을 리뷰 대상에 포함시킨다.

**탐지 방법**:
- `Result::Err` 분기 안에서 대형 타입(`YuvPlane`, `Vec<u8>` 등)을 `clone()`하거나 필드에 대입해 보관하는 패턴을 grep(Manual/코드 리뷰 위주).
- 의도적으로 손상시킨 테스트 파일(비정상 NAL 길이, 잘못된 syntax 값 등)로 반복 에러를 유발하는 fuzz/스트레스 테스트를 runtime에 추가해 RSS 증가 여부를 관찰.

**예외**:
- 에러가 세션당 최대 한 번만 발생할 수 있는 구조(예: 파일 열기 실패로 즉시 세션 자체가 종료됨)라면 상한을 두지 않아도 실질적 위험이 없다.

**Bitvue 판정**: N/A — 에러 발생 시 프레임 스냅샷을 `last_error_frame` 등으로 보관하는 패턴은 grep으로 발견되지 않음.
