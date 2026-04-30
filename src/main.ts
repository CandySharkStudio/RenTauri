import { invoke } from "@tauri-apps/api/core";
import {} from "./store/backend-tauri";

let root: HTMLDivElement | null;
window.addEventListener("DOMContentLoaded", () => {
  root = document.getElementById("root") as HTMLDivElement;
  root.innerHTML = `
  
  `
  invoke("init_copywriting", {fileName: 'main.rrs'})
    .then(console.log)
    .catch(console.error)
});
window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  e.stopPropagation();
})