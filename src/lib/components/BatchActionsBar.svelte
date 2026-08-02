<script lang="ts">
  interface Props {
    selectedCount: number;
    isAllSelected: boolean;
    onToggleSelectAll: () => void;
    onDeleteSelected: () => void;
  }

  let { selectedCount, isAllSelected, onToggleSelectAll, onDeleteSelected }: Props = $props();
</script>

{#if selectedCount > 0}
  <div class="segment-batch-toolbar active">
    <label class="select-all-segments-label">
      <input type="checkbox" checked={isAllSelected} onchange={onToggleSelectAll} />
      Chọn tất cả
    </label>
    <div class="batch-segment-actions">
      <span class="selected-count">Đã chọn <strong>{selectedCount}</strong> đoạn</span>
      <button class="btn btn-danger btn-sm" onclick={onDeleteSelected}>
        🗑️ Xóa hàng loạt
      </button>
    </div>
  </div>
{/if}

<style>
  .segment-batch-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    background: var(--color-bg-surface-raised, #f3f4f6);
    border-bottom: 1px solid var(--color-border, #e5e7eb);
    font-size: 13px;
  }

  .segment-batch-toolbar.active {
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .select-all-segments-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-weight: 500;
  }

  .batch-segment-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
</style>
