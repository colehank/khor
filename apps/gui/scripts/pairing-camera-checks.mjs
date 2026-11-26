import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const credential = Buffer.from(JSON.stringify({
  id: "1".repeat(64), name: "Mac server", direct: [], relays: [], token: "lab-camera-test",
})).toString("base64url");

async function checkCameraPaint(page) {
  const expectedPaper = [20, 60, 100, 255];
  const samples = await page.evaluate(async paper => {
    const camera = document.querySelector("[data-camera-live]");
    const sheet = camera.closest("[data-join-sheet]");
    await Promise.all(sheet.getAnimations().map(animation => animation.finished));
    sheet.style.setProperty("--popover", `rgb(${paper.slice(0, 3).join(",")})`);
    document.body.style.backgroundColor = "rgb(230, 30, 146)";
    const home = document.createElement("div");
    home.dataset.pairingPaintProbe = "";
    home.style.cssText = "position:fixed;inset:0;z-index:2147483647;pointer-events:none;background:rgb(90, 220, 70)";
    document.getElementById("root").append(home);
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const box = camera.getBoundingClientRect();
    const scale = devicePixelRatio;
    const x = Math.floor((box.left + box.width / 2) * scale);
    return {
      inside: [x, Math.floor((box.top + box.height / 2) * scale)],
      below: [x, Math.ceil(box.bottom * scale)],
      safeArea: [x, Math.floor((innerHeight - 5) * scale)],
    };
  }, expectedPaper);
  try {
    const shot = await page.screenshot({ scale: "device" });
    const pixels = await page.evaluate(async ({ png, samples }) => {
      const image = new Image();
      image.src = `data:image/png;base64,${png}`;
      await image.decode();
      const canvas = document.createElement("canvas");
      canvas.width = image.width;
      canvas.height = image.height;
      const context = canvas.getContext("2d");
      context.drawImage(image, 0, 0);
      return Object.fromEntries(Object.entries(samples).map(([name, [x, y]]) =>
        [name, Array.from(context.getImageData(x, y, 1, 1).data)]));
    }, { png: shot.toString("base64"), samples });
    assert.deepEqual(pixels.inside, [230, 30, 146, 255], "picker exposes the camera within the rounded opening");
    assert.deepEqual(pixels.below, expectedPaper, "sheet paper covers the first pixel below the camera");
    assert.deepEqual(pixels.safeArea, expectedPaper, "sheet paper covers the bottom safe area");
  } finally {
    await page.evaluate(() => {
      document.querySelector("[data-pairing-paint-probe]").remove();
      document.querySelector("[data-join-sheet]").style.removeProperty("--popover");
      document.body.style.removeProperty("background-color");
    });
  }
}

