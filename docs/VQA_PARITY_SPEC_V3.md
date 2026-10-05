# Bitvue — VQ Analyzer Parity Specification v3
> 작성일: 2026-04-08 (경쟁 제품 조사 추가: 2026-07-31, features.yaml 이전: 2026-10-05)
> 기준: VQ Analyzer User Guide (https://cdn.vicuesoft.com/vqAnalyzer/docs/VQAnalyzerUserGuide.html)
> 추가 조사 대상: ViCueSoft VQ Probe, Interra VEGA Media Analyzer, Elecard StreamEye, Codecian CodecVisa — §1.5
> 현재 Bitvue 버전: v0.12.0
> **See also:** `docs/specs/features.yaml` (**기능/상태의 유일한 source of truth**), `COMPETITOR_FEATURE_MATRIX.md`,
> `PARITY_CHECKLIST.md`, `UX_PARITY_MATRIX.md` (메뉴 구조 §10, 오버레이 색상 스케일 §11, 단축키/마우스), `DEVELOPMENT_PHASES.md`

이 문서는 목표, 설계 결정, 근거, 참조 데이터만 다룬다. 항목별 구현 상태(✅/⚠️/❌)는 여기에 두지 않는다.

---

## 1. Project Overview

### 1.1 목표
- **1차 (Parity):** ViCueSoft VQ Analyzer 기능을 재현하는 오픈소스 대체제.
- **2차 (Extension):** 모듈식 아키텍처로 신규 코덱, AI 기반 분석, WebAssembly 브라우저 모드를 쉽게 확장한다.

> 현황 요약: features.yaml (area: codec/decode/overlay). 데스크톱 파이프라인(sidecar/indexer)은 현재 IVF/AV1 전용이다.
> 그 밖의 코덱은 crate 파서/오버레이 추출기와 CLI `decode`에서만 쓸 수 있다.

### 1.2 기술 스택 (실제 코드 기준)

| 영역 | 사용 기술 |
|---|---|
| 앱 셸 | Electron (`bitvue-desktop/`), Rust `bitvue-sidecar` 프로세스. 9-byte 헤더 framed protocol, stdin/stdout (`bitvue-protocol`). 2026-08-08 Tauri에서 이전 |
| Bitstream parsing | 코덱별 자체 crate(`bitvue-av1-codec`, `-avc`, `-hevc`, `-vp9`, `-vvc`, `-mpeg2-codec`, `-avs3`, `-jpegxs`, `-vc3`), 자체 bitreader |
| AV1 decode | dav1d (항상 포함) |
| H.264/HEVC/VP9 decode | `ffmpeg-next`, optional feature `ffmpeg` (기본 빌드에 없음) |
| VVC decode | vvdec, optional feature `vvdec` (시스템 라이브러리 필요) |
| Container | `bitvue-formats` 자체 구현 (MP4, MKV, MPEG-TS, 감지). IVF는 `bitvue-av1-codec::ivf` |
| YUV 처리 | `bitvue-decode` (yuv_to_rgb, YuvLoader), `std::arch` SIMD (`strategy/{avx2,neon}.rs`) |
| Metrics | `bitvue-metrics` (PSNR/SSIM, BD-rate, VMAF는 optional feature `vmaf` = libvmaf-rs) |
| Async | sidecar는 동기 방식. tokio는 `bitvue-mcp`에서만 사용 |
| Frontend | React 18 + TypeScript + Vite, Canvas 2D (+ MV용 WebGL), `react-resizable-panels`, React Context, 자체 가상화 필름스트립 |
| 자동화 | CLI `bitvue` (`bitvue-cli`), MCP 서버 `bitvue-mcp-server` |

### 1.3 주요 기술적 도전 과제

| 과제 | 난이도 (1~10) | 설명 |
|------|-------------|------|
| AV1 심볼 디코딩 (arithmetic coding) | 9 | CDF 업데이트, tile 병렬화 |
| HEVC CABAC 풀 파싱 | 8 | 엔트로피 디코딩 없이 오버레이 불가 |
| VVC 파싱 (dual-tree, LMCS, ALF) | 9 | 스펙 복잡도 최고 |
| YUVDiff 픽셀 정합 | 7 | display order vs coding order 동기화 |
| 대용량 스트림 (4K+ > 1GB) | 7 | 메모리 맵 + 청크 스트리밍 필요 |
| MV 벡터 렌더링 (수천 개) | 6 | Canvas 성능, WebGL 경로 |
| AVS3 / JPEG XS 지원 | 7 | 스펙/레퍼런스 구현 접근성 낮음 |
| Pixel-perfect 오버레이 매칭 | 8 | 색상 스케일, 블록 경계 위치 |
| 크로스플랫폼 빌드 (CI) | 5 | FFmpeg/vvdec linkage 복잡 |

### 1.4 전체 프로젝트 예상 기간 (2026-04-08 추정, 참조용)

| 개발자 수준 | 전체 기간 |
|------------|---------|
| 초보 Rust + UI | 18~24개월 |
| 중급 Rust + 코덱 지식 | 10~14개월 |
| 시니어 + 팀 (2~3인) | 5~7개월 |

(원문의 "현재 진행 상황 약 40~50%" 행은 삭제했다. 진행 상황은 features.yaml을 본다.)

---

## 1.5 경쟁 제품 조사 — 우선순위 근거 (2026-07-31)

전체 제품별 매트릭스는 `COMPETITOR_FEATURE_MATRIX.md`, 항목 상태는 features.yaml(`competitors:` 필드)을 본다.
조사 근거는 벤더 공식 사이트/릴리스노트만 사용했다. 트라이얼/로그인 게이트 뒤의 내용은 "미확인"으로 남겼다(부록 C).

| 제품 | 카테고리 | Bitvue 관련성 |
|---|---|---|
| ViCueSoft VQ Probe | 듀얼 스트림 비교/오프라인 QC | 프레임 비교 원조 (§4.9) |
| Interra VEGA Media Analyzer | 코덱 분석기 (직접 경쟁) | CABAC 상태, 배치 검증, 방송 컨포먼스 |
| Elecard StreamEye | 코덱 분석기 + 멀티스트림 비교 | 비교 모드 명칭 체계, VMAF 포함 품질지표 |
| Codecian CodecVisa/Pelscope | 코덱 분석기 (2017, 레거시) | 참고용, 공개 정보 제한적 |

**우선순위 판단 근거**

| 순위 | 항목 | 근거 |
|---|---|---|
| 🔴 Must | 듀얼 스트림/프레임 비교 (CMP-01..04) | VQ Probe와 StreamEye의 핵심 기능. YUVDiff(디코딩 vs 디버그 YUV)와 다른 유스케이스 |
| 🔴 Must | VMAF (CMP-06/07) | StreamEye와 VQ Probe 둘 다 계산 |
| 🔴 Must | RD 곡선 + BD-rate (CMP-05) | VQ Probe 핵심 오프라인 QC 기능 |
| 🟡 Check | CABAC range/state 시각화 (CMP-10) | VQ Analyzer(v6.1+)와 VEGA 공통 |
| 🟡 Check | APV 코덱 (CMP-08) | VQ Analyzer v7.7/7.8 추가 |
| Resolved | AVM 네이밍 (CMP-09 → `CODEC-006`) | AOM 차세대 코덱의 공식명은 AVM(→AV2). Bitvue에는 "AV3" 코덱이 없다(구 `bitvue-av3-codec`는 삭제됨). AVS3(IEEE 1857.10)는 별개 코덱 |
| 🟢 Out-of-scope | 방송 컨포먼스/자막/오디오/ABR QC | 아래 근거 |
| ⚪ 미확인 | VEGA "AI/ML 이상 탐지" | 마케팅 문구, 스펙 비공개 |

**Out-of-scope 근거:** Bitvue는 코덱 비트스트림 *분석기*이며, 실시간 방송/TS *모니터링 프로브*가 아니다. VQ Probe/VEGA의
라이브 TS 컨포먼스(TR101290 등), 자막, 오디오 라우드니스, ABR 래더(Convex Hull) 기능은 다른 제품 카테고리이므로 제외한다
(features.yaml `status: dropped`).

---

## 2. Overall UI Layout & Navigation

### 2.1 메인 윈도우 레이아웃 (목표 레이아웃)

```
┌─────────────────────────────────────────────────────────────────┐
│ [TitleBar]  File | Mode | View | YUVDiff | Options | Help       │
│ [Toolbar]  [Open] [Close] | [←][→][⏮][⏭] | [F1][F2]...[F12] │
├──────────────┬──────────────────────────────┬───────────────────┤
│ Stream View  │      Main Panel              │  Syntax Info      │
│ (Filmstrip)  │  (Decoded Frame + Overlays)  │  (Tab Panel)      │
│  Thumbnails  │  Canvas / WebGL              │  NAL/OBU/Frame    │
│  FrameSizes  │  Zoom: mouse wheel           │  SPS/PPS/APS      │
│  B-Pyramid   │  Pan: click+drag             │  Slice/Block      │
│  HRD Buffer  │  Click: select block         │  Ref Lists        │
│  Metrics     │  F: fullscreen, Y/U/V        │  Stats            │
├──────────────┴──────────────────────────────┴───────────────────┤
│ Selection Info Panel  │  Unit Info / HEX View                   │
├───────────────────────┴─────────────────────────────────────────┤
│ [Status Bar]  frame N/total | codec | resolution | status       │
└─────────────────────────────────────────────────────────────────┘
```

패널 규칙: 모든 패널은 splitter로 resize하고, View 메뉴로 표시/숨김한다. Stream View와 Main Panel은 선택 프레임을 항상 동기화한다.
Selection Info는 블록을 클릭하면 갱신된다.

### 2.2 메뉴 구조
→ `UX_PARITY_MATRIX.md` §10 "Menu structure reference". 코덱별 F키 매핑은 §4.4.

### 2.3 툴바
```
[Open ▼] [Close] | [⏮][←][→][⏭] | [F1]..[F12] | [Y][U][V] | [QP][HM][MV][...] | [Zoom- Fit Zoom+]
```

### 2.4 색상 / 시각적 스타일 가이드

| 요소 | 색상 |
|------|------|
| I / P / B frame (Stream View) | 빨강 #FF4444 / 파랑 #4488FF / 초록 #44BB44 |
| 장기 참조 프레임 화살표 | 초록 |
| CVS 경계 밴드 | 연초록 반투명 |
| QP Heatmap (낮음→높음) | 파랑 → 초록 → 노랑 → 빨강 (jet) |
| MV Heat (크기) | 흑백 → 파랑 → 빨강 |
| 블록 경계 / CTU 경계 | 흰색 1px / 노란색 2px |
| 선택된 블록 | 노란색 반투명 채움 + 빨간 경계 |
| PSNR Heatmap (높음→낮음) | 파랑 → 빨강 |

오버레이별 세부 색상 스케일 → `UX_PARITY_MATRIX.md` §11.

---

## 3. Supported Codecs & Containers

> Feature/status items for this section live in `docs/specs/features.yaml` (area: codec, decode, container; legacy ids CMP-08, CMP-09).
> 대상 코덱: HEVC, VVC, AV1, VP9, AVC, MPEG-2, AVS3, JPEG XS, VC-3, APV, AVM. 컨테이너: IVF, MKV/WebM, MP4/MOV, MPEG-2 TS/PS,
> AVI, MXF, MMT, Annex B, raw OBU. 코덱별 특수 기능(HEVC RExt/SCC/SHVC/MCTS/SEI, VVC dual-tree/LMCS/ALF/CCLM/IBC/subpicture/APS/MRL,
> AV1 128×128 SB/film grain/CDEF/LR/SuperRes/tile/show-existing/IntraBC, VP9 superframe/prob tables/segment/frame-parallel)도 항목으로 관리한다.

---

## 4. Detailed Feature Breakdown

> Stream View(§4.1), Main Panel(§4.2), Syntax 탭(§4.3), Selection Info(§4.5), Hex(§4.6), YUVDiff(§4.7), Stats(§4.8),
> Dual-stream(§4.9)의 항목은 `docs/specs/features.yaml`에 있다 (area: ui, ux, syntax, overlay, compare, metrics).
> 우선순위 표기는 Must-have→P1, Important→P2, Nice-to-have→P3, §1.5 🔴→P0로 옮겼다.

### 4.3 Syntax 탭 구성 (참조)

| 코덱 | 탭 |
|---|---|
| HEVC | NAL · Header (VPS/SPS/PPS/SEI) · Block Info (CU/TU) · QM · Ref Lists (L0/L1, POC, 가중치) · Stats |
| VVC | NAL · SPS · PPS · APS (ALF/LMCS/QM) · Slice · CU (dual-tree) · Ref Lists · Stats |
| AV1 | IVF · OBU · Sequence · Frame · Block · Refs · Stats |
| VP9 | MKV · Frame · Probabilities · Counts · Refs · Block · TX · Stats |
| AVC | NAL · Header (SPS/PPS/SEI) · QM · Ref Lists · Stats |

### 4.4 코덱별 F키 모드 매핑 (Bitvue UI 설계 참조)

VQ Analyzer는 코덱마다 F키 모드 조합이 다르다. 2026-04 시점에는 이것이 Bitvue(고정 6모드)와의 가장 큰 UX 차이였고,
이후 코덱별 기능을 붙이려면 이 분기 구조를 먼저 잡아야 했다(구 §8.1 1순위). 이 절은 F키 번호와 모드 이름 할당만 다룬다. 구현은 `frontend/utils/codecModeRegistry.ts`이고, 모드별 상태는 features.yaml(area: overlay)에 있다.
F키가 없는 Info Overlay(QP Map, Heat Map, MV Heat, PU Type, PSNR/SSIM 등)는 레지스트리에서 `fKey: null`로 정의된다.

| F키 | HEVC | VVC | AV1 | VP9 | AVC | MPEG-2 | AVS3 | JPEG XS |
|---|---|---|---|---|---|---|---|---|
| F1 | Coding Flow | Dual Tree | Coding Flow | Coding Flow | Coding Flow | Predictions | Coding Flow | Precinct |
| F2 | Predictions | Coding Flow | Predictions | Predictions | Predictions | Transform | Predictions | Dequant |
| F3 | Transform | Predictions | Transform | Transform | Transform | YUV | Transform | Transform |
| F4 | Reconstruction | Transform | Reconstruction | Reconstruction | Reconstruction | | Reconstruction | MCT |
| F5 | Loop Filter | Reconstruction | Loop Filter | Loop Filter | Loop Filter | | Loop Filter | NLT |
| F6 | SAO | Inverse Map | CDEF Filter | YUV | YUV | | SAO | YUV |
| F7 | YUV | Loop Filter | SuperRes Filter | | | | ESAO | |
| F8 | | SAO | Loop Restoration | | | | CCSAO | |
| F9 | | Adaptive Filter (ALF) | Film Grain Pixels | | | | YUV | |
| F10 | | YUV | YUV | | | | | |

VC-3: F1 = Segment (레지스트리 정의). 이 표를 바꾸면 `COMPETITOR_FEATURE_MATRIX.md`와 features.yaml의 overlay 항목도 함께 확인한다.

### 4.5 Selection Info 패널 (목표 표시 내용)
```
Block Position: (x=64, y=128)   Block Size: 32x32   Partition: SPLIT_QT → SPLIT_BT_H
Prediction Mode: INTER   Ref L0: POC=5, idx=0   MV L0: (+3.25, -1.5) [1/4pel]   Ref/MV L1 …
QP: 28 (Chroma 28)   CBF: Y=1 Cb=0 Cr=0   Transform: 16x16 (DCT-II)
RD Cost: 1024.5   Bits: 48
```

### 4.6 Unit Info / HEX View (목표 동작)
```
Offset   00 01 02 03 04 05 06 07   ASCII
0x0000:  00 00 00 01 67 64 00 2A   ...gd.*
0x0008:  AC D9 40 4B FF FF FF E7   ..@K....
[강조: 현재 파싱 중인 비트 위치 하이라이트]
```
Offset/bytes/ASCII 덤프를 표시하고, 현재 파싱 비트 위치를 하이라이트한다. 비트 오프셋은 신택스 요소에 매핑된다.
Syntax 트리와 HEX는 양방향으로 연결된다: 바이트를 클릭하면 해당 NAL/OBU 요소로 Syntax 탭이 포커스되고, 트리에서 요소를 클릭하면 HEX의 해당 바이트가 하이라이트된다.

### 4.7 YUVDiff / Debug YUV (목표 UI)
```
[Debug YUV 로드]  Format: Planar / NV12 | Bit Depth: 8/10/12/16/Match | Display Order | Crop L/R/T/B | Picture Offset | Auto-reload
PSNR Y/U/V/Avg, SSIM
[Show Decoded] [Show Debug YUV] [Show Diff] [Amplified Diff ×N] [Find First Difference]
```
요구사항: 크기 기반 포맷 추론, |decoded−reference| 픽셀 차이, 증폭 표시, 첫 불일치 프레임 탐색, coding/display 순서 변환 후 비교.
VMAF 출력 목표: pooled score(0~100) + per-frame + 서브스코어(ADM2, VIF, motion2). 서브스코어까지 노출하면 VQ Probe 대비 우위다.
지표 범위 근거: StreamEye는 PSNR, APSNR, SSIM, DELTA, MSE, MSAD, VQM, NQI, VMAF, VMAF phone, EPSNR, VIF를 계산하고 VQ Probe도 VMAF를 포함한다.
그래서 PSNR/SSIM 다음으로 VMAF를 최우선 추가 대상으로 잡았다(경로: libvmaf 바인딩 또는 `ffmpeg -lavfi libvmaf` 서브프로세스).

### 4.8 Stats 탭 (목표 내용)
Picture(I/P/B별 프레임 수, 평균 QP, 평균 크기), Coding Unit(CU 크기 분포, 인터/인트라 비율, 변환 크기 분포),
Stream(총 비트/헤더/데이터, 평균 비트레이트, 최대/최소 프레임 크기).

### 4.9 Dual-Stream / Frame Compare
YUVDiff(§4.7)는 "디코딩 결과 vs 디버그 YUV"를 비교한다. §4.9는 **독립된 두 비트스트림**(예: 인코더 A vs B)을 비교한다.
VQ Probe와 StreamEye가 둘 다 핵심 기능으로 취급하고, 사용자가 "인코더 A vs 인코더 B" 비교를 위해 가장 먼저 찾을 워크플로우라서 🔴 Must로 잡았다.
설계 메모: 프레임 수가 다르면 PTS/POC 기반 정렬 옵션을 둔다. Temperature/Subtraction과 Find First Difference는 §4.7 diff 엔진과 로직을 공유한다.
StreamEye 비교 모드 명칭(참조 기준):

| 모드 | 설명 |
|---|---|
| Compare | side-by-side 재생, 동기화된 프레임 인덱스 |
| PSNR / PSNR Clip | 스트림 간 프레임별 PSNR / 임계값 이하 구간 하이라이트 |
| Temperature | 차이값 히트맵 |
| Subtraction | 픽셀 차이 이미지 |
| Horizontal/Vertical Split | 슬라이더로 A/B 분할 |

Parity 검증: PSNR은 ffmpeg `psnr` 필터와 ±0.01dB 이내로 일치해야 한다. BD-rate는 표준 Bjøntegaard 구현(vmaf `bd_rate.py`)과 일치해야 한다.
A/B 정렬 규칙은 `UX_PARITY_MATRIX.md` §2.1을 따른다.

---

## 5–7. 이전된 절
- §5 Development Phases → `DEVELOPMENT_PHASES.md`
- §6 Keyboard/Mouse → features.yaml (area `ux`, UX-058 등; 구 Layer 4 표는 `docs/history/parity-checklist-log.md`), `UX_PARITY_MATRIX.md` §3-5
- §7 Parity Validation Strategy → `PARITY_CHECKLIST.md` "Parity validation strategy & test sources"

---

## 8. Architecture & References

### 8.1 진행률 / 우선순위
> 진행률 표(구 §8.5)는 삭제했다. 현재 상태와 우선순위는 `docs/specs/features.yaml`을 본다.

구 §8.1 우선순위 결정과 근거 (2026-04-08 작성, 2026-07-31 갱신. 결정 기록이며 현재 상태가 아니다):
0. **Dual-Stream Compare & VMAF (Phase 7.5)**: §1.5 조사에서 발견된 가장 명확한 갭이다. 기존 체크리스트가 "완료"로 표기한 항목에 이 기능이 아예 없었다. 패리티 기준 자체의 사각지대였다.
1. **코덱별 F키 모드 분기 (Phase 1)**: 가장 큰 UX 차이였다. 코덱별 동적 모드로 바꾸지 않으면 이후 코덱별 기능 추가가 모두 어색해지므로 아키텍처 기반을 먼저 잡는다.
2. **VVC 디코딩 연결 (Phase 3 일부)**: 디코딩이 없으면 Main Panel에 프레임을 표시할 수 없다. vvdec 바인딩의 임팩트가 가장 크다.
3. **AVS3 코덱 (Phase 5)**: VQ Analyzer는 지원하고 Bitvue에는 없던 코덱이다. 시작이 늦을수록 기술 부채가 된다.
4. **YUVDiff 완성 (Phase 7)**: 코덱 디버깅 시 가장 많이 쓰는 기능이다.

### 8.2 아키텍처 확장성 가이드 — 새 코덱 추가 (표준 패턴)
```
1. crates/bitvue-{codec}/ 크레이트 생성 (파서 + overlay_extraction.rs)
2. bitvue-codecs / bitvue-codecs-parser에 등록
3. frontend/utils/codecModeRegistry.ts에 F키/Info overlay 매핑 등록
4. bitvue-decode에 디코더 연결 (필요시 optional feature)
5. bitvue-sidecar request_dispatch.rs 커맨드(open/index/get_frame_analysis 등)에 코덱 분기 추가
   — 현재 sidecar/indexer는 IVF/AV1 전용이므로 비-AV1 코덱 GUI 지원의 선행 조건이다
6. bitvue-cli decode/info 디스패치에 코덱 추가
7. Frontend: 코덱별 Syntax 탭 + OverlayRenderer/renderers/* 추가
```

플러그인 시스템 (장기 목표, features.yaml INFRA):
```rust
pub trait CodecPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn can_parse(&self, data: &[u8]) -> bool;
    fn parse_stream(&self, data: &[u8]) -> Result<StreamInfo>;
    fn get_modes(&self) -> Vec<AnalysisModeInfo>;
    fn get_overlay_data(&self, frame: usize, mode: AnalysisMode) -> Result<OverlayData>;
}
```

### 8.3 참조 자료

| 자료 | 목적 | 소스 |
|------|------|------|
| ITU-T H.265 (HEVC) | RExt/SCC/SHVC | ITU-T |
| ITU-T H.266 (VVC) | dual-tree, LMCS, ALF | ITU-T |
| AV1 스펙 | CDEF, Film Grain | https://aomedia.org/av1-bitstream-and-decoding-process-specification/ |
| AVS3 스펙 | ESAO, CCSAO | http://www.avs.org.cn/AVS3/ |
| AVS3 디코더 후보 | AVS3 디코딩 | `openavs3d` 또는 FFmpeg |
| JPEG XS | precinct, NLT | ISO/IEC 21122 |
| HM | HEVC 테스트 비트스트림 | https://hevc.hhi.fraunhofer.de/svn/svn_HEVCSoftware/ |
| VTM | VVC 테스트 비트스트림 | https://vcgit.hhi.fraunhofer.de/jvet/VVCSoftware_VTM |
| VQ Analyzer 원본 | 시각적 매칭 기준 | 트라이얼 버전 |

### 8.4 WebAssembly 확장 (장기 목표)
WASM 모드에는 Rust-native 또는 WASM 지원 디코더가 필요하다(FFmpeg 불가, dav1d는 dav1d.js 가능). 파일 시스템 접근은 Web File API로 대체한다.
`wasm` cargo feature는 아직 없다.

---

## 부록 C: 경쟁 제품 조사 — 소싱 메모

| 제품 | 조사 방법 | 신뢰도 / 제약 |
|---|---|---|
| ViCueSoft VQ Analyzer | 공식 릴리스노트 PDF v5.1–v7.8 (텍스트 추출) | 버전별 날짜 미표기라 "2026-04 이후 추가분"은 확정 불가. v7.8이 최신(조사 시점). ViCueSoft는 현재 Allegro DVT 산하 |
| ViCueSoft VQ Probe | 제품 페이지 | 공식, 날짜 정보 없음 |
| Interra VEGA Media Analyzer | 제품 사이트 | 공개 스펙시트/트라이얼 없음, 세부 UI/CLI 다수 미확인 |
| Elecard StreamEye | WebFetch (elecard.com/products/video-analysis/streameye) | 공식, 상세도 높음 |
| Codecian CodecVisa/Pelscope | 제품 다운로드 페이지 | 최종 릴리스 2017, 레거시/저신뢰. 프레임 비교·상세 메트릭 여부는 페이지에 미명시 |

조사 도구: Agent(general-purpose, WebSearch) + WebFetch. 검증할 수 없는 항목(트라이얼/로그인 게이트, 날짜 미표기)은
features.yaml 해당 항목 notes에 "미확인"으로 남긴다. 트라이얼을 확보하면
`PARITY_CHECKLIST.md` Tier 2(오버레이 시각 검증) 스크린샷 비교에 넣는다.
