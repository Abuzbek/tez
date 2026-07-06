import { effect } from "@tez/signals";

function toNode(value: unknown): Node {
  if (value instanceof Node) return value;
  return document.createTextNode(value == null ? "" : String(value));
}

export function insert(parent: Node, accessor: () => unknown, marker: Node): void {
  let current: Node | null = null;

  effect(() => {
    const next = toNode(accessor());

    if (current) {
      parent.replaceChild(next, current);
    } else {
      parent.insertBefore(next, marker);
    }
    current = next;
  });
}

export function setAttr(el: Element, name: string, accessor: () => unknown): void {
  effect(() => {
    const value = accessor();
    if (value == null || value === false) {
      el.removeAttribute(name);
    } else {
      el.setAttribute(name, String(value));
    }
  });
}

export function toggleClass(el: Element, className: string, accessor: () => boolean): void {
  effect(() => {
    el.classList.toggle(className, Boolean(accessor()));
  });
}
