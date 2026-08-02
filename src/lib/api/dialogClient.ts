import { invoke } from "@tauri-apps/api/core";

export async function saveMasterWavDialog(defaultFilename: string): Promise<string | null> {
  return await invoke<string | null>("save_master_wav_dialog", { defaultFilename });
}

export async function saveSrtFileDialog(defaultFilename: string): Promise<string | null> {
  return await invoke<string | null>("save_srt_file_dialog", { defaultFilename });
}

export async function readTextFileDialog(multiple?: boolean): Promise<{ file_path: string; content: string } | string[] | string | null> {
  return await invoke<{ file_path: string; content: string } | string[] | string | null>("read_text_file_dialog", { multiple });
}

export async function readTextFileContent(path: string): Promise<string> {
  return await invoke<string>("read_text_file_content", { path });
}
