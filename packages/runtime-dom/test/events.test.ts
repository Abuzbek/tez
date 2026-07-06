import { describe, expect, it, vi } from "vitest";
import { listen } from "../src/events";

describe("listen", () => {
  it("invokes the handler when the element is clicked", () => {
    const button = document.createElement("button");
    document.body.appendChild(button);
    const handler = vi.fn();
    listen(button, "click", handler);
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(handler).toHaveBeenCalledOnce();
    button.remove();
  });

  it("delegates from a descendant of the listening element up to the handler", () => {
    const button = document.createElement("button");
    const span = document.createElement("span");
    button.appendChild(span);
    document.body.appendChild(button);
    const handler = vi.fn();
    listen(button, "click", handler);
    span.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(handler).toHaveBeenCalledOnce();
    button.remove();
  });

  it("does not invoke a handler on an unrelated element", () => {
    const button = document.createElement("button");
    const other = document.createElement("div");
    document.body.appendChild(button);
    document.body.appendChild(other);
    const handler = vi.fn();
    listen(button, "click", handler);
    other.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(handler).not.toHaveBeenCalled();
    button.remove();
    other.remove();
  });

  it("supports arbitrary event names via the same $$<name> convention", () => {
    const input = document.createElement("input");
    document.body.appendChild(input);
    const handler = vi.fn();
    listen(input, "input", handler);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    expect(handler).toHaveBeenCalledOnce();
    input.remove();
  });

  it("only registers one document listener per event type across multiple listen() calls", () => {
    const addEventListenerSpy = vi.spyOn(document, "addEventListener");
    const a = document.createElement("button");
    const b = document.createElement("button");
    listen(a, "dblclick", vi.fn());
    listen(b, "dblclick", vi.fn());
    const dblclickCalls = addEventListenerSpy.mock.calls.filter(([type]) => type === "dblclick");
    expect(dblclickCalls.length).toBe(1);
    addEventListenerSpy.mockRestore();
  });
});
