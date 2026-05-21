<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";
import { useAuthStore } from "@/stores/auth";

const $q   = useQuasar();
const auth = useAuthStore();
const isStaff = computed(() => auth.user?.role === "staff")

// Admin: can act on anyone (including themselves).
// Manager: can only act on staff-role accounts (not self, not other managers, not admin).
function canActOn(row: Staff): boolean {
  if (auth.user?.role === "admin") return true;
  if (row.user_id === auth.user?.id) return false;  // manager can't touch self
  return row.role === "staff";                       // manager can only touch staff
};

interface Staff {
  id: number;
  user_id: number | null;
  employee_id: string;
  full_name: string;
  role: string;
  department: string;
  position: string;
  phone: string;
  email: string;
  hire_date: string;
  hourly_rate: number | null;
  is_active: boolean;
}

const staff = ref<Staff[]>([]);
const loading = ref(false);
const showAddDialog  = ref(false);
const showEditDialog = ref(false);
const addStaffFormRef  = ref();
const editStaffFormRef = ref();
const submitting       = ref(false);
const selectedStaff = ref<Staff | null>(null);

const addForm = ref({
  full_name: "",
  username: "",
  password: "",
  role: "staff",
  employee_id: "",
  department: "",
  position: "",
  hire_date: "",
  phone: "",
  email: "",
  hourly_rate: "",
});

const editForm = ref({
  department: "",
  position: "",
  hire_date: "",
  phone: "",
  email: "",
  hourly_rate: "",
});

const roleOptions = [
  { label: "Staff", value: "staff" },
  { label: "Manager", value: "manager" },
  { label: "Admin", value: "admin" },
  
];

const roleColor: Record<string, string> = {
  admin: "red",
  staff: "grey-7",
  manager: "blue",
  
};

const columns = [
  { name: "employee_id", label: "Employee ID", field: "employee_id", align: "left" as const },
  { name: "full_name", label: "Name", field: "full_name", align: "left" as const },
  { name: "role", label: "Role", field: "role", align: "left" as const },
  { name: "department", label: "Department", field: "department", align: "left" as const },
  { name: "position", label: "Position", field: "position", align: "left" as const },
  { name: "phone", label: "Phone", field: "phone", align: "left" as const },
  { name: "hire_date", label: "Hire Date", field: "hire_date", align: "left" as const },
  { name: "actions", label: "Actions", field: "actions", align: "center" as const },
];

// Employee ID format: digits only, 7 chars zero-padded (e.g. 0000001)
const ruleEmployeeId = (v: string) => {
  if (!v?.trim()) return true;                             // optional
  if (!/^\d+$/.test(v.trim())) return "Digits only (e.g. 0000001)";
  return true;
};

function nextEmployeeId(): string {
  const nums = staff.value
    .map(s => s.employee_id?.trim())
    .filter(id => id && /^\d+$/.test(id))
    .map(id => parseInt(id!, 10));
  const max = nums.length ? Math.max(...nums) : 0;
  return String(max + 1).padStart(7, "0");
}

// Returns true if employee_id is already used by another staff member
function isEmployeeIdTaken(empId: string, excludeId?: number): boolean {
  if (!empId?.trim()) return false;
  return staff.value.some(s =>
    s.employee_id?.trim() === empId.trim() &&
    (excludeId === undefined || s.id !== excludeId)
  );
}

async function loadStaff() {
  loading.value = true;
  try {
    staff.value = await invoke<Staff[]>("list_staff", { activeOnly: true });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `Failed to load staff: ${e}` });
  } finally {
    loading.value = false;
  }
}

async function submitAdd() {
  const valid = await addStaffFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await invoke("create_staff_member", {
      input: {
        full_name: addForm.value.full_name,
        username: addForm.value.username,
        password: addForm.value.password,
        role: addForm.value.role,
        employee_id: addForm.value.employee_id,
        department: addForm.value.department,
        position: addForm.value.position,
        hire_date: addForm.value.hire_date || null,
        phone: addForm.value.phone,
        email: addForm.value.email,
        hourly_rate: addForm.value.hourly_rate ? parseFloat(addForm.value.hourly_rate) : null,
      },
    });
    $q.notify({ type: "positive", message: "Staff member added." });
    showAddDialog.value = false;
    addForm.value = { full_name: "", username: "", password: "", role: "staff", employee_id: "", department: "", position: "", hire_date: "", phone: "", email: "", hourly_rate: "" };
    await loadStaff();
  } catch (e: any) {
    const msg = String(e);
    if (msg.includes("UNIQUE") && msg.includes("employee_id")) {
      $q.notify({ type: "negative", message: "Employee ID already exists. Please use a unique ID." });
    } else if (msg.includes("UNIQUE") && msg.includes("username")) {
      $q.notify({ type: "negative", message: "Username already taken. Please choose another." });
    } else {
      $q.notify({ type: "negative", message: `Failed to add staff: ${e}` });
    }
  } finally {
    submitting.value = false;
  }
}

