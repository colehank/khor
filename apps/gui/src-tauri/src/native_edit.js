(() => {
  const at = document.activeElement;
  // Search boxes and ordinary inputs within an editor keep native undo.
  if (!(at instanceof HTMLElement) || !at.matches('.cm-content') ||
      !at.isContentEditable || !at.closest('[data-code-view]')) return false;
  const event = new InputEvent('beforeinput', {
    inputType: '__KHOR_HISTORY_INPUT__', bubbles: true, cancelable: true,
  });
  at.dispatchEvent(event);
  return event.defaultPrevented;
})()
