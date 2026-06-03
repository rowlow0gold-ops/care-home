<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const router = useRouter();
const session = useServerSessionStore();
// Day-off approval is a desk task — both 행정 and 접수 (branch_manager) can decide.
const canDecide = computed(() => session.hasRole("branch_manager"));

type Row = Awaited<ReturnType<typeof server.leaveRequestsPaged>>["items"][number];

function openDetail(r: Row) {
  router.push({ path: `/staff/${r.user_id}`, query: { from: "leave" } });
}
async function startChat(r: Row) {
  try {
    const c = await server.createConversation({ invitee_id: r.user_id });
    router.push({ name: "chat", query: { conv: c.id, name: r.user_name } });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `대화 시작 실패: ${e?.message ?? e}` });
  }
}
const rows = ref<Row[]>([]);
const loading = ref(false);
const statusFilter = ref("pending");
const acting = ref<string | null>(null);
// server-side pagination
const page = ref(1);
const pageSize = ref(20);
const total = ref(0);
const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)));

const statusOptions = [
  { label: "대기", value: "pending" },
  { label: "승인", value: "approved" },
  { label: "반려", value: "rejected" },
  { label: "전체", value: "" },
];
const leaveTypeKo: Record<string, string> = {
  annual: "연차", sick: "병가", half_day: "반차", special: "경조사", unpaid: "무급", other: "기타",
};
const statusKo: Record<string, string> = { pending: "대기", approved: "승인", rejected: "반려", cancelled: "취소" };
const statusColor: Record<string, string> = { pending: "orange", approved: "positive", rejected: "negative", cancelled: "grey" };
const roleKo: Record<string, string> = { caregiver: "요양보호사", nurse: "간호사", branch_manager: "시설장", hq: "본사" };

function fmt(iso: string) { return new Date(iso).toLocaleString("ko-KR", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }); }

