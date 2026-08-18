# Anti-Pattern Catalog — SER: Serialization·Schema·대용량 데이터 모델

이 문서는 Bitvue 안티패턴 카탈로그의 한 갈래(Phase 4)이며, 전체 색인은 `docs/anti-patterns/INDEX.md`를 참고한다. Wave 1의 `IPC.md`가 Tauri IPC 경계에서의 페이로드 크기·포맷 문제를 다뤘다면, 이 문서는 그보다 넓은 범위 — CLI export, MCP 서버 응답, 스키마 버전 진화, 장기 보관되는 파일 포맷 호환성 — 를 다룬다. 즉 "IPC 호출 한 번의 응답"이 아니라 "디스크에 남거나 다른 프로세스가 소비하는 직렬화 산출물 전반"이 대상이다.

---

### SER-001: domain model을 그대로 JSON export
**분류**: API/Export 결합 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// 디코더 내부 상태를 그대로 표현하는 구조체
#[derive(serde::Serialize)]
pub struct Frame {
    pub index: u32,
    pub nal_units: Vec<NalUnit>,       // 파서 내부 표현
    pub decoder_state: DecoderState,   // 디코더 세션 캐시, ref pic list 등
    pub slice_headers: Vec<SliceHeader>,
    pub internal_cache_hits: u64,      // 디버깅용 카운터
}

// CLI export도, MCP 응답도, Tauri 커맨드도 모두 이 타입을 그대로 직렬화
pub fn export_json(frame: &Frame) -> String {
    serde_json::to_string(frame).unwrap()
}
```

**문제**:
- `DecoderState`, `internal_cache_hits` 같은 순수 구현 세부사항이 외부 파일 포맷의 일부가 되어버려, 디코더 리팩터링이 곧 파일 포맷 breaking change가 된다.
- 도메인 모델에 필드를 하나 추가하면(성능 캐시, 임시 플래그 등) export 파일도 자동으로 커진다 — 의도치 않은 스키마 변경.
- export 소비자(외부 도구, 회귀 테스트, 다른 팀)가 내부 구현에 암묵적으로 결합되어, "이 필드는 안 써도 되는데 빼면 안 되는" 필드가 누적된다.
- 직렬화 가능하게 만들기 위해 내부 타입에 `#[derive(Serialize)]`를 붙이다 보면 캡슐화가 깨지고, `pub` 필드가 늘어난다.

**발생 조건**:
- 프로토타입 단계에서 "일단 되게" `#[derive(Serialize)]`를 디코더 구조체에 바로 붙였다가 그대로 굳어질 때.
- CLI export, MCP 툴 응답, Tauri IPC 세 경로가 서로 다른 팀/시점에 만들어져 공용 DTO 대신 각자 도메인 타입을 직접 노출할 때.

**권장**:
```rust
// 도메인 모델 (직렬화 불가, 내부 전용)
pub struct Frame {
    pub index: u32,
    pub nal_units: Vec<NalUnit>,
    pub decoder_state: DecoderState,
}

// export 전용 DTO — 안정적인 외부 계약
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FrameExport {
    pub schema_version: u32,
    pub frame_index: u32,
    pub frame_type: FrameTypeExport,
    pub pts: i64,
}

impl From<&Frame> for FrameExport {
    fn from(f: &Frame) -> Self {
        FrameExport {
            schema_version: 1,
            frame_index: f.index,
            frame_type: f.slice_type().into(),
            pts: f.pts,
        }
    }
}
```
- DTO와 도메인 모델을 명시적으로 분리하고 `From`/`TryFrom`으로만 변환한다.
- DTO에는 "외부에 약속한 필드"만 남기고, 디코더 리팩터링이 DTO에 전파되지 않게 한다.

**탐지 방법**:
- Structural: export/MCP/IPC 경로에서 사용되는 `Serialize` 타입이 `decoder`, `parser` 등 내부 모듈에 정의된 타입과 동일한지 검사.
- Manual: 코드 리뷰에서 "이 필드가 디코더를 리팩터링해도 의미가 유지되는가?" 질문.

**예외**:
- 완전히 휘발성인 디버그 덤프(`--debug-dump`, 버전 호환성 보장 없음을 명시)는 도메인 모델을 그대로 찍어도 무방하다.

**Bitvue 판정**: N/A — 표본 조사(`crates/bitvue-cli/src/commands/export.rs`의 `ExportFrame`/`ExportDocument`, `crates/bitvue-engine/src/export/frames.rs`의 `FrameExportRow`, `crates/bitvue-engine/src/mcp.rs`의 `McpMetricsSummary`)는 모두 목적별 DTO를 따로 정의하고 있고, decoder_state/cache_hits류 내부 필드가 export 경로로 새는 사례는 grep으로 찾지 못함(구 감사가 인용했던 `bitvue-core`/`src-tauri`는 Electron 이관으로 각각 `bitvue-engine`/`bitvue-sidecar`로 이름이 바뀌었을 뿐, 결론은 재검증 후에도 동일)

---

### SER-002: schema version 없음
**분류**: 스키마 버전 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportFile {
    pub frames: Vec<FrameExport>,
    pub stream_info: StreamInfo,
}

// 파일에 버전 정보가 전혀 없음 — 필드가 추가/삭제되면
// 과거 파일과 현재 파일을 구분할 방법이 없다.
```

**문제**:
- 6개월 뒤 필드를 추가하거나 의미를 바꾸면, 옛날 export 파일을 읽을 때 어떤 스키마인지 런타임에 판별할 수 없다.
- 버그 리포트에 첨부된 export 파일이 어느 버전의 Bitvue로 생성됐는지 알 수 없어 재현이 어렵다.
- 마이그레이션 로직을 작성하려 해도 분기할 기준이 없어 휴리스틱(필드 존재 여부 추측)에 의존하게 된다.

**발생 조건**:
- CLI export 포맷이 처음 만들어질 때 "지금은 필드가 안정적이니 버전이 필요 없다"고 판단한 경우.
- 내부 도구로 시작했다가 외부 사용자에게 공유되는 파일 포맷으로 승격된 경우.

**권장**:
```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportFile {
    pub schema_version: u32,   // 최상위, 항상 첫 필드
    pub bitvue_version: String,
    pub frames: Vec<FrameExport>,
    pub stream_info: StreamInfo,
}

pub const CURRENT_SCHEMA_VERSION: u32 = 3;

pub fn load(bytes: &[u8]) -> Result<ExportFile, LoadError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let version = value.get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or(LoadError::MissingSchemaVersion)?;
    match version {
        1 => migrate_v1_to_current(value),
        2 => migrate_v2_to_current(value),
        CURRENT_SCHEMA_VERSION_U64 => serde_json::from_value(value).map_err(Into::into),
        v => Err(LoadError::UnsupportedVersion(v)),
    }
}
```
- 최상위 레벨에 `schema_version` 필드를 항상 두고, 파일을 처음 열 때 가장 먼저 읽는다.
- 버전은 단조 증가하는 정수로 관리하고, 각 버전 간 마이그레이션 함수를 명시적으로 둔다(→ SER-020).

**탐지 방법**:
- Structural: export 최상위 struct에 `schema_version` 필드 존재 여부를 grep/AST 검사로 확인.
- Manual: export 포맷 설계 리뷰에서 필수 체크리스트 항목으로 포함.

**예외**:
- 프로세스 수명 내에서만 존재하고 디스크/네트워크로 나가지 않는 순수 IPC 응답(요청-응답이 같은 빌드에서만 발생)은 버전 필드 없이도 상대적으로 안전하다 — 다만 Tauri처럼 프런트/백엔드가 별도로 배포될 수 있는 경우는 예외에서 제외.

**Bitvue 판정**: Confirmed(제한적) — CLI export(`crates/bitvue-cli/src/commands/export.rs:25` `ExportDocument`/`ExportFrame`)와 timeline export DTO(`crates/bitvue-engine/src/export/frames.rs:11` `FrameExportRow`)는 디스크에 쓰는 JSON에 버전 필드가 없음. 다만 Evidence Bundle 포맷(`crates/bitvue-engine/src/export/evidence.rs:26` `CURRENT_BUNDLE_SCHEMA_VERSION`/`bundle_version` 필드)은 예외로, 실제 schema_version과 호환성 정책 문서(같은 파일 9-25행)까지 갖추고 있음 — 즉 "전무"가 아니라 export 경로별로 편차가 큼

---

### SER-003: optional field 추가를 항상 호환 가능하다고 가정
**분류**: 스키마 진화 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// v1
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MotionVectorExport {
    pub x: i16,
    pub y: i16,
}

// v2 — "optional이니 하위 호환일 것"이라 가정하고 추가
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MotionVectorExport {
    pub x: i16,
    pub y: i16,
    #[serde(default)]
    pub ref_idx: Option<u8>,          // 새 필드
    #[serde(default)]
    pub confidence: Option<f32>,      // AV1 전용, 다른 코덱엔 의미 없음
}
```

**문제**:
- "필드를 추가하는 것"은 forward-compatible(새 리더가 옛 파일을 읽는 것)하지만, backward-compatible(옛 리더가 새 파일을 읽는 것)은 보장하지 않는다 — 옛 버전 CLI가 새 파일을 읽으면 `deny_unknown_fields` 설정에 따라 실패할 수 있다(→ SER-019).
- `Option<T>`을 추가했다고 해서 **의미론적** 호환이 보장되지는 않는다: `confidence: None`이 "값 없음"인지 "이 코덱에서는 애초에 정의되지 않음"인지 "0.0으로 기본값 처리됨"인지 소비자가 구분할 수 없다.
- 여러 optional 필드가 쌓이면 실제로는 서로 배타적인 필드 조합(코덱별)인데 타입 시스템이 이를 강제하지 못해 잘못된 조합(HEVC 프레임인데 VP9 전용 필드가 채워짐)이 조용히 만들어질 수 있다.

**발생 조건**:
- 신규 코덱(AV1, VP9) 지원을 추가하면서 기존 export 구조체에 `Option` 필드를 계속 끼워 넣을 때.
- 여러 버전의 CLI/MCP 클라이언트가 동시에 배포되어 있어 서로 다른 스키마 기대치를 가질 때.

**권장**:
```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "codec")]
pub enum MotionVectorExport {
    #[serde(rename = "avc")]
    Avc { x: i16, y: i16, ref_idx: u8 },
    #[serde(rename = "av1")]
    Av1 { x: i16, y: i16, ref_idx: u8, confidence: f32 },
}
```
- 코덱별로 의미가 갈리는 필드는 `Option`으로 뭉치지 말고 태그된 enum(또는 별도 struct)으로 분리한다.
- 정말 "선택적"인 필드에 한해서만 `Option`을 쓰고, 각 필드 문서에 `None`의 의미를 명시한다.
- 스키마 변경 시 호환성 정책(forward-only, N-1 지원 등)을 문서화하고 CI에서 검증한다(→ SER-020).

**탐지 방법**:
- Semantic: 필드 추가 PR에서 "이 Option이 코덱별 배타적 의미를 가지는가"를 리뷰 체크리스트로 확인.
- Runtime: 여러 스키마 버전의 fixture 파일로 라운드트립 테스트를 CI에 고정.

**예외**:
- 진짜로 코덱에 무관하게 "있을 수도 없을 수도" 있는 부가 정보(예: 사용자 주석, 선택적 메타데이터)는 `Option`이 적절하다.

