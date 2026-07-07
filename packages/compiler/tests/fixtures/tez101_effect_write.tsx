import { signal, effect } from "@tez/signals";

export function Logger() {
  let count = signal(0);
  let last = signal(-1);
  effect(() => {
    last.set(count.get());
  });
  return <span>{last}</span>;
}
