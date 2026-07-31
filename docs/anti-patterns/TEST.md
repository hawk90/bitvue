# Anti-Pattern Catalog — TEST: 테스트 전략 안티패턴

이 문서는 더 큰 안티패턴 카탈로그의 한 카테고리입니다 (전체 인덱스는 `docs/anti-patterns/INDEX.md` 참고, Wave 1~3에 이어 Phase 4에서 작성). Wave 1의 `PARSE.md`가 "malformed input을 코드가 어떻게 다뤄야 하는가"라는 코드 레벨 문제를 다뤘다면, 이 문서는 그 짝을 이루는 테스트-전략 레벨 문제를 다룬다 — 즉 `PARSE.md`의 수정이 실제로 효과가 있다는 것을 *어떻게 아는가*. 좋은 파서 코드와 나쁜 테스트 스위트가 공존하면, 회귀는 리뷰가 아니라 사용자 리포트로 발견된다.

---

### TEST-001: 작은 정상 샘플만 테스트
**분류**: 커버리지 편향 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[test]
fn test_parse_avc_frame() {
    let data = include_bytes!("../fixtures/sample_320x240_30fps.h264");
    let frames = parse_avc(data).unwrap();
    assert_eq!(frames.len(), 10);
}

#[test]
fn test_parse_hevc_frame() {
    let data = include_bytes!("../fixtures/sample_320x240_30fps.hevc");
    let frames = parse_hevc(data).unwrap();
    assert_eq!(frames.len(), 10);
}
// 이 파일에 있는 모든 테스트가 같은 인코더(x264/x265 기본 설정)로
// 같은 해상도, 같은 GOP 구조로 뽑은 10프레임짜리 샘플만 사용한다.
```

**문제**:
- 테스트 스위트 전체가 "정상적으로 인코딩된, 작고, 균일한" 입력 공간의 한 점만 반복 검증한다.
- 해상도 극단값(1x1, 8192x8, odd width/height), 다중 슬라이스/타일, 인터레이스, 10bit/12bit, 다중 레퍼런스 프레임 같은 실제 프로덕션에서 마주치는 조합이 커버리지에서 완전히 빠진다.
- 커버리지 리포트(라인/브랜치)는 높게 나오지만 이는 착시다 — 같은 코드 경로를 다른 이름의 테스트로 반복 실행했을 뿐 새로운 상태 공간을 탐색하지 않는다.
- 실제 사용자가 보내는 파일(다양한 인코더, 다양한 프로파일/레벨, 비표준이지만 유효한 조합)에서만 재현되는 버그가 릴리스 이후 발견된다.

**발생 조건**:
- 프로젝트 초기에 "일단 돌아가는지" 확인할 목적으로 만든 샘플이 그대로 회귀 스위트의 전부가 되었을 때.
- 새 코덱(VP9, AV1)을 추가할 때 기존 스위트의 샘플 패턴을 복사-붙여넣기해서 동일한 편향을 재생산할 때.

**권장**:
```rust
// 해상도/프로파일/인코더 축을 명시적으로 매트릭스화해서 생성
#[rstest]
#[case::tiny("1x1_i420.av1")]
#[case::odd_dims("321x241_i420.av1")]           // non-16-aligned
#[case::multi_tile("3840x2160_4tile.av1")]
#[case::hdr10("1920x1080_hdr10_bt2020.av1")]
#[case::superres("1920x1080_superres.av1")]
#[case::film_grain("1920x1080_film_grain.av1")]
#[case::many_refs("1280x720_8refs.av1")]
fn test_parse_av1_matrix(#[case] fixture: &str) {
    let data = load_fixture(fixture);
    let frames = parse_av1(&data).expect(&format!("failed on {fixture}"));
    assert_frame_invariants(&frames);
}
```
- 코덱 spec의 프로파일/레벨/tool 목록을 기준으로 커버리지 매트릭스를 별도 문서로 관리하고, 테스트 이름에서 어떤 축을 커버하는지 드러낸다.
- 여러 인코더(libaom, SVT-AV1, rav1e 등)로 뽑은 샘플을 섞어 "특정 인코더의 습관"에 파서가 암묵적으로 의존하지 않는지 검증한다.

**탐지 방법**:
- Structural: 픽스처 디렉터리의 파일명/메타데이터를 스캔해 해상도·프로파일·인코더 축의 분포를 집계하는 스크립트. 한 축에 값이 하나뿐이면 경고.
- Manual: 코덱 spec 목차(프로파일, 툴, 레벨 제약)를 체크리스트로 두고 테스트 매트릭스와 대조.

**예외**:
- 유닛 레벨에서 특정 함수 하나의 로직만 검증하는 테스트는 작은 샘플이 적절하다 — 이 안티패턴은 "이것이 스위트 전체의 유일한 검증 방식"일 때 해당한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-002: malformed input 테스트 없음
**분류**: 강건성 커버리지 · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// tests/parse_tests.rs 전체를 grep해도 "malformed", "truncat", "corrupt",
// "invalid" 같은 이름의 테스트가 하나도 없다.
#[test]
fn test_all_valid_bitstreams_parse() {
    for fixture in list_fixtures("valid/") {
        assert!(parse(&fixture).is_ok());
    }
}
// negative-path 테스트가 전무 — parse()가 Err를 반환하거나
// panic하는 경로는 유닛 테스트로 한 번도 실행되지 않는다.
```

**문제**:
- `parse()`가 malformed 입력에서 panic/OOM/무한루프를 일으켜도 CI가 이를 절대 잡아내지 못한다.
- "에러 처리 코드가 존재한다"와 "에러 처리 코드가 테스트에서 실행된다"는 다르다 — 커버리지 도구도 실행되지 않은 브랜치는 미달로 표시하지만, 아무도 그 리포트를 보지 않으면 의미가 없다.
- fuzzing이 있다고 해서 유닛 레벨 negative-path 테스트가 불필요해지지 않는다. fuzzing은 "아직 모르는" 크래시를 찾고, 유닛 테스트는 "한 번 찾은" 크래시가 다시 발생하지 않음을 빠르고 결정적으로 보장한다 — 둘은 대체재가 아니라 보완재다.
- 리뷰어가 "에러 핸들링이 잘 되어 있다"고 판단할 근거가 코드를 읽는 것뿐이라, 리팩터링 중 조용히 깨져도 아무도 모른다.

**발생 조건**:
- 신뢰할 수 있는 소스(자체 인코더, 검증된 컨포먼스 스트림)의 입력만 다루던 초기 단계의 테스트 습관이 "사용자가 임의 파일을 올린다"는 요구사항이 추가된 후에도 갱신되지 않았을 때.
- fuzz 타깃은 있지만 CI에서 짧게(수 초) 스모크로만 돌고, 발견된 크래시가 회귀 테스트로 승격되지 않을 때.

**권장**:
```rust
#[test]
fn test_truncated_at_every_offset_does_not_panic() {
    let full = load_fixture("valid/1080p_baseline.h264");
    for len in 0..full.len().min(4096) {
        let truncated = &full[..len];
        // panic이 아니라 Err를 기대 — catch_unwind로 이중 안전망
        let result = std::panic::catch_unwind(|| parse_avc(truncated));
        assert!(result.is_ok(), "panicked at truncation length {len}");
    }
}

#[test]
fn test_declared_size_exceeds_buffer() {
    let mut data = load_fixture("valid/small.h264");
    corrupt_nal_length_field(&mut data, /*inflate_by=*/ 1_000_000);
    assert!(matches!(parse_avc(&data), Err(ParseError::OutOfBounds { .. })));
}
```
- "유효한 파일의 모든 truncation prefix", "필드별 bit-flip", "선언된 크기와 실제 크기 불일치" 같은 malformed 카테고리를 파서 크레이트마다 표준 테스트 헬퍼로 제공한다.
- `PARSE.md`에서 식별한 각 항목(unchecked offset, integer overflow 등)에 대응하는 회귀 테스트를 1:1로 유지한다.

**탐지 방법**:
- Structural: 테스트 파일명/함수명에서 negative-path 키워드 부재를 grep으로 감지, 커버리지 리포트에서 `Err` 반환 브랜치의 실행 여부를 별도 집계.
- Runtime: `catch_unwind` 기반 truncation/bit-flip 스윕을 CI 필수 단계로 실행.

**예외**:
- 파서 레이어가 아니라 이미 검증된 내부 데이터 구조를 다루는 상위 레이어(예: 이미 파싱된 프레임 리스트를 정렬하는 함수)는 malformed 바이트 테스트가 불필요하다 — 신뢰 경계 안쪽이기 때문이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-003: decoder 결과를 자기 parser 결과로 검증
**분류**: 검증 무결성 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
#[test]
fn test_motion_vector_extraction() {
    let bitstream = load_fixture("valid/1080p_bframes.hevc");

    // "정답"을 우리 자신의 파서로 만든다
    let expected_mvs = our_hevc_parser::extract_mvs(&bitstream);

    // 그리고 그 정답과 우리 파서를 비교한다
    let actual_mvs = our_hevc_parser::extract_mvs(&bitstream);

    assert_eq!(expected_mvs, actual_mvs); // 항상 통과 — 자기 자신과 비교
}

