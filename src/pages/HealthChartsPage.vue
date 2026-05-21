<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { useAuthStore } from "@/stores/auth";
import { useRoute } from "vue-router";

const $q    = useQuasar();
const auth  = useAuthStore();
const route = useRoute();
const isStaff = computed(() => auth.user?.role === "staff");

// ── Types ─────────────────────────────────────────────────────────────────────
interface Resident {
  id: number;
  first_name: string;
  last_name: string;
  room_number: string | null;
  date_of_birth: string;
  gender: string;
  admission_date: string;
  discharge_date: string | null;
}

interface Vital {
  id: number;
  resident_id: number;
  resident_name: string;
  staff_name: string | null;
  bp_systolic: number | null;
  bp_diastolic: number | null;
  heart_rate: number | null;
  temperature: number | null;
  weight: number | null;
  blood_sugar: number | null;
  spo2: number | null;
  notes: string | null;
  measured_at: string;
}

// ── State ─────────────────────────────────────────────────────────────────────
const residents        = ref<Resident[]>([]);
const vitals           = ref<Vital[]>([]);
const selectedResident = ref<number | null>(null);
const loading          = ref(false);
const showDialog       = ref(false);
const submitting       = ref(false);
const pageTab          = ref<"active" | "history">("active");

const pagination = ref({
  page:        1,
  rowsPerPage: 25,
  rowsNumber:  0,
  sortBy:      "",
  descending:  true,   // newest first by default
});

// Autocomplete filter
const residentFilter          = ref<{ label: string; value: number } | null>(null);
const filteredResidentOptions = ref<{ label: string; value: number }[]>([]);

const residentOptions = computed(() =>
  residents.value.map(r => ({ label: `${r.first_name} ${r.last_name}`, value: r.id }))
);

const selectedResidentObj = computed(() =>
  residents.value.find(r => r.id === selectedResident.value) ?? null
);

// ── Record form ───────────────────────────────────────────────────────────────
const form = ref({
  resident_id:  null as number | null,
  bp_systolic:  "",
  bp_diastolic: "",
  heart_rate:   "",
  temperature:  "",
  weight:       "",
  blood_sugar:  "",
  spo2:         "",
  notes:        "",
});

// ── Age helper ────────────────────────────────────────────────────────────────
function calcAge(dob: string): number {
  const birth = new Date(dob + "T00:00:00");
  const now   = new Date();
  let age = now.getFullYear() - birth.getFullYear();
  const m = now.getMonth() - birth.getMonth();
  if (m < 0 || (m === 0 && now.getDate() < birth.getDate())) age--;
  return age;
}

function genderLabel(g: string): string {
  const map: Record<string, string> = { male: "M", female: "F", other: "Other" };
  return map[g?.toLowerCase()] ?? g;
}

// ── Stay-period helper ────────────────────────────────────────────────────────
function stayDuration(admissionDate: string, dischargeDate?: string | null): string {
  const start = new Date(admissionDate + "T00:00:00");
  const end   = dischargeDate ? new Date(dischargeDate + "T00:00:00") : new Date();
  if (isNaN(start.getTime())) return "";
  let years  = end.getFullYear() - start.getFullYear();
  let months = end.getMonth()    - start.getMonth();
  if (months < 0) { years--; months += 12; }
  const days = Math.floor((end.getTime() - start.getTime()) / 86_400_000);
  if (years > 0 && months > 0) return `${years}y ${months}m`;
  if (years > 0)  return `${years} year${years > 1 ? "s" : ""}`;
  if (months > 0) return `${months} month${months > 1 ? "s" : ""}`;
  return `${days} day${days !== 1 ? "s" : ""}`;
}

// ── Autocomplete ──────────────────────────────────────────────────────────────
function onResidentFilter(val: string, update: (fn: () => void) => void) {
  update(() => {
    const q = val.toLowerCase();
    filteredResidentOptions.value = residentOptions.value.filter(r =>
      r.label.toLowerCase().includes(q)
    );
  });
}

function onResidentSelected(opt: { label: string; value: number } | null) {
  selectedResident.value   = opt?.value ?? null;
  pagination.value.page    = 1;
  loadVitals();
}

