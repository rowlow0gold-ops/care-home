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

interface Medication {
  id: number;
  resident_id: number;
  resident_name: string;
  name: string;
  dosage: string;
  frequency: string;
  route: string;
  start_date: string;
  end_date: string | null;
  prescriber: string;
  instructions: string;
  is_active: boolean;
}

const residents = ref<Resident[]>([]);
const medications = ref<Medication[]>([]);
const selectedResident = ref<number | null>(null);
const loading = ref(false);
const pageTab = ref<"active" | "history">("active");
const showAddDialog = ref(false);
const submitting = ref(false);
const addMedFormRef = ref();

const pagination = ref({
  page:        1,
  rowsPerPage: 25,
  rowsNumber:  0,
  sortBy:      "",
  descending:  false,
});

// Autocomplete filter state
const residentFilter = ref<{ label: string; value: number } | null>(null);
const filteredResidentOptions = ref<{ label: string; value: number }[]>([]);

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
  loadMedications();
}

function localToday(): string {
  const d = new Date();
  const y  = d.getFullYear();
  const m  = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

const form = ref({
  resident_id: null as number | null,
  name: "",
  dosage: "",
  frequency: "",
  route: "oral",
  start_date: localToday(),
  end_date: "" as string,
  prescriber: "",
  instructions: "",
});

const routeOptions = [
  { label: "Oral", value: "oral" },
  { label: "Injection", value: "injection" },
  { label: "Topical", value: "topical" },
  { label: "Inhaled", value: "inhaled" },
];

const residentOptions = computed(() =>
  residents.value.map((r) => ({ label: `${r.first_name} ${r.last_name}`, value: r.id }))
);

// medications.value is already the current page from the server (active-only filtered server-side)

const columns = [
  { name: "resident_name", label: "Resident",   field: "resident_name", align: "left" as const },
  { name: "name",          label: "Medication", field: "name",          align: "left" as const },
  { name: "dosage",        label: "Dosage",     field: "dosage",        align: "left" as const },
  { name: "frequency",     label: "Frequency",  field: "frequency",     align: "left" as const },
  { name: "route",         label: "Route",      field: "route",         align: "left" as const },
  { name: "start_date",    label: "Start Date", field: "start_date",    align: "left" as const },
  { name: "end_date",      label: "End Date",   field: "end_date",      align: "left" as const },
  { name: "prescriber",    label: "Prescriber", field: "prescriber",    align: "left" as const },
  { name: "actions",       label: "",           field: "actions",       align: "center" as const },
];

async function loadResidents() {
  try {
    residents.value = await invoke<Resident[]>("list_residents", { search: "", activeOnly: true });
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load residents: ${e}` });
  }
}

async function loadMedications() {
  loading.value = true;
  try {
    // true = active, false = history (stopped), null = all
    const activeOnly = pageTab.value === "active" ? true : false;
    const result = await invoke<{ data: Medication[]; total: number }>("list_medications", {
      residentId: selectedResident.value ?? null,
      activeOnly,
      page:       pagination.value.page,
      pageSize:   pagination.value.rowsPerPage,
      sortBy:     pagination.value.sortBy || null,
      sortDesc:   pagination.value.descending,
    });
    medications.value           = result.data;
    pagination.value.rowsNumber = result.total;
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load medications: ${e}` });
  } finally {
    loading.value = false;
  }
}

function onRequest(props: { pagination: { page: number; rowsPerPage: number; sortBy: string; descending: boolean } }) {
  pagination.value.page        = props.pagination.page;
  pagination.value.rowsPerPage = props.pagination.rowsPerPage;
  pagination.value.sortBy      = props.pagination.sortBy;
  pagination.value.descending  = props.pagination.descending;
  loadMedications();
}

function onRowClick(_evt: Event, row: Medication) {
  if (selectedResident.value === row.resident_id) return;
  const opt = residentOptions.value.find(r => r.value === row.resident_id) ?? null;
  residentFilter.value     = opt;
  selectedResident.value   = row.resident_id;
  pagination.value.page    = 1;
  loadMedications();
}

