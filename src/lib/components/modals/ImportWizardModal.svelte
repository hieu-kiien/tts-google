<script lang="ts">
  import { uiState } from "../../state/uiState.svelte";
  import { projectState } from "../../state/projectState.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { toastStore } from "../../state/toasts.svelte";
  import { getErrorMessage } from "../../utils/errorUtils";
  import { createProject, getProjectSegments } from "../../api/projectClient";

  let inputMode = $state<"file" | "text">("file");
  let currentStep = $state(1);
  let fileName = $state("");
  let fileSize = $state("0 KB");
  let estimatedChars = $state(0);
  let chunkCount = $state(0);
  let isReadingFile = $state(false);

  let cleanHeaders = $state(true);
  let cleanFooters = $state(true);

  let importedContent = $state("");
  let fileInputEl = $state<HTMLInputElement | null>(null);

  function closeModal() {
    uiState.showImportWizard = false;
    currentStep = 1;
  }

  // Calculate stats & chunk preview using backend Rust chunker
  async function updateTextStats(text: String) {
    estimatedChars = text.length;
    if (!text.trim()) {
      chunkCount = 0;
      return;
    }
    try {
      const chunks = await invoke<any[]>("chunk_text_preview", { text: text, mode: "auto" });
      chunkCount = chunks.length;
    } catch {
      chunkCount = Math.ceil(text.length / 500);
    }
  }

  // Handle native Tauri file picker
  async function handlePickFile() {
    try {
      isReadingFile = true;
      const result = await invoke<{ file_path: string; content: string } | null>("read_text_file_dialog");
      if (result) {
        fileName = result.file_path.split(/[/\\]/).pop() || result.file_path;
        fileSize = `${(result.content.length / 1024).toFixed(1)} KB`;
        importedContent = result.content;
        await updateTextStats(result.content);
        toastStore.showSuccess(`Đã đọc thành công tệp ${fileName}`);
        currentStep = 2;
      }
    } catch (err: unknown) {
      toastStore.showError(`Lỗi đọc tệp: ${getErrorMessage(err)}`);
    } finally {
      isReadingFile = false;
    }
  }

  // Fallback for HTML input file change
  async function handleFileInputChange(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      const file = input.files[0];
      fileName = file.name;
      fileSize = `${(file.size / 1024).toFixed(1)} KB`;
      isReadingFile = true;
      try {
        const text = await file.text();
        importedContent = text;
        await updateTextStats(text);
        toastStore.showSuccess(`Đã nạp file ${fileName}`);
        currentStep = 2;
      } catch (err: unknown) {
        toastStore.showError(`Lỗi đọc file: ${getErrorMessage(err)}`);
      } finally {
        isReadingFile = false;
      }
    }
  }

  // Auto update stats when pasting text directly
  function handlePastedTextChange() {
    updateTextStats(importedContent);
  }

  async function handleImportConfirm() {
    if (!importedContent.trim()) {
      toastStore.showError("Vui lòng chọn tệp tin hoặc dán văn bản trước khi nhập.");
      return;
    }

    try {
      if (projectState.currentProject) {
        // Re-chunk current project
        toastStore.showInfo("Đang cập nhật và tự động tách đoạn cho dự án hiện tại...");
        const updatedSegs = await invoke<any[]>("rechunk_project_segments", {
          projectId: projectState.currentProject.id,
          sourceText: importedContent,
          mode: "auto"
        });
        projectState.currentProject.source_text = importedContent;
        projectState.segments = updatedSegs;
        toastStore.showSuccess(`Đã nhập và tự động tách thành ${updatedSegs.length} đoạn audio!`);
      } else {
        // Create new project
        toastStore.showInfo("Đang tạo dự án mới và tự động tách đoạn...");
        const projName = fileName ? `Nhập từ ${fileName}` : `Dự án TTS ${new Date().toLocaleDateString('vi-VN')}`;
        const newProj = await createProject({
          name: projName,
          sourceText: importedContent,
          voice: "Kore",
          preset: "Tự nhiên",
          chunkMode: "auto"
        });
        projectState.projects = [newProj, ...projectState.projects];
        projectState.currentProject = newProj;
        const segs = await getProjectSegments(newProj.id);
        projectState.segments = segs;
        toastStore.showSuccess(`Đã tạo dự án mới với ${segs.length} đoạn audio chuẩn!`);
      }
      closeModal();
      uiState.activeView = "editor";
    } catch (err: unknown) {
      toastStore.showError(`Lỗi nhập văn bản: ${getErrorMessage(err)}`);
    }
  }
</script>

