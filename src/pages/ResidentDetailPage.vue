<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type Resident } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const route = useRoute();
const router = useRouter();
const $q = useQuasar();
const session = useServerSessionStore();
const id = computed(() => String(route.params.id));

const resident = ref<Resident | null>(null);
const tab = ref<"care" | "vitals" | "meds" | "photos">("care");
const canCreate = computed(() => session.canCreate);
const canEdit = computed(() => session.canEdit);
const canDelete = computed(() => session.canDelete);

const sexLabel: Record<string, string> = { male: "남", female: "여", other: "기타" };
const statusLabel: Record<string, string> = { active: "재원", discharged: "퇴소", deceased: "사망" };
function gradeLabel(g: string | null) { return !g ? "—" : g === "cognitive_support" ? "인지지원" : g + "등급"; }
function age(b?: string) {
  if (!b) return "—";
  const d = new Date(b), n = new Date();
  let a = n.getFullYear() - d.getFullYear();
  if (n.getMonth() < d.getMonth() || (n.getMonth() === d.getMonth() && n.getDate() < d.getDate())) a--;
  return a + "세";
}
function fmt(iso: string) {
  return new Date(iso).toLocaleString("ko-KR", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

async function loadResident() {
  try { resident.value = await server.resident(id.value); }
  catch (e: any) { $q.notify({ type: "negative", message: `어르신 정보를 불러오지 못했습니다: ${e?.message ?? e}` }); }
}

// ── 어르신 수정 / 상태변경 (CRUD) ───────────────────────────────────────────
const showEdit = ref(false);
const editRef = ref();
const savingEdit = ref(false);
const editForm = ref({ full_name: "", sex: "female" as "male" | "female" | "other", birth_date: "", care_grade: "", room_number: "", admitted_on: "" });
const sexOptions = [{ label: "여", value: "female" }, { label: "남", value: "male" }, { label: "기타", value: "other" }];
const gradeOptions = [
  { label: "1등급", value: "1" }, { label: "2등급", value: "2" }, { label: "3등급", value: "3" },
  { label: "4등급", value: "4" }, { label: "5등급", value: "5" }, { label: "인지지원", value: "cognitive_support" },
];
function openEdit() {
  if (!resident.value) return;
  const r = resident.value;
  editForm.value = {
    full_name: r.full_name, sex: r.sex, birth_date: r.birth_date,
    care_grade: r.care_grade ?? "", room_number: r.room_number ?? "", admitted_on: r.admitted_on,
  };
  showEdit.value = true;
}
async function submitEdit() {
  if (!resident.value) return;
  const valid = await editRef.value?.validate();
  if (!valid) return;
  savingEdit.value = true;
  try {
    await server.updateResident(resident.value.id, {
      full_name: editForm.value.full_name.trim(),
      sex: editForm.value.sex,
      birth_date: editForm.value.birth_date,
      care_grade: editForm.value.care_grade || null,
      room_number: editForm.value.room_number.trim() || null,
      admitted_on: editForm.value.admitted_on,
    });
    $q.notify({ type: "positive", message: "어르신 정보가 수정되었습니다." });
    showEdit.value = false;
    await loadResident();
  } catch (e: any) { $q.notify({ type: "negative", message: `수정 실패: ${e?.message ?? e}` }); }
  finally { savingEdit.value = false; }
}
function confirmDischarge() {
  if (!resident.value) return;
  $q.dialog({
    title: "퇴소 처리", message: `${resident.value.full_name} 어르신을 퇴소 처리하시겠습니까?`,
    cancel: { label: "취소", flat: true }, ok: { label: "퇴소", color: "negative", unelevated: true }, persistent: true,
  }).onOk(async () => {
    try {
      await server.dischargeResident(resident.value!.id, new Date().toISOString().slice(0, 10));
      $q.notify({ type: "positive", message: "퇴소 처리되었습니다." });
      await loadResident();
    } catch (e: any) { $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` }); }
  });
}
function confirmDecease() {
  if (!resident.value) return;
  $q.dialog({
    title: "사망 처리", message: `${resident.value.full_name} 어르신을 사망 처리하시겠습니까?`,
    cancel: { label: "취소", flat: true }, ok: { label: "사망 처리", color: "negative", unelevated: true }, persistent: true,
  }).onOk(async () => {
    try {
      await server.deceaseResident(resident.value!.id);
      $q.notify({ type: "positive", message: "처리되었습니다." });
      await loadResident();
    } catch (e: any) { $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` }); }
  });
}

