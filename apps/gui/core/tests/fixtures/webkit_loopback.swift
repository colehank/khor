import AppKit
import Network
import WebKit

// An isolated native networking probe. No app/user website data is opened.
final class Probe: NSObject, WKNavigationDelegate {
    var web: WKWebView!
    var finished = false
    let mode = CommandLine.arguments[1]

    func start() {
        let config = WKWebViewConfiguration()
        let store = WKWebsiteDataStore.nonPersistent()
        if mode != "direct" {
            let endpoint = NWEndpoint.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(CommandLine.arguments[3])!)
            var proxy = ProxyConfiguration(httpCONNECTProxy: endpoint, tlsOptions: nil)
            proxy.allowFailover = false
            if mode == "explicit" {
                proxy.matchDomains = ["", "127.0.0.1", "localhost"]
                proxy.excludedDomains = []
            }
            store.proxyConfigurations = [proxy]
        }
        config.websiteDataStore = store
        web = WKWebView(frame: CGRect(x: 0, y: 0, width: 480, height: 320), configuration: config)
        web.navigationDelegate = self
        web.load(URLRequest(url: URL(string: CommandLine.arguments[2])!))
        DispatchQueue.main.asyncAfter(deadline: .now() + 20) {
            self.finish(["error": "navigation timeout"])
        }
    }

    func finish(_ result: [String: Any]) {
        guard !finished else { return }
        finished = true
        var value = result
        value["mode"] = mode
        let data = try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
        print(String(data: data, encoding: .utf8)!)
        fflush(stdout)
        NSApplication.shared.terminate(nil)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        webView.evaluateJavaScript("document.body.innerText") { value, error in
            if let error = error { self.finish(["error": error.localizedDescription]) }
            else { self.finish(["body": value as? String ?? ""]) }
        }
    }

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        finish(["error": error.localizedDescription])
    }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        finish(["error": error.localizedDescription])
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
let probe = Probe()
probe.start()
app.run()
