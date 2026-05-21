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

  return { me, hydrating, isLoggedIn, role, branchName, tenantName, hydrate, login, logout, hasRole };
});
