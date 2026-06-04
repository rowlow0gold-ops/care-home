<script setup lang="ts">
// ── 스케쥴러 — 북미식 2주(Biweekly) 스케줄 ────────────────────────────────────
// 북미 요양원은 스케줄을 2주 단위로 발행한다. 직원은 근무조가 고정:
//   · 요양 12시간조 — Day 07:00–19:30 / Night 19:00–07:30, 3일 연속 근무·4일 휴무 보장
//   · 요양 8시간 교대조 — 고정 쉬프트(주간/오후/야간), 주 5일·휴무 2일 보장.
//     주말은 순환 근무(Rotational Commitment): 3주마다 한 번 토·일 의무 근무,
//     그 주는 주중 연속 2일 휴무. 평주는 토·일 휴무. 달력 고정이라 미리 알 수 있다.
// '발행'은 위 규칙으로 2주치 근무를 계산해 서버에 확정 저장(solid data)한다.
// 인원 매칭: 입소(요양) 어르신 실시간 수 기준 1:10 — 매 시간 커버리지 검사.
import { ref, computed, onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type OrgPerson, type RosterEntry, type UpsertRoster } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const route = useRoute();
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
  return BLOCKS.find((b) => b.group === group && b.shift === shift) ?? BLOCKS[2];
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
  const seen = new Set<string>();
  caregivers.value = all.filter((p) => {
    if (p.is_inactive || p.position !== "caregiver" || seen.has(p.id)) return false;
    seen.add(p.id);
    return true;
  });
}
async function loadElders() {
  const res = await server.residentsPaged({ care_type: "residential", status: "active", page: 1, page_size: 1 });
  elders.value = res.total;
}
// ── 공휴일 — 달력에 표시 (DB holiday API) ────────────────────────────────────
const holidays = ref<Map<string, string>>(new Map());
async function loadHolidays() {
  const years = [...new Set(periodDays.value.map((d) => d.getFullYear()))];
  const m = new Map<string, string>();
  for (const y of years) {
    try {
      const rows = await server.holidays(y);
      for (const h of rows) if (h.is_holiday) m.set(h.locdate, h.name);
    } catch { /* 공휴일 조회 실패 시 표시 생략 */ }
  }
  holidays.value = m;
}
function holidayName(ds: string): string | null {
  return holidays.value.get(ds) ?? null;
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
watch(periodStart, () => { loadEntries().catch(() => {}); loadHolidays(); });

function prev() { periodStart.value = addDays(periodStart.value, -14); }
function next() { periodStart.value = addDays(periodStart.value, 14); }
function goToday() { periodStart.value = periodStartOf(new Date()); }

// ── 그룹 구성 요약 ────────────────────────────────────────────────────────────
const groupStats = computed(() => {
  const c = { h12d: 0, h12n: 0, h8: 0, none: 0 };
  for (const p of caregivers.value) {
    if (p.shift_group === "h12") (p.preferred_shift === "night" ? c.h12n++ : c.h12d++);
    else if (p.shift_group === "h8") c.h8++;
    else c.none++;
  }
  return c;
});
const unassigned = computed(() => caregivers.value.filter((p) => !p.shift_group));

// ── 필요 인원 (실시간 입소 어르신 1:10) ──────────────────────────────────────
const required = computed(() => Math.max(1, Math.ceil(elders.value / 10)));

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

// 균등 배치 + 주말 로테이션 — 모두가 주말을 싫어하니 공평하게 나눈다.
//
// 주말 규칙(전 직원 공통): 2주에 정확히 주말 1일 근무. 4개의 주말 슬롯
//   (일₁·토₁·일₂·토₂)에 직원을 1/4씩 배치하고, 기간이 바뀔 때마다 슬롯이
//   한 칸씩 돌아간다(달력 고정) — 다음 내 주말이 언제인지 미리 안다.
//
// 12시간조: 2주 6일 = 3일 연속 런 × 2 (북미 2-3-2식 — 근무 요일이 주마다 다름).
//   한 런은 내 주말 슬롯을 포함하고(일→일월화, 토→목금토), 다른 런은
//   반대 주의 평일 3일. 연속 근무는 최대 3일.
//
// 8시간조: 2주 10일 = 평일 9일 + 주말 1일. 기본은 주말 주에 평일 휴무를 받아
//   최대 주 5일. '추가 근무 선호' 직원만 반대 주에 쉬어 주말 주 6일이 된다.
function weekendSlot(idx: number, periodIdx: number): number {
  return (((idx + periodIdx) % 4) + 4) % 4; // 0=일₁ 1=토₁ 2=일₂ 3=토₂
}
function slotDay(slot: number, weeks: string[][]): string {
  return [weeks[0][0], weeks[0][6], weeks[1][0], weeks[1][6]][slot];
}

function assignH12(
  members: OrgPerson[], weeks: string[][], periodIdx: number,
  assigned: Map<string, OrgPerson[]>,
) {
  const sorted = [...members].sort((a, b) => a.id.localeCompare(b.id));
  const put = (p: OrgPerson, ds: string) => {
    if (!assigned.has(ds)) assigned.set(ds, []);
    assigned.get(ds)!.push(p);
  };
  // 슬롯별 [주말 포함 런(주, 시작일), 평일 런(반대 주, 시작일)] — 모두 3일 연속
  const RUNS: Array<[[number, number], [number, number]]> = [
    [[0, 0], [1, 3]], // 일₁: 일월화(1주) + 수목금(2주)
    [[0, 4], [1, 1]], // 토₁: 목금토(1주) + 월화수(2주)
    [[1, 0], [0, 3]], // 일₂: 일월화(2주) + 수목금(1주)
    [[1, 4], [0, 1]], // 토₂: 목금토(2주) + 월화수(1주)
  ];
  sorted.forEach((p, idx) => {
    const [[w1, s1], [w2, s2]] = RUNS[weekendSlot(idx, periodIdx)];
    for (let k = 0; k < 3; k++) put(p, weeks[w1][s1 + k]);
    for (let k = 0; k < 3; k++) put(p, weeks[w2][s2 + k]);
  });
}

function assignH8(
  members: OrgPerson[], weeks: string[][], periodIdx: number,
  assigned: Map<string, OrgPerson[]>,
) {
  const sorted = [...members].sort((a, b) => a.id.localeCompare(b.id));
  const put = (p: OrgPerson, ds: string) => {
    if (!assigned.has(ds)) assigned.set(ds, []);
    assigned.get(ds)!.push(p);
  };
  sorted.forEach((p, idx) => {
    const slot = weekendSlot(idx, periodIdx);
    put(p, slotDay(slot, weeks)); // 내 주말 1일
    // 기본: 주말이 낀 주에 평일 휴무 → 최대 주 5일.
    // 추가 근무 선호자(extra_shift_ok)만 반대 주에 쉬어 주말 주 6일이 된다.
    const weekendWeek = slot < 2 ? 0 : 1;
    const offWeek = p.extra_shift_ok ? 1 - weekendWeek : weekendWeek;
    const offDow = 1 + ((idx + periodIdx) % 5); // 월~금 분산 (기간마다 회전)
    for (let w = 0; w < 2; w++) {
      for (let d = 1; d <= 5; d++) {
        if (w === offWeek && d === offDow) continue;
        put(p, weeks[w][d]);
      }
    }
  });
}

function buildEntries(): UpsertRoster[] {
  const out: UpsertRoster[] = [];
  const dates = periodDays.value.map(localDateStr);
  const SUBGROUPS: Array<{ group: string; shift: string; note: string }> = [
    { group: "h8",  shift: "day",     note: "8시간조 주간 (2주 10일 · 주말 1일)" },
    { group: "h8",  shift: "evening", note: "8시간조 오후 (2주 10일 · 주말 1일)" },
    { group: "h8",  shift: "night",   note: "8시간조 야간 (2주 10일 · 주말 1일)" },
    { group: "h12", shift: "day",     note: "12시간조 Day (3일 런 ×2 · 주말 1일)" },
    { group: "h12", shift: "night",   note: "12시간조 Night (3일 런 ×2 · 주말 1일)" },
  ];
  const weeks = [dates.slice(0, 7), dates.slice(7, 14)];
  const periodIdx = Math.floor((periodStart.value.getTime() - EPOCH.getTime()) / (14 * 86400000));
  for (const sg of SUBGROUPS) {
    const members = caregivers.value.filter((p) =>
      p.shift_group === sg.group &&
      (sg.group === "h12"
        ? (p.preferred_shift === "night" ? "night" : "day") === sg.shift
        : (p.preferred_shift ?? "day") === sg.shift));
    if (!members.length) continue;
    const b = blockFor(sg.group, sg.shift);
    const assigned = new Map<string, OrgPerson[]>();
    if (sg.group === "h8") assignH8(members, weeks, periodIdx, assigned);
    else assignH12(members, weeks, periodIdx, assigned);
    assigned.forEach((list, ds) => {
      for (const p of list) {
        out.push({ user_id: p.id, shift_date: ds, shift_start: b.start, shift_end: b.end, shift_hours: b.hours, notes: sg.note });
      }
    });
  }
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
      const list = buildEntries();
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
    $q.notify({ type: "info", message: "이미 발행된 기간입니다. 발행된 근무표는 변경하지 않습니다." });
    return;
  }
  await doIt();
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
// 24시간 인원 그래프 — 시설 하루는 07시에 시작 (07 → 익일 06시)
interface HourBar { hour: number; label: string; count: number; x: number; y: number; h: number; ok: boolean }
const CHART = { w: 492, h: 150, top: 12, bottom: 22, barW: 16, gap: 4.5 };
function dayChart(ds: string): { bars: HourBar[]; max: number; reqY: number } {
  const cov = hourCover(ds);
  const hours = Array.from({ length: 24 }, (_, i) => (7 + i) % 24);
  const max = Math.max(required.value, ...hours.map((h) => cov[h]), 1);
  const plotH = CHART.h - CHART.top - CHART.bottom;
  const bars = hours.map((h, i) => {
    const c = cov[h];
    const bh = Math.round((c / max) * plotH);
    return {
      hour: h, label: `${String(h).padStart(2, "0")}시`, count: c,
      x: i * (CHART.barW + CHART.gap), y: CHART.top + plotH - bh, h: bh,
      ok: c >= required.value,
    };
  });
  return { bars, max, reqY: CHART.top + plotH - (required.value / max) * plotH };
}
// 호버 툴팁: 그 사람이 이 기간에 일하는 날들
function personDaysText(userId: string): string {
  const mine = entries.value.filter((e) => e.user_id === userId).map((e) => e.shift_date).sort();
  if (!mine.length) return "";
  const week2start = localDateStr(addDays(periodStart.value, 7));
  const fmt = (ds: string) => {
    const d = new Date(ds + "T00:00:00");
    const w = DAY_KO[d.getDay()];
    return `${d.getDate()}(${w})`;
  };
  const w1 = mine.filter((ds) => ds < week2start).map(fmt).join("·");
  const w2 = mine.filter((ds) => ds >= week2start).map(fmt).join("·");
  return `근무 ${mine.length}일 — 1주차: ${w1 || "없음"} / 2주차: ${w2 || "없음"}`;
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
// 팝업에서 이름 클릭 → 직원 상세. 상세의 '근무표' 버튼이 이 날짜 팝업으로 되돌린다.
function openStaffFromDay(e: RosterEntry) {
  router.push(`/staff/${e.user_id}?from=schedule&day=${dayDs.value}`);
}
onMounted(async () => {
  loadHolidays();
  await loadAll();
  // 직원 상세에서 복귀(?day=…): 그 날짜의 기간으로 이동 후 팝업을 다시 연다
  const day = typeof route.query.day === "string" ? route.query.day : "";
  if (/^\d{4}-\d{2}-\d{2}$/.test(day)) {
    const d = new Date(day + "T00:00:00");
    const ps = periodStartOf(d);
    if (localDateStr(ps) !== localDateStr(periodStart.value)) {
      periodStart.value = ps;
      await loadEntries().catch(() => {});
    }
    openDay(d);
    router.replace({ path: route.path }); // 쿼리 제거 — 닫은 뒤 재오픈 방지
  }
});
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-sm">
      <div class="col">
        <div class="text-h5 text-weight-bold">스케쥴러 — 2주 근무표</div>
        <div class="text-caption text-grey-6">
          북미식 2주 단위 발행 · 주말은 전원 2주에 1일만 (일·토 슬롯 로테이션) · 12시간조 3일 런 ×2 / 8시간조 2주 10일
        </div>
      </div>
      <div class="col-auto row items-center q-gutter-sm">
        <q-btn outline dense color="primary" label="이번 기간" @click="goToday" />
        <q-btn flat round dense icon="o_chevron_left" @click="prev" />
        <div class="text-subtitle1 text-weight-medium" style="min-width: 175px; text-align: center">{{ periodLabel }}</div>
        <q-btn flat round dense icon="o_chevron_right" @click="next" />
        <q-btn v-if="canPublish && !published" color="primary" unelevated icon="o_publish" :loading="publishing"
          label="2주 스케줄 발행" @click="publish">
          <q-tooltip>근무조 규칙(휴무 보장·주말 순환)으로 2주치 근무를 계산해 확정 저장합니다.</q-tooltip>
        </q-btn>
        <q-badge v-else-if="published" color="green-2" text-color="green-10" class="q-pa-sm">
          <q-icon name="o_verified" size="14px" class="q-mr-xs" /> 발행 완료
        </q-badge>
      </div>
    </div>

    <!-- 실시간 매칭 요약 -->
    <div class="row q-col-gutter-sm q-mb-md">
      <div class="col-12 col-md-7">
        <q-banner dense rounded class="bg-blue-1 text-blue-10 full-height">
          <template #avatar><q-icon name="o_diversity_3" /></template>
          입소 어르신 <b>{{ elders }}명</b> (실시간) → 매 시간 요양보호사 <b>{{ required }}명</b> 필요 (1:10)<br />
          <span class="text-caption">
            조 구성: 12시간조 {{ groupStats.h12d + groupStats.h12n }}명 (Day {{ groupStats.h12d }} / Night {{ groupStats.h12n }}, 3일 런 ×2/2주)
            · 8시간조 {{ groupStats.h8 }}명 (2주 10일) — <b>전원 주말 근무는 2주에 1일</b>, 슬롯은 기간마다 회전
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
          <div class="row items-center q-mb-xs no-wrap">
            <span class="text-caption text-weight-bold"
              :class="holidayName(localDateStr(d)) || d.getDay() === 0 ? 'text-red' : d.getDay() === 6 ? 'text-blue' : ''">
              {{ d.getMonth() + 1 }}/{{ d.getDate() }}
            </span>
            <q-badge v-if="holidayName(localDateStr(d))" color="red-2" text-color="red-10" class="q-ml-xs holiday-badge">
              {{ holidayName(localDateStr(d)) }}
            </q-badge>
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
    <q-dialog v-model="dayDialog" @hide="dayDialog = false">
      <q-card style="min-width: 520px; max-height: 80vh" class="scroll">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">
            {{ dayLabel }} — 근무 {{ dayTotal(dayDs) }}명
            <q-badge v-if="holidayName(dayDs)" color="red-2" text-color="red-10" class="q-ml-sm">{{ holidayName(dayDs) }}</q-badge>
          </div>
          <q-space />
          <q-btn icon="o_close" flat round dense @click="dayDialog = false" />
        </q-card-section>
        <q-card-section class="q-gutter-md">
          <!-- 24시간 인원 그래프 — 빨간 점선 = 필요 인원(1:10) -->
          <div>
            <div class="text-caption text-grey-7 q-mb-xs">24시간 근무 인원 (필요 {{ required }}명/시간 — 빨간 선)</div>
            <svg :width="CHART.w" :height="CHART.h" class="hour-chart">
              <template v-for="b in dayChart(dayDs).bars" :key="b.hour">
                <rect :x="b.x" :y="b.y" :width="CHART.barW" :height="b.h" rx="2"
                  :class="b.ok ? 'bar-ok' : 'bar-bad'">
                  <title>{{ b.label }} — {{ b.count }}명</title>
                </rect>
                <text v-if="b.count" :x="b.x + CHART.barW / 2" :y="b.y - 3" class="bar-val">{{ b.count }}</text>
                <text v-if="b.hour % 3 === 1 || b.hour === 7" :x="b.x + CHART.barW / 2" :y="CHART.h - 8" class="bar-lbl">{{ b.hour }}</text>
              </template>
              <line x1="0" :x2="CHART.w" :y1="dayChart(dayDs).reqY" :y2="dayChart(dayDs).reqY" class="req-line" />
            </svg>
          </div>
          <q-separator />
          <div v-for="{ block, list } in dayBlocks(dayDs)" :key="block.key">
            <div class="row items-center q-mb-xs">
              <q-badge :color="block.color" :text-color="block.text">{{ block.label }}</q-badge>
              <span class="text-caption text-grey-6 q-ml-sm">{{ list.length }}명</span>
            </div>
            <div class="row q-gutter-xs">
              <q-chip v-for="e in list" :key="e.id" dense clickable size="sm" color="grey-2" text-color="grey-9"
                @click="openStaffFromDay(e)">
                {{ e.staff_name }}
                <q-tooltip max-width="340px">
                  {{ personDaysText(e.user_id) }}<br />클릭 → 직원 상세 (근무표 버튼으로 복귀)
                </q-tooltip>
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
.holiday-badge { max-width: 64px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: inline-block; }
.hour-chart { display: block; }
.bar-ok { fill: #34d399; }
.bar-bad { fill: #f87171; }
.bar-val { font-size: 9px; fill: #475569; text-anchor: middle; }
.bar-lbl { font-size: 9px; fill: #94a3b8; text-anchor: middle; }
.req-line { stroke: #ef4444; stroke-width: 1.5; stroke-dasharray: 4 3; }
</style>
