/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

import CoreGraphics
import Foundation
import Vision

// MARK: - C FFI Types

/// C-compatible face rectangle result
public struct CFaceRectResult {
    public var x: Int32
    public var y: Int32
    public var width: UInt32
    public var height: UInt32
}

// MARK: - Speed Mode

/// Speed mode matching Rust SpeedMode / iOS FaceDetectionSpeedMode
/// 0=Fastest, 1=Fast, 2=Normal, 3=Slow, 4=Slowest
enum SpeedMode: Int32 {
    case fastest = 0
    case fast = 1
    case normal = 2
    case slow = 3
    case slowest = 4

    var pyramidDepth: Int {
        switch self {
        case .fastest: return 0
        case .fast: return 0 // Fast uses min(w,h) sweep only
        case .normal: return 1
        case .slow: return 2
        case .slowest: return 3
        }
    }
}

// MARK: - ILC Camera Detection

private func isIlcCameraMake(_ make: String) -> Bool {
    // Function-local static to ensure proper initialization in FFI context
    // (module-level `let` may not initialize correctly when called from Rust)
    struct Static {
        static let ilcMakes: [String] = [
            "sony", "canon", "nikon", "fujifilm", "panasonic",
            "olympus", "om system", "leica", "hasselblad", "pentax",
            "sigma", "phase one", "mamiya", "red", "arri",
        ]
    }
    let lower = make.lowercased()
    return Static.ilcMakes.contains { lower.contains($0) }
}

// MARK: - Face Detection Helpers

/// Internal face rect for processing
private struct FaceRect {
    let x: Int
    let y: Int
    let width: Int
    let height: Int

    var x2: Int { x + width }
    var y2: Int { y + height }
    var area: Int { width * height }
}

/// Get the dedicated dispatch queue for all Vision framework work.
/// Created as a function-local static to ensure proper initialization in FFI context.
private func getVisionQueue() -> DispatchQueue {
    struct Static {
        static let queue = DispatchQueue(label: "com.chamaoptics.vision", qos: .userInitiated)
    }
    return Static.queue
}

/// Detect faces in a CGImage region, returning pixel coordinates relative to the full image.
private func detectFacesInRegion(
    cgImage: CGImage,
    imageSize: (width: Int, height: Int),
    regionOffset: (x: Int, y: Int),
    scaleFactor: Double = 1.0
) -> [FaceRect] {
    let request = VNDetectFaceRectanglesRequest()
    // Force CPU-only to avoid ANE resource exhaustion with many sequential calls
    if #available(macOS 10.15, *) {
        request.usesCPUOnly = true
    }
    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

    do {
        try handler.perform([request])
        guard let observations = request.results, !observations.isEmpty else { return [] }

        let regionWidth = Double(cgImage.width)
        let regionHeight = Double(cgImage.height)

        return observations.compactMap { observation in
            let rect = observation.boundingBox
            var x = Int(rect.origin.x * regionWidth / scaleFactor + Double(regionOffset.x))
            var y = Int(
                (1.0 - rect.origin.y - rect.height) * regionHeight / scaleFactor
                    + Double(regionOffset.y))
            var w = Int(rect.width * regionWidth / scaleFactor)
            var h = Int(rect.height * regionHeight / scaleFactor)

            // Clamp to image boundaries
            x = max(0, min(x, imageSize.width))
            y = max(0, min(y, imageSize.height))
            w = min(w, imageSize.width - x)
            h = min(h, imageSize.height - y)

            guard w >= 20, h >= 20 else { return nil }
            return FaceRect(x: x, y: y, width: w, height: h)
        }
    } catch {
        return []
    }
}

// MARK: - Duplicate Merging (iOS-compatible two-tier)

/// Containment ratio: fraction of smaller rect covered by intersection
private func containmentRatio(_ a: FaceRect, _ b: FaceRect) -> Double {
    let ix1 = max(a.x, b.x)
    let iy1 = max(a.y, b.y)
    let ix2 = min(a.x2, b.x2)
    let iy2 = min(a.y2, b.y2)

    let iw = max(0, ix2 - ix1)
    let ih = max(0, iy2 - iy1)
    guard iw > 0, ih > 0 else { return 0.0 }

    let intersectionArea = Double(iw * ih)
    let smallerArea = Double(min(a.area, b.area))
    return smallerArea > 0 ? intersectionArea / smallerArea : 0.0
}

