// The server and sign-in settings (MOB-1, MOB-3). A bad address is an error at its field, never corrected.
import CRMCore
import LilyUI
import SwiftUI

struct SettingsView: View {
    let model: AppModel
    @State private var draft = Settings(server: "", issuer: "", clientID: "")
    @State private var serverError: String?
    @State private var issuerError: String?
    @State private var saved = false
    @FocusState private var focus: Field?

    enum Field { case server, issuer }

    var body: some View {
        Form {
            if serverError != nil || issuerError != nil {
                ErrorSummary(
                    title: S.problemTitle,
                    problems: [
                        serverError.map { message in ErrorSummary.Problem(message) { focus = .server } },
                        issuerError.map { message in ErrorSummary.Problem(message) { focus = .issuer } },
                    ].compactMap { $0 })
            }
            if saved { LilyAlert(kind: .success, message: S.settingsSaved, politeness: .polite) }
            Section {
                LilyUI.Field(label: S.settingsServer, description: S.settingsServerHelp, error: serverError, required: true,
                    requiredLabel: S.required) {
                    TextField(S.settingsServer, text: $draft.server, prompt: Text(verbatim: "https://crm.example.org"))
                        .textContentType(.URL)
                        .autocorrectionDisabled()
                        #if os(iOS)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        #endif
                        .focused($focus, equals: .server)
                }
                LilyUI.Field(label: S.settingsIssuer, description: S.settingsIssuerHelp, error: issuerError) {
                    TextField(S.settingsIssuer, text: $draft.issuer)
                        .autocorrectionDisabled()
                        #if os(iOS)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        #endif
                        .focused($focus, equals: .issuer)
                }
                LilyUI.Field(label: S.settingsClient) {
                    TextField(S.settingsClient, text: $draft.clientID)
                        .autocorrectionDisabled()
                        #if os(iOS)
                        .textInputAutocapitalization(.never)
                        #endif
                }
                Button(S.settingsSave, action: save)
            }
            Section {
                InsetText { Text(verbatim: S.settingsStored) }
                if model.token != nil { Button(S.settingsSignOut, role: .destructive) { model.signOut() } }
            }
        }
        .navigationTitle(S.settingsTitle)
        .onAppear { draft = model.settings }
    }

    private func save() {
        saved = false
        let problems = model.apply(draft)
        serverError = problems.server.map(S.serverError)
        issuerError = problems.issuer.map(S.serverError)
        if problems.server == nil, problems.issuer == nil { saved = true }
    }
}
