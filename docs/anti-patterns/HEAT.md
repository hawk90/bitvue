# Anti-Pattern Catalog — HEAT: Heatmap과 공간 통계 (VQ-Probe domain)

이 문서는 더 큰 안티패턴 카탈로그의 일부입니다(전체 목록은 별도로 작성 중인 `docs/anti-patterns/INDEX.md` 참고). VQ-Probe(듀얼 스트림 품질 비교) 도메인 절반(ALIGN/SPATIAL/COLOR/METRIC/PIPE/STAT/HEAT)에 속하며, Bitvue 본체(bitstream-analyzer 도메인)와는 media-core 레이어만 공유하는 별도 아키텍처다. "대형 2D 오버레이 데이터 + 뷰포트 렌더링 + 줌 + IPC"라는 일반론은 Bitvue 자체의 QP/MV 오버레이와 동일하게 적용되며 그 렌더링 메커니즘 관점은 1단계 `PIXEL.md`/`IPC.md`에서 이미 다뤘다. 본 파일은 그 미러로서, VQ-Probe가 자체 계산하는 프레임/블록 단위 **품질 점수** 히트맵의 계산 정확성(정규화, NaN, 타일 경계, raw/downsample 혼용 등) 문제에 집중한다 — 렌더링 방식이 아니라 "그 숫자가 맞는가"의 문제.

---

### HEAT-001: 픽셀별 score를 객체로 저장
**분류**: 자료구조 설계 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct PixelScore {
    x: u32,
    y: u32,
    score: f32,
    metric: MetricKind,
}

struct FrameHeatmap {
    scores: Vec<PixelScore>, // 1920x1080이면 2백만 개 이상의 구조체
}

fn build_heatmap(width: u32, height: u32, raw: &[f32]) -> FrameHeatmap {
    let mut scores = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            scores.push(PixelScore {
                x,
                y,
                score: raw[(y * width + x) as usize],
                metric: MetricKind::Ssim,
            });
        }
    }
    FrameHeatmap { scores }
}
```

**문제**:
- `PixelScore`는 24바이트(패딩 포함)인데 실제 유효 정보는 `f32` 4바이트뿐이다 — 6배 이상의 메모리 낭비.
- `x`, `y`, `metric`은 격자 구조상 인덱스로부터 항상 유도 가능한데도 매 픽셀마다 중복 저장한다.
- 1920x1080 프레임 하나만으로도 2백만 개 이상의 힙 할당된(또는 Vec 내부의) 구조체가 생성되어 캐시 지역성이 나빠지고, GC/드롭 비용도 커진다(Rust는 GC가 없지만 `Vec<PixelScore>` 드롭 시 필드별 정렬·패딩 처리 비용이 flat 배열보다 크다).
- 프레임 여러 개를 버퍼링하는 순간(재생 중 앞뒤 프레임 프리페치) 이 낭비가 프레임 수만큼 곱해진다.

**발생 조건**:
- 4K 이상 해상도 또는 블록 단위가 아닌 픽셀 단위 score를 다루는 경우 손실이 특히 크다.
- 여러 프레임을 동시에 메모리에 유지하는 스크러빙/프리로드 시나리오에서 메모리 압박이 누적된다.

**권장**:
```rust
struct FrameHeatmap {
    width: u32,
    height: u32,
    metric: MetricKind,
    scores: Vec<f32>, // row-major flat 배열, len == width * height
}

impl FrameHeatmap {
    fn score_at(&self, x: u32, y: u32) -> f32 {
        self.scores[(y * self.width + x) as usize]
    }
}
```
- 좌표는 격자 규약(row-major, width 고정)으로부터 유도하고 데이터에는 저장하지 않는다.
- `metric`처럼 프레임 전체에 공통인 값은 헤더에 한 번만 둔다.
- 대용량 배열이 필요하면 `Vec<f32>` 대신 `Box<[f32]>` 또는 memory-mapped 버퍼를 고려해 재할당을 줄인다.

**탐지 방법**:
- Static: 힙 프로파일러(`heaptrack`, `dhat`)로 프레임당 할당 개수가 픽셀 수 오더인지 확인.
- Structural: `Vec<StructWithXY>` 형태로 격자 데이터를 담는 타입을 코드베이스에서 검색.

**예외**:
- score가 존재하는 픽셀이 극히 희소한 경우(예: 임계값 초과 outlier만 표시)라면 sparse 표현(`(x, y, score)` 목록)이 오히려 dense 배열보다 효율적이다.

**Bitvue 판정**: N/A — 이미 권장안대로 구현됨. `DiffHeatmapData.values: Vec<f32>`(diff_heatmap.rs:87)와 `BlockMetricsGrid.values: Vec<f32>`(block_metrics.rs:127) 모두 row-major flat 배열. `BlockMetricValue{x,y,value,metric_type}` 구조체(block_metrics.rs:73)가 존재하지만 `Vec<BlockMetricValue>`로 대량 저장하는 곳은 없고 단일 값 전달/테스트용으로만 쓰인다.

---

### HEAT-002: 전체 해상도 heatmap을 항상 계산
**분류**: 연산 낭비 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn compute_frame_heatmap(a: &YPlane, b: &YPlane) -> FrameHeatmap {
    // 뷰포트가 1/8 축소 상태로 썸네일만 보고 있어도 항상 풀 해상도로 계산
    let scores = compute_block_ssim_grid(a, b, BLOCK_SIZE);
    FrameHeatmap::new(a.width, a.height, scores)
}
```

**문제**:
- UI가 실제로 요구하는 해상도(현재 줌 레벨, 뷰포트 크기)와 무관하게 항상 최대 해상도로 계산해 CPU를 낭비한다.
- 필름스트립 썸네일이나 축소된 프레임 목록을 스크롤할 때조차 프레임마다 전체 해상도 SSIM/PSNR 그리드를 계산하면 스크롤이 버벅인다.
- 계산 결과 대부분이 다운샘플링되어 화면에 표시조차 안 되는데 그 연산 비용은 고스란히 지불된다.

**발생 조건**:
- 다중 프레임을 빠르게 훑어보는 필름스트립/타임라인 뷰, 또는 저사양 뷰포트에서 특히 두드러진다.
- 4K/8K 콘텐츠에서 블록 단위 metric 계산 자체가 무겁다면 낭비가 배가된다.

**권장**:
```rust
fn compute_frame_heatmap(a: &YPlane, b: &YPlane, target_resolution: HeatmapResolution) -> FrameHeatmap {
    match target_resolution {
        HeatmapResolution::Full => compute_block_ssim_grid(a, b, BLOCK_SIZE),
        HeatmapResolution::Downsampled(factor) => {
            // 다운샘플된 plane에서 직접 계산 — 풀 해상도를 거치지 않는다
            let (a_ds, b_ds) = (downsample_plane(a, factor), downsample_plane(b, factor));
            compute_block_ssim_grid(&a_ds, &b_ds, BLOCK_SIZE)
        }
    }
}
```
- 뷰포트/줌 레벨에서 요구하는 해상도를 계산 요청 시점에 명시적으로 전달한다.
- 다운샘플은 "계산 후 축소"가 아니라 "축소 후 계산"으로 설계해 연산량 자체를 줄인다(단, HEAT-015 참고 — raw와 downsample 결과의 의미 차이를 명확히 문서화해야 한다).

**탐지 방법**:
- Runtime: 썸네일/필름스트립 스크롤 시 CPU 사용률과 프레임당 heatmap 계산 시간 프로파일링.
- Structural: heatmap 계산 함수 시그니처에 해상도/줌 파라미터가 없는지 검색.

**예외**:
- 내보내기(export) 또는 리포트 생성처럼 최종적으로 풀 해상도가 필요한 배치 작업에서는 항상 풀 해상도로 계산하는 것이 맞다.

**Bitvue 판정**: Confirmed — `DiffHeatmapData::from_luma_planes`(diff_heatmap.rs:103-175)는 뷰포트/줌 파라미터를 전혀 받지 않고 항상 고정 2x2 다운샘플(half-res)로 계산한다("Half-res default" 주석, line 6). "항상 풀 해상도"는 아니지만 "실제 필요 해상도와 무관하게 계산 해상도가 하드코딩되어 있다"는 핵심 구조는 동일하게 present. 다만 이 파이프라인 자체가 어떤 `#[tauri::command]`에서도 호출되지 않아(아래 HEAT-003/004 참고) 실사용 스크러빙 시나리오에서 체감되는지는 검증 불가.

---

