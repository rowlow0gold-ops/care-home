<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type Resident } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const router = useRouter();
const session = useServerSessionStore();

// ── Filters ─────────────────────────────────────────────────────────────────
const q = ref("");
const gradeFilter = ref("");
const careTypeFilter = ref("");
const statusFilter = ref("active");
const page = ref(1);
const pageSize = ref(25);
const sortBy = ref<"full_name" | "room_number" | "care_grade" | "admitted_on">("full_name");
const sortDesc = ref(false);

const rows = ref<Resident[]>([]);
const total = ref(0);
const loading = ref(false);
const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)));

const gradeOptions = [
  { label: "전체 등급", value: "" },
  { label: "1등급", value: "1" }, { label: "2등급", value: "2" }, { label: "3등급", value: "3" },
  { label: "4등급", value: "4" }, { label: "5등급", value: "5" }, { label: "인지지원", value: "cognitive_support" },
];
const statusOptions = [
  { label: "재원중", value: "active" },
  { label: "퇴소", value: "discharged" },
  { label: "사망", value: "deceased" },
];
// 서비스 유형: 요양(residential) / 주간(day) / 방문(visit)
const careTypeOptions = [
  { label: "전체 유형", value: "" },
  { label: "요양", value: "residential" },
  { label: "주간", value: "day" },
  { label: "방문", value: "visit" },
];
const careTypeLabel: Record<string, string> = { residential: "요양", day: "주간", visit: "방문" };
const careTypeColor: Record<string, string> = { residential: "teal", day: "indigo", visit: "deep-orange" };

const sexLabel: Record<string, string> = { male: "남", female: "여", other: "기타" };
const statusLabel: Record<string, string> = { active: "재원", discharged: "퇴소", deceased: "사망" };
const statusColor: Record<string, string> = { active: "primary", discharged: "grey", deceased: "negative" };
function gradeLabel(g: string | null) {
  if (!g) return "—";
  return g === "cognitive_support" ? "인지지원" : g + "등급";
}
function age(birth: string) {
  const b = new Date(birth), now = new Date();
  let a = now.getFullYear() - b.getFullYear();
  if (now.getMonth() < b.getMonth() || (now.getMonth() === b.getMonth() && now.getDate() < b.getDate())) a--;
  return a;
}

