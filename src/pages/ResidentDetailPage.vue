<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type Resident } from "@/lib/server";

const route = useRoute();
const router = useRouter();
const $q = useQuasar();
const id = computed(() => String(route.params.id));

const resident = ref<Resident | null>(null);
const tab = ref<"care" | "vitals" | "meds">("care");

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

// ── 케어 기록 ───────────────────────────────────────────────────────────────
const CARE_CATEGORIES = [
  { label: "식사", value: "meal" }, { label: "투약", value: "medication" },
  { label: "위생", value: "hygiene" }, { label: "이동", value: "mobility" },
  { label: "정서", value: "mood" }, { label: "특이사항", value: "incident" },
  { label: "기타", value: "other" },
];
const careCatKo = Object.fromEntries(CARE_CATEGORIES.map((c) => [c.value, c.label]));
const careLogs = ref<any[]>([]);
const careForm = ref({ category: "meal", body: "", flagged: false });
const savingCare = ref(false);
async function loadCare() { careLogs.value = await server.careLogsFor(id.value).catch(() => []) as any[]; }
async function addCare() {
  if (!careForm.value.body.trim()) { $q.notify({ type: "warning", message: "내용을 입력하세요." }); return; }
  savingCare.value = true;
  try {
    await server.createCareLog({ resident_id: id.value, category: careForm.value.category, body: careForm.value.body.trim(), flagged: careForm.value.flagged });
    $q.notify({ type: "positive", message: "케어 기록이 저장되었습니다." });
    careForm.value = { category: "meal", body: "", flagged: false };
    await loadCare();
  } catch (e: any) { $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` }); }
  finally { savingCare.value = false; }
}

// ── 활력징후 ────────────────────────────────────────────────────────────────
const VITAL_KINDS = [
  { label: "체온(℃)", value: "temperature_celsius" }, { label: "맥박(bpm)", value: "heart_rate" },
  { label: "수축기혈압", value: "blood_pressure_systolic" }, { label: "이완기혈압", value: "blood_pressure_diastolic" },
  { label: "산소포화도(%)", value: "spo2" }, { label: "혈당", value: "blood_glucose" },
];
const vitalKindKo = Object.fromEntries(VITAL_KINDS.map((k) => [k.value, k.label]));
const vitals = ref<any[]>([]);
const vitalForm = ref({ kind: "temperature_celsius", value: "", note: "" });
const savingVital = ref(false);
async function loadVitals() { vitals.value = await server.vitalsFor(id.value).catch(() => []) as any[]; }
async function addVital() {
  const v = parseFloat(vitalForm.value.value);
  if (Number.isNaN(v)) { $q.notify({ type: "warning", message: "값을 입력하세요." }); return; }
  savingVital.value = true;
  try {
    await server.createVital({ resident_id: id.value, kind: vitalForm.value.kind, value: v, note: vitalForm.value.note.trim() || null });
    $q.notify({ type: "positive", message: "활력징후가 저장되었습니다." });
    vitalForm.value = { kind: vitalForm.value.kind, value: "", note: "" };
    await loadVitals();
  } catch (e: any) { $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` }); }
  finally { savingVital.value = false; }
}

// ── 투약 ────────────────────────────────────────────────────────────────────
const meds = ref<any[]>([]);
const showMedAdd = ref(false);
const medForm = ref({ name: "", dosage: "", frequency: "", start_date: new Date().toISOString().slice(0, 10) });
const savingMed = ref(false);
const acting = ref<string | null>(null);
async function loadMeds() { meds.value = await server.medsFor(id.value).catch(() => []) as any[]; }
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
    await loadMeds();
  } catch (e: any) { $q.notify({ type: "negative", message: `추가 실패: ${e?.message ?? e}` }); }
  finally { savingMed.value = false; }
}
async function administer(m: any) {
  acting.value = m.id;
  try {
    await server.administerMedication(m.id);
    $q.notify({ type: "positive", message: `${m.name} 투약 기록됨.` });
  } catch (e: any) { $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` }); }
  finally { acting.value = null; }
}

onMounted(async () => {
  await loadResident();
  await Promise.all([loadCare(), loadVitals(), loadMeds()]);
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
        <q-badge v-if="resident" :color="resident.status === 'active' ? 'primary' : 'grey'" :label="statusLabel[resident.status]" />
      </q-card-section>
    </q-card>

    <q-tabs v-model="tab" align="left" active-color="primary" indicator-color="primary" class="q-mb-sm">
      <q-tab name="care" icon="o_assignment" label="케어 기록" />
      <q-tab name="vitals" icon="o_monitor_heart" label="활력징후" />
      <q-tab name="meds" icon="o_medication" label="투약" />
    </q-tabs>

    <!-- 케어 기록 -->
    <div v-show="tab === 'care'">
      <q-card flat bordered class="q-pa-md q-mb-md">
        <div class="row q-col-gutter-sm items-start">
          <q-select class="col-12 col-sm-3" v-model="careForm.category" :options="CARE_CATEGORIES" label="구분" outlined dense emit-value map-options />
          <q-input class="col" v-model="careForm.body" label="내용" outlined dense type="textarea" autogrow />
          <div class="col-auto column q-gutter-xs">
            <q-toggle v-model="careForm.flagged" label="관리자 보고" color="negative" dense />
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
          <q-item-section side>{{ fmt(c.recorded_at) }}</q-item-section>
        </q-item>
        <q-item v-if="!careLogs.length"><q-item-section class="text-grey-5 text-center q-py-md">기록 없음</q-item-section></q-item>
      </q-list>
    </div>

    <!-- 활력징후 -->
    <div v-show="tab === 'vitals'">
      <q-card flat bordered class="q-pa-md q-mb-md">
        <div class="row q-col-gutter-sm items-start">
          <q-select class="col-12 col-sm-3" v-model="vitalForm.kind" :options="VITAL_KINDS" label="항목" outlined dense emit-value map-options />
          <q-input class="col-6 col-sm-2" v-model="vitalForm.value" label="값" type="number" outlined dense />
          <q-input class="col" v-model="vitalForm.note" label="메모" outlined dense />
          <q-btn class="col-auto" color="primary" label="저장" unelevated :loading="savingVital" @click="addVital" />
        </div>
      </q-card>
      <q-list bordered separator>
        <q-item v-for="v in vitals" :key="v.id">
          <q-item-section>
            <q-item-label><span class="text-weight-medium">{{ vitalKindKo[v.kind] ?? v.kind }}</span> · {{ v.value }}</q-item-label>
            <q-item-label caption v-if="v.note">{{ v.note }}</q-item-label>
          </q-item-section>
          <q-item-section side>{{ fmt(v.recorded_at) }}</q-item-section>
        </q-item>
        <q-item v-if="!vitals.length"><q-item-section class="text-grey-5 text-center q-py-md">기록 없음</q-item-section></q-item>
      </q-list>
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
            <q-item-label caption>{{ m.dosage }} · {{ m.frequency }}<span v-if="m.status === 'stopped'"> · 중단됨</span></q-item-label>
          </q-item-section>
          <q-item-section side>
            <q-btn dense unelevated color="primary" label="투약 기록" :loading="acting === m.id" :disable="m.status === 'stopped'" @click="administer(m)" />
          </q-item-section>
        </q-item>
        <q-item v-if="!meds.length"><q-item-section class="text-grey-5 text-center q-py-md">처방 없음</q-item-section></q-item>
      </q-list>
    </div>

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
