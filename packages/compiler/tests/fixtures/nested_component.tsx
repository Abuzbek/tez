import { signal } from "@tez/signals";

export function Outer() {
  let count = signal(0);
  function Inner() {
    return <span>{count}</span>;
  }
  return <div>{count}</div>;
}
