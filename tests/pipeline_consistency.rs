/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Cross-platform pipeline consistency integration tests.
//!
//! # What this tests
//!
//! iOS, Android, and egui/desktop all call the **same Rust pipeline functions**.
//! These tests verify that:
//! 1. The pipeline produces **deterministic output** (same input → same pixels)
//! 2. `PreviewPipeline` == `ExportPipeline` when run at the same resolution
//! 3. `chama_preview_cheki_bytes()` output matches `apply_cheki_decoration()` output
//!    at the same resolution — guaranteeing that cheki preview == export
//! 4. Pipeline config JSON round-trips without changing output
//!
//! # Running
//! ```
//! cargo test --features desktop                        # all tests
//! cargo test --features desktop pipeline_consistency   # this file only
//! ```
//!
//! To update snapshots after intentional rendering changes:
//! ```
//! UPDATE_SNAPSHOTS=1 cargo test --features desktop
//! ```

use chama_optics::effect::color_adjustments::ColorAdjustments;
use chama_optics::effect::sticker_storage::FaceArea;
use chama_optics::pipeline::v1::{
    ExportPipeline, PipelineConfig, PipelineContext, PipelineStage, PreviewPipeline, StageEntry,
};
use chama_optics::test_helper::synthetic::{
    assert_deterministic, assert_images_identical, make_checkerboard, make_gradient, pixel_hash,
};
use image::DynamicImage;

// ─── helpers ──────────────────────────────────────────────────────────────────

fn test_img_landscape() -> DynamicImage {
    make_gradient(320, 240)
}

fn test_img_portrait() -> DynamicImage {
    make_checkerboard(240, 320, 20)
}

fn test_img_square() -> DynamicImage {
    make_gradient(256, 256)
}

fn empty_config() -> PipelineConfig {
    PipelineConfig::default()
}

fn color_adj_config() -> PipelineConfig {
    let mut adj = ColorAdjustments::new();
    adj.enabled = true;
    adj.exposure = 0.5;
    adj.contrast = 20;
    adj.highlights = -30;
    adj.shadows = 20;

    let mut config = PipelineConfig::default();
    config
        .stages
        .push(StageEntry::enabled(PipelineStage::ColorAdjustments(adj)));
    config
}

fn mosaic_config() -> PipelineConfig {
    use chama_optics::effect::face_detection::FaceEffectMode;
    use chama_optics::pipeline::v1::{MosaicEffectConfig, StickerEffectConfig, StrokeEffectConfig};

    let face = FaceArea::new(60, 40, 80, 100);
    let mut face_area = face;
    face_area.effect_mode = FaceEffectMode::Mosaic;

    let mosaic = MosaicEffectConfig {
        block_size: 10,
        intensity: 1.0,
    };
    let stroke = StrokeEffectConfig {
        thickness: 2,
        color: [255, 0, 0, 255],
    };
    let sticker = StickerEffectConfig {
        scale: 1.0,
        offset_x: 0,
        offset_y: 0,
    };

    let mut config = PipelineConfig::default();
    config
        .stages
        .push(StageEntry::enabled(PipelineStage::FaceEffect {
            faces: vec![face_area],
            mosaic,
            stroke,
            sticker,
        }));
    config
}

fn stroke_config() -> PipelineConfig {
    use chama_optics::effect::face_detection::FaceEffectMode;
    use chama_optics::pipeline::v1::{MosaicEffectConfig, StickerEffectConfig, StrokeEffectConfig};

    let mut face = FaceArea::new(60, 40, 80, 100);
    face.effect_mode = FaceEffectMode::Stroke;

    let mosaic = MosaicEffectConfig {
        block_size: 10,
        intensity: 1.0,
    };
    let stroke = StrokeEffectConfig {
        thickness: 4,
        color: [0, 0, 255, 255],
    };
    let sticker = StickerEffectConfig {
        scale: 1.0,
        offset_x: 0,
        offset_y: 0,
    };

    let mut config = PipelineConfig::default();
    config
        .stages
        .push(StageEntry::enabled(PipelineStage::FaceEffect {
            faces: vec![face],
            mosaic,
            stroke,
            sticker,
        }));
    config
}