### HEAT-003: zoom level과 관계없이 원본 resolution 전송
**분류**: IPC 페이로드 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn get_quality_heatmap(frame_index: u32) -> Vec<f32> {
    // 프론트엔드가 현재 25% 줌으로 보고 있어도 항상 원본 해상도 배열을 반환
    let heatmap = STORE.compute_heatmap(frame_index);
    heatmap.scores // width * height 크기의 원본 배열
}
```

**문제**:
- IPC 페이로드 크기가 실제 화면에 필요한 픽셀 수와 무관하게 항상 최대치로 고정되어 직렬화/역직렬화 비용과 IPC 대역폭을 낭비한다.
- 프론트엔드는 어차피 캔버스 렌더링 시 다운샘플링하므로, 원본 해상도 전체를 받는 것은 "버릴 데이터를 굳이 전송"하는 셈이다.
- 줌 인/아웃을 반복할 때마다 매번 동일한 원본 크기 데이터가 재전송되면 UI 반응성이 줌 레벨과 무관하게 느려진다.

**발생 조건**:
- 4K 이상 해상도 프레임을 축소 뷰(필름스트립, 오버뷰 패널)에서 표시할 때 특히 낭비가 크다.
- 줌 레벨 변경이 빈번한 인터랙티브 세션에서 누적 IPC 비용이 체감된다.

**권장**:
```rust
#[tauri::command]
fn get_quality_heatmap(frame_index: u32, zoom: ZoomLevel) -> HeatmapPayload {
    let target_res = resolution_for_zoom(zoom); // 줌에 맞는 목표 해상도 결정
    let heatmap = STORE.compute_heatmap_at(frame_index, target_res);
    HeatmapPayload {
        width: heatmap.width,
        height: heatmap.height,
        resolution_tag: target_res, // 어떤 해상도로 계산됐는지 명시(HEAT-008/015 참고)
        scores: heatmap.scores,
    }
}
```
- 줌 레벨을 IPC 요청 파라미터로 명시하고, 백엔드는 그에 맞는 해상도의 heatmap을 반환한다.
- 응답에 실제 사용된 해상도를 태그로 포함시켜 프론트엔드가 원본과 축소본을 혼동하지 않게 한다.

**탐지 방법**:
- Structural: `#[tauri::command]` 함수 중 해상도/줌 파라미터 없이 heatmap을 반환하는 것을 검색.
- Runtime: 줌 아웃 상태에서 IPC 페이로드 크기를 측정해 줌 레벨과 무관하게 고정인지 확인.

**예외**:
- 원본 해상도 자체가 이미 작은 콘텐츠(SD급)라면 다운샘플링 경로를 별도로 둘 필요가 없다.

**Bitvue 판정**: Suspected — `diff_heatmap.rs`/`block_metrics.rs`를 실제로 프론트엔드에 노출하는 `#[tauri::command]`가 코드베이스 어디에도 없다(`src-tauri/src/commands`에 `DiffHeatmap`/`BlockMetric` 참조 0건). 따라서 "줌과 무관하게 원본 해상도 전송"이 실제로 일어나는지 확인할 IPC 경로 자체가 아직 존재하지 않는다. `cache_provenance.rs`의 `CacheKey::QpHeatmap{ hm_res, scale_mode, .. }` 설계는 해상도별 캐시 분리 의도를 보이지만, diff/quality heatmap 쪽은 그 설계조차 실사용 커맨드로 이어지지 않은 상태.

---

### HEAT-004: heatmap을 JSON float 배열로 전달
**분류**: IPC 직렬화 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn get_quality_heatmap(frame_index: u32) -> serde_json::Value {
    let heatmap = STORE.compute_heatmap(frame_index);
    serde_json::json!({
        "width": heatmap.width,
        "height": heatmap.height,
        "scores": heatmap.scores, // Vec<f32> -> JSON 배열, 원소마다 텍스트 표현
    })
}
```

**문제**:
- `f32` 하나가 JSON에서는 `"0.87234561"`처럼 최대 10바이트 이상의 텍스트로 부풀려지고, 파싱 시 문자열→부동소수 변환 비용까지 추가된다.
- 1920x1080 블록 그리드(예: 16x16 블록 기준 약 8100개)라도 프레임마다 반복되면 누적 비용이 크고, 픽셀 단위라면 수백 배로 증폭된다.
- JSON은 NaN/Infinity를 표준적으로 표현하지 못해 별도 sentinel 처리가 필요해지고(HEAT-012 문제와 얽힘), 직렬화 실패나 손실이 발생하기 쉽다.

**발생 조건**:
- 고해상도 프레임, 또는 픽셀 단위(다운샘플 안 된) heatmap을 매 프레임 전송할 때 체감된다.
- 재생 중 실시간으로 heatmap을 갱신하는 시나리오에서 특히 병목이 된다.

**권장**:
```rust
#[tauri::command]
fn get_quality_heatmap(frame_index: u32) -> tauri::ipc::Response {
    let heatmap = STORE.compute_heatmap(frame_index);
    let mut buf = Vec::with_capacity(8 + heatmap.scores.len() * 4);
    buf.extend_from_slice(&heatmap.width.to_le_bytes());
    buf.extend_from_slice(&heatmap.height.to_le_bytes());
    for &s in &heatmap.scores {
        buf.extend_from_slice(&s.to_le_bytes()); // raw f32 바이너리, NaN도 그대로 왕복
    }
    tauri::ipc::Response::new(buf)
}
```
- 바이너리 프로토콜(raw `f32` 배열 또는 Tauri의 raw response)로 전송하고, 프론트엔드에서 `Float32Array`로 직접 매핑한다.
- NaN/Infinity는 IEEE754 비트 패턴 그대로 왕복시키고, 프론트엔드에서 명시적으로 처리한다(HEAT-012 참고).

**탐지 방법**:
- Structural: heatmap/score 배열을 반환하는 `#[tauri::command]`가 `serde_json::Value`나 `Vec<f32>`(JSON 직렬화되는 타입)를 쓰는지 검색.
- Runtime: 동일 payload를 JSON vs 바이너리로 전송했을 때 직렬화 시간·페이로드 크기 비교.

**예외**:
- 개발/디버그 빌드에서 사람이 읽기 쉬운 형태로 검사해야 하는 소규모(수십 개 이하) score만 다룰 때는 JSON도 허용된다.

**Bitvue 판정**: Suspected — 코드베이스 전체(`src-tauri/src`)에 `tauri::ipc::Response`/raw 바이너리 IPC 사용례가 0건이고, 모든 `#[tauri::command]`는 표준 serde(JSON) 직렬화 경로를 쓴다. `DiffHeatmapData`/`BlockMetricsGrid`는 `#[derive(Serialize, Deserialize)]`로 `Vec<f32>` 필드를 그대로 노출하므로, 만약 이 구조체가 향후 tauri command로 반환되면 기본 경로는 JSON float 배열이 될 것이다. 다만 현재 이 구조체를 반환하는 커맨드가 아예 없어(HEAT-003 참고) 실제 발생 여부는 확인 불가.

---

### HEAT-005: 매 frame마다 color map까지 backend에서 적용
**분류**: 관심사 분리 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn build_heatmap_rgba(scores: &[f32], colormap: &Colormap) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(scores.len() * 4);
    for &s in scores {
        let (r, g, b) = colormap.map(s); // 색 매핑을 backend가 매 프레임 수행
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    rgba // 이미 컬러가 입혀진 픽셀 버퍼를 IPC로 전송
}
```

**문제**:
- 사용자가 컬러맵(viridis, jet, diverging 등)을 바꾸거나 min/max 범위를 조정할 때마다 backend가 원본 score를 다시 계산·재매핑해야 해서 왕복 지연이 생긴다.
- score(f32, 4바이트) 대신 RGBA(4바이트)를 보내는 것 자체는 크기가 같아 보이지만, 색 매핑이 backend에 고정되면 프론트엔드가 즉시 컬러맵을 바꾸는 인터랙션(대비 조정, 팔레트 전환)을 할 수 없다.
- score와 시각화 색상이 한 번 합쳐지면 원본 수치를 복원할 수 없어 HEAT-006과 동일한 비가역성 문제가 발생한다.

**발생 조건**:
- 사용자가 실시간으로 컬러 팔레트나 min/max 범위를 조정하는 UI를 제공하는 경우 왕복 비용이 두드러진다.
- 여러 프레임에 걸쳐 일관된 컬러 스케일을 유지해야 하는 비교 작업에서 재계산 비용이 누적된다.

**권장**:
```rust
// backend: 순수 score만 전달
fn get_heatmap_scores(frame_index: u32) -> HeatmapPayload {
    STORE.compute_heatmap(frame_index).into_payload()
}
```
```ts
// frontend: GPU 셰이더 또는 캔버스에서 컬러맵 적용, 팔레트 변경 시 재요청 불필요
function applyColormap(scores: Float32Array, min: number, max: number, palette: Palette): Uint8ClampedArray { /* ... */ }
```
- score 계산(backend)과 색 매핑(frontend)을 분리해 컬러맵/범위 변경이 순수 프론트엔드 연산이 되게 한다.
- 프론트엔드는 GPU 셰이더(WebGL/WebGPU) 또는 캔버스 `ImageData`로 컬러맵을 실시간 적용한다.

**탐지 방법**:
- Structural: backend 코드에 `Colormap`/팔레트 룩업 테이블을 참조하는 함수가 있는지 검색.
- Manual: 컬러 팔레트 변경 UI 조작 시 IPC 재호출이 발생하는지 네트워크/IPC 로그로 확인.

**예외**:
- 정적 리포트/이미지 내보내기처럼 최종 산출물이 고정된 컬러 이미지여야 하는 경우는 backend(또는 export 전용 경로)에서 색을 입혀도 무방하다.

**Bitvue 판정**: N/A — backend에서 컬러맵을 적용하는 코드는 `export::overlay::create_diff_heatmap_export`(export/overlay.rs:299-330) 하나뿐이며, 이는 정확히 이 항목의 "예외"에 해당하는 export 전용 경로다. 인터랙티브 표시용으로 backend가 매 프레임 색을 입히는 라이브 커맨드는 존재하지 않는다(애초에 diff/quality heatmap을 노출하는 커맨드 자체가 없음).

---

### HEAT-006: score와 visualization color를 같은 데이터로 저장
**분류**: 자료구조 설계 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct HeatmapCell {
    rgba: [u8; 4], // 색상만 저장, 원본 score는 버려짐
}

fn build_heatmap(scores: &[f32], colormap: &Colormap) -> Vec<HeatmapCell> {
    scores.iter().map(|&s| HeatmapCell { rgba: colormap.map_to_rgba(s) }).collect()
}

// 이후 "이 픽셀의 실제 SSIM 값은?" 같은 질의가 들어오면 복원 불가능
```

