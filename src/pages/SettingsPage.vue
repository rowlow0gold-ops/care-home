<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useQuasar } from "quasar";
import { useServerSessionStore } from "@/stores/server-session";
import { useSettingsStore, type ShiftModel } from "@/stores/settings";

const $q = useQuasar();
const session = useServerSessionStore();
const appSettings = useSettingsStore();

const roleLabel: Record<string, string> = {
  caregiver: "요양보호사",
  nurse: "간호사",
  branch_manager: "시설장",
  hq: "본사",
  super_admin: "최고관리자",
};

const me = computed(() => session.me);
const stationName = computed(() => session.branchName ?? session.tenantName ?? "—");

// ── Shift model preference (local app pref) ─────────────────────────────────
const shiftModel = ref<ShiftModel>("12h");
const savingShift = ref(false);

const shiftOptions = [
  { label: "2교대 (12시간)", value: "12h" as ShiftModel },
  { label: "3교대 (8시간)", value: "8h" as ShiftModel },
];

async function saveShift(model: ShiftModel) {
  savingShift.value = true;
  try {
    await appSettings.saveShiftModel(model);
    $q.notify({ type: "positive", message: "근무 교대 방식을 저장했습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `저장 실패: ${e?.message ?? e}` });
  } finally {
    savingShift.value = false;
  }
}

async function logout() {
  await session.logout();
  window.location.hash = "#/login";
}

onMounted(async () => {
  await appSettings.load();
  shiftModel.value = appSettings.shiftModel;
});
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="text-h5 text-weight-bold q-mb-md">설정</div>

    <div class="row q-col-gutter-md">
      <!-- Account -->
      <div class="col-12 col-md-6">
        <q-card flat bordered>
          <q-card-section>
            <div class="text-subtitle1 text-weight-bold q-mb-sm">내 계정</div>
            <q-list dense>
              <q-item>
                <q-item-section>이름</q-item-section>
                <q-item-section side class="text-weight-medium">{{ me?.name ?? "—" }}</q-item-section>
              </q-item>
              <q-item>
                <q-item-section>이메일</q-item-section>
                <q-item-section side>{{ me?.email ?? "—" }}</q-item-section>
              </q-item>
              <q-item>
                <q-item-section>권한</q-item-section>
                <q-item-section side>
                  <q-badge color="blue">{{ roleLabel[me?.role ?? ""] ?? me?.role }}</q-badge>
                </q-item-section>
              </q-item>
              <q-item>
                <q-item-section>소속</q-item-section>
                <q-item-section side>{{ stationName }}</q-item-section>
              </q-item>
            </q-list>
            <q-btn
              outline
              color="negative"
              icon="o_logout"
              label="로그아웃"
              class="q-mt-md"
              @click="logout"
            />
          </q-card-section>
        </q-card>
      </div>

      <!-- App preferences -->
      <div class="col-12 col-md-6">
        <q-card flat bordered>
          <q-card-section>
            <div class="text-subtitle1 text-weight-bold q-mb-sm">근무 교대 방식</div>
            <div class="text-caption text-grey-6 q-mb-sm">
              근무일정에서 사용하는 기본 교대 방식입니다. (이 기기에만 저장됩니다)
            </div>
            <q-option-group
              v-model="shiftModel"
              :options="shiftOptions"
              color="primary"
              :disable="savingShift"
              @update:model-value="saveShift"
            />
          </q-card-section>
        </q-card>

      </div>
    </div>
  </q-page>
</template>
