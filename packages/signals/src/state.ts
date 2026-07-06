import { trackAccess, type Observer, type Source } from "./graph";
import { markObserversStale } from "./propagate";

export interface StateOptions<T> {
  equals?: (a: T, b: T) => boolean;
}

export class State<T> implements Source {
  private value: T;
  private readonly observers = new Set<Observer>();
  private readonly equals: (a: T, b: T) => boolean;

  constructor(initialValue: T, options?: StateOptions<T>) {
    this.value = initialValue;
    this.equals = options?.equals ?? Object.is;
  }

  get(): T {
    trackAccess(this);
    return this.value;
  }

  set(newValue: T): void {
    if (this.equals(this.value, newValue)) return;
    this.value = newValue;
    markObserversStale(this.observers);
  }

  addObserver(observer: Observer): void {
    this.observers.add(observer);
  }

  removeObserver(observer: Observer): void {
    this.observers.delete(observer);
  }
}
