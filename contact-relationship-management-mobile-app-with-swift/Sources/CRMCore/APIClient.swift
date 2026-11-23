// The service's REST API, as the app uses it. Every request sends `Accepts-version: 1.0` and the bearer token when there
// is one; nothing is cached (MOB-1); a list is whole or an error (MOB-8); a POST carries an `Idempotency-Key` (MOB-7).
import Foundation

public struct APIClient: Sendable {
    public static let apiVersion = "1.0"

    public let server: ServerURL
    public let token: String?
    private let session: URLSession

    public init(server: ServerURL, token: String?, session: URLSession = APIClient.ephemeralSession()) {
        self.server = server
        self.token = token
        self.session = session
    }

    /// A session that keeps nothing: no cache, no cookies, no credential store (MOB-1).
    public static func ephemeralSession(protocolClasses: [AnyClass]? = nil) -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.urlCache = nil
        configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
        configuration.httpCookieStorage = nil
        configuration.urlCredentialStorage = nil
        if let protocolClasses { configuration.protocolClasses = protocolClasses }
        return URLSession(configuration: configuration)
    }

    // MARK: - Endpoints

    public func contacts() async throws(APIError) -> [Contact] { try await list("/api/contacts") }
    public func deals() async throws(APIError) -> [Deal] { try await list("/api/deals") }
    public func tickets() async throws(APIError) -> [Ticket] { try await list("/api/tickets") }
    public func pipelines() async throws(APIError) -> [Pipeline] { try await get("/api/pipelines") }

    public func contact(_ pid: UUID) async throws(APIError) -> ContactDetail {
        try await get("/api/contacts/\(pid.uuidString.lowercased())")
    }

    /// Records `activity`. Pass the same `idempotencyKey` when retrying the same form, so it is recorded once.
    @discardableResult
    public func log(_ activity: NewActivity, idempotencyKey: String) async throws(APIError) -> Activity {
        guard let url = server.appending("/api/activities") else { throw .badURL }
        var request = request(url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue(idempotencyKey, forHTTPHeaderField: "Idempotency-Key")
        do { request.httpBody = try JSONEncoder.crm.encode(activity) } catch { throw .unreadable("\(error)") }
        let (data, _) = try await send(request)
        return try decode(data)
    }

    // MARK: - Plumbing

    func get<T: Decodable>(_ path: String) async throws(APIError) -> T {
        guard let url = server.appending(path) else { throw .badURL }
        let (data, _) = try await send(request(url))
        return try decode(data)
    }

    /// Every page of a list, following `Link: rel="next"` on this server only.
    func list<T: Decodable>(_ path: String) async throws(APIError) -> [T] {
        guard var next = server.appending(path) else { throw .badURL }
        var rows: [T] = []
        for _ in 0..<Paging.maxPages {
            let (data, response) = try await send(request(next))
            let page: [T] = try decode(data)
            rows.append(contentsOf: page)
            guard let link = Paging.nextLink(response.value(forHTTPHeaderField: "Link")) else { return rows }
            guard let resolved = Paging.resolve(link, against: server) else { throw .unsafeNextLink(link) }
            next = resolved
        }
        throw .tooManyPages
    }

    func request(_ url: URL) -> URLRequest {
        var request = URLRequest(url: url, cachePolicy: .reloadIgnoringLocalCacheData)
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue(Self.apiVersion, forHTTPHeaderField: "Accepts-version")
        if let token { request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization") }
        return request
    }

    func send(_ request: URLRequest) async throws(APIError) -> (Data, HTTPURLResponse) {
        let data: Data
        let response: URLResponse
        do { (data, response) = try await session.data(for: request, delegate: NoRedirects.shared) } catch {
            throw .transport(error.localizedDescription)
        }
        guard let http = response as? HTTPURLResponse else { throw .unreadable("not an HTTP response") }
        guard (200..<300).contains(http.statusCode) else { throw Problem.error(status: http.statusCode, body: data) }
        return (data, http)
    }

    func decode<T: Decodable>(_ data: Data) throws(APIError) -> T {
        do { return try JSONDecoder.crm.decode(T.self, from: data) } catch { throw .unreadable("\(error)") }
    }
}

/// Refuses every redirect, so a request with a bearer token is never sent on to another address (MOB-8); the `3xx`
/// itself comes back and is an error.
final class NoRedirects: NSObject, URLSessionTaskDelegate {
    static let shared = NoRedirects()

    func urlSession(
        _ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest
    ) async -> URLRequest? {
        nil
    }
}

/// The identity provider's side of sign-in (MOB-4).
public struct OIDCClient: Sendable {
    public let issuer: ServerURL
    private let session: URLSession

    public init(issuer: ServerURL, session: URLSession = APIClient.ephemeralSession()) {
        self.issuer = issuer
        self.session = session
    }

    public func configuration() async throws(SignInError) -> OIDCConfiguration {
        guard let url = issuer.appending("/.well-known/openid-configuration") else { throw .discovery("bad issuer") }
        let data: Data
        do { (data, _) = try await session.data(from: url, delegate: NoRedirects.shared) } catch {
            throw .discovery(error.localizedDescription)
        }
        let configuration: OIDCConfiguration
        do { configuration = try JSONDecoder.crm.decode(OIDCConfiguration.self, from: data) } catch {
            throw .discovery("\(error)")
        }
        try configuration.check(issuer: issuer)
        return configuration
    }

    public func exchange(code: String, for attempt: AuthorizationRequest, at configuration: OIDCConfiguration)
        async throws(SignInError) -> TokenResponse
    {
        var request = URLRequest(url: configuration.tokenEndpoint)
        request.httpMethod = "POST"
        request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        request.httpBody = attempt.tokenRequestBody(code: code)
        let data: Data
        let response: URLResponse
        do { (data, response) = try await session.data(for: request, delegate: NoRedirects.shared) } catch {
            throw .tokenExchange(error.localizedDescription)
        }
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw .tokenExchange(Problem.message(status: (response as? HTTPURLResponse)?.statusCode ?? 0, body: data))
        }
        do { return try JSONDecoder.crm.decode(TokenResponse.self, from: data) } catch {
            throw .tokenExchange("\(error)")
        }
    }
}
