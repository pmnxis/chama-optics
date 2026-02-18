/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

pub mod art_impl;
pub mod types;

use types::ArtAsset;

#[cfg(has_logo_asset_path)]
include!(env!("LOGO_ASSET_PATH"));
#[cfg(not(has_logo_asset_path))]
include!("../../assets/auto_generated/logo_assets.rs");

pub const ART_UNIFY: ArtUnify = ArtUnify {
    builtin_logos: LOGO_ASSETS,
};

pub struct ArtUnify {
    pub builtin_logos: &'static [ArtAsset],
}

impl ArtUnify {
    pub fn get_camera_logo(
        &'static self,
        exif: &crate::exif_impl::SimplifiedExif,
    ) -> Option<&'static types::ArtAsset> {
        let param = (exif.camera_mnf.as_str(), exif.camera_model.as_str());

        types::ArtAsset::get_match_arr(self.builtin_logos, param)
    }
}
