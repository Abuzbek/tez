import { signal } from "@tez/signals";

export function Labeled() {
  let count = signal(0);
  let label = "Count:";
  return (
    <div>
      <span>{label}</span>
      <span>{count}</span>
    </div>
  );
}
