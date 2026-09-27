// The native gate's eyes (`check-native.mjs`): what a browser gate cannot
// see because AppKit draws it — the three window buttons and the sidebar
// material under the left block. Compiled by the gate on each run.
//
//   window <pid>                 → "<id> <x> <y> <w> <h>" of that process's main window
//   activate <pid>               → bring that process to the front
//   activate-finder              → put another app in front (the window loses focus)
//   measure <png> <w> <x,y>...   → JSON: scale, the close button's box, and the
//                                  colour at each logical point
import AppKit
import CoreGraphics
import Foundation

let args = CommandLine.arguments

func spin(_ s: Double) { RunLoop.current.run(until: Date().addingTimeInterval(s)) }

switch args.count > 1 ? args[1] : "" {
case "window":
  let pid = Int32(args[2])!
  let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
  var best: (Int, CGRect)? = nil
  for w in list {
    guard (w[kCGWindowOwnerPID as String] as? Int32) == pid,
          (w[kCGWindowLayer as String] as? Int) == 0,
          let b = w[kCGWindowBounds as String] as? [String: Any],
          let r = CGRect(dictionaryRepresentation: b as CFDictionary) else { continue }
    if best == nil || r.width * r.height > best!.1.width * best!.1.height {
      best = (w[kCGWindowNumber as String] as? Int ?? 0, r)
    }
  }
  if let b = best { print(b.0, Int(b.1.minX), Int(b.1.minY), Int(b.1.width), Int(b.1.height)) }

case "activate":
  NSRunningApplication(processIdentifier: Int32(args[2])!)?.activate()
  spin(0.3)

case "activate-finder":
  NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.finder").first?.activate()
  spin(0.3)

case "measure":
  guard let ns = NSImage(contentsOfFile: args[2]),
        let img = ns.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    print("{\"error\":\"unreadable\"}"); exit(0)
  }
  let logicalW = Double(args[3])!
  let W = img.width, H = img.height
  var buf = [UInt8](repeating: 0, count: W * H * 4)
  let ctx = CGContext(data: &buf, width: W, height: H, bitsPerComponent: 8, bytesPerRow: W * 4,
                      space: CGColorSpaceCreateDeviceRGB(),
                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
  ctx.draw(img, in: CGRect(x: 0, y: 0, width: W, height: H))
  let scale = Double(W) / logicalW
  func px(_ x: Int, _ y: Int) -> (Int, Int, Int) {
    let i = (y * W + x) * 4
    return (Int(buf[i]), Int(buf[i + 1]), Int(buf[i + 2]))
  }
  // The close button: the red one, in the corner — only the corner (30
  // wide, the first row tall), or a reddish face in the list's first row
  // is counted with it (first run of this gate: a box 8–98 wide).
  var minX = Int.max, maxX = -1, minY = Int.max, maxY = -1
  for y in 0..<min(H, Int(48 * scale)) {
    for x in 0..<min(W, Int(30 * scale)) {
      let (r, g, b) = px(x, y)
      if r > 200 && g < 120 && b < 120 {
        minX = min(minX, x); maxX = max(maxX, x); minY = min(minY, y); maxY = max(maxY, y)
      }
    }
  }
  var samples: [String] = []
  for p in args.dropFirst(4) {
    let xy = p.split(separator: ",").map { Double($0)! }
    let (r, g, b) = px(min(W - 1, Int(xy[0] * scale)), min(H - 1, Int(xy[1] * scale)))
    samples.append("[\(r),\(g),\(b)]")
  }
  let close = maxX < 0 ? "null"
    : "[\(Double(minX) / scale),\(Double(maxX + 1) / scale),\(Double(minY) / scale),\(Double(maxY + 1) / scale)]"
  print("{\"scale\":\(scale),\"close\":\(close),\"samples\":[\(samples.joined(separator: ","))]}")

default:
  FileHandle.standardError.write("usage: window <pid> | activate <pid> | activate-finder | measure <png> <w> <x,y>...\n".data(using: .utf8)!)
  exit(2)
}
