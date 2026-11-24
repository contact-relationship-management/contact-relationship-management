# AGENTS.md — Android app

Read [spec/mobile-apps](../spec/mobile-apps/index.md) first (MOB-1 to MOB-15, CRM-R111, CRM-D28); the root
[AGENTS.md](../AGENTS.md) applies too.

- **Logic goes in `core`**, plain Kotlin with a JVM test for each rule. `app` holds the activity, the view model and
  the Compose screens, and stays thin.
- **Nothing on disk but the three settings** (`SettingsStore`). No Room, no file cache, no HTTP cache, no logging of a
  record or a token. Backup is off.
- **Check input; never correct it.** A URL that fails MOB-3 is an error at the field. Amount text that is not an integer
  is an error, not zero; a masked amount is "Hidden".
- **The bearer token goes only to the server's origin.** A next link elsewhere is an error (MOB-8).
- **Every user-facing string is in `res/values/strings.xml`.** A new locale is a machine-written draft until a speaker
  reviews it; say so, and never call it reviewed.
- **Lily for content** (`LilyCard`, `SummaryList`, `LilyBadge`, `Tag`, `Field`, `ErrorSummary`, `LilyAlert`,
  `InsetText`); Material 3 only for the chrome (bar, navigation, text fields, buttons). Lily comes from the commit in
  `../lily-design-system.lock` (`scripts/fetch-lily.sh`); keep `gradle/libs.versions.toml` at Lily's AGP, Kotlin and
  Compose BOM versions.
- Verify with `./gradlew --no-daemon :core:test :app:testDebugUnitTest :app:assembleDebug`, and record what was not run
  (an emulator, a real identity provider, TalkBack).
