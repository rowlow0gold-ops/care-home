/**
 * HTTP client for care-home-server. Desktop counterpart of the tablet's
 * lib/api.ts, but persists the JWT via Tauri plugin-store (OS keychain).
 *
 * Migration in progress: existing pages still use `invoke()` against the
 * local SQLite. Replace one page at a time:
 *   - Old:   const x = await invoke('list_residents')
 *   - New:   const x = await server.residents()
 */
import { Store } from "@tauri-apps/plugin-store";

const API_BASE =
  (import.meta.env.VITE_API_BASE as string | undefined) ??
  "https://care.minhojan-world.site";

const STORE_FILE = "session.dat";
const TOKEN_KEY = "access_token";
const EXPIRES_KEY = "expires_at";
const USER_KEY = "user";

interface MeUser {
  id: string;
  email: string;
  name: string;
  role: string;
  tenant_id: string;
  branch_id: string | null;
  branch_name: string | null;
  tenant_name: string | null;
}

let backing: Store | null = null;
async function backingStore() {
  if (!backing) backing = await Store.load(STORE_FILE);
  return backing;
}

async function getToken(): Promise<string | null> {
  const s = await backingStore();
  const t = await s.get<string>(TOKEN_KEY);
  if (!t) return null;
  const exp = await s.get<number>(EXPIRES_KEY);
  if (exp && Date.now() / 1000 >= exp) return null;
  return t;
}

async function setSession(token: string, ttlSec: number, user: MeUser) {
  const s = await backingStore();
  await s.set(TOKEN_KEY, token);
  await s.set(EXPIRES_KEY, Math.floor(Date.now() / 1000) + ttlSec - 60);
  await s.set(USER_KEY, user);
  await s.save();
}

async function clearSession() {
  const s = await backingStore();
  await s.delete(TOKEN_KEY);
  await s.delete(EXPIRES_KEY);
  await s.delete(USER_KEY);
  await s.save();
}

async function loadUser(): Promise<MeUser | null> {
  const s = await backingStore();
  return (await s.get<MeUser>(USER_KEY)) ?? null;
}

export interface ServerError extends Error {
  status: number;
  body?: unknown;
}

async function fetchJson<T>(
  path: string,
  init: RequestInit = {},
  authed = true,
): Promise<T> {
  const headers = new Headers(init.headers);
  if (authed) {
    const t = await getToken();
    if (!t) {
      const err = new Error("not authenticated") as ServerError;
      err.status = 401;
      throw err;
    }
    headers.set("Authorization", `Bearer ${t}`);
  }
  if (init.body && !(init.body instanceof FormData)) {
    headers.set("Content-Type", "application/json");
  }
  const res = await fetch(`${API_BASE}${path}`, { ...init, headers });
  if (!res.ok) {
    const err = new Error(`API ${res.status} ${res.statusText}`) as ServerError;
    err.status = res.status;
    try {
      err.body = await res.json();
    } catch {
      // ignore
    }
    throw err;
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const server = {
  // === session ===
  async login(email: string, password: string): Promise<MeUser> {
    const res = await fetchJson<{
      access_token: string;
      expires_in: number;
    }>(
      "/api/v1/auth/login",
      { method: "POST", body: JSON.stringify({ email, password }) },
      false,
    );
    // hydrate user from /me to get joined branch_name etc
    const meRes = await fetchJson<{ user: MeUser }>("/api/v1/auth/me", {
      method: "GET",
      headers: { Authorization: `Bearer ${res.access_token}` },
    }, false);
    await setSession(res.access_token, res.expires_in, meRes.user);
    return meRes.user;
  },

  async me(): Promise<MeUser | null> {
    // try cache first, then re-fetch
    const cached = await loadUser();
    if (cached) {
      try {
        const res = await fetchJson<{ user: MeUser }>("/api/v1/auth/me");
        return res.user;
      } catch (err: any) {
        if (err.status === 401) {
          await clearSession();
          return null;
        }
        return cached;
      }
    }
    return null;
  },

  async logout() {
    await clearSession();
  },

  // === domain (called from pages as they migrate) ===
  residents() {
    return fetchJson<
      Array<{
        id: string;
        full_name: string;
        sex: string;
        birth_date: string;
        care_grade: string | null;
        room_number: string | null;
        admitted_on: string;
        status: string;
      }>
    >("/api/v1/residents");
  },
  resident(id: string) {
    return fetchJson(`/api/v1/residents/${id}`);
  },
  createResident(payload: {
    full_name: string;
    sex: "male" | "female" | "other";
    birth_date: string;
    care_grade?: string | null;
    room_number?: string | null;
    admitted_on: string;
  }) {
    return fetchJson("/api/v1/residents", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  updateResident(
    id: string,
    payload: Partial<{
      full_name: string;
      sex: string;
      care_grade: string | null;
      room_number: string | null;
      birth_date: string;
      admitted_on: string;
    }>,
  ) {
    return fetchJson(`/api/v1/residents/${id}`, {
      method: "PATCH",
      body: JSON.stringify(payload),
    });
  },
  dischargeResident(id: string, dischargedOn: string) {
    return fetchJson(`/api/v1/residents/${id}/discharge`, {
      method: "POST",
      body: JSON.stringify({ discharged_on: dischargedOn }),
    });
  },
  deceaseResident(id: string) {
    return fetchJson(`/api/v1/residents/${id}/decease`, { method: "POST" });
  },
  createMedication(payload: {
    resident_id: string;
    name: string;
    dosage: string;
    frequency: string;
    route?: string | null;
    start_date: string;
    end_date?: string | null;
    prescriber?: string | null;
    instructions?: string | null;
  }) {
    return fetchJson("/api/v1/medications", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  stopMedication(id: string) {
    return fetchJson(`/api/v1/medications/${id}/stop`, { method: "PATCH" });
  },
  flagCareLog(id: string) {
    return fetchJson(`/api/v1/care-logs/${id}/flag`, { method: "PATCH" });
  },
  vitalsFor(residentId: string) {
    return fetchJson(`/api/v1/residents/${residentId}/vitals`);
  },
  createVital(payload: { resident_id: string; kind: string; value: number; note?: string | null }) {
    return fetchJson("/api/v1/vitals", { method: "POST", body: JSON.stringify(payload) });
  },
  careLogsFor(residentId: string) {
    return fetchJson(`/api/v1/residents/${residentId}/care-logs`);
  },
  createCareLog(payload: {
    resident_id: string;
    category: string;
    body: string;
    flagged?: boolean;
  }) {
    return fetchJson("/api/v1/care-logs", { method: "POST", body: JSON.stringify(payload) });
  },
  medsFor(residentId: string) {
    return fetchJson(`/api/v1/residents/${residentId}/medications`);
  },
  staff() {
    return fetchJson("/api/v1/staff");
  },
  dashboard() {
    return fetchJson("/api/v1/dashboard/summary");
  },
};

export type { MeUser };
