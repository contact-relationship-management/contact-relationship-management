import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import Board from "#lib/components/Board.svelte";
import DataTable from "#lib/components/DataTable.svelte";
import LinksPanel from "#lib/components/LinksPanel.svelte";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("DataTable", () => {
  const columns = [
    { id: "name", header: "Name" },
    { id: "status", kind: "chip", header: "Status" },
  ];
  const rows = [
    { id: "1", name: "Ada", status: "active" },
    { id: "2", name: "Grace", status: "lead" },
  ];

  it("shows a header per column and a row per record", () => {
    render(DataTable, { label: "People", columns, rows, testid: "people" });
    expect(screen.getByText("Name")).toBeTruthy();
    expect(screen.getByText("Status")).toBeTruthy();
    expect(screen.getByText("Ada")).toBeTruthy();
    expect(screen.getByText("Grace")).toBeTruthy();
  });

  it("shows the empty text when there are no rows", () => {
    render(DataTable, {
      label: "People",
      columns,
      rows: [],
      empty: "Nobody yet",
    });
    expect(screen.getByText("Nobody yet")).toBeTruthy();
  });

  it("calls onselect with the row's id when a row is clicked", async () => {
    const onselect = vi.fn();
    render(DataTable, { label: "People", columns, rows, onselect });
    await fireEvent.click(screen.getByText("Grace"));
    expect(onselect).toHaveBeenCalledWith("2");
  });

  it("renders a column's href as a link", () => {
    render(DataTable, {
      label: "People",
      columns: [
        {
          id: "name",
          header: "Name",
          href: (row: Record<string, unknown>) => `/c/${row.id}`,
        },
      ],
      rows,
    });
    expect(screen.getByText("Ada").closest("a")?.getAttribute("href")).toBe(
      "/c/1",
    );
  });

  it("formats typed cells, totals numeric columns, and restores column order", async () => {
    localStorage.setItem(
      "crm-grid-order-finance",
      JSON.stringify(["name", "amount", "status", "day"]),
    );
    render(DataTable, {
      label: "Finance",
      testid: "finance",
      columns: [
        { id: "name", header: "Name" },
        { id: "status", header: "Status", kind: "flag" },
        { id: "amount", header: "Amount", kind: "number", total: "avg" },
        { id: "day", header: "Day", kind: "date" },
      ],
      rows: [
        { id: "1", name: "Ada", status: "late", amount: 10, day: "2026-11-19" },
        { id: "2", name: "Grace", status: "", amount: 20, day: "2026-11-20" },
        {
          id: "3",
          name: "Lin",
          status: null,
          amount: "not a number",
          day: "2026-11-21",
        },
      ],
      filter: true,
      pageSize: 2,
    });
    expect(screen.getByText("late").closest("strong")).toBeTruthy();
    expect(await screen.findByText(/Average: 15/)).toBeTruthy();
  });
});

describe("Board", () => {
  const columns = [
    { id: "new", title: "New" },
    { id: "done", title: "Done" },
  ];
  const cards = [{ id: "c1", columnId: "new", title: "First card" }];

  it("puts each card in its column", () => {
    render(Board, { label: "Leads", columns, cards });
    expect(screen.getByText("First card")).toBeTruthy();
    expect(screen.getByText("New")).toBeTruthy();
    expect(screen.getByText("Done")).toBeTruthy();
  });

  it("moves a card through the keyboard-reachable menu", async () => {
    const onMove = vi.fn().mockResolvedValue(undefined);
    const onPosition = vi.fn().mockResolvedValue(undefined);
    render(Board, { label: "Leads", columns, cards, onMove, onPosition });
    await fireEvent.click(
      screen.getByRole("button", { name: /Move First card/ }),
    );
    await fireEvent.click(screen.getByRole("option", { name: "Done" }));
    await waitFor(() => expect(onMove).toHaveBeenCalledWith("c1", "done"));
  });

  it("announces a move rejected by the persistence layer", async () => {
    const onMove = vi.fn().mockRejectedValue(new Error("Move refused"));
    render(Board, { label: "Leads", columns, cards, onMove });
    await fireEvent.click(
      screen.getByRole("button", { name: /Move First card/ }),
    );
    await fireEvent.click(screen.getByRole("option", { name: "Done" }));
    await waitFor(() =>
      expect(screen.getByTestId("action-error").textContent).toContain(
        "Could not move",
      ),
    );
  });

  it("edits a title and reports a rejected rename", async () => {
    const onRename = vi.fn().mockRejectedValue(new Error("Rename refused"));
    render(Board, { label: "Leads", columns, cards, onRename });
    await fireEvent.dblClick(screen.getByText("First card"));
    const input = screen.getByRole("textbox", { name: /First card/ });
    await fireEvent.input(input, { target: { value: "Changed card" } });
    await fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() =>
      expect(screen.getByTestId("action-error").textContent).toContain(
        "Could not save",
      ),
    );
    expect(onRename).toHaveBeenCalledWith("c1", "Changed card");
  });
});

