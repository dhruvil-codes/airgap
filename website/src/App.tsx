import { useEffect, type ComponentType } from "react";
import { Tooltip } from "@base-ui/react/tooltip";
import { LazyMotion, domAnimation } from "motion/react";
import { Header } from "./sections/Header";
import { Footer } from "./sections/Footer";
import GradualBlur from "@/components/GradualBlur";
import { scrollToTarget } from "./smooth";
import { BASE } from "./content";

export const routeOf = (pathname: string) => pathname.slice(BASE.length).replace(/\/+$/, "").replace(/\/index\.html$/, "") || "/";

export default function App({
  Page,
  title,
  missing = false,
  bare = false,
}: {
  Page: ComponentType;
  title?: string;
  missing?: boolean;
  bare?: boolean;
}) {
  useEffect(() => {
    if (title) document.title = `${title} · airgap`;
    else if (missing) document.title = "Page not found · airgap";
    // Content mounts after the browser's own hash jump, so retry it once React has rendered.
    const id = decodeURIComponent(window.location.hash.slice(1));
    const el = id ? document.getElementById(id) : null;
    if (el) requestAnimationFrame(() => scrollToTarget(el, true));
  }, [title, missing]);

  if (bare) {
    return (
      <LazyMotion features={domAnimation} strict>
        <Page />
      </LazyMotion>
    );
  }
  return (
    <LazyMotion features={domAnimation} strict>
      <Tooltip.Provider delay={250}>
        <a className="skip" href="#main">
          Skip to content
        </a>
        <Header />
        <main id="main">
          <Page />
        </main>
        <Footer />
        <GradualBlur target="page" position="bottom" height="4.5rem" strength={1.6} layers={6} curve="bezier" autoHide zIndex={30} />
      </Tooltip.Provider>
    </LazyMotion>
  );
}
