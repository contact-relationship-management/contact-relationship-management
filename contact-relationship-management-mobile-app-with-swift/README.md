# Contact Relationship Management — iOS app (Swift, SwiftUI, Lily)

A native iOS client of the CRM service for a sales rep or support agent away from a desk: contacts with their
activities, deals and tickets, deals by pipeline and stage, tickets with their SLA breaches, and logging a call, email,
meeting, note or task about a contact. The rules are in [spec/mobile-apps](../spec/mobile-apps/index.md) (CRM-R111,
CRM-D28).

> ⚠️ **Not ready for production**, like the rest of the project. Sign-in has not been tried against a real identity
> provider, and the app has had no accessibility audit and no App Store review.

## Layout

| Path | What |
| --- | --- |
| `Package.swift` | The package: `CRMCore` (Foundation only) and `CRMMobileUI` (SwiftUI with Lily's `LilyUI` and `LilyThemes`) |
| `Sources/CRMCore` | Models, the server URL check, money, paging, problem documents, PKCE and OIDC, the API client |
| `Sources/CRMMobileUI` | The screens, the in-memory app model, and the strings (`Resources/en.lproj`) |
| `Tests/CRMCoreTests` | Swift Testing tests of the rules and of the API client against a stubbed `URLProtocol` |
| `App/` | The Xcode project for the iOS app target, which is only `@main` and `Info.plist` |

## Build and test

Lily for Swift is not published yet; fetch it at the pinned commit first (once):

```sh
scripts/fetch-lily.sh                 # from the repository root
scripts/ci-check.sh mobile-swift      # swift test, then an iOS Simulator build
```

Or by hand, in this directory:

```sh
swift test
xcodebuild -project App/CRMMobile.xcodeproj -scheme CRMMobile \
  -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build
```

Needs Xcode 16 or later (Swift 6, the iOS 18 SDK); built here with Xcode 27.2. Open `App/CRMMobile.xcodeproj` in Xcode to run it in a simulator.

## Try it against the demo

`podman compose up --build` at the repository root serves the API on `http://localhost:1241` with sign-in off and
synthetic data. In the app's settings, enter `http://localhost:1241` as the server and leave the issuer empty. The app
shows the demo warning (MOB-5) while the server answers without a token.

## Sign-in

Set the issuer (for Keycloak, `https://<host>/realms/<realm>`) and the client ID of a **public** client with the
redirect URI `io.github.contactrelationshipmanagement.mobile:/oauth/callback` and PKCE `S256` required. The access
token's audience must be one the service accepts. The token is kept in memory only: closing the app signs you out.

## What is stored

The server, issuer and client ID, in `UserDefaults`. No record, and no token. The URL session is ephemeral with no
cache (MOB-1, MOB-2).

## Managed devices, tablets and no copy out

- **Managed configuration (MOB-14).** A deployer may supply `server_url`, `issuer_url` and `client_id` under
  `com.apple.configuration.managed`. Supplied values are shown as fixed and checked like typed ones; one that fails is an
  error, not a default. The configuration is read again each time the app becomes active. `ManagedConfig` in CRMCore
  holds the rule.
- **iPhone and iPad (MOB-13).** The Contacts tab is a split view: list and contact side by side on an iPad, one after the
  other on an iPhone.
- **No record text copied out (MOB-15).** Text selection is off, a blank cover hides every screen while the app is not
  active or the screen is captured or mirrored, autocorrection is off in the log form (iOS has no switch for keyboard
  learning on a plain field), and an unreadable answer is reported by field path, never by the text it held.

## Not built or not verified

See [spec/mobile-apps](../spec/mobile-apps/index.md#not-built). In particular: no UI tests, sign-in not tried against a
real identity provider, no device build (signing), English only.
