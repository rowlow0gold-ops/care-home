<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { Store } from "@tauri-apps/plugin-store";
import { useQuasar } from "quasar";
import { server, type Team, type OrgPerson, type Resident } from "@/lib/server";
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
// Korean convention: weeks start on SUNDAY. Returns the Sunday on/before d.
function weekMonday(d: Date): Date {
  const c = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  c.setDate(c.getDate() - c.getDay()); // getDay 0=Sun
  return c;
}
function addDays(d: Date, n: number): Date {
  const c = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  c.setDate(c.getDate() + n);
  return c;
}
const DAY_LABELS = ["일", "월", "화", "수", "목", "금", "토"];
const todayStr   = computed(() => localDateStr(new Date()));
function isToday(d: Date) { return localDateStr(d) === todayStr.value; }

// ── 빨간날 (holidays + weekends) ────────────────────────────────────────────
const holidayMap = ref<Map<string, string>>(new Map());
const loadedYears = new Set<number>();
async function loadHolidays(year: number) {
  if (loadedYears.has(year)) return;
  loadedYears.add(year);
  try {
    const hs = await server.holidays(year);
    for (const h of hs) holidayMap.value.set(h.locdate, h.name);
    holidayMap.value = new Map(holidayMap.value); // trigger reactivity
  } catch { /* holidays optional */ }
}
function holidayName(d: Date): string | null { return holidayMap.value.get(localDateStr(d)) ?? null; }
function isRedDay(d: Date): boolean {
  const g = d.getDay();
  return g === 0 || g === 6 || holidayMap.value.has(localDateStr(d));
}

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

  const gridStart = weekMonday(firstDay);            // Sunday on/before the 1st
  const gridEnd   = addDays(lastDay, 6 - lastDay.getDay()); // Saturday on/after last

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

// ── Data + Draft overlay ──────────────────────────────────────────────────────
// `serverRows` = last fetch. The draft is a PERSISTENT overlay (creates/updates/
// deletes) that survives team/week switches; `entries` is rebuilt = server+overlay.
const serverRows = ref<ScheduleEntry[]>([]);
const entries    = ref<ScheduleEntry[]>([]);
const loading    = ref(false);
const staffList  = ref<StaffOption[]>([]);
const saving     = ref(false);

const pendingCreates = ref<ScheduleEntry[]>([]);
const pendingUpdates = ref<Map<string, ScheduleEntry>>(new Map());
const pendingDeletes = ref<Set<string>>(new Set());
const dirty = computed(() =>
  pendingCreates.value.length > 0 || pendingUpdates.value.size > 0 || pendingDeletes.value.size > 0,
);
let tempSeq = 0;
function tempId() { return `new-${++tempSeq}`; }
function isNew(id: string) { return id.startsWith("new-"); }

// Rebuild the visible `entries` from the last server fetch + the overlay.
function rebuild() {
  const start = viewMode.value === "week" ? weekStartStr.value : monthStartStr.value;
  const end   = viewMode.value === "week" ? weekEndStr.value   : monthEndStr.value;
  const base = serverRows.value
    .filter((e) => !pendingDeletes.value.has(e.id))
    .map((e) => pendingUpdates.value.get(e.id) ?? e);
  const creates = pendingCreates.value.filter((c) => c.shift_date >= start && c.shift_date <= end);
  entries.value = [...base, ...creates];
}

