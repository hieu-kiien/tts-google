<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { uiState } from "../../state/uiState.svelte";
  import { toastStore } from "../../state/toasts.svelte";
  import { projectState } from "../../state/projectState.svelte";
  import { playerState } from "../../state/playerState.svelte";
  import { getErrorMessage } from "../../utils/errorUtils";

  interface BatchFileItem {
    id: string;
    fileName: string;
    text: string;
    charCount: number;
    voice: string;
    status: "pending" | "processing" | "completed" | "failed";
    progressPct: number;
    outputPath?: string;
    errorMessage?: string;
  }

  let batchFiles = $state<BatchFileItem[]>([]);
  let selectedVoice = $state("Kore");
  let selectedModel = $state("gemini-3.1-flash-tts-preview");
  let isBatchRunning = $state(false);
  let currentProcessingIndex = $state(-1);
  let overallProgress = $state(0);

  const voices = [
    { id: "Kore", name: "Kore (Nữ tự nhiên, trầm ấm)" },
    { id: "Aoede", name: "Aoede (Nữ sâu lắng, truyền cảm)" },
    { id: "Zephyr", name: "Zephyr (Nữ nhẹ nhàng, bình tĩnh)" },
    { id: "Puck", name: "Puck (Nam trẻ trung, năng động)" },
    { id: "Charon", name: "Charon (Nam trang trọng, thời sự)" },
    { id: "Fenrir", name: "Fenrir (Nam mạnh mẽ, cuốn hút)" },
  ];

  async function handleSelectBatchFiles() {
    try {
      const selected = await invoke<string[] | string | null>("read_text_file_dialog", {
        multiple: true
      });

      if (!selected) return;

      const filePaths = Array.isArray(selected) ? selected : [selected];
      for (const pathStr of filePaths) {
        if (typeof pathStr !== "string" || !pathStr) continue;
        const name = pathStr.split(/[/\\]/).pop() || pathStr;
        
        // Read file content
        try {
          const content = await invoke<string>("read_text_file_content", { path: pathStr });
          batchFiles.push({
            id: `batch_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
            fileName: name,
            text: content,
            charCount: content.length,
            voice: selectedVoice,
            status: "pending",
            progressPct: 0
          });
        } catch (err: unknown) {
          console.warn("Lỗi đọc file batch:", err);
          toastStore.showError(`Không thể đọc file: ${name}`);
        }
      }
      toastStore.showSuccess(`Đã thêm ${filePaths.length} file vào danh sách xử lý hàng loạt!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi chọn file hàng loạt: ${getErrorMessage(err)}`);
    }
  }

  function handleRemoveFile(id: string) {
    if (isBatchRunning) return;
    batchFiles = batchFiles.filter(f => f.id !== id);
  }

  function handleClearAll() {
    if (isBatchRunning) return;
    batchFiles = [];
    currentProcessingIndex = -1;
    overallProgress = 0;
  }

  async function handleStartBatchProcessing() {
    if (batchFiles.length === 0) {
      toastStore.showInfo("Vui lòng thêm ít nhất 1 file văn bản để xử lý.");
      return;
    }

    isBatchRunning = true;
    toastStore.showInfo(`Bắt đầu xử lý hàng loạt ${batchFiles.length} file...`);

    let completedCount = 0;

    for (let i = 0; i < batchFiles.length; i++) {
      const item = batchFiles[i];
      if (item.status === "completed") {
        completedCount++;
        continue;
      }

      currentProcessingIndex = i;
      item.status = "processing";
      item.progressPct = 15;

      try {
        // 1. Create temporary project for this file
        const projName = `[Batch] ${item.fileName}`;
        const createdProj = await invoke<{ id: string }>("create_project", {
          name: projName,
          sourceText: item.text,
          voice: selectedVoice,
          model: selectedModel
        });

        item.progressPct = 40;

        // 2. Synthesize audio preview/chunks
        const segs = await invoke<Array<{ id: string; audio_path?: string }>>("get_project_segments", {
          projectId: createdProj.id
        });

        item.progressPct = 70;

        // 3. Merge project audio
        try {
          const mergedRes = await invoke<{ file_path: string }>("merge_project_audio", {
            projectId: createdProj.id,
            silenceMs: 300,
            normalizeVolume: true
          });
          item.outputPath = mergedRes.file_path;
        } catch {
          // Ignore merge if single segment
        }

        item.status = "completed";
        item.progressPct = 100;
        completedCount++;
        overallProgress = Math.round((completedCount / batchFiles.length) * 100);
        toastStore.showSuccess(`✓ Đã tạo xong file (${i + 1}/${batchFiles.length}): ${item.fileName}`);

      } catch (err: unknown) {
        item.status = "failed";
        item.errorMessage = getErrorMessage(err);
        toastStore.showError(`✕ Lỗi tạo file ${item.fileName}: ${getErrorMessage(err)}`);
      }
    }

    isBatchRunning = false;
    currentProcessingIndex = -1;
    toastStore.showSuccess(`🎉 Đã hoàn tất xử lý hàng loạt ${completedCount}/${batchFiles.length} file audio!`);
  }

  function handleClose() {
    if (isBatchRunning) {
      if (!confirm("Tiến trình xử lý hàng loạt đang chạy. Bạn có chắc muốn đóng cửa sổ?")) return;
    }
    uiState.showBatchProcessorModal = false;
  }
</script>

<div class="modal-backdrop" onclick={handleClose} onkeydown={(e) => e.key === 'Escape' && handleClose()} tabindex="-1" role="presentation">
  <div class="modal-card batch-modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="batch-title" tabindex="-1">
    <!-- Header -->
    <header class="modal-header">
      <div class="title-row">
        <span class="icon">🚀</span>
        <div>
          <h2 id="batch-title">Xử Lý File Hàng Loạt (Batch Audio Converter)</h2>
          <p class="subtitle">Chuyển đổi nhiều file văn bản (.txt, .md) thành file audio voice AI cùng lúc</p>
        </div>
      </div>
      <button class="btn-close" onclick={handleClose} aria-label="Đóng">✕</button>
    </header>

    <!-- Controls Toolbar -->
    <div class="batch-controls">
      <div class="setting-group">
        <label for="batch-voice">Giọng đọc hàng loạt:</label>
        <select id="batch-voice" bind:value={selectedVoice} disabled={isBatchRunning}>
          {#each voices as v}
            <option value={v.id}>{v.name}</option>
          {/each}
        </select>
      </div>

      <div class="setting-group">
        <label for="batch-model">Model AI:</label>
        <select id="batch-model" bind:value={selectedModel} disabled={isBatchRunning}>
          <option value="gemini-3.1-flash-tts-preview">Gemini 3.1 Flash (Tối ưu giọng nói)</option>
          <option value="gemini-2.5-flash-preview-tts">Gemini 2.5 Flash (Dự phòng)</option>
        </select>
      </div>

      <div class="action-buttons">
        <button class="btn btn-secondary" onclick={handleSelectBatchFiles} disabled={isBatchRunning}>
          📂 Thêm File Văn Bản
        </button>
        {#if batchFiles.length > 0}
          <button class="btn btn-secondary btn-danger-text" onclick={handleClearAll} disabled={isBatchRunning}>
            🧹 Xóa tất cả
          </button>
        {/if}
      </div>
    </div>

    <!-- Overall Progress Bar -->
    {#if isBatchRunning || overallProgress > 0}
      <div class="overall-progress-box">
        <div class="progress-info">
          <span>Tiến trình tổng thể: <strong>{overallProgress}%</strong></span>
          <span>{batchFiles.filter(f => f.status === 'completed').length} / {batchFiles.length} file hoàn thành</span>
        </div>
        <div class="progress-bar-track">
          <div class="progress-bar-fill" style="width: {overallProgress}%;"></div>
        </div>
      </div>
    {/if}

    <!-- File Queue List -->
    <div class="file-queue-container">
      {#if batchFiles.length === 0}
        <div class="empty-queue-zone">
          <div class="empty-icon">📁</div>
          <h3>Chưa có file nào trong hàng đợi xử lý</h3>
          <p>Bấm nút <strong>"📂 Thêm File Văn Bản"</strong> để chọn nhiều file .TXT, .MD cần chuyển thành audio giọng đọc AI.</p>
        </div>
      {:else}
        <table class="queue-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Tên File</th>
              <th>Số ký tự</th>
              <th>Trạng thái</th>
              <th>Tiến trình</th>
              <th>Thao tác</th>
            </tr>
          </thead>
          <tbody>
            {#each batchFiles as item, idx (item.id)}
              <tr class={item.status === 'processing' ? 'row-processing' : ''}>
                <td>{idx + 1}</td>
                <td class="file-name-cell" title={item.fileName}>
                  📄 {item.fileName}
                </td>
                <td>{item.charCount.toLocaleString()} ký tự</td>
                <td>
                  {#if item.status === 'completed'}
                    <span class="badge-status success">✓ Hoàn thành</span>
                  {:else if item.status === 'processing'}
                    <span class="badge-status processing">⚡ Đang chuyển đổi...</span>
                  {:else if item.status === 'failed'}
                    <span class="badge-status danger" title={item.errorMessage}>✕ Lỗi</span>
                  {:else}
                    <span class="badge-status pending">⏳ Đang chờ</span>
                  {/if}
                </td>
                <td class="progress-cell">
                  <div class="mini-progress-track">
                    <div class="mini-progress-fill" style="width: {item.progressPct}%;"></div>
                  </div>
                  <span class="pct-num">{item.progressPct}%</span>
                </td>
                <td>
                  {#if item.outputPath}
                    <button 
                      class="btn-action-sm" 
                      onclick={() => playerState.playUrl(item.outputPath!, null)}
                      title="Nghe file audio đã tạo"
                    >
                      ▶ Nghe
                    </button>
                  {/if}
                  {#if !isBatchRunning}
                    <button 
                      class="btn-action-sm danger" 
                      onclick={() => handleRemoveFile(item.id)}
                      title="Xóa khỏi danh sách hàng đợi"
                    >
                      🗑️
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <!-- Footer Action Bar -->
    <footer class="modal-footer">
      <div class="summary-text">
        Tổng số: <strong>{batchFiles.length}</strong> file văn bản | 
        Tổng ký tự: <strong>{batchFiles.reduce((acc, f) => acc + f.charCount, 0).toLocaleString()}</strong> ký tự
      </div>
      <div class="footer-buttons">
        <button class="btn btn-secondary" onclick={handleClose}>Đóng</button>
        <button 
          class="btn btn-primary btn-lg" 
          onclick={handleStartBatchProcessing}
          disabled={isBatchRunning || batchFiles.length === 0}
        >
          {isBatchRunning ? '⚡ Đang Xử Lý Hàng Loạt...' : '🚀 Bắt Đầu Xử Lý Hàng Loạt'}
        </button>
      </div>
    </footer>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 999;
    padding: var(--space-4);
  }

  .batch-modal {
    width: 100%;
    max-width: 900px;
    max-height: 88vh;
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-xl);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-surface-raised);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .title-row .icon { font-size: 26px; }

  .title-row h2 {
    font-size: 18px;
    font-weight: 700;
    margin: 0;
    color: var(--color-text-primary);
  }

  .subtitle {
    font-size: 13px;
    color: var(--color-text-muted);
    margin: 2px 0 0 0;
  }

  .btn-close {
    background: transparent;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: var(--color-text-muted);
    padding: 4px 8px;
  }
  .btn-close:hover { color: var(--color-text-primary); }

  .batch-controls {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 20px;
    background: var(--color-bg-surface);
    border-bottom: 1px solid var(--color-border);
    flex-wrap: wrap;
  }

  .setting-group {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .setting-group select {
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-bg-surface-raised);
    color: var(--color-text-primary);
    font-size: 13px;
  }

  .action-buttons {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }

  .btn-danger-text { color: #dc2626; }

  .overall-progress-box {
    padding: 10px 20px;
    background: rgba(37, 99, 235, 0.06);
    border-bottom: 1px solid var(--color-border);
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    margin-bottom: 6px;
  }

  .progress-bar-track {
    height: 8px;
    background: var(--color-border);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.3s ease;
  }

  .file-queue-container {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    min-height: 240px;
  }

  .empty-queue-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px var(--space-4);
    text-align: center;
    border: 2px dashed var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-surface-raised);
  }

  .empty-queue-zone .empty-icon { font-size: 40px; margin-bottom: 8px; }

  .empty-queue-zone h3 { margin: 0 0 4px 0; font-size: 16px; }

  .empty-queue-zone p { margin: 0; color: var(--color-text-muted); font-size: 13px; }

  .queue-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  .queue-table th, .queue-table td {
    padding: 10px 12px;
    text-align: left;
    border-bottom: 1px solid var(--color-border);
  }

  .queue-table th {
    background: var(--color-bg-surface-raised);
    color: var(--color-text-secondary);
    font-weight: 600;
  }

  .row-processing {
    background: rgba(37, 99, 235, 0.06);
  }

  .file-name-cell {
    max-width: 260px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
  }

  .badge-status {
    padding: 3px 8px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
  }

  .badge-status.success { background: rgba(34, 197, 94, 0.15); color: #16a34a; }
  .badge-status.processing { background: rgba(37, 99, 235, 0.15); color: #2563eb; }
  .badge-status.pending { background: rgba(100, 116, 139, 0.15); color: #64748b; }
  .badge-status.danger { background: rgba(239, 68, 68, 0.15); color: #dc2626; }

  .progress-cell {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 140px;
  }

  .mini-progress-track {
    flex: 1;
    height: 6px;
    background: var(--color-border);
    border-radius: 3px;
    overflow: hidden;
  }

  .mini-progress-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.2s ease;
  }

  .pct-num { font-size: 11px; color: var(--color-text-muted); width: 32px; }

  .btn-action-sm {
    padding: 3px 8px;
    font-size: 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-bg-surface-raised);
    cursor: pointer;
    margin-right: 4px;
  }
  .btn-action-sm:hover { background: var(--color-bg-surface-hover); }
  .btn-action-sm.danger { color: #dc2626; }

  .modal-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 20px;
    background: var(--color-bg-surface-raised);
    border-top: 1px solid var(--color-border);
  }

  .summary-text {
    font-size: 13px;
    color: var(--color-text-secondary);
  }

  .footer-buttons {
    display: flex;
    gap: 12px;
  }
</style>
