package io.github.contactrelationshipmanagement.mobile.core

/** What the app keeps (MOB-1): the server, and the sign-in settings when the server has sign-in on. */
public data class Settings(val server: ServerUrl, val oidc: OidcSettings?)

/** The Server screen's fields, checked as typed (MOB-3, MOB-4). */
public object SettingsForm {
    public data class Errors(val server: UrlProblem?, val issuer: UrlProblem?, val clientIdMissing: Boolean)

    public sealed interface Result {
        public data class Valid(val settings: Settings) : Result
        public data class Invalid(val errors: Errors) : Result
    }

    /** An empty [issuer] means no sign-in (a demo server); otherwise it is checked as a server URL and needs a [clientId]. */
    public fun check(server: String, issuer: String, clientId: String): Result {
        val serverResult = ServerUrl.parse(server)
        val issuerResult = if (issuer.isEmpty()) null else ServerUrl.parse(issuer)
        val clientIdMissing = issuerResult != null && clientId.isBlank()
        val serverProblem = (serverResult as? ServerUrl.Result.Invalid)?.problem
        val issuerProblem = (issuerResult as? ServerUrl.Result.Invalid)?.problem
        if (serverProblem != null || issuerProblem != null || clientIdMissing) {
            return Result.Invalid(Errors(serverProblem, issuerProblem, clientIdMissing))
        }
        val url = (serverResult as ServerUrl.Result.Valid).url
        val oidc = (issuerResult as? ServerUrl.Result.Valid)?.let { OidcSettings(it.url, clientId) }
        return Result.Valid(Settings(url, oidc))
    }
}