async function load() {
  loading.value = true;
  try {
    const res = await server.residentsPaged({
      q: q.value.trim() || undefined,
      branch_id: session.me?.branch_id ?? undefined,
      care_grade: gradeFilter.value || undefined,
      care_type: careTypeFilter.value || undefined,
      status: statusFilter.value,
      page: page.value,
      page_size: pageSize.value,
      sort_by: sortBy.value,
      sort_desc: sortDesc.value,
    });
    rows.value = res.items;
    total.value = res.total;
  } catch (e: any) {
    $q.notify({ type: "negative", message: `목록을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}
watch([gradeFilter, careTypeFilter, statusFilter, page, pageSize, sortBy, sortDesc], load);
function applySearch() { page.value = 1; load(); }
function setSort(col: typeof sortBy.value) {
  if (sortBy.value === col) sortDesc.value = !sortDesc.value;
  else { sortBy.value = col; sortDesc.value = false; }
  page.value = 1;
}

const columns = [
  { name: "full_name", label: "이름", field: "full_name", align: "left" as const },
  { name: "room_number", label: "호실", field: "room_number", align: "left" as const },
  { name: "care_type", label: "유형", field: "care_type", align: "center" as const },
  { name: "sex", label: "성별", field: "sex", align: "center" as const },
  { name: "age", label: "나이", field: "birth_date", align: "right" as const },
  { name: "care_grade", label: "장기요양", field: "care_grade", align: "right" as const },
  { name: "admitted_on", label: "입소일", field: "admitted_on", align: "left" as const },
  { name: "status", label: "상태", field: "status", align: "center" as const },
];

function open(r: Resident) { router.push(`/residents/${r.id}`); }

// ── Export ──────────────────────────────────────────────────────────────────
const exporting = ref(false);
async function exportXlsx() {
  exporting.value = true;
  try {
    const bytes = await server.exportXlsx("/api/v1/residents/export.xlsx");
    const saved = await invoke<string | null>("save_excel", { filename: "어르신_명단.xlsx", data: Array.from(bytes) });
    if (saved) $q.notify({ type: "positive", message: "어르신 명단을 저장했습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `내보내기 실패: ${e?.message ?? e}` });
  } finally {
    exporting.value = false;
  }
}

// ── Add resident ────────────────────────────────────────────────────────────
const showAdd = ref(false);
const addRef = ref();
const submitting = ref(false);
const emptyAdd = () => ({
  full_name: "", sex: "female" as "male" | "female" | "other",
  birth_date: "", care_grade: "" as string,
  care_type: "residential" as "residential" | "day" | "visit",
  room_number: "", admitted_on: new Date().toISOString().slice(0, 10),
});
const addForm = ref(emptyAdd());
const sexOptions = [{ label: "여", value: "female" }, { label: "남", value: "male" }, { label: "기타", value: "other" }];
const addGradeOptions = gradeOptions.filter((g) => g.value);

async function submitAdd() {
  const valid = await addRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await server.createResident({
      full_name: addForm.value.full_name.trim(),
      sex: addForm.value.sex,
      birth_date: addForm.value.birth_date,
      care_grade: addForm.value.care_grade || null,
      care_type: addForm.value.care_type,
      room_number: addForm.value.room_number.trim() || null,
      admitted_on: addForm.value.admitted_on,
    });
    $q.notify({ type: "positive", message: "어르신이 등록되었습니다." });
    showAdd.value = false;
    addForm.value = emptyAdd();
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `등록 실패: ${e?.message ?? e}` });
  } finally {
    submitting.value = false;
  }
}

onMounted(() => { load(); });
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-md">
      <div class="col">
        <div class="text-h5 text-weight-bold">어르신</div>
        <div class="text-caption text-grey-6">총 {{ total }}명</div>
      </div>
      <div class="col-auto q-gutter-sm">
        <q-btn outline color="primary" icon="o_download" label="내보내기" :loading="exporting" @click="exportXlsx" />
        <q-btn v-if="session.canCreate" color="primary" icon="o_person_add" label="어르신 등록" unelevated
               @click="() => { addForm = emptyAdd(); showAdd = true; }" />
      </div>
    </div>

    <!-- Filters -->
    <div class="row q-col-gutter-sm q-mb-md items-center">
      <div class="col-12 col-sm-5 col-md-4">
        <q-input v-model="q" dense outlined clearable placeholder="이름 또는 호실 검색"
                 @keyup.enter="applySearch" @clear="applySearch">
          <template #prepend><q-icon name="search" /></template>
          <template #append><q-btn flat dense label="검색" color="primary" @click="applySearch" /></template>
        </q-input>
      </div>
      <div class="col-6 col-sm-3 col-md-2">
        <q-select v-model="careTypeFilter" :options="careTypeOptions" dense outlined emit-value map-options />
      </div>
      <div class="col-6 col-sm-3 col-md-2">
        <q-select v-model="gradeFilter" :options="gradeOptions" dense outlined emit-value map-options />
      </div>
      <div class="col-6 col-sm-3 col-md-2">
        <q-select v-model="statusFilter" :options="statusOptions" dense outlined emit-value map-options />
      </div>
    </div>

    <q-table
      :rows="rows"
      :columns="columns"
      row-key="id"
      flat bordered
      :loading="loading"
      hide-pagination
      :rows-per-page-options="[0]"
      @row-click="(_e, r) => open(r)"
      :class="rows.length ? 'cursor-pointer-rows' : ''"
    >
      <template #header-cell-full_name="props">
        <q-th :props="props" class="cursor-pointer" @click="setSort('full_name')">이름 <q-icon name="unfold_more" size="14px" /></q-th>
      </template>
      <template #header-cell-room_number="props">
        <q-th :props="props" class="cursor-pointer" @click="setSort('room_number')">호실 <q-icon name="unfold_more" size="14px" /></q-th>
      </template>
      <template #header-cell-care_grade="props">
        <q-th :props="props" class="cursor-pointer" @click="setSort('care_grade')">장기요양 <q-icon name="unfold_more" size="14px" /></q-th>
      </template>
      <template #header-cell-admitted_on="props">
        <q-th :props="props" class="cursor-pointer" @click="setSort('admitted_on')">입소일 <q-icon name="unfold_more" size="14px" /></q-th>
      </template>

      <template #body-cell-full_name="props">
        <q-td :props="props"><span class="text-weight-medium">{{ props.row.full_name }}</span></q-td>
      </template>
      <template #body-cell-room_number="props">
        <q-td :props="props">{{ props.row.room_number ?? "—" }}</q-td>
      </template>
      <template #body-cell-care_type="props">
        <q-td :props="props" class="text-center">
          <q-badge :color="careTypeColor[props.row.care_type] ?? 'grey'" :label="careTypeLabel[props.row.care_type] ?? props.row.care_type" />
        </q-td>
      </template>
      <template #body-cell-sex="props"><q-td :props="props">{{ sexLabel[props.row.sex] }}</q-td></template>
      <template #body-cell-age="props"><q-td :props="props">{{ age(props.row.birth_date) }}세</q-td></template>
      <template #body-cell-care_grade="props"><q-td :props="props">{{ gradeLabel(props.row.care_grade) }}</q-td></template>
      <template #body-cell-admitted_on="props"><q-td :props="props" class="text-grey-7">{{ props.row.admitted_on }}</q-td></template>
      <template #body-cell-status="props">
        <q-td :props="props" class="text-center">
          <q-badge :color="statusColor[props.row.status]" :label="statusLabel[props.row.status]" />
        </q-td>
      </template>
      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_elderly" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">조건에 맞는 결과가 없습니다</div>
        </div>
      </template>
    </q-table>

    <div class="row items-center justify-end q-mt-md q-gutter-md">
      <q-select v-model="pageSize" :options="[10, 25, 50, 100]" dense outlined style="min-width:90px" />
      <q-pagination v-model="page" :max="totalPages" :max-pages="7" boundary-numbers direction-links />
    </div>

    <!-- Add Resident -->
    <q-dialog v-model="showAdd" persistent>
      <q-card style="min-width: 480px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">어르신 등록</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section>
          <q-form ref="addRef" class="q-gutter-sm">
            <q-input v-model="addForm.full_name" label="성함 *" outlined dense :rules="[(v)=>!!v?.trim()||'성함을 입력하세요']" lazy-rules="ondemand" />
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="addForm.sex" :options="sexOptions" label="성별" outlined dense emit-value map-options />
              <q-input class="col" v-model="addForm.birth_date" label="생년월일 *" mask="####-##-##" placeholder="1940-01-01" outlined dense :rules="[(v)=>/^\d{4}-\d{2}-\d{2}$/.test(v)||'YYYY-MM-DD']" lazy-rules="ondemand" />
            </div>
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="addForm.care_grade" :options="addGradeOptions" label="장기요양등급" outlined dense emit-value map-options clearable />
              <q-select class="col" v-model="addForm.care_type" :options="careTypeOptions.filter((c) => c.value)" label="서비스 유형" outlined dense emit-value map-options />
            </div>
            <div class="row q-gutter-sm">
              <q-input class="col" v-model="addForm.room_number" label="호실" outlined dense />
            </div>
            <q-input v-model="addForm.admitted_on" label="입소일 *" mask="####-##-##" outlined dense :rules="[(v)=>/^\d{4}-\d{2}-\d{2}$/.test(v)||'YYYY-MM-DD']" lazy-rules="ondemand" />
          </q-form>
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="등록" unelevated :loading="submitting" @click="submitAdd" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<style scoped>
.cursor-pointer-rows :deep(tbody tr) { cursor: pointer; }
.cursor-pointer-rows :deep(tbody tr:hover td) { background: #f0f9ff; }
</style>
