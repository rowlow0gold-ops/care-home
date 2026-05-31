<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { server, type BillingRun } from "@/lib/server";

const $q = useQuasar();

const runs = ref<BillingRun[]>([]);
const loading = ref(false);
const running = ref(false);
const downloading = ref<string | null>(null);

// Month to close. Defaults to last month (typical month-end close).
function lastMonth(): string {
  const d = new Date();
  d.setDate(1);
  d.setMonth(d.getMonth() - 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
}
const targetMonth = ref(lastMonth());

function won(v: number | null): string {
  if (v == null) return "—";
  return "₩" + v.toLocaleString("ko-KR");
}

const STATUS_META: Record<string, { label: string; color: string }> = {
  queued: { label: "대기", color: "grey" },
  running: { label: "진행중", color: "blue" },
  completed: { label: "완료", color: "positive" },
  failed: { label: "실패", color: "negative" },
};
function statusMeta(s: string) {
  return STATUS_META[s] ?? { label: s, color: "grey" };
}

function fmtTime(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("ko-KR", {
    year: "numeric", month: "2-digit", day: "2-digit",
    hour: "2-digit", minute: "2-digit",
  });
}

const columns = [
  { name: "year_month", label: "정산월", field: "year_month", align: "left" as const },
  { name: "status", label: "상태", field: "status", align: "center" as const },
  { name: "resident_count", label: "어르신 수", field: "resident_count", align: "right" as const },
  { name: "total_amount", label: "총액", field: "total_amount", align: "right" as const },
  { name: "triggered_at", label: "실행시각", field: "triggered_at", align: "left" as const },
  { name: "completed_at", label: "완료시각", field: "completed_at", align: "left" as const },
  { name: "actions", label: "명세서", field: "actions", align: "center" as const },
];

const totalClosed = computed(() =>
  runs.value
    .filter((r) => r.status === "completed")
    .reduce((sum, r) => sum + (r.total_amount ?? 0), 0),
);

async function load() {
  loading.value = true;
  try {
    runs.value = await server.billingRuns();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `정산 내역을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}

async function runClose() {
  if (!/^\d{4}-\d{2}$/.test(targetMonth.value)) {
    $q.notify({ type: "negative", message: "정산월 형식은 YYYY-MM 입니다." });
    return;
  }
  running.value = true;
  try {
    await server.runBilling({ year_month: targetMonth.value });
    $q.notify({ type: "positive", message: `${targetMonth.value} 정산 마감을 시작했습니다.` });
    // worker processes asynchronously — reload to reflect queued/running state
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `정산 실행 실패: ${e?.message ?? e}` });
  } finally {
    running.value = false;
  }
}

async function download(r: BillingRun) {
  downloading.value = r.id;
  try {
    const bytes = await server.billingXlsx(r.id);
    const saved = await invoke<string | null>("save_excel", {
      filename: `정산_${r.year_month}.xlsx`,
      data: Array.from(bytes),
    });
    if (saved) $q.notify({ type: "positive", message: "명세서를 저장했습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `다운로드 실패: ${e?.message ?? e}` });
  } finally {
    downloading.value = null;
  }
}

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">정산</div>
        <div class="text-caption text-grey-6">월별 정산 마감 및 명세서</div>
      </div>
      <div class="col-auto">
        <q-input
          v-model="targetMonth"
          label="정산월"
          mask="####-##"
          outlined
          dense
          style="width: 130px"
        />
      </div>
      <div class="col-auto">
        <q-btn
          color="primary"
          icon="o_play_arrow"
          label="정산 마감 실행"
          unelevated
          :loading="running"
          @click="runClose"
        />
      </div>
    </div>

    <!-- Summary -->
    <q-card flat bordered class="q-mb-md">
      <q-card-section class="row items-center">
        <div class="col">
          <div class="text-caption text-grey-6">완료된 정산 합계</div>
          <div class="text-h6 text-weight-bold">{{ won(totalClosed) }}</div>
        </div>
        <q-btn flat round dense icon="o_refresh" :loading="loading" @click="load">
          <q-tooltip>새로고침</q-tooltip>
        </q-btn>
      </q-card-section>
    </q-card>

    <!-- Skeleton -->
    <template v-if="loading">
      <q-skeleton type="rect" height="40px" class="q-mb-sm" />
      <q-skeleton type="rect" height="44px" class="q-mb-sm" v-for="n in 5" :key="n" />
    </template>

    <!-- Table -->
    <q-table
      v-else
      :rows="runs"
      :columns="columns"
      row-key="id"
      flat
      bordered
      :rows-per-page-options="[12, 24, 50]"
    >
      <template #body-cell-status="props">
        <q-td :props="props" class="text-center">
          <q-badge :color="statusMeta(props.row.status).color" :label="statusMeta(props.row.status).label" />
          <q-tooltip v-if="props.row.failure_reason">{{ props.row.failure_reason }}</q-tooltip>
        </q-td>
      </template>
      <template #body-cell-resident_count="props">
        <q-td :props="props">{{ props.row.resident_count ?? "—" }}</q-td>
      </template>
      <template #body-cell-total_amount="props">
        <q-td :props="props">{{ won(props.row.total_amount) }}</q-td>
      </template>
      <template #body-cell-triggered_at="props">
        <q-td :props="props">{{ fmtTime(props.row.triggered_at) }}</q-td>
      </template>
      <template #body-cell-completed_at="props">
        <q-td :props="props">{{ fmtTime(props.row.completed_at) }}</q-td>
      </template>
      <template #body-cell-actions="props">
        <q-td :props="props" class="text-center">
          <q-btn
            v-if="props.row.has_xlsx"
            flat round dense icon="o_download" color="primary"
            :loading="downloading === props.row.id"
            @click="download(props.row)"
          >
            <q-tooltip>명세서 다운로드</q-tooltip>
          </q-btn>
          <span v-else class="text-caption text-grey-4">—</span>
        </q-td>
      </template>
      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_account_balance" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">정산 내역이 없습니다</div>
          <div class="text-grey-4 text-caption">위에서 정산월을 선택해 마감을 실행하세요</div>
        </div>
      </template>
    </q-table>
  </q-page>
</template>
