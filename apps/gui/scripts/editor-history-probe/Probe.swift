import AppKit
import WebKit

let app=NSApplication.shared
app.setActivationPolicy(.regular)
let win=NSWindow(contentRect:NSRect(x:100,y:100,width:900,height:700),styleMask:[.titled,.closable,.resizable],backing:.buffered,defer:false)
let web=WKWebView(frame:win.contentView!.bounds)
win.contentView=web
win.makeKeyAndOrderFront(nil)
app.activate(ignoringOtherApps:true)
final class EditTarget:NSObject{
 var calls=0
 @objc func undo(_ sender:Any?){calls += 1;khor_native_edit(Unmanaged.passUnretained(web).toOpaque(),false)}
 @objc func redo(_ sender:Any?){calls += 1;khor_native_edit(Unmanaged.passUnretained(web).toOpaque(),true)}
}
let target=EditTarget(),menu=NSMenu(),root=NSMenuItem(),edit=NSMenu(title:"Edit")
root.submenu=edit;menu.addItem(root);edit.autoenablesItems=false
for (title,selector,flags) in [("Undo",#selector(EditTarget.undo(_:)),NSEvent.ModifierFlags.command),("Redo",#selector(EditTarget.redo(_:)),NSEvent.ModifierFlags([.command,.shift]))]{let item=NSMenuItem(title:title,action:selector,keyEquivalent:"z");item.target=target;item.keyEquivalentModifierMask=flags;edit.addItem(item)}
app.mainMenu=menu
web.load(URLRequest(url:URL(string:CommandLine.arguments[1])!))
var checks:[[String:Any]]=[]
func check(_ label:String,_ yes:Bool){checks.append(["label":label,"passed":yes]);print(yes ? "PASS" : "FAIL",label)}
func pause(_ ms:UInt64=250)async{try? await Task.sleep(nanoseconds:ms*1_000_000)}
@MainActor func js(_ code:String,_ view:WKWebView=web)async throws->Any?{try await view.evaluateJavaScript(code)}
@MainActor func text()async throws->String{try await js("document.querySelector('[data-code-view]').__view.state.doc.toString()") as? String ?? ""}
func keys(_ code:CGKeyCode,_ flags:CGEventFlags=[]){for down in [true,false]{let event=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:down)!;event.flags=flags;event.postToPid(getpid())}}
@MainActor func typeASD()async{for code:CGKeyCode in [0,1,2]{keys(code);await pause(80)};await pause()}
Task{@MainActor in
 do{
  for _ in 0..<80{if (try? await js("!!document.querySelector('[data-code-view]')?.__view")) as? Bool == true {break};await pause(100)}
  await pause(2000)
  // A command-line AppKit process can finish launching after the first
  // makeKey call. Establish the native responder after loading, then
  // measure it explicitly rather than accepting a background no-op.
  win.makeKeyAndOrderFront(nil);app.activate(ignoringOtherApps:true);win.makeFirstResponder(web)
  for _ in 0..<30{if win.isKeyWindow && app.isActive{break};await pause(100)}
  check("probe owns the native key window",win.isKeyWindow && app.isActive)
  _=try await js("window.nativeEvents=[];document.addEventListener('beforeinput',e=>nativeEvents.push(e.inputType));(()=>{let v=document.querySelector('[data-code-view]').__view;v.focus();v.dispatch({changes:{from:v.state.doc.length,insert:' abc'},userEvent:'input.type'});})()")
  await pause();let typed=try await text();check("CM starts with an undo entry",typed=="hello abc")
  _=app.sendAction(NSSelectorFromString("undo:"),to:nil,from:nil);await pause();check("RED control: native responder undo does not reach CM",try await text()==typed)
  target.undo(nil);await pause();check("production Rust menu undo reaches CM",try await text()=="hello")
  target.redo(nil);await pause();check("production Rust menu redo reaches CM",try await text()==typed)
  target.calls=0;keys(6,[.maskCommand]);await pause();check("actual Command Z undoes exactly once",try await text()=="hello")
  keys(6,[.maskCommand,.maskShift]);await pause();check("actual Command Shift Z redoes exactly once",try await text()==typed);check("DOM-handled key does not repeat through menu",target.calls==0)

  _=try await js("(()=>{let e=document.createElement('textarea');e.id='ordinary';e.style.cssText='position:fixed;top:30px;left:30px;width:200px;height:60px';document.body.append(e);e.focus()})()")
  win.makeFirstResponder(web);_=try await js("document.execCommand('insertText',false,'asd')");await pause();check("ordinary HTML input receives typing",try await js("document.querySelector('#ordinary').value") as? String=="asd")
  target.undo(nil);await pause();check("ordinary HTML input uses native undo fallback",try await js("document.querySelector('#ordinary').value") as? String=="")
  target.redo(nil);await pause();check("ordinary HTML input uses native redo fallback",try await js("document.querySelector('#ordinary').value") as? String=="asd")

  let remote=WKWebView(frame:NSRect(x:450,y:150,width:400,height:300));web.addSubview(remote)
  remote.loadHTMLString("<textarea id='r' autofocus></textarea>",baseURL:nil)
  for _ in 0..<30{if (try? await js("!!document.querySelector('#r')",remote)) as? Bool == true{break};await pause(100)}
  _=try await js("document.querySelector('#r').focus()",remote);win.makeFirstResponder(remote);_=try await js("document.execCommand('insertText',false,'asd')",remote);await pause()
  check("nested remote input receives typing",try await js("document.querySelector('#r').value",remote) as? String=="asd")
  let before=try await text();target.undo(nil);await pause();check("nested remote input keeps its native undo",try await js("document.querySelector('#r').value",remote) as? String=="");check("remote focus never edits trusted CM",try await text()==before)
  remote.removeFromSuperview()

  let field=NSTextField(frame:NSRect(x:350,y:300,width:250,height:30));web.addSubview(field);win.makeFirstResponder(field);(win.firstResponder as? NSTextView)?.allowsUndo=true;(win.firstResponder as? NSTextView)?.insertText("asd",replacementRange:NSRange(location:NSNotFound,length:0));await pause();check("native text field receives typing",(win.firstResponder as? NSTextView)?.string=="asd")
  target.undo(nil);await pause();check("native text field keeps its responder undo",(win.firstResponder as? NSTextView)?.string=="")
  (win.firstResponder as? NSTextView)?.insertText("stays",replacementRange:NSRange(location:NSNotFound,length:0));await pause()
  _=try await js("document.querySelector('#ordinary').focus()");win.makeFirstResponder(web);await pause()
  target.undo(nil);win.makeFirstResponder(field);await pause()
  check("late native fallback cannot undo newly focused field",(win.firstResponder as? NSTextView)?.string=="stays")
 }catch{check("probe error: \(error)",false)}
 let data=try! JSONSerialization.data(withJSONObject:["passed":checks.allSatisfy{$0["passed"] as? Bool == true},"checks":checks],options:[.prettyPrinted,.sortedKeys]);try! data.write(to:URL(fileURLWithPath:CommandLine.arguments[2]));print(String(data:data,encoding:.utf8)!)
 app.terminate(nil)
}
DispatchQueue.main.asyncAfter(deadline:.now()+30){print("probe timed out");app.terminate(nil)}
app.run()
