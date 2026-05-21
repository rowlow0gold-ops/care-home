import { createRouter, createWebHashHistory } from "vue-router";
import { useAuthStore, type Role } from "@/stores/auth";

// Role hierarchy: higher number = more access
const roleLevel: Record<Role, number> = {
  staff: 1,
  manager: 2,
  admin: 3,
  
};

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/login",
      name: "login",
      component: () => import("@/pages/LoginPage.vue"),
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
        { path: "notifications", name: "notifications", component: () => import("@/pages/NotificationsPage.vue"),meta: { minRole: 2 } },
        { path: "staff",         name: "staff",         component: () => import("@/pages/StaffPage.vue"),        meta: { minRole: 2 } },
        { path: "reports",       name: "reports",       component: () => import("@/pages/ReportsPage.vue"),      meta: { minRole: 2 } },
        { path: "accounting",    name: "accounting",    component: () => import("@/pages/AccountingPage.vue"),   meta: { minRole: 3 } },
        { path: "insurance",     name: "insurance",     component: () => import("@/pages/InsurancePage.vue"),    meta: { minRole: 3 } },
        { path: "settings",      name: "settings",      component: () => import("@/pages/SettingsPage.vue"),     meta: { minRole: 2 } },
      ],
    },
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ],
});

router.beforeEach((to) => {
  const auth = useAuthStore();

  // Redirect unauthenticated users to login
  if (!to.meta.public && !auth.isLoggedIn) {
    return { name: "login" };
  }

  // Redirect already-logged-in users away from login page
  if (to.name === "login" && auth.isLoggedIn) {
    return { name: "residents" };
  }

  // Check role-based access for protected routes
  const required = to.meta.minRole as number | undefined;
  if (required && auth.isLoggedIn) {
    const userLevel = roleLevel[auth.user!.role as Role] ?? 0;
    if (userLevel < required) {
      // Redirect to highest accessible page
      return { name: "residents" };
    }
  }
});

export default router;
