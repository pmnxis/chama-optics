// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT

//
//  Use this file to import your target's public headers that you would like to expose to Swift.
//

#ifndef ChamaOptics_Bridging_Header_h
#define ChamaOptics_Bridging_Header_h

// Rust library FFI functions
// These are declared in src/ffi.rs

#include <stdbool.h>

// Initialize the Chama Optics library
void chama_optics_init(void);

// Get library version string
const char* chama_optics_version(void);

// Free a string allocated by Rust
void chama_optics_free_string(char* ptr);

// Opaque pointer to ChamaOptics instance
typedef struct ChamaOpticsHandle ChamaOpticsHandle;

// Create a new ChamaOptics instance
ChamaOpticsHandle* chama_optics_create(void);

// Destroy a ChamaOptics instance
void chama_optics_destroy(ChamaOpticsHandle* handle);

// Load an image from path
bool chama_optics_load_image(ChamaOpticsHandle* handle, const char* path);

// Apply theme to image and save
bool chama_optics_apply_theme(
    ChamaOpticsHandle* handle,
    const char* theme_name,
    const char* output_path
);

#endif /* ChamaOptics_Bridging_Header_h */
