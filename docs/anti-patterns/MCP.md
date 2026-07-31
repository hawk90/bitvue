# Anti-Pattern Catalog — MCP: MCP·AI 연동

이 문서는 Bitvue 안티패턴 카탈로그의 한 분류이며, 전체 목록은 `docs/anti-patterns/INDEX.md`(별도 작성)를 참고한다. Wave 1-3(파싱/캐시/동시성/UI 등)에 이은 Phase 4 웨이브로, 비트스트림 분석기의 내부 상태를 LLM에 노출하는 MCP(Model Context Protocol) 연동 계층을 다룬다. Bitvue는 실제로 `crates/bitvue-mcp`(질의 도구를 제공하는 독립 바이너리 `bitvue-mcp-server`)와 `crates/bitvue-core/src/mcp.rs`(현재 서버 바이너리에는 연결되지 않은 별도의 `McpIntegration`/resources 모델)를 워크스페이스에 가지고 있으므로, 이 카탈로그는 추후 2단계 저장소 감사에서 그 두 코드 경로에 실측 대조된다. 이 카테고리의 반복 주제는 컨텍스트 예산, hallucination 위험, deterministic 분석과 AI 해석 사이의 경계, 정보 유출 네 가지다.

---

### MCP-001: 전체 syntax tree를 한 번에 모델에 전달
**분류**: MCP · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct ParseTreeResource {
    root: SyntaxNode, // NAL 단위 전체를 재귀적으로 담은 트리
}

async fn tool_get_syntax_tree(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let unit = ctx.parsed_stream(stream_id)?; // 수만 개 NAL/OBU를 포함할 수 있음
    let tree = unit.to_syntax_tree(); // 필터·깊이 제한 없이 전체 직렬화
    McpToolResult::json(ParseTreeResource { root: tree })
}
```

**문제**:
- HEVC/AV1 스트림 하나의 syntax tree는 프레임당 수백~수천 개 노드(slice header, CU, PU, TU, MB partition)를 가지며, 수천 프레임짜리 스트림 전체를 직렬화하면 수십~수백 MB의 JSON이 생성된다.
- LLM 컨텍스트 윈도우는 이를 감당하지 못해 truncation이 발생하고, 잘린 JSON은 파싱 실패 또는 트리 구조가 깨진 채로 모델에 전달된다.
- 토큰 비용이 폭발해 응답 지연·요금 급증으로 이어지고, 실제로 모델이 필요로 하는 정보(예: "이 프레임의 QP 분포")는 트리 전체 중 극히 일부에 불과하다.

**발생 조건**:
- 사용자가 "이 프레임 이상해 보이는데 왜 그런지 설명해줘" 같은 개방형 질문을 했을 때, 도구가 스코프를 좁히지 않고 전체 구조를 반환하는 경우.
- 디버깅 편의를 위해 "일단 다 주고 모델이 알아서 찾게 하자"는 설계를 택한 경우.

**권장**:
```rust
#[derive(serde::Serialize)]
struct SyntaxNodeSummary {
    node_type: String,
    frame_idx: u64,
    offset: u64,
    size: u32,
    key_fields: BTreeMap<String, String>, // 노드 타입별 핵심 필드만
}

async fn tool_get_syntax_node(
    ctx: &AnalyzerContext,
    stream_id: StreamId,
    frame_idx: u64,
    max_depth: u8,
) -> McpToolResult {
    let unit = ctx.parsed_stream(stream_id)?;
    let frame_node = unit.node_for_frame(frame_idx)
        .ok_or(McpError::not_found("frame_idx out of range"))?;
    let summary = frame_node.summarize(max_depth.min(3)); // 깊이 상한 강제
    McpToolResult::json(summary)
}
```
- 프레임/영역 단위로 스코프를 강제하고, 트리 깊이 상한을 도구 시그니처에 명시한다.
- 전체 구조가 필요하면 "다음 레벨 조회"용 별도 도구(drill-down)를 제공해 모델이 필요한 만큼만 요청하게 한다.

**탐지 방법**:
- Structural: MCP 도구 함수 시그니처에 `frame_idx`, `range`, `max_depth` 같은 스코프 파라미터 없이 `StreamId`만 받아 전체 트리를 반환하는 패턴을 grep.
- Runtime: 도구 응답 페이로드 크기를 로깅해 임계치(예: 100KB) 초과 빈도를 측정.

**예외**:
- 스트림 자체가 매우 작은 경우(수 프레임, 테스트용 클립)는 전체 반환이 실용적일 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-002: raw bitstream을 무제한 context에 포함
**분류**: MCP · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
async fn tool_get_frame_bytes(ctx: &AnalyzerContext, stream_id: StreamId, frame_idx: u64) -> McpToolResult {
    let frame = ctx.raw_frame_bytes(stream_id, frame_idx)?; // 수 MB의 압축 바이트
    // base64 인코딩만 하고 그대로 모델에 넘김 — 크기 제한 없음
    McpToolResult::text(base64::encode(&frame))
}
```

**문제**:
- 압축된 프레임이라도 base64 인코딩 시 원본의 약 1.33배로 커지며, I-frame 하나가 수백 KB~수 MB인 경우 단일 도구 호출로 컨텍스트 예산을 소진한다.
- LLM은 raw 바이트를 "이해"할 수 없다 — 토큰만 소비하고 의미 있는 추론에 기여하지 못하는 순수 낭비다.
- 여러 프레임을 연달아 조회하면 대화 세션 자체가 컨텍스트 한도를 넘어 이전 분석 맥락이 잘려나간다.

**발생 조건**:
- "AI가 바이트 패턴을 보고 뭔가 알아낼 수 있지 않을까"라는 근거 없는 기대로 raw dump를 노출하는 경우.
- hex view 패널과 동일한 데이터 소스를 MCP 도구로 그대로 재노출할 때, UI용 페이지네이션 로직을 빼먹는 경우.

**권장**:
```rust
async fn tool_get_frame_summary(ctx: &AnalyzerContext, stream_id: StreamId, frame_idx: u64) -> McpToolResult {
    let frame = ctx.raw_frame_bytes(stream_id, frame_idx)?;
    McpToolResult::json(FrameByteSummary {
        size_bytes: frame.len(),
        first_16_bytes_hex: hex::encode(&frame[..frame.len().min(16)]),
        entropy_estimate: shannon_entropy(&frame),
        // raw payload는 별도의 파일 저장/링크 참조로만 제공, 모델에는 넣지 않음
        export_path: ctx.export_frame_to_temp(stream_id, frame_idx)?,
    })
}
```
- raw 바이트가 정말 필요하면 파일로 내보내고 경로/해시만 모델에 전달한다(사람이나 후속 도구가 열람).
- 모델에는 항상 요약 통계(크기, 엔트로피, 헤더 필드)만 제공한다.

**탐지 방법**:
- Structural: MCP 도구 반환 타입에 `Vec<u8>`/`base64::encode` 호출이 크기 상한 검사 없이 존재하는지 grep.
- Runtime: 도구 응답 바이트 수를 프레임 크기와 비교해 1:1에 가까우면 raw dump로 판정.

