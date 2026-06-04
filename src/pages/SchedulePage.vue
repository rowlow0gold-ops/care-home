<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { onBeforeRouteLeave } from "vue-router";
import { Store } from "@tauri-apps/plugin-store";
import { useQuasar } from "quasar";
import { server, type Team, type OrgPerson, type Resident } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const session = useServerSessionStore();

// 저장하지 않은 근무 드래프트가 있으면 화면을 떠날 때 확인을 받는다.
onBeforeRouteLeave((_to, _from, next) => {
  if (!dirty.value) { next(); return; }
  $q.dialog({
    title: "저장하지 않은 변경",
    message: "저장하지 않은 작업사항이 있습니다. 나가면 사라집니다. 계속할까요?",
    cancel: { label: "취소", flat: true },
    ok: { label: "나가기", color: "negative", unelevated: true },
    persistent: true,
  }).onOk(() => next()).onCancel(() => next(false));
});

// ── Role helpers (server permission ladder) ─────────────────────────────────
// 접수(create only) vs 행정+(full CRUD)
const canCreate = computed(() => session.canCreate);
const canEdit = computed(() => session.canEdit);
const canDelete = computed(() => session.canDelete);

// ── View mode ─────────────────────────────────────────────────────────────────
const viewMode = ref<"week" | "month">("week"); // 기본 주간 보기

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

// 월간 보기는 그리드 전체(앞뒤 달 포함)를 로드해 다른 달 데이터도 표시한다.
const gridStartStr = computed(() => localDateStr(monthGrid.value[0][0]));
const gridEndStr = computed(() => {
  const lastWeek = monthGrid.value[monthGrid.value.length - 1];
  return localDateStr(lastWeek[lastWeek.length - 1]);
});

