import UIKit
import WebKit

@main
final class KeyboardProbe: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        let origin = Bundle.main.object(forInfoDictionaryKey: "KhorProbeURL") as! String
        let controller = UIViewController()
        let web = WKWebView()
        web.isInspectable = true
        web.scrollView.bounces = false
        web.scrollView.contentInsetAdjustmentBehavior = .never
        controller.view = web

        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = controller
        window.makeKeyAndVisible()
        self.window = window
        web.load(URLRequest(url: URL(string: origin)!))
        return true
    }
}
