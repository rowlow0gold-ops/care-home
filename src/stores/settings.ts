import { defineStore } from "pinia";
import { ref } from "vue";
import { Store } from "@tauri-apps/plugin-store";

// Local app preferences (not business data) — persisted in a Tauri store file,
// NOT the server and NOT the old SQLite DB.
const PREFS_FILE = "prefs.dat";
let prefs: Store | null = null;
async function prefsStore() {
  if (!prefs) prefs = await Store.load(PREFS_FILE);
  return prefs;
}

export type ShiftModel = "12h" | "8h";

export interface ShiftOption {
  value: string;
  label: string;
  start: number; // hour (0–23)
  end: number;   // hour (0–23), wraps midnight if end < start
}

export const SHIFTS: Record<ShiftModel, ShiftOption[]> = {
  "12h": [
    { value: "day",   label: "Day (07:00–19:00)",   start: 7,  end: 19 },
    { value: "night", label: "Night (19:00–07:00)",  start: 19, end: 7  },
  ],
  "8h": [
    { value: "morning",   label: "Morning (07:00–15:00)",   start: 7,  end: 15 },
    { value: "afternoon", label: "Afternoon (15:00–23:00)",  start: 15, end: 23 },
    { value: "night",     label: "Night (23:00–07:00)",      start: 23, end: 7  },
  ],
};

// Visit is always available regardless of shift model
export const VISIT_OPTION: ShiftOption = {
  value: "visit",
  label: "Visit",
  start: 0,
  end: 0,
};

export function detectCurrentShift(model: ShiftModel): string {
  const hour = new Date().getHours();
  const options = SHIFTS[model];
  for (const opt of options) {
    if (opt.end > opt.start) {
      // Normal range e.g. 7–19
      if (hour >= opt.start && hour < opt.end) return opt.value;
    } else {
      // Wraps midnight e.g. 19–7 or 23–7
      if (hour >= opt.start || hour < opt.end) return opt.value;
    }
  }
  return options[0].value;
}

export const useSettingsStore = defineStore("settings", () => {
  const shiftModel = ref<ShiftModel>("12h");
  const facilityName = ref("케어닥");
  const loaded = ref(false);

  async function load() {
    try {
      const s = await prefsStore();
      const model = await s.get<string>("shift_model");
      if (model === "12h" || model === "8h") shiftModel.value = model;
      const name = await s.get<string>("facility_name");
      if (name) facilityName.value = name;
    } catch (_) {
      // use defaults
    }
    loaded.value = true;
  }

  async function saveShiftModel(model: ShiftModel) {
    shiftModel.value = model;
    const s = await prefsStore();
    await s.set("shift_model", model);
    await s.save();
  }

  async function saveFacilityName(name: string) {
    facilityName.value = name;
    const s = await prefsStore();
    await s.set("facility_name", name);
    await s.save();
  }

  function getShiftOptions(): ShiftOption[] {
    return [...SHIFTS[shiftModel.value], VISIT_OPTION];
  }

  return { shiftModel, facilityName, loaded, load, saveShiftModel, saveFacilityName, getShiftOptions };
});
