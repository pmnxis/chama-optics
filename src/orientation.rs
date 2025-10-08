/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

/// Orientation (TIFF tag 0x112)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    None,           // 0
    Normal,         // 1
    FlipHorizontal, // 2
    Rotate180,      // 3
    FlipVertical,   // 4
    Transpose,      // 5
    Rotate90,       // 6
    Transverse,     // 7
    Rotate270,      // 8
    Reserved(u32),
}

impl Orientation {
    #[allow(unused)]
    pub fn from_tiff(value: u32) -> Self {
        match value {
            0 => Orientation::None,
            1 => Orientation::Normal,
            2 => Orientation::FlipHorizontal,
            3 => Orientation::Rotate180,
            4 => Orientation::FlipVertical,
            5 => Orientation::Transpose,
            6 => Orientation::Rotate90,
            7 => Orientation::Transverse,
            8 => Orientation::Rotate270,
            other => Orientation::Reserved(other),
        }
    }

    #[allow(unused)]
    pub fn description(&self) -> &'static str {
        match self {
            Orientation::None => "no orientation info",
            Orientation::Normal => "row 0 at top and column 0 at left",
            Orientation::FlipHorizontal => "row 0 at top and column 0 at right",
            Orientation::Rotate180 => "row 0 at bottom and column 0 at right",
            Orientation::FlipVertical => "row 0 at bottom and column 0 at left",
            Orientation::Transpose => "row 0 at left and column 0 at top",
            Orientation::Rotate90 => "row 0 at right and column 0 at top",
            Orientation::Transverse => "row 0 at right and column 0 at bottom",
            Orientation::Rotate270 => "row 0 at left and column 0 at bottom",
            Orientation::Reserved(_) => "reserved orientation value",
        }
    }

    #[allow(unused)]
    pub fn egui_rotate(&self) -> (f32, egui::Vec2) {
        let origin = egui::Vec2::splat(0.5); // center

        let angle_deg: f32 = match self {
            Orientation::None | Orientation::Normal => 0.0,
            Orientation::Rotate90 => 90.0,
            Orientation::Rotate180 => 180.0,
            Orientation::Rotate270 => 270.0,
            Orientation::FlipHorizontal => 0.0,
            Orientation::FlipVertical => 0.0,
            Orientation::Transpose => 0.0,
            Orientation::Transverse => 180.0,
            Orientation::Reserved(_) => 0.0,
        };

        let angle_rad: f32 = angle_deg.to_radians();
        (angle_rad, origin)
    }

    #[allow(unused)]
    pub fn is_vertical_rotated(&self) -> bool {
        matches!(self, Orientation::Rotate90 | Orientation::Rotate270)
    }
}

impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.description())
    }
}
