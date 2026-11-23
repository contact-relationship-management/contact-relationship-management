import CRMCore
import Foundation
import Testing

@testable import CRMMobileUI

@Suite("MOB-11 strings")
struct StringsTests {
    /// Every key the sources name has English: a missing key comes back as itself.
    @Test func everyKeyTheCodeUsesHasEnglish() throws {
        let sources = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("../../Sources/CRMMobileUI").standardizedFileURL
        let files = try FileManager.default.contentsOfDirectory(at: sources, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "swift" }
        let pattern = try Regex(#"\bt\("([a-zA-Z.]+)"\)"#)
        var keys = Set<String>()
        for file in files {
            let text = try String(contentsOf: file, encoding: .utf8)
            for match in text.matches(of: pattern) {
                if let key = match.output[1].substring { keys.insert(String(key)) }
            }
        }
        #expect(keys.count > 40)
        for key in keys { #expect(S.t(key) != key, "no English for \(key)") }
        for kind in ActivityKind.allCases { #expect(S.kind(kind.rawValue) != "kind.\(kind.rawValue)") }
        for error in [ServerURLError.empty, .unreadable, .notHTTPS, .noHost, .hasCredentials, .hasQueryOrFragment] {
            #expect(!S.serverError(error).hasPrefix("url."))
        }
    }

    @Test func consentStatesAreWordsAndAnUnknownOneIsShownAsSent() {
        #expect(S.consent("granted") == "Granted")
        #expect(S.consent("never") == "Never given")
        #expect(S.consent("bogus") == "bogus")
    }

    @Test func aDealCountAgreesWithItsNumber() {
        #expect(S.dealCount(1) == "1 deal")
        #expect(S.dealCount(2) == "2 deals")
    }
}
