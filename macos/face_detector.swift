#!/usr/bin/env swift

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

import CoreGraphics
import Foundation
import Vision

/// Face detector using VisionKit
/// Reads JSON from stdin and outputs face rectangles as JSON

struct FaceDetector {
    func detectFaces(in imagePath: String) -> [[String: Any]] {
        return autoreleasepool {
            let imageUrl = URL(fileURLWithPath: imagePath)

            guard let imageSource = CGImageSourceCreateWithURL(imageUrl as CFURL, nil),
                  let cgImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil)
            else {
                fputs("[FAIL][VisionKit] Failed to load image at: \(imagePath)\n", stderr)
                return []
            }

            let originalWidth = cgImage.width
            let originalHeight = cgImage.height

            // Downsample large images for better performance
            // Best resolution found: 1024px (9 faces detected)
            let maxDimension = 1024
            var imageToProcess = cgImage
            var scaleFactor: Float = 1.0

            if originalWidth > maxDimension || originalHeight > maxDimension {
                let scale = Float(maxDimension) / Float(max(originalWidth, originalHeight))
                scaleFactor = scale
                let newWidth = Int(Float(originalWidth) * scale)
                let newHeight = Int(Float(originalHeight) * scale)

                let colorSpace = cgImage.colorSpace ?? CGColorSpaceCreateDeviceRGB()
                let bitmapInfo = cgImage.bitmapInfo
                let context = CGContext(
                    data: nil,
                    width: newWidth,
                    height: newHeight,
                    bitsPerComponent: 8,
                    bytesPerRow: newWidth * 4,
                    space: colorSpace,
                    bitmapInfo: bitmapInfo.rawValue
                )

                context?.interpolationQuality = .high
                context?.draw(cgImage, in: CGRect(x: 0, y: 0, width: newWidth, height: newHeight))

                if let resizedImage = context?.makeImage() {
                    imageToProcess = resizedImage
                    fputs(
                        "[INFO][VisionKit] Downsampled from \(originalWidth)x\(originalHeight) to \(newWidth)x\(newHeight) (scale: \(scale))\n",
                        stderr
                    )
                }
            }

            let imageWidth = imageToProcess.width
            let imageHeight = imageToProcess.height

            fputs(
                "[INFO][VisionKit] Starting face detection on image: \(imageWidth)x\(imageHeight)\n",
                stderr
            )

            let request = VNDetectFaceRectanglesRequest { _, error in
                if let error = error {
                    fputs(
                        "[FAIL][VisionKit] Detection error: \(error.localizedDescription)\n",
                        stderr
                    )
                }
            }

            // Set revision for better accuracy
            // Available revisions: 1, 2, 3
            // Best revision found: Revision2 (9 faces with 1024px)
            request.revision = VNDetectFaceRectanglesRequestRevision2

            // Try different VNImageOptions
            let handler = VNImageRequestHandler(
                cgImage: imageToProcess, orientation: .up, options: [:]
            )

            do {
                fputs(
                    "[INFO][VisionKit] Running Vision framework with Revision2 and 1024px downsampling (BEST CONFIG)...\n",
                    stderr
                )
                try handler.perform([request])

                guard let observations = request.results as? [VNFaceObservation] else {
                    fputs("[INFO][VisionKit] No faces detected\n", stderr)
                    return []
                }

                let faceCount = observations.count
                fputs("[PASS] [VisionKit] Successfully detected \(faceCount) face(s)\n", stderr)

                let imageSize = CGSize(width: imageWidth, height: imageHeight)
                let originalSize = CGSize(width: originalWidth, height: originalHeight)

                var faceResults: [[String: Any]] = []

                for (index, observation) in observations.enumerated() {
                    let rect = observation.boundingBox

                    // Vision uses normalized coordinates (0-1) with origin at bottom-left
                    // Convert to pixel coordinates with origin at top-left
                    var x = Float(rect.origin.x) * Float(imageWidth)
                    var y = (1.0 - Float(rect.origin.y) - Float(rect.height)) * Float(imageHeight)
                    var width = Float(rect.width) * Float(imageWidth)
                    var height = Float(rect.height) * Float(imageHeight)

                    // Scale back to original image dimensions
                    x = x / scaleFactor
                    y = y / scaleFactor
                    width = width / scaleFactor
                    height = height / scaleFactor

                    // Clamp to image boundaries
                    let xi = max(0, min(Int(x), Int(originalWidth)))
                    let yi = max(0, min(Int(y), Int(originalHeight)))
                    let wi = min(Int(width), Int(originalWidth) - xi)
                    let hi = min(Int(height), Int(originalHeight) - yi)

                    let faceRect: [String: Any] = [
                        "x": xi,
                        "y": yi,
                        "width": wi,
                        "height": hi,
                    ]

                    faceResults.append(faceRect)

                    fputs(
                        "[INFO][VisionKit] Face #\(index + 1): x=\(xi), y=\(yi), w=\(wi), h=\(hi)\n",
                        stderr
                    )
                }

                return faceResults
            } catch {
                fputs(
                    "[FAIL][VisionKit] Failed to perform detection: \(error.localizedDescription)\n",
                    stderr
                )
                return []
            }
        }
    }
}

// Read input JSON
if let data = readLine(), let jsonData = data.data(using: .utf8),
   let input = try? JSONSerialization.jsonObject(with: jsonData) as? [String: String],
   let imagePath = input["image_path"]
{
    let detector = FaceDetector()
    let faces = detector.detectFaces(in: imagePath)

    // Output JSON
    let output = ["faces": faces]
    let jsonData = try! JSONSerialization.data(withJSONObject: output)
    print(String(data: jsonData, encoding: .utf8)!)
} else {
    fputs("[FAIL]Invalid input JSON\n", stderr)
    exit(1)
}
