const FORM_SELECTOR = "form[data-pending]";
const GUARD_SELECTOR = "form[data-dirty-guard]";
const PROGRESS_DELAY = 150;
const ANNOUNCE_DELAY = 500;
const LEAVE_PROMPT = "You have unsaved changes. Leave without saving them?";

const originals = new WeakMap();
const pendingTexts = new WeakMap();
const disabledByScript = new WeakSet();
const busyByScript = new WeakSet();
const all = (selector, root = document) => root.querySelectorAll(selector);

let leaveApproved = false;
let sent = null;
let progressTimer;

function submitButtons(form) {
  return Array.from(form.elements).filter(
    (el) => (el instanceof HTMLButtonElement || el instanceof HTMLInputElement) && el.type === "submit",
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

function swapTexts(form, busy) {
  for (const el of all("[data-pending-text]", form)) {
    if (busy && !pendingTexts.has(el)) {
      pendingTexts.set(el, el.textContent);
      el.textContent = el.dataset.pendingText;
    } else if (!busy && pendingTexts.has(el)) {
      el.textContent = pendingTexts.get(el);
      pendingTexts.delete(el);
    }
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
  swapTexts(form, busy);
  if (busy) {
    form.setAttribute("aria-busy", "true");
  } else {
    form.removeAttribute("aria-busy");
  }
}

function controlDiffers(el) {
  if (el instanceof HTMLInputElement) {
    if (el.type === "checkbox" || el.type === "radio") {
      return el.checked !== el.defaultChecked;
    }
    if (["hidden", "submit", "button", "reset", "image", "file"].includes(el.type)) {
      return false;
    }
    return el.value !== el.defaultValue;
  }
  if (el instanceof HTMLTextAreaElement) return el.value !== el.defaultValue;
  if (el instanceof HTMLSelectElement) {
    const options = Array.from(el.options);
    if (el.multiple) return options.some((o) => o.selected !== o.defaultSelected);
    const initial = options.findIndex((o) => o.defaultSelected);
    return el.selectedIndex !== (initial === -1 ? 0 : initial);
  }
  return false;
}

function isDirty(form) {
  return Array.from(form.elements).some(controlDiffers);
}

function refreshDirty(form) {
  const dirty = isDirty(form);
  form.toggleAttribute("data-dirty", dirty);
  for (const el of all("[data-dirty-status], [data-discard]", form)) el.hidden = !dirty;
  for (const bar of all("[data-save-bar]", form)) bar.toggleAttribute("data-dirty", dirty);
}

function guardedForm(node) {
  const form = node instanceof Element && node.closest("form");
  return form && form.matches(GUARD_SELECTOR) ? form : null;
}

function dirtyBesides(except) {
  return Array.from(all(GUARD_SELECTOR)).some((form) => form !== except && form !== sent && isDirty(form));
}

// The approval also answers the beforeunload that the navigation fires.
function confirmLeave(except) {
  leaveApproved = !dirtyBesides(except) || window.confirm(LEAVE_PROMPT);
  return leaveApproved;
}

function startProgress() {
  clearTimeout(progressTimer);
  progressTimer = setTimeout(() => {
    for (const bar of all("[data-nav-progress]")) bar.setAttribute("data-active", "");
  }, PROGRESS_DELAY);
}

function stopProgress() {
  clearTimeout(progressTimer);
  for (const bar of all("[data-nav-progress]")) bar.removeAttribute("data-active");
}

function leavesBy(link, event) {
  if (event.defaultPrevented || event.button !== 0) return false;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return false;
  if (link.hasAttribute("download") || (link.target && link.target !== "_self")) return false;
  const url = new URL(link.href, location.href);
  if (url.protocol !== "http:" && url.protocol !== "https:") return false;
  return !(url.origin === location.origin && url.pathname === location.pathname &&
    url.search === location.search && url.href.includes("#"));
}

function release() {
  leaveApproved = false;
  sent = null;
  stopProgress();
}

function canModal(dialog) {
  return dialog instanceof HTMLDialogElement && typeof dialog.showModal === "function";
}

function openDialog(event, dialog) {
  if (!canModal(dialog)) return false;
  event.preventDefault();
  if (!dialog.open) dialog.showModal();
  return true;
}

document.addEventListener("submit", (event) => {
  const form = event.target;
  if (!(form instanceof HTMLFormElement)) return;
  const pending = form.matches(FORM_SELECTOR) ? form : null;
  if (pending?.hasAttribute("aria-busy")) {
    event.preventDefault();
    return;
  }
  if (event.defaultPrevented) return;
  const submitter = event.submitter;
  const method = submitter?.formMethod || form.method;
  const target = submitter?.formTarget || form.target;
  if (method !== "dialog" && (!target || target === "_self")) {
    if (!confirmLeave(form)) {
      event.preventDefault();
      return;
    }
    sent = form;
    startProgress();
  }
  pending?.setAttribute("aria-busy", "true");
  setTimeout(() => {
    if (!event.defaultPrevented) {
      if (pending) setBusy(pending, true);
      return;
    }
    release();
    pending?.removeAttribute("aria-busy");
  }, 0);
});

window.addEventListener("beforeunload", (event) => {
  if (leaveApproved) {
    leaveApproved = false;
    return;
  }
  if (!dirtyBesides(null)) return;
  event.preventDefault();
  event.returnValue = "";
});

window.addEventListener("pageshow", (event) => {
  for (const form of all(GUARD_SELECTOR)) refreshDirty(form);
  if (!event.persisted) return;
  release();
  for (const form of all(FORM_SELECTOR)) setBusy(form, false);
});

for (const type of ["input", "change"]) {
  document.addEventListener(type, (event) => {
    const el = event.target;
    const form = guardedForm(el);
    if (form) refreshDirty(form);
    if (el instanceof HTMLInputElement && el.matches("[data-filter-input]")) applyFilter(el);
  });
}

document.addEventListener("reset", (event) => {
  const form = guardedForm(event.target);
  if (form) setTimeout(() => refreshDirty(form), 0);
});

document.addEventListener("click", (event) => {
  const target = event.target;
  if (!(target instanceof Element)) return;

  const discard = target.closest("[data-discard]");
  const discarded = guardedForm(discard);
  if (discarded) {
    discarded.reset();
    refreshDirty(discarded);
    return;
  }

  const trigger = target.closest("[data-confirm-trigger]");
  if (trigger && !trigger.disabled &&
      openDialog(event, trigger.closest("[data-confirm]")?.querySelector("dialog"))) {
    return;
  }

  const opener = target.closest("[data-dialog-open]");
  if (opener) {
    openDialog(event, document.getElementById(opener.dataset.dialogOpen));
    return;
  }

  const closer = target.closest("[data-dialog-close]");
  if (closer) {
    closer.closest("dialog")?.close();
    return;
  }

  if (target instanceof HTMLDialogElement && target.open) {
    target.close();
    return;
  }

  const link = target.closest("a[href]");
  if (!link) return;
  if (leavesBy(link, event)) {
    if (!confirmLeave(null)) {
      event.preventDefault();
      return;
    }
    startProgress();
    setTimeout(() => event.defaultPrevented && release(), 0);
  }
  const panel = link.closest("[popover]");
  if (panel && typeof panel.hidePopover === "function") {
    try {
      panel.hidePopover();
    } catch {}
  }
});

document.addEventListener(
  "toggle",
  (event) => {
    const panel = event.target;
    if (!(panel instanceof HTMLElement) || !panel.hasAttribute("popover") || !panel.id) {
      return;
    }
    const open = event.newState === "open";
    for (const invoker of all("[popovertarget]")) {
      if (invoker.getAttribute("popovertarget") === panel.id &&
          invoker.getAttribute("popovertargetaction") !== "hide") {
        invoker.setAttribute("aria-expanded", open ? "true" : "false");
      }
    }
    if (!open) {
      for (const input of all("[data-filter-input]", panel)) {
        input.value = "";
        applyFilter(input);
      }
    }
  },
  true,
);

function applyFilter(input) {
  const root = input.closest("[data-filter]");
  if (!root) return;
  const query = input.value.trim().toLowerCase();
  const items = Array.from(all("[data-filter-item]", root));
  let shown = 0;
  for (const item of items) {
    const match = query === "" || item.textContent.toLowerCase().includes(query);
    item.hidden = !match;
    if (match) shown += 1;
  }
  const count = root.querySelector("[data-filter-count]");
  if (count) {
    const noun = root.dataset.filterNoun || "results";
    count.textContent = query === "" ? "" : `${shown} of ${items.length} ${noun}`;
  }
}

for (const form of all(GUARD_SELECTOR)) refreshDirty(form);

// A live region does not announce what it holds at load; a fresh copy does.
setTimeout(() => {
  for (const note of all("[role=status] > [data-flash]")) note.replaceWith(note.cloneNode(true));
}, ANNOUNCE_DELAY);
