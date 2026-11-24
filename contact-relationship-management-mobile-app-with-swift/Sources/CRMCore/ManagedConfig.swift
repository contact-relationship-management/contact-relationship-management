// Managed app configuration (MOB-14): the server, the issuer and the client ID a deployer may supply under
// `com.apple.configuration.managed`. A key that is not supplied is absent; one that is supplied is checked like a typed
// value, blank or not, and is never replaced by a default or by what the user typed (RCA-IN-2).
import Foundation

public enum ManagedConfig: Equatable, Sendable {
    /// Nothing supplied: the user's own settings apply.
    case none
    /// Supplied and valid: shown as fixed.
    case managed(server: String, issuer: String, clientID: String)
    /// Supplied and not valid: an error at start-up.
    case invalid(server: ServerURLError?, issuer: ServerURLError?, clientIDMissing: Bool)

    public static let key = "com.apple.configuration.managed"
    public static let keys = (server: "server_url", issuer: "issuer_url", clientID: "client_id")

    /// A value that is present but not a string is read as blank, so it fails the check.
    public static func read(_ values: [String: Any]?) -> ManagedConfig {
        guard let values else { return .none }
        func supplied(_ name: String) -> String? { values[name].map { ($0 as? String) ?? "" } }
        let server = supplied(keys.server)
        let issuer = supplied(keys.issuer)
        let clientID = supplied(keys.clientID)
        if server == nil, issuer == nil, clientID == nil { return .none }
        let serverError = problem(server ?? "")
        let issuerError = (issuer ?? "").isEmpty ? nil : problem(issuer ?? "")
        let clientIDMissing = !(issuer ?? "").isEmpty && (clientID ?? "").trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        if serverError != nil || issuerError != nil || clientIDMissing {
            return .invalid(server: serverError, issuer: issuerError, clientIDMissing: clientIDMissing)
        }
        return .managed(server: server ?? "", issuer: issuer ?? "", clientID: clientID ?? "")
    }

    public static func read(from defaults: UserDefaults = .standard) -> ManagedConfig {
        read(defaults.dictionary(forKey: key))
    }

    private static func problem(_ text: String) -> ServerURLError? {
        do throws(ServerURLError) { _ = try ServerURL(text) } catch { return error }
        return nil
    }
}
