<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { useAuthStore } from "@/stores/auth";
import { useSettingsStore, detectCurrentShift } from "@/stores/settings";
import { useRoute } from "vue-router";

const $q      = useQuasar();
const auth    = useAuthStore();
const settings = useSettingsStore();
const route   = useRoute();

// ── Types ────────────────────────────────────────────────────────────────────
interface Resident { id: number; first_name: string; last_name: string; }
interface CareLog {
  id: number;
  resident_id: number;
  resident_name: string;
  staff_id: number | null;
  staff_name: string | null;
  shift: string;
  category: string;
  content: string;
  is_incident: boolean;
  is_flagged: boolean;
  logged_at: string;
}

// ── Role helpers ─────────────────────────────────────────────────────────────
const isStaff   = computed(() => auth.user?.role === "staff");
const canDelete = computed(() => ["manager", "admin"].includes(auth.user?.role ?? ""));

function canEditLog(log: CareLog): boolean {
  if (!auth.user) return false;
  if (auth.user.role === "staff") return log.staff_id === auth.user.id;
  return true;
}

// ── Config ───────────────────────────────────────────────────────────────────
const shiftOptions = computed(() => settings.getShiftOptions());
const CATEGORY_OPTIONS = [
  { label: "Bathing",    value: "bathing"    },
  { label: "Meals",      value: "meals"      },
  { label: "Medication", value: "medication" },
  { label: "Mood",       value: "mood"       },
  { label: "Incident",   value: "incident"   },
  { label: "Note",       value: "note"       },
];
const SHIFT_COLOR: Record<string,string> = {
  morning:"orange", afternoon:"blue", day:"teal", night:"deep-purple", visit:"green",
};
const CATEGORY_COLOR: Record<string,string> = {
  bathing:"cyan", meals:"green", medication:"teal",
  mood:"purple",  incident:"red", note:"grey-7",
};

// ── Shared state ─────────────────────────────────────────────────────────────
const tab       = ref<"daily"|"history">("daily");
const residents = ref<Resident[]>([]);

const residentOptions = computed(() =>
  residents.value.map(r => ({ label: `${r.first_name} ${r.last_name}`, value: r.id }))
);

function detectShift(): string { return detectCurrentShift(settings.shiftModel); }

// ── DAILY TAB ────────────────────────────────────────────────────────────────
const today = new Date().toISOString().slice(0, 10);

function shiftDay(delta: number) {
  const d = new Date(dailyDate.value + "T12:00:00");
  d.setDate(d.getDate() + delta);
  dailyDate.value = d.toISOString().slice(0, 10);
  loadDaily();
}
const dailyDate       = ref(today);
const activeShift     = ref(detectShift());
const dailyLogs       = ref<CareLog[]>([]);
const dailyLoading    = ref(false);
const showEntryDialog = ref(false);
const newEntryFormRef = ref();

const form = ref({
  resident_id: null as number | null,
  shift:       activeShift.value,
  category:    "note",
  content:     "",
});
const submitting = ref(false);

// Edit form
const showEditDialog = ref(false);
const editFormRef    = ref();
const editingLog     = ref<CareLog | null>(null);
const editForm       = ref({ content: "" });
const editSubmitting = ref(false);

function openEdit(log: CareLog) {
  editingLog.value       = log;
  editForm.value.content = log.content;
  showEditDialog.value   = true;
}

async function submitEdit() {
  if (!editingLog.value || !auth.user) return;
  const valid = await editFormRef.value?.validate();
  if (!valid) return;
  editSubmitting.value = true;
  try {
    await invoke("update_care_log", {
      id:        editingLog.value.id,
      input:     { content: editForm.value.content },
      actorId:   auth.user.id,
      actorRole: auth.user.role,
    });
    $q.notify({ type: "positive", message: "Log updated." });
    showEditDialog.value = false;
    if (tab.value === "daily") loadDaily(); else loadHistory(false);
  } catch (e) {
    $q.notify({ type: "negative", message: `${e}` });
  } finally {
    editSubmitting.value = false;
  }
}

