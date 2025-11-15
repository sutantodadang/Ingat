import { useMemo } from "react";
import type { ContextSummary } from "../types/context";

interface HistoryPanelProps {
  title?: string;
  items: ContextSummary[];
  isLoading?: boolean;
  onRefresh?: () => void;
  onSelect?: (item: ContextSummary) => void;
}

const formatTimestamp = (iso: string) =>
  new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

const kindLabel = (kind: ContextSummary["kind"]) => {
  switch (kind.type) {
    case "CodeSnippet":
      return "Code";
    case "FixHistory":
      return "Fix";
    case "ProjectSummary":
      return "Summary";
    case "Discussion":
      return "Discussion";
    case "ToolLog":
      return "Tool Log";
    case "Other":
      return kind.label || "Other";
    default:
      return "Context";
  }
};

export function HistoryPanel({
  title = "Latest Captures",
  items,
  isLoading = false,
  onRefresh,
  onSelect,
}: HistoryPanelProps) {
  const emptyMessage = useMemo(() => {
    if (isLoading) return "Loading history…";
    return "No captured contexts yet.";
  }, [isLoading]);

  return (
    <section className="rounded-3xl border border-white/5 bg-surface-200/70 p-5 shadow-card backdrop-blur">
      <header className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.35em] text-brand-200">
            Context Log
          </p>
          <h2 className="mt-1 text-xl font-semibold text-white">{title}</h2>
          <p className="text-sm text-slate-300">
            Most recent IDE snapshots ingested into Ingat.
          </p>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          disabled={isLoading}
          className="inline-flex items-center justify-center rounded-2xl border border-white/10 px-4 py-2 text-sm font-semibold text-white transition hover:border-brand-400 hover:bg-brand-500/10 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {isLoading ? "Refreshing…" : "Refresh"}
        </button>
      </header>

      {items.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-white/10 bg-black/10 p-6 text-center text-sm text-slate-400">
          {emptyMessage}
        </div>
      ) : (
        <ol className="space-y-4">
          {items.map((item, index) => (
            <li
              key={item.id}
              className="group relative flex cursor-pointer gap-4 rounded-2xl border border-white/5 bg-black/15 p-4 transition hover:border-brand-500/60 hover:bg-black/25"
              onClick={() => onSelect?.(item)}
            >
              <span className="absolute left-4 top-4 text-xs font-semibold text-slate-500">
                {String(items.length - index).padStart(2, "0")}
              </span>
              <div className="mt-6 flex min-w-[2.5rem] flex-col items-center gap-2">
                <span className="h-10 w-0.5 rounded-full bg-gradient-to-b from-brand-400 to-transparent" />
                <span className="rounded-full bg-brand-500/20 px-2 py-0.5 text-[0.65rem] font-semibold uppercase tracking-widest text-brand-100">
                  {kindLabel(item.kind)}
                </span>
              </div>
              <div className="flex-1 space-y-2">
                <div className="flex flex-wrap items-center gap-2 text-xs text-slate-400">
                  <span className="rounded-full border border-white/10 px-2 py-0.5 text-white/70">
                    {item.project}
                  </span>
                  <span>{formatTimestamp(item.created_at)}</span>
                </div>
                <h3 className="text-base font-semibold text-white transition group-hover:text-brand-100">
                  {item.summary}
                </h3>
                <div className="flex flex-wrap gap-1.5 text-xs text-slate-300">
                  {item.tags.slice(0, 6).map((tag) => (
                    <span
                      key={tag}
                      className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[0.7rem]"
                    >
                      #{tag}
                    </span>
                  ))}
                  {item.tags.length > 6 && (
                    <span className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[0.7rem] text-slate-400">
                      +{item.tags.length - 6} more
                    </span>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
