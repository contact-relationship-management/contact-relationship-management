// MOB-8: a list is whole or an error, and a next link never leaves the server.
import Foundation

public enum Paging {
    /// Pages followed before giving up, as the Svelte client's `MAX_PAGES`.
    public static let maxPages = 50

    /// The target of `rel="next"` in a `Link` header, as written; `nil` when there is none.
    public static func nextLink(_ header: String?) -> String? {
        guard let header else { return nil }
        for part in header.split(separator: ",") {
            let pieces = part.split(separator: ";").map { $0.trimmingCharacters(in: .whitespaces) }
            guard let target = pieces.first, target.hasPrefix("<"), target.hasSuffix(">") else { continue }
            let isNext = pieces.dropFirst().contains { param in
                let pair = param.split(separator: "=", maxSplits: 1).map { $0.trimmingCharacters(in: .whitespaces) }
                guard pair.count == 2, pair[0].lowercased() == "rel" else { return false }
                let value = pair[1].trimmingCharacters(in: ["\""])
                return value.split(separator: " ").contains("next")
            }
            if isNext { return String(target.dropFirst().dropLast()) }
        }
        return nil
    }

    /// `link` resolved against `server`; `nil` when it is unreadable or on another scheme, host or port.
    public static func resolve(_ link: String, against server: ServerURL) -> URL? {
        guard let url = URL(string: link, relativeTo: server.url)?.absoluteURL, server.isSameOrigin(url) else {
            return nil
        }
        return url
    }
}
