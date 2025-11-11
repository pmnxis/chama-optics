#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
#
# SPDX-License-Identifier: CC0-1.0

# run with `. ./build_deps_debian.sh` or `source ./build_deps_debian.sh`

sudo apt-get install pkg-config fontconfig libfontconfig-dev

rm -rf libheif
git clone --branch v1.19.8 https://github.com/strukturag/libheif.git
cd libheif
mkdir build
cd build
cmake --preset=release ..
make
cd ../../

export PKG_CONFIG_PATH=`pwd`/libheif/build
echo "PKG_CONFIG_PATH set to: $PKG_CONFIG_PATH"