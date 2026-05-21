<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { useAuthStore } from "@/stores/auth";
import { useSettingsStore } from "@/stores/settings";

const $q = useQuasar();
const auth = useAuthStore();
const appSettings = useSettingsStore();

// ── Facility settings ────────────────────────────────────────────────────────
const savingFacility = ref(false);
const facilityNameEdit = ref("");
const isAdmin = computed(() => auth.user?.role === "admin");

async function loadFacilitySettings() {
  facilityNameEdit.value = appSettings.facilityName;
}

async function saveFacilityName() {
  if (!isAdmin.value) return;
  $q.dialog({
    title: "Confirm name change",
    message: `Rename this facility to "${facilityNameEdit.value}"?`,
    cancel: { label: "Cancel", flat: true },
    ok:     { label: "Save",   color: "primary", unelevated: true },
    persistent: true,
  }).onOk(async () => {
    savingFacility.value = true;
    try {
      await appSettings.saveFacilityName(facilityNameEdit.value);
      $q.notify({ type: "positive", message: "Facility name updated." });
    } catch (e) {
      $q.notify({ type: "negative", message: `Failed to save: ${e}` });
    } finally {
      savingFacility.value = false;
    }
  });
}

interface User {
  id: number;
  username: string;
  full_name: string;
  role: string;
  is_active: boolean;
}

// ── Role hierarchy ──────────────────────────────────────────────────────────
const ROLE_LEVEL: Record<string, number> = {
  staff: 1,
  manager: 2,
  admin: 3,
  
};

const myLevel = computed(() => ROLE_LEVEL[auth.user?.role ?? ""] ?? 0);

// Roles the current user is allowed to create / delete (strictly lower level)
const manageableRoles = computed(() =>
  Object.entries(ROLE_LEVEL)
    .filter(([, lvl]) => lvl < myLevel.value)
    .map(([role]) => role)
);

const roleOptions = computed(() =>
  [
    { label: "Staff",   value: "staff" },
    { label: "Manager",     value: "manager" },
    { label: "Admin",       value: "admin" },
    
  ].filter(opt => manageableRoles.value.includes(opt.value))
);

const roleColor: Record<string, string> = {
  staff:   "grey-7",
  manager:     "blue",
  admin:       "red",
  
};

// ── State ───────────────────────────────────────────────────────────────────
const tab = ref("facility");
const users = ref<User[]>([]);
const loading = ref(false);
const showAddUserDialog = ref(false);
const submitting = ref(false);
const confirmDeleteId = ref<number | null>(null);
const deleting = ref(false);

const form = ref({
  username: "",
  password: "",
  full_name: "",
  role: "",
});

// Reset role default whenever manageable roles change
function resetForm() {
  form.value = {
    username: "",
    password: "",
    full_name: "",
    role: roleOptions.value[0]?.value ?? "",
  };
}

// ── Table columns ────────────────────────────────────────────────────────────
const userColumns = [
  { name: "full_name", label: "Full Name", field: "full_name", align: "left" as const },
  { name: "username",  label: "Username",  field: "username",  align: "left" as const },
  { name: "role",      label: "Role",      field: "role",      align: "left" as const },
  { name: "actions",   label: "",          field: "actions",   align: "right" as const },
];

// Only show rows that the current user can manage (+ their own row for reference)
const visibleUsers = computed(() =>
  users.value.filter(u =>
    manageableRoles.value.includes(u.role) || u.id === auth.user?.id
  )
);