// 조금 덜 노골적이지만 본질은 같은 변형:
#[test]
fn test_mv_prediction_matches_our_model() {
    let mvs = our_hevc_parser::extract_mvs(&bitstream);
    let predicted = our_mv_predictor::predict(&mvs.neighbors); // 같은 가정 공유
    assert_eq!(predicted, mvs.actual);
}
```

**문제**:
- 이것은 "테스트가 없는 것"보다 더 위험하다 — 테스트가 초록불로 통과하기 때문에 심리적 안전감을 주지만 실제로는 아무것도 검증하지 않는다.
- **핵심 함정**: parser와 test가 동일한 스펙 오독이나 잘못된 가정을 공유하면, self-referential 테스트 스위트는 그 가정이 틀렸다는 사실을 영원히 발견하지 못한 채 통과한다. 예를 들어 HEVC MV 예측에서 `mvd_l1`의 부호 처리를 파서가 잘못 이해했다면, 같은 이해로 작성된 "기대값"도 같은 방향으로 틀리므로 테스트는 계속 초록불이다.
- 리팩터링/최적화 중 실제로 동작이 바뀌어도(예: SIMD 경로 도입, 캐시 추가) 이 테스트는 "우리가 우리와 같다"만 확인하므로 회귀를 절대 잡지 못한다.
- 이 패턴은 "출력 형식이 복잡해서 손으로 기대값을 만들기 귀찮다"는 실용적 이유에서 시작되는 경우가 많고, 그래서 코드 리뷰에서도 "테스트가 있으니 됐다"고 넘어가기 쉽다.

**발생 조건**:
- MV, 양자화 계수, in-loop filter 강도처럼 스펙 문서만 보고 손으로 기대값을 유도하기 번거로운 저수준 디코딩 산출물을 검증할 때.
- 참조 디코더(FFmpeg 등)를 아직 CI에 통합하지 않은 프로젝트 초기 단계에서 "일단 테스트를 채워야 한다"는 압박이 있을 때.
- 코드 커버리지 목표(예: 90%)를 채우기 위해 빠르게 작성한 테스트가 그대로 남을 때.

**권장**:
```rust
// 방법 1: spec conformance vector (JVT/JCT-VC, AOMedia가 배포하는 공식 벡터) 사용
#[test]
fn test_mv_matches_conformance_vector() {
    let bitstream = load_fixture("conformance/hevc/MVEDGE_A_qualcomm.bit");
    let expected = load_conformance_expected("MVEDGE_A_qualcomm.mv.json"); // 외부 출처
    let actual = our_hevc_parser::extract_mvs(&bitstream);
    assert_eq!(actual, expected);
}

// 방법 2: 독립 구현(참조 디코더)과 차분 비교
#[test]
fn test_mv_matches_ffmpeg_reference() {
    let bitstream = load_fixture("valid/1080p_bframes.hevc");
    let ffmpeg_mvs = run_ffmpeg_debug_mv_dump(&bitstream); // 별도 프로세스, 별도 코드베이스
    let our_mvs = our_hevc_parser::extract_mvs(&bitstream);
    assert_mv_within_tolerance(&our_mvs, &ffmpeg_mvs);
}
```
- 정답의 출처가 테스트 대상 코드와 **완전히 독립적**이어야 한다: (1) 표준화 기구가 배포하는 spec conformance vector, (2) FFmpeg/libavcodec 같은 별도 구현의 디버그 출력, (3) 스펙 문서에서 손으로 유도한 소규모 사례.
- "정답을 어떻게 구했는가"를 테스트 코드 옆 주석/커밋 메시지로 남겨, 나중에 그 정답의 신뢰성을 재검증할 수 있게 한다.

**탐지 방법**:
- Semantic: 테스트 함수 내에서 expected 값의 출처를 추적 — 같은 크레이트/모듈의 함수 호출로 생성되었다면 플래그. 코드 리뷰 체크리스트 항목으로 "이 expected는 어디서 왔는가"를 명시적으로 묻는다.
- Manual: 새 테스트 PR 리뷰 시 "이 assert의 우변이 좌변과 같은 코드 경로를 타는가"를 확인.

**예외**:
- 순수 회귀 테스트(golden output이 "정답"이 아니라 "이전 실행 결과와 달라지지 않았음"을 확인하는 용도로 명시된 경우)는 이 패턴이 아니다 — 단, 그 경우 테스트 이름과 문서에 "correctness가 아니라 stability를 검증한다"는 점을 명확히 해야 한다(TEST-009 참고).

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-004: golden file 하나에 너무 많은 의미 포함
**분류**: 픽스처 설계 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[test]
fn test_full_pipeline_golden() {
    let data = load_fixture("valid/complex_hdr_bframes_multitile.av1");
    let result = full_pipeline(&data).unwrap();
    // golden.json 안에 SPS/PPS 파싱, MV, 양자화, 필름그레인,
    // 컬러 메타데이터, 타일 레이아웃까지 전부 한 번에 들어있다
    assert_eq!(result, load_golden("golden_001.json"));
}
```

**문제**:
- 이 테스트가 실패했을 때 무엇이 깨졌는지 diff만으로 알 수 없다 — 필름그레인 파싱 버그인지 타일 레이아웃 회귀인지 golden.json의 거대한 diff를 사람이 다시 해석해야 한다.
- 하나의 golden 파일이 여러 기능 영역을 동시에 커버하도록 설계되면, 그 중 하나만 의도적으로 바꿔도(예: 컬러 메타데이터 필드 이름 변경) 전체 골든이 깨져 리뷰어가 "정말 회귀인지 의도된 변경인지" 판단하기 어려워진다.
- 실패 시 디버깅 비용이 테스트 작성 비용보다 커진다 — 특히 CI에서 실패했을 때 로컬 재현/이등분(bisect)에 시간이 오래 걸린다.

**발생 조건**:
- "복잡한 실제 파일 하나로 최대한 많이 커버하자"는 효율주의가 픽스처 설계를 지배할 때.
- golden 파일 생성이 수동/느린 프로세스라서 파일 개수를 늘리기보다 기존 파일에 검증 항목을 계속 추가하는 쪽을 택할 때.

**권장**:
```rust
// 기능 영역별로 골든을 분리하고, 각 골든은 해당 영역만 검증
#[test]
fn test_golden_film_grain_params() {
    let data = load_fixture("valid/film_grain_only.av1");
    let fg = extract_film_grain(&data).unwrap();
    assert_eq!(fg, load_golden("film_grain.json"));
}

#[test]
fn test_golden_tile_layout() {
    let data = load_fixture("valid/4tile_only.av1");
    let layout = extract_tile_layout(&data).unwrap();
    assert_eq!(layout, load_golden("tile_layout.json"));
}
```
- 골든 파일 하나당 검증하는 "의미 단위"를 하나로 제한하고, 파일명/테스트 이름에서 그 단위를 드러낸다.
- 골든 diff를 사람이 읽기 쉬운 구조화 포맷(JSON with stable key order, 또는 텍스트 dump)으로 유지해 `git diff`만으로 무엇이 바뀌었는지 알 수 있게 한다.

**탐지 방법**:
- Structural: golden 파일 크기/필드 수를 정적으로 집계해 이상치(다른 골든보다 10배 큰 파일)를 탐지.
- Manual: 골든 테스트 실패 시 리뷰어가 diff를 읽고 원인을 30초 내에 특정할 수 있는지를 리뷰 기준으로 삼는다.

**예외**:
- end-to-end smoke test로 "전체 파이프라인이 어쨌든 끝까지 도는가"만 확인하는 목적이라면 하나의 큰 골든도 괜찮다 — 단, 이 경우 세밀한 회귀 원인 분석은 다른 세분화된 테스트에 위임한다는 전제가 필요하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-005: binary fixture 출처·라이선스 불명
**분류**: 픽스처 위생 · **심각도**: High · **탐지**: Manual

**나쁜 예**:
```rust
// fixtures/README.md 없음, 커밋 메시지: "add test files"
// fixtures/random_4k_sample.mp4        (312 MB, 출처 불명)
// fixtures/movie_clip_do_not_share.mkv (어디서 다운로드했는지 아무도 기억 못함)
// fixtures/from_bugreport_1234.ts      (사용자가 첨부한 파일, 라이선스 언급 없음)
```

**문제**:
- 라이선스가 불명확한 상업 콘텐츠(영화/방송 클립)가 오픈소스 리포지토리에 커밋되면 배포 시 법적 리스크가 된다 — 특히 저장소가 public이 되거나 fork/mirror될 때.
- 사용자가 버그 리포트에 첨부한 파일을 그대로 픽스처화하면 그 파일에 개인정보(스크린 레코딩에 찍힌 화면, 워터마크, 사설 콘텐츠)가 포함되어 있을 수 있다.
- 출처를 모르면 "이 파일이 왜 이 코드 경로를 테스트하기 위해 선택되었는가"라는 의도가 사라져, 나중에 누군가 "이 파일 지워도 되나?"를 판단할 근거가 없다.
- 대용량 바이너리가 git 히스토리에 그대로 쌓이면 clone 크기가 계속 불어나고, 나중에 삭제해도 히스토리에서 완전히 제거하려면 rewrite가 필요하다.

**발생 조건**:
- 버그 재현을 급하게 처리하느라 사용자가 보낸 파일을 그대로 `fixtures/`에 커밋할 때.
- 인터넷에서 "적당히 복잡한 4K 샘플"을 검색해 다운로드한 후 출처 기록 없이 추가할 때.

**권장**:
```rust
// fixtures/MANIFEST.toml
[[fixture]]
path = "conformance/av1/av1-1-b8-01-size-16x16.ivf"
source = "https://aomedia.googlesource.com/aom (test vector, BSD-2-Clause)"
license = "BSD-2-Clause"
purpose = "minimum block size edge case"

[[fixture]]
path = "synthetic/truncated_sps.h264"
source = "generated by scripts/gen_truncated_fixtures.py from conformance/avc/CANL1.264"
license = "derived from JVT conformance vector (royalty-free for conformance testing)"
purpose = "truncation at SPS boundary regression (issue #482)"
```
- 모든 바이너리 픽스처를 매니페스트에 등록하고 출처/라이선스/용도를 필수 필드로 강제한다(CI에서 매니페스트 누락 파일을 검사).
- 가능한 한 실제 콘텐츠 대신 synthetic하게 생성한 최소 비트스트림을 우선 사용하고, 실제 콘텐츠가 필요하면 표준화 기구의 공개 conformance vector(royalty-free 명시)를 우선한다.
- 사용자 제보 파일은 개인정보/저작권 소거 후 최소 재현 사례로 축소(minimize)해서 커밋한다 — 원본 그대로 커밋하지 않는다.
- 대용량 바이너리는 Git LFS 또는 별도 아티팩트 스토리지로 분리해 리포지토리 히스토리를 가볍게 유지한다.

**탐지 방법**:
- Manual: 신규 바이너리 파일 추가 PR에 매니페스트 항목 필수화, 코드 리뷰 체크리스트에 라이선스 확인 항목 추가.
- Structural: pre-commit/CI 훅으로 `fixtures/` 하위 신규 바이너리가 `MANIFEST.toml`에 등록됐는지 검사.

