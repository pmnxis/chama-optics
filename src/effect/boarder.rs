/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Border {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
    pub color: egui::Color32,
}

impl Border {
    #[allow(unused)]
    pub const fn uniform(size: u32, color: egui::Color32) -> Self {
        Self {
            left: size,
            right: size,
            top: size,
            bottom: size,
            color,
        }
    }

    #[allow(unused)]
    pub const fn bottom(size: u32, color: egui::Color32) -> Self {
        Self {
            left: 0,
            right: 0,
            top: 0,
            bottom: size,
            color,
        }
    }

    #[allow(unused)]
    pub const fn top_and_bottom(size: u32, color: egui::Color32) -> Self {
        Self {
            left: 0,
            right: 0,
            top: size,
            bottom: size,
            color,
        }
    }

    pub fn take_from_exist(&self, img: &image::DynamicImage) -> image::DynamicImage {
        use image::GenericImage;
        use imageproc::drawing::Canvas;

        let (w, h) = img.dimensions();
        let new_w = w + self.left + self.right;
        let new_h = h + self.top + self.bottom;
        let color = crate::theme::color32_to_rgba(self.color);
        let mut bordered = image::DynamicImage::new_rgba8(new_w, new_h);
        let inner = bordered.as_mut_rgba8().unwrap();

        // let color_u32 = u32::from_le_bytes([color.r(), color.g(), color.b(), color.a()]);
        // unsafe {
        //     let buf = inner.as_flat_samples_mut().samples.as_mut_ptr();
        //     let len = inner.as_flat_samples_mut().samples.len();
        //
        //     let pixel_count = len / 4;
        //     let buf32 = buf as *mut u32;
        //
        //     for i in 0..pixel_count {
        //         core::ptr::write(buf32.add(i), color_u32);
        //     }
        // }

        unsafe {
            let buf = inner.as_flat_samples_mut().samples;
            let len = buf.len();
            let mut i = 0;
            while i + 3 < len {
                *buf.get_unchecked_mut(i) = color[0];
                *buf.get_unchecked_mut(i + 1) = color[1];
                *buf.get_unchecked_mut(i + 2) = color[2];
                *buf.get_unchecked_mut(i + 3) = color[3];
                i += 4;
            }
        }

        bordered.copy_from(img, self.left, self.top).unwrap();
        bordered
    }
}
