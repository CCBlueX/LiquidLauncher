<script>
    import { invoke } from "@tauri-apps/api";
    import LoginScreen from "./login/LoginScreen.svelte";
    import MainScreen from "./main/MainScreen.svelte";
    import { listen, once } from "@tauri-apps/api/event";

    export let options;

    // Easy way to store options
    options.store = function() {
        invoke("store_options", { options })
            .catch(e => console.error(e));
    };

    // Debug options - might be interesting to see what's in there
    console.debug("read options", options);

    // Refresh the current account if it exists
    if (options.currentAccount !== null) {
        // This will be run in the background
        invoke("refresh", { accountData: options.currentAccount })

        once("refreshed", (e) => {
            console.debug("refreshed account data", e.payload);

            options.currentAccount = e.payload;
            options.store();
        });
    }

    // Logout from current account
    function logout() {
        // Revoke the actual session
        invoke("logout", { accountData: options.currentAccount })
            .catch(e => console.error(e));

        // Remove account data from options data
        options.currentAccount = null;
        options.store();
    }
</script>

<div class="window">
    <!-- TODO: Animation? -->
    {#if options.currentAccount !== null }
        <MainScreen bind:options on:logout={logout} />
    {:else}
        <LoginScreen bind:options />
    {/if}
</div>

<style>
    .window {
        background-color: rgba(0, 0, 0, 0.6);
        width: 100vw;
        height: 100vh;
        padding: 32px;
        /* border-radius: 14px; */
    }
</style>
