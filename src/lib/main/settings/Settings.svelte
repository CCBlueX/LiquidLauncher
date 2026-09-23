<script>
    import {createEventDispatcher} from "svelte";
    import GeneralSettings from "./GeneralSettings.svelte";
    import PremiumSettings from "./PremiumSettings.svelte";
    import SettingsContainer from "../../settings/SettingsContainer.svelte";
    import Tabs from "../../settings/tab/Tabs.svelte";
    import MinecraftSettings from "./MinecraftSettings.svelte";
    import ClientSettings from "./ClientSettings.svelte";

    export let client;
    export let options;
    export let versionState;
    export let activeTab = "General";

    const dispatch = createEventDispatcher();
</script>

<SettingsContainer
        title="Settings"
        fill={activeTab === "Client"}
        on:hideSettings={() => dispatch('hide')}
>
    <Tabs
            tabs={["General", "Minecraft", "Client", "Premium"]}
            bind:activeTab
            slot="tabs"
    />

    {#if activeTab === "General"}
        <GeneralSettings
                bind:options
        />
    {:else if activeTab === "Minecraft"}
        <MinecraftSettings
                bind:options
        />
    {:else if activeTab === "Client"}
        <ClientSettings
                {client}
                bind:options
                {versionState}
                on:updateData
                on:updateModStates
                on:updateMods
        />
    {:else if activeTab === "Premium"}
        <PremiumSettings
                {client}
                bind:options
        />
    {/if}
</SettingsContainer>
