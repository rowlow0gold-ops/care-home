<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { server } from "@/lib/server";

const $q = useQuasar();

interface ReportDef {
  key: string;
  label: string;
  description: string;
  icon: string;
  color: string;
  path: string;     // server export.xlsx endpoint
  filename: string;
}

const reports: ReportDef[] = [
  {
    key: "residents",
    label: "어르신 명부",
    description: "전체 어르신 명단 · 등급 · 입소일 등 기본 정보",
    icon: "o_people",
    color: "blue",
    path: "/api/v1/residents/export.xlsx",
    filename: "어르신_명부.xlsx",
  },
  {
    key: "care_logs",
    label: "케어 기록",
    description: "케어 기록 전체 내역 (카테고리 · 작성자 · 시각)",
    icon: "o_assignment",
    color: "teal",
    path: "/api/v1/care-logs/export.xlsx",
    filename: "케어기록.xlsx",
  },
  {
    key: "medications",
    label: "투약 기록",
    description: "투약 처방 및 투약 내역",
    icon: "o_medication",
    color: "pink",
    path: "/api/v1/medications/export.xlsx",
    filename: "투약기록.xlsx",
  },
  {
    key: "shifts",
    label: "근무 현황",
    description: "지점별 근무 배정 현황",
    icon: "o_calendar_month",
    color: "indigo",
    path: "/api/v1/shifts/export.xlsx",
    filename: "근무현황.xlsx",
  },
  {
    key: "staff",
    label: "직원 명부",
    description: "재직 직원 명단 · 직책 · 고용형태",
    icon: "o_badge",
    color: "deep-purple",
    path: "/api/v1/staff/export.xlsx",
    filename: "직원_명부.xlsx",
  },
];

const downloading = ref<string | null>(null);
const summary = ref<Record<string, any> | null>(null);

async function download(r: ReportDef) {
  downloading.value = r.key;
  try {
    const bytes = await server.exportXlsx(r.path);
    const saved = await invoke<string | null>("save_excel", {
      filename: r.filename,
      data: Array.from(bytes),
    });
    if (saved) $q.notify({ type: "positive", message: `${r.label} 다운로드 완료.` });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `다운로드 실패: ${e?.message ?? e}` });
  } finally {
    downloading.value = null;
  }
}

async function loadSummary() {
  try {
    summary.value = await server.dashboardSummary();
  } catch (_) {
    summary.value = null;
  }
}

onMounted(loadSummary);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="text-h5 text-weight-bold q-mb-xs">보고서</div>
    <div class="text-caption text-grey-6 q-mb-md">엑셀(.xlsx) 내보내기</div>

    <div class="row q-col-gutter-md">
      <div v-for="r in reports" :key="r.key" class="col-12 col-sm-6 col-md-4">
        <q-card flat bordered class="full-height">
          <q-card-section class="row items-center no-wrap">
            <q-avatar :color="r.color" text-color="white" :icon="r.icon" />
            <div class="q-ml-md">
              <div class="text-subtitle1 text-weight-bold">{{ r.label }}</div>
            </div>
          </q-card-section>
          <q-card-section class="q-pt-none text-body2 text-grey-7">
            {{ r.description }}
          </q-card-section>
          <q-card-actions align="right">
            <q-btn
              :color="r.color"
              icon="o_download"
              label="다운로드"
              unelevated
              dense
              :loading="downloading === r.key"
              @click="download(r)"
            />
          </q-card-actions>
        </q-card>
      </div>
    </div>
  </q-page>
</template>
