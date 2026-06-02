<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from "vue";
import { useQuasar } from "quasar";
import {
  server, type ConversationSummary, type ChatMessage, type ChatInvite, type OrgPerson,
} from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const $q = useQuasar();
const session = useServerSessionStore();
const myId = computed(() => session.me?.id ?? "");

const convos = ref<ConversationSummary[]>([]);
const invites = ref<ChatInvite[]>([]);
const active = ref<ConversationSummary | null>(null);
const messages = ref<ChatMessage[]>([]);
const draft = ref("");
const loadingList = ref(false);
const loadingMsgs = ref(false);
const sending = ref(false);
const threadEl = ref<HTMLElement | null>(null);
let poll: number | undefined;

function fmtTime(iso: string) {
  return new Date(iso).toLocaleString("ko-KR", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}
function convTitle(c: ConversationSummary) {
  return c.title || c.other_names || "대화";
}

async function loadList() {
  loadingList.value = true;
  try {
    const [cs, iv] = await Promise.all([server.conversations(), server.myInvites()]);
    convos.value = cs;
    invites.value = iv;
    if (active.value) {
      const fresh = cs.find((c) => c.id === active.value!.id);
      if (fresh) active.value = fresh;
    }
  } catch (e: any) {
    $q.notify({ type: "negative", message: `목록을 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loadingList.value = false;
  }
}

async function openConv(c: ConversationSummary) {
  active.value = c;
  messages.value = [];
  loadingMsgs.value = true;
  try {
    messages.value = await server.messages(c.id);
    await scrollBottom();
    c.unread_count = 0; // server marks read on fetch
  } catch (e: any) {
    $q.notify({ type: "negative", message: `메시지를 불러오지 못했습니다: ${e?.message ?? e}` });
  } finally {
    loadingMsgs.value = false;
  }
}

async function pollActive() {
  if (!active.value) return;
  const since = messages.value.length ? messages.value[messages.value.length - 1].id : undefined;
  try {
    const fresh = await server.messages(active.value.id, since);
    if (fresh.length) {
      messages.value.push(...fresh);
      await scrollBottom();
    }
  } catch { /* transient; ignore */ }
}

async function send() {
  const body = draft.value.trim();
  if (!body || !active.value || sending.value) return;
  sending.value = true;
  try {
    const msg = await server.postMessage(active.value.id, body);
    messages.value.push(msg);
    draft.value = "";
    await scrollBottom();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `전송 실패: ${e?.message ?? e}` });
  } finally {
    sending.value = false;
  }
}

async function scrollBottom() {
  await nextTick();
  const el = threadEl.value;
  if (el) el.scrollTop = el.scrollHeight;
}

// ── invites ───────────────────────────────────────────────────────────────
async function respondInvite(iv: ChatInvite, accept: boolean) {
  try {
    if (accept) await server.acceptInvite(iv.id); else await server.rejectInvite(iv.id);
    $q.notify({ type: "positive", message: accept ? "참여했습니다." : "거절했습니다." });
    await loadList();
  } catch (e: any) {
    $q.notify({ type: "negative", message: `처리 실패: ${e?.message ?? e}` });
  }
}

// ── new conversation / invite ───────────────────────────────────────────────
const showNew = ref(false);
const people = ref<OrgPerson[]>([]);
const newTitle = ref("");
const newInvitee = ref<string | null>(null);
const peopleOptions = computed(() =>
  people.value
    .filter((p) => !p.is_inactive && p.id !== myId.value)
    .map((p) => ({ label: `${p.full_name} · ${p.position_ko}`, value: p.id })),
);

async function openNew() {
  newTitle.value = "";
  newInvitee.value = null;
  showNew.value = true;
  if (!people.value.length) {
    try { people.value = (await server.orgPaged({ page: 1, page_size: 500 })).items; } catch { /* */ }
  }
}
async function createConv() {
  if (!newInvitee.value) { $q.notify({ type: "negative", message: "초대할 상대를 선택하세요." }); return; }
  try {
    const c = await server.createConversation({ title: newTitle.value.trim() || null, invitee_id: newInvitee.value });
    showNew.value = false;
    await loadList();
    const fresh = convos.value.find((x) => x.id === c.id) ?? c;
    await openConv(fresh);
    $q.notify({ type: "positive", message: "대화를 시작했습니다. 상대가 수락하면 대화할 수 있습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `생성 실패: ${e?.message ?? e}` });
  }
}

const showInvite = ref(false);
const inviteId = ref<string | null>(null);
async function openInvite() {
  inviteId.value = null;
  showInvite.value = true;
  if (!people.value.length) {
    try { people.value = (await server.orgPaged({ page: 1, page_size: 500 })).items; } catch { /* */ }
  }
}
async function doInvite() {
  if (!active.value || !inviteId.value) return;
  try {
    await server.inviteToConversation(active.value.id, inviteId.value);
    showInvite.value = false;
    $q.notify({ type: "positive", message: "초대했습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `초대 실패: ${e?.message ?? e}` });
  }
}

watch(active, () => { /* keep poll cursor fresh */ });

onMounted(() => {
  loadList();
  poll = window.setInterval(() => { loadList(); pollActive(); }, 4000);
});
onBeforeUnmount(() => { if (poll) clearInterval(poll); });
</script>

<template>
  <q-page class="chat-page">
    <!-- 좌측: 대화 목록 -->
    <div class="chat-sidebar">
      <div class="row items-center q-pa-md q-gutter-sm">
        <div class="text-h6 text-weight-bold col">대화</div>
        <q-btn unelevated dense color="primary" icon="o_edit_note" label="새 대화" @click="openNew" />
      </div>

      <q-list v-if="invites.length" bordered class="invite-box q-mx-md q-mb-sm rounded-borders">
        <q-item-label header class="text-orange-9">받은 초대 {{ invites.length }}</q-item-label>
        <q-item v-for="iv in invites" :key="iv.id">
          <q-item-section>
            <q-item-label>{{ iv.conversation_title || "대화 초대" }}</q-item-label>
            <q-item-label caption>{{ iv.invited_by_name }} 님이 초대</q-item-label>
          </q-item-section>
          <q-item-section side>
            <div class="row q-gutter-xs">
              <q-btn dense unelevated color="positive" label="수락" @click="respondInvite(iv, true)" />
              <q-btn dense outline color="grey-7" label="거절" @click="respondInvite(iv, false)" />
            </div>
          </q-item-section>
        </q-item>
      </q-list>

      <q-scroll-area class="chat-list">
        <q-list separator>
          <q-item v-for="c in convos" :key="c.id" clickable :active="active?.id === c.id" active-class="conv-active" @click="openConv(c)">
            <q-item-section avatar>
              <q-avatar color="blue-grey-2" text-color="blue-grey-9" size="40px">{{ convTitle(c).slice(0, 1) }}</q-avatar>
            </q-item-section>
            <q-item-section>
              <q-item-label lines="1" class="text-weight-medium">{{ convTitle(c) }}</q-item-label>
              <q-item-label caption lines="1">{{ c.last_body || "메시지 없음" }}</q-item-label>
            </q-item-section>
            <q-item-section side top>
              <q-item-label caption>{{ fmtTime(c.last_message_at) }}</q-item-label>
              <q-badge v-if="c.unread_count" color="red" rounded :label="c.unread_count" class="q-mt-xs" />
            </q-item-section>
          </q-item>
          <div v-if="!convos.length && !loadingList" class="text-center text-grey-5 q-py-xl">대화가 없습니다</div>
        </q-list>
      </q-scroll-area>
    </div>

    <!-- 우측: 메시지 스레드 -->
    <div class="chat-main">
      <template v-if="active">
        <div class="row items-center q-pa-md chat-header">
          <div class="col">
            <div class="text-subtitle1 text-weight-bold">{{ convTitle(active) }}</div>
            <div class="text-caption text-grey-6">{{ active.other_names || "" }}</div>
          </div>
          <q-btn flat dense icon="o_person_add" label="초대" @click="openInvite" />
        </div>

        <div ref="threadEl" class="chat-thread">
          <q-inner-loading :showing="loadingMsgs" />
          <div v-for="m in messages" :key="m.id" class="msg-row" :class="{ mine: m.sender_id === myId }">
            <div class="msg-bubble" :class="{ mine: m.sender_id === myId }">
              <div v-if="m.sender_id !== myId" class="msg-sender">{{ m.sender_name }}</div>
              <div class="msg-body">{{ m.body }}</div>
              <div class="msg-time">{{ fmtTime(m.sent_at) }}</div>
            </div>
          </div>
          <div v-if="!messages.length && !loadingMsgs" class="text-center text-grey-5 q-py-xl">첫 메시지를 보내보세요</div>
        </div>

        <div class="chat-input row q-pa-sm q-gutter-sm items-end">
          <q-input v-model="draft" outlined dense autogrow class="col" placeholder="메시지를 입력하세요"
            @keydown.enter.exact.prevent="send" />
          <q-btn unelevated color="primary" icon="o_send" :loading="sending" @click="send" />
        </div>
      </template>
      <div v-else class="column flex-center full-height text-grey-5">
        <q-icon name="o_forum" size="4rem" color="grey-4" />
        <div class="q-mt-sm">대화를 선택하거나 새 대화를 시작하세요</div>
      </div>
    </div>

    <!-- 새 대화 -->
    <q-dialog v-model="showNew">
      <q-card style="min-width: 360px">
        <q-card-section class="text-h6">새 대화</q-card-section>
        <q-card-section class="q-gutter-md">
          <q-select v-model="newInvitee" :options="peopleOptions" emit-value map-options outlined dense use-input
            label="상대 선택" input-debounce="0" :loading="!people.length"
            @filter="(_, u) => u(() => {})" />
          <q-input v-model="newTitle" label="제목 (선택)" outlined dense />
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="취소" v-close-popup />
          <q-btn unelevated color="primary" label="시작" @click="createConv" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- 초대 -->
    <q-dialog v-model="showInvite">
      <q-card style="min-width: 340px">
        <q-card-section class="text-h6">대화에 초대</q-card-section>
        <q-card-section>
          <q-select v-model="inviteId" :options="peopleOptions" emit-value map-options outlined dense use-input
            label="초대할 상대" input-debounce="0" @filter="(_, u) => u(() => {})" />
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="취소" v-close-popup />
          <q-btn unelevated color="primary" label="초대" @click="doInvite" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<style scoped>
.chat-page { display: flex; height: calc(100vh - 50px); padding: 0; }
.chat-sidebar { width: 340px; border-right: 1px solid #e0e0e0; display: flex; flex-direction: column; }
.chat-list { flex: 1; }
.invite-box { background: #fff8e1; }
.conv-active { background: #e3f2fd; }
.chat-main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.chat-header { border-bottom: 1px solid #e0e0e0; }
.chat-thread { flex: 1; overflow-y: auto; padding: 16px; background: #f7f9fb; position: relative; }
.msg-row { display: flex; margin-bottom: 10px; }
.msg-row.mine { justify-content: flex-end; }
.msg-bubble { max-width: 70%; padding: 8px 12px; border-radius: 12px; background: #fff; border: 1px solid #e6e6e6; }
.msg-bubble.mine { background: #d7ebff; border-color: #c2e0ff; }
.msg-sender { font-size: 11px; color: #6b7785; margin-bottom: 2px; font-weight: 600; }
.msg-body { white-space: pre-wrap; word-break: break-word; }
.msg-time { font-size: 10px; color: #9aa5b1; text-align: right; margin-top: 2px; }
.chat-input { border-top: 1px solid #e0e0e0; background: #fff; }
</style>
