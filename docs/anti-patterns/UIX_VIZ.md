# Anti-Pattern Catalog — UIX_VIZ: 전문 시각화의 의미 왜곡

이 문서는 더 큰 안티패턴 카탈로그의 일부입니다(전체 색인은 별도 작성 중인 `docs/anti-patterns/INDEX.md` 참고). Phase 1(Rust/media engineering)과 Phase 2(VQ-Probe 도메인 계산 정확성: ALIGN/SPATIAL/COLOR/METRIC/PIPE/HEAT/STAT)에 이어 Phase 3(UI/UX + Tauri) 웨이브의 한 갈래이며, 이 파일은 전문 시각화(QP heatmap, motion vector overlay, VQ-Probe 품질 지표 차트)를 다룬다. VQ-Probe 지표 차트 절(UIX-VIZ-016~025)은 특히 Phase 2의 `HEAT.md`/`STAT.md`와 짝을 이룬다 — 그쪽이 "그 숫자가 맞게 계산됐는가"(정규화, percentile, outlier 처리 등 계산 정확성)를 다뤘다면, 이 파일은 "맞게 계산된 숫자가 화면에서 맞게 읽히는가"(시각적 인코딩, 축, 스케일, 동기화 등 해석 정확성)를 다룬다. 같은 결함이 계산 단계에도, 표현 단계에도 각각 독립적으로 존재할 수 있다.

## 이 카테고리의 판단 기준

**전문 시각화는 렌더링됐는지가 아니라 해석을 오도하는지로 감사해야 한다.** QP heatmap, motion vector field, 품질 지표 차트는 모두 "정답이 있는" 정량적 도구다 — 미학적으로 만족스럽거나 인터랙션이 매끄러운 것과, 사용자가 그 그림을 보고 내리는 결론이 실제 데이터와 일치하는 것은 완전히 다른 문제다. 이 문서의 각 항목은 "예쁘지 않다"가 아니라 "이 시각화를 근거로 사용자가 내릴 결론이 틀릴 수 있다"는 오판(misinterpretation) 위험을 기준으로 심각도를 매긴다. Bitvue/VQ-Probe 사용자는 이 도구의 출력을 근거로 인코더 설정을 바꾸고, 회귀를 판정하고, 릴리스를 승인한다 — 시각화가 틀린 결론을 유도하면 그 피해는 화면 밖, 실제 제품 결정으로 전파된다.

---

## QP 히트맵 (Quantization Parameter Heatmap)

압축 강도의 공간적 분포를 색으로 인코딩하는 오버레이. 절대 QP 값 비교, bit depth/코덱 간 비교, 블록 경계 정합성이 핵심 신뢰 기반이다.

### UIX-VIZ-001: 프레임마다 자동 min/max를 적용

**분류**: QP Heatmap · **심각도**: Critical · **탐지**: Visual

**사용자 목표**:
프레임 간 QP 분포를 비교해 인코더가 특정 구간에서 얼마나 강하게 압축했는지 판단한다.

**증상**:
- 프레임을 전환할 때마다 heatmap의 색상 범위(스케일)가 바뀐다.
- QP가 거의 균일한 프레임인데도 색이 빨강-파랑 전체 스펙트럼으로 표시된다.
- 반대로 QP가 극단적으로 튀는 프레임도 색상만 보면 "정상적인 그라디언트"처럼 보인다.

**원인**:
각 프레임의 min/max QP 값으로 컬러 스케일을 auto-normalize하는 구현. 항상 시각적으로 "꽉 찬" 풀 스펙트럼 heatmap을 만들어내지만, 그 대가로 절대적 QP 값의 프레임 간 비교가 불가능해진다.

**구현 냄새**:
- `colorScale = scaleSequential().domain([frameMin, frameMax])`가 프레임 렌더 루프 안에서 매번 재계산됨.
- legend의 눈금 숫자가 프레임마다 다르게 표시됨.
- "고정 범위" 옵션 자체가 UI에 존재하지 않음.

**영향**:
사용자가 "이 프레임이 다른 프레임보다 더 강하게 압축됐다"고 색상만으로 판단해 잘못된 결론에 도달한다. 씬 전환 전후 QP 비교, 인코더 rate-control 튜닝 검증처럼 프레임 간 절대 비교가 핵심인 작업에서 결론이 통째로 뒤집힐 수 있다 — heatmap이 "예쁘게 보이는 것"과 "정확하게 보이는 것"을 맞바꾼 결과다.

**권장**:
- 시퀀스 전체(또는 사용자가 선택한 구간) 기준의 고정 min/max를 기본값으로 사용.
- 코덱별 유효 QP 범위(H.264/HEVC: 0-51, AV1: 0-255 등) 기준 legend 고정 옵션 제공.
- per-frame auto-scale은 명시적 토글로만 제공하고, 활성화 시 legend에 "relative to this frame" 표시.

**탐지**:
- 균일 QP 합성 프레임(constant-QP)과 극단 QP 프레임을 연달아 재생하며 legend 값이 유지되는지 확인.
- 코드 리뷰: 컬러 스케일 domain 계산이 프레임 루프 내부에 있는지 검색.

**관련**: `HEAT.md` HEAT-009 참고 — 동일한 per-frame auto-normalize 패턴이나 QP heatmap(Bitvue)과 VQ-Probe 품질-score heatmap은 별도 서브시스템.

