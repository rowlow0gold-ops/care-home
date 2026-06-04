<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type OrgPerson } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const router = useRouter();
const session = useServerSessionStore();

const canCreate = computed(() => session.canCreate);
const canEdit = computed(() => session.canEdit);
const canDelete = computed(() => session.canDelete);
function openDetail(p: OrgPerson) { router.push(`/staff/${p.id}`); }

// ── Position / employment options (server enums) ────────────────────────────
const POSITION_OPTIONS = [
  { label: "시설장", value: "branch_manager" },
  { label: "사무국장", value: "office_manager" },
  { label: "사회복지사", value: "social_worker" },
  { label: "간호사", value: "nurse_rn" },
  { label: "간호조무사", value: "nurse_assistant" },
  { label: "영양사", value: "dietitian" },
  { label: "물리치료사", value: "physical_therapist" },
  { label: "작업치료사", value: "occupational_therapist" },
  { label: "요양보호사", value: "caregiver" },
  { label: "조리원", value: "cook" },
  { label: "환경미화원", value: "cleaner" },
  { label: "운전기사", value: "driver" },
  { label: "촉탁의", value: "doctor_visiting" },
  { label: "사무직", value: "it" },
  { label: "기타", value: "other" },
];
const EMPLOYMENT_OPTIONS = [
  { label: "정규직", value: "regular" },
  { label: "계약직", value: "contract" },
  { label: "아르바이트", value: "part_time" },
  { label: "위촉직", value: "consultant" },
];
// 근무조 (북미식 2주 스케줄) — 직원마다 조가 고정되고 2주 단위로 발행된다
const GROUP_OPTIONS = [
  { label: "미지정 (스케줄 제외)", value: null as string | null },
  { label: "요양 12시간조 — 3일 근무·4일 휴무", value: "h12" },
  { label: "요양 8시간 교대조 — 주 5일·주말 순환", value: "h8" },
];
const GROUP_KO: Record<string, string> = { h12: "12시간조", h8: "8시간조" };
const SHIFT12_OPTIONS = [
  { label: "Day 07:00–19:30", value: "day" },
  { label: "Night 19:00–07:30", value: "night" },
];
const SHIFT8_OPTIONS = [
  { label: "주간 07:00–15:00", value: "day" },
  { label: "오후 15:00–23:00", value: "evening" },
  { label: "야간 23:00–07:00", value: "night" },
];
const SHIFT_PREF_KO: Record<string, string> = { day: "주간", evening: "오후", night: "야간" };
function prefText(p: OrgPerson): string {
  if (!p.shift_group) return "미지정";
  const shift = p.preferred_shift ? SHIFT_PREF_KO[p.preferred_shift] ?? p.preferred_shift : "?";
  return `${GROUP_KO[p.shift_group] ?? p.shift_group} ${shift}`;
}

const ROLE_OPTIONS = [
  { label: "요양보호사", value: "caregiver" },
  { label: "간호사", value: "nurse" },
  { label: "시설장", value: "branch_manager" },
  { label: "본사", value: "hq" },
];

// ── List state (HQ-style: search + 고용형태 filter + pagination) ─────────────
const rows = ref<OrgPerson[]>([]);
const total = ref(0);
const loading = ref(false);
const q = ref("");
const empFilter = ref<string>("");
const page = ref(1);
const pageSize = ref(25);
const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)));

const empOptions = [{ label: "전체 고용형태", value: "" }, ...EMPLOYMENT_OPTIONS];

