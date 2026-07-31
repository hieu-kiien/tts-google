<script lang="ts">
  import { getQuotaMetrics } from "../api/settingsClient";
  import type { QuotaMetrics } from "../types/tts";

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  let metrics = $state<QuotaMetrics | null>(null);
  let isLoading = $state(false);
  let intervalId: number | null = null;

  $effect(() => {
    if (isOpen) {
      fetchMetrics();
      intervalId = window.setInterval(fetchMetrics, 3000);
    } else if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }

    return () => {
      if (intervalId) clearInterval(intervalId);
    };
  });

  async function fetchMetrics() {
    try {
      metrics = await getQuotaMetrics();
    } catch (e) {
      console.error("Failed to fetch quota metrics:", e);
    }
  }

  function formatNumber(num: number): string {
    return new Intl.NumberFormat('vi-VN').format(num);
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" onclick={onClose} role="button" tabindex="-1" onkeydown={(e) => e.key === 'Escape' && onClose()}>
    <div class="modal-content" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="modal-icon">📊</span>
          <h2>Bảng Thống Kê & Giám Sát Gemini API</h2>
        </div>
        <button class="btn-icon-close" onclick={onClose} aria-label="Đóng">✕</button>
      </div>

      <div class="modal-body">
        {#if metrics}
          <div class="metrics-grid">
            <div class="stat-card">
              <div class="stat-header">
                <span class="stat-icon">⚡</span>
                <span class="stat-label">Request Hôm Nay</span>
              </div>
              <div class="stat-value">{formatNumber(metrics.today_requests)}</div>
              <div class="stat-sub">Tổng tích lũy: {formatNumber(metrics.total_requests)}</div>
            </div>

            <div class="stat-card">
              <div class="stat-header">
                <span class="stat-icon">📝</span>
                <span class="stat-label">Ký Tự Đã Chuyển Đổi</span>
              </div>
              <div class="stat-value">{formatNumber(metrics.today_chars)}</div>
              <div class="stat-sub">Tổng tích lũy: {formatNumber(metrics.total_chars)} chars</div>
            </div>

            <div class="stat-card">
              <div class="stat-header">
                <span class="stat-icon">⏱️</span>
                <span class="stat-label">Độ Trễ Phản Hồi (Avg)</span>
              </div>
              <div class="stat-value">{metrics.avg_latency_ms > 0 ? `${metrics.avg_latency_ms} ms` : 'N/A'}</div>
              <div class="stat-sub">Tốc độ trung bình Gemini REST</div>
            </div>

            <div class="stat-card">
              <div class="stat-header">
                <span class="stat-icon">🛡️</span>
                <span class="stat-label">Sự Cố Rate Limit (429)</span>
              </div>
              <div class="stat-value highlight-warning">{formatNumber(metrics.today_rate_limits)}</div>
              <div class="stat-sub">Đã tự động hoãn & thử lại</div>
            </div>
          </div>

          <div class="dashboard-info">
            <p>🔄 <i>Dữ liệu thống kê được cập nhật thời gian thực mỗi 3 giây từ SQLite Local DB.</i></p>
          </div>
        {:else}
          <div class="loading-state">
            <span>⏳ Đang tải dữ liệu thống kê...</span>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={onClose}>Đóng</button>
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
    max-width: 640px;
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
    font-size: 1.2rem;
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

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
  }

  .stat-card {
    background: #0f131c;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .stat-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .stat-icon {
    font-size: 1.1rem;
  }

  .stat-label {
    font-size: 0.82rem;
    color: #94a3b8;
    font-weight: 500;
  }

  .stat-value {
    font-size: 1.6rem;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: -0.5px;
  }

  .highlight-warning {
    color: #f59e0b;
  }

  .stat-sub {
    font-size: 0.75rem;
    color: #64748b;
  }

  .dashboard-info {
    margin-top: 1.25rem;
    font-size: 0.8rem;
    color: #64748b;
    text-align: center;
  }

  .loading-state {
    padding: 2rem;
    text-align: center;
    color: #94a3b8;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
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

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.96); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