// ── API calls ────────────────────────────────────────────────────────────────
async function loadUsers() {
  loading.value = true;
  try {
    users.value = await invoke<User[]>("list_users");
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to load users: ${e}` });
  } finally {
    loading.value = false;
  }
}

async function submitAddUser() {
  if (!form.value.username || !form.value.password || !form.value.full_name || !form.value.role) {
    $q.notify({ type: "negative", message: "All fields are required." });
    return;
  }
  submitting.value = true;
  try {
    await invoke("create_user", {
      input: {
        username: form.value.username,
        password: form.value.password,
        full_name: form.value.full_name,
        role: form.value.role,
      },
      actorRole: auth.user?.role,
    });
    $q.notify({ type: "positive", message: "User created successfully." });
    showAddUserDialog.value = false;
    resetForm();
    await loadUsers();
  } catch (e) {
    $q.notify({ type: "negative", message: `Failed to create user: ${e}` });
  } finally {
    submitting.value = false;
  }
}

async function confirmDelete(userId: number) {
  confirmDeleteId.value = userId;
}

async function doDelete() {
  if (confirmDeleteId.value === null) return;
  deleting.value = true;
  try {
    await invoke("delete_user", {
      userId: confirmDeleteId.value,
      actorId: auth.user?.id,
      actorRole: auth.user?.role,
    });
    $q.notify({ type: "positive", message: "User deactivated." });
    confirmDeleteId.value = null;
    await loadUsers();
  } catch (e) {
    $q.notify({ type: "negative", message: `${e}` });
  } finally {
    deleting.value = false;
  }
}

function canManage(user: User) {
  return (
    user.id !== auth.user?.id &&
    manageableRoles.value.includes(user.role)
  );
}

onMounted(async () => {
  loadUsers();
  resetForm();
  await appSettings.load();
  loadFacilitySettings();
});
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="text-h5 text-weight-bold">Settings</div>
        <div class="text-caption text-grey-6">Application settings and user management</div>
      </div>
    </div>

    <q-tabs
      v-model="tab"
      align="left"
      indicator-color="primary"
      active-color="primary"
      class="q-mb-md"
    >
      <q-tab name="facility" label="Facility"       icon="o_home_work" />
      <q-tab name="users"    label="Login Accounts" icon="o_manage_accounts" />
      <q-tab name="about"    label="About"          icon="o_info" />
    </q-tabs>

    <q-tab-panels v-model="tab" animated>

      <!-- ── Facility tab ─────────────────────────────────────────────────── -->
      <q-tab-panel name="facility" class="q-pa-none">
        <div class="row q-gutter-lg">
          <div class="col-12 col-md-5">
            <q-card flat bordered>
              <q-card-section>
                <div class="text-h6 q-mb-md">Facility Settings</div>

                <div class="text-subtitle2 text-grey-7 q-mb-xs">Facility Name</div>
                <div v-if="!isAdmin" class="text-caption text-amber-8 q-mb-xs">
                  <q-icon name="o_lock" size="xs" class="q-mr-xs" />Only admins can change the facility name.
                </div>
                <q-input
                  v-model="facilityNameEdit"
                  outlined dense
                  class="q-mb-lg"
                  placeholder="e.g. Sunrise Care Home"
                  :readonly="!isAdmin"
                  :bg-color="isAdmin ? undefined : 'grey-2'"
                />

                <q-btn
                  v-if="isAdmin"
                  color="primary"
                  label="Save Name"
                  icon="o_save"
                  unelevated
                  :loading="savingFacility"
                  @click="saveFacilityName"
                />
              </q-card-section>
            </q-card>
          </div>
        </div>
      </q-tab-panel>

      <!-- ── Login Accounts tab ────────────────────────────────────────── -->
      <q-tab-panel name="users" class="q-pa-none">
        <div class="row items-center q-mb-sm">
          <div class="col">
            <div class="text-h6">Login Accounts</div>
            <div class="text-caption text-grey-6">
              Manage usernames, passwords and roles for app access.
              For employee profiles (department, position, hire date etc.) go to the
              <strong>Staff</strong> page instead.
            </div>
          </div>
          <div class="col-auto">
            <q-btn
              color="primary"
              icon="o_person_add"
              label="Add User"
              unelevated
              :disable="roleOptions.length === 0"
              @click="resetForm(); showAddUserDialog = true"
            />
          </div>
        </div>

        <q-table
          :rows="visibleUsers"
          :columns="userColumns"
          row-key="id"
          :loading="loading"
          flat
          bordered
          :rows-per-page-options="[10, 25, 50]"
        >
          <template #body-cell-role="props">
            <q-td :props="props">
              <q-chip
                :color="roleColor[props.row.role] || 'grey'"
                text-color="white"
                dense
                :label="props.row.role.replace('_', ' ')"
                class="text-capitalize"
              />
            </q-td>
          </template>

          <template #body-cell-actions="props">
            <q-td :props="props" class="text-right">
              <q-btn
                v-if="canManage(props.row)"
                flat
                round
                dense
                icon="o_delete"
                color="negative"
                @click="confirmDelete(props.row.id)"
              >
                <q-tooltip>Deactivate user</q-tooltip>
              </q-btn>
              <span v-else-if="props.row.id === auth.user?.id" class="text-caption text-grey-5">you</span>
            </q-td>
          </template>

          <template #no-data>
            <div class="full-width column flex-center q-py-xl">
              <q-icon name="o_group" size="3rem" color="grey-4" />
              <div class="text-grey-5 q-mt-sm">No users found</div>
            </div>
          </template>
        </q-table>
      </q-tab-panel>

      <!-- ── About tab ──────────────────────────────────────────────────── -->
      <q-tab-panel name="about" class="q-pa-none">
        <div class="row q-gutter-lg">
          <div class="col-12 col-md-6">
            <q-card flat bordered>
              <q-card-section>
                <div class="row items-center q-mb-md">
                  <q-icon name="o_local_hospital" size="2.5rem" color="primary" class="q-mr-md" />
                  <div>
                    <div class="text-h6 text-weight-bold">Sunshine Care Home</div>
                    <div class="text-caption text-grey-6">Care Home Management System</div>
                  </div>
                </div>
                <q-separator class="q-my-md" />
                <q-list dense>
                  <q-item>
                    <q-item-section>
                      <q-item-label caption>Version</q-item-label>
                      <q-item-label>0.1.0</q-item-label>
                    </q-item-section>
                  </q-item>
                  <q-item>
                    <q-item-section>
                      <q-item-label caption>Build</q-item-label>
                      <q-item-label>Phase 1 — Local Desktop App</q-item-label>
                    </q-item-section>
                  </q-item>
                  <q-item>
                    <q-item-section>
                      <q-item-label caption>Platform</q-item-label>
                      <q-item-label>Tauri 2 + Vue 3 + Quasar</q-item-label>
                    </q-item-section>
                  </q-item>
                  <q-item>
                    <q-item-section>
                      <q-item-label caption>Database</q-item-label>
                      <q-item-label>SQLite (local)</q-item-label>
                    </q-item-section>
                  </q-item>
                </q-list>
              </q-card-section>
            </q-card>
          </div>

          <div class="col-12 col-md-5">
            <q-card flat bordered class="bg-blue-1">
              <q-card-section>
                <div class="row items-center q-mb-sm">
                  <q-icon name="o_info" color="blue" class="q-mr-sm" />
                  <span class="text-weight-bold text-blue-8">Phase 1 — Local Only</span>
                </div>
                <div class="text-body2 text-blue-9">
                  This version runs entirely on your local machine. All data is stored in a local SQLite database.
                  No network connectivity or cloud sync is required.
                </div>
                <q-separator class="q-my-md" />
                <div class="text-caption text-blue-7 text-weight-bold q-mb-xs">Phase 2 Features (Planned)</div>
                <q-list dense class="text-caption text-blue-8">
                  <q-item dense class="q-pa-none">
                    <q-item-section avatar><q-icon name="o_cloud" size="1rem" /></q-item-section>
                    <q-item-section>Cloud backup &amp; sync</q-item-section>
                  </q-item>
                  <q-item dense class="q-pa-none">
                    <q-item-section avatar><q-icon name="o_sms" size="1rem" /></q-item-section>
                    <q-item-section>Family SMS/email notifications</q-item-section>
                  </q-item>
                  <q-item dense class="q-pa-none">
                    <q-item-section avatar><q-icon name="o_description" size="1rem" /></q-item-section>
                    <q-item-section>AHS &amp; CIHI reporting (RAI-MDS)</q-item-section>
                  </q-item>
                  <q-item dense class="q-pa-none">
                    <q-item-section avatar><q-icon name="o_photo_library" size="1rem" /></q-item-section>
                    <q-item-section>Photo &amp; video records</q-item-section>
                  </q-item>
                </q-list>
              </q-card-section>
            </q-card>
          </div>
        </div>
      </q-tab-panel>
    </q-tab-panels>

    <!-- ── Add User Dialog ────────────────────────────────────────────────── -->
    <q-dialog v-model="showAddUserDialog" persistent>
      <q-card style="min-width: 420px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">Add User</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section class="q-gutter-sm">
          <q-input v-model="form.full_name" label="Full Name *" outlined dense />
          <q-input v-model="form.username"  label="Username *"  outlined dense />
          <q-input v-model="form.password"  label="Password *"  type="password" outlined dense />
          <q-select
            v-model="form.role"
            :options="roleOptions"
            label="Role"
            outlined
            dense
            emit-value
            map-options
          />
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn
            color="primary"
            label="Create User"
            unelevated
            :loading="submitting"
            @click="submitAddUser"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- ── Delete Confirmation Dialog ────────────────────────────────────── -->
    <q-dialog :model-value="confirmDeleteId !== null" persistent @update:model-value="val => { if (!val) confirmDeleteId = null }">
      <q-card style="min-width: 340px">
        <q-card-section class="row items-center">
          <q-avatar icon="o_warning" color="negative" text-color="white" />
          <span class="q-ml-sm text-weight-medium">Deactivate this account?</span>
        </q-card-section>
        <q-card-section class="text-grey-7">
          The user will no longer be able to log in. This action can be reversed by an admin directly in the database.
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="Cancel" @click="confirmDeleteId = null" />
          <q-btn
            color="negative"
            label="Deactivate"
            unelevated
            :loading="deleting"
            @click="doDelete"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>
