/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Metal-based egui renderer for iOS/macOS integration

use egui::{Context, RawInput};
use std::sync::Arc;

// Use wgpu from egui_wgpu to ensure version compatibility
type Device = egui_wgpu::wgpu::Device;
type Queue = egui_wgpu::wgpu::Queue;
type Surface = egui_wgpu::wgpu::Surface<'static>;
type SurfaceConfiguration = egui_wgpu::wgpu::SurfaceConfiguration;
#[cfg(feature = "metal_rendering")]
type CommandEncoder = egui_wgpu::wgpu::CommandEncoder;

pub struct Renderer {
    // wgpu components
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Surface,
    config: SurfaceConfiguration,

    // egui components
    context: Context,
    raw_input: RawInput,
    egui_renderer: egui_wgpu::Renderer,

    // Screen info
    width: u32,
    height: u32,
    scale_factor: f32,
}

impl Renderer {
    /// Create a new renderer from a CAMetalLayer pointer
    ///
    /// # Safety
    /// `layer_ptr` must be a valid pointer to a CAMetalLayer
    pub unsafe fn new(
        layer_ptr: *mut std::ffi::c_void,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) -> Self {
        log::info!(
            "Creating Metal renderer: {}x{} @ {}x",
            width,
            height,
            scale_factor
        );

        // Create wgpu instance with Metal backend only
        let instance = egui_wgpu::wgpu::Instance::default();

        // Create surface from CAMetalLayer pointer
        let surface = unsafe {
            instance
                .create_surface_unsafe(egui_wgpu::wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                    layer_ptr,
                ))
                .expect("Failed to create Metal surface")
        };

        // Request high-performance adapter
        let adapter = futures::executor::block_on(instance.request_adapter(
            &egui_wgpu::wgpu::RequestAdapterOptions {
                power_preference: egui_wgpu::wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ))
        .expect("Failed to find suitable GPU adapter");

        log::info!("Using GPU adapter: {:?}", adapter.get_info());

        // Create device and queue
        let (device, queue) =
            futures::executor::block_on(adapter.request_device(&Default::default()))
                .expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        log::info!("Using surface format: {:?}", surface_format);

        let config = SurfaceConfiguration {
            usage: egui_wgpu::wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: egui_wgpu::wgpu::PresentMode::Fifo, // VSync
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Create egui context
        let context = Context::default();

        // Load system fonts if needed
        Self::load_fonts(&context);

        // Set display info
        let mut raw_input = RawInput::default();
        raw_input.viewport_id = egui::ViewportId::ROOT;
        raw_input.viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                native_pixels_per_point: Some(scale_factor),
                ..Default::default()
            },
        );
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width as f32 / scale_factor, height as f32 / scale_factor),
        ));

        // Create egui renderer
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                ..Default::default()
            },
        );

        log::info!("Metal renderer created successfully");

        Self {
            device,
            queue,
            surface,
            config,
            context,
            raw_input,
            egui_renderer,
            width,
            height,
            scale_factor,
        }
    }

    /// Load system fonts for egui
    fn load_fonts(ctx: &Context) {
        let mut fonts = egui::FontDefinitions::default();

        // Try to load SF Pro font on macOS/iOS
        #[cfg(target_os = "macos")]
        {
            if let Ok(font_data) = Self::load_system_font("SF Pro Text") {
                fonts.font_data.insert(
                    "SF Pro".to_owned(),
                    Arc::new(egui::FontData::from_owned(font_data)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "SF Pro".to_owned());
            }
        }

        ctx.set_fonts(fonts);
    }

    /// Load a system font by name (macOS only)
    #[cfg(target_os = "macos")]
    fn load_system_font(name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use font_kit::family_name::FamilyName;
        use font_kit::properties::Properties;
        use font_kit::source::SystemSource;

        let source = SystemSource::new();
        let handle =
            source.select_best_match(&[FamilyName::Title(name.to_string())], &Properties::new())?;

        let font = handle.load()?;
        Ok(font
            .copy_font_data()
            .ok_or("Failed to copy font data")?
            .to_vec())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 && (width != self.width || height != self.height) {
            log::info!("Resizing renderer: {}x{}", width, height);
            self.width = width;
            self.height = height;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            // Update screen rect
            self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(
                    width as f32 / self.scale_factor,
                    height as f32 / self.scale_factor,
                ),
            ));
        }
    }

    /// Process input events and render a frame
    pub fn render(
        &mut self,
        events: Vec<egui::Event>,
        ui_fn: impl FnMut(&egui::Context),
    ) -> Result<(), egui_wgpu::wgpu::SurfaceError> {
        // Add events to input
        self.raw_input.events.extend(events);

        // Run egui
        let full_output = self.context.run(self.raw_input.take(), ui_fn);

        // Get surface texture
        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&egui_wgpu::wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder =
            self.device
                .create_command_encoder(&egui_wgpu::wgpu::CommandEncoderDescriptor {
                    label: Some("egui encoder"),
                });

        // Screen descriptor for egui
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.width, self.height],
            pixels_per_point: self.scale_factor,
        };

        // Update egui textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // Update buffers
        let paint_jobs = self
            .context
            .tessellate(full_output.shapes, self.scale_factor);
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // Render egui - create a scoped block for render pass
        {
            let mut render_pass =
                encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                    label: Some("egui render pass"),
                    color_attachments: &[Some(egui_wgpu::wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: egui_wgpu::wgpu::Operations {
                            load: egui_wgpu::wgpu::LoadOp::Clear(egui_wgpu::wgpu::Color {
                                r: 0.1,
                                g: 0.1,
                                b: 0.1,
                                a: 1.0,
                            }),
                            store: egui_wgpu::wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            // SAFETY: We know that render_pass will not outlive this scope,
            // but egui_renderer.render() requires 'static lifetime.
            // This is safe because we immediately drop render_pass after calling render().
            let render_pass_static: &mut egui_wgpu::wgpu::RenderPass<'static> =
                unsafe { std::mem::transmute(&mut render_pass) };

            self.egui_renderer
                .render(render_pass_static, &paint_jobs, &screen_descriptor);
        } // render_pass dropped here

        // Free textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    /// Get egui context for external access
    pub fn context(&self) -> &Context {
        &self.context
    }
}
