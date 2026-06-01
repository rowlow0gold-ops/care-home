<script setup lang="ts">
import { computed } from "vue";
import { useServerSessionStore } from "@/stores/server-session";

const session = useServerSessionStore();

const roleLabel: Record<string, string> = {
  caregiver: "요양보호사",
  nurse: "간호사",
  branch_manager: "시설장",
  hq: "본사",
  super_admin: "최고관리자",
};

const me = computed(() => session.me);
const stationName = computed(() => session.branchName ?? session.tenantName ?? "—");
const jobLabel = computed(() => session.positionLabel ?? roleLabel[me.value?.role ?? ""] ?? me.value?.role);

async function logout() {
  await session.logout();
  window.location.hash = "#/login";
}
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="text-h5 text-weight-bold q-mb-md">설정</div>

    <div class="row q-col-gutter-md">
      <div class="col-12 col-md-6">
        <q-card flat bordered>
          <q-card-section>
            <div class="text-subtitle1 text-weight-bold q-mb-sm">내 계정</div>
            <q-list dense>
              <q-item>
                <q-item-section>이름</q-item-section>
                <q-item-section side class="text-weight-medium">{{ me?.name ?? "—" }}</q-item-section>
              </q-item>
              <q-item>
                <q-item-section>이메일</q-item-section>
                <q-item-section side>{{ me?.email ?? "—" }}</q-item-section>
              </q-item>
              <q-item>
                <q-item-section>역할</q-item-section>
                <q-item-section side>
                  <q-badge color="blue">{{ jobLabel }}</q-badge>
                </q-item-section>
              </q-item>
              <q-item>
                <q-item-section>소속</q-item-section>
                <q-item-section side>{{ stationName }}</q-item-section>
              </q-item>
            </q-list>
            <q-btn
              outline
              color="negative"
              icon="o_logout"
              label="로그아웃"
              class="q-mt-md"
              @click="logout"
            />
          </q-card-section>
        </q-card>
      </div>
    </div>
  </q-page>
</template>
