# Anti-Pattern Catalog — CACHE: 캐시와 인덱싱

이 문서는 Bitvue 안티패턴 카탈로그의 한 분류이며, 전체 목록은 `docs/anti-patterns/INDEX.md`(별도 작성)를 참고한다.

---

### CACHE-001: entry-count LRU
**분류**: CACHE · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct FrameCache {
    // "100개까지 보관" — 엔트리 크기는 전혀 고려하지 않음
    inner: LruCache<u64, Arc<DecodedFrame>>,
}

impl FrameCache {
    pub fn new() -> Self {
        Self { inner: LruCache::new(NonZeroUsize::new(100).unwrap()) }
    }

    pub fn put(&mut self, frame_idx: u64, frame: Arc<DecodedFrame>) {
        self.inner.put(frame_idx, frame);
    }
}
```

**문제**:
- 8K RGBA 프레임(약 132MB)과 320x180 썸네일(약 230KB)이 "엔트리 1개"로 동일하게 취급되어 실제 메모리 사용량이 예측 불가능하다.
- 해상도가 큰 스트림을 열면 동일한 `capacity=100` 설정이 수십 GB를 요구할 수 있고, 저해상도 스트림에서는 캐시가 조기에 무의미해진다(용량은 남는데 항목 수 제한에 걸림).
- 실제 RSS(메모리 사용량)와 캐시 설정값 사이의 상관관계가 없어 OOM 예측·튜닝이 불가능하다.

**발생 조건**:
- 여러 해상도/픽셀 포맷 스트림을 같은 캐시 인스턴스로 처리할 때.
- 디코드된 프레임처럼 항목 크기 편차가 큰(수 KB ~ 수백 MB) 데이터를 캐싱할 때.

**권장**:
```rust
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct ByteBudgetCache {
    inner: LruCache<u64, Arc<DecodedFrame>>,
    used_bytes: usize,
    budget_bytes: usize,
}

impl ByteBudgetCache {
    pub fn new(budget_bytes: usize) -> Self {
        // capacity는 "이론상 최대 엔트리 수"로만 두고, 실제 상한은 budget_bytes로 관리
        Self {
            inner: LruCache::new(NonZeroUsize::new(usize::MAX >> 1).unwrap()),
            used_bytes: 0,
            budget_bytes,
        }
    }

    pub fn put(&mut self, key: u64, frame: Arc<DecodedFrame>) {
        let cost = frame.byte_size();
        self.used_bytes += cost;
        if let Some(old) = self.inner.put(key, frame) {
            self.used_bytes -= old.byte_size();
        }
        while self.used_bytes > self.budget_bytes {
            if let Some((_, evicted)) = self.inner.pop_lru() {
                self.used_bytes -= evicted.byte_size();
            } else {
                break;
            }
        }
    }
}
```
- 항목당 바이트 비용(`byte_size()`)을 계산해 총합이 예산을 넘으면 LRU 순서로 축출한다.
- 예산은 사용 가능한 시스템 메모리·해상도에 따라 런타임에 조정 가능해야 한다(CACHE-023 참조).

**탐지 방법**:
- Structural: `LruCache::new(NonZeroUsize::new(<literal>))` 패턴을 grep하고, 값에 해당하는 캐시가 가변 크기 페이로드(프레임/버퍼)를 저장하는지 타입 시그니처로 확인.
- Runtime: 캐시 적중 시 RSS 변화를 해상도별로 프로파일링해 편차가 크면 의심.

**예외**:
- 항목 크기가 사실상 고정인 캐시(예: 고정 크기 메타데이터 레코드, 파싱된 헤더 구조체)는 entry-count 방식이 적절하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-002: 압축 데이터와 decoded frame을 같은 캐시에 저장
**분류**: CACHE · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
enum CachedPayload {
    CompressedPacket(Bytes),        // 수 KB
    DecodedFrame(Arc<DecodedFrame>),// 수십~수백 MB
}

pub struct UnifiedCache {
    inner: LruCache<u64, CachedPayload>, // 동일한 LRU 큐, 동일한 축출 우선순위
}
```

**문제**:
- 압축 패킷(재취득 비용: 파일 I/O 1회, 저렴)과 디코드된 프레임(재취득 비용: 전체 GOP 재디코드, 비쌈)이 같은 축출 순서를 공유하여, "최근 사용" 기준만으로 비싼 디코드 결과가 먼저 밀려날 수 있다.
- 두 데이터의 크기 단위가 3~4자릿수 차이가 나기 때문에 CACHE-001의 문제가 더 심하게 겹친다.
- 캐시 하나의 락(Mutex/RwLock) 경합이 압축 패킷 읽기(자주, 짧게)와 프레임 디코드 결과 쓰기(드물게, 크게)를 하나의 임계구역으로 묶어 지연시간에 서로 영향을 준다.

**발생 조건**:
- "캐시는 하나로 통일하자"는 단순화 목적으로 enum 기반 통합 캐시를 설계했을 때.
- 압축 스트림 버퍼링과 디코드 결과 캐싱을 같은 모듈에서 처리하도록 리팩터링했을 때.

**권장**:
```rust
pub struct TieredCache {
    packets: LruCache<u64, Bytes>,               // 저비용 티어, 큰 capacity
    decoded: ByteBudgetCache,                     // 고비용 티어, byte-budget (CACHE-001)
}

impl TieredCache {
    pub fn get_decoded(&mut self, key: u64) -> Option<Arc<DecodedFrame>> {
        self.decoded.get(key)
    }

    pub fn get_packet(&mut self, key: u64) -> Option<Bytes> {
        self.packets.get(&key).cloned()
    }
}
```
- 재취득 비용(recompute cost)이 다른 데이터는 별도 캐시 인스턴스·별도 락으로 분리한다.
- 티어별로 축출 정책과 예산을 독립적으로 튜닝할 수 있게 한다(예: packets는 개수 기반, decoded는 바이트 기반).

**탐지 방법**:
- Structural: 캐시 value 타입이 enum이고 variant 간 크기 편차가 큰 경우를 코드 리뷰에서 표시.
- Manual: 캐시 설계 문서/PR에서 "unified cache", "single LRU for everything" 같은 표현 검색.

**예외**:
- 두 데이터의 재취득 비용과 크기가 실제로 비슷한 자릿수라면(예: 두 종류 모두 소형 메타데이터) 통합해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-003: cache key에 analysis option 누락
**분류**: CACHE · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
pub struct OverlayCache {
    inner: LruCache<u64 /* frame_idx */, Arc<RgbaImage>>,
}

// overlay_mode(MV/QP/PartitionGrid), deblock on/off, HDR tone-map 여부는
// 함수 인자로만 전달되고 key에는 반영되지 않는다.
pub fn render_overlay(cache: &mut OverlayCache, frame_idx: u64, opts: &OverlayOptions) -> Arc<RgbaImage> {
    if let Some(img) = cache.inner.get(&frame_idx) {
        return img.clone(); // opts가 바뀌어도 예전 렌더링 결과를 그대로 반환!
    }
    let img = Arc::new(render(frame_idx, opts));
    cache.inner.put(frame_idx, img.clone());
    img
}
```

**문제**:
- 사용자가 오버레이 모드를 MV → QP로 전환해도 캐시가 이전 렌더링 결과를 그대로 돌려줘 화면이 갱신되지 않는다.
- 버그가 "가끔" 재현되는 형태로 나타난다 — 캐시가 비어 있을 때는 정상 작동하다가, 같은 프레임을 재방문하면 잘못된 결과가 나온다.
- 리뷰에서 잡기 어렵다: 함수 시그니처에 `opts`가 있어 "옵션을 반영한다"는 인상을 주지만 실제로 key에는 안 들어간다.

**발생 조건**:
- 렌더링/분석 함수에 새로운 옵션 파라미터를 추가했지만 캐시 키 계산 로직을 갱신하지 않았을 때.
- 옵션 종류가 많아 key 구조체를 매번 손으로 확장해야 하는 설계일 때(휴먼 에러 유발).

**권장**:
```rust
#[derive(Hash, PartialEq, Eq, Clone)]
pub struct OverlayCacheKey {
    frame_idx: u64,
    opts_fingerprint: u64, // OverlayOptions 전체를 해시한 값
}

impl OverlayCacheKey {
    fn new(frame_idx: u64, opts: &OverlayOptions) -> Self {
        use std::hash::{Hash, Hasher};
        let mut h = rustc_hash::FxHasher::default();
        opts.hash(&mut h); // OverlayOptions는 #[derive(Hash)]로 모든 필드를 포함
        Self { frame_idx, opts_fingerprint: h.finish() }
    }
}

pub fn render_overlay(cache: &mut LruCache<OverlayCacheKey, Arc<RgbaImage>>,
                       frame_idx: u64, opts: &OverlayOptions) -> Arc<RgbaImage> {
    let key = OverlayCacheKey::new(frame_idx, opts);
    if let Some(img) = cache.get(&key) {
        return img.clone();
    }
    let img = Arc::new(render(frame_idx, opts));
    cache.put(key, img.clone());
    img
}
```
- 캐시 결과에 영향을 주는 모든 입력을 key 파생 대상에 포함시킨다. `#[derive(Hash)]`로 옵션 구조체 전체를 해시하면 필드 추가 시 자동으로 key에 반영되어 누락을 구조적으로 방지한다.
- 옵션 구조체에 새 필드를 추가할 때 `Hash` derive를 깨는 실수(예: 수동 `impl Hash`에서 필드 누락)를 CI에서 잡을 수 있도록 테스트를 둔다.

**탐지 방법**:
- Semantic: 캐시 `get`/`put` 호출부에서 사용된 key 표현식과 렌더링/계산 함수에 실제로 전달되는 인자 집합을 비교해 key에 없는 인자가 있는지 분석.
- Runtime: 옵션을 변경하며 같은 프레임을 반복 조회하는 회귀 테스트(골든 이미지 비교)를 추가.

**예외**:
- 옵션이 렌더링 결과에 전혀 영향을 주지 않는 순수 UI 힌트(예: 툴팁 표시 여부)라면 key에서 제외해도 안전하다 — 단, 이 판단은 명시적 주석으로 남겨야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-004: stale generation 결과 캐싱
**분류**: CACHE · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
pub struct AnalysisCache {
    inner: LruCache<u64, Arc<FrameMetrics>>,
}

