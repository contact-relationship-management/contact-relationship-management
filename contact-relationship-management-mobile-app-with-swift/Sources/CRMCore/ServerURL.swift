// MOB-3: the server URL (and the issuer URL) is checked, never corrected.
import Foundation

public enum ServerURLError: Error, Equatable, Sendable {
    case empty
    case unreadable
    case notHTTPS
    case noHost
    case hasCredentials
    case hasQueryOrFragment
}

/// A base URL that passed MOB-3: `https`, or `http` to a loopback host; no user, password, query or fragment.
public struct ServerURL: Sendable, Hashable, CustomStringConvertible {
    /// The URL with no trailing `/`.
    public let url: URL

    /// The hosts `http` is allowed to: this device, and the Android emulator's alias for its host.
    public static let loopbackHosts: Set<String> = ["localhost", "127.0.0.1", "::1", "10.0.2.2"]

    public init(_ text: String) throws(ServerURLError) {
        if text.isEmpty { throw .empty }
        guard let parts = URLComponents(string: text), let scheme = parts.scheme?.lowercased() else {
            throw .unreadable
        }
        guard let host = parts.host, !host.isEmpty else { throw .noHost }
        if parts.user != nil || parts.password != nil { throw .hasCredentials }
        if parts.query != nil || parts.fragment != nil { throw .hasQueryOrFragment }
        let loopback = Self.loopbackHosts.contains(host.lowercased().trimmingCharacters(in: ["[", "]"]))
        guard scheme == "https" || (scheme == "http" && loopback) else { throw .notHTTPS }
        var trimmed = text
        if trimmed.hasSuffix("/") { trimmed.removeLast() }
        guard let url = URL(string: trimmed) else { throw .unreadable }
        self.url = url
    }

    public var description: String { url.absoluteString }

    /// `path` (which starts with `/`) on this server, keeping any path the server URL has.
    public func appending(_ path: String) -> URL? {
        URL(string: url.absoluteString + path)
    }

    /// Whether `other` has this server's scheme, host and port (MOB-8).
    public func isSameOrigin(_ other: URL) -> Bool {
        other.scheme?.lowercased() == url.scheme?.lowercased()
            && other.host?.lowercased() == url.host?.lowercased()
            && other.port == url.port
    }
}
