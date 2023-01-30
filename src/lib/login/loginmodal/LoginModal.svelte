<script>
    import ModalButton from "./ModalButton.svelte";
    import ModalInput from "./ModalInput.svelte";
    
    import { invoke } from "@tauri-apps/api/tauri";
    import { listen } from "@tauri-apps/api/event";

    export let options;

    let offlineUsername = "";

    function handleOfflineLoginClick(e) {
        invoke("login_offline", { username: offlineUsername })
            .then((accountData) => {
                console.debug("login_offline", accountData)

                options.currentAccount = accountData;
                options.store();
            })
            .catch(e => console.error(e));
    }

    function handleMicrosoftLoginClick(e) {
        invoke("login_microsoft");
    }

    let microsoftCode;

    listen("microsoft_code", (e) => {
        console.debug("microsoft_code", e.payload);

        microsoftCode = e.payload;
    });

    listen("microsoft_successful", (e) => {
        console.debug("microsoft_successful", e.payload);

        options.currentAccount = e.payload;
        options.store();
    });

    function linkMicrosoftOpen() {
        invoke("open_url", { url: "https://microsoft.com/link" })
    }

    function cancelMicrosoft() {
        microsoftCode = null;
    }
</script>

<div class="modal">
    {#if microsoftCode == null}
        <div class="title">Log in</div>

        <ModalInput placeholder="Username" icon="person" characterLimit={16} bind:value={offlineUsername} />
        <ModalButton text="Offline login" primary={false} on:click={handleOfflineLoginClick} />
        <ModalButton text="Microsoft login" primary={true} on:click={handleMicrosoftLoginClick} />
    {:else}
        <div class="title">Microsoft Login</div>

        <ModalInput placeholder="Microsoft Code" characterLimit={16} icon="lock" bind:value={microsoftCode} />
        <ModalButton text="Link" primary={true} on:click={linkMicrosoftOpen} />
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