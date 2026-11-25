"""Small W3C client for the pinned Appium/XCUITest simulator probe."""
from contextlib import suppress
import json
import os
from pathlib import Path
import subprocess
import time
import xml.etree.ElementTree as ET
from urllib.request import Request, urlopen
from urllib.error import HTTPError


def keyboard_probe(cache, simulator, identifier, entry, report, *, terminal_documents=None, paste_text=None):
    tools = cache / 'mobile-tools/appium-3.9.0-xcuitest-12.12.4'
    binary = tools / 'node_modules/.bin/appium'
    environment = dict(os.environ, APPIUM_HOME=str(tools / 'drivers'))
    if not binary.exists():
        subprocess.run(['npm', 'install', '--prefix', str(tools), '--no-audit', '--no-fund',
                        'appium@3.9.0', 'appium-xcuitest-driver@12.12.4'], check=True, timeout=900)
    if not (tools / 'drivers/node_modules/appium-xcuitest-driver/package.json').exists():
        subprocess.run([str(binary), 'driver', 'install', '--source', 'local',
                        str(tools / 'node_modules/appium-xcuitest-driver')], env=environment, check=True, timeout=900)
    origin = 'http://127.0.0.1:4727'
    def request(method, path, value=None, timeout=240):
        body = json.dumps(value).encode() if value is not None else None
        try:
            with urlopen(Request(origin + path, method=method, data=body,
                                 headers={'Content-Type':'application/json'}), timeout=timeout) as response:
                result = json.load(response)['value']
        except HTTPError as error:
            message = json.loads(error.read()).get('value', {}).get('message', str(error))
            raise ValueError(message[:4000]) from None
        if isinstance(result, dict) and result.get('error'):
            raise ValueError(result['message'])
        return result
    session = None
    with (report / 'appium.log').open('w') as log:
        server = subprocess.Popen([str(binary), '--address', '127.0.0.1', '--port', '4727',
                                   '--log-no-colors', '--log-level', 'info'], env=environment, stdout=log, stderr=subprocess.STDOUT)
        try:
            for _ in range(100):
                if server.poll() is not None:
                    raise ValueError('Appium stopped before accepting the simulator session')
                try:
                    if request('GET', '/status', timeout=2)['ready']:
                        break
                except OSError:
                    time.sleep(.2)
            capabilities = {
                'platformName':'iOS', 'appium:automationName':'XCUITest', 'appium:udid':simulator,
                'appium:bundleId':identifier, 'appium:noReset':True,
                'appium:connectHardwareKeyboard':False, 'appium:forceSimulatorSoftwareKeyboardPresence':True,
                'appium:newCommandTimeout':180, 'appium:webviewConnectTimeout':60000}
            if terminal_documents:
                # Keep the already booted hosted simulator. Opening its UI
                # would restart it while WebDriverAgent is being prepared.
                capabilities.update({'appium:isHeadless':True, 'appium:skipLogCapture':True,
                                     'appium:wdaLaunchTimeout':180000, 'appium:showXcodeLog':True})
            result = request('POST', '/session', {'capabilities': {'alwaysMatch': capabilities}},
                             timeout=600 if terminal_documents else 240)
            session = '/session/' + result['sessionId']
            def command(path, value=None, method='POST'):
                return request(method, session + path, value)
            def evaluate(script, args=None):
                return command('/execute/sync', {'script':script, 'args':args or []})
            def main_context():
                deadline = time.monotonic() + 60
                observed = {}
                while time.monotonic() < deadline:
                    contexts = command('/contexts', method='GET')
                    observed['contexts'] = contexts
                    for context in contexts:
                        if not context.startswith('WEBVIEW'):
                            continue
                        try:
                            command('/context', {'name':context})
                            document = evaluate('return {lab:!!window.__KHOR_LAB__,url:location.href,title:document.title,ready:document.readyState,text:document.body?.innerText.slice(0,400),scripts:[...document.scripts].map(s=>s.src)}')
                            observed[context] = document
                            if document.get('lab'):
                                return
                        except Exception as error:
                            observed[context] = str(error)
                    time.sleep(1)
                (report / 'webviews.json').write_text(json.dumps(observed, indent=2))
                command('/context', {'name':'NATIVE_APP'})
                (report / 'native-source.xml').write_text(command('/source', method='GET'))
                subprocess.run(['xcrun','simctl','io',simulator,'screenshot',str(report / 'ios-webview-failure.png')], check=True, timeout=30)
                raise ValueError('The actual Khor Lab WebView is not available for inspection: ' + json.dumps(observed))
            if terminal_documents:
                return terminal_keyboard_check(command, terminal_documents, simulator, report, paste_text=paste_text)
            main_context()
            command('/timeouts', {'script':30000})
            seed = '''const done=arguments[arguments.length-1]; (async()=>{
              const invoke=window.__TAURI_INTERNALS__.invoke;
              const devices=await invoke('devices'); const me=devices.find(d=>d.me);
              if(!me)throw Error('Missing actual local backend');
              const tab=await invoke('tabs_open',{kind:'page',device:me.id,url:arguments[0]});
              localStorage.setItem('khor.place',JSON.stringify({landing:'browser',screen:'detail'}));
              return {device:me.id,tab,realBackend:true};
            })().then(done,e=>done({error:String(e)}));'''
            backend = command('/execute/async', {'script':seed, 'args':[entry.replace('/app/', '/lab/example')]})
            if backend.get('error'):
                raise ValueError(backend['error'])
            evaluate('location.reload();return true')
            time.sleep(3)
            main_context()
            for _ in range(60):
                opened = evaluate("const row=document.querySelector('[data-side-row=\"'+CSS.escape(arguments[0])+'\"] [data-side-open]');if(!row)return false;row.click();return true", [backend['tab']])
                if opened:
                    break
                time.sleep(.2)
            else:
                raise ValueError('The actual browser list did not offer the newly opened backend tab')
            point = None
            for _ in range(60):
                point = evaluate('const el=document.querySelector("[data-session-deck] [data-deck-title]");if(!el)return null;const r=el.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}')
                if point:
                    break
                time.sleep(1)
            if not point:
                raise ValueError('The real browser address pill is missing')
            evaluate("window.__labNativeAddressClicks=[];window.addEventListener('click',event=>{window.__labNativeAddressClicks.push({title:!!event.target.closest('[data-deck-title]'),detail:event.detail})},true);return true")
            command('/context', {'name':'NATIVE_APP'})
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                source = command('/source', method='GET')
                buttons = [node for node in ET.fromstring(source).iter()
                           if node.get('type') == 'XCUIElementTypeButton' and node.get('visible') == 'true']
                hit = next((node for node in buttons
                            if float(node.get('x','0')) <= point['x'] <= float(node.get('x','0')) + float(node.get('width','0'))
                            and float(node.get('y','0')) <= point['y'] <= float(node.get('y','0')) + float(node.get('height','0'))), None)
                if hit is not None:
                    (report / 'native-address-ready.xml').write_text(source)
                    break
                time.sleep(.2)
            else:
                (report / 'native-address-missing.xml').write_text(source)
                raise ValueError('The native title button is not ready at the actual DOM address rectangle')
            command('/execute/sync', {'script':'mobile: tap', 'args':[point]})
            visible = []
            for _ in range(30):
                visible = command('/elements', {'using':'class name','value':'XCUIElementTypeKeyboard'})
                if visible:
                    break
                time.sleep(.2)
            subprocess.run(['xcrun','simctl','io',simulator,'screenshot',str(report / 'ios-keyboard.png')], check=True, timeout=30)
            if not visible:
                main_context()
                state = evaluate('return {clicks:window.__labNativeAddressClicks,focus:document.activeElement?.outerHTML.slice(0,400),fields:[...document.querySelectorAll("[data-page-address]")].map(input=>({html:input.outerHTML,visibility:getComputedStyle(input).visibility})),text:document.body.innerText.slice(-1000)}')
                (report / 'native-address-failure.json').write_text(json.dumps(state,indent=2))
                raise ValueError('Tapping the ready native browser title did not show the iOS software keyboard: ' + json.dumps(state))
            main_context()
            focused = evaluate('return document.activeElement?.matches("[data-page-address]") === true')
            if not focused:
                raise ValueError('The iOS keyboard is visible but the browser address does not own focus')
            css = Path.cwd() / 'apps/gui/src/styles/app.css'
            original = css.read_bytes()
            address = evaluate('window.__labNativeHotIdentity="same-document";return document.activeElement.value')
            try:
                css.write_bytes(original + b'\n:root { --lab-native-hot-proof: received; }\n')
                deadline = time.monotonic() + 20
                while time.monotonic() < deadline:
                    if evaluate('return getComputedStyle(document.documentElement).getPropertyValue("--lab-native-hot-proof").trim()') == 'received':
                        break
                    time.sleep(.2)
                else:
                    raise ValueError('The native WebView did not receive the source CSS hot update')
                if evaluate('return window.__labNativeHotIdentity') != 'same-document':
                    raise ValueError('The native hot update reloaded the document')
                if evaluate('return document.activeElement?.value') != address:
                    raise ValueError('The native hot update lost the active address editor')
            finally:
                css.write_bytes(original)
            return dict(keyboard_verified=True, native_hmr_verified=True, real_backend=backend, automation='Appium 3.9.0 / XCUITest 12.12.4',
                        session_browser_native_verified=False, human_accepted=False)
        finally:
            if session:
                try:
                    request('DELETE', session)
                except OSError:
                    pass
            server.terminate()
            try:
                server.wait(timeout=20)
            except subprocess.TimeoutExpired:
                server.kill()


