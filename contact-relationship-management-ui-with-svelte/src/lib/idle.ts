// Idle sign-out timer (CRM-T43). Browser-side: resets on real user input and
// calls `onIdle` once the quiet period passes. Kept framework-free so vitest
// can drive it with fake timers.

/** Events that count as a person using the page (polling does not). */
export const ACTIVITY_EVENTS = [
  "pointerdown",
  "keydown",
  "scroll",
  "touchstart",
] as const;

/**
 * Start watching `target` for activity. Returns a stop function.
 * `onIdle` fires at most once per quiet period of `timeoutMs`.
 */
export function watchIdle(
  target: Pick<EventTarget, "addEventListener" | "removeEventListener">,
  timeoutMs: number,
  onIdle: () => void,
): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const arm = () => {
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(onIdle, timeoutMs);
  };
  for (const name of ACTIVITY_EVENTS)
    target.addEventListener(name, arm, { passive: true });
  arm();
  return () => {
    if (timer !== undefined) clearTimeout(timer);
    for (const name of ACTIVITY_EVENTS) target.removeEventListener(name, arm);
  };
}
