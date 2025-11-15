import { invoke } from "@tauri-apps/api/core";

import type {
  ContextSummary,
  EmbeddingBackendListResponse,
  HealthStatusResponse,
  IngestContextRequest,
  SearchRequest,
  SearchResponse,
  SummaryListResponse,
  UpdateEmbeddingBackendRequest,
} from "../types/context";

const COMMANDS = {
  ingest: "ingest_context",

  search: "search_contexts",

  recent: "recent_contexts",

  projects: "list_projects",

  health: "health",

  embeddingBackends: "embedding_backends",
  setEmbeddingBackend: "set_embedding_backend",
} as const;

type CommandKey = keyof typeof COMMANDS;

async function invokeOrThrow<T>(
  command: CommandKey,
  payload?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(COMMANDS[command], payload);
  } catch (error) {
    const message =
      error instanceof Error ? error.message : JSON.stringify(error);
    throw new Error(`[memoryClient:${COMMANDS[command]}] ${message}`);
  }
}

export const memoryClient = {
  ingestContext(payload: IngestContextRequest): Promise<ContextSummary> {
    return invokeOrThrow("ingest", { payload });
  },

  searchContexts(payload: SearchRequest): Promise<SearchResponse> {
    return invokeOrThrow("search", { payload });
  },

  fetchRecent(options: {
    project?: string;
    limit?: number;
  }): Promise<SummaryListResponse> {
    return invokeOrThrow("recent", options);
  },

  listProjects(): Promise<string[]> {
    return invokeOrThrow("projects");
  },

  health(): Promise<HealthStatusResponse> {
    return invokeOrThrow("health");
  },

  listEmbeddingBackends(): Promise<EmbeddingBackendListResponse> {
    return invokeOrThrow("embeddingBackends");
  },

  updateEmbeddingBackend(
    payload: UpdateEmbeddingBackendRequest,
  ): Promise<EmbeddingBackendListResponse> {
    return invokeOrThrow("setEmbeddingBackend", { payload });
  },
};