**예외**:
- 명시적으로 "바이트 비교" 목적의 개발자 전용 디버그 도구이며, 모델이 아닌 사람이 직접 호출·검토하는 채널이라면 예외.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-003: frame ID 없이 자연어 결과만 반환
**분류**: MCP · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
async fn tool_analyze_quality_drop(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let drops = ctx.detect_quality_drops(stream_id)?;
    // "몇 번째 프레임"인지 구조화하지 않고 텍스트로만 뭉뚱그림
    let text = format!(
        "스트림에서 화질 저하가 몇 군데 감지되었습니다. 주로 중반부와 후반부에 나타나며, \
         모션이 심한 장면에서 두드러집니다.",
    );
    McpToolResult::text(text)
}
```

**문제**:
- 사용자가 "그 프레임으로 이동해줘"라고 요청해도 UI가 앵커링할 수 있는 `frame_idx`/`timestamp`가 응답에 없어 후속 액션이 불가능하다.
- "중반부", "후반부" 같은 자연어 위치 표현은 스트림 길이에 따라 완전히 다른 프레임을 가리킬 수 있어 재현 불가능하다.
- 같은 질문을 다시 던졌을 때 모델이 다른 표현을 생성하면 이전 답변과 대조할 기준점이 없다.

**발생 조건**:
- 도구 반환 스키마가 자유 텍스트(`McpToolResult::text`)만 허용하고 구조화 필드를 강제하지 않을 때.
- 프롬프트 엔지니어링으로 "친절하게 설명해줘"를 강조하다 구조화된 근거 필드가 누락될 때.

**권장**:
```rust
#[derive(serde::Serialize)]
struct QualityDropFinding {
    frame_idx: u64,
    timestamp_ms: u64,
    metric: String,   // e.g. "vmaf"
    value: f64,
    baseline: f64,
}

async fn tool_analyze_quality_drop(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let drops: Vec<QualityDropFinding> = ctx.detect_quality_drops(stream_id)?;
    McpToolResult::json(drops) // 자연어 요약은 모델이 이 구조화 데이터를 보고 스스로 생성하게 함
}
```
- 모든 "발견(finding)"에는 `frame_idx`(또는 byte offset)를 필수 필드로 강제한다.
- 자연어 설명은 구조화 데이터의 부가 필드로만 두고, UI 네비게이션은 구조화 필드에서만 파생시킨다.

**탐지 방법**:
- Structural: MCP 도구 반환 타입에 `frame_idx`/`offset` 필드가 없는 순수 `String`/`text` 응답을 grep.
- Manual: UI에서 "해당 위치로 이동" 버튼이 도구 응답을 소비할 때 파싱 가능한 좌표가 있는지 리뷰.

**예외**:
- 프레임 단위가 아닌 스트림 전체 요약(코덱 종류, 컨테이너 정보 등)처럼 본질적으로 위치가 없는 질의는 예외.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-004: AI 설명을 분석 사실과 동일하게 표시
**분류**: MCP · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```tsx
// AI 도구 응답과 deterministic 파서 결과를 같은 컴포넌트로 렌더링
function FrameInfoPanel({ fact, aiExplanation }: { fact: ParsedFrameInfo; aiExplanation: string }) {
  return (
    <div className="info-panel">
      <Row label="QP" value={fact.qp} />
      <Row label="Frame Type" value={fact.frameType} />
      {/* 파서가 뽑은 값과 시각적으로 구분되지 않음 */}
      <Row label="설명" value={aiExplanation} />
    </div>
  );
}
```

**문제**:
- 사용자는 `Row` 컴포넌트가 동일하게 렌더링되므로 AI 설명도 파서가 검증한 사실이라고 오인한다.
- HEVC/AV1 신택스는 규격상 결정론적으로 파싱 가능한데, hallucination이 섞인 텍스트가 같은 신뢰 등급으로 노출되면 분석 도구의 정확성 자체가 의심받는다.
- 버그 리포트나 QA 라운드에서 "파서가 틀렸다"는 잘못된 결론이 AI 설명 오류 때문에 발생할 수 있다.

**발생 조건**:
- AI 요약 기능을 빠르게 붙이면서 기존 정보 패널 컴포넌트를 재사용할 때.
- 디자인 시스템에 "AI 생성 콘텐츠" 전용 스타일 토큰이 정의되어 있지 않을 때.

**권장**:
```tsx
function FrameInfoPanel({ fact, aiExplanation }: { fact: ParsedFrameInfo; aiExplanation?: string }) {
  return (
    <div className="info-panel">
      <Row label="QP" value={fact.qp} sourceBadge="parsed" />
      <Row label="Frame Type" value={fact.frameType} sourceBadge="parsed" />
      {aiExplanation && (
        <AiExplanationBlock text={aiExplanation} disclaimer="AI 생성 — 검증되지 않은 해석입니다" />
      )}
    </div>
  );
}
```
- deterministic 값과 AI 해석을 별도 컴포넌트/시각 스타일(아이콘, 배경색, 라벨)로 항상 구분한다.
- AI 블록에는 고정된 disclaimer와, 가능하면 근거가 된 필드로의 링크를 함께 표시한다.

**탐지 방법**:
- Manual/Structural: 프론트엔드에서 파서 결과와 AI 응답이 동일한 컴포넌트/props 셰이프를 공유하는지 코드 리뷰.
- Semantic: 디자인 QA에서 AI 콘텐츠와 사실 콘텐츠를 스크린샷 diff로 구분 가능한지 확인.

**예외**:
- 내부 개발자 전용 디버그 뷰로, 명시적으로 "raw MCP response"라고 표기된 화면이라면 구분 없이 표시해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-005: 불확실성·근거 위치가 없음
**분류**: MCP · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct AiFinding {
    summary: String, // "이 프레임은 블록킹 아티팩트가 있습니다" — 확신도, 근거 없음
}
```

**문제**:
- 모델이 스스로 얼마나 확신하는지, 어떤 필드/픽셀 영역을 근거로 판단했는지 알 수 없어 사용자가 결과를 검증할 방법이 없다.
- 동일한 입력에도 모델 샘플링 편차로 다른 결론이 나올 수 있는데, 확신도가 없으면 "우연히 맞은 답"과 "근거 있는 답"을 구별할 수 없다.
- 후속 자동화(예: 확신도 낮은 결과는 사람 검토 큐로 보내기)를 구현할 데이터가 애초에 없다.

**발생 조건**:
- 도구 응답 스키마를 설계할 때 confidence/evidence 필드를 처음부터 넣지 않은 경우.
- 모델 프롬프트가 "확신을 가지고 답하라"는 식으로 확신도 표현을 억제하는 경우.

**권장**:
```rust
#[derive(serde::Serialize)]
struct AiFinding {
    summary: String,
    confidence: f32,              // 0.0-1.0, 모델이 self-report
    evidence: Vec<EvidenceRef>,   // 근거가 된 frame_idx/offset/metric
}

#[derive(serde::Serialize)]
struct EvidenceRef {
    frame_idx: u64,
    field: String,
    observed_value: String,
}
```
- 모든 AI 도구 응답 스키마에 `confidence`와 `evidence` 필드를 필수로 강제한다.
- confidence가 임계치 미만이면 UI에서 "불확실 — 추가 검토 필요" 배지를 강제로 붙인다.

**탐지 방법**:
- Structural: AI 관련 응답 구조체(`Ai*`, `*Finding`)에 `confidence`/`evidence` 필드 존재 여부를 grep.
- Semantic: 실제 응답 샘플을 모아 confidence 분포가 항상 1.0 근처로 몰려 있는지(=사실상 무의미한 필드인지) 점검.

