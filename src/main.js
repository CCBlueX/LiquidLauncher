import "./app.css";
import "./i18n.js";
import App from "./App.svelte";

const app = new App({
    target: document.getElementById("app"),
    props: { }
});

export default app;
