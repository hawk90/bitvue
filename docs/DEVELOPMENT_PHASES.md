# Bitvue — Development Phases & Roadmap

> Extracted from `VQA_PARITY_SPEC_V3.md` §5 "Development Phases" (2026-07-31 doc-family split). Phase-by-phase
> implementation roadmap for closing the VQ Analyzer parity gap — Rust code sketches, task checklists, and time
> estimates per phase. This is roadmap/planning material, not current-state spec: for "what a feature should do"
> see `VQA_PARITY_SPEC_V3.md`, for "is it done yet" see `PARITY_CHECKLIST.md`.
> See also: `VQA_PARITY_SPEC_V3.md` (backend/codec parity spec), `PARITY_CHECKLIST.md` (implementation tracking),
> `COMPETITOR_FEATURE_MATRIX.md` (per-product feature matrix), `UX_PARITY_MATRIX.md` (UI/UX interaction parity).

---

## Phase 0: Project Setup & Foundation ✅ (이미 완료)

**현재 상태:** Bitvue v0.12.0에서 대부분 완료됨

- [x] Tauri + Rust + React TypeScript 프로젝트 구조
- [x] Cargo workspace with 32 crates
- [x] 기본 Tauri 커맨드 인터페이스 (23개)
- [x] React 컨텍스트 + 훅 아키텍처
- [x] DockableLayout 패널 시스템
- [x] 크로스플랫폼 빌드 (macOS/Windows/Linux)
- [x] CI/CD 파이프라인 기초
- [ ] MXF 컨테이너 지원
- [ ] AVI 컨테이너 지원

**추가 해야 할 일:**
- [ ] vvdec-sys 바인딩 구현 및 VVC 디코딩 연결
- [ ] openavs3d 또는 FFmpeg AVS3 디코딩 연결
- [ ] MPEG-2 디코딩 FFmpeg 연결

**예상 소요:** 2~3주 (신규), 현재 이미 완료

---

## Phase 1: 코덱별 F키 모드 분기 시스템 🔴 (핵심 갭)

**목표:** 현재 고정 6모드 → 코덱별 동적 모드 시스템으로 교체

- [ ] `CodecModeRegistry` 구조체 설계 (코덱 → 가능한 모드 목록)
- [ ] 각 코덱별 F키 → 모드 이름 매핑 테이블 구현
- [ ] 프론트엔드 Mode 메뉴 동적 생성 (파일 로드 후 코덱 감지 → 메뉴 갱신)
- [ ] 툴바의 F키 버튼 동적 표시/숨기기
- [ ] `get_available_modes` Tauri 커맨드 추가

```rust
// 설계 예시
pub enum AnalysisMode {
    // 공통
    YuvDirect,
    CodingFlow,
    // HEVC 전용
    HevcSao,
    // VVC 전용
    VvcDualTree,
    VvcInverseMap,
    VvcAdaptiveFilter,
    // AV1 전용
    Av1CdefFilter,
    Av1SuperRes,
    Av1LoopRestoration,
    Av1FilmGrain,
    // AVS3 전용
    Avs3Esao,
    Avs3Ccsao,
    // JPEG XS 전용
    JpegXsPrecinct,
    JpegXsNlt,
    // ...
}

pub struct CodecModeRegistry {
    pub modes: HashMap<CodecType, Vec<(u8, AnalysisMode, &'static str)>>, // (f_key, mode, label)
}
```

**Tauri 커맨드:**
```typescript
// 신규 커맨드
get_codec_modes(codec: string) -> Vec<{f_key: number, mode: string, label: string}>
```

**Parity 검증:**
- 각 코덱 파일 로드 후 Mode 메뉴에 올바른 항목만 표시되는지 확인
- F키 눌렀을 때 올바른 오버레이 렌더러 활성화

**예상 소요:** 중급 1~2주

---

## Phase 2: 코덱별 Info Overlay 토글 시스템 🔴

**목표:** QP Map, Heat Map, MV Heat 등 오버레이를 코덱별로 사용 가능/불가능 처리

- [ ] `InfoOverlayCapabilities` 코덱별 매트릭스 구현
- [ ] View/Mode 메뉴의 Info Overlays 하위 메뉴 동적 활성화/비활성화
- [ ] 다중 오버레이 동시 표시 지원 (예: CodingFlow + QP Map 중첩)
- [ ] 각 오버레이별 토글 상태 저장 (Options에서 유지)

**구현할 오버레이 우선순위:**

| 오버레이 | 코덱 지원 | Bitvue 현황 | 작업 |
|---------|---------|-----------|-----|
| QP Map | HEVC, VVC, AVC, MPEG-2 | ✅ QPMapRenderer | 코덱 분기 추가 |
| Heat Map | 전체 | ✅ TransformRenderer 일부 | 전용 렌더러 완성 |
| MV Heat | HEVC | ⚠️ MVFieldRenderer | HEVC 전용 강도 맵 완성 |
| PU Type | HEVC | ⚠️ PredictionRenderer | HEVC PU 타입별 색상 완성 |
| PU Reference Indices | HEVC | ⚠️ | 참조 인덱스 색상 맵 |
| MB Type | AVC | ❌ | 신규 구현 |
| MB Reference Indices | AVC | ❌ | 신규 구현 |
| Block Type | AV1, VP9 | ⚠️ | 블록 타입 색상 맵 |
| Efficiency Map | AV1, VP9 | ⚠️ | 면적당 비트 계산 |
| PSNR Overlay | 전체 | ⚠️ | Debug YUV 필요 |
| SSIM Overlay | HEVC | ⚠️ | Debug YUV 필요 |
| Inter Memory Reads | VVC | ❌ | 신규 구현 |
| Simple Motion | 전체 | ⚠️ | 단순화 MV 화살표 |

