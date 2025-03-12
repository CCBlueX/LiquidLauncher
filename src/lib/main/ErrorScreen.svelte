<script>
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import Logo from "../common/Logo.svelte";
    import ButtonClose from "../common/ButtonClose.svelte";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import ButtonCopyClipboard from "../common/ButtonCopyClipboard.svelte";
    
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
            <button 
                class="help-button quick-help" 
                on:click={async () => await openUrl('https://liquidbounce.net/docs/Tutorials/Fixing%20LiquidLauncher')}
            >
                Quick Help
            </button>
            <button 
                class="help-button contact" 
                on:click={async () => await openUrl('https://ccbluex.net/contact')}
            >
                Contact Support
            </button>
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
        border-radius: 8px;
        width: 100%;
        margin-bottom: 2rem;
        max-height: 300px;
        overflow: auto;
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

    .help-button {
        padding: 0.8rem 1.5rem;
        border-radius: 8px;
        font-weight: bold;
        cursor: pointer;
        transition: background-color 0.3s ease;
        border: none;
        color: white;
    }

    .quick-help {
        background-color: #4677ff;
    }

    .quick-help:hover {
        background-color: #3a63d2;
    }

    .contact {
        background-color: #45a049;
    }

    .contact:hover {
        background-color: #388e3c;
    }
</style>