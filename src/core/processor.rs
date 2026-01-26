/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Image processor - headless core for applying themes

use super::CoreImage;
use super::ThemeConfig;

#[cfg(test)]
use super::ThemeType;
use std::path::{Path, PathBuf};

/// Headless image processor
/// This provides the core functionality without any GUI dependencies
pub struct ImageProcessor {
    /// Loaded images
    images: Vec<CoreImage>,

    /// Current theme configuration
    theme_config: ThemeConfig,
}

impl ImageProcessor {
    /// Create a new image processor
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            theme_config: ThemeConfig::default(),
        }
    }

    /// Load an image from path
    pub fn load_image(&mut self, path: PathBuf) -> Result<usize, image::ImageError> {
        let core_image = CoreImage::from_path(path)?;
        self.images.push(core_image);
        Ok(self.images.len() - 1)
    }

    /// Load an image directly without storing in the processor
    /// This is used for FFI functions where we need to apply effects and save immediately
    pub fn load_image_direct(&self, path: &Path) -> Result<image::DynamicImage, image::ImageError> {
        CoreImage::load_image_direct(path)
    }

    /// Save an image directly to a path
    /// This is used for FFI functions where we need to apply effects and save immediately
    pub fn save_image_direct(
        &self,
        dyn_image: &image::DynamicImage,
        path: &Path,
    ) -> Result<(), image::ImageError> {
        dyn_image.save(path)?;
        Ok(())
    }

    /// Get the number of loaded images
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Get a reference to an image by index
    pub fn get_image(&self, index: usize) -> Option<&CoreImage> {
        self.images.get(index)
    }

    /// Get a mutable reference to an image by index
    pub fn get_image_mut(&mut self, index: usize) -> Option<&mut CoreImage> {
        self.images.get_mut(index)
    }

    /// Clear all loaded images
    pub fn clear_images(&mut self) {
        self.images.clear();
    }

    /// Set the theme configuration
    pub fn set_theme(&mut self, theme_config: ThemeConfig) {
        self.theme_config = theme_config;
    }

    /// Get the current theme configuration
    pub fn theme_config(&self) -> &ThemeConfig {
        &self.theme_config
    }

    /// Apply theme to an image and save to file
    pub fn apply_theme_to_image(
        &self,
        image_index: usize,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let core_image = self
            .images
            .get(image_index)
            .ok_or("Image index out of bounds")?;

        log::info!(
            "Applying theme {:?} to {:?} -> {:?}",
            self.theme_config.theme_type,
            core_image.path,
            output_path
        );

        // 1. Load the image with appropriate scaling
        let scale = super::scale_config::ScaleConfig::default();
        let dyn_image = core_image.load_with_scale(scale)?;

        // 2. Apply the theme using the theme renderer
        let themed_image = super::theme_renderer::apply_theme(
            dyn_image,
            &core_image.view_exif,
            self.theme_config.theme_type,
        )?;

        // 3. Save to the output path
        themed_image.save(output_path)?;

        log::info!("Successfully saved themed image to {:?}", output_path);

        Ok(())
    }

    /// Apply a theme trait object directly to an image and save to file
    /// This is used when we have an already-configured theme instance with custom parameters
    #[cfg(feature = "desktop")]
    pub fn apply_theme_direct(
        &self,
        image_index: usize,
        theme: &dyn crate::theme::Theme,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::image::packed_image::PackedImage;
        use uuid::Uuid;

        // Get the CoreImage from processor
        let core_image = self
            .images
            .get(image_index)
            .ok_or("Image index out of bounds")?;

        log::info!(
            "Applying theme {} to {:?} -> {:?}",
            theme.unique_name(),
            core_image.path,
            output_path
        );

        // Create a temporary PackedImage with just the fields needed for apply_to_image
        // Theme will use view_exif from CoreImage
        let packed_image = PackedImage {
            uuid: Uuid::new_v4(),
            path: core_image.path.clone(),
            src_exif: crate::exif_impl::OriginalExif::new(None),
            view_exif: core_image.view_exif.clone(),
            editable: core_image.editable,
            texture: crate::image::common::PackedTexture::Dummy,
            #[cfg(not(feature = "desktop"))]
            image_bytes: None,
            sticker_bytes: None,
            perceptual_hash: None,
        };

        // Use ExportConfig for theme application
        let export_config = crate::export_config::ExportConfig::default();

        // Apply theme to get the themed image
        let mut themed_image = theme.apply_to_image(&packed_image, &export_config)?;

        // Save the themed image
        export_config.save_image(&mut themed_image, None, output_path)?;

        log::info!("Successfully saved themed image to {:?}", output_path);

        Ok(())
    }

    /// Apply theme to all loaded images
    pub fn apply_theme_to_all(
        &self,
        output_dir: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut output_paths = Vec::new();

        for (index, core_image) in self.images.iter().enumerate() {
            let file_name = core_image
                .path
                .file_stem()
                .ok_or("Invalid file name")?
                .to_string_lossy();

            let output_path = output_dir.join(format!("{}_themed.jpg", file_name));

            self.apply_theme_to_image(index, &output_path)?;
            output_paths.push(output_path);
        }

        Ok(output_paths)
    }
}

impl Default for ImageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let processor = ImageProcessor::new();
        assert_eq!(processor.image_count(), 0);
        assert_eq!(processor.theme_config().theme_type, ThemeType::Nothing);
    }

    #[test]
    fn test_theme_config() {
        let mut processor = ImageProcessor::new();
        let mut theme_config = ThemeConfig::new(ThemeType::Film);
        theme_config
            .set_parameter("test_param".to_string(), "test_value")
            .unwrap();

        processor.set_theme(theme_config);

        assert_eq!(processor.theme_config().theme_type, ThemeType::Film);
        assert_eq!(
            processor
                .theme_config()
                .get_parameter::<String>("test_param"),
            Some("test_value".to_string())
        );
    }
}
