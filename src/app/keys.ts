const TYPES = ['keydown', 'keyup', 'keypress'];

// Typed as a plain Event, so it can't read which key was pressed. Cancelling the default keeps Tab,
// Space, Enter and the arrows from moving focus, clicking or scrolling while a test runs.
function cancel(event: Event): void {
  event.preventDefault();
}

export function cancelKeys(): () => void {
  for (const type of TYPES) window.addEventListener(type, cancel, true);
  return () => {
    for (const type of TYPES) window.removeEventListener(type, cancel, true);
  };
}

// A focused button would take Space or Enter, so nothing keeps focus while a test runs.
export function dropFocus(): void {
  const active = document.activeElement;
  if (active instanceof HTMLElement) active.blur();
}
