// What real WebKit does with the page shim, printed as JSON for
// `scripts/check-wk.mjs` to read.
//
// **Why a separate binary at all.** `check-shim.mjs` runs the shim in a
// `vm` realm and answers 「the rules are written correctly」. It cannot
// answer whether `delete window.WebTransport` takes on WebKit, or which
// realms a `forMainFrameOnly: false` user script actually reaches — and
// both of those are WebKit's behaviour, not the shim's.
//
// It loads the same page twice, once **without** the shim. Without that
// arm, 「WebTransport is gone」 and 「this WebKit never had it」 are the
// same reading.
//
// Argument 1: the shim's path. Argument 2: the port the probe pages are
// served on (they must be http, not a file:// or data: origin, because
// that is what a borrowed page is).
import WebKit
import Foundation

let shim = try! String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
let port = CommandLine.arguments[2]

final class Waiter: NSObject, WKNavigationDelegate {
  var done = false
  var trouble: String? = nil
  func webView(_ w: WKWebView, didFinish n: WKNavigation!) { done = true }
  func webView(_ w: WKWebView, didFail n: WKNavigation!, withError e: Error) {
    trouble = "didFail: \(e.localizedDescription)"; done = true
  }
  func webView(_ w: WKWebView, didFailProvisionalNavigation n: WKNavigation!, withError e: Error) {
    trouble = "provisional: \(e.localizedDescription)"; done = true
  }
}

/// Five realms a page can reach, read synchronously: the page itself,
/// the two frames the parser made, and the two a script makes for
/// itself after load. `srcdocScripted` is read here **and** again after
/// its load event — those are two different documents and only the
/// second one is the srcdoc.
let REALMS = """
(() => {
  const kind = (w) => { try { return typeof w.WebTransport; } catch (e) { return "threw:" + e.name; } };
  const cred = (w) => { try { return typeof w.PublicKeyCredential; } catch (e) { return "threw:" + e.name; } };
  const both = (w) => kind(w) + " / " + cred(w);
  const parsed = (id) => {
    const el = document.getElementById(id);
    if (!el) return "no such frame";
    try { return both(el.contentWindow); } catch (e) { return "threw:" + e.name; }
  };
  const made = (fill) => {
    const f = document.createElement("iframe");
    if (fill) f.srcdoc = "<p>hi</p>";
    document.body.appendChild(f);
    try { return both(f.contentWindow); } catch (e) { return "threw:" + e.name; }
  };
  return JSON.stringify({
    top: both(window),
    blankParsed: parsed("blank"),
    kidParsed: parsed("kid"),
    blankScripted: made(false),
    srcdocScripted: made(true),
  });
})()
"""

let SRCDOC = """
const f = document.createElement("iframe");
f.srcdoc = "<p>hi</p>";
const loaded = new Promise((ok) => {
  f.onload = () => ok("load fired");
  setTimeout(() => ok("no load event in 4s"), 4000);
});
document.body.appendChild(f);
const when = await loaded;
const w = f.contentWindow;
let body = "?";
try { body = w.document.body.textContent.trim(); } catch (e) { body = "threw:" + e.name; }
return when + ", body=" + JSON.stringify(body) + ", " + (typeof w.WebTransport) + " / " + (typeof w.PublicKeyCredential);
"""

/// A worker global is not a document, so a user script is not injected
/// into it. Measured rather than assumed — it decides whether the
/// shim's note about workers is a fact or a guess.
let WORKER = """
const src = "onmessage = () => postMessage(typeof WebTransport);";
const w = new Worker(URL.createObjectURL(new Blob([src], { type: "text/javascript" })));
const said = await new Promise((ok) => {
  w.onmessage = (e) => ok(e.data);
  w.onerror = (e) => ok("worker error: " + (e.message || "?"));
  setTimeout(() => ok("worker never answered"), 4000);
  w.postMessage(1);
});
w.terminate();
return said;
"""

func spin(_ until: () -> Bool, seconds: Double) {
  let stop = Date(timeIntervalSinceNow: seconds)
  while !until() && Date() < stop {
    RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.05))
  }
}

func ask(_ web: WKWebView, _ js: String, async: Bool, args: [String: Any] = [:]) -> String {
  var answer = "no answer"
  var got = false
  if async {
    web.callAsyncJavaScript(js, arguments: args, in: nil, in: .page) { r in
      switch r {
      case .success(let v): answer = "\(v)"
      case .failure(let e): answer = "eval failed: \(e.localizedDescription)"
      }
      got = true
    }
  } else {
    web.evaluateJavaScript(js) { r, e in
      if let e = e { answer = "eval failed: \(e.localizedDescription)" }
      else { answer = (r as? String) ?? "not a string" }
      got = true
    }
  }
  spin({ got }, seconds: 10)
  return answer
}

func quoted(_ s: String) -> String {
  let d = try! JSONSerialization.data(withJSONObject: [s], options: [])
  var t = String(data: d, encoding: .utf8)!
  t.removeFirst()
  t.removeLast()
  return t
}

