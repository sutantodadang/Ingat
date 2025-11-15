import { useMemo } from "react";
import type { ContextSummary, SearchResult } from "../types/context";

type ContextDetailsPayload =
  | (SearchResult & { body: string })
  | (ContextSummary & { body?: string });

interface ContextDetailsProps {
  selection?: ContextDetailsPayload | null;
  onClose?: () => void;
  onCopy?: (text: string) => void;
}

const formatDateTime = (iso: string) =>
  new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

const kindLabel = (kind: ContextDetailsPayload["kind"]) => {
  switch (kind.type) {
    case "CodeSnippet":
      return "Code Snippet";
    case "FixHistory":
      return "Fix History";
    case "ProjectSummary":
      return "Project Summary";
    case "Discussion":
      return "Discussion";
    case "ToolLog":
      return "Tool Log";
    case "Other":
      return kind.label || "Custom";
    default:
      return "Context";
  }
};

export function ContextDetails({
  selection,
  onClose,
  onCopy,
}: ContextDetailsProps) {
  const content = useMemo(() => {
    if (!selection) return null;

    const {
      summary,
      project,
      created_at,
      tags,
      body = "No body available for this entry.",
      kind,
    } = selection;

    return {
      summary,
      project,
      created_at,
      tags,
      body,
      kindLabel: kindLabel(kind),
    };
  }, [selection]);

  if (!content) {
    return (
      <aside className="rounded-3xl border border-dashed border-white/10 bg-black/10 p-6 text-center text-sm text-slate-400">
        <h3 className="text-lg font-semibold text-white">Context details</h3>
        <p>Select a search result or capture to inspect its content.</p>
      </aside>
    );
  }

  return (
    <aside className="rounded-3xl border border-white/5 bg-surface-200/70 p-6 shadow-card backdrop-blur">
      <header className="mb-4 flex flex-col gap-3 border-b border-white/5 pb-4 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.35em] text-brand-200">
            {content.kindLabel}
          </p>
          <h3 className="mt-1 text-2xl font-semibold text-white">
            {content.summary}
          </h3>
          <p className="text-sm text-slate-300">{content.project}</p>
          <small className="text-xs text-slate-400">
            {formatDateTime(content.created_at)}
          </small>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="self-start rounded-2xl border border-white/10 px-4 py-2 text-xs font-semibold text-white transition hover:border-brand-400"
        >
          Close
        </button>
      </header>

      <section className="mb-4 space-y-3">
        <h4 className="text-sm font-semibold uppercase tracking-[0.3em] text-brand-100">
          Tags
        </h4>
        <div className="flex flex-wrap gap-2">
          {content.tags.length === 0 && (
            <span className="rounded-full border border-white/10 bg-black/20 px-2 py-0.5 text-xs text-slate-400">
              No tags
            </span>
          )}
          {content.tags.map((tag) => (
            <span
              key={tag}
              className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-xs text-slate-100"
            >
              #{tag}
            </span>
          ))}
        </div>
      </section>

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-semibold uppercase tracking-[0.3em] text-brand-100">
            Captured body
          </h4>
          <button
            type="button"
            className="rounded-2xl border border-white/10 px-3 py-1 text-xs font-semibold text-white transition hover:border-brand-400"
            onClick={() => onCopy?.(content.body)}
          >
            Copy text
          </button>
        </div>
        <pre className="max-h-80 overflow-auto rounded-2xl border border-white/5 bg-black/40 p-4 text-sm text-slate-200">
          {content.body}
        </pre>
      </section>
    </aside>
  );
}
