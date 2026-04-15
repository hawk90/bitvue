# Bitvue — VQ Analyzer Parity Specification v3
> 작성일: 2026-04-08  
> 기준: VQ Analyzer User Guide (https://cdn.vicuesoft.com/vqAnalyzer/docs/VQAnalyzerUserGuide.html)  
> 현재 Bitvue 버전: v0.12.0

---

## 목차

1. [Project Overview](#1-project-overview)
2. [Overall UI Layout & Navigation](#2-overall-ui-layout--navigation)
3. [Supported Codecs & Containers](#3-supported-codecs--containers)
4. [Detailed Feature Breakdown](#4-detailed-feature-breakdown)
5. [Development Phases](#5-development-phases)
6. [Keyboard Shortcuts & Mouse Interactions](#6-keyboard-shortcuts--mouse-interactions)
7. [Parity Validation Strategy](#7-parity-validation-strategy)
8. [Next Steps & Recommendations](#8-next-steps--recommendations)

---

## 1. Project Overview

### 1.1 프로젝트 목표

**1차 목표 (Parity):** ViCueSoft VQ Analyzer의 모든 기능을 100% 재현하는 오픈소스 대체제 구현.  
**2차 목표 (Extension):** 모듈식 아키텍처로 신규 코덱 / AI 기반 분석 / WebAssembly 브라우저 모드 확장 용이.

현재 상태 평가:
- ✅ **코어 파싱 레이어**: AV1, HEVC, AVC, VP9, VVC, MPEG-2, AV3 — 7개 코덱 구현 완료
- ✅ **기본 UI 프레임워크**: 50+ React 컴포넌트, DockableLayout, Tauri IPC 23개 커맨드
- ⚠️ **Parity 갭**: AVS3, JPEG XS, VC-3, APV, AVM 코덱 미구현; 코덱별 세부 모드 불완전
- ⚠️ **YUVDiff 모드**: 인프라 존재하나 원본 수준 UX 미완성
- ❌ **코덱별 F1~F12 모드 분기**: VQ Analyzer는 코덱마다 서로 다른 모드 조합 — 현재 고정 6모드

### 1.2 기술 스택

#### Rust Backend

| 용도 | 추천 Crate | 비고 |
|------|-----------|------|
| Bitstream parsing | `bitvec`, `nom`, `bytes` | 현재 자체 구현, 유지 권장 |
| AV1 decoding | `dav1d` (via `dav1d-sys`) | ✅ 현재 사용 중 |
| H.264/HEVC/VP9 decoding | `ffmpeg-next` | ✅ feature-gated 사용 중 |
| VVC decoding | `vvdec-sys` | ⚠️ placeholder만 있음 |
| AVS3 decoding | `openavs3d` or FFmpeg | ❌ 미구현 |
| YUV processing | `yuv` crate, 자체 SIMD | 고성능 필요시 rayon 병렬화 |
| Container: MP4 | `mp4` or 자체 | ✅ 자체 구현 |
| Container: MKV | `matroska` or 자체 | ✅ 자체 구현 |
| Container: TS | `mpeg2ts` or 자체 | ✅ 자체 구현 |
| Container: MXF | `mxf` crate | ❌ 미구현 |
| Metrics | `libvmaf-sys` | ✅ optional feature |
| Async | `tokio` | ✅ 사용 중 |
| Serialization | `serde`, `serde_json` | ✅ 사용 중 |
| Memory mapping | `memmap2` | 대용량 스트림 |
| SIMD | `packed_simd` / `std::simd` | YUV 변환 가속 |

#### Frontend

| 용도 | 기술 | 비고 |
|------|------|------|
| UI 프레임워크 | React 18 + TypeScript | ✅ 현재 사용 |
| 2D 렌더링 | Canvas 2D API | ✅ 현재 사용 |
| 고성능 오버레이 | WebGL / WebGPU | 대형 프레임시 필요 |
| 레이아웃 | `react-resizable-panels` | ✅ 현재 사용 |
| 상태관리 | React Context + useReducer | ✅ 현재 사용 |
| 그래프 | D3.js or `recharts` | 메트릭 그래프 |
| 가상화 | `react-virtual` | 대형 필름스트립 |

### 1.3 주요 기술적 도전 과제

| 과제 | 난이도 (1~10) | 설명 |
|------|-------------|------|
| AV1 심볼 디코딩 (arithmetic coding) | 9 | CDF 업데이트, tile 병렬화 |
| HEVC CABAC 풀 파싱 | 8 | 엔트로피 디코딩 없이 오버레이 불가 |
| VVC 파싱 (dual-tree, LMCS, ALF) | 9 | 스펙 복잡도 최고 |
| YUVDiff 픽셀 정합 | 7 | display order vs coding order 동기화 |
| 대용량 스트림 (4K+ > 1GB) | 7 | 메모리 맵 + 청크 스트리밍 필요 |
| MV 벡터 렌더링 (수천 개) | 6 | Canvas 성능 — WebGL 필요 가능 |
| AVS3 지원 | 7 | 스펙/레퍼런스 구현 접근성 낮음 |
| JPEG XS 지원 | 7 | 특수 분야, 레퍼런스 제한적 |
| 100% Pixel-perfect 오버레이 매칭 | 8 | 색상 스케일, 블록 경계 위치 |
| 크로스플랫폼 빌드 (CI) | 5 | FFmpeg linkage 복잡 |

### 1.4 전체 프로젝트 예상 기간

| 개발자 수준 | 전체 기간 |
|------------|---------|
| 초보 Rust + UI | 18~24개월 |
| 중급 Rust + 코덱 지식 | 10~14개월 |
| 시니어 + 팀 (2~3인) | 5~7개월 |
| **현재 진행 상황 (v0.12.0)** | **약 40~50% 완료 추정** |

---

## 2. Overall UI Layout & Navigation

### 2.1 메인 윈도우 레이아웃

```
┌─────────────────────────────────────────────────────────────────┐
│ [TitleBar]  File | Mode | View | YUVDiff | Options | Help       │
│ [Toolbar]  [Open] [Close] | [←][→][⏮][⏭] | [F1][F2]...[F12] │
├──────────────┬──────────────────────────────┬───────────────────┤
│              │                              │                   │
│ Stream View  │      Main Panel              │  Syntax Info      │
│ (Filmstrip)  │  (Decoded Frame + Overlays)  │  (Tab Panel)      │
│              │                              │                   │
│  Thumbnails  │  Canvas / WebGL              │  NAL/OBU/Frame    │
│  FrameSizes  │  Zoom: mouse wheel           │  SPS/PPS/APS      │
│  B-Pyramid   │  Pan: click+drag             │  Slice/Block      │
│  HRD Buffer  │  Click: select block         │  Ref Lists        │
│  Metrics     │  F: fullscreen toggle        │  Stats            │
│              │  Y/U/V: component select     │                   │
├──────────────┴──────────────────────────────┴───────────────────┤
│ Selection Info Panel  │  Unit Info / HEX View                   │
│ (Block details,       │  (Raw bytes, bit offset, parsed fields) │
│  MV, QP, mode, refs)  │                                         │
├───────────────────────┴─────────────────────────────────────────┤
│ [Status Bar]  frame N/total | codec | resolution | status       │
└─────────────────────────────────────────────────────────────────┘
```

**패널 동작 규칙:**
- 모든 패널은 독립적으로 resize 가능 (splitter)
- 패널 표시/숨기기 — View 메뉴로 토글
- Stream View와 Main Panel은 항상 동기화 (선택된 프레임)
- Selection Info는 Main Panel에서 블록 클릭 시 업데이트

### 2.2 메뉴 구조

#### File 메뉴
```
File
├── Open Bitstream...          (Ctrl+O)
├── Open Recent ▶
│   └── [최근 파일 목록]
├── Close                      (Ctrl+W)
├── ─────────────────────
├── Extract Frames...          → 프레임 YUV/PNG 추출 다이얼로그
├── Extract NAL/OBU Units...   → 원시 비트스트림 유닛 저장
├── ─────────────────────
└── Exit                       (Alt+F4 / Cmd+Q)
```

#### Mode 메뉴 (코덱별 F키 매핑 — 아래 §4 참조)
```
Mode
├── F1: [코덱 의존 모드 1]
├── F2: [코덱 의존 모드 2]
├── ...
├── F12: [코덱 의존 모드 12]
├── ─────────────────────
├── Info Overlays ▶
│   ├── QP Map            (Toggle)
│   ├── Heat Map          (Toggle)
│   ├── MV Heat Map       (Toggle, HEVC)
│   ├── PSNR Overlay      (Toggle)
│   ├── SSIM Overlay      (Toggle)
│   ├── Block Type        (Toggle, AV1/VP9)
│   ├── PU Type           (Toggle, HEVC)
│   ├── MB Type           (Toggle, AVC)
│   ├── Reference Indices (Toggle)
│   └── Efficiency Map    (Toggle, AV1/VP9)
└── Simple Motion         (Toggle)
```

#### View 메뉴
```
View
├── Stream View            (Toggle)
├── Syntax Info            (Toggle)
├── Selection Info         (Toggle)
├── Unit Info / HEX View   (Toggle)
├── Status Panel           (Toggle)
├── ─────────────────────
├── Stream View Mode ▶
│   ├── Thumbnails
│   ├── Buffer/Frame Sizes
│   ├── B-Pyramid (Hierarchy)
│   ├── Metrics (PSNR/SSIM)
│   └── Active/DPB References
├── ─────────────────────
├── Y Component Only       (Y key)
├── U Component Only       (U key)
├── V Component Only       (V key)
├── YUV Combined           (reset)
├── ─────────────────────
├── Zoom In                (+)
├── Zoom Out               (-)
├── Fit to Window          (0)
└── Full Screen            (F)
```

#### YUVDiff 메뉴
```
YUVDiff
├── Open Debug YUV...
├── Close Debug YUV
├── ─────────────────────
├── Display Mode ▶
│   ├── Decoded (bitstream only)
│   ├── Debug YUV
│   ├── Difference (|decoded - debug|)
│   └── Amplified Difference
├── ─────────────────────
├── Bit Depth ▶
│   ├── Match Stream
│   ├── 8-bit
│   ├── 10-bit
│   ├── 12-bit
│   └── 16-bit (max)
├── ─────────────────────
├── Picture Offset...      → 프레임 오프셋 다이얼로그
├── Crop Values...         → 크롭 설정 다이얼로그
├── Auto-reload            (Toggle)
├── ─────────────────────
├── Calculate PSNR         → 실시간 PSNR/SSIM 계산
└── Show First Difference  → 첫 불일치 프레임으로 이동
```

#### Options 메뉴
```
Options
├── Color Conversion ▶
│   ├── ITU-R BT.601
│   ├── ITU-R BT.709      (기본값)
│   └── ITU-R BT.2020
├── ─────────────────────
├── Loop Playback          (Toggle)
├── ─────────────────────
├── CPU Optimizations ▶
│   ├── Auto-detect (default)
│   ├── SSE2 only
│   ├── SSSE3
│   ├── SSE4.1
│   └── AVX2
├── ─────────────────────
├── HEVC Extensions ▶
│   ├── RExt (Range Extensions)
│   ├── SCC (Screen Content Coding)
│   └── SHVC (Scalable)
├── ─────────────────────
├── Digest Calculation ▶
│   ├── As in Bitstream
│   ├── Always Calculate
│   └── Skip
├── ─────────────────────
├── VVC Options ▶
│   ├── Dynamic Selection Info
│   └── Detail Popup Windows
└── JPEG XS Options ▶
    └── Reference CFA Pattern...
```

#### Help 메뉴
```
Help
├── User Guide (F1 — 별도 창)
├── Keyboard Shortcuts...
├── ─────────────────────
├── About VQ Analyzer...
├── ─────────────────────
├── Activation...
├── Deactivate License
└── License Server...
```

### 2.3 툴바 구성

```
[Open ▼] [Close]  |  [⏮ First][← Prev][→ Next][⏭ Last]  |  [F1][F2][F3][F4][F5][F6]...[F12]  |  [Y][U][V]  |  [QP][HM][MV][...]  |  [Zoom- Fit Zoom+]
```

### 2.4 색상 / 시각적 스타일 가이드

| 요소 | 색상 |
|------|------|
| I-frame (Stream View) | 빨강 (#FF4444) |
| P-frame | 파랑 (#4488FF) |
| B-frame | 초록 (#44BB44) |
| 장기 참조 프레임 화살표 | 초록 |
| CVS 경계 밴드 | 연초록 반투명 |
| QP Heatmap (낮음→높음) | 파랑 → 초록 → 노랑 → 빨강 (jet colormap) |
| MV Heat (크기) | 흑백 → 파랑 → 빨강 |
| 블록 경계 | 흰색 (얇은 선, 1px) |
| CTU 경계 | 노란색 (2px) |
| 선택된 블록 하이라이트 | 노란색 반투명 채움 + 빨간 경계 |
| PSNR Heatmap (높음→낮음) | 파랑 → 빨강 (역방향) |

---

## 3. Supported Codecs & Containers

### 3.1 코덱 지원 매트릭스

| 코덱 | 파싱 | 디코딩 | 코덱별 특수 기능 | Bitvue 현황 |
|------|------|-------|----------------|------------|
| **HEVC / H.265** | ✅ | ✅ (FFmpeg) | RExt, SCC, SHVC, MCTS, SEI | ✅ 구현됨 |
| **VVC / H.266** | ✅ | ⚠️ (vvdec 미연결) | Dual-tree, LMCS, ALF, VPS, APS | ⚠️ 파싱만 |
| **AV1** | ✅ | ✅ (dav1d) | Superblock, tile, film grain, CDEF, loop restore | ✅ 구현됨 |
| **VP9** | ✅ | ✅ (FFmpeg) | Superframe, segments, prob tables | ✅ 구현됨 |
| **AVC / H.264** | ✅ | ✅ (FFmpeg) | MB types, CAVLC/CABAC, FMO, ASO | ✅ 구현됨 |
| **MPEG-2 Video** | ✅ | ⚠️ (FFmpeg, 미연결) | I/P/B, slices, QM | ⚠️ 파싱만 |
| **AVS3** | ❌ | ❌ | ESAO, CCSAO (AV1 유사 엔트로피) | ❌ 미구현 |
| **JPEG XS** | ❌ | ❌ | Precinct, NLT, MCT, wavelet | ❌ 미구현 |
| **VC-3 / DNxHD** | ❌ | ❌ | Intra-only, segment structure | ❌ 미구현 |
| **APV** | ❌ | ❌ | Apple ProRes 기반 신규 포맷 | ❌ 미구현 |
| **AVM** | ❌ | ❌ | AV2 실험용 코덱 (libaom 기반) | ❌ 미구현 |
| **AV3** | ⚠️ (실험적) | ❌ | 차세대 AOM 코덱 | ⚠️ 실험적 |

### 3.2 컨테이너 지원 매트릭스

| 컨테이너 | 지원 코덱 | Bitvue 현황 |
|---------|---------|-----------|
| **IVF** | AV1, VP9 | ✅ 구현됨 |
| **MKV / WebM** | 모든 코덱 | ✅ 구현됨 |
| **MP4 / MOV** | H.264, HEVC, AV1, VP9, MPEG-4 | ✅ 구현됨 |
| **MPEG-2 TS** | MPEG-2, H.264, HEVC | ✅ 구현됨 |
| **MPEG-2 PS** | MPEG-2, H.264 | ⚠️ 부분 |
| **AVI** | H.264, MPEG-2 | ❌ 미구현 |
| **MXF** | ProRes, DNxHD, JPEG XS 등 방송 | ❌ 미구현 |
| **MMT** | HEVC (방송용) | ❌ 미구현 |
| **Annex B (raw)** | HEVC, AVC, VVC | ✅ 구현됨 |
| **OBUS (raw)** | AV1 | ✅ 구현됨 |

### 3.3 코덱별 특수 기능 상세

#### HEVC 특수 기능
- **RExt (Range Extensions)**: 4:2:2, 4:4:4, 고비트뎁스(12/16bit), 비직교 변환
- **SCC (Screen Content Coding)**: Intra BC, ACT (Adaptive Color Transform), Palette Mode
- **SHVC (Scalable)**: 레이어 구조, inter-layer 예측, scalability_mask
- **MCTS (Motion Constrained Tile Sets)**: 타일 의존성 없는 인코딩
- SEI: 모든 SEI 메시지 파싱 및 표시

#### VVC 특수 기능
- **Dual-tree partitioning**: 루마/크로마 별도 CU 트리
- **LMCS**: 루마 매핑 + 크로마 스케일링 (역방향 매핑 시각화 필요)
- **ALF (Adaptive Loop Filter)**: 픽셀별 필터 파라미터
- **CCLM**: 크로스-컴포넌트 선형 모델 예측
- **IBC**: Intra Block Copy (SCC 유사)
- **Subpictures**: VVC 전용 서브픽처 구조
- **APS (Adaptation Parameter Sets)**: ALF/LMCS/QM별 APS
- **Multiple Ref Lines (MRL)**: 다중 참조 라인 인트라 예측

#### AV1 특수 기능
- **Superblock 128x128**: 최대 파티션 단위
- **Film Grain**: 영화 그레인 필터 (합성 전/후 비교)
- **CDEF**: Constrained Directional Enhancement Filter
- **Loop Restoration**: Wiener / Self-guided 필터
- **SuperRes**: 다운스케일 + 업스케일 (초해상도 유사)
- **Tile Groups**: 병렬 타일 디코딩
- **Show Existing Frame**: 캐시된 프레임 재사용
- **Intra BC**: 화면 내 복사 예측

#### VP9 특수 기능
- **Superframe Index**: 다중 프레임 패킹
- **Probability Tables**: 컨텍스트 확률 히스토리 (Counts 탭)
- **Segment Map**: 8 세그먼트별 QP/필터 오버라이드
- **Frame Parallel Decoding**: 프레임 병렬화 메타데이터

---

## 4. Detailed Feature Breakdown

### 4.1 Stream View (필름스트립)

| 항목 | 우선순위 | UI 위치 | Rust 해야 할 일 | Frontend 해야 할 일 | Parity 검증 포인트 |
|------|---------|--------|----------------|-------------------|----------------|
| **Thumbnail 스트립** | Must-have | 좌측 세로 패널 | 썸네일 JPEG 생성 (디코딩 기반) | VirtualizedThumbnailsView, I/P/B 배지 | 프레임 타입 색상 (I=빨강, P=파랑, B=초록) |
| **Frame 타입 배지** | Must-have | 각 썸네일 위 | FrameInfo.frame_type 파싱 | 배지 컴포넌트 | VQ Analyzer 색상과 위치 매칭 |
| **Buffer/Frame Sizes** | Must-have | 뷰 전환 | 프레임 바이트 크기 누적 | 세로 바 차트, 이동 평균 겹치기 | Y축 스케일, 바 색상 |
| **B-Pyramid (Hierarchy)** | Must-have | 뷰 전환 | GOP 구조 / 계층 레벨 파싱 | 다이아몬드/박스 트리 레이아웃 | 계층 깊이, 화살표 방향 |
| **HRD Buffer 그래프** | Important | 뷰 전환 | CPB 점유도 계산 (HRD) | 시계열 라인 차트 | 버퍼 점유 패턴 |
| **Metrics (PSNR/SSIM)** | Important | 뷰 전환 | YUV diff → PSNR/SSIM 계산 | 이중 라인 차트 | PSNR 값 ±0.1dB 허용 오차 |
| **Active/DPB References** | Important | 뷰 전환 | DPB 상태 추출 | 참조 프레임 표시 | 장기/단기 참조 구분 |
| **CVS 경계 밴드** | Important | 썸네일 사이 | CVS 경계 프레임 감지 | 연초록 세로 구분선 | 원본과 위치 매칭 |
| **슈퍼프레임 그룹화** | Nice-to-have | VVC/AV1 전용 | 슈퍼프레임 감지 | 그룹 브래킷 렌더링 | VVC/AV1만 해당 |
| **오른쪽 클릭 추출** | Important | 썸네일 컨텍스트 메뉴 | 선택 프레임 YUV/PNG 저장 | ContextMenu 컴포넌트 | 저장 포맷 (.yuv, .png) |
| **Fields/Frames 보기** | Nice-to-have | 뷰 전환 | 인터레이스 필드 감지 | 필드 페어 표시 | MPEG-2 전용 |
| **Filter (Not Displayed)** | Nice-to-have | 뷰 상단 토글 | 비표시 프레임 마스킹 | 필터링 토글 버튼 | |

### 4.2 Main Panel (메인 뷰어)

| 항목 | 우선순위 | Rust 해야 할 일 | Frontend 해야 할 일 | Parity 검증 포인트 |
|------|---------|----------------|-------------------|----------------|
| **디코딩된 프레임 표시** | Must-have | YUV → RGB 변환 | Canvas 렌더링 | 색공간 변환 정확도 (BT.709) |
| **줌/패닝** | Must-have | - | mouse wheel zoom, drag pan | 줌 레벨 유지, 중심점 기준 |
| **전체화면 토글 (F키)** | Must-have | - | F 키 핸들러 | 전체화면 진입/해제 |
| **Y/U/V 컴포넌트 선택** | Must-have | 채널 분리 전달 | Y/U/V 키, 회색조 표시 | 채널 격리 정확도 |
| **오버레이 렌더링** | Must-have | 오버레이 데이터 제공 | Canvas 2D overlay pass | 블록 크기/위치 pixel-perfect |
| **블록 클릭 → 선택** | Must-have | 픽셀 → CU 좌표 매핑 | 클릭 이벤트 → 선택 상태 | 선택된 블록 하이라이트 |
| **픽셀값 툴팁** | Important | 픽셀 YUV/RGB 값 | 마우스 호버 → 툴팁 | YUV 값 정확도 |
| **오버레이 블록 툴팁** | Important | 블록 메타데이터 | 블록 호버 → QP/MV/모드 툴팁 | 값 정확도 |

### 4.3 Syntax Info 패널 (탭 구조)

#### HEVC Syntax 탭
| 탭 | 내용 | Parity 검증 포인트 |
|----|------|----------------|
| **NAL** | NAL 유닛 테이블 (타입, 레이어, 시간 ID, 크기, offset) | 모든 NAL 유닛 나열, 타입명 |
| **Header** | VPS/SPS/PPS 파싱 트리, SEI 메시지 | 모든 시퀀스 파라미터 값 |
| **Block Info** | 선택된 CU/TU 트리 (luma/chroma 분리) | 블록 크기, 분할 타입, 변환 크기 |
| **QM** | 양자화 매트릭스 테이블 (4x4, 8x8, 16x16, 32x32) | 매트릭스 값 정확도 |
| **Ref Lists** | L0/L1 참조 리스트, POC, 가중치/오프셋 | 참조 인덱스 매핑 |
| **Stats** | 픽처/신택스/스트림 통계, 파이/바 차트 | 통계 수치 정확도 |

#### VVC Syntax 탭
| 탭 | 내용 |
|----|------|
| **NAL** | NAL 유닛 (NALU 타입 + 레이어 + 시간 ID) |
| **SPS** | SPS 파라미터 전체 |
| **PPS** | PPS 파라미터 |
| **APS** | ALF/LMCS/QM APS |
| **Slice** | 슬라이스 헤더 |
| **CU** | 선택된 CU 트리 (dual-tree 표시) |
| **Ref Lists** | L0/L1/L2 |
| **Stats** | 통계 |

#### AV1 Syntax 탭
| 탭 | 내용 |
|----|------|
| **IVF** | IVF 헤더, 프레임 크기 |
| **OBU** | OBU 타입/크기/오프셋 |
| **Sequence** | Sequence Header OBU 파라미터 |
| **Frame** | Frame Header OBU 파라미터 |
| **Block** | 선택된 수퍼블록/블록 신택스 |
| **Refs** | 참조 프레임 정보 |
| **Stats** | 통계 |

#### VP9 Syntax 탭
| 탭 | 내용 |
|----|------|
| **MKV** | MKV 클러스터/블록 정보 |
| **Frame** | 압축/비압축 헤더 |
| **Probabilities** | 컨텍스트 확률 테이블 |
| **Counts** | 심볼 카운트 (적응형 확률) |
| **Refs** | 참조 프레임 |
| **Block** | 선택된 블록 신택스 |
| **TX** | 변환 타입/크기 |
| **Stats** | 통계 |

#### AVC Syntax 탭
| 탭 | 내용 |
|----|------|
| **NAL** | NAL 유닛 테이블 |
| **Header** | SPS/PPS/SEI |
| **QM** | 양자화 매트릭스 |
| **Ref Lists** | 참조 리스트 |
| **Stats** | 통계 |

### 4.4 코덱별 F키 모드 매핑 (핵심)

이것이 현재 Bitvue와 VQ Analyzer의 가장 큰 차이점입니다. VQ Analyzer는 코덱별로 F키 매핑이 다릅니다.

#### HEVC F키 모드
| F키 | 모드 이름 | 설명 | Parity 우선순위 |
|-----|---------|------|--------------|
| F1 | **Coding Flow** | CTU 분할 트리, CU/TU/PU 경계 표시 | Must-have |
| F2 | **Predictions** | 인트라 모드 화살표, 인터 MV 화살표, 참조 인덱스 | Must-have |
| F3 | **Transform** | TU 경계, 변환 계수 (0=검정, 非0=흰색), RDOQ 효과 | Must-have |
| F4 | **Reconstruction** | 예측 + 잔차 재구성 단계 시각화 | Important |
| F5 | **Loop Filter** | 디블로킹 필터 경계 강도 (BS=0,1,2 색상) | Important |
| F6 | **SAO** | SAO 타입별 색상 (Edge=파랑, Band=빨강, None=회색) | Important |
| F7 | **YUV** | 순수 디코딩 결과 (오버레이 없음) | Must-have |
| — | **QP Map** (Info Overlay) | 블록별 QP 히트맵 | Must-have |
| — | **Heat Map** (Info Overlay) | 코딩 비용(비트) 히트맵 | Must-have |
| — | **MV Heat** (Info Overlay) | MV 크기 히트맵 | Important |
| — | **PU Type** (Info Overlay) | PU 타입별 색상 | Important |
| — | **PU Reference Indices** (Info Overlay) | 참조 인덱스 색상 | Important |
| — | **PSNR** (Info Overlay) | 블록별 PSNR 히트맵 | Important |
| — | **SSIM** (Info Overlay) | 블록별 SSIM | Nice-to-have |
| — | **Simple Motion** | 단순화된 MV 시각화 | Nice-to-have |

#### VVC F키 모드
| F키 | 모드 이름 | HEVC 대비 차이점 |
|-----|---------|--------------|
| F1 | **Dual Tree** | 루마/크로마 별도 파티션 트리 표시 |
| F2 | **Coding Flow** | HEVC Coding Flow와 유사 |
| F3 | **Predictions** | CCLM, MRLP, IBC, 기하학적 파티션(GPM) 포함 |
| F4 | **Transform** | 다중 변환 선택(MTS), LFNST 포함 |
| F5 | **Reconstruction** | LMCS 역 매핑 포함 |
| F6 | **Inverse Map** | LMCS 역 매핑 시각화 (VVC 전용) |
| F7 | **Loop Filter** | DBF 경계 강도 |
| F8 | **SAO** | SAO (HEVC와 동일 구조) |
| F9 | **Adaptive Filter (ALF)** | ALF 파라미터 시각화 (VVC 전용) |
| F10 | **YUV** | 순수 디코딩 결과 |
| — | **QP Map** | 블록별 QP |
| — | **Heat Map** | 비트 히트맵 |
| — | **Inter Memory Reads** | inter 예측 메모리 접근 패턴 (VVC 전용) |

#### AV1 F키 모드
| F키 | 모드 이름 | 특이사항 |
|-----|---------|--------|
| F1 | **Coding Flow** | 슈퍼블록 → 블록 분할 (쿼드/직사각형) |
| F2 | **Predictions** | 인트라 방향 모드 63종, 인터 MV (복합 예측 포함) |
| F3 | **Transform** | 변환 타입 (DCT/ADST/FLIPADST/ID) 색상 |
| F4 | **Reconstruction** | 복원 단계 시각화 |
| F5 | **Loop Filter** | 디블로킹 + CDEF + Loop Restoration |
| F6 | **CDEF Filter** | CDEF 방향/강도 시각화 (AV1 전용) |
| F7 | **SuperRes Filter** | 다운/업스케일 시각화 (AV1 전용) |
| F8 | **Loop Restoration** | Wiener/Self-guided 필터 (AV1 전용) |
| F9 | **Film Grain Pixels** | 필름 그레인 합성 전/후 비교 (AV1 전용) |
| F10 | **YUV** | 순수 디코딩 결과 |
| — | **Heat Map** | 비트 히트맵 |
| — | **Efficiency Map** | 단위 면적당 비트 효율 맵 |
| — | **Block Type** | 블록 타입별 색상 |
| — | **PSNR** | 블록별 PSNR |

#### VP9 F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Coding Flow** |
| F2 | **Predictions** |
| F3 | **Transform** |
| F4 | **Reconstruction** |
| F5 | **Loop Filter** |
| F6 | **YUV** |
| — | **Heat Map** |
| — | **Block Type** |
| — | **Efficiency Map** |
| — | **PSNR** |

#### AVC F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Coding Flow** |
| F2 | **Predictions** |
| F3 | **Transform** |
| F4 | **Reconstruction** |
| F5 | **Loop Filter** |
| F6 | **YUV** |
| — | **QP Map** |
| — | **Heat Map** |
| — | **MB Type** |
| — | **MB Reference Indices** |
| — | **PSNR** |

#### MPEG-2 F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Predictions** |
| F2 | **Transform** |
| F3 | **YUV** |
| — | **QP Map** |
| — | **Simple Motion** |

#### AVS3 F키 모드 (구현 필요)
| F키 | 모드 이름 | 특이사항 |
|-----|---------|--------|
| F1 | **Coding Flow** | CTU 구조 (HEVC 유사) |
| F2 | **Predictions** | |
| F3 | **Transform** | |
| F4 | **Reconstruction** | |
| F5 | **Loop Filter** | |
| F6 | **SAO** | |
| F7 | **ESAO** | Enhanced SAO (AVS3 전용) |
| F8 | **CCSAO** | Cross-Component SAO (AVS3 전용) |
| F9 | **YUV** | |

#### JPEG XS F키 모드 (구현 필요)
| F키 | 모드 이름 | 특이사항 |
|-----|---------|--------|
| F1 | **Precinct** | 프리신트 경계 시각화 |
| F2 | **Dequant** | 역양자화 계수 시각화 |
| F3 | **Transform** | 웨이블릿 변환 계수 |
| F4 | **MCT** | 다중 컴포넌트 변환 |
| F5 | **NLT** | 비선형 변환 |
| F6 | **YUV** | |

### 4.5 Selection Info 패널

블록을 클릭했을 때 표시되는 상세 정보:

```
┌─────────────────────────────────────────┐
│ Block Position: (x=64, y=128)           │
│ Block Size: 32x32                        │
│ Partition: SPLIT_QT → SPLIT_BT_H        │
│ ─────────────────────────────────────── │
│ Prediction Mode: INTER                  │
│ Reference L0: POC=5, idx=0              │
│ MV L0: (dx=+3.25, dy=-1.5) [1/4pel]    │
│ Reference L1: POC=8, idx=1              │
│ MV L1: (dx=-2.0, dy=+0.5)              │
│ ─────────────────────────────────────── │
│ QP: 28 (Chroma QP: 28)                  │
│ CBF: Y=1, Cb=0, Cr=0                    │
│ Transform: 16x16 (DCT-II)              │
│ ─────────────────────────────────────── │
│ RD Cost: 1024.5                         │
│ Bits: 48                                │
└─────────────────────────────────────────┘
```

### 4.6 Unit Info / HEX View 패널

```
Offset   00 01 02 03 04 05 06 07   ASCII
0x0000:  00 00 00 01 67 64 00 2A   ...gd.*
0x0008:  AC D9 40 4B FF FF FF E7   ..@K....
...
[강조: 현재 파싱 중인 비트 위치 하이라이트]
```

- 비트 오프셋 → 신택스 요소 매핑
- 클릭한 바이트 → 해당 NAL/OBU 요소로 Syntax 탭 포커스
- 양방향 연결: Syntax 트리에서 요소 클릭 → HEX에서 해당 바이트 하이라이트

### 4.7 YUVDiff / Debug YUV 모드 (상세)

```
┌────────────────────────────────────────────────┐
│ [Debug YUV 파일 로드]                           │
│ Format: Planar (YUV) / Interleaved (NV12)      │
│ Bit Depth: [8/10/12/16/Match Stream]           │
│ Display Order: [Yes/No]                        │
│ Crop: [L:0 R:0 T:0 B:0]                       │
│ Picture Offset: [+0]                           │
│ Auto-reload: [On/Off]                          │
│ ─────────────────────────────────────────────  │
│ PSNR Y: 43.21 dB                               │
│ PSNR U: 45.78 dB                               │
│ PSNR V: 45.12 dB                               │
│ PSNR Avg: 43.89 dB                             │
│ SSIM: 0.9921                                   │
│ ─────────────────────────────────────────────  │
│ [Show Decoded] [Show Debug YUV] [Show Diff]    │
│ [Amplified Diff ×10] [Find First Difference]  │
└────────────────────────────────────────────────┘
```

**구현 요구사항:**
- YUV 파일 포맷 자동 감지 (크기 기반 추론)
- 픽셀 단위 차이 계산 (|decoded[i] - reference[i]|)
- 차이 증폭 표시 (×2, ×4, ×8, ×16)
- 첫 불일치 프레임 자동 탐색
- 코딩/표시 순서 변환 후 비교

### 4.8 Stats 탭 (통계)

```
Picture Statistics:
  - 총 프레임 수 (I/P/B 각각)
  - 평균 QP (I/P/B)
  - 평균 프레임 크기 (I/P/B)
  
Coding Unit Statistics:
  - CU 크기 분포 파이 차트 (8x8, 16x16, 32x32, 64x64)
  - 인터/인트라 비율 파이 차트
  - 변환 크기 분포

Stream Statistics:
  - 총 비트수 / 헤더 / 데이터
  - 평균 비트레이트
  - 최대/최소 프레임 크기
```

---

## 5. Development Phases

### Phase 0: Project Setup & Foundation ✅ (이미 완료)

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

### Phase 1: 코덱별 F키 모드 분기 시스템 🔴 (핵심 갭)

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

### Phase 2: 코덱별 Info Overlay 토글 시스템 🔴

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

### Phase 3: VVC 전용 기능 완성 🟡

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

### Phase 4: AV1 전용 고급 모드 완성 🟡

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

### Phase 5: AVS3 지원 구현 🔴 (신규)

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

### Phase 6: JPEG XS + VC-3 + APV 지원 🟢 (Nice-to-have)

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

### Phase 7: YUVDiff 모드 완성 🟡

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

### Phase 8: Syntax 패널 완성 (코덱별 탭 상세화) 🟡

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

### Phase 9: 커맨드라인 인터페이스 (CLI) 강화 🟡

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

### Phase 10: 성능 최적화 & 대용량 스트림 지원 🟡

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

**예상 소요:** 중급 3~4주

---

### Phase 11: 폴리시, 키보드 단축키, 옵션 완성 🟢

- [ ] 모든 키보드 단축키 완성 (§6 참조)
- [ ] Options 메뉴 모든 항목 구현
- [ ] 창 레이아웃 저장/복원 (패널 크기, 위치)
- [ ] 최근 파일 목록 (최대 10개)
- [ ] 에러/경고 Status Panel 완성
- [ ] 접근성 (ARIA 레이블, 키보드 네비게이션)
- [ ] 다크/라이트 테마 완성
- [ ] 라이선스 활성화 시스템 (오픈소스이므로 생략 가능)

**예상 소요:** 중급 2~3주

---

### Phase 12: 전체 Parity 검증 & 테스트 🔴

- [ ] 공개 테스트 비트스트림으로 모든 모드 검증
- [ ] 자동화 스크린샷 비교 테스트
- [ ] 신택스 값 비교 테스트
- [ ] 회귀 테스트 스위트 구축

**예상 소요:** 중급 2~3주

---

## 6. Keyboard Shortcuts & Mouse Interactions

### 6.1 파일 조작
| 단축키 | 기능 |
|-------|------|
| `Ctrl+O` / `Cmd+O` | 파일 열기 |
| `Ctrl+W` / `Cmd+W` | 파일 닫기 |
| `Ctrl+R` | 파일 다시 열기 (Auto-reload) |

### 6.2 프레임 네비게이션
| 단축키 | 기능 |
|-------|------|
| `→` / `Space` | 다음 프레임 |
| `←` | 이전 프레임 |
| `Ctrl+→` | 다음 I프레임으로 이동 |
| `Ctrl+←` | 이전 I프레임으로 이동 |
| `Home` | 첫 프레임 |
| `End` | 마지막 프레임 |
| `Ctrl+G` / `Ctrl+F` | 특정 프레임 번호로 이동 다이얼로그 |

### 6.3 분석 모드 전환
| 단축키 | 기능 |
|-------|------|
| `F1` ~ `F12` | 코덱별 분석 모드 전환 (코덱 의존) |
| `Ctrl+F1` ~ `Ctrl+F6` | Info Overlay 토글 |

### 6.4 뷰어 조작
| 단축키/마우스 | 기능 |
|-------------|------|
| `Mouse Wheel` | 줌 인/아웃 (커서 위치 기준) |
| `Click + Drag` | 이동 (패닝) |
| `Double Click` | 줌 리셋 (Fit to Window) |
| `F` | 전체화면 토글 |
| `0` | 원본 크기 (100%) |
| `+` / `=` | 줌 인 |
| `-` | 줌 아웃 |
| `Y` | Y 채널만 표시 |
| `U` | U 채널만 표시 |
| `V` | V 채널만 표시 |
| `Escape` | 전체화면 해제 / 선택 해제 |

### 6.5 블록 선택
| 단축키/마우스 | 기능 |
|-------------|------|
| `Click` (Main Panel) | 블록 선택 → Selection Info 업데이트 |
| `Ctrl+Click` | 여러 블록 선택 (VVC Dynamic Selection) |
| `Escape` | 선택 해제 |

### 6.6 Stream View
| 단축키/마우스 | 기능 |
|-------------|------|
| `Click` (썸네일) | 해당 프레임으로 이동 |
| `Right Click` (썸네일) | 컨텍스트 메뉴 (추출 등) |
| `Scroll` | 필름스트립 스크롤 |

### 6.7 기타
| 단축키 | 기능 |
|-------|------|
| `Ctrl+Z` | 이전 선택으로 되돌아가기 |
| `Ctrl+C` | 선택 블록 정보 복사 |
| `Ctrl+S` | 현재 프레임 PNG 저장 |
| `Ctrl+E` | 내보내기 다이얼로그 |
| `F11` | 전체화면 (OS 수준) |
| `?` | 키보드 단축키 도움말 |

---

## 7. Parity Validation Strategy

### 7.1 테스트 비트스트림 세트

#### 공개 표준 테스트 시퀀스

| 코덱 | 소스 | URL |
|------|------|-----|
| HEVC | JCT-VC HM 공식 테스트 | MPEG/ITU-T FTP |
| VVC | VTM 공식 테스트 | https://vcgit.hhi.fraunhofer.de/jvet/VVCSoftware_VTM |
| AV1 | AOM test vectors | https://storage.googleapis.com/aom-test-data/ |
| VP9 | Chrome/WebM test vectors | https://chromium.googlesource.com/webm/vp9-test-vectors |
| AVC | JM/x264 테스트 | 자체 생성 권장 |
| MPEG-2 | MPEG 공식 | 자체 생성 권장 |
| AVS3 | AVS 공식 | http://www.avs.org.cn/ |

#### 추천 자체 생성 테스트 시퀀스
```bash
# HEVC 테스트 (x265)
ffmpeg -i input.mp4 -c:v libx265 -x265-params "ctu=64:qp=28" output_hevc.mkv

# AV1 테스트 (libaom)
ffmpeg -i input.mp4 -c:v libaom-av1 -cpu-used 4 output_av1.mkv

# VP9 테스트
ffmpeg -i input.mp4 -c:v libvpx-vp9 -b:v 2M output_vp9.webm

# AVC 테스트
ffmpeg -i input.mp4 -c:v libx264 -profile:v high output_avc.mp4
```

### 7.2 레이어별 검증 전략

#### Layer 1: 파싱 정확도 검증
```
목표: 신택스 값이 원본과 100% 일치

방법:
1. 공개 레퍼런스 디코더로 동일 파일 파싱 (HM, VTM, dav1d 등)
2. 핵심 신택스 요소 추출 (QP, MV, 분할 등) → JSON
3. Bitvue 출력 JSON과 diff 비교
4. 허용 오차: 0% (파싱은 정확해야 함)

자동화:
cargo test --test syntax_parity -- --test-threads=1
```

#### Layer 2: 오버레이 시각적 검증
```
목표: 오버레이 색상/위치가 VQ Analyzer와 pixel-near 일치

방법:
1. 동일 프레임에서 VQ Analyzer 스크린샷 캡처
2. 동일 프레임에서 Bitvue 스크린샷 캡처
3. 픽셀 단위 비교 (허용 오차: ±2 RGB값, 경계 ±1px)

자동화 (향후):
scripts/parity_screenshot_compare.py
```

#### Layer 3: 값 정확도 검증
```
목표: PSNR/SSIM/QP 등 수치가 원본과 일치

방법:
1. Debug YUV 모드에서 PSNR 계산
2. ffmpeg psnr 필터 결과와 비교
3. 허용 오차: ±0.01dB (PSNR), ±0.0001 (SSIM)
```

#### Layer 4: UX 동작 검증
```
목표: 클릭/키보드/줌 동작이 원본과 동일

방법:
1. 체크리스트 기반 수동 테스트
2. Playwright E2E 테스트 (자동화)
```

### 7.3 Parity 검증 체크리스트 (릴리스 전 필수)

```
HEVC Parity:
[ ] NAL 유닛 개수 및 타입 일치
[ ] SPS/PPS 파라미터 값 일치
[ ] 슬라이스 헤더 값 일치
[ ] CU/TU/PU 분할 트리 구조 일치
[ ] QP Map 히트맵 색상 일치
[ ] MV 벡터 크기/방향 일치 (HEVC PU 단위)
[ ] SAO 타입/파라미터 일치
[ ] 디블로킹 경계 위치 일치
[ ] Stats 탭 수치 일치

VVC Parity:
[ ] Dual Tree 루마/크로마 경계 일치
[ ] LMCS APS 파라미터 일치
[ ] ALF 파라미터 일치
[ ] CCLM 예측 파라미터 표시

AV1 Parity:
[ ] OBU 타입/크기 일치
[ ] Superblock 분할 구조 일치
[ ] CDEF 방향/강도 일치
[ ] Loop Restoration 타입 일치
[ ] Film Grain 파라미터 표시 일치

VP9 Parity:
[ ] 프레임 헤더 파라미터 일치
[ ] 세그먼트 맵 일치
[ ] 확률 테이블 초기값 일치
```

---

## 8. Next Steps & Recommendations

### 8.1 즉시 시작해야 할 작업 (우선순위 순)

#### 1순위: 코덱별 F키 모드 분기 시스템 (Phase 1)
**이유:** 현재 가장 큰 UX 차이. 고정 6모드 → 코덱별 동적 모드로 교체하지 않으면
이후 모든 코덱별 기능 추가가 어색해짐. 아키텍처 기반을 먼저 잡아야 함.

#### 2순위: VVC 디코딩 연결 (Phase 3 일부)
**이유:** 파싱은 완료됐지만 디코딩이 없으면 Main Panel에 프레임을 표시할 수 없음.
vvdec-sys 바인딩이 가장 높은 임팩트.

#### 3순위: AVS3 코덱 신규 구현 (Phase 5)
**이유:** VQ Analyzer가 지원하는데 Bitvue가 전혀 없는 코덱. 시작이 늦을수록 기술 부채.

#### 4순위: YUVDiff 모드 완성 (Phase 7)
**이유:** 코덱 디버깅 시 가장 많이 쓰는 기능. 현재 인프라는 있지만 UX 완성도 부족.

### 8.2 아키텍처 확장성 가이드

#### 새 코덱 추가 방법 (표준 패턴)
```
1. crates/bitvue-{codec}/ 크레이트 생성
2. bitvue-codecs/src/lib.rs에 re-export 추가
3. CodecModeRegistry에 F키 매핑 등록
4. bitvue-decode에 디코더 연결
5. Tauri 커맨드에 코덱 분기 추가
6. Frontend: 코덱별 Syntax 탭 컴포넌트 추가
7. Frontend: 코덱별 오버레이 렌더러 추가
```

#### 플러그인 시스템 (장기 목표)
```rust
// 코덱 플러그인 트레이트 (향후)
pub trait CodecPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn can_parse(&self, data: &[u8]) -> bool;
    fn parse_stream(&self, data: &[u8]) -> Result<StreamInfo>;
    fn get_modes(&self) -> Vec<AnalysisModeInfo>;
    fn get_overlay_data(&self, frame: usize, mode: AnalysisMode) -> Result<OverlayData>;
}
```

### 8.3 추가 필요 자료

| 자료 | 목적 | 소스 |
|------|------|------|
| ITU-T H.265 스펙 (HEVC) | RExt/SCC/SHVC 구현 | ITU-T 공식 사이트 |
| ITU-T H.266 스펙 (VVC) | dual-tree, LMCS, ALF | ITU-T 공식 사이트 |
| AV1 스펙 | CDEF, Film Grain 세부 | https://aomedia.org/av1-bitstream-and-decoding-process-specification/ |
| AVS3 스펙 | ESAO, CCSAO | http://www.avs.org.cn/AVS3/ |
| JPEG XS 스펙 | precinct, NLT | ISO/IEC 21122 |
| HM reference encoder | HEVC 테스트 비트스트림 생성 | https://hevc.hhi.fraunhofer.de/svn/svn_HEVCSoftware/ |
| VTM reference encoder | VVC 테스트 비트스트림 | https://vcgit.hhi.fraunhofer.de/jvet/VVCSoftware_VTM |
| VQ Analyzer 원본 | 시각적 매칭 기준 | 트라이얼 버전 다운로드 |

### 8.4 WebAssembly 확장 준비

```toml
# Cargo.toml (장기 목표)
[features]
wasm = ["wasm-bindgen", "web-sys"]
tauri = ["tauri", "tauri-plugin-dialog"]
```

WebAssembly 지원 시 고려사항:
- FFmpeg 대신 Rust-native 디코더 필수 (wasm-bindgen 호환)
- `dav1d`는 WASM 지원 있음 (dav1d.js)
- 파일 시스템 접근 → Web File API 대체

### 8.5 현재 진행률 요약

| 카테고리 | 완성도 | 주요 갭 |
|---------|--------|--------|
| 코어 파싱 | 85% | AVS3, JPEG XS, VC-3, APV 미구현 |
| 디코딩 | 70% | VVC, MPEG-2, AVS3 연결 미완 |
| UI 프레임워크 | 80% | 코덱별 F키 분기 없음 |
| 오버레이 시스템 | 65% | 코덱별 전용 오버레이 미완 |
| Syntax 패널 | 70% | Stats 탭, QM 탭, VP9 Prob 탭 미완 |
| YUVDiff 모드 | 50% | UI 완성도 부족 |
| CLI | 60% | VQ Analyzer CLI 옵션 일부 미구현 |
| 테스트/검증 | 40% | 자동화 비교 테스트 부족 |
| **전체** | **~68%** | |

---

## 부록 A: 오버레이 색상 스케일 기준

### Jet Colormap (QP Map, Heat Map 기본)
```
value: 0.0  → #0000FF (파랑)
value: 0.25 → #00FFFF (시안)
value: 0.5  → #00FF00 (초록)
value: 0.75 → #FFFF00 (노랑)
value: 1.0  → #FF0000 (빨강)
```

### MV 크기 히트맵
```
크기 0 (정지) → 검정 #000000
크기 소 → 파랑 #0000FF
크기 중 → 초록 #00FF00
크기 대 → 빨강 #FF0000
```

### HEVC 루프 필터 경계 강도
```
BS=0 → 표시 안 함
BS=1 → 파랑 (약한 필터)
BS=2 → 빨강 (강한 필터)
```

### SAO 타입
```
Edge Filter → 파랑 #4488FF
Band Filter → 빨강 #FF4444
None → 회색 #888888
```

### AV1 Loop Restoration
```
WIENER → 파랑 #4488FF
SGRPROJ (Self-guided) → 초록 #44BB44
NONE → 회색 #888888
```

---

## 부록 B: 중요 Tauri 커맨드 추가 목록 (신규 필요)

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

*이 문서는 Bitvue v0.12.0 기준으로 작성되었습니다. VQ Analyzer User Guide의 모든 기능을 포함하며, 개발 진행에 따라 업데이트가 필요합니다.*
