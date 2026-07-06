const registeredEventTypes = new Set<string>();

export function listen(el: Element, eventName: string, handler: (ev: Event) => void): void {
  (el as unknown as Record<string, unknown>)[`$$${eventName}`] = handler;

  if (registeredEventTypes.has(eventName)) return;
  registeredEventTypes.add(eventName);

  document.addEventListener(eventName, (event) => {
    const prop = `$$${eventName}`;
    let node: Node | null = event.target as Node | null;

    while (node) {
      const handlerOnNode = (node as unknown as Record<string, unknown>)[prop];
      if (typeof handlerOnNode === "function") {
        (handlerOnNode as (ev: Event) => void)(event);
        return;
      }
      node = node.parentNode;
    }
  });
}
