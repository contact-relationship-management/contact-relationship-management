// See https://svelte.dev/docs/kit/types#app
declare global {
  namespace App {
    interface PageData {
      /** The page title, also used in <title>. */
      title?: string;
      /** The role-brief locale of the page, when it has one. */
      locale?: string;
    }
  }
}

export {};
