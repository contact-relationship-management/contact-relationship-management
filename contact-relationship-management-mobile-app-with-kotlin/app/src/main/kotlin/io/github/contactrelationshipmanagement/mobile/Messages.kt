package io.github.contactrelationshipmanagement.mobile

import androidx.annotation.StringRes
import io.github.contactrelationshipmanagement.mobile.core.ApiException
import io.github.contactrelationshipmanagement.mobile.core.Oidc
import io.github.contactrelationshipmanagement.mobile.core.UrlProblem
import io.github.lilydesignsystem.compose.BadgeKind

/** A string resource and its arguments, resolved where a Context is at hand. */
data class Message(@StringRes val id: Int, val args: List<Any> = emptyList())

/** Which string says what (MOB-11). Every case is covered by MessagesTest. */
object Messages {
    @StringRes fun urlProblem(problem: UrlProblem): Int = when (problem) {
        UrlProblem.Empty -> R.string.url_empty
        UrlProblem.Unparsable -> R.string.url_unparsable
        UrlProblem.NoHost -> R.string.url_no_host
        UrlProblem.Credentials -> R.string.url_credentials
        UrlProblem.QueryOrFragment -> R.string.url_query_or_fragment
        UrlProblem.Scheme -> R.string.url_scheme
        UrlProblem.CleartextNotLoopback -> R.string.url_cleartext
    }

    fun callback(error: Oidc.Callback.Error): Message = when (error.reason) {
        Oidc.Callback.Reason.NotTheRedirect -> Message(R.string.sign_in_not_redirect)
        Oidc.Callback.Reason.StateMismatch -> Message(R.string.sign_in_state_mismatch)
        Oidc.Callback.Reason.ProviderError -> Message(R.string.sign_in_provider_error, listOf(error.detail.orEmpty()))
        Oidc.Callback.Reason.MissingCode -> Message(R.string.sign_in_missing_code)
    }

    /** A consent state in words (the service's CONSENT_STATES); `null` for a state this app does not know. */
    fun consent(state: String): Int? = when (state) {
        "granted" -> R.string.consent_granted
        "withdrawn" -> R.string.consent_withdrawn
        "never" -> R.string.consent_never
        "pending" -> R.string.consent_pending
        else -> null
    }

    /** The badge for a consent state: the words say the state, the kind only adds colour (MOB-10). */
    fun consentKind(state: String): BadgeKind = when (state) {
        "granted" -> BadgeKind.Success
        "withdrawn" -> BadgeKind.Warning
        "pending" -> BadgeKind.Info
        else -> BadgeKind.Default
    }

    /**
     * MOB-9: the service's words where it gave some. A `401` from a server the app has no sign-in for says to set the
     * issuer, since signing in again is not possible there.
     */
    fun error(error: ApiException, signInConfigured: Boolean = true): Message = when (error) {
        is ApiException.Unauthorized ->
            if (signInConfigured) Message(R.string.error_unauthorized, listOf(error.message.orEmpty()))
            else Message(R.string.error_sign_in_not_set)
        is ApiException.Http -> Message(R.string.error_http, listOf(error.message.orEmpty()))
        is ApiException.TooManyPages -> Message(R.string.error_too_many_pages)
        is ApiException.ForeignNextLink -> Message(R.string.error_foreign_link)
        is ApiException.Refused -> Message(R.string.error_refused, listOf(error.message.orEmpty()))
        is ApiException.Network -> Message(R.string.error_network, listOf(error.message.orEmpty()))
        is ApiException.Unreadable -> Message(R.string.error_unreadable)
    }

    @StringRes fun kind(kind: String): Int? = when (kind) {
        "call" -> R.string.kind_call
        "email" -> R.string.kind_email
        "meeting" -> R.string.kind_meeting
        "note" -> R.string.kind_note
        "task" -> R.string.kind_task
        else -> null
    }
}
