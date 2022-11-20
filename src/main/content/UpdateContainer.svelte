<script>
    import SocialBar from "./social-bar/SocialBar.svelte";
    import { invoke } from '@tauri-apps/api/tauri'
    
    export let updateData;
    export let ignoreUpdate;
</script>

<div class="wrapper">
    <div class="update">
        <h1>A new launcher update has been found!</h1>

        <label>Version: {updateData.name}</label><br>
        <label>Published: {updateData.published_at}</label><br>
        <button class="download" on:click={invoke("open", { url: updateData.html_url })}>Download</button>
        <button class="ignore" on:click={ignoreUpdate}>Ignore</button>
    </div>

    <SocialBar />
</div>

<style>
    label, h1, h2, ul, li {
        color: white;
    }

    .wrapper {
        width: 1*;
        height: 1*;
        border-spacing: 24px;
        overflow: hidden;
    }

    .update {
        background-color: rgba(0, 0, 0, .36);
        padding: 24px;
        border-radius: 6px;
        height: 1*;
        border-spacing: 20px;
    }

    button {
        margin-top: 0.5rem;
        margin-bottom: 0.5rem;
        background: unset;
        border: none;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        width: 10rem;
        display: block;
        height: 2rem;
        transition: background-color ease .2s;
        font-family: "Gilroy",serif;
    }

    .download {
        background-color: #4677FF;
    }

    .download:hover {
        background-color: #3c77ff;
    }

    .ignore {
        background-color: #f83939;
    }

    .ignore:hover {
        background-color: #e23e4c;
    }
</style>