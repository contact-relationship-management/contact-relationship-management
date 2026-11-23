package io.github.contactrelationshipmanagement.mobile.core

import kotlin.test.Test
import kotlin.test.assertEquals

class ServerUrlTest {
    private fun valid(text: String): String = (ServerUrl.parse(text) as ServerUrl.Result.Valid).url.base
    private fun problem(text: String): UrlProblem = (ServerUrl.parse(text) as ServerUrl.Result.Invalid).problem

    @Test fun httpsIsAccepted() = assertEquals("https://crm.example.org", valid("https://crm.example.org"))

    @Test fun aPathAndPortAreKept() = assertEquals("https://crm.example.org:8443/crm", valid("https://crm.example.org:8443/crm"))

    @Test fun oneTrailingSlashIsTheOnlyChange() {
        assertEquals("https://crm.example.org", valid("https://crm.example.org/"))
        assertEquals("https://crm.example.org/crm", valid("https://crm.example.org/crm/"))
    }

    @Test fun httpIsAcceptedOnlyToLoopback() {
        assertEquals("http://localhost:1241", valid("http://localhost:1241"))
        assertEquals("http://127.0.0.1:1241", valid("http://127.0.0.1:1241"))
        assertEquals("http://[::1]:1241", valid("http://[::1]:1241"))
        assertEquals("http://10.0.2.2:1241", valid("http://10.0.2.2:1241"))
        assertEquals(UrlProblem.CleartextNotLoopback, problem("http://crm.example.org"))
        assertEquals(UrlProblem.CleartextNotLoopback, problem("http://192.168.1.10:1241"))
    }

    @Test fun nothingIsCorrected() {
        assertEquals(UrlProblem.Scheme, problem("crm.example.org"))
        assertEquals(UrlProblem.Unparsable, problem(" https://crm.example.org"))
        assertEquals(UrlProblem.Unparsable, problem("https://crm.example.org "))
    }

    @Test fun emptyIsAnError() = assertEquals(UrlProblem.Empty, problem(""))

    @Test fun otherSchemesAreRefused() {
        assertEquals(UrlProblem.Scheme, problem("ftp://crm.example.org"))
        assertEquals(UrlProblem.Scheme, problem("mailto:someone@example.org"))
    }

    @Test fun credentialsAreRefused() = assertEquals(UrlProblem.Credentials, problem("https://user:pass@crm.example.org"))

    @Test fun aQueryOrFragmentIsRefused() {
        assertEquals(UrlProblem.QueryOrFragment, problem("https://crm.example.org?x=1"))
        assertEquals(UrlProblem.QueryOrFragment, problem("https://crm.example.org?"))
        assertEquals(UrlProblem.QueryOrFragment, problem("https://crm.example.org#top"))
    }

    @Test fun aHostIsRequired() {
        assertEquals(UrlProblem.NoHost, problem("https:///path"))
        assertEquals(UrlProblem.NoHost, problem("https://bad_host.example.org"))
    }

    @Test fun resolveAppendsThePath() =
        assertEquals("https://crm.example.org/crm/api/contacts", (ServerUrl.parse("https://crm.example.org/crm/") as ServerUrl.Result.Valid).url.resolve("/api/contacts"))
}
