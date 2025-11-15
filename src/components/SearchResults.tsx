import type { SearchResult } from "../types/context";

interface SearchResultsProps {
  results: SearchResult[];
  isSearching?: boolean;
  emptyMessage?: string;
  onSelect?: (result: SearchResult) => void;
  onCopyBody?: (result: SearchResult) => void;
}

const formatScore = (score: number) => `${(score * 100).toFixed(1)}% match`;

const formatDate = (iso: string) =>
  new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

export function SearchResults({
  results,
  isSearching = false,
  emptyMessage = "No semantic matches yet.",
  onSelect,
  onCopyBody,
}: SearchResultsProps) {
  if (isSearching) {
    return (
      <section className="rounded-3xl border border-white/5 bg-surface-200/60 p-6 text-center text-slate-200 shadow-card backdrop-blur">
        <p>Searching contexts…</p>
      </section>
    );
  }

  if (!results.length) {
    return (
      <section className="rounded-3xl border border-dashed border-white/10 bg-black/15 p-6 text-center text-sm text-slate-400">
        {emptyMessage}
      </section>
    );
  }

  return (
    <section className="space-y-4">
      {results.map((result) => (
        <article
          key={result.id}
          className="group rounded-3xl border border-white/5 bg-surface-200/70 p-5 shadow-card backdrop-blur transition hover:border-brand-500/60 hover:bg-black/20 focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-400"
          role="button"
          tabIndex={0}
          onClick={() => onSelect?.(result)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              onSelect?.(result);
            }
          }}
        >
          <header className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p className="text-xs uppercase tracking-[0.25em] text-brand-200">
                {result.project}
              </p>
              <h3 className="mt-1 text-lg font-semibold text-white group-hover:text-brand-100">
                {result.summary}
              </h3>
            </div>
            <span className="rounded-full bg-brand-500/15 px-3 py-1 text-xs font-semibold text-brand-100">
              {formatScore(result.score)}
            </span>
          </header>

          <p className="mt-3 max-h-48 overflow-hidden whitespace-pre-line text-sm text-slate-200">
            {result.body}
          </p>

          <footer className="mt-4 flex flex-col gap-3 border-t border-white/5 pt-4 text-sm text-slate-300 md:flex-row md:items-center md:justify-between">
            <div className="flex flex-wrap gap-2">
              {result.tags.slice(0, 4).map((tag) => (
                <span
                  key={tag}
                  className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-xs"
                >
                  #{tag}
                </span>
              ))}
              {result.tags.length > 4 && (
                <span className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-xs text-slate-400">
                  +{result.tags.length - 4} more
                </span>
              )}
            </div>

            <div className="flex flex-wrap items-center gap-3 text-xs text-slate-400">
              <span>{formatDate(result.created_at)}</span>
              <button
                type="button"
                className="rounded-2xl border border-white/10 px-3 py-1 text-xs font-semibold text-white transition hover:border-brand-400"
                onClick={(event) => {
                  event.stopPropagation();
                  onSelect?.(result);
                }}
              >
                Inspect
              </button>
              <button
                type="button"
                className="rounded-2xl border border-white/10 px-3 py-1 text-xs font-semibold text-white transition hover:border-brand-400"
                onClick={(event) => {
                  event.stopPropagation();
                  onCopyBody?.(result);
                }}
              >
                Copy body
              </button>
            </div>
          </footer>
        </article>
      ))}
    </section>
  );
}
