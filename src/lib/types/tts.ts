export type QueueState = "Idle" | "Running" | "Paused" | "Cancelled";

export interface ProjectRecord {
  id: string;
  name: string;
  source_text: string;
  model: string;
  voice: string;
  preset: string;
  pacing: string;
  pronunciation_notes?: string | null;
  output_directory: string;
  status: string;
  created_at: string;
  updated_at: string;
  chapter_count?: number;
  segment_count?: number;
  completed_count?: number;
  is_pinned?: boolean;
}

export type ReviewStatus = "unreviewed" | "approved" | "needs_fix";

export type SegmentIssue = 
  | "mispronunciation"
  | "missing_words"
  | "extra_words"
  | "repeated_words"
  | "wrong_voice"
  | "wrong_emotion"
  | "speed_mismatch"
  | "unusual_pause"
  | "audio_clipping"
  | "noise_distortion";

export interface AudioVersion {
  id: string;
  version_number: number;
  audio_path: string;
  created_at: string;
  voice: string;
  model: string;
  note?: string;
}

export interface SegmentRecord {
  id: string;
  project_id: string;
  position: number;
  text: string;                  // Original source text
  spoken_text?: string;          // Normalized text to send to TTS
  prompt: string;                // Audio style / director prompt
  voice?: string | null;
  model?: string | null;
  status: "pending" | "queued" | "processing" | "success" | "approved" | "retry_wait" | "failed" | "stale";

  // Standard Segment Flags (Section 5.3 & 5.7)
  is_locked?: boolean;
  is_skipped?: boolean;
  notes?: string;
  review_status?: ReviewStatus;
  reported_issue?: SegmentIssue | null;
  versions?: AudioVersion[];

  attempts: number;
  audio_path?: string | null;
  duration_ms: number;
  error_code?: number | null;
  error_message?: string | null;
  created_at: string;
  updated_at: string;
  fingerprint?: string | null;
  output_fingerprint?: string | null;
  attempt_count: number;
  next_retry_at?: number | null;
  queued_at?: number | null;
  started_at?: number | null;
  finished_at?: number | null;
  lease_owner?: string | null;
  lease_expires_at?: number | null;
  last_error_code?: string | null;
  last_error_message?: string | null;
  cancel_requested: boolean;
  state_revision: number;
  output_size: number;
}

export interface QueueSnapshot {
  project_id: string;
  queue_state: QueueState;
  total_segments: number;
  completed_segments: number;
  failed_segments: number;
  pending_segments: number;
  snapshot_revision: number;
}

export interface QueueProgressEvent {
  stream_id: string;
  sequence: number;
  project_id: string;
  segment_id?: string | null;
  position: number;
  total_segments: number;
  completed_segments: number;
  status: string;
  revision: number;
  error_message?: string | null;
}

export interface ExportReadiness {
  state: "ready" | "queue_not_started" | "queue_running" | "partial" | "empty_project";
  successful_segments: number;
  failed_segments: number;
  total_segments: number;
  output_directory: string;
}

export interface CommandError {
  code: string;
  message: string;
  retryable: boolean;
  diagnostic_id?: string | null;
}

export interface PronunciationRule {
  id: string;
  find: string;
  replace: string;
}
