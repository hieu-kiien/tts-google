<script lang="ts">
  import { uiState } from "../../state/uiState.svelte";
  import { projectState } from "../../state/projectState.svelte";
  import type { SegmentRecord } from "../../types/tts";
  import { toastStore } from "../../state/toasts.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getErrorMessage } from "../../utils/errorUtils";

  import { convertWavBufferToMp3 } from "../../utils/mp3Exporter";

  let exportScope = $state<"all" | "chapter" | "approved">("all");
  let exportMode = $state<"single" | "chapters">("single");
  let fileFormat = $state<"wav" | "mp3" | "m4b">("wav");
  let mp3Bitrate = $state<number>(192);
  let exportSubtitles = $state(true);
  let normalizeLoudness = $state(true);
  let interChapterPauseMs = $state(1000);

  let customMasterPath = $state<string | null>(null);
  let customSrtPath = $state<string | null>(null);

  let bookTitle = $state(projectState.currentProject?.name || "Dự án Audio Book");
  let bookAuthor = $state("Tác giả");
  let bookReader = $state("Gemini Voice");

  // Pre-export check indicators using Svelte 5 derived
  let ungeneratedCount = $derived.by<number>(() => {
    return projectState.segments.filter((s: SegmentRecord) => s.status !== "success" && s.status !== "approved").length;
  });

  let approvedCount = $derived.by<number>(() => {
    return projectState.segments.filter((s: SegmentRecord) => s.status === "approved").length;
  });

  let totalCount = $derived.by<number>(() => {
    return projectState.segments.length;
  });

  async function handlePickMasterPath() {
    try {
      const ext = fileFormat === "mp3" ? "mp3" : "wav";
      const defaultName = `${projectState.currentProject?.name || "TTS_Master"}.${ext}`.replace(/[\\/:*?"<>|]/g, "_");
      const path = await invoke<string | null>("save_master_wav_dialog", { defaultFilename: defaultName });
      if (path) {
        customMasterPath = path;
        toastStore.showSuccess(`Đã chọn đường dẫn lưu Master ${ext.toUpperCase()}: ${path}`);
      }
    } catch (err: unknown) {
      toastStore.showError(`Lỗi chọn đường dẫn lưu: ${getErrorMessage(err)}`);
    }
  }

  async function handlePickSrtPath() {
    try {
      const defaultName = `${projectState.currentProject?.name || "TTS_Subtitle"}.srt`.replace(/[\\/:*?"<>|]/g, "_");
      const path = await invoke<string | null>("save_srt_file_dialog", { defaultFilename: defaultName });
      if (path) {
        customSrtPath = path;
        toastStore.showSuccess(`Đã chọn đường dẫn lưu SRT: ${path}`);
      }
    } catch (err: unknown) {
      toastStore.showError(`Lỗi chọn đường dẫn lưu: ${getErrorMessage(err)}`);
    }
  }

  function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binaryString = window.atob(base64);
    const len = binaryString.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes.buffer;
  }

  function uint8ArrayToBase64(bytes: Uint8Array): string {
    let binary = '';
    const len = bytes.byteLength;
    for (let i = 0; i < len; i++) {
      binary += String.fromCharCode(bytes[i]);
    }
    return window.btoa(binary);
  }

  async function handleStartExport() {
    if (!projectState.currentProject) {
      toastStore.showError("Chưa có dự án nào được chọn để xuất file.");
      return;
    }

    if (ungeneratedCount > 0 && exportScope === "all") {
      if (!confirm(`Cảnh báo: Có ${ungeneratedCount} đoạn chưa tạo âm thanh. Bạn có muốn tiếp tục ghép các đoạn đã hoàn tất?`)) {
        return;
      }
    }

    try {
      let masterPathToUse = customMasterPath;
      if (!masterPathToUse) {
        const ext = fileFormat === "mp3" ? "mp3" : "wav";
        const defaultName = `${projectState.currentProject.name || "TTS_Master"}.${ext}`.replace(/[\\/:*?"<>|]/g, "_");
        masterPathToUse = await invoke<string | null>("save_master_wav_dialog", { defaultFilename: defaultName });
      }

      if (!masterPathToUse) {
        toastStore.showInfo("Đã hủy thao tác xuất file audio.");
        return;
      }

      toastStore.showInfo("Đang tiến hành ghép các đoạn WAV thành file âm thanh Master...");
      
      const tempWavPath = masterPathToUse.endsWith(".mp3") 
        ? masterPathToUse.replace(/\.mp3$/i, ".tmp.wav") 
        : masterPathToUse;

      const mergeRes = await invoke<{ output_path: string; total_duration_ms: number }>("merge_project_audio", {
        projectId: projectState.currentProject.id,
        silenceGapMs: interChapterPauseMs,
        customOutputPath: tempWavPath
      });

      let finalMasterPath = mergeRes.output_path || tempWavPath;

      if (fileFormat === "mp3") {
        toastStore.showInfo(`Đang nén tệp WAV sang MP3 (${mp3Bitrate} kbps)...`);
        const dataUrl = await invoke<string>("read_audio_data_url", { path: finalMasterPath });
        const base64Data = dataUrl.split(",")[1] || dataUrl;
        const wavBuffer = base64ToArrayBuffer(base64Data);
        
        const mp3Bytes = convertWavBufferToMp3(wavBuffer, mp3Bitrate);
        const mp3Base64 = uint8ArrayToBase64(mp3Bytes);
        
        const targetMp3Path = masterPathToUse.endsWith(".mp3") ? masterPathToUse : `${masterPathToUse}.mp3`;
        await invoke("write_binary_file", { targetPath: targetMp3Path, base64Data: mp3Base64 });
        finalMasterPath = targetMp3Path;
      }

      if (exportSubtitles) {
        let srtPathToUse = customSrtPath;
        if (!srtPathToUse) {
          const defaultSrtName = `${projectState.currentProject.name || "TTS_Subtitle"}.srt`.replace(/[\\/:*?"<>|]/g, "_");
          srtPathToUse = await invoke<string | null>("save_srt_file_dialog", { defaultFilename: defaultSrtName });
        }

        if (srtPathToUse) {
          toastStore.showInfo("Đang tạo file phụ đề SRT...");
          await invoke("export_project_srt", {
            projectId: projectState.currentProject.id,
            silenceGapMs: interChapterPauseMs,
            customOutputPath: srtPathToUse
          });
        }
      }

      toastStore.showSuccess(`Đã xuất file âm thanh ${fileFormat.toUpperCase()} master hoàn tất!\nĐường dẫn: ${finalMasterPath}`);
      uiState.activeView = "editor";
    } catch (err: unknown) {
      toastStore.showError("Lỗi xuất file audio: " + getErrorMessage(err));
    }
  }
</script>

<div class="export-screen-container" role="dialog" aria-modal="true" aria-labelledby="export-title">
  <header class="export-header">
    <h2 id="export-title">📦 Xuất File Âm Thanh & Dự Án</h2>
    <button class="btn btn-secondary" onclick={() => uiState.activeView = 'editor'}>✕ Đóng</button>
  </header>

  <div class="export-body">
    <!-- Left Settings Column -->
    <div class="settings-column">
      <!-- Scope Group -->
      <fieldset class="export-group">
        <legend>1. Nội dung xuất</legend>
        <label><input type="radio" name="scope" value="all" bind:group={exportScope} /> Toàn bộ dự án ({projectState.segments.length} đoạn)</label>
        <label><input type="radio" name="scope" value="chapter" bind:group={exportScope} /> Chương hiện tại được chọn</label>
        <label><input type="radio" name="scope" value="approved" bind:group={exportScope} /> Chỉ các đoạn đã duyệt (Approved)</label>

        <hr />

        <label><input type="radio" name="mode" value="single" bind:group={exportMode} /> Gộp thành một file duy nhất</label>
        <label><input type="radio" name="mode" value="chapters" bind:group={exportMode} /> Tách mỗi chương thành một file âm thanh riêng</label>
      </fieldset>

      <!-- Format & Quality Group -->
      <fieldset class="export-group">
        <legend>2. Định dạng & Đường dẫn xuất OS</legend>
        <div class="field-row">
          <label for="format-select">Định dạng file:</label>
          <select id="format-select" bind:value={fileFormat}>
            <option value="wav">WAV (Uncompressed 24kHz PCM)</option>
            <option value="mp3">MP3 (Compressed Audio)</option>
            <option value="m4b">M4B Audiobook (Hỗ trợ Bookmark)</option>
          </select>
        </div>

        {#if fileFormat === 'mp3'}
          <div class="field-row">
            <label for="bitrate-select">Chất lượng MP3 (Bitrate):</label>
            <select id="bitrate-select" bind:value={mp3Bitrate}>
              <option value={128}>128 kbps (Tiết kiệm dung lượng)</option>
              <option value={192}>192 kbps (Khuyên dùng - Cân bằng tốt nhất)</option>
              <option value={320}>320 kbps (Chất lượng âm thanh tối đa)</option>
            </select>
          </div>
        {/if}

        <div class="field-row">
          <span>File Master Audio ({fileFormat.toUpperCase()}):</span>
          <button class="btn btn-secondary btn-sm" onclick={handlePickMasterPath}>
            📂 {customMasterPath ? `Đổi đường dẫn ${fileFormat.toUpperCase()}...` : `Chọn nơi lưu ${fileFormat.toUpperCase()} (Hộp thoại OS)`}
          </button>
        </div>
        {#if customMasterPath}
          <div style="font-size: 11px; color: var(--color-accent-text); word-break: break-all;">
            📍 {customMasterPath}
          </div>
        {/if}

        <label><input type="checkbox" bind:checked={exportSubtitles} /> Xuất kèm phụ đề đồng bộ SRT & VTT</label>

        {#if exportSubtitles}
          <div class="field-row">
            <span>File Phụ Đề (SRT):</span>
            <button class="btn btn-secondary btn-sm" onclick={handlePickSrtPath}>
              📂 {customSrtPath ? 'Đổi đường dẫn SRT...' : 'Chọn nơi lưu SRT (Hộp thoại OS)'}
            </button>
          </div>
          {#if customSrtPath}
            <div style="font-size: 11px; color: var(--color-accent-text); word-break: break-all;">
              📍 {customSrtPath}
            </div>
          {/if}
        {/if}

        <label><input type="checkbox" bind:checked={normalizeLoudness} /> Chuẩn hóa âm lượng tự động (EBU R128 Loudness)</label>

        <div class="field-row">
          <label for="pause-input">Khoảng nghỉ giữa các chương (ms):</label>
          <input id="pause-input" type="number" min="0" max="5000" step="100" bind:value={interChapterPauseMs} />
        </div>
      </fieldset>

      <!-- Metadata Group -->
      <fieldset class="export-group">
        <legend>3. Thông tin sách / Metadata</legend>
        <div class="field-row">
          <label for="meta-title">Tiêu đề sách:</label>
          <input id="meta-title" type="text" bind:value={bookTitle} />
        </div>
        <div class="field-row">
          <label for="meta-author">Tác giả:</label>
          <input id="meta-author" type="text" bind:value={bookAuthor} />
        </div>
        <div class="field-row">
          <label for="meta-reader">Người đọc / Voice:</label>
          <input id="meta-reader" type="text" bind:value={bookReader} />
        </div>
      </fieldset>
    </div>

    <!-- Right Check Column (Section 5.8 Integrity Check) -->
    <div class="check-column">
      <h3>🔍 Kiểm tra trước khi xuất (Pre-Export Integrity Check)</h3>

      <div class="check-list">
        <div class="check-item {ungeneratedCount > 0 ? 'warning' : 'success'}">
          <span class="icon">{ungeneratedCount > 0 ? '⚠️' : '✓'}</span>
          <div class="check-text">
            <strong>Đoạn chưa hoàn thành:</strong>
            <p>{ungeneratedCount > 0 ? `Còn ${ungeneratedCount} đoạn chưa có âm thanh.` : 'Tất cả các đoạn đã sẵn sàng xuất file.'}</p>
          </div>
        </div>

        <div class="check-item success">
          <span class="icon">✓</span>
          <div class="check-text">
            <strong>Cấu hình Model & Voice:</strong>
            <p>Sử dụng {projectState.currentProject?.model || 'gemini-3.1-flash-tts-preview'} ({projectState.currentProject?.voice || 'Kore'})</p>
          </div>
        </div>

        <div class="check-item info">
          <span class="icon">ℹ️</span>
          <div class="check-text">
            <strong>Thư mục / File xuất:</strong>
            <p><code>{customMasterPath || projectState.currentProject?.output_directory || 'C:\\TTS_Exports'}</code></p>
          </div>
        </div>
      </div>

      <div class="export-actions">
        <button class="btn btn-primary btn-lg" onclick={handleStartExport}>
          🚀 Bắt đầu xuất âm thanh
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .export-screen-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-app);
    padding: var(--space-6);
    gap: var(--space-4);
    overflow-y: auto;
  }

  .export-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--color-border);
    padding-bottom: var(--space-3);
  }

  .export-body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-6);
  }

  .settings-column, .check-column {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .export-group {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    background: var(--color-bg-surface);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    font-size: var(--font-size-sm);
  }

  .export-group legend {
    font-weight: 600;
    padding: 0 var(--space-2);
    color: var(--color-accent);
  }

  .field-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
  }

  .field-row input[type="text"], .field-row select, .field-row input[type="number"] {
    flex: 1;
    height: var(--target-btn-sm);
  }

  .check-column {
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }

  .check-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-top: var(--space-3);
  }

  .check-item {
    display: flex;
    gap: var(--space-3);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--font-size-xs);
  }

  .check-item.success { background: var(--color-success-bg); color: var(--color-success-text); }
  .check-item.warning { background: var(--color-warning-bg); color: var(--color-warning-text); }
  .check-item.info { background: var(--color-info-bg); color: var(--color-info-text); }

  .export-actions {
    margin-top: auto;
    padding-top: var(--space-4);
  }

  .btn {
    height: var(--target-btn-md);
    padding: 0 var(--space-4);
    border-radius: var(--radius-md);
    font-weight: 500;
  }
  .btn-lg { height: var(--target-btn-lg); width: 100%; font-size: var(--font-size-base); }
  .btn-primary { background: var(--color-accent); color: white; }
  .btn-secondary { background: var(--color-bg-surface-raised); color: var(--color-text-primary); border: 1px solid var(--color-border); }
</style>