// 파일을 다시 열거나(reload), 디코더 설정을 바꿔도 캐시는 그대로 재사용된다.
pub fn get_metrics(state: &mut AppState, frame_idx: u64) -> Arc<FrameMetrics> {
    if let Some(m) = state.cache.inner.get(&frame_idx) {
        return m.clone();
    }
    let m = Arc::new(compute_metrics(&state.decoder, frame_idx));
    state.cache.inner.put(frame_idx, m.clone());
    m
}
```

**문제**:
- 파일 reload, 디코더 파라미터 변경(예: 색공간 변환 방식 전환), 스트림 재파싱 이후에도 이전 "세대(generation)"의 계산 결과가 그대로 반환된다.
- 버그가 세션 내내 잠복하다가 특정 순서(파일 A 열기 → 옵션 변경 → 파일 A 다시 열기)에서만 터져 재현이 어렵다.

**발생 조건**:
- 파일을 닫지 않고 다시 로드(hot reload)하는 기능이 있는데 캐시를 명시적으로 비우지 않을 때.
- 전역 상태(디코더 설정, 색공간, HDR 메타데이터 해석 방식)가 바뀌는데 그 상태가 cache key에 반영되지 않을 때(CACHE-003과 연관되지만 여기서는 "파일/세션 단위" 세대 문제).

**권장**:
```rust
pub struct AnalysisCache {
    inner: LruCache<(u64 /* generation */, u64 /* frame_idx */), Arc<FrameMetrics>>,
    generation: u64,
}

impl AnalysisCache {
    /// 파일 reload, 디코더 설정 변경 등 "세대"가 바뀌는 모든 이벤트에서 호출
    pub fn bump_generation(&mut self) {
        self.generation += 1;
        // 굳이 clear()로 즉시 비우지 않아도 되지만(구세대 키는 자연히 LRU 밀림),
        // 메모리를 즉시 회수하고 싶다면 명시적으로 clear한다.
    }

    pub fn get(&mut self, frame_idx: u64) -> Option<Arc<FrameMetrics>> {
        self.inner.get(&(self.generation, frame_idx)).cloned()
    }

    pub fn put(&mut self, frame_idx: u64, metrics: Arc<FrameMetrics>) {
        self.inner.put((self.generation, frame_idx), metrics);
    }
}
```
- generation 카운터를 key에 포함시켜, 무효화가 필요한 이벤트에서 카운터만 증가시키면 이전 세대 항목은 자연스럽게 "미스"로 처리된다.
- 메모리 회수가 시급하면 `bump_generation` 직후 `retain`으로 구세대 항목만 선택적으로 제거하는 옵션도 둔다.

**탐지 방법**:
- Semantic: 파일 reload/설정 변경 경로를 추적해 캐시 무효화 호출(`bump_generation`, `clear`) 존재 여부를 확인.
- Runtime: "옵션 변경 → 같은 프레임 재조회" 시나리오를 자동화 테스트로 고정.

**예외**:
- 세션 동안 절대 재로드/설정 변경이 없는 단발성 CLI 도구라면 generation 개념이 불필요할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-005: 파일 변경 후 캐시 무효화 누락
**분류**: CACHE · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct FileFrameCache {
    path: PathBuf,
    inner: LruCache<u64, Arc<DecodedFrame>>,
}

// 파일이 외부에서 덮어써지거나(다른 인코더 도구가 같은 경로에 재출력),
// 사용자가 "다시 분석" 버튼을 눌러도 캐시는 mtime/hash를 확인하지 않는다.
pub fn get_frame(cache: &mut FileFrameCache, frame_idx: u64) -> Arc<DecodedFrame> {
    if let Some(f) = cache.inner.get(&frame_idx) {
        return f.clone();
    }
    decode_and_cache(cache, frame_idx)
}
```

**문제**:
- 파일이 외부 도구(FFmpeg 재인코딩, 다른 분석기)에 의해 같은 경로로 덮어써지면, 이미 캐시된 프레임은 새 파일 내용과 무관한 "유령 데이터"가 되어 사용자가 잘못된 분석 결과를 보게 된다.
- 오프셋 기반 인덱스(CACHE-016/017)까지 함께 캐시되어 있다면, 무효화 누락은 잘못된 데이터 반환을 넘어 파일 범위를 벗어난 읽기(out-of-bounds read)나 파싱 크래시로 이어질 수 있다.

**발생 조건**:
- 사용자가 파일을 외부에서 수정한 뒤 앱을 재로드하지 않고 그대로 스크럽할 때.
- 워치 모드(파일 변경 감시)가 있는데 변경 이벤트와 캐시 무효화가 배선되어 있지 않을 때.

**권장**:
```rust
pub struct FileFrameCache {
    path: PathBuf,
    file_fingerprint: (u64 /* mtime_nanos */, u64 /* len */),
    inner: LruCache<u64, Arc<DecodedFrame>>,
}

impl FileFrameCache {
    pub fn ensure_fresh(&mut self) -> std::io::Result<()> {
        let meta = std::fs::metadata(&self.path)?;
        let fp = (meta.modified()?.duration_since(std::time::UNIX_EPOCH)
                      .unwrap().as_nanos() as u64, meta.len());
        if fp != self.file_fingerprint {
            self.inner.clear();
            self.file_fingerprint = fp;
        }
        Ok(())
    }
}

pub fn get_frame(cache: &mut FileFrameCache, frame_idx: u64) -> std::io::Result<Arc<DecodedFrame>> {
    cache.ensure_fresh()?; // 매 조회 전 값싼 stat() 호출로 신선도 확인
    Ok(cache.inner.get(&frame_idx).cloned().unwrap_or_else(|| decode_and_cache(cache, frame_idx)))
}
```
- mtime+size(또는 더 견고하게는 파일 앞부분 해시)로 지문을 만들어 매 접근 전 값싸게 검증한다.
- 워치 모드가 있다면 `notify` 크레이트 이벤트 콜백에서 즉시 `inner.clear()`를 호출해 stat 폴링 지연조차 없앤다.

**탐지 방법**:
- Runtime: 테스트에서 파일을 캐시 워밍업 후 다른 내용으로 덮어쓰고, 이전 캐시가 반환되는지 확인하는 회귀 테스트.
- Manual: 파일 handle을 open 시점에 한 번만 얻고 이후 재검증 로직이 없는 캐시 구조를 코드 리뷰에서 표시.

**예외**:
- 앱이 파일을 열 때 배타적 락을 걸어 외부 수정이 원천적으로 불가능한 설계라면(read-only 마운트, 임시 복사본 사용) 무효화 검사를 생략해도 안전 — 단 이 가정을 코드에 명시해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-006: timestamp만 cache key로 사용
**분류**: CACHE · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub struct PtsCache {
    // PTS(f64, 초 단위)를 그대로 key로 사용
    inner: HashMap<u64 /* f64.to_bits() */, Arc<DecodedFrame>>,
}

pub fn cache_by_pts(cache: &mut PtsCache, pts_seconds: f64, frame: Arc<DecodedFrame>) {
    cache.inner.insert(pts_seconds.to_bits(), frame);
}
```

**문제**:
- 부동소수점 PTS는 컨테이너 timebase 변환·반올림 오차로 동일 프레임이 매번 미세하게 다른 비트 패턴을 가질 수 있어(`to_bits()` 비교는 `==` 의미론), 사실상 같은 프레임인데 캐시 미스가 반복된다.
- timestamp는 프레임의 "고유성"을 보장하지 않는다(CACHE-007) — 신뢰할 수 없는 identity를 key로 쓰는 것 자체가 근본 문제다.
- 디코드 순서(decode order)와 표시 순서(display order)가 다른 B-frame 스트림에서는 timestamp만으로 디코더 내부 프레임 버퍼와 매핑하기 어렵다(CACHE-008과 연관).

**발생 조건**:
- 컨테이너 depacketizing 단계에서 timebase 변환을 거친 PTS를 그대로 키로 재사용할 때.
- 여러 트랙/스트림이 합쳐진 타임라인에서 프레임을 timestamp만으로 식별하려 할 때.

**권장**:
```rust
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct FrameKey {
    stream_id: u32,
    decode_index: u64, // 디코더가 부여하는 단조 증가 정수 — 부동소수점 비교 문제 없음
}

pub struct FrameCache {
    inner: LruCache<FrameKey, Arc<DecodedFrame>>,
}
```
- 정수 기반의 단조 증가 인덱스(디코드 순서 인덱스, 또는 컨테이너가 제공하는 sample index)를 identity로 쓰고, timestamp는 표시/탐색용 메타데이터로만 별도 보관한다.
- 부동소수점을 key로 써야 하는 불가피한 경우, timebase 단위의 정수(예: `(pts_num, pts_den)` 유리수 또는 timescale 기준 정수 tick)로 변환해 사용한다.

**탐지 방법**:
- Semantic: 캐시 key 타입이 `f64`/`f32` 또는 이를 감싼 wrapper인지 타입 검사로 탐지.
- Static: `.to_bits()`를 해시/키 용도로 사용하는 패턴 grep.

**예외**:
- 외부 API 경계에서 timestamp가 유일한 식별자로 주어지고, 그 API가 timestamp 유일성을 명세로 보장하는 경우(예: 컨테이너 스펙상 PTS 유일성이 강제됨)라면 허용 가능 — 단 문서화 필요.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-007: 동일 timestamp가 가능한 stream 고려 없음
**분류**: CACHE · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn cache_key_for(stream: &Stream, pts: i64) -> u64 {
    // 여러 스트림(멀티 트랙, 또는 seek 후 재시작된 서브스트림)이
    // 같은 pts 값을 가질 수 있다는 사실을 무시
    pts as u64
}
```

**문제**:
- 멀티 트랙 컨테이너(예: MKV의 여러 비디오 트랙, 또는 스티치된 클립)에서 서로 다른 트랙이 동일 PTS를 갖는 경우 캐시 항목이 서로를 덮어쓴다.
- seek 후 디코더가 재시작되며 PTS 카운터가 리셋되거나 wrap-around되는 스트림(일부 저사양 인코더, 손상된 스트림)에서는 세션 내에서도 중복이 발생한다.
- 결함이 "특정 파일에서만" 나타나 재현·디버깅 비용이 크다.

**발생 조건**:
- 여러 스트림/트랙을 동시에 열람하는 멀티뷰 UI.
- 손상되었거나 표준을 엄밀히 따르지 않는 스트림을 다루는 관용적 파서(permissive parser).

