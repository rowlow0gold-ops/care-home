<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { server, type OrgPerson } from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const route = useRoute();
const router = useRouter();
const $q = useQuasar();
const session = useServerSessionStore();
const id = computed(() => String(route.params.id));

const canEdit = computed(() => session.canEdit);
const canDelete = computed(() => session.canDelete);

const person = ref<OrgPerson | null>(null);
const loading = ref(false);

const POSITION_OPTIONS = [
  { label: "시설장", value: "branch_manager" }, { label: "사무국장", value: "office_manager" },
  { label: "사회복지사", value: "social_worker" }, { label: "간호사", value: "nurse_rn" },
  { label: "간호조무사", value: "nurse_assistant" }, { label: "영양사", value: "dietitian" },
  { label: "물리치료사", value: "physical_therapist" }, { label: "작업치료사", value: "occupational_therapist" },
  { label: "요양보호사", value: "caregiver" }, { label: "조리원", value: "cook" },
  { label: "환경미화원", value: "cleaner" }, { label: "운전기사", value: "driver" },
  { label: "촉탁의", value: "doctor_visiting" }, { label: "사무직", value: "it" }, { label: "기타", value: "other" },
];
const EMPLOYMENT_OPTIONS = [
  { label: "정규직", value: "regular" }, { label: "계약직", value: "contract" },
  { label: "단기계약직", value: "short_contract" }, { label: "시간제", value: "part_time" },
  { label: "일용직", value: "temporary" }, { label: "파견직", value: "dispatched" }, { label: "위촉직", value: "consultant" },
];

function tenure(hired: string | null) {
  if (!hired) return "—";
  const s = new Date(hired + "T00:00:00"), n = new Date();
  let m = (n.getFullYear() - s.getFullYear()) * 12 + (n.getMonth() - s.getMonth());
  if (n.getDate() < s.getDate()) m--;
  if (m < 0) return "—";
  const y = Math.floor(m / 12), mm = m % 12;
  return y > 0 ? (mm > 0 ? `${y}년 ${mm}개월` : `${y}년`) : `${mm}개월`;
}

async function load() {
  loading.value = true;
  try { person.value = await server.staffOne(id.value); }
  catch (e: any) { $q.notify({ type: "negative", message: `직원 정보를 불러오지 못했습니다: ${e?.message ?? e}` }); }
  finally { loading.value = false; }
}

// ── Edit ──────────────────────────────────────────────────────────────────
const showEdit = ref(false);
const editRef = ref();
const submitting = ref(false);
const editForm = ref({ full_name: "", email: "", phone: "", position: "caregiver", employment_type: "regular" });
function openEdit() {
  if (!person.value) return;
  editForm.value = {
    full_name: person.value.full_name, email: person.value.email, phone: person.value.phone ?? "",
    position: person.value.position, employment_type: person.value.employment_type,
  };
  showEdit.value = true;
}
async function submitEdit() {
  if (!person.value) return;
  const valid = await editRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await server.updateStaff(person.value.id, {
      expected_updated_at: person.value.updated_at,
      full_name: editForm.value.full_name.trim(), email: editForm.value.email.trim(),
      phone: editForm.value.phone.trim() || null, position: editForm.value.position,
      employment_type: editForm.value.employment_type,
    });
    $q.notify({ type: "positive", message: "직원 정보가 수정되었습니다." });
    showEdit.value = false;
    await load();
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.status === 409 ? "다른 사용자가 먼저 수정했습니다. 새로고침 후 다시 시도하세요." : `수정 실패: ${e?.message ?? e}` });
  } finally { submitting.value = false; }
}

function confirmDeactivate() {
  if (!person.value) return;
  $q.dialog({
    title: "직원 비활성화", message: `${person.value.full_name}님을 비활성화하시겠습니까?`,
    cancel: { label: "취소", flat: true }, ok: { label: "비활성화", color: "negative", unelevated: true }, persistent: true,
  }).onOk(async () => {
    try {
      await server.deactivateStaff(person.value!.id);
      $q.notify({ type: "positive", message: "비활성화되었습니다." });
      router.push("/staff");
    } catch (e: any) { $q.notify({ type: "negative", message: `실패: ${e?.message ?? e}` }); }
  });
}

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <q-btn flat dense icon="o_arrow_back" label="직원 목록" class="q-mb-sm text-grey-7" @click="router.push('/staff')" />

    <q-card flat bordered>
      <q-card-section class="row items-center q-gutter-md">
        <q-avatar color="primary" text-color="white" icon="o_badge" size="48px" />
        <div>
          <div class="text-h6 text-weight-bold">{{ person?.full_name ?? "…" }}</div>
          <div class="text-caption text-grey-7">
            {{ person?.position_ko }} · {{ person?.employment_type_ko }} · 경력 {{ tenure(person?.hired_on ?? null) }}
          </div>
        </div>
        <q-space />
        <q-btn v-if="canEdit && person" outline color="primary" icon="o_edit" label="수정" @click="openEdit" />
        <q-btn v-if="canDelete && person" outline color="negative" icon="o_person_off" label="비활성화" @click="confirmDeactivate" />
      </q-card-section>
      <q-separator />
      <q-card-section v-if="person">
        <q-list>
          <q-item><q-item-section>이메일</q-item-section><q-item-section side>{{ person.email }}</q-item-section></q-item>
          <q-item><q-item-section>연락처</q-item-section><q-item-section side>{{ person.phone || "—" }}</q-item-section></q-item>
          <q-item><q-item-section>소속</q-item-section><q-item-section side>{{ person.branch_name ?? "—" }}</q-item-section></q-item>
          <q-item><q-item-section>입사일</q-item-section><q-item-section side>{{ person.hired_on ?? "—" }}</q-item-section></q-item>
        </q-list>
      </q-card-section>
    </q-card>

    <!-- Edit dialog -->
    <q-dialog v-model="showEdit" persistent>
      <q-card style="min-width: 480px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">직원 정보 수정</div><q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>
        <q-card-section>
          <q-form ref="editRef" class="q-gutter-sm">
            <div class="row q-gutter-sm">
              <q-input class="col" v-model="editForm.full_name" label="이름 *" outlined dense :rules="[(v)=>!!v?.trim()||'이름을 입력하세요']" lazy-rules="ondemand" />
              <q-input class="col" v-model="editForm.email" label="이메일" type="email" outlined dense />
            </div>
            <div class="row q-gutter-sm">
              <q-select class="col" v-model="editForm.position" :options="POSITION_OPTIONS" label="직책" outlined dense emit-value map-options />
              <q-select class="col" v-model="editForm.employment_type" :options="EMPLOYMENT_OPTIONS" label="고용형태" outlined dense emit-value map-options />
            </div>
            <q-input v-model="editForm.phone" label="연락처" outlined dense />
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
