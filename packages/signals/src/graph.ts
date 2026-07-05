export interface Observer {
  notify(): void;
}

export interface Source {
  addObserver(observer: Observer): void;
  removeObserver(observer: Observer): void;
}

export interface TrackingObserver extends Observer {
  addSource(source: Source): void;
}

let currentConsumer: TrackingObserver | null = null;

export function getCurrentConsumer(): TrackingObserver | null {
  return currentConsumer;
}

export function withTracking<T>(consumer: TrackingObserver, fn: () => T): T {
  const previous = currentConsumer;
  currentConsumer = consumer;
  try {
    return fn();
  } finally {
    currentConsumer = previous;
  }
}

export function withoutTracking<T>(fn: () => T): T {
  const previous = currentConsumer;
  currentConsumer = null;
  try {
    return fn();
  } finally {
    currentConsumer = previous;
  }
}

export function trackAccess(source: Source): void {
  if (currentConsumer) {
    currentConsumer.addSource(source);
  }
}
