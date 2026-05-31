<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useServerSessionStore } from "@/stores/server-session";

const session = useServerSessionStore();
const router = useRouter();
const miniMode = ref(false);

// Server permission ladder (matches router/index.ts). Higher = more access.
//   caregiver < nurse < branch_manager < hq < super_admin
const roleLevel: Record<string, number> = {
  caregiver: 1,
  nurse: 2,
  branch_manager: 3,
  hq: 4,
  super_admin: 5,
};

// Korean display label per server role.
const roleLabel: Record<string, string> = {
  caregiver: "요양보호사",
  nurse: "간호사",
  branch_manager: "센터장",
  hq: "본사",
  super_admin: "최고관리자",
};

interface NavItem { to: string; icon: string; label: string; minRole: number; }
interface NavGroup { heading: string; items: NavItem[]; }

// minRole values mirror router/index.ts so shell + routes never disagree.
const navGroups: NavGroup[] = [
  {
    heading: "",
    items: [
      { to: "/residents", icon: "o_people", label: "어르신", minRole: 1 },
    ],
  },
  {
    heading: "직원",
    items: [
      { to: "/staff",    icon: "o_badge",          label: "직원",      minRole: 3 },
      { to: "/schedule", icon: "o_calendar_month", label: "근무일정",  minRole: 1 },
      { to: "/leave",    icon: "o_event_busy",     label: "휴가 신청", minRole: 1 },
    ],
  },
  {
    heading: "서비스",
    items: [
      { to: "/media",         icon: "o_photo_camera", label: "사진 승인", minRole: 1 },
      { to: "/notifications", icon: "o_mail",         label: "알림",      minRole: 3 },
      { to: "/reports",       icon: "o_description",  label: "보고서",    minRole: 3 },
      { to: "/meals",         icon: "o_restaurant",   label: "식단",      minRole: 1 },
    ],
  },
  {
    heading: "관리",
    items: [
      { to: "/accounting", icon: "o_account_balance",   label: "정산",   minRole: 4 },
      { to: "/settings",   icon: "o_settings",          label: "설정",   minRole: 3 },
    ],
  },
  {
    heading: "",
    items: [
      { to: "/help", icon: "o_help", label: "도움말", minRole: 1 },
    ],
  },
];

const myLevel = computed(() => roleLevel[session.me?.role ?? ""] ?? 0);

const visibleGroups = computed(() =>
  navGroups
    .map((g) => ({ ...g, items: g.items.filter((i) => myLevel.value >= i.minRole) }))
    .filter((g) => g.items.length > 0),
);

// Header shows where the operator is signed in: branch (hub) name, else tenant.
const stationName = computed(
  () => session.branchName ?? session.tenantName ?? "케어닥",
);
const myRoleLabel = computed(
  () => roleLabel[session.me?.role ?? ""] ?? session.me?.role ?? "",
);

async function handleLogout() {
  await session.logout();
  router.replace({ name: "login" });
}
</script>

<template>
  <q-layout view="lHh Lpr lFf">
    <!-- Header -->
    <q-header elevated class="bg-primary">
      <q-toolbar>
        <q-btn flat round dense icon="o_menu" @click="miniMode = !miniMode" />
        <q-toolbar-title class="text-weight-medium">
          케어닥 — {{ stationName }}
        </q-toolbar-title>
        <q-chip
          square
          color="secondary"
          text-color="white"
          size="sm"
          class="q-mr-sm"
        >
          {{ myRoleLabel }}
        </q-chip>
        <q-btn flat round dense icon="o_logout" @click="handleLogout">
          <q-tooltip>로그아웃</q-tooltip>
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
        <div class="text-weight-semibold text-white">{{ session.me?.name }}</div>
        <div class="text-caption sidebar-sub">{{ session.me?.email }}</div>
      </div>
      <q-separator dark v-if="!miniMode" />

      <q-list padding>
        <template v-for="(group, gi) in visibleGroups" :key="gi">
          <q-separator v-if="gi > 0" dark spaced="sm" class="q-mx-md" />
          <q-item-label
            v-if="group.heading && !miniMode"
            header
            class="sidebar-group-label"
          >
            {{ group.heading }}
          </q-item-label>
          <q-item
            v-for="item in group.items"
            :key="item.to"
            clickable
            v-ripple
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
  background: rgba(255, 255, 255, 0.08);
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
