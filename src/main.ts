import { invoke } from "@tauri-apps/api/core";
import {} from "./store/backend-tauri";
import Loading from "./assets/loading.jpg";

let root: HTMLDivElement | null;
window.addEventListener("DOMContentLoaded", () => {
  root = document.getElementById("root") as HTMLDivElement;
  root.innerHTML = `
  <img src="${Loading}" style="width: 100vw; height: 100vh; position: fixed; top: 0; left: 0;">
  `;
  invoke("init_copywriting", { fileName: "main.rrs" })
    .then(console.log)
    .catch(console.error);
});
window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  e.stopPropagation();
});
