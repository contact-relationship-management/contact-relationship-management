// Pins the idle sign-out timer (CRM-T43).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { watchIdle } from "../../src/lib/idle";

describe("watchIdle", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("fires after the quiet period", () => {
    const target = new EventTarget();
    const onIdle = vi.fn();
    watchIdle(target, 1000, onIdle);
    vi.advanceTimersByTime(999);
    expect(onIdle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it("is postponed by user activity", () => {
    const target = new EventTarget();
    const onIdle = vi.fn();
    watchIdle(target, 1000, onIdle);
    vi.advanceTimersByTime(900);
    target.dispatchEvent(new Event("keydown"));
    vi.advanceTimersByTime(900);
    expect(onIdle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(100);
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it("ignores events that are not user input", () => {
    const target = new EventTarget();
    const onIdle = vi.fn();
    watchIdle(target, 1000, onIdle);
    vi.advanceTimersByTime(900);
    target.dispatchEvent(new Event("message"));
    vi.advanceTimersByTime(100);
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it("stops cleanly", () => {
    const target = new EventTarget();
    const onIdle = vi.fn();
    const stop = watchIdle(target, 1000, onIdle);
    stop();
    vi.advanceTimersByTime(5000);
    expect(onIdle).not.toHaveBeenCalled();
  });
});