**Bitvue 판정**: Suspected — CLI export DTO(`ExportFrame`)류는 schema_version이 없어(SER-002) 스키마 진화 정책 자체가 약하지만, 실제로 코덱별 배타적 의미가 뭉개진 `Option` 필드 사례는 코드에서 직접 확인하지 못함. Evidence Bundle(`crates/bitvue-engine/src/export/evidence.rs`)은 반대로 additions를 `#[serde(default)]`로 명시 처리하는 문서화된 정책이 있어 이 항목의 위험이 낮음

---

### SER-004: 숫자 precision 손실
**분류**: 숫자 표현 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct QpStats {
    pub avg_qp: f32,           // 내부 계산은 f64 누적 평균
    pub psnr_y: f32,           // 실제 계산은 f64로 수행 후 f32로 캐스팅
    pub bitrate_bps: f32,      // 초당 수십 Mbps → f32로는 유효자릿수 부족
}

pub fn compute(frames: &[Frame]) -> QpStats {
    let sum: f64 = frames.iter().map(|f| f.qp as f64).sum();
    QpStats {
        avg_qp: (sum / frames.len() as f64) as f32,   // 여기서 손실
        psnr_y: compute_psnr_f64(frames) as f32,
        bitrate_bps: (total_bits as f64 / duration_sec) as f32,
    }
}
```

**문제**:
- `f32`는 약 7자리 유효숫자만 표현 가능해, `bitrate_bps`처럼 큰 값(수천만~수억)에서는 정수 단위 오차가 발생하고 export → re-import 라운드트립 시 값이 미세하게 달라진다.
- PSNR처럼 회귀 테스트에서 "이전 값과 비교"하는 지표를 `f32`로 내려버리면, 실제로는 동일한 결과인데도 부동소수점 표현 차이로 diff가 발생해 테스트가 flaky해진다.
- 누적 합산을 `f64`로 하고 마지막에만 `f32`로 캐스팅하는 패턴은 "정밀 계산 후 손실 저장"이라 계산 자체의 이점이 export 단계에서 사라진다.

**발생 조건**:
- 통계치(평균 QP, PSNR/SSIM, 비트레이트)를 장시간 스트림에 대해 누적 계산할 때.
- export 파일을 다시 읽어 diff/회귀 비교하는 CI 파이프라인에서 float 오차가 threshold를 넘나들 때.

**권장**:
```rust
#[derive(serde::Serialize)]
pub struct QpStats {
    pub avg_qp: f64,
    pub psnr_y: f64,
    pub bitrate_bps: f64,
}
```
- 저장 시에는 계산에 사용한 정밀도(`f64`)를 그대로 유지하고, 표시(프런트엔드 렌더링) 단계에서만 반올림한다.
- 라운드트립 비교가 필요한 값은 `f64` 비트 패턴을 그대로 비교하거나, 명시적 허용 오차(epsilon)를 문서화한 비교 함수를 사용한다.
- 정말 `f32`로 충분한 필드(예: 정규화된 0~1 confidence)는 그렇게 명시하되 주석으로 근거를 남긴다.

**탐지 방법**:
- Static: export DTO 필드 타입에서 `f32`를 사용하는 곳을 나열하고 원본 계산 타입(`f64`)과 비교.
- Runtime: export → re-import 라운드트립 테스트에서 값 동일성(또는 명시된 epsilon 이내)을 검증.

**예외**:
- 시각화 전용(오버레이 렌더링에만 쓰이고 재계산/비교에 쓰이지 않는) 값은 `f32`로도 충분하다.

**Bitvue 판정**: N/A — 크기가 큰 값(bitrate/byte 총량)은 일관되게 `u64`/`f64`로 저장됨(`crates/bitvue-engine/src/picture_stats.rs:220-234` `SequenceStats.total_size_bytes: u64`, `avg_size_bytes: f64`); 발견된 `f32` 통계 필드들(`FrameExportRow.qp_avg`, `timeline_lane_types.rs`/`spatial_hierarchy.rs`의 `avg_qp: f32`)은 전부 0~255 범위 QP값으로 문서가 인정하는 "0~1 정규화 값급" 예외 케이스에 해당, 큰 값에 f32를 쓰는 사례는 없음

---

### SER-005: u64 offset을 JavaScript number로 전달
**분류**: 숫자 표현 · **심각도**: Critical · **탐지**: Runtime

> JavaScript의 `Number`는 IEEE-754 double이라 안전하게 표현 가능한 정수 범위가 ±2^53(약 9×10^15)로 제한된다. 대용량 비디오 파일의 바이트 오프셋, 마이크로초 단위 PTS/DTS, 장시간 스트림의 누적 바이트 범위는 이 한계에 실제로 근접하거나 넘어설 수 있어 이 문제가 특히 위험하다.

**나쁜 예**:
```rust
#[tauri::command]
fn get_nal_offset(state: tauri::State<AppState>, index: u32) -> u64 {
    let parser = state.parser.lock().unwrap();
    parser.nal_units[index as usize].byte_offset  // u64, 파일 오프셋
}

#[derive(serde::Serialize)]
pub struct FrameExport {
    pub pts_us: u64,          // 마이크로초 단위 PTS
    pub byte_offset: u64,     // 파일 내 바이트 오프셋
    pub byte_size: u64,
}
```
```typescript
// 프런트엔드에서 그대로 number로 수신
const offset: number = await invoke("get_nal_offset", { index });
// offset이 2^53을 넘으면 이 시점에서 이미 정밀도가 깨져 있다.
```

**문제**:
- 수십 GB 캡처 파일이나 장시간(수십 시간) 스트림에서는 바이트 오프셋과 누적 PTS가 2^53에 근접할 수 있고, 이 경우 JS 쪽에서 오프셋이 반올림되어 잘못된 바이트를 가리키게 된다.
- Rust `u64` → JSON number → JS `number` 변환은 `serde_json`이 값 자체는 손실 없이 직렬화하지만, **JS가 파싱하는 순간** 정밀도가 소실된다 — 버그가 Rust 쪽에는 전혀 보이지 않고 프런트엔드에서만 재현된다.
- hex view에서 "오프셋 0x1_0000_0000 근방을 클릭하면 엉뚱한 바이트가 열린다" 같은 재현이 어려운 버그로 나타난다.

**발생 조건**:
- 4K/8K 장시간 캡처, 멀티 GB IVF/MP4 분석.
- PTS를 나노초/마이크로초 단위로 유지하며 스트림이 몇 시간 이상 지속될 때.
- CLI export한 JSON을 Node.js 기반 후처리 스크립트나 MCP 클라이언트(JS/TS)가 `JSON.parse`로 읽을 때.

**권장**:
```rust
#[derive(serde::Serialize)]
pub struct FrameExport {
    #[serde(with = "string_u64")]
    pub pts_us: u64,
    #[serde(with = "string_u64")]
    pub byte_offset: u64,
    #[serde(with = "string_u64")]
    pub byte_size: u64,
}

mod string_u64 {
    use serde::{Serializer, Deserializer, Deserialize};
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        String::deserialize(d)?.parse().map_err(serde::de::Error::custom)
    }
}
```
- 2^53을 넘을 가능성이 있는 정수(바이트 오프셋, PTS/DTS, 총 파일 크기)는 JSON 문자열로 직렬화하거나 `BigInt` 대응 포맷을 사용한다.
- Tauri IPC의 경우 프런트엔드에서 `BigInt`로 받는 커스텀 (de)serializer를 쓰거나, 애초에 문자열로 왕복시킨다.
- CLI export 스키마 문서에 "이 필드는 63비트를 초과할 수 있으므로 문자열로 인코딩됨"을 명시한다.

**탐지 방법**:
- Static: `u64`/`i64`/`usize`이면서 `Serialize`가 파생된 필드 중 오프셋/PTS/크기 계열 이름(`offset`, `pts`, `size`, `bytes`)을 grep.
- Runtime: 2^53을 넘는 값을 가진 fixture로 export → JS `JSON.parse` 라운드트립 테스트를 CI에 포함.

**예외**:
- 값의 상한이 도메인적으로 2^32 이내로 보장되는 필드(예: 프레임 인덱스, NAL 개수)는 일반 number로 안전하다.

**Bitvue 판정**: Confirmed — (Tauri 이관 후 경로 갱신) `crates/bitvue-sidecar/src/main.rs:509` 등에서 `start_bit: u64`가 그대로 JSON-RPC 응답에 실리고, `frontend/services/electronBridgeService.ts:313`가 `bit_range: { start_bit: number; end_bit: number }`로, `frontend/components/panels/SyntaxDetailPanel/FrameSyntaxTab.tsx:29,42`가 `byte_offset?: number`(= `start_bit / 8`)로 그대로 수신 — 프런트엔드 전체에서 `BigInt` 사용처 0건(grep). 바이트가 아니라 비트 오프셋이라 2^53 문턱이 8배 늘어나 실사용 위험은 더 낮지만, 문자열/BigInt 인코딩 안전장치는 여전히 전혀 없음

---

### SER-006: NaN/Inf가 JSON에서 사라짐
**분류**: 숫자 표현 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct PsnrResult {
    pub psnr_y: f64,   // 완전히 동일한 프레임이면 MSE=0 → PSNR = Infinity
}

pub fn psnr(mse: f64) -> f64 {
    if mse == 0.0 { f64::INFINITY } else { 10.0 * (255.0 * 255.0 / mse).log10() }
}

// serde_json::to_string은 기본적으로 NaN/Infinity를 표현할 수 없어
// 조용히 실패하거나(Result::Err) null로 대체하는 wrapper를 쓰는 경우가 많다.
```

**문제**:
- JSON 스펙 자체가 `NaN`/`Infinity`를 지원하지 않기 때문에 `serde_json`은 기본적으로 이런 값을 만나면 직렬화를 **에러로 실패**시킨다 — CLI export가 "이유 없이" 실패하는 원인이 된다.
- 일부 코드는 이를 피하려고 `f64::NAN`/`INFINITY`를 사전에 `null`이나 임의의 큰 수(`f64::MAX`, `-1.0`)로 치환하는데, 이 경우 "완전 무손실(PSNR=∞)"과 "측정 실패"가 같은 값으로 뭉개져 소비자가 구분할 수 없다.
- PSNR=∞는 코덱 분석에서 실제로 흔히 발생하는 정상 케이스(스킵 프레임, 무손실 구간)라 무시할 수 없다.

**발생 조건**:
- 동일 프레임 비교(무손실 인코딩, 스킵 프레임)로 MSE가 0이 되는 경우.
- 0으로 나누는 비율 계산(압축률, 비트레이트 변화율)에서 분모가 0인 예외 케이스.

**권장**:
```rust
#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum PsnrValue {
    Finite(f64),
    Infinite,      // MSE == 0, 무손실
    Undefined,     // 계산 불가(예: 참조 프레임 없음)
}

impl From<f64> for PsnrValue {
    fn from(v: f64) -> Self {
        if v.is_infinite() { PsnrValue::Infinite }
        else if v.is_nan() { PsnrValue::Undefined }
        else { PsnrValue::Finite(v) }
    }
}
```
- `NaN`/`Infinity`가 나올 수 있는 모든 지표는 명시적인 tagged enum(또는 sentinel이 아닌 별도 상태 필드)으로 표현해 "정상적인 무한대"와 "계산 실패"를 구분한다.
- 정말 raw float이 필요하면 `serde_json::Number`가 아닌 커스텀 포맷(JS5 스타일 `"Infinity"` 문자열 등)을 명시적으로 채택하고 문서화한다.

