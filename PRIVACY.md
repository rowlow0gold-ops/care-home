# Privacy Policy

Last updated: 2026-05-21

**케어닥 Desktop (care-home)** is a thin client application that connects to a
remote API maintained by the 케어닥 service operator. This document covers
**only the desktop client's behavior** — it does not describe the server's
data handling.

## What the desktop client does

- Sends your email and password to the configured server (default:
  `https://care.minhojan-world.site`) once, when you log in, to obtain a
  JSON Web Token (JWT).
- Stores that JWT in the operating system's encrypted application data
  store (via `@tauri-apps/plugin-store`) so you remain signed in across
  application restarts.
- Sends authenticated HTTP requests (with the JWT) to the configured server
  to read and write care-home data (residents, vitals, care logs, etc.).

## What the desktop client does NOT do

- It does **not** send any telemetry, analytics, crash reports, or usage
  data to any third party.
- It does **not** contact any service other than the server URL you (or
  your administrator) configure in `VITE_API_BASE`.
- It does **not** include any embedded advertising, fingerprinting, or
  third-party tracking SDK.

## Data stored locally

| Item | Location | Purpose |
|---|---|---|
| JWT access token | OS keychain / app data dir (via Tauri store plugin) | Persist your sign-in across restarts |
| User profile (email, name, role) | Same as above | Display in the UI |

To delete the local data, sign out from the app, uninstall it, or delete the
application data directory for `world.minhojan.carehome.desktop`.

## Server-side handling

All resident, staff, vitals, care-log, medication, and billing data is stored
on the server you connect to, under the privacy policy of the care-home
operator that runs that server. If you are a caregiver or staff member, ask
your facility's administrator for the applicable server-side privacy notice.

## Contact

For questions about this client's privacy behaviour, open an issue at
https://github.com/rowlow0gold-ops/care-home/issues.
