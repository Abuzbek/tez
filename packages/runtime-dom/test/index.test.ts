import { describe, expect, it } from "vitest";
import { signal } from "@tez/signals";
import { template, insert, setAttr, toggleClass, listen, mapArray, mount } from "../src/index";

describe("public API", () => {
  it("re-exports every primitive from a single entry point", () => {
    expect(typeof template).toBe("function");
    expect(typeof insert).toBe("function");
    expect(typeof setAttr).toBe("function");
    expect(typeof toggleClass).toBe("function");
    expect(typeof listen).toBe("function");
    expect(typeof mapArray).toBe("function");
    expect(typeof mount).toBe("function");
  });

  it("composes template + insert + listen + mount into a working counter", () => {
    const makeCounter = template("<button>Count: <!></button>");

    function Counter() {
      const count = signal(0);
      const el = makeCounter() as HTMLButtonElement;
      const marker = el.childNodes[1] as Node;
      insert(el, () => count.get(), marker);
      listen(el, "click", () => count.set(count.get() + 1));
      return el;
    }

    const target = document.createElement("main");
    document.body.appendChild(target);
    mount(Counter, target);

    const button = target.querySelector("button")!;
    expect(button.textContent).toBe("Count: 0");
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(button.textContent).toBe("Count: 1");
    target.remove();
  });
});
