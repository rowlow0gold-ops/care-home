<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { useAuthStore } from "@/stores/auth";

const $q   = useQuasar();
const auth = useAuthStore();

// ── Role helpers ──────────────────────────────────────────────────────────────
const isManager = computed(() => auth.user?.role === "manager" || auth.user?.role === "admin")
const isStaff   = computed(() => auth.user?.role === "staff")

// ── Team types & state ────────────────────────────────────────────────────────
interface TeamMember { user_id: number; full_name: string; position: string | null }
interface Team { id: number; name: string; color: string; manager_id: number | null; manager_name: string | null; staff: TeamMember[] }

const allTeams       = ref<Team[]>([])
const myTeam         = ref<Team | null>(null)
const selectedTeamId = ref<number | null>(null)   // null = All Teams (admin/manager only)

const teamOptions = computed(() => [
  { label: "All Teams", value: null },
  ...allTeams.value.map(t => ({ label: t.name, value: t.id, color: t.color })),
])

async function loadTeams() {
  try {
    allTeams.value = await invoke<Team[]>("list_teams")
    const me = await invoke<Team | null>("get_user_team", { userId: auth.user!.id })
    myTeam.value = me
    if (isStaff.value) {
      // Staff: locked to their own team
      selectedTeamId.value = me?.id ?? null
    } else if (auth.user?.role === "manager") {
      // Manager: default to their own team, but can switch
      selectedTeamId.value = me?.id ?? null
    } else {
      // Admin: default to all
      selectedTeamId.value = null
    }
  } catch (_) {}
};

// ── View mode ─────────────────────────────────────────────────────────────────
const viewMode = ref<"week" | "month">("week");

// ── Types ─────────────────────────────────────────────────────────────────────
interface ScheduleEntry {
  id:          number;
  staff_id:    number;
  staff_name:  string;
  shift_date:  string;
  shift_start: string;
  shift_end:   string;
  shift_hours: number;
  notes:       string | null;
}
interface StaffOption { label: string; value: number; role: string }

// ── Date helpers ──────────────────────────────────────────────────────────────
function localDateStr(d: Date): string {
  const y  = d.getFullYear();
  const m  = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}
function weekMonday(d: Date): Date {
  const c = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const dow = c.getDay();
  c.setDate(c.getDate() + (dow === 0 ? -6 : 1 - dow));
  return c;
}
function addDays(d: Date, n: number): Date {
  const c = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  c.setDate(c.getDate() + n);
  return c;
}
const DAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const todayStr   = computed(() => localDateStr(new Date()));
function isToday(d: Date) { return localDateStr(d) === todayStr.value; }

// ── Week navigation ───────────────────────────────────────────────────────────
const currentMonday = ref<Date>(weekMonday(new Date()));
const weekDates = computed<Date[]>(() =>
  Array.from({ length: 7 }, (_, i) => addDays(currentMonday.value, i))
);
const weekStartStr = computed(() => localDateStr(weekDates.value[0]));
const weekEndStr   = computed(() => localDateStr(weekDates.value[6]));

function weekLabel(): string {
  const s = weekDates.value[0], e = weekDates.value[6];
  const fmt = (d: Date) => d.toLocaleDateString("en-CA", { month: "short", day: "numeric" });
  return `${fmt(s)} – ${fmt(e)}, ${e.getFullYear()}`;
}
function prevWeek() { currentMonday.value = addDays(currentMonday.value, -7); }
function nextWeek() { currentMonday.value = addDays(currentMonday.value,  7); }
function goToday() {
  currentMonday.value = weekMonday(new Date());
  currentMonth.value  = new Date(new Date().getFullYear(), new Date().getMonth(), 1);
}

// ── Month navigation ──────────────────────────────────────────────────────────
const currentMonth = ref<Date>(new Date(new Date().getFullYear(), new Date().getMonth(), 1));

function prevMonth() {
  const d = currentMonth.value;
  currentMonth.value = new Date(d.getFullYear(), d.getMonth() - 1, 1);
}
function nextMonth() {
  const d = currentMonth.value;
  currentMonth.value = new Date(d.getFullYear(), d.getMonth() + 1, 1);
}
function monthLabel(): string {
  return currentMonth.value.toLocaleDateString("en-CA", { month: "long", year: "numeric" });
}
const monthStartStr = computed(() => localDateStr(currentMonth.value));
const monthEndStr   = computed(() => {
  const d = currentMonth.value;
  return localDateStr(new Date(d.getFullYear(), d.getMonth() + 1, 0));
});

/**
 * Build a grid of weeks for the current month.
 * Grid starts on the Monday on-or-before the 1st, ends on the Sunday on-or-after the last day.
 */
const monthGrid = computed<Date[][]>(() => {
  const year  = currentMonth.value.getFullYear();
  const month = currentMonth.value.getMonth();

  const firstDay = new Date(year, month, 1);
  const lastDay  = new Date(year, month + 1, 0);

  // Grid start = Monday of week containing the 1st
  const gridStart = weekMonday(firstDay);

  // Grid end = Sunday of week containing the last day
  const lastDow   = lastDay.getDay(); // 0=Sun … 6=Sat
  const gridEnd   = addDays(lastDay, lastDow === 0 ? 0 : 7 - lastDow);

  const weeks: Date[][] = [];
  let cur = new Date(gridStart.getTime());

  while (cur <= gridEnd) {
    const week: Date[] = [];
    for (let i = 0; i < 7; i++) {
      week.push(new Date(cur));
      cur.setDate(cur.getDate() + 1);
    }
    weeks.push(week);
  }
  return weeks;
});

