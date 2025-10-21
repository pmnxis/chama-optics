#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2020 Johann Woelper (woelper@gmail.com)
#
# SPDX-License-Identifier: MIT

# cloned and forked from https://github.com/woelper/oculante

rustup target list | grep installed
TOOLCHAIN=$(rustc --version --verbose | grep host | cut -f2 -d":" | tr -d "[:space:]")
echo we are using $TOOLCHAIN
export MACOSX_DEPLOYMENT_TARGET=10.15
cargo install cargo-bundle --quiet
brew install libheif x265 libde265 ffmpeg nasm --quiet

rustup target add aarch64-apple-darwin
# rustup target add x86_64-apple-darwin

cargo bundle --release

echo otool for aarch64:
# otool -L target/aarch64-apple-darwin/release/chama-optics
echo "# Linked shared library"
otool -L target/release/chama-optics
# lipo -create -output target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics target/x86_64-apple-darwin/release/chama-optics target/aarch64-apple-darwin/release/chama-optics 
file target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics
# echo otool for universal binary:
# otool -L target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics

mkdir target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/
chmod +rw target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/*

libs=( /opt/homebrew/opt/x265/lib/libx265.215.dylib /opt/homebrew/opt/libheif/lib/libheif.1.dylib /opt/homebrew/opt/x265/lib/libx265.209.dylib /opt/homebrew/opt/libde265/lib/libde265.0.dylib /opt/homebrew/opt/aom/lib/libaom.3.dylib /opt/homebrew/opt/webp/lib/libsharpyuv.0.dylib /opt/homebrew/opt/libvmaf/lib/libvmaf.3.dylib )

for lib in ${libs[@]}
do
    echo COPY $lib
    # deploy lib
    cp $lib target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/
    install_name_tool -add_rpath "@executable_path/../Frameworks/$(basename $lib)" target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics
    # install_name_tool -change $lib "@executable_path/../Frameworks/$(basename $lib)" target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics

done

install_name_tool -change /opt/homebrew/opt/x265/lib/libx265.209.dylib "@executable_path/../Frameworks/libx265.209.dylib"       target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libheif.1.dylib
install_name_tool -change /opt/homebrew/opt/libde265/lib/libde265.0.dylib "@executable_path/../Frameworks/libde265.0.dylib"     target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libheif.1.dylib
install_name_tool -change /opt/homebrew/opt/libheif/lib/libheif.1.dylib "@executable_path/../Frameworks/libheif.1.dylib"        target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics
install_name_tool -change /opt/homebrew/opt/aom/lib/libaom.3.dylib "@executable_path/../Frameworks/libaom.3.dylib"              target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libheif.1.dylib
install_name_tool -change /opt/homebrew/opt/webp/lib/libsharpyuv.0.dylib "@executable_path/../Frameworks/libsharpyuv.0.dylib"   target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libheif.1.dylib
install_name_tool -change /opt/homebrew/opt/libvmaf/lib/libvmaf.3.dylib "@executable_path/../Frameworks/libvmaf.3.dylib"        target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libaom.3.dylib
install_name_tool -change /opt/homebrew/lib/libx265.215.dylib "@executable_path/../Frameworks/libx265.215.dylib"                target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libheif.1.dylib
install_name_tool -change /opt/homebrew/opt/x265/lib/libx265.215.dylib "@executable_path/../Frameworks/libx265.215.dylib"       target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libheif.1.dylib

for lib in ${libs[@]}
do
    # sign lib
    echo SIGN $lib
    codesign -s "-" -fv target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/$(basename $lib)
done

# echo try this to test the build:
# echo brew uninstall libheif ffmpeg
# brew uninstall libheif ffmpeg

echo "# Check inner binary again"
otool -L target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics
otool -L target/release/bundle/osx/Chama\ Optics.app/Contents/Frameworks/libheif.1.dylib

echo "###########################################################"
echo ""
echo you can test target/release/bundle/osx/Chama\ Optics.app now
echo ""
echo "###########################################################"
