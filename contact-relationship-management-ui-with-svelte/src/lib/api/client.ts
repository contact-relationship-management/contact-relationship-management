// Minimal typed fetch wrapper over the BFF proxy.

import { API_BASE_URL } from "#lib/config.js";

/** Error carrying the upstream status + parsed body, for page handling. */
export class ApiError extends Error {
  status: number;
  body: unknown;
  constructor(status: number, body: unknown) {
    const description =
      typeof body === "object" && body !== null && "description" in body
        ? String((body as { description: unknown }).description)
        : `API error ${status}`;
    super(description);
    this.status = status;
    this.body = body;
  }
}

/** A conditional GET's outcome: `body` is `null` on `304 Not Modified`. */
export interface Conditional<T> {
  status: number;
  etag: string | null;
  body: T | null;
}

/**
 * Conditional GET with `If-None-Match` (the board-polling path,
 * PF-T17): `304` yields `body: null` so the caller keeps its current
 * render; anything else parses as usual.
 */
export async function apiConditional<T>(
  path: string,
  etag: string | null,
  init?: { fetch?: typeof fetch },
): Promise<Conditional<T>> {
  const doFetch = init?.fetch ?? fetch;
  const response = await doFetch(`${API_BASE_URL}${path}`, {
    headers: etag ? { "if-none-match": etag } : {},
  });
  const nextEtag = response.headers.get("etag");
  if (response.status === 304) {
    return { status: 304, etag: nextEtag ?? etag, body: null };
  }
  const text = await response.text();
  const parsed: unknown = text ? JSON.parse(text) : null;
  if (!response.ok) throw new ApiError(response.status, parsed);
  return { status: response.status, etag: nextEtag, body: parsed as T };
}

/** Safety stop for [`apiAll`]: pages followed before giving up. */
export const MAX_PAGES = 50;

/** The target of a `Link: <...>; rel="next"` header, if any. */
export function nextLink(header: string | null): string | null {
  if (!header) return null;
  for (const part of header.split(",")) {
    const match = /^\s*<([^>]*)>\s*;\s*rel="?next"?\s*$/.exec(part);
    if (match) return match[1] ?? null;
  }
  return null;
}

/**
 * GET a list and follow `Link: rel="next"` until it ends, so a list longer
 * than one page is returned whole instead of cut off. Throws rather than
 * return a partial list if [`MAX_PAGES`] is reached.
 */
export async function apiAll<T>(
  path: string,
  init?: { fetch?: typeof fetch },
): Promise<T[]> {
  const doFetch = init?.fetch ?? fetch;
  const rows: T[] = [];
  let url: string | null = `${API_BASE_URL}${path}`;
  for (let page = 0; url; page++) {
    if (page >= MAX_PAGES) {
      throw new ApiError(500, {
        description: `list has more than ${MAX_PAGES} pages`,
      });
    }
    const response: Response = await doFetch(url);
    const text = await response.text();
    const parsed: unknown = text ? JSON.parse(text) : null;
    if (!response.ok) throw new ApiError(response.status, parsed);
    rows.push(...(parsed as T[]));
    url = nextLink(response.headers.get("link"));
  }
  return rows;
}

/** A fresh random key; `crypto.randomUUID` needs a secure context, so fall back. */
export function newIdempotencyKey(): string {
  const c = globalThis.crypto;
  if (typeof c?.randomUUID === "function") return c.randomUUID();
  const bytes = new Uint8Array(16);
  if (c?.getRandomValues) c.getRandomValues(bytes);
  else for (let i = 0; i < bytes.length; i++) bytes[i] = Math.random() * 256;
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Identical POSTs currently in flight, so a double click sends one request. */
const inflight = new Map<string, Promise<unknown>>();

/**
 * Perform one JSON request against the proxy; throws [`ApiError`] on non-2xx.
 *
 * Every `POST` carries an `Idempotency-Key` (CRM-T52): a retry that reuses the
 * key gets the first response back instead of repeating the work. Pass
 * `idempotencyKey` to keep one key across a user's retries of the same
 * action; otherwise each call gets a fresh one. Two identical `POST`s issued
 * while the first is still in flight share that one request.
 */
export function api<T>(
  path: string,
  init?: {
    method?: string;
    body?: unknown;
    fetch?: typeof fetch;
    idempotencyKey?: string;
  },
): Promise<T> {
  const method = init?.method ?? "GET";
  if (method !== "POST") return send<T>(path, init, method);
  const dedupe = `${path}\u0000${JSON.stringify(init?.body ?? null)}`;
  const pending = inflight.get(dedupe);
  if (pending) return pending as Promise<T>;
  const request = send<T>(path, init, method).finally(() => {
    inflight.delete(dedupe);
  });
  inflight.set(dedupe, request);
  return request;
}

async function send<T>(
  path: string,
  init:
    | {
        body?: unknown;
        fetch?: typeof fetch;
        idempotencyKey?: string;
      }
    | undefined,
  method: string,
): Promise<T> {
  const doFetch = init?.fetch ?? fetch;
  const url = `${API_BASE_URL}${path}`;
  const headers: Record<string, string> = {};
  if (init?.body !== undefined) headers["content-type"] = "application/json";
  if (method === "POST") {
    headers["idempotency-key"] = init?.idempotencyKey ?? newIdempotencyKey();
  }
  const response = await doFetch(url, {
    method,
    headers,
    body: init?.body !== undefined ? JSON.stringify(init.body) : undefined,
  });
  const text = await response.text();
  const parsed: unknown = text ? JSON.parse(text) : null;
  if (!response.ok) throw new ApiError(response.status, parsed);
  return parsed as T;
}
