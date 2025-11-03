/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

pub(crate) fn int_sh_weighted_sum(left: u32, right: u32, right_weight: u32) -> u8 {
    let left_weight = 256 - right_weight;

    let ll = left * left_weight;
    let rr = right * right_weight;
    // let ret = 0xFFFF - (0xFFFF - ll) * (0xFFFF - rr);
    // to allow overflow
    let ret = 0xFFFFu32
        .wrapping_sub((0xFFFFu32.wrapping_sub(ll)).wrapping_mul(0xFFFFu32.wrapping_sub(rr)));

    (ret >> 24) as u8
}

#[allow(unused)]
pub fn sh_weighted_sum<P: image::Pixel>(left: P, right: P, right_weight: u32) -> P
where
    P::Subpixel: Into<u8> + From<u8>,
{
    left.map2(&right, |p, q| {
        let lp: u32 = p.into().into();
        let rq: u32 = q.into().into();
        int_sh_weighted_sum(lp, rq, right_weight).into()
    })
}

pub(crate) fn int_weighted_sum(left: u32, right: u32, right_weight: u32) -> u8 {
    let left_weight = 256 - right_weight;
    let ret = (left * left_weight) + (right * right_weight);
    (ret >> 8) as u8
}

#[allow(unused)]
pub fn weighted_sum<P: image::Pixel>(left: P, right: P, right_weight: u32) -> P
where
    P::Subpixel: Into<u8> + From<u8>,
{
    left.map2(&right, |p, q| {
        let lp: u32 = p.into().into();
        let rq: u32 = q.into().into();
        int_weighted_sum(lp, rq, right_weight).into()
    })
}

#[allow(unused)]
pub(crate) fn int_weighted_sum_rev(left: u32, right: u32, right_weight: u32) -> u8 {
    let left_weight = 256 - right_weight;

    let bg_factor = 64 + (((256 - left) * 192) >> 8);

    let adjusted_weight = (right_weight * bg_factor) >> 8;

    let ret = left * (256 - adjusted_weight) + right * adjusted_weight;
    ((ret + 128) >> 8).clamp(0, 255) as u8
}
