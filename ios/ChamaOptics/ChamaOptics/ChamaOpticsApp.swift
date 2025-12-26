// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT

import SwiftUI

@main
struct ChamaOpticsApp: App {
    init() {
        // Initialize Rust library on app startup
        RustBridge.initialize()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
