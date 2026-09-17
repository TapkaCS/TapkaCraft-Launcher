import "@fontsource/press-start-2p";
import "@fontsource/vt323";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@/styles/theme.css";
import "@/styles/global.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element #root not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
