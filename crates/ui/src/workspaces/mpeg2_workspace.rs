//! MPEG-2 Video Visualization Workspace
//!
//! VQAnalyzer parity: Complete MPEG-2-specific analysis views
//! - Macroblock partitions (16x16 fixed MB size)
//! - Picture types (I/P/B/D)
//! - Motion compensation visualization
//! - DCT coefficient visualization
//! - GOP structure analysis
//! - Quantization scale display

use egui::{self, Color32, Rect, RichText, Stroke, Vec2};

/// MPEG-2-specific color palette
mod colors {
    use egui::Color32;

    // Picture type colors
    pub const PICTURE_I: Color32 = Color32::from_rgb(255, 0, 0); // Red
    pub const PICTURE_P: Color32 = Color32::from_rgb(0, 255, 0); // Green
    pub const PICTURE_B: Color32 = Color32::from_rgb(0, 0, 255); // Blue
    pub const PICTURE_D: Color32 = Color32::from_rgb(255, 165, 0); // Orange

    // MB type colors
    pub const MB_INTRA: Color32 = Color32::from_rgb(255, 215, 0); // Gold
    pub const MB_FORWARD: Color32 = Color32::from_rgb(50, 205, 50); // Lime green
    pub const MB_BACKWARD: Color32 = Color32::from_rgb(30, 144, 255); // Dodger blue
    pub const MB_INTERPOLATED: Color32 = Color32::from_rgb(147, 112, 219); // Medium purple
    pub const MB_SKIPPED: Color32 = Color32::from_rgb(128, 128, 128); // Gray

    // Structure colors
    pub const MB_BOUNDARY: Color32 = Color32::from_rgb(255, 128, 0); // Orange
    pub const SLICE_BOUNDARY: Color32 = Color32::from_rgb(255, 255, 0); // Yellow
    pub const DCT_BLOCK: Color32 = Color32::from_rgb(144, 238, 144); // Light green
}

/// MPEG-2 view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mpeg2View {
    #[default]
    Overview,
    GopStructure,
    Macroblocks,
    MotionVectors,
    Quantization,
}

impl Mpeg2View {
    fn label(&self) -> &'static str {
        match self {
            Mpeg2View::Overview => "Overview",
            Mpeg2View::GopStructure => "GOP",
            Mpeg2View::Macroblocks => "Macroblocks",
            Mpeg2View::MotionVectors => "Motion",
            Mpeg2View::Quantization => "Quant",
        }
    }
}

/// MPEG-2 picture type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mpeg2PictureType {
    #[default]
    I,
    P,
    B,
    D, // DC intra-coded (rare)
}

impl Mpeg2PictureType {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Mpeg2PictureType::I,
            2 => Mpeg2PictureType::P,
            3 => Mpeg2PictureType::B,
            4 => Mpeg2PictureType::D,
            _ => Mpeg2PictureType::I,
        }
    }

    fn color(&self) -> Color32 {
        match self {
            Mpeg2PictureType::I => colors::PICTURE_I,
            Mpeg2PictureType::P => colors::PICTURE_P,
            Mpeg2PictureType::B => colors::PICTURE_B,
            Mpeg2PictureType::D => colors::PICTURE_D,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Mpeg2PictureType::I => "I",
            Mpeg2PictureType::P => "P",
            Mpeg2PictureType::B => "B",
            Mpeg2PictureType::D => "D",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Mpeg2PictureType::I => "Intra-coded (keyframe)",
            Mpeg2PictureType::P => "Predictive (forward)",
            Mpeg2PictureType::B => "Bi-directional",
            Mpeg2PictureType::D => "DC intra-coded",
        }
    }
}

/// MPEG-2 picture structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PictureStructure {
    TopField,
    BottomField,
    #[default]
    Frame,
}

impl PictureStructure {
    fn label(&self) -> &'static str {
        match self {
            PictureStructure::TopField => "Top Field",
            PictureStructure::BottomField => "Bottom Field",
            PictureStructure::Frame => "Frame",
        }
    }
}

/// MPEG-2 macroblock type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mpeg2MbType {
    #[default]
    Intra,
    ForwardPred,
    BackwardPred,
    Interpolated, // Both forward and backward
    Skipped,
}

