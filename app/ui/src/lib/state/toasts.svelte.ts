export interface Toast {
  id: number;
  message: string;
  detail: string | null;
  tone: "neutral" | "error";
}

class Toasts {
  items = $state<Toast[]>([]);
  private nextId = 1;

  show(message: string, options: { detail?: string | null; tone?: Toast["tone"] } = {}): void {
    const toast: Toast = {
      id: this.nextId++,
      message,
      detail: options.detail ?? null,
      tone: options.tone ?? "neutral",
    };
    // One message at a time; a repeat of the same text replaces the old one.
    this.items = [...this.items.filter((t) => t.message !== message), toast].slice(-3);
    setTimeout(() => this.dismiss(toast.id), toast.tone === "error" ? 6000 : 3200);
  }

  dismiss(id: number): void {
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();
