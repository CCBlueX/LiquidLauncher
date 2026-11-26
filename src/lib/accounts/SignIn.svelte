<script>
    import {createEventDispatcher, onDestroy} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import {listen} from "@tauri-apps/api/event";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import Tabs from "../settings/tab/Tabs.svelte";
    import IconButtonSetting from "../settings/IconButtonSetting.svelte";
    import ModalButton from "../login/loginmodal/ModalButton.svelte";
    import ModalInput from "../login/loginmodal/ModalInput.svelte";
    import ButtonCopyClipboard from "../common/ButtonCopyClipboard.svelte";
    import RippleLoader from "../common/RippleLoader.svelte";
    import {uniqueNamesGenerator, adjectives, animals, NumberDictionary} from "unique-names-generator";

    export let options;
    export let offline = true;

    const dispatch = createEventDispatcher();

    const types = offline ? ["Microsoft", "Offline", "Session"] : ["Microsoft", "Session"];
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

    function randomName() {
        const name = uniqueNamesGenerator({
            dictionaries: [adjectives, animals, NumberDictionary.generate({length: 2})],
            separator: "",
            style: "capital",
        });
        // Some dictionary words are too long for a Minecraft name.
        return /^\w{1,16}$/.test(name) ? name : randomName();
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

    const deviceCodeListener = listen("microsoft_device_code", ({payload}) => {
        if (view !== "deviceCode") return;
        deviceCode = payload;
        openUrl(payload.directVerificationUri).catch(console.error);
    });

    onDestroy(() => {
        attempt++;
        deviceCodeListener.then(unlisten => unlisten());
    });
</script>

{#if view === "webview"}
    <p class="hint">Finish signing in in the popup window.</p>
    <RippleLoader />
    <ModalButton text="Cancel" primary={false} on:click={cancel} />
{:else if view === "deviceCode"}
    {#if deviceCode}
        <p class="hint">
            Confirm the sign-in in your browser, or enter this code at
            {deviceCode.verificationUri.replace("https://", "")}.
        </p>
        <ButtonCopyClipboard text={deviceCode.userCode} textToCopy={deviceCode.userCode} />
    {:else}
        <p class="hint">Requesting a sign-in code&hellip;</p>
    {/if}
    <RippleLoader />
    <ModalButton text="Cancel" primary={false} on:click={cancel} />
{:else if view === "noMinecraft"}
    <p class="hint">This account does not own Minecraft: Java Edition.</p>
    <ModalButton text="Buy Minecraft" primary on:click={() => openUrl("https://www.minecraft.net/store/minecraft-java-bedrock-edition-pc")} />
    <ModalButton text="Use another account" primary={false} on:click={cancel} />
{:else}
    <Tabs tabs={types} bind:activeTab={type} />

    {#if type === "Microsoft"}
        <ModalButton text="Microsoft login" primary on:click={() => add("add_microsoft_webview", {}, "webview")} />
        <ModalButton text="Microsoft device login" primary={false} on:click={signInWithBrowser} />
    {:else if type === "Offline"}
        <ModalInput
                icon="person"
                placeholder="Username"
                characterLimit={16}
                bind:value={username}
                on:keydown={e => e.key === "Enter" && add("add_offline", {username})}
        />
        <div class="random">
            <IconButtonSetting text="Random username" icon="icon-random" on:click={() => username = randomName()} />
        </div>
        <ModalButton text="Offline login" primary disabled={pending} on:click={() => add("add_offline", {username})} />
    {:else}
        <ModalInput
                icon="lock"
                placeholder="Access or refresh token"
                password
                bind:value={token}
                on:keydown={e => e.key === "Enter" && add("add_session", {token})}
        />
        <ModalButton text="Session login" primary disabled={pending} on:click={() => add("add_session", {token})} />
    {/if}

    {#if error}
        <p class="hint error" role="alert">{error}</p>
    {/if}
{/if}

<style>
    .hint {
        color: rgba(255, 255, 255, 0.5);
        font-size: 12px;
        text-align: center;
        line-height: 1.4;
        padding: 0 6px;
    }

    .error {
        color: #FE4C2E;
    }

    .random {
        display: flex;
        justify-content: center;
    }
</style>
