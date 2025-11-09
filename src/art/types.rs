/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ColorType {
    Black,
    BlackMixed,
    Color,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum FillOperation {
    Default,
    Monochrome,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MnfRelation {
    Any,
    Both,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct ArtAsset {
    pub key: &'static str,
    pub data: &'static [u8],
    pub color_type: ColorType,
    pub fill_ops: FillOperation,
    pub mnf: &'static str,
    pub model: &'static str,
    pub mnf_model_rel: MnfRelation,
}