**권장**:
```rust
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct FrameKey {
    stream_id: u32,
    seek_epoch: u32, // seek/재시작마다 증가하는 카운터 — PTS 리셋/wrap 대응
    pts: i64,
}
```
- key에 `stream_id`와 "이 PTS가 속한 seek epoch"을 함께 포함해 재시작 후 리셋된 PTS와 이전 세션의 PTS를 구분한다.
- 가능하면 CACHE-006 권장안처럼 PTS 대신 단조 증가 decode_index를 primary identity로 사용하고 PTS는 보조 정보로만 취급한다.

**탐지 방법**:
- Semantic: 멀티 스트림/멀티 트랙을 지원하는 코드 경로에서 캐시 key에 `stream_id`(또는 track id)가 포함되어 있는지 확인.
- Manual: 손상 스트림 fuzzing 테스트에서 PTS 중복/리셋 케이스를 재생해 캐시 오염 여부 관찰.

**예외**:
- 단일 스트림, 단일 세션, PTS 단조 증가가 파서 레벨에서 이미 보장되는 좁은 시나리오라면 생략 가능.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-008: frame index와 display index 혼용
**분류**: CACHE · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
// 디코더는 decode order로 프레임을 내보내지만(IPBBP 스트림에서 B가 뒤로 밀림),
// UI 타임라인은 display order(표시 순서)로 프레임을 요청한다.
pub fn get_frame_for_timeline(cache: &mut LruCache<u64, Arc<DecodedFrame>>, ui_index: u64) -> Arc<DecodedFrame> {
    // ui_index(=display index)를 그대로 decode 결과 캐시의 key로 사용
    cache.get(&ui_index).cloned().unwrap_or_else(|| decode_next_frame(ui_index))
}
```

**문제**:
- B-frame이 존재하는 GOP 구조(예: IBBPBBP)에서 decode order와 display order가 어긋나므로, index를 혼용하면 사용자가 요청한 화면 위치와 실제로 캐시에서 반환되는 프레임이 어긋난다.
- 오버레이(모션 벡터, QP 맵 등)가 잘못된 프레임에 그려져 분석 결과 자체의 신뢰성이 깨진다 — 단순 성능 버그가 아니라 정합성(correctness) 버그다.
- 재현이 간헐적이다: All-Intra나 저지연(B-frame 없음) 프로파일에서는 두 순서가 우연히 일치해 버그가 숨어 있다가, B-frame이 있는 일반 프로파일 스트림을 열면 드러난다.

**발생 조건**:
- B-frame을 사용하는 표준 GOP 구조 스트림(HEVC/AVC/AV1 등)을 다룰 때.
- 디코더 레이어와 UI 타임라인 레이어 사이에 "index"라는 이름만 같고 의미가 다른 값이 별도 변환 없이 넘나들 때.

**권장**:
```rust
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct DecodeIndex(u64);
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct DisplayIndex(u64);

pub struct IndexMap {
    display_to_decode: Vec<DecodeIndex>, // GOP 파싱 시점에 미리 구축
}

impl IndexMap {
    pub fn resolve(&self, ui_index: DisplayIndex) -> DecodeIndex {
        self.display_to_decode[ui_index.0 as usize]
    }
}

pub fn get_frame_for_timeline(
    cache: &mut LruCache<DecodeIndex, Arc<DecodedFrame>>,
    index_map: &IndexMap,
    ui_index: DisplayIndex,
) -> Arc<DecodedFrame> {
    let decode_idx = index_map.resolve(ui_index);
    cache.get(&decode_idx).cloned().unwrap_or_else(|| decode_and_cache(decode_idx))
}
```
- 두 인덱스 공간을 타입 레벨(newtype)로 분리해 컴파일러가 혼용을 막도록 한다.
- 변환은 반드시 명시적 `IndexMap`(또는 이에 준하는 GOP 구조 기반 매핑 테이블)을 거치도록 강제한다.

**탐지 방법**:
- Static/Structural: 서로 다른 의미의 index가 동일한 원시 타입(`u64`)으로 함수 경계를 넘나드는지 타입 시그니처 검사; newtype 미사용 시 경고.
- Runtime: B-frame이 포함된 골든 스트림에서 오버레이 렌더링 결과를 프레임 단위로 비교하는 회귀 테스트.

**예외**:
- All-Intra 전용 스트림만 지원하는 모듈(예: 정지 이미지 시퀀스 분석기)이라면 decode order == display order가 항상 성립하므로 구분이 불필요할 수 있다 — 단, 다른 코덱 지원이 추가될 가능성이 있다면 애초에 분리해두는 편이 안전하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-009: 캐시 안에 Arc cycle 생성
**분류**: CACHE · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
pub struct DecodedFrame {
    data: Vec<u8>,
    // 프레임이 자신을 캐싱한 캐시를 다시 참조 (예: "이 프레임을 무효화하는 콜백"을 위해)
    owner_cache: Arc<Mutex<FrameCache>>,
}

pub struct FrameCache {
    inner: LruCache<u64, Arc<DecodedFrame>>,
}
```

**문제**:
- `FrameCache` → `Arc<DecodedFrame>` → `owner_cache: Arc<Mutex<FrameCache>>` 로 이어지는 강한 참조 순환이 생겨, `Arc`의 참조 카운트가 0이 되지 않고 두 객체 모두 영원히 해제되지 않는다.
- `lru`의 `pop_lru()`/`clear()`로 캐시에서 항목을 제거해도, 프레임이 여전히 캐시를 가리키고 있어 실제 메모리(수백 MB 단위 디코드 버퍼)가 회수되지 않는 조용한 메모리 누수가 발생한다.
- 누수가 크래시 없이 서서히 RSS를 늘리기 때문에 장시간 세션(파일을 여러 개 연속으로 열람)에서야 증상이 드러난다.

**발생 조건**:
- 프레임/캐시 엔트리가 "자신을 무효화해달라"고 부모 캐시에 콜백을 걸기 위해 역참조를 들고 있을 때.
- 캐시 엔트리가 통계 수집기, 이벤트 버스 등 상위 컨테이너를 강하게 참조할 때.

**권장**:
```rust
use std::sync::Weak;

pub struct DecodedFrame {
    data: Vec<u8>,
    owner_cache: Weak<Mutex<FrameCache>>, // 약한 참조로 순환 차단
}

impl DecodedFrame {
    pub fn invalidate_self(&self, key: u64) {
        if let Some(cache) = self.owner_cache.upgrade() {
            cache.lock().unwrap().inner.pop(&key);
        }
        // upgrade() 실패 시(캐시가 이미 drop됨) 조용히 무시 — 정상 상황
    }
}
```
- 자식(캐시 엔트리)이 부모(캐시)를 가리켜야 할 때는 항상 `Weak`를 사용한다. 부모→자식(`inner: LruCache<K, Arc<V>>`)만 강한 참조로 유지한다.
- 순환 가능성이 있는 구조는 설계 단계에서 소유권 방향을 다이어그램으로 명시해두면 리뷰에서 걸러내기 쉽다.

**탐지 방법**:
- Structural: 캐시 value 타입 정의를 재귀적으로 펼쳐 자기 자신(또는 캐시 컨테이너 타입)을 강한 참조로 포함하는지 정적 분석.
- Runtime: `Arc::strong_count`를 주기적으로 로깅하거나, `valgrind`/`heaptrack`으로 장시간 세션 후 미회수 할당을 확인.

**예외**:
- 순환이 있더라도 프로세스 생명주기 동안 단 한 번만 생성되고 프로세스 종료와 함께 해제되면 충분한 전역 싱글턴 구조라면(실질적 누수가 문제되지 않음) 허용 가능 — 단 명시적 주석 필요.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-010: 캐시 eviction 중 UI thread 정지
**분류**: CACHE · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
#[tauri::command]
fn get_frame_thumbnail(state: tauri::State<AppState>, frame_idx: u64) -> Vec<u8> {
    // Tauri command 핸들러(기본적으로 메인/이벤트 루프 스레드에서 동기 실행)에서
    // 캐시 put()을 직접 호출 — eviction 시 GPU 텍스처 해제, 락 경합 등 비용 발생
    let mut cache = state.thumbnail_cache.lock().unwrap();
    cache.put(frame_idx, generate_thumbnail(frame_idx)); // put()이 pop_lru()를 유발할 수 있음
    cache.get(&frame_idx).unwrap().clone()
}
```

**문제**:
- `LruCache::put()`이 용량 초과로 `pop_lru()`를 트리거하면, 축출되는 항목의 `Drop`(GPU 텍스처 해제, 파일 핸들 정리 등)이 호출부와 같은 스레드에서 동기적으로 실행된다.
- 이 호출부가 UI 이벤트 루프/Tauri command 핸들러라면, 무거운 drop 하나가 프레임 스크럽 중 입력 처리를 수십~수백 ms 멈추게 해 "끊김"으로 체감된다.
- `Mutex` 락을 쥔 채로 비싼 drop이 실행되므로, 같은 락을 기다리는 다른 스레드(백그라운드 디코더 등)까지 연쇄적으로 지연된다.

**발생 조건**:
- 캐시가 GPU 리소스, mmap, 대형 힙 버퍼처럼 drop 비용이 큰 타입을 보관할 때.
- 캐시 접근이 UI 스레드에서 동기 호출로 이루어지는 아키텍처(Tauri command가 기본적으로 이렇게 동작하기 쉽다)일 때.

**권장**:
```rust
#[tauri::command]
async fn get_frame_thumbnail(state: tauri::State<'_, AppState>, frame_idx: u64) -> Result<Vec<u8>, String> {
    let cache = state.thumbnail_cache.clone(); // Arc<Mutex<_>> clone, 값 자체는 아님
    // 캐시 조작(잠재적 eviction 포함)을 블로킹 스레드 풀로 이전
    tauri::async_runtime::spawn_blocking(move || {
        let mut cache = cache.lock().unwrap();
        cache.put(frame_idx, generate_thumbnail(frame_idx));
        cache.get(&frame_idx).unwrap().clone()
    })
    .await
    .map_err(|e| e.to_string())
}
```
- eviction이 발생할 수 있는 캐시 연산은 `spawn_blocking` 또는 전용 워커 스레드로 옮겨 UI 이벤트 루프를 절대 막지 않는다.
- 축출된 값을 즉시 drop하지 않고 채널로 다른(더 한가한) 스레드에 넘겨 지연 drop하는 방식(CACHE-011 권장안)과 결합하면 효과가 배가된다.

**탐지 방법**:
- Runtime: UI 스레드 프레임 타임(예: Tauri 이벤트 루프 tick 간격)을 계측해 캐시 put 호출과 스파이크의 상관관계 확인.
- Structural: `#[tauri::command]` 함수(비-`async`) 내부에서 `LruCache::put`/`pop_lru`를 직접 호출하는 패턴 검색.

