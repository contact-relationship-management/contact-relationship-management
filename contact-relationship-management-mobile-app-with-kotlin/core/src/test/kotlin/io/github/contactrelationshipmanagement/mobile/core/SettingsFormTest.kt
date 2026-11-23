package io.github.contactrelationshipmanagement.mobile.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull

class SettingsFormTest {
    @Test fun noIssuerMeansNoSignIn() {
        val result = assertIs<SettingsForm.Result.Valid>(SettingsForm.check("http://10.0.2.2:1241", "", ""))
        assertEquals("http://10.0.2.2:1241", result.settings.server.base)
        assertNull(result.settings.oidc)
    }

    @Test fun anIssuerNeedsAClientId() {
        val result = assertIs<SettingsForm.Result.Invalid>(SettingsForm.check("https://crm.example.org", "https://id.example.org/realms/crm", " "))
        assertEquals(SettingsForm.Errors(null, null, clientIdMissing = true), result.errors)
    }

    @Test fun theIssuerIsCheckedLikeTheServer() {
        val result = assertIs<SettingsForm.Result.Invalid>(SettingsForm.check("https://crm.example.org", "http://id.example.org", "c"))
        assertEquals(UrlProblem.CleartextNotLoopback, result.errors.issuer)
    }

    @Test fun everyFieldIsReportedAtOnce() {
        val result = assertIs<SettingsForm.Result.Invalid>(SettingsForm.check("", "id.example.org", ""))
        assertEquals(SettingsForm.Errors(UrlProblem.Empty, UrlProblem.Scheme, clientIdMissing = true), result.errors)
    }

    @Test fun aGoodFormGivesTheSettings() {
        val result = assertIs<SettingsForm.Result.Valid>(SettingsForm.check("https://crm.example.org/", "https://id.example.org/realms/crm/", "crm-mobile"))
        assertEquals("https://id.example.org/realms/crm", result.settings.oidc?.issuer?.base)
        assertEquals("https://id.example.org/realms/crm/.well-known/openid-configuration", result.settings.oidc?.discoveryUrl)
        assertEquals("crm-mobile", result.settings.oidc?.clientId)
    }
}
