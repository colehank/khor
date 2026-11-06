"""Small W3C client for the pinned Appium/XCUITest simulator probe."""
import json
import os
from pathlib import Path
import subprocess
import time
import xml.etree.ElementTree as ET
from urllib.request import Request, urlopen
from urllib.error import HTTPError


def keyboard_probe(cache, simulator, identifier, entry, report):
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
            result = request('POST', '/session', {'capabilities': {'alwaysMatch': {
                'platformName':'iOS', 'appium:automationName':'XCUITest', 'appium:udid':simulator,
                'appium:bundleId':identifier, 'appium:noReset':True,
                'appium:connectHardwareKeyboard':False, 'appium:forceSimulatorSoftwareKeyboardPresence':True,
                'appium:newCommandTimeout':180, 'appium:webviewConnectTimeout':60000}}})
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
