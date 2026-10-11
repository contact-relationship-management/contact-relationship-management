// One stable idempotency key per user intent (CRM-T52).
//
// A form holds an intent for as long as the user is trying to do one thing.
// While the content they are submitting is unchanged, every attempt (a retry
// after a timeout, a second click after an error) carries the same key, so the
// service performs the action at most once. Changing the content starts a new
// intent, because the service refuses a key reused for a different request.
// A success ends the intent; the next submission is a new action.

import { newIdempotencyKey } from "#lib/api/client.js";

/** A held key for one action, rotated by content change or by success. */
export interface Intent {
  /** The key to send for this content; stable until the content changes. */
  keyFor(content: unknown): string;
  /** Call after the action succeeded: the next submission is a new action. */
  done(): void;
}

/** A new, empty intent. */
export function createIntent(): Intent {
  let key: string | null = null;
  let seen: string | null = null;
  return {
    keyFor(content: unknown): string {
      const fingerprint = JSON.stringify(content);
      if (key === null || fingerprint !== seen) {
        key = newIdempotencyKey();
        seen = fingerprint;
      }
      return key;
    },
    done(): void {
      key = null;
      seen = null;
    },
  };
}

/** One intent per id (for a list of rows, each with its own action). */
export function createIntents(): (id: string) => Intent {
  const held = new Map<string, Intent>();
  return (id) => {
    let intent = held.get(id);
    if (!intent) {
      intent = createIntent();
      held.set(id, intent);
    }
    return intent;
  };
}
