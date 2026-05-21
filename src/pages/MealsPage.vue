<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { useAuthStore } from "@/stores/auth";

import * as XLSX from "xlsx";

const $q = useQuasar();
const auth = useAuthStore();

// ── Role check ──────────────────────────────────────────────────────────────
const isStaff   = computed(() => auth.user?.role === "staff");
const canManage = computed(() => {
  const role = auth.user?.role;
  return role === "manager" || role === "admin";
});
const canImport = canManage;
const isAdmin   = computed(() => auth.user?.role === "admin");

// ── Types ───────────────────────────────────────────────────────────────────
interface MealPlan {
  id?: number;
  week_start: string;
  day_of_week: number;
  meal_type: string;
  menu: string;
  calories: number | null;
  notes: string | null;
}

const DAYS       = ["Monday","Tuesday","Wednesday","Thursday","Friday","Saturday","Sunday"];
const MEAL_TYPES = ["breakfast","lunch","dinner","snack"];
const MEAL_LABEL: Record<string,string> = { breakfast:"Breakfast", lunch:"Lunch", dinner:"Dinner", snack:"Snack" };
const MEAL_COLOR: Record<string,string> = { breakfast:"orange", lunch:"green", dinner:"blue", snack:"purple" };
const MONTHS = ["January","February","March","April","May","June","July","August","September","October","November","December"];

// ── Tab / navigation state ──────────────────────────────────────────────────
const tab = ref<"weekly"|"monthly"|"stats">("weekly");

// Weekly — initialise immediately so computed properties don't receive an empty string
const weekStart   = ref(getMondayOf(new Date()));
const weekMeals   = ref<MealPlan[]>([]);
const weekLoading = ref(false);

// Monthly / Stats (shared)
const viewYear  = ref(new Date().getFullYear());
const viewMonth = ref(new Date().getMonth()); // 0-indexed
const monthMeals  = ref<MealPlan[]>([]);
const monthLoading = ref(false);

// Edit dialog (shared across tabs)
const showEditDialog = ref(false);
const mealFormRef    = ref();
const submitting     = ref(false);
const editTarget     = ref<{ week_start: string; day: number; mealType: string } | null>(null);
const editForm       = ref({ menu: "", calories: "", notes: "" });

// Import
const importInput = ref<HTMLInputElement | null>(null);
const importing   = ref(false);
const importMode  = ref<"all" | null>(null);

// Export combined loading state
const exporting = ref(false);