**탐지 방법**:
- Runtime: PSNR/SSIM 등 무한대가 나올 수 있는 계산 경로에 대해 MSE=0 fixture로 export 성공 여부 테스트.
- Static: `f64`/`f32` 필드에 `Serialize`를 파생하면서 `is_infinite`/`is_nan` 체크가 export 경로 어디에도 없는 경우를 탐지.

**예외**:
- 내부 전용 디버그 로그(구조화 로깅이 아닌 텍스트 로그)는 `{:.2}` 포맷팅으로 `inf`/`NaN`을 그대로 찍어도 무방하다.

**Bitvue 판정**: Confirmed — `crates/bitvue-metrics/src/lib.rs:77`/`simd.rs:765,870`이 동일 프레임 비교 시 실제로 `f64::INFINITY`를 반환하고, 이 값을 JSON으로 내보내는 실제 경로(`crates/bitvue-sidecar/src/debug_yuv.rs:544-556`, `get_yuv_diff_metrics` 응답)가 정확히 "나쁜 예"가 경고하는 패턴을 구현: `PSNR_INFINITY_SENTINEL = 100.0`으로 무손실 값을 클램프해 실제 매우 높은(하지만 유한한) PSNR과 "완전 무손실"을 같은 100.0 값으로 뭉갬. 다만 이 sentinel은 프런트엔드 `fmt()` 헬퍼의 ">=99.99 dB → ∞ 표시" 규칙과 명시적으로 맞춰 문서화된 의도적 설계(주석 544-547행)라, "조용한 은폐"라기보다 "합의된 근사"에 가까움 — CLI 경로(`bitvue-cli/quality.rs`)는 텍스트 stdout이라 이 문제가 없음

---

### SER-007: binary 데이터를 base64로 기본 저장
**분류**: 인코딩 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct HexDumpExport {
    pub frame_index: u32,
    #[serde(serialize_with = "as_base64")]
    pub raw_bytes: Vec<u8>,   // 프레임 원본 바이트 전체, 수 MB
}

fn as_base64<S: serde::Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&base64::encode(bytes))
}
```

**문제**:
- base64는 원본 대비 약 33% 크기 증가를 유발하고, JSON 문자열 이스케이프 오버헤드까지 더해지면 실질적으로 40% 이상 커질 수 있다 — 대용량 hex dump export에서 파일 크기가 눈에 띄게 부풀어 오른다.
- 텍스트 기반 JSON 파서가 수 MB짜리 base64 문자열을 통째로 읽고 디코딩해야 하므로, "구조화된 메타데이터는 조금인데 파싱 시간은 거대한 base64 블록이 지배"하는 비효율이 생긴다.
- JSON 안에 바이너리를 억지로 문자열로 넣는 것 자체가 "JSON은 사람이 읽는 포맷"이라는 장점을 무의미하게 만든다 — 결과물은 어차피 사람이 읽을 수 없는 base64 덩어리다.

**발생 조건**:
- hex view, raw NAL 바이트, 잔차(residual) 블록처럼 바이너리 성격이 강한 데이터를 export 포맷에 포함할 때.
- "일단 JSON 하나로 통일하자"는 압박으로 모든 산출물을 단일 JSON 파일에 우겨넣을 때(→ SER-008).

**권장**:
```rust
// 옵션 A: 바이너리는 별도 파일/사이드카로 분리하고 JSON은 참조만 가짐
#[derive(serde::Serialize)]
pub struct HexDumpExport {
    pub frame_index: u32,
    pub raw_bytes_file: String,   // "frame_0042.bin"
    pub raw_bytes_range: (u64, u64),
}

// 옵션 B: 애초에 바이너리 컨테이너 포맷(MessagePack, 또는 자체 TLV)을 채택
```
- 대용량 바이너리는 JSON에 인라인하지 않고, 별도 `.bin` 사이드카 파일이나 zip 내 개별 엔트리로 분리한다.
- 정말 단일 파일 포맷이 필요하면 base64가 아니라 바이너리를 직접 담을 수 있는 포맷(MessagePack, CBOR, Protobuf)을 검토한다.
- CLI export 옵션에 `--include-raw-bytes`처럼 기본적으로는 빠지는 opt-in 플래그를 둔다.

**탐지 방법**:
- Static: `base64::encode`/`STANDARD.encode` 호출부를 export 모듈에서 grep, 대상 바이트 크기 확인.
- Structural: export DTO에서 `Vec<u8>`을 문자열로 직렬화하는 필드를 나열.

**예외**:
- 수 KB 이내의 작은 바이너리 조각(썸네일, 짧은 헤더 덤프)은 base64 오버헤드가 무시할 만하므로 실용적으로 허용된다.

**Bitvue 판정**: N/A(구 판정 뒤집힘) — Tauri 시절 `YUVFrameData`(base64 평면)는 Electron 이관 과정에서 제거됨. 현재 `get_decoded_frame_yuv`/`get_hex_range`(`crates/bitvue-sidecar/src/decode_bridge.rs:22-38`, `main.rs:1302` 등)는 "Control 프레임 + raw bytes Data 프레임" 2-프레임 바이너리 와이어 패턴을 명시적으로 채택해 대용량 YUV 평면·hex 바이트에 base64를 쓰지 않음(주석에 "no base64" 명시). base64가 실제로 쓰이는 곳은 PNG 스크린샷(Evidence Bundle, `evidence_export.rs`)과 필름스트립 썸네일(`decode_bridge.rs:128-132`)뿐이며, 두 곳 다 코드 주석이 "작은 바이너리라 base64가 합리적"이라고 명시 — 이 항목의 예외 조항에 해당하는 사례로 이미 스스로 전환됨

---

### SER-008: 거대한 단일 JSON 파일
**분류**: 대용량 데이터 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct FullExport {
    pub stream_info: StreamInfo,
    pub frames: Vec<FrameExport>,       // 수만 프레임
    pub gop_structure: Vec<GopExport>,
    pub motion_vectors: Vec<MvFieldExport>,   // 프레임당 수천 개
    pub qp_maps: Vec<QpMapExport>,
}

pub fn export_all(path: &Path, data: &FullExport) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(data)?;   // 단일 문자열, 수백 MB~GB
    std::fs::write(path, json)
}
```

**문제**:
- 장시간 스트림(수만 프레임)을 분석하면 단일 JSON이 수백 MB~수 GB에 이를 수 있고, 이를 열어보려는 텍스트 에디터/브라우저가 그대로 멎어버린다.
- 이 파일을 다시 읽으려면 전체를 메모리에 파싱해야 하므로(→ SER-009), "프레임 100번만 보고 싶다"는 요구조차 파일 전체 파싱을 강제한다.
- `to_string_pretty`는 들여쓰기 문자를 추가로 생성해 파일 크기를 더 키운다 — 사람이 읽을 목적이 아니라면 불필요한 비용이다.
- 파일 하나가 손상되면(쓰기 도중 크래시 등) 전체 export가 통째로 무효화된다 — 부분 손상에 대한 내성이 없다.

**발생 조건**:
- `--export-full` 같은 CLI 옵션으로 스트림 전체를 한 번에 덤프할 때.
- MV/QP처럼 프레임당 데이터량이 큰 필드를 프레임 수만큼 반복할 때.

**권장**:
```rust
// 프레임 단위로 JSON Lines(NDJSON)로 분리 저장
pub fn export_streaming(path: &Path, frames: impl Iterator<Item = FrameExport>) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    for frame in frames {
        serde_json::to_writer(&mut writer, &frame)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}
```
- 프레임 단위/GOP 단위로 JSON Lines(NDJSON)나 다중 파일로 분리해, 부분 읽기와 스트리밍 처리를 가능하게 한다.
- 메타데이터(스트림 정보, 스키마 버전)는 별도의 작은 헤더 파일로 분리하고, 대용량 시계열 데이터는 별도 스트림으로 둔다.
- 필요하면 압축(zstd/gzip)까지 포함한 컨테이너 포맷(예: tar+zstd)으로 묶는다.

**탐지 방법**:
- Structural: export 함수가 `Vec<T>` 전체를 하나의 `to_string`/`to_writer` 호출에 넘기는지 확인.
- Runtime: 수만 프레임 fixture로 export 시 메모리 피크와 소요 시간을 측정하는 벤치마크.

**예외**:
- 짧은 클립(수백 프레임 이내)이나 요약 통계만 담는 export는 단일 JSON으로도 문제가 없다.

**Bitvue 판정**: Confirmed — `crates/bitvue-cli/src/commands/export.rs::run()`(32-72행)이 전체 스트림의 `Vec<ExportFrame>`을 `ExportDocument` 하나에 담아 `serde_json::to_string_pretty(&doc)` 한 번으로 직렬화 — NDJSON/청크 분리 옵션 없음. 재검증 시점(2026-08-18, Electron 이관 이후)에도 이 파일·함수는 그대로 존재해 결론 불변

---

### SER-009: stream export 없이 전체 메모리 직렬화
**분류**: 대용량 데이터 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
pub fn export_frames(all_frames: &[Frame]) -> Vec<u8> {
    let exports: Vec<FrameExport> = all_frames.iter().map(FrameExport::from).collect();
    serde_json::to_vec(&exports).unwrap()   // 전체를 메모리 위 Vec<u8>로 만든 뒤 반환
}

