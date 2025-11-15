import { useCallback, useEffect, useMemo, useState } from "react";
import { ContextForm } from "./components/ContextForm";
import { SearchPanel } from "./components/SearchPanel";
import { SearchResults } from "./components/SearchResults";
import { ContextDetails } from "./components/ContextDetails";
import { HistoryPanel } from "./components/HistoryPanel";
import { EmbeddingSettings } from "./components/EmbeddingSettings";
import ServiceStatus from "./components/ServiceStatus";
import { memoryClient } from "./lib/memoryClient";
import type {
  ContextSummary,
  HealthStatusResponse,
  SearchRequest,
  SearchResult,
} from "./types/context";

const initialSearchPayload: SearchRequest = {
  prompt: "",
  filters: {},
  limit: 10,
};

type Selection =
  | (SearchResult & { source: "search" })
  | (ContextSummary & { source: "history"; body?: string });

function McpGuideCard() {
  return (
    <section className="rounded-2xl border border-white/5 bg-surface-200/60 p-5 shadow-card backdrop-blur">
      <ServiceStatus />
    </section>
  );
}

function App() {
  const [searchPayload, setSearchPayload] =
    useState<SearchRequest>(initialSearchPayload);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  const [projects, setProjects] = useState<string[]>([]);
  const [historyItems, setHistoryItems] = useState<ContextSummary[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);

  const [selection, setSelection] = useState<Selection | null>(null);

  const [health, setHealth] = useState<HealthStatusResponse | null>(null);
  const [statusMessage, setStatusMessage] = useState<string>("");
  const [errorMessage, setErrorMessage] = useState<string>("");

  const handleError = useCallback((message: string) => {
    setErrorMessage(message);
    setTimeout(() => setErrorMessage(""), 4000);
  }, []);

  const refreshProjects = useCallback(async () => {
    try {
      const data = await memoryClient.listProjects();
      setProjects(data);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Unable to load project list.";
      handleError(message);
    }
  }, [handleError]);

  const refreshHistory = useCallback(
    async (limit = 12) => {
      try {
        setHistoryLoading(true);
        const response = await memoryClient.fetchRecent({ limit });
        setHistoryItems(response.items);
      } catch (error) {
        const message =
          error instanceof Error
            ? error.message
            : "Unable to load recent contexts.";
        handleError(message);
      } finally {
        setHistoryLoading(false);
      }
    },
    [handleError],
  );

  const refreshHealth = useCallback(async () => {
    try {
      const result = await memoryClient.health();
      setHealth(result);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Health check failed.";
      handleError(message);
    }
  }, [handleError]);

  const runSearch = useCallback(async () => {
    if (!searchPayload.prompt.trim()) {
      setStatusMessage("Enter a search prompt to begin.");
      return;
    }

    try {
      setIsSearching(true);
      setStatusMessage("Searching contexts…");
      const response = await memoryClient.searchContexts(searchPayload);
      setSearchResults(response.results);
      setStatusMessage(
        response.results.length
          ? `Found ${response.results.length} relevant matches.`
          : "No semantic matches yet.",
      );
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Unable to complete search request.";
      handleError(message);
      setStatusMessage(message);
    } finally {
      setIsSearching(false);
    }
  }, [handleError, searchPayload]);

  const handleContextSaved = useCallback(
    async (summaryId: string) => {
      setStatusMessage(`Context stored (id: ${summaryId}).`);
      await refreshHistory();
      await refreshProjects();
    },
    [refreshHistory, refreshProjects],
  );

  useEffect(() => {
    refreshProjects();
    refreshHistory();
    refreshHealth();
  }, [refreshHistory, refreshProjects, refreshHealth]);

  const handleCopy = useCallback(
    async (text: string) => {
      try {
        await navigator.clipboard.writeText(text);
        setStatusMessage("Copied body to clipboard.");
      } catch {
        handleError("Clipboard copy failed. Select text manually.");
      }
    },
    [handleError],
  );

  const healthIndicator = useMemo(() => {
    if (!health) return "Unknown";
    return health.ok ? "Ready" : "Degraded";
  }, [health]);

  return (
    <div className="min-h-screen bg-surface-50 text-slate-100">
      <div className="mx-auto flex max-w-7xl flex-col gap-8 px-6 py-10">
        <header className="flex flex-col justify-between gap-6 rounded-3xl border border-white/5 bg-surface-100/70 p-6 shadow-card backdrop-blur md:flex-row md:items-center">
          <div>
            <p className="inline-flex items-center rounded-full bg-brand-500/10 px-3 py-1 text-xs font-semibold uppercase tracking-widest text-brand-200">
              Ingat Local Vault
            </p>
            <h1 className="mt-3 text-3xl font-semibold text-white">
              Context that stays on your machine
            </h1>
            <p className="mt-2 max-w-2xl text-sm text-slate-300">
              Capture IDE snippets, fixes, and discussions locally, then
              retrieve them with deterministic or semantic embeddings. Optional
              MCP tooling lets any editor stream fresh context in.
            </p>
          </div>

          <div className="flex flex-col items-start gap-2 text-sm text-slate-200 md:items-end">
            <span
              className={`inline-flex items-center rounded-full px-3 py-1 text-xs font-semibold ${
                health?.ok
                  ? "bg-emerald-500/15 text-emerald-200"
                  : "bg-amber-500/15 text-amber-200"
              }`}
            >
              Health: {healthIndicator}
            </span>
            {statusMessage && (
              <small className="text-slate-300">{statusMessage}</small>
            )}
            {errorMessage && (
              <small className="text-rose-300">{errorMessage}</small>
            )}
            {health?.details && (
              <small className="text-xs text-slate-400">{health.details}</small>
            )}
          </div>
        </header>

        <main className="grid grid-cols-1 gap-6 lg:grid-cols-[320px_minmax(0,1fr)_360px]">
          <div className="space-y-6">
            <ContextForm onSaved={handleContextSaved} onError={handleError} />
            <HistoryPanel
              title="Recent Captures"
              items={historyItems}
              isLoading={historyLoading}
              onRefresh={() => refreshHistory()}
              onSelect={(item) => setSelection({ ...item, source: "history" })}
            />
          </div>

          <div className="space-y-6">
            <SearchPanel
              value={searchPayload}
              projects={projects}
              disabled={isSearching}
              isSearching={isSearching}
              onChange={setSearchPayload}
              onSubmit={runSearch}
            />
            <SearchResults
              results={searchResults}
              isSearching={isSearching}
              emptyMessage={statusMessage}
              onSelect={(result) =>
                setSelection({ ...result, source: "search" })
              }
              onCopyBody={(result) => handleCopy(result.body)}
            />
          </div>

          <div className="space-y-6">
            <ContextDetails
              selection={selection}
              onClose={() => setSelection(null)}
              onCopy={(text) => handleCopy(text)}
            />
            <EmbeddingSettings />
            <McpGuideCard />
          </div>
        </main>
      </div>
    </div>
  );
}

export default App;
