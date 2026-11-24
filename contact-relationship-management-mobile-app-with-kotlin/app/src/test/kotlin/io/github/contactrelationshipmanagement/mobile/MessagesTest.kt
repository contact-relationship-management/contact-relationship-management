package io.github.contactrelationshipmanagement.mobile

import io.github.contactrelationshipmanagement.mobile.core.ACTIVITY_KINDS
import io.github.contactrelationshipmanagement.mobile.core.ApiException
import io.github.contactrelationshipmanagement.mobile.core.Oidc
import io.github.contactrelationshipmanagement.mobile.core.UrlProblem
import io.github.lilydesignsystem.compose.BadgeKind
import java.io.IOException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

/** Every case the core can report has its own string (MOB-9, MOB-11). */
class MessagesTest {
    @Test fun everyUrlProblemHasItsOwnString() {
        val ids = UrlProblem.entries.map(Messages::urlProblem)
        assertEquals(ids.size, ids.toSet().size)
    }

    @Test fun everyCallbackErrorHasItsOwnString() {
        val ids = Oidc.Callback.Reason.entries.map { Messages.callback(Oidc.Callback.Error(it, "d")).id }
        assertEquals(ids.size, ids.toSet().size)
        assertEquals(listOf("d"), Messages.callback(Oidc.Callback.Error(Oidc.Callback.Reason.ProviderError, "d")).args)
    }

    @Test fun everyActivityKindHasALabel() {
        ACTIVITY_KINDS.forEach { assertNotNull(Messages.kind(it), it) }
        assertNull(Messages.kind("fax"))
    }

    @Test fun everyConsentStateHasWordsAndAnUnknownOneIsShownAsSent() {
        val states = listOf("granted", "withdrawn", "never", "pending")
        val ids = states.map { assertNotNull(Messages.consent(it), it) }
        assertEquals(ids.size, ids.toSet().size)
        assertNull(Messages.consent("bogus"))
        assertEquals(BadgeKind.Success, Messages.consentKind("granted"))
        assertEquals(BadgeKind.Warning, Messages.consentKind("withdrawn"))
        assertEquals(BadgeKind.Info, Messages.consentKind("pending"))
        assertEquals(BadgeKind.Default, Messages.consentKind("never"))
    }

    @Test fun aRefusalWithNoSignInSaysToSetTheIssuer() {
        assertEquals(Message(R.string.error_sign_in_not_set), Messages.error(ApiException.Unauthorized("missing bearer token"), signInConfigured = false))
        assertEquals(Message(R.string.error_unauthorized, listOf("expired")), Messages.error(ApiException.Unauthorized("expired"), signInConfigured = true))
    }

    @Test fun theServiceWordsAreKept() {
        assertEquals(Message(R.string.error_http, listOf("not your contact")), Messages.error(ApiException.Http(403, "not your contact")))
        assertEquals(R.string.error_unauthorized, Messages.error(ApiException.Unauthorized("expired")).id)
        assertEquals(R.string.error_network, Messages.error(ApiException.Network(IOException("refused"))).id)
        assertEquals(R.string.error_too_many_pages, Messages.error(ApiException.TooManyPages()).id)
        assertEquals(R.string.error_foreign_link, Messages.error(ApiException.ForeignNextLink("x")).id)
        assertEquals(R.string.error_unreadable, Messages.error(ApiException.Unreadable(null)).id)
        assertEquals(R.string.error_refused, Messages.error(ApiException.Refused("x")).id)
    }

    @Test fun aRetryKeepsTheFormsKey() {
        val form = LogForm(summary = "Rang")
        val failed = form.copy(sending = false, error = Message(R.string.error_unreadable))
        assertEquals(form.key, failed.key)
        // A fresh form, after a save, gets a fresh key.
        assert(LogForm().key != form.key)
    }
}
