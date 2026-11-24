// MOB-11: every user-facing string, by key; the English is in Resources/en.lproj/Localizable.strings. A locale is added
// by adding its .lproj; it is a machine-written draft until a speaker reviews it.
import CRMCore
import Foundation

enum S {
    static func t(_ key: String) -> String {
        NSLocalizedString(key, tableName: nil, bundle: .module, value: key, comment: "")
    }

    static func f(_ key: String, _ args: CVarArg...) -> String {
        String(format: t(key), locale: Locale.current, arguments: args)
    }

    static var appName: String { t("app.name") }
    static var tabContacts: String { t("tab.contacts") }
    static var tabDeals: String { t("tab.deals") }
    static var tabTickets: String { t("tab.tickets") }
    static var tabSettings: String { t("tab.settings") }

    static var settingsTitle: String { t("settings.title") }
    static var settingsServer: String { t("settings.server") }
    static var settingsServerHelp: String { t("settings.server.help") }
    static var settingsIssuer: String { t("settings.issuer") }
    static var settingsIssuerHelp: String { t("settings.issuer.help") }
    static var settingsClient: String { t("settings.client") }
    static var settingsSave: String { t("settings.save") }
    static var settingsSaved: String { t("settings.saved") }
    static var settingsSignOut: String { t("settings.signout") }
    static var settingsStored: String { t("settings.stored") }
    static var settingsManaged: String { t("settings.managed") }
    static var settingsClientRequired: String { t("settings.clientRequired") }

    static var signInTitle: String { t("signin.title") }
    static var signInBody: String { t("signin.body") }
    static var signInButton: String { t("signin.button") }

    static var problemTitle: String { t("problem.title") }
    static var required: String { t("field.required") }
    static var retry: String { t("action.retry") }
    static var loading: String { t("state.loading") }
    static var empty: String { t("state.empty") }
    static var demoHeading: String { t("demo.heading") }
    static var demoBody: String { t("demo.body") }

    static var contactsSelect: String { t("contacts.select") }
    static var contactsSearch: String { t("contacts.search") }
    static var contactJobTitle: String { t("contact.jobTitle") }
    static var contactStatus: String { t("contact.status") }
    static var contactConsent: String { t("contact.consent") }
    static var contactChannel: String { t("contact.channel") }
    static var contactSummary: String { t("contact.summary") }
    static var contactActivities: String { t("contact.activities") }
    static var contactDeals: String { t("contact.deals") }
    static var contactTickets: String { t("contact.tickets") }
    static var contactConsentNote: String { t("contact.consentNote") }

    static var logTitle: String { t("log.title") }
    static var logKind: String { t("log.kind") }
    static var logSummary: String { t("log.summary") }
    static var logSummaryMissing: String { t("log.summary.missing") }
    static var logSave: String { t("log.save") }
    static var logCancel: String { t("log.cancel") }
    static func kind(_ kind: String) -> String { t("kind.\(kind)") }
    /// A consent state in words; a state this app does not know is shown as the service sent it.
    static func consent(_ state: String) -> String { t("consent.\(state)") == "consent.\(state)" ? state : t("consent.\(state)") }
    static var signInNoIssuer: String { t("error.signInNoIssuer") }

    static var dealsUnplaced: String { t("deals.unplaced") }
    static var dealHidden: String { t("deal.hidden") }
    static var dealWon: String { t("deal.won") }
    static func dealCount(_ count: Int) -> String { f("deals.count", count) }

    static var ticketFirstResponseBreached: String { t("ticket.breach.firstResponse") }
    static var ticketResolutionBreached: String { t("ticket.breach.resolution") }
    static var ticketPriority: String { t("ticket.priority") }
    static var ticketStatus: String { t("ticket.status") }

    static func serverError(_ error: ServerURLError) -> String {
        switch error {
        case .empty: t("url.empty")
        case .unreadable: t("url.unreadable")
        case .notHTTPS: t("url.notHTTPS")
        case .noHost: t("url.noHost")
        case .hasCredentials: t("url.hasCredentials")
        case .hasQueryOrFragment: t("url.hasQueryOrFragment")
        }
    }
}
