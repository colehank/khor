"""Small W3C client for the pinned Appium/XCUITest simulator probe."""
import json
import os
import subprocess
import time
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
                while time.monotonic() < deadline:
                    for context in command('/contexts', method='GET'):
                        if not context.startswith('WEBVIEW'):
                            continue
                        try:
                            command('/context', {'name':context})
                            if evaluate('return !!window.__KHOR_LAB__'):
                                return
                        except Exception:
                            pass
                    time.sleep(1)
                raise ValueError('The actual Khor Lab WebView is not available for inspection')
            main_context()
            command('/timeouts', {'script':30000})
            seed = '''const done=arguments[arguments.length-1]; (async()=>{
              const invoke=window.__TAURI_INTERNALS__.invoke;
              const devices=await invoke('devices'); const me=devices.find(d=>d.me);
              if(!me)throw Error('Missing actual local backend');
              await invoke('tabs_open',{kind:'page',device:me.id,url:arguments[0]});
              localStorage.setItem('khor.place',JSON.stringify({landing:'browser',screen:'detail'}));
              return {device:me.id,realBackend:true};
            })().then(done,e=>done({error:String(e)}));'''
            backend = command('/execute/async', {'script':seed, 'args':[entry.replace('/app/', '/lab/example')]})
            if backend.get('error'):
                raise ValueError(backend['error'])
            evaluate('location.reload();return true')
            time.sleep(3)
            main_context()
            point = None
            for _ in range(60):
                point = evaluate('const el=document.querySelector("[data-session-deck] [data-deck-title]");if(!el)return null;const r=el.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}')
                if point:
                    break
                time.sleep(1)
            if not point:
                raise ValueError('The real browser address pill is missing')
            command('/context', {'name':'NATIVE_APP'})
            command('/execute/sync', {'script':'mobile: tap', 'args':[point]})
            visible = []
            for _ in range(30):
                visible = command('/elements', {'using':'class name','value':'XCUIElementTypeKeyboard'})
                if visible:
                    break
                time.sleep(.2)
            subprocess.run(['xcrun','simctl','io',simulator,'screenshot',str(report / 'ios-keyboard.png')], check=True, timeout=30)
            if not visible:
                raise ValueError('Tapping the actual browser pill did not show the iOS software keyboard')
            main_context()
            focused = evaluate('return document.activeElement?.matches("[data-page-address]") === true')
            if not focused:
                raise ValueError('The iOS keyboard is visible but the browser address does not own focus')
            return dict(keyboard_verified=True, real_backend=backend, automation='Appium 3.9.0 / XCUITest 12.12.4',
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
