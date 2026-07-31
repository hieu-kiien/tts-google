<script lang="ts">
  import { toastStore } from "../state/toasts.svelte";
</script>

<div class="toast-region" role="region" aria-label="Thông báo hệ thống" aria-live="polite">
  {#each toastStore.toasts as toast (toast.id)}
    <div class="toast-item {toast.type}" role="status">
      <span class="toast-text">{toast.text}</span>
      {#if toast.diagnosticId}
        <span class="diag-id">ID: {toast.diagnosticId}</span>
      {/if}
      <button class="toast-close" onclick={() => toastStore.dismiss(toast.id)} aria-label="Đóng thông báo">✕</button>
    </div>
  {/each}
</div>

<style>
  .toast-region {
    position: fixed;
    top: 20px;
    right: 20px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 400px;
  }
  .toast-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 12px 16px;
    border-radius: var(--radius-md, 8px);
    background: var(--color-bg-surface, #ffffff);
    box-shadow: var(--shadow-lg, 0 4px 12px rgba(0, 0, 0, 0.15));
    border-left: 4px solid var(--color-info, #3b82f6);
    color: var(--color-text-primary, #0f172a);
    font-size: 14px;
    animation: slideIn 0.2s ease-out;
  }
  .toast-item.error {
    border-left-color: var(--color-error, #dc2626);
    background: var(--color-error-bg, #fee2e2);
    color: var(--color-error-text, #b91c1c);
  }
  .toast-item.success {
    border-left-color: var(--color-success, #16a34a);
    background: var(--color-success-bg, #dcfce7);
    color: var(--color-success-text, #15803d);
  }
  .toast-item.warning {
    border-left-color: var(--color-warning, #d97706);
    background: var(--color-warning-bg, #fef3c7);
    color: var(--color-warning-text, #b45309);
  }
  .toast-item.info {
    border-left-color: var(--color-info, #0284c7);
    background: var(--color-info-bg, #e0f2fe);
    color: var(--color-info-text, #0369a1);
  }
  .toast-close {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    color: currentColor;
    opacity: 0.7;
    padding: 2px 6px;
  }
  .toast-close:hover { opacity: 1; }
  .diag-id {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    opacity: 0.85;
  }
  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
</style>