func probe(_ withShim: Bool) -> String {
  let cfg = WKWebViewConfiguration()
  if withShim {
    cfg.userContentController.addUserScript(
      WKUserScript(source: shim, injectionTime: .atDocumentStart, forMainFrameOnly: false))
  }
  let web = WKWebView(frame: CGRect(x: 0, y: 0, width: 400, height: 300), configuration: cfg)
  let w = Waiter()
  web.navigationDelegate = w
  web.load(URLRequest(url: URL(string: "http://127.0.0.1:\(port)/page.html")!))
  spin({ w.done }, seconds: 20)
  if let t = w.trouble { return "{\"loadFailed\":\(quoted(t))}" }
  if !w.done { return "{\"loadFailed\":\"never finished\"}" }
  // The parser's child frames load after the main document is done.
  spin({ false }, seconds: 1.5)
  let realms = ask(web, REALMS, async: false)
  let worker = ask(web, WORKER, async: true)
  let srcdoc = ask(web, SRCDOC, async: true)
  var out = realms
  if out.hasPrefix("{") && out.hasSuffix("}") {
    out.removeLast()
    out += ",\"worker\":\(quoted(worker)),\"srcdocAfterLoad\":\(quoted(srcdoc))}"
  }
  return out
}

/// A one-page PDF with a black square on it, built by CoreGraphics.
///
/// **Built rather than typed**, because a hand-written PDF has no xref
/// table and a viewer that refused it would make this gate red about
/// the fixture. The square is what the reading below looks for: an
/// empty page and a page that never rendered are both white.
func tinyPdf() -> String {
  let data = NSMutableData()
  var box = CGRect(x: 0, y: 0, width: 200, height: 200)
  guard let consumer = CGDataConsumer(data: data as CFMutableData),
    let ctx = CGContext(consumer: consumer, mediaBox: &box, nil)
  else { return "" }
  ctx.beginPDFPage(nil)
  ctx.setFillColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
  ctx.fill(CGRect(x: 20, y: 20, width: 160, height: 160))
  ctx.endPDFPage()
  ctx.closePDF()
  return (data as Data).base64EncodedString()
}

/// How dark the frame's box is, as a percentage of the pixels sampled.
///
/// **A snapshot, because nothing inside a PDF frame is readable from
/// JavaScript.** WebKit draws a PDF with its own viewer and hands the
/// page no document to query — `contentDocument` is null whether it
/// rendered or not. The only witness to「it drew」 is the picture, so
/// the picture is what is read.
func darkness(_ web: WKWebView) -> String {
  var answer = "no snapshot"
  var got = false
  let cfg = WKSnapshotConfiguration()
  web.takeSnapshot(with: cfg) { image, err in
    defer { got = true }
    if let err = err {
      answer = "snapshot failed: \(err.localizedDescription)"
      return
    }
    guard let image = image,
      let rep = NSBitmapImageRep(
        data: image.tiffRepresentation ?? Data())
    else {
      answer = "snapshot gave no bitmap"
      return
    }
    // The frame sits at 40,40 and is 300 square (pdf.html); sampled in
    // page points scaled to the bitmap, so a retina snapshot reads the
    // same box.
    let sx = Double(rep.pixelsWide) / 400.0
    let sy = Double(rep.pixelsHigh) / 400.0
    var dark = 0
    var seen = 0
    var y = 60
    while y < 320 {
      var x = 60
      while x < 320 {
        let px = Int(Double(x) * sx)
        let py = Int(Double(y) * sy)
        if px < rep.pixelsWide, py < rep.pixelsHigh,
          let c = rep.colorAt(x: px, y: py)
        {
          seen += 1
          if c.brightnessComponent < 0.5 { dark += 1 }
        }
        x += 4
      }
      y += 4
    }
    answer = seen == 0 ? "nothing sampled" : "\(dark * 100 / seen)"
  }
  spin({ got }, seconds: 15)
  return answer
}

/// A `blob:` PDF in an iframe — what khor's file view does (工作区批⑤).
///
/// No shim arm: this is not about the shim. It is about whether the
/// engine khor draws on has a PDF viewer at all, which decides whether
/// the face hands over bytes or falls back to the document's words
/// (`khor_node::preview::PDF_HERE`).
func pdfProbe() -> String {
  let web = WKWebView(frame: CGRect(x: 0, y: 0, width: 400, height: 400))
  let w = Waiter()
  web.navigationDelegate = w
  web.load(URLRequest(url: URL(string: "http://127.0.0.1:\(port)/pdf.html")!))
  spin({ w.done }, seconds: 20)
  if let t = w.trouble { return "{\"loadFailed\":\(quoted(t))}" }
  if !w.done { return "{\"loadFailed\":\"never finished\"}" }
  let b64 = tinyPdf()
  if b64.isEmpty { return "{\"loadFailed\":\"could not build a pdf\"}" }
  let said = ask(web, "return window.__put(b);", async: true, args: ["b": b64])
  // The viewer lays out and paints after the load event; a snapshot
  // taken in the same turn catches an empty frame.
  spin({ false }, seconds: 2.0)
  return "{\"frame\":\(quoted(said)),\"dark\":\(quoted(darkness(web)))}"
}

print(
  "{\"bare\":\(probe(false)),\"shimmed\":\(probe(true)),\"pdf\":\(pdfProbe())}")
