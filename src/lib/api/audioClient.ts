import { invoke } from "@tauri-apps/api/core";

export interface SubtitleExportResult {
  output_path: string;
  content: string;
}

export interface MergeResult {
  output_path: string;
  total_duration_ms: number;
  warning?: string | null;
}

export async function writeBinaryFile(targetPath: string, base64Data: string): Promise<string> {
  return await invoke<string>("write_binary_file", { targetPath, base64Data });
}

export async function exportProjectSrt(
  projectId: string,
  silenceGapMs: number = 500,
  customOutputPath?: string
): Promise<SubtitleExportResult> {
  return await invoke<SubtitleExportResult>("export_project_srt", {
    projectId,
    silenceGapMs,
    customOutputPath,
  });
}

export async function exportProjectVtt(
  projectId: string,
  silenceGapMs: number = 500,
  customOutputPath?: string
): Promise<SubtitleExportResult> {
  return await invoke<SubtitleExportResult>("export_project_vtt", {
    projectId,
    silenceGapMs,
    customOutputPath,
  });
}

export async function mergeProjectAudio(
  projectId: string,
  silenceGapMs: number = 500,
  customOutputPath?: string
): Promise<MergeResult> {
  return await invoke<MergeResult>("merge_project_audio", {
    projectId,
    silenceGapMs,
    customOutputPath,
  });
}

export async function readAudioDataUrl(path?: string, filePath?: string): Promise<string> {
  return await invoke<string>("read_audio_data_url", { path, filePath });
}

export async function synthesizePreviewAudio(params: {
  voice: string;
  text: string;
  model?: string;
  speed?: number;
  pitch?: number;
}): Promise<{ data_url: string; duration_ms: number; sample_rate?: number }> {
  return await invoke<{ data_url: string; duration_ms: number; sample_rate?: number }>(
    "synthesize_preview_audio",
    params
  );
}
