package io.github.contactrelationshipmanagement.mobile.core

import java.net.URI
import java.net.URLDecoder
import java.nio.charset.StandardCharsets
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class OidcTest {
    private val endpoints = OidcEndpoints("https://id.example.org/realms/crm", "https://id.example.org/auth", "https://id.example.org/token")
    private val settings = OidcSettings((ServerUrl.parse("https://id.example.org/realms/crm") as ServerUrl.Result.Valid).url, "crm-mobile")

    @Test fun rfc7636AppendixB() {
        assertEquals("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM", Pkce.challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"))
    }

    @Test fun aVerifierIsLongEnoughAndUnreserved() {
        val verifier = Pkce.verifier()
        assertTrue(verifier.length in 43..128)
        assertTrue(verifier.all { it.isLetterOrDigit() || it in "-._~" })
        assertTrue(Pkce.verifier() != verifier)
    }

    @Test fun theAuthorizationRequest() {
        val request = Oidc.authorizationRequest(endpoints, "crm-mobile", state = "s1", verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")
        val query = URI(request.url).rawQuery.split("&").associate {
            it.substringBefore("=") to URLDecoder.decode(it.substringAfter("="), StandardCharsets.UTF_8)
        }
        assertTrue(request.url.startsWith("https://id.example.org/auth?"))
        assertEquals("code", query["response_type"])
        assertEquals("crm-mobile", query["client_id"])
        assertEquals("io.github.contactrelationshipmanagement.mobile:/oauth/callback", query["redirect_uri"])
        assertEquals("openid", query["scope"])
        assertEquals("s1", query["state"])
        assertEquals("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM", query["code_challenge"])
        assertEquals("S256", query["code_challenge_method"])
        assertEquals("s1", request.state)
    }

    @Test fun anEndpointWithAQueryGetsAnAmpersand() {
        val request = Oidc.authorizationRequest(endpoints.copy(authorizationEndpoint = "https://id.example.org/auth?tenant=a"), "c")
        assertTrue(request.url.startsWith("https://id.example.org/auth?tenant=a&response_type=code"))
    }

    private val redirect = Oidc.REDIRECT_URI

    @Test fun aGoodCallbackGivesTheCode() =
        assertEquals(Oidc.Callback.Code("abc"), Oidc.parseCallback("$redirect?code=abc&state=s1", "s1"))

    @Test fun aWrongStateIsRefusedEvenWithACode() =
        assertEquals(Oidc.Callback.Error(Oidc.Callback.Reason.StateMismatch), Oidc.parseCallback("$redirect?code=abc&state=other", "s1"))

    @Test fun aMissingStateIsRefused() =
        assertEquals(Oidc.Callback.Error(Oidc.Callback.Reason.StateMismatch), Oidc.parseCallback("$redirect?code=abc", "s1"))

    @Test fun aProviderErrorIsAnError() =
        assertEquals(
            Oidc.Callback.Error(Oidc.Callback.Reason.ProviderError, "user said no"),
            Oidc.parseCallback("$redirect?error=access_denied&error_description=user%20said%20no&state=s1", "s1"),
        )

    @Test fun aMissingCodeIsAnError() {
        assertEquals(Oidc.Callback.Error(Oidc.Callback.Reason.MissingCode), Oidc.parseCallback("$redirect?state=s1", "s1"))
        assertEquals(Oidc.Callback.Error(Oidc.Callback.Reason.MissingCode), Oidc.parseCallback("$redirect?code=&state=s1", "s1"))
    }

    @Test fun anotherUriIsNotTheRedirect() {
        for (uri in listOf("https://evil.example.net/?code=abc&state=s1", "${redirect}x?code=abc&state=s1", "io.github.contactrelationshipmanagement.mobile://evil/oauth/callback?code=a&state=s1")) {
            assertEquals(Oidc.Callback.Error(Oidc.Callback.Reason.NotTheRedirect), Oidc.parseCallback(uri, "s1"), uri)
        }
    }

    @Test fun theTokenRequestHasTheVerifierAndNoSecret() {
        val body = Oidc.tokenRequestBody("c 1", "crm-mobile", "v")
        assertEquals(
            "grant_type=authorization_code&code=c%201&redirect_uri=io.github.contactrelationshipmanagement.mobile%3A%2Foauth%2Fcallback&client_id=crm-mobile&code_verifier=v",
            body,
        )
    }

    @Test fun discoveryMustNameTheConfiguredIssuer() {
        assertNull(Oidc.checkDiscovery(settings, endpoints))
        assertNull(Oidc.checkDiscovery(settings, endpoints.copy(issuer = "https://id.example.org/realms/crm/")))
        assertEquals(Oidc.DiscoveryProblem.IssuerMismatch, Oidc.checkDiscovery(settings, endpoints.copy(issuer = "https://evil.example.net")))
    }

    @Test fun discoveryEndpointsMustBeSecure() {
        assertEquals(Oidc.DiscoveryProblem.InsecureEndpoint, Oidc.checkDiscovery(settings, endpoints.copy(tokenEndpoint = "http://id.example.org/token")))
        assertEquals(Oidc.DiscoveryProblem.InsecureEndpoint, Oidc.checkDiscovery(settings, endpoints.copy(authorizationEndpoint = "javascript:alert(1)")))
    }
}
