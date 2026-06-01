<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useQuasar } from "quasar";
import { server, type StaffMember } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const session = useServerSessionStore();

// Create: 시설장(branch_manager)+; Edit/Deactivate: 본사(hq)만 — 서버가 강제.
const canCreate = computed(() => session.hasRole("branch_manager"));
const canAdmin = computed(() => session.hasRole("hq"));

// ── Server role labels (권한 레벨) ──────────────────────────────────────────
const ROLE_OPTIONS = [
  { label: "요양보호사", value: "caregiver" },
  { label: "간호사", value: "nurse" },
  { label: "시설장", value: "branch_manager" },
  { label: "본사", value: "hq" },
];
const roleLabel: Record<string, string> = {
  caregiver: "요양보호사",
  nurse: "간호사",
  branch_manager: "시설장",
  hq: "본사",
  super_admin: "최고관리자",
};
const roleColor: Record<string, string> = {
  caregiver: "grey-7",
  nurse: "teal",
  branch_manager: "blue",
  hq: "deep-purple",
  super_admin: "red",
};

// ── Position (직책) options — staff_position enum ───────────────────────────
const POSITION_OPTIONS = [
  { label: "대표", value: "ceo" },
  { label: "운영총괄", value: "coo" },
  { label: "재무이사", value: "cfo" },
  { label: "인사이사", value: "hr_director" },
  { label: "품질관리이사", value: "quality_director" },
  { label: "컴플라이언스", value: "compliance" },
  { label: "교육연수", value: "training" },
  { label: "IT/마케팅", value: "it" },
  { label: "시설장", value: "branch_manager" },
  { label: "사무국장", value: "office_manager" },
  { label: "사회복지사", value: "social_worker" },
  { label: "간호사", value: "nurse_rn" },
  { label: "간호조무사", value: "nurse_assistant" },
  { label: "영양사", value: "dietitian" },
  { label: "물리치료사", value: "physical_therapist" },
  { label: "작업치료사", value: "occupational_therapist" },
  { label: "요양보호사", value: "caregiver" },
  { label: "조리원", value: "cook" },
  { label: "환경미화원", value: "cleaner" },
  { label: "운전기사", value: "driver" },
  { label: "촉탁의", value: "doctor_visiting" },
  { label: "기타", value: "other" },
];

// ── Employment type (고용형태) ─────────────────────────────────────────────
const EMPLOYMENT_OPTIONS = [
  { label: "정규직", value: "regular" },
  { label: "계약직", value: "contract" },
  { label: "단기계약직", value: "short_contract" },
  { label: "시간제", value: "part_time" },
  { label: "일용직", value: "temporary" },
  { label: "파견직", value: "dispatched" },
  { label: "위촉직", value: "consultant" },
];

const staff = ref<StaffMember[]>([]);
const loading = ref(false);
const showAddDialog = ref(false);
const showEditDialog = ref(false);
const addFormRef = ref();
const editFormRef = ref();
const submitting = ref(false);
const selected = ref<StaffMember | null>(null);

const emptyAdd = () => ({
  full_name: "",
  email: "",
  password: "",
  role: "caregiver",
  position: "caregiver",
  employment_type: "regular",
  phone: "",
});
const addForm = ref(emptyAdd());

const editForm = ref({
  full_name: "",
  email: "",
  phone: "",
  position: "caregiver",
  employment_type: "regular",
});

const columns = [
  { name: "full_name", label: "이름", field: "full_name", align: "left" as const },
  { name: "role", label: "권한", field: "role", align: "left" as const },
  { name: "email", label: "이메일", field: "email", align: "left" as const },
  { name: "phone", label: "연락처", field: "phone", align: "left" as const },
  { name: "status", label: "상태", field: "status", align: "center" as const },
  { name: "actions", label: "", field: "actions", align: "center" as const },
];

