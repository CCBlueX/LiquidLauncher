<script>
    import { createEventDispatcher } from "svelte";

    export let username;
    export let uuid;
    export let accountType;

    const dispatch = createEventDispatcher();
</script>

<div class="account">
    <object data="https://crafatar.com/avatars/{uuid}?size=100" type="image/png" class="avatar">
        <img src="img/steve.png" alt=avatar class="avatar"/>
    </object>
    <div class="details">
        <div class="username">{username}</div>
        <div class="type">{accountType}</div>
    </div>
    <button class="button-settings" type="button" on:click={() => dispatch("showSettings")} >
        <img class="icon" src="img/icon/icon-cog.svg" alt="settings">
    </button>
</div>

<style>
    .account {
        display: grid;
        grid-template-columns: max-content auto max-content;
        background-color: rgba(0, 0, 0, .36);
        height: 50px;
        border-radius: 6px;
        padding: 8px 12px;
        gap: 8px;
        align-items: center;
        min-width: 200px;
    }

    .avatar {
        border-radius: 50%;
        height: 35px;
        overflow: hidden;
    }

    .details {
        font-size: 11px;
    }

    .details .username {
        color: white;
        font-weight: bold;
    }

    .details .type {
        color: rgba(255, 255, 255, .5);
        font-weight: bold;
    }

    .button-settings {
        background-color: transparent;
        border: none;
        display: flex;
        align-items: center;
        cursor: pointer;
    }
</style>