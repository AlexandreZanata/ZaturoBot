"use client";

import { useEffect, useState } from "react";
import { ANALYTICS_EVENT, STORAGE_CONSENT_KEY, STORAGE_EVENTS_KEY } from "@/components/site/analytics";

type Event = { event: string; detail: string; at: string };

export function AnalyticsConsent() {
  const [granted, setGranted] = useState(false);
  const [events, setEvents] = useState<Event[]>([]);

  useEffect(() => {
    const load = () => {
      try {
        setGranted(window.localStorage.getItem(STORAGE_CONSENT_KEY) === "granted");
        const raw = window.localStorage.getItem(STORAGE_EVENTS_KEY);
        setEvents(raw ? JSON.parse(raw) : []);
      } catch {
        setEvents([]);
      }
    };
    load();
    window.addEventListener(ANALYTICS_EVENT, load);
    return () => window.removeEventListener(ANALYTICS_EVENT, load);
  }, []);

  function toggle(next: boolean) {
    setGranted(next);
    try {
      if (next) window.localStorage.setItem(STORAGE_CONSENT_KEY, "granted");
      else window.localStorage.removeItem(STORAGE_CONSENT_KEY);
    } catch {
      // Storage unavailable: consent stays off.
    }
  }

  function clear() {
    setEvents([]);
    try {
      window.localStorage.removeItem(STORAGE_EVENTS_KEY);
    } catch {
      // ignore
    }
  }

  return (
    <div className="mt-8 rounded-lg border border-line bg-white p-6">
      <h3 className="text-lg font-semibold text-ink">Controle de medição opcional (demonstração local)</h3>
      <p id="analytics-consent-status" className="mt-1 text-sm text-muted">
        Medição opcional {granted ? "ATIVADA neste navegador" : "DESATIVADA por padrão"}.
      </p>
      <label className="mt-4 flex cursor-pointer items-start gap-3 rounded-lg border border-line p-4 text-sm font-semibold text-ink hover:bg-canvas">
        <input
          type="checkbox"
          id="analytics-consent-toggle"
          checked={granted}
          onChange={(e) => toggle(e.target.checked)}
          className="mt-0.5 h-5 w-5 accent-neutral-900"
        />
        Permitir registro local demonstrativo de cliques (separa cliques em ofertas de cliques em comunidade; nada sai do navegador)
      </label>
      <div id="analytics-demo-log" className="mt-4 space-y-1.5">
        {events.length === 0 ? (
          <p className="text-sm text-muted">Nenhum evento local registrado.</p>
        ) : (
          events.map((e, idx) => (
            <p key={idx} className="rounded-lg bg-canvas px-3 py-2 font-mono text-xs text-neutral-700">
              {e.at} · {e.event} · {e.detail}
            </p>
          ))
        )}
      </div>
      <button
        type="button"
        id="analytics-clear-log"
        onClick={clear}
        className="mt-4 inline-flex min-h-11 items-center rounded-md border border-line px-4 text-sm font-semibold hover:bg-canvas"
      >
        Limpar log local
      </button>
    </div>
  );
}