async function load() {
  loading.value = true;
  try {
    const res = await server.leaveRequestsPaged({
      status: statusFilter.value || undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    rows.value = res.items;
    total.value = res.total;
  } catch (e: any) {
    $q.notify({ type: "negative", message: `휴가 신청을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}
watch([statusFilter, pageSize], () => { page.value = 1; load(); });
watch(page, load);

async function doDecide(r: Row, status: "approved" | "rejected", note?: string) {
  acting.value = r.id;
  try {
    await server.decideLeave(r.id, status, note);
    $q.notify({ type: "positive", message: status === "approved" ? "승인되었습니다." : "반려되었습니다." });
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `처리 실패: ${e?.message ?? e}` });
  } finally {
    acting.value = null;
  }
}
function decide(r: Row, status: "approved" | "rejected") {
  if (status === "rejected") {
    // 반려 사유를 입력 — 요양보호사가 태블릿에서 읽고 협의합니다.
    $q.dialog({
      title: "반려 사유",
      message: `${r.user_name}님에게 전달할 반려 사유를 입력하세요.`,
      prompt: { model: "", type: "textarea", isValid: (v: string) => v.trim().length > 0 },
      cancel: { label: "취소", flat: true },
      ok: { label: "반려", color: "negative", unelevated: true },
      persistent: true,
    }).onOk((note: string) => doDecide(r, "rejected", note.trim()));
  } else {
    doDecide(r, "approved");
  }
}

const approvingAll = ref(false);
function approveAll() {
  $q.dialog({
    title: "전체 승인",
    message: "대기 중인 휴가 신청을 모두 승인할까요?",
    cancel: { label: "취소", flat: true },
    ok: { label: "전체 승인", color: "positive", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    approvingAll.value = true;
    try {
      const ids: string[] = [];
      let p = 1, totalP = 1;
      do {
        const res = await server.leaveRequestsPaged({ status: "pending", page: p, page_size: 100 });
        res.items.forEach((r) => ids.push(r.id));
        totalP = Math.max(1, Math.ceil(res.total / 100));
        p++;
      } while (p <= totalP);
      if (!ids.length) { $q.notify({ type: "info", message: "대기 중인 신청이 없습니다." }); return; }
      for (const id of ids) await server.decideLeave(id, "approved");
      $q.notify({ type: "positive", message: `${ids.length}건을 승인했습니다.` });
      await load();
    } catch (e: any) {
      $q.notify({ type: "negative", message: `전체 승인 실패: ${e?.message ?? e}` });
    } finally {
      approvingAll.value = false;
    }
  });
}

const columns = [
  { name: "user_name", label: "직원", field: "user_name", align: "left" as const },
  { name: "leave_type", label: "유형", field: "leave_type", align: "left" as const },
  { name: "period", label: "기간", field: "start_date", align: "left" as const },
  { name: "days", label: "일수", field: "days", align: "right" as const },
  { name: "reason", label: "사유", field: "reason", align: "left" as const },
  { name: "requested_at", label: "신청", field: "requested_at", align: "left" as const },
  { name: "status", label: "상태", field: "status", align: "center" as const },
  { name: "actions", label: "", field: "actions", align: "center" as const },
];

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">휴가</div>
        <div class="text-caption text-grey-6">태블릿에서 신청된 휴무를 승인/반려합니다</div>
      </div>
      <q-btn v-if="canDecide" unelevated color="positive" icon="o_done_all" label="전체 승인" :loading="approvingAll" @click="approveAll" />
      <div class="col-auto" style="min-width: 130px">
        <q-select v-model="statusFilter" :options="statusOptions" outlined dense emit-value map-options />
      </div>
      <q-btn flat round dense icon="o_refresh" :loading="loading" @click="load" />
    </div>

    <q-table :rows="rows" :columns="columns" row-key="id" flat bordered :loading="loading" hide-pagination :rows-per-page-options="[0]"
      class="cursor-pointer" @row-click="(_evt: Event, row: Row) => openDetail(row)">
      <template #body-cell-user_name="props">
        <q-td :props="props">
          <span class="text-weight-medium">{{ props.row.user_name }}</span>
          <span class="text-caption text-grey-6 q-ml-xs">{{ roleKo[props.row.user_role] ?? "" }}</span>
        </q-td>
      </template>
      <template #body-cell-leave_type="props">
        <q-td :props="props"><q-badge color="blue-grey-1" text-color="blue-grey-9" :label="leaveTypeKo[props.row.leave_type] ?? props.row.leave_type" /></q-td>
      </template>
      <template #body-cell-period="props">
        <q-td :props="props">{{ props.row.start_date }} ~ {{ props.row.end_date }}</q-td>
      </template>
      <template #body-cell-days="props"><q-td :props="props">{{ props.row.days }}일</q-td></template>
      <template #body-cell-reason="props"><q-td :props="props">{{ props.row.reason || "—" }}</q-td></template>
      <template #body-cell-requested_at="props"><q-td :props="props" class="text-grey-7">{{ fmt(props.row.requested_at) }}</q-td></template>
      <template #body-cell-status="props">
        <q-td :props="props" class="text-center"><q-badge :color="statusColor[props.row.status]" :label="statusKo[props.row.status]" /></q-td>
      </template>
      <template #body-cell-actions="props">
        <q-td :props="props" class="text-center">
          <q-btn dense flat round color="primary" icon="o_chat" :title="`${props.row.user_name}님과 대화`"
            @click.stop="startChat(props.row)" class="q-mr-xs" />
          <template v-if="canDecide && props.row.status === 'pending'">
            <q-btn dense unelevated color="positive" label="승인" :loading="acting === props.row.id" @click.stop="decide(props.row, 'approved')" class="q-mr-xs" />
            <q-btn dense outline color="negative" label="반려" :loading="acting === props.row.id" @click.stop="decide(props.row, 'rejected')" />
          </template>
        </q-td>
      </template>
      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_event_available" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">신청 내역이 없습니다</div>
        </div>
      </template>
    </q-table>

    <div class="row items-center justify-end q-mt-md q-gutter-md">
      <span class="text-caption text-grey-6">총 {{ total }}건</span>
      <q-select v-model="pageSize" :options="[10, 20, 50]" dense outlined style="min-width:80px" />
      <q-pagination v-model="page" :max="totalPages" :max-pages="7" boundary-numbers direction-links />
    </div>
  </q-page>
</template>
