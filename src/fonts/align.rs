/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[derive(Clone, Default, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

impl TextAlign {
    #[cfg(not(feature = "ios_integration"))]
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        use rust_i18n::t;

        ui.horizontal(|ui| {
            ui.selectable_value(self, TextAlign::Left, t!("text_align.left"))
                .on_hover_text(t!("text_align.left_hint"));

            ui.selectable_value(self, TextAlign::Center, t!("text_align.center"))
                .on_hover_text(t!("text_align.center_hint"));

            ui.selectable_value(self, TextAlign::Right, t!("text_align.right"))
                .on_hover_text(t!("text_align.right_hint"));
        });
    }

    pub fn x_point<I0, I1, I2, I3>(&self, ll: I0, dyn_w: I1, gap: I2, text_width: I3) -> i32
    where
        I0: Into<i64> + Copy,
        I1: Into<i64> + Copy,
        I2: Into<i64> + Copy,
        I3: Into<i64> + Copy,
    {
        let ll = ll.into();
        let dyn_w = dyn_w.into();
        let gap = gap.into();
        let text_width = text_width.into();

        let ret = match *self {
            TextAlign::Left => ll + gap,
            TextAlign::Center => ((dyn_w / 2) + ll).max(ll) - (text_width / 2),
            TextAlign::Right => (ll + dyn_w - text_width - gap).max(0),
        };

        ret as i32
    }
}
