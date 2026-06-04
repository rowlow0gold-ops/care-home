<script setup lang="ts">
// ── 스케쥴러 — 북미식 2주(Biweekly) 스케줄 ────────────────────────────────────
// 북미 요양원은 스케줄을 2주 단위로 발행한다. 직원은 근무조가 고정:
//   · 요양 12시간조 — Day 07:00–19:30 / Night 19:00–07:30, 3일 연속 근무·4일 휴무
//   · 요양 8시간 교대조 — 고정 쉬프트(주간/오후/야간), 주 5일
//   · 요양 알바조 — 지정 요일에만 (예: 일요일만)
// '발행'은 위 규칙으로 2주치 근무를 계산해 서버에 확정 저장(solid data)한다.
// 인원 매칭: 입소(요양) 어르신 실시간 수 기준 1:10 — 매 시간 커버리지 검사.
import { ref, computed, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type OrgPerson, type RosterEntry, type UpsertRoster } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const router = useRouter();
const session = useServerSessionStore();
const canPublish = computed(() => session.canEdit);

// ── Date helpers ──────────────────────────────────────────────────────────────
function localDateStr(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}
function addDays(d: Date, n: number): Date {
  const c = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  c.setDate(c.getDate() + n);
  return c;
}
const DAY_KO = ["일", "월", "화", "수", "목", "금", "토"];

// 2주 기간은 달력에 고정 — 기준 일요일(2026-01-04)에서 14일 단위로 나눈다.
const EPOCH = new Date(2026, 0, 4);
function periodStartOf(d: Date): Date {
  const days = Math.floor((new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() - EPOCH.getTime()) / 86400000);
  const idx = Math.floor(days / 14);
  return addDays(EPOCH, idx * 14);
}

// ── 근무 블록 정의 ────────────────────────────────────────────────────────────
interface Block { key: string; group: string; shift: string; label: string; short: string; start: string; end: string; hours: number; color: string; text: string }
const BLOCKS: Block[] = [
  { key: "d12", group: "h12", shift: "day",     label: "12시간 Day 07:00–19:30",   short: "12D", start: "07:00", end: "19:30", hours: 12.5, color: "amber-2",  text: "amber-10" },
  { key: "n12", group: "h12", shift: "night",   label: "12시간 Night 19:00–07:30", short: "12N", start: "19:00", end: "07:30", hours: 12.5, color: "indigo-2", text: "indigo-10" },
  { key: "d8",  group: "h8",  shift: "day",     label: "8시간 주간 07:00–15:00",   short: "8주", start: "07:00", end: "15:00", hours: 8,    color: "green-2",  text: "green-10" },
  { key: "e8",  group: "h8",  shift: "evening", label: "8시간 오후 15:00–23:00",   short: "8오", start: "15:00", end: "23:00", hours: 8,    color: "orange-2", text: "orange-10" },
  { key: "n8",  group: "h8",  shift: "night",   label: "8시간 야간 23:00–07:00",   short: "8야", start: "23:00", end: "07:00", hours: 8,    color: "blue-2",   text: "blue-10" },
];
function blockFor(group: string, shift: string): Block {
  // 알바조는 8시간 블록을 쓴다
  const g = group === "pt" ? "h8" : group;
  return BLOCKS.find((b) => b.group === g && b.shift === shift) ?? BLOCKS[2];
}
function classify(e: RosterEntry): Block {
  const b = BLOCKS.find((x) => x.start === e.shift_start && x.end === e.shift_end);
  if (b) return b;
  const sh = Number(e.shift_start.slice(0, 2));
  if (e.shift_hours >= 12) return sh < 12 ? BLOCKS[0] : BLOCKS[1];
  return sh < 12 ? BLOCKS[2] : sh < 20 ? BLOCKS[3] : BLOCKS[4];
}

// ── Data ──────────────────────────────────────────────────────────────────────
const loading = ref(false);
const caregivers = ref<OrgPerson[]>([]);   // 요양보호사만 — 요양(입소) 스케줄 대상
const elders = ref(0);                     // 입소(residential) 어르신 — 실시간
const entries = ref<RosterEntry[]>([]);
const periodStart = ref<Date>(periodStartOf(new Date()));
const todayStr = localDateStr(new Date());

