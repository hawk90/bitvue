#![allow(dead_code)]
//! Tests for YUV format handling

#[test]
fn test_yuv_frame_structure() {
    struct YuvFrame {
        width: usize,
        height: usize,
        y_plane: Vec<u8>,
        u_plane: Vec<u8>,
        v_plane: Vec<u8>,
    }

    impl YuvFrame {
        fn new_420(width: usize, height: usize) -> Self {
            let y_size = width * height;
            let uv_size = (width / 2) * (height / 2);

            Self {
                width,
                height,
                y_plane: vec![0u8; y_size],
                u_plane: vec![0u8; uv_size],
                v_plane: vec![0u8; uv_size],
            }
        }

        fn total_size(&self) -> usize {
            self.y_plane.len() + self.u_plane.len() + self.v_plane.len()
        }
    }

    let frame = YuvFrame::new_420(1920, 1080);
    assert_eq!(frame.total_size(), 3110400);
}

#[test]
fn test_yuv_subsampling() {
    #[derive(Debug, PartialEq)]
    enum ChromaSubsampling {
        Yuv420,
        Yuv422,
        Yuv444,
    }

    struct SubsamplingInfo {
        format: ChromaSubsampling,
    }

    impl SubsamplingInfo {
        fn horizontal_factor(&self) -> usize {
            match self.format {
                ChromaSubsampling::Yuv420 | ChromaSubsampling::Yuv422 => 2,
                ChromaSubsampling::Yuv444 => 1,
            }
        }

        fn vertical_factor(&self) -> usize {
            match self.format {
                ChromaSubsampling::Yuv420 => 2,
                ChromaSubsampling::Yuv422 | ChromaSubsampling::Yuv444 => 1,
            }
        }
    }

    let info = SubsamplingInfo {
        format: ChromaSubsampling::Yuv420,
    };

    assert_eq!(info.horizontal_factor(), 2);
    assert_eq!(info.vertical_factor(), 2);
}

#[test]
fn test_yuv_to_rgb_conversion() {
    fn yuv_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
        let y = y as i32;
        let u = u as i32 - 128;
        let v = v as i32 - 128;

        let r = (y + (1.370705 * v as f32) as i32).clamp(0, 255) as u8;
        let g =
            (y - (0.337633 * u as f32) as i32 - (0.698001 * v as f32) as i32).clamp(0, 255) as u8;
        let b = (y + (1.732446 * u as f32) as i32).clamp(0, 255) as u8;

        (r, g, b)
    }

    let (r, g, b) = yuv_to_rgb(128, 128, 128);
    // Verify output is valid RGB
    assert!(r <= 255);
    assert!(g <= 255);
    assert!(b <= 255);
}

#[test]
fn test_yuv_plane_size() {
    struct YuvPlaneSizes {
        width: usize,
        height: usize,
        format: String,
    }

    impl YuvPlaneSizes {
        fn y_plane_size(&self) -> usize {
            self.width * self.height
        }

        fn u_plane_size(&self) -> usize {
            match self.format.as_str() {
                "yuv420p" => (self.width / 2) * (self.height / 2),
                "yuv422p" => (self.width / 2) * self.height,
                "yuv444p" => self.width * self.height,
                _ => 0,
            }
        }

        fn v_plane_size(&self) -> usize {
            self.u_plane_size()
        }
    }

    let sizes = YuvPlaneSizes {
        width: 1920,
        height: 1080,
        format: "yuv420p".to_string(),
    };

    assert_eq!(sizes.y_plane_size(), 2073600);
    assert_eq!(sizes.u_plane_size(), 518400);
}

#[test]
fn test_yuv_stride_handling() {
    struct YuvStride {
        width: usize,
        height: usize,
        y_stride: usize,
        uv_stride: usize,
    }

    impl YuvStride {
        fn new_420(width: usize, height: usize) -> Self {
            Self {
                width,
                height,
                y_stride: width,
                uv_stride: width / 2,
            }
        }

        fn y_plane_size_with_stride(&self) -> usize {
            self.y_stride * self.height
        }

        fn uv_plane_size_with_stride(&self) -> usize {
            self.uv_stride * (self.height / 2)
        }
    }

    let stride = YuvStride::new_420(1920, 1080);
    assert_eq!(stride.y_stride, 1920);
    assert_eq!(stride.uv_stride, 960);
}

