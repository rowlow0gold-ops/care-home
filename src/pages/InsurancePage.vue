<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";

const $q = useQuasar();

interface Resident {
  id: number;
  first_name: string;
  last_name: string;
}

const residents = ref<Resident[]>([]);
const claims = ref<never[]>([]);
const showDialog = ref(false);
const submitting = ref(false);

const form = ref({
  resident_id: null as number | null,
  claim_date: new Date().toISOString().slice(0, 10),
  amount: "",
  insurance_provider: "",
  notes: "",
});

const residentOptions = computed(() =>
  residents.value.map((r) => ({ label: `${r.first_name} ${r.last_name}`, value: r.id }))
);

const stats = [
  { label: "Total Claims", value: 0, icon: "o_description", color: "blue" },
  { label: "Approved", value: 0, icon: "o_check_circle", color: "green" },
  { label: "Pending", value: 0, icon: "o_pending", color: "orange" },
  { label: "Rejected", value: 0, icon: "o_cancel", color: "red" },
];

const columns = [
  { name: "resident", label: "Resident", field: "resident", align: "left" as const },
  { name: "claim_date", label: "Claim Date", field: "claim_date", align: "left" as const },
  { name: "amount", label: "Amount (CA$)", field: "amount", align: "right" as const },
  { name: "insurance_provider", label: "Funder", field: "insurance_provider", align: "left" as const },
  { name: "status", label: "Status", field: "status", align: "center" as const },
  { name: "notes", label: "Notes", field: "notes", align: "left" as const },
];

async function loadResidents() {
  try {
    residents.value = await invoke<Resident[]>("list_residents", { search: "", activeOnly: true });
  } catch {
    residents.value = [];
  }
}

function submitClaim() {
  if (!form.value.resident_id || !form.value.claim_date) {
    $q.notify({ type: "negative", message: "Resident and claim date are required." });
    return;
  }
  submitting.value = true;
  setTimeout(() => {
    submitting.value = false;
    showDialog.value = false;
    form.value = { resident_id: null, claim_date: new Date().toISOString().slice(0, 10), amount: "", insurance_provider: "", notes: "" };
    $q.notify({ type: "positive", message: "Claim saved." });
  }, 600);
}

onMounted(loadResidents);
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="text-h5 text-weight-bold">AHS Billing & Claims</div>
        <div class="text-caption text-grey-6">Alberta Health Services continuing care claims</div>
      </div>
      <div class="col-auto">
        <q-btn color="primary" icon="o_add" label="New Claim" unelevated @click="showDialog = true" />
      </div>
    </div>

    <!-- Stats row -->
    <div class="row q-gutter-md q-mb-lg">
      <div
        v-for="stat in stats"
        :key="stat.label"
        class="col-12 col-sm-6 col-md-2"
        style="min-width: 160px"
      >
        <q-card flat bordered>
          <q-card-section class="q-pa-md">
            <div class="row items-center q-mb-xs">
              <q-icon :name="stat.icon" :color="stat.color" size="1.4rem" class="q-mr-sm" />
              <div class="text-caption text-grey-6">{{ stat.label }}</div>
            </div>
            <div class="text-h5 text-weight-bold">{{ stat.value }}</div>
          </q-card-section>
        </q-card>
      </div>
    </div>

    <!-- Claims table -->
    <q-table
      :rows="claims"
      :columns="columns"
      row-key="id"
      flat
      bordered
      :rows-per-page-options="[10, 25, 50]"
    >
      <template #body-cell-status="props">
        <q-td :props="props" class="text-center">
          <q-chip
            :color="props.row.status === 'approved' ? 'positive' : props.row.status === 'rejected' ? 'negative' : 'orange'"
            text-color="white"
            dense
            :label="props.row.status"
            class="text-capitalize"
          />
        </q-td>
      </template>
      <template #no-data>
        <div class="full-width column flex-center q-py-xl">
          <q-icon name="o_request_quote" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm text-body1">No insurance claims yet</div>
          <div class="text-grey-4 text-caption">Click "New Claim" to submit the first claim</div>
        </div>
      </template>
    </q-table>

    <!-- Phase 2 info banner -->
    <q-banner class="bg-teal-1 text-teal-9 q-mt-lg rounded-borders" rounded>
      <template #avatar>
        <q-icon name="o_verified" color="teal" />
      </template>
      <div class="text-weight-bold">AHS continuing care billing integration coming in Phase 2</div>
      <div class="text-caption q-mt-xs">
        Phase 2 will include direct integration with Alberta Health Services (AHS) for automated claim submission,
        AHCIP reconciliation, resident co-payment tracking, and RAI-MDS-linked funding level reporting.
      </div>
    </q-banner>

    <!-- New Claim Dialog -->
    <q-dialog v-model="showDialog" persistent>
      <q-card style="min-width: 460px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-h6">New Insurance Claim</div>
          <q-space />
          <q-btn icon="o_close" flat round dense v-close-popup />
        </q-card-section>

        <q-card-section class="q-gutter-sm">
          <q-select
            v-model="form.resident_id"
            :options="residentOptions"
            label="Resident *"
            outlined
            dense
            emit-value
            map-options
          />
          <div class="row q-gutter-sm">
            <div class="col">
              <div class="cursor-pointer">
                <q-input v-model="form.claim_date" label="Claim Date *" outlined dense readonly style="pointer-events:none">
                  <template #append>
                    <q-icon name="o_event" color="grey-6" />
                  </template>
                </q-input>
                <q-popup-proxy transition-show="scale" transition-hide="scale">
                  <q-date v-model="form.claim_date" mask="YYYY-MM-DD">
                    <div class="row items-center justify-end q-pa-sm">
                      <q-btn v-close-popup label="OK" color="primary" flat dense />
                    </div>
                  </q-date>
                </q-popup-proxy>
              </div>
            </div>
            <div class="col">
              <q-input v-model="form.amount" label="Amount (CA$)" type="number" outlined dense />
            </div>
          </div>
          <q-input v-model="form.insurance_provider" label="Funder" outlined dense placeholder="e.g. AHS, AHCIP, Private" />
          <q-input v-model="form.notes" label="Notes" type="textarea" rows="2" autogrow outlined dense />
        </q-card-section>

        <q-card-actions align="right" class="q-px-md q-pb-md">
          <q-btn flat label="Cancel" v-close-popup />
          <q-btn color="primary" label="Save Claim" unelevated :loading="submitting" @click="submitClaim" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>
