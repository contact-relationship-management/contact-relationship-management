package io.github.contactrelationshipmanagement.mobile.core

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.builtins.ListSerializer
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import java.util.UUID

/** Why a request did not give the app what it asked for. */
public sealed class ApiException(message: String, cause: Throwable? = null) : Exception(message, cause) {
    /** `401`: the token is missing, expired or refused; the user signs in again (MOB-9). */
    public class Unauthorized(message: String) : ApiException(message)

    /** Any other non-2xx answer, with the service's words (MOB-9). */
    public class Http(public val status: Int, message: String) : ApiException(message)

    /** A list longer than [Paging.MAX_PAGES] pages (MOB-8). */
    public class TooManyPages : ApiException("the list has more than ${Paging.MAX_PAGES} pages")

    /** A next link to another origin (MOB-8): not followed, so the token is not sent there. */
    public class ForeignNextLink(public val link: String) : ApiException("the next page is on another server: $link")

    /** The identity provider answered with something the app must not use (MOB-4). */
    public class Refused(message: String) : ApiException(message)

    /** The connection failed. */
    public class Network(cause: IOException) : ApiException(cause.message ?: "network error", cause)

    /** A 2xx answer whose body is not what the API documents. */
    public class Unreadable(cause: Throwable) : ApiException(cause.message ?: "unreadable answer", cause)
}

/** A whole list, and whether the server gave it without a token (a demo server, MOB-5). */
public data class Listed<T>(val rows: List<T>, val servedWithoutToken: Boolean)

/** One write's `Idempotency-Key` (MOB-7): keep the same instance across retries of the same form. */
public data class IdempotencyKey(val value: String = UUID.randomUUID().toString())

/** An HTTP answer, read whole. */
internal data class Answer(val status: Int, val body: String, val link: String?)

/** Blocking HTTP over [HttpURLConnection] with the cache off (MOB-1). Callers run it off the main thread. */
internal object Http {
    const val TIMEOUT_MILLIS: Int = 30_000

    fun send(url: String, method: String, headers: Map<String, String>, body: String? = null, contentType: String? = null): Answer {
        try {
            val connection = URI(url).toURL().openConnection() as HttpURLConnection
            try {
                connection.useCaches = false
                connection.instanceFollowRedirects = false
                connection.connectTimeout = TIMEOUT_MILLIS
                connection.readTimeout = TIMEOUT_MILLIS
                connection.requestMethod = method
                headers.forEach(connection::setRequestProperty)
                if (body != null) {
                    connection.doOutput = true
                    contentType?.let { connection.setRequestProperty("Content-Type", it) }
                    connection.outputStream.use { it.write(body.toByteArray(StandardCharsets.UTF_8)) }
                }
                val status = connection.responseCode
                val stream = if (status in 200..299) connection.inputStream else connection.errorStream
                val text = stream?.use { String(it.readBytes(), StandardCharsets.UTF_8) }.orEmpty()
                return Answer(status, text, connection.getHeaderField("Link"))
            } finally {
                connection.disconnect()
            }
        } catch (e: IOException) {
            throw ApiException.Network(e)
        }
    }
}

/** The service's REST API (spec/mobile-apps). [token] is read for each request; null sends none. */
public class ApiClient(public val server: ServerUrl, private val token: () -> String?) {
    public fun contacts(): Listed<Contact> = all("/api/contacts", Contact.serializer())

    public fun contact(pid: String): ContactDetail = one("/api/contacts/${checkedPid(pid)}", ContactDetail.serializer())

    public fun pipelines(): List<Pipeline> = all("/api/pipelines", Pipeline.serializer()).rows

    public fun deals(): List<Deal> = all("/api/deals", Deal.serializer()).rows

    public fun tickets(): List<Ticket> = all("/api/tickets", Ticket.serializer()).rows

    /** `POST /api/activities`, sent with [key]; a retry with the same key records one activity (MOB-7). */
    public fun logActivity(payload: ActivityPayload, key: IdempotencyKey): Activity {
        val answer = Http.send(
            server.resolve("/api/activities"),
            "POST",
            headers(token()) + ("Idempotency-Key" to key.value),
            ApiJson.encodeToString(ActivityPayload.serializer(), payload),
            "application/json",
        )
        return decode(checked(answer).body, Activity.serializer())
    }

    private fun <T> one(path: String, serializer: KSerializer<T>): T {
        val answer = Http.send(server.resolve(path), "GET", headers(token()))
        return decode(checked(answer).body, serializer)
    }

    /** Every page of a list, following `Link: rel="next"` on the same origin, up to [Paging.MAX_PAGES] (MOB-8). */
    private fun <T> all(path: String, serializer: KSerializer<T>): Listed<T> {
        val bearer = token()
        val rows = mutableListOf<T>()
        var url: String? = server.resolve(path)
        var pages = 0
        while (url != null) {
            if (pages >= Paging.MAX_PAGES) throw ApiException.TooManyPages()
            pages += 1
            val answer = checked(Http.send(url, "GET", headers(bearer)))
            rows += decode(answer.body, ListSerializer(serializer))
            url = Paging.nextLink(answer.link)?.let { link ->
                Paging.sameOrigin(server, link) ?: throw ApiException.ForeignNextLink(link)
            }
        }
        return Listed(rows, servedWithoutToken = bearer == null)
    }

    private fun headers(bearer: String?): Map<String, String> = buildMap {
        put("Accept", "application/json")
        put(ACCEPTS_VERSION, API_VERSION)
        if (bearer != null) put("Authorization", "Bearer $bearer")
    }

    public companion object {
        public const val ACCEPTS_VERSION: String = "Accepts-version"
        public const val API_VERSION: String = "1.0"

        /** A record id must be a UUID before it goes into a path. */
        internal fun checkedPid(pid: String): String {
            UUID.fromString(pid)
            require(pid.length == 36) { "not a record id: $pid" }
            return pid
        }
    }
}

internal fun checked(answer: Answer): Answer = when {
    answer.status in 200..299 -> answer
    answer.status == 401 -> throw ApiException.Unauthorized(Problem.message(answer.status, answer.body))
    else -> throw ApiException.Http(answer.status, Problem.message(answer.status, answer.body))
}

internal fun <T> decode(body: String, serializer: KSerializer<T>): T = try {
    ApiJson.decodeFromString(serializer, body)
} catch (e: SerializationException) {
    throw ApiException.Unreadable(e)
} catch (e: IllegalArgumentException) {
    throw ApiException.Unreadable(e)
}

/** Discovery and the token exchange (MOB-4). No bearer token and no `Accepts-version`: these go to the identity provider. */
public class OidcClient(public val settings: OidcSettings) {
    public fun discover(): OidcEndpoints {
        val answer = checked(Http.send(settings.discoveryUrl, "GET", mapOf("Accept" to "application/json")))
        val endpoints = decode(answer.body, OidcEndpoints.serializer())
        Oidc.checkDiscovery(settings, endpoints)?.let { throw ApiException.Refused("discovery document refused: $it") }
        return endpoints
    }

    public fun exchange(endpoints: OidcEndpoints, code: String, verifier: String): TokenResponse {
        val answer = Http.send(
            endpoints.tokenEndpoint,
            "POST",
            mapOf("Accept" to "application/json"),
            Oidc.tokenRequestBody(code, settings.clientId, verifier),
            "application/x-www-form-urlencoded",
        )
        val token = decode(checked(answer).body, TokenResponse.serializer())
        if (!token.tokenType.equals("Bearer", ignoreCase = true)) {
            throw ApiException.Refused("the identity provider sent a ${token.tokenType} token, not a bearer token")
        }
        return token
    }
}
