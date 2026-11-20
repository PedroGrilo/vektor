import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/orbitron/600.css";
import "@fontsource/orbitron/800.css";
import "@fontsource-variable/space-grotesk";
import "@fontsource-variable/jetbrains-mono";
import "./styles.css";
import "./components.css";
import App from "./App";

// Lite swaps the five palette colours (see styles.css).
if (__LITE__) document.documentElement.dataset.edition = "lite";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
