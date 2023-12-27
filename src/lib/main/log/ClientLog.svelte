<script>
    import { fly } from "svelte/transition";
    import VirtualList from "./VirtualList.svelte";
    import { createEventDispatcher } from "svelte";
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte"
    import LogMessage from "./LogMessage.svelte";

    export let messages;

    let autoScroll = true;

    const dispatch = createEventDispatcher();

    async function handleUploadSetting(e) {
        const log = messages.join("");

        if (await confirm("The entire log of this session will be uploaded. It may contain sensitive information like private chat messages. Are you sure you want to proceed?") !== true) {
            return;
        }

        const response = await fetch("https://paste.ccbluex.net/api.php", {
            body: `content=${log}`,
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
            },
            method: "POST"
        });

        const responseData = await response.text();

        if (response.status !== 200) {
            alert(`Failed to upload log: ${responseData}`);
            return;
        }

        prompt("Your log is available at the following URL: ", responseData);
    }
</script>

<div class="log" transition:fly={{ y: -10, duration: 200 }}>
    <div class="header">
        <div class="title">Client log</div>
        <button class="button-hide" on:click={() => dispatch("hideClientLog")}>
            <img class="icon" src="img/icon/icon-button-close.svg" alt="hide">
        </button>
    </div>

    <div class="output">
        <VirtualList items={messages} let:item {autoScroll}>
            <LogMessage text={item} />
        </VirtualList>
    </div>

    <div class="settings">
        <ButtonSetting text="Upload log" color="#4677FF" on:click={handleUploadSetting}></ButtonSetting>
        <ToggleSetting title="Auto scroll" disabled={false} bind:value={autoScroll} />
    </div>
</div>

<style>
    .log {
        width: calc(100% - 150px);
        height: calc(100% - 150px);
        position: fixed;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        background-color: rgba(0, 0, 0, 0.58);
        backdrop-filter: blur(10px);
        padding: 25px;
        border-radius: 6px;
        z-index: 1000;
        display: flex;
        flex-direction: column;
        box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
    }

    .settings {
        display: flex;
        gap: 20px;
        align-items: center;
    }

    .output {
        flex: 1;
        overflow: hidden;
        margin-bottom: 10px;
    }

    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 20px;
    }

    .title {
        font-size: 16px;
        color: white;
    }

    .button-hide {
        background-color: transparent;
        border: none;
        cursor: pointer;
    }
</style>
