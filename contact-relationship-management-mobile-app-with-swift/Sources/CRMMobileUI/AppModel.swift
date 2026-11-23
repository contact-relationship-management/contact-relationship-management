// The app's state, in memory (MOB-1, MOB-2). Only `Settings` goes to UserDefaults; records and the token never leave
// the process.
import CRMCore
import Foundation
import Observation

/// What is stored on the device: the server, and where to sign in. None of it is a secret.
public struct Settings: Equatable, Sendable {
    public var server: String
    public var issuer: String
    public var clientID: String

    static let keys = (server: "crm.server", issuer: "crm.issuer", clientID: "crm.clientID")

    public static func load(from defaults: UserDefaults = .standard) -> Settings {
        Settings(
            server: defaults.string(forKey: keys.server) ?? "",
            issuer: defaults.string(forKey: keys.issuer) ?? "",
            clientID: defaults.string(forKey: keys.clientID) ?? "")
    }

    public func save(to defaults: UserDefaults = .standard) {
        defaults.set(server, forKey: Self.keys.server)
        defaults.set(issuer, forKey: Self.keys.issuer)
        defaults.set(clientID, forKey: Self.keys.clientID)
    }
}

/// A screen's load state.
enum Loadable<Value> {
    case idle
    case loading
    case loaded(Value)
    case failed(String)
}

@MainActor
@Observable
public final class AppModel {
    public private(set) var settings: Settings
    private(set) var server: ServerURL?
    private(set) var issuer: ServerURL?
    /// The access token, in memory only (MOB-2).
    private(set) var token: String?
    /// The server answered without a token (MOB-5).
    private(set) var isDemo = false
    /// The last request answered `401`.
    private(set) var needsSignIn = false

    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        settings = Settings.load(from: defaults)
        server = try? ServerURL(settings.server)
        issuer = settings.issuer.isEmpty ? nil : try? ServerURL(settings.issuer)
    }

    var isConfigured: Bool { server != nil }
    /// Whether the user must sign in before anything is fetched.
    var mustSignIn: Bool { issuer != nil && (token == nil || needsSignIn) }

    var api: APIClient? { server.map { APIClient(server: $0, token: token) } }

    /// Checks and keeps `settings`; on a problem nothing changes and the problems come back by field.
    func apply(_ candidate: Settings) -> (server: ServerURLError?, issuer: ServerURLError?) {
        let server = Result { () throws(ServerURLError) in try ServerURL(candidate.server) }
        let issuer: Result<ServerURL?, ServerURLError> =
            candidate.issuer.isEmpty ? .success(nil) : Result { () throws(ServerURLError) in try ServerURL(candidate.issuer) }
        guard case .success(let newServer) = server, case .success(let newIssuer) = issuer else {
            return (server.failure, issuer.failure)
        }
        settings = candidate
        settings.save(to: defaults)
        if newServer != self.server || newIssuer != self.issuer { signOut() }
        self.server = newServer
        self.issuer = newIssuer
        return (nil, nil)
    }

    func signedIn(_ token: String) {
        self.token = token
        needsSignIn = false
    }

    func signOut() {
        token = nil
        isDemo = false
        needsSignIn = false
    }

    /// Runs `call`, noting a demo server and a lapsed sign-in on the way.
    func run<T: Sendable>(_ call: @Sendable (APIClient) async throws(APIError) -> T) async -> Result<T, APIError> {
        guard let api else { return .failure(.badURL) }
        do {
            let value = try await call(api)
            if api.token == nil { isDemo = true }
            return .success(value)
        } catch {
            if error == .unauthorized { needsSignIn = true; isDemo = false }
            return .failure(error)
        }
    }
}

extension Result {
    var failure: Failure? { if case .failure(let error) = self { error } else { nil } }
}

extension APIError {
    /// The message to show (MOB-9).
    var message: String {
        switch self {
        case .unauthorized: S.signInNoIssuer
        case .problem(_, let message): message
        case .tooManyPages: S.t("error.tooManyPages")
        case .unsafeNextLink: S.t("error.unsafeNextLink")
        case .unreadable: S.t("error.unreadable")
        case .transport(let detail): detail
        case .badURL: S.serverError(.unreadable)
        }
    }
}
