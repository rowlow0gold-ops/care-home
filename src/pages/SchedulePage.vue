<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { useQuasar } from "quasar";
import { server } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const session = useServerSessionStore();

// ── Role helpers (server permission ladder) ─────────────────────────────────
// 접수(create only) vs 행정+(full CRUD)
const canCreate = computed(() => session.canCreate);
const canEdit = computed(() => session.canEdit);
const canDelete = computed(() => session.canDelete);

// ── View mode ─────────────────────────────────────────────────────────────────
const viewMode = ref<"week" | "month">("week");

// ── Types ─────────────────────────────────────────────────────────────────────
interface ScheduleEntry {
  id:          string;
  staff_id:    string;
  staff_name:  string;
  shift_date:  string;
  shift_start: string;
  shift_end:   string;
  shift_hours: number;
  notes:       string | null;
}
interface StaffOption { label: string; value: string; role: string }

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
const DAY_LABELS = ["월", "화", "수", "목", "금", "토", "일"];
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
  const fmt = (d: Date) => d.toLocaleDateString("ko-KR", { month: "short", day: "numeric" });
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
  return currentMonth.value.toLocaleDateString("ko-KR", { month: "long", year: "numeric" });
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

  const gridStart = weekMonday(firstDay);
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
  if (staffList.value.length > 0) return staffList.value;
  // Fallback: derive roster rows from the entries themselves.
  const seen = new Map<string, string>();
  for (const e of entries.value) seen.set(e.staff_id, e.staff_name);
  return Array.from(seen.entries())
    .sort((a, b) => a[1].localeCompare(b[1]))
    .map(([value, label]) => ({ label, value, role: "caregiver" }));
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
function cellEntries(staffId: string, date: Date): ScheduleEntry[] {
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
function weekHours(staffId: string): number {
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
    const rows = await server.roster(start, end);
    entries.value = rows.map(r => ({
      id:          r.id,
      staff_id:    r.user_id,
      staff_name:  r.staff_name,
      shift_date:  r.shift_date,
      shift_start: r.shift_start,
      shift_end:   r.shift_end,
      shift_hours: r.shift_hours,
      notes:       r.notes,
    }));
  } catch (e: any) {
    $q.notify({ type: "negative", message: `근무일정을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}
async function loadStaffList() {
  try {
    const users = await server.staff();
    // Branch-scoped: only this 센터's rosterable workers.
    const myBranch = session.me?.branch_id;
    staffList.value = users
      .filter(u => !u.deactivated_at
        && ["caregiver", "nurse", "branch_manager"].includes(u.role)
        && (!myBranch || u.branch_id === myBranch))
      .map(u => ({ label: u.full_name, value: u.id, role: u.role }))
      .sort((a, b) => a.label.localeCompare(b.label));
  } catch (_) { /* best effort */ }
}

watch(viewMode, loadSchedule);
watch([weekStartStr, weekEndStr], () => { if (viewMode.value === "week")   loadSchedule(); });
watch([monthStartStr, monthEndStr], () => { if (viewMode.value === "month") loadSchedule(); });
onMounted(async () => { await loadStaffList(); await loadSchedule(); });

// ── Shift presets ─────────────────────────────────────────────────────────────
const SHIFT_PRESETS = [
  { label: "주간 12h (07:00–19:00)",   start: "07:00", end: "19:00", hours: 12 },
  { label: "야간 12h (19:00–07:00)",   start: "19:00", end: "07:00", hours: 12 },
  { label: "오전 8h (07:00–15:00)",    start: "07:00", end: "15:00", hours:  8 },
  { label: "오후 8h (15:00–23:00)",    start: "15:00", end: "23:00", hours:  8 },
  { label: "야간 8h (23:00–07:00)",    start: "23:00", end: "07:00", hours:  8 },
  { label: "직접입력",                 start: "",      end: "",      hours:  0 },
];

// ── Add shift (cell hover button) ─────────────────────────────────────────────
const showAdd    = ref(false);
const submitting = ref(false);
const form = ref({
  staff_id:    null as string | null,
  shift_date:  "",
  preset:      SHIFT_PRESETS[0],
  shift_start: "07:00",
  shift_end:   "19:00",
  shift_hours: 12,
  notes:       "",
});
function applyPreset() {
  if (form.value.preset.label !== "직접입력") {
    form.value.shift_start = form.value.preset.start;
    form.value.shift_end   = form.value.preset.end;
    form.value.shift_hours = form.value.preset.hours;
  }
}
function openAddForCell(staffId: string, date: Date) {
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
// 직접입력 — open the add dialog with a blank custom shift (pick staff/date/time).
function openCustomAdd() {
  form.value = {
    staff_id:    null,
    shift_date:  "",
    preset:      SHIFT_PRESETS.find((p) => p.label === "직접입력")!,
    shift_start: "",
    shift_end:   "",
    shift_hours: 0,
    notes:       "",
  };
  showAdd.value = true;
}
async function submitAdd() {
  if (!form.value.staff_id || !form.value.shift_date) {
    $q.notify({ type: "negative", message: "직원과 날짜를 선택하세요." });
    return;
  }
  submitting.value = true;
  try {
    await server.createRoster({
      user_id:     form.value.staff_id,
      shift_date:  form.value.shift_date,
      shift_start: form.value.shift_start,
      shift_end:   form.value.shift_end,
      shift_hours: form.value.shift_hours,
      notes:       form.value.notes || null,
    });
    $q.notify({ type: "positive", message: "근무가 추가되었습니다." });
    showAdd.value = false;
    await loadSchedule();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` });
  } finally {
    submitting.value = false;
  }
}

