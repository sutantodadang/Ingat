import { useCallback, useEffect, useMemo, useState } from "react";
import {
  type EmbeddingBackendListResponse,
  type EmbeddingBackendOption,
} from "../types/context";
import { memoryClient } from "../lib/memoryClient";

interface PendingSelection {
  backendId: string;
  modelOverride: string;
}

interface RequestState {
  status: "idle" | "loading" | "saving" | "error";
  message: string | null;
}

export function EmbeddingSettings() {
  const [backends, setBackends] = useState<EmbeddingBackendListResponse | null>(
    null,
  );
  const [pending, setPending] = useState<PendingSelection>({
    backendId: "",
    modelOverride: "",
  });
  const [request, setRequest] = useState<RequestState>({
    status: "idle",
    message: null,
  });

  const activeOption = useMemo(() => {
    if (!backends) return undefined;
    return backends.options.find((option) => option.id === backends.active);
  }, [backends]);

  const hasChanges = useMemo(() => {
    if (!backends || !pending.backendId) return false;

    const selectedOption =
      backends.options.find((option) => option.id === pending.backendId) ??
      activeOption;

    const backendChanged = pending.backendId !== backends.active;

    const overrideChanged =
      pending.modelOverride.trim() !== (selectedOption?.model ?? "").trim();

    return backendChanged || overrideChanged;
  }, [activeOption, backends, pending.backendId, pending.modelOverride]);

  const loadBackends = useCallback(async () => {
    setRequest({ status: "loading", message: "Loading embedding backends…" });
    try {
      const response = await memoryClient.listEmbeddingBackends();
      setBackends(response);
      const active = response.options.find(
        (option) => option.id === response.active,
      );
      setPending({
        backendId: response.active,
        modelOverride: active?.model ?? "",
      });
      setRequest({ status: "idle", message: null });
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Unable to fetch embedding backends.";
      setRequest({ status: "error", message });
    }
  }, []);

  const applyChanges = useCallback(async () => {
    if (!hasChanges || !pending.backendId) {
      return;
    }
    setRequest({ status: "saving", message: "Updating embedding backend…" });
    try {
      const response = await memoryClient.updateEmbeddingBackend({
        backend_id: pending.backendId,
        model_override: pending.modelOverride.trim() || undefined,
      });
      setBackends(response);
      const active = response.options.find(
        (option) => option.id === response.active,
      );
      setPending({
        backendId: response.active,
        modelOverride: active?.model ?? "",
      });
      setRequest({ status: "idle", message: "Embedding backend updated." });
      setTimeout(() => setRequest({ status: "idle", message: null }), 2500);
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Failed to update embedding backend.";
      setRequest({ status: "error", message });
    }
  }, [hasChanges, pending.backendId, pending.modelOverride]);

  useEffect(() => {
    loadBackends();
  }, [loadBackends]);

  if (!backends && request.status === "loading") {
    return (
      <section className="rounded-3xl border border-white/5 bg-surface-200/70 p-6 text-center text-sm text-slate-300 shadow-card backdrop-blur">
        Loading available backends…
      </section>
    );
  }

  if (!backends) {
    return (
      <section className="rounded-3xl border border-white/5 bg-surface-200/70 p-6 shadow-card backdrop-blur">
        <header className="mb-4">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-brand-200">
            Embedding Settings
          </p>
          <h2 className="text-xl font-semibold text-white">
            Configure embedding backend
          </h2>
        </header>
        <p className="text-sm text-rose-300">
          {request.message ?? "Embedding backends are unavailable."}
        </p>
        <button
          type="button"
          onClick={loadBackends}
          className="mt-4 rounded-2xl border border-white/10 px-4 py-2 text-sm font-semibold text-white transition hover:border-brand-400"
        >
          Retry
        </button>
      </section>
    );
  }

  return (
    <section className="rounded-3xl border border-white/5 bg-surface-200/70 p-6 shadow-card backdrop-blur">
      <header className="mb-5">
        <p className="text-xs font-semibold uppercase tracking-[0.3em] text-brand-200">
          Embedding Settings
        </p>
        <h2 className="text-2xl font-semibold text-white">
          Choose your embedding engine
        </h2>
        <p className="text-sm text-slate-300">
          Swap between the deterministic hash embedder or optional semantic
          engines compiled into this build. Changes apply instantly.
        </p>
      </header>

      <div className="space-y-4">
        {backends.options.map((option: EmbeddingBackendOption) => {
          const isActive = pending.backendId === option.id;
          return (
            <label
              key={option.id}
              className={`flex cursor-pointer flex-col gap-2 rounded-2xl border p-4 transition ${
                isActive
                  ? "border-brand-400 bg-brand-500/5 shadow-glow"
                  : "border-white/8 bg-black/10 hover:border-brand-400/50"
              }`}
            >
              <div className="flex items-center gap-3">
                <input
                  type="radio"
                  name="embedding-backend"
                  value={option.id}
                  checked={isActive}
                  disabled={request.status === "saving"}
                  onChange={() =>
                    setPending((current) => ({
                      ...current,
                      backendId: option.id,
                      modelOverride: option.model,
                    }))
                  }
                  className="h-4 w-4 accent-brand-400"
                />
                <div className="flex flex-col">
                  <span className="text-base font-semibold text-white">
                    {option.label}
                  </span>
                  <small className="text-xs text-slate-400">
                    Default model: {option.model}
                  </small>
                </div>
                {option.feature_gated && (
                  <span className="ml-auto rounded-full bg-white/10 px-2 py-0.5 text-[0.65rem] font-semibold uppercase tracking-[0.2em] text-slate-200">
                    Feature gated
                  </span>
                )}
              </div>
              <p className="text-sm text-slate-300">{option.description}</p>
              {option.dimensions && (
                <span className="text-xs text-slate-400">
                  Expected dimensions: {option.dimensions}
                </span>
              )}
            </label>
          );
        })}
      </div>

      <label className="mt-5 flex flex-col gap-2 text-sm text-slate-200">
        Custom model (optional)
        <input
          type="text"
          placeholder="Override model identifier, e.g. BAAI/bge-small-en-v1.5"
          value={pending.modelOverride}
          onChange={(event) =>
            setPending((current) => ({
              ...current,
              modelOverride: event.target.value,
            }))
          }
          disabled={request.status === "saving"}
          className="rounded-2xl border border-white/10 bg-black/20 px-3 py-2 text-white placeholder-slate-500 focus:border-brand-400 focus:outline-none disabled:opacity-50"
        />
        <small className="text-xs text-slate-400">
          Leave empty to use the backend&apos;s default model.
        </small>
      </label>

      <footer className="mt-6 flex flex-col gap-3 border-t border-white/5 pt-4 text-sm text-slate-300 md:flex-row md:items-center md:justify-between">
        <div
          className={`text-xs ${
            request.status === "error" ? "text-rose-300" : "text-slate-400"
          }`}
        >
          {request.message ?? "Changes apply immediately after saving."}
        </div>
        <div className="flex gap-3">
          <button
            type="button"
            className="rounded-2xl border border-white/10 px-4 py-2 text-xs font-semibold text-white transition hover:border-brand-400 disabled:cursor-not-allowed disabled:opacity-40"
            onClick={loadBackends}
            disabled={request.status === "saving"}
          >
            Refresh
          </button>
          <button
            type="button"
            onClick={applyChanges}
            disabled={!hasChanges || request.status === "saving"}
            className="rounded-2xl bg-gradient-to-r from-brand-500 to-brand-400 px-6 py-2 text-xs font-semibold text-white shadow-glow transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-40"
          >
            {request.status === "saving" ? "Saving…" : "Save changes"}
          </button>
        </div>
      </footer>
    </section>
  );
}
