<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useQuasar } from "quasar";
import { server } from "@/lib/server";

const $q = useQuasar();

interface PhotoSummary {
  id: string;
  resident_id: string;
  resident_name: string;
  branch_id: string;
  branch_name: string;
  taken_by_name: string;
  taken_at: string;
  caption: string | null;
  status: string;
  data_url: string;
}

interface Branch { id: string; name: string }

const tab = ref<"pending" | "approved" | "rejected">("pending");
const branchFilter = ref<string | null>(null);
const searchQ = ref("");
const photos = ref<PhotoSummary[]>([]);
const loading = ref(false);

// counts per tab
const counts = ref({ pending: 0, approved: 0, rejected: 0 });

// modal
const selected = ref<PhotoSummary | null>(null);
const note = ref("");
const acting = ref<string | null>(null);

const branches = ref<Branch[]>([]);
const branchOptions = computed(() =>
  [{ label: "전체 지점", value: null as string | null }].concat(
    branches.value.map((b) => ({ label: b.name, value: b.id })),
  ),
);

async function loadCounts() {
  try {
    const [p, a, r] = await Promise.all([
      server.photosPending("pending"),
      server.photosPending("approved"),
      server.photosPending("rejected"),
    ]);
    counts.value = { pending: p.length, approved: a.length, rejected: r.length };
  } catch {/* ignore */}
}

async function load() {
  loading.value = true;
  try {
    photos.value = await server.photosPending(tab.value);
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.message ?? "사진을 불러오지 못했습니다" });
    photos.value = [];
  } finally {
    loading.value = false;
  }
}

watch(tab, load);

async function loadBranches() {
  try {
    const d = await server.dashboard();
    branches.value = (d as { branches: Branch[] }).branches ?? [];
  } catch {/* ignore */}
}

const filtered = computed(() => {
  let rows = photos.value;
  if (branchFilter.value) rows = rows.filter((r) => r.branch_id === branchFilter.value);
  if (searchQ.value.trim()) {
    const n = searchQ.value.toLowerCase();
    rows = rows.filter(
      (r) =>
        r.resident_name.toLowerCase().includes(n) ||
        r.taken_by_name.toLowerCase().includes(n) ||
        (r.caption ?? "").toLowerCase().includes(n),
    );
  }
  return rows;
});

async function decide(p: PhotoSummary, status: "approved" | "rejected") {
  if (acting.value) return;
  acting.value = p.id;
  try {
    await server.decidePhoto(p.id, status, note.value || undefined);
    $q.notify({
      type: "positive",
      message:
        status === "approved"
          ? "승인 — 가족에게 전송됩니다"
          : "반려되었습니다",
      icon: status === "approved" ? "o_check_circle" : "o_cancel",
    });
    selected.value = null;
    note.value = "";
    await Promise.all([load(), loadCounts()]);
  } catch (e: any) {
    $q.notify({ type: "negative", message: e?.message ?? "처리 실패" });
  } finally {
    acting.value = null;
  }
}

function fmtTime(iso: string) {
  return new Date(iso).toLocaleString("ko-KR", {
    month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit",
  });
}

onMounted(async () => {
  await Promise.all([load(), loadBranches(), loadCounts()]);
});
</script>

