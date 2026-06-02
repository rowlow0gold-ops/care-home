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
const saving = ref<string | null>(null);
// 요양보호사·간호사만 배정 대상으로 본다 (돌봄 인력).
const careRoles = new Set(["caregiver", "nurse"]);
const onlyCare = ref(true);

const careStaff = computed(() =>
  staff.value.filter((s) => !s.is_inactive && (!onlyCare.value || careRoles.has(s.role))),
);

const teamOptions = computed(() => [
  { label: "미배정", value: null as string | null },
  ...teams.value.map((t) => ({ label: t.name, value: t.id })),
]);

function workersOf(teamId: string) {
  return staff.value.filter((s) => !s.is_inactive && careRoles.has(s.role) && s.team_id === teamId).length;
}
function residentsOf(teamId: string) {
  return residents.value.filter((r) => r.status === "active" && r.team_id === teamId).length;
}
/** 어르신 : 돌봄인력 비율. 10을 초과하면 경고. */
function ratioOf(teamId: string): { text: string; warn: boolean } {
  const w = workersOf(teamId);
  const r = residentsOf(teamId);
  if (w === 0) return { text: r > 0 ? "인력 없음" : "—", warn: r > 0 };
  const per = r / w;
  return { text: `1 : ${per.toFixed(1)}`, warn: per > 10 };
}
const unassignedCount = computed(
  () => careStaff.value.filter((s) => !s.team_id).length,
);

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

async function reassign(person: OrgPerson, teamId: string | null) {
  if (person.team_id === teamId) return;
  saving.value = person.id;
  try {
    await server.assignTeam(person.id, teamId);
    person.team_id = teamId;
    person.team_name = teams.value.find((t) => t.id === teamId)?.name ?? null;
    $q.notify({ type: "positive", message: `${person.full_name} → ${person.team_name ?? "미배정"}` });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `배정 실패: ${e?.message ?? e}` });
  } finally {
    saving.value = null;
  }
}

// ── 팀 추가 / 수정 ──────────────────────────────────────────────────────────
const showTeamDialog = ref(false);
const editingId = ref<string | null>(null);
const form = ref({ name: "", shift_start_hm: "06:00", shift_end_hm: "18:00", color_hue: 210 });
const HUE_PRESETS = [210, 260, 150, 30, 340, 110];

function openNew() {
  editingId.value = null;
  form.value = { name: "", shift_start_hm: "06:00", shift_end_hm: "18:00", color_hue: 210 };
  showTeamDialog.value = true;
}
function openEdit(t: Team) {
  editingId.value = t.id;
  form.value = { name: t.name, shift_start_hm: t.shift_start_hm, shift_end_hm: t.shift_end_hm, color_hue: t.color_hue };
  showTeamDialog.value = true;
}
async function saveTeam() {
  const f = form.value;
  if (!f.name.trim() || !/^\d{2}:\d{2}$/.test(f.shift_start_hm) || !/^\d{2}:\d{2}$/.test(f.shift_end_hm)) {
    $q.notify({ type: "negative", message: "이름·근무 시작·종료(HH:MM)를 입력하세요." });
    return;
  }
  try {
    if (editingId.value) {
      await server.updateTeam(editingId.value, { ...f, name: f.name.trim() });
    } else {
      await server.createTeam({ ...f, name: f.name.trim(), sort_order: teams.value.length + 1 });
    }
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
    title: "조 삭제", message: `'${t.name}' 조를 삭제할까요?`,
    cancel: { label: "취소", flat: true }, ok: { label: "삭제", color: "negative", unelevated: true }, persistent: true,
  }).onOk(async () => {
    try { await server.deleteTeam(t.id); await load(); $q.notify({ type: "positive", message: "삭제되었습니다." }); }
    catch (e: any) { $q.notify({ type: "negative", message: `삭제 실패: ${e?.message ?? e}` }); }
  });
}

