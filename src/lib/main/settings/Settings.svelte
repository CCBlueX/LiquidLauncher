<script>
    import {createEventDispatcher} from "svelte";
    import GeneralSettings from "./GeneralSettings.svelte";
    import PremiumSettings from "./PremiumSettings.svelte";
    import SettingsContainer from "../../settings/SettingsContainer.svelte";
    import Tabs from "../../settings/tab/Tabs.svelte";
    import MinecraftSettings from "./MinecraftSettings.svelte";
    import MarketplaceSettings from "./MarketplaceSettings.svelte";

    export let client;
    export let options;
    let activeSettingsTab = "General";

    const dispatch = createEventDispatcher();
</script>

<SettingsContainer
        title="Settings"
        on:hideSettings={() => dispatch('hide')}
>
    <Tabs
            tabs={["General", "Minecraft", "Marketplace", "Premium"]}
            bind:activeTab={activeSettingsTab}
            slot="tabs"
    />

    {#if activeSettingsTab === "General"}
        <GeneralSettings
                bind:options
        />
    {:else if activeSettingsTab === "Minecraft"}
        <MinecraftSettings
                bind:options
        />
    {:else if activeSettingsTab === "Marketplace"}
        <MarketplaceSettings
                {client}
        />
    {:else if activeSettingsTab === "Premium"}
        <PremiumSettings
                {client}
                bind:options
        />
    {/if}
</SettingsContainer>