**예외**:
- 순수 deterministic 계산 결과(파서가 직접 뽑은 QP 값 등)에는 confidence 개념이 필요 없다 — 이 항목은 AI 생성 콘텐츠에만 적용.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-006: 모델이 생성한 offset을 검증 없이 이동
**분류**: MCP · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
async fn tool_apply_seek(ctx: &mut AnalyzerContext, ai_suggested_offset: u64) -> McpToolResult {
    // 모델이 응답에 포함한 byte offset을 그대로 신뢰
    ctx.seek_to_byte_offset(ai_suggested_offset)?; // 범위 밖이면 파서가 UB에 가까운 상태로 진입 가능
    McpToolResult::text("이동했습니다")
}
```

**문제**:
- LLM이 생성한 숫자는 hallucination일 수 있고, 실제 스트림 범위를 벗어나거나 NAL 경계와 무관한 임의의 바이트를 가리킬 수 있다.
- 검증 없이 파서 상태를 그 offset으로 이동시키면 잘못된 정렬(misalignment)로 이후 파싱이 전부 깨지거나 패닉/크래시로 이어질 수 있다.
- 이 도구가 실제 UI 네비게이션(Filmstrip, hex view)과 연결되어 있다면, 잘못된 offset이 사용자에게 그대로 표시되는 화면 훼손으로 번진다.

**발생 조건**:
- "AI가 offset을 찾아서 바로 이동" 같은 원스텝 UX를 구현할 때 중간 검증 단계를 생략한 경우.
- offset 단위(byte vs bit vs frame index)에 대한 스키마 계약이 모호해 모델이 잘못된 단위로 값을 생성한 경우.

**권장**:
```rust
async fn tool_apply_seek(ctx: &mut AnalyzerContext, ai_suggested_offset: u64) -> McpToolResult {
    let unit = ctx.parsed_stream_ref()?;
    let nearest_boundary = unit.nearest_nal_boundary(ai_suggested_offset)
        .ok_or(McpError::invalid_arg("offset out of stream range"))?;
    if nearest_boundary.distance_from(ai_suggested_offset) > MAX_SNAP_TOLERANCE {
        return Err(McpError::invalid_arg("offset does not align to a known unit boundary"));
    }
    ctx.seek_to_byte_offset(nearest_boundary.offset)?;
    McpToolResult::json(SeekResult { snapped_to: nearest_boundary.offset, requested: ai_suggested_offset })
}
```
- 모델이 준 offset은 항상 "제안값"으로 취급하고, 알려진 유닛 경계에 스냅되는지 검증한 뒤에만 적용한다.
- 범위/정렬 검증 실패 시 조용히 무시하지 말고 명시적 에러를 반환해 모델이 재시도하거나 사용자에게 알리게 한다.

**탐지 방법**:
- Structural: 모델 응답 필드를 직접 `seek_to_*`, `read_at_offset` 같은 위험 API에 전달하는 경로에서 중간 검증 함수 호출이 있는지 grep.
- Runtime: fuzzing으로 임의의 offset을 도구에 주입해 패닉/OOB가 발생하는지 확인.

**예외**:
- offset이 이미 deterministic 파서가 생성한 값(모델이 그대로 반환만 하는 경우)이라면 재검증 비용이 낮아 생략 가능하지만, 여전히 range check는 유지해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-007: tool call 결과의 schema validation 없음
**분류**: MCP · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
async fn handle_tool_call(name: &str, args: serde_json::Value) -> McpToolResult {
    match name {
        "get_frame_info" => {
            let frame_idx = args["frame_idx"].as_u64().unwrap(); // 타입/존재 가정, panic 위험
            let info = analyze_frame(frame_idx);
            McpToolResult::json(info) // 응답도 구조 검증 없이 직렬화만
        }
        _ => McpToolResult::error("unknown tool"),
    }
}
```

**문제**:
- `args["frame_idx"].as_u64().unwrap()`은 모델이 문자열이나 음수, 또는 필드를 아예 누락한 JSON을 보내면 그대로 패닉한다.
- 응답 측에서도 스키마 검증이 없으면 내부 리팩터링으로 필드가 빠지거나 이름이 바뀌어도 컴파일은 통과하고 MCP 클라이언트(모델)만 조용히 망가진 데이터를 받는다.
- MCP 프로토콜은 여러 클라이언트(Claude, 다른 LLM, 테스트 하네스)가 붙을 수 있는데, 스키마가 코드에만 암묵적으로 존재하면 계약이 문서화되지 않는다.

**발생 조건**:
- 도구를 빠르게 프로토타이핑하면서 `serde_json::Value`를 직접 인덱싱할 때.
- MCP SDK가 JSON Schema 선언을 지원함에도 도구 등록 시 생략한 경우.

**권장**:
```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GetFrameInfoArgs {
    frame_idx: u64,
    stream_id: StreamId,
}

async fn handle_tool_call(name: &str, args: serde_json::Value) -> McpToolResult {
    match name {
        "get_frame_info" => {
            let parsed: GetFrameInfoArgs = serde_json::from_value(args)
                .map_err(|e| McpError::invalid_arg(format!("schema mismatch: {e}")))?;
            let info = analyze_frame(parsed.stream_id, parsed.frame_idx)?;
            McpToolResult::json(info) // info 타입도 JsonSchema derive로 스키마 공개
        }
        _ => McpToolResult::error("unknown tool"),
    }
}
```
- 모든 도구 입력/출력 타입에 `JsonSchema`(또는 동등한 스키마 선언)를 derive하고 도구 등록 시 함께 노출한다.
- 입력 역직렬화 실패는 panic이 아니라 구조화된 `McpError`로 반환한다.

**탐지 방법**:
- Static: `args[...].as_*().unwrap()` / `.expect(...)` 패턴을 MCP 핸들러 내에서 grep.
- Structural: 도구 등록 코드에 스키마 선언(JsonSchema derive 또는 수동 스키마 상수)이 없는 항목을 목록화.

**예외**:
- 내부 전용, 입력이 없는(no-arg) 도구는 입력 스키마 검증이 불필요하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-008: 동일한 질문에 매번 전체 분석 재전송
**분류**: MCP · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
async fn tool_summarize_stream(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    // 매 호출마다 전체 스트림을 다시 스캔하고 다시 직렬화
    let stats = ctx.compute_full_stream_stats(stream_id)?; // 수초~수십초 소요
    McpToolResult::json(stats)
}
```

**문제**:
- 같은 스트림에 대해 모델이 대화 중 여러 번 같은 도구를 호출하면(예: 후속 질문마다) 매번 전체 재계산 비용을 지불한다.
- 분석 대상 스트림이 세션 동안 변하지 않는데도 캐싱이 없으면 응답 지연이 누적되어 대화형 UX가 느려진다.
- 반복 호출로 인한 서버 리소스(CPU, I/O) 낭비가 다중 세션 환경에서 배가된다.

**발생 조건**:
- 모델이 대화 맥락을 유지하지 못해(또는 프롬프트 설계상) 같은 도구를 반복 호출하는 패턴에서.
- 스트림 자체는 불변(이미 로드되어 분석 중인 파일)인데 도구가 이를 활용하지 않을 때.

**권장**:
```rust
async fn tool_summarize_stream(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let key = CacheKey::new(stream_id, ctx.stream_content_hash(stream_id)?);
    if let Some(cached) = ctx.mcp_cache.get(&key) {
        return McpToolResult::json(cached);
    }
    let stats = ctx.compute_full_stream_stats(stream_id)?;
    ctx.mcp_cache.put(key, stats.clone());
    McpToolResult::json(stats)
}
```
- 스트림 콘텐츠 해시를 캐시 키에 포함해(MCP-016 참조) 동일 입력에 대한 재계산을 방지한다.
- 캐시 적중 여부를 응답 메타데이터에 표시해 디버깅과 지연시간 분석을 돕는다.

**탐지 방법**:
- Runtime: 동일 `(tool_name, args)` 쌍의 호출 지연시간이 매번 동일하게 크면(캐시 미스 상태) 의심.
- Structural: 계산 비용이 큰 도구 핸들러 내부에 캐시 조회 코드가 없는지 grep.

**예외**:
- 스트림이 라이브 캡처처럼 계속 변하는 소스라면 매번 재계산이 정당하다 — 이 경우 캐시 무효화 전략이 핵심이지 캐시 부재 자체가 문제는 아니다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-009: 개인정보·파일 경로·메타데이터 과다 노출
**분류**: MCP · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
async fn tool_get_stream_info(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let meta = ctx.stream_metadata(stream_id)?;
    McpToolResult::json(serde_json::json!({
        "container": meta.container_format,
        "codec": meta.codec,
        // 로컬 파일 시스템 절대 경로를 그대로 노출
        "source_path": meta.absolute_path, // "/Users/jdoe/Desktop/client_footage_confidential.mp4"
        "creation_tool": meta.encoder_tag,   // 인코더가 남긴 사용자/호스트명 문자열 그대로
    }))
}
```