pub fn write_export(path: &Path, all_frames: &[Frame]) -> std::io::Result<()> {
    let bytes = export_frames(all_frames);  // 피크 메모리 = 원본 데이터 + DTO + JSON 문자열
    std::fs::write(path, bytes)
}
```

**문제**:
- `to_vec`으로 전체 JSON을 메모리에 문자열/바이트로 완성한 뒤에야 디스크에 쓰기 때문에, 피크 메모리 사용량이 "원본 프레임 데이터 + DTO 복사본 + 직렬화된 JSON 바이트"의 합이 되어 원본 데이터 크기의 2~3배에 달할 수 있다.
- 대용량 스트림 분석 중 export를 시도하면 OOM으로 프로세스 전체가 죽을 수 있고, CLI/MCP 서버라면 다른 요청까지 함께 실패한다.
- `to_vec()` 호출 자체가 끝날 때까지 아무 진행률(progress)도 보고할 수 없어 사용자는 멈춘 것처럼 느낀다.

**발생 조건**:
- 4K/8K, 장시간 캡처 스트림을 CLI로 일괄 export할 때.
- MCP 서버가 대용량 분석 결과를 한 번의 tool 응답으로 반환하려 할 때(메모리와 네트워크 페이로드 모두 문제가 됨).

**권장**:
```rust
pub fn write_export(path: &Path, frames: impl Iterator<Item = Frame>) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::with_capacity(64 * 1024, file);
    writer.write_all(b"[")?;
    for (i, frame) in frames.enumerate() {
        if i > 0 { writer.write_all(b",")?; }
        let export = FrameExport::from(&frame);
        serde_json::to_writer(&mut writer, &export)?;
        // frame은 여기서 drop되어 다음 프레임 처리 전 메모리 반환
    }
    writer.write_all(b"]")?;
    writer.flush()
}
```
- `serde_json::to_writer`로 `BufWriter` 위에 직접 스트리밍하고, 입력도 전체를 `Vec`으로 모으지 말고 iterator/채널로 하나씩 흘려보낸다.
- CLI에는 진행률 콜백을 붙여 "N/total 프레임 처리됨"을 보고한다.
- MCP 응답이 커질 수 있는 경우, 전체 반환 대신 파일 경로를 반환하고 클라이언트가 별도로 스트리밍 읽기를 하도록 설계한다.

**탐지 방법**:
- Runtime: 대용량 fixture(수십만 프레임)로 export 시 RSS 피크를 측정하는 메모리 벤치마크를 CI에 포함.
- Static: `to_vec`/`to_string`(전체 버퍼링 API)이 대용량 컬렉션에 직접 호출되는지 grep, `to_writer`/스트리밍 API 사용 여부와 대조.

**예외**:
- 결과 크기가 애초에 작다고 보장되는 요약/통계 export(프레임당이 아니라 스트림당 하나)는 버퍼링해도 무방하다.

**Bitvue 판정**: Confirmed — 같은 함수(`crates/bitvue-cli/src/commands/export.rs::run()`, 37행 `std::fs::read`)가 입력 파일 전체를 메모리에 올리고, 프레임 전체를 `Vec<ExportFrame>`으로 모은 뒤 `to_string_pretty`(67행)로 완성된 JSON 문자열을 만들고 나서야 `std::fs::write`(71행) 한 번으로 씀 — `to_writer` 스트리밍 사용처 없음, 경로 재검증 완료

---

### SER-010: map key로 dynamic string 사용
**분류**: 스키마 구조 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct StreamStats {
    // key: "frame_0", "frame_1", ... 동적으로 생성되는 문자열
    pub per_frame_bits: std::collections::HashMap<String, u64>,
    // key: NAL 타입 이름을 그때그때 Debug 포맷으로 생성
    pub nal_counts: std::collections::HashMap<String, u32>,
}

fn build(frames: &[Frame]) -> StreamStats {
    let mut per_frame_bits = std::collections::HashMap::new();
    for (i, f) in frames.iter().enumerate() {
        per_frame_bits.insert(format!("frame_{i}"), f.bits);
    }
    // ...
    StreamStats { per_frame_bits, nal_counts: Default::default() }
}
```

**문제**:
- `frame_0`, `frame_1`처럼 순번을 문자열 키로 인코딩하면 사실상 배열인데 맵으로 표현한 것이라, JSON 키 순서(비보장)에 의존하거나 프레임 수만큼 정렬/파싱 비용이 생긴다.
- `HashMap<String, _>`은 직렬화 시 키 순서가 불안정(HashMap 순회 순서 비결정적)해서, 동일 입력에 대해 export를 두 번 돌리면 바이트가 다른 파일이 나와 diff 기반 회귀 테스트가 불가능해진다.
- NAL 타입 이름을 `format!("{:?}", nal_type)`처럼 Debug 포맷에서 얻으면, enum variant 이름을 리팩터링하는 순간 export 파일의 키가 조용히 바뀐다(→ SER-011과 연결).

**발생 조건**:
- "프레임별 통계"를 배열 대신 맵으로 설계했을 때(인덱스 접근이 편해 보인다는 이유로).
- enum을 문자열화해서 맵 키로 쓰는 카운터 집계 코드.

**권장**:
```rust
#[derive(serde::Serialize)]
pub struct StreamStats {
    pub per_frame_bits: Vec<u64>,           // 인덱스 = 프레임 번호
    pub nal_counts: Vec<NalTypeCount>,      // 안정적인 구조체 배열
}

#[derive(serde::Serialize)]
pub struct NalTypeCount {
    pub nal_type: NalTypeExport,   // 명시적 enum, #[serde(rename_all = "snake_case")]
    pub count: u32,
}
```
- 순번이 있는 데이터는 맵이 아니라 배열(인덱스가 곧 의미)로 표현한다.
- 카테고리형 키가 필요하면 `HashMap<String, _>` 대신 명시적 `enum` + `Vec<(Key, Value)>` 또는 `BTreeMap`(결정적 순서)을 사용한다.
- enum을 문자열로 노출할 때는 `Debug` 대신 `#[serde(rename_all = ...)]`로 명시적 문자열 계약을 건다.

**탐지 방법**:
- Structural: export DTO에서 `HashMap<String, _>` 필드를 찾아 키가 순번/enum 유래인지 확인.
- Runtime: 동일 입력으로 export를 두 번 실행해 바이트 단위로 동일한지 검증(결정성 테스트).

**예외**:
- 진짜로 사용자가 정의한 임의 문자열 키(예: 사용자 커스텀 태그/주석)를 담는 맵은 이 패턴에 해당하지 않는다.

**Bitvue 판정**: Confirmed(제한적) — `crates/bitvue-engine/src/picture_stats.rs:225,228`의 `SequenceStats.frame_type_counts`/`frame_type_percentages`가 `HashMap<String, _>`로 프레임 타입 문자열을 키로 사용. "frame_N" 순번 키는 아니지만(그 변형은 없음), `HashMap` 순회 순서가 보장되지 않아 동일 입력 재실행 시 바이트 동일성이 깨질 수 있음(`BTreeMap` 미사용)

---

### SER-011: enum rename이 과거 파일을 깨뜨림
**분류**: 스키마 진화 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
// v1
#[derive(serde::Serialize, serde::Deserialize)]
pub enum FrameType {
    I, P, B,
}

// v2 — 가독성을 위해 이름을 바꿈, serde rename 없이
#[derive(serde::Serialize, serde::Deserialize)]
pub enum FrameType {
    Intra,   // 과거 파일의 "I"를 더 이상 읽을 수 없다
    Predicted,
    Bidirectional,
}
```

**문제**:
- `#[derive(Deserialize)]`는 기본적으로 Rust 식별자 이름 그대로를 매칭하므로, enum variant 이름을 바꾸는 순간 과거에 저장된 `"I"` 문자열을 가진 파일은 역직렬화 에러를 낸다.
- 이런 변경은 컴파일러가 전혀 잡아주지 못한다 — Rust 코드는 정상 컴파일되고, 실패는 오직 "옛날 export 파일을 다시 열 때" 런타임에만 나타난다.
- CI에 옛 버전 fixture를 두지 않으면 이 breaking change가 릴리스될 때까지 아무도 눈치채지 못한다.

**발생 조건**:
- 코드 가독성/네이밍 컨벤션 개선을 위해 enum variant나 struct 필드 이름을 리팩터링할 때.
- IDE의 "Rename Symbol"을 직렬화 가능한 타입에 무심코 적용할 때.

**권장**:
```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub enum FrameType {
    #[serde(rename = "I", alias = "Intra")]
    Intra,
    #[serde(rename = "P", alias = "Predicted")]
    Predicted,
    #[serde(rename = "B", alias = "Bidirectional")]
    Bidirectional,
}
```
- 직렬화되는 모든 `enum`/`struct` 필드에 `#[serde(rename = "...")]`로 **직렬화 문자열을 코드 내부 이름과 분리**해, 내부 리팩터링이 외부 계약에 영향을 주지 않게 한다.
- 이름을 실제로 바꿔야 한다면 `alias`로 과거 이름을 함께 받아들이도록 하고, 마이그레이션 완료 후에만 alias를 제거한다.
- 과거 버전의 export 파일을 fixture로 저장소에 보관하고, "이 파일들을 항상 역직렬화할 수 있어야 한다"는 회귀 테스트를 CI에 고정한다.

**탐지 방법**:
- Semantic: git diff에서 `#[derive(Serialize/Deserialize)]`가 붙은 타입의 필드/variant 이름 변경을 자동 플래그.
- Runtime: 저장된 옛 버전 fixture 파일들에 대한 역직렬화 회귀 테스트(SER-020과 공유).

**예외**:
- 아직 한 번도 릴리스되지 않은(외부에 파일이 존재하지 않는) 개발 중 스키마는 자유롭게 리네임해도 무방하다.

**Bitvue 판정**: Confirmed(구조적, 예외 있음) — `crates/bitvue-engine/src/timeline.rs:14` `FrameMarker` 등 대다수 enum이 `#[serde(rename_all)]`/`rename` 없이 derive 기본 이름에 의존; 저장소 전체에서 실제 `#[serde(alias = ...)]` 적용 사례는 0건(grep). 단 Evidence Bundle 모듈(`crates/bitvue-engine/src/export/evidence.rs:24-25`)은 "리네임 시 최소 2개 MINOR 버전 동안 `#[serde(alias = "old_name")]` 유지"라는 정책을 문서화해뒀음(아직 실제 리네임 사례가 없어 정책만 존재, 강제하는 코드/CI 검증은 없음) — 다른 export 타입들은 이 정책조차 없어 안전장치가 여전히 없음

---

### SER-012: codec별 schema 차이를 flatten으로 숨김
**분류**: 스키마 구조 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct BlockInfo {
    pub x: u16,
    pub y: u16,
    // AVC/HEVC 전용
    pub mb_type: Option<String>,
    pub cu_depth: Option<u8>,
    // VP9/AV1 전용
    pub superblock_size: Option<u16>,
    pub tx_mode: Option<String>,
    // AV1 전용
    pub segment_id: Option<u8>,
    pub cdef_strength: Option<u8>,
}
// 결과: HEVC 프레임을 export해도 VP9/AV1 필드가 전부 null로 따라다닌다.
```

**문제**:
- 모든 코덱의 필드를 하나의 struct에 `Option`으로 flatten하면, 실제로는 상호 배타적인 필드 조합(HEVC면 `cu_depth`만 유효, AV1이면 `segment_id`만 유효)인데 타입이 이를 강제하지 못한다.
- 어떤 코덱을 export했는지에 따라 유효한 필드 조합이 달라지는데, 소비자는 문서를 뒤지지 않는 한 "이 필드 조합이 유효한 조합인지" 알 수 없다.
- 코덱이 하나 늘어날 때마다(예: VVC 추가) 이 struct에 필드가 계속 쌓여, null 비율이 코덱 수에 비례해 증가한다 — 파일 크기와 가독성 모두 악화된다.
- 실수로 두 코덱의 필드가 동시에 채워진 잘못된 레코드(버그)가 있어도 타입 시스템이 잡아주지 못한다.

**발생 조건**:
- 멀티 코덱(AVC/HEVC/VP9/AV1) 분석 결과를 단일 export 스키마로 통합하려 할 때.
- "공통 뷰어에서 하나의 테이블로 렌더링하기 편하다"는 이유로 UI 요구사항을 export 스키마에 그대로 반영할 때.

**권장**:
```rust
#[derive(serde::Serialize)]
#[serde(tag = "codec")]
pub enum BlockInfo {
    #[serde(rename = "hevc")]
    Hevc { x: u16, y: u16, cu_depth: u8, mb_type: HevcCuType },
    #[serde(rename = "av1")]
    Av1 { x: u16, y: u16, superblock_size: u16, segment_id: u8, cdef_strength: u8 },
}
```
- 코덱별로 실제 의미가 다른 필드 집합은 태그된 enum(internally tagged)으로 분리해, "이 레코드는 정확히 하나의 코덱 스키마만 따른다"를 타입으로 보장한다.
- 공통 필드(좌표, 크기 등)만 별도 struct로 뽑아 공유하고, 코덱 전용 필드는 variant 안에 둔다.
- UI 렌더링 편의는 프런트엔드 어댑터 계층에서 처리하고, export 스키마 자체를 UI에 맞춰 flatten하지 않는다.

**탐지 방법**:
- Structural: export DTO에서 `Option<T>` 필드 비율이 높고, 필드 이름에 특정 코덱을 암시하는 접두어(`hevc_`, `av1_`)가 섞여 있는 struct를 탐지.
- Manual: 코덱별 테스트 fixture를 export했을 때 null 필드 비율을 확인.

**예외**:
- 필드 수가 적고(2~3개) 코덱 간 겹침이 실질적으로 큰 경우(예: 모든 코덱 공통 QP 범위)는 flatten이 실용적일 수 있다.

**Bitvue 판정**: N/A(구 판정 뒤집힘) — 구 감사가 인용한 `src-tauri/src/commands/mod.rs`의 `FrameAnalysisData`(Option 필드 flatten struct)는 Electron 이관 후 폐기됨. 현재 `get_frame_analysis`(`crates/bitvue-sidecar/src/frame_analysis.rs:75-90`)는 named struct가 아니라 `serde_json::json!({"qp_grid": ..., "mv_grid": ..., "partition_grid": ...})`로 각 그리드를 무조건(Option 아님) 채워 응답을 구성 — Option flatten 패턴 자체가 없음. `BlockInfo`(`crates/bitvue-engine/src/types.rs:364`)도 여전히 코덱 공통 필드(`qp`/`bits`/`motion_vector`만 Option)만 가져 이 패턴에 해당하지 않음

---

### SER-013: 내부 ID를 외부 안정 ID로 사용
**분류**: 식별자 안정성 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct FrameExport {
    pub frame_id: usize,   // Vec<Frame>의 인덱스를 그대로 노출
    pub gop_id: usize,     // 파싱 순서상의 GOP 순번
}

// 디코더가 프레임을 재정렬(display order vs decode order)하거나
// 파서 로직이 바뀌면 이 인덱스가 가리키는 프레임이 달라진다.
```

