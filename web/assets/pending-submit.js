const FORM_SELECTOR = "form[data-pending]";
const originals = new WeakMap();
const disabledByScript = new WeakSet();
const busyByScript = new WeakSet();

function submitButtons(form) {
  return Array.from(form.elements).filter(
    (el) =>
      (el instanceof HTMLButtonElement && el.type === "submit") ||
      (el instanceof HTMLInputElement && el.type === "submit"),
  );
}

function swapLabel(button) {
  const label = button.dataset.pendingLabel;
  if (label === undefined || originals.has(button)) return;
  if (button instanceof HTMLInputElement) {
    originals.set(button, button.value);
    button.value = label;
  } else {
    originals.set(button, Array.from(button.childNodes));
    button.replaceChildren(document.createTextNode(label));
  }
}

function restoreLabel(button) {
  if (!originals.has(button)) return;
  const original = originals.get(button);
  originals.delete(button);
  if (button instanceof HTMLInputElement) {
    button.value = original;
  } else {
    button.replaceChildren(...original);
  }
}

function markBusy(button) {
  if (!button.disabled) {
    button.disabled = true;
    disabledByScript.add(button);
  }
  if (!button.hasAttribute("aria-busy")) {
    button.setAttribute("aria-busy", "true");
    busyByScript.add(button);
  }
  swapLabel(button);
}

function clearBusy(button) {
  if (disabledByScript.delete(button)) button.disabled = false;
  if (busyByScript.delete(button)) button.removeAttribute("aria-busy");
  restoreLabel(button);
}

function setBusy(form, busy) {
  for (const button of submitButtons(form)) {
    if (busy) {
      markBusy(button);
    } else {
      clearBusy(button);
    }
  }
  if (busy) {
    form.setAttribute("aria-busy", "true");
  } else {
    form.removeAttribute("aria-busy");
  }
}

document.addEventListener("submit", (event) => {
  const form = event.target;
  if (!(form instanceof HTMLFormElement) || !form.matches(FORM_SELECTOR)) return;
  if (form.hasAttribute("aria-busy")) {
    event.preventDefault();
    return;
  }
  if (event.defaultPrevented) return;
  form.setAttribute("aria-busy", "true");
  setTimeout(() => {
    if (event.defaultPrevented) {
      form.removeAttribute("aria-busy");
      return;
    }
    setBusy(form, true);
  }, 0);
});

window.addEventListener("pageshow", (event) => {
  if (!event.persisted) return;
  for (const form of document.querySelectorAll(FORM_SELECTOR)) setBusy(form, false);
});