// ── Table columns ─────────────────────────────────────────────────────────────
const columns = [
  { name: "resident_name", label: "Resident",    field: "resident_name", align: "left"   as const },
  { name: "measured_at",   label: "Date/Time",   field: "measured_at",   align: "left"   as const },
  { name: "bp",            label: "BP (mmHg)",   field: "bp",            align: "center" as const },
  { name: "heart_rate",    label: "HR (bpm)",    field: "heart_rate",    align: "center" as const },
  { name: "temperature",   label: "Temp (°C)",   field: "temperature",   align: "center" as const },
  { name: "weight",        label: "Weight (kg)", field: "weight",        align: "center" as const },
  { name: "spo2",          label: "SpO₂ (%)",    field: "spo2",          align: "center" as const },
  { name: "blood_sugar",   label: "Blood Sugar", field: "blood_sugar",   align: "center" as const },
  { name: "staff_name",    label: "Recorded by", field: "staff_name",    align: "left"   as const },
  { name: "actions",       label: "",            field: "actions",       align: "center" as const },
];

// ── Formatters ────────────────────────────────────────────────────────────────
function formatBP(v: Vital) {
  if (v.bp_systolic && v.bp_diastolic) return `${v.bp_systolic}/${v.bp_diastolic}`;
  if (v.bp_systolic) return `${v.bp_systolic}/—`;
  return "—";
}

function bpColor(v: Vital) {
  return v.bp_systolic && v.bp_systolic > 140 ? "text-negative" : "";
}

function spo2Color(v: Vital) {
  if (!v.spo2) return "";
  if (v.spo2 < 90) return "text-negative";
  if (v.spo2 < 94) return "text-warning";
  return "";
}

function formatDateTime(iso: string) {
  if (!iso) return "";
  return new Date(iso).toLocaleString("en-CA", {
    year: "numeric", month: "2-digit", day: "2-digit",
    hour: "2-digit", minute: "2-digit",
  });
}

// ── API ───────────────────────────────────────────────────────────────────────
async function loadResidents() {
  try {
    residents.value = await invoke<Resident[]>("list_residents", { search: "", activeOnly: true });
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load residents: ${e}` });
  }
}

async function loadVitals() {
  loading.value = true;
  try {
    const result = await invoke<{ data: Vital[]; total: number }>("list_vitals", {
      residentId:   selectedResident.value ?? null,
      showArchived: pageTab.value === "history",
      page:         pagination.value.page,
      pageSize:     pagination.value.rowsPerPage,
      sortBy:       pagination.value.sortBy || null,
      sortDesc:     pagination.value.descending,
    });
    vitals.value                 = result.data;
    pagination.value.rowsNumber  = result.total;
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load vitals: ${e}` });
  } finally {
    loading.value = false;
  }
}

function onRequest(props: { pagination: { page: number; rowsPerPage: number; sortBy: string; descending: boolean } }) {
  pagination.value.page        = props.pagination.page;
  pagination.value.rowsPerPage = props.pagination.rowsPerPage;
  pagination.value.sortBy      = props.pagination.sortBy;
  pagination.value.descending  = props.pagination.descending;
  loadVitals();
}

function onRowClick(_evt: Event, row: Vital) {
  // If already filtered to this resident, do nothing
  if (selectedResident.value === row.resident_id) return;
  const opt = residentOptions.value.find(r => r.value === row.resident_id) ?? null;
  residentFilter.value     = opt;
  selectedResident.value   = row.resident_id;
  pagination.value.page    = 1;
  loadVitals();
}

