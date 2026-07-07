import { signal } from "@tez/signals";

export function Counter() {
  let count = signal(0);
  count.set(1);
  return <span>{count}</span>;
}
