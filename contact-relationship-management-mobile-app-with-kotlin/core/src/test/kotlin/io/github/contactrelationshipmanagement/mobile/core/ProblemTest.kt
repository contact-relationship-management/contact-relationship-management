package io.github.contactrelationshipmanagement.mobile.core

import kotlin.test.Test
import kotlin.test.assertEquals

class ProblemTest {
    @Test fun detailFirst() =
        assertEquals("summary is required", Problem.message(422, """{"title":"Unprocessable","detail":"summary is required"}"""))

    @Test fun thenTitle() = assertEquals("Not Found", Problem.message(404, """{"title":"Not Found","detail":""}"""))

    @Test fun thenStatus() {
        assertEquals("HTTP 502", Problem.message(502, "<html>bad gateway</html>"))
        assertEquals("HTTP 500", Problem.message(500, ""))
        assertEquals("HTTP 500", Problem.message(500, """["a"]"""))
        assertEquals("HTTP 500", Problem.message(500, """{"detail":42}"""))
    }
}
