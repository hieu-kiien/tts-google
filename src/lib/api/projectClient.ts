import { invoke } from "@tauri-apps/api/core";
import type { ProjectRecord, SegmentRecord } from "../types/tts";

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

export async function getProjectSegments(projectId: string): Promise<SegmentRecord[]> {
  return await invoke<SegmentRecord[]>("get_project_segments", { projectId });
}
