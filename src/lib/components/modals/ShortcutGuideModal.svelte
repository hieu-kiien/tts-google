<script lang="ts">
  import { uiState } from "../../state/uiState.svelte";

  const shortcuts = [
    { key: "Ctrl + S", mac: "Cmd + S", desc: "Lưu dự án an toàn" },
    { key: "Ctrl + O", mac: "Cmd + O", desc: "Mở dự án" },
    { key: "Ctrl + I", mac: "Cmd + I", desc: "Mở Wizard nhập tài liệu" },
    { key: "Ctrl + F", mac: "Cmd + F", desc: "Tìm kiếm & Thay thế trong editor" },
    { key: "Ctrl + Z", mac: "Cmd + Z", desc: "Undo (Hoàn tác)" },
    { key: "Ctrl + Y", mac: "Cmd + Shift + Z", desc: "Redo (Làm lại)" },
    { key: "Ctrl + Enter", mac: "Cmd + Enter", desc: "Nghe thử đoạn đang chọn" },
    { key: "Space", mac: "Space", desc: "Phát / Tạm dừng audio (Khi không gõ văn bản)" },
    { key: "Ctrl + .", mac: "Cmd + .", desc: "Dừng phát âm thanh" },
    { key: "Ctrl + Shift + Enter", mac: "Cmd + Shift + Enter", desc: "Bắt đầu tạo âm thanh hàng đợi" },
    { key: "Alt + ↑ / ↓", mac: "Option + ↑ / ↓", desc: "Chuyển giữa các đoạn" },
    { key: "Ctrl + L", mac: "Cmd + L", desc: "Khóa / Mở khóa đoạn" },
    { key: "Ctrl + J", mac: "Cmd + J", desc: "Mở / Thu gọn hàng đợi" },
  ];

  function closeModal() {
    uiState.showShortcutGuide = false;
  }
</script>

{#if uiState.showShortcutGuide}
  <div 
    class="modal-backdrop" tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) closeModal(); }}
    onkeydown={(e) => { if (e.key === 'Escape') closeModal(); }}
    role="presentation"
  >
    <div 
      class="modal-dialog" 
      role="dialog" 
      aria-modal="true" 
      aria-labelledby="shortcut-title"
    >
      <header class="modal-header">
        <h2 id="shortcut-title">⌨️ Tra Cứu Phím Tắt Bàn Phím</h2>
        <button class="close-btn" onclick={closeModal} aria-label="Đóng bảng phím tắt">✕</button>
      </header>

      <div class="modal-body">
        <table class="shortcuts-table">
          <thead>
            <tr>
              <th>Chức năng</th>
              <th>Windows / Linux</th>
              <th>macOS</th>
            </tr>
          </thead>
          <tbody>
            {#each shortcuts as s}
              <tr>
                <td>{s.desc}</td>
                <td><kbd>{s.key}</kbd></td>
                <td><kbd>{s.mac}</kbd></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <footer class="modal-footer">
        <button class="btn btn-primary" onclick={closeModal}>Đã hiểu (Esc)</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-dialog {
    background: var(--color-bg-surface);
    border-radius: var(--radius-lg);
    width: 580px;
    max-width: 90vw;
    box-shadow: var(--shadow-lg);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .close-btn { font-size: var(--font-size-lg); color: var(--color-text-muted); }

  .modal-body {
    padding: var(--space-4);
    max-height: 400px;
    overflow-y: auto;
  }

  .shortcuts-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-xs);
  }

  .shortcuts-table th, .shortcuts-table td {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border-subtle);
    text-align: left;
  }

  .shortcuts-table th {
    background: var(--color-bg-surface-raised);
    color: var(--color-text-secondary);
  }

  kbd {
    background: var(--color-bg-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    box-shadow: 0 1px 0 rgba(0,0,0,0.2);
  }

  .modal-footer {
    padding: var(--space-4);
    border-top: 1px solid var(--color-border);
    display: flex;
    justify-content: flex-end;
  }

  .btn {
    height: var(--target-btn-md);
    padding: 0 var(--space-4);
    border-radius: var(--radius-md);
  }
  .btn-primary { background: var(--color-accent); color: white; }
</style>
