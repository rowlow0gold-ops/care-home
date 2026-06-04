import { createRouter, createWebHashHistory } from "vue-router";
import { useServerSessionStore } from "@/stores/server-session";

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
        { path: "residents",     name: "residents",       component: () => import("@/pages/ResidentsPage.vue"),      meta: { minRole: 1 } },
        { path: "residents/:id", name: "resident-detail", component: () => import("@/pages/ResidentDetailPage.vue"), meta: { minRole: 1 } },
        { path: "schedule",      name: "schedule",      component: () => import("@/pages/SchedulePage.vue"),     meta: { minRole: 1 } },
        { path: "leave",         name: "leave",         component: () => import("@/pages/LeaveApprovalPage.vue"), meta: { minRole: 3 } },
        { path: "chat",          name: "chat",          component: () => import("@/pages/ChatPage.vue"),          meta: { minRole: 1 } },
        { path: "meal-plan",     name: "meals",         component: () => import("@/pages/MealPlanPage.vue"),      meta: { minRole: 3 } },
        { path: "reports",       name: "reports",       component: () => import("@/pages/ReportsPage.vue"),       meta: { minRole: 3 } },
        { path: "staff",         name: "staff",         component: () => import("@/pages/StaffPage.vue"),        meta: { minRole: 3 } },
        { path: "staff/:id",     name: "staff-detail",  component: () => import("@/pages/StaffDetailPage.vue"),  meta: { minRole: 3 } },
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

  // Redirect already-logged-in users away from login page → their first page
  if (to.name === "login" && session.isLoggedIn) {
    return { name: session.firstAllowed() };
  }

  // Position-based access: bounce to the user's first allowed page if the
  // target route isn't permitted for their job title.
  if (session.isLoggedIn && to.name && typeof to.name === "string") {
    if (!to.meta.public && !session.canAccess(to.name)) {
      return { name: session.firstAllowed() };
    }
  }
});

export default router;
