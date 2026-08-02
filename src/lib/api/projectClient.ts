import { invoke } from "@tauri-apps/api/core";
import type { ProjectRecord, SegmentRecord, TextChunk } from "../types/tts";

export async function createProject(params: {
  name: string;
  sourceText: string;
  voice: string;
  preset: string;
  chunkMode?: string;
  model?: string;
}): Promise<ProjectRecord> {
  return await invoke<ProjectRecord>("create_project", params);
}

export async function listProjects(): Promise<ProjectRecord[]> {
  return await invoke<ProjectRecord[]>("list_projects");
}

export async function deleteProject(projectId: string): Promise<void> {
  return await invoke<void>("delete_project", { projectId });
}

export async function deleteProjectsBatch(projectIds: string[]): Promise<void> {
  return await invoke<void>("delete_projects_batch", { projectIds });
}

export async function getProjectSegments(projectId: string): Promise<SegmentRecord[]> {
  return await invoke<SegmentRecord[]>("get_project_segments", { projectId });
}

export async function updateSegmentText(projectId: string, segmentId: string, text: string): Promise<void> {
  return await invoke<void>("update_segment_text", { projectId, segmentId, text });
}

export async function updateProjectVoice(projectId: string, voiceId: string): Promise<void> {
  return await invoke<void>("update_project_voice", { projectId, voiceId });
}

export async function updateSegmentVoice(projectId: string, segmentId: string, voice?: string): Promise<void> {
  return await invoke<void>("update_segment_voice", { projectId, segmentId, voice });
}

export async function splitSegment(projectId: string, segmentId: string, splitIndex: number): Promise<void> {
  return await invoke<void>("split_segment", { projectId, segmentId, splitIndex });
}

export async function mergeSegments(projectId: string, segmentId: string): Promise<void> {
  return await invoke<void>("merge_segments", { projectId, segmentId });
}

export async function deleteSegment(projectId: string, segmentId: string): Promise<void> {
  return await invoke<void>("delete_segment", { projectId, segmentId });
}

export async function deleteSegmentsBatch(projectId: string, segmentIds: string[]): Promise<void> {
  return await invoke<void>("delete_segments_batch", { projectId, segmentIds });
}

export async function moveSegment(projectId: string, segmentId: string, direction: string): Promise<void> {
  return await invoke<void>("move_segment", { projectId, segmentId, direction });
}

export async function insertSegmentAt(projectId: string, position: number, text: string): Promise<void> {
  return await invoke<void>("insert_segment_at", { projectId, position, text });
}

export async function rechunkProjectSegments(projectId: string, sourceText: string, mode?: string): Promise<SegmentRecord[]> {
  return await invoke<SegmentRecord[]>("rechunk_project_segments", { projectId, sourceText, mode });
}

export async function chunkTextPreview(text: string, mode?: string): Promise<TextChunk[]> {
  return await invoke<TextChunk[]>("chunk_text_preview", { text, mode: mode || "auto" });
}