**예외**:
- CI에서만 다운로드하고 리포지토리에는 커밋하지 않는 대용량 corpus(예: 외부 스토리지에서 캐시로 받아오는 fuzz corpus)는 라이선스 조건이 다운로드 스크립트/CI 설정에 명시되어 있으면 충분하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-006: codec spec edge case 미포함
**분류**: 커버리지 편향 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
// AV1 spec 7.11.3의 quantizer index 특수값(0, 255)이나
// HEVC의 chroma_format_idc == 3(4:4:4) + separate_colour_plane_flag 조합처럼
// "스펙에 명시되어 있지만 흔한 인코더는 절대 만들지 않는" 값들이
// 테스트 목록 어디에도 등장하지 않는다.
#[test]
fn test_parse_qp_values() {
    let data = load_fixture("valid/qp_30.av1"); // 흔한 값만
    assert!(parse(&data).is_ok());
}
```

**문제**:
- 실제 인코더가 만들지 않는 조합이라도 스펙상 유효하므로, 표준을 준수하는 다른 도구가 만든 파일이나 의도적으로 이런 값을 사용하는 파일(스트레스 테스트, 상호운용성 테스트용 스트림)에서 우리 파서가 깨질 수 있다.
- "흔한 인코더 출력만 다룬다"는 암묵적 가정이 코드 구조에 스며들면(예: 특정 필드가 항상 특정 범위라고 가정한 산술), 나중에 그 가정을 깨는 입력이 들어왔을 때 단순 파싱 실패가 아니라 잘못된 계산 결과를 조용히 반환할 위험이 있다.
- reserved bits, must-be-zero 필드에 대한 spec의 요구사항(예: "디코더는 이 필드가 0이 아니어도 무시해야 한다" vs "값이 다르면 미래 버전을 위한 것이므로 특정 처리를 해야 한다")을 놓치면 향후 스펙 버전과의 호환성이 깨진다.

**발생 조건**:
- 테스트 픽스처를 전부 자체 인코더 파이프라인에서 생성해서, 인코더가 절대 만들지 않는 값 조합이 애초에 입력 후보에 오르지 않을 때.
- 코덱 spec을 처음부터 끝까지 읽고 "이 필드의 전체 값 도메인"을 표로 정리하는 작업을 생략하고 구현부터 시작했을 때.

**권장**:
```rust
// spec 문서의 각 절을 참조하는 명시적 edge-case 테스트 목록 유지
#[test]
fn test_hevc_444_separate_colour_plane() {
    // spec 7.4.3.2: chroma_format_idc == 3 && separate_colour_plane_flag == 1
    let data = load_fixture("spec_edge/hevc_444_separate_plane.hevc");
    let sps = parse_sps(&data).unwrap();
    assert_eq!(sps.chroma_array_type, 0); // spec 7.4.3.2 정의대로
}

#[test]
fn test_av1_qindex_boundary_values() {
    for qindex in [0u8, 1, 254, 255] {
        let data = build_synthetic_frame_with_qindex(qindex);
        assert!(parse_av1(&data).is_ok(), "failed at qindex={qindex}");
    }
}
```
- 코덱별로 "spec edge case 체크리스트"를 별도 문서로 관리하고(스펙 절 번호 참조 포함), 각 항목에 대응하는 테스트를 1:1로 연결한다.
- edge case 픽스처는 실제 인코더로 만들 수 없는 경우가 많으므로, 비트스트림을 직접 조립하는 synthetic 빌더 유틸리티를 갖춘다.

**탐지 방법**:
- Semantic: 코덱 spec 목차/필드 정의 테이블과 테스트 커버리지 매트릭스를 대조하는 수동 감사(주기적으로).
- Manual: 신규 코덱 지원 추가 시 "spec의 모든 reserved/conditional 필드 조합을 나열했는가"를 리뷰 체크리스트에 포함.

**예외**:
- 정말로 스펙에서 deprecated/미사용으로 명시된 조합(향후 버전에서 제거 예정)은 우선순위를 낮춰도 된다 — 단, "무시해도 되는 이유"를 문서화해야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-007: timestamp·VFR·B-frame 테스트 부족
**분류**: 시간축 커버리지 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
#[test]
fn test_frame_ordering() {
    // CFR(고정 프레임레이트), IPPP만 있는 단순 GOP 하나로만 검증
    let data = load_fixture("valid/cfr_ipppp_30fps.h264");
    let frames = decode_order_to_display_order(&data).unwrap();
    assert_eq!(frames.len(), 30);
    // PTS/DTS 재정렬, VFR, 열린/닫힌 GOP 경계는 어디에도 없다
}
```

**문제**:
- B-frame이 있으면 decode order와 display order가 달라지는데(PTS/DTS 분리), 이 재정렬 로직은 IPPP 전용 픽스처로는 절대 실행되지 않는다.
- VFR(가변 프레임레이트) 콘텐츠에서 PTS 간격이 균일하지 않은 경우, "프레임 간격 = 1/fps"라는 암묵적 가정이 있는 코드는 CFR 픽스처만으로는 절대 걸리지 않는다.
- 컨테이너의 edit list, negative composition offset(B-frame이 있는 MP4의 첫 샘플), timestamp wraparound(장시간 스트림에서 PTS가 32bit/33bit 경계를 넘는 경우) 같은 실무에서 흔히 문제가 되는 케이스가 테스트 스위트에 없으면, 이런 버그는 항상 "특정 사용자의 특정 파일"에서만 재현되는 형태로 나타나 디버깅 비용이 커진다.
- 스트림 중간에 GOP 구조가 바뀌는 경우(예: 방송 스트림에서 splice point), 열린 GOP와 닫힌 GOP가 섞인 경우를 다루지 못하면 재생/탐색(seek) 기능에서 프레임 드롭이나 아티팩트로 나타난다.

**발생 조건**:
- 테스트 픽스처가 전부 단순한 CFR, 낮은 B-frame depth로 인코딩된 "이상적인" 스트림일 때.
- 컨테이너 레벨 타임스탬프 처리(edit list, composition time)와 코덱 레벨 프레임 순서(POC, decode order)를 분리해서 테스트하지 않고 뭉뚱그려 "재생이 되는지"만 확인할 때.

**권장**:
```rust
#[rstest]
#[case::vfr("valid/vfr_screen_capture.mp4")]           // 15~60fps 혼재
#[case::high_bframe_depth("valid/bframe_depth4_hierarchical.hevc")]
#[case::negative_composition_offset("valid/mp4_first_sample_negative_cts.mp4")]
#[case::pts_wraparound("valid/long_stream_pts_wrap_33bit.ts")]
#[case::gop_structure_change("valid/splice_open_to_closed_gop.ts")]
fn test_timestamp_and_reordering(#[case] fixture: &str) {
    let data = load_fixture(fixture);
    let timeline = build_display_timeline(&data).unwrap();
    assert_monotonic_pts(&timeline);
    assert_no_duplicate_or_gap_beyond_tolerance(&timeline);
}
```
- decode order → display order 재정렬 로직을, B-frame depth를 파라미터화한 여러 GOP 구조에 대해 명시적으로 검증한다.
- 컨테이너 timestamp 처리(edit list, composition offset, wraparound)와 코덱 레벨 POC 계산을 별도 테스트 축으로 분리해 각각 독립적으로 커버리지를 추적한다.

**탐지 방법**:
- Semantic: "timestamp", "pts", "dts", "vfr", "reorder", "gop" 키워드로 테스트 함수명을 집계해 커버리지 공백을 식별.
- Runtime: 랜덤화된 GOP 구조/B-frame depth로 synthetic 스트림을 생성해 display order 재구성이 항상 단조 증가하는지 property test로 검증(TEST-013과 연계).

**예외**:
- 컨테이너를 다루지 않고 순수 엘리멘터리 스트림 파싱만 담당하는 모듈은 컨테이너 레벨 timestamp(edit list 등) 테스트가 범위 밖이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-008: deterministic하지 않은 병렬 테스트
**분류**: 테스트 신뢰성 · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
#[test]
fn test_frame_cache_eviction_order() {
    let cache = FrameCache::new(capacity: 4);
    let handles: Vec<_> = (0..8).map(|i| {
        let cache = cache.clone();
        std::thread::spawn(move || cache.insert(i, make_frame(i)))
    }).collect();
    for h in handles { h.join().unwrap(); }

    // 삽입 스레드 실행 순서에 따라 결과가 달라지는데
    // 특정 순서를 가정하고 단언한다 — 로컬에서는 우연히 항상 통과
    assert_eq!(cache.evicted_order(), vec![0, 1, 2, 3]);
}
```

**문제**:
- 스레드 스케줄링 순서에 결과가 의존하는데 특정 순서를 하드코딩해서 검증하면, CI 러너의 코어 수/부하에 따라 간헐적으로 실패하는 "flaky test"가 된다.
- flaky test는 반복되면 팀이 "재실행하면 통과하니까 무시"하는 학습된 무기력을 만들고, 결국 진짜 회귀가 발생해도 flaky의 일종으로 오인해 놓친다.
- 병렬 테스트 실행기(cargo nextest 등)가 테스트 순서/코어 배정을 바꿀 때마다 실패율이 달라지면 "재현 방법"을 특정하기 어려워 디버깅 비용이 크다.
- CI에서만 실패하고 로컬에서는 항상 통과하는 경우, 코어 수 차이·타이밍 차이가 원인인 것을 모른 채 "CI 환경 문제"로 오분류되어 방치되기 쉽다.

**발생 조건**:
- 캐시 축출, 워커 풀 스케줄링, 비동기 프레임 디코딩 파이프라인처럼 본질적으로 실행 순서가 비결정적인 컴포넌트를 테스트할 때 순서 의존적 단언을 그대로 사용할 때.
- `std::thread::sleep`으로 "충분히 기다리면 될 것"이라고 가정하고 타이밍을 맞추는 테스트.

**권장**:
```rust
#[test]
fn test_frame_cache_respects_capacity_invariant() {
    let cache = FrameCache::new(capacity: 4);
    let handles: Vec<_> = (0..8).map(|i| {
        let cache = cache.clone();
        std::thread::spawn(move || cache.insert(i, make_frame(i)))
    }).collect();
    for h in handles { h.join().unwrap(); }

    // 순서가 아니라 순서에 무관한 불변식을 검증
    assert!(cache.len() <= 4);
    assert!(cache.all_entries_valid());
}