**문제**:
- 컬러맵은 일반적으로 비가역 매핑(예: 8비트 RGBA로 양자화)이라 색상에서 원본 score를 정확히 복원할 수 없다.
- 사용자가 특정 블록을 클릭해 "정확한 SSIM 값"을 툴팁으로 보고 싶어도 데이터가 없어 재계산이 필요해진다.
- min/max 범위나 컬러맵을 바꾸는 것만으로도 원본 score 없이는 재매핑 자체가 불가능해 HEAT-005와 동일한 재계산 강제가 발생한다.

**발생 조건**:
- 툴팁/픽셀 인스펙터처럼 원본 수치를 다시 조회해야 하는 UI 기능이 있는 경우 특히 문제가 된다.
- 여러 컬러맵/임계값 프리셋을 전환하며 비교하는 워크플로에서 재계산 비용이 반복된다.

**권장**:
```rust
struct FrameHeatmap {
    scores: Vec<f32>, // 항상 원본 수치를 보존
}
// 색상은 렌더링 시점에만 파생되는 뷰(view)로 취급 — 저장하지 않는다
fn render_view(scores: &[f32], colormap: &Colormap, range: (f32, f32)) -> Vec<u8> {
    scores.iter().map(|&s| colormap.map(normalize(s, range))).flat_map(|(r,g,b)| [r,g,b,255]).collect()
}
```
- score는 항상 원본 정밀도(f32 등)로 보존하고, 색상은 렌더링 시점에만 파생되는 휘발성 뷰로 취급한다.
- 캐시가 필요하다면 "score 캐시"와 "렌더링된 색상 캐시"를 별도 계층으로 분리해, 색상 캐시는 언제든 폐기·재생성 가능하게 한다.

**탐지 방법**:
- Structural: heatmap 관련 구조체에 `f32`/score 필드 없이 `[u8; 4]`/RGBA 필드만 있는지 검색.
- Manual: "이 블록의 정확한 값" 조회 기능이 재계산 없이 동작하는지 확인.

**예외**:
- 아카이브/내보내기용 최종 이미지처럼 이후 수치 조회가 애초에 불필요한 산출물이라면 색상만 저장해도 된다.

**Bitvue 판정**: N/A — `DiffHeatmapData`는 원본 `values: Vec<f32>`(diff_heatmap.rs:87)를 항상 보존하고, RGBA는 `export::create_diff_heatmap_export`가 그때그때 파생시켜 별도의 `OverlayExportData`로 만들 뿐 원본을 덮어쓰지 않는다. `BlockMetricsOverlay`도 `grid`(raw f32)와 파생 `to_rgba()`가 분리되어 있다(block_metrics.rs:538-597).

---

### HEAT-007: block overlap 경계가 일관되지 않음
**분류**: 계산 정확성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_block_grid(width: u32, height: u32, block: u32) -> Vec<BlockRect> {
    let mut blocks = Vec::new();
    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            // 마지막 열/행에서 block 크기를 넘지 않도록 clamp하지만,
            // 인접 블록과 경계 픽셀을 두 번 세거나(overlap) 아예 빠뜨리는(gap) 경우를 처리하지 않음
            let w = block.min(width - x);
            let h = block.min(height - y);
            blocks.push(BlockRect { x, y, w, h });
            x += block; // width가 block의 배수가 아니면 다음 블록과 경계가 미세하게 어긋날 수 있음
        }
        y += block;
    }
    blocks
}
```

**문제**:
- 프레임 크기가 block 크기의 배수가 아닐 때 가장자리 블록의 크기가 줄어드는데, 이 축소된 블록의 score를 다른(완전한) 블록과 동일 가중치로 평균/집계하면 통계가 왜곡된다.
- 어떤 경로는 블록 간 겹침(overlap, 예: sliding window 방식)을 쓰고 다른 경로는 겹치지 않는 tiling을 써서 같은 좌표의 픽셀이 이중 집계되거나 아예 빠지는 불일치가 생긴다.
- 이 불일치는 시각적으로는 잘 드러나지 않고 프레임 전체 평균 score를 미묘하게 틀리게 만들어 발견이 늦어진다(semantic 버그).

**발생 조건**:
- 프레임 해상도가 block size의 배수가 아닌 경우(예: 1080 / 16 = 67.5) 항상 발생한다.
- 서로 다른 metric 구현(SSIM은 겹침 window, PSNR block은 비겹침 tiling)을 같은 그리드로 취급해 합성할 때 특히 위험하다.

**권장**:
```rust
fn compute_block_grid(width: u32, height: u32, block: u32) -> Vec<BlockRect> {
    let mut blocks = Vec::new();
    for y in (0..height).step_by(block as usize) {
        for x in (0..width).step_by(block as usize) {
            let w = block.min(width - x);
            let h = block.min(height - y);
            blocks.push(BlockRect { x, y, w, h, is_partial: w < block || h < block });
        }
    }
    blocks
}

fn aggregate_scores(blocks: &[(BlockRect, f32)]) -> f32 {
    // 부분 블록은 면적 가중 평균으로 집계 — 크기가 다른 블록을 동일 가중치로 섞지 않는다
    let (sum, area) = blocks.iter().fold((0.0, 0.0), |(s, a), (r, score)| {
        let w = (r.w * r.h) as f32;
        (s + score * w, a + w)
    });
    sum / area
}
```
- 경계 블록에 `is_partial` 플래그를 명시하고, 집계 시 면적 가중 평균을 사용한다.
- 겹침(overlap) 방식과 비겹침(tiling) 방식을 같은 그리드 좌표계로 섞지 않도록 metric별 그리드 규약을 하나로 통일하고 문서화한다.

**탐지 방법**:
- Semantic: block 크기의 배수가 아닌 해상도로 알려진 정답값(레퍼런스 SSIM 등)과 비교해 프레임 평균이 일치하는지 검증.
- Manual: 경계 블록의 크기와 가중치 처리 로직을 코드 리뷰로 확인.

**예외**:
- 프레임 해상도가 항상 block size의 배수로 고정된 파이프라인(예: 인코더가 이미 패딩된 해상도로만 출력)이라면 부분 블록 처리 자체가 불필요하다.

**Bitvue 판정**: Confirmed — `BlockMetricsGrid`(block_metrics.rs:130-152)는 `div_ceil`로 경계 블록을 만들고 `block_to_pixel_range`(block_metrics.rs:196-203)가 프레임 경계로 clamp하지만 `is_partial` 플래그가 없고, 집계 함수 `BlockMetricsStatistics::from_grid`(block_metrics.rs:231-272)와 `MultiFrameBlockMetrics::aggregate_statistics`(block_metrics.rs:640-680)는 전부 `values.iter().sum()/values.len()` 식 단순 평균으로, 잘린 경계 블록과 완전 블록을 동일 가중치로 섞는다(면적 가중 없음). 다만 실제 plane 데이터에서 블록을 잘라 `BlockMetricsCalculator::calculate_block`을 호출하는 driver 코드가 코드베이스 어디에도 없어(`BlockMetricsGrid::new`/`BlockMetricsCalculator` 호출부가 block_metrics.rs 자체 테스트 외에 없음), 이 경계 처리가 실제 프레임에서 발동하는지는 구조적 증거로만 확인됨.

---

### HEAT-008: stride와 window 크기를 결과에 기록하지 않음
**분류**: 메타데이터 누락 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct FrameHeatmap {
    width: u32,
    height: u32,
    scores: Vec<f32>, // 이 grid가 8px stride인지 16px window인지 결과만 봐서는 알 수 없음
}

fn compute_heatmap(a: &YPlane, b: &YPlane, window: u32, stride: u32) -> FrameHeatmap {
    let grid = sliding_window_ssim(a, b, window, stride);
    FrameHeatmap { width: grid.cols, height: grid.rows, scores: grid.into_vec() }
}
```

**문제**:
- 결과 구조체에 window/stride 정보가 없으면, 이후 이 heatmap을 소비하는 코드(캐시 조회, 프레임 간 비교, export)가 어떤 파라미터로 생성됐는지 알 수 없다.
- window=8/stride=8(비겹침)로 만든 heatmap과 window=16/stride=4(겹침 많음)로 만든 heatmap은 같은 `width x height` grid라도 통계적 의미가 전혀 다른데, 필드가 없으면 둘을 실수로 같은 것처럼 비교하게 된다.
- 캐시 키에 파라미터가 반영되지 않으면 사용자가 window 크기를 바꿔도 이전 결과가 그대로 재사용되는 버그(HEAT-017과 연결)로 이어진다.