async function submitVital() {
  if (!form.value.resident_id) {
    $q.notify({ type: "negative", message: "Please select a resident." });
    return;
  }
  submitting.value = true;
  try {
    await invoke("create_vital", {
      input: {
        resident_id:  form.value.resident_id,
        bp_systolic:  form.value.bp_systolic  ? parseInt(form.value.bp_systolic)   : null,
        bp_diastolic: form.value.bp_diastolic ? parseInt(form.value.bp_diastolic)  : null,
        heart_rate:   form.value.heart_rate   ? parseInt(form.value.heart_rate)    : null,
        temperature:  form.value.temperature  ? parseFloat(form.value.temperature) : null,
        weight:       form.value.weight       ? parseFloat(form.value.weight)      : null,
        blood_sugar:  form.value.blood_sugar  ? parseFloat(form.value.blood_sugar) : null,
        spo2:         form.value.spo2         ? parseInt(form.value.spo2)          : null,
        notes:        form.value.notes || null,
      },
    });
    $q.notify({ type: "positive", message: "Vital signs recorded." });
    showDialog.value = false;
    form.value = { resident_id: selectedResident.value, bp_systolic: "", bp_diastolic: "",
                   heart_rate: "", temperature: "", weight: "", blood_sugar: "", spo2: "", notes: "" };
    pagination.value.page = 1;
    await loadVitals();
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to save: ${e}` });
  } finally {
    submitting.value = false;
  }
}

function confirmArchive(vital: Vital) {
  $q.dialog({
    title: "Archive this record?",
    message: `This vital record for ${vital.resident_name} (${formatDateTime(vital.measured_at)}) will be moved to History. It will not be deleted.`,
    cancel: { label: "Cancel", flat: true },
    ok:     { label: "Archive", color: "warning", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    try {
      await invoke("archive_vital", { id: vital.id });
      $q.notify({ type: "positive", message: "Vital record archived." });
      await loadVitals();
    } catch (e) {
      $q.notify({ type: "negative", message: `Failed: ${e}` });
    }
  });
}

onMounted(async () => {
  if (isStaff.value && pageTab.value === "history") pageTab.value = "active";
  await loadResidents();
  const rid = route.query.resident ? Number(route.query.resident) : null;
  if (rid) {
    selectedResident.value = rid;
    const opt = residentOptions.value.find(r => r.value === rid) ?? null;
    residentFilter.value   = opt;
    form.value.resident_id = rid;
  }
  await loadVitals();
});
</script>

<template>
  <q-page class="q-pa-lg">

    <!-- Header -->
    <div class="row items-center q-mb-md">
      <div class="col">
        <div class="text-h5 text-weight-bold">Health Charts</div>
        <div class="text-caption text-grey-6">Vital signs monitoring</div>
      </div>
      <div v-if="!isStaff && pageTab === 'active'" class="col-auto">
        <q-btn
          color="primary" icon="o_add" label="Record Vitals" unelevated
          @click="() => { form.resident_id = selectedResident; showDialog = true; }"
        />
      </div>
    </div>

    <!-- Search + stay period chip -->
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col-12 col-md-4">
        <q-select
          v-model="residentFilter"
          :options="filteredResidentOptions"
          label="Filter by resident…"
          outlined dense
          use-input input-debounce="150" clearable
          @filter="onResidentFilter"
          @update:model-value="onResidentSelected"
        >
          <template #prepend><q-icon name="o_search" /></template>
          <template #no-option>
            <q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item>
          </template>
        </q-select>
      </div>
      <div v-if="selectedResidentObj" class="col-auto">
        <q-chip icon="o_person" color="teal" text-color="white" dense>
          {{ selectedResidentObj.first_name }} {{ selectedResidentObj.last_name }}
          · {{ genderLabel(selectedResidentObj.gender) }}
          · Age {{ calcAge(selectedResidentObj.date_of_birth) }}
          · DOB {{ selectedResidentObj.date_of_birth }}
          · Admitted {{ selectedResidentObj.admission_date }}
          · {{ stayDuration(selectedResidentObj.admission_date, selectedResidentObj.discharge_date) }}
        </q-chip>
      </div>
    </div>

    <!-- Active / History tabs -->
    <q-tabs
      v-model="pageTab"
      align="left"
      indicator-color="primary"
      active-color="primary"
      class="q-mb-sm"
      @update:model-value="() => { pagination.page = 1; loadVitals(); }"
    >
      <q-tab name="active"  icon="o_monitor_heart" label="Active" />
      <q-tab v-if="!isStaff" name="history" icon="o_history" label="History" />
    </q-tabs>

    <!-- Skeleton loading -->
    <template v-if="loading">
      <q-skeleton type="rect" height="40px" class="q-mb-sm" />
      <q-skeleton type="rect" height="44px" class="q-mb-sm" v-for="n in 6" :key="n" />
    </template>

    <!-- Table -->
    <q-table
      v-else
      :rows="vitals"
      :columns="columns"
      row-key="id"
      flat bordered
      v-model:pagination="pagination"
      :rows-per-page-options="[10, 25, 50, 100]"
      @request="onRequest"
      @row-click="onRowClick"
      :class="!selectedResident ? 'cursor-pointer-rows' : ''"
    >
      <template #body-cell-resident_name="props">
        <q-td :props="props">
          <span class="text-weight-medium">{{ props.row.resident_name }}</span>
        </q-td>
      </template>

      <template #body-cell-measured_at="props">
        <q-td :props="props">{{ formatDateTime(props.row.measured_at) }}</q-td>
      </template>

      <template #body-cell-bp="props">
        <q-td :props="props">
          <span :class="bpColor(props.row)">{{ formatBP(props.row) }}</span>
          <q-icon v-if="(props.row.bp_systolic ?? 0) > 140" name="o_warning" color="negative" size="0.85rem" class="q-ml-xs" />
        </q-td>
      </template>

      <template #body-cell-heart_rate="props">
        <q-td :props="props">{{ props.row.heart_rate ?? "—" }}</q-td>
      </template>

      <template #body-cell-temperature="props">
        <q-td :props="props">{{ props.row.temperature ?? "—" }}</q-td>
      </template>

      <template #body-cell-weight="props">
        <q-td :props="props">{{ props.row.weight ?? "—" }}</q-td>
      </template>

      <template #body-cell-spo2="props">
        <q-td :props="props">
          <span :class="spo2Color(props.row)">
            {{ props.row.spo2 != null ? props.row.spo2 + "%" : "—" }}
          </span>
        </q-td>
      </template>

      <template #body-cell-blood_sugar="props">
        <q-td :props="props">{{ props.row.blood_sugar ?? "—" }}</q-td>
      </template>

      <template #body-cell-staff_name="props">
        <q-td :props="props">
          <span v-if="props.row.staff_name">{{ props.row.staff_name }}</span>
          <span v-else class="text-grey">—</span>
        </q-td>
      </template>

      <template #body-cell-actions="props">
        <q-td :props="props" class="text-center">
          <q-btn
            v-if="!isStaff && pageTab === 'active'"
            flat round dense icon="o_archive" color="warning" size="sm"
            @click.stop="confirmArchive(props.row)"
          >
            <q-tooltip>Archive record</q-tooltip>
          </q-btn>
          <q-badge v-else-if="pageTab === 'history'" color="grey-4" label="Archived" />
        </q-td>
      </template>

      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_monitor_heart" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">
            {{ pageTab === 'history' ? 'No archived records' : selectedResident ? 'No vital records for this resident' : 'No vital records yet' }}
          </div>
          <div class="text-grey-4 text-caption" v-if="!isStaff && pageTab === 'active'">Click "Record Vitals" to add the first entry</div>
        </div>
      </template>
    </q-table>

    <!-- Record Vitals Dialog -->
    <q-dialog v-model="showDialog" persistent>
      <q-card style="min-width: 500px">
        <q-card-section class="row items-center q-pb-none">
          <div>
            <div class="text-h6">Record Vital Signs</div>
            <div v-if="selectedResidentObj" class="text-caption text-grey-6 q-mt-xs">
              {{ selectedResidentObj.first_name }} {{ selectedResidentObj.last_name }}
              · {{ genderLabel(selectedResidentObj.gender) }}
              · Age {{ calcAge(selectedResidentObj.date_of_birth) }}
              · {{ stayDuration(selectedResidentObj.admission_date, selectedResidentObj.discharge_date) }} stay
            </div>
          </div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section class="q-gutter-sm">
          <q-select
            v-model="form.resident_id"
            :options="residentOptions"
            :label="residentFilter?.label ? `Resident: ${residentFilter.label}` : 'Resident *'"
            outlined dense
            emit-value map-options
            :disable="!!selectedResident"
          />
          <div class="text-subtitle2 text-grey-7">Blood Pressure</div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="form.bp_systolic"  label="Systolic (mmHg)"  type="number"            outlined dense />
            </div>
            <div class="col">
              <q-input v-model="form.bp_diastolic" label="Diastolic (mmHg)" type="number"            outlined dense />
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="form.heart_rate"   label="Heart Rate (bpm)" type="number"            outlined dense />
            </div>
            <div class="col">
              <q-input v-model="form.temperature"  label="Temperature (°C)" type="number" step="0.1" outlined dense />
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="form.weight"       label="Weight (kg)"      type="number" step="0.1" outlined dense />
            </div>
            <div class="col">
              <q-input v-model="form.spo2"         label="SpO₂ (%)"         type="number"            outlined dense />
            </div>
          </div>
          <q-input v-model="form.blood_sugar" label="Blood Sugar (mg/dL)" type="number" step="0.1" outlined dense />
          <q-input v-model="form.notes"       label="Notes" type="textarea" outlined dense rows="2" autogrow />
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Save" unelevated :loading="submitting" @click="submitVital" />
        </q-card-actions>
      </q-card>
    </q-dialog>

  </q-page>
</template>

<style scoped>
.cursor-pointer-rows :deep(tbody tr) {
  cursor: pointer;
}
.cursor-pointer-rows :deep(tbody tr:hover td) {
  background: #f0f9ff;
}
</style>