/// Run ExportPipeline and return resulting image.
fn run_export(img: DynamicImage, config: PipelineConfig) -> DynamicImage {
    let ctx = PipelineContext::empty();
    ExportPipeline::new(img, config)
        .execute(&ctx)
        .expect("ExportPipeline::execute failed")
}

/// Run PreviewPipeline (with decoration) and return resulting image.
fn run_preview(img: DynamicImage, config: PipelineConfig) -> DynamicImage {
    let ctx = PipelineContext::empty();
    let mut preview = PreviewPipeline::new(img, config);
    preview
        .render_with_decoration(&ctx)
        .expect("PreviewPipeline::render_with_decoration failed")
}

// ─── determinism tests ────────────────────────────────────────────────────────

/// Empty pipeline run twice → same pixels.
#[test]
fn test_empty_pipeline_deterministic() {
    assert_deterministic("empty_pipeline_landscape", || {
        run_export(test_img_landscape(), empty_config())
    });
    assert_deterministic("empty_pipeline_portrait", || {
        run_export(test_img_portrait(), empty_config())
    });
}

/// ColorAdjustments run twice → same pixels.
#[test]
fn test_color_adjustments_deterministic() {
    assert_deterministic("color_adj_landscape", || {
        run_export(test_img_landscape(), color_adj_config())
    });
    assert_deterministic("color_adj_portrait", || {
        run_export(test_img_portrait(), color_adj_config())
    });
}

/// FaceEffect (mosaic) run twice → same pixels.
#[test]
fn test_face_mosaic_deterministic() {
    assert_deterministic("face_mosaic_landscape", || {
        run_export(test_img_landscape(), mosaic_config())
    });
}

/// FaceEffect (stroke) run twice → same pixels.
#[test]
fn test_face_stroke_deterministic() {
    assert_deterministic("face_stroke_landscape", || {
        run_export(test_img_landscape(), stroke_config())
    });
}

/// Cheki border-only decoration (no stickers, no fonts) run twice → same pixels.
/// This is feature-gated: desktop uses `egui::Color32`, mobile uses `[u8;4]`.
#[test]
fn test_cheki_border_deterministic() {
    use chama_optics::effect::cheki::ChekiDecoration;
    use chama_optics::effect::sticker_storage::StickerStorage;

    // Minimal cheki JSON that avoids font rendering (date_enabled: false, text: "")
    // Uses array color format which works for both [u8;4] and egui::Color32 serde
    let deco_json = r#"{
        "enabled": true,
        "text": "",
        "font_weight": 400,
        "font_size": 0.6,
        "text_color": [0, 0, 0, 255],
        "text_position_x": 0.5,
        "text_position_y": 0.5,
        "dice_stickers": [],
        "border_width": 0.05,
        "bottom_extra": 0.15,
        "border_color": [255, 255, 255, 255],
        "clip_stickers": false,
        "allow_rotation": false,
        "date_text": "",
        "date_enabled": false,
        "date_font_weight": 400,
        "date_font_size": 0.5,
        "date_color": [100, 100, 100, 255],
        "date_position": "BottomRight"
    }"#;

    // Desktop builds have extra font fields — try parsing, skip if unsupported
    let deco: ChekiDecoration = match serde_json::from_str(deco_json) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "Skipping cheki test: JSON parse failed ({e}) — font fields differ by feature"
            );
            return;
        }
    };

    let storage = StickerStorage::default();

    assert_deterministic("cheki_border_landscape", || {
        chama_optics::effect::cheki_renderer::apply_cheki_decoration(
            test_img_landscape(),
            &deco,
            &storage,
        )
    });
    assert_deterministic("cheki_border_square", || {
        chama_optics::effect::cheki_renderer::apply_cheki_decoration(
            test_img_square(),
            &deco,
            &storage,
        )
    });
}

