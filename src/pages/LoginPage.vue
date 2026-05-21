<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useQuasar } from "quasar";

const auth = useAuthStore();
const router = useRouter();
const $q = useQuasar();

const username = ref("staff");
const password = ref("1234");
const loading = ref(false);
const showPassword = ref(false);
const loginFormRef = ref();

async function handleLogin() {
  const valid = await loginFormRef.value?.validate();
  if (!valid) return;
  loading.value = true;
  try {
    await auth.login(username.value, password.value);
    router.push("/residents");
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.message || "Invalid credentials" });
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="login-page flex flex-center">
    <div class="login-card">
      <!-- Logo -->
      <div class="login-header">
        <div class="login-logo-border">
          <div class="login-logo">
            <img src="/app-icon.png" style="width: 76px; height: 76px; object-fit: cover;" />
          </div>
        </div>
        <h1 class="login-title">Sunshine Care Home</h1>
        <p class="login-subtitle">Staff Portal</p>
      </div>

      <!-- Form -->
      <q-form ref="loginFormRef" @submit.prevent="handleLogin" class="login-form">
        <q-input
          v-model="username"
          label="Username"
          outlined
          dense
          autocomplete="username"
          class="q-mb-md"
          :rules="[v => !!v?.trim() || 'Username is required']"
          lazy-rules="ondemand"
        >
          <template #prepend>
            <q-icon name="o_person" />
          </template>
        </q-input>

        <q-input
          v-model="password"
          label="Password"
          outlined
          dense
          :type="showPassword ? 'text' : 'password'"
          autocomplete="current-password"
          class="q-mb-lg"
          :rules="[v => !!v || 'Password is required']"
          lazy-rules="ondemand"
        >
          <template #prepend>
            <q-icon name="o_lock" />
          </template>
          <template #append>
            <q-icon
              :name="showPassword ? 'o_visibility_off' : 'o_visibility'"
              class="cursor-pointer"
              @click="showPassword = !showPassword"
            />
          </template>
        </q-input>

        <q-btn
          type="submit"
          color="primary"
          label="Sign In"
          class="full-width"
          size="md"
          :loading="loading"
          unelevated
        />
      </q-form>

      <!-- Dev credentials hint -->
      <div class="dev-hint q-mt-lg">
        <div class="dev-hint-title">Dev accounts</div>
        <div class="dev-hint-row" @click="username = 'staff'; password = '1234'">
          <span class="dev-hint-user">staff</span>
          <span class="dev-hint-sep">/</span>
          <span>1234</span>
          <span class="dev-hint-badge">staff</span>
        </div>
        <div class="dev-hint-row" @click="username = 'manager'; password = '1234'">
          <span class="dev-hint-user">manager</span>
          <span class="dev-hint-sep">/</span>
          <span>1234</span>
          <span class="dev-hint-badge dev-hint-badge--manager">manager</span>
        </div>
        <div class="dev-hint-row" @click="username = 'admin'; password = '1234'">
          <span class="dev-hint-user">admin</span>
          <span class="dev-hint-sep">/</span>
          <span>1234</span>
          <span class="dev-hint-badge dev-hint-badge--admin">admin</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%);
  min-height: 100vh;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.login-card {
  background: #fff;
  border-radius: 16px;
  padding: 2.5rem 2rem;
  width: 100%;
  max-width: 380px;
  box-shadow: 0 24px 64px rgba(0,0,0,0.3);
}

.login-header {
  text-align: center;
  margin-bottom: 2rem;
}

.login-logo-border {
  width: 84px;
  height: 84px;
  border-radius: 20px;
  background: linear-gradient(225deg, #f59e0b 50%, #14b8a6 50%);
  padding: 7px;
  margin: 0 auto 1rem;
  box-shadow: 0 4px 16px rgba(0,0,0,0.25);
}
.login-logo {
  width: 100%;
  height: 100%;
  border-radius: 18px;
  background: #242f2f;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.login-title {
  font-size: 1.4rem;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 0.25rem;
}

.login-subtitle {
  color: #64748b;
  font-size: 0.9rem;
}

.dev-hint {
  border-top: 1px solid #e2e8f0;
  padding-top: 0.75rem;
  font-size: 0.78rem;
  color: #94a3b8;
}
.dev-hint-title {
  font-weight: 600;
  margin-bottom: 0.35rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-size: 0.72rem;
}
.dev-hint-row {
  display: flex;
  gap: 4px;
  padding: 2px 4px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
  font-family: monospace;
}
.dev-hint-row:hover {
  background: #f1f5f9;
  color: #475569;
}
.dev-hint-user {
  color: #475569;
  font-weight: 600;
}
.dev-hint-sep {
  color: #cbd5e1;
}
.dev-hint-badge {
  margin-left: auto;
  font-size: 0.68rem;
  background: #e2e8f0;
  color: #64748b;
  border-radius: 4px;
  padding: 1px 5px;
  font-family: sans-serif;
}
.dev-hint-badge--manager {
  background: #dbeafe;
  color: #1d4ed8;
}
.dev-hint-badge--admin {
  background: #fee2e2;
  color: #b91c1c;
}
</style>