// ── Data ──────────────────────────────────────────────────────────────────────
const entries   = ref<ScheduleEntry[]>([]);
const loading   = ref(false);
const staffList = ref<StaffOption[]>([]);

const staffRows = computed<StaffOption[]>(() => {
  const base = staffList.value.length > 0
    ? staffList.value
    : (() => {
        const seen = new Map<number, string>();
        for (const e of entries.value) seen.set(e.staff_id, e.staff_name);
        return Array.from(seen.entries())
          .sort((a, b) => a[1].localeCompare(b[1]))
          .map(([value, label]) => ({ label, value, role: "staff" }));
      })();

  // When a specific team is selected, only show members of that team, manager first
  if (selectedTeamId.value !== null) {
    const team = allTeams.value.find(t => t.id === selectedTeamId.value);
    if (team) {
      const memberIds = new Set<number>([
        ...(team.manager_id ? [team.manager_id] : []),
        ...team.staff.map(s => s.user_id),
      ]);
      const filtered = base.filter(s => memberIds.has(s.value));
      // Sort: manager first, then alphabetically
      return filtered.sort((a, b) => {
        const aIsManager = a.value === team.manager_id;
        const bIsManager = b.value === team.manager_id;
        if (aIsManager && !bIsManager) return -1;
        if (!aIsManager && bIsManager) return 1;
        return a.label.localeCompare(b.label);
      });
    }
  }
  return base;
});

const cellMap = computed(() => {
  const map = new Map<string, ScheduleEntry[]>();
  for (const e of entries.value) {
    const k = `${e.staff_id}-${e.shift_date}`;
    if (!map.has(k)) map.set(k, []);
    map.get(k)!.push(e);
  }
  return map;
});
function cellEntries(staffId: number, date: Date): ScheduleEntry[] {
  return cellMap.value.get(`${staffId}-${localDateStr(date)}`) ?? [];
}

const dayMap = computed(() => {
  const map = new Map<string, ScheduleEntry[]>();
  for (const e of entries.value) {
    if (!map.has(e.shift_date)) map.set(e.shift_date, []);
    map.get(e.shift_date)!.push(e);
  }
  return map;
});
function dayEntries(date: Date): ScheduleEntry[] {
  return dayMap.value.get(localDateStr(date)) ?? [];
}

function shiftLabel(e: ScheduleEntry): string {
  return `${e.shift_start}–${e.shift_end} (${e.shift_hours}h)`;
}
function shiftColor(e: ScheduleEntry): string {
  if (e.shift_hours >= 12) return "teal";
  if (e.shift_start === "07:00") return "blue";
  if (e.shift_start === "15:00") return "deep-orange";
  return "purple";
}
function weekHours(staffId: number): number {
  return entries.value
    .filter(e => e.staff_id === staffId)
    .reduce((sum, e) => sum + e.shift_hours, 0);
}

