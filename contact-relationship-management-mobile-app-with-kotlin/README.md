# Contact Relationship Management for Android

A native Android client of the CRM service, for a sales rep or support agent away from a desk: look up a contact, see
their activities, deals and tickets, log a call, and see the deal pipeline and the ticket queue. Kotlin, Jetpack Compose,
Material 3 for the app's chrome and the [Lily Design System for Kotlin](https://lilydesignsystem.com/) for its content.

> ⚠️ **Not ready for production.** A first version (CRM-T303). It has been built and unit-tested, not run against a real
> identity provider or reviewed by users of TalkBack. See [Not built or not verified](#not-built-or-not-verified).

The rules are in [spec/mobile-apps](../spec/mobile-apps/index.md) (MOB-1 to MOB-12), the requirement is CRM-R111 and
the decision CRM-D28: the app calls the service's `/api` directly with a bearer token, keeps records in memory only, and
stores nothing but its three settings.

## Build and test

Needs JDK 17 or later and the Android SDK with platform 37 (`ANDROID_HOME`, or `sdk.dir` in a `local.properties` that
git ignores). Lily is not published yet, so fetch it at the pinned commit first:

```bash
scripts/fetch-lily.sh                               # from the repository root, once per pin
cd contact-relationship-management-mobile-app-with-kotlin
export ANDROID_HOME="$HOME/Library/Android/sdk"     # where the SDK is on macOS
./gradlew --no-daemon :core:test :app:testDebugUnitTest :app:assembleDebug
```

That is the `mobile-kotlin` stage of `scripts/ci-check.sh`. The APK is `app/build/outputs/apk/debug/app-debug.apk`.

| Module | What | Tests |
| --- | --- | --- |
| `core` | Plain Kotlin on the JVM: the models, the URL check, money, paging, PKCE and OIDC, the settings form, the session, and the API client over `HttpURLConnection` | JUnit, with the API client against an in-process `HttpServer` |
| `app` | The Android app: one activity, a view model, Compose screens | JUnit for the string table's coverage |

## Run against the demo server

The demo (`podman compose up --build` at the repository root) serves the API on the computer's `127.0.0.1:1241`, with
sign-in off and synthetic data only. From the Android emulator the computer is `10.0.2.2`:

1. Install the APK on an emulator: `adb install app/build/outputs/apk/debug/app-debug.apk`.
2. On the **Server** tab, set the server URL to `http://10.0.2.2:1241`, leave the issuer empty, and save.
3. The **Contacts** tab shows the demo contacts under a warning that the server has sign-in off.

If the app times out on `10.0.2.2` while `adb shell nc 10.0.2.2 1241` connects (seen on 2026-11-23 with the
`Pixel_10_API_37` image, which has both a cellular and a Wi-Fi link on `10.0.2.0/24`), forward the port instead:
`adb reverse tcp:1241 tcp:1241`, and set the server URL to `http://localhost:1241`.

`http` is allowed only to `localhost`, `127.0.0.1`, `::1` and `10.0.2.2` (MOB-3), in the URL check and in
`res/xml/network_security_config.xml`. A phone on the same network reaches a computer at another address, which needs
`https`.

## Sign-in

With an issuer URL and a client ID set, **Sign in** reads the issuer's discovery document, checks that it names that
issuer and uses `https`, and opens the authorization request (code flow, PKCE `S256`, scope `openid`) in the browser.
The identity provider redirects to `io.github.contactrelationshipmanagement.mobile:/oauth/callback`, which returns to
the app. The app checks `state`, exchanges the code with the verifier (no client secret), and holds the access token in
memory until the process ends (MOB-2). The deployer registers a public client with that redirect URI, and gives its
tokens an audience the service accepts.

## Not built or not verified

- **Not verified:** sign-in has not been tried against a real identity provider (Keycloak or another); the app has not
  been run on an emulator or a device, so no screen has been looked at; nothing has been tried with TalkBack, a large
  font scale or a right-to-left locale.
- **Not built:** everything in the spec's [Not built](../spec/mobile-apps/index.md#not-built) list, and: Custom Tabs (the
  default browser is opened with `ACTION_VIEW`); a refresh when the token expires (a `401` signs the user out);
  localized labels for the other values the service sends (a status and a priority are shown as sent; consent states and activity kinds have words); dates
  in the reader's format (shown as the service sends them); instrumented or screenshot tests.
- A private-use redirect scheme can be claimed by another app on the phone; PKCE is what keeps a stolen code useless
  (RFC 8252 §8.1). An App Link (`https` redirect) would be stronger, and needs a domain the deployer controls.
