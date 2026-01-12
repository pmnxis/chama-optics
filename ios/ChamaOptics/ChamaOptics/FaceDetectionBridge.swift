// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT

import CoreGraphics
import Foundation
import UIKit
import Vision

/// Face detection bridge using VisionKit
/// This class provides face detection functionality using Apple's Vision framework
class FaceDetectionBridge {

    // MARK: - C Function Declarations

    /// Apply face detection with rectangles to an image
    @_silgen_name("chama_optics_apply_face_detection")
    private static func chama_optics_apply_face_detection(
        _ handle: OpaquePointer,
        _ faceRects: UnsafePointer<CFaceRect>,
        _ faceCount: Int,
        _ imagePath: UnsafePointer<CChar>,
        _ outputPath: UnsafePointer<CChar>,
        _ borderR: UInt8,
        _ borderG: UInt8,
        _ borderB: UInt8,
        _ borderA: UInt8,
        _ borderThickness: UInt32,
        _ maskFaces: Bool,
        _ maskBlurRadius: Float
    ) -> Bool

    /// Free face rectangle list
    @_silgen_name("chama_optics_free_face_rect_list")
    private static func chama_optics_free_face_rect_list(
        _ list: UnsafeMutablePointer<CFaceRectList>)

    // MARK: - Swift Wrapper Methods

    /// Upsample image to improve detection for small faces
    /// - Parameter cgImage: Original CGImage
    /// - Returns: Upsampled CGImage (2x resolution)
    private static func upsampleImage(_ cgImage: CGImage) -> CGImage? {
        let newWidth = cgImage.width * 2
        let newHeight = cgImage.height * 2

        let context = CGContext(
            data: nil,
            width: newWidth,
            height: newHeight,
            bitsPerComponent: cgImage.bitsPerComponent,
            bytesPerRow: 0,
            space: cgImage.colorSpace ?? CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: cgImage.bitmapInfo.rawValue
        )

        context?.interpolationQuality = .high
        context?.draw(cgImage, in: CGRect(x: 0, y: 0, width: newWidth, height: newHeight))

        return context?.makeImage()
    }

    /// Detect faces in an image using VisionKit
    /// - Parameter image: The UIImage to detect faces in
    /// - Returns: An array of face rectangles as (x, y, width, height) tuples
    static func detectFaces(in image: UIImage) -> [(x: Int, y: Int, width: Int, height: Int)] {
        return autoreleasepool {
            guard let originalCgImage = image.cgImage else {
                print("[FAIL]Failed to get CGImage from UIImage")
                return []
            }

            // Check if image needs upsampling
            let imageWidth = originalCgImage.width
            let imageHeight = originalCgImage.height

            // Use upsampling if image is smaller than 2000px in any dimension
            let needsUpsampling = imageWidth < 2000 || imageHeight < 2000

            let cgImage: CGImage
            let scaleFactor: Double

            if needsUpsampling {
                print(
                    "[INFO]Upsampling image for better face detection: \(imageWidth)x\(imageHeight) -> ",
                    terminator: "")
                if let upsampled = upsampleImage(originalCgImage) {
                    cgImage = upsampled
                    scaleFactor = 2.0
                    print("\(cgImage.width)x\(cgImage.height)")
                } else {
                    cgImage = originalCgImage
                    scaleFactor = 1.0
                    print("Failed, using original")
                }
            } else {
                cgImage = originalCgImage
                scaleFactor = 1.0
            }

            let request = VNDetectFaceRectanglesRequest { request, error in
                if let error = error {
                    print("[FAIL]Vision error: \(error.localizedDescription)")
                }
            }

            let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

            do {
                try handler.perform([request])

                guard let observations = request.results as? [VNFaceObservation] else {
                    print("⚠️ No face observations found")
                    return []
                }

                print("[INFO]Detected \(observations.count) face(s)")

                let imageSize = CGSize(
                    width: originalCgImage.width, height: originalCgImage.height
                )

                let faceRects = observations.compactMap { observation in
                    // Convert Vision coordinates to image coordinates
                    let boundingBox = observation.boundingBox

                    // Vision uses normalized coordinates (0-1) with origin at bottom-left
                    // We need to convert to pixel coordinates with origin at top-left
                    // Apply scale factor if we used upsampling
                    var x = Int(
                        boundingBox.origin.x * imageSize.width * scaleFactor)
                    var y =
                        Int(
                            (1.0 - boundingBox.origin.y - boundingBox.height) * imageSize.height
                                * scaleFactor)
                    var width = Int(boundingBox.width * imageSize.width * scaleFactor)
                    var height = Int(boundingBox.height * imageSize.height * scaleFactor)

                    // Clamp to image boundaries
                    x = max(0, min(x, Int(imageSize.width)))
                    y = max(0, min(y, Int(imageSize.height)))
                    width = min(width, Int(imageSize.width) - x)
                    height = min(height, Int(imageSize.height) - y)

                    return (x: x, y: y, width: width, height: height)
                }

                return faceRects
            } catch {
                print("[FAIL]Failed to perform face detection: \(error.localizedDescription)")
                return []
            }
        }
    }