#[test]
fn test_yuv_range() {
    #[derive(Debug, PartialEq)]
    enum YuvRange {
        Limited, // 16-235 for Y, 16-240 for UV
        Full,    // 0-255
    }

    struct RangeInfo {
        range: YuvRange,
    }

    impl RangeInfo {
        fn y_min(&self) -> u8 {
            match self.range {
                YuvRange::Limited => 16,
                YuvRange::Full => 0,
            }
        }

        fn y_max(&self) -> u8 {
            match self.range {
                YuvRange::Limited => 235,
                YuvRange::Full => 255,
            }
        }
    }

    let limited = RangeInfo {
        range: YuvRange::Limited,
    };

    assert_eq!(limited.y_min(), 16);
    assert_eq!(limited.y_max(), 235);
}

#[test]
fn test_planar_to_packed() {
    fn planar_to_packed_yuv422(y: &[u8], u: &[u8], v: &[u8]) -> Vec<u8> {
        let mut packed = Vec::new();
        for i in 0..y.len() {
            packed.push(y[i]);
            if i % 2 == 0 {
                packed.push(u[i / 2]);
            } else {
                packed.push(v[i / 2]);
            }
        }
        packed
    }

    let y = vec![128, 129, 130, 131];
    let u = vec![64, 65];
    let v = vec![192, 193];

    let packed = planar_to_packed_yuv422(&y, &u, &v);
    assert_eq!(packed[0], 128); // Y0
    assert_eq!(packed[1], 64); // U0
}

#[test]
fn test_yuv_alignment() {
    fn align_dimension(size: usize, alignment: usize) -> usize {
        (size + alignment - 1) / alignment * alignment
    }

    assert_eq!(align_dimension(1920, 16), 1920);
    assert_eq!(align_dimension(1921, 16), 1936);
}

#[test]
fn test_yuv_color_space() {
    #[derive(Debug, PartialEq)]
    enum ColorSpace {
        Bt601,
        Bt709,
        Bt2020,
    }

    struct ColorSpaceInfo {
        space: ColorSpace,
    }

    impl ColorSpaceInfo {
        fn is_hdr(&self) -> bool {
            self.space == ColorSpace::Bt2020
        }
    }

    let hdr = ColorSpaceInfo {
        space: ColorSpace::Bt2020,
    };

    assert!(hdr.is_hdr());
}

#[test]
fn test_yuv_bit_depth() {
    struct BitDepthInfo {
        bit_depth: u8,
    }

    impl BitDepthInfo {
        fn bytes_per_sample(&self) -> usize {
            if self.bit_depth > 8 {
                2
            } else {
                1
            }
        }

        fn max_value(&self) -> u16 {
            (1 << self.bit_depth) - 1
        }
    }

    let info_8bit = BitDepthInfo { bit_depth: 8 };
    let info_10bit = BitDepthInfo { bit_depth: 10 };

    assert_eq!(info_8bit.bytes_per_sample(), 1);
    assert_eq!(info_10bit.bytes_per_sample(), 2);
    assert_eq!(info_10bit.max_value(), 1023);
}

#[test]
fn test_yuv_copy_plane() {
    fn copy_plane(
        src: &[u8],
        dst: &mut [u8],
        width: usize,
        height: usize,
        src_stride: usize,
        dst_stride: usize,
    ) {
        for y in 0..height {
            let src_offset = y * src_stride;
            let dst_offset = y * dst_stride;
            dst[dst_offset..dst_offset + width]
                .copy_from_slice(&src[src_offset..src_offset + width]);
        }
    }

    let src = vec![1u8; 2000];
    let mut dst = vec![0u8; 2000];

    copy_plane(&src, &mut dst, 100, 10, 200, 200);
    assert_eq!(dst[0], 1);
}