**문제**:
- 절대 파일 경로에는 사용자명, 회사 프로젝트명, 기밀 정보로 추정되는 파일명이 포함될 수 있으며, 이 값이 외부 LLM API로 전송되면 로컬 정보가 제3자 서비스로 유출된다.
- 인코더/뮤서(muxer)가 파일에 남기는 메타데이터(작성자, 조직명, GPS 태그 등 컨테이너 레벨 메타데이터)도 동일한 경로로 유출될 수 있다.
- 이런 노출은 MCP 서버 로그, 모델 제공자의 요청 로그 등 여러 지점에 잔류해 삭제가 어렵다.

**발생 조건**:
- 디버깅 편의를 위해 내부 구조체를 그대로 직렬화(`#[derive(Serialize)]`로 전체 메타데이터 struct를 노출)할 때.
- 로컬 전용 도구를 원격 LLM API와 연동할 때 "로컬이니 괜찮다"고 가정한 경우.

**권장**:
```rust
async fn tool_get_stream_info(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let meta = ctx.stream_metadata(stream_id)?;
    McpToolResult::json(serde_json::json!({
        "container": meta.container_format,
        "codec": meta.codec,
        "filename_only": meta.absolute_path.file_name(), // 경로가 아닌 파일명만
        // encoder_tag, GPS 등 사용자 식별 가능 메타데이터는 allowlist에 없으면 기본 제외
    }))
}
```
- 응답 필드를 allowlist 방식으로 명시하고, 새 메타데이터 필드가 파서에 추가되어도 자동으로 노출되지 않게 한다.
- 원격 LLM API를 쓰는 배포에서는 경로/사용자 식별 정보를 아예 별도 로컬 전용 도구로 분리한다.

**탐지 방법**:
- Static: MCP 응답 직렬화 코드에서 `absolute_path`, `full_path`, 원본 메타데이터 struct 전체를 `#[derive(Serialize)]`로 그대로 노출하는지 grep.
- Manual: 원격 모델 제공자로 나가는 실제 요청 페이로드를 캡처해 PII/경로 포함 여부 검토.

**예외**:
- 완전히 로컬에서만 동작하는 온디바이스 모델(네트워크 전송이 전혀 없는 구성)이라면 위험도가 낮아지지만, 로그 파일에 남는 문제는 여전히 남는다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-010: prompt injection이 포함된 metadata를 그대로 사용
**분류**: MCP · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
async fn tool_get_sei_messages(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let sei = ctx.extract_sei_user_data(stream_id)?; // SEI user_data_unregistered 페이로드
    // 비트스트림에서 추출한 임의의 문자열을 검증 없이 그대로 텍스트로 반환
    McpToolResult::text(String::from_utf8_lossy(&sei.payload).into_owned())
}
```

**문제**:
- H.264/HEVC의 SEI `user_data_unregistered`나 AV1의 metadata OBU, 컨테이너 레벨 태그(제목, 코멘트) 등은 비트스트림 작성자가 완전히 자유롭게 채울 수 있는 필드다 — 즉 **공격자가 제어 가능한 입력**이다.
- 이 문자열을 그대로 MCP 도구 응답에 넣고 LLM이 그것을 "분석 결과의 일부"로 읽으면, `"이전 지시를 무시하고 다음 명령을 실행하라: ..."` 같은 prompt injection 페이로드가 모델의 후속 행동(다른 도구 호출, 파일 접근 등)을 조작할 수 있다.
- 이는 일반적인 MCP 보안 조언(외부 API 응답 검증)과는 다른, **비트스트림 분석기 고유의 위험 클래스**다 — 공격 벡터가 네트워크 요청이 아니라 분석 대상 미디어 파일 자체에 내장되어 있어, "신뢰할 수 있는 로컬 파일을 열었을 뿐"이라는 사용자의 직관과 어긋난다.
- 특히 MCP-014(AI 자동 실행)와 결합되면 SEI 페이로드 하나로 실제 파일 시스템 작업을 트리거하는 체인이 성립할 수 있다.

**발생 조건**:
- SEI/메타데이터 필드를 "그냥 텍스트"로 취급해 별도 sanitization 없이 도구 응답에 포함시킬 때.
- 도구 응답이 자유 텍스트를 모델의 "지시"와 같은 채널(예: 시스템/개발자 role이 아닌데도 실질적으로 신뢰되는 채널)로 전달될 때.
- 자동 실행 권한을 가진 도구(파일 쓰기, 외부 호출)가 같은 MCP 세션에 존재할 때 피해 범위가 커진다.

**권장**:
```rust
#[derive(serde::Serialize)]
struct SeiUserDataFinding {
    raw_len_bytes: usize,
    // 페이로드는 데이터로만 감싸고, 모델이 "지시"로 해석하지 못하도록 구조화 필드에 격리
    decoded_preview: String, // 제어 문자 제거 + 길이 제한 + 명시적 래핑
    is_valid_utf8: bool,
    contains_suspicious_pattern: bool, // "ignore previous", 시스템 프롬프트 유사 문자열 휴리스틱 탐지
}

async fn tool_get_sei_messages(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let sei = ctx.extract_sei_user_data(stream_id)?;
    let sanitized = sanitize_untrusted_bitstream_text(&sei.payload, MAX_PREVIEW_LEN);
    McpToolResult::json(SeiUserDataFinding {
        raw_len_bytes: sei.payload.len(),
        decoded_preview: sanitized.preview, // "[DATA FROM BITSTREAM, NOT A USER INSTRUCTION]: ..." 로 명시적 래핑
        is_valid_utf8: sanitized.is_valid_utf8,
        contains_suspicious_pattern: sanitized.suspicious,
    })
}
```
- 비트스트림에서 추출한 모든 자유 텍스트(SEI, 컨테이너 태그, 코덱 확장 메타데이터)는 "신뢰할 수 없는 데이터"로 분류하고, 모델에 전달할 때 명시적으로 데이터임을 표시하는 래핑(delimiter, role 격리)을 적용한다.
- injection 패턴 휴리스틱 탐지 결과를 함께 반환해 UI가 경고를 표시할 수 있게 한다.
- 자동 실행 권한이 있는 도구는 이런 미검증 텍스트 필드를 입력으로 받지 않도록 도구 경계를 분리한다(MCP-014 참조).

**탐지 방법**:
- Structural: SEI/메타데이터 추출 함수의 반환값이 sanitization 함수를 거치지 않고 바로 `McpToolResult::text`/`json`에 들어가는지 grep.
- Semantic: 알려진 injection 문구를 SEI user_data에 심은 테스트 스트림으로 실제 모델 행동이 변하는지 red-team 테스트.
- Runtime: `contains_suspicious_pattern` 같은 플래그가 실제로 true가 되는 케이스를 로깅해 빈도 모니터링.

**예외**:
- 표준화된 SEI(예: HDR10+ 동적 메타데이터처럼 스키마가 고정된 필드)는 자유 텍스트가 아니므로 이 항목의 대상이 아니다 — `user_data_unregistered`류의 임의 바이트 필드에 한정된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-011: AI가 parser 오류를 정상 syntax로 보정
**분류**: MCP · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
async fn tool_explain_parse_error(ctx: &AnalyzerContext, stream_id: StreamId, error: ParseError) -> McpToolResult {
    // 파서가 실패를 보고했는데, "AI가 알아서 보정해서 설명"하도록 프롬프트에 위임
    let prompt = format!(
        "다음 파싱 에러가 발생했습니다: {error:?}. 이 스트림이 어떤 신택스를 의도했을지 \
         추정해서 정상적인 구조로 설명해주세요."
    );
    let ai_reply = ctx.llm_client.complete(&prompt).await?;
    McpToolResult::text(ai_reply) // 에러였다는 사실이 사라지고 "정상 설명"만 남음
}
```