**Parity 검증:**
- 각 오버레이의 색상 스케일이 VQ Analyzer와 일치
- QP Map: jet colormap (blue→green→yellow→red) 범위 정확도
- 오버레이 중첩 시 투명도 처리

**예상 소요:** 중급 3~4주

---

## Phase 3: VVC 전용 기능 완성 🟡

**목표:** VVC 파싱 → 디코딩 → 전용 모드 완전 구현

- [ ] vvdec-sys Rust 바인딩 구현 (`vvdec` C API 래핑)
- [ ] VVC 디코딩 파이프라인 연결 (`bitvue-decode/src/decoder.rs`)
- [ ] **Dual Tree 모드 렌더러** 구현:
  - 루마 파티션 트리 (파란색 경계)
  - 크로마 파티션 트리 (빨간색 경계)
  - 루마/크로마 중첩 표시 모드
- [ ] **Inverse Map 모드** (LMCS 역 매핑 시각화):
  - 루마 픽셀 값 → 매핑 함수 → 변환된 값 표시
  - 색상: 밝기 변화량에 따른 히트맵
- [ ] **Adaptive Filter (ALF) 모드**:
  - ALF 필터 사용 블록 표시 (파랑 = ALF 적용, 회색 = 미적용)
  - ALF 파라미터 (필터 계수) 선택 정보에 표시
- [ ] **Inter Memory Reads** 오버레이:
  - inter 예측 시 참조 메모리 접근 패턴 시각화
  - 접근 빈도 히트맵
- [ ] VVC Syntax 탭 완성 (APS 탭 추가)

**핵심 구현 과제 — vvdec 연결:**
```rust
// src-tauri/src/decode/vvc_decoder.rs
pub struct VvcDecoder {
    ctx: *mut vvdec_sys::vvdecDecoder,
}

impl VvcDecoder {
    pub fn decode_frame(&mut self, nal_data: &[u8]) -> Result<YuvFrame> {
        // vvdec_decode() 호출 → VvcYUVBuffer 변환
    }
}
```

**Parity 검증:**
- Dual Tree: 루마/크로마 경계가 VQ Analyzer와 pixel-perfect 매칭
- LMCS 적용된 시퀀스에서 Inverse Map 시각화 확인

**예상 소요:** 중급 4~6주 (vvdec 연결 포함)

---

## Phase 4: AV1 전용 고급 모드 완성 🟡

**목표:** AV1 전용 F6~F9 모드 구현 (CDEF, SuperRes, Loop Restoration, Film Grain)

- [ ] **CDEF Filter 모드**:
  - CDEF 방향 화살표 (8방향 × 강도)
  - 적용 블록 하이라이트
  - CDEF 강도(primary_strength, secondary_strength) 색상 스케일
- [ ] **SuperRes Filter 모드**:
  - 다운스케일된 영역 vs 업스케일된 영역 구분 표시
  - denom 파라미터 시각화
- [ ] **Loop Restoration 모드**:
  - Wiener 필터 적용 블록 (파랑)
  - Self-guided 필터 적용 블록 (초록)
  - 미적용 블록 (회색)
  - 필터 파라미터 selection info에 표시
- [ ] **Film Grain 모드**:
  - 그레인 합성 전 픽셀 표시
  - 그레인 합성 후 픽셀 표시
  - 두 뷰 나란히 또는 오버레이로 비교
  - film_grain_params selection info에 표시
- [ ] AV1 Efficiency Map 완성:
  - 각 블록의 비트수 / 블록 면적 계산
  - jet colormap으로 시각화
- [ ] AV1 Block Type 오버레이 완성:
  - INTRA, INTER, INTRA_BC 색상 구분
  - 복합 예측 모드 별도 색상

**AV1 파싱 보강 필요:**
```rust
// bitvue-av1-codec/src/advanced_features.rs
pub struct FilmGrainParams {
    pub apply_grain: bool,
    pub grain_seed: u16,
    pub num_y_points: u8,
    pub point_y_value: [u8; 14],
    pub point_y_scaling: [u8; 14],
    // ...
}

pub struct CdefParams {
    pub cdef_damping_minus_3: u8,
    pub cdef_bits: u8,
    pub cdef_y_pri_strength: [u8; 8],
    pub cdef_y_sec_strength: [u8; 8],
    // per-unit direction and variance
}
```

**Parity 검증:**
- CDEF 방향 화살표가 실제 CDEF 방향 결정과 일치
- Film Grain 전/후 픽셀 값이 dav1d 출력과 일치

**예상 소요:** 중급 3~4주

---

## Phase 5: AVS3 지원 구현 🔴 (신규)

**목표:** AVS3 코덱 전체 파싱 + 디코딩 + 전용 모드

- [ ] `bitvue-avs3-codec` 크레이트 신규 생성
- [ ] AVS3 비트스트림 구조 파싱:
  - 시작 코드 (AVS3 prefix: 0x000001xx)
  - Sequence Header, Picture Header 파싱
  - CTU/CU 분할 구조 (HEVC 유사)
