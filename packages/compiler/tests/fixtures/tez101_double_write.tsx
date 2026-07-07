import { signal } from "@tez/signals";

export function Doubled() {
  let a = signal(0);
  let b = signal(0);
  a.set(1);
  b.set(2);
  return (
    <span>
      {a}
      {b}
    </span>
  );
}