#[test]
fn test_frame_cache_eviction_order_deterministic_scheduling() {
    // 순서를 반드시 검증해야 한다면, 결정적 스케줄러(loom 등)로 모든 인터리빙을 탐색
    loom::model(|| {
        let cache = FrameCache::new(capacity: 4);
        // ...
    });
}
```
- 병렬 실행 결과를 검증할 때는 "특정 순서"가 아니라 "순서 무관 불변식"(용량 초과 없음, 데이터 손상 없음, 각 항목이 정확히 한 번 처리됨)을 단언한다.
- 정말로 순서/인터리빙 자체가 버그의 원인일 수 있는 동시성 로직은 `loom` 같은 결정적 모델 체커로 가능한 모든 스케줄을 탐색한다.
- CI에서 flaky test를 자동 격리(quarantine)하고 실패율을 추적하는 대시보드를 두어, "무시해도 되는 것"과 "진짜 회귀"를 구분한다.

**탐지 방법**:
- Runtime: 동일 테스트를 스레드 수/실행 순서를 무작위화해 수백 번 반복 실행하는 CI job(stress job)으로 flaky 여부 자체를 검출.
- Structural: 테스트 코드에서 `thread::sleep` + 고정 시간값, 병렬 실행 후 순서 의존적 `assert_eq!` 패턴을 정적으로 grep.

**예외**:
- 순서가 스펙/계약으로 보장된 경우(예: 단일 스레드 실행 경로, 또는 명시적으로 FIFO를 보장하는 자료구조)라면 순서 단언이 정당하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-009: snapshot test가 구조 변경을 모두 승인하게 만듦
**분류**: 검증 무결성 · **심각도**: High · **탐지**: Manual

**나쁜 예**:
```rust
#[test]
fn test_bitstream_tree_snapshot() {
    let data = load_fixture("valid/1080p_sample.hevc");
    let tree = build_unit_tree(&data);
    insta::assert_yaml_snapshot!(tree);
}
```
```
# 리뷰 습관: CI가 실패하면 diff를 읽지 않고 바로
$ cargo insta review
> a  (모두 accept)
```

**문제**:
- snapshot diff가 수백~수천 줄로 커지면(트리 구조 전체 dump 등) 리뷰어가 실제로 diff를 읽지 않고 "테스트 갱신"으로 일괄 승인하는 습관이 생긴다.
- 이렇게 되면 snapshot test는 "의도한 변경만 반영되었는가"를 검증하는 안전망이 아니라, 실제로는 어떤 회귀도 걸러내지 못하는 요식 행위로 전락한다.
- 특히 필드 이름 변경, 직렬화 포맷 변경처럼 "구조적이지만 의미상 무관한" 변경과 "실제 파싱 결과가 달라진" 변경이 같은 diff 안에 섞여 있으면 사람이 구분하기 매우 어렵다.
- snapshot 승인이 PR 작성자 본인에 의해 로컬에서 이루어지고 그 결과만 커밋되면, 리뷰어는 diff의 텍스트만 보고 "진짜 검토"를 하지 않은 채 머지하는 경우가 많다.

**발생 조건**:
- 출력 구조가 복잡하고 자주 바뀌는 컴포넌트(UI 트리, 파싱된 신택스 트리 dump)에 snapshot test를 전체 구조 단위로 걸었을 때.
- 리팩터링이 잦은 시기에 snapshot 갱신이 반복되면서 "diff 읽기"가 습관적으로 생략될 때.

**권장**:
```rust
// 스냅샷 범위를 좁혀서 diff가 "무엇이 왜 바뀌었는지" 한눈에 보이게 한다
#[test]
fn test_sps_fields_snapshot() {
    let sps = parse_sps(&load_fixture("valid/1080p_sample.hevc")).unwrap();
    insta::assert_yaml_snapshot!(sps, {
        ".profile_tier_level" => "[filtered]", // 관심 없는 필드는 명시적으로 제외
    });
}

// 구조 변경과 의미 변경을 분리: 필드 이름 변경은 별도 마이그레이션 테스트로,
// 파싱 로직 변경은 값 자체를 검증하는 assert_eq!로
#[test]
fn test_sps_semantic_values_unaffected_by_refactor() {
    let sps = parse_sps(&load_fixture("valid/1080p_sample.hevc")).unwrap();
    assert_eq!(sps.pic_width_in_luma_samples, 1920);
    assert_eq!(sps.chroma_format_idc, 1);
}
```
- snapshot 대상을 기능 단위로 좁혀서, diff 한 건이 한 가지 의미 변화만 담도록 유지한다(TEST-004와 같은 원리).
- CI에서 snapshot 갱신 커밋에는 "왜 갱신했는지" 설명을 PR 템플릿으로 강제하고, 리뷰어가 diff를 실제로 읽었는지 확인하는 절차(예: diff 라인 수 임계값 초과 시 2인 이상 승인 요구)를 둔다.
- 진짜로 검증하고 싶은 불변식은 snapshot이 아니라 명시적 `assert_eq!`로 분리해, snapshot이 깨져도 핵심 불변식 테스트는 별도로 계속 지킨다.

**탐지 방법**:
- Manual: PR 리뷰에서 snapshot diff의 라인 수와 실제 코드 변경 diff의 비율이 비정상적으로 큰 경우(예: 코드 3줄 변경에 snapshot 2000줄 변경) 플래그.
- Structural: snapshot 파일 크기/갱신 빈도를 추적해 지나치게 자주 갱신되는 snapshot을 식별, 세분화 후보로 표시.

**예외**:
- 정말로 "출력이 안정적으로 유지되는가"만 확인하면 충분한 저위험 영역(예: 디버그 전용 로그 포맷)은 전체 구조 snapshot이 실용적이다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-010: UI screenshot test만으로 UX 검증
**분류**: 검증 범위 · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```typescript
test('overlay renderer matches screenshot', async ({ page }) => {
  await page.goto('/viewer?file=sample.hevc&frame=10');
  await page.waitForSelector('[data-testid="mb-type-overlay"]');
  await expect(page).toHaveScreenshot('overlay-mb-type.png');
  // 픽셀이 이전과 같다는 것만 확인 — 키보드 내비게이션, 스크린 리더,
  // 실제 상호작용(hover로 툴팁 표시, 클릭으로 선택) 흐름은
  // 이 테스트 스위트 어디에도 검증되지 않는다
});
```

**문제**:
- 픽셀 단위 스크린샷 비교는 "레이아웃이 우연히 안 바뀌었다"만 보장하지, "사용자가 실제로 이 기능을 쓸 수 있는가"는 전혀 보장하지 않는다.
- 키보드만으로 접근 가능한지(포커스 순서, 단축키), 스크린 리더가 overlay 정보를 어떻게 읽는지, 느린 네트워크/큰 파일에서 로딩 상태가 사용자에게 어떻게 보이는지 같은 실제 UX 품질은 스크린샷에 담기지 않는다.
- 스크린샷 테스트는 폰트 렌더링, OS 버전, GPU 드라이버 차이로 미세하게 흔들려 flaky해지기 쉽고(TEST-008과 유사한 문제), 그 결과 threshold를 느슨하게 잡다 보면 정작 의미 있는 시각적 회귀도 통과시켜 버린다.
- "테스트가 초록불이니 UX도 괜찮다"는 잘못된 확신을 팀에 줄 수 있다 — 특히 이 프로젝트처럼 hex view, timeline, overlay 같은 정보 밀도가 높은 UI에서는 상호작용 흐름 자체가 핵심 가치인데 그 부분이 검증에서 빠진다.

**발생 조건**:
- 시각적 회귀 방지 도구(Playwright, Chromatic 등)를 도입하면서 "이제 UI 테스트는 충분하다"고 판단하고 상호작용/접근성 테스트를 별도로 계획하지 않을 때.
- E2E 테스트 작성 비용을 줄이기 위해 스크린샷 비교가 상호작용 테스트보다 작성하기 쉽다는 이유로 우선될 때.

**권장**:
```typescript
test('overlay tooltip reachable via keyboard and announces MB type', async ({ page }) => {
  await page.goto('/viewer?file=sample.hevc&frame=10');
  await page.keyboard.press('Tab'); // 포커스가 overlay 컨트롤에 도달하는지
  await page.keyboard.press('Enter');
  const tooltip = page.getByRole('tooltip');
  await expect(tooltip).toBeVisible();
  await expect(tooltip).toHaveAccessibleName(/MB type: Intra_4x4/);
});

test('overlay renders correctly under large file loading state', async ({ page }) => {
  await page.goto('/viewer?file=huge_4gb_sample.mp4');
  await expect(page.getByRole('progressbar')).toBeVisible();
  await expect(page.getByTestId('mb-type-overlay')).toBeVisible({ timeout: 30_000 });
});
```
- 스크린샷 테스트는 "레이아웃 회귀 방지"라는 좁은 목적으로 한정하고, 상호작용/접근성/상태 전이는 role/label 기반의 별도 E2E 테스트로 명시적으로 커버한다.
- 접근성 자동 검사 도구(axe-core 등)를 CI에 통합해 스크린샷으로는 드러나지 않는 시맨틱 문제(ARIA 속성 누락, 대비 부족)를 잡는다.

**탐지 방법**:
- Manual: E2E 테스트 스위트를 "스크린샷 비교"와 "상호작용 검증"으로 분류해 후자의 커버리지가 0에 가까운지 감사.
- Structural: 테스트 파일에서 `toHaveScreenshot` 호출 수 대비 `getByRole`/`keyboard.press` 호출 수의 비율을 집계.

**예외**:
- 순수 렌더링 로직(캔버스에 픽셀을 정확히 그리는가)을 검증하는 것이 테스트의 유일한 목적이라면 스크린샷 비교만으로 충분하다 — 상호작용은 그 컴포넌트의 관심사가 아닐 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-011: fuzz crash만 보고 semantic mismatch 무시
**분류**: 퍼징 전략 · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
// fuzz/fuzz_targets/parse_av1.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // panic/OOM/timeout만 신경 쓴다 — Ok(잘못된 결과)는
    // 크래시가 아니므로 fuzzer 관점에서 "성공"으로 취급된다
    let _ = parse_av1(data);
});
```

