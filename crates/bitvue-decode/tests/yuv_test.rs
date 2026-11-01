//! YUV -> RGB conversion through the public API: `yuv_to_rgb` and every conversion strategy
//! available on this machine, against the textbook BT.601 full-range formula. (This file used to
//! hold eleven tests that defined their own structs and asserted on literals they had just
//! written, #386.)

use bitvue_decode::decoder::{ChromaFormat, DecodedFrame, FrameType};
use bitvue_decode::strategy::{ScalarStrategy, YuvConversionStrategy};
use bitvue_decode::yuv::{rgb_to_image, yuv_to_rgb};

fn frame(
    width: u32,
    height: u32,
    bit_depth: u8,
    chroma: ChromaFormat,
    y: &[u8],
    u: Option<&[u8]>,
    v: Option<&[u8]>,
) -> DecodedFrame {
    DecodedFrame {
        width,
        height,
        bit_depth,
        y_plane: y.to_vec().into_boxed_slice().into(),
        y_stride: width as usize,
        u_plane: u.map(|p| p.to_vec().into_boxed_slice().into()),
        u_stride: 0,
        v_plane: v.map(|p| p.to_vec().into_boxed_slice().into()),
        v_stride: 0,
        timestamp: 0,
        frame_type: FrameType::Key,
        qp_avg: None,
        chroma_format: chroma,
    }
}

/// BT.601 full range in floating point, rounded and clamped: the reference the fixed-point
/// implementations must stay within a couple of levels of.
fn textbook(y: f64, u: f64, v: f64) -> [i32; 3] {
    let c = |x: f64| x.round().clamp(0.0, 255.0) as i32;
    [
        c(y + 1.402 * (v - 128.0)),
        c(y - 0.344136 * (u - 128.0) - 0.714136 * (v - 128.0)),
        c(y + 1.772 * (u - 128.0)),
    ]
}

fn assert_close(got: &[u8], want: [i32; 3], tolerance: i32, what: &str) {
    for (channel, (&g, &w)) in got.iter().zip(want.iter()).enumerate() {
        assert!(
            (i32::from(g) - w).abs() <= tolerance,
            "{what}: channel {channel} is {g}, textbook value {w}"
        );
    }
}

/// A deterministic pseudo-random byte source.
fn bytes(seed: &mut u64, n: usize) -> Vec<u8> {
    (0..n)
        .map(|_| {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            (*seed >> 24) as u8
        })
        .collect()
}

#[test]
fn grey_stays_grey_whatever_the_luma() {
    // U = V = 128 carries no colour: R = G = B = Y for every luma value
    let y: Vec<u8> = (0..=255).collect();
    let neutral = vec![128u8; 256];
    let f = frame(
        16,
        16,
        8,
        ChromaFormat::Yuv444,
        &y,
        Some(&neutral),
        Some(&neutral),
    );
    let rgb = yuv_to_rgb(&f);
    for (i, px) in rgb.chunks(3).enumerate() {
        assert_eq!(px, [i as u8; 3], "luma {i}");
    }
}

#[test]
fn primary_colours_match_the_textbook_formula() {
    // (Y, U, V) of full-range BT.601 red, green, blue, white, black, yellow
    let colours: [(u8, u8, u8, [i32; 3]); 6] = [
        (76, 85, 255, [255, 0, 0]),
        (150, 44, 21, [0, 255, 0]),
        (29, 255, 107, [0, 0, 255]),
        (255, 128, 128, [255, 255, 255]),
        (0, 128, 128, [0, 0, 0]),
        (226, 1, 149, [255, 255, 0]),
    ];
    let y: Vec<u8> = colours.iter().map(|c| c.0).collect();
    let u: Vec<u8> = colours.iter().map(|c| c.1).collect();
    let v: Vec<u8> = colours.iter().map(|c| c.2).collect();
    let f = frame(6, 1, 8, ChromaFormat::Yuv444, &y, Some(&u), Some(&v));
    let rgb = yuv_to_rgb(&f);
    for (i, c) in colours.iter().enumerate() {
        assert_close(&rgb[i * 3..i * 3 + 3], c.3, 4, &format!("colour {i}"));
        // and the formula itself agrees with the labelled colour
        assert_close(
            &rgb[i * 3..i * 3 + 3],
            textbook(f64::from(c.0), f64::from(c.1), f64::from(c.2)),
            2,
            &format!("colour {i} vs formula"),
        );
    }
}

