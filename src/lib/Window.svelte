<script>
    import { invoke } from "@tauri-apps/api/core";
    import { check } from "@tauri-apps/plugin-updater";
    import { relaunch } from "@tauri-apps/plugin-process";
    import { ask } from "@tauri-apps/plugin-dialog";
    import { writable, get } from 'svelte/store';
    import {onMount} from "svelte";
    import MainScreen from "./main/MainScreen.svelte";
    import LoginScreen from "./login/LoginScreen.svelte";

    const appState = writable({
        loading: true,
        error: null,
        options: null
    });

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
            const options = await invoke("get_options");
            console.debug("Options loaded:", options);

            options.store = async function() {
                console.debug("Storing options...", options);
                try {
                    await invoke("store_options", { options: options });
                } catch (error) {
                    console.error("Failed to store options:", error);
                    throw error;
                }
            };

            appState.update(state => ({
                ...state,
                loading: false,
                options
            }));
        } catch (error) {
            console.error("Failed to load options:", error);
            appState.update(state => ({
                ...state,
                loading: false,
                error: "Failed to load launcher options"
            }));
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

    async function handleLogout() {
        const { options } = get(appState);

        try {
            await invoke("logout", { accountData: options.start.account });
            options.start.account = null;
            await options.store();

            appState.update(state => ({
                ...state,
                options: { ...options }
            }));
        } catch (error) {
            console.error("Logout failed:", error);
            alert("Failed to logout properly. Please try again.");
        }
    }

    onMount(async () => {
        try {
            await Promise.all([handleUpdate(), checkHealth()]);
            await setupOptions();
        } catch (error) {
            console.error("App initialization failed:", error);
            appState.update(state => ({
                ...state,
                loading: false,
                error: "Failed to initialize launcher"
            }));
        }
    });
</script>

<div class="window">
    <div class="drag-area" data-tauri-drag-region></div>

    {#if $appState.error}
        <h1 class="error">Error: {$appState.error}</h1>
    {:else if $appState.loading}
        <h1>The launcher is loading...</h1>
    {:else if $appState.options}
        {#if $appState.options.start.account}
            <MainScreen
                    bind:options={$appState.options}
                    on:logout={handleLogout}
            />
        {:else}
            <LoginScreen
                    bind:options={$appState.options}
            />
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
