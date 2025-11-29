/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

#[macro_export]
macro_rules! dump {
    ($arr:expr) => {{
        let name = stringify!($arr);
        $crate::dump::dump_bytes(name, $arr.as_ref());
    }};
}

#[macro_export]
macro_rules! dump_or {
    ($arr:expr) => {{
        $arr.map(|inside| {
            let name = stringify!($arr);
            $crate::dump::dump_bytes(name, inside.as_ref());
            inside
        })
    }};
}

pub fn dump_bytes(name: &str, bytes: &[u8]) {
    let len = bytes.len();
    let len_hex = format!("0x{len:X}");
    let header = format!("{name} [len={len} ({len_hex})]");

    let show = 64usize;

    if len > show * 2 {
        let head = &bytes[..show];
        let tail = &bytes[len - show..];

        let s1 = head
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let s2 = tail
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        let offset_width = len.to_string().len();
        let head_label = format!("[{:<1$}]      ", format!("0..{}", show), offset_width);
        let tail_label = format!(
            "[{start:<width$}..{end}] ",
            start = len - show,
            end = len,
            width = offset_width
        );

        log::debug!("======= {header} =======");
        log::debug!("{head_label}{s1}");
        log::debug!("{tail_label}{s2}");
        log::debug!("======================");
    } else {
        let s = bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        log::debug!("======= {header} =======");
        log::debug!("{s}");
        log::debug!("======================");
    }
}
