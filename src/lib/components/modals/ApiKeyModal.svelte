<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let {
    show = false,
    apiKeyInput = $bindable(""),
    rememberKey = $bindable(true),
    isTesting = false,
    onSave,
    onTest,
    onClose,
  }: {
    show?: boolean;
    apiKeyInput: string;
    rememberKey: boolean;
    isTesting?: boolean;
    onSave?: () => void;
    onTest?: () => void;
    onClose?: () => void;
  } = $props();

  // Multi-key state
  let extraKeys = $state<string[]>([]);
  let savedKeysMasked = $state<string[]>([]);
  let keyCount = $state(0);

  // Load saved keys info when modal opens
  $effect(() => {
    if (show) {
      loadKeysInfo();
    }
  });

  async function loadKeysInfo() {
    try {
      const info = await invoke<{ count: number; keys_masked: string[]; configured: boolean }>("get_api_keys_info");
      savedKeysMasked = info.keys_masked;
      keyCount = info.count;
    } catch {
      // Fallback — no multi-key support yet
    }
  }

  function addExtraKeyField() {
    if (extraKeys.length < 4) {
      extraKeys = [...extraKeys, ""];
    }
  }

  function removeExtraKey(index: number) {
    extraKeys = extraKeys.filter((_, i) => i !== index);
  }

  async function saveAllKeys() {
    // Collect all non-empty keys
    const allKeys = [apiKeyInput, ...extraKeys].filter(k => k.trim().length > 0);

    if (allKeys.length === 0) {
      onSave?.();
      return;
    }

    if (allKeys.length === 1) {
      // Single key — use original flow
      onSave?.();
      return;
    }

    // Multi-key — use new API
    try {
      const result = await invoke<{ count: number; keys_masked: string[]; configured: boolean }>(
        "save_api_keys",
        { keys: allKeys, remember: rememberKey }
      );
      savedKeysMasked = result.keys_masked;
      keyCount = result.count;
    } catch {
      // Fallback to single key
      onSave?.();
    }
  }

  async function removeKeyAt(index: number) {
    try {
      const result = await invoke<{ count: number; keys_masked: string[]; configured: boolean }>(
        "remove_api_key_at",
        { index }
      );
      savedKeysMasked = result.keys_masked;
      keyCount = result.count;
    } catch {
      // ignore
    }
  }
</script>

