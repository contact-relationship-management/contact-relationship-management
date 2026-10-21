import adapter from "@sveltejs/adapter-node";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { sveltekit } from "@sveltejs/kit/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
    // `svelteTesting()` selects Svelte's browser/client resolve condition so
    // components mount client-side under jsdom (rather than SSR), and wires
    // testing-library auto-cleanup. It only affects the vitest run.
    plugins: [
        sveltekit({
            preprocess: vitePreprocess(),
            adapter: adapter(),
            // Route matchers (`[locale=locale]`).
            files: { params: "src/params.ts" },
            // CRM-T33: CSP with hashes for the framework's inline bootstrap.
            // Component libraries set inline styles, so style-src keeps
            // 'unsafe-inline'; scripts do not.
            csp: {
                mode: "auto",
                directives: {
                    "default-src": ["self"],
                    "script-src": ["self"],
                    "style-src": ["self", "unsafe-inline"],
                    "img-src": ["self", "data:"],
                    "font-src": ["self", "data:"],
                    "connect-src": ["self"],
                    "object-src": ["none"],
                    "base-uri": ["self"],
                    "form-action": ["self"],
                    "frame-ancestors": ["none"]
                }
            }
        }),
        svelteTesting()
    ],
    server: { port: 5173, strictPort: false },
    test: {
        include: ["tests/unit/**/*.{test,spec}.{ts,js}"],
        environment: "jsdom",
        globals: true,
        // The package's `dist` imports without file extensions, which Node's ESM
        // loader refuses; Vite resolves them, so Vitest must bundle it too.
        server: { deps: { inline: [/@lilydesignsystem\/locale-translations/] } },
    },
});