// Rebuild the visible `entries` from the last server fetch + the overlay.
function rebuild() {
  const start = viewMode.value === "week" ? weekStartStr.value : gridStartStr.value;
  const end   = viewMode.value === "week" ? weekEndStr.value   : gridEndStr.value;
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
    title: "클리어", message: "현재 보이는 기간의 근무를 모두 비우고, 근무 유형도 초기화합니다. (근무는 저장 시 반영)",
    cancel: { label: "취소", flat: true }, ok: { label: "클리어", color: "negative", unelevated: true }, persistent: true,
  }).onOk(async () => {
    for (const e of entries.value) {
      if (isNew(e.id)) pendingCreates.value = pendingCreates.value.filter((c) => c.id !== e.id);
      else pendingDeletes.value.add(e.id);
    }
    pendingDeletes.value = new Set(pendingDeletes.value);
    rebuild();
    // 근무 유형 팔레트 초기화 (자동 생성 설정 + 커스텀 유형)
    genConfigured.value = false;
    customPresets.value = [];
    try {
      const s = await presetsStore();
      await s.delete("gen_form");
      await s.set("custom_presets", []);
      await s.save();
    } catch { /* best effort */ }
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
    const start = viewMode.value === "week" ? weekStartStr.value : gridStartStr.value;
    const end   = viewMode.value === "week" ? weekEndStr.value   : gridEndStr.value;
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
// 현재 스케줄러는 요양1팀을 기준으로 구현·검증되어 있다. 그 외 팀(방문1팀·주간1팀·
// 요양2팀 등)을 선택하면 '준비 중' 알림을 띄운다.
const READY_TEAM = "요양1팀";
const teamNotReady = computed(() => {
  const t = teams.value.find((x) => x.id === selectedTeam.value);
  return !!t && t.name !== READY_TEAM;
});
const selectedTeamName = computed(() => teams.value.find((x) => x.id === selectedTeam.value)?.name ?? "");

watch(selectedTeam, async () => {
  if (teamNotReady.value) {
    $q.notify({ type: "warning", icon: "o_construction", message: `'${selectedTeamName.value}' 필터는 아직 구현되지 않았습니다.`, caption: "현재 스케줄러는 요양1팀 기준으로 동작합니다." });
  }
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
// 기본 유형은 하드코딩하지 않는다. ‘자동 생성’에서 설정한 교대 패턴이 곧 근무 유형이
// 되고(설정 전에는 비어 있음), 사용자는 커스텀 유형을 추가로 만들 수 있다.
interface Preset { label: string; start: string; end: string; hours: number }
const MANUAL: Preset = { label: "직접", start: "", end: "", hours: 0 };

const genConfigured = ref(false); // 자동 생성 설정을 한 번이라도 했는지
const genPresets = computed<Preset[]>(() =>
  genConfigured.value
    ? genShifts.value.map((s) => ({
        label: `${s.name} (${s.start}–${s.end})`,
        start: s.start, end: s.end, hours: netHours(s.start, s.end),
      }))
    : [],
);

// Custom 근무 유형 — saved locally (per device), deletable.
const customPresets = ref<Preset[]>([]);
const allPresets = computed(() => [...genPresets.value, ...customPresets.value]);
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
    // 저장된 자동 생성 설정 복원 → 근무 유형 팔레트에 반영
    const gf = await s.get<{ pattern: string; cycleWeeks: 1 | 2; maxWeekHours: number; shiftStarts: string[]; restDays?: 1 | 2; resetExisting?: boolean; target?: "this" | "next" }>("gen_form");
    if (gf?.pattern && Array.isArray(gf.shiftStarts)) {
      genForm.value.pattern = gf.pattern;
      await nextTick(); // 패턴 watcher 가 shiftStarts 를 리셋한 뒤에 복원
      genForm.value.cycleWeeks = gf.cycleWeeks ?? 1;
      genForm.value.maxWeekHours = gf.maxWeekHours ?? 52;
      genForm.value.shiftStarts = gf.shiftStarts;
      genForm.value.restDays = gf.restDays ?? 1;
      genForm.value.resetExisting = gf.resetExisting ?? true;
      genForm.value.target = gf.target ?? "next";
      // genConfigured 는 복원하지 않는다 — 팔레트는 항상 비어 있게 시작하고,
      // 이번 세션에서 자동 생성을 실행해야 근무 유형이 나타난다.
    }
  } catch { customPresets.value = []; }
}
async function saveGenSettings() {
  genConfigured.value = true;
  try {
    const s = await presetsStore();
    await s.set("gen_form", { ...genForm.value, shiftStarts: [...genForm.value.shiftStarts] });
    await s.save();
  } catch { /* best effort */ }
}
async function savePresets() {
  const s = await presetsStore();
  await s.set("custom_presets", customPresets.value);
  await s.save();
}
function isCustom(p: Preset) { return customPresets.value.some((c) => c.label === p.label); }

// 화면의 근무(저장된 일정 포함)에서 서로 다른 시간대를 근무 유형으로 가져온다.
function importTypesFromSchedule() {
  const seen = new Map<string, Preset>();
  for (const e of entries.value) {
    const key = `${e.shift_start}-${e.shift_end}`;
    if (seen.has(key)) continue;
    const h = parseInt(e.shift_start.slice(0, 2), 10);
    const name = h >= 5 && h < 12 ? "주간" : h >= 12 && h < 18 ? "오후" : "야간";
    seen.set(key, { label: `${name} (${e.shift_start}–${e.shift_end})`, start: e.shift_start, end: e.shift_end, hours: e.shift_hours });
  }
  const fresh = [...seen.values()].filter(
    (p) => !allPresets.value.some((x) => (x.start === p.start && x.end === p.end) || x.label === p.label),
  );
  if (!fresh.length) {
    $q.notify({ type: "info", message: "가져올 근무 유형이 없습니다. (근무가 없거나 이미 등록됨)" });
    return;
  }
  customPresets.value = [...customPresets.value, ...fresh];
  savePresets();
  $q.notify({ type: "positive", message: `근무 유형 ${fresh.length}개를 가져왔습니다.` });
}

// ── Add shift ─────────────────────────────────────────────────────────────────
const showAdd    = ref(false);
const submitting = ref(false);
function defaultPreset(): Preset {
  return allPresets.value[0] ?? MANUAL;
}
const form = ref({
  staff_id:    null as string | null,
  shift_date:  "",
  preset:      MANUAL as Preset,
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
  const p = defaultPreset();
  form.value = {
    staff_id:    staffId,
    shift_date:  localDateStr(date),
    preset:      p,
    shift_start: p.label === "직접" ? "07:00" : p.start,
    shift_end:   p.label === "직접" ? "19:00" : p.end,
    shift_hours: p.label === "직접" ? 12 : p.hours,
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
// 휴게(식사)시간 — 근로기준법 기준 4시간당 30분을 근무시간에서 자동 차감한다.
//   예: 8시간 블록 → 휴게 1h → 근무 7h 인정 / 12시간 블록 → 휴게 1.5h → 10.5h.
function breakHoursOf(block: number): number {
  return Math.floor(block / 4) * 0.5;
}
function netHours(start: string, end: string): number {
  const block = hoursBetween(start, end);
  return Math.max(0.5, Math.round((block - breakHoursOf(block)) * 10) / 10);
}
// "HH:MM" + 분 (24시간 래핑)
function addToHM(hm: string, mins: number): string {
  const [h, m] = hm.split(":").map(Number);
  const t = (((h * 60 + m + mins) % 1440) + 1440) % 1440;
  return `${String(Math.floor(t / 60)).padStart(2, "0")}:${String(t % 60).padStart(2, "0")}`;
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
    label: t.label.trim(), start: t.start, end: t.end, hours: netHours(t.start, t.end),
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
  shift_date: "", preset: MANUAL, shift_start: "07:00", shift_end: "19:00", shift_hours: 12, notes: "",
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
// 다른 달 칸의 근무는 표시만 — 수정하려면 그 달로 이동해야 한다.
function notifyOtherMonth() {
  $q.notify({ type: "info", message: "다른 달의 근무입니다. 해당 달로 이동해서 수정하세요." });
}

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

// ── 자동 생성 — 24시간 교대 패턴 시스템 ─────────────────────────────────────
// 패턴(2조 맞교대·3조 3교대·4조 2교대·4조 3교대)을 고르면 팀 인력을 조로 나누고,
// 달력 기준 주기(1~2주)마다 주야를 회전시키며 각 교대를 1:10 인원으로 채운다.
// 승인 휴가는 건너뛰고, 부족하면 경고 + 인력 알림에 표시. 결과는 draft 오버레이.
const generating = ref(false);
const showGenDialog = ref(false);

interface PatShift { name: string; start: string; end: string }
interface GenPattern { value: string; label: string; groups: number; shifts: PatShift[] }
const SHIFTS_12 = [
  { name: "주간", start: "07:00", end: "19:00" },
  { name: "야간", start: "19:00", end: "07:00" },
];
const SHIFTS_8 = [
  { name: "주간", start: "07:00", end: "15:00" },
  { name: "오후", start: "15:00", end: "23:00" },
  { name: "야간", start: "23:00", end: "07:00" },
];
const PATTERNS: GenPattern[] = [
  { value: "2x12", label: "2조 맞교대 · 12시간 (주간/야간)", groups: 2, shifts: SHIFTS_12 },
  { value: "3x8",  label: "3조 3교대 · 8시간 (주간/오후/야간)", groups: 3, shifts: SHIFTS_8 },
  { value: "4x2",  label: "4조 2교대 · 12시간 (2개 조 휴무)", groups: 4, shifts: SHIFTS_12 },
  { value: "4x3",  label: "4조 3교대 · 8시간 (1개 조 휴무)", groups: 4, shifts: SHIFTS_8 },
];
const SHIFT_NAMES_2 = ["주간", "야간"];
const SHIFT_NAMES_3 = ["주간", "오후", "야간"];
const genForm = ref({
  pattern: "2x12", cycleWeeks: 1 as 1 | 2, maxWeekHours: 52, shiftStarts: ["07:00", "19:00"],
  target: "next" as "this" | "next", // 생성 대상: 이번 달 / 다음 달 (기본 다음 달)
  restDays: 1 as 1 | 2, // 주당 휴무일(인력별) — 절대 규칙. 시설은 매일(공휴일 포함) 운영
  resetExisting: true, // 생성 기간 내 기존 근무 초기화 후 생성
});
// 패턴을 바꾸면 시작 시각을 그 패턴 기본값으로 리셋
watch(() => genForm.value.pattern, () => {
  const pat = PATTERNS.find((p) => p.value === genForm.value.pattern) ?? PATTERNS[0];
  genForm.value.shiftStarts = pat.shifts.map((s) => s.start);
});
// 커스텀 시작 시각 → 교대 파생. 체류시간 = 식사 포함:
//   3교대(8/8/8): 9시간 체류(실근무 8h+식사 1h) → 교대 간 1시간 겹침(인수인계·식사 커버)
//   2교대(12h):  12시간 체류(실근무 10.5h, 휴게 1.5h 포함)
const genShifts = computed<PatShift[]>(() => {
  const starts = genForm.value.shiftStarts;
  const names = starts.length === 2 ? SHIFT_NAMES_2 : SHIFT_NAMES_3;
  const stayMin = (starts.length === 2 ? 12 : 9) * 60;
  return starts.map((s, i) => ({ name: names[i] ?? `교대${i + 1}`, start: s, end: addToHM(s, stayMin) }));
});
const genWorkers = ref<OrgPerson[]>([]);
const genTeam = computed(() => teams.value.find((t) => t.id === selectedTeam.value) ?? null);
const genElders = computed(() => (genTeam.value ? activeResidents.value.filter((r) => r.team_id === genTeam.value!.id).length : 0));
// 교대당 필요 인원: 1:10 + 요양팀은 최소 2명(2인 1조 페어 — 식사·응급 커버)
const genRequired = computed(() => {
  const base = Math.max(1, Math.ceil(genElders.value / 10));
  return genTeam.value?.team_type === "residential" ? Math.max(2, base) : base;
});
const genPattern = computed(() => PATTERNS.find((p) => p.value === genForm.value.pattern) ?? PATTERNS[0]);
const genGroupMin = computed(() => Math.floor(genWorkers.value.length / genPattern.value.groups));
const genGroupMax = computed(() => Math.ceil(genWorkers.value.length / genPattern.value.groups));
// 조당 최소 인원이 교대당 필요 인원(1:10)보다 적으면 인력 부족.
const genShort = computed(() => genGroupMin.value < genRequired.value);
// 주당 필요 시간(24h×7×필요인원) vs 가용 시간(인력×주 최대시간) — 부족하면 초과근무 발생.
const genNeedHours = computed(() => 24 * 7 * genRequired.value);
const genCapHours = computed(() => genWorkers.value.length * Math.max(8, genForm.value.maxWeekHours || 52));
const genOvertimeExpected = computed(() => genCapHours.value < genNeedHours.value);

// 달력에 고정된 주 번호 → 어느 주를 생성해도 회전이 일관되게 이어진다.
function weekIndexOf(d: Date): number {
  const epoch = new Date(2026, 0, 4).getTime(); // 기준 일요일
  const day = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  return Math.floor((day - epoch) / (7 * 86400000));
}

// 승인된 휴가 수집 (해당 날짜들에 겹치는 것만)
async function collectApprovedLeave(dates: string[]): Promise<Set<string>> {
  const onLeave = new Set<string>();
  try {
    let p = 1, fetched = 0, total = Infinity;
    while (fetched < total && p <= 10) {
      const res = await server.leaveRequestsPaged({ status: "approved", page: p, page_size: 100 });
      total = res.total;
      for (const lr of res.items) for (const ds of dates) if (ds >= lr.start_date && ds <= lr.end_date) onLeave.add(`${lr.user_id}-${ds}`);
      fetched += res.items.length;
      if (!res.items.length) break;
      p++;
    }
  } catch { /* 휴가 조회 실패 시 제외 없이 진행 */ }
  return onLeave;
}

// 자동 생성 버튼 → 팀 인력 로드 후 패턴 다이얼로그(요양) 또는 즉시 생성(주간팀)
async function openGenerate() {
  if (!canCreate.value) return;
  const team = genTeam.value;
  if (!team) { $q.notify({ type: "warning", message: "팀을 선택하세요." }); return; }
  if (teamNotReady.value) {
    $q.notify({ type: "warning", icon: "o_construction", message: `'${selectedTeamName.value}' 자동 생성은 아직 구현되지 않았습니다.`, caption: "요양1팀에서 사용해 주세요." });
    return;
  }
  if (team.team_type === "visit") { $q.notify({ type: "info", message: "방문팀은 케이스별로 수동 편성합니다." }); return; }
  generating.value = true;
  try {
    await loadResidentCounts();
    const org = await server.orgPaged({ page: 1, page_size: 500 });
    const myBranch = session.me?.branch_id;
    genWorkers.value = org.items.filter(
      (u) => !u.is_inactive && ["caregiver", "nurse"].includes(u.role) && u.team_id === team.id && (!myBranch || u.branch_id === myBranch),
    );
  } finally { generating.value = false; }
  if (!genWorkers.value.length) {
    $q.notify({ type: "warning", message: "이 팀에 배정된 인력이 없습니다. ‘팀’ 탭에서 먼저 배정하세요." });
    return;
  }
  showGenDialog.value = true; // 요양=패턴 선택 / 주간=휴무·시간 옵션
}

// 다이얼로그 '생성' → 팀 유형에 따라 분기
async function runGeneration() {
  const team = genTeam.value;
  if (!team) return;
  if (team.team_type === "day") {
    showGenDialog.value = false;
    saveGenSettings();
    await generateDayTeam(team);
  } else {
    await runPatternGeneration();
  }
}

// 생성 기간 내 기존 근무 초기화 — 드래프트로 처리(저장 전 미반영, 되돌리기 가능).
async function resetSpan(dates: string[]) {
  try {
    const rows = await server.roster(dates[0], dates[dates.length - 1], selectedTeam.value || undefined);
    for (const r of rows) {
      pendingDeletes.value.add(r.id);
      pendingUpdates.value.delete(r.id);
    }
    pendingDeletes.value = new Set(pendingDeletes.value);
    pendingUpdates.value = new Map(pendingUpdates.value);
  } catch { /* 조회 실패 시 초기화 생략 */ }
  pendingCreates.value = pendingCreates.value.filter((c) => c.shift_date < dates[0] || c.shift_date > dates[dates.length - 1]);
}


// 생성 스팬 전체의 기존 근무 + 주별 누적 시간/근무일수 상태
// (중복 방지 · 주 최대시간 한도 · 주당 휴무일 보장용)
async function buildSpanState(dates: string[]) {
  const existing = new Set<string>();              // `${id}-${ds}` 이미 근무 있는 날
  const weekHours = new Map<string, number>();     // `${id}-${weekIdx}` → 누적 시간
  const weekDays = new Map<string, number>();      // `${id}-${weekIdx}` → 근무 일수
  const wkOf = (ds: string) => weekIndexOf(new Date(ds + "T00:00:00"));
  const addHours = (id: string, ds: string, h: number) => {
    const k = `${id}-${wkOf(ds)}`;
    weekHours.set(k, (weekHours.get(k) ?? 0) + h);
    weekDays.set(k, (weekDays.get(k) ?? 0) + 1);
  };
  try {
    const rows = await server.roster(dates[0], dates[dates.length - 1], selectedTeam.value || undefined);
    for (const r of rows) {
      if (pendingDeletes.value.has(r.id)) continue;
      existing.add(`${r.user_id}-${r.shift_date}`);
      addHours(r.user_id, r.shift_date, r.shift_hours);
    }
  } catch { /* 조회 실패 시 빈 상태로 진행 */ }
  for (const c of pendingCreates.value) {
    if (c.shift_date >= dates[0] && c.shift_date <= dates[dates.length - 1]) {
      existing.add(`${c.staff_id}-${c.shift_date}`);
      addHours(c.staff_id, c.shift_date, c.shift_hours);
    }
  }
  const hoursOf = (id: string, ds: string) => weekHours.get(`${id}-${wkOf(ds)}`) ?? 0;
  const daysOf = (id: string, ds: string) => weekDays.get(`${id}-${wkOf(ds)}`) ?? 0;
  return { existing, addHours, hoursOf, daysOf };
}

// 생성 대상: 이번 주 일요일 기준 4주 블록.
//   this = 이번 주 일요일부터 4주 / next(기본) = 4주 후부터 4주
function spanDates(): { days: Date[]; dates: string[] } {
  const base = weekMonday(new Date()); // 이번 주 일요일
  const first = genForm.value.target === "this" ? base : addDays(base, 28);
  const days: Date[] = [];
  for (let i = 0; i < 28; i++) days.push(addDays(first, i));
  return { days, dates: days.map(localDateStr) };
}

// 주간팀: 팀 근무창 1교대, 1:10 인원, 주 최대시간 한도 + 휴가 제외, 대상 달 생성
async function generateDayTeam(team: Team) {
  generating.value = true;
  try {
    const required = genRequired.value;
    const maxH = Math.max(8, genForm.value.maxWeekHours || 52);
    const W = genWorkers.value;
    const { days, dates } = spanDates();
    if (genForm.value.resetExisting) await resetSpan(dates);
    const onLeave = await collectApprovedLeave(dates);
    const { existing, addHours, hoursOf, daysOf } = await buildSpanState(dates);
    const maxDays = 7 - genForm.value.restDays; // 주당 휴무일 보장
    const hrs = netHours(team.shift_start_hm, team.shift_end_hm); // 휴게 차감
    const fresh: ScheduleEntry[] = [];
    const put = (w: OrgPerson, ds: string, tag: string) => {
      existing.add(`${w.id}-${ds}`);
      addHours(w.id, ds, hrs);
      fresh.push({ id: tempId(), staff_id: w.id, staff_name: w.full_name, shift_date: ds,
        shift_start: team.shift_start_hm, shift_end: team.shift_end_hm, shift_hours: hrs, notes: `${team.name} 자동${tag}` });
    };
    let ptr = 0, shortfall = 0, overtime = 0;
    for (const d of days) {
      const ds = localDateStr(d);
      let filled = 0, tries = 0;
      while (filled < required && tries < W.length) {
        const w = W[ptr % W.length]; ptr++; tries++;
        const key = `${w.id}-${ds}`;
        if (onLeave.has(key) || existing.has(key) || hoursOf(w.id, ds) + hrs > maxH || daysOf(w.id, ds) >= maxDays) continue;
        put(w, ds, "");
        filled++;
      }
      // 커버리지 우선: 시간 한도는 넘을 수 있지만(초과), 휴가·하루1근무·주당 휴무일은 절대 규칙
      if (filled < required) {
        const forced = W
          .filter((w) => !onLeave.has(`${w.id}-${ds}`) && !existing.has(`${w.id}-${ds}`) && daysOf(w.id, ds) < maxDays)
          .sort((a, b) => hoursOf(a.id, ds) - hoursOf(b.id, ds));
        for (const w of forced) {
          if (filled >= required) break;
          put(w, ds, " 초과");
          filled++;
          overtime++;
        }
      }
      if (filled < required) shortfall++;
    }
    finishGeneration(fresh, shortfall, overtime, "주간 1교대", dates[0]);
  } finally { generating.value = false; }
}

// 요양팀: 선택한 패턴으로 24시간 시스템 생성 (1개월)
// 휴가·주 최대시간으로 조가 못 채우면 다른 조의 쉬는 인력으로 자동 대체(백필).
async function runPatternGeneration() {
  const team = genTeam.value;
  const pat = genPattern.value;
  if (!team) return;
  if (genWorkers.value.length < pat.groups) {
    $q.notify({ type: "negative", message: `${pat.groups}개 조를 나눌 인력이 부족합니다 (최소 ${pat.groups}명, 현재 ${genWorkers.value.length}명).` });
    return;
  }
  if (genForm.value.shiftStarts.some((s) => !/^\d{2}:\d{2}$/.test(s))) {
    $q.notify({ type: "negative", message: "교대 시작 시각(HH:MM)을 확인하세요." });
    return;
  }
  showGenDialog.value = false;
  saveGenSettings(); // 설정한 교대 패턴이 근무 유형 팔레트가 된다
  generating.value = true;
  try {
    const required = genRequired.value;
    const maxH = Math.max(8, genForm.value.maxWeekHours || 52);
    const shifts = genShifts.value.map((s) => ({ ...s, hours: netHours(s.start, s.end) })); // 휴게 차감
    // 조 나누기 (라운드로빈)
    const groups: OrgPerson[][] = Array.from({ length: pat.groups }, () => []);
    genWorkers.value.forEach((w, i) => groups[i % pat.groups].push(w));
    const { days, dates } = spanDates();
    if (genForm.value.resetExisting) await resetSpan(dates);
    const onLeave = await collectApprovedLeave(dates);
    const { existing, addHours, hoursOf, daysOf } = await buildSpanState(dates);
    const maxDays = 7 - genForm.value.restDays; // 주당 휴무일 보장 — 절대 규칙

    const canWork = (w: OrgPerson, ds: string, h: number) =>
      !onLeave.has(`${w.id}-${ds}`) && !existing.has(`${w.id}-${ds}`) &&
      hoursOf(w.id, ds) + h <= maxH && daysOf(w.id, ds) < maxDays;
    const fresh: ScheduleEntry[] = [];
    // 2인 1조 페어: 같은 교대의 짝수 번째 인원은 +30분 늦게 출근 — 식사·인수인계를 서로 커버
    const assign = (w: OrgPerson, ds: string, sh: { name: string; start: string; end: string; hours: number }, tag: string, slot: number) => {
      const off = slot % 2 === 1 ? 30 : 0;
      const st = off ? addToHM(sh.start, off) : sh.start;
      const en = off ? addToHM(sh.end, off) : sh.end;
      existing.add(`${w.id}-${ds}`);
      addHours(w.id, ds, sh.hours);
      fresh.push({ id: tempId(), staff_id: w.id, staff_name: w.full_name, shift_date: ds,
        shift_start: st, shift_end: en, shift_hours: sh.hours,
        notes: `${team.name} ${sh.name}${tag}${off ? " ·페어B" : ""}` });
    };

    const ptrs = groups.map(() => 0);
    let shortfall = 0, overtime = 0;
    for (const d of days) {
      const ds = localDateStr(d);
      const period = Math.floor(weekIndexOf(d) / genForm.value.cycleWeeks); // 1~2주마다 주야 회전
      shifts.forEach((sh, si) => {
        const gi = (si + period) % pat.groups;       // 교대 si ← 조 gi (휴무 조는 자동으로 빠짐)
        const g = groups[gi];
        let filled = 0, tries = 0;
        while (filled < required && tries < g.length) {
          const w = g[ptrs[gi] % g.length]; ptrs[gi]++; tries++;
          if (!canWork(w, ds, sh.hours)) continue;
          assign(w, ds, sh, `(${["A", "B", "C", "D"][gi] ?? gi + 1}조)`, filled);
          filled++;
        }
        // 2차 백필: 휴가·한도로 부족하면 그날 쉬는 인력 중 누적시간 적은 순으로 대체
        if (filled < required) {
          const subs = genWorkers.value
            .filter((w) => canWork(w, ds, sh.hours))
            .sort((a, b) => hoursOf(a.id, ds) - hoursOf(b.id, ds));
          for (const w of subs) {
            if (filled >= required) break;
            assign(w, ds, sh, " 대체", filled);
            filled++;
          }
        }
        // 3차 강제 배치: 24시간·매일 커버를 위해 주 최대시간은 넘을 수 있다('초과' 표기).
        //   단, 휴가·하루 1근무·주당 휴무일은 절대 규칙 — 못 채우면 진짜 공백(충원 필요).
        if (filled < required) {
          const forced = genWorkers.value
            .filter((w) => !onLeave.has(`${w.id}-${ds}`) && !existing.has(`${w.id}-${ds}`) && daysOf(w.id, ds) < maxDays)
            .sort((a, b) => hoursOf(a.id, ds) - hoursOf(b.id, ds));
          for (const w of forced) {
            if (filled >= required) break;
            assign(w, ds, sh, " 초과", filled);
            filled++;
            overtime++;
          }
        }
        if (filled < required) shortfall++; // 그날 전원이 휴가/근무중일 때만 남는 진짜 공백
      });
    }
    finishGeneration(fresh, shortfall, overtime, pat.label, dates[0]);
  } finally { generating.value = false; }
}

async function finishGeneration(fresh: ScheduleEntry[], shortfall: number, overtime: number, patternLabel: string, firstDs: string) {
  if (!fresh.length) {
    $q.notify({ type: "info", message: "생성할 빈 칸이 없습니다. (이미 채워져 있습니다)" });
    return;
  }
  pendingCreates.value = [...pendingCreates.value, ...fresh];
  if (selectedTeam.value) await loadTeamStaff(selectedTeam.value);
  // 생성한 달이 보이도록 뷰 이동
  const d0 = new Date(firstDs + "T00:00:00");
  if (viewMode.value === "week") currentMonday.value = weekMonday(d0);
  else currentMonth.value = new Date(d0.getFullYear(), d0.getMonth(), 1);
  rebuild();
  if (shortfall > 0) {
    $q.notify({
      type: "warning", icon: "o_warning", timeout: 8000,
      message: `${fresh.length}건 생성 — 공백 ${shortfall}건!`,
      caption: `해당 시간대에 배치 가능한 인력이 아무도 없습니다(전원 휴가/근무중). 인력 알림을 확인하세요.`,
    });
  } else if (overtime > 0) {
    $q.notify({
      type: "warning", icon: "o_schedule", timeout: 8000,
      message: `${fresh.length}건 생성 — 24시간 커버 완료, 초과근무 ${overtime}건`,
      caption: `주 최대 근무시간을 넘겨 배치된 근무가 있습니다('초과' 표기). 인력 충원을 권장합니다.`,
    });
  } else {
    $q.notify({ type: "positive", message: `${fresh.length}건 생성 (${patternLabel} · 24시간 커버). 확인 후 저장하세요.` });
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
// 시간대(시간 단위) 커버리지 점검 — 패턴(8h/12h/수동 혼합)과 무관하게 동작한다.
//   요양: 24시간 전체, 매 시각 근무 인원이 ceil(어르신/10) 이상인지.
//   주간: 팀 근무창 시간대만 점검. 방문: 점검 없음.
//   아직 편성하지 않은 날(근무 0건)은 표시하지 않는다.
const staffingAlerts = computed<Alert[]>(() => {
  const team = teams.value.find((t) => t.id === selectedTeam.value);
  if (!team || team.team_type === "visit") return [];
  const elders = activeResidents.value.filter((r) => r.team_id === team.id).length;
  if (!elders) return [];
  const base = Math.max(1, Math.ceil(elders / 10)); // 1:10
  const required = team.team_type === "residential" ? Math.max(2, base) : base; // 요양: 페어 최소 2명
  // 점검 대상 시간대
  let checkHours: number[];
  if (team.team_type === "residential") {
    checkHours = Array.from({ length: 24 }, (_, h) => h);
  } else {
    const s = parseInt(team.shift_start_hm.slice(0, 2), 10);
    let e = parseInt(team.shift_end_hm.slice(0, 2), 10);
    if (e <= s) e += 24;
    checkHours = [];
    for (let h = s; h < e; h++) checkHours.push(h % 24);
  }
  const dates =
    viewMode.value === "week"
      ? weekDates.value.map(localDateStr)
      : Array.from(new Set(monthGrid.value.flat().map(localDateStr))).filter(
          (d) => d.slice(0, 7) === monthStartStr.value.slice(0, 7),
        );
  const out: Alert[] = [];
  for (const ds of dates) {
    const dayEs = entries.value.filter((e) => e.shift_date === ds);
    if (!dayEs.length) continue;
    // 시간대별 근무 인원 수
    const cnt = new Array(24).fill(0) as number[];
    for (const e of dayEs) {
      const sh = parseInt(e.shift_start.slice(0, 2), 10);
      let eh = parseInt(e.shift_end.slice(0, 2), 10);
      if (e.shift_end === "00:00") eh = 24;
      if (eh <= sh) eh += 24;
      for (let h = sh; h < eh; h++) cnt[h % 24]++;
    }
    const lacking = checkHours.filter((h) => cnt[h] < required).sort((a, b) => a - b);
    if (!lacking.length) continue;
    // 연속 시간대로 묶어 표시
    const ranges: string[] = [];
    let i = 0;
    let hasGap = false;
    while (i < lacking.length) {
      let j = i;
      while (j + 1 < lacking.length && lacking[j + 1] === lacking[j] + 1) j++;
      const seg = lacking.slice(i, j + 1);
      const minCnt = Math.min(...seg.map((h) => cnt[h]));
      if (minCnt === 0) hasGap = true;
      ranges.push(`${String(seg[0]).padStart(2, "0")}:00–${String((seg[seg.length - 1] + 1) % 24).padStart(2, "0")}:00 ${minCnt}/${required}명`);
      i = j + 1;
    }
    out.push({ date: ds, kind: hasGap ? "gap" : "ratio", text: `인원 부족: ${ranges.join(", ")} (어르신 ${elders}명 · 1:10)` });
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
        <q-btn outline color="primary" icon="o_auto_awesome" label="자동 생성" dense :loading="generating" @click="openGenerate">
          <q-tooltip>대상 달(이번 달/다음 달)의 근무를 24시간 교대 패턴으로 생성합니다. 주야 변경 주기·휴일 옵션·1:10 인원 자동 배치.</q-tooltip>
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
        <span v-if="!allPresets.length" class="text-caption text-grey-5">
          ‘자동 생성’에서 교대 패턴을 설정하면 근무 유형이 여기에 표시됩니다.
        </span>
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
          v-if="entries.length"
          outline no-caps dense color="grey-8" icon="o_download"
          label="근무 유형 가져오기"
          class="palette-custom-btn"
          @click="importTypesFromSchedule"
        >
          <q-tooltip>화면의 근무에서 시간대를 추출해 유형으로 등록합니다.</q-tooltip>
        </q-btn>
        <q-btn
          v-if="allPresets.length"
          outline no-caps dense color="grey-8" icon="o_add"
          label="근무 유형 추가"
          class="palette-custom-btn"
          @click="openNewType"
        />
      </div>
    </div>

    <!-- 준비 중인 팀 안내 -->
    <q-banner v-if="teamNotReady" dense rounded class="bg-orange-1 text-orange-10 q-mb-md">
      <template #avatar><q-icon name="o_construction" /></template>
      ‘{{ selectedTeamName }}’ 필터는 아직 구현되지 않았습니다. 현재 스케줄러는 <b>요양1팀</b> 기준으로 동작합니다.
    </q-banner>

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
            :data-cell-key="date.getMonth() === currentMonth.getMonth() ? dropKey(null, date) : undefined"
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
                :class="[`shift-chip--${shiftColor(entry)}`, { 'shift-chip--clickable': canEdit && date.getMonth() === currentMonth.getMonth(), 'shift-chip--dropped': droppedId === entry.id }]"
                @pointerdown.stop="canEdit && date.getMonth() === currentMonth.getMonth() && startDrag($event, { kind: 'entry', id: entry.id }, shiftLabel(entry), shiftColor(entry))"
                @click.stop="date.getMonth() === currentMonth.getMonth() ? openEditDialog(entry) : notifyOtherMonth()"
              >
                <span class="month-shift-name">{{ entry.staff_name.split(' ')[0] }}</span>
                <span class="month-shift-time">{{ entry.shift_start }}</span>
                <q-btn
                  v-if="canDelete && date.getMonth() === currentMonth.getMonth()"
                  flat round dense icon="o_close" size="xs"
                  class="shift-delete month-delete"
                  @click.stop="deleteShift(entry)"
                />
                <q-tooltip>{{ entry.staff_name }} · {{ shiftLabel(entry) }}<span v-if="entry.notes"> · {{ entry.notes }}</span><span v-if="date.getMonth() !== currentMonth.getMonth()"> · 다른 달 (이동해서 수정)</span></q-tooltip>
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

    <!-- 자동 생성 — 24시간 교대 패턴 -->
    <q-dialog v-model="showGenDialog">
      <q-card style="min-width: 460px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">자동 생성 — 24시간 교대</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-md">
          <div class="row q-gutter-sm">
            <q-select class="col" v-model="genForm.target" outlined dense emit-value map-options label="생성 대상 (4주)"
              :options="[
                { label: '다음 4주 (4주 후부터)', value: 'next' },
                { label: '이번 4주 (이번 주 일요일부터)', value: 'this' },
              ]" />
            <q-select class="col" v-model="genForm.restDays" outlined dense emit-value map-options label="주당 휴무일 (인력별)"
              :options="[
                { label: '주 1일 휴무', value: 1 },
                { label: '주 2일 휴무', value: 2 },
              ]" hint="시설은 매일(공휴일 포함) 운영 — 인력별 휴무 보장" />
          </div>
          <template v-if="genTeam?.team_type === 'residential'">
            <q-select v-model="genForm.pattern" :options="PATTERNS" option-value="value" option-label="label"
              emit-value map-options outlined dense label="교대 패턴" />
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="genForm.cycleWeeks" outlined dense emit-value map-options label="주야 변경 주기"
                :options="[{ label: '1주마다 변경', value: 1 }, { label: '2주마다 변경', value: 2 }]" />
              <q-input class="col" v-model.number="genForm.maxWeekHours" type="number" outlined dense
                label="주 최대 근무시간" suffix="h" hint="기본 52h — 한도 내에서 휴무 자동 발생" />
            </div>
            <div>
              <div class="text-caption text-grey-7 q-mb-xs">교대 시작 시각 — 종료는 다음 교대 시작 (24시간 자동 커버)</div>
              <div class="row q-gutter-sm">
                <q-input v-for="(s, i) in genForm.shiftStarts" :key="i" class="col" v-model="genForm.shiftStarts[i]"
                  :label="genShifts[i]?.name" outlined dense mask="##:##" hint="HH:MM" />
              </div>
            </div>
          </template>
          <q-input v-else v-model.number="genForm.maxWeekHours" type="number" outlined dense
            label="주 최대 근무시간" suffix="h" hint="기본 52h" />
          <q-checkbox v-model="genForm.resetExisting" dense label="기존 근무 초기화 후 생성 (대상 4주 전체, 저장 전까지 되돌리기 가능)" />
          <q-banner v-if="genTeam?.team_type === 'residential'" dense rounded class="bg-blue-1 text-blue-10">
            <template #avatar><q-icon name="o_groups_2" /></template>
            <b>체류·페어 규칙</b><br />
            · {{ genForm.shiftStarts.length === 2 ? "12시간 체류 (휴게 1.5h 포함, 실근무 10.5h)" : "교대당 9시간 체류 (식사 1h 포함, 실근무 8h) — 교대 간 1시간 겹침으로 인수인계" }}<br />
            · 2인 1조 페어: 짝은 30분 차이로 출근해 식사·응급 시 서로 커버합니다 (교대당 최소 2명).
          </q-banner>
          <div class="text-caption text-grey-7">
            <q-icon name="o_event" size="14px" /> 승인된 휴가는 제외되고, 부족분은 쉬는 인력으로 자동 대체됩니다.
          </div>
          <q-banner dense rounded
            :class="genShort ? 'bg-red-1 text-red-9' : genOvertimeExpected ? 'bg-orange-1 text-orange-10' : 'bg-green-1 text-green-9'">
            <template #avatar><q-icon :name="genShort || genOvertimeExpected ? 'o_warning' : 'o_check_circle'" /></template>
            어르신 {{ genElders }}명 → 교대당 <b>{{ genRequired }}명</b> 필요 (1:10)<br />
            배정 인력 {{ genWorkers.length }}명 → {{ genPattern.groups }}개 조, 조당 {{ genGroupMin }}~{{ genGroupMax }}명<br />
            주당 필요 {{ genNeedHours }}h / 가용 {{ genCapHours }}h
            <template v-if="genShort">
              <br /><b>⚠ 인력 부족:</b> 조당 최소 {{ genRequired }}명이 필요합니다.
            </template>
            <template v-if="genOvertimeExpected">
              <br /><b>⚠ 초과근무 발생:</b> 24시간·매일 커버를 위해 일부 인력이 주 최대시간을
              넘겨 배치됩니다('초과' 표기). 인력 충원을 권장합니다.
            </template>
          </q-banner>
        </q-card-section>
        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn :color="genShort ? 'negative' : 'primary'" :label="genShort ? '부족해도 생성' : '생성'" unelevated
            :loading="generating" @click="runGeneration" />
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
