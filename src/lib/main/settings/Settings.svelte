<script>
    import {createEventDispatcher} from "svelte";
    import GeneralSettings from "./GeneralSettings.svelte";
    import PremiumSettings from "./PremiumSettings.svelte";
    import VanillaIntegration from "./VanillaIntegration.svelte";
    import SettingsContainer from "../../settings/SettingsContainer.svelte";
    import Tabs from "../../settings/tab/Tabs.svelte";

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
            tabs={["General", "Minecraft", "Premium"]}
            bind:activeTab={activeSettingsTab}
            slot="tabs"
    />

    {#if activeSettingsTab === "General"}
        <GeneralSettings
                bind:options
        />
    {:else if activeSettingsTab === "Minecraft"}
        <VanillaIntegration
                bind:options
        />
    {:else if activeSettingsTab === "Premium"}
        <PremiumSettings
                {client}
                bind:options
        />
    {/if}
</SettingsContainer>