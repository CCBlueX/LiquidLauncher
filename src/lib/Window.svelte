<script>
    import {invoke} from "@tauri-apps/api/core";
    import {listen} from "@tauri-apps/api/event";
    import {exit} from "@tauri-apps/plugin-process";
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

    // Set once the launcher starts updating itself, drives the splash progress bar
    let updateProgress = null;

    async function handleUpdate() {
        const unlisten = await listen("progress-update", (event) => {
            const { type, value } = event.payload;
            updateProgress ??= { value: 0, max: 0, text: "", speed: 0 };

            switch (type) {
                case "max":
                    updateProgress.max = value;
                    break;
                case "progress":
                    updateProgress.value = value;
                    break;
                case "label":
                    updateProgress.text = value;
                    break;
                case "speed":
                    updateProgress.speed = value;
                    break;
            }
        });

        try {
            await invoke("check_for_updates");
        } catch (e) {
            console.error("Update process failed:", e);
        } finally {
            unlisten();
            updateProgress = null;
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

<!-- Windows 11 rounds the window itself -->
<div class="window" class:rounded={!navigator.platform.startsWith("Win")}>
    <div class="drag-area" data-tauri-drag-region></div>

    {#if error}
        <ErrorScreen {error} />
    {:else if loading || !client}
        <LoadingScreen progressState={updateProgress} />
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
            <MainScreen {client} bind:options bind:error />
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

    /* Same radius as CORNER_RADIUS in backdrop.rs. Paint containment clips the fixed overlays too. */
    .window.rounded {
        border-radius: 8px;
        contain: paint;
    }

    /* Windows 11's window border, drawn above the overlays. */
    .window.rounded::after {
        content: "";
        position: fixed;
        inset: 0;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: inherit;
        pointer-events: none;
        z-index: 1001;
    }

    .drag-area {
        position: fixed;
        top: 0;
        left: 0;
        /* Stops left of the account card, which has to stay clickable. */
        width: calc(100vw - 400px);
        height: 100px;
        z-index: 100;
    }

    @media (prefers-color-scheme: light) {
        .window {
            background-color: rgba(0, 0, 0, 0.8);
        }
    }
</style>
