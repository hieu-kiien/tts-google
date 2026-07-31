<script lang="ts">
  import { uiState } from "../state/uiState.svelte";
  import { projectState } from "../state/projectState.svelte";

  let {
    keyConfigured = false,
    onOpenApiKeyModal,
    onOpenDictionary,
    onStartQueue,
    onPauseQueue,
    onOpenQuotaDashboard,
    onOpenSettings,
  }: {
    keyConfigured?: boolean;
    onOpenApiKeyModal?: () => void;
    onOpenDictionary?: () => void;
    onStartQueue?: () => void;
    onPauseQueue?: () => void;
    onOpenQuotaDashboard?: () => void;
    onOpenSettings?: () => void;
  } = $props();

  let completedCount = $derived(
    projectState.segments.filter(s => s.status === 'success' || s.status === 'approved').length
  );
  let totalSegments = $derived(projectState.segments.length);
</script>

<header class="app-top-header">
  <!-- Zone A1: Brand & Project Name / Save Status -->
  <div class="header-left">
    <button 
      class="btn-nav-library" 
      onclick={() => uiState.activeView = 'library'}
      title="Thư viện dự án"
      aria-label="Thư viện dự án"
    >
      <span class="app-logo-icon">🎙️</span>
    </button>
    <button
      class="btn-icon pane-toggle"
      onclick={() => uiState.showSidebar = !uiState.showSidebar}
      title={uiState.showSidebar ? "Ẩn sidebar trái" : "Hiện sidebar trái"}
      aria-label={uiState.showSidebar ? "Ẩn sidebar trái" : "Hiện sidebar trái"}
      aria-expanded={uiState.showSidebar}
    >
      {uiState.showSidebar ? "◀" : "▶"}
    </button>

    <div class="project-info">
      <div class="title-row">
        <h1 class="project-title">{projectState.currentProject?.name || 'Auto TTS Studio'}</h1>
        {#if totalSegments > 0}
          <span class="progress-badge">
            {completedCount}/{totalSegments} đoạn
          </span>
        {/if}
      </div>
      <span class="save-status">
        {#if projectState.hasPendingSaves}
          <span class="pulse-dot pending"></span> ⏳ Đang lưu...
        {:else}
          <span class="pulse-dot"></span> Đã tự động lưu
        {/if}
      </span>
    </div>
  </div>

  <!-- Zone A2: Main Action Toolbar -->
  <div class="toolbar-actions" role="toolbar" aria-label="Thanh công cụ chính">
    <button class="btn btn-secondary" onclick={() => uiState.showImportWizard = true} title="Chọn hoặc dán tệp tin văn bản (Ctrl+I)">
      <span>📂</span> <span class="label">Nhập file</span>
    </button>

    <button class="btn btn-secondary" onclick={() => uiState.showBatchProcessorModal = true} title="Chuyển đổi nhiều file văn bản (.txt, .md) thành audio hàng loạt">
      <span>🚀</span> <span class="label">Hàng loạt</span>
    </button>

    <div class="divider"></div>

    <!-- Main Highlight Action -->
    <button class="btn btn-primary-gradient" onclick={onStartQueue} title="Bắt đầu tạo giọng đọc cho tất cả các đoạn">
      <span>⚡</span> <span class="label">Tạo Tất Cả</span>
    </button>

    <button class="btn btn-secondary" onclick={onPauseQueue} title="Tạm dừng hàng đợi">
      <span>⏸️</span> <span class="label">Tạm dừng</span>
    </button>

    <button class="btn btn-secondary" onclick={() => uiState.activeView = 'export'} title="Xuất file âm thanh">
      <span>📦</span> <span class="label">Xuất file</span>
    </button>
  </div>

  <!-- Zone A3: Utilities, Key, Dictionary, Theme -->
  <div class="header-right">
    <button class="btn-icon" onclick={onOpenQuotaDashboard} title="Bảng thống kê Gemini Quota" aria-label="Thống kê Quota API">
      📊
    </button>

    <button class="btn-icon" onclick={onOpenSettings} title="Cấu hình số luồng Concurrency" aria-label="Cài đặt hệ thống">
      ⚙️
    </button>

    <button class="btn-icon" onclick={() => uiState.showShortcutGuide = true} title="Phím tắt bàn phím" aria-label="Phím tắt bàn phím">
      ⌨️
    </button>

    <button class="btn-icon dict-btn" onclick={onOpenDictionary} title="Từ điển phát âm" aria-label="Từ điển phát âm">
      📖
      {#if projectState.dictionaryRules.length > 0}
        <span class="dict-count">{projectState.dictionaryRules.length}</span>
      {/if}
    </button>

    <button class="key-status-btn {keyConfigured ? 'configured' : ''}" onclick={onOpenApiKeyModal} aria-label={keyConfigured ? "API Key: Đã kết nối" : "Nhập Gemini API Key"}>
      <span class="dot"></span>
      <span class="key-text">{keyConfigured ? "API Key: Đã kết nối" : "Nhập API Key"}</span>
    </button>

    <button class="btn-icon" onclick={() => uiState.toggleTheme()} title="Chuyển chế độ Sáng / Tối" aria-label="Chuyển chế độ giao diện">
      {uiState.theme === "light" ? "🌙" : "☀️"}
    </button>

    <button
      class="btn-icon pane-toggle"
      onclick={() => uiState.showInspector = !uiState.showInspector}
      title={uiState.showInspector ? "Ẩn bảng cài đặt phải" : "Hiện bảng cài đặt phải"}
      aria-label={uiState.showInspector ? "Ẩn bảng cài đặt phải" : "Hiện bảng cài đặt phải"}
      aria-expanded={uiState.showInspector}
    >
      {uiState.showInspector ? "▶" : "◀"}
    </button>
  </div>
</header>

<style>
  .app-top-header {
    height: 56px;
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border-bottom: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-4);
    gap: var(--space-4);
    z-index: 100;
    box-shadow: var(--shadow-sm);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .btn-nav-library {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--gradient-primary);
    box-shadow: var(--shadow-sm);
    transition: transform 0.2s ease;
  }
  .btn-nav-library:hover {
    transform: scale(1.05);
  }

  .app-logo-icon {
    font-size: 1.2rem;
  }

  .project-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .project-title {
    font-size: var(--font-size-base);
    font-weight: 700;
    color: var(--color-text-primary);
    max-width: 220px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .progress-badge {
    font-size: var(--font-size-xs);
    font-weight: 600;
    background: var(--color-bg-surface-raised);
    color: var(--color-accent-text);
    padding: 2px 8px;
    border-radius: var(--radius-full);
    border: 1px solid var(--color-border-subtle);
  }

  .save-status {
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 6px var(--color-success);
  }

  .pulse-dot.pending {
    background: var(--color-warning-bg);
    box-shadow: 0 0 6px var(--color-warning-bg);
    animation: pulseAvatar 1s infinite alternate;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .divider {
    width: 1px;
    height: 24px;
    background: var(--color-border);
    margin: 0 var(--space-1);
  }

  .btn {
    height: var(--target-btn-md);
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: var(--font-size-xs);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    transition: all 0.2s ease;
  }

  .btn-primary-gradient {
    background: var(--gradient-primary);
    color: white;
    box-shadow: 0 2px 8px rgba(37, 99, 235, 0.35);
  }
  .btn-primary-gradient:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(37, 99, 235, 0.45);
  }

  .btn-secondary {
    background: var(--color-bg-surface);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
  }
  .btn-secondary:hover {
    background: var(--color-bg-surface-hover);
    border-color: var(--color-border-focus);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .btn-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border-subtle);
    transition: background 0.15s ease;
  }
  .btn-icon:hover {
    background: var(--color-bg-surface-hover);
  }

  .dict-count {
    position: absolute;
    top: -4px;
    right: -4px;
    background: var(--color-accent);
    color: white;
    font-size: 10px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: var(--radius-full);
    border: 1px solid var(--color-bg-surface);
  }

  .key-status-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    height: var(--target-btn-sm);
    border-radius: var(--radius-full);
    border: 1px solid var(--color-error-border);
    background: var(--color-error-bg);
    color: var(--color-error-text);
    font-size: var(--font-size-xs);
    font-weight: 600;
    transition: all 0.2s ease;
  }

  .key-status-btn.configured {
    border-color: var(--color-success-border);
    background: var(--color-success-bg);
    color: var(--color-success-text);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }

  @media (max-width: 1100px) {
    .label { display: none; }
  }
</style>