**발생 조건**:
- 사용자가 UI에서 block/window 크기를 조정할 수 있는 기능이 있는 경우 특히 위험하다.
- 여러 metric(SSIM/PSNR/VMAF-per-block)이 서로 다른 기본 window를 쓰는 파이프라인에서 결과를 섞어 쓸 때 문제가 드러난다.

**권장**:
```rust
struct HeatmapMetadata {
    window: u32,
    stride: u32,
    metric: MetricKind,
    source_resolution: (u32, u32), // raw 원본 해상도(HEAT-015 참고)
}

struct FrameHeatmap {
    meta: HeatmapMetadata,
    width: u32,
    height: u32,
    scores: Vec<f32>,
}
```
- window, stride, metric 종류, 원본 해상도를 결과에 항상 동반시켜 결과만 보고도 재현·비교 가능하게 한다.
- 캐시 키는 `(frame_index, metric, window, stride, resolution)` 전체를 포함시킨다.

**탐지 방법**:
- Static: heatmap 결과 구조체 정의에 window/stride/metric 필드가 있는지 검사.
- Structural: heatmap을 캐싱하는 코드의 캐시 키 구성 요소를 검색해 파라미터 누락 여부 확인.

**예외**:
- window/stride가 애플리케이션 전역에서 상수로 고정되어 절대 바뀌지 않는다면 매 결과마다 반복 기록하는 대신 전역 문서화만으로 충분할 수 있다.

**Bitvue 판정**: N/A — 이미 권장안대로 구현됨. `BlockMetricsGrid`는 `block_size`/`metric_type`/`frame_width`/`frame_height`를 결과 구조체 자체에 항상 동반한다(block_metrics.rs:111-128). `DiffHeatmapData`도 `mode`와 `heatmap_width/height` 대 `frame_width/height` 비율(다운샘플 배율을 항상 역산 가능)을 함께 저장한다(diff_heatmap.rs:73-97). 두 구조체 모두 window==block_size(비겹침, 별도 stride 개념 없음)라 "window/stride가 결과에 없는" 문제 자체가 발생하지 않는다.

---

### HEAT-009: heatmap normalization을 frame별로 다르게 수행
**분류**: 계산 정확성 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn normalize_for_display(scores: &[f32]) -> Vec<f32> {
    let min = scores.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    // 매 프레임마다 그 프레임 자신의 min/max로 0..1 정규화
    scores.iter().map(|&s| (s - min) / (max - min).max(1e-6)).collect()
}
```

**문제**:
- 각 프레임이 자기 자신의 min/max 기준으로 독립적으로 정규화되면, 실제로는 프레임 5의 최저 품질(0.3)이 프레임 20의 최저 품질(0.05)보다 훨씬 나쁜데도 두 프레임 모두 히트맵에서 "가장 붉은 색(0.0)"으로 표시되어 동일하게 보인다.
- 사용자가 필름스트립을 스크러빙하며 프레임 간 품질 변화 추이를 시각적으로 파악하려는 목적 자체가 무너진다 — 색은 절대 품질이 아니라 "그 프레임 내 상대 순위"만 나타내게 된다.
- 이 문제는 단일 프레임만 볼 때는 전혀 드러나지 않고, 여러 프레임을 비교하거나 순차 재생할 때만 나타나므로 발견이 어렵다(semantic 버그의 전형).

**발생 조건**:
- 프레임 간 품질 비교, 시간축 트렌드 시각화, 필름스트립 오버뷰처럼 여러 프레임을 동시에/순차적으로 보여주는 모든 기능에서 발생한다.
- 특히 장면 전환 등으로 프레임별 품질 분산이 큰 콘텐츠에서 왜곡이 심하다.

**권장**:
```rust
fn normalize_for_display(scores: &[f32], global_range: (f32, f32)) -> Vec<f32> {
    let (min, max) = global_range; // 시퀀스 전체(또는 명시적으로 고정된) 범위
    scores.iter().map(|&s| ((s - min) / (max - min).max(1e-6)).clamp(0.0, 1.0)).collect()
}

fn compute_global_range(all_frame_scores: &[Vec<f32>]) -> (f32, f32) {
    // 전체 시퀀스(또는 metric별 알려진 이론적 범위, 예: SSIM은 0..1)를 기준으로 고정
    all_frame_scores.iter().flatten().fold((f32::INFINITY, f32::NEG_INFINITY),
        |(mn, mx), &s| (mn.min(s), mx.max(s)))
}
```
- 정규화 범위는 프레임 단위가 아니라 시퀀스 전체(또는 metric의 이론적 고정 범위, 예: SSIM 0..1)로 고정한다.
- 사용자가 명시적으로 "현재 프레임만 강조"를 선택했을 때만 프레임별 정규화를 옵션으로 제공하고, 기본값은 항상 전역 정규화로 둔다.

**탐지 방법**:
- Semantic: 서로 다른 두 프레임에서 동일 색상이 나타내는 절대 score 값이 다른지 회귀 테스트로 검증.
- Manual: 필름스트립에서 품질이 크게 다른 두 프레임을 나란히 놓고 색상 대비가 실제 수치 차이를 반영하는지 육안 검토.

**예외**:
- 단일 프레임의 내부 상대 분포(그 프레임 안에서 어디가 상대적으로 나쁜가)만 보여주는 것이 명시적 목적인 UI 모드라면 프레임별 정규화가 의도된 설계다(단, HEAT-010처럼 반드시 UI에 그 사실을 표시해야 한다).

**관련**: `UIX_VIZ.md` UIX-VIZ-001 참고 — 동일한 per-frame auto-normalize 패턴이나 VQ-Probe 품질-score heatmap(여기)과 QP heatmap(Bitvue, UIX-VIZ-001)은 별도 서브시스템.

**Bitvue 판정**: Confirmed — `DiffHeatmapData::from_luma_planes`는 매 호출마다 그 프레임 자신의 `min_value`/`max_value`를 로컬로 계산해 저장하고(diff_heatmap.rs:118-119, 160-161), `get_normalized`(diff_heatmap.rs:187-201)는 항상 `self.min_value`/`self.max_value`만 사용한다 — 시퀀스 전체나 metric의 이론적 고정 범위를 받는 파라미터가 전혀 없다. 다만 `BlockMetricsGrid`/`BlockMetricValue::normalized()`(block_metrics.rs:94-102, 174-184)는 반대로 `metric_type.typical_range()`라는 metric별 고정 범위를 쓰고 있어 이 문제가 없다 — 같은 도메인 안에서 두 heatmap 구현의 정규화 전략이 서로 다르다는 점 자체도 주목할 만하다.

---

### HEAT-010: 자동 min/max 때문에 프레임 간 시각 비교 불가능
**분류**: UX/계산 정확성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
#[tauri::command]
fn get_heatmap_for_display(frame_index: u32) -> HeatmapPayload {
    let heatmap = STORE.compute_heatmap(frame_index);
    let (min, max) = auto_range(&heatmap.scores); // 항상 "이 프레임에 맞춰" 자동 계산
    HeatmapPayload { scores: normalize(&heatmap.scores, min, max), range_used: None } // 어떤 범위를 썼는지도 안 알려줌
}
```

**문제**:
- HEAT-009와 원인은 같지만 여기서는 "범위를 사용자가 고정할 수 있는 옵션이 아예 없다"는 UX 설계 결함이 핵심이다 — auto-range가 항상 강제된다.
- 응답에 실제 사용된 min/max조차 포함하지 않아, 프론트엔드가 범례(legend)에 "이 색은 몇 점을 의미하는지"조차 표시할 수 없다.
- 스트림 A와 스트림 B를 나란히 비교하는 dual-stream 워크플로에서 두 히트맵이 각자 다른 auto-range로 그려지면, 같은 색이 서로 다른 절대 품질을 의미해 비교 자체가 무의미해진다.

**발생 조건**:
- 두 스트림/두 프레임을 나란히(side-by-side) 비교하는 모든 뷰에서 발생한다.
- 사용자가 명시적으로 "0.8~1.0 구간만 확대해서 보고 싶다"는 식의 커스텀 범위 요구가 있는 경우 기능 자체가 불가능해진다.

**권장**:
```rust
#[tauri::command]
fn get_heatmap_for_display(frame_index: u32, range: Option<(f32, f32)>) -> HeatmapPayload {
    let heatmap = STORE.compute_heatmap(frame_index);
    let range_used = range.unwrap_or_else(|| STORE.global_range_for_metric(heatmap.meta.metric));
    HeatmapPayload {
        scores: normalize(&heatmap.scores, range_used.0, range_used.1),
        range_used: Some(range_used), // 범례 렌더링을 위해 항상 반환
    }
}
```
- 범위를 명시적 파라미터로 받고, 미지정 시에도 "그때그때 auto"가 아니라 metric별 고정 기본 범위를 사용한다.
- 실제 사용된 범위를 응답에 포함시켜 프론트엔드 범례가 항상 정확한 스케일을 표시하게 한다.

**탐지 방법**:
- Structural: heatmap 응답 payload에 사용된 min/max(range)가 포함되는지 검사.
- Manual: dual-stream 비교 뷰에서 양쪽 범례 스케일이 동일한지 확인.

**예외**:
- 단일 프레임 단독 상세 검사 모드(비교 목적이 아닌)에서는 auto-range가 대비를 극대화해 오히려 유용할 수 있다 — 이 경우 UI에 "auto-range 사용 중"임을 명시해야 한다.