- [ ] AVS3 엔트로피 코딩: CABAC 변형 (AEC — Adaptive Entropy Coding)
- [ ] **ESAO 모드 렌더러** (Enhanced SAO, AVS3 전용):
  - ESAO 타입/파라미터 시각화
- [ ] **CCSAO 모드 렌더러** (Cross-Component SAO, AVS3 전용):
  - 크로스 컴포넌트 필터 적용 블록 시각화
- [ ] openavs3d 또는 FFmpeg 기반 AVS3 디코딩
- [ ] AVS3 Syntax 탭 구현

**레퍼런스:**
- AVS 표준 문서: http://www.avs.org.cn/
- openavs3d 오픈소스 구현

**예상 소요:** 중급 6~8주

---

## Phase 6: JPEG XS + VC-3 + APV 지원 🟢 (Nice-to-have)

#### JPEG XS
- [ ] `bitvue-jpegxs-codec` 크레이트 신규 생성
- [ ] JPEG XS 마커 파싱 (SOC, SLH, SLI, SLD, EOC)
- [ ] Precinct 구조 파싱
- [ ] 웨이블릿 계수 추출
- [ ] NLT, MCT 파라미터 파싱
- [ ] 전용 F1~F6 모드 렌더러

#### VC-3 / DNxHD
- [ ] `bitvue-vc3-codec` 크레이트 신규 생성
- [ ] DNxHD 세그먼트 구조 파싱
- [ ] FFmpeg 기반 디코딩

#### APV (Apple ProRes Video)
- [ ] APV 비트스트림 파싱
- [ ] FFmpeg ProRes 디코딩

**예상 소요:** 각각 중급 4~6주

---

## Phase 7: YUVDiff 모드 완성 🟡

**목표:** Debug YUV 비교 기능을 VQ Analyzer 수준으로 완성

- [ ] YUV 파일 로더 강화:
  - 자동 포맷 감지 (파일 크기 / 해상도 기반)
  - Planar (YUV420p, YUV422p, YUV444p) 지원
  - Semi-planar (NV12, NV21) 지원
  - 비트뎁스 (8/10/12/16bit) 지원
- [ ] 픽셀 단위 차이 계산 엔진 (Rust, SIMD 가속):
  ```rust
  pub fn compute_diff_frame(
      decoded: &YuvFrame,
      reference: &YuvFrame,
      amplification: u8,
  ) -> YuvFrame
  ```
- [ ] YUVDiff UI 완성:
  - [Decoded / Debug YUV / Difference / Amplified] 전환 버튼
  - PSNR/SSIM 실시간 표시
  - 차이 증폭 슬라이더 (×1 ~ ×64)
  - 첫 불일치 프레임 탐색 버튼
- [ ] 픽처 오프셋 조정 다이얼로그
- [ ] 크롭 설정 다이얼로그 (L/R/T/B 픽셀)
- [ ] Display Order vs Coding Order 전환
- [ ] Auto-reload 감시 (inotify/FSEvents 기반)

**Tauri 커맨드 추가:**
```typescript
load_debug_yuv(path: string, format: YuvFormat, bitdepth: number) -> Result<void>
get_yuv_diff_frame(frame_index: number, amplification: number) -> Result<FrameData>
get_yuv_psnr(frame_index: number) -> Result<PsnrResult>
find_first_diff_frame() -> Result<number>  // 첫 불일치 프레임 인덱스
```

**Parity 검증:**
- PSNR 값이 `ffmpeg -i ref.yuv -i decoded.yuv -lavfi psnr` 결과와 ±0.01dB 일치
- 차이 프레임 시각화가 VQ Analyzer의 색상 표현과 일치

**예상 소요:** 중급 2~3주

---

## Phase 7.5: Dual-Stream Compare & VMAF 🔴 (신규 — VQA_PARITY_SPEC_V3.md §4.9, §1.5)

**목표:** VQ Probe/StreamEye 패리티의 핵심 갭인 "두 비트스트림 비교" 워크플로우와 VMAF 품질 지표 추가.

**⚠️ 아키텍처 경계 (필수 준수):** Bitvue는 VQ Analyzer 성격(비트스트림 구조/코덱 상태 분석)과 VQ Probe 성격(화질
비교/정렬/메트릭 분석)을 **동시에** 만드는 프로젝트다. 이 둘은 실패 모드가 다르다 — Analyzer 쪽은 파서 안전성·syntax
표현 폭증·codec state가 핵심 위험이고, Probe 쪽은 프레임 정렬·색공간 정합성·중복 decode·메트릭 정확성이 핵심 위험이다.
**절대 하나의 거대 구조체로 합치지 말 것**:
```rust
// 안티패턴 — 하지 말 것
struct UniversalFrameAnalysis {
    syntax: Option<SyntaxTree>,
    motion_vectors: Option<Vec<MotionVector>>,
    qp_map: Option<QpMap>,
    psnr: Option<f64>, ssim: Option<f64>, vmaf: Option<f64>,
    heatmap: Option<Heatmap>,
    reference_frame: Option<FrameId>, distorted_frame: Option<FrameId>,
}
```
공유해야 하는 것은 **미디어 입력과 실행 인프라**(Demuxer, Decoder abstraction, PixelFormat, ColorMetadata,
FrameBuffer/Pool, Timeline, Job scheduler, Cache budget)이지 **분석 결과 도메인 자체가 아니다**. 이미 존재하는
`crates/bitvue-core/src/{compare,alignment,...}.rs`가 비트스트림 분석 코드와 섞이지 않고 독립 모듈로 남아있는지
구현 착수 전 확인할 것. `docs/anti-patterns/`(작성 중)의 카탈로그도 이 경계를 그대로 따라 BIT-*(Analyzer 전용)와
VQ-*(Probe 전용)를 분리된 하위 카탈로그로 유지한다 — 문서 구조와 실제 코드 구조가 어긋나면 이 원칙이 무의미해짐.

