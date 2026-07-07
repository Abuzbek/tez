import { signal as sig } from "@tez/signals";

export function Aliased() {
  let count = sig(0);
  count.set(1);
  return <span>{count}</span>;
}
