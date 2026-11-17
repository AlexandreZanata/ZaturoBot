"use client";

import Link from "next/link";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Bookmark, BookmarkCheck, Heart, X, Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";

const STORAGE_SAVED_KEY = "procurali_saved_v1";

type SavedItem = { slug: string; title: string; priceLabel: string };
type SavedContextValue = {
  items: SavedItem[];
  isSaved: (slug: string) => boolean;
  toggle: (item: SavedItem) => void;
  remove: (slug: string) => void;
  clear: () => void;
  open: boolean;
  setOpen: (open: boolean) => void;
};

const SavedContext = createContext<SavedContextValue | null>(null);

function read(): SavedItem[] {
  try {
    const raw = window.localStorage.getItem(STORAGE_SAVED_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((i) => i && typeof i.slug === "string") : [];
  } catch {
    return [];
  }
}

function write(items: SavedItem[]) {
  try {
    window.localStorage.setItem(STORAGE_SAVED_KEY, JSON.stringify(items));
  } catch {
    // Storage may be unavailable (private mode); saving stays in memory for this visit.
  }
}

export function SavedProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<SavedItem[]>([]);
  const [open, setOpen] = useState(false);

  useEffect(() => setItems(read()), []);

  const update = useCallback((next: SavedItem[]) => {
    setItems(next);
    write(next);
  }, []);

  const value = useMemo<SavedContextValue>(
    () => ({
      items,
      open,
      setOpen,
      isSaved: (slug) => items.some((i) => i.slug === slug),
      toggle: (item) =>
        update(items.some((i) => i.slug === item.slug) ? items.filter((i) => i.slug !== item.slug) : [...items, item]),
      remove: (slug) => update(items.filter((i) => i.slug !== slug)),
      clear: () => update([]),
    }),
    [items, open, update]
  );

  return (
    <SavedContext.Provider value={value}>
      {children}
      <SavedDrawer />
    </SavedContext.Provider>
  );
}

export function useSaved() {
  const ctx = useContext(SavedContext);
  if (!ctx) throw new Error("useSaved must be used inside SavedProvider");
  return ctx;
}

export function SavedTrigger({ className }: { className?: string }) {
  const { items, setOpen } = useSaved();
  return (
    <button
      type="button"
      onClick={() => setOpen(true)}
      aria-haspopup="dialog"
      aria-controls="saved-drawer"
      aria-label="Abrir itens salvos"
      className={cn(
        "relative grid h-11 w-11 place-items-center text-ink",
        className
      )}
    >
      <Bookmark className="h-5 w-5" strokeWidth={1.75} aria-hidden="true" />
      {items.length > 0 && (
        <span className="absolute top-1.5 right-1 grid h-4 min-w-4 place-items-center rounded-full bg-ink px-1 text-[0.65rem] font-medium text-white">
          {items.length}
        </span>
      )}
    </button>
  );
}

export function SaveButton({
  slug,
  title,
  priceLabel,
  variant = "button",
  className,
}: {
  slug: string;
  title: string;
  priceLabel: string;
  variant?: "button" | "icon";
  className?: string;
}) {
  const { isSaved, toggle } = useSaved();
  const saved = isSaved(slug);
  if (variant === "icon") {
    return (
      <button
        type="button"
        onClick={() => toggle({ slug, title, priceLabel })}
        aria-pressed={saved}
        aria-label={saved ? `Remover ${title} dos salvos` : `Salvar ${title} na lista`}
        className={cn(
          "grid h-11 w-11 place-items-center rounded-full bg-white/90 transition-colors",
          saved ? "text-ink" : "text-neutral-400 hover:text-ink",
          className
        )}
      >
        <Heart className={cn("h-[1.1rem] w-[1.1rem]", saved && "fill-current")} strokeWidth={1.75} aria-hidden="true" />
      </button>
    );
  }
  return (
    <button
      type="button"
      onClick={() => toggle({ slug, title, priceLabel })}
      aria-pressed={saved}
      className={cn(
        "tap inline-flex min-h-11 items-center justify-center gap-2 rounded-md border px-4 py-2.5 text-sm font-medium transition-colors",
        saved ? "border-ink text-ink" : "border-line bg-white text-ink hover:border-ink",
        className
      )}
    >
      {saved ? <BookmarkCheck className="h-4 w-4" aria-hidden="true" /> : <Bookmark className="h-4 w-4" aria-hidden="true" />}
      {saved ? "Salvo na sua lista" : "Salvar item"}
    </button>
  );
}

function SavedDrawer() {
  const { items, open, setOpen, remove, clear } = useSaved();

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, setOpen]);

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 z-[60] flex justify-end bg-black/30"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={(e) => e.target === e.currentTarget && setOpen(false)}
        >
          <motion.div
            id="saved-drawer"
            role="dialog"
            aria-modal="true"
            aria-labelledby="saved-drawer-title"
            className="flex h-full w-full max-w-md flex-col bg-white"
            initial={{ x: "100%" }}
            animate={{ x: 0 }}
            exit={{ x: "100%" }}
            transition={{ type: "spring", damping: 30, stiffness: 300 }}
          >
            <div className="flex items-center justify-between border-b border-line p-5">
              <h2 id="saved-drawer-title" className="flex items-center gap-2 text-lg font-semibold text-ink">
                <Bookmark className="h-5 w-5" aria-hidden="true" /> Itens salvos
              </h2>
              <button
                type="button"
                onClick={() => setOpen(false)}
                aria-label="Fechar lista de salvos"
                className="grid h-11 w-11 place-items-center rounded-md hover:bg-neutral-100"
                autoFocus
              >
                <X className="h-5 w-5" aria-hidden="true" />
              </button>
            </div>
            <p className="border-b border-line bg-canvas px-5 py-3 text-xs text-muted">
              Salvos apenas neste navegador (<code>localStorage</code>) para comparar durante a visita. Nada é enviado a servidores.
            </p>
            <div className="flex-1 space-y-3 overflow-y-auto p-5">
              {items.length === 0 ? (
                <div className="grid place-items-center rounded-lg border border-dashed border-line px-6 py-12 text-center text-sm text-muted">
                  <Heart className="mb-3 h-8 w-8 text-neutral-300" aria-hidden="true" />
                  Nenhum item salvo ainda. Use o botão “Salvar item” em qualquer oferta para comparar depois.
                </div>
              ) : (
                items.map((item) => (
                  <div key={item.slug} className="rounded-lg border border-line p-4">
                    <p className="font-semibold text-ink">{item.title}</p>
                    <p className="mt-0.5 text-sm text-muted">{item.priceLabel}</p>
                    <div className="mt-3 flex gap-2">
                      <Link
                        href={`/ofertas/${item.slug}/`}
                        onClick={() => setOpen(false)}
                        className="tap inline-flex min-h-11 items-center rounded-md bg-ink px-4 text-sm font-medium text-white hover:bg-petrol-800"
                      >
                        Abrir ficha
                      </Link>
                      <button
                        type="button"
                        onClick={() => remove(item.slug)}
                        className="inline-flex min-h-11 items-center gap-1.5 rounded-md border border-line px-4 text-sm font-semibold hover:bg-neutral-50"
                      >
                        <Trash2 className="h-4 w-4" aria-hidden="true" /> Remover
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
            <div className="border-t border-line p-5">
              <button
                type="button"
                onClick={clear}
                disabled={items.length === 0}
                className="w-full rounded-md border border-line px-4 py-3 text-sm font-semibold hover:bg-neutral-50 disabled:opacity-50"
              >
                Limpar itens salvos
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