- [ ] Stream B 로딩 인프라 완성 (Phase 3 진행 상황 메모 P1-4와 통합)
- [ ] Compare 모드 UI: Side-by-side / Split(H/V) / Subtraction / Temperature 전환 버튼
- [ ] RD-curve 패널에 BD-rate 계산 추가
- [ ] VMAF 통합: `libvmaf-sys` 연결 (이미 optional feature로 존재 — spec §1.2), pooled score + per-frame score + ADM2/VIF/motion2 서브스코어
- [ ] "Find First Difference" (두 스트림 간)
- [ ] CLI: `bitvue compare --stream-a --stream-b --vmaf` 서브커맨드

**Parity 검증:** `VQA_PARITY_SPEC_V3.md` §4.9 참조.

**현황 정정 (2026-07-31, `docs/_import_v14` 마이닝 중 grep으로 발견):** `PARITY_CHECKLIST.md` Layer 6은
CMP-01/02를 `[ ]`(미시작)으로 표시하지만 실제로는 이미 부분 구현되어 있음 — `crates/bitvue-core/src/{compare,alignment,compare_cache,compare_evidence,compare_strategy}.rs`,
`src-tauri/src/commands/compare.rs`(`create_compare_workspace`/`get_aligned_frame`/`set_sync_mode`/`set_manual_offset`/`reset_offset`,
전부 `lib.rs`에 등록됨), `frontend/components/CompareWorkspace/CompareWorkspace.tsx`(side-by-side 렌더링 확인)가
이미 존재. Split(H/V)·Subtraction/Temperature 뷰는 미확인. `PARITY_CHECKLIST.md` Layer 6에 `[-]`로 정정함 — 아래 참조.

**Diff Heatmap 구현 상세 (from `_import_v14/.../visualization/DIFF_HEATMAP_IMPLEMENTATION_SPEC.md`, CMP-03 대상):**

| 항목 | 값 |
|---|---|
| 입력 | 해상도/색공간 정렬된 프레임 A/B (luma 또는 RGBA→luma 변환), 선택적 블록 단위 메트릭 델타 맵 |
| 모드 (UI 토글) | `abs`(\|lumaA-lumaB\|, 기본값) / `signed`(lumaA-lumaB) / `metric`(블록별 메트릭 델타) |
| 텍스처 생성 | QP 히트맵과 동일 규칙으로 기본 Half-res; abs 모드는 4단계 램프, diff=0은 완전 투명; alpha는 diff에 비례, 최대 `180 * user_opacity`로 클램프 |
| 캐시 키 | `overlay_diff:<codec>:<filehashA>:<filehashB>:f<frame>\|hm<hw>x<hh>\|mode<abs\|signed\|metric>\|op<bucket>` |
| 인터랙션 | hover 시 픽셀/블록 diff 값 표시; 클릭 시 툴팁 고정(ESC로 해제) |
| 승인 테스트 | diff=0 영역 완전 투명 / opacity만 바뀌면 캐시 재사용 / 모드 전환 시에만 텍스처 재생성 |

이미 존재하는 `crates/bitvue-core/src/diff_heatmap.rs`(YUVDiff §4.7용)를 Compare A/B 컨텍스트로 재사용 가능한지 확인 필요 —
현재 diff_heatmap.rs가 단일 스트림 YUVDiff용인지 A/B 두 스트림용인지 구현 시 확인.

**예상 소요:** 중급 3~4주

---

## Phase 8: Syntax 패널 완성 (코덱별 탭 상세화) 🟡

**목표:** 각 코덱의 Syntax 탭을 VQ Analyzer 수준으로 완성

- [ ] **Stats 탭 완성**:
  - 파이 차트: 프레임 타입 분포, CU 크기 분포, 인트라/인터 비율
  - 바 차트: 프레임별 크기, QP 분포
  - 스트림 통계 테이블
- [ ] **신택스 트리 네비게이션**:
  - 트리 노드 펼치기/접기
  - 노드 클릭 → HEX View에서 비트 오프셋 강조
  - HEX View 바이트 클릭 → 신택스 트리 해당 요소 포커스
- [ ] **HEVC QM 탭**:
  - 양자화 매트릭스 테이블 (4x4 ~ 32x32, 색상 시각화)
- [ ] **VP9 Probabilities/Counts 탭**:
  - 컨텍스트 확률 테이블 시각화
  - 프레임별 업데이트 이력
- [ ] **VVC APS 탭**:
  - ALF APS, LMCS APS, Scaling List APS 분리 표시
