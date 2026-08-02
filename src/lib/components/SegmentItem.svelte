<script lang="ts">
  import type { SegmentRecord } from "../types/tts";
  import { playerState } from "../state/playerState.svelte";
  import { projectState } from "../state/projectState.svelte";

  interface Props {
    seg: SegmentRecord;
    isSelected: boolean;
    isActive: boolean;
    isPlaying: boolean;
    isSynthesizingPreview: boolean;
    totalSegments: number;
    registerTextarea?: (id: string, el: HTMLTextAreaElement | null) => void;
    onSelect: (id: string, e: MouseEvent) => void;
    onClick: (seg: SegmentRecord) => void;
    onTextInput: (seg: SegmentRecord) => void;
    onPlayPreview: (seg: SegmentRecord) => void;
    onResynthesize: (seg: SegmentRecord) => void;
    onSplit: (seg: SegmentRecord) => void;
    onMove: (seg: SegmentRecord, direction: 'up' | 'down') => void;
    onInsertBelow: (seg: SegmentRecord) => void;
    onMerge: (seg: SegmentRecord) => void;
    onDelete: (seg: SegmentRecord) => void;
  }

  let {
    seg,
    isSelected,
    isActive,
    isPlaying,
    isSynthesizingPreview,
    totalSegments,
    registerTextarea,
    onSelect,
    onClick,
    onTextInput,
    onPlayPreview,
    onResynthesize,
    onSplit,
    onMove,
    onInsertBelow,
    onMerge,
    onDelete
  }: Props = $props();

  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  $effect(() => {
    if (registerTextarea) {
      registerTextarea(seg.id, textareaEl);
    }
    return () => {
      if (registerTextarea) {
        registerTextarea(seg.id, null);
      }
    };
  });
</script>

<div 
  id={`seg_card_${seg.id}`}
  class="segment-row {isSelected ? 'selected-batch' : ''} {isActive ? 'selected' : ''} {isPlaying ? 'playing' : ''} {seg.status === 'processing' || seg.status === 'queued' ? 'processing-active' : ''} {seg.is_locked ? 'locked' : ''} {seg.is_skipped ? 'skipped' : ''}"
  onclick={() => onClick(seg)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick(seg); }}
  role="button"
  tabindex="0"
  aria-label={`Đoạn ${seg.position}, trạng thái ${seg.status}`}
