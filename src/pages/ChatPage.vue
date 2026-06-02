<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from "vue";
import { useRoute } from "vue-router";
import { useQuasar } from "quasar";
import {
  server, type ConversationSummary, type ChatMessage, type ChatInvite, type OrgPerson,
} from "@/lib/server";
import { useServerSessionStore } from "@/stores/server-session";

const props = defineProps<{ initialConvId?: string | null; initialConvName?: string | null }>();

const $q = useQuasar();
const route = useRoute();
const session = useServerSessionStore();
const myId = computed(() => session.me?.id ?? "");

const convos = ref<ConversationSummary[]>([]);
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
// 상대가 자동 수락하기 전에는 other_names 가 비어 있어, 내가 시작한 대화는
// 선택했던 상대 이름을 기억해 바로 보여준다.
const localNames = ref<Record<string, string>>({});
function convTitle(c: ConversationSummary) {
  return c.title || c.other_names || localNames.value[c.id] || "대화";
}

async function loadList() {
  loadingList.value = true;
  try {
    // 수락/거절 개념 없이 대화가 바로 뜨도록, 들어온 초대는 조용히 자동 수락한다.
    const iv = await server.myInvites().catch(() => [] as ChatInvite[]);
    if (iv.length) {
      await Promise.all(iv.map((i) => server.acceptInvite(i.id).catch(() => {})));
    }
    const cs = await server.conversations();
    convos.value = cs;
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

// ── new conversation (상대 검색 후 대화 시작) ────────────────────────────────
const showNew = ref(false);
const people = ref<OrgPerson[]>([]);
const peopleQuery = ref("");
const newInvitee = ref<string | null>(null);
const peopleOptions = computed(() => {
  const q = peopleQuery.value.trim().toLowerCase();
  return people.value
    .filter((p) => !p.is_inactive && p.id !== myId.value)
    .filter((p) => !q || p.full_name.toLowerCase().includes(q) || (p.position_ko ?? "").toLowerCase().includes(q))
    .map((p) => ({ label: `${p.full_name} · ${p.position_ko}`, value: p.id }));
});
function onPeopleFilter(val: string, update: (fn: () => void) => void) {
  update(() => { peopleQuery.value = val; });
}

async function openNew() {
  newInvitee.value = null;
  peopleQuery.value = "";
  showNew.value = true;
  if (!people.value.length) {
    try { people.value = (await server.orgPaged({ page: 1, page_size: 500 })).items; } catch { /* */ }
  }
}
async function createConv() {
  if (!newInvitee.value) { $q.notify({ type: "negative", message: "대화할 상대를 선택하세요." }); return; }
  try {
    const c = await server.createConversation({ invitee_id: newInvitee.value });
    const picked = people.value.find((x) => x.id === newInvitee.value);
    if (picked) localNames.value[c.id] = picked.full_name;
    showNew.value = false;
    await loadList();
    const fresh = convos.value.find((x) => x.id === c.id) ?? c;
    await openConv(fresh);
  } catch (e: any) {
    $q.notify({ type: "negative", message: `시작 실패: ${e?.message ?? e}` });
  }
}

// 대화에 사람 추가 (수락 없이 바로 합류).
const showInvite = ref(false);
const inviteId = ref<string | null>(null);
async function openInvite() {
  inviteId.value = null;
  peopleQuery.value = "";
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
    $q.notify({ type: "positive", message: "추가했습니다." });
  } catch (e: any) {
    $q.notify({ type: "negative", message: `추가 실패: ${e?.message ?? e}` });
  }
}

onMounted(async () => {
  await loadList();
  // 휴가 승인 등에서 ?conv=<id> 로 넘어오면 해당 대화를 자동으로 연다.
  const wanted = props.initialConvId ?? (route.query.conv as string | undefined);
  const wantedName = props.initialConvName ?? (route.query.name as string | undefined);
  if (wanted) {
    if (wantedName) localNames.value[wanted] = wantedName;
    const c = convos.value.find((x) => x.id === wanted);
    if (c) await openConv(c);
  }
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
        <q-btn unelevated dense color="primary" icon="o_search" label="대화 상대 찾기" @click="openNew" />
      </div>

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
          <q-btn flat dense icon="o_person_add" label="추가" @click="openInvite" />
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

    <!-- 대화 상대 찾기 -->
    <q-dialog v-model="showNew">
      <q-card style="min-width: 380px">
        <q-card-section class="text-h6">대화 상대 찾기</q-card-section>
        <q-card-section>
          <q-select v-model="newInvitee" :options="peopleOptions" emit-value map-options outlined dense use-input
            label="이름으로 검색" input-debounce="0" :loading="!people.length" autofocus
            @filter="onPeopleFilter">
            <template #prepend><q-icon name="o_search" /></template>
            <template #no-option><q-item><q-item-section class="text-grey-6">검색 결과 없음</q-item-section></q-item></template>
          </q-select>
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="취소" v-close-popup />
          <q-btn unelevated color="primary" label="대화 시작" @click="createConv" />
        </q-card-actions>
      </q-card>
    </q-dialog>

    <!-- 대화에 추가 -->
    <q-dialog v-model="showInvite">
      <q-card style="min-width: 360px">
        <q-card-section class="text-h6">대화에 추가</q-card-section>
        <q-card-section>
          <q-select v-model="inviteId" :options="peopleOptions" emit-value map-options outlined dense use-input
            label="이름으로 검색" input-debounce="0" @filter="onPeopleFilter">
            <template #prepend><q-icon name="o_search" /></template>
          </q-select>
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="취소" v-close-popup />
          <q-btn unelevated color="primary" label="추가" @click="doInvite" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<style scoped>
.chat-page { display: flex; height: 72vh; padding: 0; }
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