**예외**:
- 캐시 value의 drop 비용이 무시할 수준(단순 `Vec<u8>` 등, 대형 GPU 리소스 아님)이면 동기 호출도 허용 가능. 다만 이 경우도 값 크기가 커지면 재검토가 필요하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-011: 대형 객체 drop이 한 번에 발생
**분류**: CACHE · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn on_file_closed(state: &mut AppState) {
    // 수백~수천 개의 대형 디코드 프레임을 한 함수 호출에서 모두 drop
    state.frame_cache.inner.clear(); // 내부적으로 모든 엔트리를 순차 drop
}
```

**문제**:
- `clear()` 한 번의 호출 안에서 수 GB 분량의 버퍼가 연속으로 `free()`되며, 힙 할당자 락 경합과 페이지 반환(`madvise`/`munmap`) 비용이 한 스레드에 몰려 눈에 띄는 정지(수십~수백 ms)를 유발한다.
- 이 정지가 "파일 닫기"라는, 사용자가 즉시 반응을 기대하는 조작 직후에 발생해 체감 품질을 떨어뜨린다.
- GC가 없는 Rust에서는 이런 비용이 명시적 drop 호출 지점에 온전히 몰리므로, 다른 언어보다 이 패턴이 더 두드러지게 나타난다.

**발생 조건**:
- 파일 닫기, 세션 종료, 캐시 강제 초기화(설정 변경으로 인한 `clear()`) 등 "한 번에 다 비우기"가 필요한 이벤트에서.
- 캐시 항목 크기가 크고 개수가 많을 때(고해상도 프레임 캐시).

**권장**:
```rust
pub fn on_file_closed(state: &mut AppState) {
    // 캐시 내용물의 소유권만 넘기고, 실제 drop은 백그라운드 스레드에서 분산 수행
    let old_entries: Vec<Arc<DecodedFrame>> = state.frame_cache.take_all();
    std::thread::spawn(move || {
        for entry in old_entries {
            drop(entry); // 필요하면 sleep/yield로 페이싱 조절 가능
        }
    });
    state.frame_cache = FrameCache::new(state.frame_cache.budget_bytes());
}
```
- `clear()`가 즉시 모든 것을 동기적으로 drop하는 대신, 엔트리 소유권을 벡터로 옮겨 백그라운드 스레드에 넘기고 그 스레드에서 점진적으로(필요하면 몇 개씩 배치+짧은 yield) 해제한다.
- 애초에 캐시를 나눠 관리해(작은 캐시 여러 개) 한 번에 비워야 하는 범위를 줄이는 것도 완화책이다.

**탐지 방법**:
- Runtime: 파일 닫기/캐시 clear 시점의 프레임 타임 스파이크를 프로파일러(예: `tracing` span + flamegraph)로 확인.
- Manual: `LruCache::clear()` 호출부를 검색해 호출 스레드가 UI/메인 스레드인지 확인.

**예외**:
- 프로세스 자체가 종료되는 경로(앱 quit)에서는 OS가 프로세스 전체 메모리를 일괄 회수하므로 명시적 drop 페이싱이 불필요하다 — `std::process::exit()` 등으로 우회 가능.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-012: 썸네일과 원본 RGBA 중복 저장
**분류**: CACHE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct FrameEntry {
    full_rgba: Arc<Vec<u8>>,       // 예: 3840x2160x4 bytes ≈ 33MB
    thumbnail_rgba: Arc<Vec<u8>>,  // 리사이즈 없이 동일 해상도로 재생성/재저장한 사본
}
```

**문제**:
- "썸네일"이라는 이름과 달리 다운스케일 없이 원본과 동일하거나 유사한 해상도로 별도 저장되어, 같은 시각적 정보를 두 배 가까운 메모리로 들고 있게 된다.
- 필름스트립(다수의 미니어처를 한 화면에 나열하는 UI)처럼 동시에 많은 프레임의 썸네일이 필요한 화면에서 이 중복이 누적되어 캐시 예산을 빠르게 소진시킨다.
- 원본이 캐시에서 축출된 뒤에도 썸네일은 독립 엔트리라 남아있을 수 있어, "썸네일은 원본의 파생물"이라는 의도된 관계가 캐시 수명 관리에 반영되지 않는다.

**발생 조건**:
- 원본 디코드 파이프라인에서 썸네일도 "일단 같은 파이프라인으로" 생성해 별도 최적화 없이 캐시에 넣을 때.
- 필름스트립/타임라인 미리보기처럼 다수의 저해상도 이미지가 동시에 필요한 UI 기능을 나중에 추가했을 때.

**권장**:
```rust
pub struct FrameEntry {
    full_rgba: Arc<Vec<u8>>, // 필요 시에만 캐시에 존재 (별도 예산/티어)
}

pub fn get_thumbnail(cache: &mut FrameCache, thumb_cache: &mut LruCache<u64, Arc<Vec<u8>>>,
                      frame_idx: u64, thumb_size: (u32, u32)) -> Arc<Vec<u8>> {
    if let Some(t) = thumb_cache.get(&frame_idx) {
        return t.clone();
    }
    // 원본이 캐시에 있으면 그것을 다운스케일, 없으면 디코더에 저해상도 디코드를 직접 요청
    // (가능한 코덱/디코더는 저해상도 전용 디코드 경로를 지원 — 전체 해상도 디코드보다 훨씬 저렴)
    let small = match cache.inner.get(&frame_idx) {
        Some(full) => downscale(full, thumb_size),
        None => decode_low_res(frame_idx, thumb_size),
    };
    let small = Arc::new(small);
    thumb_cache.put(frame_idx, small.clone());
    small
}
```
- 썸네일은 실제로 축소된 별도 크기 버퍼로만 저장하고, 원본과 독립된 작은 예산의 캐시 티어에 둔다(CACHE-002의 티어 분리 원칙 재사용).
- 가능하다면 디코더의 저해상도 디코드 경로(예: 낮은 해상도로 직접 디코드하는 fast-path)를 활용해 "원본을 다운스케일"하는 것보다 더 저렴하게 썸네일을 만든다.

**탐지 방법**:
- Structural: 썸네일 버퍼와 원본 버퍼의 실제 바이트 크기를 런타임에 비교해 유사하면 경고.
- Manual: `thumbnail`이라는 이름의 필드/함수가 실제로 리사이즈 로직을 거치는지 코드 리뷰에서 확인.

**예외**:
- 원본 자체가 이미 매우 작은 해상도(예: 480p 이하)라면 "썸네일 = 원본과 거의 같은 크기"가 자연스러운 결과이며 별도 다운스케일이 오히려 불필요한 연산이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-013: 실패 결과를 캐싱하지 않아 반복 실패
**분류**: CACHE · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn get_frame(cache: &mut LruCache<u64, Arc<DecodedFrame>>, frame_idx: u64) -> Option<Arc<DecodedFrame>> {
    if let Some(f) = cache.get(&frame_idx) {
        return Some(f.clone());
    }
    match decode(frame_idx) {
        Ok(f) => {
            let f = Arc::new(f);
            cache.put(frame_idx, f.clone());
            Some(f)
        }
        Err(_) => None, // 실패는 캐시에 남기지 않음 — 다음 조회에서 다시 디코드 시도
    }
}
```

**문제**:
- 손상된 프레임 하나가 있으면, 사용자가 그 지점 근처를 스크럽할 때마다(타임라인 hover, 필름스트립 재렌더 등 UI가 같은 프레임을 여러 번 요청하는 경우가 흔함) 매번 동일한 실패 디코드를 재시도해 CPU를 낭비한다.
- 실패한 디코드가 예외적으로 비용이 큰 경로(예: 손상 지점 근처에서 에러 복구를 여러 스텝 시도하는 방어적 파서)라면 반복 비용이 누적되어 체감 성능 저하로 이어진다.
- 로그가 있다면 동일 에러가 반복 출력되어 실제 문제 진단을 방해한다.

**발생 조건**:
- 동일 프레임에 대한 조회가 짧은 시간에 여러 번 발생하는 UI 패턴(스크럽, 반복 렌더)과 결합될 때.
- 손상/미지원 스트림을 다루는 관용적 파서에서 에러가 드물지 않게 발생할 때.

**권장**:
```rust
pub enum CacheEntry {
    Ok(Arc<DecodedFrame>),
    Failed(DecodeError), // negative cache: 실패도 하나의 캐시 가능한 결과로 취급
}

pub fn get_frame(cache: &mut LruCache<u64, CacheEntry>, frame_idx: u64) -> Result<Arc<DecodedFrame>, DecodeError> {
    if let Some(entry) = cache.get(&frame_idx) {
        return match entry {
            CacheEntry::Ok(f) => Ok(f.clone()),
            CacheEntry::Failed(e) => Err(e.clone()), // 재시도 없이 즉시 반환
        };
    }
    match decode(frame_idx) {
        Ok(f) => {
            let f = Arc::new(f);
            cache.put(frame_idx, CacheEntry::Ok(f.clone()));
            Ok(f)
        }
        Err(e) => {
            cache.put(frame_idx, CacheEntry::Failed(e.clone()));
            Err(e)
        }
    }
}
```
- 실패 결과도 캐시 엔트리로 저장하는 negative caching을 적용해 반복 실패로 인한 재계산을 막는다.
- 단, 모든 실패를 무조건 영구 캐싱하면 CACHE-014 문제가 생기므로 반드시 에러 종류를 구분해야 한다.

**탐지 방법**:
- Runtime: 동일 실패 프레임을 반복 조회하는 부하 테스트에서 CPU 사용량/디코드 호출 횟수를 계측.
- Manual: `Err`/`None` 분기에서 캐시에 아무것도 기록하지 않는 패턴을 코드 리뷰에서 표시.

**예외**:
- 실패가 극히 드물고(스트림이 대부분 건강함) 재시도 비용이 무시할 수준이라면 negative caching의 복잡도를 감수하지 않아도 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-014: 영구 오류와 일시 오류를 동일하게 negative cache
**분류**: CACHE · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
pub fn get_frame(cache: &mut LruCache<u64, CacheEntry>, frame_idx: u64) -> Result<Arc<DecodedFrame>, DecodeError> {
    if let Some(CacheEntry::Failed(e)) = cache.get(&frame_idx) {
        return Err(e.clone()); // 에러 종류 구분 없이 무조건 영구 캐싱된 실패를 반환
    }
    match decode(frame_idx) {
        Ok(f) => { /* ... */ Ok(Arc::new(f)) }
        Err(e) => {
            // "파일이 잠깐 다른 프로세스에 의해 잠김(일시적)"이든
            // "비트스트림이 근본적으로 손상됨(영구적)"이든 구분 없이 동일하게 캐싱
            cache.put(frame_idx, CacheEntry::Failed(e.clone()));
            Err(e)
        }
    }
}
```

