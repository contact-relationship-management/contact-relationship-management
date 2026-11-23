package io.github.contactrelationshipmanagement.mobile.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PagingTest {
    private val server = (ServerUrl.parse("https://crm.example.org") as ServerUrl.Result.Valid).url

    @Test fun findsTheNextLink() {
        assertEquals("/api/contacts?cursor=abc", Paging.nextLink("</api/contacts?cursor=abc>; rel=\"next\""))
        assertEquals("/b", Paging.nextLink("</a>; rel=\"prev\", </b>; rel=next"))
        assertNull(Paging.nextLink("</a>; rel=\"prev\""))
        assertNull(Paging.nextLink(null))
    }

    @Test fun aRelativeLinkResolvesAgainstTheServer() =
        assertEquals("https://crm.example.org/api/contacts?cursor=abc", Paging.sameOrigin(server, "/api/contacts?cursor=abc"))

    @Test fun anAbsoluteLinkOnTheSameOriginIsFollowed() {
        assertEquals("https://crm.example.org/api/x", Paging.sameOrigin(server, "https://crm.example.org/api/x"))
        assertEquals("https://CRM.example.org:443/api/x", Paging.sameOrigin(server, "https://CRM.example.org:443/api/x"))
    }

    @Test fun anotherOriginIsRefused() {
        assertNull(Paging.sameOrigin(server, "https://evil.example.net/api/contacts"))
        assertNull(Paging.sameOrigin(server, "http://crm.example.org/api/contacts"))
        assertNull(Paging.sameOrigin(server, "https://crm.example.org:8443/api/contacts"))
        assertNull(Paging.sameOrigin(server, "//evil.example.net/api"))
        assertNull(Paging.sameOrigin(server, "not a uri"))
    }
}
