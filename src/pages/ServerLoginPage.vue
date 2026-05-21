<template>
  <q-page class="row items-center justify-center bg-grey-1">
    <q-card class="q-pa-lg" style="width: 460px; max-width: 95vw">
      <div class="text-h5 text-primary q-mb-xs">케어닥</div>
      <div class="text-subtitle2 text-grey-7 q-mb-lg">
        간호 스테이션 로그인 — 서버 연결 모드
      </div>

      <q-form @submit="onLogin" class="q-gutter-md">
        <q-input
          v-model="email"
          label="이메일"
          type="email"
          autocomplete="username"
          autofocus
          :rules="[(v) => !!v || '이메일을 입력하세요']"
        />
        <q-input
          v-model="password"
          label="비밀번호"
          :type="showPwd ? 'text' : 'password'"
          autocomplete="current-password"
          :rules="[(v) => !!v || '비밀번호를 입력하세요']"
        >
          <template #append>
            <q-icon
              :name="showPwd ? 'visibility_off' : 'visibility'"
              class="cursor-pointer"
              @click="showPwd = !showPwd"
            />
          </template>
        </q-input>

        <q-btn
          type="submit"
          color="primary"
          label="로그인"
          class="full-width"
          size="lg"
          :loading="loading"
        />
      </q-form>

      <q-banner v-if="error" class="bg-negative text-white q-mt-md">
        {{ error }}
      </q-banner>

      <div class="text-caption text-grey-6 q-mt-md">
        서버: {{ apiBase }}
      </div>
    </q-card>
  </q-page>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { useServerSessionStore } from "@/stores/server-session";

const email = ref("manager@demo.com");
const password = ref("");
const showPwd = ref(false);
const loading = ref(false);
const error = ref<string | null>(null);
const session = useServerSessionStore();
const router = useRouter();
const $q = useQuasar();

const apiBase =
  (import.meta.env.VITE_API_BASE as string | undefined) ??
  "https://care.minhojan-world.site";

async function onLogin() {
  if (loading.value) return;
  loading.value = true;
  error.value = null;
  try {
    const me = await session.login(email.value, password.value);
    $q.notify({
      type: "positive",
      message: `${me.name}님 환영합니다 — ${me.branch_name ?? me.tenant_name ?? ""}`,
    });
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
</script>
