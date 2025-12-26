// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT

import SwiftUI
import PhotosUI

struct ContentView: View {
    @State private var rustVersion: String = "Loading..."
    @State private var selectedItem: PhotosPickerItem?
    @State private var selectedImage: UIImage?
    @State private var selectedImageData: Data? // Preserve original data with EXIF
    @State private var processedImage: UIImage?
    @State private var statusMessage: String = "Ready"
    @State private var processor: RustBridge?
    @State private var showPhotoPicker = false

    var body: some View {
        NavigationView {
            VStack(spacing: 16) {
                // Header
                VStack(spacing: 4) {
                    Text("Chama Optics")
                        .font(.title)
                        .fontWeight(.bold)
                    Text("Rust v\(rustVersion)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.top)

                // Status
                Text(statusMessage)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding(.horizontal)

                // Image Display
                ScrollView {
                    VStack(spacing: 16) {
                        if let image = selectedImage {
                            VStack(alignment: .leading, spacing: 8) {
                                Text("Original")
                                    .font(.headline)
                                Image(uiImage: image)
                                    .resizable()
                                    .scaledToFit()
                                    .frame(maxHeight: 200)
                                    .cornerRadius(8)
                            }
                        }

                        if let image = processedImage {
                            VStack(alignment: .leading, spacing: 8) {
                                Text("Processed")
                                    .font(.headline)
                                Image(uiImage: image)
                                    .resizable()
                                    .scaledToFit()
                                    .frame(maxHeight: 200)
                                    .cornerRadius(8)
                            }
                        }

                        if selectedImage == nil {
                            VStack(spacing: 12) {
                                Image(systemName: "photo.on.rectangle.angled")
                                    .font(.system(size: 64))
                                    .foregroundColor(.secondary)
                                Text("Select a photo to begin")
                                    .foregroundColor(.secondary)
                            }
                            .frame(height: 200)
                        }
                    }
                    .padding()
                }

                Divider()

                // Action Buttons
                VStack(spacing: 12) {
                    PhotosPicker(
                        selection: $selectedItem,
                        matching: .images
                    ) {
                        Label("Select Photo", systemImage: "photo")
                            .frame(maxWidth: .infinity)
                            .frame(height: 44)
                    }
                    .buttonStyle(.borderedProminent)
                    .onChange(of: selectedItem) { newItem in
                        Task {
                            await loadImage(from: newItem)
                        }
                    }

                    HStack(spacing: 12) {
                        Button {
                            applyTheme("film")
                        } label: {
                            Text("Film")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        .disabled(selectedImage == nil)

                        Button {
                            applyTheme("lightroom")
                        } label: {
                            Text("Lightroom")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        .disabled(selectedImage == nil)
                    }
                    .frame(height: 44)

                    HStack(spacing: 12) {
                        Button {
                            applyTheme("strap")
                        } label: {
                            Text("Strap")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        .disabled(selectedImage == nil)

                        Button {
                            applyTheme("monitor")
                        } label: {
                            Text("Monitor")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        .disabled(selectedImage == nil)
                    }
                    .frame(height: 44)
                }
                .padding()
            }
            .navigationBarTitleDisplayMode(.inline)
        }
        .onAppear {
            rustVersion = RustBridge.version()
            statusMessage = "Ready - FFI initialized"
        }
    }

    func loadImage(from item: PhotosPickerItem?) async {
        guard let item = item else { return }

        statusMessage = "Loading image..."

        do {
            guard let data = try await item.loadTransferable(type: Data.self) else {
                statusMessage = "Failed to load image data"
                return
            }

            guard let image = UIImage(data: data) else {
                statusMessage = "Failed to decode image"
                return
            }

            await MainActor.run {
                selectedImage = image
                selectedImageData = data // Preserve original data with EXIF
                processedImage = nil
                statusMessage = "Image loaded - \(Int(image.size.width))x\(Int(image.size.height))"
            }

        } catch {
            await MainActor.run {
                statusMessage = "Error: \(error.localizedDescription)"
            }
        }
    }

    func applyTheme(_ themeName: String) {
        guard let image = selectedImage else { return }
        guard let originalData = selectedImageData else {
            statusMessage = "No image data available"
            return
        }

        statusMessage = "Applying \(themeName) theme..."
        print("📸 Starting theme application: \(themeName)")

        Task {
            // Save to temp file
            let tempInputURL = FileManager.default.temporaryDirectory
                .appendingPathComponent("input_\(UUID().uuidString).jpg")
            let tempOutputURL = FileManager.default.temporaryDirectory
                .appendingPathComponent("output_\(UUID().uuidString).jpg")

            print("📂 Input path: \(tempInputURL.path)")
            print("📂 Output path: \(tempOutputURL.path)")

            // Use original data to preserve EXIF
            let jpegData = originalData
            print("📋 Using original data with EXIF (\(jpegData.count) bytes)")

            do {
                try jpegData.write(to: tempInputURL)
                print("✅ Wrote \(jpegData.count) bytes to temp file")

                // Create processor and apply theme
                guard let proc = RustBridge() else {
                    print("❌ Failed to create RustBridge processor")
                    await MainActor.run {
                        statusMessage = "Failed to create processor"
                    }
                    return
                }
                print("✅ Created processor")

                let loadSuccess = proc.loadImage(path: tempInputURL.path)
                print("Load result: \(loadSuccess)")
                guard loadSuccess else {
                    print("❌ Failed to load image in Rust")
                    await MainActor.run {
                        statusMessage = "Failed to load image in Rust"
                    }
                    return
                }
                print("✅ Image loaded in Rust")

                let themeSuccess = proc.applyTheme(
                    themeName: themeName,
                    outputPath: tempOutputURL.path
                )
                print("Theme apply result: \(themeSuccess)")

                if themeSuccess {
                    print("✅ Theme applied, checking output file...")

                    if FileManager.default.fileExists(atPath: tempOutputURL.path) {
                        let fileSize = try? FileManager.default.attributesOfItem(atPath: tempOutputURL.path)[.size] as? Int
                        print("✅ Output file exists, size: \(fileSize ?? 0) bytes")

                        if let outputData = try? Data(contentsOf: tempOutputURL),
                           let outputImage = UIImage(data: outputData) {
                            print("✅ Successfully decoded output image")
                            await MainActor.run {
                                processedImage = outputImage
                                statusMessage = "Theme '\(themeName)' applied! (\(outputData.count) bytes)"
                            }
                        } else {
                            print("❌ Failed to decode output image")
                            await MainActor.run {
                                statusMessage = "Failed to decode output image"
                            }
                        }
                    } else {
                        print("❌ Output file doesn't exist")
                        await MainActor.run {
                            statusMessage = "Output file not created"
                        }
                    }
                } else {
                    print("❌ Theme application returned false")
                    await MainActor.run {
                        statusMessage = "Theme '\(themeName)' failed"
                    }
                }

                // Cleanup
                try? FileManager.default.removeItem(at: tempInputURL)
                try? FileManager.default.removeItem(at: tempOutputURL)
                print("🧹 Cleaned up temp files")

            } catch {
                print("❌ Exception: \(error)")
                await MainActor.run {
                    statusMessage = "Error: \(error.localizedDescription)"
                }
            }
        }
    }
}

#Preview {
    ContentView()
}