**문제**:
- libFuzzer/AFL류 coverage-guided fuzzer는 기본적으로 "크래시했는가"만 신호로 사용한다. 파서가 malformed 입력에서 panic 없이 **조용히 잘못된 결과**(잘못된 프레임 개수, 잘못된 MV, 잘못 계산된 크기)를 반환하면 fuzzer는 그것을 흥미로운 입력으로 인식하지 못하고 그냥 지나친다.
- "크래시 0건"이라는 지표가 "파서가 정확하다"는 뜻으로 오해되기 쉽다 — 실제로는 강건성(안 죽는다)과 정확성(맞게 파싱한다)은 별개의 속성이다.
- 이런 semantic mismatch는 사용자에게는 "재생이 되긴 하는데 화면이 깨진다", "메타데이터가 이상하다" 같은 형태로만 나타나므로 재현/원인 규명이 fuzz crash보다 훨씬 어렵다.

**발생 조건**:
- fuzz harness가 파서 함수 호출 자체만 감싸고, 결과값에 대한 어떤 assertion/invariant 체크도 하지 않을 때.
- "fuzzing = 크래시 찾기"라는 좁은 인식이 팀에 있어 harness 설계 시 semantic validation을 고려하지 않을 때.

**권장**:
```rust
fuzz_target!(|data: &[u8]| {
    let Ok(frames) = parse_av1(data) else { return };

    // 크래시하지 않는 것과 별개로, 반환된 결과가 스스로 일관적인지 검증
    for frame in &frames {
        assert!(frame.width > 0 && frame.width <= MAX_SPEC_WIDTH);
        assert!(frame.tile_layout.tile_count() >= 1);
        assert!(frame.mv_field.iter().all(|mv| mv.within_spec_range()));
    }

    // 가능하면 독립 구현(참조 디코더)과의 차분 비교를 fuzz 루프 안에 포함
    if let Some(reference) = try_decode_with_reference(data) {
        assert_frames_consistent(&frames, &reference);
    }
});
```
- fuzz harness 안에 "파싱 결과가 만족해야 하는 불변식"(TEST-013의 property test와 동일한 자산을 재사용 가능)을 assertion으로 심어, semantic bug도 크래시로 전환시킨다 — 이를 통해 fuzzer가 semantic bug를 찾는 입력을 적극적으로 탐색하도록 유도한다.
- 여력이 되면 참조 구현과의 차분 테스트를 fuzz 루프에 결합한 "differential fuzzing"을 구성한다(TEST-015와 연계).

**탐지 방법**:
- Semantic: fuzz harness 코드 리뷰에서 "반환값을 사용하는가, 버리는가(`let _ =`)"를 체크.
- Structural: harness 파일마다 assertion 개수를 집계해 0건인 harness를 식별.

**예외**:
- 초기 단계에서 파서가 아직 크래시를 자주 일으키는 상태라면, 우선 크래시 제거에 집중하고 semantic assertion은 다음 단계로 미루는 것이 합리적인 우선순위 판단일 수 있다 — 단, 이 경우 "다음 단계"가 실제로 계획되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-012: corpus minimization 없음
**분류**: 퍼징 인프라 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// fuzz/corpus/parse_av1/ 안에 18개월간 누적된 47,000개 파일
// - 중복/거의 동일한 커버리지를 내는 입력이 다수
// - 어떤 입력이 어떤 코드 경로를 처음 발견했는지 기록 없음
// - CI에서 "corpus 전체 재실행"에 40분 소요, 그마저도 점점 느려짐
```

**문제**:
- corpus가 커버리지 기여 없이 계속 누적되면 CI의 fuzz 회귀 실행 시간이 선형 이상으로 늘어나, 결국 "시간이 오래 걸리니 corpus 실행을 스킵하자"는 결정으로 이어지기 쉽다(TEST-017과 유사한 압력).
- 거의 동일한 입력이 중복으로 쌓이면 diff 검토(새 크래시 재현 파일이 기존 것과 정말 다른 버그인지)가 어려워지고, 저장소 크기도 불필요하게 커진다.
- minimize되지 않은 크래시 재현 파일은 대체로 크고 노이즈가 많아, 실제 버그와 무관한 바이트까지 포함하고 있어 디버깅 시 "어떤 바이트가 크래시를 유발하는가"를 파악하기 어렵다.

**발생 조건**:
- fuzz 타깃을 CI에 연결한 뒤 corpus 관리(중복 제거, 최소화)를 자동화하지 않고 방치했을 때.
- 크래시 재현 파일을 찾은 그대로(원본 크기 그대로) 회귀 테스트 픽스처로 커밋할 때.

**권장**:
```bash
# CI 정기 job (예: 주간)으로 corpus를 커버리지 기준으로 축소
cargo fuzz cmin parse_av1 \
  --corpus-in fuzz/corpus/parse_av1 \
  --corpus-out fuzz/corpus/parse_av1.min

# 새 크래시를 회귀 테스트로 승격하기 전에 반드시 최소화
cargo fuzz tmin parse_av1 fuzz/artifacts/parse_av1/crash-abcd1234
# 최소화된 재현 파일만 회귀 스위트에 커밋
cp fuzz/artifacts/parse_av1/minimized-from-crash-abcd1234 \
   fixtures/regression/crash_abcd1234_min.bin
```
- corpus minimization(`cargo fuzz cmin`)을 정기 CI job으로 자동화하고, 실행 시간/corpus 크기 추이를 대시보드로 추적한다.
- 새로 발견된 크래시는 `tmin`으로 최소화한 뒤에만 회귀 스위트에 승격해, 회귀 테스트 자체가 가볍고 원인이 명확하게 유지되도록 한다.

**탐지 방법**:
- Structural: corpus 디렉터리 크기/파일 수 추이를 CI 메트릭으로 추적, 임계값 초과 시 경고.
- Runtime: 정기 cmin 실행 후 파일 수 감소율이 낮으면(예: <5%) corpus가 이미 잘 관리되고 있다는 신호로, 반대로 감소율이 크면 오랫동안 minimize가 안 됐다는 신호.

**예외**:
- corpus 크기가 작고(수백 개 이하) CI 실행 시간에 실질적 영향이 없는 초기 단계 fuzz 타깃은 minimization 자동화 우선순위가 낮다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-013: property test invariant가 약함
**분류**: 속성 기반 테스트 · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
proptest! {
    #[test]
    fn test_parse_does_not_panic(data in prop::collection::vec(any::<u8>(), 0..4096)) {
        // "panic만 안 하면 통과" — 이미 catch_unwind 기반 fuzz와 사실상 동일한 검증만 반복
        let _ = parse_av1(&data);
    }
}
```

**문제**:
- "panic하지 않는다"는 강건성 관점에서는 유효하지만 property test의 진짜 가치(입력-출력 관계에 대한 불변식 검증)를 전혀 활용하지 못한다 — TEST-011의 fuzz harness와 본질적으로 같은 약점을 property test 이름만 붙여 반복하는 셈이다.
- invariant가 "패닉하지 않는다" 수준으로 약하면, shrinking(실패를 최소 재현 사례로 축소하는 기능)이 찾아주는 최소 실패 사례도 결국 크래시 재현일 뿐 로직 버그를 드러내지 못한다.
- 랜덤 바이트 시퀀스는 대부분 파서 초반에 reject되므로(예: 매직 넘버 불일치), 실제로 의미 있는 파싱 경로 안쪽까지 도달하는 입력이 거의 생성되지 않아 property test가 사실상 아무것도 탐색하지 못하는 경우가 많다.

**발생 조건**:
- property test를 "fuzz의 손쉬운 대체재"로 도입하면서, strategy를 구조화된 생성기가 아니라 완전 무작위 바이트로만 정의할 때.
- "이 함수가 만족해야 하는 진짜 수학적/논리적 불변식이 무엇인가"를 고민하지 않고 우선 뭔가 property test를 추가하는 데 그칠 때.

**권장**:
```rust
// 구조화된 strategy로 "의미 있는" 입력을 생성하고,
// 도메인 불변식을 명시적으로 검증한다
prop_compose! {
    fn arb_valid_frame_header()(
        width in 1u32..=7680,
        height in 1u32..=4320,
        qindex in 0u8..=255,
        tile_cols_log2 in 0u32..=6,
    ) -> FrameHeader {
        build_synthetic_frame_header(width, height, qindex, tile_cols_log2)
    }
}

proptest! {
    #[test]
    fn test_roundtrip_encode_decode_preserves_dimensions(header in arb_valid_frame_header()) {
        let bitstream = encode_frame_header(&header);
        let parsed = parse_frame_header(&bitstream).unwrap();
        // 진짜 불변식: round-trip이 원본 필드를 보존해야 한다
        prop_assert_eq!(parsed.width, header.width);
        prop_assert_eq!(parsed.height, header.height);
    }

    #[test]
    fn test_tile_layout_covers_frame_exactly(header in arb_valid_frame_header()) {
        let layout = compute_tile_layout(&header);
        // 진짜 불변식: 타일들의 합집합이 프레임 전체를 정확히, 겹침 없이 덮어야 한다
        prop_assert!(layout.tiles_partition_frame_exactly());
    }
}
```
- strategy를 완전 무작위 바이트가 아니라 도메인 구조(유효한 필드 범위, spec 제약)를 반영한 생성기로 정의해, 실제로 의미 있는 파싱 경로를 탐색하게 한다.
- invariant는 "안 죽는다"가 아니라 round-trip 보존, 파티션/커버리지 완전성, 단조성 같은 도메인 고유의 수학적 성질로 정의한다.

**탐지 방법**:
- Semantic: property test 함수 내 `prop_assert*` 호출이 "panic 여부"만 확인하는지, 실제 값에 대한 관계를 검증하는지 코드 리뷰로 판별.
- Runtime: strategy로 생성된 입력 샘플을 로깅해, 파서 초반 검증에서 대부분 reject되는지(= strategy가 얕다는 신호) 통계로 확인.

