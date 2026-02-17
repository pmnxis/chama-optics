/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

mod builtin_logos {
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    include!("../src/art/types.rs");

    #[derive(Debug, serde::Deserialize)]
    struct BuildArtAsset {
        key: String,
        url: String,
        expected_md5: String,
        color_type: String,
        fill_ops: String,
        mnf: String,
        model: String,
        mnf_model_rel: String,
    }

    impl BuildArtAsset {
        /// Fallback mirror on GitHub for when primary URLs (e.g. Wikipedia) fail.
        const FALLBACK_BASE: &'static str =
            "https://raw.githubusercontent.com/pmnxis/chama-optics-assets/master/logo_mnf";

        pub fn load(&self, out_dir: &Path) -> PathBuf {
            use md5::{Digest, Md5};

            const MAX_RETRIES: usize = 3;
            const RETRY_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

            let file_path = out_dir.join(self.key.clone());

            println!("cargo:rerun-if-changed=build.rs");

            let user_agent = format!(
                "{}/{} ({}; {})",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                option_env!("CARGO_PKG_REPOSITORY").unwrap_or("https://example.org"),
                "pmnxis@gmail.com"
            );

            println!("{user_agent}");

            // Download
            if !file_path.exists() {
                println!("Downloading {} ...", self.url);

                let bytes: Vec<u8> = if self.url.starts_with("http://")
                    || self.url.starts_with("https://")
                {
                    let client = reqwest::blocking::Client::builder()
                        .user_agent(&user_agent)
                        .gzip(true)
                        .build()
                        .expect("failed to build reqwest client");

                    let fallback_url = format!("{}/{}", Self::FALLBACK_BASE, self.key);

                    // Try primary URL first, then fallback
                    let urls = [self.url.as_str(), fallback_url.as_str()];

                    let mut final_bytes = None;

                    for (url_idx, url) in urls.iter().enumerate() {
                        let is_primary = url_idx == 0;
                        let label = if is_primary { "primary" } else { "fallback" };

                        println!("cargo:warning=Trying {} URL: {}", label, url);

                        let mut last_error = None;

                        for attempt in 0..=MAX_RETRIES {
                            if attempt > 0 {
                                println!(
                                    "cargo:warning=Retrying {} attempt {}/{} after {} seconds...",
                                    label,
                                    attempt,
                                    MAX_RETRIES,
                                    RETRY_DURATION.as_secs()
                                );
                                std::thread::sleep(RETRY_DURATION);
                            }

                            let resp = match client.get(*url).send() {
                                Ok(r) => r,
                                Err(e) => {
                                    last_error =
                                        Some(format!("Failed to download {}: {e}", url));
                                    continue;
                                }
                            };

                            if resp.status().is_success() {
                                let bytes = resp
                                    .bytes()
                                    .expect("failed to read response bytes")
                                    .to_vec();
                                final_bytes = Some(bytes);
                                last_error = None;
                                break;
                            } else {
                                last_error = Some(format!(
                                    "HTTP {} for {}",
                                    resp.status(),
                                    url
                                ));
                                if resp.status() == reqwest::StatusCode::FORBIDDEN
                                    || resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                                {
                                    if attempt < MAX_RETRIES {
                                        continue;
                                    }
                                } else {
                                    // Non-retryable error, break to try fallback
                                    break;
                                }
                            }
                        }

                        if final_bytes.is_some() {
                            break;
                        }

                        if let Some(ref err) = last_error {
                            if is_primary {
                                println!(
                                    "cargo:warning=Primary download failed ({}), trying fallback...",
                                    err
                                );
                            } else {
                                panic!("All download attempts failed for {}: {}", self.key, err);
                            }
                        }
                    }

                    final_bytes.expect("Should have downloaded bytes")
                } else {
                    // use local file
                    let src_path = PathBuf::from(&self.url);
                    fs::read(&src_path)
                        .unwrap_or_else(|e| panic!("Failed to read local file {src_path:?}: {e}"))
                };

                fs::write(&file_path, &bytes).expect("failed to write downloaded file");
            } else {
                println!("File already exists, verifying MD5 ...");
            }

            // MD5 check
            let mut buffer = Vec::new();
            fs::File::open(&file_path)
                .and_then(|mut f| f.read_to_end(&mut buffer))
                .expect("failed to read file for MD5");

            let mut hasher = Md5::new();
            hasher.update(&buffer);
            let actual_md5 = format!("{:x}", hasher.finalize());

            if actual_md5 != self.expected_md5 {
                panic!(
                    "MD5 mismatch for {}\nExpected: {}\nActual:   {}",
                    file_path.display(),
                    self.expected_md5,
                    actual_md5
                );
            } else {
                println!("MD5 verified for {}", self.key);
            }

            // println!("cargo:rustc-env=ASSET_PATH_{}={}", self.key, file_path.display());

            file_path
        }

        pub fn display(&self, out_dir: &Path) -> String {
            let file_path = out_dir.join(self.key.clone());
            let abs_path = file_path.canonicalize().unwrap();

            format!(
                "    ArtAsset {{\n        key: \"{}\",\n        data: include_bytes!(r#\"{}\"#),\n        color_type: ColorType::{},\n        fill_ops: FillOperation::{},\n        mnf: \"{}\",\n        model: \"{}\",\n        mnf_model_rel: MnfRelation::{},\n    }},\n",
                self.key,
                abs_path.display(),
                self.color_type,
                self.fill_ops,
                self.mnf.to_ascii_lowercase(),
                self.model.to_ascii_lowercase(),
                self.mnf_model_rel
            )
        }
    }

    pub fn generate(out_dir: &Path, csv_path: &Path) -> String {
        let mut rdr = csv::Reader::from_path(csv_path).expect("failed to open CSV");
        let assets: Vec<BuildArtAsset> = rdr
            .deserialize()
            .map(|r| r.expect("invalid CSV entry"))
            .collect();

        let mut decls = String::new();
        decls.push_str("// AUTO-GENERATED by build.rs\n");
        decls.push_str("use crate::art::types::*;\n\n");
        decls.push_str("pub const LOGO_ASSETS: &[ArtAsset] = &[\n");

        for asset in &assets {
            let _ffp = asset.load(out_dir);
            decls.push_str(&asset.display(out_dir));
        }

        decls.push_str("];\n");

        decls
    }
}

pub fn write_if_changed<P: AsRef<std::path::Path>>(path: P, new_content: &str) {
    let path = path.as_ref();

    let old_content = std::fs::read_to_string(path).ok();

    if let Some(old) = old_content
        && old == new_content
    {
        println!("[PASS] No change detected in {}", path.display());
        return;
    }

    std::fs::write(path, new_content).expect("failed to write generated file");
    println!("✏️  Updated {}", path.display());
}
