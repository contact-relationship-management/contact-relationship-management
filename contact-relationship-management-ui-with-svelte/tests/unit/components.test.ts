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
});
