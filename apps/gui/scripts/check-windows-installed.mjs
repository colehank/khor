// Called by the Windows installer check after opening the packaged app
// and creating a local cmd.exe session in an isolated store.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import { once } from "node:events";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { basename, join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { chromium } from "playwright-core";

const [, , port, title, ready, workspace, exitMachine, peerTitle, peerReady, ...checks] = process.argv;
const nativeWindow = checks.find(check => check.startsWith("--native-drag="))?.split("=")[1];
const shellSelection = checks.find(check => check.startsWith("--shell-selection="))?.split("=")[1];
const parity = checks.find(check => check.startsWith("--desktop-parity="))?.split("=")[1];
const endpoint = `http://127.0.0.1:${port}`;
function isAppPage(url) {
  try {
    const parsed = new URL(url);
    return ["http:", "https:"].includes(parsed.protocol) && parsed.hostname === "tauri.localhost";
  } catch {
    return false;
  }
}

async function waitForWindow() {
  const until = Date.now() + 60_000;
  let last = "no response";
  while (Date.now() < until) {
    try {
      const response = await fetch(`${endpoint}/json/list`, {signal: AbortSignal.timeout(1000)});
      if (response.ok) {
        const targets = await response.json();
        if (targets.some(target => target.type === "page" && isAppPage(target.url))) return;
        last = targets.map(({type, url}) => ({type, url: url?.split(/[?#]/, 1)[0]}));
      } else {
        last = `HTTP ${response.status}`;
      }
    } catch (error) {
      last = error.cause?.code ?? error.message;
    }
    await delay(200);
  }
  throw new Error(`WebView2 did not report the bundled App page on port ${port}: ${JSON.stringify(last)}`);
}

function command(page, name, args, timeout = 15_000) {
  return page.evaluate(async ({name, args, timeout}) => {
    const transport = window.__KHOR_WINDOW_TRANSPORT__;
    const response = await fetch(`${transport.url}/${name}`, {
      method: "POST",
      headers: {"Content-Type": "application/json", "X-Khor-Window-Token": transport.token},
      body: JSON.stringify(args),
      signal: AbortSignal.timeout(timeout),
    });
    if (!response.ok) throw new Error(`${name} returned ${response.status}: ${await response.text()}`);
    return response.json();
  }, {name, args, timeout});
}

async function checkDesktopParity(page) {
  const [extension, expectedScale] = parity.split(",");
  const scales = await page.evaluate(() => {
    const native = window.__KHOR_SYSTEM_TEXT_SCALE__;
    const wear = () => window.dispatchEvent(new Event("khor:system-text-size"));
    const reading = () => ({
      ui: Number(document.documentElement.style.getPropertyValue("--ui-scale")),
      terminal: Number.parseFloat(document.documentElement.style.getPropertyValue("--text-term")),
    });
    const initial = reading();
    try {
      window.__KHOR_SYSTEM_TEXT_SCALE__ = 1.5;
      wear();
      return {native, initial, enlarged: reading()};
    } finally {
      window.__KHOR_SYSTEM_TEXT_SCALE__ = native;
      wear();
    }
  });
  assert.equal(scales.native, Number(expectedScale), "the installed App must read the real Windows text setting");
  assert.deepEqual(scales.initial, {ui: scales.native, terminal: Math.max(10, Math.round(14 * scales.native))});
  assert.deepEqual(scales.enlarged, {ui: 1.5, terminal: 21}, "both surfaces must respond to the native text-scale event");

  const path = join(workspace, `Original 资料.${extension}`);
  const bytes = "KHOR_WINDOWS_ORIGINAL_FILE_资料";
  const id = randomUUID();
  await writeFile(path, bytes);
  assert.equal(await command(page, "file_handoff_supported", {}), true);
  assert.equal(await command(page, "file_preview_supported", {kind: "document", path}), false);
  await command(page, "file_handoff_start", {id, machine: exitMachine, path});
  for (const deadline = Date.now() + 15_000; ;) {
    const state = await command(page, "file_handoff_state", {id});
    assert.equal(state.error, null);
    assert.equal(state.cancelled, false);
    if (state.ready) break;
    assert(Date.now() < deadline, "the paired file never reached the local cache");
    await delay(100);
  }
  assert.equal(await command(page, "file_handoff_open", {id}), true);
  const receipt = join(workspace, "file-handoff-receipt.txt");
  let opened;
  for (const deadline = Date.now() + 15_000; ; ) {
    const record = await readFile(receipt, "utf8").catch(error => {
      if (error.code !== "ENOENT") throw error;
      return "";
    });
    if (record) {
      assert(record.endsWith(bytes), `The associated application received unexpected data: ${record.split("\n").at(-1)}`);
      opened = record.slice(0, -bytes.length).trim();
      break;
    }
    assert(Date.now() < deadline, "Windows did not open the cached original in the associated app");
    await delay(100);
  }
  assert.equal(basename(opened), basename(path));
  assert.notEqual(opened.toLowerCase(), path.toLowerCase(), "the viewer must receive the local copy");
  console.log("Windows system text scaling and opening the paired original in a local app passed.");

  const dimensions = () => page.evaluate(() => [innerWidth, innerHeight]);
  const before = await dimensions();
  try {
    await command(page, "set_window_appearance", {material: "frost"});
    assert.deepEqual(await dimensions(), before, "changing window material must keep the viewport size");
  } finally {
    await command(page, "set_window_appearance", {material: "solid"});
  }
}

async function checkBrowser(page, navigation) {
  const requests = new Map();
  const fixture = createServer((request, response) => {
    requests.set(request.url, request.headers.connection);
    response.writeHead(200, {"Content-Type": "text/html; charset=utf-8", "Connection": "close"});
    response.end("<!doctype html><title>Khor installer browser</title>");
  });
  fixture.listen(0, "127.0.0.1");
  await once(fixture, "listening");
  try {
    const base = `http://127.0.0.1:${fixture.address().port}`;
    const view = await page.evaluate(() => innerHeight);
    const agent = await page.evaluate(() => navigator.userAgent);
    const at = {x: 280, y: 75, w: 540, h: 400, view};
    const tabs = [
      {tab: "windows-installer-direct", machine: "", path: "/direct"},
      {tab: "windows-installer-borrow", machine: exitMachine, path: "/borrow"},
    ];
    for (const {tab, machine, path} of tabs) {
      await command(page, "open_page", {tab, machine, url: base + path, agent, at}, 90_000);
      const until = Date.now() + 15_000;
      let state;
      do {
        state = await command(page, "page_state", {tab});
        if (requests.has(path) && ["finished", "settled"].includes(state?.phase)
            && navigation.some(event => event.tab === tab)) break;
        await delay(200);
      } while (Date.now() < until);
      assert(requests.has(path), `The native WebView must request ${path}`);
      assert(["finished", "settled"].includes(state?.phase), `The ${path} WebView did not finish: ${JSON.stringify(state)}`);
      assert.equal(state.proxied, !!machine);
      assert(navigation.some(event => event.tab === tab), `The main WebView missed navigation events for ${tab}`);
    }
    // Both nodes are on the runner. A direct WebView would reach the same
    // address, but only the Khor tunnel's HTTP proxy rewrites Connection.
    assert.equal(requests.get("/direct"), "keep-alive");
    assert.equal(requests.get("/borrow"), "close");
    for (const {tab} of tabs) {
      await command(page, "close_page", {tab});
      assert.equal(await command(page, "page_state", {tab}), null, "Closing retires the native child WebView");
    }
  } finally {
    fixture.close();
  }
}

async function findWindowPage(browser) {
  const until = Date.now() + 15_000;
  let pages = [];
  do {
    pages = browser.contexts().flatMap(context => context.pages());
    const main = pages.find(page => isAppPage(page.url()));
    if (main) return main;
    await delay(200);
  } while (Date.now() < until);
  const targets = pages.map(page => page.url().split(/[?#]/, 1)[0]);
  throw new Error(`WebView2 connected but did not attach the App page: ${JSON.stringify(targets)}`);
}

async function createSession(page, launchId, title, expectedLabel) {
  await page.locator("[data-side-head] [data-pane-new]:visible").click();
  await page.locator('[data-new-item="new"]:visible').click();
  await page.locator("[data-new-session-dir]").fill(workspace);
  const launch = page.locator("[data-new-session-launch] [role=combobox]");
  await launch.click();
  await page.locator(`[data-choice-option=${JSON.stringify(launchId)}]`).click();
  if (expectedLabel) assert.equal(await launch.inputValue(), expectedLabel);
  await page.locator("[data-new-session-name]").fill(title);
  await page.locator("[data-new-session-create]").click();
  await page.waitForFunction(() => {
    const error = document.querySelector("[data-new-session-error]");
    if (error) throw new Error(error.textContent);
    return !document.querySelector("[data-new-session-page]");
  });
}

async function checkAcp(page) {
  await createSession(page, "agent:installer-acp", "Khor installer ACP");
  const chat = page.locator("[data-detail-pane]:visible");
  const input = chat.locator("[data-chat-input]:visible");
  for (const [message, answer] of [["Windows ACP round trip", "go"], ["Continue the same conversation", "stop"]]) {
    await input.fill(message);
    await chat.locator("[data-chat-send]:visible").click();
    await chat.locator(`[data-ask-active] [data-ask-option=${JSON.stringify(answer)}]:visible`).click();
    await chat.getByText(`picked:${answer}`, {exact: false}).waitFor();
    await chat.locator("[data-chat-stop]:visible").waitFor({state: "hidden"});
  }
  await chat.getByText("picked:go", {exact: false}).waitFor();
  console.log("The installed Windows App created an ACP session and completed two turns with different permission answers.");
}

async function dropKhorPath(terminal, path, machine = null) {
  await terminal.evaluate((element, {path, machine}) => {
    const data = new DataTransfer();
    data.setData("application/x-khor-path", JSON.stringify({machine, path}));
    element.dispatchEvent(new DragEvent("drop", {bubbles: true, cancelable: true, dataTransfer: data}));
  }, {path, machine});
}

async function sendAttachment(page, terminal, path) {
  const panel = page.locator("[data-files-panel]:visible");
  await panel.getByText(basename(path), {exact: true}).waitFor();
  await panel.locator("[data-files-send]").click();
  await panel.waitFor({state: "hidden"});
  await terminal.evaluate(element => element.__xterm.focus());
  await page.keyboard.press("Enter");
}

async function checkTerminalAttachment(page, terminal, shell, readTerminal) {
  const path = join(workspace, "John's attachment 资料.txt");
  const marker = `KHOR_${shell.toUpperCase()}_ATTACHMENT_CONTENT`;
  await writeFile(path, marker + "\r\n");
  await terminal.evaluate(element => element.__xterm.focus());
  await page.keyboard.type(shell === "cmd" ? "type " : "Get-Content -LiteralPath ");
  if (nativeWindow) {
    const point = await terminal.evaluate(element => {
      const bounds = element.getBoundingClientRect();
      return [(bounds.left + bounds.width / 2) * devicePixelRatio,
        (bounds.top + bounds.height / 2) * devicePixelRatio].map(Math.round);
    });
    const drag = fileURLToPath(new URL("../../../scripts/check-windows-drag.ps1", import.meta.url));
    const gesture = await promisify(execFile)("powershell.exe", ["-NoProfile", "-NonInteractive", "-Sta",
      "-ExecutionPolicy", "Bypass", "-File", drag, nativeWindow, ...point.map(String), path],
    {timeout: 18_000, windowsHide: true});
    console.log(gesture.stdout.trim());
  } else {
    await dropKhorPath(terminal, path);
  }
  await sendAttachment(page, terminal, path);
  await readTerminal(marker);
  console.log(`The installed Windows App opened the ${nativeWindow ? "native drop" : "attachment"} in ${shell}.`);
}

async function checkPeerAttachment(page, terminal, shell, readTerminal) {
  const file = "Agent's notes 资料.txt";
  const path = join(workspace, file);
  const destination = join(workspace, "Attachment receive");
  const marker = "KHOR_PAIRED_NODE_ATTACHMENT_CONTENT";
  const bytes = marker + "\r\n";
  await mkdir(destination);
  await writeFile(path, bytes);
  await command(page, "set_receive_dir", {machine: "installer-client", dir: destination});
  await terminal.evaluate(element => element.__xterm.focus());
  await page.keyboard.type(shell === "cmd" ? "type " : "Get-Content -LiteralPath ");
  await dropKhorPath(terminal, path, exitMachine);
  await sendAttachment(page, terminal, path);
  assert.equal(await readFile(join(destination, file), "utf8"), bytes);
  await readTerminal(marker);
  console.log(`The installed Windows App transferred the selected attachment between paired nodes and opened it in ${shell}.`);
}

await waitForWindow();
const browser = await chromium.connectOverCDP(endpoint, {timeout: 15_000});
let page;
const errors = [];
const reads = [];
const shellRequests = [];
try {
  page = await findWindowPage(browser);
  page.on("pageerror", error => errors.push(error.message));
  page.on("request", request => {
    if (new URL(request.url()).pathname === "/open_shell" && request.method() === "POST") {
      const {shell, title} = request.postDataJSON();
      shellRequests.push({shell, title});
    }
  });
  page.on("response", response => {
    const route = new URL(response.url()).pathname;
    if (["/lock_state", "/read_text"].includes(route)) reads.push(`${route} ${response.status()}`);
  });
  page.setDefaultTimeout(20_000);
  const windowButtons = page.locator("[data-window-controls], [data-window-control]");

  const navigation = [];
  await page.exposeFunction("recordInstallerNavigation", (event) => navigation.push(event));
  const callback = await page.evaluate(() => window.__TAURI_INTERNALS__.transformCallback(
    event => { void window.recordInstallerNavigation(event.payload); },
  ));
  const eventId = await command(page, "window_event_listen", {
    event: "page:navigation", target: {kind: "Any"}, handler: callback,
  });
  assert(Number.isInteger(eventId), "The Windows event bridge must return its listener ID");
  assert(Array.isArray(await command(page, "ssh_config_hosts")), "The Windows App must answer the SSH host picker");
  console.log("Windows event registration and SSH host lookup answered.");

  const sessions = page.locator('[data-landing="sessions"]:visible').first();
  await sessions.waitFor();
  if (await sessions.getAttribute("aria-current") !== "page") await sessions.click();
  await page.locator('[data-pane-filter]:visible').click();
  const conversations = page.locator('[data-filter-option="talk:yes"]');
  await conversations.waitFor();
  assert.equal(await conversations.getAttribute("aria-checked"), "false");
  assert.equal(await page.locator('[data-arrange-option="recent"]').getAttribute("aria-checked"), "true");
  await page.keyboard.press("Escape");

  const shellRow = (name) => page.locator(`[data-list] [data-kind="shell"][data-title=${JSON.stringify(name)}] [data-row-open]:visible`);
  await shellRow(title).click();
  const terminal = page.locator("[data-terminal]:visible");
  await terminal.waitFor();

  const readTerminal = marker => page.waitForFunction(value => {
    const terminal = [...document.querySelectorAll("[data-terminal]")].find(node =>
      node.getClientRects().length && getComputedStyle(node).visibility === "visible");
    const buffer = terminal?.__xterm?.buffer.active;
    if (!buffer) return false;
    for (let index = Math.max(0, buffer.length - 48); index < buffer.length; index++) {
      if (buffer.getLine(index)?.translateToString(true).trim() === value) return true;
    }
    return false;
  }, marker);
  await readTerminal(ready);
  await terminal.evaluate(element => element.__xterm.focus());
  await page.keyboard.type("echo KHOR_TERMINAL_INPUT_^OK");
  await page.keyboard.press("Enter");
  await readTerminal("KHOR_TERMINAL_INPUT_OK");
  assert.equal(await windowButtons.count(), 0, "Session panes inherit the native caption");
  console.log("Windows terminal accepted keyboard input and displayed the result.");
  await page.keyboard.type("exit");
  await page.keyboard.press("Enter");

  await shellRow(peerTitle).click();
  await readTerminal(peerReady);
  await terminal.evaluate(element => element.__xterm.focus());
  await page.keyboard.type("echo KHOR_PEER_TERMINAL_INPUT_^OK");
  await page.keyboard.press("Enter");
  await readTerminal("KHOR_PEER_TERMINAL_INPUT_OK");
  console.log("The installed Windows App opened the terminal on its paired node and received command output.");
  await page.keyboard.type("exit");
  await page.keyboard.press("Enter");

  const shells = [
    {id: "cmd", label: "CMD", commandLine: "echo KHOR_SELECTED_^CMD", output: "KHOR_SELECTED_CMD"},
    {id: "powershell", label: "PowerShell", commandLine: "Write-Output ('KHOR_' + 'SELECTED_POWERSHELL')", output: "KHOR_SELECTED_POWERSHELL"},
  ].filter(shell => !shellSelection || shell.id === shellSelection);
  for (const {id, label, commandLine, output} of shells) {
    await createSession(page, `shell:${id}`, `Khor installer ${label}`, label);
    await terminal.waitFor();
    await terminal.locator("[data-term-opening]").waitFor({state: "hidden"});
    await terminal.evaluate(element => element.__xterm.focus());
    await page.keyboard.type(commandLine);
    await page.keyboard.press("Enter");
    await readTerminal(output);
    console.log(`The new-session wizard opened ${label} and an interactive command returned its expected output.`);
    if (!checks.includes("--legacy-shell-ui")) {
      await checkTerminalAttachment(page, terminal, id, readTerminal);
      if (id === shells[0].id) await checkPeerAttachment(page, terminal, id, readTerminal);
    }
    await page.keyboard.type("exit");
    await page.keyboard.press("Enter");
  }

  if (checks.includes("--acp")) await checkAcp(page);
  await checkBrowser(page, navigation);
  console.log("Windows browser loaded both direct and borrowed localhost pages.");

  await command(page, "ws_add", {machine: "", path: workspace});
  await page.locator('[data-landing="workspaces"]:visible').first().click();
  await page.locator("[data-ws-row]:visible").filter({hasText: basename(workspace)}).click();
  const fileTree = page.locator("[data-file-tree]:visible");
  await fileTree.getByRole("button", {name: "资料", exact: true}).click();
  await fileTree.getByRole("button", {name: "readme.md", exact: true}).click();
  await page.locator('[data-file-face-toggle="source"]:visible').click();
  const preview = page.getByText("Khor installation workspace", {exact: true});
  await preview.waitFor();
  assert.equal(await windowButtons.count(), 0, "Workspace panes inherit the native caption");
  console.log("Windows workspace opened a file and displayed its preview.");
  if (parity) await checkDesktopParity(page);

  await command(page, "window_event_unlisten", {eventId});
  await page.evaluate(id => window.__TAURI_INTERNALS__.unregisterCallback(id), callback);

  // The installer checker closes the native window after CDP disconnects.
  console.log("Installed Windows App verified a live terminal, workspace file and child browser.");
} catch (error) {
  if (page && !page.isClosed()) {
    const state = await page.evaluate(() => ({
      size: [innerWidth, innerHeight],
      ready: document.readyState,
      lockWaiting: !!document.querySelector("[data-lock-wait]"),
      lockShown: !!document.querySelector("[data-lock-screen]"),
      lockUnavailable: !!document.querySelector("[data-lock-unavailable]"),
      faults: Array.from(document.querySelectorAll("[data-fault]"), node => node.getAttribute("data-fault")),
      landings: Array.from(document.querySelectorAll("[data-landing]"), node => node.getAttribute("data-landing")),
      fileFaces: Array.from(document.querySelectorAll("[data-file-face-toggle]"), node => node.getAttribute("data-file-face-toggle")),
      editorReady: !!document.querySelector("[data-file-view]"),
      preview: document.querySelector("[data-file-preview]")?.textContent?.slice(0, 400),
    })).catch(cause => ({error: cause.message}));
    const visibleTerminal = page.locator("[data-terminal]:visible").first();
    if (await visibleTerminal.count()) {
      const lines = await visibleTerminal.evaluate(element => {
        const buffer = element.__xterm?.buffer.active;
        if (!buffer) return [];
        return Array.from({length: buffer.length}, (_, index) => buffer.getLine(index)?.translateToString(true))
          .filter(line => line?.trim()).slice(-12);
      }).catch(cause => [cause.message]);
      console.error("Windows terminal output:", JSON.stringify(lines));
    }
    console.error("Windows App state:", JSON.stringify(state), "page errors:", errors.slice(-5), "reads:", reads.slice(-10), "shell requests:", shellRequests);
    if (state.lockWaiting || state.lockUnavailable) {
      const lockOn = await command(page, "lock_state", {}, 5000)
        .then(facts => facts.on, cause => cause.message);
      console.error("The installed App's lock read reports:", lockOn);
    }
    if (state.fileFaces?.length) {
      const read = await command(page, "read_text", {machine: "", path: join(workspace, "资料", "readme.md")}, 5000)
        .then(file => ({hasHeading: file.text.includes("Khor installation workspace"), binary: file.binary}),
          cause => cause.message);
      console.error("The installed App's fixture file read reports:", read);
    }
  }
  throw error;
} finally {
  await browser.close();
}
