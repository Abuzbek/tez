import { Computed } from "./computed";
import { Watcher } from "./watcher";
import { scheduleEffect } from "./batch";
import { untrack } from "./untrack";

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
    // untrack(): this.computed.get() is Effect's own internal recomputation
    // trigger, not a "read" any external consumer should depend on. Without
    // untrack, Computed.get()'s trailing trackAccess(this) call would run
    // with whatever currentConsumer happens to be active at that moment —
    // and if this Effect was constructed synchronously during another
    // effect's own execution (e.g. a nested effect() call), that outer
    // effect's computed would spuriously become a dependent of this one,
    // even though nothing ever reads an Effect's (always-void) return value.
    untrack(() => this.computed.get());
    this.watcher.watch(this.computed);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.disposeChildren();
    this.watcher.unwatch(this.computed);
    this.computed.dispose();
    this.cleanup?.();
    this.cleanup = undefined;
  }
}

export function effect(fn: EffectFn): () => void {
  const instance = new Effect(fn);
  return () => instance.dispose();
}
