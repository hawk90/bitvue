# Bitvue

**Video Bitstream Analyzer** - A tool for analyzing compressed video bitstreams across multiple modern codecs.

[![CI](https://github.com/hawk90/bitvue/workflows/CI/badge.svg)](https://github.com/hawk90/bitvue/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/hawk90/bitvue/graph/badge.svg)](https://codecov.io/gh/hawk90/bitvue)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org)
[![GitHub stars](https://img.shields.io/github/stars/hawk90/bitvue?style=social)](https://github.com/hawk90/bitvue)

---

## 🎉 Version 0.10.0

**Bitvue 0.10.0** - Video bitstream analyzer supporting multiple modern codecs.

### Supported Codecs

| Codec | Status | Features |
|-------|--------|----------|
| **AVC/H.264** | ✅ Full | NAL parsing, slice analysis, deblocking |
| **HEVC/H.265** | ✅ Full | CTU parsing, SAO, transform units |
| **VP9** | ✅ Full | Superblock parsing, loop filter |
| **VVC/H.266** | ✅ Full | Dual-tree, ALF, SAO, MIP, GPM, SbTMVP |
| **AV1** | ✅ Full | OBU parsing, CDEF, Loop Restoration, Film Grain |
| **AV3** | ✅ Experimental | OBU parsing, basic analysis |

---

## Features

### Analysis Modes (F1-F7)

- **F1: Overview** - Stream information and statistics
- **F2: Coding Flow** - Encoder/decoder pipeline visualization
- **F3: Prediction** - Intra/Inter prediction modes and motion vectors
- **F4: Transform** - Transform coefficient visualization
- **F5: QP Map** - Quantization parameter heatmap
- **F6: MV Field** - Motion vector field display
- **F7: Reference Frames** - Frame dependency graph

### Filmstrip Visualizations

1. **Thumbnails** - Frame thumbnails with type indicators (I/P/B)
2. **Frame Sizes** - Bar chart with moving average overlay
3. **B-Pyramid** - GOP structure visualization
4. **HRD Buffer** - Buffer occupancy graph
5. **Enhanced** - Multi-metric overlay with GOP/Scene navigation

### Core Functionality

- **Syntax Tree Visualization** - Navigate bitstream hierarchy
- **Hex View** - Raw byte inspection with syntax highlighting
- **Frame Analysis** - QP heatmaps, MV overlays, partition grids
- **YUV Viewer** - Raw pixel data inspection
- **Export Data** - CSV/JSON export and report generation
- **A/B Comparison** - Side-by-side stream comparison

---

## Quick Start

### Prerequisites

- **Node.js** 18+ and npm
- **Rust** 1.70+ and Cargo
- System dependencies based on your OS:
  ```bash
  # macOS
  brew install dav1d

  # Ubuntu/Debian
  sudo apt install libdav1d-dev libwebkit2gtk-4.0-dev build-essential

  # Fedora
  sudo dnf install dav1d-devel webkit2gtk4.0-devel
  ```

### Installation

```bash
# Clone the repository
git clone https://github.com/user/bitvue.git
cd bitvue

# Install dependencies
npm install

# Build the application
npm run tauri build

# Run in development mode
npm run tauri dev
```

### Basic Usage

1. **Launch Bitvue** - Double-click the application or run `npm run tauri dev`
2. **Open a video** - Click "Open Bitstream" or press `Ctrl/Cmd+O`
3. **Navigate frames** - Use arrow keys or click on filmstrip thumbnails
4. **Switch modes** - Press F1-F7 for different analysis views
5. **Export data** - Use File → Export to save analysis results

### Supported File Formats

| Container | Codecs |
|-----------|--------|
| `.ivf` | AV1, VP9, AV3 |
| `.webm` | VP9, AV1 |
| `.mkv` | HEVC, VP9, AV1, VVC |
| `.mp4`, `.mov` | AVC, HEVC, VVC, AV1, AV3 |
| `.hevc`, `.h265` | HEVC |
| `.avc`, `.h264` | AVC |
| `.vvc`, `.h266` | VVC |

---


---

## Architecture

```
bitvue/
├── crates/
│   ├── bitvue-core/       # Core types and business logic
│   ├── bitvue-codecs/      # Codec abstractions
│   ├── bitvue-formats/     # Container parsers (IVF, MP4, MKV, TS)
│   ├── bitvue-decode/      # Decoder bindings (dav1d for AV1)
│   ├── bitvue-metrics/     # Quality metrics (PSNR, SSIM)
│   ├── bitvue-av1/         # AV1 OBU parser
│   ├── bitvue-avc/         # AVC/H.264 parser
│   ├── bitvue-hevc/        # HEVC/H.265 parser
│   ├── bitvue-vp9/         # VP9 parser
│   ├── bitvue-vvc/         # VVC/H.266 parser
│   ├── bitvue-av3/         # AV3 parser
│   ├── ui/                  # egui UI (legacy reference)
│   └── app/                # eframe/egui shell
├── src/                    # Tauri + React frontend
│   ├── components/         # React components
│   ├── contexts/           # React contexts
│   ├── utils/              # TypeScript utilities
│   └── test/               # Test files and fixtures
└── src-tauri/              # Rust backend (Tauri)
    ├── src/
    │   ├── commands/       # Tauri IPC commands
    │   └── services/       # Backend services
    └── Cargo.toml
```

---

## Development

### Running Tests

```bash
# Run all tests
npm test

# Run tests with UI
npm run test:ui

# Run tests with coverage
npm run test:coverage

# Run Rust tests
cargo test --workspace
```

### Code Quality

```bash
# Run lints
npm run lint
cargo clippy --workspace

# Format code
npm run format
cargo fmt --all

# Check license compliance
cargo deny check
```

### Git Hooks

```bash
# Install lefthook (Rust-based git hooks)
cargo install lefthook
lefthook install
```

Hooks auto-run on commit/push:
- **Rust**: cargo fmt, clippy, test
- **JS/TS/CSS**: prettier, eslint, vitest

---

## Keyboard Shortcuts

### Navigation
| Key | Action |
|-----|--------|
| `←` / `→` | Previous/Next frame |
| `Home` / `End` | First/Last frame |

### Modes
| Key | Action |
|-----|--------|
| `F1` | Overview mode |
| `F2` | Coding Flow |
| `F3` | Prediction |
| `F4` | Transform |
| `F5` | QP Map |
| `F6` | MV Field |
| `F7` | Reference Frames |

### Filmstrip
| Key | Action |
|-----|--------|
| `1-5` | Switch filmstrip visualization mode |

### Other
| Key | Action |
|-----|--------|
| `Ctrl/Cmd+O` | Open file |
| `Ctrl/Cmd+W` | Close file |
| `Ctrl/Cmd+E` | Export data |
| `?` | Show shortcuts |

---

## Contributing

Contributions are welcome! Please read our [contributing guidelines](CONTRIBUTING.md) before submitting PRs.

---

## Release & Downloads

### Downloads

Get the latest release for your platform:
- **Windows**: `Bitvue-1.0.0-setup.exe`
- **macOS (Intel)**: `Bitvue-1.0.0-x86_64.dmg`
- **macOS (Apple Silicon)**: `Bitvue-1.0.0-aarch64.dmg`
- **Linux**: `bitvue_1.0.0_amd64.deb` or `bitvue-1.0.0-x86_64.AppImage`

See [Releases](https://github.com/user/bitvue/releases) for all downloads.

---

## Security

🔒 Report security vulnerabilities responsibly: See [SECURITY.md](SECURITY.md)

---

## License

This project is licensed under **GNU Affero General Public License v3.0** - see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- [dav1d](https://code.videolan.org/videolan/dav1d) - AV1 decoder
- [Tauri](https://tauri.app/) - Cross-platform desktop framework
- [React](https://react.dev/) - UI framework

---

*Bitvue 0.10.0 - Video Bitstream Analysis*