def terminal_keyboard_check(command, documents, simulator, report, *, paste_text=None):
    """Tap the production terminal in WKWebView and observe UIKit's keyboard."""
    samples = {}
    serial = 0
    state = """const ready = location.pathname === '/app/' && window.__KHOR_LAB__?.scene === 'terminal';
      const box = document.querySelector('[data-terminal]');
      const term = box?.__xterm;
      if (!ready || !term || !term.rows)
        return {ready, page:location.pathname, document:document.readyState, rows:0, prompt:false};
      const r = box.getBoundingClientRect(), picker = document.querySelector('[data-files-panel]');
      const buffer = term.buffer.active;
      const prompt = buffer.getLine(buffer.baseY + buffer.cursorY)?.translateToString(true).trimEnd() === 'preview %';
      return {ready, rows:term.rows, cols:term.cols, height:r.height, bottom:r.bottom,
        screen:document.documentElement.clientHeight, visible:visualViewport.height,
        prompt, mode:term.textarea.inputMode, panel:picker&&picker.getBoundingClientRect().toJSON()};"""

    def wait_for(read, allowed, label, *, timeout=15):
        last = None
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            last = read()
            if allowed(last):
                return last
            time.sleep(.12)
        raise ValueError(f'{label}: {last!r}')

    def evaluate(script, *, timeout=15):
        nonlocal serial
        serial += 1
        request = documents / 'keyboard-command.json'
        pending = request.with_suffix('.tmp')
        pending.write_text(json.dumps({'serial': serial, 'script': script}))
        pending.replace(request)
        reply = documents / 'keyboard-observation.json'
        def read():
            return json.loads(reply.read_text()) if reply.is_file() else {}
        result = wait_for(read, lambda value: value.get('serial') == serial,
                          'WKWebView did not answer an observation', timeout=timeout)
        if 'error' in result:
            raise ValueError(result['error'])
        return result.get('value')

    def native_keyboard():
        return bool(command('/elements', {'using':'class name','value':'XCUIElementTypeKeyboard'}))

    def tap(script, keyboard=None):
        point = wait_for(lambda: evaluate(script), bool, 'No visible tap target')
        command('/execute/sync', {'script':'mobile: tap', 'args':[point]})
        if keyboard is not None:
            wait_for(native_keyboard, lambda visible: visible == keyboard, 'Wrong UIKit keyboard state')

    def button(selector):
        return f"""const button=document.querySelector({json.dumps(selector)});
          const box=button?.getBoundingClientRect();
          return box?.width&&box.height?{{x:box.x+box.width/2,y:box.y+box.height/2}}:null;"""

    def sizes():
        return evaluate("return await (await fetch('/api/__ai_preview/terminal-sizes',"
                        "{method:'POST',body:'{}'})).json();")

    def grid(value):
        return {key:value[key] for key in ('rows','cols','height','bottom')}

    try:
        # A successful JavaScript call can come from about:blank or a Lab
        # document whose fixture has not attached. Give startup its own
        # deadline; each subsequent interaction keeps its shorter deadline.
        started = time.monotonic()
        startup_timeout = 90
        opened_fixture = False

        def read_startup():
            nonlocal opened_fixture
            observation = evaluate(state, timeout=max(0, started + startup_timeout - time.monotonic()))
            samples['startup'] = observation
            if observation and observation['ready'] and not observation['rows'] and not opened_fixture:
                # The Lab's auto-open observer expires while a slow hosted
                # simulator is still loading the list. Open the visible row.
                opened_fixture = evaluate("""if (document.querySelector('[data-session-screen]')) return false;
                  const row = document.querySelector('[data-row="shell/layout-lab"] [data-row-open]');
                  if (!row?.getClientRects().length) return false;
                  row.click();
                  return true;""")
            return observation

        down = wait_for(read_startup, lambda s: s and s['ready'] and s['rows'] > 10 and s['prompt'],
                        'Terminal fixture never rendered its input prompt', timeout=startup_timeout)
        print(f'Terminal fixture ready {time.monotonic() - started:.1f}s after Appium was ready')
        samples['down'] = down
        tap("""const term=document.querySelector('[data-terminal]')?.__xterm;
          if(!term) return null;
          const screen=term.element.querySelector('.xterm-screen').getBoundingClientRect();
          const buffer=term.buffer.active;
          const line=buffer.cursorY+buffer.baseY;
          if (!buffer.getLine(line)?.translateToString(true).startsWith('preview %')) return null;
          const row=line-buffer.viewportY;
          return {x:screen.x+(buffer.cursorX+.5)*screen.width/term.cols,
                  y:screen.y+(row+.5)*screen.height/term.rows};""", keyboard=True)
        up = wait_for(lambda:evaluate(state), lambda s:s and s['rows']<down['rows'] and
                      s['height']<down['height'], 'UIKit keyboard covered an unfitted terminal')
        samples['keyboard'] = up
        resized = wait_for(sizes, lambda rows:rows and rows[-1]['rows']==up['rows'], 'PTY never received the smaller grid')
        attach = button('[data-term-key="attach"]')
        for close in ('cancel','toggle','send'):
            tap(attach, keyboard=False)
            panel = wait_for(lambda:evaluate(state), lambda s:s and s['panel'] and
                             s['visible']>=s['screen']-1, 'Picker failed to replace the system keyboard')
            samples[close] = panel
            expected_top = up['visible']
            if abs(panel['panel']['top']-expected_top)>2 or abs(panel['panel']['bottom']-panel['screen'])>2:
                raise ValueError(f'Picker is outside the keyboard band: {panel}')
            if grid(panel) != grid(up) or sizes() != resized:
                raise ValueError(f'Opening attachments resized the terminal: {panel}')
            if close == 'send':
                evaluate("""const input=document.querySelector('[data-files-input]');
                  const selected=new DataTransfer();
                  selected.items.add(new File(['keyboard'], 'keyboard.txt'));
                  input.files=selected.files;
                  input.dispatchEvent(new Event('change',{bubbles:true}));""")
            tap(attach if close == 'toggle' else button(f'[data-files-{close}]'), keyboard=True)
            returned = wait_for(lambda:evaluate(state), lambda s:s and not s['panel'] and
                                abs(s['visible']-up['visible'])<=1, 'Original keyboard did not return')
            if grid(returned) != grid(up) or sizes() != resized:
                raise ValueError(f'Closing attachments resized the terminal: {returned}')
            samples[f'after_{close}'] = returned
        evaluate("""window.__tabProbe = [];
          document.querySelector('[data-terminal]').__xterm.onData(value => window.__tabProbe.push(value));""")
        for key, expected in (('tab', '\t'), ('backtab', '\x1b[Z')):
            selector = f'[data-term-key="{key}"]'
            evaluate(f"document.querySelector({json.dumps(selector)}).scrollIntoView({{block:'nearest',inline:'center'}});")
            for side in ('left', 'right'):
                position = f"""const cap=document.querySelector({json.dumps(selector)});
                  const box=cap.getBoundingClientRect();
                  const row=cap.closest('[data-term-keys-scroll]').getBoundingClientRect();
                  const left=Math.max(box.left,row.left), right=Math.min(box.right,row.right);
                  if(right-left<12) return null;
                  return {{x:{'left+5' if side == 'left' else 'right-5'},y:box.y+box.height/2}};"""
                tap(position, keyboard=True)
                actual = evaluate('return window.__tabProbe.splice(0);')
                if actual != [expected]:
                    raise ValueError(f'{key} from the {side} edge sent {actual!r}')
        samples['tab_key_edges'] = True
        if paste_text is not None:
            evaluate("""await new Promise(resolve =>
              document.querySelector('[data-terminal]').__xterm.write('\x1b[?2004h', resolve));""")
            selector = '[data-term-key="paste"]'
            evaluate(f"document.querySelector({json.dumps(selector)}).scrollIntoView({{block:'nearest',inline:'center'}});")
            expected = f'\x1b[200~{paste_text}\x1b[201~'

            def copied_input():
                try:
                    buttons = command('/execute/sync', {'script':'mobile: alert', 'args':[{'action':'getButtons'}]})
                except ValueError:
                    buttons = []
                if 'Allow Paste' in buttons:
                    command('/execute/sync', {'script':'mobile: alert',
                                              'args':[{'action':'accept', 'buttonLabel':'Allow Paste'}]})
                return evaluate('return window.__tabProbe.slice();')

            for _ in range(2):
                tap(button(selector))
                wait_for(copied_input, lambda chunks: chunks == [expected],
                         'A single tap did not deliver the iOS clipboard to the terminal')
                evaluate('window.__tabProbe.length = 0;')
                if not native_keyboard():
                    raise ValueError('Pasting dismissed the terminal keyboard')
                if not evaluate(f"""const key=document.querySelector({json.dumps(selector)});
                  return !!key?.getBoundingClientRect().width && getComputedStyle(key).opacity === '1';"""):
                    raise ValueError('The paste key disappeared after tapping it')
            samples['native_paste_twice'] = True
        return dict(terminal_keyboard_verified=True, samples=samples, pty_resizes=resized,
                    automation='Appium 3.9.0 / XCUITest 12.12.4', human_accepted=False)
    except Exception:
        with suppress(subprocess.TimeoutExpired):
            subprocess.run(['xcrun','simctl','io',simulator,'screenshot',str(report/'terminal-keyboard.png')],
                           check=False, timeout=30)
        raise
    finally:
        (report / 'terminal-keyboard-samples.json').write_text(json.dumps(samples, indent=2))
