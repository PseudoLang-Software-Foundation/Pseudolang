export type LayoutMode = "vertical" | "horizontal";

const STORAGE_KEY = "pseudolang-layout";

let currentLayout: LayoutMode = loadLayout();
let panelsEl: HTMLElement | null = null;
const onChangeCallbacks: Array<() => void> = [];

function loadLayout(): LayoutMode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "horizontal" || stored === "vertical") return stored;
  } catch {
    // localStorage unavailable
  }
  return "vertical";
}

function saveLayout(mode: LayoutMode): void {
  try {
    localStorage.setItem(STORAGE_KEY, mode);
  } catch {
    // localStorage unavailable
  }
}

export function getLayout(): LayoutMode {
  return currentLayout;
}

export function toggleLayout(): void {
  currentLayout = currentLayout === "vertical" ? "horizontal" : "vertical";
  saveLayout(currentLayout);
  applyLayout();
  for (const cb of onChangeCallbacks) cb();
}

export function onLayoutChange(callback: () => void): void {
  onChangeCallbacks.push(callback);
}

export function initLayout(el: HTMLElement): void {
  panelsEl = el;
  applyLayout();
}

function applyLayout(): void {
  if (!panelsEl) return;
  panelsEl.classList.remove("layout-vertical", "layout-horizontal");
  panelsEl.classList.add(`layout-${currentLayout}`);
}
