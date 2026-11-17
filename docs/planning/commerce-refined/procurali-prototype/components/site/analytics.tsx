"use client";

import { useEffect } from "react";

export const STORAGE_CONSENT_KEY = "procurali_analytics_consent_v1";
export const STORAGE_EVENTS_KEY = "procurali_demo_events_v1";
export const ANALYTICS_EVENT = "procurali:analytics";

export function hasConsent() {
  try {
    return window.localStorage.getItem(STORAGE_CONSENT_KEY) === "granted";
  } catch {
    return false;
  }
}

export function recordLocalEvent(event: string, detail: string) {
  if (!hasConsent()) return;
  try {
    const raw = window.localStorage.getItem(STORAGE_EVENTS_KEY);
    const list = raw ? JSON.parse(raw) : [];
    const next = [{ event, detail, at: new Date().toISOString() }, ...(Array.isArray(list) ? list : [])].slice(0, 30);
    window.localStorage.setItem(STORAGE_EVENTS_KEY, JSON.stringify(next));
    window.dispatchEvent(new Event(ANALYTICS_EVENT));
  } catch {
    // Local-only demo log; ignore storage failures.
  }
}

/** Records clicks on elements carrying data-analytics-event, only after opt-in consent. */
export function AnalyticsListener() {
  useEffect(() => {
    const onClick = (e: MouseEvent) => {
      const el = (e.target as HTMLElement | null)?.closest?.("[data-analytics-event]");
      if (!el) return;
      recordLocalEvent(el.getAttribute("data-analytics-event") || "click", el.getAttribute("data-analytics-detail") || "");
    };
    document.addEventListener("click", onClick);
    return () => document.removeEventListener("click", onClick);
  }, []);
  return null;
}
