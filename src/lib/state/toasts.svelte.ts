export interface ToastMessage {
  id: string;
  type: "success" | "error" | "info" | "warning";
  text: string;
  diagnosticId?: string;
}

class ToastStore {
  toasts = $state<ToastMessage[]>([]);

  show(text: string, type: ToastMessage["type"] = "info", diagnosticId?: string) {
    // Limit max concurrent toasts
    if (this.toasts.length >= 5) {
      this.toasts = this.toasts.slice(-4);
    }
    const id = Math.random().toString(36).substring(2, 9);
    const item: ToastMessage = { id, type, text, diagnosticId };
    this.toasts.push(item);

    setTimeout(() => {
      this.dismiss(id);
    }, 5000);
  }

  showError(text: string, diagnosticId?: string) {
    this.show(text, "error", diagnosticId);
  }

  showSuccess(text: string) {
    this.show(text, "success");
  }

  showInfo(text: string) {
    this.show(text, "info");
  }

  showWarning(text: string) {
    this.show(text, "warning");
  }

  dismiss(id: string) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }
}

export const toastStore = new ToastStore();
