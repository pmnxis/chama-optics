/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Input and output types for Metal renderer FFI

/// Input event from Swift (mouse, keyboard, etc.)
#[derive(Debug, Clone)]
pub enum InputEvent {
    PointerMoved(f32, f32),
    LeftMouseDown(f32, f32, bool), // x, y, pressed
    RightMouseDown(f32, f32, bool),
    MiddleMouseDown(f32, f32, bool),
    MouseWheel(f32, f32), // delta_x, delta_y
    Key {
        key: String,
        pressed: bool,
        modifiers: Modifiers,
    },
    Text(String),
    WindowFocused(bool),
    WindowResized(u32, u32),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub command: bool,
}

impl From<InputEvent> for egui::Event {
    fn from(event: InputEvent) -> Self {
        match event {
            InputEvent::PointerMoved(x, y) => egui::Event::PointerMoved(egui::pos2(x, y)),
            InputEvent::LeftMouseDown(x, y, pressed) => egui::Event::PointerButton {
                pos: egui::pos2(x, y),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
            InputEvent::RightMouseDown(x, y, pressed) => egui::Event::PointerButton {
                pos: egui::pos2(x, y),
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Default::default(),
            },
            InputEvent::MiddleMouseDown(x, y, pressed) => egui::Event::PointerButton {
                pos: egui::pos2(x, y),
                button: egui::PointerButton::Middle,
                pressed,
                modifiers: Default::default(),
            },
            InputEvent::MouseWheel(dx, dy) => egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(dx, dy),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
            },
            InputEvent::Key {
                key,
                pressed,
                modifiers: mods,
            } => {
                let modifiers = egui::Modifiers {
                    shift: mods.shift,
                    ctrl: mods.ctrl,
                    alt: mods.alt,
                    command: mods.command,
                    ..Default::default()
                };

                // Try to parse as egui Key
                if let Some(egui_key) = parse_key(&key) {
                    egui::Event::Key {
                        key: egui_key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers,
                    }
                } else {
                    // Return a dummy event if key is not recognized
                    egui::Event::Text(String::new())
                }
            }
            InputEvent::Text(text) => egui::Event::Text(text),
            InputEvent::WindowFocused(focused) => egui::Event::WindowFocused(focused),
            InputEvent::WindowResized(_, _) => {
                // Window resize is handled separately in the renderer
                egui::Event::Text(String::new()) // Dummy event
            }
        }
    }
}

fn parse_key(key_str: &str) -> Option<egui::Key> {
    use egui::Key;

    match key_str {
        "ArrowDown" => Some(Key::ArrowDown),
        "ArrowLeft" => Some(Key::ArrowLeft),
        "ArrowRight" => Some(Key::ArrowRight),
        "ArrowUp" => Some(Key::ArrowUp),
        "Escape" => Some(Key::Escape),
        "Tab" => Some(Key::Tab),
        "Backspace" => Some(Key::Backspace),
        "Enter" => Some(Key::Enter),
        "Space" => Some(Key::Space),
        "Insert" => Some(Key::Insert),
        "Delete" => Some(Key::Delete),
        "Home" => Some(Key::Home),
        "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        "a" | "A" => Some(Key::A),
        "b" | "B" => Some(Key::B),
        "c" | "C" => Some(Key::C),
        "d" | "D" => Some(Key::D),
        "e" | "E" => Some(Key::E),
        "f" | "F" => Some(Key::F),
        "g" | "G" => Some(Key::G),
        "h" | "H" => Some(Key::H),
        "i" | "I" => Some(Key::I),
        "j" | "J" => Some(Key::J),
        "k" | "K" => Some(Key::K),
        "l" | "L" => Some(Key::L),
        "m" | "M" => Some(Key::M),
        "n" | "N" => Some(Key::N),
        "o" | "O" => Some(Key::O),
        "p" | "P" => Some(Key::P),
        "q" | "Q" => Some(Key::Q),
        "r" | "R" => Some(Key::R),
        "s" | "S" => Some(Key::S),
        "t" | "T" => Some(Key::T),
        "u" | "U" => Some(Key::U),
        "v" | "V" => Some(Key::V),
        "w" | "W" => Some(Key::W),
        "x" | "X" => Some(Key::X),
        "y" | "Y" => Some(Key::Y),
        "z" | "Z" => Some(Key::Z),
        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),
        _ => None,
    }
}

/// Output state from renderer (cursor icon, etc.)
#[derive(Debug, Clone)]
pub struct OutputState {
    pub cursor_icon: CursorIcon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorIcon {
    Default,
    PointingHand,
    ResizeHorizontal,
    ResizeVertical,
    ResizeNeSw,
    ResizeNwSe,
    Text,
    Crosshair,
    NotAllowed,
    Grab,
    Grabbing,
}

impl From<egui::CursorIcon> for CursorIcon {
    fn from(icon: egui::CursorIcon) -> Self {
        match icon {
            egui::CursorIcon::Default => CursorIcon::Default,
            egui::CursorIcon::PointingHand => CursorIcon::PointingHand,
            egui::CursorIcon::ResizeHorizontal => CursorIcon::ResizeHorizontal,
            egui::CursorIcon::ResizeVertical => CursorIcon::ResizeVertical,
            egui::CursorIcon::ResizeNeSw => CursorIcon::ResizeNeSw,
            egui::CursorIcon::ResizeNwSe => CursorIcon::ResizeNwSe,
            egui::CursorIcon::Text => CursorIcon::Text,
            egui::CursorIcon::Crosshair => CursorIcon::Crosshair,
            egui::CursorIcon::NotAllowed => CursorIcon::NotAllowed,
            egui::CursorIcon::Grab => CursorIcon::Grab,
            egui::CursorIcon::Grabbing => CursorIcon::Grabbing,
            _ => CursorIcon::Default,
        }
    }
}
