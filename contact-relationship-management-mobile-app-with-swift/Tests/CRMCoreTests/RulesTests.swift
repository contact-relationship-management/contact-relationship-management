import Foundation
import Testing

@testable import CRMCore

@Suite("MOB-3 server URL")
struct ServerURLTests {
    @Test func acceptsHTTPSAndKeepsThePath() throws {
        #expect(try ServerURL("https://crm.example.org").description == "https://crm.example.org")
        #expect(try ServerURL("https://crm.example.org/crm/").description == "https://crm.example.org/crm")
    }

    @Test(arguments: ["http://localhost:5160", "http://127.0.0.1:5160", "http://[::1]:5160", "http://10.0.2.2:5160"])
    func acceptsHTTPToLoopback(_ text: String) throws {
        #expect(try ServerURL(text).description == text)
    }

    @Test func refusesAndNeverCorrects() {
        #expect(throws: ServerURLError.empty) { try ServerURL("") }
        #expect(throws: ServerURLError.notHTTPS) { try ServerURL("http://crm.example.org") }
        #expect(throws: ServerURLError.notHTTPS) { try ServerURL("ftp://crm.example.org") }
        #expect(throws: ServerURLError.unreadable) { try ServerURL("crm.example.org") }
        #expect(throws: ServerURLError.hasCredentials) { try ServerURL("https://user:pw@crm.example.org") }
        #expect(throws: ServerURLError.hasQueryOrFragment) { try ServerURL("https://crm.example.org/?a=1") }
        #expect(throws: ServerURLError.hasQueryOrFragment) { try ServerURL("https://crm.example.org/#top") }
        #expect(throws: ServerURLError.noHost) { try ServerURL("https:///path") }
    }

    @Test func appendsAPathAfterTheServersOwn() throws {
        let server = try ServerURL("https://crm.example.org/crm")
        #expect(server.appending("/api/contacts")?.absoluteString == "https://crm.example.org/crm/api/contacts")
    }
}

@Suite("MOB-6 money")
struct MoneyTests {
    let us = Locale(identifier: "en_US")

    @Test func formatsExactlyFromMinorText() throws {
        #expect(try Money.format(minorText: "123456", currency: "USD", locale: us) == "$1,234.56")
        #expect(try Money.format(minorText: "-5", currency: "USD", locale: us) == "-$0.05")
        #expect(try Money.format(minorText: "1500", currency: "JPY", locale: us) == "¥1,500")
        #expect(try Money.format(minorText: "1234", currency: "KWD", locale: us)?.contains("1.234") == true)
    }

    @Test func keepsEveryDigitOfTheLargestInt64() throws {
        let amount = try Money.decimal(minorText: "9223372036854775807", currency: "USD")
        #expect(amount.description == "92233720368547758.07")
    }

    @Test func aMaskedAmountIsHiddenNotZero() throws {
        #expect(try Money.format(minorText: nil, currency: "USD", locale: us) == nil)
    }

    @Test func anUnknownCurrencyHasTwoDigitsAndItsCode() throws {
        #expect(Money.minorDigits("XZZ") == 2)
        #expect(try Money.format(minorText: "1234", currency: "XZZ", locale: us) == "12.34 XZZ")
    }

    @Test(arguments: ["", "-", "12.5", "1e3", " 12", "12 ", "+12", "١٢", "abc"])
    func textThatIsNotAnIntegerIsAnError(_ text: String) {
        #expect(throws: MoneyError.notAnInteger(text)) { try Money.decimal(minorText: text, currency: "USD") }
    }

    @Test func aDealPrefersItsExactText() {
        var deal = DealFixtures.deal(stage: UUID(), pipeline: UUID(), name: "A", position: 0)
        deal.amountMinor = 1
        deal.amountMinorText = "9007199254740993"
        #expect(deal.amountText == "9007199254740993")
        deal.amountMinorText = nil
        #expect(deal.amountText == "1")
        deal.amountMinor = nil
        #expect(deal.amountText == nil)
    }
}