- [ ] **모든 코덱 Ref Lists 탭**:
  - L0/L1 리스트, POC, 가중치/오프셋, long-term 여부

**예상 소요:** 중급 3~4주

---

## Phase 9: 커맨드라인 인터페이스 (CLI) 강화 🟡

**목표:** bitvue-cli가 VQ Analyzer CLI와 동일한 기능 제공

- [ ] `-hevc`, `-vp9`, `-av1`, `-avc`, `-mpeg2`, `-vvc`, `-avs3` 강제 코덱 플래그
- [ ] `-o <file>` 출력 YUV 파일명
- [ ] `-y4m` Y4M 포맷 출력
- [ ] `-dump` 디코딩된 YUV 덤프
- [ ] `-dump_bitdepth <N>` 출력 비트뎁스 설정
- [ ] `-regress` 헤드리스 디코딩 (GUI 없음)
- [ ] `-frames <N>` 처리할 프레임 수
- [ ] `-fast <0|1|2>` 성능 모드
- [ ] `-md5` 프레임별 MD5 체크섬
- [ ] `-psnr` PSNR 계산
- [ ] `-errors` 에러 로그 파일
- [ ] `-stats` 통계 출력
- [ ] `-stream_stats` HEVC 스트림 통계
- [ ] `-display_order` 표시 순서 YUV 출력
- [ ] `-nocrop` 크롭 비활성화
- [ ] `-cpu_max_feature <feature>` CPU 최적화 선택
- [ ] `-film_grain` 필름 그레인 전/후 별도 출력 (AV1)

**예상 소요:** 중급 1~2주

---

## Phase 10: 성능 최적화 & 대용량 스트림 지원 🟡

**목표:** 4K/8K 스트림, 1GB+ 파일 안정적 처리

- [ ] **메모리 맵 기반 스트리밍** (`memmap2`):
  - 파일 전체를 메모리에 로드하지 않음
  - 청크 단위 탐색
- [ ] **썸네일 지연 생성**:
  - 뷰포트에 보이는 프레임만 디코딩
  - LRU 캐시 (최대 N개 썸네일 유지)
- [ ] **오버레이 WebGL 렌더러** (Canvas 대체):
  - 수천 개 MV 벡터 60fps 렌더링
  - GPU 가속 히트맵
- [ ] **Syntax 트리 가상화**:
  - 수천 개 노드 → 뷰포트만 렌더링
- [ ] **병렬 파싱** (rayon):
  - 타일 병렬 파싱 (AV1)
  - 프레임 병렬 메트릭 계산
- [ ] **프로파일링**:
  - Criterion 벤치마크로 핫스팟 식별
  - SIMD 최적화 (YUV→RGB, PSNR 계산)

**추가 구체안 (from `_import_v14/.../performance/*.md`, 2026-07-31 마이닝 — Bitvue는 egui가 아닌 Tauri/React라 "paint_ms/egui" 언급은 프레임워크 불일치 주의, 개념·수치만 차용):**

| 항목 | 구체 값 |
|---|---|
| 캐시 레벨 (레퍼런스 예산, 스트림당) | Decode cache 64 frame LRU · Texture cache 256MB · QP heatmap texture 128MB · Diff heatmap texture 128MB(A/B) · MV visible-list 32MB · Grid line cache 16MB |
| 캐시 축출 정책 | 가중치 LRU(메모리 바이트 기준), 사용량 80% 초과 시 공격적 축출, 축출 이벤트 perf HUD 로깅 |
| Fast-path/Quality-path 2단계 프리뷰 | Fast: 파일 열기·스크럽 중 Quarter/Half-res, 오버레이도 저해상도 강제(QP half-res, Diff quarter/half-res, MV stride 샘플링) — 목표 첫 프레임 표시 ≤60ms(1080p 기준). Quality: 입력 200ms 없을 때 트리거, 고품질 디코드→RGBA 변환→고해상도 오버레이 순 업그레이드, 사용자 입력 시 즉시 중단 |
| 필수 프로파일링 타이머 | open_file_total_ms, io_read_ms, mmap_setup_ms, parse_ms, index_build_ms, decode_ms, convert_ms, overlay_build_ms(오버레이별), upload_texture_ms, paint_ms, cache_hit_rate — Dev HUD 토글 + 세션별 export 가능한 perf report로 노출 |
| 자동 성능 저하 규칙 | paint_ms 평균(60프레임 롤링) 16ms 초과 2초 지속 → LOD 1단계 상승 + MV stride 2 이상 강제 / 33ms 초과 → Diff 오버레이 자동 비활성(토스트) + heatmap Quarter-res로 하향 / 캐시 사용량 80% 초과 → diff→qp→mv→grid 순으로 오버레이부터 축출 / 스크럽 중 → Quality-path 비활성, Diff 기본 off, MV는 항상 샘플링 |
| LOD 정책 (Timeline/Chart, `LOD_PERF_CACHE_SPEC.md`) | LOD0 가시프레임 2,000 이하(전체 포인트) / LOD1 2,000~20,000(버킷당 min/max 엔벨로프) / LOD2 20,000~200,000(분위수 엔벨로프+희소 마커) / LOD3 200,000 초과(밀도 히트밴드+키 마커만); 버킷 수 = min(2048, ceil(N/target)), target=1,200(라인)/2,400(바); 마커는 절대 드롭하지 않고 밀도 1/6px 초과일 때만 클러스터링 |
| 캐시 키 정규 포맷 | `<kind>:<stream>:<codec>:<file_hash>:<params_hash>` — file_hash = SHA1(첫 4MB)+파일크기; 예: `overlay_tile:A:AV1:<hash>:qp_map frame120 tile12,8 scale2` |
| 성능 예산 (60fps 기준) | 프레임 전체 16.7ms 중 렌더링 6ms 이하, 인터랙션 처리 2ms 이하, 레이아웃 2ms 이하 |
| 세부 예산 (from `parity_harness/perf_budget_and_instrumentation.json`) | hit-test ≤1.5ms, tooltip 빌드 ≤0.8ms, selection 전파 ≤2.0ms; 예산 초과 시 단계적 저하: 라벨 비활성화 → 벡터 집계 → 히트맵 다운샘플 → 사유 표시 placeholder |

