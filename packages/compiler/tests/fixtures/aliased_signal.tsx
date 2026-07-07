import { signal as sig } from "@tez/signals";

export function AliasedCounter() {
  let count = sig(0);
  return <span>{count}</span>;
}
