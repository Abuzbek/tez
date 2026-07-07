import { signal } from "@tez/signals";

export function Weird() {
  let count = signal(0);
  return <div>{count.set(1)}</div>;
}
