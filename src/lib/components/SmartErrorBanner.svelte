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