const periodDays = computed(() => Array.from({ length: 14 }, (_, i) => addDays(periodStart.value, i)));
const periodEnd = computed(() => addDays(periodStart.value, 13));
const periodLabel = computed(() => {
  const a = periodStart.value, b = periodEnd.value;
  return `${a.getFullYear()}.${a.getMonth() + 1}.${a.getDate()} ~ ${b.getMonth() + 1}.${b.getDate()}`;
});
const weeks = computed(() => [periodDays.value.slice(0, 7), periodDays.value.slice(7, 14)]);

async function loadPeople() {
  const all: OrgPerson[] = [];
  let page = 1, total = Infinity;
  while (all.length < total && page <= 5) {
    const res = await server.orgPaged({ page, page_size: 200 });
    total = res.total;
    all.push(...res.items);
    if (!res.items.length) break;
    page++;
  }
  caregivers.value = all.filter((p) => !p.is_inactive && p.position === "caregiver");
}
async function loadElders() {
  const res = await server.residentsPaged({ care_type: "residential", status: "active", page: 1, page_size: 1 });
  elders.value = res.total;
}
async function loadEntries() {
  entries.value = await server.roster(localDateStr(periodStart.value), localDateStr(periodEnd.value));
}
async function loadAll() {
  loading.value = true;
  try {
    await Promise.all([loadPeople(), loadElders(), loadEntries()]);
  } catch (e: any) {
    $q.notify({ type: "negative", message: `불러오기 실패: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}
watch(periodStart, () => { loadEntries().catch(() => {}); });

function prev() { periodStart.value = addDays(periodStart.value, -14); }
function next() { periodStart.value = addDays(periodStart.value, 14); }
function goToday() { periodStart.value = periodStartOf(new Date()); }

// ── 그룹 구성 요약 ────────────────────────────────────────────────────────────
const groupStats = computed(() => {
  const c = { h12d: 0, h12n: 0, h8: 0, pt: 0, none: 0 };
  for (const p of caregivers.value) {
    if (p.shift_group === "h12") (p.preferred_shift === "night" ? c.h12n++ : c.h12d++);
    else if (p.shift_group === "h8") c.h8++;
    else if (p.shift_group === "pt") c.pt++;
    else c.none++;
  }
  return c;
});
const unassigned = computed(() => caregivers.value.filter((p) => !p.shift_group));

// ── 필요 인원 (실시간 입소 어르신 1:10, 최소 2명) ────────────────────────────
const required = computed(() => Math.max(2, Math.ceil(Math.max(elders.value, 1) / 10)));

// ── 표시용: 날짜별 블록별 명단 ───────────────────────────────────────────────
const byDate = computed<Map<string, Map<string, RosterEntry[]>>>(() => {
  const m = new Map<string, Map<string, RosterEntry[]>>();
  for (const e of entries.value) {
    const b = classify(e);
    if (!m.has(e.shift_date)) m.set(e.shift_date, new Map());
    const dm = m.get(e.shift_date)!;
    if (!dm.has(b.key)) dm.set(b.key, []);
    dm.get(b.key)!.push(e);
  }
  return m;
});
function blockCount(ds: string, key: string): number { return byDate.value.get(ds)?.get(key)?.length ?? 0; }
function dayTotal(ds: string): number {
  let n = 0; byDate.value.get(ds)?.forEach((v) => { n += v.length; }); return n;
}

// ── 시간 커버리지 매칭 — 매 시각 근무 인원 ≥ 필요 인원 ───────────────────────
function hourCover(ds: string): number[] {
  const hours = new Array(24).fill(0);
  const dm = byDate.value.get(ds);
  if (!dm) return hours;
  dm.forEach((list, key) => {
    const b = BLOCKS.find((x) => x.key === key)!;
    const s = Number(b.start.slice(0, 2));
    let e = Number(b.end.slice(0, 2));
    if (b.end.slice(3) === "30") e += 1; // 30분 꼬리는 다음 시간까지 커버로 취급
    const span = e > s ? e - s : e + 24 - s;
    for (let i = 0; i < span; i++) hours[(s + i) % 24] += list.length;
  });
  // 자정 넘는 전날 야간분은 전날 entry가 커버 — 단순화를 위해 당일 기준으로만 계산
  return hours;
}
interface Gap { ds: string; label: string; range: string; have: number }
const gaps = computed<Gap[]>(() => {
  const out: Gap[] = [];
  for (const d of periodDays.value) {
    const ds = localDateStr(d);
    if (!dayTotal(ds)) continue; // 발행 전 날은 검사하지 않는다
    const cov = hourCover(ds);
    let runStart = -1, runMin = Infinity;
    for (let h = 0; h <= 24; h++) {
      const bad = h < 24 && cov[h] < required.value;
      if (bad) { if (runStart < 0) { runStart = h; runMin = cov[h]; } else runMin = Math.min(runMin, cov[h]); }
      else if (runStart >= 0) {
        out.push({ ds, label: `${d.getMonth() + 1}/${d.getDate()}(${DAY_KO[d.getDay()]})`, range: `${String(runStart).padStart(2, "0")}–${String(h).padStart(2, "0")}시`, have: runMin });
        runStart = -1;
      }
    }
  }
  return out;
});
const published = computed(() => entries.value.length > 0);

// ── 2주 스케줄 발행 — 근무조 규칙으로 계산해 서버에 확정 저장 ────────────────
const publishing = ref(false);
async function collectLeave(dates: string[]): Promise<Set<string>> {
  const set = new Set<string>();
  try {
    let p = 1, fetched = 0, total = Infinity;
    while (fetched < total && p <= 10) {
      const res = await server.leaveRequestsPaged({ status: "approved", page: p, page_size: 100 });
      total = res.total;
      for (const lr of res.items) for (const ds of dates) if (ds >= lr.start_date && ds <= lr.end_date) set.add(`${lr.user_id}-${ds}`);
      fetched += res.items.length;
      if (!res.items.length) break;
      p++;
    }
  } catch { /* 휴가 조회 실패 시 제외 없이 발행 */ }
  return set;
}

// 균등 배치 — 시설은 매일 안정적으로 돌아가야 한다 (어르신 케어 비율 유지).
// 직원마다 '근무 창(window)'을 골라 준다:
//   12시간조 = 연속 3일 창(이후 4일 휴무) · 8시간조 = 연속 2일 휴무 창(주 5일 근무)
// 창은 (1) 휴가와 겹쳐 근무일 손실이 가장 적고 (2) 그날 전체 인원이 가장 적은
// 날을 채우는 쪽으로 고른다 → 휴가가 금·토에 60명 몰려도 일별 총원이 평탄해진다.
// 같은 창을 2주 내내 유지해 개인 리듬(3 on 4 off)이 고정된다.
function assignWindows(
  members: OrgPerson[], group: "h12" | "h8", weeks: string[][], onLeave: Set<string>,
  counts: Map<string, number>, assigned: Map<string, OrgPerson[]>,
) {
  const avail = (p: OrgPerson, ds: string) => !onLeave.has(`${p.id}-${ds}`);
  for (const p of members) {
    let best: string[] | null = null;
    let bestWorked = -1, bestLoad = Infinity;
    for (let s0 = 0; s0 < 7; s0++) {
      const days: string[] = [];
      for (const week of weeks) {
        if (group === "h12") {
          for (let k = 0; k < 3; k++) days.push(week[(s0 + k) % 7]);
        } else {
          for (let d = 0; d < 7; d++) if (d !== s0 && d !== (s0 + 1) % 7) days.push(week[d]);
        }
      }
      const workable = days.filter((ds) => avail(p, ds));
      const load = workable.reduce((sum, ds) => sum + (counts.get(ds) ?? 0), 0) / Math.max(1, workable.length);
      if (workable.length > bestWorked || (workable.length === bestWorked && load < bestLoad)) {
        best = workable; bestWorked = workable.length; bestLoad = load;
      }
    }
    for (const ds of best ?? []) {
      counts.set(ds, (counts.get(ds) ?? 0) + 1);
      if (!assigned.has(ds)) assigned.set(ds, []);
      assigned.get(ds)!.push(p);
    }
  }
}

function buildEntries(onLeave: Set<string>): UpsertRoster[] {
  const out: UpsertRoster[] = [];
  const dates = periodDays.value.map(localDateStr);
  // 고정 조 단위(블록별)로 주마다 균등 배치
  const SUBGROUPS: Array<{ group: string; shift: string; note: string }> = [
    { group: "h12", shift: "day",     note: "12시간조 Day (3일 근무·4일 휴무)" },
    { group: "h12", shift: "night",   note: "12시간조 Night (3일 근무·4일 휴무)" },
    { group: "h8",  shift: "day",     note: "8시간조 주간 (주 5일)" },
    { group: "h8",  shift: "evening", note: "8시간조 오후 (주 5일)" },
    { group: "h8",  shift: "night",   note: "8시간조 야간 (주 5일)" },
  ];
  const weeks = [dates.slice(0, 7), dates.slice(7, 14)];
  const counts = new Map<string, number>(); // 일별 총원 — 조를 가로질러 공유해 평탄화
  for (const sg of SUBGROUPS) {
    const members = caregivers.value.filter((p) =>
      p.shift_group === sg.group &&
      (sg.group === "h12"
        ? (p.preferred_shift === "night" ? "night" : "day") === sg.shift
        : (p.preferred_shift ?? "day") === sg.shift));
    if (!members.length) continue;
    const b = blockFor(sg.group, sg.shift);
    const assigned = new Map<string, OrgPerson[]>();
    assignWindows(members, sg.group as "h12" | "h8", weeks, onLeave, counts, assigned);
    assigned.forEach((list, ds) => {
      for (const p of list) {
        out.push({ user_id: p.id, shift_date: ds, shift_start: b.start, shift_end: b.end, shift_hours: b.hours, notes: sg.note });
      }
    });
  }
  // 알바조 — 지정 요일 계약은 그대로 (옮길 수 없다)
  periodDays.value.forEach((d, di) => {
    const ds = dates[di];
    for (const p of caregivers.value) {
      if (p.shift_group !== "pt" || onLeave.has(`${p.id}-${ds}`)) continue;
      if (!p.work_days?.includes(d.getDay())) continue;
      const b = blockFor("pt", p.preferred_shift ?? "day");
      out.push({ user_id: p.id, shift_date: ds, shift_start: b.start, shift_end: b.end, shift_hours: b.hours, notes: "알바조 (지정 요일)" });
    }
  });
  return out;
}

async function publish() {
  if (!caregivers.value.some((p) => p.shift_group)) {
    $q.notify({ type: "warning", message: "근무조가 지정된 요양보호사가 없습니다. 직원 페이지에서 먼저 지정하세요." });
    return;
  }
  const doIt = async () => {
    publishing.value = true;
    try {
      const onLeave = await collectLeave(periodDays.value.map(localDateStr));
      const list = buildEntries(onLeave);
      const res = await server.rosterBulk(localDateStr(periodStart.value), localDateStr(periodEnd.value), list);
      await loadEntries();
      $q.notify({ type: "positive", timeout: 6000,
        message: `2주 스케줄 발행 완료 — ${res.inserted}건 확정${res.deleted ? ` (기존 ${res.deleted}건 교체)` : ""}`,
        caption: gaps.value.length ? `⚠ 인력 부족 ${gaps.value.length}건 — 아래 알림을 확인하세요.` : "모든 시간대 인원 충족." });
    } catch (e: any) {
      $q.notify({ type: "negative", message: `발행 실패: ${e?.message ?? e}` });
    } finally {
      publishing.value = false;
    }
  };
  if (published.value) {
    $q.dialog({
      title: "스케줄 재발행",
      message: `이 기간에 이미 발행된 근무 ${entries.value.length}건이 있습니다. 새 스케줄로 교체할까요?`,
      ok: { label: "교체 발행", color: "negative", unelevated: true },
      cancel: { label: "취소", flat: true },
      persistent: true,
    }).onOk(doIt);
  } else {
    await doIt();
  }
}

// ── 일자 상세 다이얼로그 ──────────────────────────────────────────────────────
const dayDialog = ref(false);
const dayDs = ref("");
const dayLabel = ref("");
function openDay(d: Date) {
  dayDs.value = localDateStr(d);
  dayLabel.value = `${d.getMonth() + 1}월 ${d.getDate()}일 (${DAY_KO[d.getDay()]})`;
  dayDialog.value = true;
}
function dayBlocks(ds: string): Array<{ block: Block; list: RosterEntry[] }> {
  const dm = byDate.value.get(ds);
  if (!dm) return [];
  return BLOCKS.filter((b) => dm.has(b.key)).map((b) => ({
    block: b,
    list: [...dm.get(b.key)!].sort((a, c) => a.staff_name.localeCompare(c.staff_name, "ko")),
  }));
}

function goStaffPage() { router.push("/staff"); }
onMounted(loadAll);
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">스케쥴러 — 2주 근무표</div>
        <div class="text-caption text-grey-6">
          북미식 2주 단위 발행 · 12시간조(3일 근무·4일 휴무) / 8시간 교대조(주 5일) / 알바조(지정 요일)
        </div>
      </div>
      <div class="col-auto row items-center q-gutter-sm">
        <q-btn outline dense color="primary" label="이번 기간" @click="goToday" />
        <q-btn flat round dense icon="o_chevron_left" @click="prev" />
        <div class="text-subtitle1 text-weight-medium" style="min-width: 175px; text-align: center">{{ periodLabel }}</div>
        <q-btn flat round dense icon="o_chevron_right" @click="next" />
        <q-btn v-if="canPublish" color="primary" unelevated icon="o_publish" :loading="publishing"
          :label="published ? '재발행' : '2주 스케줄 발행'" @click="publish">
          <q-tooltip>근무조 규칙으로 2주치 근무를 계산해 확정 저장합니다. 승인된 휴가는 제외.</q-tooltip>
        </q-btn>
      </div>
    </div>

    <!-- 실시간 매칭 요약 -->
    <div class="row q-col-gutter-sm q-mb-md">
      <div class="col-12 col-md-7">
        <q-banner dense rounded class="bg-blue-1 text-blue-10 full-height">
          <template #avatar><q-icon name="o_diversity_3" /></template>
          입소 어르신 <b>{{ elders }}명</b> (실시간) → 매 시간 요양보호사 <b>{{ required }}명</b> 필요 (1:10 · 최소 2명)<br />
          <span class="text-caption">
            조 구성: 12시간조 {{ groupStats.h12d + groupStats.h12n }}명 (Day {{ groupStats.h12d }} / Night {{ groupStats.h12n }})
            · 8시간조 {{ groupStats.h8 }}명 · 알바조 {{ groupStats.pt }}명
            <template v-if="groupStats.none"> · <span class="text-red-8">미지정 {{ groupStats.none }}명</span></template>
          </span>
        </q-banner>
      </div>
      <div class="col-12 col-md-5">
        <q-banner v-if="!published" dense rounded class="bg-grey-2 text-grey-8 full-height">
          <template #avatar><q-icon name="o_pending_actions" /></template>
          이 기간은 아직 발행 전입니다. ‘2주 스케줄 발행’으로 근무표를 확정하세요.
        </q-banner>
        <q-banner v-else-if="gaps.length" dense rounded class="bg-red-1 text-red-10 full-height">
          <template #avatar><q-icon name="o_warning" /></template>
          <b>인력 부족 {{ gaps.length }}건</b> —
          <span class="text-caption">
            {{ gaps.slice(0, 3).map((g) => `${g.label} ${g.range} ${g.have}/${required}명`).join(", ") }}<template v-if="gaps.length > 3"> 외 {{ gaps.length - 3 }}건</template>
          </span>
        </q-banner>
        <q-banner v-else dense rounded class="bg-green-1 text-green-10 full-height">
          <template #avatar><q-icon name="o_check_circle" /></template>
          발행 완료 — 2주간 모든 시간대가 필요 인원을 충족합니다.
        </q-banner>
      </div>
    </div>

    <!-- 미지정 인력 경고 -->
    <q-banner v-if="unassigned.length" dense rounded class="bg-orange-1 text-orange-10 q-mb-md">
      <template #avatar><q-icon name="o_tune" /></template>
      근무조 미지정 요양보호사 <b>{{ unassigned.length }}명</b>은 스케줄에 오르지 않습니다.
      <template #action>
        <q-btn flat dense color="orange-10" label="직원 페이지에서 지정" @click="goStaffPage" />
      </template>
    </q-banner>

    <!-- 2주 그리드 -->
    <q-card flat bordered>
      <div class="grid7 grid-head">
        <div v-for="(d, i) in DAY_KO" :key="d" class="text-center text-caption text-weight-medium q-py-xs"
          :class="i === 0 ? 'text-red' : i === 6 ? 'text-blue' : 'text-grey-7'">{{ d }}</div>
      </div>
      <div v-for="(week, wi) in weeks" :key="wi" class="grid7">
        <div v-for="d in week" :key="localDateStr(d)" class="day-cell cursor-pointer"
          :class="localDateStr(d) === todayStr ? 'today-cell' : ''" @click="openDay(d)">
          <div class="row items-center q-mb-xs">
            <span class="text-caption text-weight-bold"
              :class="d.getDay() === 0 ? 'text-red' : d.getDay() === 6 ? 'text-blue' : ''">
              {{ d.getMonth() + 1 }}/{{ d.getDate() }}
            </span>
            <q-space />
            <q-badge v-if="dayTotal(localDateStr(d))" color="grey-3" text-color="grey-8">{{ dayTotal(localDateStr(d)) }}명</q-badge>
          </div>
          <template v-if="dayTotal(localDateStr(d))">
            <div v-for="b in BLOCKS" :key="b.key">
              <q-badge v-if="blockCount(localDateStr(d), b.key)" :color="b.color" :text-color="b.text" class="q-mb-xs q-mr-xs">
                {{ b.short }} {{ blockCount(localDateStr(d), b.key) }}
              </q-badge>
            </div>
            <q-icon v-if="gaps.some((g) => g.ds === localDateStr(d))" name="o_warning" color="red" size="16px">
              <q-tooltip>이 날 일부 시간대 인원 부족</q-tooltip>
            </q-icon>
          </template>
          <div v-else class="text-caption text-grey-4 q-mt-sm text-center">발행 전</div>
        </div>
      </div>
    </q-card>

    <!-- 범례 -->
    <div class="row q-gutter-xs q-mt-sm items-center">
      <span class="text-caption text-grey-6 q-mr-sm">범례:</span>
      <q-badge v-for="b in BLOCKS" :key="b.key" :color="b.color" :text-color="b.text">{{ b.short }} = {{ b.label }}</q-badge>
    </div>

    <!-- 일자 상세 -->
    <q-dialog v-model="dayDialog">
      <q-card style="min-width: 520px; max-height: 80vh" class="scroll">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">{{ dayLabel }} — 근무 {{ dayTotal(dayDs) }}명</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section class="q-gutter-md">
          <div v-for="{ block, list } in dayBlocks(dayDs)" :key="block.key">
            <div class="row items-center q-mb-xs">
              <q-badge :color="block.color" :text-color="block.text">{{ block.label }}</q-badge>
              <span class="text-caption text-grey-6 q-ml-sm">{{ list.length }}명</span>
            </div>
            <div class="row q-gutter-xs">
              <q-chip v-for="e in list" :key="e.id" dense size="sm" color="grey-2" text-color="grey-9">
                {{ e.staff_name }}
              </q-chip>
            </div>
          </div>
          <div v-if="!dayBlocks(dayDs).length" class="text-grey-5 text-center q-py-lg">발행된 근무가 없습니다</div>
        </q-card-section>
      </q-card>
    </q-dialog>

    <q-inner-loading :showing="loading"><q-spinner size="40px" color="primary" /></q-inner-loading>
  </q-page>
</template>

<style scoped>
.grid7 { display: grid; grid-template-columns: repeat(7, 1fr); }
.grid-head { border-bottom: 1px solid #e2e8f0; }
.day-cell { min-height: 104px; border: 1px solid #f1f5f9; padding: 6px; }
.day-cell:hover { background: #f0f9ff; }
.today-cell { outline: 2px solid var(--q-primary); outline-offset: -2px; }
</style>
