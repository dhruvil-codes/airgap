import { StrictMode } from "react";
import { renderToString } from "react-dom/server";
import App from "./App";

export { agentFiles } from "./agent/files";
export { PAGES, SITE } from "./agent/knowledge";

export function render(path: string) {
  return renderToString(
    <StrictMode>
      <App path={path} />
    </StrictMode>,
  );
}
