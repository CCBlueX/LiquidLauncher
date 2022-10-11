<script>
    export let accountName;
    export let accountType;
    export let avatarUrl;

    export let showOptions;
</script>

<div class="account">
    <img class="avatar" src={avatarUrl} alt="avatar">
    <div class="details">
        <div class="account-name">{accountName}</div>
        <div class="account-type">{accountType}</div>
    </div>
    <img class="change-account" src="img/icon/icon-cog.svg" alt="settings" on:click={showOptions}>
</div>

<style>
    .account {
        flow: horizontal;
        border-spacing: 8px;
        min-width: 193px;
        background-color: rgba(0, 0, 0, .36);
        border-radius: 6px;
        padding: 0 15px;
        height: 50px;
        margin-right: 18px;
    }

    .avatar {
        height: 35px;
        width: 35px;
        border-radius: 50%;
        vertical-align: middle;
    }

    .details {
        vertical-align: middle;
        width: 1*;
        font-size: 12px;
    }

    .account-name {
        color: white;
        font-weight: 800;
    }

    .account-type {
        color: rgba(255, 255, 255, .5);
        font-weight: 500;
    }

    .change-account {
        behavior: button;
        vertical-align: middle;
    }
</style>