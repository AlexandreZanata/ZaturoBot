"use client";

import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { cn } from "@/lib/utils";

type Img = { src: string; alt: string };

export function OfferGallery({ main, images }: { main: Img; images: Img[] }) {
  const [active, setActive] = useState(0);
  const current = images.length > 0 ? images[active] : main;
  return (
    <div>
      <div className="relative aspect-square overflow-hidden rounded-lg bg-tile">
        <AnimatePresence mode="wait">
          <motion.img
            key={current.src}
            id="detail-main-photo"
            data-gallery-main
            src={current.src}
            alt={current.alt}
            width={640}
            height={480}
            initial={{ opacity: 0, scale: 0.98 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.25 }}
            className="h-full w-full object-contain p-[12%] mix-blend-multiply"
          />
        </AnimatePresence>
      </div>
      {images.length > 1 && (
        <div role="group" aria-label="Fotos disponíveis do produto" className="scrollbar-none mt-3 flex gap-2 overflow-x-auto pb-1">
          {images.map((img, idx) => (
            <button
              key={img.src}
              type="button"
              data-gallery-btn
              onClick={() => setActive(idx)}
              aria-label={`Visualizar foto ${idx + 1} de ${images.length}`}
              aria-pressed={idx === active}
              className={cn(
                "h-16 w-16 shrink-0 overflow-hidden rounded-md border bg-tile p-1.5 transition-all",
                idx === active ? "border-ink" : "border-transparent opacity-60 hover:opacity-100"
              )}
            >
              <img src={img.src} alt={img.alt} width={96} height={96} loading="lazy" className="h-full w-full object-contain mix-blend-multiply" />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
