// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Built-in sticker presets bundled with the application.

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use crate::effect::sticker_storage::StickerItem;
use crate::effect::sticker_storage::StickerStorage;

// Desktop only: all built-in sticker definitions and embedded assets.
// On iOS/Android the files are bundled as separate resources and copied to
// the app's documents directory by native init code; the Rust binary does not
// embed or write them.
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
mod desktop {
    use crate::builtins::{BUILTIN_CHARACTER_HAATO_ID, BUILTIN_STICKER_HAATON_ID};
    use rust_i18n::t;
    use uuid::Uuid;

    static HAATO_STICKER_BYTES: &[u8] = include_bytes!("assets/haato_sticker.webp");
    static HAATON_STICKER_BYTES: &[u8] = include_bytes!("assets/haaton_sticker.png");

    pub struct BuiltinStickerDef {
        pub id: Uuid,
        pub name: String,
        pub filename: &'static str,
        pub is_character: bool,
        pub bytes: &'static [u8],
    }

    pub fn builtin_sticker_defs() -> Vec<BuiltinStickerDef> {
        vec![
            BuiltinStickerDef {
                id: BUILTIN_CHARACTER_HAATO_ID,
                name: t!("sticker.builtin_haato_name").to_string(),
                filename: "builtin_haato.webp",
                is_character: true,
                bytes: HAATO_STICKER_BYTES,
            },
            BuiltinStickerDef {
                id: BUILTIN_STICKER_HAATON_ID,
                name: t!("sticker.builtin_haaton_name").to_string(),
                filename: "builtin_haaton.png",
                is_character: false,
                bytes: HAATON_STICKER_BYTES,
            },
        ]
    }
}

/// Initialize built-in stickers in the given storage.
///
/// For each built-in sticker:
/// - If not present in storage, write the file and register it.
/// - If already present, leave it alone.
///
/// Returns the number of new built-in stickers added.
pub fn init_builtin_stickers(storage: &mut StickerStorage) -> usize {
    if let Err(e) = storage.ensure_directory() {
        log::error!("Failed to create sticker directory: {}", e);
        return 0;
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    {
        let mut added = 0;

        for def in desktop::builtin_sticker_defs() {
            if storage.stickers.iter().any(|s| s.id == def.id) {
                continue;
            }

            let file_path = storage.storage_directory.join(def.filename);

            if let Err(e) = std::fs::write(&file_path, def.bytes) {
                log::error!("Failed to write built-in sticker {}: {}", def.name, e);
                continue;
            }

            let mut item = StickerItem::new(def.name, file_path);
            item.id = def.id;
            item.is_builtin = true;
            item.is_hidden = false;
            item.is_character = def.is_character;

            log::info!("Initialized built-in sticker: {}", def.filename);
            storage.stickers.push(item);
            added += 1;
        }

        return added;
    }

    #[allow(unreachable_code)]
    0
}
