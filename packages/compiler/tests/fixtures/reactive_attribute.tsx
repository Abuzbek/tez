import { signal } from "@tez/signals";

export function ToggleButton() {
  let count = signal(0);
  let isDisabled = signal(false);
  return (
    <button disabled={isDisabled} onClick={() => count++}>
      {count}
    </button>
  );
}