**예상 소요:** 중급 3~4주

---

## Phase 11: 폴리시, 키보드 단축키, 옵션 완성 🟢

- [ ] 모든 키보드 단축키 완성 (`PARITY_CHECKLIST.md` Layer 4 참조)
- [ ] Options 메뉴 모든 항목 구현
- [ ] 창 레이아웃 저장/복원 (패널 크기, 위치)
- [ ] 최근 파일 목록 (최대 10개)
- [ ] 에러/경고 Status Panel 완성
- [ ] 접근성 (ARIA 레이블, 키보드 네비게이션)
- [ ] 다크/라이트 테마 완성
- [ ] 라이선스 활성화 시스템 (오픈소스이므로 생략 가능)

**예상 소요:** 중급 2~3주

---

## Phase 12: 전체 Parity 검증 & 테스트 🔴

- [ ] 공개 테스트 비트스트림으로 모든 모드 검증
- [ ] 자동화 스크린샷 비교 테스트
- [ ] 신택스 값 비교 테스트
- [ ] 회귀 테스트 스위트 구축

**예상 소요:** 중급 2~3주

---

## Appendix: 중요 Tauri 커맨드 추가 목록 (신규 필요)

```typescript
// Phase 1: 코덱 모드 시스템
get_codec_modes(codec: CodecType) -> Vec<ModeInfo>

// Phase 3: VVC 전용
get_vvc_dual_tree_data(frame: number) -> DualTreeData
get_vvc_alf_data(frame: number) -> AlfData
get_vvc_lmcs_data(frame: number) -> LmcsData

// Phase 4: AV1 전용
get_av1_cdef_data(frame: number) -> CdefData
get_av1_film_grain_data(frame: number) -> FilmGrainData
get_av1_loop_restore_data(frame: number) -> LoopRestoreData

// Phase 7: YUVDiff
load_debug_yuv(path: string, params: YuvParams) -> Result<void>
get_diff_frame(frame: number, amplify: number) -> FrameData
find_first_diff() -> Result<number>
get_yuv_metrics(frame: number) -> PsnrSsimResult

// Phase 8: Syntax 상세
get_stats_data(stream_id: string) -> StreamStats
get_qm_data(frame: number, codec: CodecType) -> QmData
get_vp9_prob_data(frame: number) -> ProbabilityData
```

---

## Appendix: Architecture & Correctness Reference (from `docs/_import_v14` critical_contracts/architecture pack, 2026-07-31)

**중요 발견 — 이 규칙들은 "앞으로 구현할 로드맵"이 아니라 이미 `crates/bitvue-core/src/`에 대부분 코드로 구현되어 있음.**
해당 크레이트의 모듈 주석이 이 v14 pack의 정확한 파일명을 인용한다 (예: `selection.rs`가 `SELECTION_PRECEDENCE_RULES.md`를,
`command.rs`/`event.rs`가 `ARCHITECTURE.md §3.2/§3.3`을, `diagnostics.rs`가 `ERROR_MODEL.md`를, `coordinate_transform.rs`가
`COORDINATE_SYSTEM_CONTRACT.md`를, `lockcheck.rs`가 `V12_LOCKCHECK_SPEC.md`를 인용) — 즉 이 spec pack의 이전 버전(v9~v13)이
과거 세션에서 이미 `bitvue-core`로 구현되었다는 뜻. `crates/bitvue-core/src/lib.rs`의 `pub mod` 목록은 T0-1~T10-1 단계 태그를
달고 있어 순차 구현 이력이 그대로 남아 있다. 아래 표는 그 규칙 자체(유지보수 시 지켜야 할 계약)를 압축한 것이지 신규 작업이 아님 —
새 오버레이/패널 추가 시 이 표를 어기지 않았는지 확인하는 용도로 사용.

