/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Swift-bridge FFI layer for Metal renderer

// Casts inside the swift_bridge proc-macro expansion are macro-generated artifacts.
#![allow(clippy::unnecessary_cast)]

use super::input_output::{CursorIcon, InputEvent, Modifiers, OutputState};
use super::renderer::Renderer;
use std::ffi::c_void;

#[swift_bridge::bridge]
mod ffi {
    // Expose Rust types to Swift
    extern "Rust" {
        // Main renderer type
        type ChamaEguiRenderer;

        // Create renderer from CAMetalLayer pointer
        #[swift_bridge(associated_to = ChamaEguiRenderer)]
        fn new(layer_ptr: *mut c_void, width: u32, height: u32, scale: f32) -> ChamaEguiRenderer;

        // Resize the renderer
        fn resize(self: &mut ChamaEguiRenderer, width: u32, height: u32);

        // Render a frame with theme UI
        fn render_theme_ui(
            self: &mut ChamaEguiRenderer,
            theme_name: String,
            events: Vec<RenderInputEvent>,
        ) -> RenderOutputState;

        // Input event types
        type RenderInputEvent;

        #[swift_bridge(associated_to = RenderInputEvent)]
        fn from_pointer_moved(x: f32, y: f32) -> RenderInputEvent;

        #[swift_bridge(associated_to = RenderInputEvent)]
        fn from_left_mouse_down(x: f32, y: f32, pressed: bool) -> RenderInputEvent;

        #[swift_bridge(associated_to = RenderInputEvent)]
        fn from_right_mouse_down(x: f32, y: f32, pressed: bool) -> RenderInputEvent;

        #[swift_bridge(associated_to = RenderInputEvent)]
        fn from_mouse_wheel(dx: f32, dy: f32) -> RenderInputEvent;

        #[swift_bridge(associated_to = RenderInputEvent)]
        fn from_key(key: String, pressed: bool, mods: RenderModifiers) -> RenderInputEvent;

        #[swift_bridge(associated_to = RenderInputEvent)]
        fn from_text(text: String) -> RenderInputEvent;

        #[swift_bridge(associated_to = RenderInputEvent)]
        fn from_window_focused(focused: bool) -> RenderInputEvent;

        // Modifier keys
        type RenderModifiers;

        #[swift_bridge(associated_to = RenderModifiers)]
        fn new(shift: bool, ctrl: bool, alt: bool, command: bool) -> RenderModifiers;

        // Output state
        type RenderOutputState;

        fn get_cursor_icon(self: &RenderOutputState) -> RenderCursorIcon;

        // Cursor icon enum
        type RenderCursorIcon;

        fn is_default(self: &RenderCursorIcon) -> bool;
        fn is_pointing_hand(self: &RenderCursorIcon) -> bool;
        fn is_text(self: &RenderCursorIcon) -> bool;
        fn is_grab(self: &RenderCursorIcon) -> bool;
    }
}

// Wrapper types for FFI
pub struct ChamaEguiRenderer {
    renderer: Renderer,
}

impl ChamaEguiRenderer {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(layer_ptr: *mut c_void, width: u32, height: u32, scale: f32) -> Self {
        let renderer = unsafe { Renderer::new(layer_ptr, width, height, scale) };

        Self { renderer }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    pub fn render_theme_ui(
        &mut self,
        theme_name: String,
        events: Vec<RenderInputEvent>,
    ) -> RenderOutputState {
        // Convert input events
        let egui_events: Vec<egui::Event> = events.into_iter().map(|e| e.0.into()).collect();

        // Render with theme UI
        let cursor_icon = self.renderer.context().output(|o| o.cursor_icon);

        if let Err(e) = self.renderer.render(egui_events, |ctx| {
            // Show simple test UI
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Chama Optics - Metal Renderer Test");
                ui.separator();

                ui.label(format!("Theme: {}", theme_name));
                ui.label("egui Metal rendering is working!");

                ui.separator();
                ui.label("This is a test of the Metal rendering backend.");

                if ui.button("Test Button").clicked() {
                    log::info!("Button clicked!");
                }
            });
        }) {
            log::error!("Render error: {:?}", e);
        }

        RenderOutputState(OutputState {
            cursor_icon: cursor_icon.into(),
        })
    }
}

// Input event wrapper
pub struct RenderInputEvent(InputEvent);

impl RenderInputEvent {
    pub fn from_pointer_moved(x: f32, y: f32) -> Self {
        Self(InputEvent::PointerMoved(x, y))
    }

    pub fn from_left_mouse_down(x: f32, y: f32, pressed: bool) -> Self {
        Self(InputEvent::LeftMouseDown(x, y, pressed))
    }

    pub fn from_right_mouse_down(x: f32, y: f32, pressed: bool) -> Self {
        Self(InputEvent::RightMouseDown(x, y, pressed))
    }

    pub fn from_mouse_wheel(dx: f32, dy: f32) -> Self {
        Self(InputEvent::MouseWheel(dx, dy))
    }

    pub fn from_key(key: String, pressed: bool, mods: RenderModifiers) -> Self {
        Self(InputEvent::Key {
            key,
            pressed,
            modifiers: mods.0,
        })
    }

    pub fn from_text(text: String) -> Self {
        Self(InputEvent::Text(text))
    }

    pub fn from_window_focused(focused: bool) -> Self {
        Self(InputEvent::WindowFocused(focused))
    }
}

// Modifiers wrapper
pub struct RenderModifiers(Modifiers);

impl RenderModifiers {
    pub fn new(shift: bool, ctrl: bool, alt: bool, command: bool) -> Self {
        Self(Modifiers {
            shift,
            ctrl,
            alt,
            command,
        })
    }
}

// Output state wrapper
pub struct RenderOutputState(OutputState);

impl RenderOutputState {
    pub fn get_cursor_icon(&self) -> RenderCursorIcon {
        RenderCursorIcon(self.0.cursor_icon)
    }
}

// Cursor icon wrapper
pub struct RenderCursorIcon(CursorIcon);

impl RenderCursorIcon {
    pub fn is_default(&self) -> bool {
        matches!(self.0, CursorIcon::Default)
    }

    pub fn is_pointing_hand(&self) -> bool {
        matches!(self.0, CursorIcon::PointingHand)
    }

    pub fn is_text(&self) -> bool {
        matches!(self.0, CursorIcon::Text)
    }

    pub fn is_grab(&self) -> bool {
        matches!(self.0, CursorIcon::Grab | CursorIcon::Grabbing)
    }
}
