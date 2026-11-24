import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

// Bloqueia o menu de contexto do navegador e o zoom acidental com Ctrl+roda (há zoom próprio em Configurações).
window.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement;
  if (!["INPUT", "TEXTAREA"].includes(t.tagName)) e.preventDefault();
});

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
