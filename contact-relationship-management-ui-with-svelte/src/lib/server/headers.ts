// Security response headers for every UI response (CRM-T33). Pure so
// vitest can pin the set; applied in `hooks.server.ts`. The
// Content-Security-Policy itself comes from SvelteKit's `kit.csp` so it can
// hash the framework's inline bootstrap script.

export const SECURITY_HEADERS: Readonly<Record<string, string>> = {
  "Strict-Transport-Security": "max-age=63072000; includeSubDomains",
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
  // `same-origin`, not `no-referrer`: with `no-referrer` Chrome sends
  // `Origin: null` on a plain same-origin form POST (the Sign out button), and
  // SvelteKit's CSRF check then refuses it with 403. Nothing is sent to other
  // sites either way.
  "Referrer-Policy": "same-origin",
  "Permissions-Policy":
    "camera=(), microphone=(), geolocation=(), payment=(), usb=(), interest-cohort=()",
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Resource-Policy": "same-origin",
};
