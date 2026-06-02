<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useQuasar } from "quasar";
import { server, type Team, type OrgPerson, type Resident } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const session = useServerSessionStore();
const canEdit = computed(() => session.canEdit);

const teams = ref<Team[]>([]);
const staff = ref<OrgPerson[]>([]);
const residents = ref<Resident[]>([]);
const loading = ref(false);
const careRoles = new Set(["caregiver", "nurse"]);

// 팀 유형
const TEAM_TYPE_OPTIONS = [
  { label: "요양 (24시간)", value: "residential" as const },
  { label: "주간", value: "day" as const },
  { label: "방문", value: "visit" as const },
];
const teamTypeLabel: Record<string, string> = { residential: "요양", day: "주간", visit: "방문" };
const teamTypeColor: Record<string, string> = { residential: "teal", day: "indigo", visit: "deep-orange" };
const TYPE_DEFAULT_SHIFT: Record<string, { start: string; end: string }> = {
  residential: { start: "07:00", end: "19:00" },
  day: { start: "09:00", end: "18:00" },
  visit: { start: "09:00", end: "13:00" },
};

// 근무 표기: 요양=24시간, 방문=custom, 그 외=시간창
function shiftText(t: Team): string {
  if (t.team_type === "residential") return "24시간";
  if (t.team_type === "visit") return "custom";
  return `근무 ${t.shift_start_hm} ~ ${t.shift_end_hm}`;
}

const myBranch = computed(() => session.me?.branch_id ?? null);
const caregivers = computed(() =>
  staff.value.filter((s) => !s.is_inactive && careRoles.has(s.role) && (!myBranch.value || s.branch_id === myBranch.value)),
);
function workersOf(teamId: string) {
  return caregivers.value.filter((s) => s.team_id === teamId).length;
}
function residentsOf(teamId: string) {
  return residents.value.filter((r) => r.status === "active" && r.team_id === teamId).length;
}
function ratioOf(teamId: string): { text: string; warn: boolean } {
  const w = workersOf(teamId), r = residentsOf(teamId);
  if (w === 0) return { text: r > 0 ? "인력 없음" : "—", warn: r > 0 };
  const per = r / w;
  return { text: `1 : ${per.toFixed(1)}`, warn: per > 10 };
}

async function load() {
  loading.value = true;
  try {
    const [t, org, res] = await Promise.all([
      server.teams(),
      server.orgPaged({ page: 1, page_size: 500 }),
      server.residentsPaged({ page: 1, page_size: 1000, status: "active" }),
    ]);
    teams.value = t.sort((a, b) => a.sort_order - b.sort_order);
    staff.value = org.items;
    residents.value = res.items;
  } catch (e: any) {
    $q.notify({ type: "negative", message: `불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}

// ── 팀 인력 보드 (카드 클릭 → 드래그로 추가/제거/이동) ───────────────────────
const showBoard = ref(false);
const boardTeamId = ref<string | null>(null);
// 드래프트 오버레이: 저장 전까지 서버에 반영하지 않는다. (worker_id → team_id|null)
const pending = ref(new Map<string, string | null>());
const dirty = computed(() => pending.value.size > 0);
const pendingCount = computed(() => pending.value.size);
const savingBoard = ref(false);
function effectiveTeam(w: OrgPerson): string | null {
  return pending.value.has(w.id) ? (pending.value.get(w.id) ?? null) : (w.team_id ?? null);
}
function stageAssign(w: OrgPerson, target: string | null) {
  const orig = w.team_id ?? null;
  if (orig === target) pending.value.delete(w.id); // 원래대로 → 변경 없음
  else pending.value.set(w.id, target);
  pending.value = new Map(pending.value); // 반응성
}
async function saveBoard() {
  if (!pending.value.size) return;
  savingBoard.value = true;
  try {
    for (const [id, tid] of pending.value) await server.assignTeam(id, tid);
    const n = pending.value.size;
    pending.value = new Map();
    await load();
    $q.notify({ type: "positive", message: `${n}명 배정을 저장했습니다.` });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` });
  } finally {
    savingBoard.value = false;
  }
}
function revertBoard() {
  if (!pending.value.size) return;
  pending.value = new Map();
  $q.notify({ type: "info", message: "변경을 되돌렸습니다." });
}