const roleKo: Record<string, string> = { caregiver: "요양보호사", nurse: "간호사", branch_manager: "시설장", hq: "본사" };

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-md q-gutter-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">조 관리</div>
        <div class="text-caption text-grey-6">돌봄 인력을 조로 나누고, 각 조가 담당할 어르신을 배정합니다</div>
      </div>
      <q-toggle v-model="onlyCare" label="돌봄 인력만" dense />
      <q-btn v-if="canEdit" unelevated color="primary" icon="o_add" label="조 추가" @click="openNew" />
      <q-btn flat round dense icon="o_refresh" :loading="loading" @click="load" />
    </div>

    <!-- 조 카드 -->
    <div class="row q-col-gutter-md q-mb-lg">
      <div v-for="t in teams" :key="t.id" class="col-12 col-sm-6 col-md-4 col-lg-3">
        <q-card flat bordered class="team-card">
          <div class="team-bar" :style="{ background: `hsl(${t.color_hue} 60% 55%)` }" />
          <q-card-section class="q-pb-xs">
            <div class="row items-center no-wrap">
              <div class="col text-subtitle1 text-weight-bold ellipsis">{{ t.name }}</div>
              <template v-if="canEdit">
                <q-btn flat round dense size="sm" icon="o_edit" @click="openEdit(t)" />
                <q-btn flat round dense size="sm" icon="o_delete" color="grey-6" @click="removeTeam(t)" />
              </template>
            </div>
            <div class="text-caption text-grey-6">근무 {{ t.shift_start_hm }} ~ {{ t.shift_end_hm }}</div>
          </q-card-section>
          <q-card-section class="row q-pt-none text-center">
            <div class="col"><div class="text-h6">{{ workersOf(t.id) }}</div><div class="text-caption text-grey-6">인력</div></div>
            <div class="col"><div class="text-h6">{{ residentsOf(t.id) }}</div><div class="text-caption text-grey-6">어르신</div></div>
            <div class="col">
              <q-chip dense :color="ratioOf(t.id).warn ? 'negative' : 'green-1'" :text-color="ratioOf(t.id).warn ? 'white' : 'green-9'"
                :icon="ratioOf(t.id).warn ? 'o_warning' : undefined" class="q-mt-xs">
                {{ ratioOf(t.id).text }}
              </q-chip>
              <div class="text-caption text-grey-6">비율</div>
            </div>
          </q-card-section>
        </q-card>
      </div>
      <div v-if="!teams.length && !loading" class="col-12 text-center text-grey-5 q-py-lg">조가 없습니다. ‘조 추가’로 만들어 주세요.</div>
    </div>

    <!-- 인력 배정 -->
    <div class="row items-center q-mb-sm">
      <div class="text-subtitle1 text-weight-bold col">인력 배정</div>
      <q-chip v-if="unassignedCount" dense color="orange-1" text-color="orange-9" icon="o_person_off">미배정 {{ unassignedCount }}명</q-chip>
    </div>
    <q-table :rows="careStaff" :columns="[
        { name: 'name', label: '이름', field: 'full_name', align: 'left' },
        { name: 'position', label: '직책', field: 'position_ko', align: 'left' },
        { name: 'role', label: '구분', field: 'role', align: 'left' },
        { name: 'team', label: '담당 조', field: 'team_id', align: 'left' },
      ] as any" row-key="id" flat bordered :loading="loading" :rows-per-page-options="[0]" hide-pagination>
      <template #body-cell-role="props">
        <q-td :props="props"><span class="text-grey-7">{{ roleKo[props.row.role] ?? props.row.role }}</span></q-td>
      </template>
      <template #body-cell-team="props">
        <q-td :props="props" style="min-width: 180px">
          <q-select v-if="canEdit" :model-value="props.row.team_id" :options="teamOptions" emit-value map-options
            dense outlined options-dense :loading="saving === props.row.id"
            @update:model-value="(v: string | null) => reassign(props.row, v)" />
          <q-chip v-else dense :color="props.row.team_id ? 'blue-1' : 'grey-2'" :text-color="props.row.team_id ? 'blue-9' : 'grey-7'">
            {{ props.row.team_name ?? '미배정' }}
          </q-chip>
        </q-td>
      </template>
      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_groups" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">표시할 인력이 없습니다</div>
        </div>
      </template>
    </q-table>

    <q-dialog v-model="showTeamDialog">
      <q-card style="min-width: 340px">
        <q-card-section class="text-h6">{{ editingId ? "조 수정" : "조 추가" }}</q-card-section>
        <q-card-section class="q-gutter-md">
          <q-input v-model="form.name" label="조 이름" outlined dense autofocus hint="예: 요양1팀 · 주간1팀 · 방문1팀" />
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
</style>
