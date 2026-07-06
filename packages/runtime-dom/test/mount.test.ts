import { describe, expect, it, vi } from "vitest";
import { signal } from "@tez/signals";
import { mount } from "../src/mount";
import { insert } from "../src/bindings";

describe("mount", () => {
  it("calls the component once and appends its returned node to the target", () => {
    const componentFn = vi.fn(() => document.createElement("div"));
    const target = document.createElement("main");
    mount(componentFn, target);
    expect(componentFn).toHaveBeenCalledOnce();
    expect(target.children.length).toBe(1);
  });

  it("passes props through to the component", () => {
    const target = document.createElement("main");
    mount(
      (props: { label: string }) => {
        const el = document.createElement("span");
        el.textContent = props.label;
        return el;
      },
      target,
      { label: "hi" },
    );
    expect(target.textContent).toBe("hi");
  });

  it("supports nested reactive bindings created during the component call", () => {
    const count = signal(1);
    const target = document.createElement("main");
    mount(() => {
      const el = document.createElement("span");
      const marker = document.createComment("");
      el.appendChild(marker);
      insert(el, () => count.get(), marker);
      return el;
    }, target);
    expect(target.textContent).toBe("1");
    count.set(2);
    expect(target.textContent).toBe("2");
  });

  it("dispose() removes the mounted node from the DOM", () => {
    const target = document.createElement("main");
    const dispose = mount(() => document.createElement("div"), target);
    expect(target.children.length).toBe(1);
    dispose();
    expect(target.children.length).toBe(0);
  });

  it("dispose() tears down nested bindings created inside the component", () => {
    const count = signal(1);
    const removeObserverSpy = vi.spyOn(count, "removeObserver");
    const target = document.createElement("main");
    const dispose = mount(() => {
      const el = document.createElement("span");
      const marker = document.createComment("");
      el.appendChild(marker);
      insert(el, () => count.get(), marker);
      return el;
    }, target);
    dispose();
    expect(removeObserverSpy).toHaveBeenCalled();
  });
});
