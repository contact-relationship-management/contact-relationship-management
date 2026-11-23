package io.github.contactrelationshipmanagement.mobile.core

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject

/** What the app shows for a non-2xx answer (MOB-9): the problem document's `detail`, then `title`, then the status. */
public object Problem {
    public fun message(status: Int, body: String?): String {
        val document = parse(body)
        return text(document, "detail") ?: text(document, "title") ?: "HTTP $status"
    }

    private fun parse(body: String?): JsonObject? {
        if (body.isNullOrBlank()) return null
        return try {
            Json.parseToJsonElement(body).jsonObject
        } catch (_: IllegalArgumentException) {
            null
        }
    }

    private fun text(document: JsonObject?, key: String): String? {
        val value = document?.get(key) as? JsonPrimitive ?: return null
        if (!value.isString) return null
        return value.content.takeIf { it.isNotBlank() }
    }
}
