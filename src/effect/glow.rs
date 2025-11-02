/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use image::ImageBuffer;

// // todo! - reduce glow layer computing because only short area compute area
// pub struct GlowManager {
//     pub text_layer: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
//     pub glow_radius: f32,
// }

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
    // let num_pixel = new.get_pixel(0, 0).channels();
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
            *new.get_unchecked_mut(idx * 4 + 3) = alpha_mul!(base, idx * 4 + 2, color[3] as u16);
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
    fn int_sh_weighted_sum(left: u32, right: u32, right_weight: u32) -> u8 {
        let left_weight = 256 - right_weight;

        let ll = left * left_weight;
        let rr = right * right_weight;
        // let ret = 0xFFFF - (0xFFFF - ll) * (0xFFFF - rr);
        // to allow overflow
        let ret = 0xFFFFu32.wrapping_sub(
            (0xFFFFu32.wrapping_sub(ll)).wrapping_mul(0xFFFFu32.wrapping_sub(rr)),
        );

        (ret >> 24) as u8
    }

    fn int_weighted_sum(left: u32, right: u32, right_weight: u32) -> u8 {
        let left_weight = 256 - right_weight;
        let ret = (left * left_weight) + (right * right_weight);
        (ret >> 8) as u8
    }
    fn int_weighted_sum_rev(left: u32, right: u32, right_weight: u32) -> u8 {
        let left_weight = 256 - right_weight;

        let bg_factor = 64 + (((256 - left) * 192) >> 8);

        let adjusted_weight = (right_weight * bg_factor) >> 8;

        let ret = left * (256 - adjusted_weight) + right * adjusted_weight;
        ((ret + 128) >> 8).clamp(0, 255) as u8
    }

    fn flow_glow(back: u32, glow: u32, glow_weight: u32, text: u32, text_weight: u32) -> u8 {
        // let first = int_weighted_sum(back, , right_weight)
        // let first = int_sh_weighted_sum(back, glow, glow_weight);
        let first = int_sh_weighted_sum(back, glow, glow_weight);
        // first
        int_weighted_sum(first as u32, text, text_weight)
    }

    let (width, height) = (luma_text.width(), luma_text.height());
    // let num_pixel = new.get_pixel(0, 0).channels();
    let max_idx = (width * height) as usize;
    let luma_blurred = imageproc::filter::gaussian_blur_f32(luma_text, glow_radius);

    for idx in 0..max_idx {
        unsafe {
            let blr_l = *luma_blurred.get_unchecked(idx) as u32;

            let txt_l = *luma_text.get_unchecked(idx) as u32;

            if (blr_l == 0) && (txt_l == 0) {
                continue;
            }

            let r = base.get_unchecked_mut(idx * 4);
            *r = flow_glow(
                *r as u32,
                glow_color[0] as u32,
                blr_l,
                text_color[0] as u32,
                txt_l,
            );

            // let bt_g = (text_color[1] as u32) * (bt_l) >> 8;
            let g = base.get_unchecked_mut(idx * 4 + 1);
            *g = flow_glow(
                *g as u32,
                glow_color[1] as u32,
                blr_l,
                text_color[1] as u32,
                txt_l,
            );

            // let bt_b = (text_color[2] as u32) * (bt_l) >> 8;
            let b = base.get_unchecked_mut(idx * 4 + 2);
            *b = flow_glow(
                *b as u32,
                glow_color[2] as u32,
                blr_l,
                text_color[2] as u32,
                txt_l,
            );

            let a = base.get_unchecked_mut(idx * 4 + 3);
            *a = (*a).max(txt_l.min(255) as u8);
        }
    }
}