// ── Load ──────────────────────────────────────────────────────────────────────
async function loadSchedule() {
  loading.value = true;
  try {
    const start = viewMode.value === "week" ? weekStartStr.value : monthStartStr.value;
    const end   = viewMode.value === "week" ? weekEndStr.value   : monthEndStr.value;
    entries.value = await invoke<ScheduleEntry[]>("list_schedules", {
      staffId: null, teamId: selectedTeamId.value, weekStart: start, weekEnd: end,
    });
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load schedule: ${e}` });
  } finally {
    loading.value = false;
  }
}
async function loadStaffList() {
  try {
    interface UserRow { id: number; full_name: string; role: string }
    const users = await invoke<UserRow[]>("list_users");
    staffList.value = users
      .filter(u => u.role === "staff" || u.role === "manager")
      .map(u => ({ label: u.full_name, value: u.id, role: u.role }))
      .sort((a, b) => a.label.localeCompare(b.label));
  } catch (_) { /* best effort */ }
}

// Reload schedule whenever team filter changes
watch(selectedTeamId, loadSchedule);
watch(viewMode, loadSchedule);
watch([weekStartStr, weekEndStr], () => { if (viewMode.value === "week")   loadSchedule(); });
watch([monthStartStr, monthEndStr], () => { if (viewMode.value === "month") loadSchedule(); });
onMounted(async () => { await loadTeams(); await loadStaffList(); await loadSchedule(); });

// ── Shift presets ─────────────────────────────────────────────────────────────
const SHIFT_PRESETS = [
  { label: "Day 12h (07:00–19:00)",      start: "07:00", end: "19:00", hours: 12 },
  { label: "Night 12h (19:00–07:00)",    start: "19:00", end: "07:00", hours: 12 },
  { label: "Morning 8h (07:00–15:00)",   start: "07:00", end: "15:00", hours:  8 },
  { label: "Afternoon 8h (15:00–23:00)", start: "15:00", end: "23:00", hours:  8 },
  { label: "Night 8h (23:00–07:00)",     start: "23:00", end: "07:00", hours:  8 },
  { label: "Custom",                      start: "",      end: "",      hours:  0 },
];

// ── Add shift (cell hover button) ─────────────────────────────────────────────
const showAdd    = ref(false);
const submitting = ref(false);
const form = ref({
  staff_id:    null as number | null,
  shift_date:  "",
  preset:      SHIFT_PRESETS[0],
  shift_start: "07:00",
  shift_end:   "19:00",
  shift_hours: 12,
  notes:       "",
});
function applyPreset() {
  if (form.value.preset.label !== "Custom") {
    form.value.shift_start = form.value.preset.start;
    form.value.shift_end   = form.value.preset.end;
    form.value.shift_hours = form.value.preset.hours;
  }
}
function openCustom() {
  const customPreset = SHIFT_PRESETS.find(p => p.label === "Custom")!;
  form.value = {
    staff_id:    null,
    shift_date:  localDateStr(new Date()),
    preset:      customPreset,
    shift_start: "",
    shift_end:   "",
    shift_hours: 0,
    notes:       "",
  };
  showAdd.value = true;
}
function openAddForCell(staffId: number, date: Date) {
  form.value = {
    staff_id:    staffId,
    shift_date:  localDateStr(date),
    preset:      SHIFT_PRESETS[0],
    shift_start: "07:00",
    shift_end:   "19:00",
    shift_hours: 12,
    notes:       "",
  };
  showAdd.value = true;
}
async function submitAdd() {
  if (!form.value.staff_id || !form.value.shift_date) {
    $q.notify({ type: "negative", message: "Staff member and date are required." });
    return;
  }
  submitting.value = true;
  try {
    await invoke("create_schedule", {
      input: {
        staff_id:    form.value.staff_id,
        shift_date:  form.value.shift_date,
        shift_start: form.value.shift_start,
        shift_end:   form.value.shift_end,
        shift_hours: form.value.shift_hours,
        notes:       form.value.notes || null,
      },
      actorRole: auth.user?.role,
      actorId:   auth.user?.id,
    });
    $q.notify({ type: "positive", message: "Shift added." });
    showAdd.value = false;
    await loadSchedule();
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed: ${e}` });
  } finally {
    submitting.value = false;
  }
}

// ── Drag & drop (pointer-events based — HTML5 DnD is broken in WKWebView) ────
type Preset = typeof SHIFT_PRESETS[0];
type DragState =
  | { kind: "entry";  id: number }
  | { kind: "preset"; preset: Preset }
  | null;

let _drag: DragState = null;
const isDragging    = ref(false);
const dropTargetKey = ref("");
const droppedId     = ref<number | null>(null);
const ghostStyle    = ref({ left: "0px", top: "0px", display: "none" });
const ghostLabel    = ref("");
const ghostColor    = ref("blue");

function presetColor(p: Preset): string {
  if (p.hours >= 12)       return "teal";
  if (p.start === "07:00") return "blue";
  if (p.start === "15:00") return "deep-orange";
  return "purple";
}

function dropKey(staffId: number | null, date: Date): string {
  return staffId === null ? `day-${localDateStr(date)}` : `${staffId}-${localDateStr(date)}`;
}

function getCellUnder(x: number, y: number): HTMLElement | null {
  // Temporarily hide ghost so elementFromPoint sees what's underneath
  const ghost = document.getElementById("drag-ghost");
  if (ghost) ghost.style.display = "none";
  const el = document.elementFromPoint(x, y)?.closest("[data-cell-key]") as HTMLElement | null;
  if (ghost) ghost.style.display = "";
  return el;
}

function onPointerMove(e: PointerEvent) {
  ghostStyle.value = { left: e.clientX + 14 + "px", top: e.clientY - 16 + "px", display: "flex" };
  const cell = getCellUnder(e.clientX, e.clientY);
  dropTargetKey.value = cell?.dataset.cellKey ?? "";
}