async function load() {
  loading.value = true;
  try {
    const res = await server.orgPaged({
      q: q.value.trim() || undefined,
      employment_type: empFilter.value || undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    rows.value = res.items;
    total.value = res.total;
  } catch (e: any) {
    $q.notify({ type: "negative", message: `직원 목록을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}
watch([empFilter, page, pageSize], load);
function applySearch() { page.value = 1; load(); }

// 경력 (tenure) from hired_on
function tenure(hired: string | null): string {
  if (!hired) return "—";
  const start = new Date(hired + "T00:00:00");
  const now = new Date();
  let months = (now.getFullYear() - start.getFullYear()) * 12 + (now.getMonth() - start.getMonth());
  if (now.getDate() < start.getDate()) months--;
  if (months < 0) return "—";
  const y = Math.floor(months / 12), m = months % 12;
  if (y > 0 && m > 0) return `${y}년 ${m}개월`;
  if (y > 0) return `${y}년`;
  return `${m}개월`;
}

const columns = [
  { name: "full_name", label: "이름", field: "full_name", align: "left" as const },
  { name: "branch_name", label: "소속", field: "branch_name", align: "left" as const },
  { name: "position_ko", label: "직책", field: "position_ko", align: "left" as const },
  { name: "employment_type_ko", label: "고용", field: "employment_type_ko", align: "left" as const },
  { name: "work_prefs", label: "근무조", field: "shift_group", align: "left" as const },
  { name: "tenure", label: "경력", field: "hired_on", align: "left" as const },
  { name: "email", label: "이메일", field: "email", align: "left" as const },
  { name: "phone", label: "전화", field: "phone", align: "left" as const },
  { name: "actions", label: "", field: "actions", align: "center" as const },
];

// ── Export (내보내기) ───────────────────────────────────────────────────────
const exporting = ref(false);
async function exportXlsx() {
  exporting.value = true;
  try {
    const bytes = await server.exportXlsx("/api/v1/staff/export.xlsx");
    const saved = await invoke<string | null>("save_excel", { filename: "직원_명단.xlsx", data: Array.from(bytes) });
    if (saved) $q.notify({ type: "positive", message: "직원 명단을 저장했습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `내보내기 실패: ${e?.message ?? e}` });
  } finally {
    exporting.value = false;
  }
}

// ── Add ─────────────────────────────────────────────────────────────────────
const showAddDialog = ref(false);
const addFormRef = ref();
const submitting = ref(false);
const emptyAdd = () => ({
  full_name: "", email: "", password: "", role: "caregiver",
  position: "caregiver", employment_type: "regular", phone: "",
});
const addForm = ref(emptyAdd());

async function submitAdd() {
  const valid = await addFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await server.createStaff({
      full_name: addForm.value.full_name.trim(),
      email: addForm.value.email.trim(),
      password: addForm.value.password,
      role: addForm.value.role,
      position: addForm.value.position,
      employment_type: addForm.value.employment_type,
      phone: addForm.value.phone.trim() || null,
    });
    $q.notify({ type: "positive", message: "직원이 추가되었습니다." });
    showAddDialog.value = false;
    addForm.value = emptyAdd();
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.status === 409 ? "이미 등록된 이메일입니다." : `추가 실패: ${e?.message ?? e}` });
  } finally {
    submitting.value = false;
  }
}

// ── Edit (HQ-only) ──────────────────────────────────────────────────────────
const showEditDialog = ref(false);
const editFormRef = ref();
const selected = ref<OrgPerson | null>(null);
const editForm = ref({ full_name: "", email: "", phone: "", position: "caregiver", employment_type: "regular" });

function openEdit(p: OrgPerson) {
  selected.value = p;
  editForm.value = {
    full_name: p.full_name, email: p.email, phone: p.phone ?? "",
    position: p.position, employment_type: p.employment_type,
  };
  showEditDialog.value = true;
}
async function submitEdit() {
  if (!selected.value) return;
  const valid = await editFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await server.updateStaff(selected.value.id, {
      expected_updated_at: selected.value.updated_at,
      full_name: editForm.value.full_name.trim(),
      email: editForm.value.email.trim(),
      phone: editForm.value.phone.trim() || null,
      position: editForm.value.position,
      employment_type: editForm.value.employment_type,
    });
    $q.notify({ type: "positive", message: "직원 정보가 수정되었습니다." });
    showEditDialog.value = false;
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.status === 409 ? "다른 사용자가 먼저 수정했습니다. 새로고침 후 다시 시도하세요." : `수정 실패: ${e?.message ?? e}` });
  } finally {
    submitting.value = false;
  }
}

// ── 근무조 편집 — 센터(행정)에서 직접 관리 ──────────────────────────────────
const showPrefsDialog = ref(false);
const prefsTarget = ref<OrgPerson | null>(null);
const prefsForm = ref<{ group: string | null; shift: string | null; days: number[] }>({ group: null, shift: null, days: [] });
function openPrefs(p: OrgPerson) {
  prefsTarget.value = p;
  prefsForm.value = { group: p.shift_group ?? null, shift: p.preferred_shift ?? null, days: [...(p.work_days ?? [])] };
  showPrefsDialog.value = true;
}
// 조를 바꾸면 교대 기본값을 보정 (12시간조에는 '오후'가 없다)
watch(() => prefsForm.value.group, (g) => {
  if (g === "h12" && prefsForm.value.shift !== "night") prefsForm.value.shift = "day";
  if (g === "h8" && !prefsForm.value.shift) prefsForm.value.shift = "day";
});
async function submitPrefs() {
  if (!prefsTarget.value) return;
  const f = prefsForm.value;
  submitting.value = true;
  try {
    await server.updateWorkPrefs(prefsTarget.value.id, {
      shift_group: f.group,
      preferred_shift: f.group ? f.shift : null,
      work_days: null,
    });
    $q.notify({ type: "positive", message: `${prefsTarget.value.full_name}님의 근무조를 저장했습니다. 다음 2주 스케줄 발행에 반영됩니다.` });
    showPrefsDialog.value = false;
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `저장 실패: ${e?.body?.message ?? e?.message ?? e}` });
  } finally {
    submitting.value = false;
  }
}

function confirmDeactivate(p: OrgPerson) {
  $q.dialog({
    title: "직원 비활성화",
    message: `${p.full_name}님을 비활성화하시겠습니까? 더 이상 로그인할 수 없습니다.`,
    cancel: { label: "취소", flat: true },
    ok: { label: "비활성화", color: "negative", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    try {
      await server.deactivateStaff(p.id);
      $q.notify({ type: "positive", message: "비활성화되었습니다." });
      await load();
    } catch (e: any) {
      $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` });
    }
  });
}

