<script>
    import {onDestroy, onMount} from "svelte";
    import {fade} from "svelte/transition";
    import {invoke} from "@tauri-apps/api/core";
    import {openUrl} from "@tauri-apps/plugin-opener";

    import ModalButton from "./ModalButton.svelte";
    import ModalInput from "./ModalInput.svelte";
    import SignInWaiting from "../../main/accounts/SignInWaiting.svelte";
    import {createMicrosoftLogin} from "../../main/accounts/microsoftLogin.js";
    import {MINECRAFT_STORE_URL, usernameProblem} from "../../main/accounts/accounts.js";

    export let options;

    const login = createMicrosoftLogin(() => options, useAccount);
    const status = login.status;

    // Shown while no sign-in runs: "choose" | "deviceLinks"
    let idleView = "choose";
    $: view = $status === "idle" ? idleView : $status;

    let offlineAvailable = false;
    let offlineUsername = "";

    function useAccount(account) {
        options.start.account = account;
        options.store();
    }

    function startOver() {
        login.cancel();
        idleView = "choose";
    }

    async function offlineLogin() {
        const problem = usernameProblem(offlineUsername);
        if (problem) {
            alert(problem);
            return;
        }

        try {
            useAccount(await invoke("add_offline", {options, username: offlineUsername}));
        } catch (error) {
            alert("Offline login failed.\n\n" + error);
        }
    }

    onMount(async () => {
        offlineAvailable = await invoke("offline_login_available");
    });

    onDestroy(login.destroy);
</script>

<div class="modal">
    {#key view}
        <div class="view" in:fade={{duration: 150}}>
            {#if view === "webview"}
                <div class="title">Microsoft login</div>
                <p class="hint">Finish signing in in the popup window.</p>
                <SignInWaiting />
                <ModalButton text="Cancel" primary={false} on:click={login.cancel} />
            {:else if view === "deviceLinks"}
                <div class="title">Microsoft device login</div>
                <p class="hint">Opens the Microsoft sign-in page in your browser with your code already filled in.</p>
                <ModalButton text="Link Account" primary={true} on:click={login.openLink} />
                <div class="text-buttons">
                    <button class="text-button" type="button" on:click={login.copyLink}>Copy URL</button>
                    <button class="text-button" type="button" on:click={() => idleView = "choose"}>Back</button>
                </div>
            {:else if view === "deviceCode"}
                <div class="title">Microsoft device login</div>
                <p class="hint">Finish signing in in your browser.</p>
                <SignInWaiting />
                <ModalButton text="Cancel" primary={false} on:click={login.cancel} />
            {:else if view === "noMinecraft"}
                <div class="title">Get Minecraft to play</div>
                <p class="hint">This account does not own Minecraft: Java Edition.</p>
                <ModalButton text="Buy Minecraft" primary={true} on:click={() => openUrl(MINECRAFT_STORE_URL)} />
                <ModalButton text="Use another account" primary={false} on:click={startOver} />
            {:else}
                <div class="title">Sign in to play</div>
                <p class="hint" style="margin-bottom: 5px">LiquidBounce needs a Microsoft account that owns Minecraft: Java Edition.</p>
                <ModalButton text="Microsoft login" primary={true} on:click={login.webview} />
                <ModalButton text="Microsoft device login" primary={false} on:click={() => idleView = "deviceLinks"} />

                {#if offlineAvailable}
                    <div class="divider">or</div>
                    <ModalInput placeholder="Username" icon="person" characterLimit={16} bind:value={offlineUsername} />
                    <ModalButton text="Offline login" primary={false} on:click={offlineLogin} />
                {/if}
            {/if}
        </div>
    {/key}
</div>

<style>
    .modal {
        background-color: rgba(0, 0, 0, 0.26);
        padding: 30px;
        border-radius: 12px;
        width: 320px;
    }

    .view {
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

    .divider {
        display: flex;
        align-items: center;
        gap: 12px;
        margin: -5px 20px;
        color: rgba(255, 255, 255, .4);
        font-size: 12px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 1px;
    }

    .divider::before,
    .divider::after {
        content: "";
        flex: 1;
        height: 1px;
        background: rgba(255, 255, 255, .15);
    }

    .hint {
        color: rgba(255, 255, 255, 0.5);
        font-size: 12px;
        text-align: center;
        line-height: 1.4;
        padding: 0 6px;
    }

    .text-buttons {
        display: flex;
        justify-content: center;
        column-gap: 8px;
    }

    .text-button {
        padding: 4px 8px;
        border: none;
        background: transparent;
        color: rgba(255, 255, 255, .5);
        font-family: "Inter", sans-serif;
        font-size: 14px;
        text-decoration: underline;
        text-underline-offset: 3px;
        cursor: pointer;
    }
</style>
