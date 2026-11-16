import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../index.css";
import { mockIpc } from "./mockIpc";
import { PreviewApp } from "./PreviewApp";

// The browser has no engine behind the IPC seam; the settings sheet would show
// nothing but failures. This is the harness's only substitution.
mockIpc();

const container = document.getElementById("root");
if (!container) throw new Error("No #root element in preview.html");

createRoot(container).render(
  <StrictMode>
    <PreviewApp />
  </StrictMode>,
);