const dailyColumns = [
  { name: "resident_name", label: "Resident",  field: "resident_name", align: "left"   as const },
  { name: "shift",         label: "Shift",     field: "shift",         align: "left"   as const },
  { name: "category",      label: "Category",  field: "category",      align: "left"   as const },
  { name: "content",       label: "Notes",     field: "content",       align: "left"   as const },
  { name: "staff_name",    label: "Staff",     field: "staff_name",    align: "left"   as const },
  { name: "logged_at",     label: "Time",      field: "logged_at",     align: "left"   as const },
  { name: "actions",       label: "",          field: "actions",       align: "center" as const },
];

async function loadDaily() {
  dailyLoading.value = true;
  try {
    dailyLogs.value = await invoke<CareLog[]>("list_care_logs", {
      residentId: null,
      date: dailyDate.value,
      limit: null,
    });
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load logs: ${e}` });
  } finally {
    dailyLoading.value = false;
  }
}

async function submitLog() {
  const valid = await newEntryFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await invoke("create_care_log", {
      input: {
        resident_id: form.value.resident_id,
        staff_id:    auth.user?.id ?? null,
        shift:       form.value.shift,
        category:    form.value.category,
        content:     form.value.content,
      },
    });
    $q.notify({ type: "positive", message: "Care log saved." });
    form.value.content = "";
    showEntryDialog.value  = false;
    await loadDaily();
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to save: ${e}` });
  } finally {
    submitting.value = false;
  }
}

// ── HISTORY TAB ──────────────────────────────────────────────────────────────
function daysAgo(n: number): string {
  const d = new Date();
  d.setDate(d.getDate() - n);
  return d.toISOString().slice(0, 10);
}

const hFilterResident = ref<number | null>(null);
const hFilterFrom     = ref(daysAgo(30));
const hFilterTo       = ref(today);
const hFilterCategory = ref<string | null>(null);
const hAllTime        = ref(false);
const historyLogs     = ref<CareLog[]>([]);
const historyLoading  = ref(false);

const historyPagination = ref({
  page:        1,
  rowsPerPage: 25,
  rowsNumber:  0,
  sortBy:      "logged_at",
  descending:  true,
});

const historyColumns = [
  { name: "resident_name", label: "Resident",    field: "resident_name", align: "left"   as const },
  { name: "shift",         label: "Shift",       field: "shift",         align: "left"   as const },
  { name: "category",      label: "Category",    field: "category",      align: "left"   as const },
  { name: "content",       label: "Content",     field: "content",       align: "left"   as const },
  { name: "staff_name",    label: "Staff",       field: "staff_name",    align: "left"   as const },
  { name: "logged_at",     label: "Date / Time", field: "logged_at",     align: "left"   as const },
  { name: "actions",       label: "",            field: "actions",       align: "center" as const },
];

