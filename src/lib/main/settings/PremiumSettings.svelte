<script>
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import SettingWrapper from "../../settings/SettingWrapper.svelte";
    import LiquidBounceAccount from "../../settings/LiquidBounceAccount.svelte";
    import Description from "../../settings/Description.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import {invoke} from "@tauri-apps/api/core";
    import {openUrl} from "@tauri-apps/plugin-opener";

    export let client;
    export let options;
    export let clientAccount;

    async function login() {
        try {
            clientAccount = await invoke("client_account_authenticate", {
                client,
                options
            });
        } catch (error) {
            console.error("Failed to authenticate client account:", error);
            alert(`Failed to authenticate client account: ${error}`);
        }
    }

    async function logout() {
        try {
            await invoke("client_account_logout", {options});
            clientAccount = null;
        } catch (error) {
            console.error("Failed to log out of client account:", error);
            alert(`Failed to log out of client account: ${error}`);
        }
    }
</script>

<ToggleSetting
        title="Skip Advertisements"
        disabled={!clientAccount || !clientAccount.premium}
        bind:value={options.premium.skipAdvertisement}
/>

{#if clientAccount}
    <SettingWrapper title="Account Information">
        <LiquidBounceAccount account={clientAccount} />
    </SettingWrapper>

    {#if !clientAccount.premium}
        <Description
                description="There appears to be no premium associated with this account. Please link it on the account management page."
        />
    {/if}

    <ButtonSetting
            text="Manage Account"
            on:click={() => openUrl("https://user.liquidbounce.net")}
            color="#4677FF"
    />
    <ButtonSetting
            text="Logout"
            on:click={logout}
            color="#B83529"
    />
{:else}
    <Description
            description="By going premium, you not only support the ongoing development of the client but also receive a cape and the ability to bypass ads on the launcher."
    />

    <ButtonSetting
            text="Login with LiquidBounce Account"
            on:click={login}
            color="#4677FF"
    />
{/if}
