// swift-tools-version: 5.9
// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT

import PackageDescription

let package = Package(
    name: "ChamaOptics",
    platforms: [
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "ChamaOptics",
            targets: ["ChamaOptics"]),
    ],
    targets: [
        .target(
            name: "ChamaOptics",
            dependencies: ["ChamaOpticsRust"],
            path: "ChamaOptics",
            exclude: ["libs"],
            sources: [
                "ChamaOpticsApp.swift",
                "ContentView.swift",
                "RustBridge.swift"
            ],
            publicHeadersPath: "."
        ),
        .binaryTarget(
            name: "ChamaOpticsRust",
            path: "ChamaOptics/libs/ChamaOpticsRust.xcframework"
        )
    ]
)
