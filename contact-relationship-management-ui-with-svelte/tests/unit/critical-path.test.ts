import { describe, expect, it } from "vitest";
import { criticalPath, type ScheduledTask } from "../../src/lib/critical-path";

const task = (
  id: string,
  start: string,
  end: string,
  dependsOn?: ScheduledTask["dependsOn"],
): ScheduledTask => ({ id, start, end, dependsOn });

describe("criticalPath", () => {
  it("has nothing to say about no tasks", () => {
    const r = criticalPath([]);
    expect(r.finish).toBeNull();
    expect(r.tasks.size).toBe(0);
  });

  it("with no dependencies, whatever ends last is critical and the rest have slack", () => {
    const r = criticalPath([
      task("a", "2026-10-01", "2026-10-05"),
      task("b", "2026-10-03", "2026-10-20"),
      task("c", "2026-10-10", "2026-10-15"),
    ]);
    expect(r.finish).toBe("2026-10-20");
    expect(r.tasks.get("b")).toEqual({ slack: 0, critical: true });
    expect(r.tasks.get("a")).toEqual({ slack: 15, critical: false });
    expect(r.tasks.get("c")).toEqual({ slack: 5, critical: false });
  });

  it("a chain of finish-to-start tasks is all critical, and a side task has slack", () => {
    const r = criticalPath([
      task("design", "2026-10-01", "2026-10-05"),
      task("build", "2026-10-06", "2026-10-12", ["design"]),
      task("launch", "2026-10-13", "2026-10-14", ["build"]),
      task("docs", "2026-10-06", "2026-10-08", ["design"]),
    ]);
    for (const id of ["design", "build", "launch"]) {
      expect(r.tasks.get(id)?.critical, id).toBe(true);
    }
    // docs can slip until it would hold up nothing: launch finishes the schedule.
    expect(r.tasks.get("docs")).toEqual({ slack: 6, critical: false });
  });

  it("a task that a later one waits for gets its slack from that one, not only the finish", () => {
    const r = criticalPath([
      task("a", "2026-10-01", "2026-10-03"),
      task("b", "2026-10-10", "2026-10-20", [{ id: "a", type: "e2s" }]),
      task("z", "2026-10-01", "2026-10-30"),
    ]);
    // The schedule ends 30 Oct (z), so b may end 10 days late (it would still
    // finish on 30 Oct); a may then end as late as the day before b's latest start.
    expect(r.tasks.get("z")?.critical).toBe(true);
    expect(r.tasks.get("b")).toEqual({ slack: 10, critical: false });
    expect(r.tasks.get("a")).toEqual({ slack: 16, critical: false });
  });

  it("a lag on a dependency uses up slack", () => {
    const tasks = (lag: number) => [
      task("a", "2026-10-01", "2026-10-05"),
      task("b", "2026-10-08", "2026-10-10", [{ id: "a", lag }]),
    ];
    // b ends the schedule, so it cannot slip; a may end as late as the day before b starts.
    expect(criticalPath(tasks(0)).tasks.get("a")?.slack).toBe(2);
    expect(criticalPath(tasks(1)).tasks.get("a")?.slack).toBe(1);
    expect(criticalPath(tasks(2)).tasks.get("a")?.slack).toBe(0);
  });

  it("understands start-to-start, end-to-end and start-to-end links", () => {
    const base = (type: "s2s" | "e2e" | "s2e") => [
      task("p", "2026-10-01", "2026-10-04"),
      task("s", "2026-10-02", "2026-10-12", [{ id: "p", type }]),
    ];
    // s2s: p must start by s's start (1 Oct <= 2 Oct): p may end 1 day late.
    expect(criticalPath(base("s2s")).tasks.get("p")?.slack).toBe(1);
    // e2e: p must end by s's end (12 Oct): p may end 8 days late.
    expect(criticalPath(base("e2e")).tasks.get("p")?.slack).toBe(8);
    // s2e: p must start by s's end, which allows more than the schedule does; the
    // schedule's own finish (12 Oct) is the limit, so p may end 8 days late.
    expect(criticalPath(base("s2e")).tasks.get("p")?.slack).toBe(8);
  });

  it("refuses a dependency loop instead of looping", () => {
    const r = criticalPath([
      task("a", "2026-10-01", "2026-10-02", ["b"]),
      task("b", "2026-10-03", "2026-10-04", ["a"]),
    ]);
    expect(r.cycle).toBe(true);
    expect(r.tasks.size).toBe(0);
  });

  it("ignores a dependency on a task that is not there", () => {
    const r = criticalPath([task("a", "2026-10-01", "2026-10-02", ["ghost"])]);
    expect(r.tasks.get("a")?.critical).toBe(true);
  });
});
