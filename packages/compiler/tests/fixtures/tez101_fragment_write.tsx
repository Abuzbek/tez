import { signal } from "@tez/signals";

function Pair() {
  let count = signal(0);
  count.set(1);
  return <>{count}</>;
}
