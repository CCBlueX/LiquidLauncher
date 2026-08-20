<script>
    import qrcode from "qrcode-generator";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";

    import ModalButton from "./ModalButton.svelte";
    import ModalInput from "./ModalInput.svelte";

    import {invoke} from "@tauri-apps/api/core";
    import {listen} from "@tauri-apps/api/event";
    import {openUrl} from "@tauri-apps/plugin-opener";

    export let options;

    // "choose" | "webview" | "code"
    let view = "choose";

    let offlineUsername;
    let deviceCode = null;
    let codeCopied = false;

    // Neither login command can be cancelled once invoked, so a "Cancel"
    // click or picking another login path can't stop the Rust-side future.
    // This token makes sure a late/superseded result is dropped instead of
    // silently overwriting whatever the user ended up logging in with.
    let loginAttempt = 0;

    $: deviceCodeQr = deviceCode ? buildQrSvg(deviceCode.directVerificationUri) : null;

    function buildQrSvg(text) {
        const qr = qrcode(0, "M");
        qr.addData(text);
        qr.make();
        return qr.createSvgTag({cellSize: 4, margin: 0, alt: "QR code to sign in with Microsoft"});
    }

    async function handleOfflineLoginClick() {
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

    function handleMicrosoftWebviewLoginClick() {
        const attempt = ++loginAttempt;
        view = "webview";

        invoke("login_microsoft_webview")
            .then((account) => {
                if (attempt !== loginAttempt) return;
                options.start.account = account;
                options.store();
            })
            .catch((err) => {
                if (attempt !== loginAttempt) return;
                reportMicrosoftError(err);
                view = "choose";
            });
    }

    function cancelWebviewLogin() {
        loginAttempt++;
        view = "choose";
    }

    function startDeviceCodeLogin() {
        const attempt = ++loginAttempt;
        view = "code";
        deviceCode = null;

        invoke("login_microsoft_device_code")
            .then((account) => {
                if (attempt !== loginAttempt) return;
                options.start.account = account;
                options.store();
            })
            .catch((err) => {
                if (attempt !== loginAttempt) return;
                reportMicrosoftError(err);
                view = "choose";
            });
    }

    function cancelDeviceCode() {
        loginAttempt++;
        deviceCode = null;
        view = "choose";
    }

    async function copyDeviceCode() {
        try {
            await writeText(deviceCode.userCode);
            codeCopied = true;
            setTimeout(() => codeCopied = false, 1600);
        } catch (err) {
            console.error("Failed to copy code:", err);
        }
    }

    function reportMicrosoftError(err) {
        alert(
            "Microsoft authentication failed.\n\n" +
             err + "\n\n" +
            "Should you be unable to resolve this issue, please use the 'Play offline' login option " +
            "and attempt to log in through the client's inbuilt account manager."
        );
    }

    listen("microsoft_device_code", (e) => {
        deviceCode = e.payload;
    });
</script>

<div class="modal">
    {#if view === "webview"}
        <div class="title">Microsoft sign-in</div>

        <div class="spinner"></div>
        <div class="hint">Finish signing in in the Microsoft window.</div>
        <div class="hint-subtle">Waiting for the account to be linked&hellip;</div>

        <ModalButton text="Cancel" primary={false} on:click={cancelWebviewLogin} />
    {:else if view === "code"}
        <div class="title">Sign in with a code</div>

        {#if deviceCode}
            <div class="code-box">
                <span class="code-text">{deviceCode.userCode}</span>
                <button class="copy-button" type="button" on:click={copyDeviceCode}>{codeCopied ? "COPIED" : "COPY"}</button>
            </div>

            <div class="hint">
                Open
                <button class="inline-link" type="button" on:click={() => openUrl(deviceCode.verificationUri)}>{deviceCode.verificationUri.replace('https://', '')}</button>
                on any device and enter the code above.
            </div>

            <div class="qr-row">
                <div class="qr-code">{@html deviceCodeQr}</div>
                <div class="qr-copy">
                    <span class="qr-copy-title">Or scan the code</span>
                    <span class="qr-copy-desc">Opens the page with the code already filled in.</span>
                </div>
            </div>

            <div class="waiting">
                <div class="waiting-dot"></div>
                <span>Waiting for confirmation</span>
            </div>
        {:else}
            <div class="hint">Requesting a sign-in code&hellip;</div>
        {/if}

        <ModalButton text="Cancel" primary={false} on:click={cancelDeviceCode} />
    {:else}
        <div class="title">Log in</div>

        <ModalButton text="Microsoft login" primary={true} on:click={handleMicrosoftWebviewLoginClick} />
        <ModalButton text="Microsoft device login" primary={false} on:click={startDeviceCodeLogin} />

        <div class="divider">or</div>

        <ModalInput placeholder="Username" icon="person" characterLimit={16} bind:value={offlineUsername} />
        <ModalButton text="Offline login" primary={false} on:click={handleOfflineLoginClick} />
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
        color: rgba(255, 255, 255, .6);
        font-size: 13px;
        text-align: center;
        line-height: 1.4;
        padding: 0 6px;
    }

    .hint-subtle {
        color: rgba(255, 255, 255, .4);
        font-size: 12px;
        text-align: center;
    }

    .inline-link {
        background: none;
        border: none;
        padding: 0;
        margin: 0;
        color: #4677FF;
        font-weight: 600;
        font-size: inherit;
        font-family: inherit;
        cursor: pointer;
        text-decoration: underline;
    }

    .spinner {
        align-self: center;
        width: 46px;
        height: 46px;
        border-radius: 50%;
        border: 3px solid rgba(255, 255, 255, .1);
        border-top-color: #4677FF;
        animation: spin .9s linear infinite;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }

    .code-box {
        display: flex;
        align-items: center;
        column-gap: 8px;
        height: 54px;
        padding: 0 8px 0 16px;
        border-radius: 8px;
        background: rgba(0, 0, 0, .26);
    }

    .code-text {
        flex: 1;
        min-width: 0;
        color: white;
        font-size: 21px;
        font-weight: 700;
        letter-spacing: 3px;
        white-space: nowrap;
    }

    .copy-button {
        flex: none;
        min-width: 76px;
        height: 38px;
        padding: 0 14px;
        border: none;
        border-radius: 6px;
        background: #4677FF;
        color: white;
        font-family: "Inter", sans-serif;
        font-size: 12px;
        font-weight: 700;
        letter-spacing: .4px;
        cursor: pointer;
        transition: ease background-color .2s;
    }

    .copy-button:hover {
        background: #3E69E2;
    }

    .qr-row {
        display: flex;
        align-items: center;
        column-gap: 14px;
    }

    .qr-code {
        flex: none;
        width: 84px;
        height: 84px;
        padding: 7px;
        border-radius: 8px;
        background: white;
        line-height: 0;
    }

    .qr-code :global(svg) {
        display: block;
        width: 100%;
        height: 100%;
    }

    .qr-copy {
        display: flex;
        flex-direction: column;
        row-gap: 6px;
    }

    .qr-copy-title {
        color: rgba(255, 255, 255, .85);
        font-size: 12.5px;
        font-weight: 700;
    }

    .qr-copy-desc {
        color: rgba(255, 255, 255, .45);
        font-size: 11.5px;
        line-height: 1.4;
    }

    .waiting {
        display: flex;
        align-items: center;
        justify-content: center;
        column-gap: 8px;
    }

    .waiting-dot {
        flex: none;
        width: 7px;
        height: 7px;
        border-radius: 50%;
        background: #4677FF;
        animation: pulse 1.4s ease-in-out infinite;
    }

    @keyframes pulse {
        0%, 100% { opacity: .35; transform: scale(.8); }
        50% { opacity: 1; transform: scale(1); }
    }

    .waiting span {
        color: rgba(255, 255, 255, .5);
        font-size: 11.5px;
        font-weight: 600;
    }
</style>
