#!/usr/bin/env swift

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

/// macOS face detector — pyramid detection using Vision framework.
/// Spawned by Rust as a subprocess; communicates via JSON on stdin/stdout.
///
/// Input:  { "image_path": "/path/to/image", "speed_mode": 0-4 }
/// Output: { "faces": [{ "x": N, "y": N, "width": N, "height": N }, ...] }
///
/// speed_mode: 0=Fastest  1=Fast  2=Normal  3=Slow  4=Slowest
/// Matches iOS FaceDetectionSpeedMode.intValue.

import CoreGraphics
import Foundation
import Vision

// ── Data types ───────────────────────────────────────────────────────────────

struct FaceRect {
    let x: Int
    let y: Int
    let width: Int
    let height: Int
}

// ── Core detection helper ─────────────────────────────────────────────────────

/// Run VNDetectFaceRectanglesRequest on a single CGImage crop.
/// Returned rects are translated back into original-image coordinates.
func detectFacesInRegion(
    cgImage: CGImage,
    imageSize: CGSize,
    regionOffset: CGPoint,
    scaleFactor: Double
) -> [FaceRect] {
    let request = VNDetectFaceRectanglesRequest()
    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

    do {
        try handler.perform([request])
        guard let observations = request.results else { return [] }

        let regionWidth  = CGFloat(cgImage.width)
        let regionHeight = CGFloat(cgImage.height)

        return observations.compactMap { obs -> FaceRect? in
            let bb = obs.boundingBox

            // Vision uses normalised coords (0–1), origin at bottom-left → flip Y
            var x = Int(bb.origin.x * regionWidth / scaleFactor + regionOffset.x)
            var y = Int((1.0 - bb.origin.y - bb.height) * regionHeight / scaleFactor + regionOffset.y)
            var w = Int(bb.width  * regionWidth  / scaleFactor)
            var h = Int(bb.height * regionHeight / scaleFactor)

            // Clamp to image bounds
            x = max(0, min(x, Int(imageSize.width)))
            y = max(0, min(y, Int(imageSize.height)))
            w = min(w, Int(imageSize.width)  - x)
            h = min(h, Int(imageSize.height) - y)

            // Filter tiny detections (likely false positives)
            guard w >= 20, h >= 20 else { return nil }

            return FaceRect(x: x, y: y, width: w, height: h)
        }
    } catch {
        fputs("[FAIL][VisionKit] Region detection: \(error.localizedDescription)\n", stderr)
        return []
    }
}

// ── IoU merge ────────────────────────────────────────────────────────────────

func calculateIoU(_ a: FaceRect, _ b: FaceRect) -> Double {
    let aRect = CGRect(x: a.x, y: a.y, width: a.width, height: a.height)
    let bRect = CGRect(x: b.x, y: b.y, width: b.width, height: b.height)
    let inter = aRect.intersection(bRect)
    guard !inter.isNull, !inter.isEmpty else { return 0.0 }
    let interArea = inter.width * inter.height
    let unionArea = aRect.width * aRect.height + bRect.width * bRect.height - interArea
    return Double(interArea / unionArea)
}

/// Merge overlapping detections (IoU > 0.3 → expand to union bbox).
func mergeDuplicateFaces(_ faces: [FaceRect]) -> [FaceRect] {
    guard !faces.isEmpty else { return [] }
    var merged: [FaceRect] = []
    var used = [Bool](repeating: false, count: faces.count)

    for i in 0 ..< faces.count {
        if used[i] { continue }
        var cur = faces[i]
        used[i] = true

        for j in (i + 1) ..< faces.count {
            if used[j] { continue }
            if calculateIoU(cur, faces[j]) > 0.3 {
                let minX = min(cur.x, faces[j].x)
                let minY = min(cur.y, faces[j].y)
                let maxX = max(cur.x + cur.width,  faces[j].x + faces[j].width)
                let maxY = max(cur.y + cur.height, faces[j].y + faces[j].height)
                cur = FaceRect(x: minX, y: minY, width: maxX - minX, height: maxY - minY)
                used[j] = true
            }
        }
        merged.append(cur)
    }
    return merged
}

// ── Pyramid depth lookup ──────────────────────────────────────────────────────

/// Number of pyramid levels for Normal / Slow / Slowest.
/// Matches iOS FaceDetectionSpeedMode.pyramidDepth.
func pyramidDepth(for speedMode: Int) -> Int {
    switch speedMode {
    case 0:  return 0  // Fastest  — whole image only
    case 1:  return 0  // Fast     — sweep only, no pyramid
    case 2:  return 1  // Normal
    case 3:  return 2  // Slow
    case 4:  return 3  // Slowest
    default: return 1
    }
}

// ── Main pyramid algorithm ────────────────────────────────────────────────────

