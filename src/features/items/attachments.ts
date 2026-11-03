// Note images: shrinking before upload, decrypting on demand and a small in-memory
// cache of object URLs that is wiped when the vault locks.
import { useEffect, useState } from "react";
import { addAttachment, getAttachment } from "@/services/vault";
import { useSessionStore } from "@/store/session";

/** How notes refer to an image: `![alt](attachment:<id>)`. */
export const ATTACHMENT_PREFIX = "attachment:";
/** Screenshots bigger than this (on the long side) are scaled down. */
const MAX_SIDE = 2560;
/** Files at most this big and within MAX_SIDE are stored as they are. */
const KEEP_AS_IS_BYTES = 1.5 * 1024 * 1024;
const ACCEPTED = new Set(["image/png", "image/jpeg", "image/webp", "image/gif"]);

export const imageMarkdown = (id: string, alt = "картинка") =>
  `![${alt.replace(/[[\]]/g, "")}](${ATTACHMENT_PREFIX}${id})`;

/** Attachment ids referenced in a note, in order of appearance, without repeats. */
export function referencedAttachments(markdown: string): string[] {
  const ids = [...markdown.matchAll(/attachment:([0-9a-zA-Z-]+)/g)].map((m) => m[1]!);
  return [...new Set(ids)];
}

export function isImageFile(file: { type: string }): boolean {
  return ACCEPTED.has(file.type) || file.type === "image/bmp";
}

function canvasToBlob(canvas: HTMLCanvasElement, type: string, quality?: number) {
  return new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("encode"))), type, quality),
  );
}

/**
 * Bytes to store for a pasted or dropped image. Small images keep their original
 * file; big ones are scaled to MAX_SIDE and re-encoded (PNG for screenshots, where
 * text must stay crisp, unless WebP is much smaller).
 */
export async function prepareImage(file: Blob): Promise<Uint8Array> {
  const bitmap = await createImageBitmap(file);
  try {
    const scale = Math.min(1, MAX_SIDE / Math.max(bitmap.width, bitmap.height));
    if (scale === 1 && file.size <= KEEP_AS_IS_BYTES && ACCEPTED.has(file.type)) {
      return new Uint8Array(await file.arrayBuffer());
    }
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(bitmap.width * scale);
    canvas.height = Math.round(bitmap.height * scale);
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("canvas");
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    const png = await canvasToBlob(canvas, "image/png");
    const webp = await canvasToBlob(canvas, "image/webp", 0.9);
    const best = webp.type === "image/webp" && webp.size < png.size * 0.6 ? webp : png;
    return new Uint8Array(await best.arrayBuffer());
  } finally {
    bitmap.close();
  }
}

/** Shrinks and uploads an image for a saved note; resolves to its markdown. */
export async function uploadImage(itemId: string, file: Blob, alt?: string): Promise<string> {
  const bytes = await prepareImage(file);
  const id = await addAttachment(itemId, bytes);
  return imageMarkdown(id, alt);
}

// ---- decrypted image cache ----

const cache = new Map<string, Promise<string>>();

function toObjectUrl(raw: ArrayBuffer | number[]): string {
  const bytes = raw instanceof ArrayBuffer ? new Uint8Array(raw) : Uint8Array.from(raw);
  const mimeLength = bytes[0] ?? 0;
  const mime = new TextDecoder().decode(bytes.subarray(1, 1 + mimeLength));
  return URL.createObjectURL(new Blob([bytes.subarray(1 + mimeLength)], { type: mime }));
}

/** Object URL of a decrypted image; each image is decrypted once per unlock. */
export function attachmentUrl(id: string): Promise<string> {
  let url = cache.get(id);
  if (!url) {
    url = getAttachment(id).then(toObjectUrl);
    url.catch(() => cache.delete(id));
    cache.set(id, url);
  }
  return url;
}

/** Decrypted image state for one attachment id. */
export function useAttachmentUrl(id: string) {
  const [state, setState] = useState<{ id: string; url: string | null; failed: boolean }>({
    id,
    url: null,
    failed: false,
  });
  useEffect(() => {
    let alive = true;
    attachmentUrl(id).then(
      (url) => alive && setState({ id, url, failed: false }),
      () => alive && setState({ id, url: null, failed: true }),
    );
    return () => {
      alive = false;
    };
  }, [id]);
  return state.id === id ? state : { id, url: null, failed: false };
}

export function clearAttachmentCache() {
  for (const url of cache.values()) {
    url.then((u) => URL.revokeObjectURL(u)).catch(() => undefined);
  }
  cache.clear();
}

// decrypted pixels must not outlive the session
useSessionStore.subscribe((s) => {
  if (s.status?.state !== "unlocked" && cache.size > 0) clearAttachmentCache();
});
