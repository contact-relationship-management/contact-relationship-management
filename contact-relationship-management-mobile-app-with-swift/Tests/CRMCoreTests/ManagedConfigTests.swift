import Foundation
import Testing

@testable import CRMCore

@Suite("MOB-14 managed configuration")
struct ManagedConfigTests {
    @Test func nothingSuppliedLeavesTheUsersSettings() {
        #expect(ManagedConfig.read(nil) == .none)
        #expect(ManagedConfig.read([:]) == .none)
        #expect(ManagedConfig.read(["unrelated": "x"]) == .none)
    }

    @Test func aValidSetIsManaged() {
        let values = ["server_url": "https://crm.example.org", "issuer_url": "https://id.example.org/realms/crm", "client_id": "mobile"]
        #expect(ManagedConfig.read(values) == .managed(server: "https://crm.example.org", issuer: "https://id.example.org/realms/crm", clientID: "mobile"))
    }

    @Test func aServerAloneIsManagedWithNoSignIn() {
        #expect(ManagedConfig.read(["server_url": "https://crm.example.org"]) == .managed(server: "https://crm.example.org", issuer: "", clientID: ""))
    }

    @Test func aBadServerIsAnErrorNotADefault() {
        #expect(ManagedConfig.read(["server_url": "http://crm.example.org"]) == .invalid(server: .notHTTPS, issuer: nil, clientIDMissing: false))
    }

    @Test func anIssuerWithoutAServerIsAnError() {
        #expect(ManagedConfig.read(["issuer_url": "https://id.example.org", "client_id": "c"]) == .invalid(server: .empty, issuer: nil, clientIDMissing: false))
    }

    @Test func aSuppliedBlankOrNonStringIsAnErrorNotAbsent() {
        #expect(ManagedConfig.read(["server_url": ""]) == .invalid(server: .empty, issuer: nil, clientIDMissing: false))
        #expect(ManagedConfig.read(["server_url": 5]) == .invalid(server: .empty, issuer: nil, clientIDMissing: false))
    }

    @Test func anIssuerNeedsAClientIDWhenManagedToo() {
        let values = ["server_url": "https://crm.example.org", "issuer_url": "https://id.example.org"]
        #expect(ManagedConfig.read(values) == .invalid(server: nil, issuer: nil, clientIDMissing: true))
    }
}