// ── 케어 기록 ───────────────────────────────────────────────────────────────
const CARE_CATEGORIES = [
  { label: "식사", value: "meal" }, { label: "투약", value: "medication" },
  { label: "위생", value: "hygiene" }, { label: "이동", value: "mobility" },
  { label: "정서", value: "mood" }, { label: "특이사항", value: "incident" },
  { label: "기타", value: "other" },
];
const careCatKo = Object.fromEntries(CARE_CATEGORIES.map((c) => [c.value, c.label]));
const PAGE = 10;
const careLogs = ref<any[]>([]);
const carePage = ref(1); const careTotal = ref(0);
const careMax = computed(() => Math.max(1, Math.ceil(careTotal.value / PAGE)));
const careForm = ref({ category: "meal", body: "" });
const savingCare = ref(false);
async function loadCare() {
  try { const r = await server.careLogsPaged(id.value, carePage.value, PAGE); careLogs.value = r.items; careTotal.value = r.total; }
  catch { careLogs.value = []; }
}
async function addCare() {
  if (!careForm.value.body.trim()) { $q.notify({ type: "warning", message: "내용을 입력하세요." }); return; }
  savingCare.value = true;
  try {
    await server.createCareLog({ resident_id: id.value, category: careForm.value.category, body: careForm.value.body.trim() });
    $q.notify({ type: "positive", message: "케어 기록이 저장되었습니다." });
    careForm.value = { category: "meal", body: "" };
    carePage.value = 1;
    await loadCare();
  } catch (e: any) { $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` }); }
  finally { savingCare.value = false; }
}
function deleteCare(c: any) {
  $q.dialog({ title: "기록 삭제", message: "이 케어 기록을 삭제할까요?", cancel: { label: "취소", flat: true }, ok: { label: "삭제", color: "negative", unelevated: true }, persistent: true })
    .onOk(async () => {
      try { await server.deleteCareLog(c.id); if (careLogs.value.length === 1 && carePage.value > 1) carePage.value--; await loadCare(); $q.notify({ type: "positive", message: "삭제되었습니다." }); }
      catch (e: any) { $q.notify({ type: "negative", message: `삭제 실패: ${e?.message ?? e}` }); }
    });
}

// ── 활력징후 ────────────────────────────────────────────────────────────────
const VITAL_KINDS = [
  { label: "체온(℃)", value: "temperature_celsius" }, { label: "맥박(bpm)", value: "heart_rate" },
  { label: "수축기혈압", value: "blood_pressure_systolic" }, { label: "이완기혈압", value: "blood_pressure_diastolic" },
  { label: "산소포화도(%)", value: "spo2" }, { label: "혈당", value: "blood_glucose" },
];
const vitalKindKo = Object.fromEntries(VITAL_KINDS.map((k) => [k.value, k.label]));
const vitals = ref<any[]>([]);
const vitalPage = ref(1); const vitalTotal = ref(0);
const vitalMax = computed(() => Math.max(1, Math.ceil(vitalTotal.value / PAGE)));
const vitalForm = ref({ kind: "temperature_celsius", value: "", note: "" });
const savingVital = ref(false);
async function loadVitals() {
  try { const r = await server.vitalsPaged(id.value, vitalPage.value, PAGE); vitals.value = r.items; vitalTotal.value = r.total; }
  catch { vitals.value = []; }
}
async function addVital() {
  const v = parseFloat(vitalForm.value.value);
  if (Number.isNaN(v)) { $q.notify({ type: "warning", message: "값을 입력하세요." }); return; }
  savingVital.value = true;
  try {
    await server.createVital({ resident_id: id.value, kind: vitalForm.value.kind, value: v, note: vitalForm.value.note.trim() || null });
    $q.notify({ type: "positive", message: "활력징후가 저장되었습니다." });
    vitalForm.value = { kind: vitalForm.value.kind, value: "", note: "" };
    vitalPage.value = 1;
    await loadVitals();
  } catch (e: any) { $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` }); }
  finally { savingVital.value = false; }
}
function deleteVital(v: any) {
  $q.dialog({ title: "기록 삭제", message: "이 활력징후 기록을 삭제할까요?", cancel: { label: "취소", flat: true }, ok: { label: "삭제", color: "negative", unelevated: true }, persistent: true })
    .onOk(async () => {
      try { await server.deleteVital(v.id); if (vitals.value.length === 1 && vitalPage.value > 1) vitalPage.value--; await loadVitals(); $q.notify({ type: "positive", message: "삭제되었습니다." }); }
      catch (e: any) { $q.notify({ type: "negative", message: `삭제 실패: ${e?.message ?? e}` }); }
    });
}

