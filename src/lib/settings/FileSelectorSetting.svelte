<script>
    import {open as dialogOpen} from "@tauri-apps/plugin-dialog";

    export let title;
    export let placeholder;
    export let value;
    export let filters;
    export let windowTitle;

    async function handleFileSelect(e) {
        const selected = await dialogOpen({
            directory: false,
            multiple: false,
            filters,
            defaultPath: value || placeholder,
            title: windowTitle,
        });

        if (selected) {
            value = selected;
        }
    }
</script>

<div class="file-selector-setting">
    <div class="title">{title}</div>
    <div class="wrapper">
        <input class="input" type="text" {placeholder} bind:value={value} />
        <button type="button" class="button-file" title="Select file" on:click={handleFileSelect}>
            <img src="img/icon/icon-file-choose.svg" alt="choose">
        </button>
    </div>
</div>

<style>
    .title {
        color: white;
        margin-bottom: 5px;
    }

    .input {
        width: 100%;
        background-color: rgba(0, 0, 0, .26);
        border: none;
        border-bottom: solid 1px #4677FF;
        color: white;
        font-family: "Inter", sans-serif;
        padding: 5px;
        border-radius: 3px;
    }

    .wrapper {
        display: flex;
        column-gap: 10px;
    }

    .button-file {
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-items: center;
        background-color: transparent;
        border: none;
    }

    .button-file img {
        height: 20px;
    }
</style>