// Contacts: every page of the list, filtered by name on the phone; a contact with its activities, deals and tickets.
import CRMCore
import LilyUI
import SwiftUI

struct ContactsView: View {
    let model: AppModel
    @State private var state: Loadable<[Contact]> = .idle
    @State private var filter = ""

    var body: some View {
        LoadView(state: state, retry: load) { contacts in
            let shown = filter.isEmpty ? contacts : contacts.filter { $0.displayName.localizedStandardContains(filter) }
            List {
                DemoBanner(model: model)
                if shown.isEmpty { Text(verbatim: S.empty) }
                ForEach(shown) { contact in
                    NavigationLink(value: contact) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(verbatim: contact.displayName).font(.headline)
                            if let title = contact.jobTitle { Text(verbatim: title).font(.subheadline) }
                        }
                    }
                }
            }
            .searchable(text: $filter, prompt: Text(verbatim: S.contactsSearch))
            .refreshable { await load() }
        }
        .navigationTitle(S.tabContacts)
        .navigationDestination(for: Contact.self) { ContactDetailView(model: model, pid: $0.pid) }
        .task { if case .idle = state { await load() } }
    }

    private func load() async {
        state = .loading
        switch await model.run({ api throws(APIError) in try await api.contacts() }) {
        case .success(let contacts): state = .loaded(contacts)
        case .failure(let error): state = .failed(error.message)
        }
    }
}

struct ContactDetailView: View {
    let model: AppModel
    let pid: UUID
    @State private var state: Loadable<ContactDetail> = .idle
    @State private var logging = false

    var body: some View {
        LoadView(state: state, retry: load) { detail in
            List {
                Section {
                    Card(heading: detail.contact.displayName, headingLevel: .h2) {
                        SummaryList(label: S.contactSummary) {
                            if let title = detail.contact.jobTitle {
                                SummaryListItem(term: S.contactJobTitle) { Text(verbatim: title) }
                            }
                            SummaryListItem(term: S.contactStatus) { Text(verbatim: detail.contact.status) }
                            SummaryListItem(term: S.contactConsent) {
                                Badge(S.consent(detail.contact.marketingConsent), kind: consentKind(detail.contact.marketingConsent))
                            }
                            SummaryListItem(term: S.contactChannel) { Text(verbatim: detail.contact.preferredChannel) }
                        }
                    }
                    InsetText { Text(verbatim: S.contactConsentNote) }
                }
                Section(S.contactActivities) {
                    if detail.activities.isEmpty { Text(verbatim: S.empty) }
                    ForEach(detail.activities) { activity in
                        VStack(alignment: .leading, spacing: 4) {
                            Tag(S.kind(activity.kind), label: S.logKind)
                            Text(verbatim: activity.summary)
                            Text(verbatim: activity.occurredAt).font(.caption)
                        }
                    }
                }
                Section(S.contactDeals) {
                    if detail.deals.isEmpty { Text(verbatim: S.empty) }
                    ForEach(detail.deals) { DealRow(deal: $0) }
                }
                Section(S.contactTickets) {
                    if detail.tickets.isEmpty { Text(verbatim: S.empty) }
                    ForEach(detail.tickets) { TicketRow(ticket: $0) }
                }
            }
            .refreshable { await load() }
        }
        .navigationTitle(title)
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        #endif
        .toolbar { Button(S.logTitle) { logging = true } }
        .sheet(isPresented: $logging) {
            NavigationStack {
                LogActivityView(model: model, contact: pid) {
                    logging = false
                    Task { await load() }
                }
            }
        }
        .task { if case .idle = state { await load() } }
    }

    private var title: String {
        if case .loaded(let detail) = state { detail.contact.displayName } else { S.tabContacts }
    }

    private func load() async {
        state = .loading
        let pid = pid
        switch await model.run({ api throws(APIError) in try await api.contact(pid) }) {
        case .success(let detail): state = .loaded(detail)
        case .failure(let error): state = .failed(error.message)
        }
    }

    private func consentKind(_ consent: String) -> BadgeKind {
        switch consent {
        // The service's CONSENT_STATES; the badge's words say the state, the kind only adds colour (MOB-10).
        case "granted": .success
        case "withdrawn": .warning
        case "pending": .info
        default: .default
        }
    }
}

/// MOB-7: the form keeps one `Idempotency-Key`, so a retry after a dropped answer records one activity.
struct LogActivityView: View {
    let model: AppModel
    let contact: UUID
    let done: () -> Void
    @State private var kind: ActivityKind = .call
    @State private var summary = ""
    @State private var summaryError: String?
    @State private var failure: String?
    @State private var saving = false
    @State private var idempotencyKey = UUID().uuidString.lowercased()
    @FocusState private var summaryFocused: Bool

    var body: some View {
        Form {
            if let summaryError {
                ErrorSummary(title: S.problemTitle, problems: [.init(summaryError) { summaryFocused = true }])
            }
            if let failure { LilyAlert(kind: .error, heading: S.problemTitle, message: failure) }
            LilyUI.Field(label: S.logKind) {
                Picker(S.logKind, selection: $kind) {
                    ForEach(ActivityKind.allCases, id: \.self) { Text(verbatim: S.kind($0.rawValue)).tag($0) }
                }
            }
            LilyUI.Field(label: S.logSummary, error: summaryError, required: true, requiredLabel: S.required) {
                TextField(S.logSummary, text: $summary, axis: .vertical)
                    .lineLimit(3...8)
                    // MOB-15: iOS offers no switch for keyboard learning on a plain field, so correction and
                    // prediction are off for it.
                    .autocorrectionDisabled()
                    .focused($summaryFocused)
            }
        }
        .navigationTitle(S.logTitle)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button(S.logCancel, action: done) }
            ToolbarItem(placement: .confirmationAction) {
                Button(S.logSave) { Task { await save() } }.disabled(saving)
            }
        }
        // A change to the form is a new activity, so it gets a new key; a retry of the same form keeps it.
        .onChange(of: kind) { idempotencyKey = UUID().uuidString.lowercased() }
        .onChange(of: summary) { idempotencyKey = UUID().uuidString.lowercased() }
    }

    private func save() async {
        let text = summary.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else {
            summaryError = S.logSummaryMissing
            return
        }
        summaryError = nil
        failure = nil
        saving = true
        defer { saving = false }
        let activity = NewActivity(kind: kind, contact: contact, summary: text)
        let key = idempotencyKey
        switch await model.run({ api throws(APIError) in try await api.log(activity, idempotencyKey: key) }) {
        case .success: done()
        case .failure(let error): failure = error.message
        }
    }
}
