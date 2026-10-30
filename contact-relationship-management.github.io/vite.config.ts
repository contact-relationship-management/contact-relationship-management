import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

// A static site for GitHub Pages. Every page is prerendered (`src/routes/+layout.ts`), so the build is plain
// HTML, CSS and JavaScript with no server. BASE_PATH is empty for the organization site
// (https://contact-relationship-management.github.io/); set it (e.g. "/some-repo") to serve from a project path.
export default defineConfig({
  plugins: [
    sveltekit({
      preprocess: vitePreprocess(),
      adapter: adapter({ pages: "build", assets: "build", fallback: undefined, strict: true }),
      paths: { base: (process.env.BASE_PATH ?? "") as "" | `/${string}` },
      prerender: { handleHttpError: "fail", handleMissingId: "fail" },
      // GitHub Pages cannot send headers, so the policy is a <meta http-equiv> that SvelteKit writes into each
      // prerendered page with the hashes of its own inline scripts. As in the PPM UI, computed styles use
      // Svelte's `style:` directive and never a `style="…"` attribute: style-src-attr allows only the one value
      // SvelteKit's own screen-reader announcer sets (the hash the UI's vite.config.ts records).
      csp: {
        mode: "hash",
        directives: {
          "default-src": ["self"],
          "script-src": ["self"],
          "style-src": ["self"],
          "style-src-attr": ["unsafe-hashes", "sha256-S8qMpvofolR8Mpjy4kQvEm7m1q8clzU4dfDH0AmvZjo="],
          "img-src": ["self", "data:"],
          "font-src": ["self"],
          "connect-src": ["self"],
          "object-src": ["none"],
          "base-uri": ["self"],
          "form-action": ["self"],
        },
      },
    }),
  ],
});
