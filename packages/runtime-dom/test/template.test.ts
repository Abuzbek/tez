import { describe, expect, it } from "vitest";
import { template } from "../src/template";

describe("template", () => {
  it("clones the template's root element on each call", () => {
    const factory = template("<button>Click me</button>");
    const a = factory();
    const b = factory();
    expect(a).not.toBe(b);
    expect(a).toBeInstanceOf(HTMLButtonElement);
    expect((a as HTMLButtonElement).textContent).toBe("Click me");
    expect((b as HTMLButtonElement).textContent).toBe("Click me");
  });

  it("preserves nested structure and marker comment nodes", () => {
    const factory = template("<div><span>Count: <!></span></div>");
    const node = factory() as HTMLDivElement;
    const span = node.querySelector("span");
    expect(span?.childNodes.length).toBe(2);
    expect(span?.childNodes[1]?.nodeType).toBe(Node.COMMENT_NODE);
  });

  it("returns independent clones that can be mutated without affecting each other", () => {
    const factory = template("<p>Hello</p>");
    const a = factory() as HTMLParagraphElement;
    const b = factory() as HTMLParagraphElement;
    a.textContent = "Changed";
    expect(b.textContent).toBe("Hello");
  });

  it("throws when the html has no root element", () => {
    expect(() => template("")).toThrow(/no root element/i);
  });
});