async function loadStaff() {
  loading.value = true;
  try {
    const all = await server.staff();
    // Desktop is branch-scoped: show only the signed-in 센터's staff.
    const myBranch = session.me?.branch_id;
    staff.value = myBranch ? all.filter((s) => s.branch_id === myBranch) : all;
  } catch (e: any) {
    $q.notify({ type: "negative", message: `직원 목록을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loading.value = false;
  }
}

async function submitAdd() {
  const valid = await addFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await server.createStaff({
      full_name: addForm.value.full_name.trim(),
      email: addForm.value.email.trim(),
      password: addForm.value.password,
      role: addForm.value.role,
      position: addForm.value.position,
      employment_type: addForm.value.employment_type,
      phone: addForm.value.phone.trim() || null,
    });
    $q.notify({ type: "positive", message: "직원이 추가되었습니다." });
    showAddDialog.value = false;
    addForm.value = emptyAdd();
    await loadStaff();
  } catch (e: any) {
    const msg = e?.status === 409 ? "이미 등록된 이메일입니다." : `추가 실패: ${e?.message ?? e}`;
    $q.notify({ type: "negative", message: msg });
  } finally {
    submitting.value = false;
  }
}

function openEdit(s: StaffMember) {
  selected.value = s;
  editForm.value = {
    full_name: s.full_name,
    email: s.email,
    phone: s.phone ?? "",
    position: "caregiver",
    employment_type: "regular",
  };
  showEditDialog.value = true;
}

async function submitEdit() {
  if (!selected.value) return;
  const valid = await editFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await server.updateStaff(selected.value.id, {
      expected_updated_at: selected.value.updated_at,
      full_name: editForm.value.full_name.trim(),
      email: editForm.value.email.trim(),
      phone: editForm.value.phone.trim() || null,
      position: editForm.value.position,
      employment_type: editForm.value.employment_type,
    });
    $q.notify({ type: "positive", message: "직원 정보가 수정되었습니다." });
    showEditDialog.value = false;
    await loadStaff();
  } catch (e: any) {
    const msg =
      e?.status === 409
        ? "다른 사용자가 먼저 수정했습니다. 새로고침 후 다시 시도해 주세요."
        : `수정 실패: ${e?.message ?? e}`;
    $q.notify({ type: "negative", message: msg });
  } finally {
    submitting.value = false;
  }
}

function confirmDeactivate(s: StaffMember) {
  $q.dialog({
    title: "직원 비활성화",
    message: `${s.full_name}님을 비활성화하시겠습니까? 더 이상 로그인할 수 없습니다.`,
    cancel: { label: "취소", flat: true },
    ok: { label: "비활성화", color: "negative", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    try {
      await server.deactivateStaff(s.id);
      $q.notify({ type: "positive", message: "비활성화되었습니다." });
      await loadStaff();
    } catch (e: any) {
      $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` });
    }
  });
}

