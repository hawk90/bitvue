# Contributing to bitvue

Thank you for your interest in contributing to bitvue! This document provides guidelines for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for details.

## Getting Started

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
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

3. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

4. **Make your changes**
   - Run tests: `cargo test --workspace`
   - Check formatting: `cargo fmt --all`
   - Run clippy: `cargo clippy --workspace -- -D warnings`
   - Build: `cargo build --all-targets`
   - Git hooks will auto-run these on commit

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add amazing feature"
   ```

6. **Push and create PR**
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

### Test Coverage

We use `cargo llvm-cov` for coverage reporting:

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --html --output-dir cov-html
```

Target coverage:
- Core parsers (bitvue-av1, bitvue-avc, etc.): **>90%**
- UI code: **>80%**
- Overall: **>85%**

## Pull Request Guidelines

### Before Submitting

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] No clippy warnings (`cargo clippy --workspace -- -D warnings`)
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
│   ├── app/              # Tauri application
│   ├── bitvue-av1/       # AV1 codec parser
│   ├── bitvue-avc/       # H.264/AVC codec parser
│   ├── bitvue-hevc/      # H.265/HEVC codec parser
│   ├── bitvue-vp9/       # VP9 codec parser
│   ├── bitvue-vvc/       # H.266/VVC codec parser
│   ├── bitvue-core/      # Core types and utilities
│   └── ...
├── src/                  # Tauri frontend (React)
└── docs/                 # Documentation
```

## Adding Codec Support

To add a new codec (e.g., VP8):

1. Create `crates/bitvue-vp8/`
2. Implement parser traits from `bitvue-core`
3. Add overlay extraction functions
4. Add tests (>80% coverage)
5. Update README with format support

## Questions?

- Open a GitHub Discussion for questions
- Create an issue for bug reports or feature requests
- Check existing issues and PRs first

## License

By contributing, you agree that your contributions will be licensed under the [AGPL-3.0](LICENSE).