<template>
  <q-page class="q-pa-lg">
    <!-- header -->
    <div class="row items-center q-mb-lg q-gutter-md">
      <div class="col">
        <div class="text-h5 text-weight-bold flex items-center">
          <q-icon name="o_photo_camera" class="q-mr-sm text-pink-7" />
          사진 승인
        </div>
        <div class="text-caption text-grey-7">
          요양보호사가 촬영한 어르신 사진을 검토하고, 승인 시 텔레그램으로 가족에게 전송됩니다.
        </div>
      </div>
    </div>

    <!-- tabs -->
    <q-tabs
      v-model="tab"
      class="text-grey-7 q-mb-md"
      active-color="pink-7"
      indicator-color="pink-7"
      align="left"
      dense
      no-caps
    >
      <q-tab name="pending">
        <div class="row items-center q-gutter-xs">
          <span>승인 대기</span>
          <q-badge color="pink-7">{{ counts.pending }}</q-badge>
        </div>
      </q-tab>
      <q-tab name="approved">
        <div class="row items-center q-gutter-xs">
          <span>승인됨</span>
          <q-badge color="grey-6">{{ counts.approved }}</q-badge>
        </div>
      </q-tab>
      <q-tab name="rejected">
        <div class="row items-center q-gutter-xs">
          <span>반려됨</span>
          <q-badge color="grey-6">{{ counts.rejected }}</q-badge>
        </div>
      </q-tab>
    </q-tabs>

    <!-- filters -->
    <div class="row q-col-gutter-md q-mb-md items-center">
      <div class="col-12 col-md-4">
        <q-input
          v-model="searchQ"
          dense
          outlined
          placeholder="어르신/촬영자/메모 검색"
          clearable
        >
          <template #prepend><q-icon name="o_search" /></template>
        </q-input>
      </div>
      <div class="col-12 col-md-3">
        <q-select
          v-model="branchFilter"
          :options="branchOptions"
          label="지점"
          dense
          outlined
          emit-value
          map-options
        />
      </div>
    </div>

    <!-- grid -->
    <div v-if="loading" class="row q-col-gutter-md">
      <div v-for="i in 6" :key="i" class="col-12 col-sm-6 col-md-4 col-lg-3">
        <q-card flat bordered>
          <q-skeleton height="180px" square />
          <q-card-section>
            <q-skeleton type="text" />
            <q-skeleton type="text" width="60%" />
          </q-card-section>
        </q-card>
      </div>
    </div>

    <div v-else-if="filtered.length === 0" class="flex flex-center column q-py-xl">
      <q-icon name="o_photo_library" size="3rem" color="grey-4" />
      <div class="text-grey-5 q-mt-md text-body1">표시할 사진이 없습니다</div>
    </div>

    <div v-else class="row q-col-gutter-md">
      <div
        v-for="p in filtered"
        :key="p.id"
        class="col-12 col-sm-6 col-md-4 col-lg-3"
      >
        <q-card flat bordered class="full-height column">
          <q-img
            :src="p.data_url"
            :alt="p.caption ?? p.resident_name"
            :ratio="4 / 3"
            class="cursor-pointer"
            @click="selected = p"
          />
          <q-card-section class="q-pb-none col">
            <div class="text-subtitle2 text-weight-semibold">{{ p.resident_name }}</div>
            <div class="text-caption text-grey-7">
              {{ p.branch_name }} · 촬영 {{ p.taken_by_name }}
            </div>
            <div class="text-caption text-grey-6">{{ fmtTime(p.taken_at) }}</div>
            <div v-if="p.caption" class="text-body2 q-mt-xs ellipsis-2-lines">
              {{ p.caption }}
            </div>
          </q-card-section>
          <q-card-actions v-if="tab === 'pending'" align="stretch" class="q-pa-sm">
            <q-btn
              dense
              unelevated
              color="positive"
              icon="o_check"
              label="승인"
              class="col"
              :loading="acting === p.id"
              @click.stop="decide(p, 'approved')"
            />
            <q-btn
              dense
              outline
              color="grey-8"
              icon="o_close"
              label="반려"
              class="col q-ml-sm"
              :loading="acting === p.id"
              @click.stop="decide(p, 'rejected')"
            />
          </q-card-actions>
        </q-card>
      </div>
    </div>

    <!-- detail modal -->
    <q-dialog v-model="selected" maximized-md>
      <q-card v-if="selected" style="max-width: 800px; width: 100%">
        <q-card-section class="row items-center q-pb-none">
          <div>
            <div class="text-h6">{{ selected.resident_name }}</div>
            <div class="text-caption text-grey-7">
              {{ selected.branch_name }} · 촬영 {{ selected.taken_by_name }} · {{ fmtTime(selected.taken_at) }}
            </div>
          </div>
          <q-space />
          <q-btn flat round dense icon="o_close" v-close-popup />
        </q-card-section>

        <q-card-section class="q-pa-none bg-grey-10 flex flex-center">
          <img
            :src="selected.data_url"
            :alt="selected.caption ?? ''"
            style="max-width: 100%; max-height: 60vh; object-fit: contain"
          />
        </q-card-section>

        <q-card-section v-if="selected.caption">
          <div class="text-body2">{{ selected.caption }}</div>
        </q-card-section>

        <q-card-section v-if="tab === 'pending'" class="q-pt-none">
          <q-input
            v-model="note"
            type="textarea"
            outlined
            dense
            rows="2"
            label="(선택) 검토 메모"
          />
        </q-card-section>

        <q-card-actions v-if="tab === 'pending'" align="right" class="q-pa-md">
          <q-btn
            outline
            color="grey-8"
            icon="o_close"
            label="반려"
            :loading="acting === selected.id"
            @click="decide(selected, 'rejected')"
          />
          <q-btn
            unelevated
            color="positive"
            icon="o_check"
            label="승인하고 가족에게 전송"
            :loading="acting === selected.id"
            @click="decide(selected, 'approved')"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<style scoped>
.ellipsis-2-lines {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
