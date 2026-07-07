function signal(x: number): number {
  return x;
}

export function NotReallyReactive() {
  let count = signal(5);
  return <span>{count}</span>;
}
