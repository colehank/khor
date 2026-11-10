import AppKit
import WebKit

final class FocusProbe: NSObject {
    var window: NSWindow!
    var face: WKWebView!
    var page: WKWebView!
    func value(_ view: WKWebView, _ script: String) async -> Any? {
        try? await view.evaluateJavaScript(script)
    }
    func start() {
        window = NSWindow(contentRect: NSRect(x: 50, y: 50, width: 600, height: 400), styleMask: [.titled, .closable], backing: .buffered, defer: false)
        let root = NSView(frame: NSRect(x: 0, y: 0, width: 600, height: 400))
        window.contentView = root
        face = WKWebView(frame: root.bounds)
        page = WKWebView(frame: NSRect(x: 200, y: 0, width: 400, height: 400))
        root.addSubview(face)
        root.addSubview(page)
        face.loadHTMLString("<input id='field' value='main face'>", baseURL: nil)
        page.loadHTMLString("<input id='field' value='native page'>", baseURL: nil)
        window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
        Task { @MainActor in
            for _ in 0..<50 {
                let first = await value(face, "!!document.getElementById('field')") as? Bool ?? false
                let second = await value(page, "!!document.getElementById('field')") as? Bool ?? false
                if first && second { break }
                try? await Task.sleep(nanoseconds: 100_000_000)
            }
            var records = [[String: Any]]()
            for target in ["main", "native", "main"] {
                let view = target == "main" ? face! : page!
                window.makeFirstResponder(view)
                _ = await value(view, "document.getElementById('field').focus()")
                try? await Task.sleep(nanoseconds: 200_000_000)
                records.append([
                    "target": target, "appActive": NSApplication.shared.isActive,
                    "windowKey": window.isKeyWindow,
                    "mainHasFocus": await value(face, "document.hasFocus()") as? Bool ?? false,
                    "nativeHasFocus": await value(page, "document.hasFocus()") as? Bool ?? false,
                ])
            }
            let data = try! JSONSerialization.data(withJSONObject: records, options: [.sortedKeys])
            print(String(data: data, encoding: .utf8)!)
            fflush(stdout)
            NSApplication.shared.terminate(nil)
        }
    }
}
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let probe = FocusProbe()
probe.start()
app.run()
