import { invoke } from "@tauri-apps/api";
import "./app.css";
import App from "./App.svelte";

(async function() {
    let options = await invoke("get_options");

    const app = new App({
        target: document.getElementById("app"),
        props: { options }
    });
})();