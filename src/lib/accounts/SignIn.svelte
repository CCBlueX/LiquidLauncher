<script>
    import {createEventDispatcher, onDestroy, onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import {listen} from "@tauri-apps/api/event";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import Tabs from "../settings/tab/Tabs.svelte";
    import ModalButton from "../login/loginmodal/ModalButton.svelte";
    import ModalInput from "../login/loginmodal/ModalInput.svelte";
    import ButtonCopyClipboard from "../common/ButtonCopyClipboard.svelte";
    import RippleLoader from "../common/RippleLoader.svelte";
    import {randomName} from "./randomName.js";

    export let options;

    const dispatch = createEventDispatcher();

    let types = ["Microsoft"];
    let type = "Microsoft";
    // "form" | "webview" | "deviceCode" | "noMinecraft"
    let view = "form";
    let deviceCode = null;
    let pending = false;
    let error = null;
    let username = "";
    let token = "";

    // Rust cannot stop a sign-in. A cancelled one runs on and its result is dropped.
    let attempt = 0;

    async function add(command, args, waitingView = "form") {
        const current = ++attempt;
        view = waitingView;
        pending = true;
        error = null;

        try {
            const account = await invoke(command, {options, ...args});
            if (current !== attempt) return;

            pending = false;
            options.start.account = account;
            await options.store();
            dispatch("added");
        } catch (e) {
            if (current !== attempt) return;

            pending = false;
            if (e === "no_minecraft") {
                view = "noMinecraft";
            } else {
                view = "form";
                error = `${e}`;
            }
        }
    }

    function cancel() {
        attempt++;
        pending = false;
        view = "form";
    }

    function signInWithBrowser() {
        deviceCode = null;
        add("add_microsoft_device_code", {}, "deviceCode");
    }

    function openSignInPage() {
        openUrl(deviceCode.directVerificationUri).catch(console.error);
    }

    const deviceCodeListener = listen("microsoft_device_code", ({payload}) => {
        if (view !== "deviceCode") return;
        deviceCode = payload;
        openSignInPage();
    });

    onMount(async () => {
        if (await invoke("offline_allowed", {options})) types = ["Microsoft", "Offline", "Session"];
    });

    onDestroy(() => {
        attempt++;
        deviceCodeListener.then(unlisten => unlisten());
    });
</script>

<div class="sign-in">
    {#if view === "webview"}
        <p class="hint">Finish signing in in the popup window.</p>
        <RippleLoader size={60} />
        <ModalButton text="Cancel" primary={false} on:click={cancel} />
    {:else if view === "deviceCode"}
        {#if deviceCode}
            <p class="hint">Confirm the sign-in in your browser.</p>
            <div class="code">
                <span>{deviceCode.userCode}</span>
                <ButtonCopyClipboard iconOnly text="Copy code" textToCopy={deviceCode.userCode} />
            </div>
            <p class="hint">
                No page opened? Enter the code at
                <button class="link" type="button" on:click={openSignInPage}>{deviceCode.verificationUri.replace("https://", "")}</button>.
            </p>
        {:else}
            <p class="hint">Requesting a sign-in code&hellip;</p>
        {/if}
        <RippleLoader size={60} />
        <ModalButton text="Cancel" primary={false} on:click={cancel} />
    {:else if view === "noMinecraft"}
        <p class="hint">This account does not own Minecraft: Java Edition.</p>
        <ModalButton text="Buy Minecraft" primary on:click={() => openUrl("https://www.minecraft.net/store/minecraft-java-bedrock-edition-pc")} />
        <ModalButton text="Use another account" primary={false} on:click={cancel} />
    {:else}
        {#if types.length > 1}
            <Tabs tabs={types} bind:activeTab={type} />
        {/if}

        {#if type === "Microsoft"}
            <p class="hint">Use the Microsoft account that owns Minecraft: Java Edition.</p>
            <ModalButton text="Sign in with Microsoft" primary on:click={() => add("add_microsoft_webview", {}, "webview")} />
            <ModalButton text="Sign in with browser" primary={false} on:click={signInWithBrowser} />
        {:else if type === "Offline"}
            <p class="hint">Plays on servers that do not verify accounts.</p>
            <div class="username">
                <ModalInput
                        icon="person"
                        placeholder="Username"
                        characterLimit={16}
                        bind:value={username}
                        on:keydown={e => e.key === "Enter" && add("add_offline", {username})}
                />
                <button class="random" type="button" title="Random username" aria-label="Random username" on:click={() => username = randomName()}>
                    <img src="img/icon/icon-random.svg" alt="">
                </button>
            </div>
            <ModalButton text="Add account" primary disabled={pending} on:click={() => add("add_offline", {username})} />
        {:else}
            <p class="hint">Takes a Minecraft access token or a Microsoft refresh token.</p>
            <ModalInput
                    icon="lock"
                    placeholder="Token"
                    password
                    bind:value={token}
                    on:keydown={e => e.key === "Enter" && add("add_session", {token})}
            />
            <ModalButton text="Add account" primary disabled={pending} on:click={() => add("add_session", {token})} />
        {/if}

        {#if error}
            <p class="hint error" role="alert">{error}</p>
        {/if}
    {/if}
</div>

<style>
    .sign-in {
        display: flex;
        flex-direction: column;
        row-gap: 15px;
    }

    .sign-in :global(.tabs) {
        margin-bottom: 0;
    }

    .hint {
        color: rgba(255, 255, 255, .5);
        font-size: 12px;
        line-height: 1.4;
        text-align: center;
    }

    .hint.error {
        color: #FE4C2E;
        word-break: break-word;
    }

    .link {
        padding: 0;
        border: none;
        background: none;
        color: #4677FF;
        font: inherit;
        font-weight: 600;
        text-decoration: underline;
        cursor: pointer;
    }

    .code {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 54px;
        padding: 0 12px 0 16px;
        border-radius: 6px;
        background-color: rgba(255, 255, 255, .06);
        color: white;
        font-size: 21px;
        font-weight: 600;
        letter-spacing: 3px;
        user-select: text;
    }

    .username {
        position: relative;
    }

    .random {
        position: absolute;
        top: 50%;
        right: 16px;
        transform: translateY(-50%);
        display: flex;
        padding: 0;
        border: none;
        background: none;
        cursor: pointer;
    }

    .random img {
        height: 18px;
    }

    .link:focus-visible,
    .random:focus-visible {
        outline: solid 2px white;
        outline-offset: 2px;
    }
</style>
