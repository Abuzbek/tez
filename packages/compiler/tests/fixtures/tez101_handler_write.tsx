import { signal } from "@tez/signals";

export function Clicker() {
  let count = signal(0);
  return <button onClick={() => count.set(count.get() + 1)}>{count}</button>;
}
