import Lenis from "lenis";
import "lenis/dist/lenis.css";

// Anchor offsets come from CSS: Lenis honours scroll-padding-top on <html> and scroll-margin-top on targets.
export const lenis =
  typeof window !== "undefined" && !new URLSearchParams(window.location.search).has("og")
    ? new Lenis({
        autoRaf: true,
        lerp: 0.1,
        anchors: true,
        allowNestedScroll: true,
        stopInertiaOnNavigate: true,
      })
    : null;

export function scrollToTarget(target: number | string | Element, immediate = false) {
  if (lenis) {
    // The page may have just rendered, so refresh Lenis's scroll limit before it clamps the target.
    lenis.resize();
    lenis.scrollTo(target, { immediate });
  } else if (typeof target === "number") window.scrollTo({ top: target, behavior: immediate ? "auto" : "smooth" });
  else (typeof target === "string" ? document.querySelector(target) : target)?.scrollIntoView();
}
