import { signal } from "@tez/signals";

export function Counter(props: { start: number }) {
  let count = signal(props.start);
  return (
    <button onClick={() => count++}>{count}</button>
  );
}
