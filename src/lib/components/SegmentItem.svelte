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
