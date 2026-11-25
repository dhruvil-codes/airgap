import type Lenis from "lenis";

let instance: Lenis | null = null;
let loading: Promise<Lenis | null> | null = null;

export function initSmooth(): Promise<Lenis | null> {
  if (typeof window === "undefined" || new URLSearchParams(window.location.search).has("og")) {
    return Promise.resolve(null);
  }
  if (instance) return Promise.resolve(instance);
  if (!loading) {
    loading = Promise.all([import("lenis"), import("lenis/dist/lenis.css")]).then(([{ default: LenisCtor }]) => {
      instance = new LenisCtor({
        autoRaf: true,
        lerp: 0.1,
        anchors: true,
        allowNestedScroll: true,
        stopInertiaOnNavigate: true,
      });
      return instance;
    });
  }
  return loading;
}

if (typeof window !== "undefined" && !new URLSearchParams(window.location.search).has("og")) {
  const boot = () => {
    void initSmooth();
  };
  if (typeof window.requestIdleCallback === "function") {
    window.requestIdleCallback(boot, { timeout: 1500 });
  } else {
    globalThis.setTimeout(boot, 120);
  }
}

export function scrollToTarget(target: number | string | Element, immediate = false) {
  if (instance) {
    // The page may have just rendered, so refresh Lenis's scroll limit before it clamps the target.
    instance.resize();
    instance.scrollTo(target, { immediate });
  } else if (typeof target === "number") window.scrollTo({ top: target, behavior: immediate ? "auto" : "smooth" });
  else (typeof target === "string" ? document.querySelector(target) : target)?.scrollIntoView();
}