**문제**:
- 파서가 스펙 위반 또는 손상된 비트스트림을 감지했다는 사실 자체가 중요한 진단 정보인데, 모델이 "그럴듯한 정상 구조"로 재구성해서 답하면 실제 손상/버그가 은폐된다.
- 사용자는 "AI가 설명해준 신택스"를 실제 파일의 내용으로 오인해, 정작 고쳐야 할 손상된 스트림 문제를 놓친다.
- 이는 코덱 분석기의 핵심 가치(정확한 스펙 준수 여부 판별)를 정면으로 훼손하는 패턴이다 — 도구가 신뢰할 수 없으면 존재 이유가 없다.

**발생 조건**:
- 파서 에러 메시지가 사용자 친화적이지 않아, "AI가 매끄럽게 설명해주면 UX가 좋아질 것"이라는 동기로 에러를 모델에 그대로 위임할 때.
- 에러와 정상 결과를 구분하는 응답 스키마가 없어 모델이 둘을 섞어 답할 여지가 있을 때.

**권장**:
```rust
async fn tool_explain_parse_error(ctx: &AnalyzerContext, stream_id: StreamId, error: ParseError) -> McpToolResult {
    // 1) 에러 사실 자체는 절대 AI를 거치지 않고 그대로 노출
    let structured_error = StructuredParseError {
        offset: error.offset,
        expected: error.expected_syntax_element.clone(),
        found_bytes_hex: hex::encode(&error.raw_bytes[..error.raw_bytes.len().min(16)]),
        spec_reference: error.spec_section.clone(),
    };
    // 2) AI에게는 "설명을 생성"하도록만 위임하되, 응답에는 반드시 구조화 에러가 동반됨
    let ai_context = ctx.llm_client
        .explain_error_in_plain_language(&structured_error) // 신택스를 "보정"하지 말라는 제약을 프롬프트에 명시
        .await
        .ok(); // AI 실패해도 구조화 에러는 살아있음(MCP-017 참조)
    McpToolResult::json(ParseErrorResponse {
        is_error: true, // 명시적 플래그 — AI 응답과 절대 혼동되지 않음
        structured_error,
        ai_plain_language_hint: ai_context,
    })
}
```
- 파서 에러는 항상 구조화된 필드(오프셋, 기대값, 실제 바이트, 스펙 참조)로 우선 반환하고, AI는 "쉬운 말로 풀어쓰기"에만 국한시킨다.
- AI 프롬프트에 "신택스를 추정하거나 정상으로 보정하지 말라"는 제약을 명시적으로 건다.

**탐지 방법**:
- Semantic: 파서 에러 경로가 AI 호출을 거친 뒤 `is_error`/에러 플래그 없이 순수 텍스트로만 반환되는 케이스를 리뷰.
- Runtime: 손상된 테스트 스트림을 입력으로 넣고 AI 응답에 "정상적으로 보인다", "문제없음" 같은 표현이 섞이는지 회귀 테스트.

**예외**:
- 에러가 사용자 의도(예: 의도적으로 잘라낸 스트림)로 알려진 경우, AI가 "왜 이 에러가 났는지" 원인을 설명하는 것은 유효하다 — 다만 이때도 에러 사실 자체를 지우면 안 된다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-012: 모델 버전·프롬프트 버전 미기록
**분류**: MCP · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(serde::Serialize)]
struct AiAnalysisResult {
    summary: String,
    // 어떤 모델/프롬프트로 생성됐는지 기록이 전혀 없음
}
```

**문제**:
- 모델 제공자가 모델을 업데이트하거나 프롬프트 템플릿을 수정하면 동일한 질문에 다른 답이 나올 수 있는데, 어떤 버전이 이 결과를 만들었는지 추적 불가능하다.
- 버그 리포트("AI가 이상한 답을 했다")를 재현하려 해도 당시 모델/프롬프트 버전을 알 수 없어 디버깅이 막힌다.
- 캐시된 AI 결과(MCP-008, MCP-016)가 모델/프롬프트 업그레이드 후에도 무효화되지 않고 계속 재사용될 위험이 있다.

**발생 조건**:
- AI 결과 저장/캐싱/로깅 스키마를 설계할 때 provenance 필드를 처음부터 빠뜨린 경우.
- 프롬프트 템플릿이 코드에 인라인 문자열로 흩어져 있어 "버전"이라는 개념 자체가 없는 경우.

**권장**:
```rust
#[derive(serde::Serialize)]
struct AiAnalysisResult {
    summary: String,
    provenance: AiProvenance,
}

#[derive(serde::Serialize, Clone)]
struct AiProvenance {
    model_id: String,      // "claude-sonnet-5-20260115" 등 정확한 버전 문자열
    prompt_template_id: String,
    prompt_template_version: u32,
    generated_at: chrono::DateTime<chrono::Utc>,
}
```
- 프롬프트 템플릿을 버전이 붙은 리소스(파일 또는 상수)로 관리하고, 모든 AI 응답에 `AiProvenance`를 첨부한다.
- 로그·캐시·UI 표시 어디서든 이 provenance를 함께 저장해 사후 추적 가능하게 한다.

**탐지 방법**:
- Structural: AI 결과 반환 타입에 `model_id`/`prompt_version` 계열 필드가 없는지 grep.
- Manual: 버그 리포트 템플릿에 AI 응답 재현에 필요한 provenance 항목이 포함되어 있는지 검토.

**예외**:
- 프로토타입/실험 단계의 내부 전용 도구로, 정식 릴리스 전이라 추적성 요구사항이 아직 없는 경우.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-013: deterministic 분석과 AI 해석의 경계 없음
**분류**: MCP · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
async fn tool_get_frame_report(ctx: &AnalyzerContext, stream_id: StreamId, frame_idx: u64) -> McpToolResult {
    let parsed = ctx.parse_frame(stream_id, frame_idx)?; // deterministic: QP, MV, partition 등
    let ai_guess = ctx.llm_client.estimate_visual_quality(&parsed).await.ok(); // AI: 주관적 추정
    // 하나의 struct로 합쳐져 호출자가 어디까지 신뢰 가능한지 구분할 수단이 없음
    McpToolResult::json(serde_json::json!({
        "qp": parsed.qp,
        "mv_count": parsed.mv_count,
        "perceived_quality": ai_guess, // 다른 신뢰 등급인데 형제 필드로 나열됨
    }))
}
```

**문제**:
- 같은 응답 안에서 스펙에 근거한 100% 재현 가능한 값(QP, 파티션 구조)과 모델의 확률적 추정값(perceived_quality)이 구조적으로 동급으로 취급된다.
- 다운스트림 소비자(UI, 다른 도구, 테스트)가 필드 이름만 보고 무엇이 검증 가능한 값인지 판단할 수 없어, MCP-004/MCP-020과 같은 후속 오류를 유발하는 근본 원인이 된다.
- 이 스키마를 기반으로 캐싱·재현성 테스트를 설계하면 AI 필드의 비결정성이 전체 응답의 재현성을 오염시킨다.

**발생 조건**:
- 도구를 "한 번의 호출로 풍부한 답을 주자"는 목표로 설계해 deterministic/AI 소스를 한 응답에 합칠 때.
- 응답 스키마 설계 단계에서 데이터 출처(provenance)를 타입 레벨에서 강제하지 않을 때.

**권장**:
```rust
#[derive(serde::Serialize)]
struct FrameReport {
    deterministic: DeterministicFacts, // 파서가 생성, 100% 재현 가능
    ai_interpretation: Option<AiInterpretation>, // 선택적, 별도 신뢰 등급
}

#[derive(serde::Serialize)]
struct DeterministicFacts { qp: i32, mv_count: u32 /* ... */ }

#[derive(serde::Serialize)]
struct AiInterpretation { perceived_quality: String, provenance: AiProvenance, confidence: f32 }
```
- 타입 시스템 레벨에서 두 신뢰 등급을 별도 struct로 강제 분리해, 실수로 섞는 것 자체를 컴파일 타임에 어렵게 만든다.
- `ai_interpretation`은 항상 `Option`으로 두어 AI 실패 시에도 `deterministic` 필드만으로 도구가 동작하게 한다(MCP-017과 연결).

