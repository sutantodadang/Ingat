import { useState } from "react";
import type { ContextKind, IngestContextRequest } from "../types/context";
import { memoryClient } from "../lib/memoryClient";

type SubmitState = "idle" | "submitting" | "success" | "error";

interface ContextFormProps {
  onSaved?: (summaryId: string) => void;
  onError?: (message: string) => void;
}

const defaultPayload: IngestContextRequest = {
  project: "",
  ide: "zed",
  file_path: "",
  language: "",
  summary: "",
  body: "",
  tags: [],
  kind: { type: "CodeSnippet" },
};

type KindVariant = ContextKind;
type OtherKind = Extract<KindVariant, { type: "Other" }>;

const isOtherKind = (kind: KindVariant): kind is OtherKind =>
  kind.type === "Other";

const currentOtherLabel = (kind: KindVariant) =>
  isOtherKind(kind) ? kind.label : "";

const normalizeKindSelection = (
  nextValue: string,
  current: KindVariant,
): KindVariant =>
  nextValue === "Other"
    ? { type: "Other", label: currentOtherLabel(current) }
    : ({ type: nextValue } as KindVariant);

const updateOtherLabel = (label: string): OtherKind => ({
  type: "Other",
  label,
});

export function ContextForm({ onSaved, onError }: ContextFormProps) {
  const [payload, setPayload] = useState<IngestContextRequest>(defaultPayload);
  const [tagInput, setTagInput] = useState("");
  const [submitState, setSubmitState] = useState<SubmitState>("idle");
  const [message, setMessage] = useState<string | null>(null);

  const isValid =
    payload.project.trim().length > 1 &&
    payload.summary.trim().length > 5 &&
    payload.body.trim().length > 10;

  const handleFieldChange = <K extends keyof IngestContextRequest>(
    field: K,
    value: IngestContextRequest[K],
  ) => {
    setPayload((prev) => ({
      ...prev,
      [field]: value,
    }));
  };

  const addTag = () => {
    const normalized = tagInput.trim().toLowerCase().replace(/\s+/g, "-");
    if (
      !normalized ||
      payload.tags.includes(normalized) ||
      payload.tags.length >= 12
    ) {
      setTagInput("");
      return;
    }
    handleFieldChange("tags", [...payload.tags, normalized]);
    setTagInput("");
  };

  const removeTag = (tag: string) => {
    handleFieldChange(
      "tags",
      payload.tags.filter((item) => item !== tag),
    );
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!isValid || submitState === "submitting") return;

    setSubmitState("submitting");
    setMessage(null);

    try {
      const result = await memoryClient.ingestContext({
        ...payload,
        file_path: payload.file_path || null,
        language: payload.language || null,
      });
      setSubmitState("success");
      setMessage("Context stored successfully.");
      setPayload((prev) => ({
        ...defaultPayload,
        ide: prev.ide,
      }));
      setTagInput("");
      onSaved?.(result.id);
    } catch (error) {
      const msg =
        error instanceof Error ? error.message : "Failed to save context.";
      setSubmitState("error");
      setMessage(msg);
      onError?.(msg);
    } finally {
      setTimeout(() => {
        setSubmitState("idle");
      }, 1800);
    }
  };

  const statusColor = (() => {
    switch (submitState) {
      case "success":
        return "text-emerald-300";
      case "error":
        return "text-rose-300";
      default:
        return "text-slate-400";
    }
  })();

  return (
    <section className="rounded-3xl border border-white/5 bg-surface-200/70 p-6 shadow-card backdrop-blur">
      <header className="mb-6">
        <p className="text-xs font-semibold uppercase tracking-[0.25em] text-brand-200">
          Capture Context
        </p>
        <h2 className="mt-2 text-2xl font-semibold text-white">
          Store a fresh snapshot
        </h2>
        <p className="text-sm text-slate-300">
          Paste code, fixes, and discussions from your IDE. Everything is kept
          locally.
        </p>
      </header>

      <form className="space-y-6" onSubmit={handleSubmit}>
        <div className="grid gap-4 md:grid-cols-2">
          <label className="flex flex-col gap-1 text-sm text-slate-200">
            Project<span className="text-brand-200">*</span>
            <input
              type="text"
              value={payload.project}
              onChange={(event) =>
                handleFieldChange("project", event.target.value)
              }
              placeholder="org/repo or workspace name"
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
              required
            />
          </label>

          <label className="flex flex-col gap-1 text-sm text-slate-200">
            IDE<span className="text-brand-200">*</span>
            <select
              value={payload.ide}
              onChange={(event) => handleFieldChange("ide", event.target.value)}
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white focus:border-brand-400 focus:outline-none"
            >
              <option value="zed">Zed</option>
              <option value="vscode">VS Code</option>
              <option value="intellij">IntelliJ</option>
              <option value="cursor">Cursor</option>
              <option value="other">Other</option>
            </select>
          </label>

          <label className="flex flex-col gap-1 text-sm text-slate-200">
            File Path
            <input
              type="text"
              value={payload.file_path ?? ""}
              onChange={(event) =>
                handleFieldChange("file_path", event.target.value)
              }
              placeholder="src/lib/service.rs"
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
            />
          </label>

          <label className="flex flex-col gap-1 text-sm text-slate-200">
            Language
            <input
              type="text"
              value={payload.language ?? ""}
              onChange={(event) =>
                handleFieldChange("language", event.target.value)
              }
              placeholder="Rust"
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
            />
          </label>

          <label className="flex flex-col gap-1 text-sm text-slate-200">
            Kind
            <select
              value={payload.kind.type}
              onChange={(event) =>
                handleFieldChange(
                  "kind",
                  normalizeKindSelection(event.target.value, payload.kind),
                )
              }
              className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white focus:border-brand-400 focus:outline-none"
            >
              <option value="CodeSnippet">Code snippet</option>
              <option value="FixHistory">Fix history</option>
              <option value="ProjectSummary">Project summary</option>
              <option value="Discussion">Discussion</option>
              <option value="ToolLog">Tool log</option>
              <option value="Other">Other</option>
            </select>
          </label>

          {isOtherKind(payload.kind) && (
            <label className="flex flex-col gap-1 text-sm text-slate-200">
              Custom label
              <input
                type="text"
                value={payload.kind.label}
                onChange={(event) =>
                  handleFieldChange(
                    "kind",
                    updateOtherLabel(event.target.value),
                  )
                }
                className="rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
              />
            </label>
          )}
        </div>

        <label className="flex flex-col gap-2 text-sm text-slate-200">
          Summary<span className="text-brand-200">*</span>
          <textarea
            rows={3}
            value={payload.summary}
            onChange={(event) =>
              handleFieldChange("summary", event.target.value)
            }
            placeholder="Quick synopsis of the context or issue addressed."
            className="rounded-3xl border border-white/5 bg-black/20 px-4 py-3 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
            required
          />
        </label>

        <label className="flex flex-col gap-2 text-sm text-slate-200">
          Body<span className="text-brand-200">*</span>
          <textarea
            rows={8}
            value={payload.body}
            onChange={(event) => handleFieldChange("body", event.target.value)}
            placeholder="Paste the relevant code, logs, or explanation..."
            className="rounded-3xl border border-white/5 bg-black/20 px-4 py-3 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
            required
          />
        </label>

        <label className="flex flex-col gap-2 text-sm text-slate-200">
          Tags
          <div className="flex gap-3">
            <input
              type="text"
              value={tagInput}
              onChange={(event) => setTagInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  addTag();
                }
              }}
              placeholder="refactor, performance, bugfix..."
              className="flex-1 rounded-2xl border border-white/5 bg-black/20 px-3 py-2 text-base text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none"
            />
            <button
              type="button"
              onClick={addTag}
              disabled={!tagInput.trim()}
              className="rounded-2xl bg-brand-500/80 px-4 py-2 text-sm font-semibold text-white transition hover:bg-brand-400 focus:outline-none disabled:cursor-not-allowed disabled:bg-white/10"
            >
              Add
            </button>
          </div>
        </label>

        {payload.tags.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {payload.tags.map((tag) => (
              <button
                key={tag}
                type="button"
                onClick={() => removeTag(tag)}
                className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-black/30 px-3 py-1 text-xs text-slate-200 transition hover:border-brand-400"
                title="Remove tag"
              >
                #{tag} <span aria-hidden>×</span>
              </button>
            ))}
          </div>
        )}

        <footer className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div className={`text-sm ${statusColor}`}>
            {message ?? "Fields marked * are required. Max 12 tags."}
          </div>
          <button
            type="submit"
            disabled={!isValid || submitState === "submitting"}
            className="rounded-2xl bg-gradient-to-r from-brand-500 to-brand-400 px-6 py-3 text-sm font-semibold text-white shadow-glow transition hover:opacity-90 focus:outline-none disabled:cursor-not-allowed disabled:opacity-40"
          >
            {submitState === "submitting" ? "Saving..." : "Store Context"}
          </button>
        </footer>
      </form>
    </section>
  );
}
