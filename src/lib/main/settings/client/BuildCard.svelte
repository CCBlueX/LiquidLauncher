<script>
    import { createEventDispatcher } from "svelte";
    import ButtonSetting from "../../../settings/ButtonSetting.svelte";
    import RippleLoader from "../../../common/RippleLoader.svelte";
    import { count } from "./copy.js";

    /** Tells the build that launches and whether the marketplace was checked for it. */
    export let library;
    /** Why the library could not be read. */
    export let error;

    const dispatch = createEventDispatcher();

    $: notice = library?.notice ?? {};
    $: offline = notice.kind === "offline";

    function selectionLine(selection) {
        return selection.kind === "latest"
            ? selection.release ? "Latest release" : "Latest nightly"
            : `${selection.date} · ${selection.commit}`;
    }
</script>

{#if library}
    <div class="card openable" class:problem={offline} role="status">
        <img src="img/icon/icon-version-lb.png" alt="">
        <button class="open" type="button" on:click={() => dispatch("open")}>
            {#if library.liquidbounce}
                <span>LiquidBounce {library.liquidbounce}<span class="muted">{` · Minecraft ${library.minecraft}`}</span></span>
            {/if}
            {#if library.selection}
                <span class="line">{selectionLine(library.selection)}</span>
            {/if}
            {#if offline}
                <span class="line strong">Could not reach the marketplace.</span>
                <span class="line detail">{notice.error}</span>
            {:else if notice.kind === "restart"}
                <span class="line strong">Restart LiquidBounce to apply {count(notice.changes, "change", "changes")}.</span>
            {/if}
        </button>
        <div class="side">
            {#if notice.kind === "checking"}
                <RippleLoader size={30} />
            {:else if offline}
                <ButtonSetting small text="Try again" on:click={() => dispatch("retry")} />
            {/if}
            <img class="chevron" src="img/icon/icon-next.svg" alt="" aria-hidden="true">
        </div>
    </div>
{:else if error}
    <div class="card problem" role="status">
        <img src="img/icon/icon-version-lb.png" alt="">
        <div class="line detail">{error}</div>
        <div class="side">
            <ButtonSetting small text="Try again" on:click={() => dispatch("retry")} />
        </div>
    </div>
{/if}

<style>
    .card {
        position: relative;
        display: grid;
        grid-template-columns: 30px minmax(0, 1fr) max-content;
        column-gap: 10px;
        align-items: center;
        background-color: rgba(0, 0, 0, .26);
        border-radius: 6px;
        padding: 10px;
        border: solid 1px transparent;
        transition: ease border-color .2s;
    }

    .card.openable:hover, .card.openable:focus-within {
        border-color: #4677FF;
    }

    .card.problem {
        border-bottom-color: #B83529;
    }

    .card > img {
        width: 30px;
        height: 30px;
    }

    .open {
        display: grid;
        row-gap: 2px;
        text-align: left;
        background: transparent;
        border: none;
        padding: 0;
        color: white;
        font-family: "Inter", sans-serif;
        font-size: 14px;
        cursor: pointer;
        min-width: 0;
    }

    .open::after {
        content: "";
        position: absolute;
        inset: 0;
    }

    .muted, .line {
        color: rgba(255, 255, 255, .5);
    }

    .line {
        font-size: 12px;
        line-height: 15px;
    }

    .line.strong {
        color: white;
    }

    .detail {
        word-break: break-word;
    }

    .side {
        position: relative;
        z-index: 1;
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .chevron {
        height: 10px;
        opacity: .5;
    }
</style>
