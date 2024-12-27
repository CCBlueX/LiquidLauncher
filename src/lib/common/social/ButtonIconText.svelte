<script>
    import {openUrl} from "@tauri-apps/plugin-opener";

    export let text;
    export let icon;
    export let url;

    async function handleClick(e) {
        await openUrl(url);
    }
</script>

<button class="button" on:click={handleClick}>
    <div class="icon">
        <img src="img/icon/social/icon-{icon}.svg" alt="icon"> 
    </div>
    <div class="text">{text}</div>
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

    .icon {
        height: 25px;
        width: 25px;
        background-color: #4677FF;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .text {
        font-size: 14px;
        font-weight: 500;
        color: white;
    }
</style>