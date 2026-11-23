// MOB-9: an error shows the service's words (an RFC 9457 problem document), or the status.
import Foundation

public enum APIError: Error, Equatable, Sendable {
    /// `401`: sign in again.
    case unauthorized
    /// Any other non-2xx answer, with the message to show.
    case problem(status: Int, message: String)
    /// A list longer than `Paging.maxPages` pages.
    case tooManyPages
    /// A next link to another origin, or one that could not be read.
    case unsafeNextLink(String)
    /// A body that was not the JSON expected.
    case unreadable(String)
    /// The request did not complete.
    case transport(String)
    /// The settings could not make a request URL.
    case badURL
}

public enum Problem {
    /// The message for a failed answer: the problem's `detail`, else its `title`, else the status.
    public static func message(status: Int, body: Data) -> String {
        if let object = try? JSONSerialization.jsonObject(with: body) as? [String: Any] {
            for key in ["detail", "title"] {
                if let text = object[key] as? String, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    return text
                }
            }
        }
        return "HTTP \(status)"
    }

    /// The error for a non-2xx answer.
    public static func error(status: Int, body: Data) -> APIError {
        status == 401 ? .unauthorized : .problem(status: status, message: message(status: status, body: body))
    }
}
