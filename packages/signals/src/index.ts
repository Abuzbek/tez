import { State, type StateOptions } from "./state";
import { Computed, type ComputedOptions } from "./computed";
import { Watcher } from "./watcher";

export { batch } from "./batch";
export { untrack } from "./untrack";
export { effect } from "./effect";
export type { EffectCleanup, EffectFn } from "./effect";
export type { StateOptions, ComputedOptions };

export function signal<T>(initialValue: T, options?: StateOptions<T>): State<T> {
  return new State(initialValue, options);
}

export function computed<T>(fn: () => T, options?: ComputedOptions<T>): Computed<T> {
  return new Computed(fn, options);
}

export const Signal = {
  State,
  Computed,
  subtle: {
    Watcher,
  },
};
