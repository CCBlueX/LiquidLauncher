<script>
    import {exit} from "@tauri-apps/plugin-process";

    async function handleClick() {
        await exit(0);
    }
</script>

<button class="button-close" type="button" on:click={handleClick}>
    <img class="icon" src="img/icon/icon-button-close.svg" alt="close">
</button>

<style>
    .button-close {
        border-radius: 50%;
        height: 48px;
        width: 48px;
        background-color: transparent;
        border: solid 2px rgba(255, 255, 255, .5);
        transition: ease background-color .2s;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
    }
    
    .button-close:hover {
        background-color: rgba(255, 255, 255, .1);
    }
</style>