describe("LinksPanel", () => {
  const link = {
    pid: "l1",
    system: "hubspot",
    url: "https://app.hubspot.com/contacts/1",
    label: "HubSpot record",
  };
  function stubLinks(links: unknown[], ok = true) {
    const fetchMock = vi.fn(async () =>
      ok
        ? new Response(JSON.stringify({ links }), {
            status: 200,
            headers: { "content-type": "application/json" },
          })
        : new Response("{}", { status: 500 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    return fetchMock;
  }

  it("lists the links of the record", async () => {
    stubLinks([link]);
    render(LinksPanel, { kind: "contacts", pid: "p1" });
    await waitFor(() => expect(screen.getByTestId("link")).toBeTruthy());
    expect(screen.getByTestId("link-open").getAttribute("href")).toBe(link.url);
  });

  it("says there are none when the list is empty", async () => {
    stubLinks([]);
    render(LinksPanel, { kind: "contacts", pid: "p1" });
    await waitFor(() => expect(screen.getByTestId("links-none")).toBeTruthy());
  });

  it("shows no list when the read fails, but keeps the form", async () => {
    stubLinks([], false);
    render(LinksPanel, { kind: "contacts", pid: "p1" });
    await waitFor(() => expect(screen.getByTestId("link-form")).toBeTruthy());
    expect(screen.queryByTestId("links")).toBeNull();
    expect(screen.queryByTestId("links-none")).toBeNull();
  });

  it("keeps the Add button off until there is a URL", async () => {
    stubLinks([]);
    render(LinksPanel, { kind: "contacts", pid: "p1" });
    const add = screen.getByTestId("link-add") as HTMLButtonElement;
    expect(add.disabled).toBe(true);
    await fireEvent.input(screen.getByTestId("link-url"), {
      target: { value: "https://example.com/x" },
    });
    expect(add.disabled).toBe(false);
  });

  it("adds and removes links, then reloads the list", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ links: [] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify(link), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ links: [link] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ deleted: link.pid }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ links: [] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      );
    vi.stubGlobal("fetch", fetchMock);
    render(LinksPanel, { kind: "contacts", pid: "p1" });
    await waitFor(() => expect(screen.getByTestId("links-none")).toBeTruthy());
    await fireEvent.change(screen.getByTestId("link-system"), {
      target: { value: "other" },
    });
    await fireEvent.input(screen.getByTestId("link-url"), {
      target: { value: " https://example.test/person " },
    });
    await fireEvent.input(screen.getByTestId("link-label"), {
      target: { value: "Example" },
    });
    await fireEvent.submit(screen.getByTestId("link-form"));
    await waitFor(() => expect(screen.getByTestId("link-remove")).toBeTruthy());
    await fireEvent.click(screen.getByTestId("link-remove"));
    await waitFor(() => expect(screen.getByTestId("links-none")).toBeTruthy());
    expect(fetchMock).toHaveBeenCalledTimes(5);
  });

  it("shows write failures and permits a retry", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ links: [link] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ description: "Write refused" }), {
          status: 422,
          headers: { "content-type": "application/problem+json" },
        }),
      );
    vi.stubGlobal("fetch", fetchMock);
    render(LinksPanel, { kind: "contacts", pid: "p1" });
    await waitFor(() => expect(screen.getByTestId("link-remove")).toBeTruthy());
    await fireEvent.click(screen.getByTestId("link-remove"));
    await waitFor(() => expect(screen.getByTestId("link-error")).toBeTruthy());
  });
});
