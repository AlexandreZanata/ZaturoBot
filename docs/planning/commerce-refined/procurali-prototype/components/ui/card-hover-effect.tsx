"use client";

import Link from "next/link";
import { AnimatePresence, motion } from "motion/react";
import { useState } from "react";
import { ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Aceternity "card hover effect" reduced to an index list: the shared hover background
 * glides between rows; everything else is plain typography and hairlines.
 */
export const HoverEffect = ({
  items,
  className,
}: {
  items: { title: string; description: string; link: string; meta?: string }[];
  className?: string;
}) => {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  return (
    <ul className={cn("border-b border-line", className)}>
      {items.map((item, idx) => (
        <li key={item.link} className="border-t border-line">
          <Link
            href={item.link}
            className="group relative grid items-baseline gap-x-8 gap-y-1 px-3 py-5 sm:grid-cols-[minmax(0,14rem)_1fr_auto] sm:px-4"
            onMouseEnter={() => setHoveredIndex(idx)}
            onMouseLeave={() => setHoveredIndex(null)}
          >
            <AnimatePresence>
              {hoveredIndex === idx && (
                <motion.span
                  className="absolute inset-0 bg-tile"
                  layoutId="hoverBackground"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1, transition: { duration: 0.15 } }}
                  exit={{ opacity: 0, transition: { duration: 0.15, delay: 0.1 } }}
                />
              )}
            </AnimatePresence>
            <CardTitle>{item.title}</CardTitle>
            <CardDescription>{item.description}</CardDescription>
            <span className="relative flex items-center gap-3 text-xs whitespace-nowrap text-muted">
              {item.meta}
              <ArrowRight className="h-4 w-4 text-ink transition-transform group-hover:translate-x-0.5" aria-hidden="true" />
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
};

export const CardTitle = ({ className, children }: { className?: string; children: React.ReactNode }) => (
  <h3 className={cn("relative text-lg font-medium text-ink", className)}>{children}</h3>
);

export const CardDescription = ({ className, children }: { className?: string; children: React.ReactNode }) => (
  <p className={cn("relative text-sm leading-relaxed text-muted", className)}>{children}</p>
);