**관련**: `UIX_VIZ.md` UIX-VIZ-004 참고 — legend가 실제 값을 반영하지 못하는 문제이나, 여기는 backend가 사용된 range를 응답에 포함하지 않는 것이 원인, UIX-VIZ-004는 legend 컴포넌트 자체의 부재/은닉이 원인.

**Bitvue 판정**: Confirmed (부분) — `DiffHeatmapData`는 `min_value`/`max_value`를 public 필드로 직렬화해 반환하므로(diff_heatmap.rs:92-96) "범위를 아예 안 알려줌"이라는 최악의 경우는 피했지만, 사용자가 범위를 고정 지정할 옵션 자체가 없다(`from_luma_planes` 시그니처에 range 파라미터 없음, diff_heatmap.rs:103-109) — auto-range가 항상 강제된다. dual-stream 비교 시 스트림 A/B 히트맵이 각자 다른 auto-range를 갖게 될지 검증할 IPC 경로가 없어(HEAT-003 참고) 실사용 영향은 미확인.

---

### HEAT-011: outlier가 전체 color range를 왜곡
**분류**: 계산 정확성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_display_range(scores: &[f32]) -> (f32, f32) {
    // 단순 min/max — 극단값 하나가 전체 스케일을 지배
    let min = scores.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    (min, max)
}
```

**문제**:
- 디코딩 오류, 경계 블록 아티팩트, 또는 극단적으로 나쁜 프레임 하나 때문에 score 분포에 long-tail outlier가 섞이면, min/max 기반 정규화가 그 outlier를 기준으로 스케일을 잡아버려 나머지 정상 범위(예: 대부분 0.7~0.95)가 색상 스펙트럼의 좁은 구간에 뭉개진다.
- 결과적으로 히트맵이 거의 단색으로 보이고, 실제로 의미 있는 품질 차이(정상 범위 내의 변화)는 시각적으로 구분되지 않는다.
- outlier 자체는 관심 대상(예: 심각한 열화 지점을 찾는 것이 목적)일 수도 있으므로 무조건 제거하면 안 되고, 시각화 스케일과 outlier 탐지를 분리해야 한다.

**발생 조건**:
- 부분 디코딩 실패, 프레임 경계 아티팩트, 또는 metric 계산 버그로 인한 비정상 score가 섞인 콘텐츠에서 흔하다.
- score 분포가 원래도 긴 꼬리를 가지는 metric(일부 지각 품질 metric)에서 구조적으로 발생하기 쉽다.

**권장**:
```rust
fn compute_display_range(scores: &[f32], percentile_clip: (f32, f32)) -> (f32, f32) {
    let mut sorted: Vec<f32> = scores.iter().cloned().filter(|s| s.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = percentile(&sorted, percentile_clip.0); // 예: 1st percentile
    let hi = percentile(&sorted, percentile_clip.1); // 예: 99th percentile
    (lo, hi) // 시각화 범위는 percentile clip, 값 자체는 clamp만 하고 보존
}
```
- 시각화 범위는 min/max 대신 percentile clipping(예: 1~99%)으로 산출해 outlier에 대한 강건성을 확보한다.
- outlier 자체는 데이터에서 제거하지 않고, 범위를 벗어난 값은 색상 스케일 양 끝(saturate)으로 클램프해 "범위 밖에 뭔가 있다"는 정보(경계선 강조 등)를 UI에서 별도로 표시한다.

**탐지 방법**:
- Semantic: 인위적으로 outlier를 주입한 합성 데이터로 정규화 범위가 얼마나 흔들리는지 회귀 테스트.
- Manual: 히트맵이 대부분 단색으로 보이는 프레임을 발견하면 score 분포 히스토그램을 뽑아 outlier 존재를 확인.

**예외**:
- 이상치 탐지가 명시적 목적인 뷰(예: "가장 나쁜 블록 찾기" 모드)에서는 min/max 전체 범위를 그대로 쓰는 것이 오히려 맞다.

**관련**: `UIX_VIZ.md` UIX-VIZ-023 참고 — outlier가 autoscale 범위를 왜곡하는 동일 문제이나, 여기는 heatmap color range, UIX-VIZ-023은 시계열 차트 Y축 대상.

**Bitvue 판정**: Confirmed — `DiffHeatmapData::from_luma_planes`/`get_normalized`는 순수 min/max만 사용하고(diff_heatmap.rs:118-119, 191-200) percentile clipping 로직이 없다. 흥미롭게도 코드베이스에 `percentile()` 유틸리티 자체는 이미 존재하지만(`metrics_distribution.rs:179`, 5th/50th/95th percentile 계산용) diff heatmap 정규화 경로와는 연결되어 있지 않다. `BlockMetricsGrid`는 metric별 고정 `typical_range()`를 쓰므로(block_metrics.rs:52-59) outlier에 의한 스케일 왜곡 자체가 구조적으로 발생하지 않는다.

---

### HEAT-012: NaN 위치를 0점으로 표시
**분류**: 계산 정확성 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
fn compute_block_score(a: &[u8], b: &[u8]) -> f32 {
    let variance = compute_variance(a); // 완전히 평탄한(uniform) 블록이면 0
    if variance == 0.0 {
        return f32::NAN; // SSIM 등에서 분모가 0이 되어 NaN 발생 가능
    }
    ssim_formula(a, b, variance)
}

fn normalize(scores: &[f32], min: f32, max: f32) -> Vec<f32> {
    scores.iter().map(|&s| {
        let v = (s - min) / (max - min);
        if v.is_nan() { 0.0 } else { v } // NaN을 "최저 품질(0점, 진한 빨강)"로 표시
    }).collect()
}
```

**문제**:
- NaN은 "계산 불가"를 의미하는데, 이를 0.0(정규화 후 최저 품질)으로 치환하면 "이 블록은 계산할 수 없었다"와 "이 블록은 실제로 품질이 매우 나쁘다"가 시각적으로 구분되지 않는다.
- 완전 평탄한 배경(하늘, 단색 벽 등)처럼 콘텐츠 특성상 자연스럽게 NaN이 자주 나오는 블록이 항상 "가장 나쁜 색"으로 칠해지면, 사용자가 실제 열화 지점을 찾는 데 오히려 방해가 된다.
- 프레임 평균 score를 집계할 때도 NaN이 섞인 채로 합산되면 전체가 NaN이 되거나(방치 시), 0으로 치환된 값이 평균을 부당하게 끌어내려 실제 품질보다 나쁘게 보고된다.

**발생 조건**:
- SSIM처럼 분모에 분산/공분산이 들어가는 metric에서 완전 평탄한 블록(그레이디언트 배경, 검은 레터박스 등)을 만날 때 구조적으로 발생한다.
- 두 스트림 중 한쪽이 해당 영역을 디코딩하지 못했거나 크롭/패딩된 경우에도 NaN이 나올 수 있다.

**권장**:
```rust
enum ScoreCell {
    Valid(f32),
    Undefined(UndefinedReason), // FlatBlock, DecodeFailure, OutOfBounds 등
}

fn normalize(scores: &[ScoreCell], min: f32, max: f32) -> Vec<Option<f32>> {
    scores.iter().map(|c| match c {
        ScoreCell::Valid(s) => Some(((s - min) / (max - min)).clamp(0.0, 1.0)),
        ScoreCell::Undefined(_) => None, // 색상 팔레트가 아닌 별도 패턴(해칭/회색)으로 렌더링
    }).collect()
}

fn aggregate_mean(scores: &[ScoreCell]) -> f32 {
    let valid: Vec<f32> = scores.iter().filter_map(|c| match c {
        ScoreCell::Valid(s) => Some(*s),
        ScoreCell::Undefined(_) => None,
    }).collect();
    valid.iter().sum::<f32>() / valid.len().max(1) as f32 // NaN을 집계에서 명시적으로 제외
}
```
- "계산 불가"를 `Option`/전용 enum variant로 명시적으로 표현하고 절대 유효 점수(특히 최저점)로 치환하지 않는다.
- 렌더링 시 undefined 셀은 색상 스펙트럼이 아닌 별도 시각 언어(회색, 해칭 패턴, 투명)로 구분한다.
- 집계(평균/백분위 등)는 undefined 셀을 분모/분자에서 제외하고, 제외된 비율 자체도 함께 보고한다(예: "유효 블록 98.2%").

**탐지 방법**:
- Runtime: 완전 평탄한 합성 블록 입력에 대해 metric 계산 결과가 NaN인지, 그리고 그 NaN이 어떻게 소비되는지 단위 테스트.
- Static: `is_nan()` 체크 후 리터럴 0.0/최저값으로 치환하는 패턴을 검색.

**예외**:
- 없음 — NaN을 유효 점수로 치환하는 것은 항상 정보 손실이며, 유일한 논의 대상은 "어떻게 표시할지"이지 "치환해도 되는지"가 아니다.

**관련**: `UIX_VIZ.md` UIX-VIZ-005 참고 — invalid/NaN 데이터를 유효 극단값(0)으로 치환해 오인시키는 동일 패턴. score=0은 "최하"로, QP=0은 "최상"으로 위장돼 방향은 반대.

**Bitvue 판정**: Confirmed — `diff_heatmap.rs`/`block_metrics.rs` 전체에 `is_nan()` 체크나 NaN 관련 처리가 0건(grep 결과 없음). `values`/`scores` 필드는 어디서도 `Option<f32>`나 undefined-reason enum으로 감싸지지 않은 순수 `f32`다. 실제 NaN 발생 경로도 존재한다: `calculate_ssim_block`(block_metrics.rs:380-416)은 `var_s /= n - 1.0`을 수행하는데 block이 픽셀 1개(n=1)면 `0.0/0.0`으로 NaN이 나온다(block_metrics.rs:404-406, 가드 없음). 이 NaN이 `normalized()`/`map_color()`(block_metrics.rs:499-520)를 거치면 Rust의 `f32 as u8` 캐스트가 NaN을 0으로 saturate시켜 픽셀이 조용히 검정/투명이 되며, `BlockMetricsStatistics::from_grid`의 `values.iter().sum()`(block_metrics.rs:249)은 NaN 하나로 전체 평균을 NaN으로 오염시킨다 — 원안의 "NaN→0점으로 치환" 정확히 그 형태는 아니지만 "유효/무효를 구분하는 명시적 타입이 전혀 없어 NaN이 조용히 전파·오염된다"는 핵심 문제는 동일하게 present.

---

### HEAT-013: ROI 밖 영역을 평균에 포함
**분류**: 계산 정확성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_roi_mean_score(heatmap: &FrameHeatmap, roi: &Rect) -> f32 {
    // ROI로 필터링한다고 이름은 붙였지만 실제로는 전체 grid를 순회
    let sum: f32 = heatmap.scores.iter().sum();
    sum / heatmap.scores.len() as f32 // roi 파라미터가 사용되지 않음
}
```

**문제**:
- 함수 이름과 시그니처는 ROI 평균을 계산하는 것처럼 보이지만 실제로는 프레임 전체 평균을 반환해, 사용자가 관심 영역(예: 얼굴, 텍스트 자막 영역)만 골라 품질을 확인하려는 의도가 조용히 무시된다.
- ROI 경계가 block 격자와 정확히 맞아떨어지지 않는 경우(픽셀 단위 ROI vs 블록 단위 grid), 경계에 걸친 블록을 전부 포함시킬지 부분적으로만 포함시킬지 규칙이 없으면 ROI 안팎 경계에서 값이 부정확해진다.
- 이 버그는 ROI 기능을 실제로 테스트하지 않으면(즉 "ROI를 지정했을 때와 전체 프레임일 때 값이 달라지는지" 검증하지 않으면) 코드 리뷰만으로는 잡히지 않는다.

**발생 조건**:
- ROI 지정 UI(드래그로 사각형 선택 등)가 있는 모든 기능에서, 특히 초기 구현 단계나 리팩터링 이후 회귀로 발생하기 쉽다.
- ROI가 block 경계와 정렬되지 않는 임의 사각형일 때 경계 처리 버그가 두드러진다.

**권장**:
```rust
fn compute_roi_mean_score(heatmap: &FrameHeatmap, roi: &Rect) -> f32 {
    let (mut sum, mut weight) = (0.0f32, 0.0f32);
    for block in heatmap.blocks_overlapping(roi) {
        let overlap_area = block.rect.intersect(roi).area(); // 블록-ROI 겹침 면적만 가중
        if overlap_area > 0.0 {
            sum += block.score * overlap_area;
            weight += overlap_area;
        }
    }
    sum / weight.max(1e-6)
}
```
- ROI와 겹치는 블록만 순회하고, 경계에 걸친 블록은 실제 겹침 면적으로 가중 평균한다.
- ROI 지정 여부가 결과에 실제로 반영되는지 확인하는 회귀 테스트(전체 프레임 평균과 ROI 평균이 달라야 하는 합성 케이스)를 반드시 둔다.

**탐지 방법**:
- Semantic: ROI를 프레임의 절반 등 명확히 다른 영역으로 지정했을 때 결과가 전체 평균과 달라지는지 자동 테스트.
- Static: ROI 파라미터를 받는 함수 본문에서 해당 파라미터가 실제로 필터링에 쓰이는지 데이터 흐름 검사.

**예외**:
- ROI가 명시적으로 "전체 프레임"으로 지정된 경우(기본값)라면 전체 평균과 동일한 것이 당연히 맞다.

**Bitvue 판정**: N/A — `diff_heatmap.rs`/`block_metrics.rs`에 ROI 개념(사각형 선택, `compute_roi_mean_score`류 함수) 자체가 존재하지 않는다. 코드베이스 전체에서 "roi"로 검색되는 유일한 결과는 `bitvue-vp9/src/overlay_extraction.rs`의 VP9 인코더 ROI(적응형 비트레이트 영역 인코딩) 언급뿐으로 이 항목과 무관하다. 아직 구현되지 않은 기능이라 해당 anti-pattern이 적용될 대상이 없다.

---

### HEAT-014: 타일 경계 artifact가 metric 결과에 발생
**분류**: 계산 정확성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_tiled_heatmap(a: &YPlane, b: &YPlane, tile_size: u32) -> FrameHeatmap {
    let mut scores = Vec::new();
    for tile in split_into_tiles(a.width, a.height, tile_size) {
        // 각 타일을 독립적으로 잘라 SSIM 윈도우를 적용 — 타일 경계에서 윈도우가 인접 타일 데이터를 볼 수 없음
        let a_tile = a.crop(&tile);
        let b_tile = b.crop(&tile);
        scores.extend(sliding_window_ssim(&a_tile, &b_tile, WINDOW));
    }
    scores
}
```

**문제**:
- SSIM 같은 sliding-window metric은 윈도우가 픽셀 주변 이웃을 참조해야 하는데, 타일을 독립적으로 자르면 타일 경계 근처의 윈도우가 실제로는 존재하는 인접 타일의 픽셀을 보지 못해 잘못된(보통 더 낮은) score를 계산한다.
- 그 결과 타일 격자 모양의 "가짜 열화 패턴"이 히트맵에 나타나는데, 이는 실제 비디오 품질과 무관한 순수 계산 아티팩트다.
- 사용자가 이 격자 무늬를 실제 인코딩 블록 경계 열화(블로킹 아티팩트)로 오인해 잘못된 결론(예: "인코더가 블록 경계에서 화질을 망친다")을 내릴 위험이 있다 — 계산 도구 자체의 버그가 콘텐츠 품질 이슈로 둔갑한다.

**발생 조건**:
- 병렬 처리를 위해 프레임을 타일로 나눠 각 스레드/워커가 독립적으로 처리하는 구현에서 구조적으로 발생한다.
- window/kernel 크기가 클수록(더 넓은 이웃을 참조할수록) 경계 아티팩트의 영향 범위도 커진다.

**권장**:
```rust
fn compute_tiled_heatmap(a: &YPlane, b: &YPlane, tile_size: u32, window: u32) -> FrameHeatmap {
    let halo = window / 2; // 윈도우가 필요로 하는 이웃 픽셀 폭만큼 여유(halo)를 둔다
    let mut scores = Vec::new();
    for tile in split_into_tiles(a.width, a.height, tile_size) {
        let padded = tile.expand_clamped(halo, a.width, a.height); // halo만큼 확장해서 크롭
        let a_tile = a.crop(&padded);
        let b_tile = b.crop(&padded);
        let tile_scores = sliding_window_ssim(&a_tile, &b_tile, window);
        scores.extend(tile_scores.crop_to_inner(halo)); // 결과에서 halo 영역은 버리고 실제 타일분만 채택
    }
    scores
}
```
- 타일을 자를 때 metric의 윈도우 크기만큼 여유(halo/padding)를 두고 이웃 타일의 픽셀을 함께 읽은 뒤, 계산 후 halo 영역을 잘라내 순수 타일 결과만 채택한다.
- 병렬화가 정확성에 영향을 주지 않는지 확인하는 회귀 테스트(타일 분할 없이 전체 프레임을 한 번에 계산한 결과와 타일 분할 결과를 비교)를 둔다.

**탐지 방법**:
- Semantic: 타일 분할 계산 결과와 비분할(단일 패스) 계산 결과를 동일 입력에 대해 비교하는 회귀 테스트 — 타일 경계 근처에서만 차이가 나면 이 버그다.
- Manual: 히트맵에서 타일 크기와 정확히 일치하는 격자 무늬가 보이는지 육안 확인.

**예외**:
- 타일 간 완전 독립이 의도된 non-overlapping block metric(윈도우가 없는 단순 블록 평균 등)이라면 halo가 애초에 불필요하다.

**Bitvue 판정**: N/A — `block_metrics.rs`의 SSIM/PSNR/MSE/MAD 계산은 처음부터 sliding-window가 아니라 넘겨받은 block 슬라이스 내부만으로 통계를 내는 non-overlapping block metric으로 설계되어 halo가 필요 없다. `diff_heatmap.rs`의 2x2 다운샘플도 단일 패스 nested loop(diff_heatmap.rs:122-163)로 전체 plane을 한 번에 처리하며 타일 분할/병렬화가 없다. 두 파일 모두 `par_iter`/`rayon`/`tile` 관련 코드가 0건이라 "타일을 독립적으로 잘라 병렬 처리"하는 경로 자체가 존재하지 않는다.

---

### HEAT-015: raw heatmap과 downsampled heatmap 혼용
**분류**: 계산 정확성 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn get_heatmap_for_zoom(frame_index: u32, zoom: ZoomLevel) -> FrameHeatmap {
    let cached = STORE.get_cached_heatmap(frame_index); // 캐시가 어느 해상도로 계산됐는지 모름
    match cached {
        Some(h) => h, // 원래 25% 다운샘플로 계산된 heatmap을 100% 줌 요청에도 그대로 반환
        None => STORE.compute_heatmap_full(frame_index),
    }
}
```

**문제**:
- 다운샘플된 plane에서 계산한 heatmap(HEAT-002의 권장안)은 원본 해상도에서 계산한 heatmap과 픽셀당 의미가 다르다 — 다운샘플 버전은 여러 원본 블록의 평균/보간이 섞인 "뭉개진" score다.
- 캐시나 API가 "어떤 해상도에서 계산됐는지" 구분하지 않고 하나의 `FrameHeatmap`으로 취급하면, 줌 인 했을 때 실제로는 저해상도로 계산된 뭉툭한 heatmap이 마치 원본 정밀도인 것처럼 확대되어 표시된다.
- 두 스트림을 비교할 때 한쪽은 raw로, 다른 쪽은 실수로 downsampled 캐시가 재사용되면 비교 자체가 의미를 잃는다(서로 다른 정밀도의 숫자를 나란히 비교).

**발생 조건**:
- 줌 레벨에 따라 해상도를 달리 계산하는 최적화(HEAT-002/003)를 도입한 직후, 캐시 키에 해상도가 반영되지 않았을 때 발생한다.
- 사용자가 줌 아웃 상태에서 본 뒤 빠르게 줌 인하는 인터랙션에서 캐시 미스/히트 전환 시점에 드러난다.

**권장**:
```rust
#[derive(PartialEq, Eq, Hash)]
struct HeatmapCacheKey {
    frame_index: u32,
    metric: MetricKind,
    resolution_tag: ResolutionTag, // Raw | Downsampled(factor)
}

fn get_heatmap_for_zoom(frame_index: u32, zoom: ZoomLevel) -> FrameHeatmap {
    let tag = resolution_tag_for_zoom(zoom);
    let key = HeatmapCacheKey { frame_index, metric: MetricKind::Ssim, resolution_tag: tag };
    STORE.get_or_compute(key, || STORE.compute_heatmap_at(frame_index, tag))
}
```
- 캐시 키와 결과 메타데이터(HEAT-008)에 해상도 태그를 반드시 포함시켜 raw와 downsampled를 절대 같은 키로 취급하지 않는다.
- 비교 뷰(dual-stream)에서는 양쪽 스트림의 해상도 태그가 반드시 일치하도록 요청 시점에 검증한다.

**탐지 방법**:
- Semantic: 동일 프레임을 여러 줌 레벨로 연속 요청했을 때 반환되는 heatmap의 해상도 태그가 요청과 일치하는지 회귀 테스트.
- Structural: 캐시 키 정의에 해상도/줌 관련 필드가 빠져 있는지 검색.

**예외**:
- 애초에 다운샘플 경로 자체를 두지 않고 항상 raw만 계산하는 단순한 파이프라인이라면 이 혼용 문제 자체가 발생하지 않는다(대신 HEAT-002의 연산 낭비 트레이드오프를 감수해야 한다).

**Bitvue 판정**: N/A — `DiffHeatmapData::from_luma_planes`가 만드는 해상도는 고정 2x2 half-res 하나뿐이고(diff_heatmap.rs:113-115) "raw(1:1)"로 계산하는 경로 자체가 코드베이스에 없어 raw/downsampled를 혼동할 여지가 없다. `CacheKey::DiffHeatmap`(cache_provenance.rs:63-69)도 `hm_res` 필드를 캐시 키에 포함시켜 해상도별 분리 의도를 갖추고 있다. 다만 `diff_heatmap.rs` 자체의 `DiffHeatmapData::cache_key()`(문자열 포맷, diff_heatmap.rs:228-240)와 `cache_provenance.rs`의 `CacheKey::DiffHeatmap`(enum, 필드 구성이 다름 — 전자는 `codec`/파일해시/`opacity_bucket` 포함, 후자는 `ab_mapping` 포함하고 opacity는 없음)이 서로 겹치는 두 개의 별도 캐시 키 체계로 존재해 향후 실제 caching을 연결할 때 둘 중 어느 것을 신뢰할지 혼선의 소지는 있다(HEAT-017 참고).

---

### HEAT-016: 델타(차이) heatmap에 sequential colormap을 사용
**분류**: 시각화 정확성 · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
fn colormap_for_metric(metric: MetricKind) -> Colormap {
    // 절대 품질 heatmap이든, 스트림 A-B 차이(delta) heatmap이든 항상 같은 순차 팔레트 사용
    Colormap::viridis()
}

fn compute_delta_heatmap(a: &FrameHeatmap, b: &FrameHeatmap) -> FrameHeatmap {
    let scores = a.scores.iter().zip(&b.scores).map(|(x, y)| x - y).collect();
    FrameHeatmap { scores, ..a.clone() } // 부호(A가 더 좋은지 B가 더 좋은지)를 갖는 데이터
}
```

**문제**:
- 델타 heatmap의 값은 0을 기준으로 양/음 부호가 "어느 스트림이 더 나은가"라는 질적으로 다른 의미를 가지는데, viridis 같은 순차(sequential) 팔레트는 낮은 값과 높은 값을 단조 증가하는 밝기로만 표현해 "0 근처(차이 없음)"와 "큰 음수(B가 훨씬 나음)"를 시각적으로 구분하기 어렵게 만든다.
- 사용자가 색의 밝기만으로 "더 나쁘다/더 좋다"를 직관적으로 읽을 수 없어, 결국 툴팁으로 정확한 값을 일일이 확인해야 하는 UX 저하로 이어진다.
- 절대 품질 heatmap과 델타 heatmap을 같은 팔레트 함수로 처리하면, 이후 팔레트를 바꾸는 리팩터링에서 한쪽만 고려하고 다른 쪽을 깨뜨리기 쉽다.

**발생 조건**:
- dual-stream 비교 기능에서 "A 대비 B의 차이"를 직접 시각화하는 모든 델타/diff 뷰에서 발생한다.
- 절대 score용 팔레트 선택 로직을 델타 heatmap에 그대로 재사용할 때 특히 흔하다.

**권장**:
```rust
fn colormap_for_metric(kind: HeatmapKind) -> Colormap {
    match kind {
        HeatmapKind::Absolute => Colormap::viridis(),      // 순차형: 낮음→높음
        HeatmapKind::Delta => Colormap::diverging_rdbu(),   // 발산형: 음수(파랑)-0(흰색)-양수(빨강)
    }
}

fn compute_delta_heatmap(a: &FrameHeatmap, b: &FrameHeatmap) -> FrameHeatmap {
    let scores = a.scores.iter().zip(&b.scores).map(|(x, y)| x - y).collect();
    FrameHeatmap { scores, kind: HeatmapKind::Delta, ..a.clone() }
}
```
- 절대값 heatmap은 순차(sequential) 팔레트, 부호 있는 델타/diff heatmap은 0을 중심으로 대칭인 발산(diverging) 팔레트를 사용한다.
- heatmap 데이터 자체에 `HeatmapKind`(Absolute/Delta)를 명시해 렌더링 레이어가 팔레트를 자동으로 올바르게 선택하게 한다.

**탐지 방법**:
- Manual: 델타/diff 뷰를 열어 0 근처 값과 큰 음수 값이 색상으로 명확히 구분되는지 육안 검토.
- Structural: 팔레트 선택 함수가 heatmap의 종류(절대/델타)를 파라미터로 받는지 검사.

**예외**:
- 델타의 절대값(크기)만 관심 대상이고 부호(어느 쪽이 나은지)는 이미 별도 UI 요소(예: 화살표 아이콘)로 표시되는 경우라면 순차 팔레트로 "차이의 크기"만 표현해도 무방하다.

**관련**: `UIX_VIZ.md` UIX-VIZ-003 참고 — sequential/diverging 팔레트 구분 필요성은 동일하나 대상이 VQ-Probe 스트림 간 delta score(여기)와 QP delta(UIX-VIZ-003)로 다름.

**Bitvue 판정**: Confirmed — `export::overlay::create_diff_heatmap_export`(export/overlay.rs:299-330)는 `diff_data.mode`(Abs/Signed/Metric)를 전혀 분기하지 않고 항상 동일한 4-stop 순차 램프(blue→cyan→yellow→red)를 `get_normalized()`의 0..1 값에 적용한다. `DiffMode::Signed`는 부호 있는 `a - b` 값을 만드는데(diff_heatmap.rs:143) 이 값이 정규화되면 "diff 없음(0)"이 램프의 어디에 오는지는 그 프레임의 min/max 분포에 따라 달라져 고정된 중립색(흰색/회색)으로 보장되지 않는다. 코드베이스 전체(`crates`, `src-tauri`)에 diverging/RdBu류 팔레트는 0건(grep 결과 없음) — `HeatmapKind::Absolute/Delta` 같은 구분도 없다. 참고로 frontend `VvcInverseMapRenderer.tsx`에는 blue→grey→orange 발산형 팔레트가 이미 존재하지만 완전히 다른 기능(VVC inverse mapping)용이며 diff/quality heatmap과는 무관하다.

---

### HEAT-017: heatmap 캐시 키에 metric 파라미터가 누락됨
**분류**: 캐시 정확성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct HeatmapCache {
    cache: HashMap<u32, FrameHeatmap>, // 키가 frame_index 하나뿐
}

impl HeatmapCache {
    fn get_or_compute(&mut self, frame_index: u32, metric: MetricKind, window: u32) -> FrameHeatmap {
        if let Some(cached) = self.cache.get(&frame_index) {
            return cached.clone(); // metric이나 window가 바뀌어도 이전 결과를 그대로 반환
        }
        let h = compute_heatmap(frame_index, metric, window);
        self.cache.insert(frame_index, h.clone());
        h
    }
}
```

**문제**:
- 사용자가 UI에서 metric을 SSIM에서 PSNR로 바꾸거나 window 크기를 조정해도, 캐시 키가 `frame_index`뿐이라 이전 metric/window로 계산된 stale 결과가 그대로 반환된다.
- 이 버그는 최초 조회(캐시 미스) 시점에는 정상 동작하는 것처럼 보이다가, 파라미터를 바꾼 "두 번째" 조회부터만 틀린 값을 반환하므로 초기 QA에서 놓치기 쉽다.
- HEAT-008(메타데이터 누락)과 결합되면 더 심각해진다 — 반환된 heatmap 자체에 실제로 어떤 metric/window로 계산됐는지 기록조차 없어 stale 여부를 사후에 검증할 방법도 없다.

**발생 조건**:
- metric 종류나 window/stride를 사용자가 런타임에 바꿀 수 있는 UI를 도입한 이후, 캐시 계층을 함께 업데이트하지 않은 경우 항상 재현된다.
- 캐시 적중률을 높이려는 최적화 과정에서 캐시 키를 단순화하다가 실수로 파라미터를 빠뜨리기 쉽다.

**권장**:
```rust
#[derive(PartialEq, Eq, Hash, Clone)]
struct HeatmapCacheKey {
    frame_index: u32,
    metric: MetricKind,
    window: u32,
    stride: u32,
    resolution_tag: ResolutionTag,
}

struct HeatmapCache {
    cache: HashMap<HeatmapCacheKey, FrameHeatmap>,
}

impl HeatmapCache {
    fn get_or_compute(&mut self, key: HeatmapCacheKey) -> FrameHeatmap {
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }
        let h = compute_heatmap(&key);
        self.cache.insert(key, h.clone());
        h
    }
}
```
- 캐시 키에는 결과에 영향을 주는 모든 계산 파라미터(metric, window, stride, 해상도 태그)를 빠짐없이 포함시킨다.
- 캐시 키 구조체와 heatmap 메타데이터(HEAT-008) 구조체를 같은 필드 집합에서 파생시켜 둘이 어긋나지 않게 한다.

**탐지 방법**:
- Structural: 캐시 키 타입 정의와 `compute_heatmap` 파라미터 목록을 대조해 캐시 키에 없는 파라미터가 있는지 검사.
- Runtime: 동일 프레임에 대해 metric/window를 바꿔가며 연속 조회했을 때 반환값이 실제로 달라지는지 회귀 테스트.

**예외**:
- 애플리케이션에서 metric/window가 세션 전체에 걸쳐 고정 상수이고 런타임 변경 UI가 아예 없다면 `frame_index` 단일 키로도 안전하다(단, 그 가정이 깨지는 순간 이 항목이 즉시 재발한다는 점을 문서화해 둘 것).

**Bitvue 판정**: Confirmed — `MultiFrameBlockMetrics.frames: HashMap<usize, BlockMetricsGrid>`(block_metrics.rs:608)가 정확히 이 안티패턴 모양이다: 키가 `display_idx`(frame index) 하나뿐이고, `add_frame()`(block_metrics.rs:625-627)은 삽입되는 `grid.metric_type`/`grid.block_size`가 `self.metric_type`/`self.block_size`(구조체 생성 시 고정 선언)와 일치하는지 전혀 검증하지 않는다. metric이나 block_size를 바꿔 다시 계산한 `BlockMetricsGrid`를 같은 인스턴스에 `add_frame`으로 넣으면 조용히 섞여 들어가고, `aggregate_statistics()`(block_metrics.rs:640-680)는 이 뒤섞인 값들을 하나의 min/max/avg로 집계한다. 한편 `CacheKey::DiffHeatmap`(cache_provenance.rs:63-69)은 `mode`/`hm_res`를 키에 포함시켜 이 문제를 피하고 있어, diff heatmap 캐시와 block metrics 컬렉션 사이에 설계 일관성이 없다는 점도 드러난다.

---

### HEAT-018: 보간/스무딩이 block 단위 정밀도를 시각적으로 과장
**분류**: 시각화 정확성 · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
fn render_heatmap_canvas(scores: &[f32], grid_w: u32, grid_h: u32, canvas_w: u32, canvas_h: u32) -> Vec<u8> {
    // block 단위(예: 16x16)로 계산된 거친 grid를 캔버스 해상도로 bicubic 보간
    bicubic_upscale(scores, grid_w, grid_h, canvas_w, canvas_h) // 부드러운 그라데이션으로 렌더링
}
```

