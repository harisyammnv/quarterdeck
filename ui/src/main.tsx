import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./design-system/tokens.css";
import "./design-system/components.css";
import "./styles/base.css";
import "./styles/components.css";
import "./styles/shell.css";
import "./styles/views.css";
import { App } from "./App";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root");
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