**탐지 방법**:
- Structural: 응답 struct 하나에 파서 출력 필드와 `llm_client` 호출 결과 필드가 같은 레벨에 나열되어 있는지 grep.
- Manual: 스키마 리뷰에서 "이 필드는 100% 재현 가능한가?" 질문에 아니오라고 답하는 필드가 별도 하위 구조로 분리되어 있는지 확인.

**예외**:
- AI 해석이 deterministic 값에 대한 순수 텍스트 포맷팅(예: "QP 32는 상대적으로 높은 편입니다"처럼 값 자체의 재진술)에 불과하고 원본 값이 그대로 노출되어 있다면 위험도가 낮다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-014: user action을 AI가 자동 실행
**분류**: MCP · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
async fn tool_fix_and_export(ctx: &mut AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let issues = ctx.detect_issues(stream_id)?;
    for issue in issues {
        // 모델의 판단만으로 파일 시스템에 쓰기 작업을 즉시 실행 — 사용자 확인 없음
        ctx.apply_fix_and_overwrite(stream_id, &issue)?;
    }
    McpToolResult::text("문제를 감지하고 자동으로 수정했습니다")
}
```

**문제**:
- 모델의 "문제 감지"는 확률적 추정인데, 그 판단만으로 원본 파일을 덮어쓰는 비가역적 작업이 사용자 확인 없이 실행된다.
- MCP-010(prompt injection)과 결합하면, 스트림 안에 심어진 악의적 메타데이터가 이 자동 실행 체인을 트리거해 실제 파일 시스템 조작으로 이어질 수 있다.
- "자동으로 수정했다"는 통보만 있고 무엇을 어떻게 바꿨는지 이전 상태로 되돌릴 수단이 없으면 신뢰 회복이 불가능하다.

**발생 조건**:
- "AI 에이전트가 알아서 문제를 고쳐주는" 데모 지향 기능을 구현하면서 승인 단계를 생략한 경우.
- 도구 권한 모델이 read-only 도구와 mutating 도구를 구분하지 않고 동일한 신뢰 수준으로 노출된 경우.

**권장**:
```rust
async fn tool_propose_fix(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let issues = ctx.detect_issues(stream_id)?;
    let proposals: Vec<FixProposal> = issues.into_iter()
        .map(|issue| ctx.plan_fix(&issue)) // 계획만 생성, 실행하지 않음
        .collect::<Result<_, _>>()?;
    McpToolResult::json(FixProposalResponse {
        proposals,
        requires_user_approval: true, // 스키마 레벨에서 실행 불가를 명시
    })
}

// 실제 실행은 별도의, 사용자 UI 액션으로만 호출 가능한 non-MCP 경로
fn apply_approved_fix(ctx: &mut AnalyzerContext, proposal_id: FixProposalId) -> Result<(), AppError> {
    ctx.apply_fix_and_overwrite(/* ... */)
}
```
- 상태를 변경하는(mutating) 작업은 MCP 도구에서 "제안(propose)"까지만 하고, 실제 실행은 사용자가 명시적으로 트리거하는 별도 경로로 분리한다.
- 되돌리기 가능한 작업(undo/버전 보관)이 아니면 자동화 후보에서 제외한다.

**탐지 방법**:
- Structural: MCP 도구 핸들러 내부에서 파일 쓰기/덮어쓰기 API(`overwrite`, `apply_fix`, `save`)가 사용자 승인 플로우 없이 직접 호출되는지 grep.
- Manual: 도구 카탈로그를 read-only/mutating으로 분류하고, mutating 도구에 승인 게이트가 있는지 감사.

**예외**:
- 되돌리기 쉬운 작업(예: 임시 미리보기 파일 생성처럼 원본을 건드리지 않는 부작용)은 자동 실행이 허용될 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-015: 대형 도구 결과 pagination 없음
**분류**: MCP · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
async fn tool_list_all_frames(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let frames = ctx.all_frame_summaries(stream_id)?; // 수만 개일 수 있음
    McpToolResult::json(frames) // 페이지네이션 파라미터 자체가 없음
}
```

**문제**:
- 프레임 수가 많은 스트림(수 분 길이, 수만 프레임)에서 전체 목록을 한 번에 반환하면 MCP-001/MCP-002와 동일한 컨텍스트 폭발이 발생한다.
- 모델이 "다음 페이지" 개념 없이 전체를 한 번에 받으면, 뒷부분 데이터가 컨텍스트에서 먼저 밀려나거나 truncate되어 앞부분 편향(recency/primacy bias)이 생긴다.
- 도구 시그니처에 `offset`/`limit`이 없으면 클라이언트가 부분 조회를 요청할 방법 자체가 없다.

**발생 조건**:
- 목록형 도구(`list_*`, `get_all_*`)를 설계할 때 "일단 다 주자"는 접근을 취한 경우.
- 개발 중 테스트한 스트림이 짧아서(수십 프레임) 문제가 드러나지 않다가 실제 사용자 파일(수천~수만 프레임)에서 터지는 경우.

**권장**:
```rust
async fn tool_list_frames(
    ctx: &AnalyzerContext,
    stream_id: StreamId,
    offset: u64,
    limit: u32, // 서버 측 상한(예: 200)을 강제
) -> McpToolResult {
    let limit = limit.min(MAX_PAGE_SIZE);
    let total = ctx.frame_count(stream_id)?;
    let page = ctx.frame_summaries_range(stream_id, offset, limit)?;
    McpToolResult::json(PagedResponse {
        items: page,
        offset,
        limit,
        total,
        has_more: offset + limit as u64 < total,
    })
}
```
- 모든 목록형 도구에 `offset`/`limit`과 서버 측 상한을 강제하고, 응답에 `total`/`has_more`를 포함해 모델이 다음 호출을 계획할 수 있게 한다.
- 큰 목록은 기본적으로 요약(예: 프레임 타입별 카운트)부터 반환하고, 상세 목록은 별도 drill-down 도구로 제공한다.

**탐지 방법**:
- Structural: `list_*`/`get_all_*` 이름의 MCP 도구 시그니처에 `offset`/`limit` 파라미터가 없는지 grep.
- Runtime: 긴 테스트 스트림(수천 프레임 이상)으로 각 목록형 도구를 호출해 응답 크기 분포 측정.

**예외**:
- 결과 개수가 구조적으로 작다고 보장되는 도구(예: "코덱 프로파일 목록"처럼 열거형 개수가 고정)는 페이지네이션이 불필요하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-016: cache key에 모델·prompt·analysis version 누락
**분류**: MCP · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
struct AiResultCache {
    inner: HashMap<(StreamId, u64 /* frame_idx */), AiAnalysisResult>,
}

