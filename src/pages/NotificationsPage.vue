<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useQuasar } from "quasar";
import { server, type CareLogRow } from "@/lib/server";

const $q = useQuasar();

const items = ref<CareLogRow[]>([]);
const loading = ref(false);

const CATEGORY_KO: Record<string, string> = {
  meal: "식사",
  medication: "투약",
  hygiene: "위생",
  mobility: "이동",
  mood: "정서",
  incident: "특이사항",
  vitals: "활력징후",
  other: "기타",
};

function fmtTime(iso: string): string {
  return new Date(iso).toLocaleString("ko-KR", {
    month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit",
  });
}

async function load() {
  loading.value = true;
  try {
    const res = await server.flaggedCareLogs(100);
    items.value = res.items;
  } catch (e: any) {
    $q.notify({ type: "negative", message: `알림을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-md">
      <div class="col">
        <div class="text-h5 text-weight-bold">알림</div>
        <div class="text-caption text-grey-6">관리자에게 전달된 플래그 케어 기록</div>
      </div>
      <q-btn flat round dense icon="o_refresh" :loading="loading" @click="load">
        <q-tooltip>새로고침</q-tooltip>
      </q-btn>
    </div>

    <template v-if="loading">
      <q-skeleton type="rect" height="64px" class="q-mb-sm" v-for="n in 5" :key="n" />
    </template>

    <template v-else>
      <q-list bordered separator v-if="items.length">
        <q-item v-for="it in items" :key="it.id">
          <q-item-section avatar>
            <q-icon name="o_flag" color="negative" />
          </q-item-section>
          <q-item-section>
            <q-item-label class="text-weight-medium">
              {{ it.resident_name }}
              <span v-if="it.resident_room" class="text-grey-6">· {{ it.resident_room }}호</span>
              <q-badge class="q-ml-sm" color="orange" :label="CATEGORY_KO[it.category] ?? it.category" />
            </q-item-label>
            <q-item-label caption lines="3">{{ it.body }}</q-item-label>
          </q-item-section>
          <q-item-section side top>
            <span class="text-caption text-grey-6">{{ fmtTime(it.recorded_at) }}</span>
          </q-item-section>
        </q-item>
      </q-list>

      <div v-else class="column flex-center q-py-xl">
        <q-icon name="o_notifications_off" size="3rem" color="grey-4" />
        <div class="text-grey-5 q-mt-sm">새로운 알림이 없습니다</div>
      </div>
    </template>
  </q-page>
</template>
