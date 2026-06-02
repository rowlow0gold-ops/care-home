/**
 * Server-mode auth store. Coexists with the legacy `auth.ts` (rusqlite-backed)
 * while pages are migrated. Once all pages use server.ts, delete the legacy
 * store + Tauri commands.
 *
 * Roles from the server:
 *   caregiver < nurse < branch_manager < hq < super_admin
 *
 * Old roles (legacy):
 *   staff < manager < admin
 *
 * Mapping (for UI gating during transition):
 *   staff   -> caregiver
 *   manager -> branch_manager
 *   admin   -> hq
 */
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { server, type MeUser } from "@/lib/server";

const ROLE_RANK: Record<string, number> = {
  caregiver: 1,
  nurse: 2,
  branch_manager: 3,
  hq: 4,
  super_admin: 5,
};

export const useServerSessionStore = defineStore("server-session", () => {
  const me = ref<MeUser | null>(null);
  const hydrating = ref(false);

  const isLoggedIn = computed(() => me.value !== null);
  const role = computed(() => me.value?.role ?? null);
  const branchName = computed(() => me.value?.branch_name ?? null);
  const tenantName = computed(() => me.value?.tenant_name ?? null);

  async function hydrate() {
    hydrating.value = true;
    try {
      me.value = await server.me();
    } finally {
      hydrating.value = false;
    }
  }

  async function login(email: string, password: string) {
    me.value = await server.login(email, password);
    return me.value;
  }

  async function logout() {
    await server.logout();
    me.value = null;
  }

  function hasRole(min: string): boolean {
    if (!me.value) return false;
    return (ROLE_RANK[me.value.role] ?? 0) >= (ROLE_RANK[min] ?? Infinity);
  }

  // ── Desktop page access by job title (position) ───────────────────────────
  // The hub desktop splits access by position, not just the security role.
  // Page keys match the route names in router/index.ts.
  const ALL = "*" as const;
  const POSITION_PAGES: Record<string, readonly string[] | typeof ALL> = {
    branch_manager: ALL,                                                   // 시설장 — everything
    it: ALL,                                                               // 본사 IT 지원 — everything
    office_manager: ["residents", "schedule", "teams", "leave", "meals", "reports", "staff", "chat", "settings"], // 행정
    receptionist:   ["residents", "schedule", "teams", "leave", "meals", "reports", "staff", "chat", "settings"], // 접수
  };

  /** Pages this user may open: "*" = all, or an explicit list. */
  function allowedPages(): readonly string[] | typeof ALL {
    if (!me.value) return [];
    if (me.value.role === "hq" || me.value.role === "super_admin") return ALL;
    const pos = me.value.position ?? "";
    const mapped = POSITION_PAGES[pos];
    if (mapped) return mapped;
    // Unknown/legacy position but branch_manager+ → don't lock them out.
    return hasRole("branch_manager") ? ALL : ["residents"];
  }

  function canAccess(page: string): boolean {
    // Detail routes inherit their list page's permission.
    const key = page === "resident-detail" ? "residents" : page === "staff-detail" ? "staff" : page;
    const a = allowedPages();
    return a === ALL || a.includes(key);
  }

  // ── Action-level CRUD by job title ────────────────────────────────────────
  //   접수(receptionist): Create + Read only.
  //   행정(administrator) / 시설장 / 본사 / IT: full CRUD.
  const isReceptionist = computed(() => me.value?.position === "receptionist");
  const canCreate = computed(() => isLoggedIn.value);
  const canEdit = computed(() => isLoggedIn.value && !isReceptionist.value);
  const canDelete = computed(() => isLoggedIn.value && !isReceptionist.value);

  /** First page the user is allowed to see — used as the post-login landing. */
  function firstAllowed(): string {
    const a = allowedPages();
    if (a === ALL) return "residents";
    return a[0] ?? "residents";
  }

  // Korean label for the user's position (for the header chip).
  const positionLabel = computed(() => {
    const map: Record<string, string> = {
      branch_manager: "시설장",
      office_manager: "행정",
      receptionist: "접수",
      it: "IT 지원",
    };
    return map[me.value?.position ?? ""] ?? null;
  });

  return {
    me, hydrating, isLoggedIn, role, branchName, tenantName, positionLabel,
    canCreate, canEdit, canDelete,
    hydrate, login, logout, hasRole, canAccess, firstAllowed,
  };
});
