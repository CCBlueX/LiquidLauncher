<script>
    import { createEventDispatcher } from "svelte";
    import { slide } from "svelte/transition";

    export let instances = [];
    export let instance = null;
    export let lbVersion;
    export let mcVersion;

    const dispatch = createEventDispatcher();
    const duration = matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 150;
    const states = {
        running: "Running",
        blocked: "Blocked",
        starting: "Starting",
        started: "Launched",
        attaching: "Attaching",
        attached: "Attached",
        detaching: "Detaching"
    };

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
    <legend>Instance</legend>
    {#each [null, ...instances] as choice (choice ? choice.pid ?? `starting ${choice.id}` : 0)}
        {@const isBlocked = choice?.state === "blocked"}
        {@const isChosen = choice === instance}
        <div class="card" class:chosen={isChosen} class:blocked={isBlocked} class:own={choice?.id} use:reveal={isChosen}>
            <label title={isBlocked ? "This JVM was started with attaching disabled" : choice?.directory}>
                <input type="radio" name="instance" disabled={isBlocked} checked={isChosen} on:change={() => instance = choice}>
                {#if choice}
                    <span class="title">{choice.version ? `Minecraft ${choice.version}` : "Minecraft"}</span>
                    <span class="state">{states[choice.state]}</span>
                    <span class="identity">{[choice.profile, choice.username].filter(Boolean).join(" · ")}</span>
                    {#if choice.pid}
                        <span class="pid">PID {choice.pid}</span>
                    {/if}
                {:else}
                    <span class="title">New instance</span>
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
            {:else if isChosen && choice.lbVersion}
                <div class="versions" transition:slide={{duration}}>
                    <img src="img/icon/icon-version-lb.png" alt="LiquidBounce">
                    <span>{choice.lbVersion}</span>
                </div>
            {:else if isChosen && choice.unsupported}
                <p class="unsupported" transition:slide={{duration}}>{choice.unsupported}</p>
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

    .card.blocked {
        opacity: .6;
        border-color: transparent;
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

    .blocked label {
        cursor: not-allowed;
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

    .own .state {
        color: white;
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

    .unsupported {
        padding: 0 12px 12px;
        font-size: 11px;
        color: rgba(255, 255, 255, .7);
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
