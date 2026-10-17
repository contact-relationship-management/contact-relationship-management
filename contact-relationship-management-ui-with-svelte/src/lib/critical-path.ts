// Critical-path calculation for a schedule of dated tasks.
//
// The Lily Gantt chart computes nothing: it shows `critical` and `slack` when
// the consumer supplies them. This module works them out from the tasks' own
// dates and dependencies (the critical-path method with the dates held fixed):
//
//   * The finish of the whole schedule is the latest end of any task.
//   * Going backwards from it, each task has a *latest finish*: the finish of
//     the schedule, or earlier when a task that depends on it needs it sooner.
//   * Slack is the days between a task's end and its latest finish. A task with
//     no slack is critical: if it slips, the schedule's finish slips.
//
// Dates are ISO days (YYYY-MM-DD) and counted in whole days. Dependency kinds
// follow the Gantt chart: `e2s` (finish-to-start, the default), `s2s`, `s2e`
// and `e2e`, each with an optional lag in days.

export type DependencyKind = "e2s" | "s2s" | "s2e" | "e2e";

export interface Dependency {
  id: string;
  type?: DependencyKind;
  lag?: number;
}

export interface ScheduledTask {
  id: string;
  start: string;
  end: string;
  /** The tasks this one waits for. */
  dependsOn?: (string | Dependency)[];
}

export interface PathResult {
  /** True when no schedule could be worked out (a dependency loop). */
  cycle: boolean;
  /** The schedule's last day. */
  finish: string | null;
  /** Per task: its slack in days, and whether it is critical (no slack). */
  tasks: Map<string, { slack: number; critical: boolean }>;
}

const DAY = 86_400_000;
const toDay = (iso: string): number => Math.round(Date.parse(`${iso}T00:00:00Z`) / DAY);
const toIso = (day: number): string => new Date(day * DAY).toISOString().slice(0, 10);

const normal = (d: string | Dependency): Required<Dependency> =>
  typeof d === "string"
    ? { id: d, type: "e2s", lag: 0 }
    : { id: d.id, type: d.type ?? "e2s", lag: d.lag ?? 0 };

/** Work out each task's slack and whether it lies on the critical path. */
export function criticalPath(tasks: ScheduledTask[]): PathResult {
  const byId = new Map(tasks.map((t) => [t.id, t]));
  const result: PathResult = { cycle: false, finish: null, tasks: new Map() };
  if (tasks.length === 0) return result;

  // successors[p] = the tasks that wait for p, with how.
  const successors = new Map<string, { id: string; type: DependencyKind; lag: number }[]>();
  for (const task of tasks) {
    for (const raw of task.dependsOn ?? []) {
      const dep = normal(raw);
      if (!byId.has(dep.id)) continue; // a dependency on an unknown task is ignored
      const list = successors.get(dep.id) ?? [];
      list.push({ id: task.id, type: dep.type, lag: dep.lag });
      successors.set(dep.id, list);
    }
  }

  // Order so every task comes after the tasks that wait for it (reverse topological).
  const order: string[] = [];
  const state = new Map<string, 1 | 2>();
  const visit = (id: string): boolean => {
    if (state.get(id) === 2) return true;
    if (state.get(id) === 1) return false; // a loop
    state.set(id, 1);
    for (const next of successors.get(id) ?? []) if (!visit(next.id)) return false;
    state.set(id, 2);
    order.push(id); // successors are already in `order`
    return true;
  };
  for (const task of tasks) {
    if (!visit(task.id)) return { ...result, cycle: true };
  }

  const finish = Math.max(...tasks.map((t) => toDay(t.end)));
  result.finish = toIso(finish);

  const latestFinish = new Map<string, number>();
  for (const id of order) {
    const task = byId.get(id)!;
    const duration = toDay(task.end) - toDay(task.start);
    let lf = finish;
    for (const next of successors.get(id) ?? []) {
      const succ = byId.get(next.id)!;
      const succLf = latestFinish.get(next.id)!;
      const succLs = succLf - (toDay(succ.end) - toDay(succ.start));
      switch (next.type) {
        case "e2s":
          // Days are inclusive: a task ending on the 5th lets the next start on the 6th.
          lf = Math.min(lf, succLs - 1 - next.lag);
          break;
        case "e2e":
          lf = Math.min(lf, succLf - next.lag);
          break;
        case "s2s":
          lf = Math.min(lf, succLs - next.lag + duration);
          break;
        case "s2e":
          lf = Math.min(lf, succLf - next.lag + duration);
          break;
      }
    }
    latestFinish.set(id, lf);
    const slack = Math.max(0, lf - toDay(task.end));
    result.tasks.set(id, { slack, critical: slack === 0 });
  }
  return result;
}
