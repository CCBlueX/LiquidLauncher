<script>
    import { invoke } from "@tauri-apps/api";
    import LoginScreen from "./login/LoginScreen.svelte";
    import MainScreen from "./main/MainScreen.svelte";

    // Load options from file
    let options;

    invoke("get_options")
        .then((result) => {
            options = result;

            // Debug options - might be interesting to see what's in there
            console.debug("Options", options);

            // Easy way to store options
            options.store = function () {
                console.debug("Storing options...", options);
                invoke("store_options", { options }).catch((e) =>
                    console.error(e),
                );
            };
        })
        .catch((e) => console.error(e));

    // Logout from current account
    function logout() {
        // Revoke the actual session
        invoke("logout", { accountData: options.currentAccount }).catch(console.error);

        // Remove account data from options data
        options.currentAccount = null;
        options.store();
    }

    // Check if the launcher is online and passes health checks
    invoke("check_health")
        .then(() => console.info("Health Check passed"))
        .catch((e) => {
            let message = e;
            if (message.startsWith('"')) message = message.slice(1);
            if (message.endsWith('"')) message = message.slice(0, -1);

            console.error(message);
            alert(message.replace(/\\n/g, "\n"));
            
            // Open help page
            open("https://liquidbounce.net/docs/Tutorials/Fixing%20LiquidLauncher");
        });
</script>

<div class="window">
    <div class="drag-area" data-tauri-drag-region></div>

    {#if options}
        {#if options.currentAccount}
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

    @media (prefers-color-scheme: light) {
        .window {
            background-color: rgba(0, 0, 0, 0.8);
        }
    }

    h1 {
        color: white;
    }
</style>
