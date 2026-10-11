import { describe, expect, it } from "vitest";
import { apiAll, MAX_PAGES, nextLink } from "#lib/api/client.js";

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
