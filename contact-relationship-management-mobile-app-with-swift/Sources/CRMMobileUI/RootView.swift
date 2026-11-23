// The app's screens: settings until a server is set, sign-in when the server needs it, then the tabs.
import CRMCore
import LilyThemes
import LilyUI
import SwiftUI

public struct RootView: View {
    @State private var model = AppModel()
    @Environment(\.colorScheme) private var colorScheme

    public init() {}

    public var body: some View {
        Group {
            if !model.isConfigured {
                NavigationStack { SettingsView(model: model) }
            } else if model.mustSignIn {
                NavigationStack { SignInView(model: model) }
            } else {
                TabView {
                    Tab(S.tabContacts, systemImage: "person.2") { NavigationStack { ContactsView(model: model) } }
                    Tab(S.tabDeals, systemImage: "chart.bar.doc.horizontal") { NavigationStack { DealsView(model: model) } }
                    Tab(S.tabTickets, systemImage: "lifepreserver") { NavigationStack { TicketsView(model: model) } }
                    Tab(S.tabSettings, systemImage: "gear") { NavigationStack { SettingsView(model: model) } }
                }
            }
        }
        .lilyTheme(colorScheme == .dark ? .dark : .light)
    }
}

/// The demo warning (MOB-5), shown above a list while the server answers without a token.
struct DemoBanner: View {
    let model: AppModel
    var body: some View {
        if model.isDemo {
            LilyAlert(kind: .warning, heading: S.demoHeading, message: S.demoBody, politeness: .polite)
        }
    }
}

/// A list screen's loading, failure and empty states around its content.
struct LoadView<Value, Content: View>: View {
    let state: Loadable<Value>
    let retry: () async -> Void
    @ViewBuilder let content: (Value) -> Content

    var body: some View {
        switch state {
        case .idle, .loading:
            ProgressView(S.loading).frame(maxWidth: .infinity, maxHeight: .infinity)
        case .failed(let message):
            VStack(alignment: .leading, spacing: 16) {
                LilyAlert(kind: .error, heading: S.problemTitle, message: message)
                Button(S.retry) { Task { await retry() } }
            }
            .padding()
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        case .loaded(let value):
            content(value)
        }
    }
}
