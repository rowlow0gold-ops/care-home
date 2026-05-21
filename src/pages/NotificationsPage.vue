<script setup lang="ts">
import { useQuasar } from "quasar";

const $q = useQuasar();

const mockNotifications = [
  {
    id: 1,
    resident: "Margaret Thompson (Room 101)",
    message: "Care log updated: Morning bathing completed without issues.",
    type: "info",
    icon: "o_assignment_turned_in",
    color: "blue",
    time: "Today 09:15",
  },
  {
    id: 2,
    resident: "Walter Anderson (Room 106)",
    message: "Incident reported: Minor fall during morning walk. Doctor notified.",
    type: "warning",
    icon: "o_warning",
    color: "orange",
    time: "Today 11:30",
  },
  {
    id: 3,
    resident: "Eleanor Martinez (Room 105)",
    message: "Medication reminder: Evening medication due at 18:00.",
    type: "reminder",
    icon: "o_medication",
    color: "teal",
    time: "Today 17:45",
  },
];

function handleSend() {
  $q.notify({
    type: "info",
    message: "Family notifications available in Phase 2",
    icon: "o_info",
  });
}
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="text-h5 text-weight-bold">Family Notifications</div>
        <div class="text-caption text-grey-6">Keep families informed about resident care</div>
      </div>
      <div class="col-auto">
        <q-btn color="primary" icon="o_send" label="Send Notification" unelevated @click="handleSend" />
      </div>
    </div>

    <!-- Phase 2 info banner -->
    <q-banner class="bg-blue-1 text-blue-9 q-mb-lg rounded-borders" rounded>
      <template #avatar>
        <q-icon name="o_info" color="blue" />
      </template>
      <div class="text-weight-bold">Family notification via SMS/email coming in Phase 2</div>
      <div class="text-caption q-mt-xs">
        In Phase 2, this screen will allow you to send real-time SMS and email alerts to family members
        when care logs are updated, incidents occur, or medications are due.
      </div>
    </q-banner>

    <!-- Preview label -->
    <div class="text-subtitle1 text-weight-bold q-mb-md text-grey-8">
      <q-icon name="o_preview" class="q-mr-xs" />
      Preview — Example Notifications
    </div>

    <!-- Mock notification cards -->
    <div class="q-gutter-md">
      <q-card
        v-for="notif in mockNotifications"
        :key="notif.id"
        flat
        bordered
        class="rounded-borders"
      >
        <q-card-section class="row items-start q-gutter-md">
          <q-avatar :color="notif.color" text-color="white" size="2.5rem">
            <q-icon :name="notif.icon" />
          </q-avatar>
          <div class="col">
            <div class="text-weight-bold text-body2">{{ notif.resident }}</div>
            <div class="text-body2 q-mt-xs">{{ notif.message }}</div>
            <div class="text-caption text-grey-5 q-mt-xs">
              <q-icon name="o_schedule" size="0.9rem" />
              {{ notif.time }}
            </div>
          </div>
          <div class="col-auto">
            <q-chip :color="notif.color" text-color="white" dense :label="notif.type" class="text-capitalize" />
          </div>
        </q-card-section>
      </q-card>
    </div>

    <!-- Family contact note -->
    <q-card flat bordered class="q-mt-lg bg-grey-1">
      <q-card-section class="row items-center">
        <q-icon name="o_family_restroom" size="2rem" color="grey-5" class="q-mr-md" />
        <div>
          <div class="text-subtitle2 text-grey-7">Family Contact Management</div>
          <div class="text-caption text-grey-5">
            Family contact details (phone numbers, email addresses) will be managed through the Residents module in Phase 2.
          </div>
        </div>
      </q-card-section>
    </q-card>
  </q-page>
</template>
