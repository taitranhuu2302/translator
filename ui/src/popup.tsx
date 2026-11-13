import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Popup } from "./PopupView";
import "./styles.css";

const container = document.getElementById("root");
if (!container) throw new Error("popup.html is missing #root");

createRoot(container).render(
  <StrictMode>
    <Popup />
  </StrictMode>,
);
