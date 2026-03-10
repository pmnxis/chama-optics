/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use reqwest::blocking::Client;
use rust_i18n::t;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    #[allow(dead_code)]
    prerelease: bool,
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

/// Parsed semantic version with optional pre-release tag
#[derive(Debug, Clone, PartialEq, Eq)]
struct SemVer {
    major: u32,
    minor: u32,
    patch: u32,
    /// None = stable release, Some("rc1") = pre-release
    pre: Option<String>,
}

impl SemVer {
    /// Parse "v0.2.0-rc1", "0.2.0", "v1.0.0" etc.
    fn parse(tag: &str) -> Option<Self> {
        let s = tag.strip_prefix('v').unwrap_or(tag).trim();
        let (version_part, pre) = if let Some(idx) = s.find('-') {
            (&s[..idx], Some(s[idx + 1..].to_lowercase()))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        Some(SemVer {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
            pre,
        })
    }

    fn is_prerelease(&self) -> bool {
        self.pre.is_some()
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        let base = self
            .major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch));

        if base != Ordering::Equal {
            return base;
        }

        // Pre-release < stable (e.g. 0.2.0-rc1 < 0.2.0)
        match (&self.pre, &other.pre) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => compare_pre_release(a, b),
        }
    }
}

/// Compare pre-release strings: "rc1" vs "rc2", "alpha" vs "beta" etc.
fn compare_pre_release(a: &str, b: &str) -> std::cmp::Ordering {
    // Extract numeric suffix for same-prefix comparison (rc1 vs rc2)
    fn split_prefix_num(s: &str) -> (&str, Option<u32>) {
        let num_start = s.rfind(|c: char| !c.is_ascii_digit()).map_or(0, |i| i + 1);
        if num_start < s.len() {
            (&s[..num_start], s[num_start..].parse().ok())
        } else {
            (s, None)
        }
    }

    let (a_prefix, a_num) = split_prefix_num(a);
    let (b_prefix, b_num) = split_prefix_num(b);

    a_prefix
        .cmp(b_prefix)
        .then_with(|| a_num.unwrap_or(0).cmp(&b_num.unwrap_or(0)))
}

fn get_latest_release() -> Option<(String, String)> {
    let repo_url = env!("CARGO_PKG_REPOSITORY");
    let current_version = SemVer::parse(env!("CARGO_PKG_VERSION"));
    let current_is_prerelease = current_version
        .as_ref()
        .is_some_and(|v| v.is_prerelease());

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

    let current = current_version.as_ref()?;

    for rel in releases {
        if rel.draft {
            continue;
        }

        let candidate = match SemVer::parse(&rel.tag_name) {
            Some(v) => v,
            None => continue,
        };

        // If current is stable, only show stable releases
        // If current is pre-release, show both stable and pre-release
        if !current_is_prerelease && candidate.is_prerelease() {
            continue;
        }

        // Skip early pre-release stages (alpha/beta/gamma/delta) unless current is also one
        if candidate.is_prerelease() {
            let pre = candidate.pre.as_deref().unwrap_or("");
            let is_early = pre.starts_with("alpha")
                || pre.starts_with("beta")
                || pre.starts_with("gamma")
                || pre.starts_with("delta");
            if is_early {
                let current_pre = current.pre.as_deref().unwrap_or("");
                let current_is_early = current_pre.starts_with("alpha")
                    || current_pre.starts_with("beta")
                    || current_pre.starts_with("gamma")
                    || current_pre.starts_with("delta");
                if !current_is_early {
                    continue;
                }
            }
        }

        if candidate > *current {
            return Some((rel.tag_name, rel.html_url));
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
    pub fn new() -> Self {
        let status: Arc<RwLock<CheckGithubReleaseState>> =
            Arc::new(RwLock::new(CheckGithubReleaseState::Checking));
        let new_version: Arc<RwLock<Option<(String, String)>>> = Arc::new(RwLock::new(None));

        let status_clone = status.clone();
        let new_version_clone = new_version.clone();

        thread::spawn(move || {
            let before_start = web_time::Instant::now();

            log::info!("Let's check release from internet");
            let got = get_latest_release();
            let is_new = got.is_some();
            *new_version_clone.write().unwrap() = got.clone();
            *status_clone.write().unwrap() = if is_new {
                CheckGithubReleaseState::CheckedNewUpdate
            } else {
                CheckGithubReleaseState::Checked
            };

            log::info!(
                "{} msecs passed for getting new version info : {:?}",
                (web_time::Instant::now() - before_start).as_millis(),
                got
            );
        });

        Self {
            status,
            new_version,
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
