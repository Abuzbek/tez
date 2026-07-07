import { signal } from "@tez/signals";

export function Gate(props: { reset: boolean }) {
  let count = signal(0);
  if (props.reset) {
    count.set(0);
  }
  return <span>{count}</span>;
}
