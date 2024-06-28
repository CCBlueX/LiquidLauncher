<script>
    import { jwtDecode } from "jwt-decode";

    export let account;

    const decodedJwt = jwtDecode(account.accessToken);

    console.log(decodedJwt)
    
    let username = "Senk Ju";
    let premium = true;
</script>

<div class="liquidbounce-account">
    <div class="title">Account Information</div>
    <div class="stat">
        <div class="stat-title">Username</div>
        <div class="stat-value">{decodedJwt.nickname}</div>
    </div>
    <div class="stat">
        <div class="stat-title">Donator Perks</div>
        <div class="stat-value">
            {#if account.premium}
                <span style="color: #60B675">Active</span>
            {:else}
                <span style="color: #FE4C2E">Inactive</span>
            {/if}
        </div>
    </div>
</div>

<style>
    .title {
        color: white;
        font-weight: 500;
        margin-bottom: 5px;
    }

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