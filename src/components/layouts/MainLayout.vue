<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useServerSessionStore } from "@/stores/server-session";

const session = useServerSessionStore();
const router = useRouter();
const miniMode = ref(false);

// Korean display label per server role (fallback when no position).
const roleLabel: Record<string, string> = {
  caregiver: "요양보호사",
  nurse: "간호사",
  branch_manager: "센터장",
  hq: "본사",
  super_admin: "최고관리자",
};

// `key` matches the route name; nav is gated by position via session.canAccess.
interface NavItem { to: string; key: string; icon: string; label: string; }
interface NavGroup { heading: string; items: NavItem[]; }

const navGroups: NavGroup[] = [
  {
    heading: "케어",
    items: [
      { to: "/residents", key: "residents", icon: "o_people", label: "어르신" },
    ],
  },
  {
    heading: "직원",
    items: [
      { to: "/staff",    key: "staff",    icon: "o_badge",          label: "직원" },
      { to: "/teams",    key: "teams",    icon: "o_groups",         label: "팀" },
    ],
  },
  {
    heading: "서비스",
    items: [
      { to: "/schedule", key: "schedule", icon: "o_calendar_month", label: "스케쥴러" },
      { to: "/chat",      key: "chat",    icon: "o_forum",        label: "채팅" },
      { to: "/leave",     key: "leave",   icon: "o_event_busy",   label: "휴가 승인" },
    ],
  },
  {
    heading: "업로드",
    items: [
      { to: "/meal-plan", key: "meals",   icon: "o_restaurant",   label: "식단표" },
      { to: "/reports",   key: "reports", icon: "o_description",  label: "보고서" },
    ],
  },
  {
    heading: "관리",
    items: [
      { to: "/settings",   key: "settings",   icon: "o_settings",        label: "설정" },
    ],
  },
];

const visibleGroups = computed(() =>
  navGroups
    .map((g) => ({ ...g, items: g.items.filter((i) => session.canAccess(i.key)) }))
    .filter((g) => g.items.length > 0),
);

// Header shows where the operator is signed in: branch (hub) name, else tenant.
const stationName = computed(
  () => session.branchName ?? session.tenantName ?? "케어닥",
);
// Prefer the job-title label (시설장/행정/접수/IT), else the role.
const myRoleLabel = computed(
  () => session.positionLabel ?? roleLabel[session.me?.role ?? ""] ?? session.me?.role ?? "",
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
      <div class="column full-height">
        <!-- Nav (scrolls) -->
        <q-scroll-area class="col">
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
        </q-scroll-area>

        <!-- User card + logout pinned at the bottom-left (HQ style) -->
        <q-separator dark />
        <div v-if="!miniMode" class="sidebar-userbox q-px-md q-py-sm">
          <!-- role · 소속 -->
          <div class="row items-center q-gutter-xs q-mb-sm">
            <q-chip square dense color="secondary" text-color="white" size="sm">{{ myRoleLabel }}</q-chip>
            <span class="text-caption text-white">{{ session.branchName ?? session.tenantName }}</span>
          </div>
          <!-- name -->
          <div class="row items-center no-wrap q-mb-xs">
            <q-icon name="o_person" size="18px" class="sidebar-sub q-mr-sm" />
            <span class="text-weight-semibold text-white">{{ session.me?.name ?? "—" }}</span>
          </div>
          <!-- email (full, wraps) -->
          <div class="row items-start no-wrap">
            <q-icon name="o_mail" size="18px" class="sidebar-sub q-mr-sm q-mt-xs" />
            <span class="text-caption sidebar-sub sidebar-email">{{ session.me?.email }}</span>
          </div>
        </div>
        <q-item clickable v-ripple class="sidebar-item q-my-sm" @click="handleLogout">
          <q-item-section avatar><q-icon name="o_logout" /></q-item-section>
          <q-item-section v-if="!miniMode">로그아웃</q-item-section>
          <q-tooltip v-if="miniMode" anchor="center right" self="center left">로그아웃</q-tooltip>
        </q-item>
      </div>
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
.sidebar-email {
  word-break: break-all;
  line-height: 1.3;
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
