import { useEffect, useState } from "react";
import { api } from "./api";

export interface LoadSettings {
  syncLimit: number;
  pageSize: number;
}

let cached: LoadSettings = { syncLimit: 200, pageSize: 100 };
const listeners = new Set<(v: LoadSettings) => void>();

function set(v: LoadSettings) {
  cached = v;
  listeners.forEach((l) => l(v));
}

export function useAppLoadSettings(): LoadSettings {
  const [v, setV] = useState<LoadSettings>(cached);

  useEffect(() => {
    listeners.add(setV);
    api
      .getAppSettings()
      .then((s) => set({ syncLimit: s.sync_limit, pageSize: s.page_size }))
      .catch(() => undefined);
    const onChange = () => {
      api
        .getAppSettings()
        .then((s) => set({ syncLimit: s.sync_limit, pageSize: s.page_size }))
        .catch(() => undefined);
    };
    window.addEventListener("maildeck:load-settings-changed", onChange);
    return () => {
      listeners.delete(setV);
      window.removeEventListener("maildeck:load-settings-changed", onChange);
    };
  }, []);

  return v;
}
