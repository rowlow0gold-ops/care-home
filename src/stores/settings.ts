import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

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
  const facilityName = ref("Sunshine Care Home");
  const loaded = ref(false);

  async function load() {
    try {
      const model = await invoke<string | null>("get_setting", { key: "shift_model" });
      if (model === "12h" || model === "8h") shiftModel.value = model;
      const name = await invoke<string | null>("get_setting", { key: "facility_name" });
      if (name) facilityName.value = name;
    } catch (_) {
      // use defaults
    }
    loaded.value = true;
  }

  async function saveShiftModel(model: ShiftModel) {
    shiftModel.value = model;
    await invoke("set_setting", { key: "shift_model", value: model });
  }

  async function saveFacilityName(name: string) {
    facilityName.value = name;
    await invoke("set_setting", { key: "facility_name", value: name });
  }

  function getShiftOptions(): ShiftOption[] {
    return [...SHIFTS[shiftModel.value], VISIT_OPTION];
  }

  return { shiftModel, facilityName, loaded, load, saveShiftModel, saveFacilityName, getShiftOptions };
});