async function onPointerUp(e: PointerEvent) {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup",   onPointerUp);
  ghostStyle.value    = { left: "0px", top: "0px", display: "none" };
  isDragging.value    = false;
  dropTargetKey.value = "";

  const payload = _drag;
  _drag = null;
  if (!payload) return;

  const cell = getCellUnder(e.clientX, e.clientY);
  if (!cell) return;

  const staffId = cell.dataset.staffId ? Number(cell.dataset.staffId) : null;
  const date    = new Date(cell.dataset.date + "T00:00:00");

  // ── Move existing entry ────────────────────────────────────────────────────
  if (payload.kind === "entry") {
    const entry = entries.value.find(en => en.id === payload.id);
    if (!entry) return;
    const newDate    = localDateStr(date);
    const newStaffId = staffId ?? entry.staff_id;
    if (entry.staff_id === newStaffId && entry.shift_date === newDate) return;
    try {
      await invoke("update_schedule", {
        id:        entry.id,
        input:     { staff_id: newStaffId, shift_date: newDate, shift_start: entry.shift_start, shift_end: entry.shift_end, shift_hours: entry.shift_hours, notes: entry.notes ?? null },
        actorRole: auth.user?.role,
      });
      await loadSchedule();
      await nextTick();
      droppedId.value = entry.id;
      setTimeout(() => { droppedId.value = null; }, 450);
    } catch (err) {
      $q.notify({ type: "negative", message: `Failed to move shift: ${err}` });
    }
    return;
  }

  // ── Drop preset ────────────────────────────────────────────────────────────
  if (payload.kind === "preset" && staffId !== null) {
    const preset = payload.preset;

    // Custom preset → open dialog pre-filled with the target cell
    if (preset.label === "Custom") {
      form.value = {
        staff_id:    staffId,
        shift_date:  localDateStr(date),
        preset:      preset,
        shift_start: "",
        shift_end:   "",
        shift_hours: 0,
        notes:       "",
      };
      showAdd.value = true;
      return;
    }

    try {
      await invoke("create_schedule", {
        input:     { staff_id: staffId, shift_date: localDateStr(date), shift_start: preset.start, shift_end: preset.end, shift_hours: preset.hours, notes: null },
        actorRole: auth.user?.role,
        actorId:   auth.user?.id,
      });
      $q.notify({ type: "positive", message: `${preset.label} added.` });
      await loadSchedule();
    } catch (err) {
      $q.notify({ type: "negative", message: `Failed: ${err}` });
    }
  }
}

function startDrag(e: PointerEvent, state: DragState, label: string, color: string) {
  if (!isManager.value) return;
  e.preventDefault();
  _drag           = state;
  isDragging.value = true;
  ghostLabel.value = label;
  ghostColor.value = color;
  ghostStyle.value = { left: e.clientX + 14 + "px", top: e.clientY - 16 + "px", display: "flex" };
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup",   onPointerUp);
}

