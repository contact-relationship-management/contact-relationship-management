package io.github.contactrelationshipmanagement.mobile.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class ManagedConfigTest {
    @Test fun nothingSuppliedLeavesTheUsersSettings() {
        assertEquals(ManagedConfig.Result.None, ManagedConfig.read(null, null, null))
    }

    @Test fun aValidSetIsManaged() {
        val result = assertIs<ManagedConfig.Result.Managed>(
            ManagedConfig.read("https://crm.example.org", "https://id.example.org/realms/crm", "mobile"),
        )
        assertEquals("https://crm.example.org", result.settings.server.base)
        assertEquals("mobile", result.settings.oidc?.clientId)
    }

    @Test fun aServerAloneIsManagedWithNoSignIn() {
        val result = assertIs<ManagedConfig.Result.Managed>(ManagedConfig.read("https://crm.example.org", null, null))
        assertEquals(null, result.settings.oidc)
    }

    @Test fun aBadServerIsAnErrorNotADefault() {
        val result = assertIs<ManagedConfig.Result.Invalid>(ManagedConfig.read("http://crm.example.org", null, null))
        assertEquals(UrlProblem.CleartextNotLoopback, result.errors.server)
    }

    @Test fun anIssuerWithoutAServerIsAnError() {
        val result = assertIs<ManagedConfig.Result.Invalid>(ManagedConfig.read(null, "https://id.example.org", "c"))
        assertEquals(UrlProblem.Empty, result.errors.server)
    }

    @Test fun aSuppliedBlankIsAnErrorNotAbsent() {
        assertIs<ManagedConfig.Result.Invalid>(ManagedConfig.read("", null, null))
    }

    @Test fun anIssuerNeedsAClientIdWhenManagedToo() {
        val result = assertIs<ManagedConfig.Result.Invalid>(ManagedConfig.read("https://crm.example.org", "https://id.example.org", null))
        assertEquals(true, result.errors.clientIdMissing)
    }
}