// ── Date helpers ────────────────────────────────────────────────────────────
function getMondayOf(date: Date): string {
  const d = new Date(date);
  const day = d.getDay(); // local day-of-week
  const diff = day === 0 ? -6 : 1 - day;
  d.setDate(d.getDate() + diff);
  // Use LOCAL date components — toISOString() converts to UTC and can shift the day
  const y  = d.getFullYear();
  const m  = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

function addWeeks(ws: string, n: number): string {
  const d = new Date(ws + "T12:00:00");
  d.setDate(d.getDate() + n * 7);
  // Use local date parts to avoid UTC day-shift in UTC+N timezones
  const y  = d.getFullYear();
  const m  = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

// Today's ISO date (local), recomputed fresh each call
function todayIso(): string {
  const d = new Date();
  const y  = d.getFullYear();
  const m  = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

function isToday(dayIdx: number): boolean {
  const d = dateFromWeekDay(weekStart.value, dayIdx);
  return isoDate(d) === todayIso();
}

function dateFromWeekDay(ws: string, dayIdx: number): Date {
  const d = new Date(ws + "T12:00:00");
  d.setDate(d.getDate() + dayIdx);
  return d;
}

function isoDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function dayOfWeekMon0(d: Date): number {
  // Monday = 0, Sunday = 6
  const wd = d.getDay();
  return wd === 0 ? 6 : wd - 1;
}

// ── Weekly helpers ──────────────────────────────────────────────────────────
function formatWeekLabel(ws: string): string {
  const start = new Date(ws + "T12:00:00");
  const end   = new Date(ws + "T12:00:00");
  end.setDate(end.getDate() + 6);
  const opts: Intl.DateTimeFormatOptions = { month: "short", day: "numeric" };
  return `${start.toLocaleDateString("en-CA", opts)} – ${end.toLocaleDateString("en-CA", opts)}, ${end.getFullYear()}`;
}

function dayDate(dayIdx: number): string {
  const d = dateFromWeekDay(weekStart.value, dayIdx);
  return d.toLocaleDateString("en-CA", { month: "short", day: "numeric" });
}

const weekGrid = computed(() => {
  const g: Record<string, (MealPlan | null)[]> = {};
  for (const mt of MEAL_TYPES) g[mt] = Array(7).fill(null);
  for (const p of weekMeals.value) {
    if (g[p.meal_type] && p.day_of_week >= 0 && p.day_of_week < 7) {
      g[p.meal_type][p.day_of_week] = p;
    }
  }
  return g;
});

async function loadWeek() {
  weekLoading.value = true;
  try {
    weekMeals.value = await invoke<MealPlan[]>("list_meal_plans", { weekStart: weekStart.value });
  } catch { weekMeals.value = []; }
  finally { weekLoading.value = false; }
}

// ── Monthly helpers ─────────────────────────────────────────────────────────
// Returns all calendar days shown for the month (full weeks, Mon–Sun)
const calendarDays = computed((): Date[] => {
  const firstDay = new Date(viewYear.value, viewMonth.value, 1);
  const lastDay  = new Date(viewYear.value, viewMonth.value + 1, 0);
  const start    = new Date(getMondayOf(firstDay) + "T12:00:00");
  const end      = new Date(getMondayOf(lastDay)  + "T12:00:00");
  end.setDate(end.getDate() + 6);

  const days: Date[] = [];
  const cur = new Date(start);
  while (cur <= end) {
    days.push(new Date(cur));
    cur.setDate(cur.getDate() + 1);
  }
  return days;
});

// Month range: first Monday covering the month → last Monday covering the month
const monthRange = computed(() => {
  const firstDay = new Date(viewYear.value, viewMonth.value, 1);
  const lastDay  = new Date(viewYear.value, viewMonth.value + 1, 0);
  return {
    start: getMondayOf(firstDay),
    end:   getMondayOf(lastDay),
  };
});

async function loadMonth() {
  monthLoading.value = true;
  try {
    monthMeals.value = await invoke<MealPlan[]>("list_meal_plans_range", {
      startDate: monthRange.value.start,
      endDate:   monthRange.value.end,
    });
  } catch { monthMeals.value = []; }
  finally { monthLoading.value = false; }
}

// Build lookup: "YYYY-MM-DD|mealType" → MealPlan
const monthIndex = computed(() => {
  const map: Record<string, MealPlan> = {};
  for (const p of monthMeals.value) {
    const date = dateFromWeekDay(p.week_start, p.day_of_week);
    map[`${isoDate(date)}|${p.meal_type}`] = p;
  }
  return map;
});

function getMeal(date: Date, mealType: string): MealPlan | null {
  return monthIndex.value[`${isoDate(date)}|${mealType}`] ?? null;
}

function isCurrentMonth(d: Date): boolean {
  return d.getMonth() === viewMonth.value && d.getFullYear() === viewYear.value;
}

function prevMonth() {
  if (viewMonth.value === 0) { viewMonth.value = 11; viewYear.value--; }
  else viewMonth.value--;
}

function nextMonth() {
  if (viewMonth.value === 11) { viewMonth.value = 0; viewYear.value++; }
  else viewMonth.value++;
}

// ── Statistics ───────────────────────────────────────────────────────────────
const statsMonthDays = computed((): Date[] => {
  const days: Date[] = [];
  const d = new Date(viewYear.value, viewMonth.value, 1);
  while (d.getMonth() === viewMonth.value) {
    days.push(new Date(d));
    d.setDate(d.getDate() + 1);
  }
  return days;
});

const stats = computed(() => {
  const days = statsMonthDays.value;
  let completeDays = 0, partialDays = 0, emptyDays = 0;
  let totalCals = 0, calDays = 0;
  const calsByType: Record<string, number[]> = { breakfast:[], lunch:[], dinner:[], snack:[] };
  const dailyCals: { label: string; total: number }[] = [];

  for (const day of days) {
    const meals = MEAL_TYPES.map(mt => getMeal(day, mt));
    const filled = meals.filter(m => m && m.menu).length;
    if (filled === 4) completeDays++;
    else if (filled > 0) partialDays++;
    else emptyDays++;

    const dayCal = meals.reduce((s, m) => s + (m?.calories ?? 0), 0);
    if (dayCal > 0) { totalCals += dayCal; calDays++; }
    dailyCals.push({ label: day.getDate().toString(), total: dayCal });

    for (const mt of MEAL_TYPES) {
      const m = getMeal(day, mt);
      if (m?.calories) calsByType[mt].push(m.calories);
    }
  }

  const avg = (arr: number[]) => arr.length ? Math.round(arr.reduce((a,b)=>a+b,0)/arr.length) : 0;
  const fillRate = days.length ? Math.round(((completeDays + partialDays * 0.5) / days.length) * 100) : 0;

  return {
    totalDays: days.length,
    completeDays, partialDays, emptyDays,
    avgDailyCal: calDays ? Math.round(totalCals / calDays) : 0,
    fillRate,
    calsByType: {
      breakfast: avg(calsByType.breakfast),
      lunch:     avg(calsByType.lunch),
      dinner:    avg(calsByType.dinner),
      snack:     avg(calsByType.snack),
    },
    dailyCals,
  };
});

// ── Edit dialog ──────────────────────────────────────────────────────────────
function openEditWeek(dayIdx: number, mealType: string) {
  const plan = weekGrid.value[mealType]?.[dayIdx];
  editTarget.value = { week_start: weekStart.value, day: dayIdx, mealType };
  editForm.value = { menu: plan?.menu ?? "", calories: plan?.calories?.toString() ?? "", notes: plan?.notes ?? "" };
  showEditDialog.value = true;
}

function openEditDay(date: Date, mealType: string) {
  const ws  = getMondayOf(date);
  const day = dayOfWeekMon0(date);
  const plan = getMeal(date, mealType);
  editTarget.value = { week_start: ws, day, mealType };
  editForm.value = { menu: plan?.menu ?? "", calories: plan?.calories?.toString() ?? "", notes: plan?.notes ?? "" };
  showEditDialog.value = true;
}

async function saveEdit() {
  if (!editTarget.value) return;
  const valid = await mealFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await invoke("upsert_meal_plan", {
      input: {
        week_start:  editTarget.value.week_start,
        day_of_week: editTarget.value.day,
        meal_type:   editTarget.value.mealType,
        menu:        editForm.value.menu,
        calories:    editForm.value.calories ? parseInt(editForm.value.calories) : null,
        notes:       editForm.value.notes || null,
      },
    });
    $q.notify({ type: "positive", message: "Saved." });
    showEditDialog.value = false;
    if (tab.value === "weekly") loadWeek();
    else loadMonth();
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to save: ${e}` });
  } finally {
    submitting.value = false;
  }
}

// ── Excel Export helpers ──────────────────────────────────────────────────────
const HEADER = ["Date","Day","Breakfast","B.Kcal","Lunch","L.Kcal","Dinner","D.Kcal","Snack","S.Kcal","Notes"];
const COL_WIDTHS = [10,12,25,8,25,8,25,8,20,8,30].map(w => ({ wch: w }));

function buildRow(d: Date, getMealFn: (date: Date, mt: string) => MealPlan | null): (string | number)[] {
  const get = (mt: string) => {
    const m = getMealFn(d, mt);
    return [m?.menu ?? "", m?.calories ?? ""] as [string, string | number];
  };
  const [b,bc]   = get("breakfast");
  const [l,lc]   = get("lunch");
  const [dn,dc]  = get("dinner");
  const [s,sc]   = get("snack");
  const notes    = MEAL_TYPES.map(mt => getMealFn(d, mt)?.notes).filter(Boolean).join("; ") || "";
  return [isoDate(d), DAYS[dayOfWeekMon0(d)], b, bc, l, lc, dn, dc, s, sc, notes];
}

async function saveWorkbook(wb: ReturnType<typeof XLSX.utils.book_new>, filename: string) {
  const wbout = XLSX.write(wb, { bookType: "xlsx", type: "array" });
  try {
    const savedPath = await invoke<string | null>("save_excel", {
      filename,
      data: Array.from(new Uint8Array(wbout as ArrayBuffer)),
    });
    if (savedPath) {
      $q.notify({ type: "positive", message: `Saved: ${savedPath.split("/").pop()}`, timeout: 3000 });
    }
  } catch (e) {
    $q.notify({ type: "negative", message: `Export failed: ${e}` });
  }
}

// Build a multi-sheet workbook from a date range
async function buildRangeWorkbook(startDate: string, endDate: string): Promise<ReturnType<typeof XLSX.utils.book_new> | null> {
  const all = await invoke<MealPlan[]>("list_meal_plans_range", { startDate, endDate });
  if (!all.length) {
    $q.notify({ type: "warning", message: "No meal data found in the database." });
    return null;
  }
  const index: Record<string, Record<string, MealPlan>> = {};
  for (const p of all) {
    const date = isoDate(dateFromWeekDay(p.week_start, p.day_of_week));
    if (!index[date]) index[date] = {};
    index[date][p.meal_type] = p;
  }
  const sheetMap: Record<string, (string | number)[][]> = {};
  for (const dateStr of Object.keys(index).sort()) {
    const d    = new Date(dateStr + "T12:00:00");
    const name = `${MONTHS[d.getMonth()].slice(0,3)} ${d.getFullYear()}`;
    if (!sheetMap[name]) sheetMap[name] = [HEADER];
    sheetMap[name].push(buildRow(d, (_, mt) => index[dateStr]?.[mt] ?? null));
  }
  const wb = XLSX.utils.book_new();
  for (const [name, rows] of Object.entries(sheetMap)) {
    const ws = XLSX.utils.aoa_to_sheet(rows);
    ws["!cols"] = COL_WIDTHS;
    XLSX.utils.book_append_sheet(wb, ws, name);
  }
  return wb;
}

// Export All (manager / admin only)
async function exportAllExcel() {
  exporting.value = true;
  try {
    const wb = await buildRangeWorkbook("2020-01-01", "2099-12-31");
    if (wb) await saveWorkbook(wb, "meal-plan-all.xlsx");
  } catch (e) {
    $q.notify({ type: "negative", message: `Export failed: ${e}` });
  } finally {
    exporting.value = false;
  }
}

// ── Excel Import ─────────────────────────────────────────────────────────────
function localDate(d: Date): string {
  const y  = d.getFullYear();
  const m  = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

function triggerImportAll() {
  importMode.value = "all";
  importInput.value?.click();
}

interface ParsedPlan {
  week_start: string;
  day_of_week: number;
  meal_type: string;
  menu: string;
  calories: number | null;
  notes: null;
  _date: string;
}

function parseXlsxFile(buffer: ArrayBuffer): ParsedPlan[] | null {
  let wb: ReturnType<typeof XLSX.read>;
  try {
    wb = XLSX.read(buffer, { type: "array" });
  } catch {
    $q.notify({ type: "negative", message: "Could not read the file. Make sure it is a valid .xlsx file." });
    return null;
  }

  if (!wb.SheetNames.length) {
    $q.notify({ type: "negative", message: "The file has no sheets." });
    return null;
  }

  const allPlans: ParsedPlan[] = [];

  // Parse every sheet so multi-sheet exports (e.g. "Jan 2025", "Feb 2025") all import correctly
  for (const sheetName of wb.SheetNames) {
    const ws   = wb.Sheets[sheetName];
    const rows = XLSX.utils.sheet_to_json<(string | number)[]>(ws, { header: 1 }) as (string | number)[][];

    // Validate header row
    const header = rows[0]?.map(h => h?.toString().trim()) ?? [];
    if (!header[0]?.toLowerCase().startsWith("date")) {
      // Skip sheets that don't look like meal-plan sheets
      continue;
    }

    for (let i = 1; i < rows.length; i++) {
      const row     = rows[i];
      const dateStr = row[0]?.toString().trim();
      if (!dateStr || !/^\d{4}-\d{2}-\d{2}$/.test(dateStr)) continue;

      const d      = new Date(dateStr + "T12:00:00");
      if (isNaN(d.getTime())) continue;

      const ws_    = getMondayOf(d);
      const dayIdx = dayOfWeekMon0(d);

      const push = (mealType: string, menuCol: number, calCol: number) => {
        const menu = row[menuCol]?.toString().trim() ?? "";
        if (!menu) return;
        const calRaw = row[calCol]?.toString().trim();
        const cal    = calRaw ? (parseInt(calRaw) || null) : null;
        allPlans.push({ week_start: ws_, day_of_week: dayIdx, meal_type: mealType, menu, calories: cal, notes: null, _date: dateStr });
      };
      push("breakfast", 2, 3);
      push("lunch",     4, 5);
      push("dinner",    6, 7);
      push("snack",     8, 9);
    }
  }

  return allPlans;
}

async function handleImport(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file || !importMode.value) return;
  if (importInput.value) importInput.value.value = "";

  importing.value = true;
  const mode = importMode.value;
  importMode.value = null;

  try {
    // 1. Parse file
    const buffer   = await file.arrayBuffer();
    const allPlans = parseXlsxFile(buffer);
    if (!allPlans) { importing.value = false; return; }

    if (!allPlans.length) {
      $q.notify({ type: "warning", message: "No valid meal rows found. Check that the file uses the expected format (Date | Day | Breakfast | B.Kcal | …)." });
      importing.value = false;
      return;
    }

    // 2. For "all" mode: use full date range of the file itself
    const allDates  = allPlans.map(p => p._date).sort();
    const rangeStart = mode === "all" ? allDates[0] : "2020-01-01";
    const rangeEnd   = mode === "all" ? allDates[allDates.length - 1] : "2099-12-31";

    // 3. Check for existing data (conflict detection)
    let existing: MealPlan[] = [];
    try {
      existing = await invoke<MealPlan[]>("list_meal_plans_range", { startDate: rangeStart, endDate: rangeEnd });
    } catch (e) {
      $q.notify({ type: "negative", message: `Failed to check existing data: ${e}` });
      importing.value = false;
      return;
    }

    const doImport = async () => {
      try {
        const plans = allPlans.map(({ _date, ...rest }) => rest);
        const count = await invoke<number>("bulk_upsert_meal_plans", { plans });
        $q.notify({ type: "positive", message: `Successfully imported ${count} meal entries.`, timeout: 4000 });
        loadMonth();
        if (tab.value === "weekly") loadWeek();
      } catch (e) {
        $q.notify({ type: "negative", message: `Import failed: ${e}` });
      } finally {
        importing.value = false;
      }
    };

    if (existing.length > 0) {
      importing.value = false;

      // Determine what date range will be affected for the dialog message
      const existDates   = existing.map(p => isoDate(dateFromWeekDay(p.week_start, p.day_of_week))).sort();
      const conflictFrom = existDates[0];
      const conflictTo   = existDates[existDates.length - 1];

      $q.dialog({
        title: "Existing data will be overwritten",
        message:
          `The import file covers <b>${allPlans.length}</b> meal entries.<br>` +
          `<b>${existing.length}</b> existing entries between <b>${conflictFrom}</b> and <b>${conflictTo}</b> will be replaced.<br><br>` +
          `This action cannot be undone. Continue?`,
        html: true,
        cancel: { label: "Cancel", flat: true },
        ok:     { label: "Overwrite & Import", color: "warning", unelevated: true },
        persistent: true,
      }).onOk(() => {
        importing.value = true;
        doImport();
      });
    } else {
      await doImport();
    }
  } catch (e) {
    $q.notify({ type: "negative", message: `Import error: ${e}` });
    importing.value = false;
  }
}

// ── Month picker helpers ──────────────────────────────────────────────────────
// Dynamic: 11 months back → 1 month forward, most recent first
const monthPickerOptions = computed(() => {
  const opts: { year: number; month: number; label: string }[] = [];
  const now = new Date();
  for (let i = 1; i >= -11; i--) {
    const d = new Date(now.getFullYear(), now.getMonth() + i, 1);
    opts.push({ year: d.getFullYear(), month: d.getMonth(), label: `${MONTHS[d.getMonth()].slice(0,3)} ${d.getFullYear()}` });
  }
  return opts;
});

function jumpTo(year: number, month: number) {
  viewYear.value  = year;
  viewMonth.value = month;
}

// ── Weekly navigation limit (±1 month from today) ─────────────────────────────
const todayMonday = getMondayOf(new Date());

const minWeekStart = computed(() => {
  const d = new Date(todayMonday + "T12:00:00");
  d.setMonth(d.getMonth() - 1);
  return getMondayOf(d);
});

const maxWeekStart = computed(() => {
  const d = new Date(todayMonday + "T12:00:00");
  d.setMonth(d.getMonth() + 1);
  return getMondayOf(d);
});

const canGoPrevWeek = computed(() => addWeeks(weekStart.value, -1) >= minWeekStart.value);
const canGoNextWeek = computed(() => addWeeks(weekStart.value,  1) <= maxWeekStart.value);

const weekOptions = computed(() => {
  const opts: { start: string; label: string }[] = [];
  let cur = minWeekStart.value;
  while (cur <= maxWeekStart.value) {
    opts.push({ start: cur, label: formatWeekLabel(cur) });
    cur = addWeeks(cur, 1);
  }
  return opts;
});

// ── Watchers ─────────────────────────────────────────────────────────────────
watch([viewMonth, viewYear], () => {
  if (tab.value !== "weekly") loadMonth();
});

watch(tab, (t) => {
  if (t === "weekly") loadWeek();
  else loadMonth();
});

// ── Mount ────────────────────────────────────────────────────────────────────
onMounted(() => {
  loadWeek();
  loadMonth();
});
</script>

<template>
  <q-page class="q-pa-lg">

    <!-- ── Header ── -->
    <div class="row items-center q-mb-md">
      <div class="col">
        <div class="text-h5 text-weight-bold">Meal Planning</div>
        <div class="text-caption text-grey-6">Plan and track resident meals</div>
      </div>
      <!-- Export All + Import All: manager / admin only. Staff sees nothing. -->
      <div v-if="canImport" class="col-auto row q-gutter-sm">
        <q-btn
          outline color="primary" icon="o_download" label="Export All"
          dense :loading="exporting" @click="exportAllExcel"
        />
        <q-btn
          color="primary" icon="o_upload" label="Import All"
          dense unelevated :loading="importing" @click="triggerImportAll"
        />
        <input ref="importInput" type="file" accept=".xlsx,.xls" style="display:none" @change="handleImport" />
      </div>
    </div>

    <!-- ── Nav + Tabs (same row) ── -->
    <div class="row items-center q-mb-md">
      <!-- Navigation (left) -->
      <div class="col row items-center q-gutter-xs">
        <!-- Weekly nav -->
        <template v-if="tab === 'weekly'">
          <q-btn flat round dense icon="o_chevron_left" :disable="!canGoPrevWeek"
            @click="weekStart=addWeeks(weekStart,-1); loadWeek()" />
          <q-btn flat no-caps class="text-subtitle1 text-weight-bold">
            {{ formatWeekLabel(weekStart) }}
            <q-icon name="o_arrow_drop_down" size="1.2rem" color="grey-6" class="q-ml-xs" />
            <q-menu anchor="bottom left" self="top left" style="min-width:260px">
              <q-list dense>
                <q-item v-for="opt in weekOptions" :key="opt.start"
                  clickable v-close-popup
                  :active="weekStart === opt.start"
                  active-class="text-primary text-weight-bold"
                  @click="weekStart=opt.start; loadWeek()">
                  <q-item-section>{{ opt.label }}</q-item-section>
                </q-item>
              </q-list>
            </q-menu>
          </q-btn>
          <q-btn flat round dense icon="o_chevron_right" :disable="!canGoNextWeek"
            @click="weekStart=addWeeks(weekStart,1); loadWeek()" />
          <q-btn flat dense no-caps label="This week" color="primary"
            @click="weekStart=getMondayOf(new Date()); loadWeek()" />
        </template>
        <!-- Monthly nav -->
        <template v-else-if="tab === 'monthly'">
          <q-btn flat round dense icon="o_chevron_left" @click="prevMonth" />
          <q-btn flat no-caps class="text-subtitle1 text-weight-bold">
            {{ MONTHS[viewMonth] }} {{ viewYear }}
            <q-icon name="o_arrow_drop_down" size="1.2rem" color="grey-6" class="q-ml-xs" />
            <q-menu anchor="bottom left" self="top left" style="min-width:160px">
              <q-list dense>
                <q-item v-for="opt in monthPickerOptions" :key="`${opt.year}-${opt.month}`"
                  clickable v-close-popup
                  :active="viewYear === opt.year && viewMonth === opt.month"
                  active-class="text-primary text-weight-bold"
                  @click="jumpTo(opt.year, opt.month)">
                  <q-item-section>{{ opt.label }}</q-item-section>
                </q-item>
              </q-list>
            </q-menu>
          </q-btn>
          <q-btn flat round dense icon="o_chevron_right" @click="nextMonth" />
          <q-btn flat dense no-caps label="This Month" color="primary"
            @click="viewMonth=new Date().getMonth(); viewYear=new Date().getFullYear()" />
        </template>
        <!-- Statistics nav -->
        <template v-else>
          <q-btn flat round dense icon="o_chevron_left" @click="prevMonth" />
          <q-btn flat no-caps class="text-subtitle1 text-weight-bold">
            {{ MONTHS[viewMonth] }} {{ viewYear }}
            <q-icon name="o_arrow_drop_down" size="1.2rem" color="grey-6" class="q-ml-xs" />
            <q-menu anchor="bottom left" self="top left" style="min-width:160px">
              <q-list dense>
                <q-item v-for="opt in monthPickerOptions" :key="`${opt.year}-${opt.month}`"
                  clickable v-close-popup
                  :active="viewYear === opt.year && viewMonth === opt.month"
                  active-class="text-primary text-weight-bold"
                  @click="jumpTo(opt.year, opt.month)">
                  <q-item-section>{{ opt.label }}</q-item-section>
                </q-item>
              </q-list>
            </q-menu>
          </q-btn>
          <q-btn flat round dense icon="o_chevron_right" @click="nextMonth" />
        </template>
      </div>
      <!-- Tabs (right) -->
      <div class="col-auto">
        <q-tabs v-model="tab" indicator-color="primary" active-color="primary" dense>
          <q-tab name="weekly"  icon="o_view_week"      label="Weekly" />
          <q-tab name="monthly" icon="o_calendar_month" label="Monthly" />
          <q-tab v-if="!isStaff" name="stats" icon="o_bar_chart" label="Statistics" />
        </q-tabs>
      </div>
    </div>

    <!-- ══════════════════════════════════════════════════════════════════════
         WEEKLY TAB
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-if="tab === 'weekly'">

      <!-- Skeleton while loading -->
      <template v-if="weekLoading">
        <q-skeleton type="rect" height="36px" class="q-mb-xs" />
        <q-skeleton type="rect" height="60px" class="q-mb-xs" v-for="n in 4" :key="n" />
      </template>

      <div v-else style="overflow-x:auto">
        <table style="width:100%;border-collapse:collapse;min-width:820px">
          <thead>
            <tr>
              <th style="width:90px;padding:8px;background:#f5f5f5;border:1px solid #e0e0e0;text-align:left">Meal</th>
              <th
                v-for="(day,i) in DAYS" :key="day"
                :style="`padding:8px;border:1px solid #e0e0e0;text-align:center;min-width:110px;
                         background:${isToday(i) ? '#e8f4fd' : '#f5f5f5'}`"
              >
                <div :class="isToday(i) ? 'text-weight-bold text-caption text-primary' : 'text-weight-bold text-caption'">{{ day }}</div>
                <div :class="isToday(i) ? 'text-caption text-primary text-weight-medium' : 'text-caption text-grey-6'">{{ dayDate(i) }}</div>
                <q-badge v-if="isToday(i)" color="primary" label="Today" style="font-size:0.6rem;margin-top:2px" />
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="mt in MEAL_TYPES" :key="mt">
              <td style="padding:8px;border:1px solid #e0e0e0;vertical-align:top">
                <q-badge :color="MEAL_COLOR[mt]" :label="MEAL_LABEL[mt]" />
              </td>
              <td
                v-for="(_,i) in DAYS" :key="i"
                :style="`padding:6px;border:1px solid #e0e0e0;vertical-align:top;
                         ${isToday(i) ? 'background:#f0f9ff;' : ''}
                         ${canManage ? 'cursor:pointer' : ''}`"
                class="meal-cell"
                @click="canManage && openEditWeek(i, mt)"
              >
                <div style="min-height:52px">
                  <div v-if="weekGrid[mt][i]?.menu" class="text-caption text-grey-9" style="line-height:1.4">
                    {{ weekGrid[mt][i]?.menu }}
                  </div>
                  <div v-else class="text-caption text-grey-4">—</div>
                  <div v-if="weekGrid[mt][i]?.calories" class="text-grey-5" style="font-size:0.7rem">
                    {{ weekGrid[mt][i]?.calories }} kcal
                  </div>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>

    <!-- ══════════════════════════════════════════════════════════════════════
         MONTHLY TAB
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-else-if="tab === 'monthly'">
      <div v-if="monthLoading" class="flex flex-center q-py-xl">
        <q-spinner color="primary" size="3rem" />
      </div>
      <div v-else style="overflow-x:auto">
        <table style="width:100%;border-collapse:collapse;min-width:700px">
          <thead>
            <tr>
              <th style="padding:8px;background:#f5f5f5;border:1px solid #e0e0e0;width:110px">Date</th>
              <th v-for="mt in MEAL_TYPES" :key="mt"
                style="padding:8px;background:#f5f5f5;border:1px solid #e0e0e0;text-align:center;min-width:160px">
                <q-badge :color="MEAL_COLOR[mt]" :label="MEAL_LABEL[mt]" />
              </th>
            </tr>
          </thead>
          <tbody>
            <template v-for="(day, idx) in calendarDays" :key="isoDate(day)">
              <!-- Week separator -->
              <tr v-if="idx > 0 && dayOfWeekMon0(day) === 0">
                <td :colspan="5" style="padding:0;background:#f0f4ff;height:3px;border:none" />
              </tr>
              <tr :style="!isCurrentMonth(day) ? 'opacity:0.35' : ''">
                <td style="padding:6px 8px;border:1px solid #e0e0e0;white-space:nowrap;vertical-align:top">
                  <div class="text-caption text-weight-bold">{{ DAYS[dayOfWeekMon0(day)] }}</div>
                  <div class="text-caption text-grey-6">{{ day.toLocaleDateString("en-CA",{month:"short",day:"numeric"}) }}</div>
                </td>
                <td
                  v-for="mt in MEAL_TYPES" :key="mt"
                  :style="`padding:6px 8px;border:1px solid #e0e0e0;vertical-align:top;min-height:48px;${canManage && isCurrentMonth(day) ? 'cursor:pointer' : ''}`"
                  class="meal-cell"
                  @click="canManage && isCurrentMonth(day) && openEditDay(day, mt)"
                >
                  <div style="min-height:40px">
                    <div v-if="getMeal(day,mt)?.menu" class="text-caption text-grey-9" style="line-height:1.4">
                      {{ getMeal(day,mt)?.menu }}
                    </div>
                    <div v-else class="text-caption text-grey-4">—</div>
                    <div v-if="getMeal(day,mt)?.calories" style="font-size:0.68rem;color:#999">
                      {{ getMeal(day,mt)?.calories }} kcal
                    </div>
                  </div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>
    </template>

    <!-- ══════════════════════════════════════════════════════════════════════
         STATISTICS TAB
    ══════════════════════════════════════════════════════════════════════ -->
    <template v-else>
      <div v-if="monthLoading" class="flex flex-center q-py-xl">
        <q-spinner color="primary" size="3rem" />
      </div>
      <template v-else>
        <!-- Summary cards -->
        <div class="row q-gutter-md q-mb-lg">
          <div class="col-12 col-sm-6 col-md-3">
            <q-card flat bordered>
              <q-card-section class="text-center">
                <div class="text-h4 text-weight-bold text-positive">{{ stats.completeDays }}</div>
                <div class="text-caption text-grey-6">Complete Days</div>
                <div class="text-caption text-grey-4">all 4 meals filled</div>
              </q-card-section>
            </q-card>
          </div>
          <div class="col-12 col-sm-6 col-md-3">
            <q-card flat bordered>
              <q-card-section class="text-center">
                <div class="text-h4 text-weight-bold text-warning">{{ stats.partialDays }}</div>
                <div class="text-caption text-grey-6">Partial Days</div>
                <div class="text-caption text-grey-4">some meals missing</div>
              </q-card-section>
            </q-card>
          </div>
          <div class="col-12 col-sm-6 col-md-3">
            <q-card flat bordered>
              <q-card-section class="text-center">
                <div class="text-h4 text-weight-bold text-primary">{{ stats.avgDailyCal }}</div>
                <div class="text-caption text-grey-6">Avg Daily kcal</div>
                <div class="text-caption text-grey-4">days with data</div>
              </q-card-section>
            </q-card>
          </div>
          <div class="col-12 col-sm-6 col-md-3">
            <q-card flat bordered>
              <q-card-section class="text-center">
                <div class="text-h4 text-weight-bold text-secondary">{{ stats.fillRate }}%</div>
                <div class="text-caption text-grey-6">Fill Rate</div>
                <div class="text-caption text-grey-4">plan completion</div>
              </q-card-section>
            </q-card>
          </div>
        </div>

        <!-- Avg calories by meal type -->
        <q-card flat bordered class="q-mb-lg">
          <q-card-section>
            <div class="text-subtitle1 text-weight-bold q-mb-md">Average Calories by Meal</div>
            <div class="row q-gutter-md">
              <div v-for="mt in MEAL_TYPES" :key="mt" class="col-12 col-sm-6 col-md-3">
                <div class="row items-center q-gutter-sm">
                  <q-badge :color="MEAL_COLOR[mt]" :label="MEAL_LABEL[mt]" style="min-width:70px" />
                  <div class="text-weight-bold">{{ stats.calsByType[mt] || '—' }} kcal</div>
                </div>
                <q-linear-progress
                  :value="stats.calsByType[mt] / 1000"
                  :color="MEAL_COLOR[mt]"
                  class="q-mt-xs"
                  size="8px"
                  rounded
                />
              </div>
            </div>
          </q-card-section>
        </q-card>

        <!-- Daily calorie bar chart (plain HTML/CSS bars) -->
        <q-card flat bordered>
          <q-card-section>
            <div class="text-subtitle1 text-weight-bold q-mb-sm">Daily Calories — {{ MONTHS[viewMonth] }}</div>
            <div class="row items-end q-gutter-xs" style="height:120px;overflow-x:auto;flex-wrap:nowrap">
              <div
                v-for="d in stats.dailyCals" :key="d.label"
                class="column items-center"
                style="min-width:18px;flex:1"
              >
                <div
                  :style="{
                    width:'100%',
                    background: d.total > 0 ? '#14b8a6' : '#e0e0e0',
                    height: d.total > 0 ? Math.max(4, Math.round((d.total / 3000) * 96)) + 'px' : '4px',
                    borderRadius:'3px 3px 0 0',
                    transition:'height 0.3s',
                  }"
                >
                  <q-tooltip>{{ d.label }}: {{ d.total || '—' }} kcal</q-tooltip>
                </div>
                <div class="text-grey-5" style="font-size:0.55rem;margin-top:2px">{{ d.label }}</div>
              </div>
            </div>
            <div class="text-caption text-grey-5 q-mt-xs">Bar height proportional to 3,000 kcal/day</div>
          </q-card-section>
        </q-card>
      </template>
    </template>

    <!-- ══════════════════════════════════════════════════════════════════════
         EDIT DIALOG (shared)
    ══════════════════════════════════════════════════════════════════════ -->
    <q-dialog v-model="showEditDialog" persistent>
      <q-card style="min-width:380px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">
            Edit Meal
            <span v-if="editTarget" class="text-grey-6 text-subtitle2 q-ml-sm">
              {{ editTarget.mealType.charAt(0).toUpperCase() + editTarget.mealType.slice(1) }}
            </span>
          </div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section>
          <q-form ref="mealFormRef" class="q-gutter-sm">
            <q-input
              v-model="editForm.menu"
              label="Menu *"
              type="textarea" rows="2" autogrow
              outlined dense
              placeholder="e.g. Oatmeal with fruit, whole-grain toast"
              :rules="[v => !!v?.trim() || 'Menu cannot be empty']"
              lazy-rules="ondemand"
            />
            <q-input v-model="editForm.calories" label="Calories (kcal)" type="number" outlined dense />
            <q-input v-model="editForm.notes"    label="Notes (allergies, dietary restrictions)" outlined dense />
          </q-form>
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Save" unelevated :loading="submitting" @click="saveEdit" />
        </q-card-actions>
      </q-card>
    </q-dialog>

  </q-page>
</template>

<style scoped>
.meal-cell:hover {
  background: #f0f9ff;
}
</style>