{#if show}
  <div 
    class="modal-backdrop" tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) onClose?.(); }}
    onkeydown={(e) => { if (e.key === 'Escape') onClose?.(); }}
    role="presentation"
  >
    <div 
      class="modal-card" 
      role="dialog" 
      aria-modal="true" 
      aria-labelledby="apikey-title"
    >
      <div class="modal-header">
        <h3 id="apikey-title">🔑 Cấu Hình Gemini API Key</h3>
        <button class="close-btn" onclick={onClose} aria-label="Đóng modal API key">✕</button>
      </div>

      <div class="modal-body">
        <p class="hint">
          API Key được lưu an toàn trong <strong>Windows Credential Manager</strong>.
          Dùng nhiều key để tăng gấp bội quota (RPM/RPD).
        </p>

        <!-- Saved keys display -->
        {#if savedKeysMasked.length > 0}
          <div class="saved-keys">
            <span class="section-label">🗝️ Keys đã lưu ({keyCount})</span>
            {#each savedKeysMasked as masked, i}
              <div class="saved-key-row">
                <span class="key-badge">Key {i + 1}</span>
                <code class="key-masked">{masked}</code>
                <button class="btn-remove" onclick={() => removeKeyAt(i)} title="Xóa key này">✕</button>
              </div>
            {/each}
          </div>
        {/if}

        <!-- Primary key input -->
        <div class="form-group">
          <label for="key-input">{savedKeysMasked.length > 0 ? 'Thêm key mới:' : 'Nhập Gemini API Key:'}</label>
          <input
            id="key-input"
            type="password"
            class="text-input"
            placeholder="AIzaSy..."
            bind:value={apiKeyInput}
          />
        </div>

        <!-- Extra key inputs -->
        {#each extraKeys as _, i}
          <div class="form-group extra-key">
            <div class="extra-key-header">
              <label for="key-extra-{i}">Key phụ {i + 2}:</label>
              <button class="btn-remove-small" onclick={() => removeExtraKey(i)}>✕</button>
            </div>
            <input
              id="key-extra-{i}"
              type="password"
              class="text-input"
              placeholder="AIzaSy..."
              bind:value={extraKeys[i]}
            />
          </div>
        {/each}

        <!-- Add more key button -->
        {#if extraKeys.length < 4}
          <button class="btn-add-key" onclick={addExtraKeyField}>
            ➕ Thêm API Key ({extraKeys.length + 1 + savedKeysMasked.length}/5)
          </button>
        {/if}

        <div class="checkbox-row">
          <label>
            <input type="checkbox" bind:checked={rememberKey} />
            Lưu an toàn trong hệ thống (Win32 Credential Manager)
          </label>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn secondary" onclick={onTest} disabled={isTesting}>
          {isTesting ? "Đang thử..." : "⚡ Test Kết Nối"}
        </button>
        <button class="btn primary" onclick={extraKeys.some(k => k.trim()) ? saveAllKeys : onSave}>
          💾 {extraKeys.some(k => k.trim()) ? `Lưu ${extraKeys.filter(k => k.trim()).length + 1} Keys` : 'Lưu Key'}
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
  .modal-card {
    background: var(--color-bg-surface);
    border-radius: 12px;
    width: 520px;
    max-width: 90vw;
    max-height: 85vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg);
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-border);
  }
  .modal-header h3 { margin: 0; font-size: 16px; }
  .close-btn { background: none; border: none; font-size: 18px; cursor: pointer; color: var(--color-text-muted); }
  .modal-body { padding: 20px; display: flex; flex-direction: column; gap: 14px; }
  .hint { font-size: 13px; color: var(--color-text-secondary); margin: 0; line-height: 1.5; }
  .form-group { display: flex; flex-direction: column; gap: 6px; }
  .form-group label { font-size: 13px; font-weight: 600; }
  .section-label { font-size: 13px; font-weight: 600; margin-bottom: 6px; display: block; }
  .text-input {
    padding: 10px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    font-size: 14px;
    background: var(--color-bg-surface);
    color: var(--color-text-primary);
  }
  .checkbox-row { font-size: 13px; color: var(--color-text-secondary); }

  /* Saved keys */
  .saved-keys {
    background: var(--color-bg-surface-raised);
    border-radius: 8px;
    padding: 12px;
  }
  .saved-key-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
  }
  .saved-key-row + .saved-key-row {
    border-top: 1px solid var(--color-border-subtle);
  }
  .key-badge {
    font-size: 11px;
    font-weight: 700;
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    padding: 2px 8px;
    border-radius: 4px;
    white-space: nowrap;
  }
  .key-masked {
    font-size: 13px;
    color: var(--color-text-muted);
    flex: 1;
  }
  .btn-remove {
    background: none;
    border: none;
    color: var(--color-error);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .btn-remove:hover { background: var(--color-error-bg); }

  /* Extra key */
  .extra-key-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .btn-remove-small {
    background: none;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 13px;
  }

  /* Add key button */
  .btn-add-key {
    background: none;
    border: 1px dashed var(--color-border);
    border-radius: 6px;
    padding: 8px;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 13px;
    text-align: center;
    transition: all 0.15s;
  }
  .btn-add-key:hover {
    border-color: var(--color-accent);
    color: var(--color-accent);
    background: var(--color-accent-subtle);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    background: var(--color-bg-surface-raised);
    border-top: 1px solid var(--color-border);
  }
  .btn { padding: 8px 16px; border-radius: 6px; border: none; font-weight: 600; cursor: pointer; }
  .btn.primary { background: var(--color-accent); color: white; }
  .btn.secondary { background: var(--color-bg-surface-hover); color: var(--color-text-secondary); }
</style>