async function submitMedication() {
  const valid = await addMedFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await invoke("create_medication", {
      input: {
        resident_id: form.value.resident_id,
        name: form.value.name,
        dosage: form.value.dosage,
        frequency: form.value.frequency,
        route: form.value.route,
        start_date: form.value.start_date,
        end_date: form.value.end_date || null,
        prescriber: form.value.prescriber,
        instructions: form.value.instructions,
      },
    });
    $q.notify({ type: "positive", message: "Medication added." });
    showAddDialog.value = false;
    form.value = { resident_id: null, name: "", dosage: "", frequency: "", route: "oral", start_date: localToday(), end_date: "", prescriber: "", instructions: "" };
    pageTab.value = "active";
    pagination.value.page = 1;
    await loadMedications();
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to save: ${e}` });
  } finally {
    submitting.value = false;
  }
}

function confirmStop(med: Medication) {
  $q.dialog({
    title: "Stop Medication",
    message: `Stop "${med.name}" for ${med.resident_name}? It will be moved to History.`,
    cancel: { label: "Cancel", flat: true },
    ok:     { label: "Stop", color: "negative", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    try {
      await invoke("stop_medication", { id: med.id });
      $q.notify({ type: "positive", message: "Medication stopped and moved to History." });
      // Reload without resetting page or sort — row just disappears from Active tab
      await loadMedications();
    } catch (e) {
      $q.notify({ type: "negative", message: `Failed: ${e}` });
    }
  });
}

function routeColor(route: string) {
  const map: Record<string, string> = { oral: "teal", injection: "deep-orange", topical: "blue", inhaled: "purple" };
  return map[route] || "grey";
}

const selectedResidentObj = computed(() =>
  residents.value.find(r => r.id === selectedResident.value) ?? null
);

onMounted(async () => {
  // Staff can never see history
  if (isStaff.value && pageTab.value === "history") pageTab.value = "active";
  await loadResidents();
  const rid = route.query.resident ? Number(route.query.resident) : null;
  if (rid) {
    selectedResident.value = rid;
    const opt = residentOptions.value.find(r => r.value === rid) ?? null;
    residentFilter.value   = opt;
    form.value.resident_id = rid;
  }
  await loadMedications();
});
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-md">
      <div class="col">
        <div class="text-h5 text-weight-bold">Medications</div>
        <div class="text-caption text-grey-6">Active medication management</div>
      </div>
      <div v-if="!isStaff" class="col-auto">
        <q-btn
          color="primary" icon="o_add" label="Add Medication" unelevated
          @click="() => { form.resident_id = selectedResident; showAddDialog = true; }"
        />
      </div>
    </div>

    <!-- Search + stay info row -->
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
      <!-- Stay period chip when a resident is selected -->
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
      @update:model-value="() => { pagination.page = 1; loadMedications(); }"
    >
      <q-tab name="active"  icon="o_medication"  label="Active" />
      <q-tab v-if="!isStaff" name="history" icon="o_history" label="History" />
    </q-tabs>

    <!-- Skeleton loading -->
    <template v-if="loading">
      <q-skeleton type="rect" height="40px" class="q-mb-sm" />
      <q-skeleton type="rect" height="44px" class="q-mb-sm" v-for="n in 6" :key="n" />
    </template>

    <!-- Table — click any row to filter by that resident -->
    <q-table
      v-else
      :rows="medications"
      :columns="columns"
      row-key="id"
      flat bordered
      v-model:pagination="pagination"
      :rows-per-page-options="[10, 25, 50, 100]"
      @request="onRequest"
      @row-click="onRowClick"
      :class="!selectedResident ? 'cursor-pointer-rows' : ''"
    >
      <template #body-cell-route="props">
        <q-td :props="props">
          <q-badge :color="routeColor(props.row.route)" :label="props.row.route" class="text-capitalize" />
        </q-td>
      </template>

      <template #body-cell-end_date="props">
        <q-td :props="props">
          <span v-if="props.row.end_date" class="text-caption">{{ props.row.end_date }}</span>
          <span v-else class="text-grey-4">—</span>
        </q-td>
      </template>

      <template #body-cell-actions="props">
        <q-td :props="props" class="text-center">
          <template v-if="!isStaff && pageTab === 'active'">
            <q-btn flat round dense icon="o_stop_circle" color="negative" @click="confirmStop(props.row)">
              <q-tooltip>Stop medication</q-tooltip>
            </q-btn>
          </template>
          <q-badge v-else-if="pageTab === 'history'" color="grey-4" label="Stopped" />
          <span v-else class="text-grey-4">—</span>
        </q-td>
      </template>

      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_medication_liquid" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">
            {{ selectedResident ? "No active medications for this resident" : "No active medications" }}
          </div>
          <div class="text-grey-4 text-caption" v-if="!isStaff">Click "Add Medication" to add one</div>
        </div>
      </template>
    </q-table>

    <!-- Add Dialog -->
    <q-dialog v-model="showAddDialog" persistent>
      <q-card style="min-width: 520px">
        <q-card-section class="row items-center q-pb-none">
          <div>
            <div class="text-h6">Add Medication</div>
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

        <q-form ref="addMedFormRef" class="q-gutter-sm">
          <q-select
            v-model="form.resident_id"
            :options="residentOptions"
            :label="residentFilter?.label ? `Resident: ${residentFilter.label}` : 'Resident *'"
            outlined
            dense
            emit-value
            map-options
            :disable="!!selectedResident"
            :rules="[v => v !== null && v !== undefined || 'Please select a resident']"
            lazy-rules="ondemand"
          />
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="form.name" label="Medication Name *" outlined dense
                :rules="[v => !!v?.trim() || 'Medication name is required']" lazy-rules="ondemand" />
            </div>
            <div class="col">
              <q-input v-model="form.dosage" label="Dosage *" outlined dense placeholder="e.g. 500mg"
                :rules="[v => !!v?.trim() || 'Dosage is required']" lazy-rules="ondemand" />
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="form.frequency" label="Frequency" outlined dense placeholder="e.g. Twice daily" />
            </div>
            <div class="col">
              <q-select
                v-model="form.route"
                :options="routeOptions"
                label="Route"
                outlined
                dense
                emit-value
                map-options
              />
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="form.start_date" label="Start Date" outlined dense readonly>
                <template #append>
                  <q-icon name="o_event" class="cursor-pointer" color="grey-6">
                    <q-popup-proxy transition-show="scale" transition-hide="scale">
                      <q-date v-model="form.start_date" mask="YYYY-MM-DD">
                        <div class="row items-center justify-end q-pa-sm">
                          <q-btn v-close-popup label="OK" color="primary" flat dense />
                        </div>
                      </q-date>
                    </q-popup-proxy>
                  </q-icon>
                </template>
              </q-input>
            </div>
            <div class="col">
              <q-input v-model="form.end_date" label="End Date (optional)" outlined dense readonly>
                <template #append>
                  <q-icon v-if="form.end_date" name="o_close" class="cursor-pointer" color="grey-5"
                          @click.stop="form.end_date = ''" />
                  <q-icon name="o_event" class="cursor-pointer" color="grey-6">
                    <q-popup-proxy transition-show="scale" transition-hide="scale">
                      <q-date v-model="form.end_date" mask="YYYY-MM-DD">
                        <div class="row items-center justify-end q-pa-sm">
                          <q-btn v-close-popup label="OK" color="primary" flat dense />
                          <q-btn v-close-popup label="Clear" flat dense color="grey" @click="form.end_date = ''" />
                        </div>
                      </q-date>
                    </q-popup-proxy>
                  </q-icon>
                </template>
              </q-input>
            </div>
          </div>
          <q-input v-model="form.prescriber" label="Prescriber" outlined dense />
          <q-input v-model="form.instructions" label="Instructions" type="textarea" outlined dense rows="2" autogrow />
        </q-form>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Save" unelevated :loading="submitting" @click="submitMedication" />
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
