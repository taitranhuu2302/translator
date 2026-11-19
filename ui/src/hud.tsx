import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Hud } from "./HudView";
import "./index.css";

const container = document.getElementById("root");
if (!container) throw new Error("hud.html is missing #root");

createRoot(container).render(
  <StrictMode>
    <Hud />
  </StrictMode>,
);
