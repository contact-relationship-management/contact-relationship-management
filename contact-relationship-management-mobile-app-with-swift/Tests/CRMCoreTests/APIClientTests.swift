import Foundation
import Synchronization
import Testing

@testable import CRMCore

/// Answers requests from a table and records what was asked, so the client is tested without a network.
final class StubProtocol: URLProtocol, @unchecked Sendable {
    struct Answer: Sendable {
        var status: Int
        var headers: [String: String] = [:]
        var body: String
    }

    static let answers = Mutex<[String: Answer]>([:])
    static let seen = Mutex<[URLRequest]>([])

    static func reset(_ table: [String: Answer]) {
        answers.withLock { $0 = table }
        seen.withLock { $0 = [] }
    }

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        var recorded = request
        if recorded.httpBody == nil, let stream = request.httpBodyStream {
            stream.open()
            var data = Data()
            var buffer = [UInt8](repeating: 0, count: 4096)
            while stream.hasBytesAvailable {
                let read = stream.read(&buffer, maxLength: buffer.count)
                if read <= 0 { break }
                data.append(buffer, count: read)
            }
            stream.close()
            recorded.httpBody = data
        }
        Self.seen.withLock { $0.append(recorded) }
        let key = request.url?.absoluteString ?? ""
        let answer = Self.answers.withLock { $0[key] } ?? Answer(status: 404, body: #"{"title":"no stub"}"#)
        let response = HTTPURLResponse(
            url: request.url!, statusCode: answer.status, httpVersion: "HTTP/1.1", headerFields: answer.headers)!
        if (300..<400).contains(answer.status), let location = answer.headers["Location"], let target = URL(string: location) {
            // As a real server's redirect: the session asks its delegate whether to follow it.
            // When the delegate refuses it, the session waits for this protocol to finish with the 3xx itself.
            client?.urlProtocol(self, wasRedirectedTo: URLRequest(url: target), redirectResponse: response)
        }
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: Data(answer.body.utf8))
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}
}

func contactJSON(_ name: String) -> String {
    """
    {"pid":"\(UUID().uuidString.lowercased())","display_name":"\(name)","given_name":null,"family_name":null,
     "job_title":null,"status":"active","marketing_consent":"unknown","preferred_channel":"email",
     "person_ref":"person:\(UUID().uuidString.lowercased())","account_pid":null,"updated_at":"2026-11-23T00:00:00Z",
     "id":1,"created_at":"2026-11-23T00:00:00Z","deleted_at":null}
    """
}

@Suite("API client", .serialized)
struct APIClientTests {
    let base = "https://crm.example.org"

    func client(token: String? = "t0k") throws -> APIClient {
        APIClient(
            server: try ServerURL(base), token: token,
            session: APIClient.ephemeralSession(protocolClasses: [StubProtocol.self]))
    }

