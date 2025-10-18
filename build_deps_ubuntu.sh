#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
#
# SPDX-License-Identifier: CC0-1.0

sudo apt-get install pkg-config
sudo apt-get install -y software-properties-common
sudo add-apt-repository -y ppa:strukturag/libheif
sudo apt-get update
sudo apt-get install -y libheif-dev libde265-dev libx265-dev libjpeg-dev libpng-dev libaom-dev openmpi-bin libheif
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH