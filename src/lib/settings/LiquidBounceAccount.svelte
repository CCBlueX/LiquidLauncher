<script>
    import {_} from "svelte-i18n";

    export let account;
</script>

<div class="liquidbounce-account">
    <div class="stat">
        <div class="stat-title">{$_("settings.liquidBounceAccount.username")}</div>
        <div class="stat-value">{account.nickname}</div>
    </div>
    <div class="stat">
        <div class="stat-title">{$_("settings.liquidBounceAccount.donatorPerks")}</div>
        <div class="stat-value">
            {#if account.premium}
                <span style="color: #60B675">{$_("settings.liquidBounceAccount.active")}</span>
            {:else}
                <span style="color: #FE4C2E">{$_("settings.liquidBounceAccount.inactive")}</span>
            {/if}
        </div>
    </div>
</div>

<style>
    .stat {
        color: white;
        display: flex;
    }

    .stat-title {
        margin-right: 5px;
    }

    .stat-title::after {
        content: ":";
    }
</style>