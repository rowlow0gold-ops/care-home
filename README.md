# 케어닥 (Care Doc) — Desktop

Nursing-station companion app for the **케어닥** Korean care-home franchise SaaS.

Tauri 2 + Vue 3 + TypeScript + Quasar. Runs on Windows / macOS / Linux nursing
station PCs at care facilities. Talks to the
[care-home-server](https://care.minhojan-world.site) over HTTPS — auth via JWT
stored in the OS keychain via `@tauri-apps/plugin-store`.

> This is the **client** of a multi-tenant SaaS. The server (with the data,
> tenant isolation, billing, and business logic) is intentionally not open
> source — this client repo is published under MIT to qualify for free
> code-signing via the [SignPath Foundation](https://about.signpath.io/).
> Anyone is welcome to fork the UI; functionality requires a paid care-home
> tenant on the server.

## Screens

15 pages covering residents, vitals, care logs, medications, meals, schedules,
staff, reports, accounting, insurance, settings. See `src/pages/`.

## Local dev

```bash
pnpm install
cp .env.example .env       # default points at production
pnpm tauri dev
```

## Production install

Built installers (`.msi`, `.nsis`, `.dmg`, `.deb`, `.AppImage`) are produced
by `.github/workflows/release.yml` when a `vX.Y.Z` tag is pushed.

## Stack

- Tauri 2 (Rust shell)
- Vue 3 + TypeScript + Quasar 2
- Pinia for state, Vue Router with role-based guards
- Chart.js for vitals timelines
- xlsx for client-side Excel export

## Downloads

Built installers (`.msi`, `.dmg`, `.deb`, `.AppImage`) are published on the
[GitHub Releases page](https://github.com/rowlow0gold-ops/care-home/releases)
on every `vX.Y.Z` tag.

> **Code signing.** Releases are signed using a code-signing certificate
> generously provided by the [SignPath Foundation](https://signpath.org/),
> the non-profit code-signing service for open-source projects. The
> SignPath Foundation makes Windows / macOS code signing accessible for
> independent maintainers who could not otherwise afford a commercial CA.

## Privacy

See [PRIVACY.md](PRIVACY.md). The desktop client is a thin UI over a remote
API; it does not collect telemetry of any kind.

## License

MIT — see [LICENSE](LICENSE).