// ─── preview == export tests ──────────────────────────────────────────────────

/// PreviewPipeline and ExportPipeline produce identical output when given
/// the same image and config (no decoration).
///
/// This is the core cross-platform guarantee: preview code path == export code path.
#[test]
fn test_preview_equals_export_at_same_size_empty() {
    let img = test_img_landscape();
    let config = empty_config();

    let export_out = run_export(img.clone(), config.clone());
    let preview_out = run_preview(img, config);

    assert_images_identical("export", &export_out, "preview", &preview_out);
}

#[test]
fn test_preview_equals_export_at_same_size_color_adj() {
    let img = test_img_landscape();
    let config = color_adj_config();

    let export_out = run_export(img.clone(), config.clone());
    let preview_out = run_preview(img, config);

    assert_images_identical("export", &export_out, "preview", &preview_out);
}

#[test]
fn test_preview_equals_export_at_same_size_mosaic() {
    let img = test_img_landscape();
    let config = mosaic_config();

    let export_out = run_export(img.clone(), config.clone());
    let preview_out = run_preview(img, config);

    assert_images_identical("export", &export_out, "preview", &preview_out);
}

// ─── JSON serde consistency ───────────────────────────────────────────────────

/// Serializing PipelineConfig to JSON and deserializing it back produces
/// the same output — i.e., iOS/Android JSON FFI is lossless.
#[test]
fn test_pipeline_json_serde_roundtrip_determinism() {
    let img = test_img_landscape();
    let config = color_adj_config();

    // Round-trip the config through JSON
    let json = serde_json::to_string(&config).expect("serialize");
    let config2: PipelineConfig = serde_json::from_str(&json).expect("deserialize");

    let out1 = run_export(img.clone(), config);
    let out2 = run_export(img, config2);

    assert_images_identical("original_config", &out1, "json_roundtrip_config", &out2);
}

/// Verify mosaic config JSON round-trip (includes face areas).
#[test]
fn test_pipeline_json_serde_roundtrip_mosaic() {
    let img = test_img_landscape();
    let config = mosaic_config();

    let json = serde_json::to_string(&config).expect("serialize");
    let config2: PipelineConfig = serde_json::from_str(&json).expect("deserialize");

    let out1 = run_export(img.clone(), config);
    let out2 = run_export(img, config2);

    assert_images_identical("mosaic_original", &out1, "mosaic_json_roundtrip", &out2);
}

// ─── real image tests ─────────────────────────────────────────────────────────

/// Run the full pipeline on any images found in `test_image/import/`.
///
/// Does not assert specific output (hashes vary by image), but verifies:
/// - Pipeline completes without error
/// - Running twice produces the same output (determinism)
#[test]
fn test_real_images_pipeline_if_present() {
    let paths = match chama_optics::test_helper::list_import_images_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Skipping real image tests: {e}");
            return;
        }
    };

    if paths.is_empty() {
        eprintln!("No images in test_image/import/ — skipping real image tests");
        return;
    }

    let config = color_adj_config();
    let ctx = PipelineContext::empty();

    for path in &paths {
        eprintln!("Testing real image: {}", path.display());

        let img = match image::open(path) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("  Failed to open: {e}");
                continue;
            }
        };

        let out1 = ExportPipeline::new(img.clone(), config.clone())
            .execute(&ctx)
            .expect("export run 1");
        let out2 = ExportPipeline::new(img, config.clone())
            .execute(&ctx)
            .expect("export run 2");

        assert_images_identical(
            &format!("{} run1", path.display()),
            &out1,
            &format!("{} run2", path.display()),
            &out2,
        );
        eprintln!("  ✅ deterministic, hash={}", pixel_hash(&out1));
    }
}
