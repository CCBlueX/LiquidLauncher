<script>
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import Logo from "../common/Logo.svelte";
    import ButtonClose from "../common/ButtonClose.svelte";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import ButtonCopyClipboard from "../common/ButtonCopyClipboard.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    
    export let error = {
        message: "Unknown error",
        error: null
    };
</script>

<VerticalFlexWrapper blur={false}>
    <TitleBar>
        <Logo />
        <ButtonClose />
    </TitleBar>

    <div class="error-container">
        <h1>Error Occurred</h1>
        <p class="error-message">{error.message}</p>
        
        {#if error.error}
            <div class="error-details">
                <div class="error-header">
                    <h3>Technical Details:</h3>
                    <div class="copy-button">
                        <ButtonCopyClipboard textToCopy={JSON.stringify(error)} iconOnly={true} />
                    </div>
                </div>
                <pre>{error.error}</pre>
            </div>
        {/if}

        <div class="help-buttons">
            <ButtonSetting text="Quick Help" color="#4677ff" on:click={() => openUrl('https://liquidbounce.net/docs/Tutorials/Fixing%20LiquidLauncher')}></ButtonSetting>
            <ButtonSetting text="Contact Support" color="#45a049" on:click={() => openUrl('https://ccbluex.net/contact')}></ButtonSetting>
        </div>
    </div>
</VerticalFlexWrapper>

<style>
    .error-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 2rem;
        max-width: 800px;
        margin: 0 auto;
        color: white;
        flex: 1;
    }

    h1 {
        color: #ff4444;
        font-size: 2.5rem;
        margin-bottom: 1rem;
    }

    .error-message {
        font-size: 1.2rem;
        margin-bottom: 2rem;
        text-align: center;
    }

    .error-details {
        background-color: rgba(0, 0, 0, 0.3);
        padding: 1rem;
        border-radius: 6px;
        width: 100%;
        margin-bottom: 2rem;
        max-height: 300px;
        overflow: auto;
    }

    .error-details pre {
        user-select: all;
    }

    .error-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 0.5rem;
    }
    
    .copy-button {
        flex-shrink: 0;
    }

    .error-details pre {
        white-space: pre-wrap;
        word-break: break-all;
    }

    .help-buttons {
        display: flex;
        gap: 1rem;
        margin-top: 1rem;
    }
</style>