**문제**:
- `IoError::WouldBlock`, 일시적 리소스 부족(`ResourceTemporarilyUnavailable`), 네트워크 스트리밍 중 순간적 끊김처럼 재시도하면 성공할 수 있는 일시적 오류가, 손상된 비트스트림처럼 영구적으로 실패하는 오류와 동일하게 "영원히" negative cache에 박제된다.
- 원인이 사라진(파일 잠금 해제, 네트워크 회복) 이후에도 캐시가 과거 실패를 계속 반환해 사용자가 "고쳤는데도 여전히 안 된다"고 오인하게 만든다.
- CACHE-013의 해결책(negative caching)을 도입하면서 이 구분을 빼먹으면 오히려 새로운 버그를 만든 셈이 된다.

**발생 조건**:
- negative caching을 도입하되 에러 타입을 세분화하지 않고 뭉뚱그려 처리할 때.
- 네트워크 스토리지, 외부 프로세스가 잠근 파일 등 일시적 실패가 실제로 발생 가능한 I/O 계층 위에 캐시를 얹을 때.

**권장**:
```rust
pub enum CacheEntry {
    Ok(Arc<DecodedFrame>),
    PermanentFailure(DecodeError),                 // 영구: TTL 없이 캐싱
    TransientFailure(DecodeError, Instant),         // 일시: 짧은 TTL 후 재시도 허용
}

const TRANSIENT_RETRY_AFTER: Duration = Duration::from_millis(500);

pub fn get_frame(cache: &mut LruCache<u64, CacheEntry>, frame_idx: u64) -> Result<Arc<DecodedFrame>, DecodeError> {
    match cache.get(&frame_idx) {
        Some(CacheEntry::Ok(f)) => return Ok(f.clone()),
        Some(CacheEntry::PermanentFailure(e)) => return Err(e.clone()),
        Some(CacheEntry::TransientFailure(e, since)) if since.elapsed() < TRANSIENT_RETRY_AFTER => {
            return Err(e.clone());
        }
        _ => {} // 캐시 없음, 또는 일시 오류 TTL 만료 → 재시도
    }
    match decode(frame_idx) {
        Ok(f) => { let f = Arc::new(f); cache.put(frame_idx, CacheEntry::Ok(f.clone())); Ok(f) }
        Err(e) if e.is_transient() => {
            cache.put(frame_idx, CacheEntry::TransientFailure(e.clone(), Instant::now()));
            Err(e)
        }
        Err(e) => {
            cache.put(frame_idx, CacheEntry::PermanentFailure(e.clone()));
            Err(e)
        }
    }
}
```
- 에러 타입에 `is_transient()` 같은 분류 메서드를 두고, 일시 오류에는 짧은 TTL을 부여해 자동 재시도가 가능하도록 한다.
- 영구 오류만 TTL 없이 오래 캐싱하고, 그마저도 파일 무효화(CACHE-005) 시점에는 함께 제거되어야 한다.

**탐지 방법**:
- Semantic: negative cache 엔트리 타입이 에러 종류를 구분하는 variant/필드를 갖는지 확인.
- Runtime: 일시적 오류를 인위적으로 주입한 뒤(예: 파일 잠금 후 해제) 캐시가 회복되는지 검증하는 통합 테스트.

**예외**:
- 애초에 일시적 오류가 발생할 수 없는 폐쇄 환경(로컬 디스크의 읽기 전용 파일만 다루는 배치 도구)이라면 구분 없이 단순 영구 캐싱해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-015: deep syntax를 모든 프레임에 저장
**분류**: CACHE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct SyntaxCache {
    // 코딩 유닛/매크로블록 단위까지 내려간 전체 파티션 트리를
    // 사용자가 한 번도 열어보지 않은 프레임까지 포함해 전부 저장
    inner: LruCache<u64, Arc<FullSyntaxTree>>, // 프레임당 수 MB
}

pub fn on_gop_parsed(cache: &mut SyntaxCache, frames: Vec<(u64, FullSyntaxTree)>) {
    for (idx, tree) in frames {
        cache.inner.put(idx, Arc::new(tree)); // 파싱하자마자 무조건 전부 캐싱
    }
}
```

**문제**:
- 사용자가 실제로 "신택스 뷰"를 열어 상세히 들여다보는 프레임은 세션당 소수에 불과한데, 파싱되는 모든 프레임의 전체 트리를 미리 저장해 메모리를 낭비한다.
- 신택스 트리는 디코드된 픽셀 버퍼보다도 프레임당 용량이 클 수 있어(파티션 깊이, MV/참조 인덱스 등 세부 정보를 모두 노드로 들고 있다면) 캐시 예산을 가장 먼저 잠식하는 항목이 되기 쉽다.
- 이는 CACHE-001(entry-count LRU)과 결합하면 문제가 배가된다 — 큰 트리가 자주 축출·재계산되며 다른 저비용 캐시의 효율까지 떨어뜨릴 수 있다.

**발생 조건**:
- GOP/스트림 파싱 파이프라인이 "파싱 후 항상 캐싱"을 기본 동작으로 설계했을 때.
- 신택스 뷰(상세 트리 뷰어)가 세션 전체가 아니라 사용자가 실제로 선택한 소수 프레임에만 필요한 UI 기능일 때.

**권장**:
```rust
pub struct SyntaxCache {
    inner: LruCache<u64, Arc<FullSyntaxTree>>, // 작은 용량 — 사용자가 열어본 프레임만
}

// 파싱 단계에서는 트리를 만들지만 캐시에 넣지 않고,
// 사용자가 "신택스 뷰"를 실제로 연 시점에만 lazy하게 채운다.
pub fn get_syntax_tree(cache: &mut SyntaxCache, frame_idx: u64, source: &ParsedGop) -> Arc<FullSyntaxTree> {
    if let Some(tree) = cache.inner.get(&frame_idx) {
        return tree.clone();
    }
    let tree = Arc::new(source.build_full_syntax_tree(frame_idx)); // 필요 시점에만 구축
    cache.inner.put(frame_idx, tree.clone());
    tree
}
```
- "파싱됨"과 "상세 트리로 캐싱됨"을 분리한다. 기본 파싱 단계에서는 프레임 헤더 수준의 경량 요약만 유지하고, 전체 트리는 UI가 실제로 요청할 때만 구축·캐싱한다.
- 경량 요약(프레임 타입, 크기, 대략적 QP 통계 등)은 별도의 작은 캐시(또는 무제한에 가까운 인메모리 인덱스)로 항상 유지해 타임라인/필름스트립 렌더링에 활용한다.

**탐지 방법**:
- Structural: 파싱 완료 콜백에서 캐시 `put`이 무조건 호출되는지, 아니면 UI 요청 경로에서만 호출되는지 호출 그래프로 확인.
- Runtime: 신택스 뷰를 한 번도 열지 않은 세션에서 신택스 캐시의 메모리 점유량을 측정 — 0에 가까워야 정상.

**예외**:
- 배치 분석 도구처럼 "모든 프레임의 신택스 통계를 항상 필요로 하는" 워크로드라면 전량 캐싱이 오히려 의도된 동작이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-016: seek checkpoint가 너무 촘촘함
**분류**: CACHE · **심각도**: Low · **탐지**: Structural

**나쁜 예**:
```rust
pub struct SeekIndex {
    // 모든 프레임마다 파일 오프셋 + 디코더 상태 스냅샷을 기록
    checkpoints: BTreeMap<u64 /* frame_idx */, Checkpoint>,
}

pub fn on_frame_parsed(index: &mut SeekIndex, frame_idx: u64, offset: u64, decoder_state: DecoderState) {
    index.checkpoints.insert(frame_idx, Checkpoint { offset, decoder_state }); // 매 프레임
}
```

**문제**:
- 체크포인트 하나가 디코더 상태 스냅샷(참조 프레임 버퍼 포인터, 파싱 컨텍스트 등)을 포함한다면, 모든 프레임마다 저장하는 것은 실질적으로 프레임 캐시를 통째로 복제하는 것과 다르지 않아 메모리 낭비가 크다.
- 탐색(seek) 성능 이득은 "가장 가까운 이전 체크포인트부터 재생"이 짧아지는 데서 나오는데, 이미 몇 프레임 간격이면 충분한 상황에서 촘촘함을 더 늘려도 체감 개선은 미미한 반면 인덱스 자체의 메모리·구축 비용은 선형으로 증가한다.
- 인덱스가 커지면 인덱스 자체의 조회(예: `BTreeMap` 탐색)나 직렬화(파일로 저장하는 경우) 비용도 함께 늘어 오히려 역효과가 날 수 있다.

**발생 조건**:
- "탐색을 빠르게 하자"는 목표를 과도하게 해석해 간격을 좁힐수록 무조건 좋다고 가정했을 때.
- 체크포인트 비용(메모리/구축 시간)을 측정하지 않고 간격을 정할 때.

**권장**:
```rust
pub struct SeekIndex {
    checkpoints: BTreeMap<u64, Checkpoint>,
    interval: u64, // GOP 길이, 재생 프레임레이트, 목표 seek 지연을 고려해 산정
}

