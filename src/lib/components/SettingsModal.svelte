<script lang="ts">
  import { getAppSettings, updateAppSettings } from "../api/settingsClient";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  let concurrency = $state(1);
  let isSaving = $state(false);
  let saveSuccess = $state(false);
  let errorMessage = $state<string | null>(null);

  $effect(() => {
    if (isOpen) {
      loadSettings();
    }
  });

  async function loadSettings() {
    try {
      const settings = await getAppSettings();
      concurrency = settings.concurrency || 1;
    } catch (e: any) {
      errorMessage = e?.message || "Không thể tải cấu hình cài đặt.";
    }
  }

  async function handleSave() {
    isSaving = true;
    errorMessage = null;
    saveSuccess = false;
    try {
      await updateAppSettings(concurrency);
      saveSuccess = true;
      setTimeout(() => {
        saveSuccess = false;
        onClose();
      }, 800);
    } catch (e: any) {
      errorMessage = e?.message || "Không thể lưu cài đặt.";
    } finally {
      isSaving = false;
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" onclick={onClose} role="button" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && onClose()}>
    <div class="modal-content" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="modal-icon">⚙️</span>
          <h2>Cấu Hình Ứng Dụng</h2>
        </div>
        <button class="btn-icon-close" onclick={onClose} aria-label="Đóng">✕</button>
      </div>

      <div class="modal-body">
        {#if errorMessage}
          <div class="alert alert-error">
            <span>⚠️ {errorMessage}</span>
          </div>
        {/if}

        <div class="setting-group">
          <div class="setting-header">
            <div>
              <h3>Số Luồng Xử Lý Hàng Đợi (Worker Concurrency)</h3>
              <p class="setting-desc">Tùy chỉnh số lượng câu/đoạn text gửi tới Gemini API song song.</p>
            </div>
            <span class="badge {concurrency === 1 ? 'badge-safe' : 'badge-turbo'}">
              {concurrency === 1 ? '🛡️ Free Tier (An toàn)' : `⚡ Paid Tier (${concurrency}x Turbo)`}
            </span>
          </div>

          <div class="slider-container">
            <input 
              type="range" 
              min="1" 
              max="5" 
              step="1" 
              bind:value={concurrency} 
              class="slider"
            />
            <div class="slider-labels">
              <span class={concurrency === 1 ? 'active' : ''}>1 (Tối ưu Free Tier)</span>
              <span class={concurrency === 2 ? 'active' : ''}>2</span>
              <span class={concurrency === 3 ? 'active' : ''}>3</span>
              <span class={concurrency === 4 ? 'active' : ''}>4</span>
              <span class={concurrency === 5 ? 'active' : ''}>5 (Cực nhanh)</span>
            </div>
          </div>

          <div class="info-card">
            {#if concurrency === 1}
              <p>💡 <strong>Khuyên dùng cho Gemini Free Tier:</strong> Chạy 1 luồng tuần tự tránh chạm giới hạn <code>HTTP 429 Too Many Requests</code> và đảm bảo không bị gián đoạn.</p>
            {:else}
              <p>🚀 <strong>Chế độ Turbo ({concurrency} luồng):</strong> Thích hợp cho tài khoản Google Cloud Pay-As-You-Go hoặc có nhiều API Key xoay vòng. Giúp tổng hợp các đoạn audio nhanh gấp {concurrency} lần!</p>
            {/if}
          </div>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={onClose} disabled={isSaving}>Hủy</button>
        <button class="btn btn-primary" onclick={handleSave} disabled={isSaving}>
          {#if isSaving}
            <span>⏳ Đang lưu...</span>
          {:else if saveSuccess}
            <span>✅ Đã lưu!</span>
          {:else}
            <span>Lưu Cấu Hình</span>
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(10, 15, 25, 0.75);
    backdrop-filter: blur(8px);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
  }

  .modal-content {
    background: #181d28;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    width: 100%;
    max-width: 520px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
    color: #e2e8f0;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .modal-title {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .modal-title h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: #f8fafc;
  }

  .modal-icon {
    font-size: 1.4rem;
  }

  .btn-icon-close {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 6px;
    transition: all 0.2s;
  }

  .btn-icon-close:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }

  .modal-body {
    padding: 1.5rem;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .setting-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .setting-header h3 {
    margin: 0 0 0.25rem 0;
    font-size: 1rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .setting-desc {
    margin: 0;
    font-size: 0.85rem;
    color: #94a3b8;
  }

  .badge {
    padding: 0.3rem 0.75rem;
    border-radius: 20px;
    font-size: 0.78rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .badge-safe {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(52, 211, 153, 0.3);
  }

  .badge-turbo {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(251, 191, 36, 0.3);
  }

  .slider-container {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    background: #0f131c;
    padding: 1.25rem;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .slider {
    width: 100%;
    accent-color: #6366f1;
    cursor: pointer;
  }

  .slider-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: #64748b;
    margin-top: 0.25rem;
  }

  .slider-labels .active {
    color: #818cf8;
    font-weight: 600;
  }

  .info-card {
    background: rgba(99, 102, 241, 0.08);
    border: 1px solid rgba(99, 102, 241, 0.2);
    border-radius: 10px;
    padding: 0.9rem;
    font-size: 0.88rem;
    color: #c7d2fe;
    line-height: 1.4;
  }

  .info-card p {
    margin: 0;
  }

  .alert-error {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fca5a5;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
    font-size: 0.88rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background: #111520;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .btn {
    padding: 0.6rem 1.25rem;
    border-radius: 8px;
    font-weight: 500;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.08);
    color: #cbd5e1;
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
  }

  .btn-primary {
    background: linear-gradient(135deg, #6366f1 0%, #4f46e5 100%);
    color: #fff;
    box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 6px 16px rgba(99, 102, 241, 0.4);
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.96); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
