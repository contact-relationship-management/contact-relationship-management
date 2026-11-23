package io.github.contactrelationshipmanagement.mobile.core

/**
 * The signed-in state, in memory only (MOB-2): the access token, and the one sign-in attempt in flight. Nothing here is
 * written to storage; a new process starts signed out.
 */
public class Session {
    @Volatile public var accessToken: String? = null
        private set

    private var pending: Pending? = null

    private data class Pending(val request: AuthorizationRequest, val endpoints: OidcEndpoints)

    /** Starts a sign-in: the URL to open in the browser. An earlier attempt is forgotten. */
    @Synchronized
    public fun begin(endpoints: OidcEndpoints, clientId: String): String {
        val request = Oidc.authorizationRequest(endpoints, clientId)
        pending = Pending(request, endpoints)
        return request.url
    }

    /** What to exchange, if [callbackUri] answers the attempt in flight. The attempt is used up either way. */
    @Synchronized
    public fun complete(callbackUri: String): Completion {
        val attempt = pending ?: return Completion.Refused(Oidc.Callback.Error(Oidc.Callback.Reason.StateMismatch))
        pending = null
        return when (val callback = Oidc.parseCallback(callbackUri, attempt.request.state)) {
            is Oidc.Callback.Code -> Completion.Exchange(attempt.endpoints, callback.code, attempt.request.verifier)
            is Oidc.Callback.Error -> Completion.Refused(callback)
        }
    }

    public fun signedIn(token: String) {
        accessToken = token
    }

    public fun signOut() {
        accessToken = null
        synchronized(this) { pending = null }
    }

    public sealed interface Completion {
        public data class Exchange(val endpoints: OidcEndpoints, val code: String, val verifier: String) : Completion
        public data class Refused(val error: Oidc.Callback.Error) : Completion
    }
}
