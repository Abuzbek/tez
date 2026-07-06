import { effect } from "@tez/signals";

export function mount<P>(
  Component: (props: P) => Node,
  target: Element,
  props?: P,
): () => void {
  let node!: Node;

  // Wrapping in effect() is purely for its owner-scope disposal mechanics,
  // not because the component call is meant to be reactive: insert()/
  // setAttr()/toggleClass()/mapArray() calls made during Component(props)
  // create their own nested effects, which need a parent scope to be
  // disposed under when dispose() below is called. Well-formed components
  // don't call .get() directly outside those helpers, so in practice this
  // effect has no tracked dependencies and never re-runs.
  const dispose = effect(() => {
    node = Component(props as P);
  });

  target.appendChild(node);

  return () => {
    dispose();
    (node as ChildNode).remove();
  };
}
