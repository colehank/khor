import AppKit
import WebKit
import Darwin

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let config = WKWebViewConfiguration()
config.mediaTypesRequiringUserActionForPlayback = []
let web = WKWebView(frame: NSRect(x: 0, y: 0, width: 320, height: 240), configuration: config)
let window = NSWindow(contentRect: web.frame, styleMask: [.titled], backing: .buffered, defer: false)
window.contentView = web
window.orderFront(nil)

func wait(_ seconds: Double, until condition: () -> Bool) -> Bool {
    let deadline = Date().addingTimeInterval(seconds)
    while Date() < deadline {
        if condition() { return true }
        RunLoop.current.run(until: Date().addingTimeInterval(0.02))
    }
    return condition()
}
func javascript(_ script: String) -> Any? {
    var done = false
    var value: Any?
    web.evaluateJavaScript(script) { result, error in value = result; done = true }
    precondition(wait(10) { done }, "JavaScript callback timed out")
    return value
}
func state() -> WKMediaPlaybackState {
    var value = WKMediaPlaybackState.none
    var done = false
    web.requestMediaPlaybackState { next in value = next; done = true }
    precondition(wait(10) { done }, "Media callback timed out")
    return value
}

// Silent PCM with a real HTMLMediaElement/renderer; the test emits no sound.
let samples = 8000 * 2
func little<T: FixedWidthInteger>(_ value: T) -> [UInt8] {
    var word = value.littleEndian
    return withUnsafeBytes(of: &word) { Array($0) }
}
var wav = Array("RIFF".utf8) + little(UInt32(36 + samples)) + Array("WAVEfmt ".utf8)
wav += little(UInt32(16)) + little(UInt16(1)) + little(UInt16(1))
wav += little(UInt32(8000)) + little(UInt32(16000)) + little(UInt16(2)) + little(UInt16(16))
wav += Array("data".utf8) + little(UInt32(samples)) + Array(repeating: UInt8(0), count: samples)
let data = Data(wav).base64EncodedString()
web.loadHTMLString("<audio id='audio' autoplay loop src='data:audio/wav;base64,\(data)'></audio>", baseURL: nil)
precondition(wait(15) { javascript("!!document.getElementById('audio')") as? Bool == true }, "Audio document did not load")
_ = javascript("document.getElementById('audio').play(); true")
precondition(wait(10) { state() == .playing }, "Media must really be playing before closing")
web.removeFromSuperview()

if !CommandLine.arguments.contains("--remove-only") {
    let library = dlopen(CommandLine.arguments[1], RTLD_NOW | RTLD_LOCAL)
    precondition(library != nil, "Cannot open Rust page retirement probe")
    typealias Retire = @convention(c) (UnsafeMutableRawPointer) -> Void
    let symbol = dlsym(library, "khor_retire_page")
    precondition(symbol != nil)
    let retire = unsafeBitCast(symbol, to: Retire.self)
    retire(Unmanaged.passUnretained(web).toOpaque())
}
// Hold the detached view to simulate an in-flight callback/media owner.
precondition(wait(5) { state() != .playing }, "Closed page is still playing after removal")
precondition(wait(5) { javascript("!document.getElementById('audio')") as? Bool == true }, "Closing must replace the document, including its scripts")
print("PASS native WKWebView: playing media stopped and document retired while another owner retains the view")
