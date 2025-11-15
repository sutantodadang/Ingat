import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ServiceStatus {
  is_running: boolean;
  service_url: string;
}

export default function ServiceStatus() {
  const [status, setStatus] = useState<ServiceStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showGuide, setShowGuide] = useState(false);

  const checkStatus = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ServiceStatus>("service_status");
      setStatus(result);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to check service status",
      );
    } finally {
      setLoading(false);
    }
  };

  const handleStartService = async () => {
    try {
      setLoading(true);
      setError(null);
      await invoke<string>("start_service");
      await checkStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start service");
      setLoading(false);
    }
  };

  const handleStopService = async () => {
    try {
      setLoading(true);
      setError(null);
      await invoke<string>("stop_service");
      await checkStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to stop service");
      setLoading(false);
    }
  };

  useEffect(() => {
    checkStatus();
    // Check status every 10 seconds
    const interval = setInterval(checkStatus, 10000);
    return () => clearInterval(interval);
  }, []);

  if (loading && !status) {
    return (
      <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
        <div className="flex items-center gap-2">
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-blue-500 border-t-transparent"></div>
          <span className="text-sm text-gray-600">
            Checking service status...
          </span>
        </div>
      </div>
    );
  }

  const isRunning = status?.is_running ?? false;
  const serviceUrl = status?.service_url ?? "http://127.0.0.1:3200";

  return (
    <div className="rounded-lg border border-white/10 bg-surface-200/60 p-4 shadow-sm backdrop-blur">
      {/* Header */}
      <div className="mb-4 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-lg font-semibold text-white">MCP Service</h3>
          <div
            className={`flex items-center gap-1 rounded-full px-2 py-1 text-xs font-medium ${
              isRunning
                ? "bg-green-100 text-green-700"
                : "bg-red-100 text-red-700"
            }`}
          >
            <span
              className={`h-2 w-2 rounded-full ${
                isRunning ? "bg-green-500" : "bg-red-500"
              }`}
            ></span>
            {isRunning ? "Running" : "Stopped"}
          </div>
        </div>
        <button
          onClick={checkStatus}
          disabled={loading}
          className="rounded p-1 hover:bg-white/10 disabled:opacity-50"
          title="Refresh status"
        >
          <svg
            className={`h-5 w-5 text-slate-300 ${loading ? "animate-spin" : ""}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
        </button>
      </div>

      {/* Error Message */}
      {error && (
        <div className="mb-4 rounded-md bg-red-500/20 p-3 text-sm text-red-200">
          {error}
        </div>
      )}

      {/* Status Information */}
      <div className="mb-4 space-y-2 text-sm">
        <div className="flex items-center justify-between">
          <span className="text-slate-300">Service URL:</span>
          <code className="rounded bg-black/40 px-2 py-1 text-xs text-slate-200">
            {serviceUrl}
          </code>
        </div>
        {isRunning && (
          <div className="rounded-md bg-blue-500/20 p-3">
            <p className="text-xs text-blue-200">
              ✨ <strong>Multi-client mode enabled!</strong> You can now use
              this UI and your IDEs simultaneously.
            </p>
          </div>
        )}
      </div>

      {/* Action Buttons */}
      <div className="mb-4 flex gap-2">
        {!isRunning ? (
          <button
            onClick={handleStartService}
            disabled={loading}
            className="flex-1 rounded-md bg-brand-500 px-4 py-2 text-sm font-medium text-white hover:bg-brand-600 disabled:opacity-50"
          >
            {loading ? "Starting..." : "Start Service"}
          </button>
        ) : (
          <button
            onClick={handleStopService}
            disabled={loading}
            className="flex-1 rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-50"
          >
            {loading ? "Stopping..." : "Stop Service"}
          </button>
        )}
        <button
          onClick={() => setShowGuide(!showGuide)}
          className="rounded-md border border-white/20 bg-surface-300/50 px-4 py-2 text-sm font-medium text-slate-200 hover:bg-surface-300"
        >
          {showGuide ? "Hide Guide" : "Setup Guide"}
        </button>
      </div>

      {/* Setup Guide */}
      {showGuide && (
        <div className="space-y-4 rounded-md border border-white/10 bg-surface-300/50 p-4">
          <h4 className="font-semibold text-white">
            Connect Your IDE to MCP Service
          </h4>

          <div className="space-y-3 text-sm text-slate-200">
            <div>
              <h5 className="mb-1 font-medium text-white">
                VS Code / Cursor / Windsurf
              </h5>
              <p className="mb-2 text-xs text-slate-300">
                Edit your MCP config (<code>.vscode/mcp.json</code> or
                settings):
              </p>
              <pre className="overflow-x-auto rounded bg-black/60 p-3 text-xs text-slate-100">
                {`{
  "mcpServers": {
    "ingat": {
      "command": "/path/to/mcp-stdio",
      "args": ["--proxy", "${serviceUrl}"],
      "env": {
        "INGAT_LOG": "error"
      }
    }
  }
}`}
              </pre>
            </div>

            <div>
              <h5 className="mb-1 font-medium text-white">Zed</h5>
              <p className="mb-2 text-xs text-slate-300">
                Edit <code>~/.config/zed/settings.json</code>:
              </p>
              <pre className="overflow-x-auto rounded bg-black/60 p-3 text-xs text-slate-100">
                {`{
  "context_servers": {
    "ingat": {
      "settings": {
        "url": "${serviceUrl}/sse"
      }
    }
  }
}`}
              </pre>
            </div>

            <div className="rounded-md bg-blue-500/20 p-3">
              <h5 className="mb-1 text-xs font-medium text-blue-200">
                📚 Documentation
              </h5>
              <ul className="space-y-1 text-xs text-blue-200">
                <li>
                  • <strong>Quick Start:</strong>{" "}
                  <a
                    href="https://github.com/sutantodadang/Ingat/blob/main/QUICK_START.md"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="underline hover:text-blue-100"
                  >
                    QUICK_START.md
                  </a>
                </li>
                <li>
                  • <strong>Full Setup:</strong>{" "}
                  <a
                    href="https://github.com/sutantodadang/Ingat/blob/main/UNIFIED_SERVICE_SETUP.md"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="underline hover:text-blue-100"
                  >
                    UNIFIED_SERVICE_SETUP.md
                  </a>
                </li>
                <li>
                  • <strong>IDE Setup:</strong>{" "}
                  <a
                    href="https://github.com/sutantodadang/Ingat/blob/main/IDE_MCP_SETUP.md"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="underline hover:text-blue-100"
                  >
                    IDE_MCP_SETUP.md
                  </a>
                </li>
              </ul>
            </div>

            <div className="rounded-md bg-yellow-500/20 p-3">
              <h5 className="mb-1 text-xs font-medium text-yellow-200">
                💡 Tips
              </h5>
              <ul className="space-y-1 text-xs text-yellow-200">
                <li>• The service auto-starts when you open this app</li>
                <li>
                  • It runs in the background and persists after you close the
                  UI
                </li>
                <li>
                  • All your contexts are synced across all connected clients
                </li>
                <li>• Save in VS Code, search here - it just works!</li>
                <li>
                  • The service can serve multiple UIs and IDEs simultaneously
                </li>
              </ul>
            </div>

            <div className="rounded-md border-l-4 border-slate-500 bg-surface-400/50 p-3">
              <h5 className="mb-1 text-xs font-medium text-white">
                ⚙️ Advanced
              </h5>
              <p className="text-xs text-slate-300">
                Change service port by setting environment variable:
              </p>
              <code className="mt-1 block rounded bg-black/40 px-2 py-1 text-xs text-slate-200">
                INGAT_SERVICE_PORT=3201
              </code>
            </div>
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <button
              onClick={() => {
                navigator.clipboard.writeText(serviceUrl);
                alert("Service URL copied to clipboard!");
              }}
              className="rounded-md border border-white/20 bg-surface-300/50 px-3 py-1.5 text-xs font-medium text-slate-200 hover:bg-surface-300"
            >
              Copy URL
            </button>
            <button
              onClick={() => setShowGuide(false)}
              className="rounded-md bg-surface-400 px-3 py-1.5 text-xs font-medium text-white hover:bg-surface-500"
            >
              Close Guide
            </button>
          </div>
        </div>
      )}

      {/* What is this? */}
      {!showGuide && (
        <div className="mt-3 border-t border-white/10 pt-3">
          <details className="text-xs text-slate-300">
            <summary className="cursor-pointer font-medium text-slate-200 hover:text-white">
              What is the MCP Service?
            </summary>
            <div className="mt-2 space-y-2">
              <p>
                The MCP (Model Context Protocol) service allows you to use Ingat
                simultaneously with multiple IDEs and this UI - all sharing the
                same knowledge base in real-time.
              </p>
              <p>
                <strong>Benefits:</strong>
              </p>
              <ul className="ml-4 list-disc space-y-1">
                <li>Save contexts in VS Code, search them here instantly</li>
                <li>Use multiple IDEs at once (VS Code + Cursor + Zed)</li>
                <li>All clients stay synchronized automatically</li>
                <li>No database conflicts or locking issues</li>
                <li>Persists in background - close and reopen UI anytime</li>
              </ul>
              <p className="mt-2 text-slate-400">
                The service runs as a detached background process and uses
                minimal resources. It continues running even after you close the
                UI, allowing seamless multi-client usage.
              </p>
            </div>
          </details>
        </div>
      )}
    </div>
  );
}
