import { signal, computed } from "@tez/signals";

export function Doubler() {
  let count = signal(1);
  let double = computed(() => count.get() * 2);
  double.set(4);
  return <span>{double}</span>;
}