**Bitvue 판정**: Confirmed — per-frame auto min/max: `qpToColor(qpVal, qp_min, qp_max)` in QPMapRenderer.tsx:39 uses `qp_min`/`qp_max` taken directly from `frame.qp_grid` (per-frame values set in src-tauri/src/commands/analysis/extractors.rs:59-60 from each codec's `extract_qp_grid`); no fixed/sequence-wide range option exists in the codebase.

---

### UIX-VIZ-002: 서로 다른 bit depth의 QP를 같은 범례로 표시

**분류**: QP Heatmap · **심각도**: High · **탐지**: Domain review

**사용자 목표**:
8bit/10bit 콘텐츠의 QP를 나란히 비교해 실제 압축 강도 차이를 판단한다.

**증상**:
- 10bit 스트림과 8bit 스트림의 heatmap이 동일한 color scale/legend로 표시된다.
- 같은 색이라도 두 스트림에서 실제 의미하는 양자화 강도가 다르다.

**원인**:
HEVC 등 일부 코덱은 bit depth에 따라 QP에 offset이 존재한다(예: QP' = QP + 6×(bitDepth-8)). raw QP 값을 bit-depth-dependent 정규화 없이 그대로 heatmap 컬러 함수에 통과시키면 서로 다른 bit depth 간 색상이 같은 의미를 갖지 않게 된다.

**구현 냄새**:
- 코덱 파서가 반환한 raw QP를 bit depth 인자 없이 단일 컬러 매핑 함수에 그대로 전달.
- heatmap 데이터 구조에 bit depth 필드가 없거나 있어도 정규화에 사용되지 않음.

**영향**:
10bit 스트림이 실제로는 더 미세한 양자화 단계를 쓰는데도 heatmap 색상이 8bit보다 "더 나쁘게" 보이는 등 두 스트림의 비교 결론이 뒤바뀔 수 있다. VQ-Probe 듀얼 스트림 비교 시나리오(서로 다른 인코딩 프로필 비교)에서 특히 치명적이다.

**권장**:
- heatmap 생성 전 QP를 bit-depth-정규화된 값(QP - 6×(bitDepth-8))으로 변환.
- legend에 현재 표시 중인 bit depth를 명시.
- 서로 다른 bit depth 스트림을 나란히 비교하는 뷰에서는 경고 배지 표시.

**탐지**:
- 동일 콘텐츠의 8bit/10bit 인코딩 페어로 heatmap을 나란히 렌더링해 색상 분포 일치 여부 확인.
- 도메인 리뷰: 코덱별 QP-bit depth 관계식이 정규화 로직에 반영됐는지 검토.

**관련**: `PIXEL.md` PIXEL-006 참고 — bit-depth 의존 정규화 상수를 빠뜨리는 동일 패턴이나, 대상이 QP 값의 legend 표시(여기)와 raw 픽셀 샘플 정규화(PIXEL-006)로 다름.

**Bitvue 판정**: Confirmed — bit depth is parsed (e.g. `bit_depth_luma_minus8` in crates/bitvue-hevc/src/sps.rs:133-135) but never consulted anywhere QP grids are built or colored; `extract_qp_grid` (crates/bitvue-hevc/src/overlay_extraction.rs:157-206) and `qpToColor()` (frontend/components/panels/OverlayRenderer/utils/helpers.ts:24) take no bit-depth parameter, so 8-bit/10-bit QP maps to the same color scale unadjusted.

---

### UIX-VIZ-003: 절대값과 delta 값을 같은 색상 체계로 표현

**분류**: QP Heatmap · **심각도**: High · **탐지**: Visual

**사용자 목표**:
절대 QP 분포와 QP 변화량(블록 간 delta QP)을 각각 정확히 읽는다.

**증상**:
- 절대 QP heatmap과 delta QP heatmap이 동일한 sequential 팔레트(예: 파랑→빨강, 낮음→높음)를 사용한다.
- delta 뷰에서 "변화 없음(0)"인 영역이 절대값 뷰의 "낮은 QP"와 동일한 색(예: 진한 파랑)으로 표시된다.

**원인**:
컬러 스케일 컴포넌트를 재사용하면서 sequential(단일 방향, 절대값용)과 diverging(0 중심 양방향, delta용) 팔레트를 구분하지 않고 단일 팔레트를 하드코딩했다.

**구현 냄새**:
- 하나의 `qpColorScale()` 유틸 함수가 절대값 모드와 delta 모드 양쪽에서 그대로 호출됨.
- delta 데이터의 domain을 `[min, max]`로만 계산하고 0을 중심으로 대칭시키지 않음.

**영향**:
사용자가 "이 블록은 QP가 낮다(품질이 좋다)"와 "이 블록은 QP 변화가 없다(전후 동일)"를 색만으로 혼동한다. rate control 안정성 분석, 인코더 버전 간 QP 변화 비교처럼 delta가 핵심인 작업에서 완전히 다른 결론에 도달할 수 있다.

**권장**:
- 절대값은 sequential 팔레트, delta는 0을 중심으로 한 diverging 팔레트(예: 파랑-흰색-빨강)를 코드 레벨에서 강제 분리.
- 모드 전환 시 legend와 팔레트가 함께 바뀌도록 하고, delta 모드의 legend에는 0 지점을 명시.

**탐지**:
- delta 모드에서 변화가 없는 블록의 색이 중립색(흰색/회색)인지 확인.
- 절대값 모드와 delta 모드의 팔레트 정의가 코드상 실제로 분리돼 있는지 검토.

**관련**: `HEAT.md` HEAT-016 참고 — sequential/diverging 팔레트 구분 필요성은 동일하나 대상이 QP delta(Bitvue)와 VQ-Probe 스트림 간 delta score로 다름.

**Bitvue 판정**: Confirmed (adjacent form) — absolute QP uses the sequential blue→cyan→yellow→red `QP_COLOR_STOPS` ramp (frontend/components/panels/OverlayRenderer/utils/colors.ts:14-19); the only delta view, `diffColor()` in frontend/components/CompareWorkspace/DiffOverlay.tsx:22-34, computes `t = Math.abs(delta)/maxDelta` — a magnitude-only scale with no zero-centered diverging palette, so a zero-delta block gets the same low/blue treatment as a genuinely low absolute value.

---

### UIX-VIZ-004: 범례를 숨김

**분류**: QP Heatmap · **심각도**: Critical · **탐지**: Interaction

**사용자 목표**:
heatmap 색이 실제로 어떤 QP 값을 의미하는지 확인한다.

**증상**:
- legend(color bar + 숫자 눈금)가 아예 없거나, 창 축소/반응형 breakpoint 아래에서 사라진다.
- 특정 블록의 정확한 QP 값을 알아낼 hover tooltip도 없다.

**원인**:
legend를 "보조 UI"로 취급해 초기 구현에서 생략했거나, 반응형 레이아웃 작업 중 우선순위가 낮아 좁은 화면에서 제거됐다.

**구현 냄새**:
- legend 컴포넌트가 `{!isCompact && <Legend/>}`처럼 조건부로 숨겨짐.
- legend 컴포넌트 자체가 별도로 존재하지 않고 색상 팔레트만 하드코딩됨.

**영향**:
사용자가 색상의 "상대적 인상"만으로 판단하게 된다. 빨강이 QP 40인지 51인지 알 수 없는 상태에서 "이 구간은 나쁘다"는 정성적 추측만 가능해지고, 정량적 분석 도구로서의 신뢰성이 사라진다.

**권장**:
- legend를 항상 필수 UI 요소로 취급하고 숨김을 허용하지 않음(접기는 가능해도 완전 제거는 불가).
- 최소한 hover 시 블록의 정확한 QP 값을 표시하는 tooltip을 반응형 축소 상태에서도 유지.
- legend를 별도 분리 가능한 패널로 만들어 좁은 화면에서도 접근 가능하게.

**탐지**:
- 다양한 창 크기/해상도에서 legend 가시성 시각 회귀 테스트.
- legend 없이 특정 블록의 QP 값을 알아낼 방법이 있는지 UX 워크스루로 확인.

**관련**: `HEAT.md` HEAT-010 참고 — legend가 실제 값을 반영하지 못하는 문제이나, HEAT-010은 legend 컴포넌트 부재가 아니라 backend가 사용된 range를 응답에 포함하지 않아 legend 자체가 정확할 수 없는 경우.

**Bitvue 판정**: Confirmed — QPMapRenderer.tsx (renderers/QPMapRenderer.tsx:45-51) draws only a text box ("QP: min - max") with no gradient/color-bar legend at all (no `drawLegend` call, unlike categorical overlays such as AvcRefIdxRenderer.tsx:70-109); no hover tooltip surfaces a block's exact QP either (no such handler in YuvViewerPanel/VideoCanvas.tsx). Worse than the item's "hidden on narrow screens" framing — it is never rendered.

---

### UIX-VIZ-005: invalid block을 0으로 표시

**분류**: QP Heatmap · **심각도**: Critical · **탐지**: Code

**사용자 목표**:
파싱 실패했거나 QP 정보가 없는 블록(디코딩 에러, PCM 블록 등)과 실제로 QP가 낮은 블록을 구분한다.

**증상**:
- 파서가 QP를 얻지 못한 블록이 heatmap에서 QP=0(가장 진한 파랑, "최상 품질")으로 렌더링된다.
- PCM/lossless 블록이 많은 스트림에서 heatmap이 실제보다 훨씬 "우수해" 보인다.

**원인**:
heatmap 데이터 구조가 `Option<u8>` 대신 plain `u8`을 사용해 sentinel 값 0을 "no data"로 재사용하거나, 파서 에러를 상위 레이어에서 삼키고 기본값 0을 채운다.

**구현 냄새**:
- `qp_map[idx] = block.qp.unwrap_or(0)` 패턴.
- QP 필드 타입이 부호 없는 정수라 "정보 없음"을 표현할 별도 sentinel이 없음.

**영향**:
실제로는 파싱 실패로 정보가 없는 대량의 블록을 "최고 품질 구간"으로 잘못 보고하게 된다. QA가 이 heatmap을 근거로 특정 구간을 "문제없음"으로 승인하면, 실제로는 데이터 자체가 신뢰할 수 없는 구간을 통과시키는 결과가 된다.

**권장**:
- QP 데이터 구조에 명시적 "no data" 상태(별도 알파 채널, 해칭 패턴, 회색/투명 처리)를 두고 0과 분리.
- invalid 블록 비율을 별도 통계로 상단에 노출(예: "12% blocks unavailable").

**탐지**:
- 의도적으로 파싱 실패를 유발하는 손상 스트림으로 테스트.
- invalid 블록 카운트와 heatmap상 QP=0 블록 카운트가 겹치는지 코드 감사.

**관련**: `HEAT.md` HEAT-012 참고 — invalid/NaN 데이터를 유효 극단값(0)으로 치환해 오인시키는 동일 패턴. QP=0은 "최상"으로, score=0은 "최하"로 위장돼 방향은 반대.

**Bitvue 판정**: N/A — QPMapRenderer.tsx:37 explicitly does `if (qpVal === -1) continue;`, skipping missing blocks rather than coloring them as QP=0; DiffOverlay.tsx:154 does the equivalent (`qpValA < 0 || qpValB < 0` → skip). Sentinel handling is correct.

---

### UIX-VIZ-006: block 경계와 heatmap sampling 경계가 다름

**분류**: QP Heatmap · **심각도**: High · **탐지**: Visual

**사용자 목표**:
특정 CU/블록의 QP를 정확히 그 블록 영역에 겹쳐서 확인한다.

**증상**:
- heatmap 타일 경계가 실제 CTU/CU/macroblock 경계와 미묘하게 어긋나 보인다.
- non-square partition이나 다운샘플링된 heatmap에서 한 블록이 heatmap상 2개 이상의 셀에 걸쳐 서로 다른 색으로 나뉘어 보인다.

**원인**:
heatmap을 코덱 native partition tree가 아니라 고정 grid(예: 16×16 고정 타일)로 리샘플링해 렌더링하거나, 렌더링 좌표계와 블록 좌표계 간 반올림 오차가 있다.

**구현 냄새**:
- heatmap 생성 로직에 `tileSize = 16` 같은 하드코딩이 있고 실제 가변 CU 크기(8~64)를 무시.
- 캔버스 draw 시 scale factor를 정수로 반올림해 누적 오차 발생.

**영향**:
사용자가 "이 블록 QP가 높다"고 짚은 위치가 실제로는 인접 블록의 값일 수 있다. 블록 단위 diagnostic(특정 CU 크기와 QP의 상관관계 분석)이 근본적으로 틀어진다.

**권장**:
- heatmap을 코덱이 실제 사용한 partition 경계(가변 크기)에 직접 렌더링.
- 리샘플링이 불가피하면 block-aware(원본 블록의 다수/지배적 QP) 방식 사용.
- 확대 시 실제 CU 경계선을 오버레이로 함께 그려 정합성을 육안 검증 가능하게.

**탐지**:
- 알려진 partition 구조(예: 좌상단 64×64, 우하단 8×8 강제 분할)를 가진 합성 스트림으로 픽셀 좌표 대조.

**관련**: `PIXEL.md` PIXEL-015 참고 — overlay 좌표가 실제 프레임 지오메트리와 어긋나는 문제이나, 이쪽은 heatmap grid/CU 정합, PIXEL-015는 crop/coded-size 오프셋이 원인.

**Bitvue 판정**: Confirmed — per-codec `extract_qp_grid` hardcodes a fixed native unit (HEVC: `let ctu_size = 64u32;`, crates/bitvue-hevc/src/overlay_extraction.rs:163) and collapses each CTU to a single QP via `ctu.coding_units.first().map(|cu| cu.qp)` (overlay_extraction.rs:185) rather than the actual variable-size CU partition, so sub-CTU QP variation is lost before the frontend even draws the fixed-size grid cells (QPMapRenderer.tsx:31-43).

---

### UIX-VIZ-007: opacity가 영상 내용을 완전히 가림

**분류**: QP Heatmap · **심각도**: Medium · **탐지**: Visual

**사용자 목표**:
heatmap과 원본 영상을 동시에 보며 QP가 높은 영역이 어떤 시각적 콘텐츠와 연관되는지 판단한다.

**증상**:
- heatmap opacity가 고정 고값(0.8~1.0)으로 설정돼 아래 원본 프레임이 거의 보이지 않는다.
- 색이 진한(QP가 극단적인) 영역일수록 원본 콘텐츠가 완전히 사라진다.

**원인**:
opacity를 사용자 조절 가능한 값으로 노출하지 않고 "잘 보이는" 값으로 하드코딩했거나, alpha blending 대신 alpha replace를 사용했다.

**구현 냄새**:
- `ctx.globalAlpha = 0.9` 같은 하드코딩.
- opacity slider UI 컴포넌트 부재.

**영향**:
"QP가 높은 영역이 텍스처가 복잡한 영역과 일치하는가?" 같은 핵심 분석 질문에 답할 수 없다. heatmap이 진단 도구가 아니라 원본을 가리는 마스크가 돼버리며, 정작 가장 궁금한 고-QP 영역이 가장 안 보이는 역설이 발생한다.

**권장**:
- opacity를 실시간 슬라이더로 노출(기본값 0.4~0.5 권장).
- 원본/heatmap 토글 또는 side-by-side 뷰 제공.
- 원본의 luma를 유지한 채 색상만 오버레이하는 blend mode(multiply, HSL lightness 보존 등) 검토.

**탐지**:
- 텍스처가 풍부한 참조 프레임에 heatmap을 최대 강도로 씌운 뒤 원본 에지가 육안으로 식별 가능한지 시각 검사.

**Bitvue 판정**: Confirmed — opacity is hardcoded, not user-adjustable: `qpToColor` fixes `alpha = 0.63` (utils/helpers.ts:39) and `renderInfoOverlay` fixes `INFO_OVERLAY_ALPHA = 0.72` (OverlayRenderer/index.tsx:200-211); repo-wide search found no opacity slider/control tied to any overlay renderer.

---

### UIX-VIZ-008: 확대 시 nearest/interpolation 정책이 불명확

**분류**: QP Heatmap · **심각도**: Medium · **탐지**: Visual

**사용자 목표**:
특정 블록을 확대해 정확한 QP 경계를 확인한다.

**증상**:
- 확대 시 heatmap이 부드럽게 블러(bilinear/bicubic 보간)돼 인접 블록 간 QP가 마치 연속적인 그라디언트처럼 보인다.
- 축소 시 nearest-neighbor로 인해 실제로 존재하지 않는 moiré 패턴이 생긴다.

**원인**:
브라우저/그래픽 API의 기본 이미지 스무딩(bilinear filtering)이 켜져 있고, 이를 QP 데이터의 이산적(discrete) 특성에 맞게 명시적으로 끄지 않았다.

**구현 냄새**:
- `imageSmoothingEnabled` 기본값(true) 그대로 방치.
- WebGL 텍스처 sampler를 `LINEAR`로 생성.

**영향**:
사용자가 인접 블록 사이에 "점진적인 QP 변화"가 있다고 착각한다. 실제로는 QP 20에서 45로 급격히 뛰는 경계인데 완만한 그라디언트로 보여, 인코더가 실제로 갖지 않는 "부드러운 적응 양자화"를 갖는 것처럼 오판할 수 있다.

**권장**:
- heatmap 렌더링은 기본적으로 nearest-neighbor(`imageSmoothingEnabled = false`, WebGL `NEAREST` 필터)를 강제해 블록 경계를 명확히 유지.
- smooth preview는 명시적 opt-in 토글로만 제공하고, 격자선 오버레이를 함께 표시.

**탐지**:
- 확대 상태에서 블록 경계의 색상 전이가 하드 엣지(1픽셀)인지 다수 픽셀에 걸친 그라디언트인지 픽셀 값 검사.

**관련**: `HEAT.md` HEAT-018 참고 — 이산적 block-grid 값을 보간으로 매끄럽게 렌더링해 정밀도를 과장하는 동일 패턴. QP heatmap(Bitvue) vs VQ-Probe 품질-score heatmap.

**Bitvue 판정**: N/A — the overlay canvas has `image-rendering: pixelated` (frontend/components/panels/YuvViewerPanel/YuvViewerPanel.css:296) on the exact `.yuv-canvas` element overlays draw to (VideoCanvas.tsx:238), and zoom is a single CSS `transform: scale(zoom)` on that element (VideoCanvas.tsx:131) — forcing nearest-neighbor scaling. No `imageSmoothingEnabled` usage exists anywhere in frontend/ (grep found none outside node_modules type defs), consistent with blocks being drawn as vector `fillRect`s rather than a scaled bitmap.

---

## 모션 벡터 (Motion Vector Overlay)

블록별 예측 방향/크기를 화살표로 표현하는 오버레이. 크기의 절대성, 참조 구조, global/local 모션 구분, compound prediction 표현이 핵심 신뢰 기반이다.

### UIX-VIZ-009: 모든 MV를 동일하게 그려 화면이 포화

**분류**: Motion Vector · **심각도**: High · **탐지**: Visual

**사용자 목표**:
프레임의 전반적 모션 패턴과 특이 지점(outlier motion)을 파악한다.

**증상**:
- 4K 프레임에서 8×8 블록 단위 수만 개의 화살표가 모두 동일한 두께/색/불투명도로 그려져 화면이 화살표로 뒤덮인다.
- 개별 벡터를 구분할 수 없고, 배경과 반대 방향으로 움직이는 유일한 객체 같은 이상 신호가 노이즈에 묻힌다.

**원인**:
렌더링 로직이 벡터 밀도/줌 레벨에 따른 LOD(level-of-detail)나 sub-sampling 없이 모든 블록을 1:1로 순회하며 그린다.

**구현 냄새**:
- `for block in all_blocks { drawArrow(block.mv) }` 형태로 뷰포트 줌과 무관하게 항상 전체 블록을 순회.

**영향**:
사용자가 "이 프레임은 모션이 크다/작다"는 막연한 인상만 얻을 뿐, "어디에 이상 모션이 있는가"라는 실제 분석 목적에 답할 수 없다 — 도구가 신호를 은폐하는 역설이 발생한다.

**권장**:
- 줌 레벨에 따른 sub-sampling(예: 낮은 줌에서는 N블록당 1개만 표시).
- 벡터 크기/통계적 이상치 기준 우선순위 렌더링.
- 밀도에 따른 자동 opacity 조절.

**탐지**:
- 고밀도 모션 시퀀스(스포츠, 파티클)에서 화살표 밀도와 개별 벡터 식별 가능 여부를 시각 검사.

**Bitvue 판정**: N/A — MVFieldRenderer.tsx:62-67 implements explicit density control: stride subsampling capped at `maxVectors = 8000`, plus a WebGL fast path (`WEBGL_MV_THRESHOLD = 300`, webgl/mv-webgl.ts:16) for denser grids. Not zoom-aware LOD, but the described "draw every block unconditionally" saturation bug is not present.

---

### UIX-VIZ-010: 벡터 크기를 화면 pixel과 혼동

**분류**: Motion Vector · **심각도**: Critical · **탐지**: Code

**사용자 목표**:
MV의 실제 크기(픽셀 단위 이동량, quarter/eighth-pel 정밀도 포함)를 화면에서 정확히 읽는다.

**증상**:
- 뷰포트를 확대/축소해도 화살표 길이가 원본 프레임 좌표계의 MV 크기와 무관하게 고정되거나 과장/축소된다.
- quarter-pel 단위 MV가 정수 픽셀로 반올림돼 렌더링된다.

**원인**:
벡터 렌더링 좌표 변환에서 원본 프레임 좌표계 → 화면 좌표계 스케일 팩터를 누락하거나, MV의 fractional precision을 정수로 truncate한다.

**구현 냄새**:
- `drawLine(x, y, x + mv.dx, y + mv.dy)`처럼 원본 좌표계 fractional 값을 zoom 보정 없이 화면 좌표에 그대로 더함.

**영향**:
사용자가 화살표 길이만 보고 "5픽셀 움직였다"고 판단했는데 실제로는 200% 줌에서 2.5픽셀이 5픽셀로 보이는 것일 수 있다 — 모션 크기에 대한 정량적 판단이 전부 틀어진다. quarter-pel 정밀도가 소실되면 서브픽셀 모션 보정 품질 분석 자체가 불가능해진다.

**권장**:
- MV는 항상 원본 프레임 좌표계에서 fractional precision까지 유지해 계산하고, 최종 렌더링 단계에서만 현재 zoom factor를 곱해 화면 좌표로 변환.
- 화살표 옆에 실제 크기(예: "+3.25px")를 텍스트로 병기하는 옵션 제공.

**탐지**:
- 알려진 크기의 합성 MV(예: 정확히 4.0px)를 여러 줌 레벨에서 렌더링 후 화면 픽셀 자로 측정해 zoom factor에 비례하는지 검증.

**Bitvue 판정**: N/A — MV components keep fractional pixel precision (`mv.dx_qpel / 4`, MVFieldRenderer.tsx:87-88, no rounding) and are drawn directly into the same canvas whose zoom is one uniform CSS `transform: scale(zoom)` (VideoCanvas.tsx:131) — so arrow length in screen space scales proportionally and consistently with zoom by construction; no separate/inconsistent coordinate-transform bug found.

---

### UIX-VIZ-011: reference list를 색으로만 구분

**분류**: Motion Vector · **심각도**: Medium · **탐지**: Visual

**사용자 목표**:
어떤 벡터가 L0 참조를, 어떤 벡터가 L1 참조를 가리키는지 파악해 B-frame의 양방향 예측 구조를 이해한다.

**증상**:
- L0/L1 구분이 화살표 색상만으로 표현돼 색맹 사용자나 흑백 캡처/프린트에서 구분이 불가능하다.
- 참조 프레임이 3개 이상(다중 reference)일 때 팔레트가 부족해 인접 색상끼리 혼동된다.

**원인**:
접근성(색맹 대응) 고려 없이 categorical color로만 인코딩했고, reference index 개수 확장성도 고려하지 않았다.

**구현 냄새**:
- `const refColors = ['#0000ff', '#ff0000']` 식의 하드코딩된 2색 팔레트.
- 선 스타일(두께, 점선, 화살촉 모양) 변형이 전혀 없음.

**영향**:
"L1 참조가 유독 먼 프레임을 가리키는 이상 패턴"을 찾으려는 분석이, 색 구분이 모호하고 참조 인덱스별 필터도 없으면 근본적으로 불가능해진다. 색맹 사용자에게는 참조 구조 정보가 통째로 소실된다.

**권장**:
- 색상 외에 선 스타일(실선/점선), 화살촉 모양, 두께 등 이중 인코딩 적용.
- reference index별 필터/토글(L0만 보기 등) 제공.
- 색맹 안전 팔레트(예: Okabe-Ito) 사용.

**탐지**:
- 색맹 시뮬레이터(Deuteranopia/Protanopia)로 스크린샷 검사.
- 3개 이상 참조 프레임을 갖는 B-frame 테스트 스트림에서 구분 가능 여부 확인.

**Bitvue 판정**: Confirmed — AvcRefIdxRenderer.tsx:21-30 defines a hardcoded 8-entry `REF_COLORS` array as the sole encoding of L0 reference index (color swatches only in the legend too, lines 70-109), with no line-style/pattern redundancy and no colorblind-safe palette; only `ref_idx_l0` is read — `ref_idx_l1` (present in frontend/types/video.ts:524) is never used by this renderer.

---

### UIX-VIZ-012: global motion과 local motion을 구분하지 않음

**분류**: Motion Vector · **심각도**: High · **탐지**: Domain review

**사용자 목표**:
카메라 팬/줌 같은 전역 모션과 피사체의 국소적 움직임을 분리해 실제 관심 있는 모션만 파악한다.

**증상**:
- 카메라 패닝 장면에서 배경 대부분의 블록이 같은 방향/크기의 벡터를 갖는데, 이것이 실제로 움직이는 피사체 벡터와 시각적 구분 없이 동일하게 렌더링된다.
- 화면 전체가 한쪽으로 쏠린 화살표로 덮여 배경과 다르게 움직이는 피사체를 찾기 어렵다.

**원인**:
MV 시각화가 raw motion vector만 그릴 뿐, 전역 모션 모델(affine/translation global motion — 예: AV1의 global motion 파라미터)을 추정하거나 빼는 처리를 하지 않는다.

**구현 냄새**:
- MV 오버레이 컴포넌트가 블록별 벡터만 소비하고 프레임 단위 global motion 파라미터를 아예 사용하지 않음.

**영향**:
"이 장면에서 유독 이상하게 움직이는 객체"를 찾으려는 근본 목적을, 전역 모션이 큰 장면일수록 시각화 자체가 방해한다 — 카메라 워크가 많은 스포츠/영화 콘텐츠에서 이상치 탐지가 실질적으로 불가능해진다.

**권장**:
- global motion을 추정(또는 코덱 제공 파라미터 활용)해 local residual motion(global 성분을 뺀 벡터)을 별도 레이어로 제공.
- "global 제거" 토글을 기본 분석 뷰에 포함.
- global motion 크기/방향 자체도 프레임 상단에 요약 표시.

**탐지**:
- 순수 패닝 합성 시퀀스(카메라만 이동, 피사체 없음)에서 residual 벡터가 0에 가깝게 나오는지 검증.
- 배경+국소 이동 피사체 합성 테스트로 이상치 강조 여부 확인.

**Bitvue 판정**: Confirmed — AV1/AV3 global motion parameters are parsed backend-side (crates/bitvue-av1-codec/src/tile/mv_prediction.rs, crates/bitvue-av3-codec/src/frame_header.rs) but `grep -rl global_motion frontend/` returns no hits; MVFieldRenderer.tsx draws raw per-block `mv_l0` only, with no global-motion subtraction/toggle/summary anywhere in the UI.

---

### UIX-VIZ-013: zero vector도 모두 렌더링

**분류**: Motion Vector · **심각도**: Low · **탐지**: Performance

**사용자 목표**:
실제로 움직인 블록만 빠르게 훑어본다(정적 배경은 관심 밖).

**증상**:
- MV가 (0,0)인 정적 블록에도 마커가 그려져 화면 전체에 점이 빽빽하게 깔린다.
- 정적 장면에서 "점 노이즈"가 실제 신호(움직이는 블록)를 가리거나 렌더링 성능을 불필요하게 저하시킨다.

**원인**:
렌더링 루프가 "MV가 존재하는 모든 블록"과 "MV가 0이 아닌 블록"을 구분하지 않고 동일 취급한다.

**구현 냄새**:
- `blocks.forEach(b => drawMarker(b))`에 zero-vector 조기 continue가 없음.

**영향**:
정적 장면이 "빽빽한 활동"처럼 보이거나, 반대로 실제 움직임이 점 노이즈에 묻혀 안 보인다 — 모션 밀도에 대한 인상이 왜곡된다. 대규모 정적 프레임에서는 불필요한 오버드로우로 렌더링 성능 저하도 함께 발생한다.

**권장**:
- 기본값으로 zero(또는 임계값 이하) 벡터는 렌더링 생략.
- "정적 블록도 표시" 옵션은 명시적 opt-in으로 분리.
- 정적 블록 비율을 숫자 통계로 별도 표시(예: "78% static").

**탐지**:
- 완전 정지 화면 테스트 스트림에서 오버레이가 비어 있는지 확인.
- 렌더링 프로파일러로 zero-vector 렌더 호출 수 측정.

**Bitvue 판정**: Confirmed — MVFieldRenderer.tsx's render loop (lines 78-96) only skips the missing-data sentinel (`dx_qpel === 2147483647`), not zero vectors; `drawArrow()` (utils/drawing.ts:65-96) still strokes a line and fills an arrowhead even when dx=dy=0, so every static block gets a rendered marker.

---

### UIX-VIZ-014: zoom에 따라 벡터 scaling이 일관되지 않음

**분류**: Motion Vector · **심각도**: High · **탐지**: Visual

**사용자 목표**:
여러 줌 레벨을 오가며 같은 블록의 모션 크기를 비교한다.

**증상**:
- 줌 인/아웃 시 화살표 길이의 scale factor가 뷰 전환마다 재계산돼(예: "현재 뷰포트에서 가장 긴 벡터가 화면의 N픽셀이 되도록" auto-scale), 동일한 실제 MV 크기라도 줌 레벨/프레임에 따라 화면상 길이가 달라진다.

**원인**:
UIX-VIZ-001의 auto-normalize 함정이 벡터 길이 도메인에서 반복된다 — "항상 화면에 예쁘게 맞는" 화살표를 만들기 위해 프레임별/뷰별 최대값 기준으로 스케일링한다.

**구현 냄새**:
- `const scale = viewportSize / maxMvInFrame` 형태로 프레임마다 재계산되는 scale factor.

**영향**:
사용자가 두 프레임을 오가며 "이 프레임이 저 프레임보다 모션이 크다"고 화살표 길이로 판단했는데, auto-scale 때문에 두 프레임 모두 "화면에 꽉 차는" 길이로 보여 절대 비교가 완전히 무의미해진다.

**권장**:
- 고정된 world-space-to-screen scale(예: "1px of motion = 1 screen px at 100% zoom")을 시퀀스 전체에서 일관되게 사용하고 zoom factor만 곱함.
- auto-scale이 필요하면 명시적 모드로 분리하고 legend에 현재 scale factor를 항상 표기.

**탐지**:
- 동일 MV 크기를 가진 블록을 여러 프레임/줌 레벨에서 화면 측정해 zoom factor에 비례하는 일관된 비율을 유지하는지 회귀 테스트.

**Bitvue 판정**: N/A — no per-frame/per-viewport auto-scale factor (e.g. `viewportSize / maxMvInFrame`) exists for MV arrow length anywhere in the renderers; zoom is the single uniform CSS transform described under UIX-VIZ-010, so this specific inconsistent-rescaling bug is not present.

---

### UIX-VIZ-015: compound prediction의 두 벡터 중 하나만 표시

**분류**: Motion Vector · **심각도**: High · **탐지**: Code

**사용자 목표**:
양방향(bi-prediction)/compound 예측 블록에서 L0, L1 각각의 MV가 실제로 어디를 가리키는지 모두 확인한다.

**증상**:
- bi-predicted 블록에 화살표가 하나만 그려져 마치 단방향 예측처럼 보인다.
- 실제로는 서로 다른(때로는 정반대 방향의) 두 벡터가 평균/합성돼 화면상 하나의 대표 벡터로 뭉개진다.

**원인**:
렌더링 로직이 블록당 화살표 하나만 그리도록 설계돼 compound/bi-pred 블록에서 L0 벡터만 사용하거나 두 벡터의 평균을 단일 화살표로 표시한다.

**구현 냄새**:
- `const mv = block.mvL0 ?? averageOf(block.mvL0, block.mvL1)` 같은 단순화.
- 데이터 모델 자체가 블록당 MV 필드를 하나만 가짐.

**영향**:
compound prediction은 종종 두 참조가 서로 다른 방향/크기를 가질 때(오클루전, 조명 변화 영역) 인코더가 선택하는데, 이 정보가 평균으로 뭉개지면 "왜 인코더가 이 블록에서 compound를 선택했는지" 분석이 불가능해진다. 두 벡터가 상쇄돼 평균이 0에 가까워지면 "움직임이 없다"는 정반대 결론에 도달할 위험도 있다.

**권장**:
- compound 블록은 두 화살표를 모두 그리되 시각적으로 구분(UIX-VIZ-011의 reference list 스타일과 연동).
- 화살표가 겹쳐 혼잡하면 hover/click 시 상세 패널에 L0/L1 벡터를 개별 수치로 표시.
- 두 벡터 각각의 참조 프레임 거리도 함께 표기.

**탐지**:
- 서로 반대 방향의 L0/L1 벡터를 갖는 합성 compound 블록 테스트 케이스로 렌더링 결과가 "무벡터"로 오인되지 않는지 확인.

**Bitvue 판정**: Confirmed — the Canvas2D path used for typical/small grids (≤300 blocks, below `WEBGL_MV_THRESHOLD`) destructures only `mv_l0` and never reads `mv_l1` (MVFieldRenderer.tsx:48,81), silently dropping the second compound-prediction vector rather than drawing or averaging it. The WebGL path used only for dense grids (mv-webgl.ts:186,224-225) does draw both L0/L1, so the defect is specific to the common-case fallback renderer.

---

## VQ-Probe 지표 차트 (Quality Metric Time-Series Chart)

VMAF/PSNR/SSIM 등 품질 지표의 프레임별 시계열을 보여주는 차트. Phase 2 HEAT/STAT이 이 숫자들의 계산 정확성을 다뤘다면, 여기서는 올바르게 계산된 숫자가 화면에서 올바르게 읽히는지를 다룬다.

### UIX-VIZ-016: 평균 점수만 강조

**분류**: VQ-Probe Chart · **심각도**: High · **탐지**: Visual

**사용자 목표**:
인코딩 품질을 시간축 전체에 걸쳐 파악하고 특히 나쁜 구간을 찾는다.

**증상**:
- 차트 상단에 큰 폰트로 "Average VMAF: 94.2"만 표시된다.
- 실제 프레임별 곡선은 작은 서브 차트로 축소돼 있거나 기본적으로 접혀 있다.

**원인**:
요약 지표(single number)가 대시보드/보고서에서 "가장 이해하기 쉬운 UI"로 우선시되고, 분포/시계열은 부차적 요소로 취급된다.

**구현 냄새**:
- KPI 카드 컴포넌트가 페이지 최상단을 차지하고 시계열 차트는 "더 보기" 뒤에 숨겨짐.
- 스크린샷/PDF 리포트 export 시 평균만 포함됨.

**영향**:
평균이 높아도 특정 구간(버퍼링 직후, 씬 전환)에서 크게 열화된 프레임이 있으면 실사용 체감 품질과 리포트 수치가 괴리된다. QA가 "평균 통과"로 릴리스했지만 국소적으로 심각한 아티팩트가 있는 릴리스가 그대로 통과할 위험이 있다(Phase 2 STAT-001/002와 동일한 함정이 여기서는 차트 레이아웃 문제로 재현된 것).

**권장**:
- 평균과 함께 최소값/1퍼센타일/시계열 곡선을 동일한 우선순위로 노출.
- 요약 카드 클릭 시 항상 해당 구간의 시계열로 드릴다운.
- 리포트 export 시 분포 정보를 기본 포함.

**탐지**:
- 평균은 높지만 국소 dip이 있는 합성 데이터로 대시보드를 렌더링해 사용자가 dip을 발견하는 데 걸리는 시간을 user test로 측정.

**Bitvue 판정**: Confirmed, and more severe than described — QualityMetricsPanel.tsx and QualityComparisonPanel.tsx (near-duplicate components) render only three large average numbers (PSNR/SSIM/VMAF, lines ~196-220) with zero per-frame chart of any kind; a repo-wide search found no time-series chart component for quality metrics anywhere in frontend/components (the only chart infra, frontend/components/charts/LineChart.tsx, is used solely by RDCurvesPanel.tsx for rate-distortion curves).

---

### UIX-VIZ-017: higher-is-better와 lower-is-better 방향 혼동

**분류**: VQ-Probe Chart · **심각도**: Critical · **탐지**: Domain review

**사용자 목표**:
여러 지표(VMAF/PSNR처럼 높을수록 좋은 것, 일부 오차/거리 기반 지표처럼 낮을수록 좋은 것)를 한 화면에서 일관되게 "좋음/나쁨" 방향으로 읽는다.

**증상**:
- Y축 방향이 지표마다 통일되지 않는다 — 어떤 차트는 위로 갈수록 좋고, 어떤 차트는 위로 갈수록 나쁘다.
- 색상 인코딩(초록=좋음, 빨강=나쁨)이 축 방향과 일치하지 않거나 아예 없다.

**원인**:
차트 컴포넌트가 범용 line chart를 그대로 재사용하면서 metric semantics(방향성 메타데이터)를 반영하지 않는다.

**구현 냄새**:
- metric 정의에 `higherIsBetter: boolean` 같은 필드가 없거나, 있어도 차트 렌더러가 이를 무시.
- 모든 지표에 동일한 초록/빨강 threshold 배경을 방향과 무관하게 기계적으로 적용.

**영향**:
사용자가 "이 구간이 초록색이니 좋다"고 습관적으로 판단했는데, lower-is-better 지표에서 초록이 실제로는 나쁜 구간을 가리키는 경우 정반대 결론에 도달한다. 인코더 A/B 비교에서 실제로 열등한 설정을 우수하다고 잘못 채택할 위험이 있다.

**권장**:
- 모든 metric 정의에 방향성 메타데이터를 필수화하고, 차트가 이를 읽어 "좋음=위쪽"으로 자동 정렬(필요시 축 반전).
- 색상(초록/빨강)은 항상 실제 의미(좋음/나쁨)와 일치하도록 방향 무관하게 계산.
- 축 라벨에 방향 화살표나 "▲ better" 표기.

**탐지**:
- lower-is-better 지표를 higher-is-better 지표와 나란히 배치한 대시보드에서 색상-의미 일치 여부 코드/시각 검사.
- 도메인 리뷰: 각 지표의 방향성이 metric 정의에 명시됐는지 확인.

**Bitvue 판정**: N/A — no VMAF/PSNR/SSIM time-series chart exists in the codebase (see UIX-VIZ-016 finding), so an axis-direction inconsistency between metrics cannot currently manifest; the only quality-metric UI is the single-value display in QualityMetricsPanel.tsx/QualityComparisonPanel.tsx.

---

### UIX-VIZ-018: 서로 다른 metric을 동일 Y축에 표시

**분류**: VQ-Probe Chart · **심각도**: High · **탐지**: Visual

**사용자 목표**:
VMAF와 PSNR처럼 스케일이 다른 지표를 함께 보며 상관관계를 파악한다.

**증상**:
- VMAF(0-100)와 PSNR(보통 20-50dB대)가 같은 Y축 스케일에 겹쳐 그려져 PSNR 곡선이 극단적으로 눌리거나 잘린다.

**원인**:
멀티 시리즈 차트 컴포넌트를 단일 Y축 기본 설정으로 사용하고, dual-axis 지원을 구현하지 않았다.

**구현 냄새**:
- 차트 라이브러리 설정에 `yAxisID` 분리 없이 모든 시리즈가 동일 축을 공유.

**영향**:
"PSNR은 떨어지는데 VMAF는 유지되는 구간" 같은 지각 품질과 신호 품질의 괴리를 읽으려는 목적이 스케일 왜곡으로 완전히 불가능해진다. 곡선 모양만 보고 잘못된 상관관계를 추론할 위험이 있다.

**권장**:
- 서로 다른 단위/스케일의 지표는 반드시 별도 Y축(좌/우 dual axis) 또는 별도 서브플롯으로 분리.
- 축마다 색상/라벨을 시리즈와 매칭.
- 단일 축 오버레이가 필요하면 정규화(0-1 스케일링) 여부를 명시적으로 표시.

**탐지**:
- VMAF+PSNR을 동시에 그린 뒤 두 곡선의 시각적 변동폭이 실제 수치 변동폭과 비례하는지 축 스케일 검사.

**Bitvue 판정**: N/A — no multi-series metric chart exists to share (or fail to share) a Y-axis; QualityMetricsPanel.tsx/QualityComparisonPanel.tsx show PSNR/SSIM/VMAF as three separate static numbers, not an overlaid chart.

---

### UIX-VIZ-019: VMAF·PSNR 점수를 정규화 없이 같은 그래프로 비교

**분류**: VQ-Probe Chart · **심각도**: Medium · **탐지**: Domain review

**사용자 목표**:
서로 다른 metric 체계(지각 모델 기반 VMAF, 신호 기반 PSNR/SSIM)의 결과를 하나의 "품질 추세"로 종합 판단한다.

**증상**:
- VMAF, PSNR, SSIM을 같은 그래프에 겹쳐 그리며 마치 세 곡선이 직접 비교 가능한 것처럼 제시된다.
- 실제로는 각 metric의 민감도/비선형성이 달라 동일한 화질 저하에도 서로 다른 기울기로 반응한다(SSIM은 0-1 범위의 천장 효과, PSNR은 로그스케일 특성).

**원인**:
UIX-VIZ-018과 인접하지만, 여기서는 축을 분리했더라도 "겹쳐 그리는 것 자체가 비교 가능성을 암시한다"는 시각적 착시 문제다 — 정규화/스케일링과 무관하게 같은 차트 영역에 그리는 것 자체가 오해를 유발한다.

**구현 냄새**:
- 사용자가 여러 metric을 자유롭게 골라 "겹쳐보기" 하는 기능은 있지만, metric별 특성(포화 구간, 민감도 곡선)에 대한 안내/경고가 UI 어디에도 없음.

**영향**:
"VMAF는 안정적인데 PSNR만 떨어졌다"를 보고 "실제로는 문제없다"고 성급히 결론짓거나, 반대로 metric 간 정상적인 특성 차이를 "이상 현상"으로 오인해 불필요한 재인코딩/디버깅 리소스를 소모한다.

**권장**:
- 서로 다른 metric families를 겹쳐 그릴 때 각 metric의 정상 변동 범위와 특성을 legend/tooltip에 명시.
- 정규화된 비교 뷰(z-score/percentile 기반)와 raw 값 뷰를 명확히 구분해 토글.
- metric 간 직접 비교가 방법론적으로 부적절한 경우 UI에서 경고.

**탐지**:
- 도메인 리뷰(비디오 품질 전문가)로, 같은 인코딩 열화에 대해 VMAF/PSNR/SSIM이 서로 다르게 반응하는 실제 사례를 재현해 차트가 오해를 유발하는지 확인.

**Bitvue 판정**: N/A — same root cause: there is no chart overlaying VMAF/PSNR/SSIM curves at all, so the normalization-free overlay comparison this item describes cannot occur yet.

---

### UIX-VIZ-020: missing frame을 선으로 연결

**분류**: VQ-Probe Chart · **심각도**: Critical · **탐지**: Code

**사용자 목표**:
정렬(alignment) 실패나 프레임 드롭으로 점수를 계산할 수 없는 구간을 명확히 인지한다.

**증상**:
- metric 계산이 실패했거나 원본/인코딩본 정렬이 안 된 구간에서 차트가 앞뒤 유효 데이터 포인트를 직선으로 이어버린다.
- 마치 그 구간에도 연속적인 점수가 존재하는 것처럼 보인다.

**원인**:
차트 라이브러리의 기본 line interpolation 동작(null/undefined를 건너뛰고 인접 점을 연결)을 그대로 사용하고, 결측값과 유효값을 구분하는 렌더링 로직이 없다.

**구현 냄새**:
- 데이터 배열에 결측 구간이 `null`로 들어있는데 차트 옵션에 `spanGaps: true`(또는 동급 설정)가 켜져 있음.

**영향**:
사용자가 "이 구간도 품질이 괜찮았다"고 결측 구간을 실측값으로 오인한다. alignment 실패가 잦은 구간(프레임 드롭이 심한 저품질 스트림)일수록 오히려 차트가 "매끄럽게" 보이는 역설이 발생해, 가장 문제 있는 구간이 가장 안전해 보이는 최악의 왜곡으로 이어진다.

**권장**:
- 결측 구간은 선을 끊거나(gap) 명시적 해칭/회색 음영 배경으로 "no data" 표시.
- hover 시 "alignment failed" 등 사유를 tooltip으로 제공.
- 결측 비율을 요약 통계에 별도 노출.

**탐지**:
- 의도적으로 프레임을 드롭한 테스트 시퀀스로 차트에 데이터 갭이 시각적으로 드러나는지 확인.
- `spanGaps` 유사 옵션 코드 검색.

**Bitvue 판정**: N/A — no per-frame metric time series exists to interpolate across gaps; nothing in frontend/ builds a line chart from `calculate_quality_metrics`'s per-frame `frames[]` array.

---

### UIX-VIZ-021: alignment confidence를 표시하지 않음

**분류**: VQ-Probe Chart · **심각도**: High · **탐지**: Domain review

**사용자 목표**:
원본과 인코딩본 프레임이 올바르게 매칭돼 metric이 계산됐는지 신뢰할 수 있는지 판단한다.

**증상**:
- 낮은 confidence로 정렬된 구간도 다른 구간과 동일한 시각적 스타일(실선, 동일 두께)로 그려진다.
- 정렬이 불확실한 점수와 확실한 점수가 구분 없이 제시된다.

**원인**:
alignment 알고리즘이 내부적으로 confidence score를 계산하지만 이를 차트 레이어까지 전달하는 파이프라인이 없거나, UI가 이를 사용하지 않는다.

**구현 냄새**:
- alignment 모듈 출력 타입에 confidence 필드가 있지만 차트 데이터 변환 단계에서 score만 추출하고 confidence는 drop.

**영향**:
정렬이 불확실한 구간의 낮은/이상한 점수를 "실제 품질 문제"로 오인하거나, 반대로 실제 품질 문제를 "정렬 오류 때문"이라고 잘못 넘겨짚을 근거조차 없다 — 두 방향 모두 잘못된 디버깅으로 이어진다. VQ-Probe의 핵심 신뢰 기반(정렬이 맞다는 전제)이 사용자에게 검증 불가능한 블랙박스가 된다.

**권장**:
- confidence를 선 두께/투명도/음영 밴드로 시각적으로 인코딩(낮은 confidence 구간은 옅게 또는 점선).
- confidence가 임계값 이하인 구간은 별도 경고 마커.
- 상세 패널에서 정확한 confidence 수치 노출.

**탐지**:
- 낮은 alignment confidence를 유발하는 테스트 케이스(프레임레이트 불일치, 심한 드롭)에서 UI가 이를 구분해 보여주는지 확인.
- Phase 2 ALIGN 카테고리 산출물과의 연동 여부를 도메인 리뷰로 확인.

**Bitvue 판정**: N/A (adjacent feature noted) — no metric chart exists to encode confidence visually. A related but distinct piece does exist: alignment confidence (crates/bitvue-core/src/alignment.rs) is surfaced in frontend/components/CompareWorkspace/CompareWorkspace.tsx:117-234 and CompareControls.tsx:109-112, but only as a single workspace-level badge/text (e.g. `{workspace.alignment.confidence}`), not encoded per-frame/segment in any timeline or chart.

---

### UIX-VIZ-022: scene 경계를 표시하지 않음

**분류**: VQ-Probe Chart · **심각도**: Medium · **탐지**: Visual

**사용자 목표**:
점수 변화가 실제 화질 저하 때문인지, 새로운 씬으로 전환됐기 때문인지 구분한다.

**증상**:
- 씬 전환 시점에 점수가 급격히 변해도 차트에 아무 표시가 없어, 변화의 원인이 인코더 문제인지 콘텐츠 특성 변화인지 즉시 판단할 수 없다.

**원인**:
scene detection 결과(키프레임 위치 또는 별도 씬 컷 감지 알고리즘 출력)가 metric 차트 컴포넌트와 별개 시스템으로 존재하며 서로 연결되지 않는다.

**구현 냄새**:
- 씬 경계 데이터는 타임라인/썸네일 패널에만 존재하고 metric 차트는 이를 import하지 않는 독립 컴포넌트 구조.

**영향**:
씬 전환 직후의 자연스러운 점수 dip(새 콘텐츠에 인코더가 아직 최적화되지 않은 초기 프레임들)을 "심각한 품질 결함"으로 오인해 불필요한 디버깅에 시간을 소모하거나, 반대로 진짜 결함 구간을 "씬 전환 때문이겠지"라고 넘겨짚어 실제 문제를 놓친다.

**권장**:
- scene 경계를 차트의 세로 기준선(vertical marker)으로 항상 함께 표시.
- 씬별 평균/분산을 별도로 세그먼트화해 보여주는 뷰 제공.
- 씬 전환 직후 N프레임을 별도 하이라이트("ramp-up region")로 구분.

**탐지**:
- 여러 씬 컷이 포함된 테스트 시퀀스로 차트-타임라인 간 씬 경계 위치 일치 여부를 시각 검사.

**Bitvue 판정**: N/A (adjacent feature noted) — no metric chart exists to mark scene boundaries on. Scene-change data does exist elsewhere (`is_scene_change` referenced in frontend/components/EnhancedView.tsx) but is not wired to any quality-metric visualization.

---

### UIX-VIZ-023: outlier 때문에 대부분의 변화가 평평해짐

**분류**: VQ-Probe Chart · **심각도**: High · **탐지**: Visual

**사용자 목표**:
시퀀스 전반의 정상적인 품질 변동 패턴(미세한 등락)을 읽는다.

**증상**:
- 단 하나의 극단적 outlier 프레임(alignment 실패, black frame 등으로 score가 0에 가깝게 튄 경우)이 있으면 Y축이 자동으로 그 값을 포함하도록 확장돼, 나머지 프레임의 유의미한 변동이 화면상 거의 일직선으로 눌린다.

**원인**:
차트 라이브러리의 기본 auto-range(min/max 기반) Y축 스케일링을 그대로 사용하고, outlier에 대한 별도 처리(clip, robust scaling, 별도 표시)가 없다.

**구현 냄새**:
- `yDomain: [min(data), max(data)]` 형태의 자동 범위 계산에 outlier 필터링 단계가 없음.

**영향**:
정상 범위 내 미세하지만 실제로 유의미한 품질 변동(A/B 인코더 비교에서 중요한 몇 점 차이)이 시각적으로 완전히 사라져, "두 결과가 똑같아 보인다"는 잘못된 결론에 도달한다 — outlier 하나 때문에 정작 중요한 비교 정보가 파묻힌다.

**권장**:
- Y축 범위를 percentile 기반(예: 1~99 percentile)으로 robust하게 설정.
- outlier는 화면 밖에 별도 마커/화살표로 "범위 밖 값 존재"를 표시.
- 전체 범위/robust 범위 토글 제공, outlier는 별도 리스트/알림으로 숨기지 않고 노출.

**탐지**:
- 정상 변동 데이터에 극단 outlier 하나를 주입한 합성 데이터셋으로 나머지 데이터의 시각적 변별력이 유지되는지 확인.

**관련**: `HEAT.md` HEAT-011 참고 — outlier가 autoscale 범위를 왜곡하는 동일 문제이나, 이쪽은 시계열 차트 Y축, HEAT-011은 heatmap color range 대상.

**Bitvue 판정**: N/A — no Y-axis auto-range chart for quality metrics exists in the codebase to exhibit this outlier-flattening behavior.

---

### UIX-VIZ-024: smoothing된 값을 raw 결과처럼 표시

**분류**: VQ-Probe Chart · **심각도**: Critical · **탐지**: Code

**사용자 목표**:
실제로 계산된 프레임별 원시 점수를 신뢰하고 판단 근거로 삼는다.

**증상**:
- 차트에 이동평균/스무딩 처리된 곡선이 그려지는데 legend나 축 라벨 어디에도 "smoothed" 표시가 없어 프레임별 raw 점수로 오인된다.

**원인**:
시각적 노이즈를 줄여 "보기 좋은" 차트를 만들기 위해 렌더링 단계에서 스무딩을 적용하지만, 이를 데이터 가공(transform)이 아니라 순수 스타일링으로 취급해 라벨링을 생략한다.

**구현 냄새**:
- 데이터 파이프라인에 `movingAverage(scores, windowSize)` 호출이 있지만 출력이 원본과 동일한 필드명/스타일로 차트에 전달됨.
- raw/smoothed를 전환할 UI 컨트롤이 없음.

**영향**:
단일 프레임 아티팩트(프레임 드롭 시 흔함)로 인한 급격한 열화가 스무딩으로 뭉개져 "문제 없음"으로 잘못 결론 내린다. QA가 실제 결함 프레임을 리포트에 포함하지 못하고 릴리스를 통과시킬 위험이 있다 — 이는 Phase 2 STAT이 다루는 "계산 정확성" 문제가 아니라, 계산은 맞았어도 보여주는 방식이 원본을 위장하는 문제다.

**권장**:
- raw 데이터와 smoothed 곡선을 시각적으로 명확히 구분(raw는 옅은 산점도/얇은 선, smoothed는 굵은 강조선으로 오버레이).
- legend에 smoothing window 크기 명시.
- raw-only 뷰를 항상 1클릭으로 전환 가능하게, export 시 raw 데이터를 항상 포함.

**탐지**:
- 단일 프레임 스파이크가 포함된 합성 데이터로 스무딩 적용 전/후 차트를 비교해 스파이크가 시각적으로 사라지는지 확인.
- UI에 smoothing indicator 존재 여부 코드 검사.

**Bitvue 판정**: N/A — no smoothing/moving-average code was found anywhere relevant (`grep -rl movingAverage|smoothing|rollingAverage` across frontend/ and crates/bitvue-core/src returned no hits), and no chart exists to mislabel raw-vs-smoothed data in the first place.

---

### UIX-VIZ-025: 차트 hover와 영상 프레임이 정확히 동기화되지 않음

**분류**: VQ-Probe Chart · **심각도**: Critical · **탐지**: Interaction

**사용자 목표**:
차트에서 특정 지점을 가리켰을 때 정확히 그 프레임의 영상을 확인해 점수의 원인을 검증한다.

**증상**:
- metric 차트에서 특정 x좌표(프레임 번호)에 hover/click 했을 때 실제 표시되는 프레임이 1~2프레임 어긋난다.
- VFR(가변 프레임레이트) 콘텐츠에서 시간축과 프레임 인덱스 매핑이 부정확해 완전히 다른 프레임이 표시된다.

**원인**:
차트의 x축이 프레임 인덱스가 아니라 display time 기반이고, 영상 플레이어 seek는 별도의 타임스탬프 계산 로직을 사용해 두 시스템이 동일한 프레임-시간 매핑 테이블을 공유하지 않는다. VFR/B-frame reorder가 있으면 문제가 더 심각해진다.

**구현 냄새**:
- 차트 컴포넌트와 플레이어 컴포넌트가 각각 독립적으로 `frameIndexToTime()`/`timeToFrameIndex()`를 구현(중복 로직).
- PTS/DTS 순서 처리가 한쪽에만 반영됨.

**영향**:
사용자가 "이 dip이 나는 지점의 프레임"이라고 확인한 영상이 실제로는 인접한 다른 프레임이다 — 잘못된 프레임을 근거로 "이건 인코더 버그다/아니다"라고 결론 내리는 근본적 오판으로 이어진다. B-frame reorder 구간에서 표시 순서와 디코딩 순서가 꼬이면 완전히 엉뚱한 프레임을 인코딩 결함의 증거로 제시할 위험도 있다.

**권장**:
- 차트와 플레이어가 동일한 단일 소스의 진실(single source of truth) 프레임-시간 매핑 테이블을 공유.
- hover 시 대상 프레임의 정확한 프레임 번호/타임스탬프를 명시적으로 표시해 사용자가 직접 검증 가능하게.
- VFR/B-frame reorder 스트림에 대한 동기화 정확성을 회귀 테스트에 포함.

**탐지**:
- 알려진 프레임 번호에 고유 식별자(워터마크, 프레임 번호 burn-in)를 삽입한 테스트 시퀀스로 hover된 프레임과 실제 표시 프레임의 워터마크가 일치하는지 자동화 테스트.

**Bitvue 판정**: N/A — no hover-synced quality-metric chart exists in the codebase; the closest analogue, BitrateGraphPanel.tsx, indexes hover state directly into its `displayFrames` array by position (lines 38,179-207) rather than via a separate time↔frame mapping, so the specific dual-implementation desync this item describes doesn't have a component to occur in yet.