pub fn on_frame_parsed(index: &mut SeekIndex, frame_idx: u64, offset: u64, decoder_state: DecoderState) {
    // 키프레임(랜덤 액세스 지점) 또는 일정 간격마다만 기록
    if frame_idx % index.interval == 0 || is_key_frame(frame_idx) {
        index.checkpoints.insert(frame_idx, Checkpoint { offset, decoder_state });
    }
}
```
- 간격은 "최악의 경우 재생해야 하는 프레임 수"가 사용자 체감 지연(예: 100ms 이내) 안에 들어오는 최소 밀도로 산정한다 — 대개 키프레임 간격(GOP 길이)과 프레임레이트로부터 역산 가능하다.
- 체크포인트에는 무거운 디코더 상태 전체 대신 재구성에 필요한 최소 정보(파일 오프셋, 필요하다면 참조 프레임 인덱스 목록)만 저장하고, 실제 디코더 상태는 그 지점부터 재생해 재구성한다.

**탐지 방법**:
- Structural: 체크포인트 삽입 조건이 매 프레임(`% 1` 또는 조건 없음)인지 확인.
- Runtime: 인덱스 메모리 점유량을 스트림 길이·해상도별로 측정해 프레임 캐시 대비 비정상적으로 큰 비율을 차지하는지 확인.

**예외**:
- 초단편 클립(수 초 이내)이나 GOP 길이가 원래 매우 짧은 스트림(All-Intra에 가까운 구조)이라면 촘촘한 체크포인트의 상대적 비용이 작아 문제되지 않는다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-017: checkpoint가 없어 매번 GOP 처음부터 decode
**분류**: CACHE · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn seek_to(decoder: &mut Decoder, target_frame: u64) -> Arc<DecodedFrame> {
    // 체크포인트 인덱스가 전혀 없어, 매 seek마다 파일 시작(또는 마지막으로 안 GOP 시작)부터
    // target_frame까지 전부 순차 디코드
    decoder.reset_to_start();
    let mut frame = decoder.decode_next();
    for _ in 0..target_frame {
        frame = decoder.decode_next();
    }
    frame
}
```

**문제**:
- GOP 길이가 긴 스트림(예: 250프레임 GOP)에서 뒤쪽 프레임으로 탐색할 때마다 최악의 경우 249프레임을 버리는 디코드를 수행해야 하므로, seek 하나의 비용이 O(GOP 길이)로 스케일된다.
- 사용자가 타임라인을 드래그하며 여러 지점을 빠르게 스크럽하는 동안 매 seek가 이 비용을 반복하면 UI가 사실상 응답 불가 상태가 된다.
- 체크포인트가 없다는 것은 곧 CACHE-016의 반대 극단이며, 두 문제 모두 "적절한 밀도의 체크포인트 인덱스"라는 동일한 해법이 필요함을 보여준다.

**발생 조건**:
- 초기 구현에서 순차 재생만 지원하다가 임의 탐색(random seek) 기능을 나중에 추가하면서 인덱스 설계를 누락했을 때.
- 긴 GOP를 사용하는 인코딩 프로파일(스트리밍 최적화용 저비트레이트 콘텐츠 등)을 다룰 때.

**권장**:
```rust
pub fn seek_to(decoder: &mut Decoder, index: &SeekIndex, target_frame: u64) -> Arc<DecodedFrame> {
    // target_frame 이하에서 가장 가까운 체크포인트를 찾아 그 지점부터만 재생
    let (&cp_idx, checkpoint) = index.checkpoints.range(..=target_frame).next_back()
        .expect("최소한 스트림 시작 체크포인트는 항상 존재해야 함");
    decoder.restore_from(checkpoint);
    let mut frame = decoder.decode_next();
    for _ in cp_idx..target_frame {
        frame = decoder.decode_next();
    }
    frame
}
```
- CACHE-016에서 설계한 것과 같은 체크포인트 인덱스를 도입해 seek 비용을 O(간격)으로 상한을 둔다.
- 체크포인트 사이 구간을 재생하는 동안 디코드된 중간 프레임들도 프레임 캐시에 적재해두면, 사용자가 근처를 다시 스크럽할 때 추가 이득을 볼 수 있다.

**탐지 방법**:
- Runtime: 여러 GOP 길이의 스트림에서 랜덤 seek 지연시간을 측정 — GOP 길이에 비례해 증가하면 체크포인트 부재를 의심.
- Structural: seek 구현이 항상 "파일/GOP 시작부터"를 기준점으로 삼는지 코드 검토.

**예외**:
- GOP 길이가 매우 짧거나(예: 저지연 스트리밍용 GOP=1~4) 순차 재생만 지원하는 제한된 기능 범위라면 체크포인트 없이도 seek 비용이 무시할 만하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-018: cache hit ratio만 측정
**분류**: CACHE · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
pub struct CacheStats {
    hits: u64,
    misses: u64,
}

impl CacheStats {
    pub fn hit_ratio(&self) -> f64 {
        self.hits as f64 / (self.hits + self.misses).max(1) as f64
    }
}
// 대시보드/로그에는 hit_ratio 하나만 노출 — "95% 적중률이니 캐시는 잘 동작한다"고 결론
```

**문제**:
- 적중률이 높아도 미스 하나의 비용이 매우 큰 경우(예: 긴 GOP 재생이 필요한 미스) 사용자 체감 지연은 여전히 심각할 수 있다 — 적중률은 "얼마나 자주"만 말하고 "얼마나 아픈지"는 말하지 않는다.
- 메모리 사용량이 함께 보고되지 않으면, 적중률을 올리려고 캐시 용량을 계속 늘리는 잘못된 튜닝(OOM 위험 증가)으로 이어지기 쉽다.
- 티어별(압축 패킷 vs 디코드 프레임 vs 신택스 트리, CACHE-002) 적중률을 뭉뚱그려 하나로 보고하면, 실제로 문제가 되는 티어를 특정할 수 없다.

**발생 조건**:
- 캐시 계측을 "적중률 하나"로 단순화해 대시보드에 올렸을 때.
- 성능 회귀를 적중률 지표만으로 게이팅하는 CI/모니터링을 구축했을 때.

**권장**:
```rust
pub struct CacheStats {
    hits: u64,
    misses: u64,
    miss_cost_total: Duration, // 미스 시 재계산에 걸린 시간 누적
    peak_bytes: usize,
    evictions: u64,
    tier: &'static str, // "packet" | "decoded_frame" | "syntax_tree" 등
}

impl CacheStats {
    pub fn avg_miss_latency(&self) -> Duration {
        self.miss_cost_total.checked_div(self.misses.max(1) as u32).unwrap_or_default()
    }
    pub fn effective_cost(&self) -> Duration {
        // 적중률만이 아니라 "미스가 사용자에게 실제로 준 총 지연"을 지표화
        self.miss_cost_total
    }
}
```
- 적중률과 함께 미스 평균/최대 지연, 피크 메모리, 축출 횟수, 티어별 분해를 반드시 함께 계측·보고한다.
- 성능 목표는 "적중률 X% 이상"이 아니라 "P95 seek 지연 Y ms 이하", "피크 RSS Z GB 이하"처럼 사용자 체감 지표로 정의한다.

**탐지 방법**:
- Manual: 성능 대시보드/PR 설명에서 캐시 관련 지표가 hit ratio 단일 수치로만 보고되는지 확인.
- Runtime: 미스 지연 분포(히스토그램)를 별도로 계측해 롱테일이 존재하는지 점검.

**예외**:
- 프로토타입/실험 단계에서 대략적인 신호만 필요하다면 적중률만으로 시작해도 무방하다 — 단, 제품화 전에는 반드시 비용 기반 지표를 추가해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-019: 재계산 비용을 고려하지 않는 eviction
**분류**: CACHE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
pub struct FrameCache {
    // 순수 LRU: "최근에 안 썼다"만 보고 축출 — 재계산 비용은 완전히 무시
    inner: LruCache<u64, Arc<DecodedFrame>>,
}
```

**문제**:
- 랜덤 액세스 키프레임(재계산 비용: 그 프레임 하나만 디코드, 저렴)과 GOP 중간의 P/B 프레임(재계산 비용: GOP 시작부터 재생 필요, 훨씬 비쌈)이 "마지막 접근 시각"만으로 동일하게 취급되어, 비싼 프레임이 싼 프레임보다 먼저 축출될 수 있다.
- 사용자가 특정 구간을 반복 스크럽하는 패턴(뒤로 갔다 앞으로 갔다)에서, 순수 LRU는 그 구간의 "비싼" 중간 프레임들을 계속 축출·재계산해 체감 성능이 나빠진다.

**발생 조건**:
- GOP 구조가 있는 코덱(대부분의 실전 스트림)에서 프레임별 재계산 비용 편차가 클 때.
- 캐시 정책을 별도 튜닝 없이 라이브러리 기본 LRU에 그대로 의존할 때.

**권장**:
```rust
pub struct CostAwareEntry {
    frame: Arc<DecodedFrame>,
    recompute_cost_hint: u32, // 예: GOP 내 이 프레임까지 재생해야 하는 프레임 수
}

pub struct CostAwareCache {
    inner: LruCache<u64, CostAwareEntry>,
    budget_bytes: usize,
    used_bytes: usize,
}

impl CostAwareCache {
    pub fn evict_if_needed(&mut self) {
        while self.used_bytes > self.budget_bytes {
            // "가장 오래됐으면서 재계산 비용도 낮은" 항목을 우선 축출 대상으로 스캔
            // (완전한 우선순위 큐가 아니어도, LRU 순회 중 cost가 낮은 항목을 먼저 고르는
            //  근사 정책만으로도 순수 LRU보다 개선된다)
            if let Some(key) = self.find_cheapest_among_lru_tail(8) {
                if let Some(entry) = self.inner.pop(&key) {
                    self.used_bytes -= entry.frame.byte_size();
                }
            } else {
                break;
            }
        }
    }
}
```
- 비용 힌트(재계산에 필요한 작업량 추정치)를 엔트리에 함께 저장하고, 축출 시 "오래됐고 + 재계산도 싼" 항목을 우선한다(GreedyDual-Size류 정책의 단순화 버전).
- 완벽한 비용 기반 우선순위 큐가 부담스럽다면, LRU tail 근방 N개 중에서만 비용 비교로 축출 대상을 고르는 근사만으로도 순수 LRU보다 나은 결과를 낸다.

**탐지 방법**:
- Structural: 캐시가 재계산 비용과 무관한 라이브러리 기본 LRU를 그대로 사용하는지, GOP 구조 인지 여부를 코드에서 확인.
- Runtime: 반복 스크럽 워크로드에서 순수 LRU와 비용 인지 정책의 재계산 횟수를 비교하는 벤치마크.

