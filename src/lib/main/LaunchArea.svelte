<script>
    import { createEventDispatcher } from "svelte";
    import ButtonLaunchArea from "./ButtonLaunchArea.svelte";
    import ButtonVersion from "./ButtonVersion.svelte";

    export let versionInfo;
    export let mcVersion;
    export let lbVersion;
    export let running;

    const dispatch = createEventDispatcher();
</script>

<div class="launch-area">
    <div class="version-info">
        <div class="banner" style="background-image: linear-gradient(to bottom, transparent, #4677ffc5), url({versionInfo.bannerUrl});">
            <div class="title">{versionInfo.title}</div>
            <div class="date">{versionInfo.date}</div>
        </div>
    </div>

    <pre class="description">
        {versionInfo.description}
    </pre>

    <div class="version-selector">
        <ButtonVersion icon="lb" title={lbVersion.title} date={lbVersion.date} on:click={() => dispatch("showVersionSelect")} />
        <ButtonVersion icon="mc" title={mcVersion.title} date={mcVersion.date} on:click={() => dispatch("showVersionSelect")} />
    </div> 

    {#if running}
        <div class="running-button-wrapper">
            <ButtonLaunchArea text="Terminate" active={true} on:click={() => dispatch("terminate")} /> 
            <ButtonLaunchArea text="Log" active={false} on:click={() => dispatch("showClientLog")} />  
        </div>
    {:else}
        <ButtonLaunchArea text="Launch LiquidBounce" active={false} on:click={() => dispatch("launch")} />
    {/if}
</div>

<style>
    .launch-area {
        flex-direction: column;
        display: flex;
        row-gap: 20px;
        max-height: 100%;
        overflow: hidden;
    }

    .version-info .banner {
        border-radius: 12px;
        height: 100px;
        background-size: cover;
        overflow: hidden;
        position: relative;
        display: flex;
        align-items: center;
    }

    .version-info .banner .title {
        font-weight: 800;
        color: white;
        font-size: 18px;
        max-width: 50%;
        margin-left: 20px;
    }

    .version-info .date {
        font-size: 12px;
        padding: 5px 12px;
        background-color: rgba(0, 0, 0, .58);
        color: white;
        border-radius: 4px;
        position: absolute;
        top: 8px;
        right: 8px;
    }

    .description {
        flex: 1;
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
        overflow: auto;
        white-space: pre-line;
        font-family: "Gilroy", sans-serif;
        font-weight: 400;
        font-size: 12px;
    }

    .version-selector {
        display: grid;
        grid-template-columns: 1fr 1fr;
        column-gap: 10px;
    }

    .running-button-wrapper {
        display: grid;
        grid-template-columns: 1fr max-content;
        column-gap: 10px;
    }
</style>