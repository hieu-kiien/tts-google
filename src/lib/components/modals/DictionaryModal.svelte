<script lang="ts">
  import type { PronunciationRule } from "../../types/tts";
  import { invoke } from "@tauri-apps/api/core";
  import { playerState } from "../../state/playerState.svelte";
  import { projectState } from "../../state/projectState.svelte";
  import { toastStore } from "../../state/toasts.svelte";
  import { getErrorMessage } from "../../utils/errorUtils";

  let {
    show = false,
    rules = $bindable([]),
    onClose,
    onAddRule,
    onRemoveRule,
    onLoadDefault,
  }: {
    show?: boolean;
    rules: PronunciationRule[];
    onClose?: () => void;
    onAddRule?: (find: string, replace: string) => void;
    onRemoveRule?: (id: string) => void;
    onLoadDefault?: () => void;
  } = $props();

  let newFind = $state("");
  let newReplace = $state("");
  let isPreviewing = $state(false);

  function handleClose() {
    newFind = "";
    newReplace = "";
    onClose?.();
  }

  function handleAdd() {
    if (newFind.trim()) {
      onAddRule?.(newFind.trim(), newReplace.trim());
      newFind = "";
      newReplace = "";
    }
  }

  async function handlePreviewPronunciation(textToPreview: string) {
    if (!textToPreview.trim()) {
      toastStore.showError("Vui lòng nhập từ hoặc cách đọc cần nghe thử!");
      return;
    }
    try {
      isPreviewing = true;
      toastStore.showInfo(`Đang tạo âm thanh nghe thử phát âm: "${textToPreview}"...`);
      const res = await invoke<{ data_url: string; duration_ms: number }>(
        "synthesize_preview_audio",
        {
          text: `Cách đọc: ${textToPreview}`,
          voice: projectState.currentProject?.voice || "Kore",
          model: projectState.currentProject?.model || "gemini-3.1-flash-tts-preview",
          speed: 1.0,
          pitch: 1.0
        }
      );
      playerState.playUrl(res.data_url, "dict_preview");
      toastStore.showSuccess(`Đang phát âm thanh nghe thử phát âm "${textToPreview}"`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi nghe thử phát âm: ${getErrorMessage(err)}`);
    } finally {
      isPreviewing = false;
    }
  }
</script>

{#if show}
  <div 
    class="modal-backdrop" tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) handleClose(); }}
    onkeydown={(e) => { if (e.key === 'Escape') handleClose(); }}
    role="presentation"
  >
    <div 
      class="modal-card wide" 
      role="dialog" 
      aria-modal="true" 
      aria-labelledby="dict-title"
    >
      <div class="modal-header">
        <h3 id="dict-title">📖 Từ Điển Phát Âm Chuẩn Tiếng Việt ({rules.length} Quy tắc)</h3>
        <button class="close-btn" onclick={handleClose} aria-label="Đóng từ điển">✕</button>
      </div>

      <div class="modal-body">
        <div class="add-rule-row">
          <input type="text" class="text-input" placeholder="Từ viết tắt (VD: USD, TP.HCM)..." bind:value={newFind} />
          <input type="text" class="text-input" placeholder="Cách đọc tiếng Việt (VD: đô la Mỹ)..." bind:value={newReplace} />
          <button class="btn secondary" onclick={() => handlePreviewPronunciation(newReplace || newFind)} disabled={isPreviewing} title="Nghe thử phát âm từ mới">
            🔊 Nghe thử phát âm từ mới
          </button>
          <button class="btn primary" onclick={handleAdd}>➕ Thêm Quy Tắc</button>
        </div>

        <div class="rules-table-wrapper">
          <table class="rules-table">
            <thead>
              <tr>
                <th>Từ gốc / Viết tắt</th>
                <th>Thay thế bằng cách đọc</th>
                <th>Thao tác</th>
              </tr>
            </thead>
            <tbody>
              {#each rules as r (r.id)}
                <tr>
                  <td><code>{r.find}</code></td>
                  <td>{r.replace}</td>
                  <td class="rule-actions-td">
                    <button class="btn secondary small" onclick={() => handlePreviewPronunciation(r.replace || r.find)} title="Nghe thử phát âm quy tắc này">🔊 Nghe thử</button>
                    <button class="btn danger small" onclick={() => onRemoveRule?.(r.id)}>🗑️ Xóa</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn secondary" onclick={onLoadDefault}>
          🔄 Nạp 22 Quy Tắc Mặc Định
        </button>
        <button class="btn primary" onclick={handleClose}>
          Đóng
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
  }
  .modal-card.wide {
    background: var(--color-bg-surface);
    border-radius: 12px;
    width: 680px;
    max-width: 95vw;
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-border);
  }
  .modal-header h3 { margin: 0; font-size: 16px; }
  .close-btn { background: none; border: none; font-size: 18px; cursor: pointer; }
  .modal-body { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
  .add-rule-row { display: flex; gap: 10px; }
  .text-input { flex: 1; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 6px; font-size: 13px; }
  .rules-table-wrapper { max-height: 320px; overflow-y: auto; border: 1px solid var(--color-border); border-radius: 6px; }
  .rules-table { width: 100%; border-collapse: collapse; font-size: 13px; }
  .rules-table th, .rules-table td { padding: 10px; border-bottom: 1px solid var(--color-border); text-align: left; }
  .modal-footer {
    display: flex;
    justify-content: space-between;
    padding: 14px 20px;
    background: var(--color-bg-surface-raised);
    border-top: 1px solid var(--color-border);
  }
  .btn { padding: 8px 16px; border-radius: 6px; border: none; font-weight: 600; cursor: pointer; }
  .btn.primary { background: var(--color-accent); color: white; }
  .btn.secondary { background: var(--color-bg-surface-hover); color: var(--color-text-secondary); }
  .btn.danger { background: var(--color-error-bg); color: var(--color-error-text); }
  .btn.small { padding: 4px 8px; font-size: 12px; }
</style>