impl Mpeg2MbType {
    fn color(&self) -> Color32 {
        match self {
            Mpeg2MbType::Intra => colors::MB_INTRA,
            Mpeg2MbType::ForwardPred => colors::MB_FORWARD,
            Mpeg2MbType::BackwardPred => colors::MB_BACKWARD,
            Mpeg2MbType::Interpolated => colors::MB_INTERPOLATED,
            Mpeg2MbType::Skipped => colors::MB_SKIPPED,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Mpeg2MbType::Intra => "Intra",
            Mpeg2MbType::ForwardPred => "Forward",
            Mpeg2MbType::BackwardPred => "Backward",
            Mpeg2MbType::Interpolated => "Interp",
            Mpeg2MbType::Skipped => "Skip",
        }
    }
}

/// MPEG-2 chroma format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mpeg2ChromaFormat {
    #[default]
    Yuv420,
    Yuv422,
    Yuv444,
}

impl Mpeg2ChromaFormat {
    fn label(&self) -> &'static str {
        match self {
            Mpeg2ChromaFormat::Yuv420 => "4:2:0",
            Mpeg2ChromaFormat::Yuv422 => "4:2:2",
            Mpeg2ChromaFormat::Yuv444 => "4:4:4",
        }
    }
}

/// MPEG-2 macroblock data
#[derive(Debug, Clone)]
pub struct Mpeg2Macroblock {
    /// MB index in slice
    pub mb_idx: u32,
    /// Position (x, y) in pixels
    pub x: u32,
    pub y: u32,
    /// MB type
    pub mb_type: Mpeg2MbType,
    /// Quantiser scale code (1-31)
    pub qscale: u8,
    /// Motion vector forward (x, y) in half-pels
    pub mv_forward: Option<(i16, i16)>,
    /// Motion vector backward (x, y) in half-pels
    pub mv_backward: Option<(i16, i16)>,
    /// Coded Block Pattern
    pub cbp: u8,
    /// Pattern code (which blocks are coded)
    pub pattern_code: u8,
}

impl Default for Mpeg2Macroblock {
    fn default() -> Self {
        Self {
            mb_idx: 0,
            x: 0,
            y: 0,
            mb_type: Mpeg2MbType::default(),
            qscale: 8,
            mv_forward: None,
            mv_backward: None,
            cbp: 0,
            pattern_code: 0x3F, // All 6 blocks
        }
    }
}

/// GOP (Group of Pictures) info
#[derive(Debug, Clone)]
pub struct GopInfo {
    /// Time code hours (0-23)
    pub hours: u8,
    /// Time code minutes (0-59)
    pub minutes: u8,
    /// Time code seconds (0-59)
    pub seconds: u8,
    /// Time code pictures (0-29 for 30fps)
    pub pictures: u8,
    /// Drop frame flag
    pub drop_frame: bool,
    /// Closed GOP flag
    pub closed_gop: bool,
    /// Broken link flag
    pub broken_link: bool,
}

impl Default for GopInfo {
    fn default() -> Self {
        Self {
            hours: 0,
            minutes: 0,
            seconds: 0,
            pictures: 0,
            drop_frame: false,
            closed_gop: true,
            broken_link: false,
        }
    }
}

impl GopInfo {
    fn time_code_string(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}:{:02}{}",
            self.hours,
            self.minutes,
            self.seconds,
            self.pictures,
            if self.drop_frame { " (drop)" } else { "" }
        )
    }
}

/// MPEG-2 sequence info
#[derive(Debug, Clone)]
pub struct SequenceInfo {
    /// Horizontal size in pixels
    pub width: u32,
    /// Vertical size in pixels
    pub height: u32,
    /// Aspect ratio code
    pub aspect_ratio: u8,
    /// Frame rate code
    pub frame_rate_code: u8,
    /// Bit rate in bits/s
    pub bit_rate: u32,
    /// VBV buffer size
    pub vbv_buffer_size: u32,
    /// Profile (High/Main/Simple)
    pub profile: u8,
    /// Level (High/Main/Low)
    pub level: u8,
    /// Progressive sequence
    pub progressive: bool,
    /// Chroma format
    pub chroma_format: Mpeg2ChromaFormat,
}

