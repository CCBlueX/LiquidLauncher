<script>
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";

    export let text = "Copy to clipboard";
    export let textToCopy;
    export let iconOnly = false;

    let copied = false;

    async function handleCopy() {
        try {
            writeText(textToCopy);
            copied = true;
            setTimeout(() => copied = false, 2000);
        } catch (err) {
            console.error('Failed to copy text:', err);
        }
    }
</script>

<button class="button {iconOnly ? 'icon-only' : ''}" on:click={handleCopy} title={iconOnly ? (copied ? 'Copied!' : text) : ''}>
    <div class="icon">
        <img src="img/icon/icon-copy-to-clipboard.svg" alt="Copy icon" class={iconOnly ? 'white-icon' : ''}>
    </div>
    {#if !iconOnly}
        <div class="text">{copied ? 'Copied!' : text}</div>
    {/if}
</button>

<style>
    .button {
        display: grid;
        grid-template-columns: max-content 1fr max-content;
        border-radius: 3px;
        align-items: center;
        column-gap: 10px;
        overflow: hidden;
        background: linear-gradient(to left, rgba(0, 0, 0, .36) 50%, #4677ff 50%);
        background-size: 200% 100%;
        background-position: right bottom;
        will-change: background-position;
        transition: background-position .2s ease-out;
        border: none;
        cursor: pointer;
    }

    .button:hover {
        background-position: left bottom;
    }
    
    .icon-only {
        grid-template-columns: max-content;
        width: 30px;
        height: 30px;
        border-radius: 4px;
        padding: 3px;
        background: transparent;
    }
    
    .icon-only:hover {
        background: rgba(255, 255, 255, 0.2);
    }

    .icon {
        height: 25px;
        width: 25px;
        background-color: #4677FF;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .icon-only .icon {
        background-color: transparent;
    }
    
    .white-icon {
        filter: brightness(0) invert(1);
    }

    .text {
        font-size: 14px;
        font-weight: 500;
        color: white;
    }
</style>