impl AiResultCache {
    fn get_or_compute(&mut self, stream_id: StreamId, frame_idx: u64) -> AiAnalysisResult {
        // 캐시 키에 모델/프롬프트/분석 로직 버전이 전혀 반영되지 않음
        self.inner.entry((stream_id, frame_idx))
            .or_insert_with(|| compute_ai_analysis(stream_id, frame_idx))
            .clone()
    }
}
```

**문제**:
- 프롬프트 템플릿을 수정하거나 모델을 업그레이드해도 캐시 키가 그대로라 오래된(stale) 결과가 계속 반환된다.
- deterministic 분석 로직(파서, 메트릭 계산)의 버전이 바뀌어도 마찬가지로 캐시가 무효화되지 않아, 코드 업데이트가 사용자에게 반영되지 않는 유령 버그를 만든다.
- MCP-012(provenance 미기록)와 결합되면 "이 캐시된 답은 어느 버전 기준인지" 사후 확인조차 불가능하다.

**발생 조건**:
- 캐시를 초기 구현할 때 "입력이 같으면 출력도 같다"는 가정을 deterministic 분석에서만 검증하고, AI 호출도 같은 캐시 레이어에 얹을 때.
- 프롬프트/모델 버전 관리가 애초에 코드 어디에도 명시적으로 존재하지 않을 때(MCP-012).

**권장**:
```rust
#[derive(Hash, Eq, PartialEq)]
struct AiCacheKey {
    stream_content_hash: u64,
    frame_idx: u64,
    model_id: String,
    prompt_template_version: u32,
    analysis_logic_version: u32, // deterministic 파서/메트릭 버전도 포함
}

impl AiResultCache {
    fn get_or_compute(&mut self, key: AiCacheKey) -> AiAnalysisResult {
        self.inner.entry(key).or_insert_with(compute_ai_analysis).clone()
    }
}
```
- 캐시 키에 모델 ID, 프롬프트 버전, 분석 로직 버전을 모두 포함해 어느 하나라도 바뀌면 자동으로 캐시가 무효화되게 한다.
- 배포 시 버전 상수를 올리는 것을 릴리스 체크리스트에 명시한다.

**탐지 방법**:
- Structural: AI 결과를 저장하는 캐시의 키 타입에 `model_id`/`*_version` 필드가 없는지 grep.
- Runtime: 프롬프트 템플릿을 의도적으로 변경한 뒤 동일 요청의 응답이 바뀌는지 회귀 테스트로 검증.

**예외**:
- 캐시 TTL이 매우 짧아(예: 세션 내에서만 유효, 수 분 이하) 버전 드리프트가 사실상 발생할 수 없는 구조라면 상대적으로 위험이 낮다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-017: AI failure 시 core workflow가 중단
**분류**: MCP · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
async fn tool_get_frame_report(ctx: &AnalyzerContext, stream_id: StreamId, frame_idx: u64) -> McpToolResult {
    let deterministic = ctx.parse_frame(stream_id, frame_idx)?;
    // AI 호출 실패(타임아웃, rate limit, 네트워크 단절)가 전체 도구 호출을 실패시킴
    let ai_summary = ctx.llm_client.summarize(&deterministic).await?; // `?`로 전파
    McpToolResult::json(FrameReport { deterministic, ai_summary })
}
```

**문제**:
- LLM API는 로컬 파서보다 훨씬 자주 실패한다(네트워크, rate limit, 제공자 장애, 타임아웃)는 것이 현실인데, 그 실패가 이미 계산되어 있는 deterministic 결과까지 함께 폐기시킨다.
- 오프라인 환경(에어갭된 분석 워크스테이션 등)에서는 AI 호출이 원천적으로 불가능한데, 이 경우 핵심 분석 기능 전체가 동작하지 않게 된다.
- 코덱 분석기의 본질적 가치는 deterministic 파싱/메트릭 계산이고 AI는 부가 기능인데, 부가 기능의 가용성이 핵심 기능의 가용성을 좌우하는 것은 의존성 방향이 뒤바뀐 설계다.

**발생 조건**:
- `?` 연산자로 AI 호출 에러를 무심코 상위로 전파할 때(deterministic 에러와 AI 에러를 같은 `Result` 타입으로 다룰 때).
- 프로토타입 단계에서는 AI가 항상 성공한다고 가정하고 에러 경로를 테스트하지 않은 경우.

**권장**:
```rust
async fn tool_get_frame_report(ctx: &AnalyzerContext, stream_id: StreamId, frame_idx: u64) -> McpToolResult {
    let deterministic = ctx.parse_frame(stream_id, frame_idx)?; // 이것만 필수
    let ai_summary = match ctx.llm_client.summarize(&deterministic).await {
        Ok(summary) => Some(summary),
        Err(e) => {
            tracing::warn!(error = %e, "AI summary unavailable, returning deterministic-only report");
            None // 실패해도 도구 호출 자체는 성공으로 반환
        }
    };
    McpToolResult::json(FrameReport { deterministic, ai_summary })
}
```
- AI 호출은 항상 `Option`/graceful degradation으로 다루고, 실패해도 deterministic 결과는 반환되게 한다(MCP-013의 타입 분리와 자연스럽게 맞물린다).
- 오프라인/AI 비활성 모드를 정식 지원 모드로 취급해 통합 테스트에 포함한다.

**탐지 방법**:
- Structural: AI 클라이언트 호출에 `?`가 붙어 도구 핸들러 전체의 `Result`로 바로 전파되는지 grep.
- Runtime: AI 엔드포인트를 강제로 차단한 상태에서 통합 테스트 스위트를 돌려 core 도구들이 여전히 유의미한 응답을 내는지 확인.

**예외**:
- 도구 자체가 "AI 전용" 기능(예: 자연어 요약만이 목적인 도구)이라면 AI 실패 시 도구 실패를 반환하는 것이 맞다 — 이 항목은 deterministic 결과와 AI 결과가 섞인 복합 도구에 해당한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-018: hallucinated codec field를 UI에 표시
**분류**: MCP · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```tsx
async function fetchAiCodecInsight(streamId: string): Promise<CodecInsight> {
  const res = await mcpClient.callTool("explain_codec_profile", { streamId });
  return res as CodecInsight; // 응답을 무조건 신뢰하고 캐스팅
}

function CodecProfileBadge({ insight }: { insight: CodecInsight }) {
  // 모델이 존재하지 않는 프로파일 이름을 지어내도 그대로 뱃지로 렌더링
  return <Badge>{insight.profileName}</Badge>;
}
```

**문제**:
- 코덱 프로파일/레벨 이름은 스펙에 정의된 고정된 열거값 집합인데, 모델이 존재하지 않는 이름(예: 실제로는 없는 "HEVC Main 10.5 Profile")을 생성해도 검증 없이 UI 뱃지로 노출된다.
- 사용자가 이 값을 실제 파일 속성으로 신뢰하면, 인코딩 파이프라인 설정이나 호환성 판단에서 잘못된 결론을 내릴 수 있다.
- 코덱 스펙 필드는 본질적으로 파서가 deterministic하게 뽑아낼 수 있는 값이므로, 애초에 AI가 개입할 필요가 없는 영역에 AI 경로를 잘못 사용한 사례이기도 하다(MCP-013과 연결).

**발생 조건**:
- "AI가 더 친절하게 설명해줄 것"이라는 기대로, 원래 파서가 직접 계산해야 할 스펙 필드(프로파일, 레벨, 티어)까지 AI 도구를 거치게 설계한 경우.
- 프론트엔드가 MCP 도구 응답 타입을 서버와 공유된 스키마 없이 임의로 캐스팅(`as CodecInsight`)할 때.

**권장**:
```rust
// 코덱 프로파일/레벨은 항상 deterministic 파서에서 직접 계산하고, 알려진 열거값으로만 반환
#[derive(serde::Serialize)]
enum HevcProfile { Main, Main10, MainStillPicture, /* 스펙에 정의된 값만 */ }

async fn tool_get_codec_profile(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let profile = ctx.parsed_stream(stream_id)?.profile_tier_level.profile; // enum, AI 미개입
    McpToolResult::json(profile)
}
```
```tsx
// AI는 이 deterministic 값에 대한 "설명"만 부가 정보로 제공, profileName 자체는 생성 금지
function CodecProfileBadge({ profile, aiExplanation }: { profile: HevcProfile; aiExplanation?: string }) {
  return (
    <>
      <Badge>{profile}</Badge>
      {aiExplanation && <AiNote text={aiExplanation} />}
    </>
  );
}
```
- 스펙에 고정된 열거값이 있는 필드(프로파일, 레벨, 컬러 프라이머리 등)는 항상 파서가 계산한 enum으로만 표현하고, 문자열 자유 생성을 허용하지 않는다.
- 프론트엔드는 MCP 도구 응답을 캐스팅이 아니라 런타임 스키마 검증(zod 등)으로 파싱해, 알 수 없는 값이 오면 명시적으로 실패시킨다.

