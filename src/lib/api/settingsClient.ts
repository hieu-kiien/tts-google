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

export async function getApiKeyStatus(): Promise<{ configured: boolean }> {
  return await invoke<{ configured: boolean }>("get_api_key_status");
}

export async function saveApiKey(key: string, remember: boolean): Promise<{ configured: boolean }> {
  return await invoke<{ configured: boolean }>("save_api_key", { key, remember });
}

export async function testApiConnection(testKey?: string | null): Promise<string> {
  return await invoke<string>("test_api_connection", { testKey: testKey || null });
}

export interface ApiKeysInfo {
  count: number;
  keys_masked: string[];
  configured: boolean;
}

export async function getApiKeysInfo(): Promise<ApiKeysInfo> {
  return await invoke<ApiKeysInfo>("get_api_keys_info");
}

export async function saveApiKeys(keys: string[], remember: boolean): Promise<ApiKeysInfo> {
  return await invoke<ApiKeysInfo>("save_api_keys", { keys, remember });
}

export async function removeApiKeyAt(index: number): Promise<ApiKeysInfo> {
  return await invoke<ApiKeysInfo>("remove_api_key_at", { index });
}
