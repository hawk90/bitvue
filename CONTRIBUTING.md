# Contributing to Bitvue

Thank you for your interest in contributing to Bitvue! This document provides guidelines for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for details.

## Getting Started

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Node.js 20+ (for frontend development)
- System dependencies:
  - **macOS**: `brew install dav1d`
  - **Ubuntu/Debian**: `sudo apt install libdav1d-dev`
  - **Fedora**: `sudo dnf install dav1d-devel`

### Development Workflow

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/bitvue.git
   cd bitvue
   ```

2. **Install git hooks** (recommended)
   ```bash
   # Install lefthook (Rust-based git hooks)
   cargo install lefthook
   lefthook install
   # Hooks are configured in lefthook.toml
   ```

3. **Install frontend dependencies**
   ```bash
   cd frontend && npm install
   ```

4. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

5. **Make your changes**
   - Rust tests: `cargo test --workspace`
   - Frontend tests: `cd frontend && npm test`
   - Check formatting: `cargo fmt --all`
   - Run clippy: `cargo clippy --workspace -- -D warnings`
   - Frontend lint: `cd frontend && npm run lint`
   - Build: `cargo build --workspace`
   - Git hooks will auto-run these on commit

6. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add amazing feature"
   ```

7. **Push and create PR**
   ```bash
   git push origin feature/your-feature-name
   ```
   Then open a Pull Request on GitHub.

## Commit Message Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `style:` Code style changes (formatting)
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `test:` Adding or updating tests
- `chore:` Maintenance tasks

Example:
```
feat(av1): add tile group parsing support

- Parse tile_group_obu header
- Extract tile start/end positions
- Add tests for multi-tile frames

Closes #123
```

## Coding Standards

### Rust Code

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Fix all `clippy` warnings
- Add tests for new functionality (aim for >80% coverage)
- Document public APIs with rustdoc comments

### TypeScript/React Code

- Follow ESLint rules
- Use Prettier for formatting
- Add tests for new components
- Use TypeScript types strictly

## Test Coverage

We use `cargo llvm-cov` for Rust coverage reporting:

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --html --output-dir cov-html
```

We use Vitest for frontend coverage:

```bash
cd frontend
npm run test:coverage
```

Target coverage:
- Core parsers: **>90%**
- UI code: **>80%**
- Overall: **>85%**

## Pull Request Guidelines

### Before Submitting

- [ ] All tests pass (`cargo test --workspace` && `cd frontend && npm test`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] No clippy warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] Frontend linting passes (`cd frontend && npm run lint`)
- [ ] Documentation updated if needed
- [ ] Commit messages follow convention

### PR Description Template

```markdown
## Summary
Brief description of changes

## Changes
- Added X
- Fixed Y
- Updated Z

## Testing
- Added tests for X
- Verified Y works correctly

## Checklist
- [ ] Tests pass
- [ ] Documentation updated
- [ ] No breaking changes (or documented)
```

## Project Structure

```
bitvue/
├── crates/
│   ├── bitvue/               # Main library facade
│   ├── bitvue-codecs/        # Unified codec interface
│   ├── bitvue-core/         # Core types, state, caching
│   ├── bitvue-formats/      # Container parsers (IVF, MP4, MKV, TS)
│   ├── bitvue-decode/       # Decoder bindings (dav1d for AV1)
│   ├── bitvue-metrics/      # Quality metrics (PSNR, SSIM, VMAF)
│   ├── bitvue-cli/          # CLI tool
│   ├── bitvue-codecs-parser/ # Codec integration layer
│   ├── bitvue-mcp/          # Model Context Protocol server
│   ├── bitvue-benchmarks/   # Performance benchmarks
│   │   # Individual codec parsers
│   ├── bitvue-av1-codec/    # AV1 OBU parser
│   ├── bitvue-avc/          # AVC/H.264 parser
│   ├── bitvue-hevc/         # HEVC/H.265 parser
│   ├── bitvue-vp9/          # VP9 parser
│   ├── bitvue-vvc/          # VVC/H.266 parser
│   ├── bitvue-av3-codec/    # AV3 parser
│   ├── bitvue-mpeg2-codec/  # MPEG-2 parser
│   └── vendor/              # Third-party dependencies
│       └── abseil/          # Abseil logging library
├── frontend/                # React application
│   ├── src/                 # Application source
│   ├── tests/               # Consolidated test files
│   ├── public/              # Static assets
│   └── index.html
├── src-tauri/               # Tauri backend (Rust)
├── scripts/                 # Development scripts
└── config/                  # Tool configurations
```

## Adding Codec Support

To add a new codec (e.g., VP8):

1. Create `crates/bitvue-vp8/`
2. Implement parser traits from `bitvue-core`
3. Add overlay extraction functions
4. Add tests (>80% coverage)
5. Add to `bitvue-codecs` crate
6. Update README with format support

## Using the Main Library

The `bitvue` crate provides a unified interface:

```rust
// Use the main library facade
use bitvue::prelude::*;

// Access core functionality
let state = core::SelectionState::new();

// Parse formats
let parser = formats::ivf::IvfParser::new();

// Access codecs
use bitvue::codecs::av1;
```

## Questions?

- Open a GitHub Discussion for questions
- Create an issue for bug reports or feature requests
- Check existing issues and PRs first

## License

By contributing, you agree that your contributions will be licensed under the [AGPL-3.0](LICENSE).