    @Test func followsEveryPageAndSendsTheHeaders() async throws {
        StubProtocol.reset([
            "\(base)/api/contacts": .init(
                status: 200, headers: ["Link": #"</api/contacts?cursor=2>; rel="next""#], body: "[\(contactJSON("Ada"))]"),
            "\(base)/api/contacts?cursor=2": .init(status: 200, body: "[\(contactJSON("Grace"))]"),
        ])
        let contacts = try await client().contacts()
        #expect(contacts.map(\.displayName) == ["Ada", "Grace"])
        let seen = StubProtocol.seen.withLock { $0 }
        #expect(seen.count == 2)
        for request in seen {
            #expect(request.value(forHTTPHeaderField: "Accepts-version") == "1.0")
            #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer t0k")
        }
    }

    @Test func sendsNoAuthorizationWithoutAToken() async throws {
        StubProtocol.reset(["\(base)/api/contacts": .init(status: 200, body: "[]")])
        _ = try await client(token: nil).contacts()
        #expect(StubProtocol.seen.withLock { $0.first?.value(forHTTPHeaderField: "Authorization") } == nil)
    }

    @Test func refusesANextLinkToAnotherOrigin() async throws {
        StubProtocol.reset([
            "\(base)/api/contacts": .init(
                status: 200, headers: ["Link": #"<https://evil.example.com/steal>; rel="next""#], body: "[]")
        ])
        await #expect(throws: APIError.unsafeNextLink("https://evil.example.com/steal")) { try await client().contacts() }
        #expect(StubProtocol.seen.withLock { $0.count } == 1)
    }

    @Test func followsNoRedirectSoTheTokenStaysHome() async throws {
        StubProtocol.reset([
            "\(base)/api/contacts": .init(
                status: 302, headers: ["Location": "https://evil.example.com/collect"], body: ""),
            "https://evil.example.com/collect": .init(status: 200, body: "[]"),
        ])
        await #expect(throws: APIError.problem(status: 302, message: "HTTP 302")) { try await client().contacts() }
        let hosts = StubProtocol.seen.withLock { $0.compactMap { $0.url?.host } }
        #expect(hosts == ["crm.example.org"])
    }

    @Test func failsRatherThanCutAListShort() async throws {
        var table: [String: StubProtocol.Answer] = [:]
        table["\(base)/api/tickets"] = .init(status: 200, headers: ["Link": #"</api/tickets?p=1>; rel="next""#], body: "[]")
        for page in 1...Paging.maxPages {
            table["\(base)/api/tickets?p=\(page)"] = .init(
                status: 200, headers: ["Link": "</api/tickets?p=\(page + 1)>; rel=\"next\""], body: "[]")
        }
        StubProtocol.reset(table)
        await #expect(throws: APIError.tooManyPages) { try await client().tickets() }
    }

    @Test func turnsAProblemIntoItsDetailAnd401IntoSignIn() async throws {
        StubProtocol.reset([
            "\(base)/api/deals": .init(status: 422, body: #"{"title":"Unprocessable","detail":"limit is out of range"}"#),
            "\(base)/api/tickets": .init(status: 401, body: "missing bearer token"),
        ])
        await #expect(throws: APIError.problem(status: 422, message: "limit is out of range")) {
            try await client().deals()
        }
        await #expect(throws: APIError.unauthorized) { try await client().tickets() }
    }

    @Test func logsAnActivityWithItsIdempotencyKey() async throws {
        let contact = UUID()
        let activity = """
            {"pid":"\(UUID().uuidString)","kind":"call","summary":"Rang","occurred_at":"2026-11-23T10:00:00Z",
             "done":false,"due_on":null,"subject_kind":"contact","subject_pid":"\(contact.uuidString)"}
            """
        StubProtocol.reset(["\(base)/api/activities": .init(status: 201, body: activity)])
        let saved = try await client().log(NewActivity(kind: .call, contact: contact, summary: "Rang"), idempotencyKey: "k-1")
        #expect(saved.summary == "Rang")
        let request = try #require(StubProtocol.seen.withLock { $0.first })
        #expect(request.httpMethod == "POST")
        #expect(request.value(forHTTPHeaderField: "Idempotency-Key") == "k-1")
        let body = try #require(request.httpBody)
        let json = try #require(try JSONSerialization.jsonObject(with: body) as? [String: String])
        #expect(json == ["kind": "call", "subject_kind": "contact", "subject_pid": contact.uuidString, "summary": "Rang"])
    }

    @Test func decodesPipelinesAndAContactDetail() async throws {
        let p = UUID().uuidString
        let pipelines = """
            [{"pipeline":{"pid":"\(p)","name":"Sales","id":1,"created_at":"x","updated_at":"x","deleted_at":null},
              "stages":[{"pid":"\(UUID().uuidString)","pipeline_pid":"\(p)","name":"Lead","position":0,"is_won":false,
                         "is_lost":false,"probability_percent":10,"id":1,"created_at":"x","updated_at":"x","deleted_at":null}]}]
            """
        let contact = UUID().uuidString.lowercased()
        let detail = """
            {"contact":\(contactJSON("Ada")),"activities":[],"tickets":[],"employments":[],
             "truncated":{"activities":false,"deals":false,"tickets":false},
             "deals":[{"pid":"\(UUID().uuidString)","name":"Renewal","currency":"GBP","amount_minor":null,
                       "pipeline_pid":"\(p)","stage_pid":"\(UUID().uuidString)","kanban_position":0,"won":false,
                       "closed_at":null,"expected_close_on":null}]}
            """
        StubProtocol.reset([
            "\(base)/api/pipelines": .init(status: 200, body: pipelines),
            "\(base)/api/contacts/\(contact)": .init(status: 200, body: detail),
        ])
        let loaded = try await client().pipelines()
        #expect(loaded.first?.stages.first?.name == "Lead")
        let shown = try await client().contact(UUID(uuidString: contact)!)
        #expect(shown.deals.first?.amountText == nil)
    }
}
