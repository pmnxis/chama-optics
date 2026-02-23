// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Built-in default stickers and LUT presets bundled with the application.
//!
//! Built-in items behave differently from user-added items:
//! - They cannot be permanently deleted — "deleting" hides them instead
//! - They can be restored via the "Restore Defaults" feature
//! - They are identified by fixed, deterministic UUIDs

pub mod lut_presets;
pub mod sticker_presets;

use uuid::{Uuid, uuid};

// ─── Built-in LUT UUIDs (prefix 00000000-0000-0000-0003-xxxxxxxxxxxx) ─────────

pub const BUILTIN_LUT_WARM_SUNRISE_ID: Uuid = uuid!("00000000-0000-0000-0003-000000000001");
pub const BUILTIN_LUT_COOL_DUSK_ID: Uuid = uuid!("00000000-0000-0000-0003-000000000002");
pub const BUILTIN_LUT_CINEMATIC_BW_ID: Uuid = uuid!("00000000-0000-0000-0003-000000000003");
pub const BUILTIN_LUT_VIVID_ID: Uuid = uuid!("00000000-0000-0000-0003-000000000004");

pub const ALL_BUILTIN_LUT_IDS: &[Uuid] = &[
    BUILTIN_LUT_WARM_SUNRISE_ID,
    BUILTIN_LUT_COOL_DUSK_ID,
    BUILTIN_LUT_CINEMATIC_BW_ID,
    BUILTIN_LUT_VIVID_ID,
];

// ─── Built-in sticker UUIDs (prefix 00000000-0000-0000-0001-xxxxxxxxxxxx) ─────

pub const BUILTIN_STICKER_HAATON_ID: Uuid = uuid!("00000000-0000-0000-0001-000000000001");

pub const ALL_BUILTIN_STICKER_IDS: &[Uuid] = &[BUILTIN_STICKER_HAATON_ID];

// ─── Built-in character sticker UUIDs (prefix 00000000-0000-0000-0002-xxxxxxxxx) ─

pub const BUILTIN_CHARACTER_HAATO_ID: Uuid = uuid!("00000000-0000-0000-0002-000000000001");

pub const ALL_BUILTIN_CHARACTER_IDS: &[Uuid] = &[BUILTIN_CHARACTER_HAATO_ID];

/// Returns true if the given UUID belongs to any built-in item (sticker or LUT)
pub fn is_builtin_id(id: Uuid) -> bool {
    ALL_BUILTIN_LUT_IDS.contains(&id)
        || ALL_BUILTIN_STICKER_IDS.contains(&id)
        || ALL_BUILTIN_CHARACTER_IDS.contains(&id)
}