async function loadHistory(reset = true) {
  if (reset) historyPagination.value.page = 1;
  historyLoading.value = true;
  try {
    const result = await invoke<{ data: CareLog[]; total: number }>("list_care_logs_history", {
      residentId:   hFilterResident.value ?? null,
      dateFrom:     hAllTime.value ? null : (hFilterFrom.value || null),
      dateTo:       hAllTime.value ? null : (hFilterTo.value   || null),
      category:     hFilterCategory.value ?? null,
      incidentOnly: false,
      page:         historyPagination.value.page,
      pageSize:     historyPagination.value.rowsPerPage,
      sortBy:       historyPagination.value.sortBy || null,
      sortDesc:     historyPagination.value.descending,
    });
    historyLogs.value = result.data;
    historyPagination.value.rowsNumber = result.total;
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load history: ${e}` });
  } finally {
    historyLoading.value = false;
  }
}

function onHistoryRequest(props: { pagination: { page: number; rowsPerPage: number; sortBy: string; descending: boolean } }) {
  historyPagination.value.page        = props.pagination.page;
  historyPagination.value.rowsPerPage = props.pagination.rowsPerPage;
  historyPagination.value.sortBy      = props.pagination.sortBy;
  historyPagination.value.descending  = props.pagination.descending;
  loadHistory(false);
}

async function deleteLog(id: number) {
  $q.dialog({
    title: "Delete Log",
    message: "Permanently delete this care log entry?",
    cancel: true, persistent: true,
  }).onOk(async () => {
    try {
      await invoke("delete_care_log", { id });
      $q.notify({ type: "positive", message: "Log deleted." });
      if (tab.value === "daily") loadDaily(); else loadHistory(false);
    } catch (e) {
      $q.notify({ type: "negative", message: `${e}` });
    }
  });
}

// ── Formatting ───────────────────────────────────────────────────────────────
function fmtTime(iso: string) {
  if (!iso) return "";
  return new Date(iso).toLocaleTimeString("en-CA", { hour: "2-digit", minute: "2-digit" });
}
function fmtDate(iso: string) {
  if (!iso) return "";
  return new Date(iso).toLocaleDateString("en-CA", { month: "short", day: "numeric", year: "numeric" });
}
function fmtDisplay(iso: string) {  // for the date input display
  if (!iso) return "";
  return new Date(iso + "T12:00:00").toLocaleDateString("en-CA", { year: "numeric", month: "short", day: "numeric" });
}
function capitalize(s: string) { return s.charAt(0).toUpperCase() + s.slice(1); }

// ── Mount / watch ────────────────────────────────────────────────────────────
watch(tab, t => {
  if (t === "daily") {
    hFilterResident.value = null;
    hFilterCategory.value = null;
    loadDaily();
  } else {
    loadHistory();
  }
});

onMounted(async () => {
  const rid     = route.query.resident ? Number(route.query.resident) : null;
  const tabParam = route.query.tab as string | undefined;

  residents.value = await invoke<Resident[]>("list_residents", { search: "", activeOnly: true });

  if (rid) {
    form.value.resident_id = rid;
    hFilterResident.value  = rid;
  }

  // If coming from Residents page, jump to history tab (managers only)
  if (tabParam === "history" && !isStaff.value) {
    tab.value = "history";
    await loadHistory();
  } else {
    tab.value = "daily";
    await loadDaily();
  }
});
</script>

<template>
  <q-page class="q-pa-lg">

    <!-- ── Header ── -->
    <div class="row items-center q-mb-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">Care Log</div>
        <div class="text-caption text-grey-6">Daily care records and history</div>
      </div>
      <div class="col-auto" v-if="tab === 'daily'">
        <q-btn color="primary" icon="o_add" label="New Entry" unelevated @click="showEntryDialog = true" />
      </div>
    </div>

    <!-- ── Tabs ── -->
    <q-tabs v-model="tab" align="left" indicator-color="primary" active-color="primary" class="q-mb-md">
      <q-tab name="daily"   icon="o_today"        label="Daily Log" />
      <q-tab v-if="!isStaff" name="history" icon="o_manage_search" label="History" />
    </q-tabs>

    <!-- ══════════════════════════════════════════════════════════
         DAILY LOG TAB
    ══════════════════════════════════════════════════════════ -->
    <template v-if="tab === 'daily'">
      <!-- Date navigation bar -->
      <div class="row items-center q-mb-md q-gutter-xs">
        <q-btn flat round dense icon="o_chevron_left"  @click="shiftDay(-1)" />
        <q-btn flat dense unelevated color="grey-8" class="q-px-sm">
          <q-icon name="o_event" size="1rem" class="q-mr-xs" />
          {{ fmtDisplay(dailyDate) }}
          <q-popup-proxy transition-show="scale" transition-hide="scale">
            <q-date v-model="dailyDate" mask="YYYY-MM-DD" @update:model-value="loadDaily">
              <div class="row items-center justify-end q-pa-sm">
                <q-btn v-close-popup label="OK" color="primary" flat dense />
              </div>
            </q-date>
          </q-popup-proxy>
        </q-btn>
        <q-btn flat round dense icon="o_chevron_right" @click="shiftDay(1)" />
        <q-btn flat dense label="Today" color="primary" class="q-ml-xs"
               @click="dailyDate=today; loadDaily()" />
        <q-space />
        <span class="text-caption text-grey-5">{{ dailyLogs.length }} entr{{ dailyLogs.length === 1 ? 'y' : 'ies' }}</span>
      </div>

      <!-- Daily log table -->
      <q-table
        :rows="dailyLogs"
        :columns="dailyColumns"
        row-key="id"
        flat bordered
        :loading="dailyLoading"
        :rows-per-page-options="[25, 50, 100, 0]"
        :pagination="{ rowsPerPage: 25, sortBy: 'logged_at', descending: false }"
      >
        <!-- Resident -->
        <template #body-cell-resident_name="props">
          <q-td :props="props">
            <span
              v-if="!isStaff"
              class="text-weight-medium text-primary"
              style="cursor:pointer;text-decoration:underline dotted"
              @click.stop="hFilterResident = props.row.resident_id; tab = 'history'; loadHistory()"
            >{{ props.row.resident_name }}</span>
            <span v-else class="text-weight-medium">{{ props.row.resident_name }}</span>
          </q-td>
        </template>

        <!-- Shift badge -->
        <template #body-cell-shift="props">
          <q-td :props="props">
            <q-badge :color="SHIFT_COLOR[props.row.shift] ?? 'grey'" :label="capitalize(props.row.shift)" />
          </q-td>
        </template>

        <!-- Category badge -->
        <template #body-cell-category="props">
          <q-td :props="props">
            <q-badge
              :color="CATEGORY_COLOR[props.row.category]"
              :label="capitalize(props.row.category)"
              :style="!isStaff ? 'cursor:pointer' : ''"
              @click.stop="!isStaff && (hFilterCategory = props.row.category, tab = 'history', loadHistory())"
            >
              <q-tooltip v-if="!isStaff">View history for {{ props.row.category }}</q-tooltip>
            </q-badge>
          </q-td>
        </template>

        <!-- Notes — wraps -->
        <template #body-cell-content="props">
          <q-td :props="props" style="white-space:normal;max-width:400px;word-break:break-word">
            {{ props.row.content }}
            <q-badge v-if="props.row.is_incident" color="red" label="Incident" class="q-ml-xs" />
          </q-td>
        </template>

        <!-- Staff -->
        <template #body-cell-staff_name="props">
          <q-td :props="props">
            <span class="text-caption text-grey-7">{{ props.row.staff_name ?? "—" }}</span>
          </q-td>
        </template>

        <!-- Time -->
        <template #body-cell-logged_at="props">
          <q-td :props="props">
            <span class="text-caption">{{ fmtTime(props.row.logged_at) }}</span>
          </q-td>
        </template>

        <!-- Actions -->
        <template #body-cell-actions="props">
          <q-td :props="props" class="text-center">
            <q-btn v-if="canEditLog(props.row)" flat round dense icon="o_edit"
                   color="primary" size="sm" @click="openEdit(props.row)">
              <q-tooltip>Edit</q-tooltip>
            </q-btn>
            <q-btn v-if="canDelete" flat round dense icon="o_delete"
                   color="negative" size="sm" @click="deleteLog(props.row.id)">
              <q-tooltip>Delete</q-tooltip>
            </q-btn>
          </q-td>
        </template>

        <template #no-data>
          <div class="full-width column flex-center q-py-xl">
            <q-icon name="o_assignment" size="3rem" color="grey-4" />
            <div class="text-grey-5 q-mt-sm">No care logs for {{ fmtDisplay(dailyDate) }}</div>
          </div>
        </template>
      </q-table>
    </template>

    <!-- ══════════════════════════════════════════════════════════
         HISTORY TAB
    ══════════════════════════════════════════════════════════ -->
    <template v-else>
      <!-- Filters -->
      <q-card flat bordered class="q-mb-md">
        <q-card-section class="q-py-sm">
          <div class="row q-gutter-md items-end">
            <div class="col-12 col-sm-6 col-md-3">
              <q-select v-model="hFilterResident" :options="residentOptions" label="Resident"
                outlined dense clearable emit-value map-options />
            </div>

            <div class="col-auto flex items-center">
              <q-checkbox v-model="hAllTime" label="All time" color="primary" dense
                          @update:model-value="loadHistory()" />
            </div>

            <!-- From date — Quasar picker -->
            <div class="col-12 col-sm-6 col-md-2">
              <div class="cursor-pointer">
                <q-input v-model="hFilterFrom" outlined dense readonly label="From" :disable="hAllTime" style="pointer-events:none">
                  <template #append>
                    <q-icon name="o_event" color="grey-6" />
                  </template>
                </q-input>
                <q-popup-proxy transition-show="scale" transition-hide="scale">
                  <q-date v-model="hFilterFrom" mask="YYYY-MM-DD">
                    <div class="row items-center justify-end q-pa-sm">
                      <q-btn v-close-popup label="OK" color="primary" flat dense />
                    </div>
                  </q-date>
                </q-popup-proxy>
              </div>
            </div>

            <!-- To date -->
            <div class="col-12 col-sm-6 col-md-2">
              <div class="cursor-pointer">
                <q-input v-model="hFilterTo" outlined dense readonly label="To" :disable="hAllTime" style="pointer-events:none">
                  <template #append>
                    <q-icon name="o_event" color="grey-6" />
                  </template>
                </q-input>
                <q-popup-proxy transition-show="scale" transition-hide="scale">
                  <q-date v-model="hFilterTo" mask="YYYY-MM-DD">
                    <div class="row items-center justify-end q-pa-sm">
                      <q-btn v-close-popup label="OK" color="primary" flat dense />
                    </div>
                  </q-date>
                </q-popup-proxy>
              </div>
            </div>

            <div class="col-12 col-sm-6 col-md-2">
              <q-select v-model="hFilterCategory" :options="CATEGORY_OPTIONS" label="Category"
                outlined dense clearable emit-value map-options />
            </div>
            <div class="col-auto">
              <q-btn color="primary" label="Search" icon="o_search" unelevated dense @click="loadHistory()" />
            </div>
          </div>
        </q-card-section>
      </q-card>

      <!-- History table -->
      <q-table
        :rows="historyLogs"
        :columns="historyColumns"
        row-key="id"
        flat bordered
        :loading="historyLoading"
        v-model:pagination="historyPagination"
        :rows-per-page-options="[10, 25, 50, 100]"
        @request="onHistoryRequest"
      >
        <!-- Resident -->
        <template #body-cell-resident_name="props">
          <q-td :props="props">
            <span class="text-weight-medium">{{ props.row.resident_name }}</span>
          </q-td>
        </template>

        <!-- Shift badge -->
        <template #body-cell-shift="props">
          <q-td :props="props">
            <q-badge :color="SHIFT_COLOR[props.row.shift] ?? 'grey'" :label="capitalize(props.row.shift)" />
          </q-td>
        </template>

        <!-- Category badge -->
        <template #body-cell-category="props">
          <q-td :props="props">
            <q-badge :color="CATEGORY_COLOR[props.row.category]"
                     :label="capitalize(props.row.category)" />
          </q-td>
        </template>

        <!-- Content — full text, wraps -->
        <template #body-cell-content="props">
          <q-td :props="props" style="white-space:normal;max-width:380px;word-break:break-word">
            {{ props.row.content }}
          </q-td>
        </template>

        <!-- Staff -->
        <template #body-cell-staff_name="props">
          <q-td :props="props">
            <span class="text-caption text-grey-7">{{ props.row.staff_name ?? "—" }}</span>
          </q-td>
        </template>

        <!-- Date / time -->
        <template #body-cell-logged_at="props">
          <q-td :props="props">
            <div class="text-caption">{{ fmtDate(props.row.logged_at) }}</div>
            <div class="text-caption text-grey-5">{{ fmtTime(props.row.logged_at) }}</div>
          </q-td>
        </template>

        <!-- Actions -->
        <template #body-cell-actions="props">
          <q-td :props="props" class="text-center">
            <q-btn v-if="canEditLog(props.row)" flat round dense icon="o_edit"
                   color="primary" size="sm" @click="openEdit(props.row)">
              <q-tooltip>Edit</q-tooltip>
            </q-btn>
            <q-btn v-if="canDelete" flat round dense icon="o_delete"
                   color="negative" size="sm" @click="deleteLog(props.row.id)">
              <q-tooltip>Delete</q-tooltip>
            </q-btn>
          </q-td>
        </template>

        <template #no-data>
          <div class="full-width column flex-center q-py-xl">
            <q-icon name="o_manage_search" size="3rem" color="grey-4" />
            <div class="text-grey-5 q-mt-sm">No records found</div>
          </div>
        </template>
      </q-table>
    </template>

    <!-- ── New Entry Dialog ── -->
    <q-dialog v-model="showEntryDialog" persistent>
      <q-card style="min-width:520px;max-width:620px">
        <q-card-section class="row items-center q-pb-none">
          <q-icon name="o_add_circle" color="primary" class="q-mr-sm" size="1.4rem" />
          <div class="text-h6">New Care Log Entry</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-form ref="newEntryFormRef" class="q-gutter-sm">
          <div class="row q-gutter-sm">
            <!-- Date picker in dialog -->
            <div class="col">
              <q-input v-model="dailyDate" outlined dense readonly label="Date">
                <template #append>
                  <q-icon name="o_event" class="cursor-pointer" color="primary">
                    <q-popup-proxy transition-show="scale" transition-hide="scale">
                      <q-date v-model="dailyDate" mask="YYYY-MM-DD" @update:model-value="loadDaily">
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
              <q-select v-model="form.shift" :options="shiftOptions" label="Shift"
                outlined dense emit-value map-options>
                <template #prepend>
                  <q-icon name="o_schedule" :color="SHIFT_COLOR[form.shift] ?? 'grey'" />
                </template>
              </q-select>
            </div>
          </div>
          <q-select v-model="form.resident_id" :options="residentOptions" label="Resident *"
            outlined dense emit-value map-options clearable use-input input-debounce="0"
            @filter="(val, update) => update(() => {})"
            :rules="[v => v !== null && v !== undefined || 'Please select a resident']"
            lazy-rules="ondemand" />
          <q-select v-model="form.category" :options="CATEGORY_OPTIONS" label="Category"
            outlined dense emit-value map-options />
          <q-input v-model="form.content" label="Notes *" type="textarea" outlined rows="3" autogrow
            placeholder="Describe the care activity or observation…"
            :rules="[v => !!v?.trim() || 'Please enter log content']"
            lazy-rules="ondemand" />
        </q-form>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Save Log" icon="o_save" unelevated :loading="submitting" @click="submitLog" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- ── Edit Log Dialog ── -->
    <q-dialog v-model="showEditDialog" persistent>
      <q-card style="min-width:480px;max-width:580px">
        <q-card-section class="row items-center q-pb-none">
          <q-icon name="o_edit" color="primary" class="q-mr-sm" size="1.3rem" />
          <div class="text-h6">Edit Care Log</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section>
          <div v-if="editingLog" class="text-caption text-grey-6 q-mb-sm">
            {{ editingLog.resident_name }} · {{ capitalize(editingLog.shift) }} · {{ fmtDate(editingLog.logged_at) }}
          </div>
          <q-form ref="editFormRef">
            <q-input v-model="editForm.content" label="Notes *" type="textarea" outlined rows="3" autogrow
              :rules="[v => !!v?.trim() || 'Content cannot be empty']" lazy-rules="ondemand" />
          </q-form>
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Update" icon="o_save" unelevated :loading="editSubmitting" @click="submitEdit" />
        </q-card-actions>
      </q-card>
    </q-dialog>

  </q-page>
</template>
