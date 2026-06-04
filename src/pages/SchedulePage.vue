<script setup lang="ts">
// ── 스케쥴러 — 북미식 고정 쉬프트 보기 전용 ───────────────────────────────────
// 직원마다 근무 성향(선호 교대 + 가능 요일)이 고정되어 있으므로, 스케줄은
// 생성·편집하지 않고 성향에서 자동 계산해 보여만 준다 (수정 = 직원 페이지).
// 대신 매 교대의 필요 인원(1:10, 최소 2명)이 채워지는지 항상 매칭해 알린다.
// 승인된 휴가는 해당 날짜에서 자동 제외된다.
import { ref, computed, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type OrgPerson } from "@/lib/server";

const $q = useQuasar();
const router = useRouter();

// ── Date helpers ──────────────────────────────────────────────────────────────
function localDateStr(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}
// 주는 일요일 시작 — d가 속한 주의 일요일
function weekSunday(d: Date): Date {
  const c = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  c.setDate(c.getDate() - c.getDay());
  return c;
}
function addDays(d: Date, n: number): Date {
  const c = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  c.setDate(c.getDate() + n);
  return c;
}
const DAY_KO = ["일", "월", "화", "수", "목", "금", "토"];

// ── 고정 교대 블록 (북미 표준 8시간 3교대 — 24시간 커버) ────────────────────
const SHIFTS = [
  { key: "day",     name: "주간", start: "07:00", end: "15:00", color: "amber-1",  text: "amber-10",  icon: "o_light_mode" },
  { key: "evening", name: "오후", start: "15:00", end: "23:00", color: "orange-1", text: "orange-10", icon: "o_wb_twilight" },
  { key: "night",   name: "야간", start: "23:00", end: "07:00", color: "indigo-1", text: "indigo-10", icon: "o_dark_mode" },
] as const;
type ShiftKey = (typeof SHIFTS)[number]["key"];

// ── Data ──────────────────────────────────────────────────────────────────────
const loading = ref(false);
const people = ref<OrgPerson[]>([]);
const elders = ref(0);

// 현장 인력만 1:10 매칭 대상 (관리·지원 직군 제외)
const CARE_POSITIONS = new Set(["caregiver"]);
const FIELD_POSITIONS = new Set(["caregiver", "nurse_rn", "nurse_assistant"]);

