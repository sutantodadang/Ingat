import { useMemo } from "react";
import type { SearchRequest } from "../types/context";

interface SearchPanelProps {
  value: SearchRequest;
  projects: string[];
  disabled?: boolean;
  isSearching?: boolean;
  onChange: (value: SearchRequest) => void;
  onSubmit: () => void;
}

export function SearchPanel({
  value,
  projects,
  disabled = false,
  isSearching = false,
  onChange,
  onSubmit,
}: SearchPanelProps) {
  const isReady = useMemo(() => value.prompt.trim().length > 3, [value.prompt]);

  const setField = <K extends keyof SearchRequest>(
    field: K,
    fieldValue: SearchRequest[K],
  ) => {
    onChange({
      ...value,
      [field]: fieldValue,
    });
  };

  const setFilter = (
    field: keyof NonNullable<SearchRequest["filters"]>,
    fieldValue: string,
  ) => {
    onChange({
      ...value,
      filters: {
        ...value.filters,
        [field]: fieldValue || undefined,
      },
    });
  };

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    if (isReady && !isSearching && !disabled) {
      onSubmit();
    }
  };

  return (
    <section className="rounded-3xl border border-white/5 bg-surface-200/70 p-6 shadow-card backdrop-blur">
      <header className="mb-5 flex flex-col gap-2">
        <p className="text-xs font-semibold uppercase tracking-[0.35em] text-brand-200">
          Retrieve Knowledge
        </p>
        <h2 className="text-2xl font-semibold text-white">Semantic Search</h2>
        <p className="text-sm text-slate-300">
          Describe the bug, fix, or context you need. Ingat embeds locally
          and matches against the vault.
        </p>
      </header>

      <form className="space-y-5" onSubmit={handleSubmit}>
        <label className="flex flex-col gap-2 text-sm text-slate-200">
          Prompt<span className="text-brand-200">*</span>
          <textarea
            rows={4}
            value={value.prompt}
            onChange={(event) => setField("prompt", event.target.value)}
            placeholder="“Show me the fix we used for the async deadlock in scheduler.rs”"
            className="rounded-3xl border border-white/5 bg-black/15 px-4 py-3 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
            required
            disabled={disabled}
          />
        </label>

        <div className="grid gap-4 md:grid-cols-2">
          <label className="flex flex-col gap-1 text-sm text-slate-200">
            Project
            <select
              value={value.filters?.project ?? ""}
              onChange={(event) => setFilter("project", event.target.value)}
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white focus:border-brand-400 focus:outline-none"
              disabled={disabled}
            >
              <option value="">Any</option>
              {projects.map((project) => (
                <option key={project} value={project}>
                  {project}
                </option>
              ))}
            </select>
          </label>

          <label className="flex flex-col gap-1 text-sm text-slate-200">
            Tag
            <input
              type="text"
              value={value.filters?.tag ?? ""}
              onChange={(event) => setFilter("tag", event.target.value)}
              placeholder="bugfix, perf, mcp..."
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
              disabled={disabled}
            />
          </label>

          <label className="flex flex-col gap-1 text-sm text-slate-200">
            IDE
            <select
              value={value.filters?.ide ?? ""}
              onChange={(event) => setFilter("ide", event.target.value)}
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white focus:border-brand-400 focus:outline-none"
              disabled={disabled}
            >
              <option value="">Any</option>
              <option value="zed">Zed</option>
              <option value="vscode">VS Code</option>
              <option value="intellij">IntelliJ</option>
              <option value="cursor">Cursor</option>
              <option value="other">Other</option>
            </select>
          </label>

          <label className="flex flex-col gap-1 text-sm text-slate-200">
            Limit
            <input
              type="number"
              min={1}
              max={32}
              value={value.limit ?? 10}
              onChange={(event) =>
                setField("limit", Number(event.target.value))
              }
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
              disabled={disabled}
            />
          </label>
        </div>

        <footer className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div
            className={`text-sm ${
              isReady ? "text-slate-200" : "text-slate-500"
            }`}
          >
            {isReady
              ? "Semantic search runs locally using your selected embedding engine."
              : "Enter at least 4 characters to enable search."}
          </div>
          <button
            type="submit"
            disabled={!isReady || isSearching || disabled}
            className="rounded-2xl bg-gradient-to-r from-brand-500 to-brand-400 px-6 py-3 text-sm font-semibold text-white shadow-glow transition hover:opacity-90 focus:outline-none disabled:cursor-not-allowed disabled:opacity-40"
          >
            {isSearching ? "Searching..." : "Search Contexts"}
          </button>
        </footer>
      </form>
    </section>
  );
}