**예외**:
- 강건성(패닉 없음)이 유일한 관심사인 하위 레벨 유틸리티(예: 순수 bit-reader)에는 "panic 안 함" 수준의 property test도 그 자체로 유효하다 — 다만 그 위 레이어(파서 전체)에서는 더 강한 invariant가 필요하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-014: scalar/SIMD/GPU differential test 없음
**분류**: 이식성·수치 검증 · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
#[test]
fn test_idct_avx2() {
    // AVX2 지원 머신에서만 SIMD 경로를 실행하고 그 결과만 golden과 비교
    #[cfg(target_feature = "avx2")]
    {
        let output = idct_8x8_avx2(&coeffs);
        assert_eq!(output, load_golden("idct_8x8.json"));
    }
    // scalar 경로, 다른 SIMD 폭(SSE4, NEON), GPU 컴퓨트 셰이더 경로는
    // 서로 비교되지 않는다 — 각자 자기 golden만 갖고 있거나 아예 테스트가 없다
}
```

**문제**:
- scalar 구현과 SIMD 구현이 반올림/포화(saturation) 연산 순서 차이로 미묘하게 다른 결과를 낼 수 있는데, 각 경로가 독립된 golden만 비교하면 "두 경로가 서로 일치하는가"는 절대 검증되지 않는다.
- CI 러너(x86_64 AVX2)와 개발자 로컬 머신(Apple Silicon NEON), 사용자의 실제 배포 환경(구형 CPU, SSE4까지만 지원)이 다르면, CI에서 한 번도 실행되지 않은 경로가 프로덕션에만 존재하는 상황이 생긴다.
- GPU 경로(WGPU/컴퓨트 셰이더)는 벤더별 드라이버 차이, 부동소수점 정밀도 차이로 CPU 결과와 미세하게 다를 수 있는데, 이 차이가 "허용 오차 범위인가, 실제 버그인가"를 구분하는 테스트가 없으면 GPU 렌더링 경로의 정확성을 아무도 보증하지 못한다.
- 이런 버그는 특정 하드웨어에서만 재현되므로 리포트를 받아도 재현 자체가 어렵고, "내 컴퓨터에서는 잘 되는데요"의 전형적인 사례가 된다.

**발생 조건**:
- 성능을 위해 SIMD/GPU 경로를 추가하면서 각 경로별로 별도 golden test만 작성하고, 경로 간 교차 검증(cross-validation)을 별도 축으로 설계하지 않았을 때.
- CI 러너가 단일 아키텍처(예: x86_64)만 사용해 ARM NEON 경로가 CI에서 한 번도 컴파일/실행되지 않을 때.

**권장**:
```rust
#[test]
fn test_idct_all_backends_agree() {
    let test_coeffs = generate_diverse_coefficient_sets(); // 경계값 포함 다양한 입력
    for coeffs in &test_coeffs {
        let scalar = idct_8x8_scalar(coeffs);
        let simd = idct_8x8_dispatch(coeffs); // 런타임에 사용 가능한 최상위 SIMD로 디스패치
        // 정수 IDCT는 보통 bit-exact 일치를 요구(spec 준수를 위해)
        assert_eq!(scalar, simd, "scalar/SIMD mismatch for coeffs={coeffs:?}");
    }
}

#[test]
fn test_gpu_pixel_pipeline_matches_cpu_within_tolerance(){
    let frame = load_fixture("valid/1080p_sample.hevc");
    let cpu_output = render_pipeline_cpu(&frame);
    let gpu_output = render_pipeline_gpu(&frame);
    // 부동소수점 경로는 정확한 일치 대신 허용 오차로 비교
    assert_pixel_diff_within_tolerance(&cpu_output, &gpu_output, max_abs_diff: 1);
}
```
- 모든 SIMD/GPU 최적화 경로에 대해, 최적화되지 않은 scalar 구현을 "정답"으로 삼는 differential test를 표준 패턴으로 강제한다 — 새 백엔드를 추가할 때 이 테스트가 자동으로 그 백엔드를 포함하도록 매크로/헬퍼로 일반화한다.
- CI 매트릭스에 아키텍처 축(x86_64 AVX2/SSE4, aarch64 NEON, wasm32)을 추가해, 각 SIMD 경로가 최소 한 번은 실제로 컴파일·실행되게 한다(QEMU 에뮬레이션 포함).
- 정수 연산(IDCT 등 spec이 bit-exact를 요구하는 경우)은 정확히 일치를, 부동소수점 GPU 경로는 명시적 허용 오차 기준을 문서화한 뒤 비교한다.

**탐지 방법**:
- Runtime: CI 매트릭스에 아키텍처별 job을 두고 각 job에서 differential test를 실행, 결과를 아키텍처 태그와 함께 리포트.
- Structural: `#[cfg(target_feature = ...)]`로 분기된 함수 목록을 정적으로 추출해, 대응하는 differential test 존재 여부를 매핑.

**예외**:
- bit-exact 일치가 spec 요구사항이 아니고 애초에 근사 알고리즘(예: 특정 품질 향상 필터)이라면, differential test보다 "결과가 시각적으로 허용 가능한 범위인가"를 검증하는 지각 품질 메트릭(SSIM 등)이 더 적절하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-015: FFmpeg/reference decoder와 비교 없음
**분류**: 검증 무결성 (외부 ground truth) · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
// 프로젝트 전체에 FFmpeg/libavcodec, 코덱 표준화 기구의 reference decoder를
// 호출하거나 비교하는 코드가 단 한 줄도 없다.
// 정확성의 유일한 근거는 "우리 스펙 이해"와 "우리 골든 파일"뿐이다.
#[test]
fn test_decode_matches_our_understanding() {
    let data = load_fixture("valid/sample.av1");
    let result = decode(&data).unwrap();
    assert_eq!(result, load_golden("sample_decoded.json")); // golden도 자체 생성
}
```

**문제**:
- 이것은 TEST-003(self-referential 검증)의 프로젝트 전체 버전이다 — 개별 테스트뿐 아니라 테스트 전략 자체가 외부 ground truth 없이 닫힌 루프를 이룬다.
- 자체 구현이 스펙을 오독한 지점이 있다면, 그 오독이 파서에도 golden 생성 스크립트에도 똑같이 반영되어 있을 가능성이 높다 — 팀 전체가 같은 문서를 같은 방식으로 읽었기 때문이다. 외부 비교 없이는 이런 조직적 오독을 발견할 방법이 원리적으로 없다.
- 상호운용성 문제(우리 도구로 분석한 결과가 업계 표준 도구와 다르게 나옴)는 사용자 신뢰를 직접적으로 깎아먹는데, 이 문제가 릴리스 전에 발견되려면 릴리스 전 비교 파이프라인이 존재해야 한다.
- FFmpeg 자체도 완벽하지 않지만(알려진 버그, 스펙 해석 차이가 존재), "업계에서 가장 널리 검증된 구현과 다른 결과가 나온다"는 사실 자체가 최소한 "우리가 왜 다른가"를 설명할 책임을 지우는 유용한 신호다.

**발생 조건**:
- 프로젝트가 독자적인 파서/디코더를 처음부터 구현하면서, 초기에는 "우리 스펙 이해가 맞다"는 전제 하에 빠르게 기능을 늘리는 데 집중했을 때.
- FFmpeg를 CI 의존성으로 추가하는 것의 빌드/라이선스 복잡도를 피하려고 참조 비교 자체를 스코프에서 제외했을 때.

**권장**:
```rust
// tests/reference_comparison.rs — 별도 CI job으로 분리 가능 (느릴 수 있음)
#[test]
fn test_frame_metadata_matches_ffprobe() {
    let path = fixture_path("valid/sample.av1");
    let ffprobe_json = run_command(&["ffprobe", "-show_frames", "-of", "json", &path]);
    let ffprobe_frames: Vec<FfprobeFrame> = serde_json::from_str(&ffprobe_json).unwrap();

    let our_frames = our_decoder::parse(&std::fs::read(&path).unwrap()).unwrap();

    assert_eq!(our_frames.len(), ffprobe_frames.len());
    for (ours, theirs) in our_frames.iter().zip(&ffprobe_frames) {
        assert_eq!(ours.pict_type, theirs.pict_type);
        assert_eq!(ours.width, theirs.width);
        assert_frame_pixels_match_within_tolerance(ours, theirs); // 픽셀 디코딩까지 검증 시
    }
}

// 코덱 spec conformance vector와의 비교도 같은 카테고리
#[test]
fn test_matches_jvt_conformance_vector_expected_md5() {
    let (bitstream, expected_md5) = load_jvt_conformance_vector("CANL1_Sony_E.264");
    let decoded_yuv = our_decoder::decode_to_yuv(&bitstream).unwrap();
    assert_eq!(md5(&decoded_yuv), expected_md5);
}
```
- CI에 FFmpeg(ffprobe/ffmpeg CLI, 또는 libavcodec FFI 바인딩)를 참조 오라클로 통합하고, 최소한 메타데이터 레벨(프레임 타입, 개수, 해상도, 타임스탬프)은 정기적으로 비교한다.
- 표준화 기구가 배포하는 conformance vector + expected MD5/checksum을 활용해 픽셀 레벨 정확성도 외부 기준으로 검증한다.
- 이 비교는 빌드/실행이 무겁고 느릴 수 있으므로 PR마다가 아니라 nightly/주간 CI job으로 분리해도 되지만, 완전히 생략하지는 않는다.

**탐지 방법**:
- Semantic: 테스트 스위트 전체에서 외부 프로세스 호출(FFmpeg 등) 또는 서드파티 라이브러리 바인딩 사용 여부를 감사 — 전무하면 최우선 개선 대상.
- Structural: CI 워크플로 정의에서 FFmpeg/conformance vector 관련 job 존재 여부를 확인.

**예외**:
- 아직 업계 표준 참조 구현이 없는 신규/실험적 코덱, 또는 non-standard 확장 기능(자체 메타데이터 오버레이 등)은 애초에 비교 대상이 없으므로 이 패턴이 적용되지 않는다 — 이 경우 conformance vector가 나오는 대로 채택한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-016: feature 조합별 CI 없음
**분류**: CI 매트릭스 · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```yaml
# .github/workflows/ci.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --all-features
      # --all-features로만 빌드/테스트 — 사용자가 실제로 쓰는
      # "AV1만", "GPU 렌더링 없이", "특정 코덱 feature 조합 없이"
      # 빌드하는 경우는 CI에서 단 한 번도 컴파일되지 않는다
