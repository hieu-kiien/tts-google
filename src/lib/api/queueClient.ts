import { invoke } from "@tauri-apps/api/core";
import type { QueueSnapshot, ExportReadiness } from "../types/tts";

export async function enqueueProject(projectId: string): Promise<QueueSnapshot> {
  return await invoke<QueueSnapshot>("enqueue_project", { projectId });
}

export async function pauseProject(projectId: string): Promise<void> {
  return await invoke<void>("pause_project", { projectId });
}

export async function resumeProject(projectId: string): Promise<void> {
  return await invoke<void>("resume_project", { projectId });
}

export async function cancelProject(projectId: string): Promise<void> {
  return await invoke<void>("cancel_project", { projectId });
}

export async function getQueueSnapshot(projectId: string): Promise<QueueSnapshot> {
  return await invoke<QueueSnapshot>("get_queue_snapshot", { projectId });
}

export async function checkExportReadiness(projectId: string): Promise<ExportReadiness> {
  return await invoke<ExportReadiness>("check_export_readiness", { projectId });
}
