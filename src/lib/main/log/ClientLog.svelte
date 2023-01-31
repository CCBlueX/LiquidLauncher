<script>
    import { fly } from "svelte/transition";
    import VirtualList from "@sveltejs/svelte-virtual-list";
    import { afterUpdate, createEventDispatcher } from "svelte";
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import LogMessage from "./LogMessage.svelte";

    export let messages;

    let messageOutputElement;

    const dispatch = createEventDispatcher();
</script>

<div class="log" transition:fly={{ y: -10, duration: 200 }}>
    <div class="header">
        <div class="title">Client log</div>
        <button class="button-hide" on:click={() => dispatch("hideClientLog")}>
            <img class="icon" src="img/icon/icon-button-close.svg" alt="hide">
        </button>
    </div>

    <div class="output">
        <VirtualList items={messages} let:item>
            <LogMessage text={item} />
        </VirtualList>
    </div>

    <ToggleSetting title="Auto scroll" value={true} />
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

