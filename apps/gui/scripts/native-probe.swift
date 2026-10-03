// The native gate's eyes (`check-native.mjs`): what a browser gate cannot
// see because AppKit draws it — the three window buttons and the sidebar
// material under the left block. Compiled by the gate on each run.
//
//   window <pid>                 → "<id> <x> <y> <w> <h>" of that process's main window
//   activate <pid>               → bring that process to the front
//   activate-finder              → put another app in front (the window loses focus)
//   measure <png> <w> <x,y>...   → JSON: scale, the close button's box, and the
//                                  colour at each logical point
//   key <pid> <code> [cmd|shift|ctrl|alt]...
//                                → one key press, **posted to that process only**
//   axmenu <pid> <AXRole>        → the first page element with that role asked for
//                                  its context menu (AXShowMenu); "<found> <menus up>"
//   menus <pid>                  → how many pop-up menu windows that process has up
//   menubar <pid>                → JSON: that process's menu bar, each item's name,
//                                  enabled, command key and modifier mask
//   pick <pid> <menu> <item>     → presses one menu item of that process
//   resize <pid> <w> <h>         → sets that process's main window's size
//                                  through accessibility (what the green
//                                  button and tiling do); prints the AX result
//
// **Posted to the pid, never to the HID stream** (the keys and menus half
// of the gate, 2026-09-28): a key sent through the system goes to whatever
// is in front at that instant, and if the owner clicks into their own khor
// mid-run the press lands there. Posted to the scratch app's pid, a press
// can only ever reach the scratch app — if it is not in front the press is
// dropped, which the gate then reads as a red, not as a keystroke in
// somebody else's window.
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

case "key":
  let pid = Int32(args[2])!
  let code = CGKeyCode(Int(args[3])!)
  var flags: CGEventFlags = []
  for f in args.dropFirst(4) {
    switch f {
    case "cmd": flags.insert(.maskCommand)
    case "shift": flags.insert(.maskShift)
    case "ctrl": flags.insert(.maskControl)
    case "alt": flags.insert(.maskAlternate)
    default: break
    }
  }
  // A private source, so the owner's own modifier keys (a ⌘ held down
  // at that moment) are not folded into the press.
  let src = CGEventSource(stateID: .privateState)
  let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true)!
  let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false)!
  down.flags = flags
  up.flags = flags
  down.postToPid(pid)
  spin(0.04)
  up.postToPid(pid)
  spin(0.15)

case "menubar", "pick":
  // The menu bar **through the accessibility API, by pid**. Not System
  // Events: its `set p to first process whose unix id is …` stores a
  // reference *by process name*, and every khor-gui on the machine is
  // named khor-gui — the first run of this read listed the owner's own
  // app's menus instead of the scratch app's.
  let pid = Int32(args[2])!
  let app = AXUIElementCreateApplication(pid)
  func attr(_ el: AXUIElement, _ name: String) -> AnyObject? {
    var v: AnyObject?
    return AXUIElementCopyAttributeValue(el, name as CFString, &v) == .success ? v : nil
  }
  func kids(_ el: AXUIElement) -> [AXUIElement] { (attr(el, kAXChildrenAttribute) as? [AXUIElement]) ?? [] }
  guard let barRef = attr(app, kAXMenuBarAttribute) else { print("{\"error\":\"no menu bar\"}"); exit(0) }
  let bar = barRef as! AXUIElement
  if args[1] == "pick" {
    let (menuTitle, itemTitle) = (args[3], args[4])
    for top in kids(bar) where (attr(top, kAXTitleAttribute) as? String) == menuTitle {
      for menu in kids(top) {
        for item in kids(menu) where (attr(item, kAXTitleAttribute) as? String) == itemTitle {
          print(AXUIElementPerformAction(item, kAXPressAction as CFString) == .success ? "pressed" : "refused")
          exit(0)
        }
      }
    }
    print("missing")
    exit(0)
  }
  var menus: [[String: Any]] = []
  for top in kids(bar) {
    var items: [[String: Any]] = []
    for menu in kids(top) {
      for item in kids(menu) {
        items.append([
          "name": (attr(item, kAXTitleAttribute) as? String) ?? "",
          "enabled": (attr(item, kAXEnabledAttribute) as? Bool) ?? false,
          "key": (attr(item, "AXMenuItemCmdChar") as? String) ?? "",
          "mods": (attr(item, "AXMenuItemCmdModifiers") as? Int) ?? -1,
        ])
      }
    }
    menus.append(["title": (attr(top, kAXTitleAttribute) as? String) ?? "", "items": items])
  }
  let data = try! JSONSerialization.data(withJSONObject: menus)
  print(String(data: data, encoding: .utf8)!)

