import { createRouter, createWebHashHistory } from "vue-router";
import { useServerSessionStore } from "@/stores/server-session";

// Server role hierarchy: higher number = more access
// caregiver < nurse < branch_manager < hq < super_admin
const roleLevel: Record<string, number> = {
  caregiver: 1,
  nurse: 2,
  branch_manager: 3,
  hq: 4,
  super_admin: 5,
};

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/login",
      name: "login",
      // Migrated: now uses server-side auth instead of rusqlite users
      component: () => import("@/pages/ServerLoginPage.vue"),
      meta: { public: true },
    },
    {
      path: "/",
      component: () => import("@/components/layouts/MainLayout.vue"),
      meta: { requiresAuth: true },
      children: [
        { path: "", redirect: "/residents" },
        { path: "residents",     name: "residents",     component: () => import("@/pages/ResidentsPage.vue"),    meta: { minRole: 1 } },
        { path: "care-log",      redirect: { path: "/residents", query: { section: "carelog" } } },
        { path: "medications",   redirect: { path: "/residents", query: { section: "medications" } } },
        { path: "health-charts", redirect: { path: "/residents", query: { section: "healthcharts" } } },
        { path: "meals",         name: "meals",         component: () => import("@/pages/MealsPage.vue"),        meta: { minRole: 1 } },
        { path: "schedule",      name: "schedule",      component: () => import("@/pages/SchedulePage.vue"),     meta: { minRole: 1 } },
        { path: "media",         name: "media",         component: () => import("@/pages/MediaPage.vue"),        meta: { minRole: 1 } },
        { path: "help",          name: "help",          component: () => import("@/pages/HelpPage.vue"),         meta: { minRole: 1 } },
        { path: "notifications", name: "notifications", component: () => import("@/pages/NotificationsPage.vue"),meta: { minRole: 3 } },
        { path: "staff",         name: "staff",         component: () => import("@/pages/StaffPage.vue"),        meta: { minRole: 3 } },
        { path: "reports",       name: "reports",       component: () => import("@/pages/ReportsPage.vue"),      meta: { minRole: 3 } },
        { path: "accounting",    name: "accounting",    component: () => import("@/pages/AccountingPage.vue"),   meta: { minRole: 4 } },
        { path: "insurance",     name: "insurance",     component: () => import("@/pages/InsurancePage.vue"),    meta: { minRole: 4 } },
        { path: "settings",      name: "settings",      component: () => import("@/pages/SettingsPage.vue"),     meta: { minRole: 3 } },
      ],
    },
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ],
});

router.beforeEach(async (to) => {
  const session = useServerSessionStore();

  // Hydrate from Tauri store on first nav after app start
  if (!session.me && !session.hydrating) {
    await session.hydrate();
  }

  // Redirect unauthenticated users to login
  if (!to.meta.public && !session.isLoggedIn) {
    return { name: "login" };
  }

  // Redirect already-logged-in users away from login page
  if (to.name === "login" && session.isLoggedIn) {
    return { name: "residents" };
  }

  // Check role-based access for protected routes
  const required = to.meta.minRole as number | undefined;
  if (required && session.isLoggedIn && session.me) {
    const userLevel = roleLevel[session.me.role] ?? 0;
    if (userLevel < required) {
      return { name: "residents" };
    }
  }
});

export default router;
