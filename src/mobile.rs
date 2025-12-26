/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Mobile UI optimizations for iOS and touch devices

/// Mobile UI configuration constants
pub struct MobileConfig {
    /// Minimum touch target size (Apple HIG recommends 44x44 points)
    pub min_touch_size: f32,
    /// Tab bar height for mobile
    pub tab_bar_height: f32,
    /// Gallery thumbnail size for mobile
    pub gallery_thumbnail_size: f32,
    /// Spacing between elements on mobile
    pub mobile_spacing: f32,
    /// Font scale multiplier for better readability on mobile
    pub font_scale: f32,
}

impl Default for MobileConfig {
    fn default() -> Self {
        Self {
            min_touch_size: 44.0,          // Apple HIG standard
            tab_bar_height: 60.0,          // Larger for touch
            gallery_thumbnail_size: 100.0, // Larger for easier selection
            mobile_spacing: 12.0,          // More generous spacing
            font_scale: 1.2,               // Slightly larger fonts
        }
    }
}

impl MobileConfig {
    /// Check if we're running on a mobile device
    pub fn is_mobile() -> bool {
        cfg!(target_os = "ios") || cfg!(target_os = "android")
    }

    /// Check if mobile UI features are enabled
    pub fn is_mobile_ui_enabled() -> bool {
        cfg!(feature = "mobile_ui") || Self::is_mobile()
    }

    /// Get the appropriate config for current platform
    pub fn get() -> Self {
        if Self::is_mobile_ui_enabled() {
            Self::default()
        } else {
            Self {
                min_touch_size: 24.0,
                tab_bar_height: 50.0,
                gallery_thumbnail_size: 80.0,
                mobile_spacing: 5.0,
                font_scale: 1.0,
            }
        }
    }
}

/// Platform-specific file picker
#[cfg(target_os = "ios")]
pub mod file_picker {
    use std::path::PathBuf;

    /// Open iOS Photos library picker (will be implemented via Swift FFI)
    pub fn pick_images() -> Vec<PathBuf> {
        // TODO: Implement via Swift bridge
        // This will call into Swift code that uses PHPickerViewController
        log::warn!("iOS file picker not yet implemented - using placeholder");
        Vec::new()
    }
}

#[cfg(all(not(target_os = "ios"), feature = "desktop"))]
pub mod file_picker {
    use std::path::PathBuf;

    /// Fallback to rfd file picker on non-iOS platforms
    pub fn pick_images() -> Vec<PathBuf> {
        rfd::FileDialog::new().pick_files().unwrap_or_default()
    }
}

#[cfg(all(not(target_os = "ios"), not(feature = "desktop")))]
pub mod file_picker {
    use std::path::PathBuf;

    /// Placeholder file picker for non-iOS, non-desktop builds
    pub fn pick_images() -> Vec<PathBuf> {
        Vec::new()
    }
}

/// Touch gesture helpers
pub mod gestures {
    /// Detect pinch-to-zoom gesture (for future implementation)
    pub struct PinchZoom {
        pub scale: f32,
        pub active: bool,
    }

    impl Default for PinchZoom {
        fn default() -> Self {
            Self {
                scale: 1.0,
                active: false,
            }
        }
    }

    /// Detect swipe gesture (for future implementation)
    pub enum SwipeDirection {
        Left,
        Right,
        Up,
        Down,
    }
}

/// Layout helpers for mobile
pub mod layout {
    use super::MobileConfig;

    /// Calculate optimal number of columns for grid layout based on screen width
    pub fn grid_columns(screen_width: f32) -> usize {
        let config = MobileConfig::get();
        let min_column_width = config.gallery_thumbnail_size + config.mobile_spacing * 2.0;
        (screen_width / min_column_width).floor().max(2.0) as usize
    }

    /// Check if screen is in portrait orientation
    pub fn is_portrait(width: f32, height: f32) -> bool {
        height > width
    }

    /// Get appropriate sidebar width for current orientation
    pub fn sidebar_width(is_portrait: bool) -> f32 {
        if is_portrait {
            0.0 // Hide sidebar in portrait, use bottom tabs instead
        } else {
            MobileConfig::get().tab_bar_height
        }
    }
}
