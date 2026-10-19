import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";
import "./sketch.css";
import "./refinements.css";
import "./controls.css";
import "./settings.css";
import "./management.css";
import "./listeningPolish.css";
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
