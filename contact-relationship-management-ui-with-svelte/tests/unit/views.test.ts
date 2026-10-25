import { describe, expect, it } from "vitest";
import { applyFilter, applyShape, KINDS } from "../../src/lib/views";

const tickets = [
  { pid: "a", priority: "urgent", status: "open", channel: "email" },
  { pid: "b", priority: "low", status: "open", channel: "web" },
  { pid: "c", priority: "high", status: "closed", channel: "email" },
];

describe("saved views", () => {
  it("keeps alternatives within a field and ANDs across fields", () => {
    const view = { filter: { priority: ["urgent", "high"], status: ["open"] } };
    expect(applyFilter(tickets, view).map((t) => t.pid)).toEqual(["a"]);
    expect(applyFilter(tickets, { filter: {} }).length).toBe(3);
    expect(applyFilter(tickets, null).length).toBe(3);
  });

  it("derives a deal's state from when it closed", () => {
    const deals = [
      { closed_at: null, won: false },
      { closed_at: "2026-01-01", won: true },
      { closed_at: "2026-01-02", won: false },
    ];
    const state = (s: string) =>
      applyFilter(deals, { filter: { state: [s] } }).length;
    expect([state("open"), state("won"), state("lost")]).toEqual([1, 1, 1]);
  });

  it("filtering on a masked value matches nothing instead of revealing it", () => {
    const masked = [{ preferred_channel: "•••", status: "active" }];
    expect(
      applyFilter(masked, { filter: { preferred_channel: ["email"] } }),
    ).toEqual([]);
  });

  it("sorts numbers numerically and text with numbers in order, and keeps the view's columns", () => {
    const columns = [{ id: "n" }, { id: "name" }, { id: "x" }];
    const rows = [
      { n: 10, name: "item 10" },
      { n: 9, name: "item 9" },
    ];
    expect(
      applyShape(rows, columns, {
        sort: { field: "n", direction: "asc" },
        columns: null,
      }).rows[0]!.n,
    ).toBe(9);
    expect(
      applyShape(rows, columns, {
        sort: { field: "name", direction: "desc" },
        columns: null,
      }).rows[0]!.n,
    ).toBe(10);
    const shown = applyShape(rows, columns, {
      sort: null,
      columns: ["name", "n"],
    });
    expect(shown.columns.map((c) => c.id)).toEqual(["name", "n"]);
    // A view naming no column the list has still shows the list.
    expect(
      applyShape(rows, columns, { sort: null, columns: ["nope"] }).columns
        .length,
    ).toBe(3);
  });

  it("offers the same fields the service accepts", () => {
    expect(Object.keys(KINDS).sort()).toEqual([
      "account",
      "contact",
      "deal",
      "lead",
      "ticket",
    ]);
    expect(KINDS.ticket.filters.channel).toContain("chat");
  });
});