#[test]
fn chroma_is_shared_the_way_each_subsampling_says() {
    // 4x2, luma flat 100; two distinct chroma colours
    let y = [100u8; 8];
    // 4:2:0: one chroma sample per 2x2 block -> the left two columns share the first, the right two
    // the second, in both rows
    let f = frame(
        4,
        2,
        8,
        ChromaFormat::Yuv420,
        &y,
        Some(&[60, 200]),
        Some(&[180, 40]),
    );
    let rgb = yuv_to_rgb(&f);
    let px = |x: usize, row: usize| &rgb[(row * 4 + x) * 3..(row * 4 + x) * 3 + 3];
    for row in 0..2 {
        assert_eq!(px(0, row), px(1, row));
        assert_eq!(px(2, row), px(3, row));
    }
    assert_eq!(px(0, 0), px(0, 1));
    assert_ne!(px(0, 0), px(2, 0));
    assert_close(px(0, 0), textbook(100.0, 60.0, 180.0), 2, "4:2:0 left");
    assert_close(px(3, 1), textbook(100.0, 200.0, 40.0), 2, "4:2:0 right");

    // 4:2:2: one chroma sample per 2 columns, every row has its own
    let f = frame(
        4,
        2,
        8,
        ChromaFormat::Yuv422,
        &y,
        Some(&[60, 200, 90, 30]),
        Some(&[180, 40, 70, 220]),
    );
    let rgb = yuv_to_rgb(&f);
    let px = |x: usize, row: usize| &rgb[(row * 4 + x) * 3..(row * 4 + x) * 3 + 3];
    assert_close(
        px(1, 0),
        textbook(100.0, 60.0, 180.0),
        2,
        "4:2:2 row 0 left",
    );
    assert_close(
        px(2, 0),
        textbook(100.0, 200.0, 40.0),
        2,
        "4:2:2 row 0 right",
    );
    assert_close(px(1, 1), textbook(100.0, 90.0, 70.0), 2, "4:2:2 row 1 left");
    assert_close(
        px(3, 1),
        textbook(100.0, 30.0, 220.0),
        2,
        "4:2:2 row 1 right",
    );

    // 4:4:4: every pixel its own
    let u: Vec<u8> = (0..8).map(|i| 20 + i * 30).collect();
    let v: Vec<u8> = (0..8).map(|i| 240 - i * 30).collect();
    let f = frame(4, 2, 8, ChromaFormat::Yuv444, &y, Some(&u), Some(&v));
    let rgb = yuv_to_rgb(&f);
    for i in 0..8 {
        assert_close(
            &rgb[i * 3..i * 3 + 3],
            textbook(100.0, f64::from(u[i]), f64::from(v[i])),
            2,
            &format!("4:4:4 pixel {i}"),
        );
    }
}

#[test]
fn high_bit_depth_samples_are_scaled_down_to_eight_bits() {
    // 10 bit little-endian: Y = 1023 is white, 512/512/512 is mid grey, 0 is black
    let le = |vals: &[u16]| -> Vec<u8> { vals.iter().flat_map(|v| v.to_le_bytes()).collect() };
    let f = frame(
        3,
        1,
        10,
        ChromaFormat::Yuv444,
        &le(&[1023, 512, 0]),
        Some(&le(&[512, 512, 512])),
        Some(&le(&[512, 512, 512])),
    );
    let rgb = yuv_to_rgb(&f);
    assert_eq!(rgb.len(), 9);
    assert_close(&rgb[0..3], [255, 255, 255], 1, "10-bit white");
    assert_close(&rgb[3..6], [128, 128, 128], 1, "10-bit grey");
    assert_close(&rgb[6..9], [0, 0, 0], 1, "10-bit black");
    // 12 bit: 4095 is white
    let f = frame(1, 1, 12, ChromaFormat::Monochrome, &le(&[4095]), None, None);
    assert_eq!(yuv_to_rgb(&f), vec![255, 255, 255]);
}

#[test]
fn a_frame_with_missing_chroma_planes_falls_back_to_grey() {
    for chroma in [
        ChromaFormat::Yuv420,
        ChromaFormat::Yuv422,
        ChromaFormat::Yuv444,
    ] {
        let f = frame(2, 2, 8, chroma, &[10, 90, 170, 250], None, None);
        let rgb = yuv_to_rgb(&f);
        let expected: Vec<u8> = [10u8, 90, 170, 250].iter().flat_map(|&y| [y; 3]).collect();
        assert_eq!(rgb, expected, "{chroma:?}");
    }
    let f = frame(2, 1, 8, ChromaFormat::Monochrome, &[7, 200], None, None);
    assert_eq!(yuv_to_rgb(&f), vec![7, 7, 7, 200, 200, 200]);
}

