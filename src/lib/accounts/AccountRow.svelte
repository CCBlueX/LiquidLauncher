<script>
    import Avatar from "./Avatar.svelte";

    export let entry;
</script>

<div class="row" class:active={entry.active}>
    <button class="choose" type="button" aria-current={entry.active} on:click>
        <Avatar uuid={entry.uuid} size={30} />
        <span class="details">
            <span class="name">{entry.name}</span>
            <span class="type">{entry.type}</span>
        </span>
    </button>
    <slot />
</div>

<style>
    .row {
        position: relative;
        display: flex;
        align-items: center;
        column-gap: 10px;
        padding: 6px 10px;
        border: solid 1px transparent;
        border-radius: 6px;
        transition: ease border-color .2s, ease background-color .2s;
    }

    .row:hover {
        border-color: rgba(70, 119, 255, .5);
    }

    .row.active {
        border-color: #4677FF;
        background-color: rgba(70, 119, 255, .16);
    }

    .row:has(.choose:focus-visible) {
        outline: solid 2px white;
        outline-offset: -2px;
    }

    .choose {
        flex: 1;
        display: flex;
        align-items: center;
        column-gap: 10px;
        min-width: 0;
        padding: 0;
        border: none;
        background: none;
        font-family: "Inter", sans-serif;
        text-align: left;
        cursor: pointer;
    }

    /* The whole row chooses the account, the buttons in the slot sit above. */
    .choose::after {
        content: "";
        position: absolute;
        inset: 0;
    }

    .details {
        display: grid;
        min-width: 0;
    }

    .name {
        color: white;
        font-size: 12px;
        font-weight: 600;
        line-height: 15px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .type {
        color: rgba(255, 255, 255, .5);
        font-size: 11px;
        line-height: 14px;
    }
</style>
