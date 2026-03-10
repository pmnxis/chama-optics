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

struct FaceRect {
    let x: Int
    let y: Int
    let width: Int
    let height: Int

    var x2: Int { x + width }
    var y2: Int { y + height }
    var area: Int { width * height }
}

/// IoU (Intersection over Union) between two face rectangles
func iou(_ a: FaceRect, _ b: FaceRect) -> Double {
    let ix1 = max(a.x, b.x)
    let iy1 = max(a.y, b.y)
    let ix2 = min(a.x2, b.x2)
    let iy2 = min(a.y2, b.y2)

    let intersection = max(0, ix2 - ix1) * max(0, iy2 - iy1)
    guard intersection > 0 else { return 0.0 }

    let union = a.area + b.area - intersection
    return union > 0 ? Double(intersection) / Double(union) : 0.0
}

/// NMS: remove duplicates with IoU > threshold, keep larger boxes first
func nms(_ faces: [FaceRect], threshold: Double = 0.4) -> [FaceRect] {
    let sorted = faces.sorted { $0.area > $1.area }
    var kept: [FaceRect] = []

    for candidate in sorted {
        let overlaps = kept.contains { iou(candidate, $0) > threshold }
        if !overlaps {
            kept.append(candidate)
        }
    }
    return kept
}

struct FaceDetector {
    func detectFaces(in imagePath: String) -> [[String: Any]] {
        return autoreleasepool {
            let imageUrl = URL(fileURLWithPath: imagePath)

            // Read EXIF properties to determine display dimensions
            guard let imageSource = CGImageSourceCreateWithURL(imageUrl as CFURL, nil),
                  let properties = CGImageSourceCopyPropertiesAtIndex(imageSource, 0, nil)
                      as? [String: Any],
                  let rawWidth = properties[kCGImagePropertyPixelWidth as String] as? Int,
                  let rawHeight = properties[kCGImagePropertyPixelHeight as String] as? Int
            else {
                fputs("[FAIL][VisionKit] Failed to load image at: \(imagePath)\n", stderr)
                return []
            }

            // EXIF orientations 5-8 swap width/height in the displayed image
            let exifOrientation = properties[kCGImagePropertyOrientation as String] as? Int ?? 1
            let swapsAxes = (5...8).contains(exifOrientation)
            let displayWidth = swapsAxes ? rawHeight : rawWidth
            let displayHeight = swapsAxes ? rawWidth : rawHeight

            fputs(
                "[INFO][VisionKit] Image \(rawWidth)x\(rawHeight)"
                    + " orientation:\(exifOrientation)"
                    + " → display \(displayWidth)x\(displayHeight)\n",
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

            // Best revision found: Revision2
            request.revision = VNDetectFaceRectanglesRequestRevision2

            // URL-based handler: Vision reads EXIF orientation automatically
            let handler = VNImageRequestHandler(url: imageUrl, options: [:])

            do {
                fputs("[INFO][VisionKit] Running Vision framework (URL handler, EXIF-aware)...\n", stderr)
                try handler.perform([request])

                guard let observations = request.results as? [VNFaceObservation] else {
                    fputs("[INFO][VisionKit] No faces detected\n", stderr)
                    return []
                }

                fputs("[PASS] [VisionKit] Detected \(observations.count) face(s) (pre-NMS)\n", stderr)

                // Convert normalized Vision coords to pixel coords
                // Vision: origin bottom-left of displayed image → flip Y
                var rawFaces: [FaceRect] = []
                for observation in observations {
                    let rect = observation.boundingBox
                    let x = Int(rect.origin.x * Double(displayWidth))
                    let y = Int((1.0 - rect.origin.y - rect.height) * Double(displayHeight))
                    let w = Int(rect.width * Double(displayWidth))
                    let h = Int(rect.height * Double(displayHeight))

                    let xi = max(0, min(x, displayWidth))
                    let yi = max(0, min(y, displayHeight))
                    let wi = min(w, displayWidth - xi)
                    let hi = min(h, displayHeight - yi)

                    if wi > 0 && hi > 0 {
                        rawFaces.append(FaceRect(x: xi, y: yi, width: wi, height: hi))
                    }
                }

                // IoU-based NMS deduplication (threshold 0.4)
                let dedupedFaces = nms(rawFaces)
                fputs("[PASS] [VisionKit] After NMS: \(dedupedFaces.count) face(s)\n", stderr)

                var faceResults: [[String: Any]] = []
                for (index, face) in dedupedFaces.enumerated() {
                    faceResults.append([
                        "x": face.x,
                        "y": face.y,
                        "width": face.width,
                        "height": face.height,
                    ])
                    fputs(
                        "[INFO][VisionKit] Face #\(index + 1):"
                            + " x=\(face.x), y=\(face.y), w=\(face.width), h=\(face.height)\n",
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
