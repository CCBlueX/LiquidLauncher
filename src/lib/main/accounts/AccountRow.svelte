<script>
    import {createEventDispatcher} from "svelte";
    import Avatar from "./Avatar.svelte";
    import ToolTip from "../../common/ToolTip.svelte";

    export let entry;
    export let active;
    // The first row has no room above it for tooltips.
    export let tooltipsBelow = false;

    const dispatch = createEventDispatcher();
</script>

<div class="row" class:active aria-current={active ? "true" : undefined}>
    <div class="avatar-wrapper">
        <Avatar uuid={entry.uuid} size={35} />
        {#if entry.favorite}
            <img class="favorite-mark" src="img/icon/icon-favorite-mark.svg" alt="Favorite">
        {/if}
    </div>

    <div class="details">
        <div class="title">
            <span class="name">{entry.name}</span>
            <span class="type">{entry.type}</span>
        </div>
        <span class="uuid">{entry.uuid}</span>
    </div>

    <!-- The active row is filled like a hovered one, having no buttons sets it apart. -->
    {#if !active}
        <div class="actions">
            <button class="icon-button on-hover" type="button" aria-label="Delete" on:click={() => dispatch("remove")}>
                <ToolTip text="Delete" below={tooltipsBelow} />
                <img src="img/icon/icon-trash.svg" alt="">
            </button>
            <button class="icon-button on-hover" type="button" aria-label="Favorite" aria-pressed={entry.favorite} on:click={() => dispatch("favorite")}>
                <ToolTip text="Favorite" below={tooltipsBelow} />
                <img src="img/icon/{entry.favorite ? 'icon-favorite-filled.svg' : 'icon-favorite.svg'}" alt="">
            </button>
            <button class="icon-button" type="button" aria-label="Play" on:click={() => dispatch("play")}>
                <ToolTip text="Play" below={tooltipsBelow} />
                <img src="img/icon/icon-play.svg" alt="">
            </button>
        </div>
    {/if}
</div>

<style>
    .row {
        display: flex;
        align-items: center;
        column-gap: 8px;
        padding: 8px;
        border-radius: 6px;
        transition: ease background-color .12s;
    }

    .row.active,
    .row:hover {
        background-color: #4677FF;
    }

    .avatar-wrapper {
        position: relative;
        flex: none;
    }

    .favorite-mark {
        position: absolute;
        top: -2px;
        right: -3px;
        height: 12px;
    }

    .details {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        row-gap: 2px;
    }

    .title {
        display: flex;
        align-items: baseline;
        column-gap: 8px;
    }

    .name {
        color: white;
        font-size: 14px;
        font-weight: 600;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .type {
        flex: none;
        color: rgba(255, 255, 255, .5);
        font-size: 12px;
    }

    .uuid {
        color: rgba(255, 255, 255, .5);
        font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
        font-size: 11px;
        white-space: nowrap;
    }

    .row.active .type,
    .row.active .uuid,
    .row:hover .type,
    .row:hover .uuid {
        color: rgba(255, 255, 255, .8);
    }

    .actions {
        display: flex;
        align-items: center;
        column-gap: 10px;
    }

    .icon-button {
        position: relative;
        display: flex;
        align-items: center;
        padding: 0;
        border: none;
        background-color: transparent;
        cursor: pointer;
    }

    .icon-button img {
        height: 20px;
    }

    .on-hover {
        display: none;
    }

    .row:hover .on-hover {
        display: flex;
    }
</style>