#[test]
fn every_available_strategy_agrees_with_scalar() {
    let mut strategies: Vec<Box<dyn YuvConversionStrategy>> = vec![Box::new(ScalarStrategy::new())];
    #[cfg(target_arch = "x86_64")]
    strategies.push(Box::new(bitvue_decode::strategy::Avx2Strategy::new()));
    #[cfg(target_arch = "aarch64")]
    strategies.push(Box::new(bitvue_decode::strategy::NeonStrategy::new()));

    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    // sizes that are not a multiple of the SIMD width, plus a wide one
    for (w, h) in [(2usize, 2usize), (18, 6), (34, 4), (66, 8), (130, 2)] {
        for depth in [8u8, 10, 12] {
            let bps = if depth > 8 { 2 } else { 1 };
            let mask = (1u16 << depth) - 1;
            let sample = |seed: &mut u64, n: usize| -> Vec<u8> {
                if depth > 8 {
                    bytes(seed, n * 2)
                        .chunks(2)
                        .flat_map(|c| (u16::from_le_bytes([c[0], c[1]]) & mask).to_le_bytes())
                        .collect()
                } else {
                    bytes(seed, n)
                }
            };
            let y = sample(&mut seed, w * h);
            let (u420, v420) = (
                sample(&mut seed, w / 2 * (h / 2)),
                sample(&mut seed, w / 2 * (h / 2)),
            );
            let (u422, v422) = (sample(&mut seed, w / 2 * h), sample(&mut seed, w / 2 * h));
            let (u444, v444) = (sample(&mut seed, w * h), sample(&mut seed, w * h));
            let _ = bps;
            for (name, u, v) in [
                ("420", &u420, &v420),
                ("422", &u422, &v422),
                ("444", &u444, &v444),
            ] {
                let run = |s: &dyn YuvConversionStrategy| {
                    let mut rgb = vec![0u8; w * h * 3];
                    match name {
                        "420" => s.convert_yuv420_to_rgb(&y, u, v, w, h, &mut rgb, depth),
                        "422" => s.convert_yuv422_to_rgb(&y, u, v, w, h, &mut rgb, depth),
                        _ => s.convert_yuv444_to_rgb(&y, u, v, w, h, &mut rgb, depth),
                    }
                    .unwrap_or_else(|e| panic!("{} {name} {w}x{h} {depth}-bit: {e}", s.name()));
                    rgb
                };
                let reference = run(strategies[0].as_ref());
                for s in &strategies[1..] {
                    if !s.is_available() {
                        continue;
                    }
                    let got = run(s.as_ref());
                    let worst = got
                        .iter()
                        .zip(&reference)
                        .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
                        .max()
                        .unwrap();
                    assert!(
                        worst <= 2,
                        "{} vs scalar, {name} {w}x{h} {depth}-bit: largest difference {worst} (2 allowed: the SIMD paths round each term on its own)",
                        s.name()
                    );
                }
            }
        }
    }
}

#[test]
fn rgb_data_becomes_an_image_only_if_it_fits() {
    let img = rgb_to_image(vec![1, 2, 3, 4, 5, 6], 2, 1).unwrap();
    assert_eq!((img.width(), img.height()), (2, 1));
    assert_eq!(img.get_pixel(1, 0).0, [4, 5, 6]);
    let err = rgb_to_image(vec![0; 5], 2, 1).unwrap_err();
    assert!(
        err.contains("width=2") && err.contains("data_len=5"),
        "{err}"
    );
}

#[test]
fn the_scalar_strategy_scales_twelve_bit_white_and_grey() {
    let le = |vals: &[u16]| -> Vec<u8> { vals.iter().flat_map(|v| v.to_le_bytes()).collect() };
    let mut rgb = vec![0u8; 6];
    ScalarStrategy::new()
        .convert_yuv444_to_rgb(
            &le(&[4095, 2048]),
            &le(&[2048, 2048]),
            &le(&[2048, 2048]),
            2,
            1,
            &mut rgb,
            12,
        )
        .unwrap();
    assert_close(&rgb[0..3], [255, 255, 255], 1, "12-bit white");
    assert_close(&rgb[3..6], [128, 128, 128], 1, "12-bit grey");
}

bitvue_test_alloc::install!();