| 계약 | 핵심 규칙 | 구현 위치(코드) |
|---|---|---|
| Frame Identity | Primary timeline index = Display order(PTS). decode_idx는 내부 전용. PTS/DTS mismatch는 전용 band로만 시각화 | `crates/bitvue-core/src/frame_identity.rs`, `frame_identity_test.rs` |
| Coordinate System | 파이프라인 고정: `screen_px → video_rect_norm(0..1) → coded_px → block_idx`. 모든 오버레이가 이 파이프라인만 사용, fit/zoom/pan은 screen→norm 단계만 수정 | `coordinate_transform.rs`, `coordinate_transform_test.rs` |
| Selection Precedence | 우선순위 Block > Point > Range > Marker, 한 번에 하나의 selection type만 활성 | `selection.rs` (`TemporalSelection` enum) |
| Cache Invalidation | QP/MV/Partition/Diff/Timeline 오버레이별 무효화 트리거 목록; 프레임 변경 시 프레임 종속 오버레이는 항상 무효화; 텍스처를 다른 frame_idx에 재사용 금지 | `cache_provenance.rs`, `cache_validation.rs` |
| Async Backpressure | Latest-wins 큐, 스트림당 in-flight 최대 2, 스크럽 중 비-현재 작업 취소 + quality-path 업그레이드 비활성 | `worker.rs` |
| Indexing Strategy | 2단계: Quick Index(키프레임/OBU 경계만 스캔, 즉시 표시) → Full Index(백그라운드, 진행률), UI는 Full Index 대기 안 함 | `indexing.rs`, `index_session*.rs`, `index_extractor*.rs` |
| Tri-sync 권위 순서 | 충돌 시 `bitRange > syntaxNode > unit > frameIndex/pts > stream_id` 순으로 해소; Hex→Syntax 역매핑은 "가장 작게 포함하는 노드, tie는 최대 depth, 그래도 tie면 SyntaxNodeId 사전순"으로 결정적 | `evidence.rs`(bit_offset↔syntax↔decode↔viz 4-layer evidence chain), `player_evidence.rs`, `timeline_evidence.rs` |
| Error Model | 심각도 Info/Warn/Error/Fatal; Diagnostic 레코드는 `offset_bytes` 필수; Fatal이어도 앱은 크래시하지 않고 Hex 검사는 계속 가능 | `diagnostics.rs`, `diagnostics_bands.rs`, `error.rs`, `app_error.rs` |
| File I/O | mmap 기반 랜덤 액세스, 세그먼트 캐시(64~256KB), 파일 크기 변경 시 mmap 무효화 + WARN 진단 | `byte_cache.rs` — `memmap2` 의존성 확인됨(`Cargo.toml`) |

**Layout/Grid 참고자료 (from `LAYOUT_CONTRACT.md`/`LAYOUT_GRID_SYSTEM.md`/`RESPONSIVE_VISUALIZATION_RULES.md`) — 주의:**
이 3개 문서는 egui 네이티브 5-region(R1 Toolbar/R2 Left/R3 Center/R4 Right/R5 StatusBar) 도킹 레이아웃을 전제로 하며, Bitvue는
Tauri+React `DockableLayout` 패널 시스템(Phase 0 완료)을 쓰므로 리전 이름 자체는 적용되지 않는다. 숫자만 참고할 가치가 있다면:
splitter 최소 폭 320px/최소 높이 140px, 타임라인 스트립 기본 높이 clamp(160px, 18vh, 260px), 툴팁 최대폭 360px/최대높이 40vh,
바 너비 <2px면 LOD 버킷 렌더링으로 전환, 리사이즈 이벤트 디바운스 필수 — Bitvue `DockableLayout`이 이미 이런 규칙을 갖는지는
미확인이므로 실제 적용 전 `frontend/components/layout/` 코드와 대조 필요 (이번 마이닝에서는 검증하지 않음).

---

## Appendix: Future Differentiators (beyond parity — post-parity/aspirational, from `docs/_import_v14` insight/session/ci/compliance/mcp specs)

이 섹션은 경쟁사 패리티(CMP-0N)가 아니라 v14 pack이 자체적으로 "Differentiators (our advantage)"로 분류한 신규 기능 제안이다.
**패리티 백로그에 섞지 말 것.** 다만 2026-07-31 마이닝 중 확인된 중요 사실: 아래 4개 기능 모두 `crates/bitvue-core/src/`에
데이터 모델/로직 수준으로는 이미 부분 구현되어 있으나(모듈 주석이 각 spec 파일명을 그대로 인용), **`frontend/`에서 이를 사용하는 곳은
전무하고 Tauri 커맨드로도 노출되지 않음** (grep 결과 `InsightFeed|ComplianceScoreboard|McpIntegration|SessionEvidence` 프론트엔드 매치 0건).
즉 "데이터 계층은 있음, UI/커맨드 배선이 남은 작업"이라는 뜻 — 신규 설계가 아니라 배선 작업으로 재정의됨.

