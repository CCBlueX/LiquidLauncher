<script>
    import LoginScreen from "./login/LoginScreen.svelte";
    import MainScreen from "./main/MainScreen.svelte";

    let loggedIn = false;

    function handleLogin(e) {
        loggedIn = true;
    }
</script>

<div class="window">
    <!-- TODO: Animation? -->
    {#if loggedIn}
        <MainScreen />
    {:else}
        <LoginScreen on:login={handleLogin} />
    {/if}
</div>

<style>
    .window {
        background-color: rgba(0, 0, 0, 0.6);
        width: 1000px;
        height: 670px;
        border-radius: 14px;
        padding: 32px;

        /* Testing */
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        backdrop-filter: blur(10px);
        overflow: hidden;
        resize: both;
    }
</style>
