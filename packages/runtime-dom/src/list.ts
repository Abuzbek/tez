import { signal, computed, effect, untrack, Signal } from "@tez/signals";

interface ListEntry<T, U extends Node> {
  itemSignal: ReturnType<typeof signal<T>>;
  indexSignal: ReturnType<typeof signal<number>>;
  node: U;
  dispose: () => void;
}

export function mapArray<T, U extends Node>(
  items: () => T[],
  keyFn: (item: T) => unknown,
  renderItem: (item: () => T, index: () => number) => U,
): () => U[] {
  const entries = new Map<unknown, ListEntry<T, U>>();
  const ordered = signal<U[]>([]);
  const itemsView = computed(() => items());

  function reconcile(): void {
    const currentItems = untrack(() => itemsView.get());
    const nextKeys = new Set(currentItems.map(keyFn));

    for (const [key, entry] of entries) {
      if (!nextKeys.has(key)) {
        entry.dispose();
        entries.delete(key);
      }
    }

    const nextOrdered = currentItems.map((item, index) => {
      const key = keyFn(item);
      let entry = entries.get(key);

      if (!entry) {
        const itemSignal = signal(item);
        const indexSignal = signal(index);
        let node!: U;
        const dispose = effect(() => {
          // untrack(): renderItem may read item()/index() synchronously at
          // its own top level (e.g. capturing an id for a closure) — without
          // untrack, that read would make THIS wrapping effect itself track
          // itemSignal/indexSignal, so updating a persisting entry's signal
          // (below, in the reuse branch) would reentrantly re-run renderItem
          // instead of just updating whatever nested binding (insert/setAttr/
          // etc.) explicitly subscribed to it. Nested effect() calls made
          // inside renderItem (e.g. insert()'s own internal effect) are
          // unaffected by this — they establish their own tracking scope
          // regardless of the ambient untrack.
          untrack(() => {
            node = renderItem(() => itemSignal.get(), () => indexSignal.get());
          });
        });
        entry = { itemSignal, indexSignal, node, dispose };
        entries.set(key, entry);
      } else {
        entry.itemSignal.set(item);
        entry.indexSignal.set(index);
      }

      return entry.node;
    });

    ordered.set(nextOrdered);
  }

  reconcile();

  const watcher = new Signal.subtle.Watcher(() => {
    reconcile();
    watcher.watch(itemsView);
  });
  watcher.watch(itemsView);

  return () => ordered.get();
}
