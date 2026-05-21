<script setup lang="ts">
import { useQuasar } from "quasar";

const $q = useQuasar();

const reportTypes = [
  {
    key: "census",
    label: "Monthly Resident Census",
    description: "Total resident count, admissions, and discharges — required for AHS monthly reporting.",
    icon: "o_people",
    color: "blue",
  },
  {
    key: "care_level",
    label: "Care Level Distribution",
    description: "Breakdown of residents by DSL/LTC care level for AHS funding reconciliation.",
    icon: "o_bar_chart",
    color: "teal",
  },
  {
    key: "rai_mds",
    label: "RAI-MDS Summary",
    description: "Resident Assessment Instrument — Minimum Data Set export for CIHI submission.",
    icon: "o_assignment_turned_in",
    color: "indigo",
  },
  {
    key: "incident",
    label: "Incident Report Summary",
    description: "All critical incidents logged — required under Alberta's Continuing Care Health Service Standards.",
    icon: "o_warning",
    color: "orange",
  },
  {
    key: "medication",
    label: "Medication Administration Log",
    description: "Complete medication administration record per resident for AHS audit compliance.",
    icon: "o_medication",
    color: "green",
  },
  {
    key: "staff_attendance",
    label: "Staff Hours Report",
    description: "Shift hours by role (RN, LPN, CCA) for staffing ratio compliance reporting.",
    icon: "o_badge",
    color: "purple",
  },
  {
    key: "copayment",
    label: "Resident Co-payment Summary",
    description: "Monthly accommodation cost breakdown and resident co-payment statements.",
    icon: "o_receipt_long",
    color: "brown",
  },
];

function generateReport(key: string) {
  $q.notify({
    type: "info",
    message: "Report generation coming in Phase 2",
    icon: "o_description",
  });
}
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="text-h5 text-weight-bold">AHS & Government Reports</div>
        <div class="text-caption text-grey-6">Alberta Health Services · CIHI · Continuing Care Standards</div>
      </div>
    </div>

    <!-- Report list -->
    <q-list bordered separator class="rounded-borders q-mb-lg">
      <q-item
        v-for="report in reportTypes"
        :key="report.key"
        class="q-py-md"
      >
        <q-item-section avatar>
          <q-avatar :color="report.color" text-color="white" size="2.5rem">
            <q-icon :name="report.icon" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-weight-bold">{{ report.label }}</q-item-label>
          <q-item-label caption class="text-grey-6">{{ report.description }}</q-item-label>
        </q-item-section>
        <q-item-section side>
          <q-btn
            outline
            :color="report.color"
            icon="o_download"
            label="Generate"
            dense
            @click="generateReport(report.key)"
          />
        </q-item-section>
      </q-item>
    </q-list>

    <!-- Info card -->
    <q-card flat bordered class="bg-amber-1">
      <q-card-section class="row items-start q-gutter-md">
        <q-icon name="o_info" size="1.5rem" color="amber-8" class="q-mt-xs" />
        <div class="col">
          <div class="text-weight-bold text-amber-9 q-mb-xs">
            Alberta-compliant report export coming in Phase 2
          </div>
          <div class="text-body2 text-amber-8">
            Phase 2 will include standardized templates compliant with Alberta's Continuing Care Health Service Standards,
            AHS monthly statistical reports, CIHI RAI-MDS data submissions, and accommodation cost statements.
            Reports will be exportable as PDF and Excel.
          </div>
        </div>
      </q-card-section>
    </q-card>
  </q-page>
</template>
