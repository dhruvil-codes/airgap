import { useEffect, useState } from "react";
import { NavigationMenu } from "@base-ui/react/navigation-menu";
import { Dialog } from "@base-ui/react/dialog";
import { ArrowUpRight, Menu, Star, X } from "lucide-react";
import { LINKS, PRODUCT_NAV, RESOURCE_NAV, type NavLink, at } from "../content";
import { GithubIcon, Mark, ScrollProgress, cx } from "../lib";

function MenuLinks({ items }: { items: NavLink[] }) {
  return (
    <ul className="nav-stack">
      {items.map((item) => {
        const Icon = item.icon;
        return (
          <li key={item.href}>
            <NavigationMenu.Link
              className="nav-card"
              href={item.href}
              {...(item.external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
            >
              <span className="nav-card-icon">
                <Icon size={20} strokeWidth={1.5} />
              </span>
              <span className="nav-card-copy">
                <span className="nav-card-title">
                  {item.title}
                  {item.external && <ArrowUpRight size={13} strokeWidth={1.8} />}
                </span>
                <span className="nav-card-desc">{item.desc}</span>
              </span>
            </NavigationMenu.Link>
          </li>
        );
      })}
    </ul>
  );
}

export function Header() {
  const [scrolled, setScrolled] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header className={cx("site-header", scrolled && "is-scrolled")}>
      <div className="header-inner">
        <a className="brand" href={at(at("/#top"))} aria-label="airgap home">
          <Mark size={18} />
          <span className="brand-name">airgap</span>
        </a>

        <NavigationMenu.Root className="nav" delay={0} closeDelay={220}>
          <NavigationMenu.List className="nav-list">
            <NavigationMenu.Item>
              <NavigationMenu.Trigger className="nav-trigger">
                Products
              </NavigationMenu.Trigger>
              <NavigationMenu.Content className="nav-content">
                <MenuLinks items={PRODUCT_NAV} />
              </NavigationMenu.Content>
            </NavigationMenu.Item>
            <NavigationMenu.Item>
              <NavigationMenu.Link className="nav-trigger" href={at(at("/docs"))}>
                Docs
              </NavigationMenu.Link>
            </NavigationMenu.Item>
            <NavigationMenu.Item>
              <NavigationMenu.Trigger className="nav-trigger">
                Resources
              </NavigationMenu.Trigger>
              <NavigationMenu.Content className="nav-content">
                <MenuLinks items={RESOURCE_NAV} />
              </NavigationMenu.Content>
            </NavigationMenu.Item>
            <NavigationMenu.Item>
              <NavigationMenu.Link className="nav-trigger" href={at(at("/#faq"))}>
                FAQ
              </NavigationMenu.Link>
            </NavigationMenu.Item>
          </NavigationMenu.List>
          <NavigationMenu.Portal>
            <NavigationMenu.Positioner className="nav-positioner" sideOffset={0} collisionPadding={16}>
              <div className="nav-bridge">
                <NavigationMenu.Popup className="nav-popup">
                  <NavigationMenu.Viewport className="nav-viewport" />
                </NavigationMenu.Popup>
              </div>
            </NavigationMenu.Positioner>
          </NavigationMenu.Portal>
        </NavigationMenu.Root>

        <div className="header-actions">
          <a className="nav-action hide-sm" href={LINKS.repo} target="_blank" rel="noopener noreferrer">
            <Star size={15} strokeWidth={1.8} className="star-ic" />
            Star on GitHub
          </a>
          <Dialog.Root open={open} onOpenChange={setOpen}>
            <Dialog.Trigger className="menu-btn" aria-label="Open menu">
              <Menu size={18} strokeWidth={1.8} />
            </Dialog.Trigger>
            <Dialog.Portal>
              <Dialog.Backdrop className="sheet-backdrop" />
              <Dialog.Popup className="sheet">
                <div className="sheet-head">
                  <Dialog.Title className="sheet-title">
                    <Mark size={17} /> airgap
                  </Dialog.Title>
                  <Dialog.Close className="menu-btn" aria-label="Close menu">
                    <X size={18} strokeWidth={1.8} />
                  </Dialog.Close>
                </div>
                <nav aria-label="Mobile">
                  {[{ heading: "Products", items: PRODUCT_NAV }, { heading: "Resources", items: RESOURCE_NAV }].map((g) => (
                    <div className="sheet-group" key={g.heading}>
                      <p className="sheet-label">{g.heading}</p>
                      {g.items.map((item) => (
                        <a
                          key={item.href}
                          href={item.href}
                          className="sheet-link"
                          onClick={() => setOpen(false)}
                          {...(item.external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
                        >
                          <item.icon size={16} strokeWidth={1.6} />
                          {item.title}
                        </a>
                      ))}
                    </div>
                  ))}
                </nav>
                <div className="sheet-actions">
                  <a className="btn btn--ghost" href={LINKS.repo} target="_blank" rel="noopener noreferrer">
                    <GithubIcon size={15} /> GitHub
                  </a>
                </div>
              </Dialog.Popup>
            </Dialog.Portal>
          </Dialog.Root>
        </div>
      </div>
      <ScrollProgress />
    </header>
  );
}
