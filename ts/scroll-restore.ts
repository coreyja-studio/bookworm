// Preserve window.scrollY across same-pathname form-POST navigations.
// Falls back to snap-to-top on browsers without pageswap/pagereveal support.

const KEY_PREFIX = "bw:scroll:";
const TTL_MS = 5_000;
const DEBUG = location.search.includes("debug=scroll");

let pendingSubmit = false;

document.addEventListener("submit", (e) => {
  if (e.defaultPrevented) return;
  const form = e.target;
  if (form instanceof HTMLFormElement && form.method.toLowerCase() === "post") {
    pendingSubmit = true;
  }
});

window.addEventListener("pageswap", (e) => {
  if (DEBUG)
    console.debug("[scroll-restore] pageswap fired", {
      pendingSubmit,
      scrollY: window.scrollY,
    });
  if (!pendingSubmit) return;
  pendingSubmit = false;

  const swap = e as PageSwapEvent;
  const targetUrl = swap.activation?.entry?.url;
  if (typeof targetUrl === "string" && targetUrl.length > 0) {
    try {
      const target = new URL(targetUrl);
      if (target.pathname !== location.pathname) {
        if (DEBUG)
          console.debug("[scroll-restore] skipping save (cross-pathname target)", {
            from: location.pathname,
            to: target.pathname,
          });
        return;
      }
    } catch {
      // Malformed target URL — fall through, TTL is the safety net.
    }
  }

  try {
    const key = KEY_PREFIX + location.pathname;
    const payload = JSON.stringify({ y: window.scrollY, t: Date.now() });
    sessionStorage.setItem(key, payload);
    if (DEBUG) console.debug("[scroll-restore] saved", { key, payload });
  } catch {
    // sessionStorage may throw in private mode or on quota errors.
  }
});

window.addEventListener("pagereveal", () => {
  const navEntry = performance.getEntriesByType("navigation")[0] as
    | PerformanceNavigationTiming
    | undefined;
  if (DEBUG)
    console.debug("[scroll-restore] pagereveal fired", {
      navType: navEntry?.type,
      pathname: location.pathname,
    });
  if (navEntry?.type !== "navigate") return;

  const key = KEY_PREFIX + location.pathname;
  let raw: string | null = null;
  try {
    raw = sessionStorage.getItem(key);
    sessionStorage.removeItem(key); // single-shot regardless of validity
  } catch {
    return;
  }
  if (!raw) return;

  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return;
    const { y, t } = parsed as { y?: unknown; t?: unknown };
    if (typeof y !== "number" || typeof t !== "number") return;
    if (Date.now() - t > TTL_MS) return;
    window.scrollTo(0, y);
    if (DEBUG) console.debug("[scroll-restore] restored", { key, y, age: Date.now() - t });
  } catch {
    // JSON.parse failure — entry already removed above.
  }
});