**문제**:
- 내부 `Vec` 인덱스나 파싱 순서로 생긴 순번은 "우연히 안정적인" 값일 뿐 계약이 아니다. 디코드 순서와 표시 순서(B프레임 재정렬)를 바꾸는 리팩터링이나 파서 최적화만으로도 같은 프레임이 다른 ID를 받을 수 있다.
- 외부 도구가 이 ID를 "안정적인 키"로 가정해 저장(예: 북마크, 주석, 회귀 테스트 baseline)하면, 내부 구현이 바뀔 때마다 그 참조가 조용히 깨진다.
- 병렬 처리나 프레임 스킵 로직이 도입되면 인덱스 기반 ID의 연속성(0, 1, 2, ...)마저 보장할 수 없게 된다.

**발생 조건**:
- 프레임/GOP/NAL 단위 데이터를 export하면서 "일단 배열 인덱스를 ID로 쓰자"고 넘어갈 때.
- 외부 주석 도구, 북마크 기능, 회귀 테스트가 이 ID로 특정 프레임을 다시 찾아야 할 때.

**권장**:
```rust
#[derive(serde::Serialize)]
pub struct FrameExport {
    pub frame_id: FrameId,   // 명시적으로 정의된 안정 식별자
    pub display_order: u32,  // 표시 순서(참고용, ID 아님)
    pub decode_order: u32,   // 디코드 순서(참고용, ID 아님)
}

#[derive(serde::Serialize)]
pub struct FrameId(String);  // 예: "pts:{pts_us}" 또는 콘텐츠 해시 기반

impl FrameId {
    pub fn from_pts(pts_us: i64) -> Self {
        FrameId(format!("pts:{pts_us}"))
    }
}
```
- 안정 ID는 "무엇이 이 프레임을 유일하게 식별하는가"를 도메인적으로 정의하고(PTS, 콘텐츠 해시, 원본 파일 내 바이트 오프셋), 내부 자료구조의 위치와 분리한다.
- 안정 ID와 "지금 이 실행에서의 순서" 정보를 별도 필드로 나눠, 후자가 바뀌어도 전자는 유지되게 한다.
- ID 생성 규칙 자체를 스키마 문서에 명시해, 외부 도구가 재계산 가능하게 한다.

**탐지 방법**:
- Semantic: export DTO의 ID 필드가 `usize`/`u32`이고 값의 출처가 `Vec` 인덱스인지 코드 추적.
- Manual: "파서/디코더 리팩터링 후에도 이 ID가 같은 프레임을 가리키는가?"를 리뷰 질문으로 포함.

**예외**:
- 단일 실행(단발 CLI 호출) 내에서만 소비되고 저장되지 않는 임시 참조는 인덱스를 ID로 써도 실질적 위험이 낮다.

**Bitvue 판정**: Confirmed — `crates/bitvue-cli/src/commands/export.rs:91`(`collect_ivf_frames`, `for (idx, frame) in frames.iter().enumerate()`)이 `.enumerate()`의 `idx`를 그대로 `ExportFrame.index`로 씀. 정작 이 문제를 풀기 위해 만들어진 `crates/bitvue-engine/src/frame_identity.rs`의 `FrameIndexMap`/`display_idx`(세션 간 안정성을 문서화된 불변식으로 명시, 16-21행)가 있고 `bitvue-cli`가 `bitvue-engine`을 의존성으로 갖고 있는데도(`crates/bitvue-cli/Cargo.toml:20`), `export.rs`는 이를 import하지 않고 `bitvue-av1-codec`을 직접 파싱해 `.enumerate()` 인덱스를 씀 — 우회가 아니라 단순 미사용

---

### SER-014: export 결과에 분석 설정 누락
**분류**: 재현성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct AnalysisExport {
    pub frames: Vec<FrameExport>,
    pub psnr_summary: PsnrSummary,
    // 어떤 참조 영상과 비교했는지, threshold가 뭐였는지 전혀 기록되지 않음
}

pub fn run_analysis(input: &Path, reference: Option<&Path>, threshold: f64) -> AnalysisExport {
    // threshold, reference 경로가 결과에 반영되지 않고 사라짐
    ...
}
```

**문제**:
- PSNR/SSIM 같은 품질 지표는 비교 기준(참조 영상, 정렬 방식, threshold)에 따라 값이 크게 달라지는데, export 파일만 봐서는 "무엇과 비교한 결과인지" 알 수 없다.
- 동일한 스트림을 서로 다른 설정(예: 다른 threshold, 다른 참조본)으로 두 번 분석하면 결과 파일이 겉보기엔 같은 스키마인데 실제로는 비교 불가능한 값이 되어, 잘못된 회귀 비교(false positive/negative)를 유발한다.
- 버그 리포트에 export 파일만 첨부되면 재현에 필요한 커맨드라인 인자를 별도로 물어봐야 한다.

**발생 조건**:
- CLI에 여러 옵션(threshold, 필터, 비교 대상)이 있는데 export 스키마 설계 시 "결과값"에만 집중하고 "결과를 만든 조건"을 빠뜨렸을 때.
- MCP 서버가 여러 설정 조합으로 반복 호출되는 워크플로우에서 각 응답에 호출 파라미터를 되돌려주지 않을 때.

**권장**:
```rust
#[derive(serde::Serialize)]
pub struct AnalysisExport {
    pub analysis_config: AnalysisConfig,   // 이 결과를 만든 조건 전체
    pub frames: Vec<FrameExport>,
    pub psnr_summary: PsnrSummary,
}

#[derive(serde::Serialize)]
pub struct AnalysisConfig {
    pub input_path_hash: String,       // 원본 경로 대신 해시(민감정보 최소화)
    pub reference_path_hash: Option<String>,
    pub psnr_threshold: f64,
    pub cli_args: Vec<String>,         // 실행에 사용된 원본 인자
}
```
- export 최상위에 "이 결과가 어떤 입력·설정으로 만들어졌는지"를 담는 `analysis_config` 섹션을 항상 포함한다.
- 경로처럼 민감할 수 있는 정보는 해시나 파일명만 남기고, 값 자체(threshold, 필터 목록)는 그대로 기록한다.
- MCP 도구 응답에도 호출에 사용된 파라미터를 echo하여, 클라이언트가 결과와 조건을 항상 짝지어 보관할 수 있게 한다.

**탐지 방법**:
- Structural: export 최상위 struct에 "config"/"params" 계열 필드가 있는지 확인.
- Manual: 버그 리포트 템플릿에 export 파일 첨부를 요구할 때 "이 파일만으로 재현 가능한가"를 체크.

**예외**:
- 설정이 사실상 하나뿐(고정 파이프라인)이라 재현에 모호함이 없는 경우는 생략해도 실질적 위험이 낮다.

**Bitvue 판정**: Suspected(구 판정 재평가) — 구 감사가 인용한 Tauri `BatchQualityMetrics`/`calculate_quality_metrics`는 Electron 이관 후 폐기되어 현재 코드에 없음(grep 0건). 현재 품질 비교는 `crates/bitvue-cli/src/commands/quality.rs::run()`이 담당하는데, 이는 구조화 JSON export가 아니라 텍스트 stdout(`println!`)이고 그 출력에 reference/distorted 파일 경로·metrics 옵션이 그대로 찍힘(118-131행) — "export 파일에 설정 누락"이라는 이 항목의 정확한 형태는 성립하지 않음(애초에 그런 구조화 export가 없음). 다만 향후 `--format json` 류 구조화 출력이 추가되면 지금처럼 config가 안 실릴 위험은 구조적으로 남아있어 Suspected로 남김

---

### SER-015: model/library version 누락
**분류**: 재현성 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct AnalysisExport {
    pub psnr_summary: PsnrSummary,
    pub vmaf_score: f64,   // VMAF 모델 버전에 따라 값이 달라짐
    // libavcodec/dav1d/디코더 버전, VMAF 모델 파일 버전 등이 전혀 기록되지 않음
}
```

**문제**:
- VMAF, SSIM 등의 지표는 사용된 모델 파일 버전(`vmaf_v0.6.1.json` vs 새 모델)에 따라 동일 입력에도 다른 점수를 낼 수 있는데, 어떤 모델로 계산했는지 기록이 없으면 값 자체가 무의미해진다.
- 디코더 라이브러리(dav1d, libavcodec 등) 버전 업그레이드로 파싱/디코딩 결과가 미세하게 달라질 수 있는데, export 파일만 봐서는 "구현 변경으로 값이 바뀐 것"인지 "실제 콘텐츠 차이"인지 구분할 수 없다.
- 6개월 전 export와 오늘 export를 비교하는 회귀 테스트에서, 라이브러리 버전 차이로 인한 값 변화를 "버그"로 오인하거나 반대로 실제 회귀를 "버전 차이려니" 하고 넘어가는 실수가 생긴다.

**발생 조건**:
- VMAF/SSIM 등 외부 모델·라이브러리에 의존하는 지표를 export에 포함할 때.
- 디코더를 내부적으로 라이브러리 업그레이드하면서 export 스키마는 그대로 둘 때.

