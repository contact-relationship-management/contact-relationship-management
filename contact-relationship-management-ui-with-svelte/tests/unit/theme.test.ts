// The theme (CRM-R96): "Follow this device" unless the person picks a theme, kept in the
// `crm-theme` cookie, which the server reads to send the page in that theme.
import { readFileSync } from "node:fs";
import { resolve as resolvePath } from "node:path";
import { describe, expect, it } from "vitest";
import { handle } from "../../src/hooks.server";
import {
  CHOICES,
  OLD_THEME_KEYS,
  SYSTEM_THEME,
  THEMES,
  THEME_COOKIE,
  dropServerLinks,
  forgetOldKeys,
  themeAttribute,
  themeLinks,
} from "../../src/lib/theme";

const shell = readFileSync(
  resolvePath(__dirname, "../../src/app.html"),
  "utf8",
);

/** The page the server sends for a request carrying `cookie`. */
async function pageFor(cookie: string | null): Promise<string> {
  const headers = new Headers(cookie === null ? {} : { cookie });
  const event = {
    request: new Request("http://localhost/signin", { headers }),
    cookies: { get: () => undefined },
    locals: {},
  };
  let html = "";
  type Input = Parameters<typeof handle>[0];
  await handle({
    event: event as unknown as Input["event"],
    resolve: async (
      _event: unknown,
      opts?: {
        transformPageChunk?: (input: { html: string; done: boolean }) => string;
      },
    ) => {
      html = opts?.transformPageChunk?.({ html: shell, done: true }) ?? shell;
      return new Response(html);
    },
  } as unknown as Input);
  return html;
}

describe("the theme the server sends", () => {
  it("follows the device when nothing was chosen: both themes, by the browser's setting", async () => {
    const html = await pageFor(null);
    expect(html).toContain('<html lang="en" data-theme="">');
    expect(html).toContain(
      'href="/assets/themes/light.css" media="(prefers-color-scheme: light)"',
    );
    expect(html).toContain(
      'href="/assets/themes/dark.css" media="(prefers-color-scheme: dark)"',
    );
    expect(html).not.toContain("%crm.");
  });

  it("is the chosen theme when the cookie holds one", async () => {
    const html = await pageFor(`other=1; ${THEME_COOKIE}=cupcake`);
    expect(html).toContain('data-theme="cupcake"');
    expect(html).toContain(
      'href="/assets/themes/cupcake.css" data-crm-server-theme',
    );
    expect(html).not.toContain("light.css");
  });

  it("follows the device for a cookie that says so", async () => {
    expect(await pageFor(`${THEME_COOKIE}=${SYSTEM_THEME}`)).toContain(
      'data-theme=""',
    );
  });

  it("ignores a cookie that is not one of the choices, so it never reaches the page", async () => {
    const html = await pageFor(
      `${THEME_COOKIE}=${encodeURIComponent('"><script>x</script>')}`,
    );
    expect(html).not.toContain("<script>x");
    expect(html).toContain('data-theme=""');
    expect(await pageFor(`${THEME_COOKIE}=../secrets`)).toContain(
      'data-theme=""',
    );
  });
});

describe("the theme choices", () => {
  it("are 'Follow this device' first, then every theme", () => {
    expect(CHOICES[0]).toBe(SYSTEM_THEME);
    expect(CHOICES.slice(1)).toEqual(THEMES);
    expect(THEMES).toContain("light");
    expect(THEMES).toContain("dark");
  });

  it("put no data-theme on the page for 'system', and the slug otherwise", () => {
    expect(themeAttribute(SYSTEM_THEME)).toBe("");
    expect(themeAttribute("dark")).toBe("dark");
    expect(themeLinks("dark")).toBe(
      '<link rel="stylesheet" href="/assets/themes/dark.css" data-crm-server-theme>',
    );
  });
});

describe("the earlier storage keys", () => {
  it("are removed, and storage that cannot be used is left alone", () => {
    const items = new Map(OLD_THEME_KEYS.map((key) => [key, "dark"]));
    forgetOldKeys({ removeItem: (key: string) => void items.delete(key) });
    expect(items.size).toBe(0);
    expect(() =>
      forgetOldKeys({
        removeItem: () => {
          throw new Error("blocked");
        },
      }),
    ).not.toThrow();
    expect(() => forgetOldKeys(undefined)).not.toThrow();
  });
});

describe("the server's links", () => {
  it("go once the picker's own stylesheet has loaded, not before", () => {
    document.head.innerHTML =
      themeLinks(SYSTEM_THEME) +
      '<link rel="stylesheet" data-lily-theme-picker="theme" href="/assets/themes/dark.css">';
    dropServerLinks(document);
    expect(
      document.head.querySelectorAll("link[data-crm-server-theme]"),
    ).toHaveLength(2);
    document.head
      .querySelector('link[data-lily-theme-picker="theme"]')!
      .dispatchEvent(new Event("load"));
    expect(
      document.head.querySelectorAll("link[data-crm-server-theme]"),
    ).toHaveLength(0);
    expect(document.head.querySelectorAll("link")).toHaveLength(1);
  });

  it("stay while the picker has no stylesheet", () => {
    document.head.innerHTML = themeLinks("dark");
    dropServerLinks(document);
    expect(
      document.head.querySelectorAll("link[data-crm-server-theme]"),
    ).toHaveLength(1);
  });
});

describe("the theme picker the picker bar uses", () => {
  it("is the one the layout configures: a single 0.4 in the lockfile, with no override", () => {
    const lock = readFileSync(
      resolvePath(__dirname, "../../pnpm-lock.yaml"),
      "utf8",
    );
    const versions = new Set(
      [
        ...lock.matchAll(
          /^ {2}'@lilydesignsystem\/svelte-theme-picker@([\d.]+)/gm,
        ),
      ].map((match) => match[1]),
    );
    expect([...versions]).toEqual(["0.4.0"]);
    expect(lock).not.toMatch(/^overrides:/m);
  });
});
