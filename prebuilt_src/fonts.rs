/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

mod builtin_fonts {
    include!("../src/fonts/builtin_fonts.rs");

    impl BuildAsset {
        /// Download, verify MD5, unzip (if needed), and set cargo env var
        pub fn load(&self, out_dir: &std::path::Path) {
            use md5::{Digest, Md5};
            use std::fs;
            use std::io::{self, Cursor, Read};
            use zip::ZipArchive;

            let file_name = self.file_name.unwrap_or_else(|| {
                self.url
                    .split('/')
                    .next_back()
                    .expect("Cannot determine file name from URL")
            });

            let zip_path = out_dir.join(file_name);

            // Ensure out_dir exists
            if !out_dir.exists() {
                fs::create_dir_all(out_dir).expect("failed to create out_dir");
            }
            println!("cargo:rerun-if-changed=build.rs");

            // Download
            if !zip_path.exists() {
                println!("Downloading {} ...", self.url);
                let resp = reqwest::blocking::get(self.url).expect("failed to download file");
                let bytes = resp.bytes().expect("failed to read response bytes");
                fs::write(&zip_path, &bytes).expect("failed to write downloaded file");
            }

            // MD5 check
            let mut buffer = Vec::new();
            fs::File::open(&zip_path)
                .and_then(|mut f| f.read_to_end(&mut buffer))
                .expect("failed to read downloaded file for MD5");

            let mut hasher = Md5::new();
            hasher.update(&buffer);
            let actual_md5 = format!("{:x}", hasher.finalize());

            if actual_md5 != self.expected_md5 {
                panic!(
                    "MD5 checksum mismatch for {}!\nExpected: {}\nActual:   {}",
                    zip_path.display(),
                    self.expected_md5,
                    actual_md5
                );
            } else {
                println!("MD5 checksum verified ✅");
            }

            // Unzip if necessary
            if self.unzip {
                let extract_list = self
                    .extract_file_names
                    .expect("extract_file_names required when unzip=true");
                let env_keys = self.env_keys.expect("env_keys required when unzip=true");

                if extract_list.len() != env_keys.len() {
                    panic!("extract_file_names and env_keys must have the same length");
                }

                let reader = Cursor::new(buffer);
                let mut archive = ZipArchive::new(reader).expect("failed to open zip archive");

                for (extract_name, env_key) in extract_list.iter().zip(env_keys.iter()) {
                    let out_path = out_dir.join(extract_name);

                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent).unwrap_or_else(|e| {
                            panic!("failed to create directory {parent:?}: {e}")
                        });
                    }

                    if !out_path.exists() {
                        println!("Extracting {extract_name} ...");
                        let mut file = archive
                            .by_name(extract_name)
                            .unwrap_or_else(|_| panic!("{extract_name} not found in ZIP"));
                        let mut extracted = Vec::new();
                        io::copy(&mut file, &mut extracted).expect("failed to extract file");
                        fs::write(&out_path, extracted).expect("failed to write extracted file");
                    }

                    // Export environment variable per file
                    println!("cargo:rustc-env={}={}", env_key, out_path.display());
                }
            } else {
                // If not unzip, assign directly
                if let Some(env_keys) = self.env_keys {
                    for env_key in env_keys {
                        println!("cargo:rustc-env={}={}", env_key, zip_path.display());
                    }
                }
            }
        }
    }
}
