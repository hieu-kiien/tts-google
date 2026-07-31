<script lang="ts">
  import type { SegmentRecord } from "../types/tts";

  let {
    segments = [],
    activePlayingSegmentId = null,
    voices = [],
    onPlaySegment,
    onSaveSegment,
  }: {
    segments?: SegmentRecord[];
    activePlayingSegmentId?: string | null;
    voices?: { id: string; label: string }[];
    onPlaySegment?: (seg: SegmentRecord) => void;
    onSaveSegment?: (seg: SegmentRecord) => void;
  } = $props();
</script>

<div class="segments-section">
  <div class="segment-section-header">
    <h3>Danh Sách Segment Audio Studio ({segments.length})</h3>
    <span class="multi-voice-hint">💡 Đổi giọng riêng cho từng dòng để tạo podcast hội thoại đa nhân vật</span>
  </div>
  <div class="table-wrapper">
    <table class="seg-table">
      <thead>
        <tr>
          <th>#</th>
          <th>Nội Dung Segment</th>
          <th>Giọng Đọc Đoạn</th>
          <th>Trạng Thái</th>
          <th>Thời Lượng</th>
          <th>Thao Tác Studio</th>
        </tr>
      </thead>
      <tbody>
        {#each segments as seg (seg.id)}
          <tr class:playing={activePlayingSegmentId === seg.id}>
            <td>{seg.position}</td>
            <td class="cell-text">
              <input type="text" class="inline-text-edit" bind:value={seg.text} aria-label={`Nội dung đoạn ${seg.position}`} />
            </td>
            <td>
              <select bind:value={seg.voice} class="select-input micro-select" aria-label={`Giọng đọc đoạn ${seg.position}`}>
                {#each voices as v}
                  <option value={v.id}>{v.id}</option>
                {/each}
              </select>
            </td>
            <td>
              <span class="status-badge {seg.status}">{seg.status}</span>
            </td>
            <td>{(seg.duration_ms / 1000).toFixed(1)}s</td>
            <td>
              <div class="seg-actions">
                {#if seg.audio_path || seg.status === 'success' || seg.status === 'approved'}
                  <button class="btn icon-btn small" onclick={() => onPlaySegment?.(seg)} aria-label={`Nghe đoạn ${seg.position}`}>▶️ Nghe</button>
                  <button class="btn outline small" onclick={() => onSaveSegment?.(seg)} aria-label={`Lưu đoạn ${seg.position}`}>💾 Lưu File</button>
                {:else if seg.status === "processing" || seg.status === "queued"}
                  <span class="processing-spinner">⏳ Đang xử lý...</span>
                {:else if seg.status === "retry_wait"}
                  <span class="retry-badge">🔄 Đang chờ thử lại</span>
                {:else if seg.status === "stale"}
                  <span class="stale-badge">⚠️ Cần tạo lại</span>
                {:else}
                  <span class="text-muted">Đang chờ</span>
                {/if}
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .segments-section {
    margin-top: 20px;
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 16px;
  }
  .segment-section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }
  .multi-voice-hint {
    font-size: 12px;
    color: var(--color-text-muted);
  }
  .table-wrapper {
    overflow-x: auto;
  }
  .seg-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .seg-table th, .seg-table td {
    padding: 10px;
    border-bottom: 1px solid var(--color-border);
    text-align: left;
  }
  .inline-text-edit {
    width: 100%;
    border: 1px solid transparent;
    padding: 4px;
    border-radius: 4px;
  }
  .inline-text-edit:focus {
    border-color: var(--color-accent);
    background: var(--color-bg-surface);
  }
  .status-badge {
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
  }
  .status-badge.pending { background: var(--color-bg-surface-raised); color: var(--color-text-secondary); }
  .status-badge.queued { background: var(--color-bg-surface-selected); color: var(--color-info); }
  .status-badge.processing { background: var(--color-warning-bg); color: var(--color-warning-text); }
  .status-badge.success { background: var(--color-success-bg); color: var(--color-success-text); }
  .status-badge.approved { background: var(--color-bg-surface-selected); color: var(--color-accent); }
  .status-badge.failed { background: var(--color-error-bg); color: var(--color-error-text); }
  .status-badge.stale { background: var(--color-warning-bg); color: var(--color-warning-text); }
  .tr.playing {
    background: var(--color-bg-surface-selected);
  }
</style>
