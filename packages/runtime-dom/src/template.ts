export function template(html: string): () => Node {
  const templateEl = document.createElement("template");
  templateEl.innerHTML = html;
  const root = templateEl.content.firstChild;

  if (!root) {
    throw new Error(`template(): no root element found in: ${html}`);
  }

  return () => root.cloneNode(true) as Node;
}