onMounted(loadStaff);
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="text-h5 text-weight-bold">직원 관리</div>
        <div class="text-caption text-grey-6">센터 직원 명부</div>
      </div>
      <div v-if="canCreate" class="col-auto">
        <q-btn
          color="primary"
          icon="o_person_add"
          label="직원 추가"
          unelevated
          @click="() => { addForm = emptyAdd(); showAddDialog = true; }"
        />
      </div>
    </div>

    <!-- Skeleton loading -->
    <template v-if="loading">
      <q-card flat bordered class="q-pa-md">
        <q-skeleton type="rect" height="40px" class="q-mb-sm" />
        <q-skeleton type="rect" height="48px" class="q-mb-sm" v-for="n in 6" :key="n" />
      </q-card>
    </template>

    <!-- Table -->
    <q-table
      v-else
      :rows="staff"
      :columns="columns"
      row-key="id"
      flat
      bordered
      :rows-per-page-options="[10, 25, 50]"
    >
      <template #body-cell-role="props">
        <q-td :props="props">
          <q-badge :color="roleColor[props.row.role] || 'grey'" :label="roleLabel[props.row.role] || props.row.role" />
        </q-td>
      </template>

      <template #body-cell-phone="props">
        <q-td :props="props">{{ props.row.phone || "—" }}</q-td>
      </template>

      <template #body-cell-status="props">
        <q-td :props="props" class="text-center">
          <q-badge v-if="!props.row.deactivated_at" color="green" label="재직" />
          <q-badge v-else color="grey-5" label="비활성" />
        </q-td>
      </template>

      <template #body-cell-actions="props">
        <q-td :props="props" class="text-center">
          <template v-if="canAdmin && !props.row.deactivated_at">
            <q-btn flat round dense icon="o_edit" color="primary" @click="openEdit(props.row)">
              <q-tooltip>정보 수정</q-tooltip>
            </q-btn>
            <q-btn flat round dense icon="o_person_off" color="negative" @click="confirmDeactivate(props.row)">
              <q-tooltip>비활성화</q-tooltip>
            </q-btn>
          </template>
          <span v-else class="text-caption text-grey-4">—</span>
        </q-td>
      </template>

      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_group" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">등록된 직원이 없습니다</div>
        </div>
      </template>
    </q-table>

    <!-- Add Staff Dialog -->
    <q-dialog v-model="showAddDialog" persistent>
      <q-card style="min-width: 540px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">직원 추가</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section>
          <q-form ref="addFormRef" class="q-gutter-sm">
            <div class="text-subtitle2 text-grey-7">계정 정보</div>
            <div class="row q-gutter-sm">
              <div class="col">
                <q-input
                  v-model="addForm.full_name"
                  label="이름 *"
                  outlined
                  dense
                  :rules="[(v) => !!v?.trim() || '이름을 입력하세요']"
                  lazy-rules="ondemand"
                />
              </div>
              <div class="col">
                <q-input
                  v-model="addForm.email"
                  label="이메일 *"
                  type="email"
                  outlined
                  dense
                  :rules="[(v) => !!v?.trim() || '이메일을 입력하세요']"
                  lazy-rules="ondemand"
                />
              </div>
            </div>
            <div class="row q-gutter-sm">
              <div class="col">
                <q-input
                  v-model="addForm.password"
                  label="초기 비밀번호 *"
                  type="password"
                  outlined
                  dense
                  :rules="[(v) => (v?.length ?? 0) >= 8 || '8자 이상']"
                  lazy-rules="ondemand"
                />
              </div>
              <div class="col">
                <q-select
                  v-model="addForm.role"
                  :options="ROLE_OPTIONS"
                  label="권한"
                  outlined
                  dense
                  emit-value
                  map-options
                />
              </div>
            </div>
            <q-separator class="q-my-sm" />
            <div class="text-subtitle2 text-grey-7">직책 / 근무</div>
            <div class="row q-gutter-sm">
              <div class="col">
                <q-select
                  v-model="addForm.position"
                  :options="POSITION_OPTIONS"
                  label="직책"
                  outlined
                  dense
                  emit-value
                  map-options
                />
              </div>
              <div class="col">
                <q-select
                  v-model="addForm.employment_type"
                  :options="EMPLOYMENT_OPTIONS"
                  label="고용형태"
                  outlined
                  dense
                  emit-value
                  map-options
                />
              </div>
            </div>
            <q-input v-model="addForm.phone" label="연락처" outlined dense />
          </q-form>
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="추가" unelevated :loading="submitting" @click="submitAdd" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- Edit Dialog -->
    <q-dialog v-model="showEditDialog" persistent>
      <q-card style="min-width: 480px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">직원 정보 수정</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section>
          <q-form ref="editFormRef" class="q-gutter-sm">
            <div class="row q-gutter-sm">
              <div class="col">
                <q-input
                  v-model="editForm.full_name"
                  label="이름 *"
                  outlined
                  dense
                  :rules="[(v) => !!v?.trim() || '이름을 입력하세요']"
                  lazy-rules="ondemand"
                />
              </div>
              <div class="col">
                <q-input v-model="editForm.email" label="이메일" type="email" outlined dense />
              </div>
            </div>
            <div class="row q-gutter-sm">
              <div class="col">
                <q-select
                  v-model="editForm.position"
                  :options="POSITION_OPTIONS"
                  label="직책"
                  outlined
                  dense
                  emit-value
                  map-options
                />
              </div>
              <div class="col">
                <q-select
                  v-model="editForm.employment_type"
                  :options="EMPLOYMENT_OPTIONS"
                  label="고용형태"
                  outlined
                  dense
                  emit-value
                  map-options
                />
              </div>
            </div>
            <q-input v-model="editForm.phone" label="연락처" outlined dense />
            <div class="text-caption text-grey-6">
              직책·고용형태는 현재 값을 덮어씁니다. 권한(역할) 변경은 본사 인사 시스템에서 처리합니다.
            </div>
          </q-form>
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="취소" v-close-popup />
          <q-btn color="primary" label="저장" unelevated :loading="submitting" @click="submitEdit" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>
