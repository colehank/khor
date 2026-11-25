// The sender's name is a hint from the credential. The node verifies the
// credential and authenticates the machine only when the user chooses Join.
export function pairingIssuer(credential: string): string | null {
  try {
    const binary = atob(credential.replace(/-/g, "+").replace(/_/g, "/"));
    const data = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(
      Uint8Array.from(binary, (byte) => byte.charCodeAt(0)),
    ));
    if (!data || typeof data.id !== "string" || !/^[0-9a-f]{64}$/i.test(data.id) || typeof data.token !== "string" || !data.token
      || !Array.isArray(data.direct) || !Array.isArray(data.relays) || typeof data.name !== "string") return null;
    return data.name.trim().slice(0, 128) || null;
  } catch {
    return null;
  }
}

const PHOTO_LIMIT = 64 * 1024 * 1024;
const DECODE_EDGE = 2048;

/** Read only the selected picture; the raster and photo stay on this device. */
export async function pairingCodeFromPhoto(file: File, signal: AbortSignal): Promise<string | null> {
  if (file.size > PHOTO_LIMIT) throw new Error("pairing image is too large");
  const url = URL.createObjectURL(file);
  const image = new Image();
  const cancel = () => { image.removeAttribute("src"); };
  signal.addEventListener("abort", cancel, { once: true });
  try {
    signal.throwIfAborted();
    image.src = url;
    const [{ default: jsQR }] = await Promise.all([import("jsqr"), image.decode()]);
    signal.throwIfAborted();
    const scale = Math.min(1, DECODE_EDGE / Math.max(image.naturalWidth, image.naturalHeight));
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
    canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
    const context = canvas.getContext("2d");
    if (!context) return null;
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    const raster = context.getImageData(0, 0, canvas.width, canvas.height);
    return jsQR(raster.data, raster.width, raster.height)?.data.trim() || null;
  } finally {
    signal.removeEventListener("abort", cancel);
    URL.revokeObjectURL(url);
  }
}