case "axmenu":
  // Asks for the context menu of the first element with this AX role in
  // the page (`AXShowMenu` — WebKit answers it by sending the page a real
  // `contextmenu` at that element, `ContextMenuController::showContextMenuAt`,
  // so a page that prevents it shows nothing), then counts the menus up.
  // By pid, like everything here. Prints "<found> <menus>".
  let pid = Int32(args[2])!
  let role = args[3]
  let app = AXUIElementCreateApplication(pid)
  func attr(_ el: AXUIElement, _ name: String) -> AnyObject? {
    var v: AnyObject?
    return AXUIElementCopyAttributeValue(el, name as CFString, &v) == .success ? v : nil
  }
  func kids(_ el: AXUIElement) -> [AXUIElement] { (attr(el, kAXChildrenAttribute) as? [AXUIElement]) ?? [] }
  var found: AXUIElement? = nil
  // **A few tries**: WebKit builds a page's accessibility tree when a
  // client first asks for it, and that first ask can come back empty.
  for _ in 0..<6 where found == nil {
    var queue = (attr(app, kAXWindowsAttribute) as? [AXUIElement]) ?? []
    var seen = 0
    while !queue.isEmpty && found == nil && seen < 4000 {
      let el = queue.removeFirst()
      seen += 1
      if (attr(el, kAXRoleAttribute) as? String) == role {
        // Only inside the page: the element must sit under an AXWebArea.
        var p: AXUIElement? = el
        while let q = p {
          if (attr(q, kAXRoleAttribute) as? String) == "AXWebArea" { found = el; break }
          p = attr(q, kAXParentAttribute).map { $0 as! AXUIElement }
        }
        if found != nil { break }
      }
      queue.append(contentsOf: kids(el))
    }
    if found == nil { spin(0.5) }
  }
  guard let el = found else { print("0 0"); exit(0) }
  _ = AXUIElementPerformAction(el, "AXShowMenu" as CFString)
  spin(0.8)
  let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
  let up = list.filter {
    ($0[kCGWindowOwnerPID as String] as? Int32) == pid && (($0[kCGWindowLayer as String] as? Int) ?? 0) >= 101
  }
  print("1 \(up.count)")

case "menus":
  // A context menu is a window of the process that pops it, one level up
  // from the page (`kCGPopUpMenuWindowLevel`, 101). Counted by level, not
  // by name: WebKit's and AppKit's menus have no title.
  let pid = Int32(args[2])!
  let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
  let up = list.filter {
    ($0[kCGWindowOwnerPID as String] as? Int32) == pid && (($0[kCGWindowLayer as String] as? Int) ?? 0) >= 101
  }
  print(up.count)

case "resize":
  // A size set from outside the app (UX-4): AppKit lays the title bar out
  // again, and the three buttons have to be put back (`traffic::place`).
  let pid = Int32(args[2])!
  var size = CGSize(width: Double(args[3])!, height: Double(args[4])!)
  let app = AXUIElementCreateApplication(pid)
  var wins: CFTypeRef?
  guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &wins) == .success,
        let list = wins as? [AXUIElement], let w = list.first,
        let v = AXValueCreate(.cgSize, &size) else { print("-1"); exit(0) }
  print(AXUIElementSetAttributeValue(w, kAXSizeAttribute as CFString, v).rawValue)
  spin(0.5)

default:
  FileHandle.standardError.write("usage: window <pid> | activate <pid> | activate-finder | measure <png> <w> <x,y>... | key <pid> <code> [mods] | axmenu <pid> <role> | menus <pid> | menubar <pid> | pick <pid> <menu> <item> | resize <pid> <w> <h>\n".data(using: .utf8)!)
  exit(2)
}
