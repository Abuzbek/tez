import { signal } from "@tez/signals";

export function Outer() {
  let count = signal(0);
  function Inner() {
    let inner = signal(0);
    inner.set(1);
    return <span>{inner}</span>;
  }
  return <div>{count}</div>;
}
