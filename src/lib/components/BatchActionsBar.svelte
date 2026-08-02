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