**예외**:
- 모든 항목의 재계산 비용이 사실상 균일한 경우(예: 모든 프레임이 키프레임인 All-Intra 스트림)에는 순수 LRU로도 충분하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-020: 메모리 압박 신호에 반응하지 않음
**분류**: CACHE · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
pub struct FrameCache {
    inner: LruCache<u64, Arc<DecodedFrame>>,
    budget_bytes: usize, // 프로세스 시작 시 고정값으로 설정된 뒤 절대 바뀌지 않음
}
// OS의 메모리 압박 알림(macOS의 memory pressure, Linux cgroup의 memory.pressure,
// 또는 단순히 시스템 전체 가용 메모리 감소)을 전혀 구독하지 않는다.
```

**문제**:
- 사용자가 다른 무거운 애플리케이션을 함께 실행 중이거나, 여러 개의 Bitvue 창(또는 여러 파일)을 동시에 열어 시스템 전체 메모리가 빠듯해져도 캐시는 고정된 예산까지 계속 채우려 한다.
- 시스템이 스와핑을 시작하면 캐시 "적중"조차 디스크 스왑 I/O를 유발해 캐시가 없는 것보다 느려지는 역설적 상황이 발생할 수 있다.
- 극단적인 경우 OS OOM killer가 프로세스를 강제 종료시켜, 사용자 작업(주석, 분석 결과)이 저장되지 않은 채 유실될 위험이 있다.

**발생 조건**:
- 대형 원본 파일(수 GB~수십 GB)을 다루면서 캐시 예산을 시스템 여유 메모리와 무관하게 고정 비율/고정값으로 설정했을 때.
- 여러 파일을 동시에 여는 멀티 탭/멀티 윈도우 시나리오(CACHE-021과도 연관)에서 각 인스턴스가 서로의 메모리 사용을 모른 채 독립적으로 예산을 소진할 때.

**권장**:
```rust
pub struct FrameCache {
    inner: LruCache<u64, Arc<DecodedFrame>>,
    budget_bytes: AtomicUsize, // 런타임에 조정 가능
}

impl FrameCache {
    /// OS 메모리 압박 콜백(플랫폼별 API를 얇게 감싼 리스너)에서 호출
    pub fn on_memory_pressure(&mut self, level: PressureLevel) {
        let new_budget = match level {
            PressureLevel::Normal   => self.default_budget(),
            PressureLevel::Warning  => self.default_budget() / 2,
            PressureLevel::Critical => self.default_budget() / 8,
        };
        self.budget_bytes.store(new_budget, Ordering::Relaxed);
        self.evict_down_to(new_budget); // 즉시 목표치까지 축출
    }
}

// 플랫폼별 예시: macOS는 DispatchSource memory pressure, Linux는 PSI(/proc/pressure/memory)
// 폴링 또는 cgroup memory.pressure 이벤트 fd, 최소한 폴백으로 sysinfo 크레이트로
// 주기적 가용 메모리 폴링도 가능하다.
```
- 플랫폼 메모리 압박 알림을 구독하거나, 최소한 주기적으로 시스템 가용 메모리를 폴링해 예산을 동적으로 축소·복원한다.
- 압박 신호를 받으면 즉시 목표 예산까지 축출하되, CACHE-010/011에서 다룬 것처럼 축출 자체가 UI를 막지 않도록 별도 스레드에서 처리한다.

**탐지 방법**:
- Runtime: 시스템 메모리를 인위적으로 압박한 상태(다른 프로세스로 메모리 점유)에서 앱의 RSS 추이와 OOM 발생 여부를 관찰.
- Manual: 캐시 예산 설정 코드에서 OS 메모리 압박 API 구독 여부를 확인.

**예외**:
- 캐시 예산이 시스템 전체 메모리 대비 매우 작게(예: 수십 MB 이내) 하드캡되어 있어 애초에 압박을 유발할 수 없는 규모라면 생략 가능.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-021: 파일별 캐시 격리 부재
**분류**: CACHE · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
// 여러 파일을 탭으로 동시에 열 수 있는 UI인데, 캐시는 프로세스 전역 싱글턴 하나
static FRAME_CACHE: Lazy<Mutex<LruCache<u64 /* frame_idx */, Arc<DecodedFrame>>>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(500).unwrap())));

pub fn get_frame(frame_idx: u64) -> Arc<DecodedFrame> {
    // 어느 탭/파일에 대한 요청인지 key에 전혀 반영되지 않는다
    FRAME_CACHE.lock().unwrap().get(&frame_idx).cloned().unwrap_or_else(|| decode_and_cache(frame_idx))
}
```

**문제**:
- 탭 A(파일 X)의 frame_idx=10과 탭 B(파일 Y)의 frame_idx=10이 같은 캐시 슬롯을 공유해, 탭을 전환하는 순간 완전히 다른 파일의 프레임이 화면에 표시될 수 있다 — CACHE-005/CACHE-003과 유사하지만 원인이 "파일 네임스페이스 누락"이라는 점에서 별개 항목이다.
- 두 탭을 빠르게 오가며 스크럽하면 서로의 캐시를 밀어내는 스래싱이 발생해, 탭이 하나일 때보다 두 탭을 열었을 때 오히려 각 탭의 체감 성능이 크게 떨어진다.
- 파일 닫기(CACHE-005/011)나 무효화 로직이 "이 파일에 해당하는 항목만" 선택적으로 제거할 방법이 없어, 탭 하나를 닫아도 다른 탭의 캐시까지 함께 정리되거나 반대로 전혀 정리되지 않는 극단으로 치우치기 쉽다.

**발생 조건**:
- 단일 파일만 지원하던 초기 버전에 멀티 탭/멀티 윈도우 기능을 나중에 추가했는데 캐시 설계를 함께 갱신하지 않았을 때.
- 캐시가 전역 `static`/싱글턴 `AppState` 필드로 선언되어 있고 파일 식별자가 key 구조에 원래부터 없었을 때.

**권장**:
```rust
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct FrameKey {
    file_id: FileId, // 파일을 열 때 발급되는 고유 ID (경로가 아니라 세션 내 고유 정수/UUID)
    frame_idx: u64,
}

pub struct AppState {
    // 파일별로 독립된 캐시 인스턴스와 예산을 갖는다
    caches: HashMap<FileId, FrameCache>,
}

impl AppState {
    pub fn close_file(&mut self, file_id: FileId) {
        self.caches.remove(&file_id); // 해당 파일의 캐시만 정확히 회수
    }
}
```
- 파일/탭마다 고유한 `FileId`를 key에 포함시키거나, 아예 파일 단위로 캐시 인스턴스를 분리해 상호 간섭을 원천 차단한다.
- 여러 파일이 동시에 열려 있을 때 전체 메모리 예산을 파일 수만큼 균등 분배하거나, 활성 탭에 더 많은 예산을 우선 배정하는 전역 조정 로직을 둔다(CACHE-020과 결합).

**탐지 방법**:
- Semantic: 캐시 key 타입에 파일/세션 식별자가 없는데 멀티 파일 지원 UI 코드(탭, 다중 윈도우)가 존재하는지 교차 확인.
- Runtime: 두 개의 서로 다른 파일을 열고 빠르게 탭을 전환하며 스크럽하는 테스트에서 잘못된 프레임이 표시되는지 확인.

**예외**:
- 애초에 한 번에 파일 하나만 열 수 있는 단일 문서 인터페이스(SDI)라면 파일 네임스페이스 없이 전역 캐시로 충분하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-022: cache warm-up 전략 부재로 첫 스크럽 시 stutter
**분류**: CACHE · **심각도**: Low · **탐지**: Runtime

**나쁜 예**:
```rust
#[tauri::command]
async fn open_file(path: String, state: tauri::State<'_, AppState>) -> Result<FileInfo, String> {
    let decoder = open_decoder(&path)?;
    let info = decoder.probe_info();
    state.decoders.lock().unwrap().insert(info.file_id, decoder);
    // 캐시는 완전히 빈 상태로 반환 — 사용자가 첫 조작(재생/스크럽)을 할 때
    // 비로소 콜드 디코드가 시작된다
    Ok(info)
}
```

**문제**:
- 파일을 연 직후 사용자가 재생 버튼을 누르거나 타임라인을 클릭하는 첫 조작이, 캐시가 완전히 비어 있는 상태에서 콜드 디코드를 기다려야 해 가장 나쁜 첫인상을 준다.
- 파일 열기 완료 시점(프로브/헤더 파싱 완료)과 실제로 상호작용 가능해지는 시점 사이의 유휴 시간을 활용하지 못하고 그냥 버린다.

**발생 조건**:
- "파일 열기 = 헤더/인덱스 파싱만" 으로 좁게 정의하고, 이후 사용자 조작을 전적으로 온디맨드 디코드에 맡기는 설계일 때.
- 파일 열기 UI가 프로브 완료 즉시 "준비 완료" 상태로 전환되어 사용자가 바로 조작을 시작할 수 있는 빠른 반응성을 이미 갖춘 경우, 그 이면에서 캐시는 여전히 비어 있을 때.

**권장**:
```rust
#[tauri::command]
async fn open_file(path: String, state: tauri::State<'_, AppState>) -> Result<FileInfo, String> {
    let decoder = open_decoder(&path)?;
    let info = decoder.probe_info();
    let file_id = info.file_id;
    state.decoders.lock().unwrap().insert(file_id, decoder);

    // 헤더 파싱이 끝나는 즉시, 사용자가 아직 조작하지 않은 짧은 시간을 이용해
    // 첫 N프레임(및 대응 썸네일)을 낮은 우선순위 백그라운드 작업으로 미리 디코드
    let state_clone = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        warm_up_cache(state_clone, file_id, /* first_n = */ 64).await;
    });

    Ok(info) // warm-up을 기다리지 않고 즉시 반환 — UI는 바로 조작 가능
}
```
- 파일 열기 응답을 warm-up 완료까지 지연시키지 않는다 — warm-up은 어디까지나 "덤"으로, 백그라운드 저우선순위 작업이어야 한다.
- warm-up 대상은 사용자가 실제로 먼저 볼 가능성이 높은 지점(파일 시작 N프레임, 필름스트립에 표시될 균등 간격 썸네일)으로 한정해 낭비를 최소화한다.
- 사용자가 warm-up이 아직 다루지 않은 위치로 즉시 이동하면, 온디맨드 디코드가 warm-up보다 우선순위를 갖도록 스케줄링한다(CACHE-024와 연관).