// ── Drag & drop (pointer-events based — HTML5 DnD is broken in WKWebView) ────
type Preset = typeof SHIFT_PRESETS[0];
type DragState =
  | { kind: "entry";  id: string }
  | { kind: "preset"; preset: Preset }
  | null;

let _drag: DragState = null;
const isDragging    = ref(false);
const dropTargetKey = ref("");
const droppedId     = ref<string | null>(null);
const ghostStyle    = ref({ left: "0px", top: "0px", display: "none" });
const ghostLabel    = ref("");
const ghostColor    = ref("blue");

function presetColor(p: Preset): string {
  if (p.hours >= 12)       return "teal";
  if (p.start === "07:00") return "blue";
  if (p.start === "15:00") return "deep-orange";
  return "purple";
}

function dropKey(staffId: string | null, date: Date): string {
  return staffId === null ? `day-${localDateStr(date)}` : `${staffId}-${localDateStr(date)}`;
}

function getCellUnder(x: number, y: number): HTMLElement | null {
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

  const staffId = cell.dataset.staffId ? cell.dataset.staffId : null;
  const date    = new Date(cell.dataset.date + "T00:00:00");

  // ── Move existing entry ────────────────────────────────────────────────────
  if (payload.kind === "entry") {
    const entry = entries.value.find(en => en.id === payload.id);
    if (!entry) return;
    const newDate    = localDateStr(date);
    const newStaffId = staffId ?? entry.staff_id;
    if (entry.staff_id === newStaffId && entry.shift_date === newDate) return;
    try {
      await server.updateRoster(entry.id, {
        user_id:     newStaffId,
        shift_date:  newDate,
        shift_start: entry.shift_start,
        shift_end:   entry.shift_end,
        shift_hours: entry.shift_hours,
        notes:       entry.notes ?? null,
      });
      await loadSchedule();
      await nextTick();
      droppedId.value = entry.id;
      setTimeout(() => { droppedId.value = null; }, 450);
    } catch (err: any) {
      $q.notify({ type: "negative", message: `근무 이동 실패: ${err?.message ?? err}` });
    }
    return;
  }

  // ── Drop preset ────────────────────────────────────────────────────────────
  if (payload.kind === "preset" && staffId !== null) {
    const preset = payload.preset;

    if (preset.label === "직접입력") {
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
      await server.createRoster({
        user_id:     staffId,
        shift_date:  localDateStr(date),
        shift_start: preset.start,
        shift_end:   preset.end,
        shift_hours: preset.hours,
        notes:       null,
      });
      $q.notify({ type: "positive", message: `${preset.label} 추가됨.` });
      await loadSchedule();
    } catch (err: any) {
      $q.notify({ type: "negative", message: `실패: ${err?.message ?? err}` });
    }
  }
}

