// BFF reverse proxy for contact-relationship-management API calls.
//
// The browser calls this same-origin proxy; the server forwards to the
// contact-relationship-management service. No token ever lives in browser JS: when a
// session cookie is present (PF-T18 magic-link flow), the server
// exchanges it for a short-lived PASETO and injects the bearer here.
// With `CRM_REQUIRE_AUTH` off (the shipped default) the
// unauthenticated forward is sufficient and the exchange is skipped.

import type { RequestHandler } from "./$types";
import { CRM_API_URL } from "#lib/server/config.js";
import { upstreamUrl } from "#lib/server/api-url.js";
import { exchangeToken } from "#lib/server/auth.js";
import { DEMO_TOUR_ENABLED } from "#lib/server/config.js";
import { demoHandleFor, isDemoSession } from "#lib/server/demo.js";

const proxy: RequestHandler = async ({
  request,
  params,
  url,
  locals,
  fetch,
}) => {
  // Guided tour: a demo session is answered from fictitious data and is never
  // forwarded to the service.
  if (DEMO_TOUR_ENABLED && isDemoSession(locals.sessionId)) {
    let body: unknown = null;
    const query = new URLSearchParams(url.searchParams);
    const path = params.path ?? "";
    if (request.method !== "GET" && request.method !== "HEAD") {
      if (path.endsWith("/attachments")) {
        // A file upload: the tour keeps its name and size, not its bytes.
        query.set("size", String((await request.arrayBuffer()).byteLength));
      } else {
        try {
          body = await request.json();
        } catch {
          body = null;
        }
      }
    }
    if (
      request.method === "GET" &&
      /^attachments\/[^/]+\/content$/.test(path)
    ) {
      return new Response("This is a sample file from the guided tour.\n", {
        status: 200,
        headers: {
          "content-type": "text/plain; charset=utf-8",
          "content-disposition": 'attachment; filename="sample.txt"',
          "x-content-type-options": "nosniff",
        },
      });
    }
    const reply = demoHandleFor(
      locals.sessionId!,
      request.method,
      path,
      query,
      body,
    );
    return new Response(JSON.stringify(reply.body), {
      status: reply.status,
      headers: { "content-type": "application/json" },
    });
  }

  const target = upstreamUrl(CRM_API_URL, params.path ?? "", url.search);

  // Copy request headers, but drop hop-by-hop / origin-specific ones and
  // never forward the browser's cookie to the entity service.
  const headers = new Headers(request.headers);
  headers.delete("cookie");
  headers.delete("host");
  headers.delete("connection");
  headers.delete("content-length");
  headers.set("accepts-version", "1.0");

  // Session → short-lived PASETO, server-side only (PF-T18).
  if (locals.sessionId) {
    const token = await exchangeToken(fetch, locals.sessionId);
    if (token) headers.set("authorization", `Bearer ${token}`);
  }

  const init: RequestInit = { method: request.method, headers };
  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = await request.arrayBuffer();
  }

  const upstream = await fetch(target, init);

  const responseHeaders = new Headers();
  for (const name of [
    "content-type",
    "etag",
    "content-disposition",
    "x-content-type-options",
  ]) {
    const value = upstream.headers.get(name);
    if (value) responseHeaders.set(name, value);
  }
  // Pagination (CRM-T51): forward the next-page link, re-pointed from the
  // service's /api/... path at this proxy so the browser can follow it.
  const link = upstream.headers.get("link");
  if (link) {
    responseHeaders.set("link", link.replace("</api/", "</api/proxy/"));
  }
  return new Response(upstream.body, {
    status: upstream.status,
    headers: responseHeaders,
  });
};

export const GET = proxy;
export const POST = proxy;
export const PUT = proxy;
export const PATCH = proxy;
export const DELETE = proxy;
