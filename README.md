<div align="center">

  <!-- Logo/Icon -->
  <a name="readme-top"></a>
  <img src="resources/com.github.bitvue.svg" alt="Bitvue" width="120" height="120">

  # Bitvue

  ### **Video Bitstream Analyzer**

  *[Multi-codec analysis tool for inspecting compressed video bitstreams]*

  <!-- Badges -->
  [![CI](https://img.shields.io/github/actions/workflow/status/hawk90/bitvue/ci.yml?branch=main&logo=github-actions&logoColor=white&label=build)](https://github.com/hawk90/bitvue/actions/workflows/ci.yml)
  [![codecov](https://img.shields.io/codecov/c/github/hawk90/bitvue?logo=codecov&logoColor=F01F7A&token=XXXXX)](https://codecov.io/gh/hawk90/bitvue)
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
- **Quality Metrics** — PSNR, SSIM, VMAF calculation with BD-rate analysis for codec comparison
- **Syntax Navigation** — Full bitstream syntax tree with hex view and semantic highlighting
- **Cross-Platform** — Native desktop apps for Windows, macOS, and Linux

---

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/hawk90/bitvue.git
cd bitvue

# Install frontend dependencies
cd frontend && npm install

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

### Prerequisites

| Platform | Dependencies |
|----------|--------------|
| **macOS** | `brew install dav1d` |
| **Ubuntu/Debian** | `sudo apt install libdav1d-dev libwebkit2gtk-4.1-dev build-essential` |
| **Fedora** | `sudo dnf install dav1d-devel webkit2gtk4.1-devel` |
| **Windows** | [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |

### Basic Usage

1. **Launch Bitvue** — Double-click the application or run `npm run tauri:dev`
2. **Open a video** — Click "Open Bitstream" or press `Ctrl/Cmd+O`
3. **Navigate frames** — Use arrow keys or click on filmstrip thumbnails
4. **Switch modes** — Press F1-F7 for different analysis views
5. **Export data** — Use File → Export to save analysis results

---

## Supported Codecs

| Codec | Status | Format Support |
|-------|--------|----------------|
| **AV1** | ✅ Full | `.ivf`, `.webm`, `.mkv`, `.mp4` |
| **VVC/H.266** | ✅ Full | `.mkv`, `.mp4`, `.vvc`, `.h266` |
| **HEVC/H.265** | ✅ Full | `.mkv`, `.mp4`, `.hevc`, `.h265` |
| **VP9** | ✅ Full | `.ivf`, `.webm`, `.mkv` |
| **AVC/H.264** | ✅ Full | `.mp4`, `.mkv`, `.avc`, `.h264` |
| **AV3** | ⚠️ Experimental | `.ivf` |

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

Bitvue includes comprehensive quality metrics for codec comparison:

- **PSNR** — Peak Signal-to-Noise Ratio (Y, U, V, Average)
- **SSIM** — Structural Similarity Index (Y, U, V, Average)
- **VMAF** — Netflix's Video Multimethod Assessment Fusion
- **BD-Rate** — Bjøntegaard-Delta rate for RD curve comparison

```
Usage:
1. Open reference video (original)
2. Open distorted video (encoded)
3. Click "Calculate Metrics"
4. View frame-by-frame and averaged results
5. Export as CSV/JSON for further analysis
```

---

## Architecture

```
bitvue/
├── crates/
│   ├── bitvue/               # Main library facade (re-exports all)
│   ├── bitvue-codecs/        # Unified codec interface
│   ├── bitvue-core/         # Core types, state, caching
│   ├── bitvue-formats/      # Container parsers (IVF, MP4, MKV, TS)
│   ├── bitvue-decode/       # Decoder bindings (dav1d for AV1)
│   ├── bitvue-metrics/      # Quality metrics (PSNR, SSIM, VMAF)
│   ├── bitvue-cli/          # CLI tool
│   ├── bitvue-codecs-parser/ # Codec integration layer
│   ├── bitvue-mcp/          # Model Context Protocol server
│   ├── bitvue-benchmarks/   # Performance benchmarks
│   │   # Codec parsers
│   ├── bitvue-av1-codec/    # AV1 OBU parser
│   ├── bitvue-avc/          # AVC/H.264 parser
│   ├── bitvue-hevc/         # HEVC/H.265 parser
│   ├── bitvue-vp9/          # VP9 parser
│   ├── bitvue-vvc/          # VVC/H.266 parser
│   ├── bitvue-av3-codec/    # AV3 parser
│   ├── bitvue-mpeg2-codec/  # MPEG-2 parser
│   └── vendor/              # Third-party dependencies
│       └── abseil/          # Abseil logging library (private fork)
├── frontend/                # React application
│   ├── src/                 # Application source
│   ├── tests/               # Consolidated test files
│   ├── public/              # Static assets
│   └── index.html
├── src-tauri/               # Tauri backend (Rust)
│   ├── src/commands/        # Tauri IPC commands
│   └── src/services/        # Backend services
├── scripts/                 # Development scripts
│   ├── setup.sh
│   ├── dev.sh
│   └── clean.sh
└── config/                  # Tool configurations
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
- **Tauri 2.0** - Desktop framework
- **dav1d 1.4.0** - AV1 decoder

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
| **Quality Metrics** | ✅ PSNR/SSIM/VMAF | ✅ | ✅ PSNR/SSIM |
| **Modern UI** | ✅ React/Tauri | ⚠️ Qt | ⚠️ Qt |
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
# Test hook
