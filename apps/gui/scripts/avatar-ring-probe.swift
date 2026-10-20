// Private native WebKit raster check for check-avatar-ring.mjs.
import AppKit
import WebKit
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let web = WKWebView(frame: NSRect(x: 0, y: 0, width: 1200, height: 1300))
let window = NSWindow(contentRect: web.frame, styleMask: [.titled, .closable], backing: .buffered, defer: false)
window.contentView = web
window.orderFront(nil)
func spin(_ seconds: Double) { RunLoop.current.run(until: Date().addingTimeInterval(seconds)) }
func ask(_ js: String) -> Any? {
  var done = false
  var answer: Any?
  web.evaluateJavaScript(js) { value, error in answer = value; done = true }
  let end = Date().addingTimeInterval(10)
  while !done && Date() < end { spin(0.02) }
  return answer
}
web.load(URLRequest(url: URL(string: CommandLine.arguments[1])!))
let deadline = Date().addingTimeInterval(20)
while Date() < deadline {
  if (ask("document.querySelectorAll('[data-halo-ring]').length") as? Int ?? 0) == 102 { break }
  spin(0.1)
}
// Hide the centred type glyph to measure the uniform face's painted centre.
// The real face/ring components, SVGs, layout and transparent gap remain.
let js = """
(() => {
 const style=document.createElement('style');style.textContent='[data-side-mark]{visibility:hidden}[data-halo]{animation:none!important}';document.head.append(style);
 return JSON.stringify([...document.querySelectorAll('[data-halo="busy"],[data-halo="done"],[data-halo="idle"]')].map(h=>{const r=h.parentElement.getBoundingClientRect();return {x:r.x,y:r.y,w:r.width,h:r.height,state:h.dataset.halo}}));
})()
"""
guard let json = ask(js) as? String,
      let data = json.data(using: .utf8),
      let faces = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]], faces.count == 51 else {
  print("{\"error\":\"native fixture never rendered busy/done/idle faces\"}"); exit(1)
}
spin(0.15)
var done = false
var snapshot: NSImage?
web.takeSnapshot(with: nil) { image, error in snapshot = image; done = true }
while !done { spin(0.02) }
guard let tiff = snapshot?.tiffRepresentation, let bitmap = NSBitmapImageRep(data: tiff) else { exit(1) }
try! bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
_ = ask("document.querySelectorAll('[data-halo]').forEach(e=>e.style.visibility='hidden')")
spin(0.1)
done = false
var baseline: NSImage?
web.takeSnapshot(with: nil) { image, error in baseline = image; done = true }
while !done { spin(0.02) }
guard let tiffBefore = baseline?.tiffRepresentation, let before = NSBitmapImageRep(data: tiffBefore) else { exit(1) }
let scale = Double(bitmap.pixelsWide) / 1200
var measurements = [[String: Any]]()
for f in faces {
  let cx = (f["x"] as! Double) + (f["w"] as! Double) / 2, cy = (f["y"] as! Double) + (f["h"] as! Double) / 2
  let reach = (f["w"] as! Double) / 2 + 6
  var faceMass = 0.0, ringMass = 0.0, faceX = 0.0, faceY = 0.0, ringX = 0.0, ringY = 0.0
  for y in max(0, Int((cy - reach) * scale))..<min(bitmap.pixelsHigh, Int((cy + reach) * scale + 1)) {
    for x in max(0, Int((cx - reach) * scale))..<min(bitmap.pixelsWide, Int((cx + reach) * scale + 1)) {
      guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB) else { continue }
      let r = color.redComponent, g = color.greenComponent, b = color.blueComponent
      guard let base = before.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB) else { continue }
      let fw = max(0, base.redComponent - max(base.greenComponent, base.blueComponent) - 0.05)
      let rw = max(0, max(abs(r-base.redComponent),abs(g-base.greenComponent),abs(b-base.blueComponent)) - 0.03)
      let px = (Double(x) + 0.5) / scale, py = (Double(y) + 0.5) / scale
      faceMass += fw; faceX += fw * px; faceY += fw * py
      ringMass += rw; ringX += rw * px; ringY += rw * py
    }
  }
  measurements.append(["state": f["state"]!, "size": (f["w"] as! Double), "x": cx, "y": cy, "faceMass": faceMass, "ringMass": ringMass,
    "dx": ringX / max(ringMass, 0.0001) - faceX / max(faceMass, 0.0001),
    "dy": ringY / max(ringMass, 0.0001) - faceY / max(faceMass, 0.0001)])
}
let result = try! JSONSerialization.data(withJSONObject: ["measurements": measurements], options: [.sortedKeys])
print(String(data: result, encoding: .utf8)!)
