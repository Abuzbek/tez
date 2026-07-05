import type { Observer, Source } from "./graph";

export class Watcher implements Observer {
  private readonly watched = new Set<Source>();
  private readonly armed = new Set<Source>();

  constructor(private readonly onNotify: () => void) {}

  watch(...signals: Source[]): void {
    for (const signal of signals) {
      this.watched.add(signal);
      if (!this.armed.has(signal)) {
        signal.addObserver(this);
        this.armed.add(signal);
      }
    }
  }

  unwatch(...signals: Source[]): void {
    for (const signal of signals) {
      this.watched.delete(signal);
      if (this.armed.has(signal)) {
        signal.removeObserver(this);
        this.armed.delete(signal);
      }
    }
  }

  notify(): void {
    for (const signal of this.watched) {
      signal.removeObserver(this);
    }
    this.armed.clear();
    this.onNotify();
  }

  getPending(): Source[] {
    return Array.from(this.watched).filter((signal) => !this.armed.has(signal));
  }
}