// ── Edit shift dialog ─────────────────────────────────────────────────────────
const showEdit       = ref(false);
const editingId      = ref<number | null>(null);
const editingStaffId = ref<number | null>(null);
const editSubmitting = ref(false);
const editForm = ref({
  shift_date: "", preset: SHIFT_PRESETS[0], shift_start: "07:00", shift_end: "19:00", shift_hours: 12, notes: "",
});
function applyEditPreset() {
  if (editForm.value.preset.label !== "Custom") {
    editForm.value.shift_start = editForm.value.preset.start;
    editForm.value.shift_end   = editForm.value.preset.end;
    editForm.value.shift_hours = editForm.value.preset.hours;
  }
}
function openEditDialog(entry: ScheduleEntry) {
  if (!isManager.value) return;
  editingId.value      = entry.id;
  editingStaffId.value = entry.staff_id;
  const matched = SHIFT_PRESETS.find(
    p => p.start === entry.shift_start && p.end === entry.shift_end && p.hours === entry.shift_hours
  ) ?? SHIFT_PRESETS.find(p => p.label === "Custom")!;
  editForm.value = {
    shift_date: entry.shift_date, preset: matched,
    shift_start: entry.shift_start, shift_end: entry.shift_end,
    shift_hours: entry.shift_hours, notes: entry.notes ?? "",
  };
  showEdit.value = true;
}
async function submitEdit() {
  if (!editingId.value || !editingStaffId.value) return;
  editSubmitting.value = true;
  try {
    await invoke("update_schedule", {
      id: editingId.value,
      input: {
        staff_id:    editingStaffId.value,
        shift_date:  editForm.value.shift_date,
        shift_start: editForm.value.shift_start,
        shift_end:   editForm.value.shift_end,
        shift_hours: editForm.value.shift_hours,
        notes:       editForm.value.notes || null,
      },
      actorRole: auth.user?.role,
    });
    $q.notify({ type: "positive", message: "Shift updated." });
    showEdit.value = false;
    await loadSchedule();
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed: ${e}` });
  } finally {
    editSubmitting.value = false;
  }
}

// ── Month cell expand ─────────────────────────────────────────────────────────
const expandedDays = ref(new Set<string>());
function toggleExpand(date: Date) {
  const k = localDateStr(date);
  if (expandedDays.value.has(k)) expandedDays.value.delete(k);
  else expandedDays.value.add(k);
  expandedDays.value = new Set(expandedDays.value); // trigger reactivity
}
function isExpanded(date: Date) { return expandedDays.value.has(localDateStr(date)); }

// ── Delete shift ──────────────────────────────────────────────────────────────
async function deleteShift(entry: ScheduleEntry) {
  $q.dialog({
    title: "Remove shift?",
    message: `Remove ${entry.shift_start}–${entry.shift_end} for ${entry.staff_name} on ${entry.shift_date}?`,
    cancel: { label: "Cancel", flat: true },
    ok:     { label: "Remove", color: "negative", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    try {
      await invoke("delete_schedule", { id: entry.id, actorRole: auth.user?.role });
      $q.notify({ type: "positive", message: "Shift removed." });
      await loadSchedule();
    } catch (e) {
      $q.notify({ type: "negative", message: `Failed: ${e}` });
    }
  });
}
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">Schedule</div>
        <div class="text-caption text-grey-6">
          <template v-if="isStaff && myTeam">
            <q-icon :name="'o_groups'" size="xs" class="q-mr-xs" />
            {{ myTeam.name }} · {{ myTeam.manager_name }}
          </template>
          <template v-else>
            {{ isManager ? "Manage staff shift schedule" : "View staff shift schedule (read-only)" }}
          </template>
        </div>
      </div>

      <!-- Team filter: manager / admin only -->
      <div v-if="!isStaff && allTeams.length" class="col-auto">
        <q-btn-group unelevated rounded>
          <q-btn
            v-for="opt in teamOptions"
            :key="String(opt.value)"
            :label="opt.label"
            :color="selectedTeamId === opt.value ? 'primary' : 'grey-2'"
            :text-color="selectedTeamId === opt.value ? 'white' : 'grey-8'"
            :style="opt.value !== null && selectedTeamId !== opt.value
              ? { borderBottom: `2px solid ${(opt as any).color}` }
              : {}"
            size="sm" dense no-caps unelevated
            @click="selectedTeamId = opt.value"
          />
        </q-btn-group>
      </div>

      <!-- Staff: show their team badge -->
      <div v-if="isStaff && myTeam" class="col-auto">
        <q-badge
          :style="{ background: myTeam.color }"
          text-color="white"
          :label="myTeam.name"
        />
      </div>

      <div class="col-auto">
        <q-btn-toggle
          v-model="viewMode"
          toggle-color="primary"
          :options="[{ label: 'Week', value: 'week' }, { label: 'Month', value: 'month' }]"
          unelevated rounded dense size="sm"
        />
      </div>
    </div>

    <!-- Drag palette (managers, week view only) -->
    <div v-if="isManager && viewMode === 'week'" class="drag-palette q-mb-md">
      <div class="text-caption text-grey-6 q-mb-sm">
        <q-icon name="o_drag_indicator" size="xs" class="q-mr-xs" />Drag a shift onto any cell to assign it
      </div>
      <div class="row q-gutter-sm items-center">
        <div
          v-for="p in SHIFT_PRESETS.filter(p => p.label !== 'Custom')"
          :key="p.label"
          class="palette-chip"
          :class="`palette-chip--${presetColor(p)}`"
          @pointerdown="startDrag($event, { kind: 'preset', preset: p }, p.label, presetColor(p))"
        >
          <q-icon name="o_drag_indicator" size="xs" class="q-mr-xs opacity-60" />
          {{ p.label }}
        </div>
        <div class="palette-chip palette-chip--custom"
             @pointerdown="startDrag($event, { kind: 'preset', preset: SHIFT_PRESETS.find(p => p.label === 'Custom')! }, 'Custom', 'custom')">
          <q-icon name="o_drag_indicator" size="xs" class="q-mr-xs opacity-60" />
          Custom
        </div>
      </div>
    </div>

    <!-- Navigator -->
    <div class="row items-center q-mb-md q-gutter-sm">
      <q-btn flat round dense icon="o_chevron_left"  @click="viewMode === 'week' ? prevWeek() : prevMonth()" />
      <q-btn flat round dense icon="o_chevron_right" @click="viewMode === 'week' ? nextWeek() : nextMonth()" />
      <span class="text-subtitle1 text-weight-medium q-mx-sm">
        {{ viewMode === 'week' ? weekLabel() : monthLabel() }}
      </span>
      <q-btn flat dense size="sm" label="Today" @click="goToday" class="text-grey-7" />
      <q-spinner-dots v-if="loading" color="primary" size="1.2rem" class="q-ml-sm" />
    </div>

    <!-- ══ WEEK VIEW ══════════════════════════════════════════════════════════ -->
    <template v-if="viewMode === 'week'">
      <div class="schedule-wrap">
        <table class="schedule-table">
          <thead>
            <tr>
              <th class="staff-col">Staff Member</th>
              <th
                v-for="(date, di) in weekDates" :key="di"
                :class="['day-col', { 'today-col': isToday(date) }]"
              >
                <div class="day-label">{{ DAY_LABELS[di] }}</div>
                <div class="day-date" :class="isToday(date) ? 'text-primary text-weight-bold' : 'text-grey-6'">
                  {{ date.getMonth() + 1 }}/{{ date.getDate() }}
                </div>
              </th>
              <th class="hours-col">Hrs/wk</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="staff in staffRows" :key="staff.value">
              <td class="staff-name-cell">
                <q-icon name="o_person" size="xs" color="grey-6" class="q-mr-xs" />
                <span>{{ staff.label }}</span>
                <q-badge v-if="staff.value === auth.user?.id"
                         color="primary" label="You"
                         class="q-ml-xs" style="font-size:0.65rem" />
              </td>
              <td
                v-for="(date, di) in weekDates" :key="di"
                class="day-cell"
                :class="{
                  'today-bg':  isToday(date),
                  'drop-over': isDragging && dropTargetKey === dropKey(staff.value, date)
                }"
                :data-cell-key="dropKey(staff.value, date)"
                :data-staff-id="staff.value"
                :data-date="localDateStr(date)"
              >
                <div
                  v-for="entry in cellEntries(staff.value, date)" :key="entry.id"
                  class="shift-chip"
                  :class="[`shift-chip--${shiftColor(entry)}`, { 'shift-chip--clickable': isManager, 'shift-chip--dropped': droppedId === entry.id }]"
                  @pointerdown.stop="isManager && startDrag($event, { kind: 'entry', id: entry.id }, shiftLabel(entry), shiftColor(entry))"
                  @click.stop="openEditDialog(entry)"
                >
                  <span class="shift-time">{{ shiftLabel(entry) }}</span>
                  <div v-if="isManager" class="shift-actions">
                    <q-btn flat round dense icon="o_edit"  size="xs" class="shift-edit"   @click.stop="openEditDialog(entry)" />
                    <q-btn flat round dense icon="o_close" size="xs" class="shift-delete" @click.stop="deleteShift(entry)" />
                  </div>
                  <q-tooltip v-if="entry.notes">{{ entry.notes }}</q-tooltip>
                </div>
                <!-- Add button on hover (manager only) -->
                <q-btn
                  v-if="isManager"
                  flat round dense icon="o_add" size="xs" color="grey-5"
                  class="add-btn"
                  @click="openAddForCell(staff.value, date)"
                >
                  <q-tooltip>Add shift</q-tooltip>
                </q-btn>
              </td>
              <td class="hours-cell">
                <span :class="weekHours(staff.value) > 0 ? 'text-weight-medium' : 'text-grey-4'">
                  {{ weekHours(staff.value) > 0 ? weekHours(staff.value) + 'h' : '—' }}
                </span>
              </td>
            </tr>
            <tr v-if="staffRows.length === 0 && !loading">
              <td :colspan="9" class="text-center text-grey-5 q-py-xl">
                <q-icon name="o_event_busy" size="3rem" color="grey-4" /><br />No schedule data for this week.
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>

    <!-- ══ MONTH VIEW ════════════════════════════════════════════════════════ -->
    <template v-else>
      <div class="month-grid">
        <div v-for="d in DAY_LABELS" :key="d" class="month-dow-header">{{ d }}</div>

        <template v-for="(week, wi) in monthGrid" :key="wi">
          <div
            v-for="(date, di) in week" :key="di"
            class="month-day-cell"
            :class="{
              'month-day--other': date.getMonth() !== currentMonth.getMonth(),
              'month-day--today': isToday(date),
              'month-day--drop':  isDragging && dropTargetKey === dropKey(null, date),
            }"
            :data-cell-key="dropKey(null, date)"
            :data-date="localDateStr(date)"
          >
            <div class="month-day-num" :class="isToday(date) ? 'today-badge' : ''">
              {{ date.getDate() }}
            </div>

            <template v-for="(entry, ei) in dayEntries(date)" :key="entry.id">
              <div
                v-if="isExpanded(date) || ei < 3"
                class="month-shift-bar"
                :class="[`shift-chip--${shiftColor(entry)}`, { 'shift-chip--clickable': isManager, 'shift-chip--dropped': droppedId === entry.id }]"
                @pointerdown.stop="isManager && startDrag($event, { kind: 'entry', id: entry.id }, shiftLabel(entry), shiftColor(entry))"
                @click.stop="openEditDialog(entry)"
              >
                <span class="month-shift-name">{{ entry.staff_name.split(' ')[0] }}</span>
                <span class="month-shift-time">{{ entry.shift_start }}</span>
                <q-btn
                  v-if="isManager"
                  flat round dense icon="o_close" size="xs"
                  class="shift-delete month-delete"
                  @click.stop="deleteShift(entry)"
                />
                <q-tooltip>{{ entry.staff_name }} · {{ shiftLabel(entry) }}<span v-if="entry.notes"> · {{ entry.notes }}</span></q-tooltip>
              </div>
            </template>
            <div
              v-if="dayEntries(date).length > 3"
              class="month-more"
              @click.stop="toggleExpand(date)"
            >
              {{ isExpanded(date) ? '▲ show less' : `+${dayEntries(date).length - 3} more` }}
            </div>
          </div>
        </template>
      </div>
    </template>

    <!-- Legend -->
    <div class="row q-mt-md q-gutter-sm items-center">
      <span class="text-caption text-grey-6">Legend:</span>
      <q-chip dense size="sm" color="teal"        text-color="white">12h shift</q-chip>
      <q-chip dense size="sm" color="blue"        text-color="white">Morning 8h</q-chip>
      <q-chip dense size="sm" color="deep-orange" text-color="white">Afternoon 8h</q-chip>
      <q-chip dense size="sm" color="purple"      text-color="white">Night 8h</q-chip>
    </div>

    <!-- ── Add Shift Dialog ────────────────────────────────────────────────── -->
    <q-dialog v-model="showAdd" persistent>
      <q-card style="min-width: 420px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">Add Shift</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-sm">
          <q-select v-model="form.staff_id" :options="staffList" label="Staff Member *" outlined dense emit-value map-options />
          <div class="cursor-pointer">
            <q-input v-model="form.shift_date" label="Date *" outlined dense readonly style="pointer-events:none">
              <template #append>
                <q-icon name="o_event" color="grey-6" />
              </template>
            </q-input>
            <q-popup-proxy transition-show="scale" transition-hide="scale">
              <q-date v-model="form.shift_date" mask="YYYY-MM-DD" minimal>
                <div class="row items-center justify-end q-pa-sm">
                  <q-btn v-close-popup label="OK" color="primary" flat dense />
                </div>
              </q-date>
            </q-popup-proxy>
          </div>
          <q-select v-model="form.preset" :options="SHIFT_PRESETS" label="Shift Type" outlined dense option-label="label" @update:model-value="applyPreset" />
          <template v-if="form.preset.label === 'Custom'">
            <div class="row q-gutter-sm">
              <q-input v-model="form.shift_start" label="Start" outlined dense class="col" hint="HH:MM" />
              <q-input v-model="form.shift_end"   label="End"   outlined dense class="col" hint="HH:MM" />
              <q-input v-model.number="form.shift_hours" label="Hours" type="number" outlined dense class="col" />
            </div>
          </template>
          <div v-else class="text-caption text-grey-7 q-px-xs">
            {{ form.shift_start }} – {{ form.shift_end }} · {{ form.shift_hours }}h
          </div>
          <q-input v-model="form.notes" label="Notes (optional)" outlined dense />
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Add Shift" unelevated :loading="submitting" @click="submitAdd" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- ── Edit Shift Dialog ───────────────────────────────────────────────── -->
    <q-dialog v-model="showEdit" persistent>
      <q-card style="min-width: 420px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">Edit Shift</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-sm">
          <div class="cursor-pointer">
            <q-input v-model="editForm.shift_date" label="Date *" outlined dense readonly style="pointer-events:none">
              <template #append>
                <q-icon name="o_event" color="grey-6" />
              </template>
            </q-input>
            <q-popup-proxy transition-show="scale" transition-hide="scale">
              <q-date v-model="editForm.shift_date" mask="YYYY-MM-DD" minimal>
                <div class="row items-center justify-end q-pa-sm">
                  <q-btn v-close-popup label="OK" color="primary" flat dense />
                </div>
              </q-date>
            </q-popup-proxy>
          </div>
          <q-select v-model="editForm.preset" :options="SHIFT_PRESETS" label="Shift Type" outlined dense option-label="label" @update:model-value="applyEditPreset" />
          <template v-if="editForm.preset.label === 'Custom'">
            <div class="row q-gutter-sm">
              <q-input v-model="editForm.shift_start" label="Start" outlined dense class="col" hint="HH:MM" />
              <q-input v-model="editForm.shift_end"   label="End"   outlined dense class="col" hint="HH:MM" />
              <q-input v-model.number="editForm.shift_hours" label="Hours" type="number" outlined dense class="col" />
            </div>
          </template>
          <div v-else class="text-caption text-grey-7 q-px-xs">
            {{ editForm.shift_start }} – {{ editForm.shift_end }} · {{ editForm.shift_hours }}h
          </div>
          <q-input v-model="editForm.notes" label="Notes (optional)" outlined dense />
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Save Changes" unelevated :loading="editSubmitting" @click="submitEdit" />
        </q-card-actions>
      </q-card>
    </q-dialog>

  </q-page>

  <!-- Drag ghost — floats under cursor during pointer drag -->
  <Teleport to="body">
    <div id="drag-ghost" class="drag-ghost"
         :style="ghostStyle"
         :class="`palette-chip--${ghostColor}`">
      <q-icon name="o_drag_indicator" size="xs" class="q-mr-xs opacity-60" />
      {{ ghostLabel }}
    </div>
  </Teleport>
</template>

<style scoped>
/* ── Week view ──────────────────────────────────────────────────────────────── */
.schedule-wrap { overflow-x: auto; }
.schedule-table { width: 100%; border-collapse: collapse; min-width: 820px; }
.schedule-table th,
.schedule-table td { border: 1px solid #e5e7eb; padding: 6px 8px; vertical-align: top; }
.schedule-table thead th { background: #f8fafc; font-size: 0.78rem; font-weight: 600; text-align: center; }
.staff-col  { width: 160px; min-width: 140px; }
.day-col    { min-width: 110px; text-align: center; }
.hours-col  { width: 64px; text-align: center; }
.today-col  { background: #eff6ff; }
.today-bg   { background: #f0f9ff; }
.day-label  { font-size: 0.78rem; font-weight: 600; }
.day-date   { font-size: 0.72rem; }
.staff-name-cell { font-size: 0.85rem; color: #374151; white-space: nowrap; }
.day-cell   { min-height: 52px; position: relative; }
.hours-cell { text-align: center; font-size: 0.82rem; color: #374151; }

/* Add button — appears on cell hover */
.add-btn { opacity: 0; transition: opacity 0.15s; }
.day-cell:hover .add-btn { opacity: 1; }

/* ── Month view ─────────────────────────────────────────────────────────────── */
.month-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  border: 1px solid #e5e7eb;
  border-radius: 10px;
  overflow: hidden;
}
.month-dow-header {
  background: #f8fafc;
  text-align: center;
  font-size: 0.75rem;
  font-weight: 600;
  color: #6b7280;
  padding: 8px 0;
  border-bottom: 1px solid #e5e7eb;
}
.month-day-cell {
  min-height: 110px;
  border-right: 1px solid #e5e7eb;
  border-bottom: 1px solid #e5e7eb;
  padding: 4px 5px;
  background: #fff;
  transition: min-height 0.2s ease;
}
.month-day-cell:nth-child(7n) { border-right: none; }
.month-day--other { background: #fafafa; }
.month-day--other .month-day-num { color: #d1d5db; }
.month-day--today { background: #f0f9ff; }
.month-day--drop  { background: #eff6ff !important; outline: 2px dashed #3b82f6; outline-offset: -2px; }
.month-day-num {
  font-size: 0.78rem; font-weight: 600; color: #374151;
  margin-bottom: 3px; display: inline-block; min-width: 22px; text-align: center;
}
.today-badge { background: #3b82f6; color: #fff !important; border-radius: 50%; padding: 1px 5px; }
.month-shift-bar {
  display: flex; align-items: center; gap: 3px;
  border-radius: 4px; padding: 2px 5px; margin-bottom: 2px;
  font-size: 0.68rem; font-weight: 500; overflow: hidden;
}
.month-shift-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.month-shift-time { opacity: 0.75; white-space: nowrap; flex-shrink: 0; }
.month-more {
  font-size: 0.68rem;
  color: #6b7280;
  padding: 2px 4px;
  cursor: pointer;
  border-radius: 4px;
  user-select: none;
}
.month-more:hover { background: #f3f4f6; color: #374151; }
.month-delete { opacity: 0; margin-left: auto; flex-shrink: 0; }
.month-shift-bar:hover .month-delete { opacity: 0.7; }
.month-shift-bar:hover .month-delete:hover { opacity: 1; }

/* ── Shared shift chip ──────────────────────────────────────────────────────── */
.shift-chip {
  display: flex; align-items: center; justify-content: space-between;
  border-radius: 6px; padding: 3px 6px; margin-bottom: 3px;
  font-size: 0.72rem; font-weight: 500; white-space: nowrap;
}
.shift-chip--teal        { background: #ccfbf1; color: #0f766e; }
.shift-chip--blue        { background: #dbeafe; color: #1d4ed8; }
.shift-chip--deep-orange { background: #ffedd5; color: #c2410c; }
.shift-chip--purple      { background: #ede9fe; color: #6d28d9; }
.shift-actions { display: flex; gap: 2px; }
.shift-edit,
.shift-delete { opacity: 0; transition: opacity 0.15s; }
.shift-chip:hover .shift-edit,
.shift-chip:hover .shift-delete { opacity: 0.6; }
.shift-chip:hover .shift-edit:hover,
.shift-chip:hover .shift-delete:hover { opacity: 1; }
.shift-chip--clickable { cursor: grab; user-select: none; touch-action: none; }
.shift-chip--clickable:active { cursor: grabbing; }
.shift-chip--clickable:hover { filter: brightness(0.95); }

/* ── Drag palette ───────────────────────────────────────────────────────────── */
.drag-palette { background: #f8fafc; border: 1px solid #e5e7eb; border-radius: 10px; padding: 12px 16px; }
.palette-chip { display: inline-flex; align-items: center; padding: 6px 14px; border-radius: 20px; font-size: 0.78rem; font-weight: 500; cursor: grab; user-select: none; transition: transform 0.12s, box-shadow 0.12s; touch-action: none; }
.palette-chip:hover  { transform: translateY(-2px); box-shadow: 0 3px 8px rgba(0,0,0,0.12); }
.palette-chip:active { cursor: grabbing; transform: scale(0.97); }

/* ── Drag ghost ─────────────────────────────────────────────────────────────── */
.drag-ghost { position: fixed; pointer-events: none; z-index: 9999; display: none; align-items: center; padding: 5px 12px; border-radius: 20px; font-size: 0.78rem; font-weight: 500; opacity: 0.92; box-shadow: 0 4px 12px rgba(0,0,0,0.2); }
.palette-chip--custom { background: #f8fafc; color: #475569; border: 1.5px dashed #94a3b8; }
.palette-chip--custom:hover { background: #f1f5f9; box-shadow: 0 3px 8px rgba(0,0,0,0.08); }
.palette-chip--teal        { background: #ccfbf1; color: #0f766e; border: 1px solid #99f6e4; }
.palette-chip--blue        { background: #dbeafe; color: #1d4ed8; border: 1px solid #bfdbfe; }
.palette-chip--deep-orange { background: #ffedd5; color: #c2410c; border: 1px solid #fed7aa; }
.palette-chip--purple      { background: #ede9fe; color: #6d28d9; border: 1px solid #ddd6fe; }

/* ── Drop highlight ─────────────────────────────────────────────────────────── */
.drop-over { background: #eff6ff !important; outline: 2px dashed #3b82f6; outline-offset: -2px; }

/* ── Drop animation ─────────────────────────────────────────────────────────── */
@keyframes chipDrop {
  0%   { transform: scale(1.18) translateY(-4px); box-shadow: 0 6px 18px rgba(0,0,0,0.18); }
  60%  { transform: scale(0.96) translateY(1px);  box-shadow: 0 1px 4px rgba(0,0,0,0.08); }
  100% { transform: scale(1)    translateY(0);    box-shadow: none; }
}
.shift-chip--dropped {
  animation: chipDrop 0.42s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}
</style>
