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
const filterResident = ref<number | null>(null);

const residentOptions = computed(() =>
  residents.value.map((r) => ({ label: `${r.first_name} ${r.last_name}`, value: r.id }))
);

async function loadResidents() {
  try {
    residents.value = await invoke<Resident[]>("list_residents", { search: "", activeOnly: true });
  } catch {
    residents.value = [];
  }
}

function handleUpload() {
  $q.notify({
    type: "info",
    message: "Photo/video upload available in Phase 2",
    icon: "o_photo_camera",
  });
}

onMounted(loadResidents);
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- Header -->
    <div class="row items-center q-mb-lg">
      <div class="col">
        <div class="row items-center q-gutter-sm">
          <div class="text-h5 text-weight-bold">Media Records</div>
          <q-chip color="deep-orange" text-color="white" dense icon="o_schedule" label="Phase 2" />
        </div>
        <div class="text-caption text-grey-6">Photo and video records for residents</div>
      </div>
      <div class="col-auto">
        <q-btn
          color="primary"
          icon="o_upload"
          label="Upload"
          unelevated
          @click="handleUpload"
        />
      </div>
    </div>

    <!-- Filter -->
    <div class="row q-mb-lg">
      <div class="col-12 col-md-4">
        <q-select
          v-model="filterResident"
          :options="residentOptions"
          label="Filter by Resident"
          clearable
          outlined
          dense
          emit-value
          map-options
        />
      </div>
    </div>

    <!-- Phase 2 info banner -->
    <q-banner class="bg-orange-1 text-orange-9 q-mb-lg rounded-borders" rounded>
      <template #avatar>
        <q-icon name="o_photo_camera" color="deep-orange" />
      </template>
      <div class="text-weight-bold">Photo &amp; video upload available in Phase 2</div>
      <div class="text-caption q-mt-xs">
        In Phase 2 you will be able to upload and manage resident photos and activity videos,
        organize them by date and category, and share selected media with families.
      </div>
    </q-banner>

    <!-- Empty state with placeholder grid -->
    <div class="flex flex-center column q-py-md text-center q-mb-lg">
      <q-icon name="o_photo_library" size="3rem" color="grey-4" />
      <div class="text-grey-5 q-mt-md text-body1">No media uploaded yet</div>
      <div class="text-grey-4 text-caption">Upload photos in Phase 2</div>
    </div>

    <!-- Placeholder cards grid -->
    <div class="row q-gutter-md">
      <div
        v-for="n in 6"
        :key="n"
        class="col-12 col-sm-6 col-md-4 col-lg-3"
      >
        <q-card flat bordered class="bg-grey-2" style="height: 160px">
          <q-card-section class="flex flex-center full-height column">
            <q-icon name="o_photo_camera" size="2.5rem" color="grey-4" />
            <div class="text-caption text-grey-4 q-mt-sm">No photo</div>
          </q-card-section>
        </q-card>
      </div>
    </div>
  </q-page>
</template>