```

**문제**:
- Rust의 feature flag는 조합 폭발(combinatorial explosion) 문제가 있는데, `--all-features`만 테스트하면 개별 feature를 껐을 때만 발생하는 컴파일 에러(예: `#[cfg(feature = "gpu")]` 블록 안에서만 쓰이는 타입이 다른 feature 조합에서 정의되지 않는 경우)나 링크 에러를 CI가 전혀 잡지 못한다.
- `--all-features`가 통과한다고 해서 `--no-default-features --features av1`처럼 실제 배포되는 최소 구성이 컴파일된다는 보장이 전혀 없다 — 두 feature가 동시에 켜졌을 때만 만족되는 암묵적 가정이 코드에 스며들 수 있다.
- 사용자가 특정 코덱만 필요해서 최소 feature로 빌드하는 배포 시나리오(임베디드, 경량 CLI 도구)가 있다면, 그 조합이 CI에서 한 번도 실행되지 않은 채 릴리스되는 셈이다.

**발생 조건**:
- feature flag가 하나둘 늘어나면서(코덱별, GPU/CPU 렌더링, 플랫폼별) CI 매트릭스를 갱신하지 않고 `--all-features` 한 줄로 계속 때울 때.
- feature 조합 전수 테스트가 조합 폭발로 현실적으로 불가능해지자 아예 테스트를 포기하고 "전부 켠 것 하나"로 타협했을 때.

**권장**:
```yaml
jobs:
  test-feature-matrix:
    strategy:
      matrix:
        features:
          - "--no-default-features --features av1"
          - "--no-default-features --features hevc"
          - "--no-default-features --features avc,hevc"      # 흔한 실사용 조합
          - "--no-default-features --features av1,hevc,avc,vp9,vvc"  # 전체
          - "--all-features"
          - "--no-default-features"                            # 최소 구성도 컴파일되는지
    runs-on: ubuntu-latest
    steps:
      - run: cargo check ${{ matrix.features }}
      - run: cargo test ${{ matrix.features }}
```
- `cargo hack --feature-powerset`(전수 조합은 비용이 크므로 `--depth 2` 등으로 제한) 또는 실사용 조합을 수동으로 큐레이션한 매트릭스로 CI를 구성한다.
- 최소 구성(`--no-default-features`)과 최대 구성(`--all-features`) 양 끝을 반드시 포함하고, 그 사이에 "실제로 배포되는 조합"을 우선순위로 채운다.

**탐지 방법**:
- Structural: `Cargo.toml`의 `[features]` 섹션과 CI 워크플로에 등장하는 `--features` 조합을 대조해 커버되지 않은 조합을 나열하는 스크립트.
- Static: `cargo hack check --feature-powerset`을 로컬/nightly CI에서 실행해 컴파일 실패 조합을 탐지.

**예외**:
- feature가 1~2개뿐이고 서로 독립적(상호작용 없음)이라면 `--all-features`와 `--no-default-features` 두 지점만으로도 충분할 수 있다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-017: 큰 파일 테스트가 CI에서 항상 제외
**분류**: CI 매트릭스·성능 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[test]
#[ignore] // "느려서" 기본 실행에서 제외 — 그리고 아무도 #[ignore]를 켜서 실행하지 않는다
fn test_parse_4gb_multitrack_mkv() {
    let data = load_fixture("valid/4gb_multitrack.mkv");
    let result = parse_container(&data).unwrap();
    assert!(result.tracks.len() > 0);
}
// CI 워크플로 어디에도 `cargo test -- --ignored`를 실행하는 step이 없다
```

**문제**:
- `#[ignore]`로 표시된 테스트가 CI 어느 단계에서도 `--ignored` 플래그로 실행되지 않으면, 그 테스트는 사실상 죽은 코드다 — 존재하지만 아무 가치도 만들지 못한다.
- 대용량 파일에서만 드러나는 문제(스트리밍 파싱 중 메모리 누적, seek table 인덱싱 성능, 32bit 오프셋 오버플로가 실제로 트리거되는 파일 크기)는 작은 픽스처로는 원리적으로 재현 불가능하다.
- "느리니까 제외"가 한 번 관행이 되면, 다음에 추가되는 대용량 테스트도 같은 이유로 자동으로 제외되어 이 커버리지 공백이 구조적으로 고착된다.
- 대용량 파일 처리 회귀(예: 특정 커밋 이후 4GB 파일 파싱이 O(n)에서 O(n²)로 퇴화)가 프로덕션에서야 발견되면 디버깅 비용이 훨씬 크다.

**발생 조건**:
- CI 실행 시간/비용을 줄이려는 압박이 있고, 대용량 픽스처를 리포지토리에 커밋하기 부담스러워(TEST-005와 연계) 아예 로컬에서만 수동 실행하는 관행이 굳어졌을 때.
- "느린 테스트 = 나쁜 테스트"라는 단순화된 원칙이 예외 없이 적용될 때.

**권장**:
```yaml
# 대용량 테스트를 별도 nightly/주간 job으로 분리해 반드시 정기 실행되게 한다
jobs:
  large-file-tests:
    if: github.event_name == 'schedule' || contains(github.event.pull_request.labels.*.name, 'large-file-test')
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - run: ./scripts/fetch_large_fixtures.sh   # 별도 스토리지에서 다운로드, 리포지토리에는 미커밋
      - run: cargo test --release -- --ignored --test-threads=1
```
```rust
#[test]
#[ignore = "large file (4GB); runs in nightly CI, see .github/workflows/large-files.yml"]
fn test_parse_4gb_multitrack_mkv() { /* ... */ }
```
- `#[ignore]`에는 반드시 "왜 무시되는지, 어디서 실제로 실행되는지"를 문자열로 남긴다 — 이유 없는 `#[ignore]`는 그 자체로 리뷰에서 반려한다.
- 대용량 픽스처는 리포지토리에 직접 커밋하지 않고 별도 스토리지(오브젝트 스토리지, LFS)에서 CI가 필요할 때만 받아오게 해 저장소 비대화를 막는다.
- 최소한 nightly/주간 주기로는 반드시 실행되게 스케줄을 걸어, "존재하지만 실행 안 됨" 상태를 방지한다.

**탐지 방법**:
- Structural: `#[ignore]` 태그가 붙은 테스트 목록과 CI 워크플로에서 `--ignored`를 사용하는 job을 대조해, 어느 CI에서도 커버되지 않는 ignored 테스트를 나열.
- Manual: 이유 문자열 없는 `#[ignore]`를 코드 리뷰에서 반려.

**예외**:
- 실제로 더 이상 의미가 없어진(deprecated 기능에 대한) 테스트라면 `#[ignore]`보다 삭제가 맞다 — ignore는 "지금은 못 돌리지만 언젠가 돌려야 한다"는 의도를 표현하는 것이지 삭제의 대체재가 아니다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-018: cancellation·race·stale response 테스트 없음
**분류**: 동시성 테스트 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```typescript
// 프레임 탐색(scrub) 시 이전 요청을 취소하고 최신 요청만 반영해야 하는데
test('seeking to a frame loads and displays it', async () => {
  await seekTo(frameIndex: 100);
  await waitFor(() => expect(getDisplayedFrame()).toBe(100));
  // 사용자가 스크러버를 빠르게 드래그해서 여러 요청이 연달아 발사되고,
  // 늦게 도착한 "이전" 요청 응답이 최신 화면을 덮어쓰는 시나리오는
  // 테스트 스위트에 존재하지 않는다
});
```

**문제**:
- 빠른 연속 탐색(scrub) 시 나중에 보낸 요청보다 먼저 보낸 요청의 응답이 늦게 도착하면(네트워크/디코딩 시간 차이), stale response가 최신 화면을 덮어써 사용자가 요청한 것과 다른 프레임이 표시되는 버그가 생긴다 — 이런 race는 요청을 하나씩 순차적으로 테스트해서는 절대 드러나지 않는다.
- Tauri IPC 커맨드가 취소 가능하도록 설계되어 있어도(`AbortController`, 토큰 기반 취소), 그 취소 로직이 실제로 in-flight 요청을 정리하는지, 취소된 요청의 부작용(예: 캐시에 잘못된 항목을 남기는 것)이 없는지는 명시적인 동시성 시나리오 테스트 없이는 검증되지 않는다.
- 백엔드(Rust)에서 무거운 디코딩 작업이 진행 중일 때 프런트엔드가 다른 프레임을 요청하면, 이전 작업이 실제로 중단되는지 아니면 자원을 계속 소모하며 백그라운드에서 완주하는지(리소스 낭비, 나아가 TEST-019의 메모리 회귀와도 연결)는 별도로 검증해야 하는 속성이다.

**발생 조건**:
- 스크러버/타임라인처럼 사용자가 빠르게 반복 상호작용하는 UI가 있는데, 테스트는 항상 "요청 하나 보내고 응답 기다리기"라는 단순 시나리오로만 작성됐을 때.
- 프런트엔드-백엔드 간 비동기 경계(Tauri invoke, WebWorker, 비동기 디코딩 스레드)가 있는 곳마다 취소/최신성 로직이 필요한데, 이를 명시적으로 설계/테스트하지 않고 "대체로 빠르니까 괜찮겠지"로 넘어갈 때.

**권장**:
```typescript
test('rapid scrubbing only displays the last requested frame', async () => {
  // 응답 지연을 인위적으로 역전시켜 race를 강제로 재현
  mockDecodeLatency({ frame: 50, delayMs: 500 });
  mockDecodeLatency({ frame: 200, delayMs: 10 });

  await seekTo(50);           // 늦게 응답할 요청
  await seekTo(200);          // 빨리 응답할, 진짜 최신 요청

  await waitFor(() => expect(getDisplayedFrame()).toBe(200));
  await sleep(600);           // frame=50 응답이 늦게 도착할 시간을 줌
  expect(getDisplayedFrame()).toBe(200); // 여전히 200이어야 함 — stale이 덮어쓰지 않았는지 확인
});

test('cancelled decode does not leak into frame cache', async () => {
  const decodeSpy = spyOnBackendDecode();
  const controller = seekTo(50);
  controller.cancel();
  await seekTo(200);
  expect(getFrameCache().has(50)).toBe(false); // 취소된 작업의 부작용이 남지 않았는지
});
```
- 비동기 요청마다 "요청 ID/세대(generation) 번호"를 붙이고, 응답 처리 시 최신 세대가 아니면 무시하는 로직을 명시적으로 테스트한다.
- 응답 지연을 인위적으로 제어(mock latency, 지연 역전)할 수 있는 테스트 유틸리티를 갖춰, 실제 타이밍에 의존하지 않고 결정적으로 race를 재현한다.
- 취소가 실제로 백엔드 리소스(진행 중인 디코딩 스레드, 캐시 엔트리)를 정리하는지까지 검증 범위에 포함한다.

