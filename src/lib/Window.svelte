<script>
    import { invoke } from "@tauri-apps/api/core";
    import { check } from "@tauri-apps/plugin-updater";
    import { relaunch } from "@tauri-apps/plugin-process";
    import { ask } from "@tauri-apps/plugin-dialog";
    import Logo from "./common/Logo.svelte";
    import { onMount } from "svelte";
    import {Jumper} from "svelte-loading-spinners";
    import VerticalFlexWrapper from "./common/VerticalFlexWrapper.svelte";
    import ButtonClose from "./common/ButtonClose.svelte";
    import TitleBar from "./common/TitleBar.svelte";
    import MainScreen from "./main/MainScreen.svelte";
    import LoginScreen from "./login/LoginScreen.svelte";
    import LoadingScreen from "./main/LoadingScreen.svelte";

    let loading = true;
    let error = null;
    let options = null;

    async function handleUpdate() {
        try {
            const result = await check();
            console.debug("Update Check Result", result);

            if (result?.available) {
                const shouldUpdate = await ask(
                    "A Launcher update is available. Would you like to install it now?",
                    "LiquidLauncher"
                );

                if (shouldUpdate) {
                    await result.downloadAndInstall();
                    await relaunch();
                }
            }
        } catch (error) {
            console.error("Update process failed:", error);
        }
    }

    async function setupOptions() {
        try {
            options = {
                store: async function () {
                    console.debug("Storing options...", options);
                    try {
                        await invoke("store_options", {options});
                    } catch (error) {
                        console.error("Failed to store options:", error);
                        throw error;
                    }
                },
                ...await invoke("get_options")
            };
            console.debug("Options loaded:", options);

            loading = false;
        } catch (error) {
            console.error("Failed to load options:", error);
            loading = false;
            error = "Failed to load launcher options";
        }
    }

    async function checkHealth() {
        try {
            await invoke("check_health");
            console.info("Health Check passed");
        } catch (error) {
            const message = error.toString().replace(/^"/, '').replace(/"$/, '');
            console.error("Health check failed:", message);
            alert(message.replace(/\\n/g, "\n"));
            window.open("https://liquidbounce.net/docs/Tutorials/Fixing%20LiquidLauncher", "_blank");
        }
    }

    onMount(async () => {
        try {
            await Promise.all([handleUpdate(), checkHealth()]);
            await setupOptions();
        } catch (error) {
            console.error("App initialization failed:", error);
            loading = false;
            error = "Failed to initialize launcher";
        }
    });
</script>

<div class="window">
    <div class="drag-area" data-tauri-drag-region></div>

    {#if error}
        <h1 class="error">Error: {error}</h1>
    {:else if loading}
        <LoadingScreen />
    {:else if options}
        {#if options.start.account}
            <MainScreen bind:options />
        {:else}
            <LoginScreen bind:options />
        {/if}
    {/if}
</div>

<style>
    .window {
        background-color: rgba(0, 0, 0, 0.6);
        width: 100vw;
        height: 100vh;
        padding: 32px;
        overflow: hidden;
    }

    .drag-area {
        position: fixed;
        top: 0;
        left: 0;
        width: calc(100vw - 150px);
        height: 100px;
        z-index: 100;
    }

    h1 {
        color: white;
    }

    .error {
        color: #ff4444;
    }

    @media (prefers-color-scheme: light) {
        .window {
            background-color: rgba(0, 0, 0, 0.8);
        }
    }
</style>