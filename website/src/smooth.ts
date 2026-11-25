// Native browser scrolling (honours CSS scroll-behavior: smooth, scroll-padding-top and scroll-margin-top).
export function scrollToTarget(target: number | string | Element, immediate = false) {
  const behavior: ScrollBehavior = immediate ? "auto" : "smooth";
  if (typeof target === "number") {
    window.scrollTo({ top: target, behavior });
    return;
  }
  const el = typeof target === "string" ? document.querySelector(target) : target;
  el?.scrollIntoView({ behavior });
}
