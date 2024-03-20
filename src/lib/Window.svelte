<script>
    import { invoke } from "@tauri-apps/api/core";
    import LoginScreen from "./login/LoginScreen.svelte";
    import MainScreen from "./main/MainScreen.svelte";
    import { check } from "@tauri-apps/plugin-updater";
    import { relaunch } from "@tauri-apps/plugin-process";

    check().then((result) => {
        console.debug("Update Check Result", result);
        if (result && result.available) {
            result.downloadAndInstall().then(() => {
                relaunch().catch(e => console.error(e));
            }).catch(e => console.error("Download and Install Failed", e));
        }
    }).catch(e => console.error("Update Check Failed", e));

    // Load options from file
    let options;

    invoke("get_options").then((result) => {
        options = result;

        // Debug options - might be interesting to see what's in there
        console.debug("Options", options);

        // Easy way to store options
        options.store = function() {
            console.debug("Storing options...", options);
            invoke("store_options", { options })
                .catch(e => console.error(e));
        };
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
        console.debug("Status", result);
    }).catch(e => {
        alert("You are offline! Please connect to the internet and restart the app.\n If this problem persists, please contact the developer.\n\n (Error: " + e + ")");
        console.error(e);
    });
</script>

<div class="window">
    <div class="drag-area" data-tauri-drag-region>

    </div>

    {#if options }
        {#if options.currentAccount }
            <MainScreen bind:options on:logout={logout} />
        {:else}
            <LoginScreen bind:options />
        {/if}
    {:else}
        <h1>The launcher is loading...</h1>
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
