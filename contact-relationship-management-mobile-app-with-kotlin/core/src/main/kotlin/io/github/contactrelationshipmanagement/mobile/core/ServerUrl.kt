package io.github.contactrelationshipmanagement.mobile.core

import java.net.URI
import java.net.URISyntaxException

/** Why a typed URL was refused (MOB-3). The app shows the reason at the field; it never corrects the URL. */
public enum class UrlProblem { Empty, Unparsable, NoHost, Credentials, QueryOrFragment, Scheme, CleartextNotLoopback }

/** A checked base URL: `https`, or `http` to a loopback host, with no user, query or fragment, and no trailing `/`. */
@ConsistentCopyVisibility
public data class ServerUrl private constructor(val base: String) {
    /** The URI of [base], for comparing origins (MOB-8). */
    public val uri: URI get() = URI(base)

    /** [path] (starting with `/`) appended to the base URL. */
    public fun resolve(path: String): String = base + path

    public companion object {
        /** Hosts where `http` is allowed: the phone itself, or (Android emulator) the computer running it. */
        public val LOOPBACK_HOSTS: Set<String> = setOf("localhost", "127.0.0.1", "[::1]", "10.0.2.2")

        /** Checks [text] as typed (MOB-3). Only one trailing `/` is removed; nothing else is changed. */
        public fun parse(text: String): Result = check(text)

        private fun check(text: String): Result {
            if (text.isEmpty()) return Result.Invalid(UrlProblem.Empty)
            val uri = try {
                URI(text)
            } catch (_: URISyntaxException) {
                return Result.Invalid(UrlProblem.Unparsable)
            }
            if (uri.isOpaque || uri.scheme == null) return Result.Invalid(UrlProblem.Scheme)
            val scheme = uri.scheme.lowercase()
            if (scheme != "https" && scheme != "http") return Result.Invalid(UrlProblem.Scheme)
            if (uri.rawUserInfo != null) return Result.Invalid(UrlProblem.Credentials)
            val host = uri.host ?: return Result.Invalid(UrlProblem.NoHost)
            if (uri.rawQuery != null || uri.rawFragment != null) return Result.Invalid(UrlProblem.QueryOrFragment)
            if (scheme == "http" && host.lowercase() !in LOOPBACK_HOSTS) {
                return Result.Invalid(UrlProblem.CleartextNotLoopback)
            }
            return Result.Valid(ServerUrl(text.removeSuffix("/")))
        }
    }

    public sealed interface Result {
        public data class Valid(val url: ServerUrl) : Result
        public data class Invalid(val problem: UrlProblem) : Result
    }
}