onMounted(() => { load(); });
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-md">
      <div class="col">
        <div class="text-h5 text-weight-bold">직원 관리</div>
        <div class="text-caption text-grey-6">총 {{ total }}명</div>
      </div>
      <div class="col-auto q-gutter-sm">
        <q-btn outline color="primary" icon="o_download" label="내보내기" :loading="exporting" @click="exportXlsx" />
        <q-btn v-if="canCreate" color="primary" icon="o_person_add" label="직원 추가"
               unelevated @click="() => { addForm = emptyAdd(); showAddDialog = true; }" />
      </div>
    </div>

    <!-- Filters -->
    <div class="row q-col-gutter-sm q-mb-md items-center">
      <div class="col-12 col-sm-6 col-md-4">
        <q-input v-model="q" dense outlined clearable placeholder="이름 / 이메일 / 직책 검색"
                 @keyup.enter="applySearch" @clear="applySearch">
          <template #prepend><q-icon name="search" /></template>
          <template #append><q-btn flat dense label="검색" color="primary" @click="applySearch" /></template>
        </q-input>
      </div>
      <div class="col-6 col-sm-3 col-md-2">
        <q-select v-model="empFilter" :options="empOptions" dense outlined emit-value map-options />
      </div>
    </div>

    <!-- Table -->
    <q-table
      :rows="rows"
      :columns="columns"
      row-key="id"
      flat bordered
      :loading="loading"
      hide-pagination
      :rows-per-page-options="[0]"
      @row-click="(_e, r) => openDetail(r)"
      :class="rows.length ? 'cursor-pointer-rows' : ''"
    >
      <template #body-cell-full_name="props">
        <q-td :props="props"><span class="text-weight-medium">{{ props.row.full_name }}</span></q-td>
      </template>
      <template #body-cell-position_ko="props">
        <q-td :props="props"><q-badge color="blue-grey-1" text-color="blue-grey-9" :label="props.row.position_ko" /></q-td>
      </template>
      <template #body-cell-work_prefs="props">
        <q-td :props="props">
          <q-chip dense clickable size="sm" :color="props.row.shift_group ? 'teal-1' : 'grey-2'"
            :text-color="props.row.shift_group ? 'teal-9' : 'grey-7'"
            icon="o_tune" @click.stop="openPrefs(props.row)">
            {{ prefText(props.row) }}
            <q-tooltip>클릭하여 근무조 수정</q-tooltip>
          </q-chip>
        </q-td>
      </template>
      <template #body-cell-tenure="props">
        <q-td :props="props">{{ tenure(props.row.hired_on) }}</q-td>
      </template>
      <template #body-cell-phone="props">
        <q-td :props="props">{{ props.row.phone || "—" }}</q-td>
      </template>
      <template #body-cell-actions="props">
        <q-td :props="props" class="text-center">
          <q-btn v-if="canEdit" flat round dense icon="o_edit" color="primary" @click.stop="openEdit(props.row)"><q-tooltip>수정</q-tooltip></q-btn>
          <q-btn v-if="canDelete" flat round dense icon="o_person_off" color="negative" @click.stop="confirmDeactivate(props.row)"><q-tooltip>비활성화</q-tooltip></q-btn>
          <q-icon name="o_chevron_right" color="grey-5" />
        </q-td>
      </template>
      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_group" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">직원이 없습니다</div>
        </div>
      </template>
    </q-table>

    <!-- Pagination -->
    <div class="row items-center justify-end q-mt-md q-gutter-md">
      <q-select v-model="pageSize" :options="[10, 25, 50, 100]" dense outlined style="min-width:90px" />
      <q-pagination v-model="page" :max="totalPages" :max-pages="7" boundary-numbers direction-links />
    </div>

    <!-- Add Dialog -->
    <q-dialog v-model="showAddDialog" persistent>
      <q-card style="min-width: 540px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">직원 추가</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section>
          <q-form ref="addFormRef" class="q-gutter-sm">
            <div class="row q-gutter-sm">
              <q-input class="col" v-model="addForm.full_name" label="이름 *" outlined dense :rules="[(v)=>!!v?.trim()||'이름을 입력하세요']" lazy-rules="ondemand" />
              <q-input class="col" v-model="addForm.email" label="이메일 *" type="email" outlined dense :rules="[(v)=>!!v?.trim()||'이메일을 입력하세요']" lazy-rules="ondemand" />
            </div>
            <div class="row q-gutter-sm">
              <q-input class="col" v-model="addForm.password" label="초기 비밀번호 *" type="password" outlined dense :rules="[(v)=>(v?.length??0)>=8||'8자 이상']" lazy-rules="ondemand" />
              <q-select class="col" v-model="addForm.role" :options="ROLE_OPTIONS" label="권한" outlined dense emit-value map-options />
            </div>
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="addForm.position" :options="POSITION_OPTIONS" label="직책" outlined dense emit-value map-options />
              <q-select class="col" v-model="addForm.employment_type" :options="EMPLOYMENT_OPTIONS" label="고용형태" outlined dense emit-value map-options />
            </div>
            <q-input v-model="addForm.phone" label="연락처" outlined dense />
          </q-form>
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="추가" unelevated :loading="submitting" @click="submitAdd" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- 근무조 Dialog -->
    <q-dialog v-model="showPrefsDialog">
      <q-card style="min-width: 460px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">근무조 — {{ prefsTarget?.full_name }}</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-md">
          <div class="text-caption text-grey-7">
            북미식 2주 스케줄 — 직원마다 조가 고정되고, 스케쥴러에서 2주 단위로 발행됩니다.
          </div>
          <q-select v-model="prefsForm.group" :options="GROUP_OPTIONS" label="근무조"
            outlined dense emit-value map-options />
          <q-select v-if="prefsForm.group === 'h12'" v-model="prefsForm.shift" :options="SHIFT12_OPTIONS"
            label="교대 (12시간)" outlined dense emit-value map-options
            hint="3일 연속 근무 후 4일 휴무 — 주 36시간" />
          <q-select v-else-if="prefsForm.group === 'h8'" v-model="prefsForm.shift"
            :options="SHIFT8_OPTIONS" label="교대 (8시간 고정)" outlined dense emit-value map-options
            hint="주 5일 근무 (휴무 2일 보장) — 평주는 토·일 휴무, 3주마다 주말 의무 근무(순환)" />
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="저장" unelevated :loading="submitting" @click="submitPrefs" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- Edit Dialog -->
    <q-dialog v-model="showEditDialog" persistent>
      <q-card style="min-width: 480px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">직원 정보 수정</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section>
          <q-form ref="editFormRef" class="q-gutter-sm">
            <div class="row q-gutter-sm">
              <q-input class="col" v-model="editForm.full_name" label="이름 *" outlined dense :rules="[(v)=>!!v?.trim()||'이름을 입력하세요']" lazy-rules="ondemand" />
              <q-input class="col" v-model="editForm.email" label="이메일" type="email" outlined dense />
            </div>
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="editForm.position" :options="POSITION_OPTIONS" label="직책" outlined dense emit-value map-options />
              <q-select class="col" v-model="editForm.employment_type" :options="EMPLOYMENT_OPTIONS" label="고용형태" outlined dense emit-value map-options />
            </div>
            <q-input v-model="editForm.phone" label="연락처" outlined dense />
          </q-form>
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="저장" unelevated :loading="submitting" @click="submitEdit" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<style scoped>
.cursor-pointer-rows :deep(tbody tr) { cursor: pointer; }
.cursor-pointer-rows :deep(tbody tr:hover td) { background: #f0f9ff; }
</style>