    /// Apply face detection to an image with customizable border settings
    /// - Parameters:
    ///   - handle: Rust library handle
    ///   - imagePath: Path to input image
    ///   - outputPath: Path to save output image
    ///   - borderColor: Border color as RGBA
    ///   - borderThickness: Thickness of border in pixels
    ///   - maskFaces: Whether to blur mask detected faces
    ///   - maskBlurRadius: Blur radius for masking (1-100 pixels)
    /// - Returns: True if successful, false otherwise
    static func applyFaceDetection(
        handle: OpaquePointer,
        imagePath: String,
        outputPath: String,
        borderColor: (r: UInt8, g: UInt8, b: UInt8, a: UInt8),
        borderThickness: UInt32 = 3,
        maskFaces: Bool = false,
        maskBlurRadius: Float = 20.0
    ) -> Bool {
        // Load the image
        guard let image = UIImage(contentsOfFile: imagePath) else {
            print("[FAIL]Failed to load image from path: \(imagePath)")
            return false
        }

        // Detect faces
        let faces = detectFaces(in: image)

        print("📸 Detected \(faces.count) face(s) in image")

        guard !faces.isEmpty else {
            print("⚠️ No faces detected, but still applying face detection config")
            // Continue even if no faces detected -: Rust side will handle this
        }

        // Convert faces to C array
        return faces.withUnsafeBufferPointer { buffer in
            guard let baseAddress = buffer.baseAddress else {
                return false
            }

            let faceRects = baseAddress.assumingMemoryBound(to: CFaceRect.self)

            return imagePath.withCString { cImagePath in
                outputPath.withCString { cOutputPath in
                    return chama_optics_apply_face_detection(
                        handle,
                        faceRects,
                        faces.count,
                        cImagePath,
                        cOutputPath,
                        borderColor.r,
                        borderColor.g,
                        borderColor.b,
                        borderColor.a,
                        borderThickness,
                        maskFaces,
                        maskBlurRadius
                    )
                }
            }
        }
    }

    /// Convenience method with default red border and no masking
    static func applyFaceDetection(
        handle: OpaquePointer,
        imagePath: String,
        outputPath: String
    ) -> Bool {
        return applyFaceDetection(
            handle: handle,
            imagePath: imagePath,
            outputPath: outputPath,
            borderColor: (r: 255, g: 0, b: 0, a: 255),
            borderThickness: 3,
            maskFaces: false,
            maskBlurRadius: 20.0
        )
    }
}

// MARK: - C-compatible structures

/// C-compatible face rectangle structure
struct CFaceRect {
    var x: Int32
    var y: Int32
    var width: UInt32
    var height: UInt32
}

/// Array of face rectangles
struct CFaceRectList {
    var faces: UnsafeMutablePointer<CFaceRect>
    var count: Int
}
