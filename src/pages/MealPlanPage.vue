<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useQuasar } from "quasar";
import { server } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const session = useServerSessionStore();
// 행정(administrator) uploads; 접수 prints/views only.
const canUpload = computed(() => session.canEdit);

type Doc = Awaited<ReturnType<typeof server.latestMealPlan>>;
const doc = ref<Doc>(null);
const loading = ref(false);
const uploading = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

const isImage = computed(() => !!doc.value && doc.value.mime_type.startsWith("image/"));
const isPdf = computed(() => !!doc.value && doc.value.mime_type === "application/pdf");

function fmt(iso: string) { return new Date(iso).toLocaleString("ko-KR"); }

async function load() {
  loading.value = true;
  try { doc.value = await server.latestMealPlan(); }
  catch (e: any) { $q.notify({ type: "negative", message: `식단표를 불러오지 못했습니다: ${e?.message ?? e}` }); }
  finally { loading.value = false; }
}
function pick() { fileInput.value?.click(); }
async function onFile(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (!f) return;
  uploading.value = true;
  try {
    await server.uploadMealPlan(f);
    $q.notify({ type: "positive", message: "식단표가 업로드되었습니다. 태블릿에 게시됩니다." });
    await load();
  } catch (e2: any) {
    $q.notify({ type: "negative", message: `업로드 실패: ${e2?.message ?? e2}` });
  } finally {
    uploading.value = false;
    if (fileInput.value) fileInput.value.value = "";
  }
}
function download() {
  if (!doc.value) return;
  const a = document.createElement("a");
  a.href = doc.value.data_url;
  a.download = doc.value.filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
}
function printDoc() {
  if (!doc.value) return;
  const f = document.createElement("iframe");
  f.style.cssText = "position:fixed;right:0;bottom:0;width:0;height:0;border:0";
  document.body.appendChild(f);
  const d = f.contentWindow?.document;
  if (!d) { window.print(); return; }
  const body = isImage.value
    ? `<img src="${doc.value.data_url}" style="max-width:100%" />`
    : `<iframe src="${doc.value.data_url}" style="width:100%;height:100vh;border:0"></iframe>`;
  d.write(`<html><head><title>식단표</title></head><body style="margin:0">${body}</body></html>`);
  d.close();
  setTimeout(() => { f.contentWindow?.focus(); f.contentWindow?.print(); setTimeout(() => f.remove(), 1000); }, 350);
}

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">식단표</div>
        <div class="text-caption text-grey-6">태블릿 근무자에게 게시됩니다 · 데스크는 인쇄해서 사용</div>
      </div>
      <q-btn v-if="doc" outline color="primary" icon="o_print" label="인쇄" @click="printDoc" />
      <q-btn v-if="doc" flat color="primary" icon="o_download" label="다운로드" @click="download" />
      <q-btn v-if="canUpload" color="primary" unelevated icon="o_upload" label="식단표 업로드" :loading="uploading" @click="pick" />
      <input ref="fileInput" type="file" accept="image/*,application/pdf,.xlsx,.xls" class="hidden" @change="onFile" />
    </div>

    <q-inner-loading :showing="loading" />

    <q-card v-if="doc" flat bordered>
      <q-card-section class="row items-center">
        <div>
          <div class="text-weight-medium">{{ doc.filename }}</div>
          <div class="text-caption text-grey-6">업로드 {{ fmt(doc.uploaded_at) }}</div>
        </div>
      </q-card-section>
      <q-separator />
      <q-card-section class="flex flex-center bg-grey-1">
        <q-img v-if="isImage" :src="doc.data_url" style="max-width: 800px; width: 100%" fit="contain" />
        <iframe v-else-if="isPdf" :src="doc.data_url" style="width: 100%; height: 70vh; border: 0"></iframe>
        <div v-else class="column flex-center q-py-xl text-grey-6">
          <q-icon name="o_description" size="3rem" color="grey-4" />
          <div class="q-mt-sm">{{ doc.filename }} — 다운로드하여 확인/인쇄하세요</div>
        </div>
      </q-card-section>
    </q-card>

    <div v-else-if="!loading" class="column flex-center q-py-xl text-grey-5">
      <q-icon name="o_restaurant_menu" size="3rem" color="grey-4" />
      <div class="q-mt-sm">게시된 식단표가 없습니다</div>
      <div v-if="canUpload" class="text-caption text-grey-4">상단에서 식단표 파일을 업로드하세요</div>
    </div>
  </q-page>
</template>
