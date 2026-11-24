# Mobile apps (iOS and Android)

Two native clients for the people who work away from a desk: a sales rep between meetings, a support agent on call. They
are clients of the service's REST API, as the Svelte UI is, and hold no state of their own.

| Subproject | Platform | Stack |
| --- | --- | --- |
| [contact-relationship-management-mobile-app-with-swift](../../contact-relationship-management-mobile-app-with-swift/) | iOS 18 and later | Swift 6, SwiftUI, Lily Design System for Swift (`LilyUI`, `LilyThemes`) |
| [contact-relationship-management-mobile-app-with-kotlin](../../contact-relationship-management-mobile-app-with-kotlin/) | Android 8.0 (API 26) and later | Kotlin, Jetpack Compose, Lily Design System for Kotlin (`lily-compose`, `lily-compose-themes`) |

Requirement: [CRM-R111](../requirements.md#crm-r111--a-rep-or-agent-can-work-from-a-phone). Decision:
[CRM-D28](../design.md#crm-d28--the-mobile-apps-call-the-api-directly-with-a-bearer-token-and-store-nothing).
Tasks: CRM-T302 (Swift), CRM-T303 (Kotlin), CRM-T304 (gates).

## What the first version does

| Screen | API | Notes |
| --- | --- | --- |
| Server | none | The service's base URL, and the issuer and client ID for sign-in. `https` only, except `http` to a loopback host (MOB-3). |
| Sign in | the identity provider | OIDC authorization code with PKCE (MOB-4). Skipped when the server is a demo with sign-in off. |
| Contacts | `GET /api/contacts` | Every page, following `Link: rel="next"`; a filter on the display name, on the phone. |
| Contact | `GET /api/contacts/{pid}` | Name, job title, status, marketing consent, preferred channel; the contact's activities, deals and tickets. |
| Log activity | `POST /api/activities` | A call, email, meeting, note or task about the contact, with an `Idempotency-Key` (MOB-7). |
| Deals | `GET /api/pipelines`, `GET /api/deals` | Grouped by pipeline and stage, in stage order; the amount from its exact text (MOB-6). |
| Tickets | `GET /api/tickets` | Title, status, priority, and a badge for a breached first-response or resolution deadline. |

Every request sends `Accepts-version: 1.0` (the path stays `/api`; versioning is the header's).

## Rules

- **MOB-1 Nothing on disk but the settings.** The server URL, the identity provider's issuer URL and the client ID
  are kept (none is a secret). No contact, deal, ticket or activity is written to the phone's storage,
  caches or logs. A contact is a person-service record (CRM boundary, [scope.md](../scope.md)); a copy on a lost phone
  is a copy outside every retention and erasure rule. The HTTP cache is off (an ephemeral `URLSession` with no `URLCache`;
  `HttpURLConnection` with `useCaches = false` and no response cache installed).
- **MOB-2 Tokens are held in memory.** The access token lives in the process and goes when the app ends; the user signs
  in again. A refresh token is not requested (`offline_access` is not asked for). Keeping a token in the Keychain or the
  Android Keystore is a later task, for a deployer who asks for it.
- **MOB-3 The server URL is checked, never corrected.** The issuer URL is checked the same way. It must parse, have a host, carry no user,
  password, query or fragment, and use `https`, or `http` to `localhost`, `127.0.0.1`, `::1` or (Android emulator) `10.0.2.2`. Anything else
  is an error shown at the field; the app does not add a scheme, strip a path or swap `http` for `https`. A trailing
  `/` is the one thing removed, so `…/` and `…` name the same server.
- **MOB-4 Sign-in is OIDC with PKCE, in the system browser.** With no issuer set, the app does not sign in (a demo
  server). Authorization code flow with PKCE `S256` (RFC 7636), a
  public client (no secret in the app), scope `openid`, the redirect
  `io.github.contactrelationshipmanagement.mobile:/oauth/callback` (a private-use scheme) and the system browser (RFC 8252): `ASWebAuthenticationSession` on iOS, a Custom
  Tab or the default browser on Android. `state` is checked on return; a mismatch, an `error` parameter or a missing
  `code` is an error, and nothing is exchanged. The discovery document gives the endpoints; it must name the configured issuer (OpenID
  Connect Discovery §4.3), and each endpoint must pass the MOB-3 check. The token's audience must be
  one the service accepts (the deployer configures a mobile client in the same realm; [auth.md](../auth.md), CRM-D27).
- **MOB-5 A demo server says so.** When `GET /api/contacts` answers without a token, the app shows a warning that the
  server has sign-in off and holds synthetic data only.
- **MOB-6 Money is exact and a masked amount is not zero.** An amount is read from `amount_minor_text` (exact
  base-ten text), scaled by the currency's minor-unit digits in decimal arithmetic (`Decimal`, `BigDecimal`), never
  through a binary float, and shown with the platform's currency formatter in the reader's locale. `null` is shown as
  "Hidden". A currency code the platform does not know is shown with two minor digits and its code. Text that is not
  an optional `-` followed by one or more digits is an error, not zero.
- **MOB-7 A write is sent once.** `POST /api/activities` carries an `Idempotency-Key`; a retry of the same form reuses
  the key, so a dropped connection followed by a retry records one activity.
- **MOB-8 A list is whole or an error.** A list follows `Link: rel="next"` up to 50 pages and fails past that, as the
  Svelte client does (`MAX_PAGES`); a partial list is never shown as complete. A next link is resolved against the
  server URL and must keep its scheme, host and port; one that points elsewhere is an error, so the bearer token is
  never sent to another origin. No request follows a redirect: a `3xx` is an error, for the same reason.
- **MOB-9 Errors are the service's words.** A non-2xx answer shows the problem document's `detail` or `title` (RFC
  9457) when there is one, else the status; `401` sends the user back to sign in.
- **MOB-10 Accessibility.** Content uses Lily components (`Card`, `SummaryList`, `Badge`, `Tag`, `Field`,
  `ErrorSummary`, `LilyAlert`), which carry the VoiceOver and TalkBack semantics; text follows Dynamic Type and the
  system font scale; a status is never colour alone.
- **MOB-11 English first, translatable.** Every user-facing string is in one table per app (`Strings.swift`'s keys and
  the Android `strings.xml`) so a locale can be added; v1 ships English only. Any other language would be a
  machine-written draft until a speaker reviews it.
- **MOB-12 Consent is shown, not changed.** The apps show a contact's marketing consent and never send anything to a
  contact. Changing consent stays in the web UI, which records its evidence (CRM-D21).
- **MOB-13 Phones and tablets.** Each app runs on phones and on tablets: the iOS app on iPhone and iPad, the Android
  app on phones and tablets. A screen uses the width it has: a list and the record it opens may sit side by side on a
  wide screen and one after the other on a narrow one, and nothing is cut off, hidden or made unreachable by the
  screen size, the orientation, or a split-screen or multi-window layout. Text follows Dynamic Type and the system font
  scale at every size class (MOB-10).
- **MOB-14 Managed and personal devices.** Each app works on a device the organization manages (through mobile device
  management) and on a personal device (BYOD), by the same rules, and needs no enrolment, device-administrator rights or
  configuration profile to run.
  - *Managed.* A deployer may supply the server URL, the issuer and the client ID as managed app configuration (iOS
    `com.apple.configuration.managed`, Android application restrictions). A value supplied that way is shown and cannot be
    edited in the app, and it passes the MOB-3 check like a typed one: one that fails is an error at start-up, not
    replaced by a default or by a typed value (RCA-IN-2). Removing the configuration returns the settings to what the user
    typed. Wiping an app's data, blocking copy to other apps and requiring a passcode are the device management's to
    enforce; the app does not implement or weaken them.
  - *Personal.* The user types the settings, as now. MOB-1 and MOB-2 are what make a personal device acceptable: no
    record is written to its storage and no token outlives the process, so nothing of the organization's stays on it
    after the app ends. The app does not read the address book, photos, other apps, or a device identifier, and does not
    ask for the permissions that would. Record text is never copied onto it (MOB-15).
- **MOB-15 Record text is never copied onto a device.** The text of a record (a contact's name, job title, phone and
  email; an activity's subject and note; a deal's or ticket's title; any other field the service returns) MUST NOT be
  copied out of the app onto the device, whoever owns it. The app cannot tell a personal device from a managed one with
  any certainty, so the rule holds on every install; there is no setting that relaxes it. The app has no way to do it,
  not a policy against it:
  - *No copy out.* Record text cannot be selected, copied, dragged, shared, printed or exported, and no screen offers
    "copy", "share" or "open in" for it. The app never writes record text to the clipboard or the pasteboard, and does
    not take part in Universal Clipboard or Handoff.
  - *No index, no surface outside the app.* Record text is not given to Spotlight, the Android app search or shortcuts,
    autofill, a notification, a widget, a Live Activity, a share extension, an app intent or Siri, Assistant or
    Shortcuts. The app sends no notification that carries record text.
  - *No trace in logs.* No log line, crash report, analytics event or error message the user can copy carries record text;
    an error names the field or the status, not the value (as `json::stored` does in the service).
  - *Hidden when not in front.* The app-switcher snapshot and the recent-apps thumbnail show no record text (a blank or
    blurred cover on iOS; `FLAG_SECURE` on Android). On Android a screenshot and screen recording of a record screen are
    blocked by the same flag; on iOS the app hides record text while the screen is being captured or mirrored.
  - *Typing does not teach the keyboard.* The log-activity form turns off keyboard learning and autocorrect personalization
    (`IME_FLAG_NO_PERSONALIZED_LEARNING` on Android; on iOS autocorrection off, because UIKit has no learning switch for a field that is not secure), so what the rep types is
    not kept in the device's dictionary. What the user typed is held in memory until it is sent and then dropped (MOB-1).
  - *What the app cannot stop, and says so.* A photograph of the screen, a person reading it, an iOS screenshot taken
    in a way the app is not told about, and a device that is itself compromised. The rule is that the app does not copy
    the text, not that no copy can exist.

## Lily Design System

Lily's Swift package and Kotlin libraries are not published to a registry yet, and live in subdirectories of the Lily
monorepo, which SwiftPM and Gradle cannot depend on directly. `scripts/fetch-lily.sh` makes a shallow, sparse clone of
the monorepo at the commit pinned in `lily-design-system.lock` into `vendor/lily-design-system/` (ignored by git), and
checks the commit it got. The Swift package depends on it by path; the Android build includes it as a composite build
and substitutes `io.github.lilydesignsystem:lily-compose` and `lily-compose-themes`. The repository therefore still
builds on its own: it needs the public Lily repository, as it needs crates.io and npm, and not any sibling checkout.
`CONTACT_RELATIONSHIP_MANAGEMENT_LILY_REMOTE` points the script at another clone (a local mirror) of the same history.

When Lily publishes (Maven Central `io.github.lilydesignsystem`, a SwiftPM-ready repository), the apps switch to the
published coordinates and the fetch script goes.

## Tests

| Layer | Swift | Kotlin |
| --- | --- | --- |
| Pure rules (URL check, money, Link header, PKCE, problem documents, grouping) | Swift Testing, `swift test` on macOS | JUnit, `./gradlew :core:test` on the JVM |
| API client against canned answers | `URLProtocol` stub | an in-process `HttpServer` from the JDK |
| Build of the app | `xcodebuild` for the iOS Simulator | `./gradlew :app:assembleDebug` |

`scripts/ci-check.sh mobile-swift` and `scripts/ci-check.sh mobile-kotlin` run them. They are not in `all` and not in
GitHub Actions yet: the Swift stage needs macOS with Xcode and the Kotlin stage the Android SDK (CRM-T304).

## Not built

A run on a device, emulator or simulator of managed configuration (MOB-14; the rules in both cores are tested) and of the checks that no record text leaves the app (MOB-15: `FLAG_SECURE`, the iOS cover, hiding while captured, keyboard learning, which iOS cannot turn off on a plain field), a test of the system's network-error text and of a crash report (the unreadable-answer error is tested, and neither app logs), a contact list beside the record on a tablet checked on a real device, tests of split-screen and multi-window, the other tabs laid out for a wide screen, a test with a mobile device management product, offline use, push notifications, editing a contact, deal or ticket, moving a deal between stages, consent changes,
search through `/api/search`, attachments, other locales, tokens kept across launches, biometric unlock, an App Store or
Play Store build (signing, privacy labels, store listing), UI tests on a device or simulator, and an accessibility audit
with VoiceOver or TalkBack users.