function startDrag(e: PointerEvent, state: DragState, label: string, color: string) {
  if (!state) return;
  // moving an existing entry = edit; dragging a preset = create.
  const allowed = state.kind === "entry" ? canEdit.value : canCreate.value;
  if (!allowed) return;
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
const editingId      = ref<string | null>(null);
const editingStaffId = ref<string | null>(null);
const editSubmitting = ref(false);
const editForm = ref({
  shift_date: "", preset: SHIFT_PRESETS[0], shift_start: "07:00", shift_end: "19:00", shift_hours: 12, notes: "",
});
function applyEditPreset() {
  if (editForm.value.preset.label !== "직접입력") {
    editForm.value.shift_start = editForm.value.preset.start;
    editForm.value.shift_end   = editForm.value.preset.end;
    editForm.value.shift_hours = editForm.value.preset.hours;
  }
}
function openEditDialog(entry: ScheduleEntry) {
  if (!canEdit.value) return;
  editingId.value      = entry.id;
  editingStaffId.value = entry.staff_id;
  const matched = SHIFT_PRESETS.find(
    p => p.start === entry.shift_start && p.end === entry.shift_end && p.hours === entry.shift_hours
  ) ?? SHIFT_PRESETS.find(p => p.label === "직접입력")!;
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
    await server.updateRoster(editingId.value, {
      user_id:     editingStaffId.value,
      shift_date:  editForm.value.shift_date,
      shift_start: editForm.value.shift_start,
      shift_end:   editForm.value.shift_end,
      shift_hours: editForm.value.shift_hours,
      notes:       editForm.value.notes || null,
    });
    $q.notify({ type: "positive", message: "근무가 수정되었습니다." });
    showEdit.value = false;
    await loadSchedule();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` });
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
  expandedDays.value = new Set(expandedDays.value);
}
function isExpanded(date: Date) { return expandedDays.value.has(localDateStr(date)); }

// ── Delete shift ──────────────────────────────────────────────────────────────
async function deleteShift(entry: ScheduleEntry) {
  $q.dialog({
    title: "근무 삭제",
    message: `${entry.staff_name}님의 ${entry.shift_date} ${entry.shift_start}–${entry.shift_end} 근무를 삭제할까요?`,
    cancel: { label: "취소", flat: true },
    ok:     { label: "삭제", color: "negative", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    try {
      await server.deleteRoster(entry.id);
      $q.notify({ type: "positive", message: "근무가 삭제되었습니다." });
      await loadSchedule();
    } catch (e: any) {
      $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` });
    }
  });
}
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">근무일정</div>
        <div class="text-caption text-grey-6">
          {{ canCreate ? "직원 근무 일정 관리" : "직원 근무 일정 (읽기 전용)" }}
        </div>
      </div>

      <div class="col-auto">
        <q-btn-toggle
          v-model="viewMode"
          toggle-color="primary"
          :options="[{ label: '주간', value: 'week' }, { label: '월간', value: 'month' }]"
          unelevated rounded dense size="sm"
        />
      </div>
    </div>

    <!-- Drag palette (managers, week view only) -->
    <div v-if="canCreate && viewMode === 'week'" class="drag-palette q-mb-md">
      <div class="text-caption text-grey-6 q-mb-sm">
        <q-icon name="o_drag_indicator" size="xs" class="q-mr-xs" />근무 유형을 셀로 드래그하여 배정하세요
      </div>
      <div class="row q-gutter-sm items-center">
        <div
          v-for="p in SHIFT_PRESETS.filter(p => p.label !== '직접입력')"
          :key="p.label"
          class="palette-chip"
          :class="`palette-chip--${presetColor(p)}`"
          @pointerdown="startDrag($event, { kind: 'preset', preset: p }, p.label, presetColor(p))"
        >
          <q-icon name="o_drag_indicator" size="xs" class="q-mr-xs opacity-60" />
          {{ p.label }}
        </div>
        <q-btn
          outline
          no-caps
          dense
          color="grey-8"
          icon="o_edit"
          label="직접입력"
          class="palette-custom-btn"
          @click="openCustomAdd"
        />
      </div>
    </div>

    <!-- Navigator -->
    <div class="row items-center q-mb-md q-gutter-sm">
      <q-btn flat round dense icon="o_chevron_left"  @click="viewMode === 'week' ? prevWeek() : prevMonth()" />
      <q-btn flat round dense icon="o_chevron_right" @click="viewMode === 'week' ? nextWeek() : nextMonth()" />
      <span class="text-subtitle1 text-weight-medium q-mx-sm">
        {{ viewMode === 'week' ? weekLabel() : monthLabel() }}
      </span>
      <q-btn flat dense size="sm" label="오늘" @click="goToday" class="text-grey-7" />
      <q-spinner-dots v-if="loading" color="primary" size="1.2rem" class="q-ml-sm" />
    </div>

    <!-- ══ WEEK VIEW ══════════════════════════════════════════════════════════ -->
    <template v-if="viewMode === 'week'">
      <div class="schedule-wrap">
        <table class="schedule-table">
          <thead>
            <tr>
              <th class="staff-col">직원</th>
              <th
                v-for="(date, di) in weekDates" :key="di"
                :class="['day-col', { 'today-col': isToday(date) }]"
              >
                <div class="day-label">{{ DAY_LABELS[di] }}</div>
                <div class="day-date" :class="isToday(date) ? 'text-primary text-weight-bold' : 'text-grey-6'">
                  {{ date.getMonth() + 1 }}/{{ date.getDate() }}
                </div>
              </th>
              <th class="hours-col">주간 시간</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="staff in staffRows" :key="staff.value">
              <td class="staff-name-cell">
                <q-icon name="o_person" size="xs" color="grey-6" class="q-mr-xs" />
                <span>{{ staff.label }}</span>
                <q-badge v-if="staff.value === session.me?.id"
                         color="primary" label="나"
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
                  :class="[`shift-chip--${shiftColor(entry)}`, { 'shift-chip--clickable': canEdit, 'shift-chip--dropped': droppedId === entry.id }]"
                  @pointerdown.stop="canEdit && startDrag($event, { kind: 'entry', id: entry.id }, shiftLabel(entry), shiftColor(entry))"
                  @click.stop="openEditDialog(entry)"
                >
                  <span class="shift-time">{{ shiftLabel(entry) }}</span>
                  <div v-if="canEdit || canDelete" class="shift-actions">
                    <q-btn v-if="canEdit" flat round dense icon="o_edit"  size="xs" class="shift-edit"   @click.stop="openEditDialog(entry)" />
                    <q-btn v-if="canDelete" flat round dense icon="o_close" size="xs" class="shift-delete" @click.stop="deleteShift(entry)" />
                  </div>
                  <q-tooltip v-if="entry.notes">{{ entry.notes }}</q-tooltip>
                </div>
                <q-btn
                  v-if="canCreate"
                  flat round dense icon="o_add" size="xs" color="grey-5"
                  class="add-btn"
                  @click="openAddForCell(staff.value, date)"
                >
                  <q-tooltip>근무 추가</q-tooltip>
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
                <q-icon name="o_event_busy" size="3rem" color="grey-4" /><br />이번 주 근무 데이터가 없습니다.
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
                :class="[`shift-chip--${shiftColor(entry)}`, { 'shift-chip--clickable': canEdit, 'shift-chip--dropped': droppedId === entry.id }]"
                @pointerdown.stop="canEdit && startDrag($event, { kind: 'entry', id: entry.id }, shiftLabel(entry), shiftColor(entry))"
                @click.stop="openEditDialog(entry)"
              >
                <span class="month-shift-name">{{ entry.staff_name.split(' ')[0] }}</span>
                <span class="month-shift-time">{{ entry.shift_start }}</span>
                <q-btn
                  v-if="canDelete"
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
              {{ isExpanded(date) ? '▲ 접기' : `+${dayEntries(date).length - 3} 더보기` }}
            </div>
          </div>
        </template>
      </div>
    </template>

    <!-- ── Add Shift Dialog ────────────────────────────────────────────────── -->
    <q-dialog v-model="showAdd" persistent>
      <q-card style="min-width: 420px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">근무 추가</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-sm">
          <q-select v-model="form.staff_id" :options="staffList" label="직원 *" outlined dense emit-value map-options />
          <div class="cursor-pointer">
            <q-input v-model="form.shift_date" label="날짜 *" outlined dense readonly style="pointer-events:none">
              <template #append>
                <q-icon name="o_event" color="grey-6" />
              </template>
            </q-input>
            <q-popup-proxy transition-show="scale" transition-hide="scale">
              <q-date v-model="form.shift_date" mask="YYYY-MM-DD" minimal>
                <div class="row items-center justify-end q-pa-sm">
                  <q-btn v-close-popup label="확인" color="primary" flat dense />
                </div>
              </q-date>
            </q-popup-proxy>
          </div>
          <q-select v-model="form.preset" :options="SHIFT_PRESETS" label="근무 유형" outlined dense option-label="label" @update:model-value="applyPreset" />
          <template v-if="form.preset.label === '직접입력'">
            <div class="row q-gutter-sm">
              <q-input v-model="form.shift_start" label="시작" outlined dense class="col" hint="HH:MM" />
              <q-input v-model="form.shift_end"   label="종료" outlined dense class="col" hint="HH:MM" />
              <q-input v-model.number="form.shift_hours" label="시간" type="number" outlined dense class="col" />
            </div>
          </template>
          <div v-else class="text-caption text-grey-7 q-px-xs">
            {{ form.shift_start }} – {{ form.shift_end }} · {{ form.shift_hours }}h
          </div>
          <q-input v-model="form.notes" label="메모 (선택)" outlined dense />
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="근무 추가" unelevated :loading="submitting" @click="submitAdd" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- ── Edit Shift Dialog ───────────────────────────────────────────────── -->
    <q-dialog v-model="showEdit" persistent>
      <q-card style="min-width: 420px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">근무 수정</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-sm">
          <div class="cursor-pointer">
            <q-input v-model="editForm.shift_date" label="날짜 *" outlined dense readonly style="pointer-events:none">
              <template #append>
                <q-icon name="o_event" color="grey-6" />
              </template>
            </q-input>
            <q-popup-proxy transition-show="scale" transition-hide="scale">
              <q-date v-model="editForm.shift_date" mask="YYYY-MM-DD" minimal>
                <div class="row items-center justify-end q-pa-sm">
                  <q-btn v-close-popup label="확인" color="primary" flat dense />
                </div>
              </q-date>
            </q-popup-proxy>
          </div>
          <q-select v-model="editForm.preset" :options="SHIFT_PRESETS" label="근무 유형" outlined dense option-label="label" @update:model-value="applyEditPreset" />
          <template v-if="editForm.preset.label === '직접입력'">
            <div class="row q-gutter-sm">
              <q-input v-model="editForm.shift_start" label="시작" outlined dense class="col" hint="HH:MM" />
              <q-input v-model="editForm.shift_end"   label="종료" outlined dense class="col" hint="HH:MM" />
              <q-input v-model.number="editForm.shift_hours" label="시간" type="number" outlined dense class="col" />
            </div>
          </template>
          <div v-else class="text-caption text-grey-7 q-px-xs">
            {{ editForm.shift_start }} – {{ editForm.shift_end }} · {{ editForm.shift_hours }}h
          </div>
          <q-input v-model="editForm.notes" label="메모 (선택)" outlined dense />
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="저장" unelevated :loading="editSubmitting" @click="submitEdit" />
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
.hours-col  { width: 80px; text-align: center; }
.today-col  { background: #eff6ff; }
.today-bg   { background: #f0f9ff; }
.day-label  { font-size: 0.78rem; font-weight: 600; }
.day-date   { font-size: 0.72rem; }
.staff-name-cell { font-size: 0.85rem; color: #374151; white-space: nowrap; }
.day-cell   { min-height: 52px; position: relative; }
.hours-cell { text-align: center; font-size: 0.82rem; color: #374151; }

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