function openEdit(s: Staff) {
  selectedStaff.value = s;
  editForm.value = {
    department: s.department,
    position: s.position,
    hire_date: s.hire_date,
    phone: s.phone,
    email: s.email,
    hourly_rate: s.hourly_rate?.toString() || "",
  };
  showEditDialog.value = true;
}

async function submitEdit() {
  if (!selectedStaff.value) return;
  const valid = await editStaffFormRef.value?.validate();
  if (!valid) return;
  submitting.value = true;
  try {
    await invoke("update_staff_member", {
      id: selectedStaff.value.id,
      input: {
        full_name: selectedStaff.value.full_name,
        department: editForm.value.department,
        position: editForm.value.position,
        hire_date: editForm.value.hire_date || null,
        phone: editForm.value.phone,
        email: editForm.value.email,
        hourly_rate: editForm.value.hourly_rate ? parseFloat(editForm.value.hourly_rate) : null,
      },
      actorId: auth.user?.id,
      actorRole: auth.user?.role,
    });
    $q.notify({ type: "positive", message: "Staff profile updated." });
    showEditDialog.value = false;
    await loadStaff();
  } catch (e: any) {
    const msg = String(e);
    if (msg.includes("UNIQUE") && msg.includes("employee_id")) {
      $q.notify({ type: "negative", message: "Employee ID already exists. Please use a unique ID." });
    } else {
      $q.notify({ type: "negative", message: `Failed to update: ${e}` });
    }
  } finally {
    submitting.value = false;
  }
}

function toggleRole(s: Staff) {
  const newRole = s.role === "manager" ? "staff" : "manager";
  const action  = s.role === "manager" ? "Demote to Staff" : "Promote to Manager";
  $q.dialog({
    title: action,
    message: `${action}: ${s.full_name}?`,
    cancel: true,
    persistent: true,
  }).onOk(async () => {
    try {
      await invoke("set_staff_role", { id: s.id, newRole, actorId: auth.user?.id, actorRole: auth.user?.role });
      $q.notify({ type: "positive", message: `${s.full_name} is now ${newRole}.` });
      await loadStaff();
    } catch (e) {
      $q.notify({ type: "negative", message: `Failed: ${e}` });
    }
  });
}

