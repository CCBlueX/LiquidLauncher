<script>
    import { invoke } from "@tauri-apps/api/core";
    import { check } from "@tauri-apps/plugin-updater";
    import { relaunch } from "@tauri-apps/plugin-process";
    import { ask } from "@tauri-apps/plugin-dialog";
    import {onMount} from "svelte";
    import MainScreen from "./main/MainScreen.svelte";
    import LoginScreen from "./login/LoginScreen.svelte";
    import { scale } from "svelte/transition";
    import { expoInOut } from "svelte/easing";

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
    <div class="loader-wrapper" in:scale={{ duration: 200, easing: expoInOut }} out:scale={{ duration: 200, easing: expoInOut }}>
        <div class="lds-ring">
            <div></div>
            <div></div>
            <div></div>
            <div></div>
        </div>
    </div>
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
        background: 
    repeating-conic-gradient(rgba(24, 24, 24, 0.5) 0% 25%, rgba(20, 20, 20, 0.5) 0% 50%) 
      50% / 50px 50px;
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

    .loader-wrapper {
        position: absolute;
        left: 50%;
        top: 50%;
        transform: translateY(-50%, -50%);
        width: 32px;
        height: 32px;
    }
    .lds-ring, .lds-ring div {
        box-sizing: border-box;
        position: relative;
        color: white;
    }
    .lds-ring {
        display: inline-block;
        width: 32px;
        height: 32px;
    }
    .lds-ring div {
        box-sizing: border-box;
        display: block;
        position: absolute;
        width: 25px;
        height: 25px;
        margin: 4px;
        border: 4px solid currentColor;
        border-radius: 50%;
        animation: lds-ring 1.2s cubic-bezier(0.5, 0, 0.5, 1) infinite;
        border-color: currentColor transparent transparent transparent;
    }
    .lds-ring div:nth-child(1) {
        animation-delay: -0.45s;
    }
    .lds-ring div:nth-child(2) {
        animation-delay: -0.3s;
    }
    .lds-ring div:nth-child(3) {
        animation-delay: -0.15s;
    }
    @keyframes lds-ring {
        0% {
            transform: rotate(0deg);
        }
        100% {
            transform: rotate(360deg);
        }
    }
</style>