// ── 투약 ────────────────────────────────────────────────────────────────────
const meds = ref<any[]>([]);
const medPage = ref(1); const medTotal = ref(0);
const medMax = computed(() => Math.max(1, Math.ceil(medTotal.value / PAGE)));
const showMedAdd = ref(false);
const medForm = ref({ name: "", dosage: "", frequency: "", start_date: new Date().toISOString().slice(0, 10) });
const savingMed = ref(false);
async function loadMeds() {
  try { const r = await server.medicationsPaged(id.value, medPage.value, PAGE); meds.value = r.items; medTotal.value = r.total; }
  catch { meds.value = []; }
}
async function addMed() {
  if (!medForm.value.name.trim()) { $q.notify({ type: "warning", message: "약 이름을 입력하세요." }); return; }
  savingMed.value = true;
  try {
    await server.createMedication({
      resident_id: id.value, name: medForm.value.name.trim(), dosage: medForm.value.dosage.trim(),
      frequency: medForm.value.frequency.trim(), start_date: medForm.value.start_date,
    });
    $q.notify({ type: "positive", message: "투약 처방이 추가되었습니다." });
    showMedAdd.value = false;
    medForm.value = { name: "", dosage: "", frequency: "", start_date: new Date().toISOString().slice(0, 10) };
    medPage.value = 1;
    await loadMeds();
  } catch (e: any) { $q.notify({ type: "negative", message: `추가 실패: ${e?.message ?? e}` }); }
  finally { savingMed.value = false; }
}
function confirmDeleteMed(m: any) {
  $q.dialog({
    title: "처방 삭제", message: `'${m.name}' 처방을 삭제할까요? 투약 기록도 함께 삭제됩니다.`,
    cancel: { label: "취소", flat: true }, ok: { label: "삭제", color: "negative", unelevated: true }, persistent: true,
  }).onOk(async () => {
    try { await server.deleteMedication(m.id); if (meds.value.length === 1 && medPage.value > 1) medPage.value--; await loadMeds(); $q.notify({ type: "positive", message: "삭제되었습니다." }); }
    catch (e: any) { $q.notify({ type: "negative", message: `삭제 실패: ${e?.message ?? e}` }); }
  });
}