function confirmDeactivate(s: Staff) {
  $q.dialog({
    title: "Deactivate Staff",
    message: `Deactivate ${s.full_name}? They will no longer be able to log in.`,
    cancel: true,
    persistent: true,
  }).onOk(async () => {
    try {
      await invoke("deactivate_staff", { id: s.id, actorId: auth.user?.id, actorRole: auth.user?.role });
      $q.notify({ type: "positive", message: "Staff deactivated." });
      await loadStaff();
    } catch (e) {
      $q.notify({ type: "negative", message: `Failed: ${e}` });
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
        <div class="text-h5 text-weight-bold">Staff Directory</div>
        <div class="text-caption text-grey-6">Manage care home staff members</div>
      </div>
      <div v-if="!isStaff" class="col-auto">
        <q-btn color="primary" icon="o_person_add" label="Add Staff" unelevated @click="() => { addForm.employee_id = nextEmployeeId(); showAddDialog = true; }" />
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
      :loading="false"
      flat
      bordered
      :rows-per-page-options="[10, 25, 50]"
    >
      <template #body-cell-role="props">
        <q-td :props="props">
          <q-badge :color="roleColor[props.row.role] || 'grey'" :label="props.row.role" class="text-capitalize" />
        </q-td>
      </template>

      <template #body-cell-actions="props">
        <q-td :props="props" class="text-center">
          <!-- Role toggle: admin only, for non-admin non-self rows -->
          <q-btn
            v-if="auth.user?.role === 'admin' && props.row.user_id !== auth.user?.id && props.row.role !== 'admin'"
            flat round dense
            :icon="props.row.role === 'manager' ? 'o_arrow_downward' : 'o_arrow_upward'"
            :color="props.row.role === 'manager' ? 'orange' : 'teal'"
            @click="toggleRole(props.row)"
          >
            <q-tooltip>{{ props.row.role === 'manager' ? 'Demote to Staff' : 'Promote to Manager' }}</q-tooltip>
          </q-btn>
          <template v-if="!isStaff && canActOn(props.row)">
            <q-btn flat round dense icon="o_edit" color="primary" @click="openEdit(props.row)">
              <q-tooltip>Edit profile</q-tooltip>
            </q-btn>
            <q-btn flat round dense icon="o_person_off" color="negative" @click="confirmDeactivate(props.row)">
              <q-tooltip>Deactivate</q-tooltip>
            </q-btn>
          </template>
          <span v-else-if="isStaff" class="text-caption text-grey-4">—</span>
        </q-td>
      </template>

      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_group" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">No staff members found</div>
        </div>
      </template>
    </q-table>

    <!-- Add Staff Dialog -->
    <q-dialog v-model="showAddDialog" persistent>
      <q-card style="min-width: 540px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">Add Staff Member</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section>
          <q-form ref="addStaffFormRef" class="q-gutter-sm">
          <div class="text-subtitle2 text-grey-7">Account Info</div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="addForm.full_name" label="Full Name *" outlined dense
                :rules="[v => !!v?.trim() || 'Full name is required']" lazy-rules="ondemand" />
            </div>
            <div class="col">
              <q-input v-model="addForm.username" label="Username *" outlined dense
                :rules="[v => !!v?.trim() || 'Username is required']" lazy-rules="ondemand" />
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="addForm.password" label="Password *" type="password" outlined dense
                :rules="[v => !!v || 'Password is required', v => v?.length >= 4 || 'At least 4 characters']"
                lazy-rules="ondemand" />
            </div>
            <div class="col">
              <q-select
                v-model="addForm.role"
                :options="roleOptions"
                label="Role"
                outlined
                dense
                emit-value
                map-options
              />
            </div>
          </div>
          <q-separator class="q-my-sm" />
          <div class="text-subtitle2 text-grey-7">Profile Info</div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="addForm.employee_id" label="Employee ID" outlined dense readonly
                bg-color="grey-2"
                hint="Auto-assigned" />
            </div>
            <div class="col">
              <q-input v-model="addForm.department" label="Department" outlined dense />
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="addForm.position" label="Position" outlined dense />
            </div>
            <div class="col">
              <div class="cursor-pointer">
                <q-input v-model="addForm.hire_date" label="Hire Date" outlined dense readonly style="pointer-events:none">
                  <template #append>
                    <q-icon name="o_event" color="grey-6" />
                  </template>
                </q-input>
                <q-popup-proxy transition-show="scale" transition-hide="scale">
                  <q-date v-model="addForm.hire_date" mask="YYYY-MM-DD">
                    <div class="row items-center justify-end q-pa-sm">
                      <q-btn v-close-popup label="OK" color="primary" flat dense />
                    </div>
                  </q-date>
                </q-popup-proxy>
              </div>
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="addForm.phone" label="Phone" outlined dense />
            </div>
            <div class="col">
              <q-input v-model="addForm.email" label="Email" outlined dense />
            </div>
          </div>
          <q-input v-model="addForm.hourly_rate" label="Hourly Rate (₩)" type="number" outlined dense />
          </q-form>
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Add Staff" unelevated :loading="submitting" @click="submitAdd" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- Edit Dialog -->
    <q-dialog v-model="showEditDialog" persistent>
      <q-card style="min-width: 480px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">Edit Staff Profile</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section>
          <q-form ref="editStaffFormRef" class="q-gutter-sm">
          <div class="row q-gutter-sm items-center q-mb-xs">
            <div class="col-auto text-caption text-grey-6">Employee ID</div>
            <div class="col-auto text-weight-bold text-mono">{{ selectedStaff?.employee_id || '—' }}</div>
            <q-badge color="grey-4" text-color="grey-8" label="fixed" class="q-ml-xs" style="font-size:10px" />
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="editForm.department" label="Department" outlined dense />
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="editForm.position" label="Position" outlined dense />
            </div>
            <div class="col">
              <div class="cursor-pointer">
                <q-input v-model="editForm.hire_date" label="Hire Date" outlined dense readonly style="pointer-events:none">
                  <template #append>
                    <q-icon name="o_event" color="grey-6" />
                  </template>
                </q-input>
                <q-popup-proxy transition-show="scale" transition-hide="scale">
                  <q-date v-model="editForm.hire_date" mask="YYYY-MM-DD">
                    <div class="row items-center justify-end q-pa-sm">
                      <q-btn v-close-popup label="OK" color="primary" flat dense />
                    </div>
                  </q-date>
                </q-popup-proxy>
              </div>
            </div>
          </div>
          <div class="row q-gutter-sm">
            <div class="col">
              <q-input v-model="editForm.phone" label="Phone" outlined dense />
            </div>
            <div class="col">
              <q-input v-model="editForm.email" label="Email" outlined dense />
            </div>
          </div>
          <q-input v-model="editForm.hourly_rate" label="Hourly Rate (₩)" type="number" outlined dense />
          </q-form>
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Update" unelevated :loading="submitting" @click="submitEdit" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>