**문제**:
- 원본 score는 16x16 픽셀당 하나의 이산적인 값인데, bicubic/bilinear 보간으로 부드럽게 렌더링하면 마치 픽셀 단위로 세밀하게 측정된 것처럼 보여 사용자가 실제보다 훨씬 높은 공간 해상도의 정보를 얻었다고 착각하게 만든다.
- 특히 block 경계에서 실제로는 계단식으로 뚝 떨어지는 품질 차이가, 보간으로 인해 "완만하게 전환되는 것"처럼 보여 진짜 블로킹 아티팩트 경계를 오독하게 할 수 있다.
- 이 문제는 렌더링 단만의 이슈처럼 보이지만, 사용자가 "이 정확한 지점"을 클릭해 좌표를 짚어 리포트하는 등 정량적 판단의 근거로 삼는 순간 계산 정확성 문제로 번진다.

**발생 조건**:
- block/window 단위로 계산된 성긴(coarse) grid를 고해상도 캔버스에 표시하기 위해 업스케일 보간을 적용하는 모든 렌더링 경로에서 발생한다.
- 사용자가 확대(줌 인)해서 개별 block 경계를 자세히 조사하려는 워크플로에서 특히 오도의 소지가 크다.

**권장**:
```rust
fn render_heatmap_canvas(scores: &[f32], grid_w: u32, grid_h: u32, canvas_w: u32, canvas_h: u32, mode: RenderMode) -> Vec<u8> {
    match mode {
        // 기본값: nearest-neighbor로 block 경계를 있는 그대로 계단식으로 표시
        RenderMode::BlockAccurate => nearest_upscale(scores, grid_w, grid_h, canvas_w, canvas_h),
        // 사용자가 명시적으로 선택했을 때만 부드러운 트렌드 시각화 제공(범례에 "보간됨" 명시)
        RenderMode::SmoothTrend => bicubic_upscale(scores, grid_w, grid_h, canvas_w, canvas_h),
    }
}
```
- 기본 렌더링은 nearest-neighbor(block 경계를 있는 그대로 계단식으로 표현)로 하고, 부드러운 보간은 opt-in 모드로만 제공하며 UI에 "보간된 시각화"임을 명시한다.
- 줌 인해서 개별 block을 조사하는 인터랙션에서는 항상 block-accurate 모드로 강제 전환한다.

