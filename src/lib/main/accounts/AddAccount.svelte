<script>
    import {createEventDispatcher, onDestroy} from "svelte";
    import {fade} from "svelte/transition";
    import {invoke} from "@tauri-apps/api/core";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import SettingsContainer from "../../settings/SettingsContainer.svelte";
    import Tabs from "../../settings/tab/Tabs.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import ModalInput from "../../login/loginmodal/ModalInput.svelte";
    import SignInWaiting from "./SignInWaiting.svelte";
    import {createMicrosoftLogin} from "./microsoftLogin.js";
    import {MINECRAFT_STORE_URL, usernameProblem} from "./accounts.js";
    import {randomName} from "./randomName.js";

    export let options;
    // Offline and Session need a Microsoft account in the list first.
    export let canAddOtherTypes;

    const dispatch = createEventDispatcher();
    const login = createMicrosoftLogin(() => options, useAccount);
    const status = login.status;

    $: services = canAddOtherTypes ? ["Microsoft", "Offline", "Session"] : ["Microsoft"];

    let service = "Microsoft";
    let microsoftTab = "Web View";
    let username = "";
    let token = "";

    // Leaving a tab stops its sign-in.
    $: service, microsoftTab, login.cancel();

    function useAccount(account) {
        options.start.account = account;
        options.store();
        dispatch("added");
    }

    async function add(command, args) {
        try {
            useAccount(await invoke(command, {options, ...args}));
        } catch (error) {
            alert("Failed to add account.\n\n" + error);
        }
    }

    function addOffline() {
        const problem = usernameProblem(username);
        if (problem) {
            alert(problem);
            return;
        }

        add("add_offline", {username});
    }

    onDestroy(login.destroy);
</script>

<SettingsContainer title="Add account" pinned on:hideSettings={() => dispatch("close")}>
    <div slot="tabs">
        <Tabs tabs={services} bind:activeTab={service} />
        {#if service === "Microsoft"}
            <Tabs tabs={["Web View", "Device Code"]} bind:activeTab={microsoftTab} />
        {/if}
    </div>

    {#key `${service}-${microsoftTab}-${$status}`}
        <div class="tab" in:fade={{duration: 150}}>
            {#if $status === "noMinecraft"}
                <p class="text">This account does not own Minecraft: Java Edition.</p>
                <ButtonSetting text="Buy Minecraft" on:click={() => openUrl(MINECRAFT_STORE_URL)} />
                <button class="text-button" type="button" on:click={login.cancel}>Use another account</button>
            {:else if $status !== "idle"}
                <SignInWaiting />
                <ButtonSetting text="Cancel" color="rgba(0, 0, 0, .26)" on:click={login.cancel} />
            {:else if service === "Microsoft" && microsoftTab === "Web View"}
                <ButtonSetting text="Sign in with web view" on:click={login.webview} />
            {:else if service === "Microsoft"}
                <p class="hint">Opens the Microsoft sign-in page in your browser with your code already filled in.</p>
                <ButtonSetting text="Link Account" on:click={login.openLink} />
                <button class="text-button" type="button" on:click={login.copyLink}>Copy URL</button>
            {:else if service === "Offline"}
                <div class="input-row">
                    <ModalInput icon="person" placeholder="Username" characterLimit={16} bind:value={username} />
                    <button class="random" type="button" aria-label="Random username" title="Random username" on:click={() => username = randomName()}>
                        <img src="img/icon/icon-random.svg" alt="">
                    </button>
                </div>
                <ButtonSetting text="Add account" on:click={addOffline} />
            {:else}
                <p class="hint">Paste an access token or a refresh token. Either one works.</p>
                <ModalInput icon="lock" placeholder="Access or refresh token" password bind:value={token} />
                <ButtonSetting text="Add account" disabled={!token.trim()} on:click={() => add("add_session", {token})} />
            {/if}
        </div>
    {/key}
</SettingsContainer>

<style>
    .tab {
        display: flex;
        flex-direction: column;
        row-gap: 10px;
    }

    .text {
        color: white;
        font-size: 14px;
    }

    .hint {
        color: rgba(255, 255, 255, .5);
        font-size: 14px;
        line-height: 1.4;
    }

    .text-button {
        align-self: center;
        padding: 0;
        border: none;
        background-color: transparent;
        color: rgba(255, 255, 255, .5);
        font-family: "Inter", sans-serif;
        font-size: 14px;
        text-decoration: underline;
        text-underline-offset: 3px;
        cursor: pointer;
    }

    .input-row {
        display: grid;
        grid-template-columns: minmax(0, 1fr) max-content;
        align-items: center;
        column-gap: 10px;
    }

    .random {
        display: flex;
        align-items: center;
        padding: 0;
        border: none;
        background-color: transparent;
        cursor: pointer;
    }

    .random img {
        height: 20px;
    }
</style>
