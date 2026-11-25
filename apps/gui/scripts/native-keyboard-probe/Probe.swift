import UIKit
import WebKit

#if PASTE_PROBE
@_silgen_name("read_terminal_paste_probe")
func readTerminalPasteProbe(_ path: UnsafePointer<CChar>)

final class ClipboardProbe: NSObject, WKScriptMessageHandlerWithReply {
    func userContentController(
        _ userContentController: WKUserContentController,
        didReceive message: WKScriptMessage,
        replyHandler: @escaping (Any?, String?) -> Void
    ) {
        let path = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        path.path.withCString { readTerminalPasteProbe($0) }
        Task { @MainActor in
            for _ in 0 ..< 300 {
                if let data = try? Data(contentsOf: path) {
                    try? FileManager.default.removeItem(at: path)
                    let value = try? JSONSerialization.jsonObject(with: data, options: .fragmentsAllowed)
                    replyHandler((value as? [String: Any])?["text"] as? String ?? "", nil)
                    return
                }
                try await Task.sleep(for: .milliseconds(100))
            }
            replyHandler(nil, "The clipboard read timed out")
        }
    }
}
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
        #if PASTE_PROBE
        if let payload = Bundle.main.object(forInfoDictionaryKey: "KhorClipboardSeed") as? String {
            let source = UIWindow(frame: UIScreen.main.bounds)
            source.rootViewController = UIViewController()
            source.makeKeyAndVisible()
            window = source
            UIPasteboard.general.string = payload
            let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            try? Data().write(to: documents.appendingPathComponent("clipboard-seeded"))
            return true
        }
        #endif
        let origin = Bundle.main.object(forInfoDictionaryKey: "KhorProbeURL") as! String
        let controller = UIViewController()
        let configuration = WKWebViewConfiguration()
        #if PASTE_PROBE
        configuration.userContentController.addScriptMessageHandler(ClipboardProbe(), contentWorld: .page, name: "terminalPaste")
        configuration.userContentController.addUserScript(WKUserScript(source: """
            Object.defineProperty(navigator, 'clipboard', {
                value: { readText: () => window.webkit.messageHandlers.terminalPaste.postMessage(null) }
            });
            """, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        #endif
        let web = WKWebView(frame: .zero, configuration: configuration)
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
            } catch {
                reply["error"] = error.localizedDescription
            }
            if let encoded = try? JSONSerialization.data(withJSONObject: reply) {
                try? encoded.write(to: directory.appendingPathComponent("keyboard-observation.json"), options: .atomic)
            }
        }
    }
}
