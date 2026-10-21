import UIKit
import WebKit

@main final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    func application(_ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = ProbeController()
        window.makeKeyAndVisible()
        self.window = window
        return true
    }
}

@MainActor final class ProbeController: UIViewController {
    let source = WKWebView(frame: .zero)
    let site = WKWebView(frame: .zero)
    let field = UITextField(frame: .zero)
    var checks: [[String: Any]] = []
    var running = false
    var html = ""
    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        source.frame = view.bounds
        source.autoresizingMask = [.flexibleWidth,.flexibleHeight]
        source.loadHTMLString("<html style='background:#090909;color:white'><body>khor native overlay production adapter probe</body></html>", baseURL: nil)
        view.addSubview(source)
        site.frame = CGRect(x: 0,y: 100,width: view.bounds.width,height: view.bounds.height - 220)
        site.autoresizingMask = [.flexibleWidth,.flexibleHeight]
        site.loadHTMLString("<html><head><meta name='viewport' content='width=device-width,initial-scale=1'></head><body style='margin:0;color:white;background:linear-gradient(#25435c,#396548)'><button id='target' onclick='window.clicked=(window.clicked||0)+1' style='height:70px;width:100%'>Underlying website — tap must still reach here</button><div style='height:3000px'>Scrollable native webpage</div><script>window.identity='same-loaded-document';window.clicked=0;</script></body></html>", baseURL:nil)
        view.addSubview(site)
        field.frame = CGRect(x:20,y:view.bounds.height-110,width:view.bounds.width-40,height:44)
        field.autoresizingMask = [.flexibleWidth,.flexibleTopMargin]
        field.text = "Current address remains first responder"
        field.textColor = .white
        field.backgroundColor = .darkGray
        field.borderStyle = .roundedRect
        view.addSubview(field)
    }
    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        if !running { running=true; Task { await run() } }
    }
    func pause(_ seconds: Double) async { try? await Task.sleep(for: .seconds(seconds)) }
    func js(_ web: WKWebView,_ script: String) async -> Any? { try? await web.evaluateJavaScript(script) }
    func check(_ name:String,_ passed:Bool,_ details:Any = "") { checks.append(["name":name,"passed":passed,"details":details]); print("PROBE \(passed ? "PASS" : "FAIL") \(name) \(details)") }
    func descendants(_ root:UIView) -> [UIView] { [root] + root.subviews.flatMap { descendants($0) } }
    func menu() -> WKWebView? { descendants(view).compactMap{$0 as? WKWebView}.first { $0 !== source && $0 !== site } }
    func open(_ id:String,_ document:String? = nil) -> Bool {
        let document=document ?? html
        return document.withCString { content in id.withCString { lease in
            probe_show(Unmanaged.passUnretained(source).toOpaque(),content,lease,true,20,view.bounds.height*0.62,view.bounds.width-40,view.bounds.height,view.bounds.width)
        }}
    }
    func hide(_ id:String) { id.withCString { probe_hide($0) } }
    func events() -> [[String:Any]] {
        guard let ptr=probe_events() else{return []}; defer{probe_free(ptr)}
        return (try? JSONSerialization.jsonObject(with:Data(String(cString:ptr).utf8))) as? [[String:Any]] ?? []
    }
    func waitMenu() async -> WKWebView? {
        for _ in 0..<100 {
            if let menu=menu(), menu.frame.height>20, menu.superview?.isHidden==false { return menu }
            await pause(0.05)
        }
        return menu()
    }
    func run() async {
        await pause(1.0)
        for _ in 0..<200 {
            if (await js(site,"window.identity") as? String)=="same-loaded-document" { break }
            await pause(0.05)
        }
        check("site document ready before baseline",(await js(site,"window.identity") as? String)=="same-loaded-document")
        do {html=try String(contentsOf:Bundle.main.url(forResource:"menu",withExtension:"html")!,encoding:.utf8)} catch {check("production snapshot loaded",false,String(describing:error));finish();return}
        check("production snapshot loaded", html.contains("m:local") && html.contains("m:remote"),html.utf8.count)
        _=await js(site,"window.scrollTo(0,240)")
        await pause(0.4)
        let baseline=await js(site,"JSON.stringify({identity:window.identity,y:window.scrollY,clicked:window.clicked})") as? String ?? ""
        field.becomeFirstResponder()
        await pause(0.35)
        check("native field focused before overlay",field.isFirstResponder)
        check("production adapter accepted",open("lease-1"))
        guard let overlay=await waitMenu() else {check("menu native WK exists",false);finish();return}
        await pause(0.3)
        let frame=overlay.convert(overlay.bounds,to:view)
        check("native menu measured and visible",overlay.frame.height>20 && overlay.superview?.isHidden==false,NSCoder.string(for:frame))
        check("native menu above sibling website",view.subviews.last === overlay.superview)
        check("field retains keyboard responder",field.isFirstResponder)
        check("native menu cannot become first responder",!overlay.canBecomeFirstResponder)
        check("native menu explicit focus attempt fails",!overlay.becomeFirstResponder())
        check("original field retains responder after native focus attempt",field.isFirstResponder)
        check("menu geometry follows logical anchor",abs(frame.minX-20)<1 && abs(frame.maxY-view.bounds.height*0.62)<1 && abs(frame.width-(view.bounds.width-40))<1,NSCoder.string(for:frame))
        let style=await js(overlay,"JSON.stringify({scheme:getComputedStyle(document.documentElement).colorScheme,rows:document.querySelectorAll('[data-omni-item]').length,menu:document.querySelector('[data-omni-menu]').getBoundingClientRect().height,scroll:(()=>{let v=document.querySelector('[data-menu-viewport]');return {h:v.clientHeight,total:v.scrollHeight}})()})") as? String ?? ""
        check("snapshot paints actual 30-row dark menu",style.contains("\"scheme\":\"dark\"") && style.contains("\"rows\":30"),style)
        let scrollable=await js(overlay,"(()=>{let v=document.querySelector('[data-menu-viewport]');v.scrollTop=80;return v.scrollHeight>v.clientHeight&&v.scrollTop>0})()") as? Bool ?? false
        check("candidate viewport scrolls internally",scrollable)
        // A real native hit test must land in the original website outside
        // the menu. This checks actual UIView passthrough, not a DOM mock.
        let outside=CGPoint(x:view.bounds.width/2,y:150)
        let hit=view.hitTest(outside,with:nil)
        check("outside hit passes through to original webpage",hit?.isDescendant(of:site)==true,String(describing:hit))
        let taps=view.gestureRecognizers?.compactMap{$0 as? UITapGestureRecognizer} ?? []
        check("outside tap observer does not cancel or delay page touches",taps.contains{ !$0.cancelsTouchesInView && !$0.delaysTouchesBegan && !$0.delaysTouchesEnded })
        probe_clear()
        _=await js(overlay,"document.querySelector('[data-omni-item=\"m:local\"]').click()")
        await pause(0.15)
        check("valid native bridge pick targets owning face",events().contains{($0["event"] as? String)=="menu:pick" && ($0["owner"] as? String)=="main" && (($0["payload"] as? [String:Any])?["key"] as? String)=="m:local"},events())
        probe_clear()
        _=await js(overlay,"window.webkit.messageHandlers.khorMenu.postMessage({id:'lease-1',key:'not-a-current-candidate'});window.webkit.messageHandlers.khorMenu.postMessage({id:'old-lease',key:'m:remote'});window.webkit.messageHandlers.khorMenu.postMessage('malformed');")
        await pause(0.15)
        check("invalid key stale lease malformed bridge ignored",events().isEmpty,events())
        check("same lease update accepted",open("lease-1"))
        await pause(0.15)
        let sameScroll=await js(overlay,"document.querySelector('[data-menu-viewport]').scrollTop") as? Double ?? 0
        check("same snapshot keeps candidate scroll",sameScroll>0,sameScroll)
        hide("stale-lease")
        check("stale hide cannot close live menu",probe_active())
        let after=await js(site,"JSON.stringify({identity:window.identity,y:window.scrollY,clicked:window.clicked})") as? String ?? ""
        check("web document and scroll untouched while selecting",baseline==after,["before":baseline,"after":after])
        hide("lease-1")
        check("current hide removes native host",!probe_active() && overlay.superview?.superview == nil)
        check("host removal detaches outside observer",view.gestureRecognizers?.count ?? 0 == 0)
        check("replacement lease accepted",open("lease-2"))
        guard let second=await waitMenu() else {check("replacement native host",false);finish();return}
        probe_clear()
        _=await js(overlay,"try{window.webkit.messageHandlers.khorMenu.postMessage({id:'lease-1',key:'m:remote'})}catch(e){}")
        await pause(0.15)
        check("removed old view cannot publish to new owner",events().isEmpty,events())
        _=await js(second,"window.webkit.messageHandlers.khorMenu.postMessage({id:'lease-2',key:'m:remote'})")
        await pause(0.15)
        check("replacement valid pick accepted",events().contains{(($0["payload"] as? [String:Any])?["key"] as? String)=="m:remote"})
        check("field still responder after all updates",field.isFirstResponder)
        finish()
    }
    func finish() {
        let result:[String:Any]=["passed":checks.allSatisfy{($0["passed"] as? Bool)==true},"checks":checks,"limits":["Production Rust iOS adapter with AppHandle event collector only; no Tauri transport mocked native rendering","Native hit testing and responder verified; no injected physical UITouch or actual device finger feel","Dedicated simulator only; no user app, credentials, network or sessions used"]]
        let url=FileManager.default.urls(for:.documentDirectory,in:.userDomainMask)[0].appendingPathComponent("probe-results.json")
        try? JSONSerialization.data(withJSONObject:result,options:[.prettyPrinted,.sortedKeys]).write(to:url)
        print("PROBE_RESULTS \(url.path)")
    }
}
