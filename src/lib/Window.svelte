<script>
    import {invoke} from "@tauri-apps/api/core";
    import {check} from "@tauri-apps/plugin-updater";
    import {exit, relaunch} from "@tauri-apps/plugin-process";
    import {ask} from "@tauri-apps/plugin-dialog";
    import {onMount} from "svelte";
    import MainScreen from "./main/MainScreen.svelte";
    import LoginScreen from "./login/LoginScreen.svelte";
    import LoadingScreen from "./main/LoadingScreen.svelte";
    import ErrorScreen from "./main/ErrorScreen.svelte";
    import NonSecureConnectionScreen from "./main/NonSecureConnectionScreen.svelte";

    let loading = true;
    let error = null;
    let options = null;
    let client = null;

    // Asks if the user allows non-secure connections
    let allowNonSecure = false;

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

    async function setupClient() {
        try {
            client = await invoke("setup_client", {
                sessionToken: options.launcher.sessionToken
            });
            console.info("API Client has been set up", client);
        } catch (e) {
            console.error("Failed to set up API client:", e);
            error = {
                message: "Failed to establish connection with LiquidBounce API",
                error: e
            };
        }
    }

    async function checkSystem() {
        try {
            await invoke("check_system");
        } catch (e) {
            // We want to continue allowing the user to use the launcher even 
            // if the system check fails
            alert("Looks like there is a configuration issue with your system.\n\n" + e);
        }
    }

    onMount(async () => {
        await setupOptions();
        await Promise.all([handleUpdate(), setupClient(), checkSystem()]);
        loading = false;
    });
</script>

<div class="window">
    <div class="drag-area" data-tauri-drag-region></div>

    {#if error}
        <ErrorScreen {error} />
    {:else if loading || !client}
        <LoadingScreen />
    {:else if client && !client.is_secure && !allowNonSecure}
        <NonSecureConnectionScreen
            on:allowNonSecure={() => {
                allowNonSecure = true;
            }}
            on:cancel={async () => {
                await exit(0);
            }}
        />
    {:else if options}
        {#if options.start.account}
            <MainScreen {client} bind:options />
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

    @media (prefers-color-scheme: light) {
        .window {
            background-color: rgba(0, 0, 0, 0.8);
        }
    }
</style>