impl Default for SequenceInfo {
    fn default() -> Self {
        Self {
            width: 720,
            height: 480,
            aspect_ratio: 2,     // 4:3
            frame_rate_code: 4,  // 29.97 fps
            bit_rate: 8_000_000, // 8 Mbps
            vbv_buffer_size: 1835008,
            profile: 4, // Main
            level: 8,   // Main
            progressive: true,
            chroma_format: Mpeg2ChromaFormat::Yuv420,
        }
    }
}

/// MPEG-2 Visualization Workspace
pub struct Mpeg2Workspace {
    /// Active view
    active_view: Mpeg2View,

    /// Show MB grid
    show_mb_grid: bool,

    /// Show slice boundaries
    show_slices: bool,

    /// Show motion vectors
    show_motion: bool,

    /// Show DCT blocks
    show_dct: bool,

    /// Sequence info
    sequence: SequenceInfo,

    /// Current GOP info
    current_gop: GopInfo,

    /// Picture type of current frame
    picture_type: Mpeg2PictureType,

    /// Picture structure
    picture_structure: PictureStructure,

    /// Temporal reference
    temporal_ref: u16,

    /// Mock macroblock data
    mock_mbs: Vec<Mpeg2Macroblock>,

    /// Selected MB index
    selected_mb: Option<usize>,

    /// Mock GOP pattern
    gop_pattern: Vec<Mpeg2PictureType>,

    /// Current frame in GOP
    current_frame: usize,

    /// Flag to track if mock data has been initialized
    mock_data_initialized: bool,
}

impl Default for Mpeg2Workspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Mpeg2Workspace {
    pub fn new() -> Self {
        Self {
            active_view: Mpeg2View::default(),
            show_mb_grid: true,
            show_slices: false,
            show_motion: false,
            show_dct: false,
            sequence: SequenceInfo::default(),
            current_gop: GopInfo::default(),
            picture_type: Mpeg2PictureType::P,
            picture_structure: PictureStructure::Frame,
            temporal_ref: 5,
            mock_mbs: Vec::new(),
            selected_mb: None,
            gop_pattern: Vec::new(),
            current_frame: 3,
            mock_data_initialized: false,
        }
    }

    /// Set view mode by index (F1-F4 keyboard shortcuts)
    pub fn set_mode_by_index(&mut self, index: usize) {
        self.active_view = match index {
            0 => Mpeg2View::Overview,
            1 => Mpeg2View::GopStructure,
            2 => Mpeg2View::Macroblocks,
            3 => Mpeg2View::MotionVectors,
            _ => return, // Ignore invalid indices
        };
    }

    /// Ensure mock data is initialized (lazy loading)
    fn ensure_mock_data(&mut self) {
        if !self.mock_data_initialized {
            self.generate_mock_data();
            self.mock_data_initialized = true;
        }
    }

    /// Generate mock data for demonstration
    fn generate_mock_data(&mut self) {
        self.mock_mbs.clear();
        self.gop_pattern.clear();

        // Generate 4x4 grid of MBs
        let mb_types = [
            Mpeg2MbType::ForwardPred,
            Mpeg2MbType::ForwardPred,
            Mpeg2MbType::Intra,
            Mpeg2MbType::Skipped,
            Mpeg2MbType::ForwardPred,
            Mpeg2MbType::Interpolated,
            Mpeg2MbType::BackwardPred,
            Mpeg2MbType::ForwardPred,
        ];

        let mut mb_idx = 0u32;
        for row in 0..4u32 {
            for col in 0..4u32 {
                let mb_type = mb_types[(row * 4 + col) as usize % mb_types.len()];
                let mb = Mpeg2Macroblock {
                    mb_idx,
                    x: col * 16,
                    y: row * 16,
                    mb_type,
                    qscale: 8 + (mb_idx as u8 % 16),
                    mv_forward: if matches!(
                        mb_type,
                        Mpeg2MbType::ForwardPred | Mpeg2MbType::Interpolated
                    ) {
                        Some(((col as i16 - 2) * 4, (row as i16 - 2) * 2))
                    } else {
                        None
                    },
                    mv_backward: if matches!(
                        mb_type,
                        Mpeg2MbType::BackwardPred | Mpeg2MbType::Interpolated
                    ) {
                        Some((-(col as i16 - 2) * 3, -(row as i16 - 2) * 2))
                    } else {
                        None
                    },
                    cbp: if mb_type == Mpeg2MbType::Skipped {
                        0
                    } else {
                        0x3F
                    },
                    pattern_code: 0x3F,
                };
                self.mock_mbs.push(mb);
                mb_idx += 1;
            }
        }

        // Generate typical GOP pattern: I B B P B B P B B P B B P B B
        self.gop_pattern = vec![
            Mpeg2PictureType::I,
            Mpeg2PictureType::B,
            Mpeg2PictureType::B,
            Mpeg2PictureType::P,
            Mpeg2PictureType::B,
            Mpeg2PictureType::B,
            Mpeg2PictureType::P,
            Mpeg2PictureType::B,
            Mpeg2PictureType::B,
            Mpeg2PictureType::P,
            Mpeg2PictureType::B,
            Mpeg2PictureType::B,
            Mpeg2PictureType::P,
            Mpeg2PictureType::B,
            Mpeg2PictureType::B,
        ];
    }

