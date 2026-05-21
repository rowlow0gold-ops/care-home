<script setup lang="ts">
import { ref, computed, reactive, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { useServerSessionStore } from "@/stores/server-session";
import { server } from "@/lib/server";
import { useSettingsStore, detectCurrentShift } from "@/stores/settings";
import { useRoute } from "vue-router";
import * as XLSX from "xlsx";

const $q       = useQuasar();
const session  = useServerSessionStore();
const settings = useSettingsStore();
const route    = useRoute();

// Legacy shim: `auth.user.X` keeps working in BOTH script and template while
// we migrate writes off rusqlite. Reactive getter so reads stay live.
// Map server roles -> legacy labels for any UI conditionals not yet swapped:
//   caregiver/nurse    -> "staff"
//   branch_manager     -> "manager"
//   hq / super_admin   -> "admin"
const auth = reactive({
  get user() {
    const me = session.me;
    if (!me) return null;
    return {
      id: me.id,
      username: me.email,
      full_name: me.name,
      role:
        me.role === "branch_manager"
          ? "manager"
          : me.role === "hq" || me.role === "super_admin"
            ? "admin"
            : "staff",
    };
  },
});
const isStaff = computed(() => auth.user?.role === "staff");
const canDelete = computed(() => ["manager", "admin"].includes(auth.user?.role ?? ""));

// ── Section tabs ──────────────────────────────────────────────────────────────
type Section = "residents" | "carelog" | "medications" | "healthcharts" | "history";
const section = ref<Section>("residents");

type HistorySection = "residents_h" | "carelog_h" | "medications_h" | "healthcharts_h";
const historySection = ref<HistorySection>("residents_h");
const historySectionOptions = [
  { label: "Residents",     value: "residents_h",    icon: "o_people" },
  { label: "Care Log",      value: "carelog_h",      icon: "o_assignment" },
  { label: "Medications",   value: "medications_h",  icon: "o_medication" },
  { label: "Health Charts", value: "healthcharts_h", icon: "o_monitor_heart" },
];

// ── Interfaces ────────────────────────────────────────────────────────────────
interface Resident {
  id: number; first_name: string; last_name: string; date_of_birth: string;
  gender: string; room_number: string | null; admission_date: string;
  discharge_date: string | null; care_grade: number | null;
  cognitive_support: boolean; primary_diagnosis: string | null;
  allergies: string | null; dietary_restrictions: string | null;
  insurance_number: string | null; notes: string | null;
  is_active: boolean; is_deceased: boolean;
}
interface ResidentInput {
  first_name: string; last_name: string; date_of_birth: string; gender: string;
  room_number: string | null; admission_date: string | null; care_grade: number | null;
  cognitive_support: boolean; primary_diagnosis: string | null; allergies: string | null;
  dietary_restrictions: string | null; insurance_number: string | null; notes: string | null;
}
interface CareLog {
  id: number; resident_id: number; resident_name: string;
  staff_id: number | null; staff_name: string | null;
  shift: string; category: string; content: string;
  is_incident: boolean; is_flagged: boolean; logged_at: string;
}
interface Medication {
  id: number; resident_id: number; resident_name: string;
  name: string; dosage: string; frequency: string; route: string;
  start_date: string; end_date: string | null; prescriber: string;
  instructions: string; is_active: boolean;
}
interface Vital {
  id: number; resident_id: number; resident_name: string;
  staff_name: string | null;
  bp_systolic: number | null; bp_diastolic: number | null;
  heart_rate: number | null; temperature: number | null;
  weight: number | null; blood_sugar: number | null; spo2: number | null;
  notes: string | null; measured_at: string;
}

// ── Shared helpers ────────────────────────────────────────────────────────────
// Format a number cleanly — strip float garbage, show up to `dec` decimal places
function fmtNum(val: number | null | undefined, dec = 1): string {
  if (val == null) return "—";
  const n = Math.round(val * Math.pow(10, dec)) / Math.pow(10, dec);
  return n % 1 === 0 ? String(n) : n.toFixed(dec);
}
// Validation rules for number inputs
const rulePositiveInt  = (v: string) => !v || (/^\d+$/.test(v.trim()) && Number(v) > 0)          || "Whole number only";
const rulePositiveDec  = (v: string) => !v || (/^\d+(\.\d+)?$/.test(v.trim()) && Number(v) > 0)  || "Number only";
const ruleSpo2         = (v: string) => !v || (/^\d+$/.test(v.trim()) && Number(v) >= 1 && Number(v) <= 100) || "1 – 100";
const ruleTemp         = (v: string) => !v || (/^\d+(\.\d+)?$/.test(v.trim()) && Number(v) >= 30 && Number(v) <= 45) || "30 – 45 °C";
const ruleBP           = (v: string) => !v || (/^\d+$/.test(v.trim()) && Number(v) >= 40 && Number(v) <= 300) || "40 – 300 mmHg";

function localToday(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`;
}
function daysAgo(n: number): string {
  const d = new Date(); d.setDate(d.getDate() - n); return d.toISOString().slice(0, 10);
}
function age(dob: string) {
  return Math.floor((Date.now() - new Date(dob).getTime()) / (1000*60*60*24*365.25));
}
function stayDuration(admDate: string, disDate?: string | null): string {
  const start = new Date(admDate + "T00:00:00");
  const end   = disDate ? new Date(disDate + "T00:00:00") : new Date();
  if (isNaN(start.getTime())) return "";
  let years = end.getFullYear() - start.getFullYear();
  let months = end.getMonth() - start.getMonth();
  if (months < 0) { years--; months += 12; }
  const days = Math.floor((end.getTime() - start.getTime()) / 86_400_000);
  if (years > 0 && months > 0) return `${years}y ${months}m`;
  if (years > 0)  return `${years} year${years > 1 ? "s" : ""}`;
  if (months > 0) return `${months} month${months > 1 ? "s" : ""}`;
  return `${days} day${days !== 1 ? "s" : ""}`;
}
const careLevelLabel: Record<number,string> = { 1:"SL1", 2:"SL2/SL3", 3:"SL4", 4:"SL4D", 5:"LTC" };
function careGradeColor(grade: number | null) {
  if (!grade) return "grey";
  if (grade <= 2) return "positive";
  if (grade === 3) return "warning";
  if (grade === 4) return "orange";
  return "negative";
}
function genderLabel(g: string) {
  return { male:"M", female:"F", other:"Other" }[g?.toLowerCase()] ?? g;
}
function routeColor(r: string) {
  return { oral:"teal", injection:"deep-orange", topical:"blue", inhaled:"purple" }[r] || "grey";
}
function formatBP(v: Vital) {
  if (v.bp_systolic && v.bp_diastolic) return `${v.bp_systolic}/${v.bp_diastolic}`;
  if (v.bp_systolic) return `${v.bp_systolic}/—`;
  return "—";
}
function bpColor(v: Vital) { return v.bp_systolic && v.bp_systolic > 140 ? "text-negative" : ""; }
function spo2Color(v: Vital) {
  if (!v.spo2) return "";
  if (v.spo2 < 90) return "text-negative";
  if (v.spo2 < 94) return "text-warning";
  return "";
}
function formatDateTime(iso: string) {
  if (!iso) return "";
  return new Date(iso).toLocaleString("en-CA", { year:"numeric", month:"2-digit", day:"2-digit", hour:"2-digit", minute:"2-digit" });
}
function fmtTime(iso: string) {
  if (!iso) return "";
  return new Date(iso).toLocaleTimeString("en-CA", { hour:"2-digit", minute:"2-digit" });
}
function fmtDate(iso: string) {
  if (!iso) return "";
  return new Date(iso).toLocaleDateString("en-CA", { month:"short", day:"numeric", year:"numeric" });
}
function fmtDisplay(iso: string) {
  if (!iso) return "";
  return new Date(iso + "T12:00:00").toLocaleDateString("en-CA", { year:"numeric", month:"short", day:"numeric" });
}
function capitalize(s: string) { return s.charAt(0).toUpperCase() + s.slice(1); }

const SHIFT_COLOR: Record<string,string> = {
  morning:"orange", afternoon:"blue", day:"teal", night:"deep-purple", visit:"green",
};
const CATEGORY_COLOR: Record<string,string> = {
  bathing:"cyan", meals:"green", medication:"teal", mood:"purple", incident:"red", note:"grey-7",
};
const CATEGORY_OPTIONS = [
  { label:"Bathing",    value:"bathing"    },
  { label:"Meals",      value:"meals"      },
  { label:"Medication", value:"medication" },
  { label:"Mood",       value:"mood"       },
  { label:"Incident",   value:"incident"   },
  { label:"Note",       value:"note"       },
];
const shiftOptions = [
  { label: "Morning",   value: "morning"   },
  { label: "Afternoon", value: "afternoon" },
  { label: "Day",       value: "day"       },
  { label: "Night",     value: "night"     },
  { label: "Visit",     value: "visit"     },
];
function detectShift(): string { return detectCurrentShift(settings.shiftModel); }

// ── Shared resident list (for dropdowns) ──────────────────────────────────────
interface ResidentShort {
  id: number; first_name: string; last_name: string;
  room_number: string | null; date_of_birth: string;
  gender: string; admission_date: string; discharge_date: string | null;
}
const residentList = ref<ResidentShort[]>([]);
const residentOptions = computed(() =>
  residentList.value
    .map(r => ({ label:`${r.first_name} ${r.last_name}`, value:r.id }))
    .sort((a, b) => a.label.localeCompare(b.label))
);
async function loadResidentList() {
  try {
    // MIGRATED: server-mode read path. Maps server shape -> legacy ResidentShort.
    const srv = await server.residents();
    residentList.value = srv
      .filter((r) => r.status === "active")
      .map((r) => {
        // Korean names typically lack a first/last split — put the whole name
        // into `first_name` and leave `last_name` empty so the existing
        // `${first_name} ${last_name}` interpolation still renders cleanly.
        return {
          // Legacy table expected number IDs; UUIDs render fine through Quasar
          // when only used as v-for keys + dropdown values. Cast through `any`
          // until each callsite gets typed properly during follow-up passes.
          id: r.id as unknown as number,
          first_name: r.full_name,
          last_name: "",
          room_number: r.room_number,
          date_of_birth: r.birth_date,
          gender: r.sex === "male" ? "M" : r.sex === "female" ? "F" : "O",
          admission_date: r.admitted_on,
          discharge_date: null,
        };
      });
  } catch (e) {
    console.error("loadResidentList (server):", e);
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// SECTION: RESIDENTS (active)
// ══════════════════════════════════════════════════════════════════════════════
const residents        = ref<Resident[]>([]);
const loadingActive    = ref(false);
const resSearchFilter  = ref<{ label:string; value:number }|null>(null);
const filteredResOpts  = ref<{ label:string; value:number }[]>([]);
const resSelectedId    = ref<number|null>(null);

// ── Server-side pagination + filters ─────────────────────────────────────────
const resPagination = ref({ page: 1, rowsPerPage: 25, rowsNumber: 0, sortBy: "id", descending: true });
const filterCareGrade  = ref<number|null>(null);
const filterGender     = ref<string|null>(null);
const filterCognitive  = ref<boolean|null>(null);

function onResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredResOpts.value = residentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onResSelected(opt: { label:string; value:number }|null) {
  resSelectedId.value = opt?.value ?? null;
  resPagination.value.page = 1;
  loadActive();
}
function onResRequest(props: { pagination: { page:number; rowsPerPage:number; sortBy:string; descending:boolean } }) {
  Object.assign(resPagination.value, props.pagination);
  loadActive();
}
const showDialog       = ref(false);
const residentFormRef  = ref();
const editingResId     = ref<number | null>(null);
const dialogTitle      = ref("Add Resident");
const resForm = ref<ResidentInput>({
  first_name:"", last_name:"", date_of_birth:"", gender:"female",
  room_number:null, admission_date:null, care_grade:null,
  cognitive_support:false, primary_diagnosis:null, allergies:null,
  dietary_restrictions:null, insurance_number:null, notes:null,
});

const activeResCols = [
  { name:"name",           label:"Name",         field:(r:Resident)=>`${r.first_name} ${r.last_name}`, align:"left" as const },
  { name:"room",           label:"Room",         field:"room_number", align:"center" as const },
  { name:"dob",            label:"Date of Birth",field:"date_of_birth", align:"left" as const },
  { name:"admission_date", label:"Admitted",     field:"admission_date", align:"left" as const },
  { name:"care_grade",     label:"Care Level",   field:"care_grade", align:"center" as const },
  { name:"diagnosis",      label:"Diagnosis",    field:"primary_diagnosis", align:"left" as const },
  { name:"actions",        label:"",             field:"actions",           align:"right" as const },
];

async function loadActive() {
  loadingActive.value = true;
  try {
    // MIGRATED: pull from care-home-server. Pagination + filters are done
    // client-side for v1 — typical branch has <50 active residents.
    const srv = await server.residents();
    let rows = srv.filter((r) => r.status === "active");

    if (resSelectedId.value !== null) {
      rows = rows.filter((r) => (r.id as unknown as number) === resSelectedId.value);
    }
    if (filterCareGrade.value !== null) {
      rows = rows.filter(
        (r) => (r.care_grade ?? "") === String(filterCareGrade.value),
      );
    }
    if (filterGender.value) {
      const want = filterGender.value === "M" ? "male" : "female";
      rows = rows.filter((r) => r.sex === want);
    }
    // cognitive_support filter: server stores it as care_grade='cognitive_support'
    if (filterCognitive.value !== null) {
      rows = rows.filter(
        (r) =>
          (r.care_grade === "cognitive_support") === filterCognitive.value,
      );
    }

    const total = rows.length;
    const p = resPagination.value;
    const start = (p.page - 1) * p.rowsPerPage;
    const page = rows.slice(start, start + p.rowsPerPage);

    residents.value = page.map((r) => ({
      id: r.id as unknown as number,
      first_name: r.full_name,
      last_name: "",
      date_of_birth: r.birth_date,
      gender: r.sex === "male" ? "M" : r.sex === "female" ? "F" : "O",
      room_number: r.room_number,
      admission_date: r.admitted_on,
      discharge_date: null,
      care_grade:
        r.care_grade && r.care_grade !== "cognitive_support"
          ? Number(r.care_grade)
          : null,
      cognitive_support: r.care_grade === "cognitive_support",
      primary_diagnosis: null,
      allergies: null,
      dietary_restrictions: null,
      insurance_number: null,
      notes: null,
      is_active: r.status === "active",
      is_deceased: r.status === "deceased",
    }));
    resPagination.value.rowsNumber = total;
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.message ?? String(e) });
  } finally {
    loadingActive.value = false;
  }
}
function openAdd() {
  editingResId.value = null; dialogTitle.value = "Add Resident";
  resForm.value = { first_name:"", last_name:"", date_of_birth:"", gender:"female",
    room_number:null, admission_date:null, care_grade:null, cognitive_support:false,
    primary_diagnosis:null, allergies:null, dietary_restrictions:null,
    insurance_number:null, notes:null };
  showDialog.value = true;
}
function openEdit(r: Resident) {
  editingResId.value = r.id; dialogTitle.value = "Edit Resident";
  resForm.value = { first_name:r.first_name, last_name:r.last_name, date_of_birth:r.date_of_birth,
    gender:r.gender, room_number:r.room_number, admission_date:r.admission_date,
    care_grade:r.care_grade, cognitive_support:r.cognitive_support,
    primary_diagnosis:r.primary_diagnosis, allergies:r.allergies,
    dietary_restrictions:r.dietary_restrictions, insurance_number:r.insurance_number, notes:r.notes };
  showDialog.value = true;
}
async function saveResident() {
  const valid = await residentFormRef.value?.validate();
  if (!valid) return;
  try {
    if (editingResId.value) {
      await invoke("update_resident", { id:editingResId.value, input:resForm.value });
      $q.notify({ type:"positive", message:"Resident updated" });
    } else {
      await invoke("create_resident", { input:resForm.value });
      $q.notify({ type:"positive", message:"Resident added" });
    }
    showDialog.value = false;
    loadActive();
  } catch (e:any) { $q.notify({ type:"negative", message:e }); }
}
function confirmDischarge(r: Resident) {
  $q.dialog({ title:"Discharge Resident", message:`Discharge ${r.first_name} ${r.last_name}? They will be moved to History.`, cancel:true, persistent:true })
    .onOk(async () => {
      try {
        await invoke("discharge_resident", { id:r.id, dischargeDate:new Date().toISOString().slice(0,10) });
        $q.notify({ type:"positive", message:"Resident discharged" });
        loadActive();
      } catch (e:any) { $q.notify({ type:"negative", message:e }); }
    });
}
function confirmDeceased(r: Resident) {
  $q.dialog({ title:"Mark as Deceased", message:`Mark ${r.first_name} ${r.last_name} as deceased?`, cancel:true, persistent:true })
    .onOk(async () => {
      try {
        await invoke("mark_deceased", { id:r.id });
        $q.notify({ type:"positive", message:"Record updated." });
        loadActive();
      } catch (e:any) { $q.notify({ type:"negative", message:e }); }
    });
}

// ── Resident Summary popup ────────────────────────────────────────────────────
interface CareLogSum { id:number; category:string; content:string; shift:string; staff_name:string|null; is_incident:boolean; logged_at:string; }
interface MedSum     { id:number; name:string; dosage:string; frequency:string; route:string; }
interface VitalSum   { id:number; bp_systolic:number|null; bp_diastolic:number|null; heart_rate:number|null; temperature:number|null; weight:number|null; blood_sugar:number|null; spo2:number|null; measured_at:string; }

const showSummary     = ref(false);
const summaryRes      = ref<Resident|null>(null);
const summaryLoading  = ref(false);
const summaryMeds     = ref<MedSum[]>([]);
const summaryVitals   = ref<VitalSum[]>([]);
const summaryNotes    = ref<CareLogSum[]>([]);

async function openSummary(row: Resident) {
  summaryRes.value = row;
  summaryMeds.value = []; summaryVitals.value = []; summaryNotes.value = [];
  showSummary.value = true; summaryLoading.value = true;
  try {
    const [meds, vitals, notes] = await Promise.all([
      invoke<{ data:MedSum[]; total:number }>("list_medications", { residentId:row.id, activeOnly:true, page:1, pageSize:20 }),
      invoke<{ data:VitalSum[]; total:number }>("list_vitals", { residentId:row.id, page:1, pageSize:50 }),
      invoke<CareLogSum[]>("list_care_logs", { residentId:row.id, limit:50 }).catch(()=>[]),
    ]);
    summaryMeds.value   = meds.data;
    summaryVitals.value = vitals.data;
    summaryNotes.value  = notes;
  } catch (e) { $q.notify({ type:"negative", message:`Failed: ${e}` }); }
  finally { summaryLoading.value = false; }
}

// ── Residents history ─────────────────────────────────────────────────────────
const historyResidents    = ref<Resident[]>([]);
const loadingHisRes       = ref(false);
const hisResSearchFilter  = ref<{ label:string; value:number }|null>(null);
const filteredHisResOpts  = ref<{ label:string; value:number }[]>([]);
const hisResSelectedId    = ref<number|null>(null);

// Options built from already-loaded history list so it covers discharged/deceased
const hisResidentOptions = computed(() =>
  historyResidents.value.map(r => ({ label:`${r.first_name} ${r.last_name}`, value:r.id }))
);
function onHisResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredHisResOpts.value = hisResidentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onHisResSelected(opt: { label:string; value:number }|null) {
  hisResSelectedId.value = opt?.value ?? null;
}
const historyResCols = [
  { name:"name",           label:"Name",          field:(r:Resident)=>`${r.first_name} ${r.last_name}`, align:"left" as const },
  { name:"status",         label:"Status",        field:"is_deceased",     align:"center" as const },
  { name:"room",           label:"Room",          field:"room_number", align:"center" as const },
  { name:"dob",            label:"Date of Birth", field:"date_of_birth", align:"left" as const },
  { name:"admission_date", label:"Admitted",      field:"admission_date", align:"left" as const },
  { name:"discharge_date", label:"Discharged",    field:"discharge_date", align:"left" as const },
  { name:"diagnosis",      label:"Diagnosis",     field:"primary_diagnosis", align:"left" as const },
];

const hisFilterStatus    = ref<string|null>(null);   // null=all, "discharged", "deceased"
const hisFilterGender    = ref<string|null>(null);
const hisFilterCareGrade = ref<number|null>(null);

const filteredHistoryResidents = computed(() => {
  return historyResidents.value.filter(r => {
    if (hisResSelectedId.value && r.id !== hisResSelectedId.value) return false;
    if (hisFilterStatus.value === "deceased"  && !r.is_deceased)  return false;
    if (hisFilterStatus.value === "discharged" && r.is_deceased)  return false;
    if (hisFilterGender.value    && r.gender !== hisFilterGender.value)         return false;
    if (hisFilterCareGrade.value && r.care_grade !== hisFilterCareGrade.value)  return false;
    return true;
  });
});
async function loadHistoryResidents() {
  loadingHisRes.value = true;
  hisResSearchFilter.value = null;
  hisResSelectedId.value = null;
  try {
    const [dis, dec] = await Promise.all([
      invoke<Resident[]>("list_residents", { search:null, activeOnly:null, status:"discharged" }),
      invoke<Resident[]>("list_residents", { search:null, activeOnly:null, status:"deceased" }),
    ]);
    historyResidents.value = [...dis,...dec].sort((a,b)=>(b.discharge_date??"").localeCompare(a.discharge_date??""));
  } catch (e:any) { $q.notify({ type:"negative", message:e }); }
  finally { loadingHisRes.value = false; }
}

// ══════════════════════════════════════════════════════════════════════════════
// SECTION: CARE LOG (daily)
// ══════════════════════════════════════════════════════════════════════════════
const today           = localToday();
const activeShift     = ref(detectShift());
const dailyLogs       = ref<CareLog[]>([]);
const dailyLoading    = ref(false);
const showEntryDialog = ref(false);
const newEntryFormRef = ref();
function nowDate(): string { const d = new Date(); return d.toISOString().slice(0,10); }
function nowTime(): string { const d = new Date(); return d.toTimeString().slice(0,5); }
const clForm = ref({ resident_id:null as number|null, shift:activeShift.value, category:"note", content:"", logged_date:nowDate(), logged_time:nowTime() });
const clSubmitting    = ref(false);

const clResFilter      = ref<{ label:string; value:number }|null>(null);
const filteredClOpts   = ref<{ label:string; value:number }[]>([]);
const clSelectedRes    = ref<number|null>(null);
const selectedClResObj = computed(() => residentList.value.find(r => r.id === clSelectedRes.value) ?? null);
const clCategory       = ref<string|null>(null);
const clShift          = ref<string|null>(null);

function onClResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredClOpts.value = residentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onClResSelected(opt: { label:string; value:number }|null) {
  clSelectedRes.value = opt?.value ?? null;
  loadDaily();
}

const showEditLogDialog = ref(false);
const editLogFormRef    = ref();
const editingLog        = ref<CareLog|null>(null);
const editLogForm       = ref({ content:"" });
const editLogSubmitting = ref(false);

function canEditLog(log: CareLog) {
  if (!auth.user) return false;
  if (auth.user.role === "staff") return log.staff_id === auth.user.id;
  return true;
}
const dailyCols = [
  { name:"resident_name", label:"Resident", field:"resident_name", align:"left" as const },
  { name:"shift",         label:"Shift",    field:"shift",         align:"left" as const },
  { name:"category",      label:"Category", field:"category",      align:"left" as const },
  { name:"content",       label:"Notes",    field:"content",       align:"left" as const },
  { name:"staff_name",    label:"Staff",    field:"staff_name",    align:"left" as const },
  { name:"logged_at",     label:"Date/Time",field:"logged_at",     align:"left" as const },
  { name:"actions",       label:"",         field:"actions",       align:"center" as const },
];
async function loadDaily() {
  dailyLoading.value = true;
  try {
    dailyLogs.value = await invoke<CareLog[]>("list_care_logs", { residentId:clSelectedRes.value ?? null, date:null, category:clCategory.value, shift:clShift.value, limit:null });
  } catch (e) { $q.notify({ type:"negative", message:`Failed: ${e}` }); }
  finally { dailyLoading.value = false; }
}
async function submitLog() {
  const valid = await newEntryFormRef.value?.validate();
  if (!valid) return;
  clSubmitting.value = true;
  try {
    const logged_at = `${clForm.value.logged_date} ${clForm.value.logged_time}`;
    await invoke("create_care_log", { input:{ resident_id:clForm.value.resident_id, staff_id:auth.user?.id??null, shift:clForm.value.shift, category:clForm.value.category, content:clForm.value.content, logged_at } });
    $q.notify({ type:"positive", message:"Care log saved." });
    clForm.value = { resident_id: clSelectedRes.value ?? null, shift: activeShift.value, category: "note", content: "", logged_date: nowDate(), logged_time: nowTime() };
    showEntryDialog.value = false;
    await loadDaily();
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { clSubmitting.value = false; }
}
function openEditLog(log: CareLog) {
  editingLog.value = log; editLogForm.value.content = log.content; showEditLogDialog.value = true;
}
async function submitEditLog() {
  if (!editingLog.value || !auth.user) return;
  const valid = await editLogFormRef.value?.validate();
  if (!valid) return;
  editLogSubmitting.value = true;
  try {
    await invoke("update_care_log", { id:editingLog.value.id, input:{ content:editLogForm.value.content }, actorId:auth.user.id, actorRole:auth.user.role });
    $q.notify({ type:"positive", message:"Log updated." });
    showEditLogDialog.value = false;
    if (section.value === "carelog") loadDaily(); else loadCareLogHistory(false);
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { editLogSubmitting.value = false; }
}
async function deleteLog(id: number) {
  $q.dialog({ title:"Delete Log", message:"Permanently delete this entry?", cancel:true, persistent:true })
    .onOk(async () => {
      try {
        await invoke("delete_care_log", { id });
        $q.notify({ type:"positive", message:"Log deleted." });
        if (section.value === "carelog") loadDaily(); else loadCareLogHistory(false);
      } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
    });
}

// ── Care Log history ──────────────────────────────────────────────────────────
const clhResident     = ref<number|null>(null);
const clhResFilter    = ref<{ label:string; value:number }|null>(null);
const filteredClhOpts = ref<{ label:string; value:number }[]>([]);
const clhCategory     = ref<string|null>(null);
const clhShift        = ref<string|null>(null);
const clhDateFrom     = ref<string|null>(null);
const clhDateTo       = ref<string|null>(null);
const clhIncidentOnly = ref(false);
const clHistory       = ref<CareLog[]>([]);

function onClhResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredClhOpts.value = residentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onClhResSelected(opt: { label:string; value:number }|null) {
  clhResident.value = opt?.value ?? null;
  clhPagination.value.page = 1;
  loadCareLogHistory();
}
const clhLoading   = ref(false);
const clhPagination = ref({ page:1, rowsPerPage:25, rowsNumber:0, sortBy:"logged_at", descending:true });

const clhCols = [
  { name:"resident_name", label:"Resident",    field:"resident_name", align:"left"   as const },
  { name:"shift",         label:"Shift",       field:"shift",         align:"left"   as const },
  { name:"category",      label:"Category",    field:"category",      align:"left"   as const },
  { name:"content",       label:"Content",     field:"content",       align:"left"   as const },
  { name:"staff_name",    label:"Staff",       field:"staff_name",    align:"left"   as const },
];
async function loadCareLogHistory(reset = true) {
  if (reset) clhPagination.value.page = 1;
  clhLoading.value = true;
  try {
    const result = await invoke<{ data:CareLog[]; total:number }>("list_care_logs_history", {
      residentId:   clhResident.value ?? null,
      dateFrom:     clhDateFrom.value,
      dateTo:       clhDateTo.value,
      category:     clhCategory.value,
      shift:        clhShift.value,
      incidentOnly: clhIncidentOnly.value,
      page:         clhPagination.value.page,
      pageSize:     clhPagination.value.rowsPerPage,
      sortBy:       clhPagination.value.sortBy || null,
      sortDesc:     clhPagination.value.descending,
    });
    clHistory.value = result.data;
    clhPagination.value.rowsNumber = result.total;
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { clhLoading.value = false; }
}
function onClhRequest(props: { pagination:{ page:number; rowsPerPage:number; sortBy:string; descending:boolean } }) {
  Object.assign(clhPagination.value, props.pagination);
  loadCareLogHistory(false);
}

// ── Jump to care log history for a resident ───────────────────────────────────
function jumpToCareLogHistory(residentId: number) {
  const opt = residentOptions.value.find(r => r.value === residentId) ?? null;

  if (isStaff.value) {
    // Staff cannot access history — take them to the daily care log filtered by this resident
    clSelectedRes.value = residentId;
    clResFilter.value   = opt;
    section.value       = "carelog";
    loadDaily();
    return;
  }

  clhResident.value    = residentId;
  clhResFilter.value   = opt;          // keep the dropdown in sync
  historySection.value = "carelog_h";
  section.value        = "history";
  loadCareLogHistory();
}

// ══════════════════════════════════════════════════════════════════════════════
// SECTION: MEDICATIONS
// ══════════════════════════════════════════════════════════════════════════════
const medications         = ref<Medication[]>([]);
const medLoading          = ref(false);
const showAddMedDialog    = ref(false);
const medSubmitting       = ref(false);
const addMedFormRef       = ref();
const medResFilter        = ref<{ label:string; value:number }|null>(null);
const filteredMedOpts     = ref<{ label:string; value:number }[]>([]);
const medSelectedRes      = ref<number|null>(null);
const medRoute            = ref<string|null>(null);
const medPagination       = ref({ page:1, rowsPerPage:25, rowsNumber:0, sortBy:"start_date", descending:true });

const medForm = ref({
  resident_id:null as number|null, name:"", dosage:"", frequency:"",
  route:"oral", start_date:localToday(),
  prescriber:"", instructions:"",
});
const routeOptions = [
  { label:"Oral", value:"oral" }, { label:"Injection", value:"injection" },
  { label:"Topical", value:"topical" }, { label:"Inhaled", value:"inhaled" },
];
const selectedMedResObj = computed(() => residentList.value.find(r => r.id === medSelectedRes.value) ?? null);

const medCols = [
  { name:"resident_name", label:"Resident",   field:"resident_name", align:"left" as const },
  { name:"name",          label:"Medication", field:"name",          align:"left" as const },
  { name:"dosage",        label:"Dosage",     field:"dosage",        align:"left" as const },
  { name:"frequency",     label:"Frequency",  field:"frequency",     align:"left" as const },
  { name:"route",         label:"Route",      field:"route",         align:"left" as const },
  { name:"start_date",    label:"Start Date", field:"start_date",    align:"left" as const },
  { name:"prescriber",    label:"Prescriber", field:"prescriber",    align:"left" as const },
];

function onMedResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredMedOpts.value = residentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onMedResSelected(opt: { label:string; value:number }|null) {
  medSelectedRes.value = opt?.value ?? null;
  medPagination.value.page = 1;
  loadMedications();
}
async function loadMedications() {
  medLoading.value = true;
  try {
    const result = await invoke<{ data:Medication[]; total:number }>("list_medications", {
      residentId: medSelectedRes.value ?? null, activeOnly: true,
      route: medRoute.value,
      page: medPagination.value.page, pageSize: medPagination.value.rowsPerPage,
      sortBy: medPagination.value.sortBy || null, sortDesc: medPagination.value.descending,
    });
    medications.value = result.data;
    medPagination.value.rowsNumber = result.total;
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { medLoading.value = false; }
}
function onMedRequest(props: { pagination:{ page:number; rowsPerPage:number; sortBy:string; descending:boolean } }) {
  Object.assign(medPagination.value, props.pagination);
  loadMedications();
}
async function submitMedication() {
  const valid = await addMedFormRef.value?.validate();
  if (!valid) return;
  medSubmitting.value = true;
  try {
    await invoke("create_medication", { input:{ resident_id:medForm.value.resident_id, name:medForm.value.name, dosage:medForm.value.dosage, frequency:medForm.value.frequency, route:medForm.value.route, start_date:medForm.value.start_date, end_date:null, prescriber:medForm.value.prescriber, instructions:medForm.value.instructions } });
    $q.notify({ type:"positive", message:"Medication added." });
    showAddMedDialog.value = false;
    medForm.value = { resident_id:null, name:"", dosage:"", frequency:"", route:"oral", start_date:localToday(), prescriber:"", instructions:"" };
    medPagination.value.page = 1;
    await loadMedications();
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { medSubmitting.value = false; }
}
function confirmStopMed(med: Medication) {
  $q.dialog({ title:"Stop Medication", message:`Stop "${med.name}" for ${med.resident_name}?`, cancel:{ label:"Cancel", flat:true }, ok:{ label:"Stop", color:"negative", unelevated:true }, persistent:true })
    .onOk(async () => {
      try {
        await invoke("stop_medication", { id:med.id });
        $q.notify({ type:"positive", message:"Medication stopped." });
        await loadMedications();
      } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
    });
}
// Medications history
const medHistory        = ref<Medication[]>([]);
const medHisLoading     = ref(false);
const medHisResFilter   = ref<{ label:string; value:number }|null>(null);
const filteredMedHisOpts = ref<{ label:string; value:number }[]>([]);
const medHisSelectedRes = ref<number|null>(null);
const medHisRoute       = ref<string|null>(null);
const medHisPagination  = ref({ page:1, rowsPerPage:25, rowsNumber:0, sortBy:"start_date", descending:true });

function onMedHisResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredMedHisOpts.value = residentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onMedHisResSelected(opt: { label:string; value:number }|null) {
  medHisSelectedRes.value = opt?.value ?? null;
  medHisPagination.value.page = 1;
  loadMedHistory();
}
async function loadMedHistory(reset = true) {
  if (reset) medHisPagination.value.page = 1;
  medHisLoading.value = true;
  try {
    const result = await invoke<{ data:Medication[]; total:number }>("list_medications", {
      residentId: medHisSelectedRes.value ?? null, activeOnly: false,
      route: medHisRoute.value,
      page: medHisPagination.value.page, pageSize: medHisPagination.value.rowsPerPage,
      sortBy: medHisPagination.value.sortBy || null, sortDesc: medHisPagination.value.descending,
    });
    medHistory.value = result.data;
    medHisPagination.value.rowsNumber = result.total;
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { medHisLoading.value = false; }
}
function onMedHisRequest(props: { pagination:{ page:number; rowsPerPage:number; sortBy:string; descending:boolean } }) {
  Object.assign(medHisPagination.value, props.pagination);
  loadMedHistory(false);
}

// ══════════════════════════════════════════════════════════════════════════════
// SECTION: HEALTH CHARTS
// ══════════════════════════════════════════════════════════════════════════════
const vitals           = ref<Vital[]>([]);
const vitalLoading     = ref(false);
const showVitalDialog  = ref(false);
const vitalSubmitting  = ref(false);
const vitalResFilter   = ref<{ label:string; value:number }|null>(null);
const vitalDateFrom    = ref<string|null>(null);
const vitalDateTo      = ref<string|null>(null);
const filteredVitalOpts = ref<{ label:string; value:number }[]>([]);
const vitalSelectedRes = ref<number|null>(null);
const vitalPagination  = ref({ page:1, rowsPerPage:25, rowsNumber:0, sortBy:"measured_at", descending:true });
const vitalForm = ref({
  resident_id:null as number|null, bp_systolic:"", bp_diastolic:"",
  heart_rate:"", temperature:"", weight:"", blood_sugar:"", spo2:"", notes:"",
});
const selectedVitalResObj = computed(() => residentList.value.find(r => r.id === vitalSelectedRes.value) ?? null);

const vitalCols = [
  { name:"resident_name", label:"Resident",    field:"resident_name", align:"left"   as const },
  { name:"measured_at",   label:"Date/Time",   field:"measured_at",   align:"left"   as const },
  { name:"bp",            label:"BP (mmHg)",   field:"bp",            align:"center" as const },
  { name:"heart_rate",    label:"HR (bpm)",    field:"heart_rate",    align:"center" as const },
  { name:"temperature",   label:"Temp (°C)",   field:"temperature",   align:"center" as const },
  { name:"weight",        label:"Weight (kg)", field:"weight",        align:"center" as const },
  { name:"spo2",          label:"SpO₂ (%)",    field:"spo2",          align:"center" as const },
  { name:"blood_sugar",   label:"Blood Sugar", field:"blood_sugar",   align:"center" as const },
  { name:"staff_name",    label:"Recorded by", field:"staff_name",    align:"left"   as const },
];

function onVitalResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredVitalOpts.value = residentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onVitalResSelected(opt: { label:string; value:number }|null) {
  vitalSelectedRes.value = opt?.value ?? null;
  vitalPagination.value.page = 1;
  loadVitals();
}
async function loadVitals() {
  vitalLoading.value = true;
  try {
    const result = await invoke<{ data:Vital[]; total:number }>("list_vitals", {
      residentId: vitalSelectedRes.value ?? null, showArchived: false,
      dateFrom: vitalDateFrom.value, dateTo: vitalDateTo.value,
      page: vitalPagination.value.page, pageSize: vitalPagination.value.rowsPerPage,
      sortBy: vitalPagination.value.sortBy || null, sortDesc: vitalPagination.value.descending,
    });
    vitals.value = result.data;
    vitalPagination.value.rowsNumber = result.total;
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { vitalLoading.value = false; }
}
function onVitalRequest(props: { pagination:{ page:number; rowsPerPage:number; sortBy:string; descending:boolean } }) {
  Object.assign(vitalPagination.value, props.pagination);
  loadVitals();
}
async function submitVital() {
  if (!vitalForm.value.resident_id) { $q.notify({ type:"negative", message:"Please select a resident." }); return; }
  vitalSubmitting.value = true;
  try {
    await invoke("create_vital", { input:{
      resident_id:  vitalForm.value.resident_id,
      bp_systolic:  vitalForm.value.bp_systolic  ? parseInt(vitalForm.value.bp_systolic)   : null,
      bp_diastolic: vitalForm.value.bp_diastolic ? parseInt(vitalForm.value.bp_diastolic)  : null,
      heart_rate:   vitalForm.value.heart_rate   ? parseInt(vitalForm.value.heart_rate)    : null,
      temperature:  vitalForm.value.temperature  ? parseFloat(vitalForm.value.temperature) : null,
      weight:       vitalForm.value.weight       ? parseFloat(vitalForm.value.weight)      : null,
      blood_sugar:  vitalForm.value.blood_sugar  ? parseFloat(vitalForm.value.blood_sugar) : null,
      spo2:         vitalForm.value.spo2         ? parseInt(vitalForm.value.spo2)          : null,
      notes:        vitalForm.value.notes || null,
    }});
    $q.notify({ type:"positive", message:"Vital signs recorded." });
    showVitalDialog.value = false;
    vitalForm.value = { resident_id:vitalSelectedRes.value, bp_systolic:"", bp_diastolic:"", heart_rate:"", temperature:"", weight:"", blood_sugar:"", spo2:"", notes:"" };
    vitalPagination.value.page = 1;
    await loadVitals();
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { vitalSubmitting.value = false; }
}
function confirmArchive(vital: Vital) {
  $q.dialog({ title:"Archive this record?", message:`Archive vital record for ${vital.resident_name}?`, cancel:{ label:"Cancel", flat:true }, ok:{ label:"Archive", color:"warning", unelevated:true }, persistent:true })
    .onOk(async () => {
      try {
        await invoke("archive_vital", { id:vital.id });
        $q.notify({ type:"positive", message:"Vital record archived." });
        await loadVitals();
      } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
    });
}
function confirmDeleteVital(vital: Vital) {
  $q.dialog({
    title: "Delete Record",
    message: `Permanently delete this vital record for ${vital.resident_name}? This cannot be undone.`,
    cancel: { label:"Cancel", flat:true },
    ok:     { label:"Delete", color:"negative", unelevated:true },
    persistent: true,
  }).onOk(async () => {
    try {
      await invoke("delete_vital", { id:vital.id, actorRole:auth.user?.role ?? "" });
      $q.notify({ type:"positive", message:"Vital record deleted." });
      await loadVitalHistory(false);
    } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  });
}
// Vitals history
const vitalHistory       = ref<Vital[]>([]);
const vitalHisLoading    = ref(false);
const vitalHisResFilter  = ref<{ label:string; value:number }|null>(null);
const vitalHisDateFrom   = ref<string|null>(null);
const vitalHisDateTo     = ref<string|null>(null);
const filteredVitalHisOpts = ref<{ label:string; value:number }[]>([]);
const vitalHisSelectedRes = ref<number|null>(null);
const vitalHisPagination = ref({ page:1, rowsPerPage:25, rowsNumber:0, sortBy:"measured_at", descending:true });

function onVitalHisResFilter(val: string, update: (fn:()=>void)=>void) {
  update(() => {
    const q = val.toLowerCase();
    filteredVitalHisOpts.value = residentOptions.value.filter(r => r.label.toLowerCase().includes(q));
  });
}
function onVitalHisResSelected(opt: { label:string; value:number }|null) {
  vitalHisSelectedRes.value = opt?.value ?? null;
  vitalHisPagination.value.page = 1;
  loadVitalHistory();
}
async function loadVitalHistory(reset = true) {
  if (reset) vitalHisPagination.value.page = 1;
  vitalHisLoading.value = true;
  try {
    const result = await invoke<{ data:Vital[]; total:number }>("list_vitals", {
      residentId: vitalHisSelectedRes.value ?? null, showArchived: true,
      dateFrom: vitalHisDateFrom.value, dateTo: vitalHisDateTo.value,
      page: vitalHisPagination.value.page, pageSize: vitalHisPagination.value.rowsPerPage,
      sortBy: vitalHisPagination.value.sortBy || null, sortDesc: vitalHisPagination.value.descending,
    });
    vitalHistory.value = result.data;
    vitalHisPagination.value.rowsNumber = result.total;
  } catch (e) { $q.notify({ type:"negative", message:`${e}` }); }
  finally { vitalHisLoading.value = false; }
}
function onVitalHisRequest(props: { pagination:{ page:number; rowsPerPage:number; sortBy:string; descending:boolean } }) {
  Object.assign(vitalHisPagination.value, props.pagination);
  loadVitalHistory(false);
}

// ── History section dispatcher ────────────────────────────────────────────────
function loadHistoryForSection(s: HistorySection) {
  if (s === "residents_h")    loadHistoryResidents();
  if (s === "carelog_h")      loadCareLogHistory();
  if (s === "medications_h")  loadMedHistory();
  if (s === "healthcharts_h") loadVitalHistory();
}

// ── Dynamic page title / subtitle ────────────────────────────────────────────
const sectionMeta = computed(() => {
  switch (section.value) {
    case "residents":    return { label: "Residents",     icon: "o_people",        color: "primary" };
    case "carelog":      return { label: "Care Log",      icon: "o_assignment",    color: "primary" };
    case "medications":  return { label: "Medications",   icon: "o_medication",    color: "teal" };
    case "healthcharts": return { label: "Health Charts", icon: "o_monitor_heart", color: "deep-purple" };
    case "history":      return { label: "History",       icon: "o_history",       color: "grey-7" };
    default:             return { label: "",              icon: "",                color: "grey" };
  }
});
const historySubMeta = computed(() => {
  switch (historySection.value) {
    case "residents_h":    return { label: "Residents",     icon: "o_people" };
    case "carelog_h":      return { label: "Care Log",      icon: "o_assignment" };
    case "medications_h":  return { label: "Medications",   icon: "o_medication" };
    case "healthcharts_h": return { label: "Health Charts", icon: "o_monitor_heart" };
    default:               return { label: "",              icon: "" };
  }
});
const pageSubtitle = computed(() => {
  switch (section.value) {
    case "residents":    return "Active resident management";
    case "carelog":      return "Daily care entries and incident tracking";
    case "medications":  return "Active medication management";
    case "healthcharts": return "Vital signs monitoring";
    case "history":      return "Archived and historical records";
    default:             return "";
  }
});

// ── Section watch ─────────────────────────────────────────────────────────────
watch(section, (s) => {
  if (s === "residents"    && residents.value.length === 0) loadActive();
  if (s === "carelog")     loadDaily();
  if (s === "medications") loadMedications();
  if (s === "healthcharts") loadVitals();
  if (s === "history")     loadHistoryForSection(historySection.value);
});
watch(historySection, (s) => {
  if (section.value === "history") loadHistoryForSection(s);
});

// ── Mount ─────────────────────────────────────────────────────────────────────
onMounted(async () => {
  await loadResidentList();
  const qSection  = route.query.section as string | undefined;
  const qResident = route.query.resident ? Number(route.query.resident) : null;

  const sectionMap: Record<string, Section> = {
    carelog:"carelog", medications:"medications",
    healthcharts:"healthcharts", history:"history",
  };
  if (qSection && sectionMap[qSection]) section.value = sectionMap[qSection];

  if (qResident) {
    const opt = residentOptions.value.find(r => r.value === qResident) ?? null;
    medSelectedRes.value    = qResident;
    vitalSelectedRes.value  = qResident;
    clForm.value.resident_id = qResident;
    clhResident.value       = qResident;
    medResFilter.value      = opt;
    vitalResFilter.value    = opt;
  }

  if (section.value === "residents")    await loadActive();
  else if (section.value === "carelog") await loadDaily();
  else if (section.value === "medications") await loadMedications();
  else if (section.value === "healthcharts") await loadVitals();
  else if (section.value === "history")  loadHistoryForSection(historySection.value);
});

// ── Export All ────────────────────────────────────────────────────────────────
const exporting = ref(false);

async function exportAll() {
  exporting.value = true;
  try {
    // Fetch all data in parallel
    const [
      activeRes,
      dischargedRes,
      deceasedRes,
      careLogsResult,
      careLogHistResult,
      medsActive,
      medsHistory,
      vitalsResult,
      vitalsHistResult,
    ] = await Promise.all([
      invoke<Resident[]>("list_residents", { search: null, activeOnly: null, status: "active" }),
      invoke<Resident[]>("list_residents", { search: null, activeOnly: null, status: "discharged" }),
      invoke<Resident[]>("list_residents", { search: null, activeOnly: null, status: "deceased" }),
      invoke<{ data: CareLog[]; total: number }>("list_care_logs_history", {
        residentId: null, dateFrom: null, dateTo: null, category: null,
        incidentOnly: false, page: 1, pageSize: 9999, sortBy: "logged_at", sortDesc: true,
      }),
      invoke<{ data: CareLog[]; total: number }>("list_care_logs_history", {
        residentId: null, dateFrom: null, dateTo: null, category: null,
        incidentOnly: true, page: 1, pageSize: 9999, sortBy: "logged_at", sortDesc: true,
      }),
      invoke<{ data: Medication[]; total: number }>("list_medications", {
        residentId: null, activeOnly: true, page: 1, pageSize: 9999, sortBy: "start_date", sortDesc: true,
      }),
      invoke<{ data: Medication[]; total: number }>("list_medications", {
        residentId: null, activeOnly: false, page: 1, pageSize: 9999, sortBy: "start_date", sortDesc: true,
      }),
      invoke<{ data: Vital[]; total: number }>("list_vitals", {
        residentId: null, showArchived: false, page: 1, pageSize: 9999, sortBy: "measured_at", sortDesc: true,
      }),
      invoke<{ data: Vital[]; total: number }>("list_vitals", {
        residentId: null, showArchived: true, page: 1, pageSize: 9999, sortBy: "measured_at", sortDesc: true,
      }),
    ]);

    const historyRes = [...(dischargedRes as Resident[]), ...(deceasedRes as Resident[])];

    // ── Sheet builders ────────────────────────────────────────────────────────
    function sheetResidents(rows: Resident[]) {
      return rows.map(r => ({
        "ID": r.id,
        "First Name": r.first_name,
        "Last Name": r.last_name,
        "Date of Birth": r.date_of_birth,
        "Gender": r.gender,
        "Room": r.room_number ?? "",
        "Care Level": r.care_grade ?? "",
        "Admission Date": r.admission_date,
        "Discharge Date": r.discharge_date ?? "",
        "Status": r.is_deceased ? "Deceased" : r.is_active ? "Active" : "Discharged",
        "Diagnosis": r.primary_diagnosis ?? "",
        "Allergies": r.allergies ?? "",
        "Dietary": r.dietary_restrictions ?? "",
        "Insurance #": r.insurance_number ?? "",
        "Notes": r.notes ?? "",
      }));
    }

    function sheetCareLogs(rows: CareLog[]) {
      return rows.map(r => ({
        "ID": r.id,
        "Resident": r.resident_name,
        "Staff": r.staff_name ?? "",
        "Shift": capitalize(r.shift),
        "Category": r.category,
        "Notes": r.content,
        "Incident": r.is_incident ? "Yes" : "No",
        "Flagged": r.is_flagged ? "Yes" : "No",
        "Date/Time": r.logged_at,
      }));
    }

    function sheetMedications(rows: Medication[]) {
      return rows.map(r => ({
        "ID": r.id,
        "Resident": r.resident_name,
        "Medication": r.name,
        "Dosage": r.dosage,
        "Frequency": r.frequency,
        "Route": r.route,
        "Start Date": r.start_date,
        "Prescriber": r.prescriber ?? "",
        "Instructions": r.instructions ?? "",
        "Active": r.is_active ? "Yes" : "No",
      }));
    }

    function sheetVitals(rows: Vital[]) {
      return rows.map(r => ({
        "ID": r.id,
        "Resident": r.resident_name,
        "BP": r.bp_systolic != null && r.bp_diastolic != null ? `${r.bp_systolic}/${r.bp_diastolic}` : "",
        "Heart Rate": r.heart_rate ?? "",
        "Temp (°C)": r.temperature ?? "",
        "Weight (kg)": r.weight ?? "",
        "Blood Sugar": r.blood_sugar ?? "",
        "SpO2 (%)": r.spo2 ?? "",
        "Notes": r.notes ?? "",
        "Measured At": r.measured_at,
        "Staff": r.staff_name ?? "",
      }));
    }

    // ── Build workbook ────────────────────────────────────────────────────────
    const wb = XLSX.utils.book_new();

    const sheets: [string, unknown[]][] = [
      ["Residents",             sheetResidents(activeRes as Resident[])],
      ["Residents History",     sheetResidents(historyRes)],
      ["Care Logs",             sheetCareLogs((careLogsResult as any).data)],
      ["Incident Reports",      sheetCareLogs((careLogHistResult as any).data)],
      ["Medications",           sheetMedications((medsActive as any).data)],
      ["Medications History",   sheetMedications((medsHistory as any).data)],
      ["Health Charts",         sheetVitals((vitalsResult as any).data)],
      ["Health Charts History", sheetVitals((vitalsHistResult as any).data)],
    ];

    for (const [name, rows] of sheets) {
      const ws = XLSX.utils.json_to_sheet(rows.length ? rows : [{}]);
      // Auto column widths
      const colWidths = rows.length
        ? Object.keys(rows[0] as object).map(k => ({ wch: Math.max(k.length, 12) }))
        : [];
      ws["!cols"] = colWidths;
      XLSX.utils.book_append_sheet(wb, ws, name);
    }

    // ── Save via native dialog ────────────────────────────────────────────────
    const today = new Date().toISOString().slice(0, 10);
    const filename = `residents-export-${today}.xlsx`;
    const buffer: ArrayBuffer = XLSX.write(wb, { type: "array", bookType: "xlsx" });
    const bytes = Array.from(new Uint8Array(buffer));

    const saved = await invoke<string | null>("save_excel", { filename, data: bytes });
    if (saved) {
      $q.notify({ type: "positive", message: "Export saved successfully.", icon: "o_download_done" });
    }
  } catch (e) {
    $q.notify({ type: "negative", message: `Export failed: ${e}` });
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <q-page class="q-pa-lg">

    <!-- ── Page title ────────────────────────────────────────────────────────── -->
    <div class="row items-center q-mb-sm q-gutter-x-xs" style="flex-wrap:nowrap">
      <q-icon name="o_people" size="1.4rem" color="grey-6" />
      <span class="text-subtitle1 text-grey-6 text-weight-medium">Residents</span>
      <span class="text-grey-4 text-subtitle1 q-mx-xs">—</span>
      <template v-if="section === 'history'">
        <q-icon name="o_history" size="1.4rem" color="grey-6" />
        <span class="text-subtitle1 text-grey-6 text-weight-medium">History</span>
        <span class="text-grey-4 text-subtitle1 q-mx-xs">—</span>
        <q-icon :name="historySubMeta.icon" size="1.4rem" color="grey-8" />
        <span class="text-h5 text-weight-bold">{{ historySubMeta.label }}</span>
      </template>
      <template v-else>
        <q-icon :name="sectionMeta.icon" size="1.4rem" :color="sectionMeta.color" />
        <span class="text-h5 text-weight-bold">{{ sectionMeta.label }}</span>
      </template>
      <q-space />
      <q-btn
        outline color="primary" icon="o_download" label="Export All"
        dense :loading="exporting" @click="exportAll"
      />
    </div>
    <div class="text-caption text-grey-6 q-mb-sm">{{ pageSubtitle }}</div>

    <!-- ── Section tabs ─────────────────────────────────────────────────────── -->
    <q-tabs v-model="section" align="left" indicator-color="primary" active-color="primary"
            dense class="q-mb-lg section-tabs">
      <q-tab name="residents"    icon="o_people"        label="Residents"     />
      <q-tab name="carelog"      icon="o_assignment"    label="Care Log"      />
      <q-tab name="medications"  icon="o_medication"    label="Medications"   />
      <q-tab name="healthcharts" icon="o_monitor_heart" label="Health Charts" />
      <!-- Hidden sentinel so q-tabs doesn't recalibrate when section = "history" -->
      <q-tab name="history" style="display:none;width:0;min-width:0;padding:0;overflow:hidden;" />
      <!-- History tab: click opens dropdown to pick sub-section -->
      <q-btn v-if="!isStaff" flat no-caps dense
             icon="o_history" label="History"
             :class="['history-tab-btn', section === 'history' ? 'history-tab-btn--active' : '']">
        <q-menu auto-close>
          <q-list dense style="min-width:180px">
            <q-item v-for="opt in historySectionOptions" :key="opt.value"
                    clickable
                    @click="historySection = opt.value as HistorySection; section = 'history'; loadHistoryForSection(opt.value as HistorySection)">
              <q-item-section avatar><q-icon :name="opt.icon" size="sm" /></q-item-section>
              <q-item-section>{{ opt.label }}</q-item-section>
            </q-item>
          </q-list>
        </q-menu>
      </q-btn>
    </q-tabs>

    <!-- ══════════════════════════════════════════════════════════════════════
         SECTION: RESIDENTS
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-if="section === 'residents'">
      <!-- Search + filters + action — single row -->
      <div class="row items-center q-mb-md q-gutter-sm">
        <!-- Name search -->
        <div class="col-12 col-sm-3">
          <q-select v-model="resSearchFilter" :options="filteredResOpts"
                    label="Search by name…" outlined dense use-input input-debounce="150" clearable
                    @filter="onResFilter" @update:model-value="onResSelected">
            <template #prepend><q-icon name="o_search" /></template>
            <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
          </q-select>
        </div>
        <!-- Care level -->
        <div class="col-auto">
          <q-select v-model="filterCareGrade"
                    :options="[{label:'All levels',value:null},{label:'Level 1',value:1},{label:'Level 2',value:2},{label:'Level 3',value:3},{label:'Level 4',value:4},{label:'Level 5',value:5}]"
                    label="Care level" outlined dense emit-value map-options options-dense
                    style="min-width:130px"
                    @update:model-value="resPagination.page = 1; loadActive()">
            <template #prepend><q-icon name="o_favorite" size="xs" /></template>
          </q-select>
        </div>
        <!-- Gender -->
        <div class="col-auto">
          <q-select v-model="filterGender"
                    :options="[{label:'All genders',value:null},{label:'Male',value:'male'},{label:'Female',value:'female'},{label:'Other',value:'other'}]"
                    label="Gender" outlined dense emit-value map-options options-dense
                    style="min-width:130px"
                    @update:model-value="resPagination.page = 1; loadActive()">
            <template #prepend><q-icon name="o_person" size="xs" /></template>
          </q-select>
        </div>
        <!-- Cognitive support -->
        <div class="col-auto">
          <q-select v-model="filterCognitive"
                    :options="[{label:'All',value:null},{label:'Cognitive: Yes',value:true},{label:'Cognitive: No',value:false}]"
                    label="Cognitive" outlined dense emit-value map-options options-dense
                    style="min-width:130px"
                    @update:model-value="resPagination.page = 1; loadActive()">
            <template #prepend><q-icon name="o_psychology" size="xs" /></template>
          </q-select>
        </div>
        <q-space />
        <div v-if="!isStaff" class="col-auto">
          <q-btn color="secondary" icon="o_add" label="Add Resident" unelevated @click="openAdd" />
        </div>
      </div>
      <!-- Table -->
      <template v-if="loadingActive">
        <q-card flat bordered class="q-pa-md">
          <q-skeleton type="rect" height="40px" class="q-mb-sm" />
          <q-skeleton type="rect" height="40px" class="q-mb-sm" v-for="n in 8" :key="n" />
        </q-card>
      </template>
      <q-card v-else flat bordered>
        <q-table :rows="residents" :columns="activeResCols" row-key="id" flat
                 v-model:pagination="resPagination"
                 :rows-per-page-options="[25,50,100]"
                 @request="onResRequest"
                 @row-click="(_evt,row) => openSummary(row)"
                 style="cursor:pointer">
          <template #body-cell-name="{ row }">
            <q-td>
              <div class="text-weight-medium">{{ row.first_name }} {{ row.last_name }}</div>
              <div class="text-caption text-grey">{{ age(row.date_of_birth) }} yrs · {{ row.gender }}</div>
            </q-td>
          </template>
          <template #body-cell-room="{ row }">
            <q-td class="text-center">
              <q-chip v-if="row.room_number" dense square color="primary" text-color="white" size="sm">{{ row.room_number }}</q-chip>
              <span v-else class="text-grey">—</span>
            </q-td>
          </template>
          <template #body-cell-admission_date="{ row }">
            <q-td>
              <div class="text-caption">{{ row.admission_date }}</div>
              <div class="text-caption text-grey">{{ stayDuration(row.admission_date) }}</div>
            </q-td>
          </template>
          <template #body-cell-care_grade="{ row }">
            <q-td class="text-center">
              <q-badge v-if="row.care_grade" :color="careGradeColor(row.care_grade)"
                       :label="careLevelLabel[row.care_grade] ?? `Level ${row.care_grade}`" />
              <span v-else class="text-grey">—</span>
            </q-td>
          </template>
          <template #body-cell-actions="{ row }">
            <q-td class="text-right">
              <template v-if="!isStaff">
                <q-btn flat round dense icon="o_edit" size="sm" @click.stop="openEdit(row)"><q-tooltip>Edit</q-tooltip></q-btn>
                <q-btn flat round dense icon="o_logout" size="sm" color="warning" @click.stop="confirmDischarge(row)"><q-tooltip>Discharge</q-tooltip></q-btn>
                <q-btn flat round dense icon="o_heart_broken" size="sm" color="grey-6" @click.stop="confirmDeceased(row)"><q-tooltip>Mark as Deceased</q-tooltip></q-btn>
              </template>
              <q-btn flat round dense icon="o_assignment" size="sm" color="primary"
                     @click.stop="jumpToCareLogHistory(row.id)"><q-tooltip>Care Log History</q-tooltip></q-btn>
              <q-btn flat round dense icon="o_medication" size="sm" color="teal"
                     @click.stop="() => { medSelectedRes = row.id; const opt = residentOptions.find(r=>r.value===row.id)??null; medResFilter = opt; section = 'medications'; loadMedications(); }">
                <q-tooltip>Medications</q-tooltip>
              </q-btn>
              <q-btn flat round dense icon="o_monitor_heart" size="sm" color="deep-purple"
                     @click.stop="() => { vitalSelectedRes = row.id; const opt = residentOptions.find(r=>r.value===row.id)??null; vitalResFilter = opt; section = 'healthcharts'; loadVitals(); }">
                <q-tooltip>Health Charts</q-tooltip>
              </q-btn>
            </q-td>
          </template>
          <template #no-data>
            <div class="full-width column flex-center q-py-xl">
              <q-icon name="o_people" size="3rem" color="grey-4" />
              <div class="text-grey-5 q-mt-sm">No active residents found</div>
            </div>
          </template>
        </q-table>
      </q-card>
    </template>

    <!-- ══════════════════════════════════════════════════════════════════════
         SECTION: CARE LOG
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-if="section === 'carelog'">
      <!-- Search + filters + action -->
      <div class="row items-center q-mb-md q-gutter-sm">
        <div class="col-12 col-sm-3">
          <q-select v-model="clResFilter" :options="filteredClOpts"
                    label="Search by name…" outlined dense use-input input-debounce="150" clearable
                    @filter="onClResFilter" @update:model-value="onClResSelected">
            <template #prepend><q-icon name="o_search" /></template>
            <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
          </q-select>
        </div>
        <div class="col-auto">
          <q-select v-model="clCategory"
                    :options="[{label:'All categories',value:null},{label:'Note',value:'note'},{label:'Incident',value:'incident'},{label:'Medical',value:'medical'},{label:'Personal',value:'personal'},{label:'Behavioral',value:'behavioral'}]"
                    label="Category" outlined dense emit-value map-options options-dense style="min-width:140px"
                    @update:model-value="loadDaily()">
            <template #prepend><q-icon name="o_label" size="xs" /></template>
          </q-select>
        </div>
        <div class="col-auto">
          <q-select v-model="clShift"
                    :options="[{label:'All shifts',value:null}, ...shiftOptions]"
                    label="Shift" outlined dense emit-value map-options options-dense style="min-width:120px"
                    @update:model-value="loadDaily()">
            <template #prepend><q-icon name="o_schedule" size="xs" /></template>
          </q-select>
        </div>
        <q-space />
        <q-btn color="primary" icon="o_add" label="New Entry" unelevated dense
               @click="() => { clForm.resident_id = clSelectedRes ?? null; showEntryDialog = true; }" />
      </div>
      <!-- Table -->
      <q-table :rows="dailyLogs" :columns="dailyCols" row-key="id" flat bordered
               :loading="dailyLoading"
               :rows-per-page-options="[25,50,100,0]"
               :pagination="{ rowsPerPage:25, sortBy:'logged_at', descending:true }">
        <template #body-cell-resident_name="props">
          <q-td :props="props">
            <span class="text-weight-medium">{{ props.row.resident_name }}</span>
          </q-td>
        </template>
        <template #body-cell-shift="props">
          <q-td :props="props">
            <q-badge :color="SHIFT_COLOR[props.row.shift] ?? 'grey'" :label="capitalize(props.row.shift)" />
          </q-td>
        </template>
        <template #body-cell-category="props">
          <q-td :props="props">
            <q-badge :color="CATEGORY_COLOR[props.row.category]" :label="capitalize(props.row.category)" />
          </q-td>
        </template>
        <template #body-cell-content="props">
          <q-td :props="props" style="white-space:normal;max-width:400px;word-break:break-word">
            {{ props.row.content }}
            <q-badge v-if="props.row.is_incident" color="red" label="Incident" class="q-ml-xs" />
          </q-td>
        </template>
        <template #body-cell-staff_name="props">
          <q-td :props="props"><span class="text-caption text-grey-7">{{ props.row.staff_name ?? "—" }}</span></q-td>
        </template>
        <template #body-cell-logged_at="props">
          <q-td :props="props"><span class="text-caption">{{ formatDateTime(props.row.logged_at) }}</span></q-td>
        </template>
        <template #body-cell-actions="props">
          <q-td :props="props" class="text-center">
            <q-btn v-if="canEditLog(props.row)" flat round dense icon="o_edit" color="primary" size="sm" @click="openEditLog(props.row)"><q-tooltip>Edit</q-tooltip></q-btn>
            <q-btn v-if="canDelete" flat round dense icon="o_delete" color="negative" size="sm" @click="deleteLog(props.row.id)"><q-tooltip>Delete</q-tooltip></q-btn>
          </q-td>
        </template>
        <template #no-data>
          <div class="full-width column flex-center q-py-xl">
            <q-icon name="o_assignment" size="3rem" color="grey-4" />
            <div class="text-grey-5 q-mt-sm">No care logs{{ selectedClResObj ? ` for ${selectedClResObj.first_name} ${selectedClResObj.last_name}` : '' }}</div>
          </div>
        </template>
      </q-table>
    </template>

    <!-- ══════════════════════════════════════════════════════════════════════
         SECTION: MEDICATIONS
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-if="section === 'medications'">
      <!-- Search + filters + action -->
      <div class="row items-center q-mb-md q-gutter-sm">
        <div class="col-12 col-sm-3">
          <q-select v-model="medResFilter" :options="filteredMedOpts"
                    label="Search by name…" outlined dense use-input input-debounce="150" clearable
                    @filter="onMedResFilter" @update:model-value="onMedResSelected">
            <template #prepend><q-icon name="o_search" /></template>
            <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
          </q-select>
        </div>
        <div class="col-auto">
          <q-select v-model="medRoute"
                    :options="[{label:'All routes',value:null},{label:'Oral',value:'oral'},{label:'Injection',value:'injection'},{label:'Topical',value:'topical'},{label:'Inhaled',value:'inhaled'}]"
                    label="Route" outlined dense emit-value map-options options-dense style="min-width:130px"
                    @update:model-value="medPagination.page = 1; loadMedications()">
            <template #prepend><q-icon name="o_medication" size="xs" /></template>
          </q-select>
        </div>
        <q-space />
        <div v-if="!isStaff" class="col-auto">
          <q-btn color="teal" icon="o_add" label="Add Medication" unelevated dense
                 @click="() => { medForm.resident_id = medSelectedRes; showAddMedDialog = true; }" />
        </div>
      </div>
      <!-- Table -->
      <template v-if="medLoading">
        <q-skeleton type="rect" height="40px" class="q-mb-sm" />
        <q-skeleton type="rect" height="44px" class="q-mb-sm" v-for="n in 6" :key="n" />
      </template>
      <q-table v-else :rows="medications" :columns="medCols" row-key="id" flat bordered
               v-model:pagination="medPagination" :rows-per-page-options="[10,25,50,100]"
               @request="onMedRequest">
        <template #body-cell-route="props">
          <q-td :props="props"><q-badge :color="routeColor(props.row.route)" :label="props.row.route" class="text-capitalize" /></q-td>
        </template>
        <template #body-cell-actions="props">
          <q-td :props="props" class="text-center">
            <q-btn v-if="!isStaff" flat round dense icon="o_stop_circle" color="negative"
                   @click.stop="confirmStopMed(props.row)"><q-tooltip>Stop medication</q-tooltip></q-btn>
          </q-td>
        </template>
        <template #no-data>
          <div class="full-width column flex-center q-py-xl">
            <q-icon name="o_medication_liquid" size="3rem" color="grey-4" />
            <div class="text-grey-5 q-mt-sm">No active medications{{ medSelectedRes ? ' for this resident' : '' }}</div>
          </div>
        </template>
      </q-table>
    </template>

    <!-- ══════════════════════════════════════════════════════════════════════
         SECTION: HEALTH CHARTS
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-if="section === 'healthcharts'">
      <!-- Search + filters + action -->
      <div class="row items-center q-mb-md q-gutter-sm">
        <div class="col-12 col-sm-3">
          <q-select v-model="vitalResFilter" :options="filteredVitalOpts"
                    label="Search by name…" outlined dense use-input input-debounce="150" clearable
                    @filter="onVitalResFilter" @update:model-value="onVitalResSelected">
            <template #prepend><q-icon name="o_search" /></template>
            <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
          </q-select>
        </div>
        <div class="col-auto">
          <div class="cursor-pointer">
            <q-input v-model="vitalDateFrom" label="From" outlined dense readonly style="min-width:130px; pointer-events:none">
              <template #prepend><q-icon name="o_calendar_today" size="xs" /></template>
            </q-input>
            <q-popup-proxy cover transition-show="scale" transition-hide="scale">
              <q-date v-model="vitalDateFrom" mask="YYYY-MM-DD" minimal @update:model-value="vitalPagination.page = 1; loadVitals()">
                <div class="row items-center justify-end q-pa-sm">
                  <q-btn v-close-popup label="OK" color="primary" flat dense />
                </div>
              </q-date>
            </q-popup-proxy>
          </div>
        </div>
        <div class="col-auto">
          <div class="cursor-pointer">
            <q-input v-model="vitalDateTo" label="To" outlined dense readonly style="min-width:130px; pointer-events:none">
              <template #prepend><q-icon name="o_event" size="xs" /></template>
            </q-input>
            <q-popup-proxy cover transition-show="scale" transition-hide="scale">
              <q-date v-model="vitalDateTo" mask="YYYY-MM-DD" minimal @update:model-value="vitalPagination.page = 1; loadVitals()">
                <div class="row items-center justify-end q-pa-sm">
                  <q-btn v-close-popup label="OK" color="primary" flat dense />
                </div>
              </q-date>
            </q-popup-proxy>
          </div>
        </div>
        <q-space />
        <div v-if="!isStaff" class="col-auto">
          <q-btn color="deep-purple" icon="o_add" label="Record Vitals" unelevated dense
                 @click="() => { vitalForm.resident_id = vitalSelectedRes; showVitalDialog = true; }" />
        </div>
      </div>
      <!-- Table -->
      <template v-if="vitalLoading">
        <q-skeleton type="rect" height="40px" class="q-mb-sm" />
        <q-skeleton type="rect" height="44px" class="q-mb-sm" v-for="n in 6" :key="n" />
      </template>
      <q-table v-else :rows="vitals" :columns="vitalCols" row-key="id" flat bordered
               v-model:pagination="vitalPagination" :rows-per-page-options="[10,25,50,100]"
               @request="onVitalRequest">
        <template #body-cell-resident_name="props">
          <q-td :props="props"><span class="text-weight-medium">{{ props.row.resident_name }}</span></q-td>
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
        <template #body-cell-heart_rate="props"><q-td :props="props">{{ props.row.heart_rate ?? "—" }}</q-td></template>
        <template #body-cell-temperature="props"><q-td :props="props">{{ fmtNum(props.row.temperature) }}</q-td></template>
        <template #body-cell-weight="props"><q-td :props="props">{{ fmtNum(props.row.weight) }}</q-td></template>
        <template #body-cell-spo2="props">
          <q-td :props="props"><span :class="spo2Color(props.row)">{{ props.row.spo2 != null ? props.row.spo2 + "%" : "—" }}</span></q-td>
        </template>
        <template #body-cell-blood_sugar="props"><q-td :props="props">{{ fmtNum(props.row.blood_sugar) }}</q-td></template>
        <template #body-cell-staff_name="props">
          <q-td :props="props"><span v-if="props.row.staff_name">{{ props.row.staff_name }}</span><span v-else class="text-grey">—</span></q-td>
        </template>
        <template #body-cell-actions="props">
          <q-td :props="props" class="text-center">
            <q-btn v-if="!isStaff" flat round dense icon="o_archive" color="warning" size="sm"
                   @click.stop="confirmArchive(props.row)"><q-tooltip>Archive record</q-tooltip></q-btn>
          </q-td>
        </template>
        <template #no-data>
          <div class="full-width column flex-center q-py-xl">
            <q-icon name="o_monitor_heart" size="3rem" color="grey-4" />
            <div class="text-grey-5 q-mt-sm">{{ vitalSelectedRes ? 'No vital records for this resident' : 'No vital records yet' }}</div>
          </div>
        </template>
      </q-table>
    </template>

    <!-- ══════════════════════════════════════════════════════════════════════
         SECTION: HISTORY (manager only)
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-if="section === 'history' && !isStaff">

      <!-- ── Residents History ── -->
      <template v-if="historySection === 'residents_h'">
        <div class="row items-center q-mb-md q-gutter-sm">
          <div class="col-12 col-sm-3">
            <q-select v-model="hisResSearchFilter" :options="filteredHisResOpts"
                      label="Search by name…" outlined dense use-input input-debounce="150" clearable
                      @filter="onHisResFilter" @update:model-value="onHisResSelected">
              <template #prepend><q-icon name="o_search" /></template>
              <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
            </q-select>
          </div>
          <div class="col-auto">
            <q-select v-model="hisFilterStatus"
                      :options="[{label:'All statuses',value:null},{label:'Discharged',value:'discharged'},{label:'Deceased',value:'deceased'}]"
                      label="Status" outlined dense emit-value map-options options-dense style="min-width:135px">
              <template #prepend><q-icon name="o_info" size="xs" /></template>
            </q-select>
          </div>
          <div class="col-auto">
            <q-select v-model="hisFilterGender"
                      :options="[{label:'All genders',value:null},{label:'Male',value:'male'},{label:'Female',value:'female'},{label:'Other',value:'other'}]"
                      label="Gender" outlined dense emit-value map-options options-dense style="min-width:130px">
              <template #prepend><q-icon name="o_person" size="xs" /></template>
            </q-select>
          </div>
          <div class="col-auto">
            <q-select v-model="hisFilterCareGrade"
                      :options="[{label:'All levels',value:null},{label:'Level 1',value:1},{label:'Level 2',value:2},{label:'Level 3',value:3},{label:'Level 4',value:4},{label:'Level 5',value:5}]"
                      label="Care level" outlined dense emit-value map-options options-dense style="min-width:130px">
              <template #prepend><q-icon name="o_favorite" size="xs" /></template>
            </q-select>
          </div>
          <q-space />
        </div>
        <template v-if="loadingHisRes">
          <q-card flat bordered class="q-pa-md">
            <q-skeleton type="rect" height="40px" class="q-mb-sm" />
            <q-skeleton type="rect" height="40px" class="q-mb-sm" v-for="n in 6" :key="n" />
          </q-card>
        </template>
        <q-card v-else flat bordered>
          <q-table :rows="filteredHistoryResidents" :columns="historyResCols" row-key="id" flat
                   :rows-per-page-options="[20,50,100]">
            <template #body-cell-name="{ row }">
              <q-td>
                <div class="text-weight-medium">{{ row.first_name }} {{ row.last_name }}</div>
                <div class="text-caption text-grey">{{ age(row.date_of_birth) }} yrs · {{ row.gender }}</div>
              </q-td>
            </template>
            <template #body-cell-status="{ row }">
              <q-td class="text-center">
                <q-badge v-if="row.is_deceased" color="grey-7" label="Deceased" />
                <q-badge v-else color="orange-6" label="Discharged" />
              </q-td>
            </template>
            <template #body-cell-room="{ row }">
              <q-td class="text-center">
                <q-chip v-if="row.room_number" dense square color="grey-5" text-color="white" size="sm">{{ row.room_number }}</q-chip>
                <span v-else class="text-grey">—</span>
              </q-td>
            </template>
            <template #body-cell-admission_date="{ row }">
              <q-td>
                <div class="text-caption">{{ row.admission_date }}</div>
                <div class="text-caption text-grey">{{ stayDuration(row.admission_date, row.discharge_date) }}</div>
              </q-td>
            </template>
            <template #body-cell-discharge_date="{ row }">
              <q-td>
                <span v-if="row.discharge_date" class="text-caption">{{ row.discharge_date }}</span>
                <span v-else class="text-grey-4">—</span>
              </q-td>
            </template>
            <template #no-data>
              <div class="full-width column flex-center q-py-xl">
                <q-icon name="o_history" size="3rem" color="grey-4" />
                <div class="text-grey-5 q-mt-sm">No records found</div>
              </div>
            </template>
          </q-table>
        </q-card>
      </template>

      <!-- ── Care Log History ── -->
      <template v-if="historySection === 'carelog_h'">
        <div class="row items-center q-mb-md q-gutter-sm">
          <div class="col-12 col-sm-3">
            <q-select v-model="clhResFilter" :options="filteredClhOpts"
                      label="Search by name…" outlined dense use-input input-debounce="150" clearable
                      @filter="onClhResFilter" @update:model-value="onClhResSelected">
              <template #prepend><q-icon name="o_search" /></template>
              <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
            </q-select>
          </div>
          <div class="col-auto">
            <q-select v-model="clhCategory"
                      :options="[{label:'All categories',value:null},{label:'Note',value:'note'},{label:'Incident',value:'incident'},{label:'Medical',value:'medical'},{label:'Personal',value:'personal'},{label:'Behavioral',value:'behavioral'}]"
                      label="Category" outlined dense emit-value map-options options-dense style="min-width:140px"
                      @update:model-value="loadCareLogHistory()">
              <template #prepend><q-icon name="o_label" size="xs" /></template>
            </q-select>
          </div>
          <div class="col-auto">
            <q-select v-model="clhShift"
                      :options="[{label:'All shifts',value:null}, ...shiftOptions]"
                      label="Shift" outlined dense emit-value map-options options-dense style="min-width:120px"
                      @update:model-value="loadCareLogHistory()">
              <template #prepend><q-icon name="o_schedule" size="xs" /></template>
            </q-select>
          </div>
          <div class="col-auto">
            <div class="cursor-pointer">
              <q-input v-model="clhDateFrom" label="From" outlined dense readonly style="min-width:130px; pointer-events:none">
                <template #prepend><q-icon name="o_calendar_today" size="xs" /></template>
              </q-input>
              <q-popup-proxy cover transition-show="scale" transition-hide="scale">
                <q-date v-model="clhDateFrom" mask="YYYY-MM-DD" minimal @update:model-value="loadCareLogHistory()">
                  <div class="row items-center justify-end q-pa-sm"><q-btn v-close-popup label="OK" color="primary" flat dense /></div>
                </q-date>
              </q-popup-proxy>
            </div>
          </div>
          <div class="col-auto">
            <div class="cursor-pointer">
              <q-input v-model="clhDateTo" label="To" outlined dense readonly style="min-width:130px; pointer-events:none">
                <template #prepend><q-icon name="o_event" size="xs" /></template>
              </q-input>
              <q-popup-proxy cover transition-show="scale" transition-hide="scale">
                <q-date v-model="clhDateTo" mask="YYYY-MM-DD" minimal @update:model-value="loadCareLogHistory()">
                  <div class="row items-center justify-end q-pa-sm"><q-btn v-close-popup label="OK" color="primary" flat dense /></div>
                </q-date>
              </q-popup-proxy>
            </div>
          </div>
          <div class="col-auto">
            <q-toggle v-model="clhIncidentOnly" label="Incidents only" dense color="negative"
                      @update:model-value="loadCareLogHistory()" />
          </div>
          <q-space />
        </div>
        <q-table :rows="clHistory" :columns="clhCols" row-key="id" flat bordered
                 :loading="clhLoading" v-model:pagination="clhPagination"
                 :rows-per-page-options="[10,25,50,100]" @request="onClhRequest">
          <template #body-cell-shift="props">
            <q-td :props="props"><q-badge :color="SHIFT_COLOR[props.row.shift]??'grey'" :label="capitalize(props.row.shift)" /></q-td>
          </template>
          <template #body-cell-category="props">
            <q-td :props="props"><q-badge :color="CATEGORY_COLOR[props.row.category]" :label="capitalize(props.row.category)" /></q-td>
          </template>
          <template #body-cell-content="props">
            <q-td :props="props" style="white-space:normal;max-width:380px;word-break:break-word">{{ props.row.content }}</q-td>
          </template>
          <template #body-cell-staff_name="props">
            <q-td :props="props"><span class="text-caption text-grey-7">{{ props.row.staff_name ?? "—" }}</span></q-td>
          </template>
          <template #body-cell-logged_at="props">
            <q-td :props="props">
              <div class="text-caption">{{ fmtDate(props.row.logged_at) }}</div>
              <div class="text-caption text-grey-5">{{ fmtTime(props.row.logged_at) }}</div>
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

      <!-- ── Medications History ── -->
      <template v-if="historySection === 'medications_h'">
        <div class="row items-center q-mb-md q-gutter-sm">
          <div class="col-12 col-sm-3">
            <q-select v-model="medHisResFilter" :options="filteredMedHisOpts"
                      label="Search by name…" outlined dense use-input input-debounce="150" clearable
                      @filter="onMedHisResFilter" @update:model-value="onMedHisResSelected">
              <template #prepend><q-icon name="o_search" /></template>
              <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
            </q-select>
          </div>
          <div class="col-auto">
            <q-select v-model="medHisRoute"
                      :options="[{label:'All routes',value:null},{label:'Oral',value:'oral'},{label:'Injection',value:'injection'},{label:'Topical',value:'topical'},{label:'Inhaled',value:'inhaled'}]"
                      label="Route" outlined dense emit-value map-options options-dense style="min-width:130px"
                      @update:model-value="medHisPagination.page = 1; loadMedHistory()">
              <template #prepend><q-icon name="o_medication" size="xs" /></template>
            </q-select>
          </div>
        </div>
        <q-table :rows="medHistory" :columns="medCols" row-key="id" flat bordered
                 :loading="medHisLoading" v-model:pagination="medHisPagination"
                 :rows-per-page-options="[10,25,50,100]" @request="onMedHisRequest">
          <template #body-cell-route="props">
            <q-td :props="props"><q-badge :color="routeColor(props.row.route)" :label="props.row.route" class="text-capitalize" /></q-td>
          </template>
          <template #body-cell-end_date="props">
            <q-td :props="props">
              <span v-if="props.row.end_date" class="text-caption">{{ props.row.end_date }}</span>
              <span v-else class="text-grey-4">—</span>
            </q-td>
          </template>
          <template #no-data>
            <div class="full-width column flex-center q-py-xl">
              <q-icon name="o_medication_liquid" size="3rem" color="grey-4" />
              <div class="text-grey-5 q-mt-sm">No stopped medications found</div>
            </div>
          </template>
        </q-table>
      </template>

      <!-- ── Health Charts History ── -->
      <template v-if="historySection === 'healthcharts_h'">
        <div class="row items-center q-mb-md q-gutter-sm">
          <div class="col-12 col-sm-3">
            <q-select v-model="vitalHisResFilter" :options="filteredVitalHisOpts"
                      label="Search by name…" outlined dense use-input input-debounce="150" clearable
                      @filter="onVitalHisResFilter" @update:model-value="onVitalHisResSelected">
              <template #prepend><q-icon name="o_search" /></template>
              <template #no-option><q-item><q-item-section class="text-grey">No residents found</q-item-section></q-item></template>
            </q-select>
          </div>
          <div class="col-auto">
            <div class="cursor-pointer">
              <q-input v-model="vitalHisDateFrom" label="From" outlined dense readonly style="min-width:130px; pointer-events:none">
                <template #prepend><q-icon name="o_calendar_today" size="xs" /></template>
              </q-input>
              <q-popup-proxy cover transition-show="scale" transition-hide="scale">
                <q-date v-model="vitalHisDateFrom" mask="YYYY-MM-DD" minimal @update:model-value="vitalHisPagination.page = 1; loadVitalHistory()">
                  <div class="row items-center justify-end q-pa-sm"><q-btn v-close-popup label="OK" color="primary" flat dense /></div>
                </q-date>
              </q-popup-proxy>
            </div>
          </div>
          <div class="col-auto">
            <div class="cursor-pointer">
              <q-input v-model="vitalHisDateTo" label="To" outlined dense readonly style="min-width:130px; pointer-events:none">
                <template #prepend><q-icon name="o_event" size="xs" /></template>
              </q-input>
              <q-popup-proxy cover transition-show="scale" transition-hide="scale">
                <q-date v-model="vitalHisDateTo" mask="YYYY-MM-DD" minimal @update:model-value="vitalHisPagination.page = 1; loadVitalHistory()">
                  <div class="row items-center justify-end q-pa-sm"><q-btn v-close-popup label="OK" color="primary" flat dense /></div>
                </q-date>
              </q-popup-proxy>
            </div>
          </div>
        </div>
        <q-table :rows="vitalHistory" :columns="vitalCols" row-key="id" flat bordered
                 :loading="vitalHisLoading" v-model:pagination="vitalHisPagination"
                 :rows-per-page-options="[10,25,50,100]" @request="onVitalHisRequest">
          <template #body-cell-resident_name="props">
            <q-td :props="props"><span class="text-weight-medium">{{ props.row.resident_name }}</span></q-td>
          </template>
          <template #body-cell-measured_at="props">
            <q-td :props="props">{{ formatDateTime(props.row.measured_at) }}</q-td>
          </template>
          <template #body-cell-bp="props">
            <q-td :props="props">
              <span :class="bpColor(props.row)">{{ formatBP(props.row) }}</span>
            </q-td>
          </template>
          <template #body-cell-heart_rate="props"><q-td :props="props">{{ props.row.heart_rate ?? "—" }}</q-td></template>
          <template #body-cell-temperature="props"><q-td :props="props">{{ props.row.temperature ?? "—" }}</q-td></template>
          <template #body-cell-weight="props"><q-td :props="props">{{ props.row.weight ?? "—" }}</q-td></template>
          <template #body-cell-spo2="props">
            <q-td :props="props"><span :class="spo2Color(props.row)">{{ props.row.spo2 != null ? props.row.spo2 + "%" : "—" }}</span></q-td>
          </template>
          <template #body-cell-blood_sugar="props"><q-td :props="props">{{ fmtNum(props.row.blood_sugar) }}</q-td></template>
          <template #body-cell-staff_name="props">
            <q-td :props="props"><span v-if="props.row.staff_name">{{ props.row.staff_name }}</span><span v-else class="text-grey">—</span></q-td>
          </template>
          <template #no-data>
            <div class="full-width column flex-center q-py-xl">
              <q-icon name="o_monitor_heart" size="3rem" color="grey-4" />
              <div class="text-grey-5 q-mt-sm">No archived records found</div>
            </div>
          </template>
        </q-table>
      </template>
    </template>

    <!-- ════════════════════════════════════════════════════════════════════════
         DIALOGS
    ════════════════════════════════════════════════════════════════════════ -->

    <!-- Resident Summary Dialog -->
    <q-dialog v-model="showSummary">
      <q-card v-if="summaryRes" style="min-width:820px;max-width:960px;width:100%">
        <q-card-section class="q-pb-sm" style="background:linear-gradient(135deg,#1976d2 0%,#0d47a1 100%)">
          <div class="row items-start">
            <div class="col">
              <div class="text-h6 text-white text-weight-bold">
                {{ summaryRes.first_name }} {{ summaryRes.last_name }}
                <q-chip v-if="summaryRes.room_number" dense square color="white" text-color="primary" size="sm" class="q-ml-xs">Room {{ summaryRes.room_number }}</q-chip>
                <q-chip v-if="summaryRes.care_grade" dense square color="orange" text-color="white" size="sm" class="q-ml-xs">
                  {{ careLevelLabel[summaryRes.care_grade] ?? `L${summaryRes.care_grade}` }}
                </q-chip>
              </div>
              <div class="text-caption text-blue-2 q-mt-xs">
                {{ summaryRes.gender.charAt(0).toUpperCase() + summaryRes.gender.slice(1) }}
                · Age {{ age(summaryRes.date_of_birth) }} · DOB {{ summaryRes.date_of_birth }}
                · Admitted {{ summaryRes.admission_date }} · {{ stayDuration(summaryRes.admission_date) }}
              </div>
              <div v-if="summaryRes.primary_diagnosis" class="text-caption text-blue-2 q-mt-xs">
                <q-icon name="o_medical_information" size="xs" /> {{ summaryRes.primary_diagnosis }}
              </div>
              <div v-if="summaryRes.allergies" class="text-caption text-red-2 q-mt-xs">
                <q-icon name="o_warning" size="xs" /> Allergies: {{ summaryRes.allergies }}
              </div>
            </div>
            <div class="col-auto"><q-btn icon="o_close" flat round dense color="white" v-close-popup /></div>
          </div>
        </q-card-section>
        <q-card-section class="q-pa-md" style="max-height:65vh;overflow-y:auto">
          <template v-if="summaryLoading">
            <div class="row q-gutter-md"><q-skeleton v-for="n in 3" :key="n" type="rect" height="180px" class="col" /></div>
          </template>
          <template v-else>
            <div class="row q-gutter-md">
              <div class="col">
                <div class="text-overline text-grey-6 q-mb-sm">
                  <q-icon name="o_medication" /> Active Medications
                  <q-badge v-if="summaryMeds.length" color="primary" :label="summaryMeds.length" class="q-ml-xs" />
                </div>
                <div v-if="!summaryMeds.length" class="text-caption text-grey-5 q-py-md text-center">None</div>
                <div v-for="m in summaryMeds" :key="m.id" class="q-mb-sm" style="border-left:3px solid #1976d2;padding-left:8px">
                  <div class="text-body2 text-weight-medium">{{ m.name }}</div>
                  <div class="text-caption text-grey-7">{{ m.dosage }} · {{ m.frequency }}</div>
                  <q-badge dense :color="routeColor(m.route)" :label="m.route" class="text-capitalize q-mt-xs" style="font-size:10px" />
                </div>
              </div>
              <div class="col">
                <div class="text-overline text-grey-6 q-mb-sm">
                  <q-icon name="o_monitor_heart" /> Latest Vitals
                  <span v-if="summaryVitals[0]" class="text-caption text-grey-5 q-ml-xs">{{ summaryVitals[0].measured_at.slice(0,10) }}</span>
                </div>
                <div v-if="!summaryVitals.length" class="text-caption text-grey-5 q-py-md text-center">None</div>
                <template v-else>
                  <div v-if="summaryVitals[0].bp_systolic" class="row items-center q-mb-xs">
                    <div class="col-auto text-caption text-grey-6" style="width:90px">Blood Pressure</div>
                    <div class="col text-body2 text-weight-medium">{{ summaryVitals[0].bp_systolic }}/{{ summaryVitals[0].bp_diastolic }} mmHg</div>
                  </div>
                  <div v-if="summaryVitals[0].heart_rate" class="row items-center q-mb-xs">
                    <div class="col-auto text-caption text-grey-6" style="width:90px">Heart Rate</div>
                    <div class="col text-body2 text-weight-medium">{{ summaryVitals[0].heart_rate }} bpm</div>
                  </div>
                  <div v-if="summaryVitals[0].temperature" class="row items-center q-mb-xs">
                    <div class="col-auto text-caption text-grey-6" style="width:90px">Temperature</div>
                    <div class="col text-body2 text-weight-medium">{{ fmtNum(summaryVitals[0].temperature) }} °C</div>
                  </div>
                  <div v-if="summaryVitals[0].spo2" class="row items-center q-mb-xs">
                    <div class="col-auto text-caption text-grey-6" style="width:90px">SpO2</div>
                    <div class="col text-body2 text-weight-medium">{{ summaryVitals[0].spo2 }}%</div>
                  </div>
                </template>
              </div>
              <div class="col">
                <div class="text-overline text-grey-6 q-mb-sm"><q-icon name="o_assignment" /> Recent Care Log</div>
                <div v-if="!summaryNotes.length" class="text-caption text-grey-5 q-py-md text-center">None</div>
                <div v-for="n in summaryNotes" :key="n.id" class="q-mb-md" style="border-left:3px solid #e0e0e0;padding-left:8px">
                  <div class="row items-center q-mb-xs">
                    <q-badge dense color="grey-4" text-color="grey-8" :label="n.category" class="text-capitalize q-mr-xs" style="font-size:10px" />
                    <span class="text-caption text-grey-5">{{ n.logged_at.slice(0,10) }}</span>
                  </div>
                  <div class="text-caption text-grey-8" style="white-space:pre-wrap;word-break:break-word">{{ n.content }}</div>
                </div>
              </div>
            </div>
          </template>
        </q-card-section>
        <q-separator />
        <q-card-actions align="right" class="q-px-md q-py-sm">
          <q-btn flat label="View Care Log" color="primary" icon="o_assignment"
                 @click="showSummary = false; jumpToCareLogHistory(summaryRes!.id)" />
          <q-btn flat label="Close" v-close-popup />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- Add/Edit Resident Dialog -->
    <q-dialog v-model="showDialog" persistent>
      <q-card style="min-width:600px;max-width:700px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">{{ dialogTitle }}</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section>
          <q-form ref="residentFormRef" @submit.prevent="saveResident">
            <div class="row q-col-gutter-md">
              <div class="col-6"><q-input v-model="resForm.first_name" label="First Name *" outlined dense :rules="[v => !!v?.trim() || 'Required']" lazy-rules="ondemand" /></div>
              <div class="col-6"><q-input v-model="resForm.last_name"  label="Last Name *"  outlined dense :rules="[v => !!v?.trim() || 'Required']" lazy-rules="ondemand" /></div>
              <div class="col-6">
                <div class="cursor-pointer">
                  <q-input v-model="resForm.date_of_birth" label="Date of Birth *" outlined dense readonly
                  :rules="[v => !!v || 'Required']" lazy-rules="ondemand" style="pointer-events:none">
                    <template #append>
                      <q-icon name="o_event" color="grey-6" />
                    </template>
                  </q-input>
                  <q-popup-proxy transition-show="scale" transition-hide="scale">
                    <q-date v-model="resForm.date_of_birth" mask="YYYY-MM-DD">
                      <div class="row items-center justify-end q-pa-sm">
                        <q-btn v-close-popup label="OK" color="primary" flat dense />
                      </div>
                    </q-date>
                  </q-popup-proxy>
                </div>
              </div>
              <div class="col-6"><q-select v-model="resForm.gender" label="Gender" outlined dense :options="['male','female','other']" /></div>
              <div class="col-4"><q-input v-model="resForm.room_number" label="Room Number" outlined dense /></div>
              <div class="col-4">
                <q-select v-model="resForm.care_grade" label="Care Level" outlined dense clearable emit-value map-options
                  :options="[{label:'SL1 — Basic',value:1},{label:'SL2/SL3 — Supportive',value:2},{label:'SL4 — 24hr Nursing',value:3},{label:'SL4D — Dementia',value:4},{label:'LTC — Long-Term Care',value:5}]" />
              </div>
              <div class="col-4">
                <div class="cursor-pointer">
                  <q-input v-model="resForm.admission_date" label="Admission Date" outlined dense readonly style="pointer-events:none">
                    <template #append>
                      <q-icon name="o_event" color="grey-6" />
                    </template>
                  </q-input>
                  <q-popup-proxy transition-show="scale" transition-hide="scale">
                    <q-date v-model="resForm.admission_date" mask="YYYY-MM-DD">
                      <div class="row items-center justify-end q-pa-sm">
                        <q-btn v-close-popup label="OK" color="primary" flat dense />
                      </div>
                    </q-date>
                  </q-popup-proxy>
                </div>
              </div>
              <div class="col-12"><q-input v-model="resForm.primary_diagnosis" label="Primary Diagnosis" outlined dense /></div>
              <div class="col-6"><q-input v-model="resForm.allergies" label="Allergies" outlined dense /></div>
              <div class="col-6"><q-input v-model="resForm.dietary_restrictions" label="Dietary Restrictions" outlined dense /></div>
              <div class="col-6"><q-input v-model="resForm.insurance_number" label="PHN (Personal Health Number)" outlined dense /></div>
              <div class="col-6 flex items-center"><q-checkbox v-model="resForm.cognitive_support" label="Cognitive Support Required" /></div>
              <div class="col-12"><q-input v-model="resForm.notes" label="Notes" outlined dense type="textarea" rows="3" /></div>
            </div>
            <div class="row justify-end q-mt-md q-gutter-sm">
              <q-btn label="Cancel" flat v-close-popup />
              <q-btn label="Save" color="primary" unelevated type="submit" />
            </div>
          </q-form>
        </q-card-section>
      </q-card>
    </q-dialog>

    <!-- New Care Log Entry Dialog -->
    <q-dialog v-model="showEntryDialog" persistent>
      <q-card style="min-width:520px;max-width:620px">
        <q-card-section class="row items-center q-pb-none">
          <q-icon name="o_add_circle" color="primary" class="q-mr-sm" size="1.4rem" />
          <div class="text-h6">New Care Log Entry</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-form ref="newEntryFormRef" class="q-gutter-sm">
          <div class="row q-gutter-sm">
            <div class="col">
              <q-select v-model="clForm.shift" :options="shiftOptions" label="Shift"
                        outlined dense emit-value map-options>
                <template #prepend><q-icon name="o_schedule" :color="SHIFT_COLOR[clForm.shift]??'grey'" /></template>
              </q-select>
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <div class="cursor-pointer">
                <q-input v-model="clForm.logged_date" label="Date" outlined dense readonly style="pointer-events:none">
                  <template #append>
                    <q-icon name="o_event" color="grey-6" />
                  </template>
                </q-input>
                <q-popup-proxy transition-show="scale" transition-hide="scale" cover>
                  <q-date v-model="clForm.logged_date" mask="YYYY-MM-DD" minimal>
                    <div class="row items-center justify-end q-pa-sm">
                      <q-btn v-close-popup label="OK" color="primary" flat dense />
                    </div>
                  </q-date>
                </q-popup-proxy>
              </div>
            </div>
            <div class="col">
              <div class="cursor-pointer">
                <q-input v-model="clForm.logged_time" label="Time" outlined dense readonly style="pointer-events:none">
                  <template #append>
                    <q-icon name="o_access_time" color="grey-6" />
                  </template>
                </q-input>
                <q-popup-proxy transition-show="scale" transition-hide="scale" cover>
                  <q-time v-model="clForm.logged_time" format24h>
                    <div class="row items-center justify-end q-pa-sm">
                      <q-btn v-close-popup label="OK" color="primary" flat dense />
                    </div>
                  </q-time>
                </q-popup-proxy>
              </div>
            </div>
          </div>
          <q-select v-model="clForm.resident_id" :options="residentOptions" label="Resident *"
                    outlined dense emit-value map-options clearable
                    :rules="[v => v !== null && v !== undefined || 'Please select a resident']"
                    lazy-rules="ondemand" />
          <q-select v-model="clForm.category" :options="CATEGORY_OPTIONS" label="Category"
                    outlined dense emit-value map-options />
          <q-input v-model="clForm.content" label="Notes *" type="textarea" outlined rows="3" autogrow
                   placeholder="Describe the care activity or observation…"
                   :rules="[v => !!v?.trim() || 'Please enter log content']"
                   lazy-rules="ondemand" />
        </q-form>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Save Log" icon="o_save" unelevated :loading="clSubmitting" @click="submitLog" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- Edit Care Log Dialog -->
    <q-dialog v-model="showEditLogDialog" persistent>
      <q-card style="min-width:480px;max-width:580px">
        <q-card-section class="row items-center q-pb-none">
          <q-icon name="o_edit" color="primary" class="q-mr-sm" size="1.3rem" />
          <div class="text-h6">Edit Care Log</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section>
          <div v-if="editingLog" class="text-caption text-grey-6 q-mb-sm">
            {{ editingLog.resident_name }} · {{ capitalize(editingLog.shift) }} · {{ fmtDate(editingLog.logged_at) }}
          </div>
          <q-form ref="editLogFormRef">
            <q-input v-model="editLogForm.content" label="Notes *" type="textarea" outlined rows="3" autogrow
                     :rules="[v => !!v?.trim() || 'Content cannot be empty']" lazy-rules="ondemand" />
          </q-form>
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Update" icon="o_save" unelevated :loading="editLogSubmitting" @click="submitEditLog" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- Add Medication Dialog -->
    <q-dialog v-model="showAddMedDialog" persistent>
      <q-card style="min-width:520px">
        <q-card-section class="row items-center q-pb-none">
          <div>
            <div class="text-h6">Add Medication</div>
            <div v-if="selectedMedResObj" class="text-caption text-grey-6 q-mt-xs">
              {{ selectedMedResObj.first_name }} {{ selectedMedResObj.last_name }}
              · {{ genderLabel(selectedMedResObj.gender) }}
              · Age {{ age(selectedMedResObj.date_of_birth) }}
            </div>
          </div>
          <q-space /><q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-form ref="addMedFormRef" class="q-gutter-sm">
          <q-select v-model="medForm.resident_id" :options="residentOptions"
                    :label="selectedMedResObj ? `Resident: ${selectedMedResObj.first_name} ${selectedMedResObj.last_name}` : 'Resident *'"
                    outlined dense emit-value map-options :disable="!!medSelectedRes"
                    :rules="[v => v !== null && v !== undefined || 'Please select a resident']"
                    lazy-rules="ondemand" />
          <div class="row q-gutter-sm">
            <div class="col"><q-input v-model="medForm.name"   label="Medication Name *" outlined dense :rules="[v => !!v?.trim() || 'Required']" lazy-rules="ondemand" /></div>
            <div class="col"><q-input v-model="medForm.dosage" label="Dosage *"           outlined dense :rules="[v => !!v?.trim() || 'Required']" lazy-rules="ondemand" /></div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col"><q-input v-model="medForm.frequency" label="Frequency" outlined dense /></div>
            <div class="col"><q-select v-model="medForm.route" :options="routeOptions" label="Route" outlined dense emit-value map-options /></div>
          </div>
          <div class="cursor-pointer">
            <q-input v-model="medForm.start_date" label="Start Date" outlined dense readonly style="pointer-events:none">
              <template #append>
                <q-icon name="o_event" color="grey-6" />
              </template>
            </q-input>
            <q-popup-proxy transition-show="scale" transition-hide="scale">
              <q-date v-model="medForm.start_date" mask="YYYY-MM-DD">
                <div class="row items-center justify-end q-pa-sm">
                  <q-btn v-close-popup label="OK" color="primary" flat dense />
                </div>
              </q-date>
            </q-popup-proxy>
          </div>
          <q-input v-model="medForm.prescriber"   label="Prescriber"    outlined dense />
          <q-input v-model="medForm.instructions" label="Instructions"  type="textarea" outlined dense rows="2" autogrow />
        </q-form>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="teal" label="Save" unelevated :loading="medSubmitting" @click="submitMedication" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- Record Vitals Dialog -->
    <q-dialog v-model="showVitalDialog" persistent>
      <q-card style="min-width:500px">
        <q-card-section class="row items-center q-pb-none">
          <div>
            <div class="text-h6">Record Vital Signs</div>
            <div v-if="selectedVitalResObj" class="text-caption text-grey-6 q-mt-xs">
              {{ selectedVitalResObj.first_name }} {{ selectedVitalResObj.last_name }}
              · {{ genderLabel(selectedVitalResObj.gender) }}
              · Age {{ age(selectedVitalResObj.date_of_birth) }}
            </div>
          </div>
          <q-space /><q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-sm">
          <q-select v-model="vitalForm.resident_id" :options="residentOptions"
                    :label="selectedVitalResObj ? `Resident: ${selectedVitalResObj.first_name} ${selectedVitalResObj.last_name}` : 'Resident *'"
                    outlined dense emit-value map-options :disable="!!vitalSelectedRes" />
          <div class="text-subtitle2 text-grey-7">Blood Pressure</div>
          <div class="row q-gutter-sm">
            <div class="col"><q-input v-model="vitalForm.bp_systolic"  label="Systolic (mmHg)"  outlined dense :rules="[ruleBP]"        inputmode="numeric" /></div>
            <div class="col"><q-input v-model="vitalForm.bp_diastolic" label="Diastolic (mmHg)" outlined dense :rules="[ruleBP]"        inputmode="numeric" /></div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col"><q-input v-model="vitalForm.heart_rate"  label="Heart Rate (bpm)"  outlined dense :rules="[rulePositiveInt]" inputmode="numeric" /></div>
            <div class="col"><q-input v-model="vitalForm.temperature" label="Temperature (°C)"  outlined dense :rules="[ruleTemp]"        inputmode="decimal" /></div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col"><q-input v-model="vitalForm.weight"      label="Weight (kg)"       outlined dense :rules="[rulePositiveDec]" inputmode="decimal" /></div>
            <div class="col"><q-input v-model="vitalForm.spo2"        label="SpO₂ (%)"          outlined dense :rules="[ruleSpo2]"        inputmode="numeric" /></div>
          </div>
          <q-input v-model="vitalForm.blood_sugar" label="Blood Sugar (mg/dL)" outlined dense :rules="[rulePositiveDec]" inputmode="decimal" />
          <q-input v-model="vitalForm.notes"       label="Notes" type="textarea" outlined dense rows="2" autogrow />
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="deep-purple" label="Save" unelevated :loading="vitalSubmitting" @click="submitVital" />
        </q-card-actions>
      </q-card>
    </q-dialog>

  </q-page>
</template>

<style scoped>
.section-tabs {
  border-bottom: 2px solid #e5e7eb;
}
.history-tab-btn {
  height: 48px;
  padding: 0 12px;
  color: #64748b;
  border-radius: 0;
  font-size: 0.8125rem;
  font-weight: 500;
  letter-spacing: 0.03em;
}
.history-tab-btn:hover {
  color: #1976d2;
}
.history-tab-btn--active {
  color: #1976d2;
  border-bottom: 2px solid #1976d2;
}
.cursor-pointer-rows :deep(tbody tr) { cursor: pointer; }
.cursor-pointer-rows :deep(tbody tr:hover td) { background: #f0f9ff; }
</style>