async function saveDraft() {
  if (saving.value || !dirty.value) return;
  saving.value = true;
  try {
    for (const c of pendingCreates.value) {
      await server.createRoster({
        user_id: c.staff_id, shift_date: c.shift_date,
        shift_start: c.shift_start, shift_end: c.shift_end,
        shift_hours: c.shift_hours, notes: c.notes ?? null,
      });
    }
    for (const [id, e] of pendingUpdates.value) {
      if (!isNew(id)) {
        await server.updateRoster(id, {
          user_id: e.staff_id, shift_date: e.shift_date,
          shift_start: e.shift_start, shift_end: e.shift_end,
          shift_hours: e.shift_hours, notes: e.notes ?? null,
        });
      }
    }
    for (const id of pendingDeletes.value) {
      if (!isNew(id)) await server.deleteRoster(id);
    }
    clearOverlay();
    $q.notify({ type: "positive", message: "근무일정을 저장했습니다." });
    await loadSchedule();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` });
  } finally {
    saving.value = false;
  }
}
function clearOverlay() {
  pendingCreates.value = [];
  pendingUpdates.value = new Map();
  pendingDeletes.value = new Set();
}
function rollbackDraft() {
  clearOverlay();
  rebuild();
  $q.notify({ type: "info", message: "변경사항을 되돌렸습니다." });
}
function clearDraft() {
  $q.dialog({
    title: "클리어", message: "현재 보이는 기간의 근무를 모두 비웁니다. (저장 시 반영)",
    cancel: { label: "취소", flat: true }, ok: { label: "클리어", color: "negative", unelevated: true }, persistent: true,
  }).onOk(() => {
    for (const e of entries.value) {
      if (isNew(e.id)) pendingCreates.value = pendingCreates.value.filter((c) => c.id !== e.id);
      else pendingDeletes.value.add(e.id);
    }
    pendingDeletes.value = new Set(pendingDeletes.value);
    rebuild();
  });
}

// ── Teams (조) — "split by shift" filter ───────────────────────────────────────
const teams = ref<Team[]>([]);
const selectedTeam = ref<string>("");   // 항상 특정 조 (전체 조 없음)
const teamOptions = computed(() =>
  teams.value.map((t) => ({ label: `${t.name} (${t.member_count})`, value: t.id })),
);
async function loadTeams() {
  try {
    teams.value = await server.teams();
    if (!selectedTeam.value && teams.value.length) selectedTeam.value = teams.value[0].id;
  } catch { teams.value = []; }
}

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
    loadHolidays(Number(start.slice(0, 4)));
    loadHolidays(Number(end.slice(0, 4)));
    const rows = await server.roster(start, end, selectedTeam.value || undefined);
    serverRows.value = rows.map(r => ({
      id:          r.id,
      staff_id:    r.user_id,
      staff_name:  r.staff_name,
      shift_date:  r.shift_date,
      shift_start: r.shift_start,
      shift_end:   r.shift_end,
      shift_hours: r.shift_hours,
      notes:       r.notes,
    }));
    rebuild(); // re-apply the persistent draft overlay on top of the fetch
  } catch (e: any) {
    $q.notify({ type: "negative", message: `근무일정을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}
// 선택한 조에 배정된 돌봄 인력을 그리드 행으로 — 셀에 근무를 드래그&드롭해 배치한다.
async function loadTeamStaff(teamId: string) {
  try {
    const org = await server.orgPaged({ page: 1, page_size: 500 });
    const myBranch = session.me?.branch_id;
    staffList.value = org.items
      .filter((u) => !u.is_inactive
        && ["caregiver", "nurse"].includes(u.role)
        && u.team_id === teamId
        && (!myBranch || u.branch_id === myBranch))
      .map((u) => ({ label: u.full_name, value: u.id, role: u.role }))
      .sort((a, b) => a.label.localeCompare(b.label));
  } catch { staffList.value = []; }
}

watch(viewMode, loadSchedule);
watch([weekStartStr, weekEndStr], () => { if (viewMode.value === "week")   loadSchedule(); });
watch([monthStartStr, monthEndStr], () => { if (viewMode.value === "month") loadSchedule(); });
watch(selectedTeam, async () => {
  if (selectedTeam.value) await loadTeamStaff(selectedTeam.value);
  await loadSchedule();
});
onMounted(async () => {
  await loadPresets();
  await loadTeams();
  if (selectedTeam.value) await loadTeamStaff(selectedTeam.value);
  await loadResidentCounts();
  await loadSchedule();
});

// ── Shift presets (근무 유형) ─────────────────────────────────────────────────
interface Preset { label: string; start: string; end: string; hours: number }
const DEFAULT_PRESETS: Preset[] = [
  { label: "주간 12h (07:00–19:00)",   start: "07:00", end: "19:00", hours: 12 },
  { label: "야간 12h (19:00–07:00)",   start: "19:00", end: "07:00", hours: 12 },
  { label: "오전 8h (07:00–15:00)",    start: "07:00", end: "15:00", hours:  8 },
  { label: "오후 8h (15:00–23:00)",    start: "15:00", end: "23:00", hours:  8 },
  { label: "야간 8h (23:00–07:00)",    start: "23:00", end: "07:00", hours:  8 },
];
const MANUAL: Preset = { label: "직접", start: "", end: "", hours: 0 };

// Custom 근무 유형 — saved locally (per device), deletable. Defaults are locked.
const customPresets = ref<Preset[]>([]);
const allPresets = computed(() => [...DEFAULT_PRESETS, ...customPresets.value]);
const dialogPresets = computed(() => [...allPresets.value, MANUAL]);

let presetStore: Store | null = null;
async function presetsStore() {
  if (!presetStore) presetStore = await Store.load("schedule.dat");
  return presetStore;
}
async function loadPresets() {
  try {
    const s = await presetsStore();
    customPresets.value = (await s.get<Preset[]>("custom_presets")) ?? [];
  } catch { customPresets.value = []; }
}
async function savePresets() {
  const s = await presetsStore();
  await s.set("custom_presets", customPresets.value);
  await s.save();
}
function isCustom(p: Preset) { return customPresets.value.some((c) => c.label === p.label); }

// ── Add shift ─────────────────────────────────────────────────────────────────
const showAdd    = ref(false);
const submitting = ref(false);
const form = ref({
  staff_id:    null as string | null,
  shift_date:  "",
  preset:      DEFAULT_PRESETS[0],
  shift_start: "07:00",
  shift_end:   "19:00",
  shift_hours: 12,
  notes:       "",
});
function applyPreset() {
  if (form.value.preset.label !== "직접") {
    form.value.shift_start = form.value.preset.start;
    form.value.shift_end   = form.value.preset.end;
    form.value.shift_hours = form.value.preset.hours;
  }
}
function openAddForCell(staffId: string, date: Date) {
  form.value = {
    staff_id:    staffId,
    shift_date:  localDateStr(date),
    preset:      DEFAULT_PRESETS[0],
    shift_start: "07:00",
    shift_end:   "19:00",
    shift_hours: 12,
    notes:       "",
  };
  showAdd.value = true;
}

// ── 직접입력 = define a new 근무 유형 (added to palette, persisted) ────────────
const showNewType = ref(false);
const newType = ref<Preset>({ label: "", start: "07:00", end: "15:00", hours: 8 });
function openNewType() {
  newType.value = { label: "", start: "07:00", end: "15:00", hours: 8 };
  showNewType.value = true;
}
function hoursBetween(start: string, end: string): number {
  const [sh, sm] = start.split(":").map(Number);
  const [eh, em] = end.split(":").map(Number);
  let mins = (eh * 60 + em) - (sh * 60 + sm);
  if (mins <= 0) mins += 24 * 60; // wraps past midnight
  return Math.round((mins / 60) * 10) / 10;
}
async function addNewType() {
  const t = newType.value;
  if (!t.label.trim() || !/^\d{2}:\d{2}$/.test(t.start) || !/^\d{2}:\d{2}$/.test(t.end)) {
    $q.notify({ type: "negative", message: "이름·시작·종료(HH:MM)를 입력하세요." });
    return;
  }
  if (allPresets.value.some((p) => p.label === t.label.trim())) {
    $q.notify({ type: "negative", message: "같은 이름의 근무 유형이 이미 있습니다." });
    return;
  }
  // 시간은 시작/종료로 자동 계산.
  customPresets.value = [...customPresets.value, {
    label: t.label.trim(), start: t.start, end: t.end, hours: hoursBetween(t.start, t.end),
  }];
  await savePresets();
  showNewType.value = false;
  $q.notify({ type: "positive", message: "근무 유형이 추가되었습니다." });
}
async function deleteCustomType(p: Preset) {
  customPresets.value = customPresets.value.filter((x) => x.label !== p.label);
  await savePresets();
}
function staffNameById(id: string): string {
  return staffList.value.find((s) => s.value === id)?.label
    ?? entries.value.find((e) => e.staff_id === id)?.staff_name
    ?? "";
}
function addLocal(staffId: string, date: string, start: string, end: string, hours: number, notes: string | null) {
  pendingCreates.value = [...pendingCreates.value, {
    id: tempId(), staff_id: staffId, staff_name: staffNameById(staffId),
    shift_date: date, shift_start: start, shift_end: end, shift_hours: hours, notes,
  }];
  rebuild();
}
// Record an edit/move of an existing or pending entry into the overlay.
function applyEntryEdit(entry: ScheduleEntry, patch: Partial<ScheduleEntry>) {
  const updated = { ...entry, ...patch };
  if (isNew(entry.id)) {
    pendingCreates.value = pendingCreates.value.map((c) => (c.id === entry.id ? updated : c));
  } else {
    pendingUpdates.value.set(entry.id, updated);
    pendingUpdates.value = new Map(pendingUpdates.value);
  }
  rebuild();
}
function submitAdd() {
  if (!form.value.staff_id || !form.value.shift_date) {
    $q.notify({ type: "negative", message: "직원과 날짜를 선택하세요." });
    return;
  }
  addLocal(form.value.staff_id, form.value.shift_date, form.value.shift_start,
    form.value.shift_end, form.value.shift_hours, form.value.notes || null);
  showAdd.value = false;
}

// ── Drag & drop (pointer-events based — HTML5 DnD is broken in WKWebView) ────
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

  // ── Move existing entry (local) ────────────────────────────────────────────
  if (payload.kind === "entry") {
    const entry = entries.value.find(en => en.id === payload.id);
    if (!entry) return;
    const newDate    = localDateStr(date);
    const newStaffId = staffId ?? entry.staff_id;
    if (entry.staff_id === newStaffId && entry.shift_date === newDate) return;
    applyEntryEdit(entry, { staff_id: newStaffId, staff_name: staffNameById(newStaffId) || entry.staff_name, shift_date: newDate });
    await nextTick();
    droppedId.value = entry.id;
    setTimeout(() => { droppedId.value = null; }, 450);
    return;
  }

  // ── Drop preset (local create) ─────────────────────────────────────────────
  if (payload.kind === "preset" && staffId !== null) {
    const preset = payload.preset;
    addLocal(staffId, localDateStr(date), preset.start, preset.end, preset.hours, null);
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
const editForm = ref<{ shift_date: string; preset: Preset; shift_start: string; shift_end: string; shift_hours: number; notes: string }>({
  shift_date: "", preset: DEFAULT_PRESETS[0], shift_start: "07:00", shift_end: "19:00", shift_hours: 12, notes: "",
});
function applyEditPreset() {
  if (editForm.value.preset.label !== "직접") {
    editForm.value.shift_start = editForm.value.preset.start;
    editForm.value.shift_end   = editForm.value.preset.end;
    editForm.value.shift_hours = editForm.value.preset.hours;
  }
}
function openEditDialog(entry: ScheduleEntry) {
  if (!canEdit.value) return;
  editingId.value      = entry.id;
  editingStaffId.value = entry.staff_id;
  const matched = allPresets.value.find(
    p => p.start === entry.shift_start && p.end === entry.shift_end && p.hours === entry.shift_hours
  ) ?? MANUAL;
  editForm.value = {
    shift_date: entry.shift_date, preset: matched,
    shift_start: entry.shift_start, shift_end: entry.shift_end,
    shift_hours: entry.shift_hours, notes: entry.notes ?? "",
  };
  showEdit.value = true;
}
function submitEdit() {
  if (!editingId.value || !editingStaffId.value) return;
  const entry = entries.value.find((e) => e.id === editingId.value);
  if (entry) {
    applyEntryEdit(entry, {
      shift_date:  editForm.value.shift_date,
      shift_start: editForm.value.shift_start,
      shift_end:   editForm.value.shift_end,
      shift_hours: editForm.value.shift_hours,
      notes:       editForm.value.notes || null,
    });
  }
  showEdit.value = false;
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

// ── Delete shift (local overlay) ────────────────────────────────────────────
function deleteShift(entry: ScheduleEntry) {
  if (isNew(entry.id)) {
    pendingCreates.value = pendingCreates.value.filter((c) => c.id !== entry.id);
  } else {
    pendingUpdates.value.delete(entry.id);
    pendingDeletes.value.add(entry.id);
    pendingDeletes.value = new Set(pendingDeletes.value);
  }
  rebuild();
}

// ── 자동 순환 생성 (커버리지 기반) ──────────────────────────────────────────
// 팀 유형별로 "매일 모든 어르신이 케어"되도록 커버리지를 보장한다.
//   · 요양(24h): 3교대(07–15 / 15–23 / 23–07), 각 교대를 1:10 기준 인원으로
//   · 주간: 조의 근무창 1교대, 1:10 기준 인원으로
//   · 방문: 가변 → 자동 생성 제외(수동)
// 각 조의 "배정된 인력"만 사용한다(인력 임의 분배 금지). 휴무는 회전으로 자연 발생.
// 결과는 draft 오버레이에 들어가며(저장 전 미반영) 기존 칸은 건드리지 않는다.
const generating = ref(false);

function shiftsForTeam(team: Team): { start: string; end: string; hours: number }[] {
  const mk = (start: string, end: string) => ({ start, end, hours: hoursBetween(start, end) });
  if (team.team_type === "residential") {
    return [mk("07:00", "15:00"), mk("15:00", "23:00"), mk("23:00", "07:00")]; // 24h, 8h×3
  }
  if (team.team_type === "day") {
    return [mk(team.shift_start_hm, team.shift_end_hm)]; // 주간 1교대
  }
  return []; // visit: 가변, 수동
}

async function generateRotation() {
  if (!canCreate.value) return;
  if (viewMode.value !== "week") {
    $q.notify({ type: "info", message: "주간 보기에서 자동 생성됩니다." });
    return;
  }
  generating.value = true;
  try {
    if (!teams.value.length) {
      $q.notify({ type: "warning", message: "먼저 ‘조 관리’에서 조를 만드세요." });
      return;
    }
    await loadResidentCounts(); // 1:10 산정을 위한 최신 어르신 수
    const org = await server.orgPaged({ page: 1, page_size: 500 });
    const myBranch = session.me?.branch_id;
    const workers: OrgPerson[] = org.items.filter(
      (u) => !u.is_inactive && ["caregiver", "nurse"].includes(u.role) && (!myBranch || u.branch_id === myBranch),
    );

    const targetTeams = selectedTeam.value ? teams.value.filter((t) => t.id === selectedTeam.value) : teams.value;
    const eldersOf = (tid: string) => activeResidents.value.filter((r) => r.team_id === tid).length;

    // 승인된 휴가는 자동 배정에서 제외 — 이번 주에 걸친 승인 휴가를 모은다.
    const weekSet = new Set(weekDates.value.map(localDateStr));
    const onLeave = new Set<string>(); // `${user_id}-${YYYY-MM-DD}`
    try {
      let p = 1, fetched = 0, leaveTotal = Infinity;
      while (fetched < leaveTotal && p <= 10) {
        const res = await server.leaveRequestsPaged({ status: "approved", page: p, page_size: 100 });
        leaveTotal = res.total;
        for (const lr of res.items) {
          for (const ds of weekSet) {
            if (ds >= lr.start_date && ds <= lr.end_date) onLeave.add(`${lr.user_id}-${ds}`);
          }
        }
        fetched += res.items.length;
        if (!res.items.length) break;
        p++;
      }
    } catch { /* 휴가 조회 실패 시 제외 없이 진행 */ }

    const existing = new Set(entries.value.map((e) => `${e.staff_id}-${e.shift_date}`));
    const fresh: ScheduleEntry[] = [];
    const noWorkers: string[] = [];
    let understaffed = false;

    for (const team of targetTeams) {
      const shifts = shiftsForTeam(team);
      if (!shifts.length) continue;            // 방문: 수동
      const elders = eldersOf(team.id);
      if (elders === 0) continue;              // 담당 어르신 없음 → 인력 불필요
      const required = Math.max(1, Math.ceil(elders / 10)); // 교대당 1:10
      const W = workers.filter((w) => w.team_id === team.id);
      if (!W.length) { noWorkers.push(team.name); continue; }

      let ptr = 0; // 주 전체에 걸쳐 회전 → 요일·주야 교대 + 휴무가 자연 발생
      for (const d of weekDates.value) {
        const ds = localDateStr(d);
        const usedToday = new Set<string>();   // 하루 1교대만
        for (const shift of shifts) {
          let filled = 0, attempts = 0;
          while (filled < required && attempts < W.length) {
            const w = W[ptr % W.length]; ptr++; attempts++;
            if (usedToday.has(w.id)) continue;
            if (onLeave.has(`${w.id}-${ds}`)) { usedToday.add(w.id); continue; } // 승인 휴가 → 제외
            if (existing.has(`${w.id}-${ds}`)) { usedToday.add(w.id); continue; }
            usedToday.add(w.id);
            existing.add(`${w.id}-${ds}`);
            fresh.push({
              id: tempId(), staff_id: w.id, staff_name: w.full_name, shift_date: ds,
              shift_start: shift.start, shift_end: shift.end, shift_hours: shift.hours,
              notes: `${team.name} 자동`,
            });
            filled++;
          }
          if (filled < required) understaffed = true; // 인력 부족 → 인력 알림이 공백/비율로 표시
        }
      }
    }

    const missTxt = noWorkers.length ? ` · 인력 미배정: ${[...new Set(noWorkers)].join(", ")}` : "";
    if (!fresh.length) {
      $q.notify({
        type: "warning",
        message: noWorkers.length
          ? `생성된 근무가 없습니다.${missTxt} (조 관리에서 인력을 배정하세요)`
          : "생성할 근무가 없습니다. (담당 어르신·조 유형을 확인하세요)",
      });
      return;
    }
    pendingCreates.value = [...pendingCreates.value, ...fresh];
    if (selectedTeam.value) await loadTeamStaff(selectedTeam.value); // 생성 인력이 행으로 보이도록
    rebuild();
    const shortTxt = understaffed ? " · 일부 교대 인원 부족" : "";
    $q.notify({ type: "positive", message: `${fresh.length}건 생성 (1:10 커버리지)${missTxt}${shortTxt}. 확인 후 저장하세요.` });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `자동 생성 실패: ${e?.message ?? e}` });
  } finally {
    generating.value = false;
  }
}

// ── 인력 알림 (24h 공백 + 1:10 비율) ─────────────────────────────────────────
const activeResidents = ref<Resident[]>([]);
async function loadResidentCounts() {
  try {
    const r = await server.residentsPaged({ page: 1, page_size: 1000, status: "active" });
    activeResidents.value = r.items;
  } catch { /* alerts are best-effort */ }
}
interface Alert { date: string; kind: "gap" | "ratio"; text: string }
function shiftName(start: string): string {
  return start === "07:00" ? "주간" : start === "15:00" ? "오후" : start === "23:00" ? "야간" : "주간";
}
// 선택한 조 기준, 교대별로 1:10 인원을 충족하는지 점검한다.
//   요양: 3교대(07–15/15–23/23–07) 각각, 주간: 1교대, 방문: 점검 없음.
//   아직 편성하지 않은 날(근무 0건)은 표시하지 않는다.
const staffingAlerts = computed<Alert[]>(() => {
  const team = teams.value.find((t) => t.id === selectedTeam.value);
  if (!team) return [];
  const shifts = shiftsForTeam(team);
  if (!shifts.length) return []; // 방문: 수동
  const elders = activeResidents.value.filter((r) => r.team_id === team.id).length;
  const required = Math.max(1, Math.ceil(elders / 10)); // 1:10
  const out: Alert[] = [];
  const dates =
    viewMode.value === "week"
      ? weekDates.value.map(localDateStr)
      : Array.from(new Set(monthGrid.value.flat().map(localDateStr))).filter(
          (d) => d.slice(0, 7) === monthStartStr.value.slice(0, 7),
        );
  for (const ds of dates) {
    const dayEs = entries.value.filter((e) => e.shift_date === ds);
    if (!dayEs.length) continue; // 편성 시작한 날만 점검
    for (const sh of shifts) {
      const cnt = new Set(dayEs.filter((e) => e.shift_start === sh.start).map((e) => e.staff_id)).size;
      if (cnt < required) {
        out.push({
          date: ds,
          kind: cnt === 0 ? "gap" : "ratio",
          text: `${shiftName(sh.start)}(${sh.start}–${sh.end}) ${cnt}/${required}명${cnt === 0 ? " · 공백" : ""} — 어르신 ${elders}명`,
        });
      }
    }
  }
  return out;
});
function alertDateLabel(ds: string): string {
  const d = new Date(ds + "T00:00:00");
  return `${d.getMonth() + 1}/${d.getDate()}(${DAY_LABELS[d.getDay()]})`;
}
const showAlerts = ref(true);
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">
          근무일정
          <q-badge v-if="dirty" color="orange" class="q-ml-sm" label="미저장" />
        </div>
        <div class="text-caption text-grey-6">
          {{ canCreate ? "변경 후 저장하세요. 저장 전까지는 반영되지 않습니다." : "직원 근무 일정 (읽기 전용)" }}
        </div>
      </div>

      <!-- Draft controls -->
      <div v-if="canCreate" class="col-auto q-gutter-xs">
        <q-btn v-if="viewMode === 'week'" outline color="primary" icon="o_auto_awesome" label="자동 생성" dense :loading="generating" @click="generateRotation">
          <q-tooltip>조 유형별 커버리지로 이번 주 근무를 생성합니다. 요양=24시간 3교대, 주간=1교대, 각 교대를 어르신 1:10 기준 인원으로 채웁니다(방문은 수동).</q-tooltip>
        </q-btn>
        <q-btn color="primary" icon="o_save" label="저장" unelevated dense :disable="!dirty" :loading="saving" @click="saveDraft" />
        <q-btn outline color="grey-8" icon="o_undo" label="되돌리기" dense :disable="!dirty" @click="rollbackDraft" />
        <q-btn flat color="negative" icon="o_refresh" label="클리어" dense @click="clearDraft" />
      </div>

      <div class="col-auto" style="min-width: 160px">
        <q-select
          v-model="selectedTeam"
          :options="teamOptions"
          label="팀"
          outlined dense emit-value map-options
        />
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
          v-for="p in allPresets"
          :key="p.label"
          class="palette-chip"
          :class="`palette-chip--${presetColor(p)}`"
          @pointerdown="startDrag($event, { kind: 'preset', preset: p }, p.label, presetColor(p))"
        >
          <q-icon name="o_drag_indicator" size="xs" class="q-mr-xs opacity-60" />
          {{ p.label }}
          <q-icon
            v-if="isCustom(p) && canDelete"
            name="o_close"
            size="xs"
            class="q-ml-xs palette-del"
            @pointerdown.stop
            @click.stop="deleteCustomType(p)"
          />
        </div>
        <q-btn
          outline no-caps dense color="grey-8" icon="o_add"
          label="근무 유형 추가"
          class="palette-custom-btn"
          @click="openNewType"
        />
      </div>
    </div>

    <!-- 인력 알림 -->
    <q-card v-if="staffingAlerts.length" flat bordered class="alert-card q-mb-md">
      <q-card-section class="row items-center q-py-sm">
        <q-icon name="o_warning" color="negative" class="q-mr-sm" />
        <span class="text-weight-medium text-negative col">인력 알림 {{ staffingAlerts.length }}건</span>
        <q-btn flat round dense :icon="showAlerts ? 'o_expand_less' : 'o_expand_more'" @click="showAlerts = !showAlerts" />
      </q-card-section>
      <q-slide-transition>
        <div v-show="showAlerts">
          <q-separator />
          <q-list dense>
            <q-item v-for="(a, idx) in staffingAlerts" :key="idx">
              <q-item-section avatar style="min-width:36px">
                <q-icon :name="a.kind === 'gap' ? 'o_schedule' : 'o_groups'" :color="a.kind === 'gap' ? 'orange' : 'negative'" size="20px" />
              </q-item-section>
              <q-item-section>
                <q-item-label><b>{{ alertDateLabel(a.date) }}</b> · {{ a.text }}</q-item-label>
              </q-item-section>
            </q-item>
          </q-list>
        </div>
      </q-slide-transition>
    </q-card>

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
                :class="['day-col', { 'today-col': isToday(date), 'red-col': isRedDay(date) }]"
              >
                <div class="day-label" :class="{ 'text-negative': isRedDay(date) }">{{ DAY_LABELS[di] }}</div>
                <div class="day-date" :class="isToday(date) ? 'text-primary text-weight-bold' : (isRedDay(date) ? 'text-negative' : 'text-grey-6')">
                  {{ date.getMonth() + 1 }}/{{ date.getDate() }}
                </div>
                <q-tooltip v-if="holidayName(date)">{{ holidayName(date) }}</q-tooltip>
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
            <div class="row items-center no-wrap">
              <span class="month-day-num" :class="isToday(date) ? 'today-badge' : (isRedDay(date) ? 'text-negative text-weight-bold' : '')">
                {{ date.getDate() }}
              </span>
              <span v-if="holidayName(date)" class="month-holiday text-negative">{{ holidayName(date) }}</span>
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
          <q-select v-model="form.preset" :options="dialogPresets" label="근무 유형" outlined dense option-label="label" @update:model-value="applyPreset" />
          <template v-if="form.preset.label === '직접'">
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
          <q-select v-model="editForm.preset" :options="dialogPresets" label="근무 유형" outlined dense option-label="label" @update:model-value="applyEditPreset" />
          <template v-if="editForm.preset.label === '직접'">
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

    <!-- ── 새 근무 유형 (custom shift type) ─────────────────────────────────── -->
    <q-dialog v-model="showNewType" persistent>
      <q-card style="min-width: 380px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">새 근무 유형</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-sm">
          <q-input v-model="newType.label" label="이름 (예: 오후 단축)" outlined dense />
          <div class="row q-gutter-sm">
            <q-input v-model="newType.start" label="시작" outlined dense class="col" hint="HH:MM" mask="##:##" />
            <q-input v-model="newType.end" label="종료" outlined dense class="col" hint="HH:MM" mask="##:##" />
          </div>
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="추가" unelevated @click="addNewType" />
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
.alert-card { background: #fff5f5; border-color: #ffd1d1; }
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
.red-col    { background: #fef2f2; }
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
.month-holiday { font-size: 0.62rem; margin-left: 4px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
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
.palette-del { cursor: pointer; opacity: 0.55; }
.palette-del:hover { opacity: 1; }
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
