import {prepareLaunchTheme} from './launchTheme';
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
import "./desktopConsistency.css";
import "./personalHome.css";
import "./miniPlayer.css";
import "./connectedMotion.css";
import "./listFind.css";
import "./connectedConsistency.css";
import "./interactionPolish.css";
prepareLaunchTheme();
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