{#if uiState.showImportWizard}
  <div 
    class="modal-backdrop" tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) closeModal(); }}
    onkeydown={(e) => { if (e.key === 'Escape') closeModal(); }}
    role="presentation"
  >
    <div 
      class="modal-dialog" 
      role="dialog" 
      aria-modal="true" 
      aria-labelledby="wizard-title"
    >
      <header class="modal-header">
        <h2 id="wizard-title">📄 Nhập Tài Liệu & Tự Động Phân Đoạn</h2>
        <button class="close-btn" onclick={closeModal} aria-label="Đóng wizard">✕</button>
      </header>

      <!-- Source Mode Switcher Tabs -->
      <div class="mode-tabs">
        <button 
          class="mode-btn {inputMode === 'file' ? 'active' : ''}" 
          onclick={() => inputMode = 'file'}
        >
          📂 Chọn tệp tin (TXT, MD)
        </button>
        <button 
          class="mode-btn {inputMode === 'text' ? 'active' : ''}" 
          onclick={() => { inputMode = 'text'; currentStep = 2; }}
        >
          📝 Dán văn bản trực tiếp
        </button>
      </div>

      <div class="modal-body">
        {#if inputMode === 'file' && currentStep === 1}
          <div class="step-content">
            <h3>Chọn tập tin văn bản từ máy tính</h3>
            <p class="subtitle">Hỗ trợ các tệp tin văn bản thuần: <code>.txt</code>, <code>.md</code> (Mã hóa UTF-8).</p>
            
            <div class="file-dropzone" onclick={handlePickFile} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && handlePickFile()}>
              <span class="drop-icon">{isReadingFile ? '⏳' : '📁'}</span>
              <p class="drop-text">{isReadingFile ? 'Đang đọc tập tin...' : 'Kéo thả hoặc nhấn vào đây để chọn tệp tin'}</p>
              <div class="btn-group">
                <button class="btn btn-primary" onclick={(e) => { e.stopPropagation(); handlePickFile(); }}>
                  🖥️ Mở Trình Chọn File Hệ Thống
                </button>
                <button class="btn btn-secondary" onclick={(e) => { e.stopPropagation(); fileInputEl?.click(); }}>
                  📄 Chọn qua Web Input
                </button>
              </div>
              <input 
                type="file" 
                bind:this={fileInputEl} 
                accept=".txt,.md,.text" 
                style="display: none;" 
                onchange={handleFileInputChange} 
              />
            </div>
          </div>
        {:else}
          <div class="step-content">
            {#if fileName}
              <div class="file-summary-badge">
                📄 Đã nạp file: <strong>{fileName}</strong> ({fileSize})
              </div>
            {/if}

            <h3>Nội dung văn bản & Phân tích tự động</h3>
            <textarea 
              class="imported-textarea"
              placeholder="Dán hoặc nhập nội dung văn bản tiếng Việt dài của bạn tại đây (sách, báo, truyện)..."
              bind:value={importedContent}
              oninput={handlePastedTextChange}
            ></textarea>

            <div class="stats-bar">
              <div class="stat-item">
                <span class="stat-label">Tổng số ký tự:</span>
                <strong class="stat-value">{estimatedChars.toLocaleString('vi-VN')} ký tự</strong>
              </div>
              <div class="stat-item">
                <span class="stat-label">Tự động tách thành:</span>
                <strong class="stat-value highlight">~{chunkCount} đoạn audio (30-60s)</strong>
              </div>
            </div>

            <div class="cleaning-options">
              <label>
                <input type="checkbox" bind:checked={cleanHeaders} /> 
                Loại bỏ bớt khoảng trắng thừa và dòng trống liên tiếp
              </label>
            </div>
          </div>
        {/if}
      </div>

      <footer class="modal-footer">
        <button class="btn btn-secondary" onclick={closeModal}>Hủy bỏ</button>
        {#if importedContent.trim()}
          <button class="btn btn-primary btn-lg" onclick={handleImportConfirm}>
            ✅ Tự Động Phân Đoạn & Nhập Vào Dự Án ({chunkCount} đoạn)
          </button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-dialog {
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    width: 680px;
    max-width: 92vw;
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-surface-raised);
  }

  .modal-header h2 {
    font-size: var(--font-size-md);
    font-weight: 600;
  }

  .close-btn {
    font-size: var(--font-size-lg);
    color: var(--color-text-muted);
    background: none;
    border: none;
    cursor: pointer;
  }

  .mode-tabs {
    display: flex;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-surface-raised);
  }

  .mode-btn {
    flex: 1;
    padding: 10px 16px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    font-size: 13px;
    font-weight: 500;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: all 0.2s;
  }

  .mode-btn.active {
    color: var(--color-accent);
    border-bottom-color: var(--color-accent);
    background: var(--color-bg-surface);
  }

  .modal-body {
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-height: 280px;
  }

  .step-content h3 {
    font-size: 15px;
    margin-bottom: 4px;
  }

  .subtitle {
    font-size: 12px;
    color: var(--color-text-muted);
    margin-bottom: 12px;
  }

  .file-dropzone {
    border: 2px dashed var(--color-accent);
    border-radius: var(--radius-md);
    padding: 28px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    background: var(--color-bg-surface-raised);
    cursor: pointer;
    transition: background 0.2s;
  }

  .file-dropzone:hover {
    background: var(--color-bg-surface-hover);
  }

  .drop-icon { font-size: 42px; }
  .drop-text { font-size: 14px; font-weight: 500; }

  .btn-group {
    display: flex;
    gap: 10px;
    margin-top: 8px;
  }

  .file-summary-badge {
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 13px;
    margin-bottom: 12px;
  }

  .imported-textarea {
    width: 100%;
    height: 180px;
    padding: 10px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-bg-app);
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: 13px;
    resize: vertical;
    line-height: 1.5;
  }

  .stats-bar {
    display: flex;
    justify-content: space-between;
    background: var(--color-bg-surface-raised);
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 13px;
    margin-top: 10px;
  }

  .stat-value.highlight {
    color: var(--color-accent);
    font-weight: 600;
  }

  .cleaning-options {
    margin-top: 10px;
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .modal-footer {
    padding: var(--space-4) var(--space-5);
    border-top: 1px solid var(--color-border);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    background: var(--color-bg-surface-raised);
  }

  .btn {
    height: 36px;
    padding: 0 16px;
    border-radius: 6px;
    font-weight: 500;
    font-size: 13px;
    border: none;
    cursor: pointer;
  }
  .btn-lg { height: 40px; font-weight: 600; }
  .btn-primary { background: var(--color-accent); color: white; }
  .btn-secondary { background: var(--color-bg-surface-hover); color: var(--color-text-primary); border: 1px solid var(--color-border); }
</style>