>
  <!-- Line Number & Indicators -->
  <div class="segment-meta">
    <input 
      type="checkbox" 
      class="seg-checkbox"
      checked={isSelected} 
      onclick={(e) => onSelect(seg.id, e)}
      title="Chọn đoạn này"
    />
    <span class="seg-num">#{seg.position}</span>
    {#if seg.is_locked}
      <span class="status-icon" title="Đoạn bị khóa">🔒</span>
    {/if}
    {#if seg.is_skipped}
      <span class="status-icon" title="Đã đánh dấu bỏ qua">⏭️</span>
    {/if}
    {#if seg.text.length > 700}
      <span class="warning-badge" title="Đoạn quá dài (> 700 ký tự). Bạn có thể bấm nút ✂️ Tách đoạn.">⚠️ Quá dài</span>
    {/if}
  </div>

  <!-- Segment Text Display / Editor -->
  <div class="segment-body">
    <textarea 
      id={`textarea-${seg.id}`}
      bind:this={textareaEl}
      class="segment-textarea" 
      bind:value={seg.text}
      oninput={() => onTextInput(seg)}
      aria-label={`Nội dung đoạn ${seg.position}`}
    ></textarea>

    <div class="segment-footer-info">
      <span>{seg.text.length} ký tự</span>
      {#if seg.duration_ms > 0}
        <span class="duration-badge">⏱️ {(seg.duration_ms / 1000).toFixed(1)}s</span>
      {/if}

      {#if seg.status === 'approved'}
        <span class="status-approved" title="Đã duyệt">⭐</span>
      {:else if seg.status === 'success'}
        <span class="status-success" title="Đã tạo âm thanh">🟢</span>
      {:else if seg.status === 'processing' || seg.status === 'queued'}
        <div class="status-processing-box">
          <div class="sound-wave-anim" title="Đang tổng hợp âm thanh với Gemini API">
            <span class="bar bar-1"></span>
            <span class="bar bar-2"></span>
            <span class="bar bar-3"></span>
          </div>
        </div>
      {:else if seg.status === 'failed'}
        <span class="status-error" title="Lỗi API">🔴</span>
      {:else if seg.status === 'retry_wait'}
        <span class="status-warning" title="Đang chờ thử lại">🔄</span>
      {:else if seg.status === 'stale'}
        <span class="status-warning badge-stale" title="Văn bản đã thay đổi, cần tạo lại âm thanh">⚠️</span>
      {:else}
        <span class="status-pending" title="Chưa tạo âm thanh">⚪</span>
      {/if}
    </div>
  </div>

  <!-- Segment Quick Actions -->
  <div class="segment-actions">
    <button 
      class="btn-action" 
      onclick={(e) => { e.stopPropagation(); onPlayPreview(seg); }}
      disabled={isSynthesizingPreview}
      title="Nghe thử đoạn này"
      aria-label="Nghe thử"
    >
      ▶
    </button>
    {#if seg.status === 'stale'}
      <button 
        class="btn-action btn-resynthesize" 
        onclick={(e) => { e.stopPropagation(); onResynthesize(seg); }}
        title="Tạo lại đoạn này ngay"
        aria-label="Tạo lại đoạn này"
      >
        ⚡
      </button>
    {/if}
    <button 
      class="btn-action btn-secondary-action" 
      onclick={(e) => { e.stopPropagation(); onSplit(seg); }}
      title="Tách đoạn tại vị trí con trỏ"
      aria-label="Tách đoạn"
    >
      ✂️
    </button>
    {#if seg.position > 1}
      <button 
        class="btn-action btn-secondary-action" 
        onclick={(e) => { e.stopPropagation(); onMove(seg, 'up'); }}
        title="Di chuyển lên trên"
        aria-label="Di chuyển lên"
      >
        ⬆️
      </button>
    {/if}
    {#if seg.position < totalSegments}
      <button 
        class="btn-action btn-secondary-action" 
        onclick={(e) => { e.stopPropagation(); onMove(seg, 'down'); }}
        title="Di chuyển xuống dưới"
        aria-label="Di chuyển xuống"
      >
        ⬇️
      </button>
    {/if}
    <button 
      class="btn-action btn-secondary-action" 
      onclick={(e) => { e.stopPropagation(); onInsertBelow(seg); }}
      title="Chèn thêm đoạn mới bên dưới"
      aria-label="Chèn đoạn mới"
    >
      ➕
    </button>
    {#if seg.position > 1}
      <button 
        class="btn-action btn-merge btn-secondary-action" 
        onclick={(e) => { e.stopPropagation(); onMerge(seg); }}
        title="Gộp với đoạn #{seg.position - 1} phía trên"
        aria-label="Gộp đoạn"
      >
        🔗
      </button>
    {/if}
    <button 
      class="btn-action btn-delete-segment" 
      onclick={(e) => { e.stopPropagation(); onDelete(seg); }}
      title="Xóa đoạn này"
      aria-label="Xóa đoạn"
    >
      🗑️
    </button>
  </div>
</div>

<style>
  .segment-row {
    display: flex;
    gap: var(--space-3, 12px);
    padding: var(--space-3, 12px);
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: var(--radius-md, 6px);
    background: var(--color-bg-surface, #fff);
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .segment-row.selected {
    border-color: var(--color-accent, #2563eb);
    background: var(--color-bg-surface-selected, rgba(37, 99, 235, 0.05));
  }

  .segment-row.playing {
    border-color: var(--color-success, #10b981);
    background: var(--color-success-bg, rgba(16, 185, 129, 0.08));
  }

  @keyframes wave-bar {
    0%, 100% { transform: scaleY(0.3); }
    50% { transform: scaleY(1.0); }
  }

  @keyframes pulse-ring {
    0% { box-shadow: 0 0 0 0 rgba(245, 158, 11, 0.4); }
    70% { box-shadow: 0 0 0 8px rgba(245, 158, 11, 0); }
    100% { box-shadow: 0 0 0 0 rgba(245, 158, 11, 0); }
  }

  .segment-row.processing-active {
    border-color: #f59e0b;
    animation: pulse-ring 1.8s infinite;
    background: rgba(245, 158, 11, 0.06);
  }

  .segment-row.locked { opacity: 0.8; }
  .segment-row.skipped { text-decoration: line-through; opacity: 0.6; }
  .segment-row.selected-batch { background: rgba(37, 99, 235, 0.05); }

  .segment-meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1, 4px);
    min-width: 44px;
    font-size: var(--font-size-xs, 12px);
    color: var(--color-text-muted, #6b7280);
  }

  .seg-checkbox {
    width: 16px;
    height: 16px;
    cursor: pointer;
    margin-right: 4px;
  }

  .warning-badge {
    color: var(--color-warning-text, #d97706);
    background: var(--color-warning-bg, rgba(245, 158, 11, 0.15));
    padding: 2px 4px;
    border-radius: var(--radius-sm, 4px);
    font-size: 10px;
  }

  .segment-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
  }

  .segment-textarea {
    width: 100%;
    min-height: 52px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-text-primary, #111827);
    font-size: var(--font-size-base, 14px);
    line-height: 1.6;
    font-family: inherit;
    resize: vertical;
    field-sizing: content;
    outline: none;
  }

  .segment-footer-info {
    display: flex;
    gap: var(--space-4, 16px);
    font-size: var(--font-size-xs, 12px);
    color: var(--color-text-muted, #6b7280);
    align-items: center;
  }

  .status-approved { color: var(--color-accent-text, #2563eb); font-weight: 600; }
  .status-success { color: var(--color-success-text, #10b981); font-weight: 500; }
  .status-warning { color: var(--color-warning-text, #d97706); font-weight: 500; }
  .status-error { color: var(--color-error-text, #ef4444); font-weight: 500; }
  .status-pending { color: var(--color-text-muted, #6b7280); }

  .status-processing-box {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .sound-wave-anim {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 14px;
  }

  .sound-wave-anim .bar {
    width: 3px;
    height: 100%;
    background-color: #d97706;
    border-radius: 2px;
    animation: wave-bar 0.7s ease-in-out infinite alternate;
  }

  .sound-wave-anim .bar-1 { animation-delay: 0s; }
  .sound-wave-anim .bar-2 { animation-delay: 0.2s; }
  .sound-wave-anim .bar-3 { animation-delay: 0.4s; }

  .btn-resynthesize {
    color: #d97706 !important;
    background: rgba(245, 158, 11, 0.15) !important;
    font-weight: bold;
  }
  .btn-resynthesize:hover {
    background: rgba(245, 158, 11, 0.3) !important;
  }
  .badge-stale {
    background: rgba(245, 158, 11, 0.15);
    padding: 2px 6px;
    border-radius: var(--radius-sm, 4px);
  }
  .duration-badge {
    color: var(--color-text-muted, #6b7280);
    font-weight: 500;
  }

  .segment-actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-1, 4px);
  }

  .btn-action {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm, 4px);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-surface-raised, #f3f4f6);
    border: none;
    cursor: pointer;
  }
  .btn-action:hover { background: var(--color-bg-surface-hover, #e5e7eb); }

  .btn-secondary-action {
    display: none;
  }

  .segment-row:hover .btn-secondary-action,
  .segment-row.selected .btn-secondary-action {
    display: flex;
  }

  .btn-merge {
    color: #2563eb !important;
    background: rgba(37, 99, 235, 0.1) !important;
  }
  .btn-merge:hover {
    background: rgba(37, 99, 235, 0.2) !important;
  }

  .btn-delete-segment {
    color: #dc2626 !important;
  }
  .btn-delete-segment:hover {
    background: rgba(220, 38, 38, 0.15) !important;
  }
</style>
