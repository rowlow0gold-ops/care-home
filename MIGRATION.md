# care-home → server-mode migration

The desktop app started life as a **single-device, local-SQLCipher** clinic
tool. The product is moving to **multi-tenant SaaS** powered by
[`care-home-server`](https://github.com/rowlow0gold-ops/care-home-server). This
file tracks the swap.

## Architecture target

```
┌──────────────────────────┐
│  care-home (this repo)   │
│  Tauri 2 desktop         │
│  Vue 3 + Quasar + Pinia  │
└────────────┬─────────────┘
             │ HTTPS + JWT (Tauri plugin-store, OS keychain)
             ▼
┌──────────────────────────┐
│   care-home-server       │
│   axum + sqlx + RLS      │
│   care.minhojan-world... │
└──────────────────────────┘
```

The local SQLite stays for **offline cache only** (v0.2). It is no longer the
source of truth.

## What's shipped (foundation)

- `src/lib/server.ts` — HTTP client. Reads `VITE_API_BASE` env, persists JWT
  via `@tauri-apps/plugin-store` (encrypted on disk).
- `src/stores/server-session.ts` — new Pinia store wrapping the server client.
  Coexists with `auth.ts` (legacy rusqlite) during migration.
- `src/pages/ServerLoginPage.vue` — email + password login form against the
  server. Use as a drop-in replacement for `LoginPage.vue` once main.ts /
  router.ts are switched over.
- `src-tauri/Cargo.toml` — adds `tauri-plugin-store` Rust dep.
- `src-tauri/src/lib.rs` — registers the store plugin.
- `src-tauri/capabilities/default.json` — grants `store:*` permissions.
- `package.json` — adds `@tauri-apps/plugin-store` and `@tauri-apps/plugin-dialog`.

## What's NOT shipped yet (page-by-page work)

| Page | Action |
|---|---|
| ResidentsPage.vue | replace `invoke('list_residents')` → `server.residents()` |
| CareLogPage.vue | `invoke('list_care_logs', ...)` → `server.careLogsFor(id)` |
| HealthChartsPage.vue | `invoke('list_vitals', ...)` → `server.vitalsFor(id)` |
| MedicationsPage.vue | `invoke('list_medications', ...)` → `server.medsFor(id)` |
| StaffPage.vue | `invoke('list_users')` → `server.staff()` |
| AccountingPage.vue | depends on billing slice (server task #45) |
| InsurancePage.vue | depends on billing slice (server task #45) |
| LoginPage.vue | switch `main.ts` to route to `ServerLoginPage.vue` instead |
| ReportsPage.vue | depends on dashboard summary endpoint (✓ already shipped) |

## Role rename

Legacy roles (in rusqlite users table): `staff`, `manager`, `admin`.
Server roles: `caregiver`, `nurse`, `branch_manager`, `hq`, `super_admin`.

The `useServerSessionStore.hasRole()` helper uses server role names. When
migrating a page, update its role checks:

```diff
- if (auth.role === 'manager') { ... }
+ if (serverSession.hasRole('branch_manager')) { ... }
```

## Removal of rusqlite (final step)

After all pages migrate:
1. Delete `src-tauri/src/commands/*` (except `save_excel.rs`, which is local-only).
2. Drop `rusqlite`, `bcrypt` from `Cargo.toml`.
3. Drop `src-tauri/src/db/` entirely.
4. Delete `src/stores/auth.ts` (legacy).
5. Delete `src/pages/LoginPage.vue`, rename `ServerLoginPage.vue` → `LoginPage.vue`.

The Tauri shell then only handles: native dialogs, store plugin (encrypted JWT),
and future offline cache.
