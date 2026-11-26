<script>
    import ToggleSetting from "../../settings/ToggleSetting.svelte";
    import SettingWrapper from "../../settings/SettingWrapper.svelte";
    import LiquidBounceAccount from "../../settings/LiquidBounceAccount.svelte";
    import Description from "../../settings/Description.svelte";
    import ButtonSetting from "../../settings/ButtonSetting.svelte";
    import {invoke} from "@tauri-apps/api/core";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import {_} from "svelte-i18n";

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
            alert($_("settings.premium.loginFailed", { values: { error: `${error}` } }));
        }
    }

    async function logout() {
        try {
            await invoke("client_account_logout", {options});
            clientAccount = null;
        } catch (error) {
            console.error("Failed to log out of client account:", error);
            alert($_("settings.premium.logoutFailed", { values: { error: `${error}` } }));
        }
    }
</script>

<ToggleSetting
        title={$_("settings.premium.skipAds")}
        disabled={!clientAccount || !clientAccount.premium}
        bind:value={options.premium.skipAdvertisement}
/>

{#if clientAccount}
    <SettingWrapper title={$_("settings.premium.accountInformation")}>
        <LiquidBounceAccount account={clientAccount} />
    </SettingWrapper>

    {#if !clientAccount.premium}
        <Description
                description={$_("settings.premium.noPremium")}
        />
    {/if}

    <ButtonSetting
            text={$_("settings.premium.manageAccount")}
            on:click={() => openUrl("https://user.liquidbounce.net")}
            color="#4677FF"
    />
    <ButtonSetting
            text={$_("settings.premium.logout")}
            on:click={logout}
            color="#B83529"
    />
{:else}
    <Description
            description={$_("settings.premium.description")}
    />

    <ButtonSetting
            text={$_("settings.premium.login")}
            on:click={login}
            color="#4677FF"
    />
{/if}
