// MOB-4: OIDC authorization code with PKCE (RFC 7636 S256), a public client, the system browser (RFC 8252).
import CryptoKit
import Foundation

public enum SignInError: Error, Equatable, Sendable {
    case stateMismatch
    case providerError(String)
    case missingCode
    case unreadableCallback
    case discovery(String)
    case tokenExchange(String)
}

public enum PKCE {
    /// A new code verifier: 32 random bytes, base64url, 43 characters.
    public static func newVerifier() -> String {
        var generator = SystemRandomNumberGenerator()
        let bytes = (0..<32).map { _ in UInt8.random(in: .min ... .max, using: &generator) }
        return base64URL(Data(bytes))
    }

    /// The S256 challenge for `verifier`.
    public static func challenge(for verifier: String) -> String {
        base64URL(Data(SHA256.hash(data: Data(verifier.utf8))))
    }

    /// Base64url without padding (RFC 7636 appendix A).
    public static func base64URL(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}

/// The endpoints from `/.well-known/openid-configuration`.
public struct OIDCConfiguration: Codable, Sendable, Equatable {
    public var issuer: String
    public var authorizationEndpoint: URL
    public var tokenEndpoint: URL
}

extension OIDCConfiguration {
    /// OpenID Connect Discovery 1.0 §4.3: the document must name the issuer it was fetched from; and both endpoints must
    /// pass the server URL check, so the code and the verifier never go over plain `http` to another host.
    public func check(issuer expected: ServerURL) throws(SignInError) {
        var named = issuer
        if named.hasSuffix("/") { named.removeLast() }
        guard named == expected.description else { throw .discovery("the document names another issuer: \(issuer)") }
        for endpoint in [authorizationEndpoint, tokenEndpoint] {
            var parts = URLComponents(url: endpoint, resolvingAgainstBaseURL: false)
            parts?.query = nil
            guard let text = parts?.string, (try? ServerURL(text)) != nil else {
                throw .discovery("an endpoint is not https: \(endpoint.absoluteString)")
            }
        }
    }
}

/// One sign-in attempt: what goes to the browser, and what must come back.
public struct AuthorizationRequest: Sendable, Equatable {
    public static let redirectURI = "io.github.contactrelationshipmanagement.mobile:/oauth/callback"
    public static let callbackScheme = "io.github.contactrelationshipmanagement.mobile"

    public let clientID: String
    public let state: String
    public let verifier: String

    public init(clientID: String, state: String = PKCE.newVerifier(), verifier: String = PKCE.newVerifier()) {
        self.clientID = clientID
        self.state = state
        self.verifier = verifier
    }

    /// The URL to open in the system browser.
    public func url(configuration: OIDCConfiguration) -> URL? {
        guard var parts = URLComponents(url: configuration.authorizationEndpoint, resolvingAgainstBaseURL: false) else {
            return nil
        }
        parts.queryItems = (parts.queryItems ?? []) + [
            URLQueryItem(name: "response_type", value: "code"),
            URLQueryItem(name: "client_id", value: clientID),
            URLQueryItem(name: "redirect_uri", value: Self.redirectURI),
            URLQueryItem(name: "scope", value: "openid"),
            URLQueryItem(name: "state", value: state),
            URLQueryItem(name: "code_challenge", value: PKCE.challenge(for: verifier)),
            URLQueryItem(name: "code_challenge_method", value: "S256"),
        ]
        return parts.url
    }

    /// The authorization code from the redirect, once `state` matches and there is no `error`.
    public func code(from callback: URL) throws(SignInError) -> String {
        guard let parts = URLComponents(url: callback, resolvingAgainstBaseURL: false),
            parts.scheme == Self.callbackScheme
        else { throw .unreadableCallback }
        let items = parts.queryItems ?? []
        func value(_ name: String) -> String? { items.first { $0.name == name }?.value }
        guard value("state") == state else { throw .stateMismatch }
        if let error = value("error") { throw .providerError(value("error_description") ?? error) }
        guard let code = value("code"), !code.isEmpty else { throw .missingCode }
        return code
    }

    /// The form body that exchanges `code` for tokens.
    public func tokenRequestBody(code: String) -> Data {
        var parts = URLComponents()
        parts.queryItems = [
            URLQueryItem(name: "grant_type", value: "authorization_code"),
            URLQueryItem(name: "code", value: code),
            URLQueryItem(name: "redirect_uri", value: Self.redirectURI),
            URLQueryItem(name: "client_id", value: clientID),
            URLQueryItem(name: "code_verifier", value: verifier),
        ]
        // `+` is a space in a form body, so it must be escaped where URLComponents leaves it.
        let encoded = parts.percentEncodedQuery?.replacingOccurrences(of: "+", with: "%2B") ?? ""
        return Data(encoded.utf8)
    }
}

public struct TokenResponse: Codable, Sendable, Equatable {
    public var accessToken: String
    public var tokenType: String?
    public var expiresIn: Int?
}
