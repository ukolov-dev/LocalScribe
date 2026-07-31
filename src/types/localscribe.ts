export type ModelInfo = {
  id: string;
  name: string;
  fileName?: string | null;
  sizeLabel: string;
  sizeBytes?: number | null;
  recommended: boolean;
  qualityLabel: string;
  installed: boolean;
  path?: string | null;
  custom: boolean;
};

export type SpeakerModelFileInfo = {
  id: string;
  fileName: string;
  path?: string | null;
  installed: boolean;
  sizeLabel: string;
  sizeBytes?: number | null;
};

export type SpeakerModelInfo = {
  id: string;
  name: string;
  sizeLabel: string;
  sizeBytes?: number | null;
  recommended: boolean;
  qualityLabel: string;
  runtimeLabel: string;
  installed: boolean;
  custom: boolean;
  files: SpeakerModelFileInfo[];
};

export type DownloadProgress = {
  modelId: string;
  downloadedBytes: number;
  totalBytes?: number | null;
  progress: number;
};

export type TranscriptSegment = {
  index: number;
  startMs: number;
  endMs: number;
  text: string;
  speaker?: string | null;
};

export type TranscriptionSegmentEvent = {
  jobId: string;
  segment: TranscriptSegment;
};

export type TranscriptionProgress = {
  jobId: string;
  phase: string;
  progress: number;
};

export type BenchmarkPhase = {
  phase: string;
  durationMs: number;
};

export type TranscriptionBenchmark = {
  totalMs: number;
  audioDurationMs: number;
  phases: BenchmarkPhase[];
};

export type TranscriptionResult = {
  jobId: string;
  text: string;
  segments: TranscriptSegment[];
  benchmark?: TranscriptionBenchmark | null;
};

export type TranscriptionError = {
  jobId: string;
  code: string;
  message: string;
};

export type TranscribeOptions = {
  jobId: string;
  filePath: string;
  modelId?: string;
  modelPath?: string;
  engine?: "auto" | "metal" | "cpu";
  speakerMode?: string;
  speakerModelId?: string;
  language: string;
  decodingPreset?: "balanced" | "accurate";
  initialPrompt?: string;
  timestamps: boolean;
  threads?: number;
};

export type ExportFormat = "txt" | "md";

export type AppError = {
  code?: string;
  message?: string;
};
