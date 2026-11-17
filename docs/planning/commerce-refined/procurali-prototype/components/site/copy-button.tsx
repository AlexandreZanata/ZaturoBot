"use client";

import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/utils";

/** Copies either `text` or the textContent of the element with id `targetId`. */
export function CopyButton({
  text,
  targetId,
  label,
  className,
  pill = false,
}: {
  text?: string;
  targetId?: string;
  label: React.ReactNode;
  className?: string;
  pill?: boolean;
}) {
  const [copied, setCopied] = useState(false);
  async function onCopy() {
    const value = text ?? (targetId ? document.getElementById(targetId)?.textContent ?? "" : "");
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch {
      setCopied(false);
    }
  }
  return (
    <button
      type="button"
      onClick={onCopy}
      className={cn(
        pill
          ? "inline-flex min-h-10 items-center gap-2 rounded-md border border-dashed border-neutral-300 px-3 py-1.5 text-left text-sm text-ink hover:border-ink"
          : "inline-flex min-h-11 items-center gap-2 rounded-md border border-line bg-white px-4 py-2.5 text-sm font-medium text-ink hover:border-ink",
        className
      )}
    >
      {copied ? <Check className="h-4 w-4 text-emerald-600" aria-hidden="true" /> : <Copy className="h-4 w-4" aria-hidden="true" />}
      <span>{copied ? "Copiado!" : label}</span>
      <span className="sr-only" aria-live="polite">
        {copied ? "Copiado para a área de transferência" : ""}
      </span>
    </button>
  );
}
