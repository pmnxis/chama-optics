/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use md5::{Digest, Md5};
use std::{
    env, fs,
    io::{Cursor, Read},
    path::PathBuf,
};
use zip::ZipArchive;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let font_path = out_dir.join("DS-DIGI.TTF");
    let zip_path = out_dir.join("ds_digital.zip");

    println!("cargo:rerun-if-changed=build.rs");

    // Download DS-Digital.zip
    let url = "https://github.com/aur-archive/ds-digital-fonts/raw/master/ds_digital.zip";
    let expected_md5 = "d4b7aea7106cc00daef73d51eeda826c";

    // if there's no DSDigital
    if !zip_path.exists() {
        println!("Downloading ds_digital.zip ...");
        let resp = reqwest::blocking::get(url).expect("failed to download zip");
        let bytes = resp.bytes().expect("failed to read zip bytes");
        fs::write(&zip_path, &bytes).expect("failed to write zip file");
    }

    let mut file = fs::File::open(&zip_path).expect("failed to open zip file");
    let mut hasher = Md5::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    hasher.update(&buffer);
    let result = format!("{:x}", hasher.finalize());

    if result != expected_md5 {
        panic!(
            "MD5 checksum mismatch for ds_digital.zip!\nExpected: {}\nActual:   {}",
            expected_md5, result
        );
    } else {
        println!("MD5 checksum verified ✅");
    }

    // uncompress
    if !font_path.exists() {
        let reader = Cursor::new(buffer);
        let mut archive = ZipArchive::new(reader).expect("failed to open zip archive");

        // add DS-DIGI.TTF
        let mut file = archive
            .by_name("DS-DIGI.TTF")
            .expect("DS-Digital.ttf not found in ZIP");

        let mut buf = Vec::new();
        std::io::copy(&mut file, &mut buf).expect("failed to extract font file");

        fs::write(&font_path, buf).expect("failed to write extracted font");
    }

    // add on OUT_DIR
    println!(
        "cargo:rustc-env=DS_DIGITAL_FONT_PATH={}",
        font_path.display()
    );
}