// ── 사진 ────────────────────────────────────────────────────────────────────
const photos = ref<Array<{ id: string; taken_at: string; caption: string | null; status: string; data_url: string }>>([]);
const uploading = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);
const statusKo: Record<string, string> = { pending: "대기", approved: "승인", rejected: "반려" };
const statusColor: Record<string, string> = { pending: "orange", approved: "positive", rejected: "negative" };
const photoPage = ref(1); const photoTotal = ref(0); const PHOTO_PAGE = 12;
const photoMax = computed(() => Math.max(1, Math.ceil(photoTotal.value / PHOTO_PAGE)));
async function loadPhotos() {
  try { const r = await server.residentPhotos(id.value, photoPage.value, PHOTO_PAGE); photos.value = r.items; photoTotal.value = r.total; }
  catch { photos.value = []; }
}
function deletePhoto(p: any) {
  $q.dialog({ title: "사진 삭제", message: "이 사진을 삭제할까요?", cancel: { label: "취소", flat: true }, ok: { label: "삭제", color: "negative", unelevated: true }, persistent: true })
    .onOk(async () => {
      try { await server.deletePhoto(p.id); if (photos.value.length === 1 && photoPage.value > 1) photoPage.value--; await loadPhotos(); $q.notify({ type: "positive", message: "삭제되었습니다." }); }
      catch (e: any) { $q.notify({ type: "negative", message: `삭제 실패: ${e?.message ?? e}` }); }
    });
}
// 사진 확대 보기
const showImg = ref(false);
const imgSrc = ref("");
function openImg(src: string) { imgSrc.value = src; showImg.value = true; }
function pickPhoto() { fileInput.value?.click(); }
async function onFile(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (!f) return;
  uploading.value = true;
  try {
    await server.uploadPhoto(id.value, f);
    $q.notify({ type: "positive", message: "사진이 업로드되었습니다." });
    await loadPhotos();
  } catch (e2: any) {
    $q.notify({ type: "negative", message: `업로드 실패: ${e2?.message ?? e2}` });
  } finally {
    uploading.value = false;
    if (fileInput.value) fileInput.value.value = "";
  }
}

