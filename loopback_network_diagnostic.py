import asyncio
import hashlib
import json
import os
from pathlib import Path
import subprocess

out = Path(os.environ['RUNNER_TEMP']) / 'loopback-diagnostic'
out.mkdir()
original = Path('apps/gui/core/tests/fixtures/webkit_loopback.swift').read_text()
source = original.replace('            store.proxyConfigurations = [proxy]', '''            if mode == "exclude-loopback" {
                proxy.excludedDomains = ["localhost", "127.0.0.1", "::1", "[::1]"]
            }
            store.proxyConfigurations = [proxy]''')
source = source.replace('finish(["error": error.localizedDescription])', 'finish(["error": error.localizedDescription, "domain": (error as NSError).domain, "code": (error as NSError).code, "details": String(describing: (error as NSError).userInfo)])')
(out / 'probe.swift').write_text(source)
subprocess.run(['xcrun', 'swiftc', '-swift-version', '5', str(out / 'probe.swift'), '-o', str(out / 'probe')], check=True)

async def main():
    records = []
    for host, family in [('127.0.0.1', __import__('socket').AF_INET), ('::1', __import__('socket').AF_INET6)]:
        calls = []
        async def serve(reader, writer):
            try:
                head = await asyncio.wait_for(reader.readuntil(b'\r\n\r\n'), 10)
                calls.append(head.decode().splitlines()[0])
                if head.startswith(b'CONNECT '):
                    writer.write(b'HTTP/1.1 200 Connection Established\r\n\r\n')
                    await writer.drain()
                    head = await asyncio.wait_for(reader.readuntil(b'\r\n\r\n'), 10)
                    calls.append(head.decode().splitlines()[0])
                writer.write(b'HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\nConnection: close\r\n\r\nBORROWED_EXIT')
                await writer.drain()
            except Exception as error:
                calls.append(type(error).__name__)
            finally:
                writer.close()
                await writer.wait_closed()
        origin = await asyncio.start_server(serve, host, 0, family=family)
        proxy = await asyncio.start_server(serve, '127.0.0.1', 0)
        port = origin.sockets[0].getsockname()[1]
        proxy_port = proxy.sockets[0].getsockname()[1]
        authority = f'[{host}]' if ':' in host else host
        url = f'http://{authority}:{port}/'
        for mode in ['direct', 'default', 'explicit', 'exclude-loopback']:
            calls.clear()
            process = await asyncio.create_subprocess_exec(str(out / 'probe'), mode, url, str(proxy_port), stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
            try:
                stdout, stderr = await asyncio.wait_for(process.communicate(), 30)
                result = json.loads(stdout)
            except Exception as error:
                process.kill()
                stdout, stderr = await process.communicate()
                result = {'diagnostic_error': str(error)}
            record = dict(host=host, mode=mode, url=url, result=result, received=list(calls), stderr=stderr.decode()[-2000:])
            records.append(record)
            print(json.dumps(record), flush=True)
        origin.close()
        proxy.close()
        await origin.wait_closed()
        await proxy.wait_closed()
    (out / 'results.json').write_text(json.dumps(dict(original_swift_sha256=hashlib.sha256(original.encode()).hexdigest(), records=records), indent=2))
asyncio.run(main())
