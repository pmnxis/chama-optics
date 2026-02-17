#!/bin/sh
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
#
# SPDX-License-Identifier: MIT

# Trunk pre_build hook: stage all font files into web_fonts/
# Trunk's copy-dir captures directory contents before cargo build,
# so all fonts must be ready before compilation starts.

set -e

mkdir -p web_fonts

# Static fonts from assets/fonts/
for f in \
    D2Coding-Ver1.3.2-20180524-all.ttc \
    SourceHanSansVF-remapped.otf \
    Barlow-Variable-Remapped.ttf \
    Barlow-Variable-Remapped-Narrow.ttf
do
    [ ! -f "web_fonts/$f" ] && cp "assets/fonts/$f" "web_fonts/$f" 2>/dev/null || true
done

# Downloaded fonts (skip if already present)
if [ ! -f "web_fonts/digital-7.ttf" ] || [ ! -f "web_fonts/digital-7-italic.ttf" ]; then
    TMP=$(mktemp -d)
    curl -sL "https://dl.dafont.com/dl/?f=digital_7" -o "$TMP/digital_7.zip"
    unzip -qo "$TMP/digital_7.zip" "digital-7.ttf" "digital-7 (italic).ttf" -d "$TMP" 2>/dev/null || true
    [ -f "$TMP/digital-7.ttf" ] && cp "$TMP/digital-7.ttf" web_fonts/
    [ -f "$TMP/digital-7 (italic).ttf" ] && cp "$TMP/digital-7 (italic).ttf" web_fonts/digital-7-italic.ttf
    rm -rf "$TMP"
fi

if [ ! -f "web_fonts/DynaPuff-Variable.ttf" ]; then
    curl -sL "https://github.com/googlefonts/dynapuff/raw/main/fonts/variable/DynaPuff%5Bwdth%2Cwght%5D.ttf" \
        -o web_fonts/DynaPuff-Variable.ttf
fi

echo "web_fonts/ ready: $(ls web_fonts/ | wc -l | tr -d ' ') files"
