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
  position: string | null;
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

async function fetchJson<T = any>(
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
    return fetchJson<Resident>(`/api/v1/residents/${id}`);
  },
  // HQ-style paged residents list (branch-scoped via branch_id).
  residentsPaged(params: {
    q?: string;
    branch_id?: string;
    care_grade?: string;
    status?: string;
    page?: number;
    page_size?: number;
    sort_by?: string;
    sort_desc?: boolean;
  }) {
    const qs = new URLSearchParams();
    if (params.q) qs.set("q", params.q);
    if (params.branch_id) qs.set("branch_id", params.branch_id);
    if (params.care_grade) qs.set("care_grade", params.care_grade);
    qs.set("status", params.status ?? "active");
    qs.set("page", String(params.page ?? 1));
    qs.set("page_size", String(params.page_size ?? 25));
    if (params.sort_by) qs.set("sort_by", params.sort_by);
    if (params.sort_desc) qs.set("sort_desc", "true");
    return fetchJson<{ items: Resident[]; total: number; page: number; page_size: number }>(
      `/api/v1/residents/paged?${qs.toString()}`,
    );
  },
  administerMedication(id: string) {
    return fetchJson(`/api/v1/medications/${id}/administer`, { method: "POST", body: "{}" });
  },
  medicationAdministrations(residentId: string) {
    return fetchJson<any[]>(`/api/v1/residents/${residentId}/medication-administrations`);
  },
  residentPhotos(residentId: string) {
    return fetchJson<{ items: Array<{ id: string; taken_at: string; caption: string | null; status: string; data_url: string }>; total: number }>(
      `/api/v1/residents/${residentId}/photos?page=1&page_size=60`,
    );
  },
  uploadPhoto(residentId: string, file: File, caption?: string) {
    const fd = new FormData();
    fd.append("resident_id", residentId);
    fd.append("tag", "regular");
    if (caption) fd.append("caption", caption);
    fd.append("file", file);
    return fetchJson(`/api/v1/photos`, { method: "POST", body: fd });
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
    return fetchJson<StaffMember[]>("/api/v1/staff");
  },
  // 식단표 (meal plan document) — desk uploads a file; tablet workers view latest.
  uploadMealPlan(file: File) {
    const fd = new FormData();
    fd.append("file", file);
    return fetchJson<{ id: string; filename: string }>("/api/v1/meal-plan", { method: "POST", body: fd });
  },
  latestMealPlan() {
    return fetchJson<{ id: string; filename: string; mime_type: string; uploaded_at: string; data_url: string } | null>(
      "/api/v1/meal-plan",
    );
  },

  // 휴가/연차 — branch_manager+ sees the whole branch's requests (incl. tablet-sent day-offs).
  leaveRequests(status?: string) {
    const qs = status ? `?status=${encodeURIComponent(status)}` : "";
    return fetchJson<Array<{
      id: string; user_id: string; user_name: string; user_role: string;
      leave_type: string; start_date: string; end_date: string; days: number;
      reason: string | null; status: "pending" | "approved" | "rejected" | "cancelled"; requested_at: string;
    }>>(`/api/v1/leave-requests${qs}`);
  },
  leaveRequestsPaged(params: { status?: string; page?: number; page_size?: number }) {
    const qs = new URLSearchParams();
    if (params.status) qs.set("status", params.status);
    qs.set("page", String(params.page ?? 1));
    qs.set("page_size", String(params.page_size ?? 20));
    return fetchJson<{
      items: Array<{
        id: string; user_id: string; user_name: string; user_role: string;
        leave_type: string; start_date: string; end_date: string; days: number;
        reason: string | null; status: "pending" | "approved" | "rejected" | "cancelled"; requested_at: string;
      }>;
      total: number; page: number; page_size: number;
    }>(`/api/v1/leave-requests/paged?${qs.toString()}`);
  },
  decideLeave(id: string, status: "approved" | "rejected") {
    return fetchJson(`/api/v1/leave-requests/${id}/decide`, {
      method: "PATCH",
      body: JSON.stringify({ status }),
    });
  },
  // HQ-style paged org list (이름/소속/직책/고용/경력 …). Branch-scoped server-side.
  orgPaged(params: { q?: string; employment_type?: string; page?: number; page_size?: number }) {
    const qs = new URLSearchParams();
    if (params.q) qs.set("q", params.q);
    if (params.employment_type) qs.set("employment_type", params.employment_type);
    qs.set("page", String(params.page ?? 1));
    qs.set("page_size", String(params.page_size ?? 25));
    return fetchJson<{ items: OrgPerson[]; total: number; page: number; page_size: number }>(
      `/api/v1/org/paged?${qs.toString()}`,
    );
  },
  staffOne(id: string) {
    return fetchJson<OrgPerson>(`/api/v1/staff/${id}`);
  },
  createStaff(payload: {
    email: string;
    full_name: string;
    role: string;
    position: string;
    password: string;
    phone?: string | null;
    employment_type?: string | null;
    branch_id?: string | null;
  }) {
    return fetchJson<StaffMember>("/api/v1/staff", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  updateStaff(
    id: string,
    payload: {
      expected_updated_at: string;
      full_name?: string;
      email?: string;
      phone?: string | null;
      position?: string;
      employment_type?: string;
    },
  ) {
    return fetchJson<StaffMember>(`/api/v1/staff/${id}`, {
      method: "PATCH",
      body: JSON.stringify(payload),
    });
  },
  deactivateStaff(id: string) {
    return fetchJson<StaffMember>(`/api/v1/staff/${id}/deactivate`, {
      method: "PATCH",
    });
  },

  // === roster (근무일정) — free-form per-branch shift roster ===
  roster(start: string, end: string, teamId?: string) {
    const t = teamId ? `&team_id=${encodeURIComponent(teamId)}` : "";
    return fetchJson<RosterEntry[]>(
      `/api/v1/roster?start=${encodeURIComponent(start)}&end=${encodeURIComponent(end)}${t}`,
    );
  },
  teams() {
    return fetchJson<Team[]>("/api/v1/teams");
  },
  holidays(year: number) {
    return fetchJson<Array<{ locdate: string; name: string; is_holiday: boolean }>>(
      `/api/v1/holidays?year=${year}`,
    );
  },
  assignTeam(userId: string, teamId: string | null) {
    return fetchJson(`/api/v1/staff/${userId}/team`, {
      method: "PATCH",
      body: JSON.stringify({ team_id: teamId }),
    });
  },
  createRoster(payload: UpsertRoster) {
    return fetchJson<RosterEntry>("/api/v1/roster", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  updateRoster(id: string, payload: UpsertRoster) {
    return fetchJson<RosterEntry>(`/api/v1/roster/${id}`, {
      method: "PATCH",
      body: JSON.stringify(payload),
    });
  },
  deleteRoster(id: string) {
    return fetchJson(`/api/v1/roster/${id}`, { method: "DELETE" });
  },

  // === notifications feed — flagged care logs (in-app alerts) ===
  flaggedCareLogs(pageSize = 100) {
    return fetchJson<{ items: CareLogRow[]; total: number }>(
      `/api/v1/care-logs/paged?flagged_only=true&page=1&page_size=${pageSize}`,
    );
  },

  // === reports — download server-generated XLSX exports ===
  /** Fetch any *.xlsx export endpoint as raw bytes for native save. */
  async exportXlsx(path: string): Promise<Uint8Array> {
    const t = await getToken();
    if (!t) {
      const err = new Error("not authenticated") as ServerError;
      err.status = 401;
      throw err;
    }
    const res = await fetch(`${API_BASE}${path}`, {
      headers: { Authorization: `Bearer ${t}` },
    });
    if (!res.ok) {
      const err = new Error(`API ${res.status}`) as ServerError;
      err.status = res.status;
      throw err;
    }
    return new Uint8Array(await res.arrayBuffer());
  },
  dashboardSummary() {
    return fetchJson<Record<string, any>>("/api/v1/dashboard/summary");
  },

  // === billing (정산) — monthly per-branch close ===
  billingRuns() {
    return fetchJson<BillingRun[]>("/api/v1/billing/runs");
  },
  runBilling(payload: { year_month?: string; branch_id?: string }) {
    return fetchJson<{
      billing_run_id: string;
      year_month: string;
      branch_id: string;
      status: string;
    }>("/api/v1/billing/run", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  /** Download a finished run's XLSX as raw bytes (for native save). */
  async billingXlsx(id: string): Promise<Uint8Array> {
    const t = await getToken();
    if (!t) {
      const err = new Error("not authenticated") as ServerError;
      err.status = 401;
      throw err;
    }
    const res = await fetch(`${API_BASE}/api/v1/billing/runs/${id}/xlsx`, {
      headers: { Authorization: `Bearer ${t}` },
    });
    if (!res.ok) {
      const err = new Error(`API ${res.status}`) as ServerError;
      err.status = res.status;
      throw err;
    }
    return new Uint8Array(await res.arrayBuffer());
  },
  dashboard() {
    return fetchJson("/api/v1/dashboard/summary");
  },

  // === leave (휴가 신청) ===
  leaveBalance() {
    return fetchJson<{
      year: number;
      annual_allocated: number;
      annual_used: number;
      annual_remaining: number;
      sick_used: number;
    }>("/api/v1/leave-requests/balance");
  },
  myLeaveRequests() {
    return fetchJson<Array<{
      id: string;
      leave_type: string;
      start_date: string;
      end_date: string;
      days: number;
      reason: string | null;
      status: "pending" | "approved" | "rejected" | "cancelled";
      requested_at: string;
    }>>("/api/v1/leave-requests");
  },
  createLeaveRequest(payload: {
    leave_type: string;
    start_date: string;
    end_date: string;
    days: number;
    reason?: string | null;
  }) {
    return fetchJson("/api/v1/leave-requests", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },

  // === photo approval (receptionist / admin / center manager) ===
  photosPending(status: "pending" | "approved" | "rejected" = "pending") {
    return fetchJson<
      Array<{
        id: string;
        resident_id: string;
        resident_name: string;
        branch_id: string;
        branch_name: string;
        taken_by_name: string;
        taken_at: string;
        caption: string | null;
        status: string;
        data_url: string;
      }>
    >(`/api/v1/photos/pending?status=${status}`);
  },
  decidePhoto(id: string, status: "approved" | "rejected", note?: string) {
    return fetchJson(`/api/v1/photos/${id}/decide`, {
      method: "PATCH",
      body: JSON.stringify({ status, note: note ?? null }),
    });
  },

  // === meals (식단) — per-branch weekly menu grid ===
  mealsWeek(weekStart: string) {
    return fetchJson<MealPlan[]>(
      `/api/v1/meals?week_start=${encodeURIComponent(weekStart)}`,
    );
  },
  mealsRange(start: string, end: string) {
    return fetchJson<MealPlan[]>(
      `/api/v1/meals/range?start=${encodeURIComponent(start)}&end=${encodeURIComponent(end)}`,
    );
  },
  upsertMeal(payload: UpsertMealPlan) {
    return fetchJson<MealPlan>("/api/v1/meals", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
  bulkUpsertMeals(plans: UpsertMealPlan[]) {
    return fetchJson<{ count: number }>("/api/v1/meals/bulk", {
      method: "POST",
      body: JSON.stringify(plans),
    });
  },
};

export interface CareLogRow {
  id: string;
  resident_id: string;
  resident_name: string;
  resident_room: string | null;
  recorded_at: string;
  category: string;
  body: string;
  flagged: boolean;
}

export interface Team {
  id: string;
  branch_id: string;
  branch_name: string | null;
  name: string;
  color_hue: number;
  sort_order: number;
  shift_start_hm: string;
  shift_end_hm: string;
  member_count: number;
  created_at: string;
}

export interface RosterEntry {
  id: string;
  user_id: string;
  staff_name: string;
  shift_date: string;
  shift_start: string;
  shift_end: string;
  shift_hours: number;
  notes: string | null;
}

export interface UpsertRoster {
  user_id: string;
  shift_date: string;
  shift_start: string;
  shift_end: string;
  shift_hours: number;
  notes?: string | null;
}

export interface BillingRun {
  id: string;
  branch_id: string;
  year_month: string;
  triggered_at: string;
  completed_at: string | null;
  status: string;
  resident_count: number | null;
  total_amount: number | null;
  failure_reason: string | null;
  has_xlsx: boolean;
}

export interface Resident {
  id: string;
  tenant_id: string;
  branch_id: string;
  full_name: string;
  sex: "male" | "female" | "other";
  birth_date: string;
  care_grade: string | null;
  room_number: string | null;
  admitted_on: string;
  status: "active" | "discharged" | "deceased";
}

export interface OrgPerson {
  id: string;
  branch_id: string | null;
  branch_name: string | null;
  full_name: string;
  email: string;
  phone: string | null;
  role: string;
  position: string;
  position_ko: string;
  employment_type: string;
  employment_type_ko: string;
  hired_on: string | null;
  contract_end_on: string | null;
  monthly_salary_krw: number | null;
  hourly_rate_est_krw: number | null;
  updated_at: string;
  is_inactive: boolean;
}

export interface StaffMember {
  id: string;
  branch_id: string | null;
  email: string;
  full_name: string;
  role: string;
  phone: string | null;
  deactivated_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface MealPlan {
  id: string;
  week_start: string;
  day_of_week: number;
  meal_type: string;
  menu: string;
  calories: number | null;
  notes: string | null;
}

export interface UpsertMealPlan {
  week_start: string;
  day_of_week: number;
  meal_type: string;
  menu: string;
  calories?: number | null;
  notes?: string | null;
}

export type { MeUser };
