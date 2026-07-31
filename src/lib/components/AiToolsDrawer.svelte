<script lang="ts">
  let {
    show = false,
    targetLang = $bindable("Tiếng Việt"),
    isTranslating = false,
    isPolishing = false,
    onTranslate,
    onPolish,
  }: {
    show?: boolean;
    targetLang: string;
    isTranslating?: boolean;
    isPolishing?: boolean;
    onTranslate?: () => void;
    onPolish?: () => void;
  } = $props();
</script>

{#if show}
  <div class="ai-tools-drawer">
    <div class="ai-tool-group">
      <span class="ai-label">🌐 AI Dịch Thuật:</span>
      <select bind:value={targetLang} class="select-input small-select">
        <option value="Tiếng Việt">Sang Tiếng Việt</option>
        <option value="Tiếng Anh">Sang Tiếng Anh</option>
        <option value="Tiếng Trung">Sang Tiếng Trung</option>
        <option value="Tiếng Nhật">Sang Tiếng Nhật</option>
        <option value="Tiếng Hàn">Sang Tiếng Hàn</option>
      </select>
      <button class="btn accent small" onclick={onTranslate} disabled={isTranslating}>
        {isTranslating ? "Đang dịch..." : "🌐 Dịch Ngay"}
      </button>
    </div>

    <div class="ai-tool-group">
      <button class="btn outline small polish-btn" onclick={onPolish} disabled={isPolishing}>
        {isPolishing ? "Đang tối ưu..." : "🪄 Gemini AI Tối Ưu Kịch Bản TTS"}
      </button>
    </div>
  </div>
{/if}

<style>
  .ai-tools-drawer {
    display: flex;
    gap: 16px;
    align-items: center;
    background: var(--color-success-bg);
    border: 1px solid var(--color-success);
    border-radius: 8px;
    padding: 10px 14px;
    margin-bottom: 12px;
  }
  .ai-tool-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ai-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-success-text);
  }
  .select-input {
    padding: 6px 10px;
    border: 1px solid var(--color-success);
    border-radius: 6px;
    font-size: 12px;
  }
  .btn {
    padding: 6px 12px;
    border-radius: 6px;
    border: none;
    font-weight: 600;
    font-size: 12px;
    cursor: pointer;
  }
  .btn.accent { background: var(--color-success); color: white; }
  .btn.outline { background: var(--color-bg-surface); border: 1px solid var(--color-success); color: var(--color-success-text); }
</style>
