export interface Runnable {
  run(): void;
}

let batchDepth = 0;
const pendingEffects = new Set<Runnable>();

export function batch<T>(fn: () => T): T {
  batchDepth++;
  try {
    return fn();
  } finally {
    batchDepth--;
    if (batchDepth === 0) flushEffects();
  }
}

export function scheduleEffect(effect: Runnable): void {
  pendingEffects.add(effect);
  if (batchDepth === 0) flushEffects();
}

function flushEffects(): void {
  if (pendingEffects.size === 0) return;
  const effects = Array.from(pendingEffects);
  pendingEffects.clear();
  for (const effect of effects) {
    effect.run();
  }
}
