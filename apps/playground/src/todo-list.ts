import { signal, effect } from "@tez/signals";
import { template, insert, setAttr, toggleClass, listen, mapArray } from "@tez/runtime-dom";

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

let nextId = 1;

const makeRootTemplate = template(`<div><ul></ul><button>Add</button></div>`);
const makeItemTemplate = template(
  `<li><input type="checkbox" /><span><!></span><button>Remove</button></li>`,
);

export function TodoList() {
  const todos = signal<Todo[]>([
    { id: nextId++, text: "Write the compiler", done: false },
    { id: nextId++, text: "Ship Phase 1", done: false },
  ]);

  const root = makeRootTemplate() as HTMLDivElement;
  const list = root.querySelector("ul")!;
  const addButton = root.querySelector("button")!;

  const getNodes = mapArray(
    () => todos.get(),
    (todo) => todo.id,
    (todo) => {
      const item = makeItemTemplate() as HTMLLIElement;
      const checkbox = item.querySelector("input")!;
      const label = item.querySelector("span")!;
      const marker = label.childNodes[0] as Node;
      const removeButton = item.querySelectorAll("button")[0]!;

      insert(label, () => todo().text, marker);
      toggleClass(item, "done", () => todo().done);
      setAttr(removeButton, "aria-label", () => `Remove ${todo().text}`);

      // checkbox.checked must be set as a DOM property, not an attribute:
      // once a user has interacted with a checkbox, browsers stop syncing
      // its live checked state from the "checked" attribute.
      effect(() => {
        checkbox.checked = todo().done;
      });

      listen(checkbox, "change", () => {
        const id = todo().id;
        todos.set(todos.get().map((t) => (t.id === id ? { ...t, done: !t.done } : t)));
      });

      listen(removeButton, "click", () => {
        const id = todo().id;
        todos.set(todos.get().filter((t) => t.id !== id));
      });

      return item;
    },
  );

  effect(() => {
    list.replaceChildren(...getNodes());
  });

  listen(addButton, "click", () => {
    const id = nextId++;
    todos.set([...todos.get(), { id, text: `New todo ${id}`, done: false }]);
  });

  return root;
}
