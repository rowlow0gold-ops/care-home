import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type Role = "staff" | "manager" | "admin";

export interface User {
  id: number;
  username: string;
  full_name: string;
  role: Role;
}

export const useAuthStore = defineStore("auth", () => {
  const user = ref<User | null>(null);

  const isLoggedIn = computed(() => user.value !== null);
  const role = computed(() => user.value?.role ?? null);

  const isAdmin = computed(() => user.value?.role === "admin");

  async function login(username: string, password: string) {
    const result = await invoke<User>("login", { username, password });
    user.value = result;
    return result;
  }

  function logout() {
    user.value = null;
  }

  function can(roles: Role[]) {
    return user.value ? roles.includes(user.value.role) : false;
  }

  return { user, isLoggedIn, role, isAdmin, login, logout, can };
});
