<script setup lang="ts">
const faqs = [
  {
    icon: "o_person_add",
    question: "How to add a resident?",
    answer: "Navigate to the Residents page from the left sidebar. Click \"Add Resident\". Fill in the required fields: full name, date of birth, care level, and room number. Enter the resident's Personal Health Number (PHN) for AHCIP billing. Optionally add diagnosis, allergies, dietary restrictions, and admission date. Click \"Save\" to create the record.",
  },
  {
    icon: "o_assignment",
    question: "How to record care logs?",
    answer: "Go to Care Log. Click \"New Entry\" and select the resident, shift (Morning / Afternoon / Night), and category (Bathing, Meals, Medication, Mood, Incident, or Note). Type your notes in the text area. If the event is an incident, check \"Mark as Incident\". Click \"Save\" to submit.",
  },
  {
    icon: "o_medication",
    question: "How to manage medications?",
    answer: "Open Medications. Active prescriptions are shown in the table. To add one, click \"Add Medication\", fill in the resident, drug name, dosage, frequency, route, and prescriber, then save. To stop a medication, click the stop button on the row and confirm.",
  },
  {
    icon: "o_badge",
    question: "How to add staff?",
    answer: "Managers and admins can add staff. Go to the Staff page and click \"Add Staff\".\n\nFill in two sections:\n• Account info: full name, username, password, and role (Staff / Manager / Admin)\n• Profile details: employee ID, department, position, hire date, phone, and hourly rate\n\nClick \"Add Staff\" to save. The account is created immediately and the person can log in with the username and password you set.\n\nNote: you can only assign roles lower than your own. A manager can create Staff accounts; an admin can create Staff or Manager accounts.",
  },
  {
    icon: "o_star",
    question: "What are Care Levels?",
    answer: "Care levels in Alberta reflect a resident's designation under Alberta Health Services (AHS) Continuing Care:\n\nLevel 1 — Minimal needs, largely independent. Home care or lodge setting.\nLevel 2 — Moderate needs, some daily assistance required.\nLevel 3 — Designated Supportive Living (DSL3): 24-hour on-site support, no 24-hour nursing.\nLevel 4 — Designated Supportive Living (DSL4): 24-hour nursing care on-site.\nLevel 5 / LTC — Long-Term Care (LTC): highest complexity, full 24-hour nursing and personal care for all daily needs.",
  },
  {
    icon: "o_lock_reset",
    question: "How to reset a user password?",
    answer: "Managers and admins can manage accounts in Settings → Users tab.\n\nTo reset a password: deactivate the account using the delete button, then create a new account with the same name and a new password. A dedicated password-reset feature is planned for a future update.\n\nRole permissions:\n• Manager — can create and deactivate Staff accounts\n• Admin — can create and deactivate Staff and Manager accounts",
  },
];
</script>

<template>
  <q-page class="q-pa-lg">
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="text-h5 text-weight-bold">Help &amp; Documentation</div>
        <div class="text-caption text-grey-6">Frequently asked questions and guides</div>
      </div>
    </div>

    <div class="row q-gutter-sm q-mb-lg">
      <q-chip icon="o_person_add" label="Residents" color="blue" text-color="white" dense />
      <q-chip icon="o_assignment" label="Care Logs" color="teal" text-color="white" dense />
      <q-chip icon="o_medication" label="Medications" color="green" text-color="white" dense />
      <q-chip icon="o_badge" label="Staff" color="purple" text-color="white" dense />
      <q-chip icon="o_star" label="Care Levels" color="orange" text-color="white" dense />
    </div>

    <q-list bordered class="rounded-borders q-mb-lg">
      <q-expansion-item
        v-for="faq in faqs"
        :key="faq.question"
        :icon="faq.icon"
        :label="faq.question"
        header-class="text-weight-medium"
        expand-separator
      >
        <q-card>
          <q-card-section class="bg-grey-1">
            <div class="text-body2 text-grey-8" style="white-space: pre-line; line-height: 1.7">{{ faq.answer }}</div>
          </q-card-section>
        </q-card>
      </q-expansion-item>
    </q-list>

    <q-card flat bordered>
      <q-card-section class="row items-center q-gutter-md">
        <q-avatar color="teal" text-color="white" size="3rem">
          <q-icon name="o_support_agent" />
        </q-avatar>
        <div class="col">
          <div class="text-subtitle1 text-weight-bold">Need more help?</div>
          <div class="text-body2 text-grey-7">Contact your manager. The manager will help you out.</div>
        </div>
      </q-card-section>
    </q-card>
  </q-page>
</template>