@Suite("MOB-8 paging")
struct PagingTests {
    @Test func findsTheNextLink() {
        #expect(Paging.nextLink(#"</api/contacts?cursor=abc>; rel="next""#) == "/api/contacts?cursor=abc")
        #expect(Paging.nextLink(#"</a>; rel="prev", </b>; rel=next"#) == "/b")
        #expect(Paging.nextLink(#"</a>; rel="prev""#) == nil)
        #expect(Paging.nextLink(nil) == nil)
        #expect(Paging.nextLink("garbage") == nil)
    }

    @Test func resolvesOnTheServerOnly() throws {
        let server = try ServerURL("https://crm.example.org")
        #expect(Paging.resolve("/api/contacts?cursor=x", against: server)?.absoluteString
            == "https://crm.example.org/api/contacts?cursor=x")
        #expect(Paging.resolve("https://evil.example.com/api/contacts", against: server) == nil)
        #expect(Paging.resolve("http://crm.example.org/api/contacts", against: server) == nil)
        #expect(Paging.resolve("https://crm.example.org:8443/api/contacts", against: server) == nil)
        #expect(Paging.resolve("//evil.example.com/x", against: server) == nil)
    }
}

@Suite("MOB-9 problem documents")
struct ProblemTests {
    @Test func prefersDetailThenTitleThenStatus() {
        #expect(Problem.message(status: 422, body: Data(#"{"title":"T","detail":"D"}"#.utf8)) == "D")
        #expect(Problem.message(status: 422, body: Data(#"{"title":"T","detail":"  "}"#.utf8)) == "T")
        #expect(Problem.message(status: 502, body: Data("bad gateway".utf8)) == "HTTP 502")
        #expect(Problem.error(status: 401, body: Data()) == .unauthorized)
    }
}

@Suite("MOB-4 PKCE and the callback")
struct SignInTests {
    @Test func matchesTheRFC7636Example() {
        #expect(PKCE.challenge(for: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")
            == "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM")
    }

    @Test func aVerifierIsLongEnoughAndURLSafe() {
        let verifier = PKCE.newVerifier()
        #expect(verifier.count == 43)
        #expect(verifier.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" || $0 == "_" })
        #expect(PKCE.newVerifier() != verifier)
    }

    let configuration = OIDCConfiguration(
        issuer: "https://id.example.org/realms/crm",
        authorizationEndpoint: URL(string: "https://id.example.org/auth?kc=1")!,
        tokenEndpoint: URL(string: "https://id.example.org/token")!)

    @Test func theAuthorizationURLAsksForACodeWithS256() throws {
        let attempt = AuthorizationRequest(clientID: "crm-mobile", state: "s1", verifier: "v1")
        let url = try #require(attempt.url(configuration: configuration))
        let items = try #require(URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems)
        let query = Dictionary(uniqueKeysWithValues: items.map { ($0.name, $0.value ?? "") })
        #expect(query["kc"] == "1")
        #expect(query["response_type"] == "code")
        #expect(query["client_id"] == "crm-mobile")
        #expect(query["redirect_uri"] == AuthorizationRequest.redirectURI)
        #expect(query["scope"] == "openid")
        #expect(query["state"] == "s1")
        #expect(query["code_challenge"] == PKCE.challenge(for: "v1"))
        #expect(query["code_challenge_method"] == "S256")
    }

    @Test func theCallbackGivesTheCodeOnlyWhenStateMatches() throws {
        let attempt = AuthorizationRequest(clientID: "c", state: "s1", verifier: "v")
        let base = "io.github.contactrelationshipmanagement.mobile:/oauth/callback"
        #expect(try attempt.code(from: URL(string: "\(base)?code=abc&state=s1")!) == "abc")
        #expect(throws: SignInError.stateMismatch) { try attempt.code(from: URL(string: "\(base)?code=abc&state=s2")!) }
        #expect(throws: SignInError.stateMismatch) { try attempt.code(from: URL(string: "\(base)?code=abc")!) }
        #expect(throws: SignInError.providerError("denied")) {
            try attempt.code(from: URL(string: "\(base)?error=access_denied&error_description=denied&state=s1")!)
        }
        #expect(throws: SignInError.missingCode) { try attempt.code(from: URL(string: "\(base)?state=s1")!) }
        #expect(throws: SignInError.unreadableCallback) {
            try attempt.code(from: URL(string: "https://evil.example.com/?code=abc&state=s1")!)
        }
    }

    @Test func discoveryMustNameTheIssuerAndUseHTTPS() throws {
        let issuer = try ServerURL("https://id.example.org/realms/crm")
        var good = configuration
        try good.check(issuer: issuer)
        good.issuer = "https://id.example.org/realms/crm/"
        try good.check(issuer: issuer)
        var other = configuration
        other.issuer = "https://evil.example.com/realms/crm"
        #expect(throws: SignInError.discovery("the document names another issuer: https://evil.example.com/realms/crm")) {
            try other.check(issuer: issuer)
        }
        var plain = configuration
        plain.tokenEndpoint = URL(string: "http://id.example.org/token")!
        #expect(throws: SignInError.discovery("an endpoint is not https: http://id.example.org/token")) {
            try plain.check(issuer: issuer)
        }
    }

    @Test func theTokenRequestCarriesTheVerifierAndEscapesPlus() {
        let attempt = AuthorizationRequest(clientID: "c", state: "s", verifier: "v+1")
        let body = String(decoding: attempt.tokenRequestBody(code: "a+b"), as: UTF8.self)
        #expect(body.contains("grant_type=authorization_code"))
        #expect(body.contains("code=a%2Bb"))
        #expect(body.contains("code_verifier=v%2B1"))
        #expect(!body.contains("secret"))
    }
}