| 기능 | 핵심 아이디어 | 구현 위치(백엔드, 이미 존재) | 남은 작업 |
|---|---|---|---|
| Insight Feed | 규칙/통계 기반 auto-summary 카드(QP spike, metric dip, error burst, reorder mismatch, HRD risk, A/B regression 등), Jump/Filter/Export 가능, 트리거 근거 표시 | `insight_feed.rs` (`InsightType` enum 등 확인됨) | Tauri 커맨드 노출 + 프론트엔드 카드 UI |
| Session Evidence | `.baxsession.json` 세션(열린 파일/레이아웃/선택/북마크) + 북마크를 "증거 번들"(스냅샷+수치 요약+딥링크)로 export, 버그리포트(md+이미지+csv) 생성 | `evidence.rs` (bit_offset/syntax/decode/viz 4-stage evidence chain) | 세션 직렬화 포맷 확정 + export 커맨드 |
| Compliance Scoreboard | timing/reference structure/HRD/metadata/syntax legality 카테고리별 점수 + 위반 목록(룰 id, 조건, 관측값, jump target) | **미구현** — `parity_harness/mod.rs`의 `CategoryScore`는 이름만 비슷할 뿐 competitor-parity 스코어링용(아래 행 참조)이지 codec-compliance 스코어보드가 아님. 확인 완료(2026-07-31), 착오 정정 | 전체 신규 구현 |
| Regression Guard (CI) | A/B 비교에서 metric_delta/error_burst/reorder_mismatch/HRD 조건으로 CI 게이트 규칙 생성, `regression_report.json`/`regression_summary.md` 출력 | **목적이 다른 유사 시스템 존재**: `crates/bitvue-core/src/parity_harness/mod.rs`는 REGRESSION_GUARD_SPEC이 아니라 `_import_v14/parity_harness/*.json`(경쟁사 패리티 매트릭스 스키마 검증/스코어링/semantic probe/render snapshot/evidence diff/Hard-Fail·Parity·Perf 게이트)의 구현체 — A/B 스트림 리그레션이 아닌 "Bitvue vs 경쟁툴 parity matrix" 채점용. 테스트(`tests/parity_harness.rs`, `parity_baseline_evaluation.rs`)만 있고 `scripts/parity_check.sh`/CI에는 배선 안 됨(grep 확인) | 전용 A/B 리그레션 룰 엔진 + CI 잡 (완전 별개 신규 구현 필요); 별도로 `parity_harness/mod.rs`를 실제 `scripts/parity_check.sh`에 연결하는 것도 독립적인 미완 작업 |

**Explainability Hints (from `EXPLAINABILITY_HINTS.md`)** — 오버레이별 마이크로 힌트 카피 예시, UI 문구 작성 시 참고:
QP Heatmap "Auto scale: min/max from current frame" / "Fixed scale: 0..63"; MV "Vectors shown in px (qpel/4)"; Partition
"Scaffold grid shown when partition data unavailable"; Diff "Abs diff: \|A-B\|" / "Signed diff: A-B"; Timeline "Markers never
dropped; clustered when dense".

**Onboarding First-5-Minutes flow (from `ONBOARDING_FIRST_5_MIN.md`)** — Help 메뉴에 넣을 5단계 가이드 초안: ① Timeline에서
frame size/QP 오버레이로 이상 구간 드래그 선택 → ② Player로 점프해 QP Heatmap/MV/Partition으로 공간적 원인 확인 → ③ Metrics
워크스페이스에서 전체 대비 선택구간 히스토그램 비교 → ④ Diagnostics에서 에러 버스트 자동 선택 후 evidence export → ⑤(optional)
Stream B 로드 후 Compare 워크스페이스로 A/B 델타 확인. Bitvue에 아직 없는 개념: Worst Frames 목록, Regression Guard 제안 —
이 둘은 위 표의 Insight Feed/Regression Guard가 선행되어야 함.

---

## Appendix: MCP 서버 실제 구현 대조 (from `docs/_import_v14/monster_pack/docs/mcp/MCP_INTERACTION_MODEL.md`, 2026-07-31)

**결론: Bitvue에는 서로 무관한 두 개의 MCP 관련 구현이 존재하며, spec을 실제로 따르는 쪽은 바이너리로 노출되지 않는다.**

| | spec (`MCP_INTERACTION_MODEL.md`) | `crates/bitvue-mcp`(`bitvue-mcp-server` 바이너리, 실행됨) | `crates/bitvue-core/src/mcp.rs`(`McpIntegration`) |
|---|---|---|---|
| 모델 | Read-only "resources"(8종) + "actions"(제안/설명/초안 생성, 5종) | JSON-RPC stdio, MCP 표준 "tools"(10종: load_file/analyze_frame/get_qp_map/get_motion_vectors/compare_streams/get_gop_structure/find_decoding_issues/get_stream_info/search_syntax/list_files) | spec의 resource 목록과 거의 동일: `selection_state/insight_feed/diagnostics/metrics_summary/timeline_lanes/compare/session_evidence/compliance` — `get_resource(name)`/`list_resources()` 구현 |
| 코덱 지원 | 코덱 무관 설계 | **IVF/AV1 컨테이너만 파싱** (`parse_ivf_file`, 확장자 `.ivf`/`.av1` 외 전부 미지원 에러) | `bitvue-core` 전체 모델을 재사용하므로 코덱 무관 |
| 관계 | — | `bitvue-core`/`bitvue-av1-codec`에 의존하지만 **`bitvue_core::mcp::McpIntegration`은 import하지 않음** — 자체 tool 세트를 처음부터 새로 구현 | `bitvue-mcp-server`의 `main.rs`에서 전혀 참조되지 않음 — 어디서도 호출되지 않는 죽은 코드에 가까움(테스트 커버리지만 있을 가능성) |

**정리:** 실행 가능한 `bitvue-mcp-server`는 spec과 무관한, 훨씬 단순한 자체 설계(질의형 tool-calling, AV1/IVF 전용)이고, spec을
거의 그대로 구현한 `McpIntegration`(read-only resource 모델)은 `bitvue-core` 안에 존재하지만 어떤 바이너리에서도 사용되지 않는다.
두 구현을 통합할지, `McpIntegration`을 `bitvue-mcp-server`에 연결할지는 결정되지 않은 상태 — Phase 12 이후 정리 대상으로 기록.