interface BoardCol { id: string | null; name: string; type: string | null; workers: OrgPerson[] }
const boardColumns = computed<BoardCol[]>(() => [
  { id: null, name: "미배정", type: null, workers: caregivers.value.filter((w) => effectiveTeam(w) === null) },
  ...teams.value.map((t) => ({ id: t.id, name: t.name, type: t.team_type, workers: caregivers.value.filter((w) => effectiveTeam(w) === t.id) })),
]);
function openBoard(t: Team) {
  boardTeamId.value = t.id;
  showBoard.value = true;
}

// pointer 기반 드래그 (WKWebView 에서 HTML5 DnD 불안정 → pointer 사용)
let dragId: string | null = null;
const dragging = ref(false);
const dragLabel = ref("");
const overCol = ref<string>("");
const ghost = ref({ x: 0, y: 0, show: false });

function colKey(id: string | null) { return id ?? "__none__"; }
function colUnder(x: number, y: number): string | null {
  const g = document.getElementById("team-drag-ghost");
  if (g) g.style.display = "none";
  const el = document.elementFromPoint(x, y)?.closest("[data-col-id]") as HTMLElement | null;
  if (g) g.style.display = "";
  return el ? (el.dataset.colId ?? null) : null;
}
function startDragWorker(e: PointerEvent, w: OrgPerson) {
  if (!canEdit.value) return;
  e.preventDefault();
  dragId = w.id;
  dragLabel.value = w.full_name;
  dragging.value = true;
  ghost.value = { x: e.clientX + 12, y: e.clientY - 10, show: true };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
}
function onMove(e: PointerEvent) {
  ghost.value = { x: e.clientX + 12, y: e.clientY - 10, show: true };
  overCol.value = colUnder(e.clientX, e.clientY) ?? "";
}
async function onUp(e: PointerEvent) {
  window.removeEventListener("pointermove", onMove);
  window.removeEventListener("pointerup", onUp);
  ghost.value = { ...ghost.value, show: false };
  dragging.value = false;
  const over = colUnder(e.clientX, e.clientY);
  overCol.value = "";
  const id = dragId; dragId = null;
  if (!id || over === null) return;            // 컬럼 밖에 드롭
  const target = over === "__none__" ? null : over;
  const w = staff.value.find((s) => s.id === id);
  if (!w || effectiveTeam(w) === target) return;
  stageAssign(w, target);                       // 드래프트에 반영 (저장 시 실제 반영)
}

// ── 어르신 → 팀 배정 (호실 기준 일괄) ────────────────────────────────────────
const showAssignResidents = ref(false);
const assignTeamId = ref<string | null>(null);
const roomFrom = ref("");
const roomTo = ref("");
const unassignedOnly = ref(false);
const assigningResidents = ref(false);
const assignTargetTeam = computed(() => teams.value.find((t) => t.id === assignTeamId.value) ?? null);
const matchedResidents = computed<Resident[]>(() => {
  const team = assignTargetTeam.value;
  if (!team) return [];
  const from = roomFrom.value.trim() ? Number(roomFrom.value) : null;
  const to = roomTo.value.trim() ? Number(roomTo.value) : null;
  return residents.value.filter((r) => {
    if (r.status !== "active") return false;
    if (r.branch_id !== team.branch_id) return false;
    if (unassignedOnly.value && r.team_id) return false;
    if (from !== null || to !== null) {
      const n = Number(r.room_number);
      if (Number.isNaN(n)) return false;
      if (from !== null && n < from) return false;
      if (to !== null && n > to) return false;
    }
    return true;
  });
});
function openAssignResidents() {
  assignTeamId.value = teams.value[0]?.id ?? null;
  roomFrom.value = ""; roomTo.value = ""; unassignedOnly.value = false;
  showAssignResidents.value = true;
}
async function assignResidents() {
  const team = assignTargetTeam.value;
  const list = matchedResidents.value;
  if (!team || !list.length) return;
  assigningResidents.value = true;
  try {
    for (const r of list) await server.assignResidentTeam(r.id, team.id);
    $q.notify({ type: "positive", message: `${list.length}명을 ${team.name}에 배정했습니다.` });
    showAssignResidents.value = false;
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `배정 실패: ${e?.message ?? e}` });
  } finally {
    assigningResidents.value = false;
  }
}