onMounted(async () => {
  await loadResident();
  await Promise.all([loadCare(), loadVitals(), loadMeds(), loadPhotos()]);
});
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Back + header -->
    <q-btn flat dense icon="o_arrow_back" label="어르신 목록" class="q-mb-sm text-grey-7" @click="router.push('/residents')" />

    <q-card flat bordered class="q-mb-md">
      <q-card-section class="row items-center q-gutter-md">
        <q-avatar color="primary" text-color="white" icon="o_elderly" size="48px" />
        <div>
          <div class="text-h6 text-weight-bold">{{ resident?.full_name ?? "…" }}</div>
          <div class="text-caption text-grey-7">
            {{ sexLabel[resident?.sex ?? ""] }} · {{ age(resident?.birth_date) }}
            · {{ gradeLabel(resident?.care_grade ?? null) }}
            · {{ resident?.room_number ? resident.room_number + "호" : "호실 미배정" }}
            · 입소 {{ resident?.admitted_on ?? "—" }}
          </div>
        </div>
        <q-space />
        <q-badge v-if="resident" :color="resident.status === 'active' ? 'primary' : 'grey'" :label="statusLabel[resident.status]" class="q-mr-sm" />
        <q-btn v-if="canEdit && resident" outline color="primary" icon="o_edit" label="수정" @click="openEdit" />
        <q-btn v-if="canDelete && resident && resident.status === 'active'" flat round dense icon="o_more_vert">
          <q-menu>
            <q-list style="min-width: 140px">
              <q-item clickable v-close-popup @click="confirmDischarge"><q-item-section>퇴소 처리</q-item-section></q-item>
              <q-item clickable v-close-popup @click="confirmDecease"><q-item-section>사망 처리</q-item-section></q-item>
            </q-list>
          </q-menu>
        </q-btn>
      </q-card-section>
    </q-card>

    <!-- Edit resident dialog -->
    <q-dialog v-model="showEdit" persistent>
      <q-card style="min-width: 480px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">어르신 정보 수정</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section>
          <q-form ref="editRef" class="q-gutter-sm">
            <q-input v-model="editForm.full_name" label="성함 *" outlined dense :rules="[(v)=>!!v?.trim()||'성함을 입력하세요']" lazy-rules="ondemand" />
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="editForm.sex" :options="sexOptions" label="성별" outlined dense emit-value map-options />
              <q-input class="col" v-model="editForm.birth_date" label="생년월일" mask="####-##-##" outlined dense />
            </div>
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="editForm.care_grade" :options="gradeOptions" label="장기요양등급" outlined dense emit-value map-options clearable />
              <q-input class="col" v-model="editForm.room_number" label="호실" outlined dense />
            </div>
            <q-input v-model="editForm.admitted_on" label="입소일" mask="####-##-##" outlined dense />
          </q-form>
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="저장" unelevated :loading="savingEdit" @click="submitEdit" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <q-tabs v-model="tab" align="left" active-color="primary" indicator-color="primary" class="q-mb-sm">
      <q-tab name="care" icon="o_assignment" label="케어 기록" />
      <q-tab name="vitals" icon="o_monitor_heart" label="활력징후" />
      <q-tab name="meds" icon="o_medication" label="투약" />
      <q-tab name="photos" icon="o_photo_camera" label="사진" />
    </q-tabs>

    <!-- 케어 기록 -->
    <div v-show="tab === 'care'">
      <q-card flat bordered class="q-pa-md q-mb-md">
        <div class="row q-col-gutter-sm items-start">
          <q-select class="col-12 col-sm-3" v-model="careForm.category" :options="CARE_CATEGORIES" label="구분" outlined dense emit-value map-options />
          <q-input class="col" v-model="careForm.body" label="내용" outlined dense type="textarea" autogrow />
          <div class="col-auto">
            <q-btn color="primary" label="저장" unelevated :loading="savingCare" @click="addCare" />
          </div>
        </div>
      </q-card>
      <q-list bordered separator>
        <q-item v-for="c in careLogs" :key="c.id">
          <q-item-section>
            <q-item-label>
              <q-badge color="blue-grey-1" text-color="blue-grey-9" :label="careCatKo[c.category] ?? c.category" class="q-mr-sm" />
              <q-icon v-if="c.flagged" name="o_flag" color="negative" size="16px" />
              {{ c.body }}
            </q-item-label>
          </q-item-section>
          <q-item-section side class="row items-center no-wrap">
            <span class="text-grey-7 q-mr-sm">{{ fmt(c.recorded_at) }}</span>
            <q-btn v-if="canDelete" flat round dense size="sm" icon="o_delete" color="grey-6" @click="deleteCare(c)" />
          </q-item-section>
        </q-item>
        <q-item v-if="!careLogs.length"><q-item-section class="text-grey-5 text-center q-py-md">기록 없음</q-item-section></q-item>
      </q-list>
      <div v-if="careMax > 1" class="row justify-center q-mt-md">
        <q-pagination v-model="carePage" :max="careMax" :max-pages="7" boundary-numbers direction-links @update:model-value="loadCare" />
      </div>
    </div>

    <!-- 활력징후 -->
    <div v-show="tab === 'vitals'">
      <q-card flat bordered class="q-pa-md q-mb-md">
        <div class="row q-col-gutter-sm items-center">
          <q-select class="col-12 col-sm-3" v-model="vitalForm.kind" :options="VITAL_KINDS" label="항목" outlined dense emit-value map-options />
          <q-input class="col-6 col-sm-2" v-model="vitalForm.value" label="값" type="number" outlined dense />
          <q-input class="col" v-model="vitalForm.note" label="메모" outlined dense />
          <div class="col-auto">
            <q-btn color="primary" label="저장" unelevated :loading="savingVital" @click="addVital" />
          </div>
        </div>
      </q-card>
      <q-list bordered separator>
        <q-item v-for="v in vitals" :key="v.id">
          <q-item-section>
            <q-item-label><span class="text-weight-medium">{{ vitalKindKo[v.kind] ?? v.kind }}</span> · {{ v.value }}</q-item-label>
            <q-item-label caption v-if="v.note">{{ v.note }}</q-item-label>
          </q-item-section>
          <q-item-section side class="row items-center no-wrap">
            <span class="text-grey-7 q-mr-sm">{{ fmt(v.recorded_at) }}</span>
            <q-btn v-if="canDelete" flat round dense size="sm" icon="o_delete" color="grey-6" @click="deleteVital(v)" />
          </q-item-section>
        </q-item>
        <q-item v-if="!vitals.length"><q-item-section class="text-grey-5 text-center q-py-md">기록 없음</q-item-section></q-item>
      </q-list>
      <div v-if="vitalMax > 1" class="row justify-center q-mt-md">
        <q-pagination v-model="vitalPage" :max="vitalMax" :max-pages="7" boundary-numbers direction-links @update:model-value="loadVitals" />
      </div>
    </div>

    <!-- 투약 -->
    <div v-show="tab === 'meds'">
      <div class="row q-mb-sm">
        <q-space />
        <q-btn color="primary" outline icon="o_add" label="처방 추가" @click="showMedAdd = true" />
      </div>
      <q-list bordered separator>
        <q-item v-for="m in meds" :key="m.id">
          <q-item-section>
            <q-item-label class="text-weight-medium">{{ m.name }}</q-item-label>
            <q-item-label caption>{{ m.dosage }} · {{ m.frequency }}<span v-if="m.is_active === false"> · 중단됨</span></q-item-label>
          </q-item-section>
          <q-item-section side v-if="canDelete">
            <q-btn flat round dense icon="o_delete" color="grey-6" @click="confirmDeleteMed(m)" />
          </q-item-section>
        </q-item>
        <q-item v-if="!meds.length"><q-item-section class="text-grey-5 text-center q-py-md">처방 없음</q-item-section></q-item>
      </q-list>
      <div v-if="medMax > 1" class="row justify-center q-mt-md">
        <q-pagination v-model="medPage" :max="medMax" :max-pages="7" boundary-numbers direction-links @update:model-value="loadMeds" />
      </div>
    </div>

    <!-- 사진 -->
    <div v-show="tab === 'photos'">
      <div class="row q-mb-sm items-center">
        <q-space />
        <q-btn v-if="canCreate" color="primary" outline icon="o_upload" label="사진 업로드" :loading="uploading" @click="pickPhoto" />
        <input ref="fileInput" type="file" accept="image/*" class="hidden" @change="onFile" />
      </div>
      <div v-if="photos.length" class="row q-col-gutter-sm">
        <div v-for="p in photos" :key="p.id" class="col-6 col-sm-4 col-md-3">
          <q-card flat bordered>
            <q-img :src="p.data_url" :ratio="1" class="cursor-pointer" @click="openImg(p.data_url)" />
            <q-card-section class="q-pa-xs row items-center">
              <q-badge :color="statusColor[p.status] ?? 'grey'" :label="statusKo[p.status] ?? p.status" />
              <q-space />
              <span class="text-caption text-grey-6">{{ fmt(p.taken_at) }}</span>
              <q-btn v-if="canDelete" flat round dense size="sm" icon="o_delete" color="grey-6" class="q-ml-xs" @click="deletePhoto(p)" />
            </q-card-section>
            <q-card-section v-if="p.caption" class="q-pa-xs q-pt-none text-caption">{{ p.caption }}</q-card-section>
          </q-card>
        </div>
      </div>
      <div v-if="photoMax > 1" class="row justify-center q-mt-md">
        <q-pagination v-model="photoPage" :max="photoMax" :max-pages="7" boundary-numbers direction-links @update:model-value="loadPhotos" />
      </div>
      <div v-else class="column flex-center q-py-xl text-grey-5">
        <q-icon name="o_photo_library" size="3rem" color="grey-4" />
        <div class="q-mt-sm">사진 없음</div>
      </div>
    </div>

    <!-- 사진 확대 -->
    <q-dialog v-model="showImg">
      <q-card flat class="bg-transparent shadow-0">
        <q-img :src="imgSrc" fit="contain" style="max-width: 92vw; max-height: 90vh" @click="showImg = false" class="cursor-pointer" />
      </q-card>
    </q-dialog>

    <!-- Add medication dialog -->
    <q-dialog v-model="showMedAdd" persistent>
      <q-card style="min-width: 420px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">투약 처방 추가</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-sm">
          <q-input v-model="medForm.name" label="약 이름 *" outlined dense />
          <div class="row q-gutter-sm">
            <q-input class="col" v-model="medForm.dosage" label="용량 (예: 500mg)" outlined dense />
            <q-input class="col" v-model="medForm.frequency" label="횟수 (예: 1일 2회)" outlined dense />
          </div>
          <q-input v-model="medForm.start_date" label="시작일" mask="####-##-##" outlined dense />
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="추가" unelevated :loading="savingMed" @click="addMed" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>
