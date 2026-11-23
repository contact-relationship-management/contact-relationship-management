// Deals by pipeline and stage, and tickets with their SLA breaches.
import CRMCore
import LilyUI
import SwiftUI

struct DealsView: View {
    let model: AppModel
    @State private var state: Loadable<(boards: [DealBoard], unplaced: [Deal])> = .idle

    var body: some View {
        LoadView(state: state, retry: load) { loaded in
            List {
                DemoBanner(model: model)
                ForEach(loaded.boards) { board in
                    ForEach(board.columns) { column in
                        Section {
                            if column.deals.isEmpty { Text(verbatim: S.empty) }
                            ForEach(column.deals) { DealRow(deal: $0) }
                        } header: {
                            HStack {
                                Text(verbatim: "\(board.pipeline.name) · \(column.stage.name)")
                                Spacer()
                                Badge(String(column.deals.count), label: S.dealCount(column.deals.count))
                            }
                        }
                    }
                }
                if !loaded.unplaced.isEmpty {
                    Section(S.dealsUnplaced) { ForEach(loaded.unplaced) { DealRow(deal: $0) } }
                }
            }
            .refreshable { await load() }
        }
        .navigationTitle(S.tabDeals)
        .task { if case .idle = state { await load() } }
    }

    private func load() async {
        state = .loading
        let result = await model.run({ api throws(APIError) in
            // One after the other: `async let` does not keep the typed error.
            let pipelines = try await api.pipelines()
            return (pipelines, try await api.deals())
        })
        switch result {
        case .success(let (pipelines, deals)): state = .loaded(DealBoard.boards(pipelines: pipelines, deals: deals))
        case .failure(let error): state = .failed(error.message)
        }
    }
}

struct DealRow: View {
    let deal: Deal

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(verbatim: deal.name)
                if let close = deal.expectedCloseOn { Text(verbatim: close).font(.caption) }
            }
            Spacer()
            if deal.won { Tag(S.dealWon, label: S.contactStatus) }
            Text(verbatim: amount).monospacedDigit()
        }
        .accessibilityElement(children: .combine)
    }

    /// MOB-6: exact, and "Hidden" when masked; malformed text says so rather than show a number.
    private var amount: String {
        do {
            return try Money.format(minorText: deal.amountText, currency: deal.currency, locale: .current) ?? S.dealHidden
        } catch {
            return S.t("error.unreadable")
        }
    }
}

struct TicketsView: View {
    let model: AppModel
    @State private var state: Loadable<[Ticket]> = .idle

    var body: some View {
        LoadView(state: state, retry: load) { tickets in
            List {
                DemoBanner(model: model)
                if tickets.isEmpty { Text(verbatim: S.empty) }
                ForEach(tickets) { TicketRow(ticket: $0) }
            }
            .refreshable { await load() }
        }
        .navigationTitle(S.tabTickets)
        .task { if case .idle = state { await load() } }
    }

    private func load() async {
        state = .loading
        switch await model.run({ api throws(APIError) in try await api.tickets() }) {
        case .success(let tickets): state = .loaded(tickets)
        case .failure(let error): state = .failed(error.message)
        }
    }
}

struct TicketRow: View {
    let ticket: Ticket

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(verbatim: ticket.title).font(.headline)
            HStack {
                Tag(ticket.status, label: S.ticketStatus)
                Tag(ticket.priority, label: S.ticketPriority)
            }
            // A breach is words in a badge, never colour alone (MOB-10).
            if ticket.firstResponseBreached { Badge(S.ticketFirstResponseBreached, kind: .error) }
            if ticket.resolutionBreached { Badge(S.ticketResolutionBreached, kind: .error) }
        }
    }
}
