<template>
  <q-layout view="lHh Lpr lFf">
    <q-page-container>
      <q-page class="flex flex-center bg-grey-1 q-pa-lg">
        <q-card flat bordered class="q-pa-lg" style="width: 420px; max-width: 95vw">
          <!-- Brand -->
          <div class="column items-center q-mb-lg">
            <q-avatar rounded size="56px" class="brand-mark q-mb-sm">
              <q-icon name="favorite" color="white" size="28px" />
            </q-avatar>
            <div class="text-h6 text-weight-bold">케어닥 스테이션</div>
          </div>

          <!-- Form -->
          <q-form @submit="onLogin" class="q-gutter-md">
            <q-input
              v-model="email"
              label="이메일"
              type="email"
              autocomplete="username"
              outlined
              dense
              :rules="[(v) => !!v || '이메일을 입력하세요']"
            >
              <template #prepend><q-icon name="mail" /></template>
            </q-input>

            <q-input
              v-model="password"
              label="비밀번호"
              :type="showPwd ? 'text' : 'password'"
              autocomplete="current-password"
              outlined
              dense
              :rules="[(v) => !!v || '비밀번호를 입력하세요']"
            >
              <template #prepend><q-icon name="lock" /></template>
              <template #append>
                <q-icon
                  :name="showPwd ? 'visibility_off' : 'visibility'"
                  class="cursor-pointer"
                  @click="showPwd = !showPwd"
                />
              </template>
            </q-input>

            <q-banner v-if="error" dense class="bg-red-1 text-negative rounded-borders">
              {{ error }}
            </q-banner>

            <q-btn
              type="submit"
              color="primary"
              label="로그인"
              class="full-width"
              size="lg"
              unelevated
              :loading="loading"
            />
          </q-form>

          <!-- Quick demo login -->
          <q-separator class="q-my-md" />
          <div class="text-caption text-grey-6 q-mb-sm">
            데모 빠른 로그인 (비밀번호 {{ DEMO_PW }})
          </div>
          <div class="row q-gutter-xs">
            <q-btn
              v-for="a in demoAccounts"
              :key="a.email"
              outline
              no-caps
              size="sm"
              color="primary"
              :label="a.label"
              @click="pickAndLogin(a.email)"
            />
          </div>
        </q-card>
      </q-page>
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { useServerSessionStore } from "@/stores/server-session";

const DEMO_PW = "admin1234";

const email = ref("admin.seoul-hub@demo.com");
const password = ref(DEMO_PW);
const showPwd = ref(false);
const loading = ref(false);
const error = ref<string | null>(null);
const session = useServerSessionStore();
const router = useRouter();
const $q = useQuasar();

// Demo accounts — 서울광역센터 행정 / 접수. All share password admin1234.
const demoAccounts = [
  { label: "행정", email: "admin.seoul-hub@demo.com" },
  { label: "접수", email: "reception.seoul-hub@demo.com" },
];

async function onLogin() {
  if (loading.value) return;
  loading.value = true;
  error.value = null;
  try {
    const me = await session.login(email.value, password.value);
    $q.notify({ type: "positive", message: `${me.name}님 환영합니다` });
    router.replace("/");
  } catch (err: any) {
    error.value =
      err?.status === 401
        ? "이메일이나 비밀번호가 올바르지 않습니다."
        : err?.message ?? "로그인 실패";
  } finally {
    loading.value = false;
  }
}

function pickAndLogin(addr: string) {
  email.value = addr;
  password.value = DEMO_PW;
  onLogin();
}
</script>

<style scoped>
.brand-mark {
  background: linear-gradient(135deg, #21ba45 0%, #15a03799 100%);
  box-shadow: 0 8px 20px rgba(33, 186, 69, 0.25);
}
</style>
