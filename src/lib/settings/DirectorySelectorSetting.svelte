<script>
    import {open as dialogOpen} from "@tauri-apps/plugin-dialog";
    import {revealItemInDir} from "@tauri-apps/plugin-opener";

    export let title;
    export let windowTitle;
    export let placeholder;
    export let value;

    async function handleDirectorySelect(e) {
        const selected = await dialogOpen({
            directory: true,
            multiple: false,
            defaultPath: value || placeholder,
            title: windowTitle,
        });

        if (selected) {
            value = selected;
        }
    }

    function handleDirectoryOpen(e) {
        revealItemInDir(value || placeholder);
    }
</script>

<div class="directory-selector-setting">
    <div class="title">{title}</div>
    <div class="wrapper">
        <input class="input" type="text" {placeholder} bind:value={value} />
        <button type="button" class="button-directory" title="Select directory" on:click={handleDirectorySelect}>
            <img src="img/icon/icon-directory-choose.svg" alt="choose">
        </button>
        <button type="button" class="button-directory" title="Open directory" on:click={handleDirectoryOpen}>
            <img src="img/icon/icon-directory-open.svg" alt="open">
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

    .button-directory {
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-items: center;
        background-color: transparent;
        border: none;
    }

    .button-directory img {
        height: 20px;
    }
</style>