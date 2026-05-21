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

## License

MIT — see [LICENSE](LICENSE).
