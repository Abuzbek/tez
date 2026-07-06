import { signal } from "@tez/signals";
import { template, insert, listen } from "@tez/runtime-dom";

const makeCounterTemplate = template(`<button>Count: <!></button>`);

export function Counter() {
  const count = signal(0);
  const el = makeCounterTemplate() as HTMLButtonElement;
  const marker = el.childNodes[1] as Node;

  insert(el, () => count.get(), marker);
  listen(el, "click", () => count.set(count.get() + 1));

  return el;
}
