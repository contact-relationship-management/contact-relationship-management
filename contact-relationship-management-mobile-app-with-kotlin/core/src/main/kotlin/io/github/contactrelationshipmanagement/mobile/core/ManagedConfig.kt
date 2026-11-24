package io.github.contactrelationshipmanagement.mobile.core

/**
 * The settings a device-management deployer supplies as application restrictions (MOB-14). A key that is not supplied
 * is `null`; one that is supplied is checked like a typed value, blank or not, and is never replaced by a default or by
 * what the user typed (RCA-IN-2).
 */
public object ManagedConfig {
    public sealed interface Result {
        /** Nothing supplied: the user's own settings apply. */
        public data object None : Result

        /** Supplied and valid: shown as fixed. */
        public data class Managed(val settings: Settings) : Result

        /** Supplied and not valid: an error at start-up. */
        public data class Invalid(val errors: SettingsForm.Errors) : Result
    }

    public fun read(server: String?, issuer: String?, clientId: String?): Result {
        if (server == null && issuer == null && clientId == null) return Result.None
        return when (val checked = SettingsForm.check(server.orEmpty(), issuer.orEmpty(), clientId.orEmpty())) {
            is SettingsForm.Result.Valid -> Result.Managed(checked.settings)
            is SettingsForm.Result.Invalid -> Result.Invalid(checked.errors)
        }
    }
}