**탐지 방법**:
- Runtime: 파일을 연 직후 "첫 스크럽까지의 지연시간"을 측정 — warm-up이 없으면 이후 스크럽 대비 유의미하게 느려야 정상(있으면 이 격차가 줄어듦).
- Manual: 파일 열기 커맨드 구현에서 캐시 관련 호출이 전혀 없는지 확인.

**예외**:
- 매우 낮은 사양 환경에서 warm-up 자체가 파일 열기 응답성이나 다른 백그라운드 작업(인덱싱 등)과 자원을 다투어 역효과를 낼 수 있다면, warm-up 범위를 더 줄이거나 생략하는 편이 나을 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-023: 캐시 크기 설정이 해상도/파일 특성에 적응하지 않음
**분류**: CACHE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
const FRAME_CACHE_BUDGET_BYTES: usize = 512 * 1024 * 1024; // 하드코드된 512MB, 모든 스트림 공통
```

**문제**:
- 480p 스트림 기준으로는 넉넉한 512MB가, 8K RGBA 프레임(약 132MB/프레임) 앞에서는 3~4프레임밖에 담지 못해 캐시가 사실상 무의미해진다.
- 반대로 저해상도 스트림에서는 같은 512MB가 필요 이상으로 크게 할당되어, 여러 파일을 동시에 열 때(CACHE-021) 전체 시스템 메모리를 불필요하게 압박한다.
- 상수 하나로 모든 경우를 커버하려 하면 결국 "8K에서는 부족하고 480p에서는 과하다"는 양쪽 모두 최적이 아닌 지점에 머무르게 된다.

**발생 조건**:
- 초기 개발/테스트를 특정 해상도(예: 1080p 샘플 클립)로만 진행해 그 환경에 맞는 상수를 그대로 굳혔을 때.
- 캐시 예산을 설정 파일이나 UI로 노출하지 않고 컴파일 타임 상수로 박아두었을 때.

**권장**:
```rust
pub fn compute_frame_cache_budget(stream_info: &StreamInfo, system_available_bytes: usize) -> usize {
    let bytes_per_frame = stream_info.width as usize * stream_info.height as usize * 4; // RGBA
    let target_frame_count = 64; // "캐시에 몇 프레임 분량을 유지할지"를 기준으로 역산
    let ideal = bytes_per_frame * target_frame_count;

    // 시스템 여유 메모리의 일정 비율을 상한으로 두어 과할당 방지 (CACHE-020과 연동)
    let system_cap = system_available_bytes / 4;

    ideal.min(system_cap).max(64 * 1024 * 1024) // 최소 64MB는 보장
}
```
- 예산을 "바이트 상수"가 아니라 "목표 프레임 수 × 해상도별 프레임 크기"로 유도해 해상도에 자연스럽게 비례하도록 만든다.
- 시스템 가용 메모리 대비 상한(cap)을 함께 두어 저사양 환경에서의 과할당도 방지한다.
- 필요하면 사용자가 설정 UI에서 예산을 직접 조정할 수 있게 노출한다.

**탐지 방법**:
- Static: 캐시 예산이 `const`로 선언되어 해상도/스트림 정보를 입력받지 않는지 grep.
- Runtime: 저해상도와 초고해상도 스트림 각각에서 캐시가 담을 수 있는 프레임 수를 로깅해 자릿수 차이가 나는지 확인.

**예외**:
- 지원 대상 해상도가 명세상 고정된 좁은 도메인(예: 특정 방송 표준 규격만 지원)이라면 고정 상수도 합리적인 선택일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-024: 백그라운드 프리페치가 탐색 방향을 무시함
**분류**: CACHE · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn on_frame_displayed(cache: &mut FrameCache, decoder: &mut Decoder, current: u64) {
    // 항상 "앞으로만" 미리 디코드 — 사용자가 되감기(scrub backward) 중이어도 무조건 forward
    for i in 1..=8 {
        prefetch(cache, decoder, current + i);
    }
}
```

**문제**:
- 사용자가 타임라인을 뒤로 드래그하는 동안에도 프리페처는 계속 앞쪽 프레임을 미리 디코드해, 실제로 곧 필요한(뒤쪽) 프레임 대신 당분간 쓰이지 않을 프레임으로 캐시를 채운다.
- 이렇게 채워진 "쓸모없는" 프리페치 결과가 오히려 진짜 필요한 프레임을 캐시에서 밀어내(CACHE-019와 결합) 역방향 스크럽 성능을 프리페치가 없을 때보다 더 나쁘게 만들 수 있다.
- 앞뒤로 짧게 왔다 갔다 하는(scrubbing back and forth) 흔한 UI 패턴에서 프리페치와 실제 접근 패턴이 계속 어긋나 캐시 스래싱이 반복된다.

**발생 조건**:
- 프리페치 로직을 "재생은 항상 앞으로 진행된다"는 가정 하에 처음 구현하고, 이후 스크럽/되감기 UX가 추가되었을 때 프리페치 로직을 갱신하지 않았을 때.
- 최근 탐색 방향(속도·부호)을 추적하는 상태가 프리페처에 전달되지 않을 때.

**권장**:
```rust
pub struct ScrubVelocity {
    direction: i8, // +1 forward, -1 backward, 0 idle
    recent_deltas: VecDeque<i64>, // 최근 N번의 frame_idx 변화량
}

pub fn on_frame_displayed(cache: &mut FrameCache, decoder: &mut Decoder,
                           current: u64, velocity: &ScrubVelocity) {
    let step = if velocity.direction >= 0 { 1i64 } else { -1i64 };
    for i in 1..=8i64 {
        let target = current as i64 + step * i;
        if target >= 0 {
            prefetch(cache, decoder, target as u64);
        }
    }
}
```
- 최근 접근 이력으로부터 방향(및 가능하면 속도)을 추정해 프리페치 방향을 그에 맞춰 동적으로 전환한다.
- 방향이 자주 바뀌는(빠른 왕복 스크럽) 상황을 감지하면 프리페치 폭을 줄이거나 아예 비활성화해 헛수고와 캐시 오염을 방지하는 것도 합리적인 절충이다.

**탐지 방법**:
- Runtime: 역방향 스크럽 워크로드에서 프리페치 활성/비활성 시 캐시 적중률과 재계산 횟수를 비교 — 역방향에서 프리페치가 오히려 적중률을 낮추면 문제 확정.
- Manual: 프리페치 함수 시그니처에 현재 탐색 방향/속도 정보가 인자로 존재하는지 확인.

**예외**:
- 재생 전용(순수 forward playback) UI만 지원하고 되감기/스크럽 기능이 없다면 단방향 프리페치로 충분하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### CACHE-025: 캐시 관측성 부재로 회귀를 감지하지 못함
**분류**: CACHE · **심각도**: Low · **탐지**: Manual

**나쁜 예**:
```rust
pub struct FrameCache {
    inner: LruCache<u64, Arc<DecodedFrame>>,
    // hit/miss 카운터도, 축출 카운터도, 메모리 고점(high-water mark)도 없음
    // — 캐시가 잘 동작하는지 확인할 방법이 "체감상 느려진 것 같다"는 사용자 리포트뿐
}
```

**문제**:
- 리팩터링(예: CACHE-001~024 항목을 개선하는 과정)이 의도치 않게 캐시 효율을 악화시켜도, 계측이 없으면 이를 코드 리뷰나 CI에서 잡을 방법이 없다 — 사용자가 "느려졌다"고 보고할 때까지 발견되지 않는다.
- 프로덕션에서 실제 사용 패턴(어떤 크기의 파일, 어떤 스크럽 패턴)에 대한 데이터가 없으면, 캐시 튜닝(예산 크기, 축출 정책, 프리페치 폭)이 전부 추측에 의존하게 된다.
- 메모리 관련 버그(CACHE-009 Arc cycle, CACHE-020 압박 미대응)는 특히 장시간 세션에서만 드러나는데, 고점 계측이 없으면 이런 서서히 진행되는 문제를 조기에 포착할 수 없다.

**발생 조건**:
- 캐시를 "일단 동작하게 만드는 것"까지만 구현하고 계측을 후순위로 미뤘을 때.
- 성능 이슈가 사용자 리포트로만 유입되고, 사전에 감지할 자동화된 신호가 없을 때.

**권장**:
```rust
pub struct FrameCacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    peak_bytes: AtomicUsize,
    current_bytes: AtomicUsize,
}

impl FrameCacheMetrics {
    pub fn record_put(&self, added_bytes: usize) {
        let now = self.current_bytes.fetch_add(added_bytes, Ordering::Relaxed) + added_bytes;
        self.peak_bytes.fetch_max(now, Ordering::Relaxed);
    }

    // tracing 스팬/이벤트로 노출하면 기존 로깅 인프라와 자연스럽게 결합된다
    pub fn snapshot(&self) -> CacheSnapshot {
        CacheSnapshot {
            hit_ratio: self.hits.load(Ordering::Relaxed) as f64
                / (self.hits.load(Ordering::Relaxed) + self.misses.load(Ordering::Relaxed)).max(1) as f64,
            evictions: self.evictions.load(Ordering::Relaxed),
            peak_bytes: self.peak_bytes.load(Ordering::Relaxed),
        }
    }
}
```
- 최소한 hit/miss, 축출 횟수, 현재/고점 메모리 사용량을 `tracing` 이벤트나 주기적 스냅샷으로 노출한다(CACHE-018에서 다룬 비용 지표와 함께).
- CI 벤치마크(예: 고정된 골든 스트림에 대해 표준 스크럽 시나리오를 재생)에서 이 지표들을 수집해 임계값을 벗어나면 실패하도록 회귀 게이트를 건다.
- 프로덕션에서는 사용자 동의 하에 익명화된 집계 지표만 수집하거나, 최소한 디버그 빌드/옵트인 진단 모드에서라도 노출한다.

**탐지 방법**:
- Manual: 캐시 구조체 정의에 카운터 필드가 전혀 없는지, 관련 `tracing`/로깅 호출이 존재하는지 확인.
- Structural: 성능 관련 CI 워크플로에 캐시 지표를 검증하는 단계가 있는지 확인.

**예외**:
- 매우 초기 단계의 프로토타입이거나 캐시 자체가 실험적 기능이라 아직 정식 계측 투자가 이르다고 판단되는 경우, 최소한 개발자 로그 수준의 임시 계측만으로 시작해도 된다 — 단, 정식 기능화 전에는 반드시 보강해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
</content>