**권장**:
```rust
#[derive(serde::Serialize)]
pub struct ToolchainInfo {
    pub bitvue_version: String,
    pub decoder_versions: std::collections::BTreeMap<String, String>, // "dav1d": "1.4.0"
    pub vmaf_model: String,       // "vmaf_v0.6.1"
    pub build_commit: String,     // git short hash
}

#[derive(serde::Serialize)]
pub struct AnalysisExport {
    pub toolchain: ToolchainInfo,
    pub psnr_summary: PsnrSummary,
    pub vmaf_score: f64,
}
```
- export 최상위에 사용된 모든 외부 모델/라이브러리의 버전을 기록하는 `toolchain` 섹션을 둔다.
- 빌드 시점의 git commit hash도 함께 기록해, "이 결과를 만든 정확한 빌드"를 재현 가능하게 한다.
- CI 회귀 테스트에서 툴체인 버전이 다르면 값 비교 대신 경고만 내도록 분기한다.

**탐지 방법**:
- Structural: export 스키마에 버전/툴체인 정보 필드 존재 여부 확인.
- Manual: 지표(PSNR/SSIM/VMAF) 계산에 관여하는 외부 의존성 목록과 export 필드를 대조.

**예외**:
- 외부 모델에 의존하지 않는 순수 결정적 계산(비트 카운트, NAL 개수 등)은 버전 기록의 필요성이 낮다.

**Bitvue 판정**: Confirmed — `crates/bitvue-metrics/src/vmaf.rs`의 `VmafConfig.model_path: Option<String>`은 계산 입력으로만 쓰이고, VMAF 점수를 반환하는 경로 어디에도 사용된 모델 경로/버전을 함께 돌려주는 필드가 없음(`toolchain`/`model_version` 계열 필드 저장소 전체 grep 0건, 재검증 시점에도 동일). VMAF는 CMP-06 기준 옵션 Cargo feature로 미배선 상태(`PARITY_CHECKLIST.md`)라 실제 소비 경로 자체가 아직 얕음

---

### SER-016: endian·bit depth·color metadata 누락
**분류**: 메타데이터 누락 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct RawPlaneExport {
    pub width: u32,
    pub height: u32,
    #[serde(serialize_with = "as_base64")]
    pub samples: Vec<u8>,   // 10bit 샘플을 u16으로 저장했는데
                             // little-endian인지, bit depth가 8/10/12인지,
                             // 컬러 스페이스가 BT.601/709/2020인지 전혀 기록 안 됨
}
```

**문제**:
- 10/12bit 샘플을 바이트 배열로 저장할 때 endianness(리틀/빅 엔디안)와 패킹 방식(2바이트에 10bit을 어떻게 채우는지: LSB-justified vs MSB-justified)을 명시하지 않으면, 다른 플랫폼이나 다른 도구로 읽을 때 값이 통째로 틀어진다.
- bit depth 정보 없이 raw 샘플만 있으면 8bit인지 10bit인지 값 범위(0~255 vs 0~1023)로 추측해야 하는데, 이는 신뢰할 수 없다(8bit 콘텐츠도 상위 비트가 0인 10bit처럼 보일 수 있음).
- 컬러 스페이스(BT.601/709/2020)와 컬러 레인지(full/limited)를 기록하지 않으면, 같은 YUV 값이라도 렌더링 시 완전히 다른 색으로 보일 수 있어 재import 후 시각적 비교가 무의미해진다.
- 이런 메타데이터 누락은 파싱 에러처럼 명확히 실패하지 않고 "그럴듯하지만 미묘하게 틀린" 이미지/값을 만들어내기 때문에 발견이 매우 늦어진다.

**발생 조건**:
- HDR 콘텐츠(10/12bit) 분석 결과나 raw plane 데이터를 export할 때.
- hex view/YUV export처럼 원본 샘플을 그대로 내보내는 기능.
- 서로 다른 팀/도구가 만든 export 파일을 상호 교환할 때(플랫폼 endian이 다를 수 있음).

**권장**:
```rust
#[derive(serde::Serialize)]
pub struct RawPlaneExport {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,               // 8, 10, 12
    pub endian: Endian,              // Little, Big
    pub sample_packing: SamplePacking, // LsbJustified, MsbJustified
    pub color_space: ColorSpace,     // Bt601, Bt709, Bt2020Ncl, ...
    pub color_range: ColorRange,     // Full, Limited
    #[serde(serialize_with = "as_base64")]
    pub samples: Vec<u8>,
}

#[derive(serde::Serialize)]
pub enum Endian { Little, Big }
```
- raw 샘플을 내보낼 때는 bit depth, endian, 패킹 방식, 컬러 스페이스/레인지를 **항상** 명시적 필드로 동반한다.
- 가능하면 이 메타데이터를 별도 struct(`PixelFormatInfo`)로 묶어 여러 export 타입이 공유하게 해, 어느 한 곳에서 빠뜨리는 실수를 줄인다.
- import 경로에서 이 필드들이 없으면 명시적으로 에러를 내도록 하여 "추측해서 읽기"를 원천 차단한다.

**탐지 방법**:
- Structural: `Vec<u8>`/`Vec<u16>` 형태의 샘플 데이터를 담는 export DTO에 `bit_depth`/`color_space`/`endian` 필드가 동반되는지 검사.
- Manual: HDR/10bit 콘텐츠 fixture로 export → import 라운드트립 후 픽셀 값이 원본과 일치하는지 시각적/수치적 검증.

**예외**:
- 항상 8bit BT.601로 고정된 내부 디버그 전용 덤프이며 문서에 그 고정 가정이 명시된 경우는 필드 생략이 실용적일 수 있다.

**Bitvue 판정**: Confirmed — (경로 갱신) 현재 YUV 와이어 타입인 `DecodedYuvFrame`(`crates/bitvue-sidecar/src/decode_bridge.rs:26-38`)은 `width`/`height`/`bit_depth`/`chroma_subsampling`/스트라이드만 있고 `color_space`/`color_range`/endian 필드가 없음. 코덱 파서(`crates/bitvue-vp9/src/frame_header.rs:190,196`의 `color_space: ColorSpace`/`color_range: bool`)는 실제로 이 값을 파싱하는데도 sidecar 응답 구조체에 도달하기 전에 버려짐 — 구 감사가 지적한 문제가 Tauri→Electron 이관 후에도 그대로 재현됨

---

### SER-017: frontend DTO와 export DTO를 공유
**분류**: API/Export 결합 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// frontend(Tauri IPC)용으로 설계된 타입을 CLI export에도 그대로 재사용
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FrameViewModel {
    pub frame_index: u32,
    pub thumbnail_data_url: String,   // 프런트 렌더링 편의를 위한 data: URL
    pub is_selected: bool,            // UI 상태(!)
    pub is_expanded_in_tree: bool,    // UI 상태(!)
    pub qp_display_color: String,     // "#ff0000" — UI 색상 매핑 결과
}

pub fn export_cli(frames: &[FrameViewModel]) -> String {
    serde_json::to_string(frames).unwrap()   // UI 상태가 그대로 파일에 저장됨
}
```

**문제**:
- `is_selected`, `is_expanded_in_tree` 같은 UI 상태가 CLI export 파일에 섞여 들어가면, 이 파일을 나중에 다시 읽는 외부 도구/스크립트 입장에서는 의미 없는 노이즈가 된다.
- `qp_display_color`처럼 "값이 아니라 렌더링 결과"인 필드가 저장되면, 컬러 매핑 로직(임계값, 팔레트)이 바뀔 때마다 과거 export 파일의 의미가 달라지거나 무효화된다.
- 프런트엔드 편의를 위한 필드(예: `thumbnail_data_url`)가 CLI/MCP 소비자에게는 불필요한 대역폭·저장 공간 낭비다.
- 반대 방향으로도 문제가 된다: export 스키마의 안정성 요구(버전 호환)가 UI 타입에 전이되어, UI를 자유롭게 리팩터링하기 어려워진다.

**발생 조건**:
- "타입 하나로 다 쓰면 편하다"는 이유로 Tauri IPC 응답 타입을 CLI export 함수에 그대로 전달할 때.
- 프로토타입 단계에서 시작된 단일 DTO가 여러 소비자(프런트, CLI, MCP)로 퍼져나갈 때.

**권장**:
```rust
// 도메인 모델 → 두 개의 별도 DTO로 각각 변환
#[derive(serde::Serialize)]
pub struct FrameViewModel {   // 프런트엔드 전용, UI 상태 포함 가능
    pub frame_index: u32,
    pub thumbnail_data_url: String,
    pub is_selected: bool,
}

#[derive(serde::Serialize)]
pub struct FrameExport {      // 파일로 나가는 안정적 스키마, UI 상태 없음
    pub schema_version: u32,
    pub frame_index: u32,
    pub frame_type: FrameTypeExport,
}

impl From<&Frame> for FrameViewModel { /* UI 편의 필드 채움 */ }
impl From<&Frame> for FrameExport { /* 안정적 필드만 채움 */ }
```
- "화면에 그리기 위한 타입"과 "디스크/외부 프로세스로 나가는 타입"을 처음부터 분리한다.
- 두 타입 모두 동일한 도메인 모델(`Frame`)에서 `From`으로 파생시켜, 소스가 하나임에도 계약은 분리되게 한다.
- UI 상태 필드는 이름에 (`ui_`, `is_selected` 등) 접두어를 붙여 export 스키마 리뷰에서 쉽게 걸러지게 한다.

**탐지 방법**:
- Structural: CLI export/MCP 응답 함수가 참조하는 타입이 Tauri `#[tauri::command]` 반환 타입과 동일한지 대조.
- Manual: DTO 필드 이름에 `is_selected`, `_color`, `data_url` 등 UI 냄새가 나는 필드가 export 경로에 흘러들었는지 리뷰.

**예외**:
- 내부 전용 디버그 덤프이며 UI 상태 포함이 의도된 경우(세션 복원용 스냅샷 등)는 공유해도 무방하다.

**Bitvue 판정**: N/A — 반례를 확인함: `crates/bitvue-engine/src/timeline.rs:69`의 `TimelineFrame`은 UI 상태(`is_selected: bool`)를 포함하지만, export DTO인 `FrameExportRow::from_timeline_frame`(`crates/bitvue-engine/src/export/frames.rs:24-36`)이 `is_selected`를 명시적으로 제외하고 안정 필드만 옮겨 담음 — 의도적으로 분리되어 있음(경로만 `bitvue-core`→`bitvue-engine`으로 갱신, 결론 불변)

---

### SER-018: partial result와 final result 구분 없음
**분류**: 상태 표현 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
pub struct AnalysisResult {
    pub frames_analyzed: u32,
    pub total_frames: u32,
    pub psnr_summary: PsnrSummary,   // 분석이 중간에 취소돼도 지금까지의 부분 평균이 채워짐
}

