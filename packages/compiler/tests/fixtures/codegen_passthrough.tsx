import { helper } from "./helpers";

export const answer = 42;

export function Card() {
  return <section><h1>Title</h1><p>body</p></section>;
}

export function plain(x: number) {
  return x + 1;
}
