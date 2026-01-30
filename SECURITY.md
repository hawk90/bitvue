# Security Policy

## Supported Versions

Currently, only the latest version of bitvue is supported with security updates.

## Reporting a Vulnerability

If you discover a security vulnerability in bitvue, please report it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please send an email to: security@bitvue.dev (or open a GitHub Discussion with the `security` tag)

Your email should include:
* Description of the vulnerability
* Steps to reproduce the issue
* Potential impact assessment
* Any suggested fixes (if available)

### What to Expect

* We will acknowledge receipt of your report within 48 hours
* We will provide a detailed response within 7 days
* We will notify you when the fix is deployed
* You will be credited in the security advisory (unless you prefer to remain anonymous)

### Vulnerability Response Process

1. **Receipt**: We confirm receipt of the vulnerability report
2. **Analysis**: We investigate the issue and determine severity
3. **Fix**: We develop a patch for the vulnerability
4. **Test**: We verify the fix resolves the issue
5. **Deploy**: We release a new version with the fix
6. **Disclosure**: We publish a security advisory after the fix is deployed

### Severity Levels

We use the following severity classifications:

* **Critical**: High risk to all users, data loss, or remote code execution
* **High**: Significant impact, limited exploitability
* **Medium**: Moderate impact, requires specific conditions
* **Low**: Minor impact, difficult to exploit

## Security Best Practices

### For Users

* Always download bitvue from official sources (GitHub Releases)
* Verify binary signatures (when available)
* Keep your application updated to the latest version
* Only open video files from trusted sources

### For Developers

* Follow secure coding practices
* Use `cargo audit` to check for vulnerable dependencies
* Enable all compiler security features (ASLR, DEP, stack canaries)
* Review and test all external input handling

## Dependency Security

We regularly audit our dependencies using:

* `cargo audit` - Checks for known vulnerabilities in Rust crates
* `cargo deny` - Licenses and advisory checks
* Manual review of new dependencies

## Security Features

bitvue includes the following security features:

* **Sandboxing**: Tauri provides OS-level sandboxing
* **Input Validation**: All file inputs are validated before processing
* **Memory Safety**: Rust prevents entire classes of memory vulnerabilities
* **No Remote Code Execution**: No eval or dynamic code execution

## Technical Security Details

### Threat Model

Bitvue processes potentially untrusted video files. Our threat model addresses:

1. **Malicious Video Files**
   - Attackers may craft specially encoded AV1/H.264/H.265 files to exploit parsers
   - Files may contain invalid bitstreams, extreme dimensions, or malformed structures

2. **Resource Exhaustion**
   - Extremely large files or frame dimensions
   - Deeply nested structures causing stack overflow
   - Memory exhaustion through excessive allocations

### Trust Boundaries

- **Input Files**: Treated as untrusted
- **Video Libraries**: dav1d, vvdec, ffmpeg are trusted dependencies (verified via `cargo audit`)
- **Output Data**: Sanitized and validated before export

### Security Assumptions

#### Parser Assumptions

1. **Bitstream Parsing**
   - All input files must conform to codec specifications
   - Out-of-spec data results in graceful errors, not panics
   - No unchecked buffer access (uses Rust's bounds checking)

2. **Dimension Limits**
   - Maximum resolution: 8K (7680x4320)
   - Maximum frame size: 100MB
   - Grid calculations use checked arithmetic to prevent overflow

3. **Memory Safety**
   - All allocations are bounded
   - Cache sizes limited (max 64 entries for coding unit cache)
   - No raw pointer dereferences without validation

#### Threading Model

1. **Shared State**
   - Global caches use `Mutex` for thread-safe access
   - Lock poisoning handled gracefully (returns errors, doesn't panic)
   - No `static mut` variables

2. **Decoder Isolation**
   - Each decoder instance maintains independent state
   - Decoders can be used safely across threads (one decoder per thread)

#### Safe Usage Patterns

**Recommended:**

```rust
// Validate file size before processing
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10GB
if file_metadata.len() > MAX_FILE_SIZE {
    return Err(BitvueError::InvalidFile("File too large".to_string()));
}

// Create one decoder per file
let mut decoder = Av1Decoder::new()?;

// Always handle decode errors gracefully
match decoder.decode(&data) {
    Ok(frames) => process_frames(frames),
    Err(BitvueError::UnexpectedEof(_)) => {
        eprintln!("Warning: Incomplete video file");
    }
    Err(e) => {
        eprintln!("Decode error: {}", e);
        return Err(e);
    }
}
```

**Avoid:**

```rust
// Don't share decoder instances across threads
let decoder = Arc::new(Mutex::new(Av1Decoder::new()?));
// ❌ Wrong: Internal state may get corrupted

// Don't ignore dimension validation
let (width, height) = parse_dimensions(data)?;
let buffer = vec![0u8; width * height]; // ❌ May overflow

// Do instead:
let size = (width as usize).checked_mul(height as usize)
    .ok_or(BitvueError::Decode("Dimensions too large".to_string()))?;
let buffer = vec![0u8; size];
```

### Input Validation

The following validations are performed:

1. **File Level**
   - Maximum file size: 10GB
   - File format validation (IVF, MP4, MKV signatures)

2. **Frame Level**
   - Maximum dimensions: 7680x4320 (8K)
   - Maximum frame size: 100MB
   - Bit depth: 8, 10, or 12 bits

3. **Grid Level**
   - Maximum blocks: 512x512 (16x16 blocks for 8K video)
   - Overflow checks on all grid calculations
   - Bounds checking on all array access

### Known Limitations

1. **Parser Completeness**
   - AV1 parser: Production-ready, handles standard features
   - H.264/H.265: Relies on FFmpeg (trusted dependency)
   - VVC: Experimental support, parser not exhaustive

2. **Fuzzing Coverage**
   - No continuous fuzzing currently (planned addition)
   - Relies on manual testing with real-world video files

## Privacy

bitvue is a desktop application that:
* Does NOT collect or transmit any user data
* Does NOT require network access for core functionality
* Does NOT include telemetry or analytics
* Processes all video files locally on your machine

## Contact

For security-related questions that are not vulnerability reports, please open a GitHub Discussion with the `security` tag.
