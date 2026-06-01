<script setup lang="ts">
import { ref, computed } from "vue";
import { useQuasar } from "quasar";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const session = useServerSessionStore();
const canUpload = computed(() => session.canEdit);

// Dummy for now — keeps an in-session list. Backend wiring comes later.
const uploads = ref<Array<{ name: string; size: string; at: string }>>([]);
const fileInput = ref<HTMLInputElement | null>(null);

function pick() { fileInput.value?.click(); }
function onFile(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (!f) return;
  uploads.value.unshift({
    name: f.name,
    size: (f.size / 1024).toFixed(0) + " KB",
    at: new Date().toLocaleString("ko-KR"),
  });
  $q.notify({ type: "positive", message: "보고서가 업로드되었습니다. (준비중 — 저장은 추후 연동)" });
  if (fileInput.value) fileInput.value.value = "";
}
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-md">
      <div class="col">
        <div class="text-h5 text-weight-bold">보고서</div>
        <div class="text-caption text-grey-6">엑셀 보고서 업로드 (준비중)</div>
      </div>
      <q-btn v-if="canUpload" color="primary" unelevated icon="o_upload" label="엑셀 업로드" @click="pick" />
      <input ref="fileInput" type="file" accept=".xlsx,.xls,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" class="hidden" @change="onFile" />
    </div>

    <q-list bordered separator v-if="uploads.length">
      <q-item v-for="(u, i) in uploads" :key="i">
        <q-item-section avatar><q-icon name="o_description" color="green-7" /></q-item-section>
        <q-item-section>
          <q-item-label class="text-weight-medium">{{ u.name }}</q-item-label>
          <q-item-label caption>{{ u.size }} · {{ u.at }}</q-item-label>
        </q-item-section>
      </q-item>
    </q-list>

    <div v-else class="column flex-center q-py-xl text-grey-5">
      <q-icon name="o_description" size="3rem" color="grey-4" />
      <div class="q-mt-sm">업로드된 보고서가 없습니다</div>
      <div v-if="canUpload" class="text-caption text-grey-4">상단에서 엑셀 파일을 업로드하세요</div>
    </div>
  </q-page>
</template>
