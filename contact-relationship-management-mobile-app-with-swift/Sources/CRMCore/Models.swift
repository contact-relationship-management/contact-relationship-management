// The service's JSON, as much of it as the app shows (spec/mobile-apps). Field names are the API's snake_case, decoded with
// `JSONDecoder.crm`. Dates stay text: the app shows them, it does no arithmetic on them.
import Foundation

public struct Contact: Codable, Sendable, Identifiable, Hashable {
    public var pid: UUID
    public var displayName: String
    public var givenName: String?
    public var familyName: String?
    public var jobTitle: String?
    public var status: String
    public var marketingConsent: String
    public var preferredChannel: String
    public var personRef: String
    public var accountPid: UUID?
    public var updatedAt: String

    public var id: UUID { pid }
}

public struct Activity: Codable, Sendable, Identifiable, Hashable {
    public var pid: UUID
    public var kind: String
    public var summary: String
    public var occurredAt: String
    public var done: Bool
    public var dueOn: String?
    public var subjectKind: String
    public var subjectPid: UUID

    public var id: UUID { pid }
}

/// A deal from `GET /api/deals` or inside a contact. `amountMinorText` is absent inside a contact detail, where only
/// `amountMinor` is sent; `amountText` takes whichever there is.
public struct Deal: Codable, Sendable, Identifiable, Hashable {
    public var pid: UUID
    public var name: String
    public var currency: String
    public var amountMinor: Int64?
    public var amountMinorText: String?
    public var pipelinePid: UUID
    public var stagePid: UUID
    public var kanbanPosition: Int32
    public var won: Bool
    public var closedAt: String?
    public var expectedCloseOn: String?

    public var id: UUID { pid }

    /// The exact amount in minor units as text, `nil` when masked (MOB-6).
    public var amountText: String? { amountMinorText ?? amountMinor.map { String($0) } }
}

public struct Ticket: Codable, Sendable, Identifiable, Hashable {
    public var pid: UUID
    public var title: String
    public var status: String
    public var priority: String
    public var channel: String
    public var openedAt: String
    public var contactPid: UUID?
    public var firstResponseBreached: Bool
    public var resolutionBreached: Bool

    public var id: UUID { pid }
}

public struct ContactDetail: Codable, Sendable {
    public var contact: Contact
    public var activities: [Activity]
    public var deals: [Deal]
    public var tickets: [Ticket]
}

public struct PipelineRecord: Codable, Sendable, Hashable {
    public var pid: UUID
    public var name: String
}

public struct StageRecord: Codable, Sendable, Hashable, Identifiable {
    public var pid: UUID
    public var pipelinePid: UUID
    public var name: String
    public var position: Int32
    public var isWon: Bool
    public var isLost: Bool
    public var probabilityPercent: Int32

    public var id: UUID { pid }
}

public struct Pipeline: Codable, Sendable, Hashable {
    public var pipeline: PipelineRecord
    public var stages: [StageRecord]
}

/// The kinds of activity the service accepts (`ACTIVITY_KINDS` in the domain crate).
public enum ActivityKind: String, CaseIterable, Sendable, Codable {
    case call, email, meeting, note, task
}

/// `POST /api/activities` body for an activity about a contact.
public struct NewActivity: Encodable, Sendable, Equatable {
    public var kind: ActivityKind
    public var subjectKind: String
    public var subjectPid: UUID
    public var summary: String

    public init(kind: ActivityKind, contact: UUID, summary: String) {
        self.kind = kind
        self.subjectKind = "contact"
        self.subjectPid = contact
        self.summary = summary
    }
}

extension JSONDecoder {
    /// The decoder for the service's JSON.
    public static var crm: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return decoder
    }
}

extension JSONEncoder {
    /// The encoder for request bodies.
    public static var crm: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        encoder.outputFormatting = .sortedKeys
        return encoder
    }
}
