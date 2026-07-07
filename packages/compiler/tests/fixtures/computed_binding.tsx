import { signal, computed } from "@tez/signals";

export function Doubler() {
  let count = signal(1);
  let double = computed(() => count * 2);
  return <span>{double}</span>;
}
