<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useQuasar } from "quasar";
import { server } from "@/lib/server";

const $q = useQuasar();

interface Balance {
  year: number;
  annual_allocated: number;
  annual_used: number;
  annual_remaining: number;
  sick_used: number;
}
interface LeaveReq {
  id: string;
  leave_type: string;
  start_date: string;
  end_date: string;
  days: number;
  reason: string | null;
  status: "pending" | "approved" | "rejected" | "cancelled";
  requested_at: string;
}

const LEAVE_TYPE_OPTS = [
  { label: "연차",    value: "annual" },
  { label: "병가",    value: "sick" },
  { label: "경조사",  value: "personal" },
  { label: "공가",    value: "public" },
  { label: "출산휴가", value: "maternity" },
];
const STATUS_KO: Record<LeaveReq["status"], string> = {
  pending: "승인 대기",
  approved: "승인됨",
  rejected: "반려됨",
  cancelled: "취소됨",
};
const STATUS_COLOR: Record<LeaveReq["status"], string> = {
  pending: "warning",
  approved: "positive",
  rejected: "negative",
  cancelled: "grey",
};

function todayLocal() {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

const balance = ref<Balance | null>(null);
const requests = ref<LeaveReq[]>([]);
const loading = ref(false);

const form = ref({
  leave_type: "annual",
  start_date: todayLocal(),
  end_date: todayLocal(),
  days: 1,
  reason: "",
});
const submitting = ref(false);

async function load() {
  loading.value = true;
  try {
    const [b, r] = await Promise.all([
      server.leaveBalance(),
      server.myLeaveRequests(),
    ]);
    balance.value = b;
    requests.value = r;
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.message ?? "불러오기 실패" });
  } finally {
    loading.value = false;
  }
}

async function submit() {
  if (submitting.value) return;
  if (!form.value.start_date || !form.value.end_date || form.value.days <= 0) {
    $q.notify({ type: "warning", message: "기간과 일수를 확인하세요" });
    return;
  }
  submitting.value = true;
  try {
    await server.createLeaveRequest({
      leave_type: form.value.leave_type,
      start_date: form.value.start_date,
      end_date: form.value.end_date,
      days: form.value.days,
      reason: form.value.reason || null,
    });
    $q.notify({ type: "positive", message: "휴가 신청 완료 — 승인을 기다려 주세요" });
    form.value = {
      leave_type: "annual",
      start_date: todayLocal(),
      end_date: todayLocal(),
      days: 1,
      reason: "",
    };
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.message ?? "신청 실패" });
  } finally {
    submitting.value = false;
  }
}

function fmtDateTime(iso: string) {
  return new Date(iso).toLocaleString("ko-KR", {
    month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit",
  });
}

