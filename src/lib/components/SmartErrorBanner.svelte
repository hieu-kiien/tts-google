<script lang="ts">
  import type { SegmentRecord } from "../types/tts";
  import { uiState } from "../state/uiState.svelte";
  import { projectState } from "../state/projectState.svelte";
  import { toastStore } from "../state/toasts.svelte";
  import { resumeProject } from "../api/queueClient";
  import { getErrorMessage } from "../utils/errorUtils";

  interface Props {
    activeErrorSegment: SegmentRecord;
    onResynthesize: (seg: SegmentRecord) => void;
  }

  let { activeErrorSegment, onResynthesize }: Props = $props();

  async function handleResumeQueue() {
    if (projectState.currentProject?.id) {
      try {
        await resumeProject(projectState.currentProject.id);
        toastStore.showSuccess("Đã tiếp tục chạy hàng đợi.");
      } catch (err) {
        toastStore.showError(getErrorMessage(err));
      }
    }
  }
</script>

<div class="smart-error-banner" role="alert">
  <div class="error-info">
    <span class="error-icon">⚠️</span>
    <div class="error-text">
      <strong>Sự cố API ({activeErrorSegment.error_code || '429'}):</strong> 
      Đoạn #{activeErrorSegment.position} {activeErrorSegment.error_message || activeErrorSegment.last_error_message || 'gặp sự cố khi kết nối Gemini API (Quá giới hạn quota hoặc API key không hợp lệ).'}
    </div>
  </div>
  <div class="error-actions">
    <button class="btn-banner secondary" onclick={() => uiState.showApiKeyModal = true}>
      🔑 Đổi API Key & Tiếp Tục
    </button>
    <button class="btn-banner primary" onclick={() => onResynthesize(activeErrorSegment)}>
      🔄 Thử Lại Đoạn #{activeErrorSegment.position}
    </button>
    <button 
      class="btn-banner" 
      style="background: var(--color-success-bg); color: var(--color-success-text); border: 1px solid var(--color-success-border);" 
      onclick={handleResumeQueue}
    >
      ▶ Tiếp Tục Hàng Đợi
    </button>
  </div>
</div>

<style>
  .smart-error-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--color-error-border, rgba(239, 68, 68, 0.3));
    border-radius: var(--radius-md, 6px);
    padding: 10px 16px;
    margin: 8px 16px;
  }

  .error-info {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: var(--font-size-sm, 13px);
    color: var(--color-error-text, #ef4444);
  }

  .error-icon {
    font-size: 18px;
  }

  .error-text {
    line-height: 1.4;
  }

  .error-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-banner {
    padding: 6px 12px;
    border-radius: var(--radius-sm, 4px);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid var(--color-border, #ccc);
    background: var(--color-bg-surface-raised, #fff);
    color: var(--color-text-primary, #333);
    transition: background 0.15s ease;
  }

  .btn-banner.primary {
    background: var(--color-accent, #2563eb);
    color: #fff;
    border-color: var(--color-accent, #2563eb);
  }

  .btn-banner.secondary {
    background: rgba(245, 158, 11, 0.15);
    color: #d97706;
    border-color: rgba(245, 158, 11, 0.4);
  }
</style>
