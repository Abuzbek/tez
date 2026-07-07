import { signal } from "@tez/signals";

let count = signal(0);

function reset() {
  count.set(0);
}

export function Display() {
  return <span>{count}</span>;
}
