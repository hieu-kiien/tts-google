<script lang="ts">
  import { projectState } from "../state/projectState.svelte";
  import { uiState } from "../state/uiState.svelte";
  import type { SegmentRecord } from "../types/tts";

  let segmentSearch = $state("");
  let selectedFilter = $state("all");

  const segments = $derived.by<SegmentRecord[]>(() => {
    let list = projectState.segments;
    if (selectedFilter !== "all") {
      list = list.filter((s: SegmentRecord) => s.status === selectedFilter);
    }
    if (segmentSearch.trim()) {
      const q = segmentSearch.toLowerCase();
      list = list.filter((s: SegmentRecord) => s.text.toLowerCase().includes(q) || s.position.toString().includes(q));
    }
    return list;
  });

  const completedCount = $derived.by<number>(() => {
    return projectState.segments.filter((s: SegmentRecord) => s.status === "success").length;
  });

  const totalCount = $derived.by<number>(() => {
    return Math.max(1, projectState.segments.length);
  });

  function selectSegment(seg: SegmentRecord) {
    projectState.activeSegmentId = seg.id;
    uiState.activeView = "editor";
  }
</script>

<aside class="sidebar-container" aria-label="Điều hướng sidebar trái">
  <!-- Chapters / Structure Section -->
  <div class="sidebar-header">
    <h3>📖 Cấu trúc dự án</h3>
    <button class="btn-sm btn-secondary" onclick={() => uiState.activeView = 'library'} title="Chuyển sang Thư viện dự án">
      📚 Thư viện
    </button>
  </div>

  <!-- Overall Progress Summary -->
  <div class="progress-box">
    <div class="progress-labels">
      <span>Tiến độ tạo audio</span>
      <strong>{completedCount}/{totalCount} đoạn</strong>
    </div>
    <div class="progress-bar-bg" role="progressbar" aria-valuenow={completedCount} aria-valuemax={totalCount}>
      <div 
        class="progress-bar-fill" 
        style={`width: ${Math.round((completedCount / totalCount) * 100)}%`}
      ></div>
    </div>
  </div>

  <!-- Filter & Search Segment Bar -->
  <div class="sidebar-filters">
    <input 
      type="text" 
      placeholder="Tìm đoạn / từ ngữ..." 
      bind:value={segmentSearch}
      aria-label="Tìm kiếm đoạn văn bản"
    />
    <select bind:value={selectedFilter} aria-label="Lọc đoạn theo trạng thái">
      <option value="all">Tất cả ({projectState.segments.length})</option>
      <option value="pending">Chưa tạo</option>
      <option value="processing">Đang tạo</option>
      <option value="success">Hoàn tất</option>
      <option value="approved">Đã duyệt (Approved)</option>
      <option value="failed">Có lỗi API</option>
    </select>
  </div>

  <!-- Segment List Navigation -->
  <div class="segments-nav-list" role="navigation" aria-label="Danh sách các đoạn">
    {#each segments as seg (seg.id)}
      <button 
        class="nav-item {projectState.activeSegmentId === seg.id ? 'active' : ''}"
        onclick={() => selectSegment(seg)}
        aria-label={`Đoạn ${seg.position}, trạng thái ${seg.status}`}
      >
        <div class="nav-item-top">
          <span class="seg-idx">#{seg.position}</span>
          <span class="seg-status-dot status-{seg.status}" title={`Trạng thái: ${seg.status}`}></span>
        </div>
        <div class="nav-item-preview">{seg.text}</div>
      </button>
    {/each}
  </div>
</aside>

<style>
  .sidebar-container {
    width: var(--sidebar-width);
    height: 100%;
    background: var(--color-bg-surface);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    padding: var(--space-4);
    gap: var(--space-3);
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .sidebar-header h3 {
    font-size: var(--font-size-sm);
    color: var(--color-text-primary);
  }

  .progress-box {
    background: var(--color-bg-surface-raised);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .progress-labels {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
  }

  .progress-bar-bg {
    height: 6px;
    background: var(--color-bg-surface-hover);
    border-radius: var(--radius-full);
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.3s ease;
  }

  .sidebar-filters {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .sidebar-filters input, .sidebar-filters select {
    height: var(--target-btn-sm);
    font-size: var(--font-size-xs);
  }

  .segments-nav-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .nav-item {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--color-bg-surface);
  }

  .nav-item:hover { background: var(--color-bg-surface-hover); }

  .nav-item.active {
    background: var(--color-bg-surface-selected);
    border-color: var(--color-accent);
  }

  .nav-item-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .seg-idx {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--color-text-secondary);
  }

  .seg-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-text-muted);
  }

  .seg-status-dot.status-success { background: var(--color-success); }
  .seg-status-dot.status-approved { background: var(--color-accent); }
  .seg-status-dot.status-failed { background: var(--color-error); }
  .seg-status-dot.status-processing { background: var(--color-warning); }

  .nav-item-preview {
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .btn-sm {
    height: var(--target-btn-sm);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
  }
  .btn-secondary { background: var(--color-bg-surface-raised); color: var(--color-text-primary); border: 1px solid var(--color-border); }
</style>