enum DealFixtures {
    static func deal(stage: UUID, pipeline: UUID, name: String, position: Int32) -> Deal {
        Deal(
            pid: UUID(), name: name, currency: "GBP", amountMinor: 100, amountMinorText: "100", pipelinePid: pipeline,
            stagePid: stage, kanbanPosition: position, won: false, closedAt: nil, expectedCloseOn: nil)
    }

    static func stage(_ pipeline: UUID, _ name: String, _ position: Int32) -> StageRecord {
        StageRecord(
            pid: UUID(), pipelinePid: pipeline, name: name, position: position, isWon: false, isLost: false,
            probabilityPercent: 10)
    }
}

@Suite("Deal board")
struct DealBoardTests {
    @Test func ordersStagesAndDealsAndKeepsTheUnplaced() {
        let p = UUID()
        let late = DealFixtures.stage(p, "Won", 2)
        let early = DealFixtures.stage(p, "Lead", 0)
        let b = DealFixtures.deal(stage: early.pid, pipeline: p, name: "B", position: 1)
        let a = DealFixtures.deal(stage: early.pid, pipeline: p, name: "A", position: 1)
        let first = DealFixtures.deal(stage: early.pid, pipeline: p, name: "Z", position: 0)
        let stray = DealFixtures.deal(stage: UUID(), pipeline: p, name: "Stray", position: 0)
        let pipeline = Pipeline(pipeline: PipelineRecord(pid: p, name: "Sales"), stages: [late, early])
        let (boards, unplaced) = DealBoard.boards(pipelines: [pipeline], deals: [b, stray, a, first])
        #expect(boards.count == 1)
        #expect(boards[0].columns.map(\.stage.name) == ["Lead", "Won"])
        #expect(boards[0].columns[0].deals.map(\.name) == ["Z", "A", "B"])
        #expect(boards[0].columns[1].deals.isEmpty)
        #expect(unplaced.map(\.name) == ["Stray"])
    }
}
