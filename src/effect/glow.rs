/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use crate::effect::custom_weighted_sum::*;
use image::ImageBuffer;

macro_rules! alpha_mul {
    ($image:expr, $subpixel_idx:expr, $alpha:expr) => {
        ((*$image.get_unchecked($subpixel_idx) as u16 * $alpha) >> 8) as u8
    };
}

#[allow(unused)]
pub fn multiply_color(
    base: &ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    color: image::Rgba<u8>,
) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let (width, height) = (base.width(), base.height());
    let mut new = image::RgbaImage::new(width, height);
    let max_idx = (width * height) as usize;

    for idx in 0..max_idx {
        unsafe {
            let a = *base.get_unchecked(idx * 4 + 3);
            if a == 0u8 {
                continue;
            }
            *new.get_unchecked_mut(idx * 4) = alpha_mul!(base, idx * 4, color[0] as u16);
            *new.get_unchecked_mut(idx * 4 + 1) = alpha_mul!(base, idx * 4 + 1, color[1] as u16);
            *new.get_unchecked_mut(idx * 4 + 2) = alpha_mul!(base, idx * 4 + 2, color[2] as u16);
            *new.get_unchecked_mut(idx * 4 + 3) = alpha_mul!(base, idx * 4 + 3, color[3] as u16);
        }
    }
    new
}

#[allow(unused)]
pub fn final_glow_effect(
    base: &mut ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    luma_text: &ImageBuffer<image::Luma<u8>, Vec<u8>>,
    text_color: image::Rgba<u8>,
    glow_color: image::Rgba<u8>,
    glow_radius: f32,
) {
    #[inline(always)]
    fn flow_glow(back: u32, glow: u32, glow_weight: u32, text: u32, text_weight: u32) -> u8 {
        let first = int_sh_weighted_sum(back, glow, glow_weight);
        int_weighted_sum(first as u32, text, text_weight)
    }

    let (width, height) = (luma_text.width(), luma_text.height());
    let max_idx = (width * height) as usize;
    let luma_blurred = imageproc::filter::gaussian_blur_f32(luma_text, glow_radius);

    let base_pixels: &mut [u8] = base.as_mut();
    let blur_pixels = luma_blurred.as_raw();
    let text_pixels = luma_text.as_raw();

    for idx in 0..max_idx {
        let blr_l = blur_pixels[idx] as u32;
        let txt_l = text_pixels[idx] as u32;

        if blr_l == 0 && txt_l == 0 {
            continue;
        }

        let i = idx * 4;
        base_pixels[i] = flow_glow(
            base_pixels[i] as u32,
            glow_color[0] as u32,
            blr_l,
            text_color[0] as u32,
            txt_l,
        );
        base_pixels[i + 1] = flow_glow(
            base_pixels[i + 1] as u32,
            glow_color[1] as u32,
            blr_l,
            text_color[1] as u32,
            txt_l,
        );
        base_pixels[i + 2] = flow_glow(
            base_pixels[i + 2] as u32,
            glow_color[2] as u32,
            blr_l,
            text_color[2] as u32,
            txt_l,
        );
        base_pixels[i + 3] = base_pixels[i + 3].max(txt_l.min(255) as u8);
    }
}
