/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use chrono::{DateTime, Utc};
#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
use rust_i18n::t;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub enum CheckGithubReleaseState {
    NotChecked,
    Checking,
    CheckedNewUpdate,
    Checked,
}

#[allow(dead_code)]
pub struct CheckRelease {
    pub status: Arc<RwLock<CheckGithubReleaseState>>,
    pub new_version: Arc<RwLock<Option<(String, String)>>>,
}

#[cfg(not(target_arch = "wasm32"))]
fn get_latest_stable_release() -> Option<(String, String)> {
    let repo_url = env!("CARGO_PKG_REPOSITORY");
    let build_time_str = env!("BUILD_TIME");
    let build_time: DateTime<Utc> = build_time_str.parse().unwrap();

    let parts: Vec<&str> = repo_url.trim_end_matches('/').split('/').collect();
    if parts.len() < 2 {
        return None;
    }

    let owner = parts[parts.len() - 2];
    let repo = parts[parts.len() - 1];
    let api_url = format!("https://api.github.com/repos/{owner}/{repo}/releases");

    let client = Client::new();
    let resp = client
        .get(&api_url)
        .header("User-Agent", env!("CARGO_PKG_NAME"))
        .send()
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let releases: Vec<GitHubRelease> = resp.json().ok()?;

    for rel in releases {
        // Skip draft or prerelease
        if rel.draft || rel.prerelease {
            continue;
        }

        // Skip pre-release suffixes (alpha/beta/gamma/delta/rc etc.)
        let tag = rel.tag_name.to_lowercase();
        if tag.contains("alpha")
            || tag.contains("beta")
            || tag.contains("gamma")
            || tag.contains("delta")
            || tag.contains("rc")
        {
            continue;
        }

        // Check if published after our build
        if let Some(published) = rel.published_at {
            if published > build_time + chrono::Duration::hours(8) {
                return Some((rel.tag_name, rel.html_url));
            } else {
                log::debug!("{} - {} found but pass", rel.tag_name, rel.html_url);
            }
        } else {
            log::debug!("{} - {} found but pass", rel.tag_name, rel.html_url);
        }
    }

    None
}

impl Default for CheckRelease {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckRelease {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        let status: Arc<RwLock<CheckGithubReleaseState>> =
            Arc::new(RwLock::new(CheckGithubReleaseState::Checking));
        let new_version: Arc<RwLock<Option<(String, String)>>> = Arc::new(RwLock::new(None));

        let status_clone = status.clone();
        let new_version_clone = new_version.clone();

        thread::spawn(move || {
            let before_start = std::time::Instant::now();

            log::info!("Let's check release from internet");
            let got = get_latest_stable_release();
            let is_new = got.is_some();
            *new_version_clone.write().unwrap() = got.clone();
            *status_clone.write().unwrap() = if is_new {
                CheckGithubReleaseState::CheckedNewUpdate
            } else {
                CheckGithubReleaseState::Checked
            };

            log::info!(
                "{} msecs passed for getting new version info : {:?}",
                (std::time::Instant::now() - before_start).as_millis(),
                got
            );
        });

        Self {
            status,
            new_version,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        // WASM: No background thread for update checking
        Self {
            status: Arc::new(RwLock::new(CheckGithubReleaseState::NotChecked)),
            new_version: Arc::new(RwLock::new(None)),
        }
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        let state = match self.status.read() {
            Ok(lock) => lock.clone(),
            Err(_) => {
                ui.label(t!("update.status_unavailable"));
                return;
            }
        };

        match state {
            CheckGithubReleaseState::NotChecked => {
                ui.label(t!("update.not_checked"));
            }
            CheckGithubReleaseState::Checking => {
                ui.label(t!("update.checking"));
            }
            CheckGithubReleaseState::CheckedNewUpdate => {
                if let Ok(lock) = self.new_version.read() {
                    if let Some((label, url)) = &*lock {
                        ui.horizontal(|ui| {
                            ui.label(t!("update.label"));
                            ui.hyperlink_to(egui::RichText::new(label).strong().underline(), url);
                        });
                    } else {
                        ui.label(t!("update.no_info"));
                    }
                } else {
                    ui.label(t!("update.read_fail"));
                }
            }
            CheckGithubReleaseState::Checked => {
                // ui.label(t!("update.checked"));
            }
        }
    }
}