**탐지 방법**:
- Structural: 스펙 고정 열거값을 갖는 필드(profile/level/tier/chroma format)가 AI 도구 응답 경로를 거치는지 grep.
- Semantic: 알려진 프로파일 목록과 UI에 실제로 표시된 값들을 대조해 목록에 없는 값이 나타나는지 자동 검증.

**예외**:
- 뱃지 옆에 붙는 순수 설명 텍스트(예: "Main10은 10비트 색심도를 지원합니다")처럼 열거값 자체가 아니라 그에 대한 부연 설명이라면 AI가 생성해도 무방하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-019: reference file 없이 비교 판단
**분류**: MCP · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
async fn tool_compare_to_reference(ctx: &AnalyzerContext, stream_id: StreamId) -> McpToolResult {
    let stats = ctx.compute_stream_stats(stream_id)?;
    // "레퍼런스 대비"라고 말하지만 실제 레퍼런스 파일/메트릭은 로드하지 않음
    let prompt = format!(
        "이 스트림의 통계는 {stats:?}입니다. 일반적인 레퍼런스 인코딩과 비교했을 때 \
         품질이 어떤지 평가해주세요."
    );
    let ai_reply = ctx.llm_client.complete(&prompt).await?;
    McpToolResult::text(ai_reply) // "레퍼런스"가 모델의 사전 지식(추정)일 뿐
}
```

**문제**:
- "비교"라는 단어가 응답에 등장하지만 실제로는 비교 대상 파일이 로드되지도, 어떤 레퍼런스인지 지정되지도 않아 모델이 사전 학습 지식에서 "일반적인 값"을 추측해서 답한다.
- 이는 VMAF/PSNR/SSIM 같은 정량적 비교 메트릭이 실제로 존재하는 도메인에서, 근거 없는 정성적 추측이 정량적 비교를 대체하는 결과를 낳는다.
- 사용자가 "레퍼런스 대비 이 정도 품질"이라는 답을 받으면 실제 계측값으로 오인하기 쉽다(MCP-004와 동일한 신뢰 등급 혼동).

**발생 조건**:
- 비교 기능을 구현할 때 실제 레퍼런스 파일 업로드/지정 UX보다 "AI에게 물어보면 되지 않을까"로 지름길을 택한 경우.
- 도구 이름(`compare_to_reference`)이 실제 구현(레퍼런스 미사용)과 불일치할 때.

**권장**:
```rust
async fn tool_compare_to_reference(
    ctx: &AnalyzerContext,
    stream_id: StreamId,
    reference_stream_id: StreamId, // 필수 파라미터로 강제 — 없으면 도구 호출 자체가 불가능
) -> McpToolResult {
    let metric = ctx.compute_vmaf(stream_id, reference_stream_id)?; // deterministic 계측
    McpToolResult::json(ComparisonResult {
        reference_stream_id,
        vmaf_score: metric.score,
        per_frame_scores: metric.per_frame, // 실제 계측 데이터
    })
}
```
- "비교" 도구는 레퍼런스 스트림 ID를 필수 인자로 강제해, 레퍼런스가 없는 호출 자체가 스키마 레벨에서 불가능하게 만든다.
- 실제 레퍼런스가 없을 때는 "일반적인 품질 추정"이라고 아예 다른 이름의 도구로 명확히 분리해 혼동을 막는다.

**탐지 방법**:
- Structural: 이름에 `compare`/`reference`가 들어간 도구가 두 번째 스트림 ID를 파라미터로 받지 않는지 grep.
- Semantic: 도구 응답에 "레퍼런스", "비교"라는 표현이 등장하는데 실제 두 번째 데이터 소스에 대한 참조가 응답 구조에 없는 경우를 리뷰.

**예외**:
- 도구가 처음부터 "일반적인 업계 기준값과의 정성적 추정"임을 이름과 문서에 명시하고, 정량적 비교 도구와 구분되어 있다면 예외로 허용 가능하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### MCP-020: AI 결과를 golden truth로 테스트
**분류**: MCP · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[test]
fn test_frame_report_matches_expected() {
    let report = tool_get_frame_report(&ctx, stream_id, 42).unwrap();
    // 최초 실행 시 AI가 생성한 텍스트를 그대로 golden fixture로 저장해버림
    let golden = std::fs::read_to_string("fixtures/frame_42_ai_summary.txt").unwrap();
    assert_eq!(report.ai_summary, golden); // AI 응답의 비결정성을 테스트가 그대로 고정
}
```

**문제**:
- LLM 응답은 모델 업데이트, 샘플링 온도, 프롬프트 미세 변경에 따라 자연스럽게 달라질 수 있는데, 이를 golden fixture로 고정하면 테스트가 "정확성"이 아니라 "우연히 처음 나온 문자열과의 일치"만 검증하게 된다.
- 테스트가 깨질 때마다 "AI 응답이 바뀌었으니 fixture를 업데이트"하는 것이 관행이 되면, 실제 회귀(품질 저하)와 정상적인 모델 개선을 구분할 수 없어 테스트가 무의미해진다.
- deterministic 필드(QP, 프레임 타입 등)와 AI 필드가 같은 golden fixture에 섞여 있으면, AI 텍스트가 조금만 바뀌어도 실제로는 안전한 deterministic 로직 변경까지 테스트 실패로 보고된다.

**발생 조건**:
- 통합 테스트를 스냅샷 방식(`insta`, golden file)으로 작성하면서 AI 응답 필드를 제외하지 않은 경우.
- "테스트가 있으니 안전하다"는 착각으로 AI 응답의 정확성 자체를 이 스냅샷 테스트로 검증하려 할 때.

**권장**:
```rust
#[test]
fn test_frame_report_deterministic_fields() {
    let report = tool_get_frame_report(&ctx, stream_id, 42).unwrap();
    // deterministic 필드만 golden 비교
    assert_eq!(report.deterministic.qp, 32);
    assert_eq!(report.deterministic.frame_type, FrameType::I);
}

#[test]
fn test_frame_report_ai_field_shape_only() {
    let report = tool_get_frame_report(&ctx, stream_id, 42).unwrap();
    // AI 필드는 "존재하고 스키마를 만족하는지"만 검증, 내용 일치는 검증하지 않음
    if let Some(ai) = &report.ai_summary {
        assert!(!ai.text.is_empty());
        assert!(ai.confidence >= 0.0 && ai.confidence <= 1.0);
    }
}
```
- deterministic 필드와 AI 필드를 별도 테스트로 분리하고, AI 필드는 내용이 아니라 스키마 준수(형태, 길이 제한, confidence 범위)만 검증한다.
- AI 응답 품질을 검증하고 싶다면 golden-string 비교 대신 LLM-as-judge나 규칙 기반 휴리스틱(예: 언급된 frame_idx가 실제 범위 내인지) 같은 별도 평가 파이프라인을 구축한다.

**탐지 방법**:
- Structural: 테스트 코드에서 AI 응답 필드(`ai_summary`, `explanation` 등)가 `assert_eq!`로 고정 문자열/fixture와 직접 비교되는지 grep.
- Manual: CI에서 "AI fixture 업데이트"가 반복적으로 발생하는 PR 이력을 확인 — 빈번하다면 이 안티패턴의 징후.

**예외**:
- 모델 호출을 목(mock)으로 완전히 대체해 결정론적으로 고정된 응답을 반환하는 단위 테스트(도구 로직 자체를 검증하려는 목적)라면, 그 목 응답과의 golden 비교는 문제없다 — 실제 모델 출력이 아니기 때문이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
