<script lang="ts">
  import type { QueueSnapshot, SegmentRecord } from "../types/tts";
  import { projectState } from "../state/projectState.svelte";
  import { playerState } from "../state/playerState.svelte";
  import { readAudioDataUrl } from "../api/audioClient";
  import { toastStore } from "../state/toasts.svelte";
  import { getErrorMessage } from "../utils/errorUtils";

  let {
    snapshot = null,
    onEnqueue,
    onPause,
    onResume,
    onCancel,
  }: {
    snapshot?: QueueSnapshot | null;
    onEnqueue?: () => void;
    onPause?: () => void;
    onResume?: () => void;
    onCancel?: () => void;
  } = $props();

  let showDetails = $state(false);

  async function handlePlaySegment(seg: SegmentRecord) {
    if (seg.audio_path) {
      try {
        const dataUrl = await readAudioDataUrl(undefined, seg.audio_path);
        playerState.playUrl(dataUrl, seg.id);
      } catch (err: unknown) {
        toastStore.showError("Không thể đọc tệp audio: " + getErrorMessage(err));
      }
    }
  }

  let percent = $derived.by<number>(() => {
    return snapshot && snapshot.total_segments > 0
      ? Math.round((snapshot.completed_segments / snapshot.total_segments) * 100)
      : 0;
  });

  let statusText = $derived.by<string>(() => {
    if (!snapshot) return "Chưa xếp hàng";
    switch (snapshot.queue_state) {
      case "Running": return "⚡ Đang xử lý tạo âm thanh hàng đợi...";
      case "Paused": return "⏸️ Hàng đợi đang tạm dừng";
      case "Cancelled": return "⏹️ Đã hủy tác vụ hàng đợi";
      default: return "⏳ Sẵn sàng";
    }
  });
</script>

<div class="queue-panel-container" role="region" aria-label="Bảng tiến độ hàng đợi">
  <div class="queue-summary-bar">
    <div class="summary-info">
      <span class="status-badge status-{snapshot?.queue_state?.toLowerCase() || 'idle'}">
        {statusText}
      </span>
      <span class="count-label">
        {snapshot?.completed_segments || 0} / {snapshot?.total_segments || 0} đoạn ({percent}%)
      </span>
      {#if (snapshot?.failed_segments || 0) > 0}
        <span class="error-badge">⚠️ {snapshot?.failed_segments} đoạn lỗi</span>
      {/if}
    </div>

    <div class="summary-actions">
      {#if snapshot?.queue_state === "Idle" || snapshot?.queue_state === "Cancelled"}
        <button class="btn btn-primary" onclick={onEnqueue} title="Bắt đầu tạo audio hàng đợi">
          ▶ Bắt đầu
        </button>
      {:else if snapshot?.queue_state === "Running"}
        <button class="btn btn-warning" onclick={onPause} title="Tạm dừng hàng đợi">
          ⏸ Tạm dừng
        </button>
        <button class="btn btn-danger" onclick={onCancel} title="Hủy tác vụ hàng đợi">
          ⏹ Hủy
        </button>
      {:else if snapshot?.queue_state === "Paused"}
        <button class="btn btn-primary" onclick={onResume} title="Tiếp tục hàng đợi">
          ▶ Tiếp tục
        </button>
      {/if}

      <button class="btn btn-secondary" onclick={() => showDetails = !showDetails} aria-label="Xem chi tiết hàng đợi">
        {showDetails ? '▲ Thu gọn' : '▼ Chi tiết'}
      </button>
    </div>
  </div>

  <!-- Determinate Progress Bar (Section 5.6) -->
  <div class="progress-bar-track" role="progressbar" aria-valuenow={snapshot?.completed_segments || 0} aria-valuemax={snapshot?.total_segments || 1}>
    <div class="progress-bar-fill" style={`width: ${percent}%`}></div>
  </div>

  <!-- Expanded Details Panel -->
  {#if showDetails}
    <div class="queue-details-content">
      <table class="queue-table">
        <thead>
          <tr>
            <th>Vị trí</th>
            <th>Xem trước nội dung</th>
            <th>Voice</th>
            <th>Trạng thái</th>
            <th>Hành động</th>
          </tr>
        </thead>
        <tbody>
          {#if projectState.segments.length > 0}
            {#each projectState.segments as seg}
              <tr>
                <td>Đoạn #{seg.position}</td>
                <td title={seg.text}>{seg.text.slice(0, 45)}...</td>
                <td>{seg.voice || projectState.currentProject?.voice || 'Kore'}</td>
                <td>
                  {#if seg.status === 'approved'}
                    <span class="badge-status success">⭐ Đã duyệt</span>
                  {:else if seg.status === 'success'}
                    <span class="badge-status success">✓ Hoàn tất</span>
                  {:else if seg.status === 'processing' || seg.status === 'queued'}
                    <span class="badge-status processing">⚡ Đang xử lý</span>
                  {:else if seg.status === 'failed'}
                    <span class="badge-status error">✕ Lỗi API</span>
                  {:else if seg.status === 'retry_wait'}
                    <span class="badge-status warning">🔄 Đang chờ thử lại</span>
                  {:else}
                    <span class="badge-status pending">⏳ Chưa tạo</span>
                  {/if}
                </td>
                <td>
                  <button class="btn-sm" onclick={() => handlePlaySegment(seg)}>Nghe</button>
                </td>
              </tr>
            {/each}
          {:else}
            <tr>
              <td colspan="5" style="text-align: center; color: var(--color-text-muted);">Chưa có dữ liệu đoạn trong hàng đợi</td>
            </tr>
          {/if}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .queue-panel-container {
    background: var(--color-bg-surface);
    border-top: 1px solid var(--color-border);
    padding: var(--space-2) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .queue-summary-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .summary-info {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--font-size-xs);
  }

  .status-badge {
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .error-badge {
    background: var(--color-error-bg);
    color: var(--color-error-text);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
  }

  .summary-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .progress-bar-track {
    height: 4px;
    background: var(--color-bg-surface-hover);
    border-radius: var(--radius-full);
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.3s ease;
  }

  .queue-details-content {
    max-height: 160px;
    overflow-y: auto;
    border-top: 1px solid var(--color-border-subtle);
    padding-top: var(--space-2);
  }

  .queue-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-xs);
  }

  .queue-table th, .queue-table td {
    padding: var(--space-1) var(--space-2);
    border-bottom: 1px solid var(--color-border-subtle);
    text-align: left;
  }

  .btn {
    height: var(--target-btn-sm);
    padding: 0 var(--space-3);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
  }
  .btn-primary { background: var(--color-accent); color: white; }
  .btn-warning { background: var(--color-warning-bg); color: var(--color-warning-text); }
  .btn-danger { background: var(--color-error-bg); color: var(--color-error-text); }
  .btn-secondary { background: var(--color-bg-surface-raised); color: var(--color-text-primary); border: 1px solid var(--color-border); }
  .btn-sm { height: 24px; padding: 0 8px; font-size: 11px; }
</style>
