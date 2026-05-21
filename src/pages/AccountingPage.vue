<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useQuasar } from "quasar";

const $q = useQuasar();

interface Invoice {
  id: number; invoice_number: string; resident_name: string;
  billing_period: string; base_fee: number; extra_charges: number;
  total_amount: number; status: string; due_date: string | null; issued_at: string;
}
interface Expense {
  id: number; category: string; description: string;
  amount: number; vendor: string | null; expense_date: string;
}
interface Summary {
  total_invoiced: number; total_collected: number;
  total_outstanding: number; total_expenses: number;
}

const tab = ref("invoices");
const invoices = ref<Invoice[]>([]);
const expenses = ref<Expense[]>([]);
const summary = ref<Summary>({ total_invoiced: 0, total_collected: 0, total_outstanding: 0, total_expenses: 0 });
const loading = ref(false);

function cad(v: number) {
  return "CA$" + v.toLocaleString("en-CA", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function statusColor(s: string) {
  return s === "paid" ? "positive" : s === "unpaid" ? "negative" : s === "partial" ? "warning" : "grey";
}

const invoiceColumns = [
  { name: "invoice_number", label: "Invoice #",      field: "invoice_number",  align: "left"   as const },
  { name: "resident_name",  label: "Resident",        field: "resident_name",   align: "left"   as const },
  { name: "billing_period", label: "Period",          field: "billing_period",  align: "left"   as const },
  { name: "total_amount",   label: "Amount",          field: "total_amount",    align: "right"  as const },
  { name: "due_date",       label: "Due Date",        field: "due_date",        align: "left"   as const },
  { name: "status",         label: "Status",          field: "status",          align: "center" as const },
];

const expenseColumns = [
  { name: "expense_date", label: "Date",        field: "expense_date", align: "left"  as const },
  { name: "category",     label: "Category",    field: "category",     align: "left"  as const },
  { name: "description",  label: "Description", field: "description",  align: "left"  as const },
  { name: "amount",       label: "Amount",      field: "amount",       align: "right" as const },
  { name: "vendor",       label: "Vendor",      field: "vendor",       align: "left"  as const },
];

const categoryColors: Record<string, string> = {
  "Medical Supplies": "blue",
  "Food & Catering":  "green",
  "Maintenance":      "orange",
  "Utilities":        "purple",
  "Equipment":        "teal",
  "Medications":      "pink",
  "Cleaning":         "cyan",
  "Administrative":   "grey",
};

async function load() {
  loading.value = true;
  try {
    const [inv, exp, sum] = await Promise.all([
      invoke<Invoice[]>("list_invoices", { status: null }),
      invoke<Expense[]>("list_expenses"),
      invoke<Summary>("get_accounting_summary"),
    ]);
    invoices.value = inv;
    expenses.value = exp;
    summary.value = sum;
  } catch (e: any) {
    $q.notify({ type: "negative", message: e });
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="text-h5 text-weight-bold">Accounting</div>
        <div class="text-caption text-grey-6">Invoices · Expenses · Payroll</div>
      </div>
      <div class="col-auto">
        <q-btn flat icon="o_refresh" dense @click="load" />
      </div>
    </div>

    <!-- Summary cards -->
    <div class="row q-gutter-md q-mb-lg">
      <div class="col-12 col-sm-6 col-md-2" style="min-width:200px">
        <q-card flat bordered>
          <q-card-section class="q-pa-md">
            <div class="row items-center q-mb-xs">
              <q-icon name="o_receipt_long" color="blue" size="1.3rem" class="q-mr-sm" />
              <span class="text-caption text-grey-6">Total Invoiced</span>
            </div>
            <div class="text-h6 text-weight-bold">{{ cad(summary.total_invoiced) }}</div>
          </q-card-section>
        </q-card>
      </div>
      <div class="col-12 col-sm-6 col-md-2" style="min-width:200px">
        <q-card flat bordered>
          <q-card-section class="q-pa-md">
            <div class="row items-center q-mb-xs">
              <q-icon name="o_payments" color="positive" size="1.3rem" class="q-mr-sm" />
              <span class="text-caption text-grey-6">Collected</span>
            </div>
            <div class="text-h6 text-weight-bold text-positive">{{ cad(summary.total_collected) }}</div>
          </q-card-section>
        </q-card>
      </div>
      <div class="col-12 col-sm-6 col-md-2" style="min-width:200px">
        <q-card flat bordered>
          <q-card-section class="q-pa-md">
            <div class="row items-center q-mb-xs">
              <q-icon name="o_pending_actions" color="warning" size="1.3rem" class="q-mr-sm" />
              <span class="text-caption text-grey-6">Outstanding</span>
            </div>
            <div class="text-h6 text-weight-bold text-warning">{{ cad(summary.total_outstanding) }}</div>
          </q-card-section>
        </q-card>
      </div>
      <div class="col-12 col-sm-6 col-md-2" style="min-width:200px">
        <q-card flat bordered>
          <q-card-section class="q-pa-md">
            <div class="row items-center q-mb-xs">
              <q-icon name="o_money_off" color="negative" size="1.3rem" class="q-mr-sm" />
              <span class="text-caption text-grey-6">Total Expenses</span>
            </div>
            <div class="text-h6 text-weight-bold text-negative">{{ cad(summary.total_expenses) }}</div>
          </q-card-section>
        </q-card>
      </div>
    </div>

    <!-- Tabs -->
    <q-tabs v-model="tab" align="left" class="q-mb-md" dense>
      <q-tab name="invoices" icon="o_receipt_long" label="Invoices" />
      <q-tab name="expenses" icon="o_money_off"    label="Expenses" />
      <q-tab name="payroll"  icon="o_payments"     label="Payroll" />
    </q-tabs>

    <q-tab-panels v-model="tab" animated>
      <!-- Invoices -->
      <q-tab-panel name="invoices" class="q-pa-none">
        <q-table :rows="invoices" :columns="invoiceColumns" row-key="id"
                 flat bordered :loading="loading" :rows-per-page-options="[15,30,50]">
          <template #body-cell-total_amount="{ row }">
            <q-td class="text-right text-weight-medium">{{ cad(row.total_amount) }}</q-td>
          </template>
          <template #body-cell-status="{ row }">
            <q-td class="text-center">
              <q-badge :color="statusColor(row.status)" :label="row.status" class="text-capitalize" />
            </q-td>
          </template>
        </q-table>
      </q-tab-panel>

      <!-- Expenses -->
      <q-tab-panel name="expenses" class="q-pa-none">
        <q-table :rows="expenses" :columns="expenseColumns" row-key="id"
                 flat bordered :loading="loading" :rows-per-page-options="[15,30,50]">
          <template #body-cell-category="{ row }">
            <q-td>
              <q-badge :color="categoryColors[row.category] || 'grey'" :label="row.category" />
            </q-td>
          </template>
          <template #body-cell-amount="{ row }">
            <q-td class="text-right text-weight-medium">{{ cad(row.amount) }}</q-td>
          </template>
        </q-table>
      </q-tab-panel>

      <!-- Payroll -->
      <q-tab-panel name="payroll" class="q-pa-none">
        <div class="column flex-center q-py-xl">
          <q-icon name="o_payments" size="3rem" color="grey-4" />
          <div class="text-grey-5 q-mt-sm">Payroll management coming in next update</div>
        </div>
      </q-tab-panel>
    </q-tab-panels>
  </q-page>
</template>