**탐지 방법**:
- Manual: 히트맵을 최대 줌으로 확대했을 때 block 경계가 계단식으로 보이는지, 부드러운 그라데이션으로 뭉개져 있는지 육안 확인.
- Structural: 캔버스 렌더링 코드에서 사용하는 보간 알고리즘(`bicubic`/`bilinear` vs `nearest`)을 검색하고 기본값을 확인.

**예외**:
- "정밀 조사"가 아니라 "전체적인 트렌드 미리보기"가 명시적 목적인 축소 오버뷰(예: 매우 작은 필름스트립 썸네일)에서는 부드러운 보간이 가독성 면에서 오히려 낫다.

**관련**: `UIX_VIZ.md` UIX-VIZ-008 참고 — 이산적 block-grid 값을 보간으로 매끄럽게 렌더링해 정밀도를 과장하는 동일 패턴. VQ-Probe 품질-score heatmap(여기) vs QP heatmap(Bitvue).

**Bitvue 판정**: N/A — `BlockMetricsOverlay::to_rgba()`(block_metrics.rs:571-597)는 각 block의 색을 `block_to_pixel_range`가 반환하는 픽셀 사각형 전체에 그대로 채워 넣는 방식이라 애초에 nearest-neighbor(계단식) 렌더링이며 보간이 없다. `frontend/components/panels/OverlayRenderer` 어디에도 `bicubic`/`bilinear`/`imageSmoothingEnabled`/`drawImage` 기반 업스케일 코드가 없고(grep 결과 0건), diff/quality-score heatmap 전용 프론트엔드 렌더러 자체가 아직 없다(HEAT-003/004 참고). 따라서 이 anti-pattern이 실현될 렌더링 경로가 현재 존재하지 않는다.