func detectFaces(cgImage: CGImage, speedMode: Int) -> [FaceRect] {
    let imageWidth  = cgImage.width
    let imageHeight = cgImage.height
    let imageSize   = CGSize(width: imageWidth, height: imageHeight)

    fputs("[INFO][VisionKit] \(imageWidth)×\(imageHeight), speed_mode=\(speedMode)\n", stderr)

    // ── Step 1: Whole-image detection (all modes) ─────────────────────────────
    var allFaces = detectFacesInRegion(
        cgImage: cgImage,
        imageSize: imageSize,
        regionOffset: .zero,
        scaleFactor: 1.0
    )
    fputs("[INFO][VisionKit] Step1 (whole): \(allFaces.count) face(s)\n", stderr)

    if speedMode == 0 { return allFaces }  // Fastest — done

    // ── Step 2: Fast sweep — min(w,h)×min(w,h) tiles with 10 % overlap ───────
    let minSide  = min(imageWidth, imageHeight)
    let fastStep = max(1, Int(Double(minSide) * 0.9))

    var fy = 0
    while fy < imageHeight {
        var fx = 0
        while fx < imageWidth {
            let cropW = min(minSide, imageWidth  - fx)
            let cropH = min(minSide, imageHeight - fy)
            if cropW > 0, cropH > 0,
               let crop = cgImage.cropping(to: CGRect(x: fx, y: fy, width: cropW, height: cropH))
            {
                allFaces += detectFacesInRegion(
                    cgImage: crop,
                    imageSize: imageSize,
                    regionOffset: CGPoint(x: fx, y: fy),
                    scaleFactor: 1.0
                )
            }
            fx += fastStep
        }
        fy += fastStep
    }

    if speedMode == 1 {  // Fast — sweep done
        let merged = mergeDuplicateFaces(allFaces)
        fputs("[INFO][VisionKit] Fast: \(merged.count) face(s)\n", stderr)
        return merged
    }

    // ── Step 3: Dynamic pyramid (Normal / Slow / Slowest) ────────────────────
    // m_max = floor(log2(min_side × 0.9 / 640))
    let mMaxRaw = Foundation.log2(Double(minSide) * 0.9 / 640.0).rounded(.down)
    guard mMaxRaw >= 0 else {
        let merged = mergeDuplicateFaces(allFaces)
        fputs("[INFO][VisionKit] No pyramid (image too small): \(merged.count) face(s)\n", stderr)
        return merged
    }
    let mMax      = Int(mMaxRaw)
    let numLevels = min(pyramidDepth(for: speedMode), mMax + 1)

    fputs("[INFO][VisionKit] Pyramid: mMax=\(mMax), numLevels=\(numLevels)\n", stderr)

    for depth in 0 ..< numLevels {
        // depth 0 → largest window; depth mMax → 640
        let windowSize = 640 << (mMax - depth)
        let step = max(1, Int(Double(windowSize) * 0.9))

        var py = 0
        while py < imageHeight {
            var px = 0
            while px < imageWidth {
                let cropW = min(windowSize, imageWidth  - px)
                let cropH = min(windowSize, imageHeight - py)
                if cropW > 0, cropH > 0,
                   let crop = cgImage.cropping(to: CGRect(x: px, y: py, width: cropW, height: cropH))
                {
                    allFaces += detectFacesInRegion(
                        cgImage: crop,
                        imageSize: imageSize,
                        regionOffset: CGPoint(x: px, y: py),
                        scaleFactor: 1.0
                    )
                }
                px += step
            }
            py += step
        }
    }

    let merged = mergeDuplicateFaces(allFaces)
    fputs("[PASS][VisionKit] \(merged.count) unique face(s) after merging\n", stderr)
    return merged
}

// ── Entry point ───────────────────────────────────────────────────────────────

guard
    let line     = readLine(),
    let jsonData = line.data(using: .utf8),
    let input    = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any],
    let imagePath = input["image_path"] as? String
else {
    fputs("[FAIL][VisionKit] Invalid input JSON\n", stderr)
    exit(1)
}

let speedMode = (input["speed_mode"] as? Int) ?? 2  // default: Normal

guard
    let imageSource = CGImageSourceCreateWithURL(URL(fileURLWithPath: imagePath) as CFURL, nil),
    let cgImage     = CGImageSourceCreateImageAtIndex(imageSource, 0, nil)
else {
    fputs("[FAIL][VisionKit] Failed to load image: \(imagePath)\n", stderr)
    print("{\"faces\":[]}")
    exit(1)
}

let faces = detectFaces(cgImage: cgImage, speedMode: speedMode)

let output: [[String: Any]] = faces.map { f in
    ["x": f.x, "y": f.y, "width": f.width, "height": f.height]
}
let result = try! JSONSerialization.data(withJSONObject: ["faces": output])
print(String(data: result, encoding: .utf8)!)
