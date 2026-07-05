import { Computed } from "./computed";
import { Watcher } from "./watcher";
import { scheduleEffect } from "./batch";

export type EffectCleanup = () => void;
export type EffectFn = () => void | EffectCleanup;

let currentOwner: Effect | null = null;

export class Effect {
  private readonly computed: Computed<void>;
  private readonly watcher: Watcher;
  private readonly children = new Set<Effect>();
  private cleanup: EffectCleanup | undefined;
  private disposed = false;

  constructor(private readonly fn: EffectFn) {
    if (currentOwner) {
      currentOwner.children.add(this);
    }
    this.computed = new Computed(() => {
      this.disposeChildren();
      this.cleanup?.();
      this.cleanup = undefined;
      const previousOwner = currentOwner;
      currentOwner = this;
      try {
        this.cleanup = this.fn() ?? undefined;
      } finally {
        currentOwner = previousOwner;
      }
    });
    this.watcher = new Watcher(() => scheduleEffect(this));
    this.run();
  }

  private disposeChildren(): void {
    for (const child of this.children) {
      child.dispose();
    }
    this.children.clear();
  }

  run(): void {
    if (this.disposed) return;
    this.computed.get();
    this.watcher.watch(this.computed);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.disposeChildren();
    this.watcher.unwatch(this.computed);
    this.cleanup?.();
    this.cleanup = undefined;
  }
}

export function effect(fn: EffectFn): () => void {
  const instance = new Effect(fn);
  return () => instance.dispose();
}
