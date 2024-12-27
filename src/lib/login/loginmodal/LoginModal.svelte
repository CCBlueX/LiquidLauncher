<script>
    import ModalButton from "./ModalButton.svelte";
    import ModalInput from "./ModalInput.svelte";

    import {invoke} from "@tauri-apps/api/core";
    import {listen} from "@tauri-apps/api/event";
    import {openUrl} from "@tauri-apps/plugin-opener";

    export let options;

    let offlineUsername;
    let microsoftCode = null;

    async function handleOfflineLoginClick(e) {
        if (offlineUsername.length > 16 || offlineUsername.length < 1) {
            alert("Username must be between 1 and 16 characters long.");
            return;
        }

        const usernameRegex = /^[a-zA-Z0-9_]+$/;
        if (!usernameRegex.test(offlineUsername)) {
            alert("Username can only contain letters, numbers, and underscores.");
            return;
        }

        options.start.account = await invoke("login_offline", {username: offlineUsername});
        options.store();
    }

    async function handleMicrosoftLoginClick(e) {
        try {
            options.start.account = await invoke("login_microsoft");
            options.store();
        } catch (err) {
            alert(
                "Microsoft authentication failed.\n\n" +
                 err + "\n\n" +
                "Should you be unable to resolve this issue, please use the 'Offline' login option " +
                "and attempt to log in through the client's inbuilt account manager."
            );
            cancelMicrosoft();
        }
    }

    listen("microsoft_code", (e) => {
        microsoftCode = e.payload;
    });

    function cancelMicrosoft() {
        microsoftCode = null;
    }
</script>

<div class="modal">
    {#if !microsoftCode}
        <div class="title">Log in</div>

        <ModalInput placeholder="Username" icon="person" characterLimit={16} bind:value={offlineUsername} />
        <ModalButton text="Offline login" primary={false} on:click={handleOfflineLoginClick} />
        <ModalButton text="Microsoft login" primary={true} on:click={handleMicrosoftLoginClick} />
    {:else}
        <div class="title">Microsoft Login</div>

        <ModalInput placeholder="Microsoft Code" characterLimit={16} icon="lock" bind:value={microsoftCode} />
        <ModalButton text="Link" primary={true} on:click={() => openUrl("https://microsoft.com/link")} />
        <ModalButton text="Cancel" primary={false} on:click={cancelMicrosoft} />
    {/if}
</div>

<style>
    .modal {
        background-color: rgba(0, 0, 0, 0.26);
        padding: 30px;
        border-radius: 12px;
        width: 320px;
        display: flex;
        flex-direction: column;
        row-gap: 15px;
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