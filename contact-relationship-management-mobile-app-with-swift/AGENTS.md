# AGENTS.md — iOS app (Swift, SwiftUI, Lily)

Read [spec/mobile-apps](../spec/mobile-apps/index.md) first (MOB-1 to MOB-12, CRM-D28).

- **Rules go in `CRMCore`** (Foundation only) with a Swift Testing test each; `CRMMobileUI` stays thin.
- **Nothing on disk** but `Settings` (MOB-1). Do not add a cache, Core Data, SwiftData, a file write or a log of a
  record. The token stays in memory (MOB-2).
- **Check, never correct**: an invalid URL or amount is an error (MOB-3, MOB-6), as the service treats an invalid input.
- **Lily for content**: `Card`, `SummaryList`, `Badge`, `Tag`, `Field`, `ErrorSummary`, `LilyAlert`, `InsetText`, drawn
  by `.lilyTheme(_:)`. Lily has no default text, so every string comes from `Strings.swift` and
  `Resources/en.lproj/Localizable.strings` (MOB-11). Show user text with `Text(verbatim:)`.
- **Swift 6 strict concurrency**; typed throws (`throws(APIError)`) on the client. `async let` loses the typed error,
  so fetch one after the other.
- Lily comes from `scripts/fetch-lily.sh` at the commit in `lily-design-system.lock`; do not point the package at a
  sibling checkout.
- Verify with `scripts/ci-check.sh mobile-swift`. Say what you did not run (a simulator session, a device, sign-in).