**탐지 방법**:
- Runtime: 지연 역전 mock을 이용한 결정적 race 시나리오 테스트를 CI 필수 스위트에 포함.
- Manual: 비동기 경계(IPC invoke, 이벤트 리스너)가 있는 컴포넌트마다 "빠른 연속 요청 시나리오"가 테스트 계획에 있는지 리뷰 체크리스트로 확인.

**예외**:
- 요청이 항상 순차적으로 하나씩만 발생하도록 UI 레벨에서 이미 직렬화되어 있는 경우(예: 버튼이 요청 중 비활성화됨)라면 race 자체가 발생할 수 없으므로 이 테스트의 우선순위가 낮다 — 단, 그 직렬화 보장 자체는 테스트되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-019: memory budget regression 테스트 없음
**분류**: 리소스 회귀 · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
#[test]
fn test_parse_large_file_completes() {
    let data = load_fixture("valid/2gb_sample.mp4");
    let result = parse_container(&data);
    assert!(result.is_ok());
    // 파싱이 "끝났다"만 확인 — 그 과정에서 피크 메모리 사용량이
    // 파일 크기의 몇 배였는지는 아무도 측정하지 않는다
}
```

**문제**:
- 기능적으로 정확한 결과를 반환해도 그 과정에서 파일 크기에 비례하지 않는 메모리를 사용한다면(예: 전체 파일을 메모리에 통으로 올리고 추가로 파싱 트리를 중복 보관), 대용량 파일에서 OOM으로 이어질 수 있는데 이 문제는 "테스트 통과 여부"만으로는 전혀 드러나지 않는다.
- 메모리 사용량은 최적화/리팩터링 과정에서 조용히 퇴화하기 쉬운 지표다 — 기능 테스트는 계속 통과하므로 "메모리 사용량이 3배로 늘었다"는 사실이 벤치마크 없이는 몇 달간 아무도 눈치채지 못한 채 누적될 수 있다.
- 스트리밍 파싱을 지향하는 아키텍처(전체 파일을 한 번에 읽지 않고 청크 단위로 처리)라면 "메모리가 파일 크기에 비례하지 않고 상한선 안에 머무른다"는 것 자체가 핵심 설계 목표인데, 이를 검증하는 회귀 테스트가 없으면 그 설계 목표가 코드 변경 중 조용히 깨져도 알 수 없다.

**발생 조건**:
- 성능(속도) 벤치마크는 갖춰져 있지만 메모리 사용량 추적은 "필요하면 프로파일러로 수동 확인" 수준에 머물러 있을 때.
- 초기 구현이 작은 파일 위주로 검증되어 "메모리가 문제 될 정도로 큰 파일"을 아직 마주치지 않았을 때(TEST-017과 연결되는 구조적 공백).

**권장**:
```rust
#[test]
fn test_memory_budget_scales_with_streaming_not_file_size() {
    let data = load_fixture("valid/2gb_sample.mp4"); // 스트리밍 소스로 공급
    let peak = measure_peak_rss(|| {
        parse_container_streaming(StreamingReader::new(&data)).unwrap()
    });
    // 파일 크기(2GB)가 아니라 설계상의 버퍼 상한(예: 64MB)에 비례해야 한다
    assert!(peak < 128 * 1024 * 1024, "peak RSS {peak} exceeds 128MB budget for 2GB input");
}

#[test]
fn test_memory_does_not_regress_across_versions() {
    // 벤치마크 결과를 기준선(baseline.json)과 비교, CI에서 임계치 초과 시 실패
    let peak = measure_peak_rss(|| parse_container(&load_fixture("valid/1080p_sample.mp4")).unwrap());
    let baseline = load_memory_baseline("parse_container_1080p");
    assert!(peak <= baseline.max_rss_bytes * 110 / 100, "10% memory regression: {peak} vs baseline {}", baseline.max_rss_bytes);
}
```
- 대표 워크로드에 대해 피크 RSS(또는 heap 프로파일러의 peak allocated)를 측정하는 테스트를 마련하고, 파일 크기 대비 메모리 사용량이 설계 목표(예: O(1) 또는 O(작은 상수))를 벗어나지 않는지 명시적으로 assert한다.
- CI에서 메모리 사용량을 기준선과 비교해 일정 비율(예: 10%) 이상 증가하면 실패하는 회귀 게이트를 둔다 — 절대값보다 "직전 기준선 대비 증가율"이 관리하기 쉽다.
- valgrind massif, heaptrack, 또는 Rust의 `dhat` 같은 도구로 nightly job에서 상세 프로파일을 남겨 회귀 원인 추적을 쉽게 한다.

**탐지 방법**:
- Runtime: CI에 RSS/heap 측정 스텝을 추가하고 기준선 파일과 비교하는 게이트를 둔다.
- Structural: 대용량 파일을 다루는 진입점(파서, 디코더)마다 memory budget 테스트 존재 여부를 매핑해 공백을 식별.

**예외**:
- 메모리 사용량이 설계상 파일 크기에 비례하는 것이 의도된 컴포넌트(예: 전체 파일을 랜덤 액세스해야 하는 non-streaming 분석 도구)라면 상한선 테스트 대신 "비례 계수가 합리적인 범위인가"를 검증하는 것이 더 적절하다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움

---

### TEST-020: benchmark 실패가 CI를 막지 않음
**분류**: 성능 회귀 게이트 · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```yaml
# .github/workflows/bench.yml
jobs:
  benchmark:
    runs-on: ubuntu-latest
    continue-on-error: true   # 벤치마크 job이 실패해도 워크플로 전체는 초록불
    steps:
      - run: cargo bench --bench decode_bench
      # 결과를 어딘가에 게시는 하지만, 이전 기준선과 비교해서
      # 실패시키는 로직이 없다 — 그래프만 쌓이고 아무도 정기적으로 보지 않는다
```

**문제**:
- `continue-on-error: true`나 "결과 게시만 하고 비교/게이팅 없음" 조합은 벤치마크를 사실상 장식으로 만든다 — 30% 성능 퇴화가 있는 PR도 초록불로 머지된다.
- 성능 회귀는 기능 회귀와 달리 사용자 리포트로도 늦게 발견된다("어제보다 느린 것 같은데 확신은 없다" 수준의 모호한 체감으로만 나타나다가, 여러 PR의 작은 퇴화가 누적된 뒤에야 뚜렷해진다) — 그래서 오히려 자동 게이트의 필요성이 기능 테스트보다 크다.
- 벤치마크가 CI를 막지 못한다는 사실이 알려지면, 팀 내에서 "벤치마크는 참고용"이라는 암묵적 합의가 생기고 결국 벤치마크 코드 자체가 방치되어 컴파일도 안 되는 상태로 썩는(bit-rot) 경우가 흔하다.
- 벤치마크 환경(CI 러너)이 noisy(공유 인스턴스, variable CPU throttling)하면 게이팅을 걸었을 때 flaky 실패가 잦아, TEST-008과 같은 이유로 다시 무시당하는 악순환에 빠지기 쉽다 — 그래서 "게이팅을 안 건다"가 아니라 "노이즈에 강인한 게이팅 방법"이 필요하다.

**발생 조건**:
- 벤치마크를 도입한 초기에는 CI를 막을 만큼 결과가 안정적이지 않아 일단 `continue-on-error`로 시작했는데, 이후 안정화 작업이 후순위로 밀려 영구화될 때.
- 성능이 중요한 프로젝트 초기 단계에서 "일단 기능부터"라는 우선순위 판단이 반복되며 성능 게이트 구축이 계속 미뤄질 때.

**권장**:
```yaml
jobs:
  benchmark:
    runs-on: [self-hosted, dedicated-bench]  # 공유 러너보다 노이즈가 적은 환경
    steps:
      - run: cargo bench --bench decode_bench -- --save-baseline pr
      - name: compare against main baseline
        run: |
          critcmp main pr --threshold 10 || {
            echo "::error::Performance regression exceeds 10% threshold";
            exit 1;
          }
```
```rust
// criterion 기반 벤치마크 + statistical significance 고려한 임계치
fn bench_decode_1080p(c: &mut Criterion) {
    c.bench_function("decode_1080p_hevc", |b| {
        b.iter(|| decode(black_box(&fixture)))
    });
}
```
- `criterion`(통계적 유의성 검정 내장) + `critcmp` 같은 도구로 PR 브랜치와 main 기준선을 비교하고, 노이즈를 고려한 임계치(예: p<0.05 이면서 10% 이상 퇴화)를 넘을 때만 실패시켜 flaky 게이팅을 피한다.
- 노이즈가 큰 공유 CI 러너 대신 전용/고정 스펙 러너를 사용하거나, 여러 번 반복 실행 후 중앙값으로 비교해 변동성을 낮춘다.
- 벤치마크 게이트가 처음부터 모든 PR을 막을 필요는 없다 — 우선 결과를 축적해 노이즈 특성을 파악한 뒤, 임계치를 신중히 정하고 나서 게이팅을 활성화하는 단계적 롤아웃이 현실적이다. 단, "언젠가 게이팅한다"는 계획이 실제로 실행되어야 한다.

**탐지 방법**:
- Structural: CI 워크플로에서 벤치마크 job에 `continue-on-error`가 붙어 있는지, 결과 비교/실패 조건이 존재하는지 정적으로 점검.
- Manual: 벤치마크 대시보드/그래프가 실제로 정기 리뷰되는지(마지막으로 언제 참조되었는지) 주기적으로 확인 — 아무도 안 본다면 게이트가 없는 것과 동등하다.

**예외**:
- 벤치마크 도입 극초기, 아직 기준선의 변동 폭(노이즈)조차 파악되지 않은 단계에서는 일시적으로 non-blocking으로 운영하며 데이터를 축적하는 것이 합리적이다 — 단, 이는 "영구 상태"가 아니라 "게이팅 활성화 이전의 준비 단계"로 명시적으로 취급되어야 한다.

**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움
