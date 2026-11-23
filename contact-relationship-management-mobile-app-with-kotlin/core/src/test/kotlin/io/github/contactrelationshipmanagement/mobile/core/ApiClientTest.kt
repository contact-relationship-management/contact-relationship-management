package io.github.contactrelationshipmanagement.mobile.core

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.nio.charset.StandardCharsets
import java.util.concurrent.CopyOnWriteArrayList
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ApiClientTest {
    private data class Seen(val method: String, val path: String, val query: String?, val headers: Map<String, String?>, val body: String)

    private lateinit var server: HttpServer
    private val seen = CopyOnWriteArrayList<Seen>()
    private val routes = mutableMapOf<String, (HttpExchange) -> Triple<Int, String, Map<String, String>>>()
    private lateinit var base: ServerUrl

    @BeforeTest fun start() {
        server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        server.createContext("/") { exchange ->
            val body = String(exchange.requestBody.readBytes(), StandardCharsets.UTF_8)
            val headers = listOf("Authorization", "Accepts-version", "Idempotency-Key", "Content-Type").associateWith { exchange.requestHeaders.getFirst(it) }
            seen += Seen(exchange.requestMethod, exchange.requestURI.path, exchange.requestURI.rawQuery, headers, body)
            val key = "${exchange.requestMethod} ${exchange.requestURI.path}" + (exchange.requestURI.rawQuery?.let { "?$it" } ?: "")
            val (status, text, extra) = routes[key]?.invoke(exchange) ?: Triple(404, """{"title":"Not Found"}""", emptyMap())
            extra.forEach { (k, v) -> exchange.responseHeaders.add(k, v) }
            val bytes = text.toByteArray(StandardCharsets.UTF_8)
            exchange.sendResponseHeaders(status, if (bytes.isEmpty()) -1 else bytes.size.toLong())
            if (bytes.isNotEmpty()) exchange.responseBody.use { it.write(bytes) } else exchange.close()
        }
        server.start()
        base = (ServerUrl.parse("http://127.0.0.1:${server.address.port}") as ServerUrl.Result.Valid).url
    }

    @AfterTest fun stop() = server.stop(0)

    private fun route(key: String, status: Int = 200, link: String? = null, body: () -> String) {
        routes[key] = { Triple(status, body(), if (link != null) mapOf("Link" to link) else emptyMap()) }
    }

    private fun contact(pid: String, name: String) =
        """{"pid":"$pid","display_name":"$name","person_ref":"person:$pid","job_title":null,"status":"active","marketing_consent":"granted","preferred_channel":"email","id":1,"extra":"ignored"}"""

    private val pidA = "00000000-0000-4000-8000-00000000000a"
    private val pidB = "00000000-0000-4000-8000-00000000000b"

    @Test fun followsNextLinksAndSendsTheHeaders() {
        route("GET /api/contacts", link = "</api/contacts?cursor=2>; rel=\"next\"") { "[${contact(pidA, "Ada")}]" }
        route("GET /api/contacts?cursor=2") { "[${contact(pidB, "Grace")}]" }
        val listed = ApiClient(base) { "tok" }.contacts()
        assertEquals(listOf("Ada", "Grace"), listed.rows.map { it.displayName })
        assertFalse(listed.servedWithoutToken)
        assertEquals(2, seen.size)
        seen.forEach {
            assertEquals("Bearer tok", it.headers["Authorization"])
            assertEquals("1.0", it.headers["Accepts-version"])
        }
    }

    @Test fun withoutATokenNoneIsSentAndTheListSaysSo() {
        route("GET /api/contacts") { "[]" }
        val listed = ApiClient(base) { null }.contacts()
        assertTrue(listed.servedWithoutToken)
        assertNull(seen.single().headers["Authorization"])
    }

    @Test fun aNextLinkToAnotherOriginIsNotFollowed() {
        route("GET /api/contacts", link = "<https://evil.example.net/api/contacts?cursor=2>; rel=\"next\"") { "[]" }
        val error = assertFailsWith<ApiException.ForeignNextLink> { ApiClient(base) { "tok" }.contacts() }
        assertEquals("https://evil.example.net/api/contacts?cursor=2", error.link)
        assertEquals(1, seen.size)
    }

    @Test fun moreThanFiftyPagesIsAnErrorNotAPartialList() {
        routes["GET /api/tickets"] = { Triple(200, "[]", mapOf("Link" to "</api/tickets?cursor=1>; rel=\"next\"")) }
        for (page in 1..60) {
            routes["GET /api/tickets?cursor=$page"] = { Triple(200, "[]", mapOf("Link" to "</api/tickets?cursor=${page + 1}>; rel=\"next\"")) }
        }
        assertFailsWith<ApiException.TooManyPages> { ApiClient(base) { null }.tickets() }
        assertEquals(Paging.MAX_PAGES, seen.size)
    }

    @Test fun fiftyPagesExactlyIsWhole() {
        routes["GET /api/tickets"] = { Triple(200, "[]", mapOf("Link" to "</api/tickets?cursor=1>; rel=\"next\"")) }
        for (page in 1 until Paging.MAX_PAGES) {
            val link = if (page + 1 < Paging.MAX_PAGES) mapOf("Link" to "</api/tickets?cursor=${page + 1}>; rel=\"next\"") else emptyMap()
            routes["GET /api/tickets?cursor=$page"] = { Triple(200, "[]", link) }
        }
        assertEquals(emptyList(), ApiClient(base) { null }.tickets())
        assertEquals(Paging.MAX_PAGES, seen.size)
    }

    @Test fun unauthorizedIsItsOwnError() {
        route("GET /api/contacts", status = 401) { """{"title":"Unauthorized","detail":"missing bearer token"}""" }
        val error = assertFailsWith<ApiException.Unauthorized> { ApiClient(base) { null }.contacts() }
        assertEquals("missing bearer token", error.message)
    }

    @Test fun aProblemDocumentGivesTheMessage() {
        route("GET /api/contacts/$pidA", status = 403) { """{"type":"about:blank","title":"Forbidden","detail":"not your contact"}""" }
        val error = assertFailsWith<ApiException.Http> { ApiClient(base) { "t" }.contact(pidA) }
        assertEquals(403, error.status)
        assertEquals("not your contact", error.message)
    }

    @Test fun aRecordIdMustBeAUuid() {
        assertFailsWith<IllegalArgumentException> { ApiClient(base) { "t" }.contact("../audits") }
        assertTrue(seen.isEmpty())
    }

    @Test fun theContactDetailIsRead() {
        route("GET /api/contacts/$pidA") {
            """{"contact":${contact(pidA, "Ada")},"activities":[{"pid":"$pidB","kind":"call","summary":"Rang","done":true,"occurred_at":"2026-11-01T10:00:00Z","subject_kind":"contact","subject_pid":"$pidA"}],
               "deals":[{"pid":"$pidB","name":"Renewal","currency":"GBP","amount_minor":null,"pipeline_pid":"$pidA","stage_pid":"$pidB","kanban_position":0,"won":false}],
               "tickets":[],"employments":[],"truncated":{"activities":false}}"""
        }
        val detail = ApiClient(base) { "t" }.contact(pidA)
        assertEquals("Ada", detail.contact.displayName)
        assertEquals("Rang", detail.activities.single().summary)
        assertNull(detail.deals.single().exactMinorText)
    }

    @Test fun anUnreadableBodyIsAnError() {
        route("GET /api/deals") { """{"not":"a list"}""" }
        assertFailsWith<ApiException.Unreadable> { ApiClient(base) { "t" }.deals() }
    }

    @Test fun aRetriedWriteReusesItsKey() {
        route("POST /api/activities", status = 201) {
            """{"pid":"$pidB","kind":"call","summary":"Rang","done":false,"occurred_at":"2026-11-01T10:00:00Z","subject_kind":"contact","subject_pid":"$pidA"}"""
        }
        val client = ApiClient(base) { "t" }
        val key = IdempotencyKey()
        val payload = ActivityPayload("call", "Rang", "contact", pidA)
        client.logActivity(payload, key)
        client.logActivity(payload, key)
        assertEquals(2, seen.size)
        assertEquals(listOf(key.value, key.value), seen.map { it.headers["Idempotency-Key"] })
        assertTrue(IdempotencyKey() != key)
        val sent = seen.first()
        assertEquals("application/json", sent.headers["Content-Type"])
        assertEquals("""{"kind":"call","summary":"Rang","subject_kind":"contact","subject_pid":"$pidA"}""", sent.body)
    }

    @Test fun aConnectionFailureIsANetworkError() {
        server.stop(0)
        assertFailsWith<ApiException.Network> { ApiClient(base) { null }.contacts() }
    }

    @Test fun discoveryAndTokenExchange() {
        val issuer = "http://127.0.0.1:${server.address.port}/realms/crm"
        route("GET /realms/crm/.well-known/openid-configuration") {
            """{"issuer":"$issuer","authorization_endpoint":"$issuer/auth","token_endpoint":"$issuer/token","jwks_uri":"x"}"""
        }
        route("POST /realms/crm/token") { """{"access_token":"at","token_type":"Bearer","expires_in":300}""" }
        val client = OidcClient(OidcSettings((ServerUrl.parse(issuer) as ServerUrl.Result.Valid).url, "crm-mobile"))
        val endpoints = client.discover()
        assertEquals("$issuer/token", endpoints.tokenEndpoint)
        assertEquals("at", client.exchange(endpoints, "code1", "verifier1").accessToken)
        val exchange = seen.last()
        assertEquals("application/x-www-form-urlencoded", exchange.headers["Content-Type"])
        assertNull(exchange.headers["Authorization"])
        assertTrue("code_verifier=verifier1" in exchange.body)
        assertTrue("client_secret" !in exchange.body)
    }

    @Test fun aDiscoveryDocumentForAnotherIssuerIsRefused() {
        val issuer = "http://127.0.0.1:${server.address.port}/realms/crm"
        route("GET /realms/crm/.well-known/openid-configuration") {
            """{"issuer":"https://evil.example.net","authorization_endpoint":"$issuer/auth","token_endpoint":"$issuer/token"}"""
        }
        val client = OidcClient(OidcSettings((ServerUrl.parse(issuer) as ServerUrl.Result.Valid).url, "c"))
        assertFailsWith<ApiException.Refused> { client.discover() }
    }

    @Test fun aTokenThatIsNotBearerIsRefused() {
        val issuer = "http://127.0.0.1:${server.address.port}"
        route("POST /token") { """{"access_token":"at","token_type":"mac"}""" }
        val client = OidcClient(OidcSettings((ServerUrl.parse(issuer) as ServerUrl.Result.Valid).url, "c"))
        assertFailsWith<ApiException.Refused> { client.exchange(OidcEndpoints(issuer, "$issuer/auth", "$issuer/token"), "c", "v") }
    }
}
