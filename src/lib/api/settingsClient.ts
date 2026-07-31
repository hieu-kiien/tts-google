import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, QuotaMetrics } from "../types/tts";

export async function getAppSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_app_settings");
}

export async function updateAppSettings(concurrency: number): Promise<void> {
  return await invoke<void>("update_app_settings", { concurrency });
}

export async function getQuotaMetrics(): Promise<QuotaMetrics> {
  return await invoke<QuotaMetrics>("get_quota_metrics");
}
