package io.github.contactrelationshipmanagement.mobile.core

import java.net.URI
import java.net.URISyntaxException

public object Paging {
    /** Pages followed before a list is refused as too long (MOB-8, the Svelte client's `MAX_PAGES`). */
    public const val MAX_PAGES: Int = 50

    private val NEXT = Regex("""^\s*<([^>]*)>\s*;\s*rel="?next"?\s*$""")

    /** The target of a `Link: <...>; rel="next"` header, if any. */
    public fun nextLink(header: String?): String? {
        if (header == null) return null
        for (part in header.split(",")) {
            val match = NEXT.matchEntire(part) ?: continue
            return match.groupValues[1]
        }
        return null
    }

    /**
     * [link] resolved against [server]; null when it names another scheme, host or port, so the bearer token is never
     * sent to another origin (MOB-8).
     */
    public fun sameOrigin(server: ServerUrl, link: String): String? {
        val base = server.uri
        val target = try {
            base.resolve(URI(link))
        } catch (_: URISyntaxException) {
            return null
        } catch (_: IllegalArgumentException) {
            return null
        }
        val sameScheme = target.scheme.equals(base.scheme, ignoreCase = true)
        val sameHost = target.host.equals(base.host, ignoreCase = true)
        val samePort = portOf(target) == portOf(base)
        return if (sameScheme && sameHost && samePort) target.toString() else null
    }

    private fun portOf(uri: URI): Int = when {
        uri.port != -1 -> uri.port
        uri.scheme.equals("https", ignoreCase = true) -> 443
        uri.scheme.equals("http", ignoreCase = true) -> 80
        else -> -1
    }
}
