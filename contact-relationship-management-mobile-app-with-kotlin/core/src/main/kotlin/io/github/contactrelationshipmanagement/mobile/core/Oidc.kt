package io.github.contactrelationshipmanagement.mobile.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import java.net.URI
import java.net.URISyntaxException
import java.net.URLDecoder
import java.net.URLEncoder
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.Base64

/** PKCE (RFC 7636) with `S256`. */
public object Pkce {
    private val URL_SAFE = Base64.getUrlEncoder().withoutPadding()

    /** A verifier of 43 characters from 32 random bytes (RFC 7636 §4.1 allows 43 to 128). */
    public fun verifier(random: SecureRandom = SecureRandom()): String = token(random, 32)

    /** A random `state` value, checked on return. */
    public fun state(random: SecureRandom = SecureRandom()): String = token(random, 16)

    /** BASE64URL(SHA256(ASCII(verifier))), without padding. */
    public fun challenge(verifier: String): String =
        URL_SAFE.encodeToString(MessageDigest.getInstance("SHA-256").digest(verifier.toByteArray(StandardCharsets.US_ASCII)))

    private fun token(random: SecureRandom, bytes: Int): String = URL_SAFE.encodeToString(ByteArray(bytes).also(random::nextBytes))
}

/** The two endpoints the app reads from the discovery document. */
@Serializable
public data class OidcEndpoints(
    val issuer: String,
    @SerialName("authorization_endpoint") val authorizationEndpoint: String,
    @SerialName("token_endpoint") val tokenEndpoint: String,
)

@Serializable
public data class TokenResponse(
    @SerialName("access_token") val accessToken: String,
    @SerialName("token_type") val tokenType: String,
    @SerialName("expires_in") val expiresIn: Long? = null,
)

/** The sign-in settings (MOB-1, MOB-4): the issuer and the client ID; none is a secret. */
public data class OidcSettings(val issuer: ServerUrl, val clientId: String) {
    /** The discovery document's URL. */
    public val discoveryUrl: String get() = issuer.resolve("/.well-known/openid-configuration")
}

/** One sign-in attempt: what is sent to the browser and what is kept, in memory, to check the return. */
public data class AuthorizationRequest(val url: String, val state: String, val verifier: String)

public object Oidc {
    public const val REDIRECT_URI: String = "io.github.contactrelationshipmanagement.mobile:/oauth/callback"
    public const val SCOPE: String = "openid"

    /** The authorization request (RFC 6749 §4.1.1 with RFC 7636): code flow, `S256`, scope `openid`. */
    public fun authorizationRequest(
        endpoints: OidcEndpoints,
        clientId: String,
        state: String = Pkce.state(),
        verifier: String = Pkce.verifier(),
    ): AuthorizationRequest {
        val query = form(
            "response_type" to "code",
            "client_id" to clientId,
            "redirect_uri" to REDIRECT_URI,
            "scope" to SCOPE,
            "state" to state,
            "code_challenge" to Pkce.challenge(verifier),
            "code_challenge_method" to "S256",
        )
        val separator = if (endpoints.authorizationEndpoint.contains('?')) "&" else "?"
        return AuthorizationRequest(endpoints.authorizationEndpoint + separator + query, state, verifier)
    }

    /** What came back to the redirect URI. Only [Callback.Code] may be exchanged. */
    public sealed interface Callback {
        public data class Code(val code: String) : Callback
        public data class Error(val reason: Reason, val detail: String? = null) : Callback
        public enum class Reason { NotTheRedirect, StateMismatch, ProviderError, MissingCode }
    }

    /** Checks the redirect [uri] against the [expectedState] (MOB-4). */
    public fun parseCallback(uri: String, expectedState: String): Callback {
        val parsed = try {
            URI(uri)
        } catch (_: URISyntaxException) {
            return Callback.Error(Callback.Reason.NotTheRedirect)
        }
        if ("${parsed.scheme}:${parsed.rawPath}" != REDIRECT_URI || parsed.rawAuthority != null) {
            return Callback.Error(Callback.Reason.NotTheRedirect)
        }
        val params = parseQuery(parsed.rawQuery)
        if (params["state"] != expectedState) return Callback.Error(Callback.Reason.StateMismatch)
        params["error"]?.let { return Callback.Error(Callback.Reason.ProviderError, params["error_description"] ?: it) }
        val code = params["code"]?.takeIf { it.isNotEmpty() } ?: return Callback.Error(Callback.Reason.MissingCode)
        return Callback.Code(code)
    }

    /** Why a discovery document was refused. */
    public enum class DiscoveryProblem { IssuerMismatch, InsecureEndpoint }

    /**
     * Checks a discovery document against the configured [settings]: its `issuer` must be the one configured (OIDC
     * Discovery §4.3), and each endpoint `https`, or `http` to a loopback host as for the server (MOB-3). Null when it
     * passes.
     */
    public fun checkDiscovery(settings: OidcSettings, endpoints: OidcEndpoints): DiscoveryProblem? {
        if (endpoints.issuer.removeSuffix("/") != settings.issuer.base) return DiscoveryProblem.IssuerMismatch
        for (endpoint in listOf(endpoints.authorizationEndpoint, endpoints.tokenEndpoint)) {
            if (!secure(endpoint)) return DiscoveryProblem.InsecureEndpoint
        }
        return null
    }

    private fun secure(endpoint: String): Boolean {
        val uri = try {
            URI(endpoint)
        } catch (_: URISyntaxException) {
            return false
        }
        val host = uri.host?.lowercase() ?: return false
        return when (uri.scheme?.lowercase()) {
            "https" -> true
            "http" -> host in ServerUrl.LOOPBACK_HOSTS
            else -> false
        }
    }

    /** The token request's form body (RFC 6749 §4.1.3 with the PKCE verifier; no client secret). */
    public fun tokenRequestBody(code: String, clientId: String, verifier: String): String = form(
        "grant_type" to "authorization_code",
        "code" to code,
        "redirect_uri" to REDIRECT_URI,
        "client_id" to clientId,
        "code_verifier" to verifier,
    )

    internal fun form(vararg pairs: Pair<String, String>): String =
        pairs.joinToString("&") { (k, v) -> "${encode(k)}=${encode(v)}" }

    private fun encode(text: String): String = URLEncoder.encode(text, StandardCharsets.UTF_8).replace("+", "%20")

    private fun parseQuery(query: String?): Map<String, String> {
        if (query.isNullOrEmpty()) return emptyMap()
        return query.split("&").filter { it.isNotEmpty() }.associate { part ->
            val key = part.substringBefore("=")
            val value = if ('=' in part) part.substringAfter("=") else ""
            URLDecoder.decode(key, StandardCharsets.UTF_8) to URLDecoder.decode(value, StandardCharsets.UTF_8)
        }
    }
}
