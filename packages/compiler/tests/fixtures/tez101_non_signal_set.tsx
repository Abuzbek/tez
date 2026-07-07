export function Tags() {
  let tags = new Map<string, string>();
  tags.set("color", "red");
  return <span>{tags.size}</span>;
}