/// IoU (Intersection over Union)
private func iou(_ a: FaceRect, _ b: FaceRect) -> Double {
    let ix1 = max(a.x, b.x)
    let iy1 = max(a.y, b.y)
    let ix2 = min(a.x2, b.x2)
    let iy2 = min(a.y2, b.y2)

    let intersection = max(0, ix2 - ix1) * max(0, iy2 - iy1)
    guard intersection > 0 else { return 0.0 }

    let union = a.area + b.area - intersection
    return union > 0 ? Double(intersection) / Double(union) : 0.0
}

/// Two-tier merge: containment suppression (60%) + IoU merge (30%)
/// Matches iOS FaceDetectionBridge.mergeDuplicateFaces
private func mergeDuplicateFaces(_ faces: [FaceRect]) -> [FaceRect] {
    guard !faces.isEmpty else { return [] }

    let sorted = faces.sorted { $0.area > $1.area }
    var merged: [FaceRect] = []
    var used = [Bool](repeating: false, count: sorted.count)

    for i in 0..<sorted.count {
        if used[i] { continue }
        var current = sorted[i]
        used[i] = true

        for j in (i + 1)..<sorted.count {
            if used[j] { continue }
            let other = sorted[j]

            // Containment suppression: smaller face mostly inside larger → suppress
            if containmentRatio(current, other) > 0.6 {
                used[j] = true
                continue
            }

            // IoU merge: similarly-sized overlapping faces → expand bounding box
            if iou(current, other) > 0.3 {
                let minX = min(current.x, other.x)
                let minY = min(current.y, other.y)
                let maxX = max(current.x2, other.x2)
                let maxY = max(current.y2, other.y2)
                current = FaceRect(x: minX, y: minY, width: maxX - minX, height: maxY - minY)
                used[j] = true
            }
        }

        merged.append(current)
    }

    return merged
}

// MARK: - Main Detection with Pyramid

