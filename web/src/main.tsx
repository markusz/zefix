import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { load } from "./core";
import "./styles.css";

const root = document.getElementById("root");
load()
  .then(() => {
    if (root) createRoot(root).render(<StrictMode><App /></StrictMode>);
  })
  .catch((error: unknown) => {
    if (root) root.textContent = `zefix! konnte nicht starten: ${String(error)}`;
  });
