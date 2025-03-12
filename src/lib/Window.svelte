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
    import ErrorScreen from "./main/ErrorScreen.svelte";

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
        } catch (e) {
            console.error("Failed to load options:", e);

            error = {
                message: "Failed to load launcher options",
                error: e
            };
        }
    }

    async function checkHealth() {
        try {
            await invoke("check_health");
            console.info("Health Check passed");
        } catch (e) {
            console.error("Health check failed", e);
            error = {
                message: "Failed to establish connection with LiquidBounce API",
                error: e
            };
        }
    }

    onMount(async () => {
        await Promise.all([handleUpdate(), checkHealth()]);
        await setupOptions();
        loading = false;
    });
</script>

<div class="window">
    <div class="drag-area" data-tauri-drag-region></div>

    {#if error}
        <ErrorScreen {error} />
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

    @media (prefers-color-scheme: light) {
        .window {
            background-color: rgba(0, 0, 0, 0.8);
        }
    }
</style>