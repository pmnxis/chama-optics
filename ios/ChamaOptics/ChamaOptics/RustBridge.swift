// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT

import Foundation

/// Swift bridge to Rust library functions
/// This class provides a Swift-friendly interface to the Rust FFI functions
class RustBridge {
    // MARK: - C Function Declarations

    // Initialize the Chama Optics library
    @_silgen_name("chama_optics_init")
    private static func chama_optics_init()

    // Get library version string
    @_silgen_name("chama_optics_version")
    private static func chama_optics_version() -> UnsafePointer<CChar>

    // Free a string allocated by Rust
    @_silgen_name("chama_optics_free_string")
    private static func chama_optics_free_string(_ ptr: UnsafeMutablePointer<CChar>)

    // Create a new ChamaOptics instance
    @_silgen_name("chama_optics_create")
    private static func chama_optics_create() -> OpaquePointer?

    // Destroy a ChamaOptics instance
    @_silgen_name("chama_optics_destroy")
    private static func chama_optics_destroy(_ handle: OpaquePointer)

    // Load an image from path
    @_silgen_name("chama_optics_load_image")
    private static func chama_optics_load_image(
        _ handle: OpaquePointer,
        _ path: UnsafePointer<CChar>
    ) -> Bool

    // Apply theme to image and save
    @_silgen_name("chama_optics_apply_theme")
    private static func chama_optics_apply_theme(
        _ handle: OpaquePointer,
        _ themeName: UnsafePointer<CChar>,
        _ outputPath: UnsafePointer<CChar>
    ) -> Bool

    // MARK: - Swift Wrapper Methods

    /// Initialize the Rust library
    /// Call this once at app startup
    static func initialize() {
        chama_optics_init()
        print("Chama Optics Rust library initialized")
    }

    /// Get the library version
    static func version() -> String {
        let cString = chama_optics_version()
        let version = String(cString: cString)
        // Don't free this string as it's a static constant from Rust
        return version
    }

    // MARK: - Instance Management

    /// Opaque handle to Rust instance
    private var handle: OpaquePointer?

    init?() {
        guard let handle = RustBridge.chama_optics_create() else {
            return nil
        }
        self.handle = handle
    }

    deinit {
        if let handle = handle {
            RustBridge.chama_optics_destroy(handle)
        }
    }

    /// Load an image from a file path
    func loadImage(path: String) -> Bool {
        guard let handle = handle else { return false }

        return path.withCString { cPath in
            RustBridge.chama_optics_load_image(handle, cPath)
        }
    }

    /// Apply a theme to the loaded image and save
    func applyTheme(themeName: String, outputPath: String) -> Bool {
        guard let handle = handle else { return false }

        return themeName.withCString { cTheme in
            outputPath.withCString { cOutput in
                RustBridge.chama_optics_apply_theme(handle, cTheme, cOutput)
            }
        }
    }
}

// MARK: - Helper Extensions

extension String {
    /// Convert Swift String to C string safely
    func toCString() -> UnsafeMutablePointer<CChar>? {
        return strdup(self)
    }

    /// Free a C string created by toCString()
    static func freeCString(_ ptr: UnsafeMutablePointer<CChar>?) {
        if let ptr = ptr {
            free(ptr)
        }
    }
}