// 사용자가 취소하거나 타임아웃돼도 같은 타입, 같은 필드 구조로 반환
#[tauri::command]
fn get_analysis_result(state: tauri::State<AppState>) -> AnalysisResult {
    state.analysis.lock().unwrap().current_result()
}
```

**문제**:
- `frames_analyzed < total_frames`인 상태(진행 중 취소, 타임아웃, 크래시 후 부분 저장)와 정상 완료 상태가 완전히 동일한 타입·필드 구조를 가지므로, 소비자가 `frames_analyzed`와 `total_frames`를 일일이 비교하지 않으면 부분 결과를 완료된 결과로 오인하기 쉽다.
- MCP 클라이언트나 CLI 스크립트가 이 결과를 그대로 다운스트림(회귀 비교, 리포트 생성)에 사용하면, "분석이 절반만 됐는데 마치 전체 분석인 것처럼" 잘못된 결론을 낸다.
- 파일로 저장된 partial export와 final export가 파일 이름/확장자로도 구분되지 않으면, 나중에 디렉터리를 훑어볼 때 어느 것이 신뢰할 수 있는 결과인지 알 수 없다.

**발생 조건**:
- 사용자가 긴 분석을 중간에 취소하거나, 타임아웃/크래시로 분석이 중단됐지만 지금까지의 결과는 저장하고 싶을 때.
- MCP 서버가 장시간 작업을 progress-report 방식으로 응답하다가 마지막에 최종 결과를 반환하는 구조일 때, 중간 응답과 최종 응답의 타입이 동일할 때.

**권장**:
```rust
#[derive(serde::Serialize)]
#[serde(tag = "status")]
pub enum AnalysisResult {
    #[serde(rename = "completed")]
    Completed { psnr_summary: PsnrSummary, frames_analyzed: u32 },
    #[serde(rename = "partial")]
    Partial { psnr_summary_so_far: PsnrSummary, frames_analyzed: u32, total_frames: u32, reason: PartialReason },
}

#[derive(serde::Serialize)]
pub enum PartialReason { Cancelled, Timeout, Crashed }
```
- 완료/부분 상태를 타입 레벨(tagged enum)로 분리해, 소비자가 `match`를 강제로 거치지 않으면 값을 꺼낼 수 없게 한다.
- 부분 결과 파일은 파일명에 `.partial.json`처럼 명시적 마커를 붙인다.
- 부분 결과의 통계 필드 이름 자체도 `_so_far` 접미어 등으로 "완결되지 않았음"을 값 이름에서부터 드러낸다.

**탐지 방법**:
- Structural: 분석 결과 타입이 취소/타임아웃 경로와 정상 완료 경로에서 동일한 구조체를 반환하는지 확인.
- Runtime: 분석 도중 취소하는 통합 테스트를 만들어 반환된 export의 `status`/구분 필드를 검증.

**예외**:
- 애초에 취소·타임아웃이 불가능한 짧고 원자적인 연산(단일 프레임 파싱 등)은 이 구분이 불필요하다.

**Bitvue 판정**: N/A(구 판정보다 더 강하게 성립) — 구 감사는 "취소 API 자체가 없다"고 판정했지만, 현재 sidecar는 실제 `cancel_request` 메커니즘을 갖추고 있음(`crates/bitvue-sidecar/src/main.rs:96` `CancelRegistry`, 178-224행 `spawn_request`). 취소된 요청은 성공 응답과 같은 모양의 partial 데이터가 아니라 `WireErrorCode::Cancelled`를 담은 별개의 `Response::failure`로 구분되어 반환됨(206-207행) — 실행 시작 전에만 취소를 체크하므로("cancelled before execution started", 200행) 계산 도중 부분 완료 데이터가 성공 응답에 섞여 나가는 경로 자체가 없어, 이 항목이 경고하는 실패 모드가 구조적으로 발생 불가

---

### SER-019: unknown field를 무조건 거부
**분류**: 스키마 진화 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]   // 모든 import 경로에 무조건 적용
pub struct ProjectFile {
    pub schema_version: u32,
    pub stream_path: String,
    pub bookmarks: Vec<Bookmark>,
}

// 새 버전에서 "tags" 필드를 추가해 저장한 파일을 옛 버전 Bitvue로 열면
// deny_unknown_fields 때문에 "tags"를 이해 못 해 파일 전체를 거부한다.
```

**문제**:
- `deny_unknown_fields`는 오타/스키마 드리프트를 조기에 잡아주는 장점이 있지만, 미래 버전이 추가한 필드를 담은 파일을 **과거 버전**이 열 때는 "이해할 수 없는 부가 정보 하나 때문에 파일 전체를 열지 못하는" 과도한 실패를 유발한다.
- 사용자가 새 버전에서 저장한 프로젝트 파일을 팀원의 구 버전 Bitvue로 열어보려 할 때 완전히 막혀버리면, 실무적으로 매우 불편한 경험이 된다.
- forward-compatibility(구 버전이 신 버전 파일의 "일부"라도 읽을 수 있는 능력)를 스키마 설계 단계에서부터 포기하게 만든다.

**발생 조건**:
- 프로젝트 저장 파일(북마크, 주석, 세션 상태)처럼 사용자가 여러 버전의 앱을 오가며 열어볼 가능성이 있는 포맷.
- "엄격한 스키마 검증"이 좋다는 일반론만으로 모든 역직렬화 경로에 `deny_unknown_fields`를 일괄 적용했을 때.

**권장**:
```rust
#[derive(serde::Deserialize)]
pub struct ProjectFile {      // deny_unknown_fields 없음 — 알 수 없는 필드는 무시
    pub schema_version: u32,
    pub stream_path: String,
    pub bookmarks: Vec<Bookmark>,
}

// 반대로, "정확히 이 API 계약을 지켜야 하는" 내부 IPC 경계에서는
// 여전히 deny_unknown_fields로 오타/드리프트를 조기에 잡는다.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct McpToolRequest { /* 엄격 검증이 필요한 단발성 요청 */ }
```
- 장기간 디스크에 남고 여러 버전의 애플리케이션이 오갈 수 있는 파일(프로젝트 파일, export 산출물)은 **알 수 없는 필드를 무시**하도록 두어 forward-compatibility를 확보한다.
- 반대로 같은 빌드 내에서만 오가는 단발성 요청·응답(IPC 인자, MCP 요청 스키마)은 `deny_unknown_fields`로 오타를 조기에 잡는 것이 여전히 유효하다 — "무엇을 얼마나 오래 보관하는가"에 따라 정책을 다르게 가져간다.
- 무시된 필드를 완전히 버리지 않고 `#[serde(flatten)] extra: serde_json::Map<...>`로 보존해, 재저장 시 미래 필드가 손실되지 않게 하는 방법도 고려한다.

**탐지 방법**:
- Structural: `deny_unknown_fields`가 붙은 타입 목록을 뽑아, 그중 디스크에 장기 보관되는 파일 포맷용 타입이 있는지 대조.
- Manual: "이 파일을 다른 버전의 앱으로 열 가능성이 있는가?"를 스키마 설계 체크리스트에 포함.

**예외**:
- 정확히 하나의 빌드/버전에서만 소비되는 게 보장된 임시 IPC/요청 스키마는 엄격 검증이 이득이 더 크다.

**Bitvue 판정**: N/A — 저장소 전체에서 `deny_unknown_fields` 사용처 0건(grep, 재검증 시점에도 동일) — Evidence Bundle 모듈은 이를 명시적 설계 결정으로 문서화하기까지 함(`crates/bitvue-engine/src/export/evidence.rs:19` "no `deny_unknown_fields`"), 이 항목이 경고하는 "무조건 거부"가 실제로 일어날 코드가 없음

---

### SER-020: 오래된 schema migration 경로 없음
**분류**: 스키마 진화 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
pub fn load_project(bytes: &[u8]) -> Result<ProjectFile, LoadError> {
    // schema_version을 읽긴 하지만, 최신 버전이 아니면 그냥 에러
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let version = value["schema_version"].as_u64().unwrap_or(0);
    if version != CURRENT_SCHEMA_VERSION {
        return Err(LoadError::UnsupportedVersion(version));
        // v1으로 저장된 1년 전 프로젝트 파일은 영구히 열 수 없다
    }
    serde_json::from_value(value).map_err(Into::into)
}
```

**문제**:
- 버전 필드는 있지만(SER-002는 지켰지만) 옛 버전을 새 버전으로 끌어올리는 마이그레이션 코드가 없으면, 결국 "버전이 다르면 거부"만 반복하게 되어 SER-002가 해결하려던 문제(호환성 판별)만 가능해질 뿐 실제 호환은 여전히 안 된다.
- 사용자가 1년 전에 저장한 프로젝트 파일, 북마크, export 결과가 최신 버전 앱에서 영구히 열리지 않으면 "장기 보관 가능한 파일 포맷"이라는 신뢰가 깨진다.
- 마이그레이션 경로가 없다는 것은 곧 "스키마를 절대 바꿀 수 없다"는 압박으로 이어져, 실제로는 잘못된 설계를 계속 끌고 가게 되는 역효과를 낳는다.

**발생 조건**:
- 스키마 버전을 v2, v3로 올릴 때마다 "마이그레이션은 나중에"라며 미루다가 지원 대상 버전이 계속 쌓일 때.
- 초기 버전(v1) 사용자가 소수라 마이그레이션 우선순위가 낮게 잡힐 때 — 그러나 그 소수의 파일도 결국 열려야 한다.

**권장**:
```rust
type Migration = fn(serde_json::Value) -> Result<serde_json::Value, LoadError>;

const MIGRATIONS: &[(u64, Migration)] = &[
    (1, migrate_v1_to_v2),
    (2, migrate_v2_to_v3),
];

pub fn load_project(bytes: &[u8]) -> Result<ProjectFile, LoadError> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    let mut version = value["schema_version"].as_u64().unwrap_or(0);
    for (from_version, migrate) in MIGRATIONS {
        if version == *from_version {
            value = migrate(value)?;
            version = value["schema_version"].as_u64().unwrap();
        }
    }
    if version != CURRENT_SCHEMA_VERSION {
        return Err(LoadError::UnsupportedVersion(version));
    }
    serde_json::from_value(value).map_err(Into::into)
}
```
- 버전을 올릴 때마다 "N → N+1" 마이그레이션 함수를 반드시 함께 추가하고, 로더는 이 체인을 순차 적용해 항상 최신 스키마로 끌어올린다.
- 마이그레이션 함수 각각에 대한 단위 테스트(고정된 v1 fixture → 예상되는 v2 결과)를 작성해, 마이그레이션 로직 자체의 회귀를 방지한다.
- 지원을 끊을 버전(예: v1은 3개 메이저 버전 뒤 지원 종료)에 대한 정책을 명시하고, 종료 시점에는 명확한 에러 메시지("v1 파일은 더 이상 지원되지 않습니다, Bitvue 2.x로 먼저 여세요")를 제공한다.

**탐지 방법**:
- Structural: `schema_version`을 다루는 로더에 버전별 분기(마이그레이션 함수 호출)가 존재하는지 확인.
- Runtime: 과거 버전 fixture 파일들을 저장소에 보관하고 CI에서 항상 최신 로더로 열어보는 회귀 테스트.

**예외**:
- 아직 v1만 존재하고 한 번도 릴리스되지 않은 스키마는 마이그레이션이 필요 없다(아직 "과거"가 없다).

**Bitvue 판정**: N/A(구 판정 근거는 낡았으나 결론은 유지) — 구 감사는 "schema_version도 마이그레이션 코드도 전무"라 판정했지만 이제는 Evidence Bundle에 실제 버전 필드·정책이 존재함(SER-002/SER-011 참고: `CURRENT_BUNDLE_SCHEMA_VERSION`, `check_bundle_schema_compatibility`, `crates/bitvue-engine/src/export/evidence.rs:9-26`). 다만 이 항목이 요구하는 "N→N+1 마이그레이션 체인"(`migrate_v1_to_v2`류 변환 함수)은 여전히 없음 — 단, 번들이 아직 1.0에서 한 번도 breaking 변경을 겪지 않아 마이그레이션할 대상 자체가 없고, 정책상 필드 추가는 `#[serde(default)]`로 자동 흡수되므로 지금까지는 실질적으로 마이그레이션이 필요했던 적이 없음. "옛 파일을 영구히 못 연다"는 이 항목의 나쁜 예가 실제로 재현되는 코드 경로는 없어 N/A 유지, 다만 근거는 SER-002/011과 함께 갱신 필요

