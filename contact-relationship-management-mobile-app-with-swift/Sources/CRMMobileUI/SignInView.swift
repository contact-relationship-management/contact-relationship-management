// MOB-4: OIDC with PKCE in the system browser (ASWebAuthenticationSession, through SwiftUI's environment).
import AuthenticationServices
import CRMCore
import LilyUI
import SwiftUI

struct SignInView: View {
    let model: AppModel
    @Environment(\.webAuthenticationSession) private var webAuthenticationSession
    @State private var problem: String?
    @State private var working = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            if let problem { LilyAlert(kind: .error, heading: S.problemTitle, message: problem) }
            Text(verbatim: S.signInBody)
            Button(S.signInButton) { Task { await signIn() } }
                .buttonStyle(.borderedProminent)
                .disabled(working)
            Spacer()
        }
        .padding()
        .navigationTitle(S.signInTitle)
        .toolbar { NavigationLink(S.tabSettings) { SettingsView(model: model) } }
    }

    private func signIn() async {
        guard let issuer = model.issuer else { return }
        working = true
        defer { working = false }
        problem = nil
        let oidc = OIDCClient(issuer: issuer)
        let attempt = AuthorizationRequest(clientID: model.settings.clientID)
        do {
            let configuration = try await oidc.configuration()
            guard let url = attempt.url(configuration: configuration) else { throw SignInError.discovery("") }
            let callback = try await webAuthenticationSession.authenticate(
                using: url, callbackURLScheme: AuthorizationRequest.callbackScheme, preferredBrowserSession: .ephemeral)
            let code = try attempt.code(from: callback)
            let tokens = try await oidc.exchange(code: code, for: attempt, at: configuration)
            model.signedIn(tokens.accessToken)
        } catch ASWebAuthenticationSessionError.canceledLogin {
            // The user closed the sign-in page; nothing to say.
        } catch {
            problem = S.f("error.signIn", String(describing: error))
        }
    }
}
