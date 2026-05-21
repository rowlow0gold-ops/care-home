<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore, type Role } from "@/stores/auth";

const auth = useAuthStore();
const router = useRouter();
const miniMode = ref(false);

// Role hierarchy: higher number = more access
const roleLevel: Record<Role, number> = {
  staff: 1,
  manager: 2,
  admin: 3,
  
};

interface NavItem { to: string; icon: string; label: string; minRole: number; }
interface NavGroup { heading: string; minRole: number; items: NavItem[]; }

const navGroups: NavGroup[] = [
  {
    heading: "", minRole: 1,
    items: [
      { to: "/residents", icon: "o_people", label: "Residents", minRole: 1 },
    ],
  },
  {
    heading: "Staff", minRole: 2,
    items: [
      { to: "/staff",     icon: "o_badge",          label: "Staff",     minRole: 2 },
      { to: "/schedule",  icon: "o_calendar_month", label: "Schedule",  minRole: 1 },
    ],
  },
  {
    heading: "Service", minRole: 2,
    items: [
      { to: "/media",         icon: "o_photo_library", label: "Media",         minRole: 2 },
      { to: "/notifications", icon: "o_mail",           label: "Notifications", minRole: 2 },
      { to: "/reports",       icon: "o_description",    label: "Reports",       minRole: 2 },
    ],
  },
  {
    heading: "", minRole: 1,
    items: [
      { to: "/meals", icon: "o_restaurant", label: "Meal Planning", minRole: 1 },
    ],
  },
  {
    heading: "Admin", minRole: 3,
    items: [
      { to: "/accounting", icon: "o_account_balance",   label: "Accounting", minRole: 3 },
      { to: "/insurance",  icon: "o_health_and_safety", label: "Insurance",  minRole: 3 },
      { to: "/settings",   icon: "o_settings",          label: "Settings",   minRole: 2 },
    ],
  },
  {
    heading: "", minRole: 1,
    items: [
      { to: "/help", icon: "o_help", label: "Help", minRole: 1 },
    ],
  },
];

const visibleGroups = computed(() => {
  const level = roleLevel[auth.user?.role as Role] ?? 0;
  return navGroups
    .map(g => ({ ...g, items: g.items.filter(i => level >= i.minRole) }))
    .filter(g => g.items.length > 0);
});

function handleLogout() {
  auth.logout();
  router.push("/login");
}
</script>

<template>
  <q-layout view="lHh Lpr lFf">
    <!-- Header -->
    <q-header elevated class="bg-primary">
      <q-toolbar>
        <q-btn
          flat
          round
          dense
          icon="o_menu"
          @click="miniMode = !miniMode"
        />
        <q-toolbar-title class="text-weight-medium">
          Sunshine Care Home
        </q-toolbar-title>
        <q-chip
          square
          color="secondary"
          text-color="white"
          size="sm"
          class="q-mr-sm"
        >
          {{ auth.user?.role?.toUpperCase() }}
        </q-chip>
        <q-btn flat round dense icon="o_logout" @click="handleLogout">
          <q-tooltip>Logout</q-tooltip>
        </q-btn>
      </q-toolbar>
    </q-header>

    <!-- Sidebar -->
    <q-drawer
      :mini="miniMode"
      :width="220"
      :mini-width="60"
      show-if-above
      bordered
    >
      <!-- User info -->
      <div v-if="!miniMode" class="q-pa-md sidebar-user">
        <div class="text-weight-semibold text-white">{{ auth.user?.full_name }}</div>
        <div class="text-caption sidebar-sub">{{ auth.user?.username }}</div>
      </div>
      <q-separator dark v-if="!miniMode" />

      <q-list padding>
        <template v-for="(group, gi) in visibleGroups" :key="gi">
          <!-- Group separator (not before the first group) -->
          <q-separator v-if="gi > 0" dark spaced="sm" class="q-mx-md" />
          <!-- Group heading -->
          <q-item-label v-if="group.heading && !miniMode"
                        header class="sidebar-group-label">
            {{ group.heading }}
          </q-item-label>
          <!-- Items -->
          <q-item
            v-for="item in group.items"
            :key="item.to"
            clickable v-ripple
            :to="item.to"
            active-class="sidebar-active"
            class="sidebar-item"
          >
            <q-item-section avatar>
              <q-icon :name="item.icon" />
            </q-item-section>
            <q-item-section v-if="!miniMode">{{ item.label }}</q-item-section>
            <q-tooltip v-if="miniMode" anchor="center right" self="center left">
              {{ item.label }}
            </q-tooltip>
          </q-item>
        </template>
      </q-list>
    </q-drawer>

    <!-- Main content -->
    <q-page-container>
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<style scoped>
.sidebar-user {
  padding: 16px;
  color: #fff;
}
.sidebar-sub {
  color: #94a3b8;
  margin-top: 2px;
}
.sidebar-item {
  color: #94a3b8;
  border-radius: 8px;
  margin: 2px 8px;
}
.sidebar-item:hover {
  color: #fff;
  background: rgba(255,255,255,0.08);
}
.sidebar-active {
  color: #14b8a6 !important;
  background: rgba(20, 184, 166, 0.12) !important;
}
.sidebar-group-label {
  color: #475569;
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  padding-top: 4px;
  padding-bottom: 2px;
}
</style>