/// Full face detection with speed-mode-dependent pyramid sliding windows.
/// Ported from iOS FaceDetectionBridge.detectFaces.
private func detectFacesWithPyramid(imagePath: String, mode: SpeedMode) -> [FaceRect] {
    NSLog("[VisionKit] detectFacesWithPyramid: mode=%d, path=%@", mode.rawValue, imagePath)
    let imageUrl = URL(fileURLWithPath: imagePath)

    // Read image properties for EXIF orientation and dimensions
    guard let imageSource = CGImageSourceCreateWithURL(imageUrl as CFURL, nil),
        let properties = CGImageSourceCopyPropertiesAtIndex(imageSource, 0, nil)
            as? [String: Any],
        let rawWidth = properties[kCGImagePropertyPixelWidth as String] as? Int,
        let rawHeight = properties[kCGImagePropertyPixelHeight as String] as? Int
    else {
        return []
    }

    // EXIF orientation: 5-8 swap display dimensions
    let exifOrientation = properties[kCGImagePropertyOrientation as String] as? Int ?? 1
    let swapsAxes = (5...8).contains(exifOrientation)
    let displayWidth = swapsAxes ? rawHeight : rawWidth
    let displayHeight = swapsAxes ? rawWidth : rawHeight

    // Create orientation-corrected CGImage for cropping
    // Use URL-based handler for whole-image detection (EXIF-aware),
    // but for region cropping we need the oriented CGImage
    guard let cgImageRaw = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
        return []
    }

    // Apply EXIF orientation to get the displayed CGImage
    let orientedCgImage: CGImage
    if exifOrientation == 1 {
        orientedCgImage = cgImageRaw
    } else {
        // Create oriented image using Core Graphics context
        let ctx = CGContext(
            data: nil,
            width: displayWidth,
            height: displayHeight,
            bitsPerComponent: cgImageRaw.bitsPerComponent,
            bytesPerRow: 0,
            space: cgImageRaw.colorSpace ?? CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: cgImageRaw.bitmapInfo.rawValue
        )
        if let ctx = ctx {
            applyExifTransform(
                ctx: ctx, orientation: exifOrientation,
                width: displayWidth, height: displayHeight)
            ctx.draw(cgImageRaw, in: CGRect(x: 0, y: 0, width: rawWidth, height: rawHeight))
            orientedCgImage = ctx.makeImage() ?? cgImageRaw
        } else {
            orientedCgImage = cgImageRaw
        }
    }

    let imageSize = (width: displayWidth, height: displayHeight)

    // ── Step 1: Fastest — single whole-image detection ──
    // Use URL-based handler for whole-image (reads EXIF automatically)
    var allFaces: [FaceRect] = []
    let request = VNDetectFaceRectanglesRequest()
    let handler = VNImageRequestHandler(url: imageUrl, options: [:])

    if let observations: [VNFaceObservation] = try? {
        try handler.perform([request])
        return request.results
    }() {
        for observation in observations {
            let rect = observation.boundingBox
            var x = Int(rect.origin.x * Double(displayWidth))
            var y = Int((1.0 - rect.origin.y - rect.height) * Double(displayHeight))
            var w = Int(rect.width * Double(displayWidth))
            var h = Int(rect.height * Double(displayHeight))

            x = max(0, min(x, displayWidth))
            y = max(0, min(y, displayHeight))
            w = min(w, displayWidth - x)
            h = min(h, displayHeight - y)

            if w >= 20, h >= 20 {
                allFaces.append(FaceRect(x: x, y: y, width: w, height: h))
            }
        }
    }

    NSLog("[VisionKit] Whole-image detection done: %d face(s), image %dx%d", allFaces.count, displayWidth, displayHeight)

    if mode == .fastest {
        return allFaces
    }

    // ── Step 2: Fast sweep — min(w,h) × min(w,h) with 10% overlap ──
    let minSide = min(displayWidth, displayHeight)
    let fastStep = max(1, Int(Float(minSide) * 0.9))

    var fastRegionCount = 0
    var fy = 0
    while fy < displayHeight {
        var fx = 0
        while fx < displayWidth {
            autoreleasepool {
                let cropW = min(minSide, displayWidth - fx)
                let cropH = min(minSide, displayHeight - fy)
                if cropW > 0, cropH > 0,
                    let cropped = orientedCgImage.cropping(
                        to: CGRect(x: fx, y: fy, width: cropW, height: cropH))
                {
                    let regionFaces = detectFacesInRegion(
                        cgImage: cropped,
                        imageSize: imageSize,
                        regionOffset: (x: fx, y: fy)
                    )
                    allFaces.append(contentsOf: regionFaces)
                    fastRegionCount += 1
                }
            }
            fx += fastStep
        }
        fy += fastStep
    }
    NSLog("[VisionKit] Fast sweep done: %d regions, %d faces so far", fastRegionCount, allFaces.count)

    if mode == .fast {
        return mergeDuplicateFaces(allFaces)
    }

    // ── Step 3: Dynamic pyramid — Normal / Slow / Slowest ──
    let mMaxRaw = Foundation.log2(Double(minSide) * 0.9 / 640.0).rounded(.down)
    guard mMaxRaw >= 0 else {
        return mergeDuplicateFaces(allFaces)
    }
    let mMax = Int(mMaxRaw)

    // ILC camera extension: read EXIF camera make
    let tiffDict = properties[kCGImagePropertyTIFFDictionary as String] as? [String: Any]
    let cameraMake = (tiffDict?[kCGImagePropertyTIFFMake as String] as? String) ?? ""
    let isIlc = mode == .slowest && !cameraMake.isEmpty && isIlcCameraMake(cameraMake)

    let numLevels: Int
    if mode == .slowest && isIlc {
        numLevels = mMax + 1
    } else {
        numLevels = min(mode.pyramidDepth, mMax + 1)
    }

    NSLog("[VisionKit] Pyramid: mMax=%d, numLevels=%d, isIlc=%d", mMax, numLevels, isIlc ? 1 : 0)

    for depth in 0..<numLevels {
        let windowScaled = 640 << (mMax - depth)
        let step = max(1, Int(Float(windowScaled) * 0.9))
        var pyramidRegionCount = 0

        var py = 0
        while py < displayHeight {
            var px = 0
            while px < displayWidth {
                autoreleasepool {
                    let cropW = min(windowScaled, displayWidth - px)
                    let cropH = min(windowScaled, displayHeight - py)
                    if cropW > 0, cropH > 0,
                        let cropped = orientedCgImage.cropping(
                            to: CGRect(x: px, y: py, width: cropW, height: cropH))
                    {
                        let regionFaces = detectFacesInRegion(
                            cgImage: cropped,
                            imageSize: imageSize,
                            regionOffset: (x: px, y: py)
                        )
                        allFaces.append(contentsOf: regionFaces)
                        pyramidRegionCount += 1
                    }
                }
                px += step
            }
            py += step
        }
        NSLog("[VisionKit] Pyramid depth %d/%d done: window=%d, %d regions, %d total faces",
              depth, numLevels, windowScaled, pyramidRegionCount, allFaces.count)
    }

    return mergeDuplicateFaces(allFaces)
}

