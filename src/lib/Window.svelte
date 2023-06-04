<script>
    import { invoke } from "@tauri-apps/api";
    import LoginScreen from "./login/LoginScreen.svelte";
    import MainScreen from "./main/MainScreen.svelte";
    import { once } from "@tauri-apps/api/event";
    import { appWindow } from "@tauri-apps/api/window";

    // Load options from file
    let options;

    invoke("get_options").then((result) => {
        options = result;

        // Debug options - might be interesting to see what's in there
        console.debug("read options", options);

        // Easy way to store options
        options.store = function() {
            console.debug("storing options", options);
            invoke("store_options", { options })
                .catch(e => console.error(e));
        };

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
    }).catch(e => console.error(e));

    // Logout from current account
    function logout() {
        // Revoke the actual session
        invoke("logout", { accountData: options.currentAccount })
            .catch(e => console.error(e));

        // Remove account data from options data
        options.currentAccount = null;
        options.store();
    }

    invoke("check_online_status").then((result) => {
        console.debug("online status", result);
    }).catch(e => {
        alert("You are offline! Please connect to the internet and restart the app.\n If this problem persists, please contact the developer.\n\n (Error: " + e + ")");
        console.error(e);
    });
</script>

<div class="window">
    <div class="drag-area" data-tauri-drag-region>

    </div>

    {#if options !== undefined }
        <!-- TODO: Animation? -->
        {#if options.currentAccount !== null }
            <MainScreen bind:options on:logout={logout} />
        {:else}
            <LoginScreen bind:options />
        {/if}
    {:else}
        <h1>Loading options...</h1>
    {/if}

</div>

<style>
    .drag-area {
        position: fixed;
        top: 0;
        left: 0;
        width: calc(100vw - 150px);
        height: 100px;
        z-index: 100;
    }

    .window {
        background-color: rgba(0, 0, 0, 0.6);
        width: 100vw;
        height: 100vh;
        padding: 32px;
        overflow: hidden;
        /* border-radius: 14px; */
    }

    h1 {
        color: white;
    }
</style>