async function load() {
  loading.value = true;
  try {
    const [org, res] = await Promise.all([
      server.orgPaged({ page: 1, page_size: 500 }),
      server.residentsPaged({ page: 1, page_size: 1, status: "active" }),
    ]);
    people.value = org.items.filter((p) => !p.is_inactive);
    elders.value = res.total;
  } catch (e: any) {
    $q.notify({ type: "negative", message: `데이터를 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}

// ── 승인 휴가 — 보이는 기간에서 자동 제외 ────────────────────────────────────
const onLeave = ref<Set<string>>(new Set()); // `${userId}-${ds}`
async function loadLeave(dates: string[]) {
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
  } catch { /* 휴가 조회 실패 시 제외 없이 표시 */ }
  onLeave.value = set;
}

// ── View state ────────────────────────────────────────────────────────────────
const viewMode = ref<"week" | "month">("week");
const currentSunday = ref<Date>(weekSunday(new Date()));
const currentMonth = ref<Date>(new Date(new Date().getFullYear(), new Date().getMonth(), 1));
const todayStr = localDateStr(new Date());

const weekDays = computed(() => Array.from({ length: 7 }, (_, i) => addDays(currentSunday.value, i)));
const weekLabel = computed(() => {
  const a = weekDays.value[0], b = weekDays.value[6];
  return `${a.getFullYear()}.${a.getMonth() + 1}.${a.getDate()} ~ ${b.getMonth() + 1}.${b.getDate()}`;
});
const monthLabel = computed(() => currentMonth.value.toLocaleDateString("ko-KR", { year: "numeric", month: "long" }));
const monthGrid = computed<Date[][]>(() => {
  const first = new Date(currentMonth.value.getFullYear(), currentMonth.value.getMonth(), 1);
  const start = weekSunday(first);
  const weeks: Date[][] = [];
  let cur = new Date(start.getTime());
  for (let w = 0; w < 6; w++) {
    const row: Date[] = [];
    for (let i = 0; i < 7; i++) { row.push(new Date(cur.getTime())); cur = addDays(cur, 1); }
    weeks.push(row);
    if (cur.getMonth() !== currentMonth.value.getMonth() && w >= 3) break;
  }
  return weeks;
});

function goToday() {
  currentSunday.value = weekSunday(new Date());
  currentMonth.value = new Date(new Date().getFullYear(), new Date().getMonth(), 1);
}
function prev() {
  if (viewMode.value === "week") currentSunday.value = addDays(currentSunday.value, -7);
  else currentMonth.value = new Date(currentMonth.value.getFullYear(), currentMonth.value.getMonth() - 1, 1);
}
function next() {
  if (viewMode.value === "week") currentSunday.value = addDays(currentSunday.value, 7);
  else currentMonth.value = new Date(currentMonth.value.getFullYear(), currentMonth.value.getMonth() + 1, 1);
}
function openDay(d: Date) { // 월 보기에서 날짜 클릭 → 해당 주로
  currentSunday.value = weekSunday(d);
  viewMode.value = "week";
}

// 보이는 날짜들 — 휴가 로드 트리거
const visibleDates = computed<string[]>(() =>
  viewMode.value === "week"
    ? weekDays.value.map(localDateStr)
    : monthGrid.value.flat().map(localDateStr),
);
watch(visibleDates, (ds) => { loadLeave(ds); });

// ── 고정 스케줄 계산 ──────────────────────────────────────────────────────────
// 직원의 근무 여부: 선호 교대 + 가능 요일(work_days, null=매일) + 휴가 제외
function worksOn(p: OrgPerson, d: Date, ds: string): ShiftKey | null {
  if (!p.preferred_shift) return null;
  if (p.work_days?.length && !p.work_days.includes(d.getDay())) return null;
  if (onLeave.value.has(`${p.id}-${ds}`)) return null;
  return p.preferred_shift as ShiftKey;
}
// 날짜별 교대 명단 (휴가 제외 반영)
const rosterByDate = computed<Map<string, Record<ShiftKey, OrgPerson[]>>>(() => {
  const map = new Map<string, Record<ShiftKey, OrgPerson[]>>();
  const dates = viewMode.value === "week" ? weekDays.value : monthGrid.value.flat();
  for (const d of dates) {
    const ds = localDateStr(d);
    const rec: Record<ShiftKey, OrgPerson[]> = { day: [], evening: [], night: [] };
    for (const p of people.value) {
      const sk = worksOn(p, d, ds);
      if (sk) rec[sk].push(p);
    }
    (Object.keys(rec) as ShiftKey[]).forEach((k) => rec[k].sort((a, b) => a.full_name.localeCompare(b.full_name, "ko")));
    map.set(ds, rec);
  }
  return map;
});
function shiftList(d: Date, key: ShiftKey): OrgPerson[] {
  return rosterByDate.value.get(localDateStr(d))?.[key] ?? [];
}

// ── 필요 인원 매칭 (1:10 + 최소 2명) ─────────────────────────────────────────
const required = computed(() => Math.max(2, Math.ceil(Math.max(elders.value, 1) / 10)));
function careCount(list: OrgPerson[]): number {
  return list.filter((p) => CARE_POSITIONS.has(p.position)).length;
}
interface Gap { ds: string; label: string; shift: string; have: number }
const coverageGaps = computed<Gap[]>(() => {
  const gaps: Gap[] = [];
  const dates = viewMode.value === "week" ? weekDays.value : monthGrid.value.flat();
  for (const d of dates) {
    if (viewMode.value === "month" && d.getMonth() !== currentMonth.value.getMonth()) continue;
    const ds = localDateStr(d);
    const rec = rosterByDate.value.get(ds);
    if (!rec) continue;
    for (const sh of SHIFTS) {
      const have = careCount(rec[sh.key]);
      if (have < required.value) {
        gaps.push({ ds, label: `${d.getMonth() + 1}/${d.getDate()}(${DAY_KO[d.getDay()]})`, shift: sh.name, have });
      }
    }
  }
  return gaps;
});

// ── 성향 미설정 현장 인력 — 달력에 못 올라가는 사람들 ────────────────────────
const unsetField = computed(() =>
  people.value.filter((p) => FIELD_POSITIONS.has(p.position) && !p.preferred_shift),
);
// 교대별 고정 인원 요약 (요양보호사 기준)
const shiftHeadcount = computed<Record<ShiftKey, number>>(() => {
  const rec: Record<ShiftKey, number> = { day: 0, evening: 0, night: 0 };
  for (const p of people.value) {
    if (!CARE_POSITIONS.has(p.position)) continue;
    if (p.preferred_shift && p.preferred_shift in rec) rec[p.preferred_shift as ShiftKey]++;
  }
  return rec;
});

function openStaff(p: OrgPerson) { router.push(`/staff/${p.id}`); }
function goStaffPage() { router.push("/staff"); }

onMounted(async () => {
  await load();
  await loadLeave(visibleDates.value);
});
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">스케쥴러 — 고정 쉬프트</div>
        <div class="text-caption text-grey-6">
          직원별 근무 성향(선호 교대·가능 요일)에서 자동 계산됩니다 — 수정은
          <a class="text-primary cursor-pointer" @click="goStaffPage">직원 페이지</a>의 근무 성향에서.
        </div>
      </div>
      <div class="col-auto row items-center q-gutter-sm">
        <q-btn-toggle v-model="viewMode" dense unelevated toggle-color="primary" :options="[
          { label: '주간', value: 'week' }, { label: '월간', value: 'month' },
        ]" />
        <q-btn outline dense color="primary" label="오늘" @click="goToday" />
        <q-btn flat round dense icon="o_chevron_left" @click="prev" />
        <div class="text-subtitle1 text-weight-medium" style="min-width: 170px; text-align: center">
          {{ viewMode === "week" ? weekLabel : monthLabel }}
        </div>
        <q-btn flat round dense icon="o_chevron_right" @click="next" />
      </div>
    </div>

    <!-- 요약 + 매칭 상태 -->
    <div class="row q-col-gutter-sm q-mb-md">
      <div class="col-12 col-md-7">
        <q-banner dense rounded class="bg-blue-1 text-blue-10 full-height">
          <template #avatar><q-icon name="o_diversity_3" /></template>
          어르신 {{ elders }}명 → 교대당 요양보호사 <b>{{ required }}명</b> 필요 (1:10 · 최소 2명)
          <span class="q-ml-md">
            고정 인원:
            <template v-for="sh in SHIFTS" :key="sh.key">
              <q-badge :color="shiftHeadcount[sh.key] >= required ? 'green-2' : 'red-2'"
                :text-color="shiftHeadcount[sh.key] >= required ? 'green-10' : 'red-10'" class="q-mx-xs">
                {{ sh.name }} {{ shiftHeadcount[sh.key] }}명
              </q-badge>
            </template>
          </span>
        </q-banner>
      </div>
      <div class="col-12 col-md-5">
        <q-banner v-if="coverageGaps.length" dense rounded class="bg-red-1 text-red-10 full-height">
          <template #avatar><q-icon name="o_warning" /></template>
          <b>인력 부족 {{ coverageGaps.length }}건</b> —
          <span class="text-caption">
            {{ coverageGaps.slice(0, 4).map((g) => g.label + " " + g.shift + " " + g.have + "/" + required + "명").join(", ") }}<template v-if="coverageGaps.length > 4"> 외 {{ coverageGaps.length - 4 }}건</template>
          </span>
        </q-banner>
        <q-banner v-else dense rounded class="bg-green-1 text-green-10 full-height">
          <template #avatar><q-icon name="o_check_circle" /></template>
          이 기간의 모든 교대가 필요 인원을 충족합니다.
        </q-banner>
      </div>
    </div>

    <!-- 성향 미설정 현장 인력 -->
    <q-banner v-if="unsetField.length" dense rounded class="bg-orange-1 text-orange-10 q-mb-md">
      <template #avatar><q-icon name="o_tune" /></template>
      근무 성향이 설정되지 않은 현장 인력 <b>{{ unsetField.length }}명</b>은 달력에 표시되지 않습니다.
      <q-chip v-for="p in unsetField.slice(0, 8)" :key="p.id" dense clickable size="sm"
        color="orange-2" text-color="orange-10" @click="openStaff(p)">{{ p.full_name }}</q-chip>
      <span v-if="unsetField.length > 8" class="text-caption">외 {{ unsetField.length - 8 }}명</span>
      <template #action>
        <q-btn flat dense color="orange-10" label="직원 페이지에서 설정" @click="goStaffPage" />
      </template>
    </q-banner>

    <!-- 주간 보기 -->
    <div v-if="viewMode === 'week'" class="row no-wrap q-col-gutter-sm">
      <div v-for="d in weekDays" :key="localDateStr(d)" class="col">
        <q-card flat bordered class="full-height" :class="localDateStr(d) === todayStr ? 'today-card' : ''">
          <q-card-section class="q-pa-sm q-pb-none text-center">
            <div class="text-caption" :class="d.getDay() === 0 ? 'text-red' : d.getDay() === 6 ? 'text-blue' : 'text-grey-7'">
              {{ DAY_KO[d.getDay()] }}
            </div>
            <div class="text-subtitle2 text-weight-bold">{{ d.getMonth() + 1 }}/{{ d.getDate() }}</div>
          </q-card-section>
          <q-card-section class="q-pa-sm q-gutter-y-sm">
            <div v-for="sh in SHIFTS" :key="sh.key" class="shift-block" :class="`bg-${sh.color}`">
              <div class="row items-center no-wrap q-mb-xs">
                <q-icon :name="sh.icon" size="14px" :class="`text-${sh.text}`" />
                <span class="text-caption text-weight-medium q-ml-xs" :class="`text-${sh.text}`">
                  {{ sh.name }} {{ sh.start }}–{{ sh.end }}
                </span>
                <q-space />
                <q-badge :color="careCount(shiftList(d, sh.key)) >= required ? 'green' : 'red'" rounded>
                  {{ careCount(shiftList(d, sh.key)) }}/{{ required }}
                </q-badge>
              </div>
              <template v-if="shiftList(d, sh.key).length">
                <q-chip v-for="p in shiftList(d, sh.key)" :key="p.id"
                  dense clickable size="sm" class="full-width staff-chip" color="white" text-color="grey-9"
                  @click="openStaff(p)">
                  <span class="ellipsis">{{ p.full_name }}</span>
                  <span class="text-grey-5 q-ml-xs" style="font-size: 10px">{{ p.position_ko }}</span>
                </q-chip>
              </template>
              <div v-else class="text-caption text-grey-5 text-center q-py-xs">근무자 없음</div>
            </div>
          </q-card-section>
        </q-card>
      </div>
    </div>

    <!-- 월간 보기 — 교대별 인원 수 (클릭 → 해당 주) -->
    <q-card v-else flat bordered>
      <div class="month-grid month-head">
        <div v-for="(d, i) in DAY_KO" :key="d" class="text-center text-caption text-weight-medium q-py-xs"
          :class="i === 0 ? 'text-red' : i === 6 ? 'text-blue' : 'text-grey-7'">{{ d }}</div>
      </div>
      <div v-for="(week, wi) in monthGrid" :key="wi" class="month-grid">
        <div v-for="d in week" :key="localDateStr(d)" class="month-cell cursor-pointer"
          :class="[d.getMonth() !== currentMonth.getMonth() ? 'other-month' : '', localDateStr(d) === todayStr ? 'today-cell' : '']"
          @click="openDay(d)">
          <div class="text-caption text-weight-bold q-mb-xs"
            :class="d.getDay() === 0 ? 'text-red' : d.getDay() === 6 ? 'text-blue' : ''">{{ d.getDate() }}</div>
          <div v-for="sh in SHIFTS" :key="sh.key" class="row items-center no-wrap month-shift-row">
            <span class="text-caption" :class="`text-${sh.text}`">{{ sh.name }}</span>
            <q-space />
            <q-badge dense :color="careCount(shiftList(d, sh.key)) >= required ? 'green-2' : 'red-2'"
              :text-color="careCount(shiftList(d, sh.key)) >= required ? 'green-10' : 'red-10'">
              {{ careCount(shiftList(d, sh.key)) }}
            </q-badge>
          </div>
        </div>
      </div>
    </q-card>

    <q-inner-loading :showing="loading"><q-spinner size="40px" color="primary" /></q-inner-loading>
  </q-page>
</template>

<style scoped>
.shift-block { border-radius: 8px; padding: 6px; }
.staff-chip { margin: 1px 0; }
.today-card { border-color: var(--q-primary); border-width: 2px; }
.month-grid { display: grid; grid-template-columns: repeat(7, 1fr); }
.month-head { border-bottom: 1px solid #e2e8f0; }
.month-cell { min-height: 92px; border: 1px solid #f1f5f9; padding: 6px; }
.month-cell:hover { background: #f0f9ff; }
.other-month { opacity: 0.35; }
.today-cell { outline: 2px solid var(--q-primary); outline-offset: -2px; }
.ellipsis { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 90px; }
</style>