    /// Main UI entry point
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Lazy-load mock data on first show
        self.ensure_mock_data();

        ui.heading("MPEG-2 Video Analysis");
        ui.separator();

        // View selector tabs
        ui.horizontal(|ui| {
            for view in [
                Mpeg2View::Overview,
                Mpeg2View::GopStructure,
                Mpeg2View::Macroblocks,
                Mpeg2View::MotionVectors,
                Mpeg2View::Quantization,
            ] {
                if ui
                    .selectable_label(self.active_view == view, view.label())
                    .clicked()
                {
                    self.active_view = view;
                }
            }
        });

        ui.separator();

        // Render active view
        match self.active_view {
            Mpeg2View::Overview => self.show_overview(ui),
            Mpeg2View::GopStructure => self.show_gop_structure(ui),
            Mpeg2View::Macroblocks => self.show_macroblocks(ui),
            Mpeg2View::MotionVectors => self.show_motion_vectors(ui),
            Mpeg2View::Quantization => self.show_quantization(ui),
        }
    }

    /// Overview tab
    fn show_overview(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Left: Sequence info
            cols[0].group(|ui| {
                ui.heading("Sequence Header");
                ui.add_space(4.0);

                ui.label(format!(
                    "Resolution: {}x{}",
                    self.sequence.width, self.sequence.height
                ));
                ui.label(format!(
                    "Frame Rate: {}",
                    Self::frame_rate_string(self.sequence.frame_rate_code)
                ));
                ui.label(format!(
                    "Aspect Ratio: {}",
                    Self::aspect_ratio_string(self.sequence.aspect_ratio)
                ));
                ui.label(format!(
                    "Bit Rate: {:.2} Mbps",
                    self.sequence.bit_rate as f64 / 1_000_000.0
                ));
                ui.label(format!("Chroma: {}", self.sequence.chroma_format.label()));

                ui.add_space(8.0);

                ui.label(format!(
                    "Profile: {}",
                    Self::profile_name(self.sequence.profile)
                ));
                ui.label(format!("Level: {}", Self::level_name(self.sequence.level)));

                ui.horizontal(|ui| {
                    ui.label("Progressive:");
                    if self.sequence.progressive {
                        ui.label(RichText::new("Yes").color(Color32::GREEN));
                    } else {
                        ui.label(RichText::new("Interlaced").color(Color32::YELLOW));
                    }
                });
            });

            // Right: Current picture info
            cols[1].group(|ui| {
                ui.heading("Current Picture");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    let badge = RichText::new(format!(" {} ", self.picture_type.label()))
                        .background_color(self.picture_type.color())
                        .color(Color32::WHITE);
                    ui.label(badge);
                    ui.label(self.picture_type.description());
                });

                ui.label(format!("Temporal Ref: {}", self.temporal_ref));
                ui.label(format!("Structure: {}", self.picture_structure.label()));

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.heading("GOP Info");
                ui.label(format!(
                    "Time Code: {}",
                    self.current_gop.time_code_string()
                ));
                ui.horizontal(|ui| {
                    ui.label("Closed GOP:");
                    if self.current_gop.closed_gop {
                        ui.label(RichText::new("Yes").color(Color32::GREEN));
                    } else {
                        ui.label(RichText::new("No").color(Color32::YELLOW));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Broken Link:");
                    if self.current_gop.broken_link {
                        ui.label(RichText::new("Yes").color(Color32::RED));
                    } else {
                        ui.label(RichText::new("No").color(Color32::GREEN));
                    }
                });
            });
        });
    }

    /// GOP structure tab
    fn show_gop_structure(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("GOP Pattern");
            ui.add_space(4.0);

            // Visual GOP display
            ui.horizontal(|ui| {
                for (idx, pic_type) in self.gop_pattern.iter().enumerate() {
                    let is_current = idx == self.current_frame;
                    let bg_color = if is_current {
                        pic_type.color()
                    } else {
                        pic_type.color().gamma_multiply(0.5)
                    };
                    let border = if is_current { 2.0 } else { 0.0 };

                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(24.0, 32.0), egui::Sense::click());

                    if response.clicked() {
                        self.current_frame = idx;
                        self.picture_type = *pic_type;
                    }

                    let painter = ui.painter();
                    painter.rect(rect, 4.0, bg_color, Stroke::new(border, Color32::WHITE));
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        pic_type.label(),
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );

                    // Frame number below
                    painter.text(
                        rect.center() + Vec2::new(0.0, 22.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", idx),
                        egui::FontId::proportional(8.0),
                        Color32::GRAY,
                    );
                }
            });

            ui.add_space(8.0);
            ui.label(format!("GOP Size: {} pictures", self.gop_pattern.len()));
            ui.label("Pattern: I B B P B B P B B P B B P B B");
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            // Left: GOP types explanation
            cols[0].group(|ui| {
                ui.heading("Picture Types");
                ui.add_space(4.0);

                for pic_type in [
                    Mpeg2PictureType::I,
                    Mpeg2PictureType::P,
                    Mpeg2PictureType::B,
                ] {
                    ui.horizontal(|ui| {
                        let badge = RichText::new(format!(" {} ", pic_type.label()))
                            .background_color(pic_type.color())
                            .color(Color32::WHITE);
                        ui.label(badge);
                        ui.label(pic_type.description());
                    });
                }
            });

            // Right: Open vs Closed GOP
            cols[1].group(|ui| {
                ui.heading("GOP Modes");
                ui.add_space(4.0);

                ui.label(RichText::new("Closed GOP").strong());
                ui.label("B-pictures only reference within GOP");
                ui.label("Required for random access");

                ui.add_space(8.0);

                ui.label(RichText::new("Open GOP").strong());
                ui.label("B-pictures can reference previous GOP");
                ui.label("Better compression, less seek-friendly");
            });
        });

        // Reference structure diagram
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.heading("Reference Structure");

            let (_, painter) = ui.allocate_painter(Vec2::new(400.0, 100.0), egui::Sense::hover());

            let rect = painter.clip_rect();
            let box_width = 30.0;
            let box_height = 40.0;
            let spacing = 50.0;

            // Draw I -> P -> P chain with B frames
            let positions = [
                (rect.min.x + 20.0, "I", colors::PICTURE_I),
                (rect.min.x + 20.0 + spacing * 1.0, "B", colors::PICTURE_B),
                (rect.min.x + 20.0 + spacing * 2.0, "B", colors::PICTURE_B),
                (rect.min.x + 20.0 + spacing * 3.0, "P", colors::PICTURE_P),
                (rect.min.x + 20.0 + spacing * 4.0, "B", colors::PICTURE_B),
                (rect.min.x + 20.0 + spacing * 5.0, "B", colors::PICTURE_B),
                (rect.min.x + 20.0 + spacing * 6.0, "P", colors::PICTURE_P),
            ];

            let y_center = rect.min.y + rect.height() / 2.0;

            for (x, label, color) in positions {
                let box_rect = Rect::from_center_size(
                    egui::pos2(x, y_center),
                    Vec2::new(box_width, box_height),
                );
                painter.rect_filled(box_rect, 4.0, color);
                painter.text(
                    box_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }

            // Draw reference arrows
            // I -> P (index 0 -> 3)
            Self::draw_arrow(
                &painter,
                egui::pos2(
                    positions[0].0 + box_width / 2.0,
                    y_center - box_height / 2.0 - 5.0,
                ),
                egui::pos2(
                    positions[3].0 - box_width / 2.0,
                    y_center - box_height / 2.0 - 5.0,
                ),
                Color32::GREEN,
            );

            // P -> P (index 3 -> 6)
            Self::draw_arrow(
                &painter,
                egui::pos2(
                    positions[3].0 + box_width / 2.0,
                    y_center - box_height / 2.0 - 5.0,
                ),
                egui::pos2(
                    positions[6].0 - box_width / 2.0,
                    y_center - box_height / 2.0 - 5.0,
                ),
                Color32::GREEN,
            );
        });
    }

    /// Macroblocks tab
    fn show_macroblocks(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_mb_grid, "MB Grid");
            ui.checkbox(&mut self.show_slices, "Slices");
            ui.checkbox(&mut self.show_dct, "DCT Blocks");
        });

        ui.separator();

        // MB visualization
        let available = ui.available_size();
        let grid_size = available.x.min(available.y - 100.0).min(400.0);

        let (response, painter) = ui.allocate_painter(
            Vec2::new(grid_size, grid_size),
            egui::Sense::click_and_drag(),
        );

        let rect = response.rect;

        // Background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(30, 30, 35));

        // Scale factor: show 4x4 MBs (64x64 logical -> grid_size pixels)
        let display_size = 64.0;
        let scale = grid_size / display_size;

        // Draw MB boundaries if enabled
        if self.show_mb_grid {
            for x in (0..=64).step_by(16) {
                let px = rect.min.x + x as f32 * scale;
                painter.line_segment(
                    [egui::pos2(px, rect.min.y), egui::pos2(px, rect.max.y)],
                    Stroke::new(2.0, colors::MB_BOUNDARY),
                );
            }
            for y in (0..=64).step_by(16) {
                let py = rect.min.y + y as f32 * scale;
                painter.line_segment(
                    [egui::pos2(rect.min.x, py), egui::pos2(rect.max.x, py)],
                    Stroke::new(2.0, colors::MB_BOUNDARY),
                );
            }
        }

        // Draw DCT blocks (8x8) if enabled
        if self.show_dct {
            for x in (0..64).step_by(8) {
                let px = rect.min.x + x as f32 * scale;
                painter.line_segment(
                    [egui::pos2(px, rect.min.y), egui::pos2(px, rect.max.y)],
                    Stroke::new(0.5, colors::DCT_BLOCK),
                );
            }
            for y in (0..64).step_by(8) {
                let py = rect.min.y + y as f32 * scale;
                painter.line_segment(
                    [egui::pos2(rect.min.x, py), egui::pos2(rect.max.x, py)],
                    Stroke::new(0.5, colors::DCT_BLOCK),
                );
            }
        }

        // Draw MBs
        for (idx, mb) in self.mock_mbs.iter().enumerate() {
            if mb.x >= 64 || mb.y >= 64 {
                continue;
            }

            let mb_rect = Rect::from_min_size(
                egui::pos2(
                    rect.min.x + mb.x as f32 * scale,
                    rect.min.y + mb.y as f32 * scale,
                ),
                Vec2::splat(16.0 * scale),
            );

            // Fill with MB type color
            let color = mb.mb_type.color().gamma_multiply(0.4);
            painter.rect_filled(mb_rect, 0.0, color);

            // Draw type label
            painter.text(
                mb_rect.center(),
                egui::Align2::CENTER_CENTER,
                mb.mb_type.label(),
                egui::FontId::proportional(9.0),
                Color32::WHITE,
            );

            // Highlight selected
            if Some(idx) == self.selected_mb {
                painter.rect_stroke(mb_rect, 0.0, Stroke::new(2.0, Color32::YELLOW));
            }
        }

        // Handle click for selection
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let rel_x = ((pos.x - rect.min.x) / scale) as u32;
                let rel_y = ((pos.y - rect.min.y) / scale) as u32;
                let mb_x = (rel_x / 16) * 16;
                let mb_y = (rel_y / 16) * 16;

                self.selected_mb = self
                    .mock_mbs
                    .iter()
                    .position(|mb| mb.x == mb_x && mb.y == mb_y);
            }
        }

        // Legend
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Legend:");
            Self::legend_item(ui, colors::MB_INTRA, "Intra");
            Self::legend_item(ui, colors::MB_FORWARD, "Fwd");
            Self::legend_item(ui, colors::MB_BACKWARD, "Bwd");
            Self::legend_item(ui, colors::MB_INTERPOLATED, "Interp");
            Self::legend_item(ui, colors::MB_SKIPPED, "Skip");
        });

        // Selected MB info
        if let Some(idx) = self.selected_mb {
            if let Some(mb) = self.mock_mbs.get(idx) {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading(format!("MB #{}", mb.mb_idx));
                    ui.label(format!("Position: ({}, {})", mb.x, mb.y));
                    ui.label(format!("Type: {}", mb.mb_type.label()));
                    ui.label(format!("QScale: {}", mb.qscale));
                    ui.label(format!("CBP: 0b{:06b}", mb.cbp));
                    if let Some((mvx, mvy)) = mb.mv_forward {
                        ui.label(format!("MV Forward: ({}, {})", mvx, mvy));
                    }
                    if let Some((mvx, mvy)) = mb.mv_backward {
                        ui.label(format!("MV Backward: ({}, {})", mvx, mvy));
                    }
                });
            }
        }
    }

    /// Motion vectors tab
    fn show_motion_vectors(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_motion, "Show Motion Vectors");
        });

        ui.separator();

        ui.columns(2, |cols| {
            // Left: MV explanation
            cols[0].group(|ui| {
                ui.heading("Motion Compensation");
                ui.add_space(4.0);

                ui.label(RichText::new("Forward Prediction").strong());
                ui.label("Reference: previous I or P picture");
                ui.colored_label(colors::MB_FORWARD, "Color: Green");

                ui.add_space(8.0);

                ui.label(RichText::new("Backward Prediction").strong());
                ui.label("Reference: next I or P picture");
                ui.colored_label(colors::MB_BACKWARD, "Color: Blue");

                ui.add_space(8.0);

                ui.label(RichText::new("Bi-directional").strong());
                ui.label("Interpolation of forward and backward");
                ui.colored_label(colors::MB_INTERPOLATED, "Color: Purple");

                ui.add_space(8.0);

                ui.label("Motion vector precision: half-pel");
            });

            // Right: MV visualization
            cols[1].group(|ui| {
                ui.heading("MV Visualization");

                if self.show_motion {
                    let (_, painter) =
                        ui.allocate_painter(Vec2::new(200.0, 200.0), egui::Sense::hover());

                    let rect = painter.clip_rect();
                    let scale = rect.width() / 64.0;

                    // Background
                    painter.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 45));

                    // Draw MVs for each MB
                    for mb in &self.mock_mbs {
                        let center = egui::pos2(
                            rect.min.x + (mb.x as f32 + 8.0) * scale,
                            rect.min.y + (mb.y as f32 + 8.0) * scale,
                        );

                        if let Some((mvx, mvy)) = mb.mv_forward {
                            let end = center
                                + Vec2::new(mvx as f32 * scale * 0.5, mvy as f32 * scale * 0.5);
                            painter.arrow(
                                center,
                                end - center,
                                Stroke::new(1.5, colors::MB_FORWARD),
                            );
                        }

                        if let Some((mvx, mvy)) = mb.mv_backward {
                            let end = center
                                + Vec2::new(mvx as f32 * scale * 0.5, mvy as f32 * scale * 0.5);
                            painter.arrow(
                                center,
                                end - center,
                                Stroke::new(1.5, colors::MB_BACKWARD),
                            );
                        }
                    }
                } else {
                    ui.label("Enable checkbox to visualize motion vectors");
                }
            });
        });
    }

    /// Quantization tab
    fn show_quantization(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Quantization in MPEG-2");
            ui.add_space(4.0);

            ui.label("MPEG-2 uses two quantization methods:");
            ui.add_space(4.0);

            ui.label(RichText::new("Linear Scale").strong());
            ui.label("QScale = quantiser_scale_code × 2");
            ui.label("Range: 2 to 62");

            ui.add_space(8.0);

            ui.label(RichText::new("Non-linear Scale").strong());
            ui.label("Uses lookup table for finer control at low QP");
            ui.label("Range: 1 to 112");
        });

        ui.add_space(8.0);

        // QScale distribution visualization
        ui.group(|ui| {
            ui.heading("QScale Distribution");

            let (_, painter) = ui.allocate_painter(Vec2::new(300.0, 100.0), egui::Sense::hover());

            let rect = painter.clip_rect();

            // Draw QScale histogram (mock data)
            let bar_width = rect.width() / 32.0;
            let mock_hist = [0, 1, 2, 4, 8, 12, 16, 14, 10, 8, 6, 4, 3, 2, 1, 0];

            let max_val = mock_hist.iter().max().copied().unwrap_or(1) as f32;

            for (i, &count) in mock_hist.iter().enumerate() {
                let height = (count as f32 / max_val) * (rect.height() - 20.0);
                let bar_rect = Rect::from_min_size(
                    egui::pos2(
                        rect.min.x + i as f32 * bar_width * 2.0,
                        rect.max.y - height - 15.0,
                    ),
                    Vec2::new(bar_width * 1.5, height),
                );

                // Color based on QScale (blue = low, red = high)
                let t = i as f32 / 15.0;
                let color = Color32::from_rgb(
                    (t * 255.0) as u8,
                    ((1.0 - (t - 0.5).abs() * 2.0) * 200.0) as u8,
                    ((1.0 - t) * 255.0) as u8,
                );

                painter.rect_filled(bar_rect, 2.0, color);
            }

            // Labels
            painter.text(
                rect.min + Vec2::new(5.0, rect.height() - 10.0),
                egui::Align2::LEFT_BOTTOM,
                "Low QP",
                egui::FontId::proportional(9.0),
                Color32::WHITE,
            );
            painter.text(
                rect.max + Vec2::new(-5.0, -10.0),
                egui::Align2::RIGHT_BOTTOM,
                "High QP",
                egui::FontId::proportional(9.0),
                Color32::WHITE,
            );
        });

        ui.add_space(8.0);

        // Quantization matrices
        ui.group(|ui| {
            ui.heading("Quantization Matrices");
            ui.add_space(4.0);

            ui.label("MPEG-2 uses 8x8 quantization matrices:");
            ui.label("• Intra matrix (default or custom)");
            ui.label("• Non-intra matrix (default or custom)");

            ui.add_space(4.0);

            ui.label("Default intra matrix has lower values in");
            ui.label("upper-left (DC/low frequency) for better quality.");
        });
    }

    /// Draw an arrow between two points
    fn draw_arrow(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, color: Color32) {
        painter.line_segment([from, to], Stroke::new(1.5, color));

        // Arrow head
        let dir = (to - from).normalized();
        let perp = Vec2::new(-dir.y, dir.x);
        let arrow_size = 5.0;

        painter.line_segment(
            [to, to - dir * arrow_size + perp * arrow_size * 0.5],
            Stroke::new(1.5, color),
        );
        painter.line_segment(
            [to, to - dir * arrow_size - perp * arrow_size * 0.5],
            Stroke::new(1.5, color),
        );
    }

    /// Get frame rate string from code
    fn frame_rate_string(code: u8) -> &'static str {
        match code {
            1 => "23.976 fps",
            2 => "24 fps",
            3 => "25 fps",
            4 => "29.97 fps",
            5 => "30 fps",
            6 => "50 fps",
            7 => "59.94 fps",
            8 => "60 fps",
            _ => "Unknown",
        }
    }

    /// Get aspect ratio string from code
    fn aspect_ratio_string(code: u8) -> &'static str {
        match code {
            1 => "1:1 (Square)",
            2 => "4:3",
            3 => "16:9",
            4 => "2.21:1",
            _ => "Unknown",
        }
    }

    /// Get profile name from code
    fn profile_name(code: u8) -> &'static str {
        match code {
            1 => "High",
            2 => "Spatially Scalable",
            3 => "SNR Scalable",
            4 => "Main",
            5 => "Simple",
            _ => "Unknown",
        }
    }

    /// Get level name from code
    fn level_name(code: u8) -> &'static str {
        match code {
            4 => "High",
            6 => "High 1440",
            8 => "Main",
            10 => "Low",
            _ => "Unknown",
        }
    }

    /// Helper to draw legend item
    fn legend_item(ui: &mut egui::Ui, color: Color32, label: &str) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);
            ui.label(label);
        });
    }
}
