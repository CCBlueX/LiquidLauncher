<script>
    import { createEventDispatcher } from "svelte";
    import { slide } from "svelte/transition";
    import { _ } from "svelte-i18n";

    export let instances = [];
    export let instance = null;
    export let lbVersion;
    export let mcVersion;

    const dispatch = createEventDispatcher();
    const duration = matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 150;

    function reveal(card, isChosen) {
        const update = isChosen => {
            if (!isChosen) return;
            card.scrollIntoView({ block: "nearest" });
            setTimeout(() => card.scrollIntoView({ block: "nearest" }), duration);
        };
        update(isChosen);
        return { update };
    }
</script>

<fieldset>
    <legend>{$_("launch.instance")}</legend>
    {#each [null, ...instances] as choice (choice?.id ?? 0)}
        {@const isChosen = choice === instance}
        <div class="card" class:chosen={isChosen} use:reveal={isChosen}>
            <label>
                <input type="radio" name="instance" checked={isChosen} on:change={() => instance = choice}>
                {#if choice}
                    <span class="title">Minecraft {choice.version}</span>
                    <span class="state">{$_(`launch.state.${choice.state}`)}</span>
                    <span class="identity">{choice.username}</span>
                    {#if choice.pid}
                        <span class="pid">PID {choice.pid}</span>
                    {/if}
                {:else}
                    <span class="title">{$_("launch.newInstance")}</span>
                {/if}
            </label>
            {#if isChosen && !choice}
                <button class="versions" type="button" on:click={() => dispatch("showVersionSelect")} transition:slide={{duration}}>
                    <img src="img/icon/icon-version-lb.png" alt="LiquidBounce">
                    <span>{lbVersion}</span>
                    <img src="img/icon/icon-version-mc.png" alt="Minecraft">
                    <span>1.7 - {mcVersion}</span>
                    <img class="chevron" src="img/icon/icon-next.svg" alt="">
                </button>
            {:else if isChosen}
                <div class="versions" transition:slide={{duration}}>
                    <img src="img/icon/icon-version-lb.png" alt="LiquidBounce">
                    <span>{choice.lbVersion}</span>
                </div>
            {/if}
        </div>
    {/each}
</fieldset>

<style>
    fieldset {
        display: grid;
        row-gap: 6px;
        margin: 0;
        padding: 0;
        border: none;
        min-width: 0;
    }

    legend {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
        white-space: nowrap;
    }

    .card {
        background-color: rgba(0, 0, 0, .26);
        border: solid 1px transparent;
        border-radius: 6px;
        transition: ease border-color .2s, ease background-color .2s;
    }

    .card:hover {
        border-color: rgba(70, 119, 255, .5);
    }

    .card.chosen {
        border-color: #4677FF;
        background-color: rgba(70, 119, 255, .16);
    }

    .card:has(:focus-visible) {
        outline: solid 2px white;
        outline-offset: -2px;
    }

    label {
        display: grid;
        grid-template-columns: minmax(0, 1fr) max-content;
        column-gap: 10px;
        row-gap: 4px;
        align-items: center;
        padding: 10px 12px;
        cursor: pointer;
    }

    input {
        position: absolute;
        opacity: 0;
        pointer-events: none;
    }

    .title {
        font-size: 12px;
        color: white;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .chosen .title {
        font-weight: 600;
    }

    .identity, .state, .pid {
        font-size: 10px;
        color: rgba(255, 255, 255, .5);
    }

    .identity {
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }

    .state, .pid {
        justify-self: end;
    }

    .versions {
        display: grid;
        grid-template-columns: max-content max-content max-content minmax(0, 1fr) max-content;
        column-gap: 8px;
        align-items: center;
        width: 100%;
        padding: 0 12px 12px;
        background: none;
        border: none;
        font-family: "Inter", sans-serif;
        font-size: 12px;
        color: white;
        text-align: left;
        cursor: pointer;
    }

    div.versions {
        cursor: default;
    }

    .versions img {
        height: 18px;
    }

    .versions img:nth-of-type(2) {
        margin-left: 12px;
    }

    .versions span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .versions .chevron {
        height: 10px;
        opacity: .5;
    }
</style>
