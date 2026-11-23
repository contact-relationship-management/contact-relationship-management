package io.github.contactrelationshipmanagement.mobile.core

import java.net.URI
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull

class SessionTest {
    private val endpoints = OidcEndpoints("https://id.example.org", "https://id.example.org/auth", "https://id.example.org/token")

    private fun stateOf(url: String): String =
        URI(url).rawQuery.split("&").first { it.startsWith("state=") }.substringAfter("=")

    @Test fun aMatchingCallbackIsExchangedOnce() {
        val session = Session()
        val state = stateOf(session.begin(endpoints, "c"))
        val first = session.complete("${Oidc.REDIRECT_URI}?code=abc&state=$state")
        assertIs<Session.Completion.Exchange>(first)
        assertEquals("abc", first.code)
        assertEquals(43, first.verifier.length)
        // A replay of the same callback finds no attempt in flight.
        assertIs<Session.Completion.Refused>(session.complete("${Oidc.REDIRECT_URI}?code=abc&state=$state"))
    }

    @Test fun aCallbackWithNoAttemptIsRefused() {
        assertIs<Session.Completion.Refused>(Session().complete("${Oidc.REDIRECT_URI}?code=abc&state=x"))
    }

    @Test fun aNewAttemptReplacesTheOld() {
        val session = Session()
        val old = stateOf(session.begin(endpoints, "c"))
        session.begin(endpoints, "c")
        assertIs<Session.Completion.Refused>(session.complete("${Oidc.REDIRECT_URI}?code=abc&state=$old"))
    }

    @Test fun signOutForgetsTheToken() {
        val session = Session()
        assertNull(session.accessToken)
        session.signedIn("t")
        assertEquals("t", session.accessToken)
        session.signOut()
        assertNull(session.accessToken)
    }
}
