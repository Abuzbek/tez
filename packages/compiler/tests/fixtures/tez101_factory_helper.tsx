import { signal } from "@tez/signals";

function makeCounter() {
  let count = signal(0);
  count.set(5);
  function Counter() {
    return <span>{count}</span>;
  }
  return Counter;
}