// ── 팀 추가 / 수정 ──────────────────────────────────────────────────────────
const showTeamDialog = ref(false);
const editingId = ref<string | null>(null);
const form = ref({ name: "", team_type: "residential" as "residential" | "day" | "visit", shift_start_hm: "07:00", shift_end_hm: "19:00", color_hue: 210 });
const HUE_PRESETS = [210, 260, 150, 30, 340, 110];
function openNew() {
  editingId.value = null;
  form.value = { name: "", team_type: "residential", shift_start_hm: "07:00", shift_end_hm: "19:00", color_hue: 210 };
  showTeamDialog.value = true;
}
function openEdit(t: Team) {
  editingId.value = t.id;
  form.value = { name: t.name, team_type: t.team_type, shift_start_hm: t.shift_start_hm, shift_end_hm: t.shift_end_hm, color_hue: t.color_hue };
  showTeamDialog.value = true;
}
function onTypeChange(v: "residential" | "day" | "visit") {
  const d = TYPE_DEFAULT_SHIFT[v];
  if (d) { form.value.shift_start_hm = d.start; form.value.shift_end_hm = d.end; }
}
async function saveTeam() {
  const f = form.value;
  if (!f.name.trim() || !/^\d{2}:\d{2}$/.test(f.shift_start_hm) || !/^\d{2}:\d{2}$/.test(f.shift_end_hm)) {
    $q.notify({ type: "negative", message: "이름·근무 시작·종료(HH:MM)를 입력하세요." });
    return;
  }
  try {
    if (editingId.value) await server.updateTeam(editingId.value, { ...f, name: f.name.trim() });
    else await server.createTeam({ ...f, name: f.name.trim(), sort_order: teams.value.length + 1 });
    showTeamDialog.value = false;
    await load();
    $q.notify({ type: "positive", message: "저장되었습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` });
  }
}
function removeTeam(t: Team) {
  if (workersOf(t.id) > 0 || residentsOf(t.id) > 0) {
    $q.notify({ type: "warning", message: "배정된 인력·어르신을 먼저 옮긴 뒤 삭제하세요." });
    return;
  }
  $q.dialog({
    title: "팀 삭제", message: `'${t.name}' 팀을 삭제할까요?`,
    cancel: { label: "취소", flat: true }, ok: { label: "삭제", color: "negative", unelevated: true }, persistent: true,
  }).onOk(async () => {
    try { await server.deleteTeam(t.id); await load(); $q.notify({ type: "positive", message: "삭제되었습니다." }); }
    catch (e: any) { $q.notify({ type: "negative", message: `삭제 실패: ${e?.message ?? e}` }); }
  });
}

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">팀</div>
        <div class="text-caption text-grey-6">팀 카드를 눌러 인력을 드래그로 배치하고, 각 팀이 담당할 어르신을 배정합니다</div>
      </div>
      <q-btn v-if="canEdit" outline color="primary" icon="o_elderly" label="어르신 배정" @click="openAssignResidents" />
      <q-btn v-if="canEdit" unelevated color="primary" icon="o_add" label="팀 추가" @click="openNew" />
      <q-btn flat round dense icon="o_refresh" :loading="loading" @click="load" />
    </div>

    <!-- 팀 카드 -->
    <div class="row q-col-gutter-md">
      <div v-for="t in teams" :key="t.id" class="col-12 col-sm-6 col-md-4 col-lg-3">
        <q-card flat bordered class="team-card cursor-pointer" @click="openBoard(t)">
          <div class="team-bar" :style="{ background: `hsl(${t.color_hue} 60% 55%)` }" />
          <q-card-section class="q-pb-xs">
            <div class="row items-center no-wrap">
              <div class="col text-subtitle1 text-weight-bold ellipsis">{{ t.name }}</div>
              <template v-if="canEdit">
                <q-btn flat round dense size="sm" icon="o_edit" @click.stop="openEdit(t)" />
                <q-btn flat round dense size="sm" icon="o_delete" color="grey-6" @click.stop="removeTeam(t)" />
              </template>
            </div>
            <div class="row items-center q-gutter-xs q-mt-xs">
              <q-badge :color="teamTypeColor[t.team_type] ?? 'grey'" :label="teamTypeLabel[t.team_type] ?? t.team_type" />
              <span class="text-caption text-grey-6">{{ shiftText(t) }}</span>
            </div>
          </q-card-section>
          <q-card-section class="row q-pt-none text-center">
            <div class="col"><div class="text-h6">{{ workersOf(t.id) }}</div><div class="text-caption text-grey-6">인력</div></div>
            <div class="col"><div class="text-h6">{{ residentsOf(t.id) }}</div><div class="text-caption text-grey-6">어르신</div></div>
            <div class="col">
              <q-chip dense :color="ratioOf(t.id).warn ? 'negative' : 'green-1'" :text-color="ratioOf(t.id).warn ? 'white' : 'green-9'"
                :icon="ratioOf(t.id).warn ? 'o_warning' : undefined" class="q-mt-xs">{{ ratioOf(t.id).text }}</q-chip>
              <div class="text-caption text-grey-6">비율</div>
            </div>
          </q-card-section>
        </q-card>
      </div>
      <div v-if="!teams.length && !loading" class="col-12 text-center text-grey-5 q-py-lg">팀이 없습니다. ‘팀 추가’로 만들어 주세요.</div>
    </div>

    <!-- 인력 보드 -->
    <q-dialog v-model="showBoard" maximized>
      <q-card>
        <q-card-section class="row items-center q-gutter-sm">
          <div class="text-h6">인력 배치 — 드래그로 팀 이동</div>
          <q-badge v-if="dirty" color="orange" :label="`미저장 ${pendingCount}`" />
          <q-space />
          <q-btn v-if="canEdit" unelevated dense color="primary" icon="o_save" label="저장" :disable="!dirty" :loading="savingBoard" @click="saveBoard" />
          <q-btn v-if="canEdit" outline dense color="grey-8" icon="o_undo" label="되돌리기" :disable="!dirty" @click="revertBoard" />
          <q-btn flat round dense icon="o_close" v-close-popup />
        </q-card-section>
        <q-separator />
        <q-card-section>
          <div class="text-caption text-grey-6 q-mb-sm">
            <q-icon name="o_drag_indicator" size="xs" /> 직원 칩을 다른 팀(또는 미배정)으로 드래그하세요. 미배정 → 팀 = 추가, 팀 → 미배정 = 제거.
          </div>
          <div class="board">
            <div v-for="col in boardColumns" :key="colKey(col.id)" class="board-col"
              :data-col-id="colKey(col.id)" :class="{ 'col-over': overCol === colKey(col.id), 'col-focus': boardTeamId === col.id }">
              <div class="board-col-head">
                <span class="text-weight-bold">{{ col.name }}</span>
                <q-badge v-if="col.type" :color="teamTypeColor[col.type] ?? 'grey'" :label="teamTypeLabel[col.type]" class="q-ml-xs" />
                <q-badge color="grey-4" text-color="grey-9" :label="col.workers.length" class="q-ml-xs" />
              </div>
              <div class="board-col-body">
                <div v-for="w in col.workers" :key="w.id" class="wk-chip" :class="{ disabled: !canEdit }"
                  @pointerdown="startDragWorker($event, w)">
                  <q-icon name="o_drag_indicator" size="xs" class="opacity-60 q-mr-xs" />
                  <span class="text-weight-medium">{{ w.full_name }}</span>
                  <span class="wk-pos">{{ w.position_ko }}</span>
                </div>
                <div v-if="!col.workers.length" class="text-grey-5 text-caption q-pa-sm text-center">비어 있음</div>
              </div>
            </div>
          </div>
        </q-card-section>
      </q-card>
    </q-dialog>

    <!-- drag ghost -->
    <div v-show="dragging" id="team-drag-ghost" class="drag-ghost"
      :style="{ left: ghost.x + 'px', top: ghost.y + 'px', display: ghost.show ? 'block' : 'none' }">{{ dragLabel }}</div>

    <!-- 어르신 → 팀 배정 -->
    <q-dialog v-model="showAssignResidents">
      <q-card style="min-width: 480px">
        <q-card-section class="text-h6">어르신 → 팀 배정</q-card-section>
        <q-card-section class="q-gutter-md">
          <q-select v-model="assignTeamId" emit-value map-options outlined dense label="대상 팀"
            :options="teams.map((t) => ({ label: `${t.name} · ${teamTypeLabel[t.team_type]}`, value: t.id }))" />
          <div>
            <div class="text-caption text-grey-7 q-mb-xs">호실 범위 (비우면 전체)</div>
            <div class="row q-gutter-sm">
              <q-input v-model="roomFrom" label="시작" type="number" outlined dense class="col" />
              <q-input v-model="roomTo" label="끝" type="number" outlined dense class="col" />
            </div>
          </div>
          <q-toggle v-model="unassignedOnly" label="미배정 어르신만" dense />
          <q-banner dense class="bg-blue-1 text-blue-9 rounded-borders">
            <q-icon name="o_groups" class="q-mr-xs" />{{ matchedResidents.length }}명이 배정됩니다.
          </q-banner>
          <q-list v-if="matchedResidents.length" bordered dense class="rounded-borders" style="max-height: 200px; overflow:auto">
            <q-item v-for="r in matchedResidents.slice(0, 100)" :key="r.id">
              <q-item-section>{{ r.full_name }}</q-item-section>
              <q-item-section side class="text-grey-7">{{ r.room_number ?? "—" }}호 · {{ r.team_name ?? "미배정" }}</q-item-section>
            </q-item>
          </q-list>
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="취소" v-close-popup />
          <q-btn unelevated color="primary" :label="`${matchedResidents.length}명 배정`"
            :disable="!matchedResidents.length" :loading="assigningResidents" @click="assignResidents" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- 팀 추가/수정 -->
    <q-dialog v-model="showTeamDialog">
      <q-card style="min-width: 340px">
        <q-card-section class="text-h6">{{ editingId ? "팀 수정" : "팀 추가" }}</q-card-section>
        <q-card-section class="q-gutter-md">
          <q-input v-model="form.name" label="팀 이름" outlined dense autofocus hint="예: 요양1팀 · 주간1팀 · 방문1팀" />
          <q-select v-model="form.team_type" :options="TEAM_TYPE_OPTIONS" label="유형" outlined dense emit-value map-options
            @update:model-value="onTypeChange" hint="요양=24시간 · 주간=주간만 · 방문=가변(custom)" />
          <div class="row q-gutter-sm">
            <q-input v-model="form.shift_start_hm" label="근무 시작" outlined dense class="col" hint="HH:MM" mask="##:##" />
            <q-input v-model="form.shift_end_hm" label="근무 종료" outlined dense class="col" hint="HH:MM" mask="##:##" />
          </div>
          <div>
            <div class="text-caption text-grey-7 q-mb-xs">색상</div>
            <div class="row q-gutter-sm">
              <div v-for="h in HUE_PRESETS" :key="h" class="hue-dot" :class="{ 'hue-on': form.color_hue === h }"
                :style="{ background: `hsl(${h} 60% 55%)` }" @click="form.color_hue = h" />
            </div>
          </div>
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="취소" v-close-popup />
          <q-btn unelevated color="primary" label="저장" @click="saveTeam" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<style scoped>
.team-card { overflow: hidden; }
.team-bar { height: 6px; }
.hue-dot { width: 28px; height: 28px; border-radius: 50%; cursor: pointer; border: 2px solid transparent; }
.hue-on { border-color: #1976d2; box-shadow: 0 0 0 2px white inset; }
.hidden { display: none; }

.board { display: flex; gap: 12px; overflow-x: auto; padding-bottom: 8px; }
.board-col { flex: 0 0 220px; background: #f7f9fb; border: 1px solid #e3e7ec; border-radius: 8px; display: flex; flex-direction: column; max-height: calc(100vh - 220px); }
.board-col.col-over { border-color: #1976d2; background: #e8f3ff; }
.board-col.col-focus { box-shadow: 0 0 0 2px #1976d2 inset; }
.board-col-head { padding: 8px 10px; border-bottom: 1px solid #e3e7ec; position: sticky; top: 0; background: inherit; }
.board-col-body { padding: 8px; overflow-y: auto; flex: 1; min-height: 60px; }
.wk-chip { display: flex; align-items: center; background: #fff; border: 1px solid #e0e0e0; border-radius: 6px; padding: 6px 8px; margin-bottom: 6px; cursor: grab; touch-action: none; user-select: none; }
.wk-chip.disabled { cursor: default; }
.wk-pos { margin-left: auto; font-size: 11px; color: #8a94a0; }
.drag-ghost { position: fixed; z-index: 9999; background: #1976d2; color: #fff; padding: 4px 10px; border-radius: 6px; font-size: 13px; pointer-events: none; box-shadow: 0 4px 12px rgba(0,0,0,.25); }
</style>
