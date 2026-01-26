# Security Policy

## Supported Versions

Currently, only the latest version of bitvue is supported with security updates.

## Reporting a Vulnerability

If you discover a security vulnerability in bitvue, please report it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please send an email to: [INSERT SECURITY EMAIL]

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

## Privacy

bitvue is a desktop application that:
* Does NOT collect or transmit any user data
* Does NOT require network access for core functionality
* Does NOT include telemetry or analytics
* Processes all video files locally on your machine

## Contact

For security-related questions that are not vulnerability reports, please open a GitHub Discussion with the `security` tag.
