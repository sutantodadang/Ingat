export type ContextKind =
  | { type: "CodeSnippet" }
  | { type: "FixHistory" }
  | { type: "ProjectSummary" }
  | { type: "Discussion" }
  | { type: "ToolLog" }
  | { type: "Other"; label: string };

export interface ContextSummary {
  id: string;
  project: string;
  summary: string;
  kind: ContextKind;
  tags: string[];
  created_at: string;
}

export interface ContextRecord extends ContextSummary {
  ide: string;
  file_path?: string | null;
  language?: string | null;
  body: string;
}

export interface QueryFilters {
  project?: string;
  kind?: ContextKind;
  tag?: string;
  ide?: string;
}

export interface IngestContextRequest {
  project: string;
  ide: string;
  file_path?: string | null;
  language?: string | null;
  summary: string;
  body: string;
  tags: string[];
  kind: ContextKind;
}

export interface SearchRequest {
  prompt: string;
  filters?: QueryFilters;
  limit?: number;
}

export interface SearchResult {
  id: string;
  project: string;
  summary: string;
  body: string;
  tags: string[];
  kind: ContextKind;
  score: number;
  created_at: string;
}

export interface SearchResponse {
  query: string;
  results: SearchResult[];
}

export interface SummaryListResponse {
  items: ContextSummary[];
}

export interface HealthStatusResponse {
  ok: boolean;

  message: string;

  details?: string;
}

export interface EmbeddingBackendOption {
  id: string;
  label: string;
  description: string;
  model: string;
  dimensions?: number | null;
  feature_gated: boolean;
}

export interface EmbeddingBackendListResponse {
  active: string;
  options: EmbeddingBackendOption[];
}

export interface UpdateEmbeddingBackendRequest {
  backend_id: string;
  model_override?: string;
}
