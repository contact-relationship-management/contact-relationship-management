package io.github.contactrelationshipmanagement.mobile

import android.content.Context

/** The three settings, the only things the app writes to storage (MOB-1). None is a secret. */
class SettingsStore(context: Context) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    data class Saved(val server: String, val issuer: String, val clientId: String)

    fun load(): Saved = Saved(
        prefs.getString(SERVER, "").orEmpty(),
        prefs.getString(ISSUER, "").orEmpty(),
        prefs.getString(CLIENT_ID, "").orEmpty(),
    )

    fun save(saved: Saved) {
        prefs.edit().putString(SERVER, saved.server).putString(ISSUER, saved.issuer).putString(CLIENT_ID, saved.clientId).apply()
    }

    private companion object {
        const val SERVER = "server_url"
        const val ISSUER = "issuer_url"
        const val CLIENT_ID = "client_id"
    }
}
