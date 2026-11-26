<script>
    import {invoke} from "@tauri-apps/api/core";
    import SignIn from "../../accounts/SignIn.svelte";

    export let options;
</script>

<div class="modal">
    <div class="title">Sign in to play</div>
    {#await invoke("offline_login_hidden") then hidden}
        <SignIn bind:options offline={!hidden} />
    {/await}
</div>

<style>
    .modal {
        background-color: rgba(0, 0, 0, 0.26);
        padding: 30px;
        border-radius: 12px;
        width: 320px;
    }

    .title {
        color: white;
        font-size: 22px;
        margin: 0 auto;
        position: relative;
        width: max-content;
        margin-bottom: 40px;
    }

    .title::after {
        content: "";
        position: absolute;
        height: 5px;
        width: calc(100% - 10px);
        left: 50%;
        bottom: -20px;
        transform: translateX(-50%);
        background-color: #4677FF;
        border-radius: 5px;
    }
</style>