---

### SER-021: CLI/MCP/Tauri 세 경로의 schema가 독립적으로 drift
**분류**: 다중 인터페이스 일관성 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// cli/export.rs
#[derive(serde::Serialize)]
pub struct FrameExportCli {
    pub frame_index: u32,
    pub frame_type: String,
    pub qp_avg: f64,
}

// mcp/tools/frame_info.rs — 같은 개념을 별도로 재정의
#[derive(serde::Serialize)]
pub struct FrameInfoMcp {
    pub index: u32,          // 이름이 다름: frame_index vs index
    pub kind: String,        // 이름이 다름: frame_type vs kind
    pub qp: f32,             // 타입도 다름: f64 vs f32
}

// tauri/commands/frame.rs — 또 별도로 재정의
#[derive(serde::Serialize)]
pub struct FrameDto {
    pub frame_index: u32,
    pub frame_type: FrameTypeEnum,   // 여기만 enum, 나머지는 String
    pub qp_avg: f64,
}
```

**문제**:
- 동일한 도메인 개념(프레임의 QP 평균)이 세 인터페이스에서 이름·타입이 제각각이라, "CLI export 결과와 MCP 응답이 같은 값을 가리키는지"를 확인하려면 매번 코드를 대조해야 한다.
- 한 경로에서 버그(예: QP 계산 공식 오류)를 고치면서 다른 두 경로의 동일 로직은 놓치는 일이 반복되기 쉽다 — 세 개의 독립된 struct는 세 개의 독립된 (잠재) 버그 표면이다.
- 외부 사용자가 "CLI로 export한 값과 MCP로 조회한 값이 다르다"는 리포트를 올렸을 때, 실제로는 같은 계산인데 직렬화 경로가 달라 반올림/타입 차이로 값이 미세하게 다른 경우 원인 규명이 오래 걸린다.
- 문서화 부담도 3배가 된다 — 같은 개념을 세 번 설명해야 하고, 셋 중 하나만 업데이트되고 나머지는 낡은 채로 남는 일이 흔하다.

**발생 조건**:
- CLI, MCP 서버, Tauri 커맨드가 서로 다른 시점에 서로 다른 개발자에 의해 추가되어 공용 DTO 계층 없이 각자 타입을 정의했을 때.
- "이 인터페이스는 이 값만 필요하니 필드를 줄이자"는 부분 최적화가 반복되며 세 타입이 조금씩 갈라질 때.

**권장**:
```rust
// dto/frame.rs — 세 인터페이스가 공유하는 단일 정의
#[derive(serde::Serialize, Clone)]
pub struct FrameSummary {
    pub frame_index: u32,
    pub frame_type: FrameTypeExport,
    pub qp_avg: f64,
}

// 각 인터페이스는 FrameSummary를 그대로 쓰거나,
// 꼭 필요한 경우에만 명시적으로 하위 집합을 도출한다.
pub fn to_mcp_response(s: &FrameSummary) -> serde_json::Value {
    serde_json::to_value(s).unwrap()   // 필드명이 그대로 유지됨
}
```
- 세 인터페이스가 공유하는 "핵심 DTO 계층"을 하나 두고, 인터페이스별 특수 요구사항(필드 생략, 추가 메타데이터)은 이 DTO를 감싸는 래퍼로 표현한다.
- 필드 이름·타입이 인터페이스마다 달라야 할 정당한 이유(예: MCP는 LLM 소비용이라 더 설명적인 이름이 필요)가 있다면, 명시적 변환 함수와 함께 그 이유를 주석으로 남긴다.
- 계약 테스트(같은 입력 프레임에 대해 세 인터페이스가 의미적으로 동일한 값을 반환하는지)를 CI에 추가한다.

**탐지 방법**:
- Structural: `cli/`, `mcp/`, `tauri/` 각 모듈에서 이름이 비슷한(`Frame*Export`, `Frame*Dto`, `*FrameInfo`) 타입을 자동 수집해 필드 diff.
- Manual: 신규 필드 추가 PR이 세 경로 중 하나에만 반영되고 나머지는 그대로인지 리뷰에서 확인.

**예외**:
- 인터페이스별 소비자 요구가 근본적으로 다른 필드(예: MCP 전용 자연어 설명 필드)는 공유 DTO에 억지로 넣지 않고 별도로 둔다.

**Bitvue 판정**: Confirmed(경로 갱신) — 구 감사가 인용한 Tauri `QualityMetrics`는 폐기됐지만, 동일한 "품질 지표" 개념에 대해 여전히 세 개의 독립 타입이 존재: CLI `FrameMetrics{frame,psnr_db,ssim}`(`crates/bitvue-cli/src/commands/quality.rs:15-19`, ref-vs-distorted 전체 비교), Electron sidecar `DiffMetrics{frame_index,psnr_y,psnr_u,psnr_v,psnr_avg,ssim_y,max_diff_y,has_mismatch}`(`crates/bitvue-sidecar/src/debug_yuv.rs:484-493`, Debug YUV 세션 비교 전용), MCP `McpMetricsSummary{metric_type,stats,histogram_bins,worst_frames:Vec<(usize,f32)>}`(`crates/bitvue-engine/src/mcp.rs:319`) — 필드명·타입·구조 모두 제각각이고 공유 DTO 계층 없음(용도가 미묘하게 다르긴 하나 — "psnr" 값을 각기 다른 이름/타입/스코프로 표현하는 세 벌의 독립 정의라는 핵심 문제는 동일)

---

### SER-022: #[serde(default)]로 필드 누락을 조용히 은폐
**분류**: 스키마 진화 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
#[derive(serde::Deserialize)]
pub struct FrameExport {
    pub frame_index: u32,
    #[serde(default)]
    pub qp_avg: f64,          // 필드가 없으면 0.0 — "QP=0"과 "값 없음"이 구분 불가
    #[serde(default)]
    pub frame_type: FrameTypeExport,  // enum의 Default variant가 임의로 선택됨
}

impl Default for FrameTypeExport {
    fn default() -> Self { FrameTypeExport::Intra }  // "모르면 일단 I프레임"
}
```

**문제**:
- `#[serde(default)]`는 원래 "필드 추가 시 하위 호환을 위한 안전장치"인데, 여기서는 실제로 손상되었거나 의도적으로 생략된 필드까지 조용히 기본값으로 채워버려 데이터 손상을 정상 데이터처럼 위장한다.
- `qp_avg: 0.0`이 "실제로 QP가 0"인지 "필드가 아예 없어서 기본값이 채워진 것"인지 소비자가 구분할 수 없다 — 통계 계산에 이 0.0이 섞여 들어가면 평균이 왜곡된다.
- `FrameTypeExport::default() == Intra`처럼 enum에 임의의 기본값을 부여하면, 실제로는 알 수 없는 프레임 타입인데 "I프레임"으로 잘못 분류되어 이후 분석(GOP 구조 추정 등)이 틀어진다.
- 이런 은폐는 대개 "이번 필드는 마이그레이션 대상이 아니라서 편의상 `default`를 붙였다"는 국지적 판단에서 시작되지만, 파일이 실제로 손상된 경우까지 같은 코드 경로로 조용히 통과시킨다.

**발생 조건**:
- 스키마 버전을 올리며 새 필드를 추가할 때, "옛 파일엔 이 필드가 없을 테니 `default`를 붙이면 되겠지"라고 기계적으로 적용할 때.
- 실제로는 파일 손상/버그로 필드가 빠졌는데, 마이그레이션 목적의 `default`가 이를 구분 없이 흡수할 때.

**권장**:
```rust
#[derive(serde::Deserialize)]
pub struct FrameExportRaw {
    pub frame_index: u32,
    pub qp_avg: Option<f64>,          // 명시적으로 Option — 없으면 None으로 남긴다
    pub frame_type: Option<FrameTypeExport>,
}

pub fn resolve(raw: FrameExportRaw, source_version: u32) -> Result<FrameExport, LoadError> {
    // 버전에 따라 "이 필드가 원래 없었던 게 정상인가"를 명시적으로 판단
    let qp_avg = match (raw.qp_avg, source_version) {
        (Some(v), _) => v,
        (None, v) if v < 3 => f64::NAN,   // v3 이전엔 필드 자체가 없었음 — "측정 안 됨"
        (None, _) => return Err(LoadError::CorruptField("qp_avg")),  // v3 이후인데 없으면 손상
    };
    Ok(FrameExport { frame_index: raw.frame_index, qp_avg, /* ... */ })
}
```
- 마이그레이션 목적이 아니라면 `#[serde(default)]`를 필드에 기계적으로 붙이지 않고, `Option<T>`으로 받아 "없음"을 명시적으로 표현한다.
- "이 버전부터는 이 필드가 필수"라는 경계가 있다면, 마이그레이션 코드(SER-020)에서 버전별로 분기해 기본값 채움과 손상 감지를 구분한다.
- enum에 `Default`를 구현할 때는 "정말 안전한 기본값"이 있는 경우로 한정하고, 없다면 `Unknown` variant를 두어 모호함을 명시적으로 남긴다.

**탐지 방법**:
- Static: `#[serde(default)]`가 붙은 필드 목록을 뽑아, 각각이 "버전 경계가 명확한 마이그레이션 목적"인지 리뷰.
- Runtime: 필드를 의도적으로 제거한 손상 fixture로 역직렬화했을 때 에러 대신 기본값으로 조용히 통과하는지 테스트.

**예외**:
- 정말로 "없으면 이 값"이 도메인적으로 항상 옳은 필드(예: 신규 boolean 플래그의 `false` 기본값)는 `#[serde(default)]`가 적절하다.

**Bitvue 판정**: N/A(경로·건수 갱신) — 구 감사는 4곳(그 중 `syntax.rs`는 Electron 이관 후 파일 자체가 사라짐)을 인용했으나, 재검증 시점 저장소 전체에서 `#[serde(default)]` 사용처는 테스트 제외 약 15곳(`bitvue-sidecar/{debug_yuv,main,context_menu,evidence_export}.rs`, `bitvue-engine/{insight_feed,stream_state,parity_harness/mod,export/evidence}.rs`, `bitvue-protocol/lib.rs`)으로 늘어남. 전부 확인한 결과 `Option<T>`/`Vec<T>`/`bool`/`serde_json::Value`(+ Evidence Bundle의 문서화된 struct-level default) 필드뿐이며, 유일한 스칼라 예외인 `debug_yuv.rs:73`의 `picture_offset: i64`도 디스크에 영구 저장되는 export가 아니라 세션 내 IPC 요청 파라미터(0=오프셋 없음이 항상 유효한 기본값)라 이 항목이 경고하는 "손상 은폐"에 해당하지 않음
</content>
