import { describe, expect, it } from "vitest";
import {
  ApiError,
  apiAll,
  apiConditional,
  apiUpload,
  MAX_PAGES,
  nextLink,
} from "#lib/api/client.js";

const page = (rows: unknown[], next?: string) =>
  new Response(JSON.stringify(rows), {
    status: 200,
    headers: next ? { link: `<${next}>; rel="next"` } : {},
  });

describe("nextLink", () => {
  it("reads the next target and ignores other relations", () => {
    expect(nextLink('</api/proxy/x?cursor=a>; rel="next"')).toBe(
      "/api/proxy/x?cursor=a",
    );
    expect(nextLink('</p>; rel="prev", </n>; rel="next"')).toBe("/n");
    expect(nextLink('</p>; rel="prev"')).toBeNull();
    expect(nextLink(null)).toBeNull();
  });
});

describe("apiAll", () => {
  it("follows next links and concatenates every page", async () => {
    const seen: string[] = [];
    const fetcher = (async (url: string) => {
      seen.push(url);
      if (url.endsWith("/contacts"))
        return page([1, 2], "/api/proxy/contacts?cursor=b");
      if (url.endsWith("cursor=b"))
        return page([3, 4], "/api/proxy/contacts?cursor=c");
      return page([5]);
    }) as unknown as typeof fetch;
    expect(await apiAll<number>("/contacts", { fetch: fetcher })).toEqual([
      1, 2, 3, 4, 5,
    ]);
    expect(seen).toHaveLength(3);
  });

  it("returns a single page unchanged", async () => {
    const fetcher = (async () => page([1])) as unknown as typeof fetch;
    expect(await apiAll<number>("/x", { fetch: fetcher })).toEqual([1]);
  });

  it("throws instead of returning a partial list when pages never end", async () => {
    const fetcher = (async () =>
      page([1], "/api/proxy/x?cursor=z")) as unknown as typeof fetch;
    await expect(apiAll("/x", { fetch: fetcher })).rejects.toThrow(
      String(MAX_PAGES),
    );
  });

  it("surfaces an error status", async () => {
    const fetcher = (async () =>
      new Response('{"description":"nope"}', {
        status: 422,
      })) as unknown as typeof fetch;
    await expect(apiAll("/x", { fetch: fetcher })).rejects.toThrow("nope");
  });
});

describe("conditional reads and uploads", () => {
  it("keeps the current body on 304 and carries forward an ETag", async () => {
    const fetcher = (async (_url: string, init?: RequestInit) => {
      expect(init?.headers).toEqual({ "if-none-match": '"old"' });
      return new Response(null, { status: 304 });
    }) as typeof fetch;
    await expect(
      apiConditional("/deals", '"old"', { fetch: fetcher }),
    ).resolves.toEqual({ status: 304, etag: '"old"', body: null });
  });

  it("parses a changed conditional response and its new ETag", async () => {
    const fetcher = (async () =>
      new Response('{"changed":true}', {
        status: 200,
        headers: { etag: '"new"' },
      })) as typeof fetch;
    await expect(
      apiConditional<{ changed: boolean }>("/deals", null, { fetch: fetcher }),
    ).resolves.toEqual({
      status: 200,
      etag: '"new"',
      body: { changed: true },
    });
  });

  it("surfaces conditional and upload problem responses", async () => {
    const refused = (async () =>
      new Response('{"description":"refused"}', {
        status: 422,
      })) as typeof fetch;
    await expect(
      apiConditional("/deals", null, { fetch: refused }),
    ).rejects.toBeInstanceOf(ApiError);
    await expect(
      apiUpload("/attachments", new Blob(["abc"]), { fetch: refused }),
    ).rejects.toThrow("refused");
  });

  it("uploads raw bytes with the required headers", async () => {
    const file = new Blob(["abc"]);
    const fetcher = (async (_url: string, init?: RequestInit) => {
      expect(init?.method).toBe("POST");
      expect(init?.body).toBe(file);
      expect(init?.headers).toMatchObject({
        "content-type": "application/octet-stream",
      });
      return new Response('{"pid":"att-1"}', { status: 200 });
    }) as typeof fetch;
    await expect(
      apiUpload<{ pid: string }>("/attachments", file, { fetch: fetcher }),
    ).resolves.toEqual({ pid: "att-1" });
  });
});