const pending = computed(() => requests.value.filter((r) => r.status === "pending"));
const history = computed(() => requests.value.filter((r) => r.status !== "pending"));

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="text-h5 text-weight-bold q-mb-xs">
      <q-icon name="o_event_busy" class="q-mr-sm text-primary" />
      휴가 신청
    </div>
    <div class="text-caption text-grey-7 q-mb-lg">
      연차 · 병가 · 경조사 등을 신청합니다. 신청 후 센터장 / 본사 승인을 기다리세요.
    </div>

    <!-- Balance summary -->
    <div v-if="balance" class="row q-col-gutter-md q-mb-lg">
      <div class="col-12 col-sm-3">
        <q-card flat bordered class="q-pa-md">
          <div class="text-caption text-grey-7">{{ balance.year }} 연차 잔여</div>
          <div class="text-h4 text-primary text-weight-bold q-mt-xs">
            {{ balance.annual_remaining.toFixed(1) }}<span class="text-body2 text-grey-7">일</span>
          </div>
        </q-card>
      </div>
      <div class="col-12 col-sm-3">
        <q-card flat bordered class="q-pa-md">
          <div class="text-caption text-grey-7">연차 사용</div>
          <div class="text-h4 text-weight-bold q-mt-xs">
            {{ balance.annual_used.toFixed(1) }}<span class="text-body2 text-grey-7">일</span>
          </div>
        </q-card>
      </div>
      <div class="col-12 col-sm-3">
        <q-card flat bordered class="q-pa-md">
          <div class="text-caption text-grey-7">연차 총량</div>
          <div class="text-h4 text-weight-bold q-mt-xs">
            {{ balance.annual_allocated.toFixed(1) }}<span class="text-body2 text-grey-7">일</span>
          </div>
        </q-card>
      </div>
      <div class="col-12 col-sm-3">
        <q-card flat bordered class="q-pa-md">
          <div class="text-caption text-grey-7">병가 사용</div>
          <div class="text-h4 text-weight-bold q-mt-xs">
            {{ balance.sick_used.toFixed(1) }}<span class="text-body2 text-grey-7">일</span>
          </div>
        </q-card>
      </div>
    </div>

    <!-- New leave request form -->
    <q-card flat bordered class="q-pa-md q-mb-lg">
      <div class="text-subtitle1 text-weight-semibold q-mb-md">
        <q-icon name="o_add_circle" class="q-mr-sm" /> 새 휴가 신청
      </div>
      <div class="row q-col-gutter-md">
        <div class="col-12 col-md-3">
          <q-select
            v-model="form.leave_type"
            :options="LEAVE_TYPE_OPTS"
            label="종류"
            outlined
            dense
            emit-value
            map-options
          />
        </div>
        <div class="col-6 col-md-3">
          <q-input v-model="form.start_date" type="date" label="시작일" outlined dense />
        </div>
        <div class="col-6 col-md-3">
          <q-input v-model="form.end_date" type="date" label="종료일" outlined dense />
        </div>
        <div class="col-12 col-md-3">
          <q-input
            v-model.number="form.days"
            type="number"
            step="0.5"
            min="0.5"
            label="일수 (반차 = 0.5)"
            outlined
            dense
          />
        </div>
      </div>
      <q-input
        v-model="form.reason"
        type="textarea"
        rows="2"
        label="사유 (선택)"
        outlined
        dense
        class="q-mt-md"
        placeholder="가족 행사, 본인 진료 등"
      />
      <div class="row justify-end q-mt-md">
        <q-btn
          unelevated
          color="primary"
          icon="o_send"
          label="신청"
          :loading="submitting"
          @click="submit"
        />
      </div>
    </q-card>

    <!-- Pending requests -->
    <div v-if="pending.length > 0" class="q-mb-lg">
      <div class="text-subtitle2 text-weight-semibold q-mb-sm">승인 대기 중</div>
      <q-card flat bordered>
        <q-list separator>
          <q-item v-for="r in pending" :key="r.id">
            <q-item-section>
              <q-item-label class="text-weight-medium">
                {{ LEAVE_TYPE_OPTS.find((o) => o.value === r.leave_type)?.label ?? r.leave_type }}
                · {{ r.days.toFixed(1) }}일
              </q-item-label>
              <q-item-label caption>
                {{ r.start_date }} ~ {{ r.end_date }}
                <span v-if="r.reason" class="q-ml-sm text-grey-7">· {{ r.reason }}</span>
              </q-item-label>
            </q-item-section>
            <q-item-section side>
              <q-badge :color="STATUS_COLOR[r.status]">{{ STATUS_KO[r.status] }}</q-badge>
            </q-item-section>
          </q-item>
        </q-list>
      </q-card>
    </div>

    <!-- History -->
    <div>
      <div class="text-subtitle2 text-weight-semibold q-mb-sm">신청 내역</div>
      <q-card flat bordered>
        <q-list separator>
          <q-item v-if="history.length === 0">
            <q-item-section class="text-grey-6 text-center q-pa-md">
              기록이 없습니다.
            </q-item-section>
          </q-item>
          <q-item v-for="r in history" :key="r.id">
            <q-item-section>
              <q-item-label>
                {{ LEAVE_TYPE_OPTS.find((o) => o.value === r.leave_type)?.label ?? r.leave_type }}
                · {{ r.days.toFixed(1) }}일
              </q-item-label>
              <q-item-label caption>
                {{ r.start_date }} ~ {{ r.end_date }}
                · 신청 {{ fmtDateTime(r.requested_at) }}
              </q-item-label>
            </q-item-section>
            <q-item-section side>
              <q-badge :color="STATUS_COLOR[r.status]">{{ STATUS_KO[r.status] }}</q-badge>
            </q-item-section>
          </q-item>
        </q-list>
      </q-card>
    </div>
  </q-page>
</template>
