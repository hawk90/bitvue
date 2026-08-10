<div align="center">

  <!-- Logo/Icon -->
  <a name="readme-top"></a>
  <img src="resources/com.github.bitvue.svg" alt="Bitvue" width="120" height="120">

  # Bitvue

  ### **Video Bitstream Analyzer**

  *[Multi-codec analysis tool for inspecting compressed video bitstreams]*

  <!-- Badges -->
  [![CI](https://img.shields.io/github/actions/workflow/status/hawk90/bitvue/ci.yml?branch=main&logo=github-actions&logoColor=white&label=build)](https://github.com/hawk90/bitvue/actions/workflows/ci.yml)
  [![codecov](https://img.shields.io/codecov/c/github/hawk90/bitvue?logo=codecov&logoColor=F01F7A)](https://codecov.io/gh/hawk90/bitvue)
  [![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

  <!-- Stats -->
  [![Release](https://img.shields.io/github/v/release/hawk90/bitvue?logo=github&color=blue&label=latest)](https://github.com/hawk90/bitvue/releases)
  [![Downloads](https://img.shields.io/github/downloads/hawk90/bitvue/total?logo=github&color=success)](https://github.com/hawk90/bitvue/releases)
  [![Stars](https://img.shields.io/github/stars/hawk90/bitvue?logo=github&color=yellow)](https://github.com/hawk90/bitvue)
  [![Issues](https://img.shields.io/github/issues-raw/hawk90/bitvue?logo=github&color=important)](https://github.com/hawk90/bitvue/issues)

</div>

---

## Why Bitvue?

> Understand your video codecs at the bitstream level — with **visual clarity** and **depth**.

- **Multi-Codec Support** — Parse AV1, VVC/H.266, HEVC/H.265, VP9, AVC/H.264, and experimental AV3
- **Visual Analysis** — 7 analysis modes (F1-F7) with overlaid visualization on decoded frames
- **Filmstrip Views** — 5 visualization modes including GOP structure, frame sizes, and HRD buffer
- **Quality Metrics** — PSNR/SSIM built-in; VMAF behind an opt-in build feature; BD-rate curves in progress (see [Quality Metrics](#quality-metrics))
- **Syntax Navigation** — Full bitstream syntax tree with hex view and semantic highlighting
- **Cross-Platform** — Native desktop apps for Windows, macOS, and Linux (Electron + Rust sidecar)

---

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/hawk90/bitvue.git
cd bitvue

# One-time setup: installs Rust + Node dependencies for the frontend and the
# Electron shell (bitvue-desktop)
./scripts/setup.sh

# Run in development mode: builds the bitvue-sidecar Rust binary, then
# launches the Electron shell (spawns the sidecar, loads the frontend)
./scripts/dev.sh

# Build a distributable package for your platform
./scripts/package_electron.sh mac    # or: linux | win
```

Bitvue is a **desktop app split across two processes**: a React/TypeScript
frontend running inside Electron, talking over stdio to `bitvue-sidecar` — a
Rust binary that owns codec parsing, decoding, and all analysis logic. See
`docs/DEVELOPMENT_PHASES.md` for the full architecture writeup (migrated off
Tauri in August 2026).

### Prerequisites

| Platform | Dependencies |
|----------|--------------|
| **All platforms** | [Rust](https://rustup.rs/) (stable), [Node.js](https://nodejs.org/) 18+ |
| **macOS** | `brew install dav1d` |
| **Ubuntu/Debian** | `sudo apt install libdav1d-dev build-essential` |
| **Fedora** | `sudo dnf install dav1d-devel` |
| **Windows** | dav1d via [vcpkg](https://vcpkg.io/) or a prebuilt binary on `PATH` |

### Basic Usage

1. **Launch Bitvue** — run `./scripts/dev.sh`, or double-click the packaged app once built
2. **Open a video** — Click "Open Bitstream" or press `Ctrl/Cmd+O`
3. **Navigate frames** — Use arrow keys or click on filmstrip thumbnails
4. **Switch modes** — Press F1-F7 for different analysis views
5. **Export data** — Use File → Export to save analysis results

---

## Supported Codecs

Bitstream analysis (stream tree, syntax detail, hex view, QP/MV/partition extraction) works for
every codec below. Frame-accurate **video preview** (actual pixel decode) currently ships for AV1
only, via [dav1d](https://code.videolan.org/videolan/dav1d); HEVC/VP9/AVC preview goes through an
optional ffmpeg backend and VVC through an optional [vvdec](https://github.com/fraunhoferhhi/vvdec)
backend, both present in the codebase but not yet wired into the running app.

| Codec | Bitstream Analysis | Video Preview | Format Support |
|-------|---------------------|----------------|----------------|
| **AV1** | ✅ | ✅ (dav1d) | `.ivf`, `.webm`, `.mkv`, `.mp4` |
| **VVC/H.266** | ✅ | ❌ not yet wired | `.mkv`, `.mp4`, `.vvc`, `.h266` |
| **HEVC/H.265** | ✅ | ❌ not yet wired | `.mkv`, `.mp4`, `.hevc`, `.h265` |
| **VP9** | ✅ | ❌ not yet wired | `.ivf`, `.webm`, `.mkv` |
| **AVC/H.264** | ✅ | ❌ not yet wired | `.mp4`, `.mkv`, `.avc`, `.h264` |
| **AV3** | ⚠️ Experimental | ❌ | `.ivf` |

---

## Analysis Modes (F1-F7)

| Mode | Description | Features |
|------|-------------|----------|
| **F1: Overview** | Stream information and statistics | Codec profile, resolution, frame count, bitrate |
| **F2: Coding Flow** | Encoder/decoder pipeline visualization | Block partitioning, transform, quantization flow |
| **F3: Prediction** | Intra/Inter prediction modes | Prediction mode vectors, motion vectors |
| **F4: Transform** | Transform coefficient visualization | Coefficient heatmaps with block boundaries |
| **F5: QP Map** | Quantization parameter heatmap | Per-block QP values with color scale |
| **F6: MV Field** | Motion vector field display | Motion vector grid with magnitude indication |
| **F7: Reference** | Frame dependency graph | Reference frame relationships and DPB state |

---

## Filmstrip Visualizations

| Mode | Description |
|------|-------------|
| **1. Thumbnails** | Frame thumbnails with I/P/B type indicators |
| **2. Frame Sizes** | Bar chart with moving average overlay |
| **3. B-Pyramid** | GOP structure and hierarchical B-frames |
| **4. HRD Buffer** | Hypothetical Reference Decoder buffer occupancy |
| **5. Enhanced** | Multi-metric overlay with GOP/Scene navigation |

---

## Quality Metrics

| Metric | Status |
|--------|--------|
| **PSNR** — Peak Signal-to-Noise Ratio (Y, U, V, Average) | ✅ Built in |
| **SSIM** — Structural Similarity Index (Y, U, V, Average) | ✅ Built in |
| **VMAF** — Netflix's Video Multimethod Assessment Fusion | ⚠️ Implemented behind the optional `vmaf` Cargo feature (requires libvmaf installed on the system); not enabled in default builds |
| **BD-Rate** — Bjøntegaard-Delta rate for RD-curve comparison | ❌ Not yet implemented (an RD-curve panel exists in the codebase, but the calculation itself isn't wired up) |

---

## Architecture

Bitvue runs as **two processes**: an Electron shell hosting the React frontend, and
`bitvue-sidecar` — a standalone Rust binary owning all codec/decode/analysis logic — talking over a
stdio wire protocol (`bitvue-protocol`). This replaced a Tauri-based single-process architecture in
August 2026; see `docs/DEVELOPMENT_PHASES.md` for the full rationale and wire-protocol spec.

```
bitvue/
├── crates/
│   ├── bitvue/               # Main library facade (re-exports all)
│   ├── bitvue-engine/        # Core types, SelectionState, Command/Event bus, caches
│   ├── bitvue-protocol/      # Electron<->sidecar wire protocol (control/data plane framing)
│   ├── bitvue-sidecar/       # Standalone process hosting the engine, speaks bitvue-protocol
│   ├── bitvue-formats/       # Container parsers (IVF, MP4, MKV, TS)
│   ├── bitvue-codecs/        # Unified codec interface
│   ├── bitvue-codecs-parser/ # Codec parsers integration layer
│   ├── bitvue-decode/        # Pixel decoders (dav1d for AV1; ffmpeg/vvdec backends unwired)
│   ├── bitvue-metrics/       # Quality metrics (PSNR, SSIM; VMAF behind opt-in feature)
│   ├── bitvue-indexer/       # Metadata-indexing pipeline (container/units)
│   ├── bitvue-cli/           # CLI tool
│   ├── bitvue-mcp/           # Model Context Protocol server
│   ├── bitvue-benchmarks/    # Criterion-based performance benchmarks
│   │   # Codec parsers (pure Rust, bitstream syntax only)
│   ├── bitvue-av1-codec/     # AV1 OBU parser
│   ├── bitvue-avc/           # AVC/H.264 parser
│   ├── bitvue-hevc/          # HEVC/H.265 parser
│   ├── bitvue-vp9/           # VP9 parser
│   ├── bitvue-vvc/           # VVC/H.266 parser
│   ├── bitvue-av3-codec/     # AV3 parser
│   ├── bitvue-mpeg2-codec/   # MPEG-2 Video parser
│   ├── bitvue-avs3/          # AVS3/IEEE 1857.10 parser
│   ├── bitvue-jpegxs/        # JPEG XS (ISO 21122) parser
│   ├── bitvue-vc3/           # VC-3/DNxHD parser
│   └── vendor/               # Third-party dependencies
│       └── abseil/           # Abseil logging library (private fork)
├── frontend/                 # React application (Electron renderer)
│   ├── components/, hooks/, contexts/, services/, utils/
│   ├── tests/                # Consolidated test files
│   └── index.html
├── bitvue-desktop/           # Electron shell
│   ├── electron/             # Main process (window, native menu, sidecar lifecycle, IPC)
│   └── src/                  # Sidecar client, protocol codec
├── scripts/                  # Development scripts
│   ├── setup.sh, dev.sh, clean.sh
│   ├── parity_check.sh, run_regression_suite.sh
│   └── package_electron.sh   # Builds + packages the Electron app
└── config/                   # Tool configurations
    ├── clippy.toml
    ├── deny.toml
    └── codecov.yml
```

---

## Technology Stack

### Frontend
- **React 18** - UI framework
- **TypeScript 5** - Type safety
- **Vite 5** - Build tool

### Backend
- **Rust 1.70+** - Systems programming
- **Electron** - Desktop shell (frontend host + native menu/window)
- **dav1d** - AV1 decoder (via system package, e.g. Homebrew/apt)

### Infrastructure
- **GitHub Actions** - CI/CD
- **codecov** - Code coverage
- **Lefthook 2.0** - Git hooks

---

## Development

### Running Tests

```bash
# From workspace root
npm run test              # Run frontend tests
npm run test:coverage     # Run with coverage

# Run Rust tests
cargo test --workspace
```

### Code Quality

```bash
# Format all code
cargo fmt --all
npm run format

# Run linters
cargo clippy --workspace
npm run lint

# Check license compliance
cargo deny check
```

### Git Hooks

```bash
# Install lefthook (already configured in lefthook.toml)
cargo install lefthook
lefthook install
```

Hooks auto-run on commit/push:
- **Rust**: `cargo fmt`, `clippy`, `cargo test`
- **JS/TS/CSS**: `prettier`, `eslint`, `vitest`

---

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `←` / `→` | Previous/Next frame |
| `Home` / `End` | First/Last frame |

### Modes (F1-F7)

| Key | Mode |
|-----|------|
| `F1` | Overview |
| `F2` | Coding Flow |
| `F3` | Prediction |
| `F4` | Transform |
| `F5` | QP Map |
| `F6` | MV Field |
| `F7` | Reference Frames |

### Filmstrip (1-5)

| Key | Visualization |
|-----|---------------|
| `1` | Thumbnails |
| `2` | Frame Sizes |
| `3` | B-Pyramid |
| `4` | HRD Buffer |
| `5` | Enhanced |

### Other

| Key | Action |
|-----|--------|
| `Ctrl/Cmd+O` | Open file |
| `Ctrl/Cmd+W` | Close file |
| `Ctrl/Cmd+E` | Export data |
| `?` | Show shortcuts |

---

## Downloads

Get the latest release for your platform:

| Platform | Download |
|----------|----------|
| **Windows** | `Bitvue-x.x.x-setup.exe` |
| **macOS (Intel)** | `Bitvue-x.x.x-x86_64.dmg` |
| **macOS (Apple Silicon)** | `Bitvue-x.x.x-aarch64.dmg` |
| **Linux (Debian)** | `bitvue-x.x.x_amd64.deb` |
| **Linux (AppImage)** | `Bitvue-x.x.x-x86_64.AppImage` |

[Releases Page](https://github.com/hawk90/bitvue/releases)

---

## Comparison

| Feature | Bitvue | VQAnalyzer | GitlHEVCAnalyzer |
|---------|--------|------------|------------------|
| **AV1 Support** | ✅ | ❌ | ❌ |
| **VVC Support** | ✅ | ✅ | ❌ |
| **VP9 Support** | ✅ | ❌ | ❌ |
| **Open Source** | ✅ AGPL-3.0 | ❌ Proprietary | ✅ GPL-3.0 |
| **Cross-Platform** | ✅ Win/Mac/Linux | ✅ Win/Mac/Linux | ❌ Windows only |
| **Quality Metrics** | ✅ PSNR/SSIM, ⚠️ VMAF opt-in | ✅ | ✅ PSNR/SSIM |
| **Modern UI** | ✅ React/Electron | ⚠️ Qt | ⚠️ Qt |
| **Active Development** | ✅ | ✅ | ⚠️ Limited |

---

## Contributing

We welcome contributions! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.

**Development workflow:**

1. Fork the repository
2. Create a feature branch (`feat/your-feature`, `fix/your-bug`)
3. Make your changes with tests
4. Run `npm test` and `cargo clippy`
5. Submit a pull request

---

## Documentation

- [Getting Started Guide](#quick-start)
- [Analysis Modes](#analysis-modes-f1-f7)
- [Architecture](#architecture)
- [API Reference](crates/bitvue/)
- [Contributing Guidelines](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)

---

## License

This project is licensed under **GNU Affero General Public License v3.0** — see [LICENSE](LICENSE) for details.

<div align="center">

  **Built with ❤️ for the video codec community**

  [![ hawk90/bitvue ](https://img.shields.io/badge/GitHub-hawk90%2Fbitvue-blue?style=flat-square&logo=github)](https://github.com/hawk90/bitvue)

  ---

  [![Back to Top](https://img.shields.io/badge/⬆%20Back%20to%20Top-lightgrey?style=flat-square)](#readme-top)

</div>
