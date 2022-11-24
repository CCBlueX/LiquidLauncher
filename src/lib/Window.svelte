<script>
    import { invoke } from "@tauri-apps/api";
    import LoginScreen from "./login/LoginScreen.svelte";
    import MainScreen from "./main/MainScreen.svelte";

    export let options;

    options.store = function() {
        invoke("store_options", { options })
            .catch(e => console.error(e));
    };

    console.log(options);

    function logout() {
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