export async function checkPairingCamera(page, url, openDevices) {
  let mobile = false;
  const pairs = [];
  const scanner = route => route.fulfill({ json: mobile });
  const pair = route => {
    pairs.push(route.request().postDataJSON().ticket);
    return route.fulfill({ status: 400, json: "测试拒绝" });
  };
  const sample = JSON.parse(readFileSync(new URL("../dev/lab/data/world.generated.json", import.meta.url), "utf8")).pairing_ticket;
  const invite = route => route.fulfill({ json: sample });
  await page.route("**/invite", invite);
  await page.route("**/scanner_here", scanner);
  await page.route("**/pair", pair);
  await page.addInitScript(() => {
    const scenario = new URLSearchParams(location.search).get("pairing-scan");
    if (!scenario) return;
    localStorage.setItem("khor.lang", "zh");
    const state = {
      calls: [],
      permission: scenario === "prompt" ? "prompt" : scenario === "denied" ? "denied" : "granted",
      release: null,
    };
    window.__pairingScanner = state;
    window.__TAURI_INTERNALS__ = {
      invoke(command, args) {
        if (!command.startsWith("plugin:barcode-scanner|")) return Promise.resolve(null);
        const name = command.split("|")[1];
        state.calls.push({ name, args });
        if (name === "check_permissions") return Promise.resolve({ camera: state.permission });
        if (name === "request_permissions") return Promise.resolve({ camera: "denied" });
        if (name === "scan") {
          if (scenario === "unavailable") return Promise.reject("no camera");
          return new Promise(resolve => {
            state.release = content => resolve({ content, format: "QR_CODE" });
          });
        }
        // Exercise Android 2.4's cancellation path: native cancel returns,
        // but the scan command may never settle.
        return Promise.resolve(null);
      },
      transformCallback: () => 0,
      unregisterCallback: () => {},
      convertFileSrc: path => path,
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  });

  const sheet = page.locator('[data-add-machine="ticket"]');
  const camera = sheet.locator('[data-join-camera="scan"]');
  const input = sheet.locator("[data-ticket-input]");
  const calls = () => page.evaluate(() => window.__pairingScanner.calls.map(call => call.name));
  const standardViewport = { width: 1400, height: 900 };
  const acts = () => sheet.locator("[data-pairing-mode]");
  const enter = async (scenario, viewport = { width: 390, height: 844 }) => {
    if (page.url() !== "about:blank") await page.evaluate(() => localStorage.clear());
    await page.setViewportSize(viewport);
    await page.waitForLoadState("networkidle");
    await page.goto(`${url}&pairing-scan=${scenario}`, { waitUntil: "load" });
    if (mobile) {
      await page.evaluate(() => document.documentElement.style.setProperty("--safe-bottom", "34px"));
    }
    assert(await openDevices(), "the Machines page is available");
    await page.locator("[data-side-head] [data-pane-new]:visible").first().click();
    await page.locator('[data-new-item="khor"]:visible').click();
    await sheet.locator("[data-ticket-window]").waitFor();
    const inviteHeight = await sheet.evaluate(el => el.offsetHeight);
    await acts().getByRole("radio", { name: "加入网络", exact: true }).click();
    return inviteHeight;
  };
  const photo = readFileSync(new URL("./fixtures/pairing-camera.png", import.meta.url));
  const outside = async () => {
    const bounds = await sheet.boundingBox();
    assert(bounds && bounds.y > 1);
    await page.mouse.click(bounds.x + bounds.width / 2, bounds.y / 2);
    await sheet.waitFor({ state: "hidden" });
  };

  try {
    await enter("granted", standardViewport);
    await input.waitFor();
    assert.equal(await camera.count(), 0);
    assert.deepEqual(await calls(), []);
    await input.fill(credential);
    await sheet.locator("[data-join]").click();
    await sheet.locator("[data-dialog-error]").waitFor();
    assert.deepEqual(pairs.splice(0), [credential]);

    mobile = true;
    const inviteHeight = await enter("granted");
    await page.locator("html[data-pairing-camera]").waitFor({ state: "attached" });
    assert.equal(await acts().isVisible(), true);
    assert.equal(await sheet.evaluate(el => el.offsetHeight), inviteHeight);
    const cameraState = await page.evaluate(() => {
      const finder = document.querySelector("[data-camera-live]");
      const root = document.getElementById("root");
      const sheet = finder.closest("[data-join-sheet]");
      const scrim = sheet.previousElementSibling;
      const scan = window.__pairingScanner.calls.find(value => value.name === "scan");
      return {
        app: getComputedStyle(root).clipPath,
        scrim: getComputedStyle(scrim).clipPath,
        paper: getComputedStyle(sheet, "::before").clipPath,
        camera: getComputedStyle(finder).backgroundColor,
        body: getComputedStyle(document.body).backgroundColor,
        options: scan?.args,
      };
    });
    for (const key of ["app", "scrim", "paper"]) assert.match(cameraState[key], /^path\(evenodd/);
    assert.equal(cameraState.camera, "rgba(0, 0, 0, 0)");
    assert.equal(cameraState.body, "rgba(0, 0, 0, 0)");
    assert.deepEqual(cameraState.options, {windowed: true, formats: ["QR_CODE"]});
    await checkCameraPaint(page);
    await page.evaluate(ticket => window.__pairingScanner.release(ticket), credential);
    const confirm = sheet.locator("[data-join-confirm]");
    await confirm.getByText("加入 Mac server 的网络？", { exact: true }).waitFor();
    assert.equal(await sheet.evaluate(el => el.offsetHeight), inviteHeight);
    assert.deepEqual(pairs, []);
    await confirm.getByRole("button", { name: "凭据已填入" }).click();
    assert.equal(await input.inputValue(), credential);
    await sheet.locator("[data-join]").click();
    await confirm.getByRole("alert").waitFor();
    assert.deepEqual(pairs.splice(0), [credential]);
    await sheet.getByRole("button", { name: "返回", exact: true }).click();
    await page.locator("html[data-pairing-camera]").waitFor({ state: "attached" });

    let chooser = page.waitForEvent("filechooser");
    await camera.locator("[data-join-photos]").click();
    await chooser;
    await camera.locator("[data-join-photo-file]").dispatchEvent("cancel");
    await page.waitForFunction(() => window.__pairingScanner.calls.filter(call => call.name === "scan").length === 3);
    chooser = page.waitForEvent("filechooser");
    await camera.locator("[data-join-photos]").click();
    await (await chooser).setFiles({ name: "pairing.png", mimeType: "image/png", buffer: photo });
    await confirm.getByText("加入 Mac server 的网络？", { exact: true }).waitFor();
    assert.deepEqual(pairs, []);
    assert.equal((await calls()).filter(name => name === "cancel").length, 2);
    await outside();

    for (const scenario of ["denied", "prompt", "unavailable"]) {
      await enter(scenario);
      const settings = sheet.locator("[data-scan-settings]");
      if (scenario === "unavailable") {
        await camera.getByText("无法打开相机", { exact: false }).waitFor();
      } else {
        await settings.click();
        await page.waitForFunction(() => window.__pairingScanner.calls.some(call => call.name === "open_app_settings"));
        const used = await calls();
        assert.equal(used.includes("request_permissions"), scenario === "prompt");
        assert(!used.includes("scan"));
        assert(used.includes("open_app_settings"));
      }
      await camera.locator("[data-join-manual]").click();
      await input.waitFor();
      assert.equal(await input.inputValue(), "");
      await outside();
    }
    assert.deepEqual(pairs, []);
    console.log("PASS pairing camera: native QR, opaque bottom edge, permission fallbacks, real photo decode, cancel/reopen, explicit pairing");
  } finally {
    if (process.env.FACE_ONLY !== "scan") {
      await page.setViewportSize(standardViewport);
      await page.waitForLoadState("networkidle");
      await page.goto(url, { waitUntil: "networkidle" });
    }
    await page.unroute("**/invite", invite);
    await page.unroute("**/scanner_here", scanner);
    await page.unroute("**/pair", pair);
  }
}