// MARK: - EXIF Orientation Transform

/// Apply EXIF orientation transform to a CGContext
private func applyExifTransform(ctx: CGContext, orientation: Int, width: Int, height: Int) {
    let w = CGFloat(width)
    let h = CGFloat(height)

    switch orientation {
    case 2:  // Flipped horizontally
        ctx.translateBy(x: w, y: 0)
        ctx.scaleBy(x: -1, y: 1)
    case 3:  // Rotated 180°
        ctx.translateBy(x: w, y: h)
        ctx.rotate(by: CGFloat.pi)
    case 4:  // Flipped vertically
        ctx.translateBy(x: 0, y: h)
        ctx.scaleBy(x: 1, y: -1)
    case 5:  // Transposed (rotated 90° CW + flipped horizontally)
        ctx.translateBy(x: 0, y: w)
        ctx.rotate(by: -CGFloat.pi / 2)
        ctx.scaleBy(x: -1, y: 1)
    case 6:  // Rotated 90° CW
        ctx.translateBy(x: h, y: 0)
        ctx.rotate(by: CGFloat.pi / 2)
    case 7:  // Transverse (rotated 90° CCW + flipped horizontally)
        ctx.translateBy(x: h, y: w)
        ctx.rotate(by: CGFloat.pi / 2)
        ctx.scaleBy(x: -1, y: 1)
    case 8:  // Rotated 90° CCW
        ctx.translateBy(x: 0, y: w)
        ctx.rotate(by: -CGFloat.pi / 2)
    default:
        break  // orientation 1: no transform
    }
}

// MARK: - C FFI Exports

/// Detect faces in an image file.
///
/// - Parameters:
///   - imagePath: null-terminated C string path to the image file
///   - speedMode: 0=Fastest, 1=Fast, 2=Normal, 3=Slow, 4=Slowest
///   - outCount: pointer to write the number of detected faces
/// - Returns: pointer to array of CFaceRectResult (4×Int32 each), or nil if no faces.
///            Caller must free with visionkit_free_faces.
@_cdecl("visionkit_detect_faces")
public func visionkit_detect_faces(
    _ imagePath: UnsafePointer<CChar>,
    _ speedMode: Int32,
    _ outCount: UnsafeMutablePointer<Int32>
) -> UnsafeMutableRawPointer? {
    let path = String(cString: imagePath)
    let mode = SpeedMode(rawValue: speedMode) ?? .fastest

    // Run detection directly on the calling thread.
    // With usesCPUOnly=true, no ANE access, so no thread restrictions.
    let faces = autoreleasepool {
        detectFacesWithPyramid(imagePath: path, mode: mode)
    }

    guard !faces.isEmpty else {
        outCount.pointee = 0
        return nil
    }

    let count = faces.count
    let buffer = UnsafeMutablePointer<CFaceRectResult>.allocate(capacity: count)
    for (i, face) in faces.enumerated() {
        buffer[i] = CFaceRectResult(
            x: Int32(face.x),
            y: Int32(face.y),
            width: UInt32(face.width),
            height: UInt32(face.height)
        )
    }

    outCount.pointee = Int32(count)
    return UnsafeMutableRawPointer(buffer)
}

/// Free an array allocated by visionkit_detect_faces.
@_cdecl("visionkit_free_faces")
public func visionkit_free_faces(
    _ ptr: UnsafeMutableRawPointer?,
    _ count: Int32
) {
    guard let ptr = ptr, count > 0 else { return }
    let typed = ptr.assumingMemoryBound(to: CFaceRectResult.self)
    typed.deallocate()
}
