import {getCurrentWindow} from "@tauri-apps/api/window"

export async function closeWindow() {
    await getCurrentWindow().close();
}