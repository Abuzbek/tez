import { describe, expect, it } from "vitest";
import { signal } from "@tez/signals";
import { insert, setAttr, toggleClass } from "../src/bindings";

describe("insert", () => {
  it("inserts the accessor's initial value as text before the marker", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    insert(parent, () => "hello", marker);
    expect(parent.textContent).toBe("hello");
    expect(parent.lastChild).toBe(marker);
  });

  it("updates the DOM when the accessor's signal changes", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    const count = signal(1);
    insert(parent, () => count.get(), marker);
    expect(parent.textContent).toBe("1");
    count.set(2);
    expect(parent.textContent).toBe("2");
    expect(parent.childNodes.length).toBe(2);
  });

  it("inserts a real Node value directly instead of stringifying it", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    const span = document.createElement("span");
    span.textContent = "child";
    insert(parent, () => span, marker);
    expect(parent.querySelector("span")).toBe(span);
  });

  it("renders null and undefined as empty text", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    insert(parent, () => null, marker);
    expect(parent.textContent).toBe("");
  });
});

describe("setAttr", () => {
  it("sets the attribute to the accessor's initial value", () => {
    const el = document.createElement("input");
    setAttr(el, "placeholder", () => "name");
    expect(el.getAttribute("placeholder")).toBe("name");
  });

  it("updates the attribute reactively", () => {
    const el = document.createElement("input");
    const value = signal("a");
    setAttr(el, "placeholder", () => value.get());
    value.set("b");
    expect(el.getAttribute("placeholder")).toBe("b");
  });

  it("removes the attribute when the value is null or false", () => {
    const el = document.createElement("input");
    const disabled = signal(true);
    setAttr(el, "disabled", () => disabled.get());
    expect(el.getAttribute("disabled")).toBe("true");
    disabled.set(false);
    expect(el.hasAttribute("disabled")).toBe(false);
  });
});

describe("toggleClass", () => {
  it("adds the class when the accessor is true", () => {
    const el = document.createElement("li");
    toggleClass(el, "done", () => true);
    expect(el.classList.contains("done")).toBe(true);
  });

  it("removes the class reactively when the accessor becomes false", () => {
    const el = document.createElement("li");
    const done = signal(true);
    toggleClass(el, "done", () => done.get());
    done.set(false);
    expect(el.classList.contains("done")).toBe(false);
  });
});
