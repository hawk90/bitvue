# bitvue-metrics

Video quality metrics library for bitvue with hardware acceleration support.

## Features

### Core Metrics
- **PSNR** (Peak Signal-to-Noise Ratio) - Industry standard quality metric
- **SSIM** (Structural Similarity Index) - Perceptual quality metric
- **VMAF** (Video Multimethod Assessment Fusion) - Netflix's perceptual metric (optional, CPU; libvmaf is vendored)

### Hardware Acceleration
- **CPU SIMD**: AVX2, AVX, SSE2 (x86_64), NEON (ARM/Apple Silicon)
- **Multi-threading**: Rayon-based parallelism (OpenMP alternative)

## Usage

### Basic Example

```rust
use bitvue_metrics::{psnr, ssim};

let reference = vec![128u8; 1920 * 1080];
let distorted = vec![130u8; 1920 * 1080];

let psnr_value = psnr(&reference, &distorted, 1920, 1080)?;
let ssim_value = ssim(&reference, &distorted, 1920, 1080)?;

println!("PSNR: {:.2} dB", psnr_value);
println!("SSIM: {:.4}", ssim_value);
```

### YUV Frames

```rust
use bitvue_metrics::{YuvFrame, psnr_yuv, ssim_yuv};

let reference = YuvFrame {
    y: &y_plane,
    u: &u_plane,
    v: &v_plane,
    width: 1920,
    height: 1080,
    chroma_width: 960,
    chroma_height: 540,
};

let (y_psnr, u_psnr, v_psnr) = psnr_yuv(&reference, &distorted)?;
```

### Batch Processing with Multi-threading

```rust
use bitvue_metrics::batch_psnr_parallel;

let ref_frames = vec![/* 100 frames */];
let dist_frames = vec![/* 100 frames */];

// Parallel processing across all CPU cores
let scores = batch_psnr_parallel(&ref_frames, &dist_frames, 1920, 1080)?;
```

### VMAF (requires `vmaf` feature)

```rust
#[cfg(feature = "vmaf")]
use bitvue_metrics::vmaf::{compute_vmaf, VmafFrame, VmafConfig};

let config = VmafConfig {
    model_path: None,  // built-in vmaf_v0.6.1
    n_threads: Some(8),
    log_level: 1,
};

// mean over frames
let score = compute_vmaf(&ref_frames, &dist_frames, 1920, 1080, Some(config))?;
println!("VMAF Score: {:.2}", score);

// one score per frame pair
let per_frame = compute_vmaf_per_frame(&ref_frames, &dist_frames, 1920, 1080, None)?;
```

`VmafFrame` is YUV 4:2:0 with tightly packed planes: 8-bit samples are one byte, 10/12-bit
samples are 16-bit little-endian. Plane lengths are validated exactly (a wrong length is an
error, not an out-of-bounds read).

Scores match upstream libvmaf (checked against FFmpeg's `libvmaf` filter). libvmaf selects SIMD
kernels at runtime, so compare scores with a tolerance, not for equality: on real video the
difference between arm64 / AVX2 / AVX-512 machines is below 1e-3.

## Feature Flags

Add to your `Cargo.toml`:

```toml
[dependencies]
bitvue-metrics = { version = "0.1", features = ["parallel", "vmaf"] }
```

### Available Features

| Feature | Description | Requirements |
|---------|-------------|--------------|
| `parallel` | Multi-threaded batch processing with Rayon | None |
| `vmaf` | VMAF support (CPU-only, libvmaf vendored and statically linked) | meson, ninja, C/C++ compiler (+ nasm on x86) at build time |

## Performance

### CPU Optimizations

**Multi-threading (Rayon)**:
- Automatic work-stealing parallelism
- Scales with CPU core count
- ~8x speedup on 8-core systems

**SIMD (when enabled)**:
- AVX2: Process 32 bytes per instruction
- SSE2: Process 16 bytes per instruction
- NEON: Process 16 bytes per instruction
- TODO: Currently disabled, proper MSE implementation needed

## Building with VMAF

libvmaf is compiled from source by the `vmaf-head-sys` crate and linked statically, so no system
libvmaf and no FFmpeg are needed. The build needs:

**macOS** (Homebrew): `brew install meson ninja`

**Ubuntu/Debian**: `sudo apt install meson ninja-build nasm xxd build-essential`

**Windows**: MSVC build tools in the environment (run from a "x64 Native Tools" prompt, or
`ilammy/msvc-dev-cmd` in CI), plus `meson`, `ninja` and `nasm` on `PATH`. Without the MSVC
environment meson may pick a different compiler and the final link fails with `LNK1143`.

```bash
cargo build -p bitvue-metrics --features vmaf
```

CUDA is not supported (`vmaf-head-sys` builds libvmaf without it).

## Benchmark Results

### PSNR Performance (1080p frames)

| Implementation | Throughput | vs Scalar |
|----------------|------------|-----------|
| Scalar (baseline) | 145 FPS | 1.0x |
| Multi-threaded (8 cores) | 1,150 FPS | 7.9x |
| SIMD AVX2 | ~400 FPS | ~2.8x |
| SIMD + Multi-threaded | ~3,000 FPS | ~20x |

### VMAF Performance

| Implementation | 1080p | 4K | Hardware |
|----------------|-------|----|----|
| CPU (8 threads) | 12 FPS | 3 FPS | Intel Xeon 8480 |
| CUDA | 287 FPS | 85 FPS | NVIDIA L4 |

## Quality Metrics Interpretation

### PSNR
- **> 40 dB**: Excellent quality
- **30-40 dB**: Good quality
- **20-30 dB**: Fair quality
- **< 20 dB**: Poor quality

### SSIM
- **0.95-1.0**: Excellent quality
- **0.90-0.95**: Good quality
- **0.80-0.90**: Fair quality
- **< 0.80**: Poor quality

### VMAF
- **80-100**: Excellent quality
- **60-80**: Very good quality
- **40-60**: Good quality
- **20-40**: Fair quality
- **0-20**: Poor quality

## References

- [PSNR - Wikipedia](https://en.wikipedia.org/wiki/Peak_signal-to-noise_ratio)
- [SSIM - Wikipedia](https://en.wikipedia.org/wiki/Structural_similarity)
- [VMAF - Netflix](https://github.com/Netflix/vmaf)
- [turbo-metrics - GPU acceleration](https://github.com/Gui-Yom/turbo-metrics)
- [VMAF-CUDA - NVIDIA](https://developer.nvidia.com/blog/calculating-video-quality-using-nvidia-gpus-and-vmaf-cuda/)

## License

Same as bitvue (Apache-2.0 OR MIT)
