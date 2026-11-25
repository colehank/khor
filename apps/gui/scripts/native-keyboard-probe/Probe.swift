import UIKit
import WebKit

#if PASTE_PROBE
@_silgen_name("mount_terminal_paste_probe")
func mountTerminalPasteProbe(
    _ web: UnsafeMutableRawPointer,
    _ frame: UnsafePointer<Double>,
    _ path: UnsafePointer<CChar>
) -> Bool
#endif

@main
final class KeyboardProbe: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    private var commandTimer: Timer?
    private var lastCommand = 0

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        let origin = Bundle.main.object(forInfoDictionaryKey: "KhorProbeURL") as! String
        let controller = UIViewController()
        let web = WKWebView()
        web.scrollView.bounces = false
        web.scrollView.contentInsetAdjustmentBehavior = .never
        controller.view = web

        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = controller
        window.makeKeyAndVisible()
        self.window = window
        web.load(URLRequest(url: URL(string: origin)!))

        // The host can read the sandbox of this disposable simulator app.
        // Keep JavaScript observations independent of Web Inspector discovery.
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        commandTimer = Timer.scheduledTimer(withTimeInterval: 0.15, repeats: true) { [weak self, weak web] _ in
            DispatchQueue.main.async {
                if let self, let web { self.observe(web, in: documents) }
            }
        }
        return true
    }

    private func observe(_ web: WKWebView, in directory: URL) {
        guard !web.isLoading,
              let data = try? Data(contentsOf: directory.appendingPathComponent("keyboard-command.json")),
              let command = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let serial = command["serial"] as? Int, serial > lastCommand,
              let script = command["script"] as? String
        else { return }
        lastCommand = serial
        Task { @MainActor in
            var reply: [String: Any] = ["serial": serial]
            do {
                let value = try await web.callAsyncJavaScript(script, arguments: [:], in: nil, contentWorld: .page)
                reply["value"] = value ?? NSNull()
                #if PASTE_PROBE
                if command["pasteSeat"] as? Bool == true, let frame = value as? [Double], frame.count == 4 {
                    let path = directory.appendingPathComponent("paste-result.json").path
                    reply["pasteAttached"] = path.withCString { name in
                        frame.withUnsafeBufferPointer { rectangle in
                            mountTerminalPasteProbe(Unmanaged.passUnretained(web).toOpaque(), rectangle.baseAddress!, name)
                        }
                    }
                }
                #endif
            } catch {
                reply["error"] = error.localizedDescription
            }
            if let encoded = try? JSONSerialization.data(withJSONObject: reply) {
                try? encoded.write(to: directory.appendingPathComponent("keyboard-observation.json"), options: .atomic)
            }
        }
    }
}
