<script>
    import ButtonClose from "../common/ButtonClose.svelte";
    import Logo from "../common/Logo.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import ButtonSetting from "../settings/ButtonSetting.svelte";
    import RangeSetting from "../settings/RangeSetting.svelte";
    import SelectSetting from "../settings/SelectSetting.svelte";
    import SettingsContainer from "../settings/SettingsContainer.svelte";
    import SettingWrapper from "../settings/SettingWrapper.svelte";
    import TextSetting from "../settings/TextSetting.svelte";
    import ToggleSetting from "../settings/ToggleSetting.svelte";
    import Account from "./Account.svelte";
    import ContentWrapper from "./ContentWrapper.svelte";
    import LaunchArea from "./LaunchArea.svelte";
    import ClientLog from "./log/ClientLog.svelte";
    import NewsArea from "./news/NewsArea.svelte";
    import ProgressStatus from "./statusbar/ProgressStatus.svelte";
    import StatusBar from "./statusbar/StatusBar.svelte";
    import TextStatus from "./statusbar/TextStatus.svelte";

    const versionInfo = {
        bannerUrl: "img/b73.jpg",
        title: "Lorem ipsum dolor sit amet",
        date: "2021-05-07",
        description:
            "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.\n\nAt vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.\n\nAt vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet.",
    };

    const lbVersion = {
        title: "b73",
        date: "2021-05-07",
    };

    const mcVersion = {
        title: "1.8.9",
        date: "2021-05-07",
    };

    let settingsShown = false;
    let versionSelectShown = false;
    let clientLogShown = false;

    const log = [];
</script>

{#if clientLogShown}
    <ClientLog messages={log} on:hideClientLog={() => clientLogShown = false} />
{/if}

{#if settingsShown}
    <SettingsContainer title="Settings" on:hideSettings={() => settingsShown = false}>
        <TextSetting title="JVM Arguments" placeholder="Arguments" value="-Xmx4G -XX:+UseConcMarkSweepGC -XX:+CMSIncrementalMode -XX:-UseAdaptiveSizePolicy -Xmn128M"></TextSetting>
        <TextSetting title="JVM Location" placeholder="Internal" value=""></TextSetting>
        <ToggleSetting title="Keep launcher running" value={true} />
        <RangeSetting title="Memory" min={0} max={4096} value={[1024, 3072]} valueSuffix="MB" step={1}></RangeSetting>
        <ButtonSetting text="Logout" />
        <SelectSetting title="Version" items={["Minecraft", "Roblox", "Axolotl", "Brot"]} value="Minecraft"></SelectSetting>
    </SettingsContainer>
{/if}

{#if versionSelectShown}
    <SettingsContainer title="Select version" on:hideSettings={() => versionSelectShown = false}>
        <SelectSetting title="Minecraft" items={["1.8.9", "1.12.2", "nextgen"]} value="1.8.9"></SelectSetting>
        <SelectSetting title="LiquidBounce" items={["b71", "b72", "b73"]} value="b73"></SelectSetting>
        <ToggleSetting title="Show nightly builds" value={false} />
        <SettingWrapper title="Additional mods">
            <ToggleSetting title="Sodium" value={true} />
            <ToggleSetting title="SuperAxolotl+" value={true} />
            <ToggleSetting title="Iridium" value={false} />
            <ToggleSetting title="Sodium" value={false} />
            <ToggleSetting title="SuperAxolotl+" value={true} />
            <ToggleSetting title="Iridium" value={false} />
            <ToggleSetting title="Sodium" value={true} />
            <ToggleSetting title="SuperAxolotl+" value={true} />
            <ToggleSetting title="Iridium" value={false} />
        </SettingWrapper>
    </SettingsContainer>
{/if}

<VerticalFlexWrapper blur={settingsShown || versionSelectShown || clientLogShown}>
    <TitleBar>
        <Logo />
        <StatusBar>
<!--             <TextStatus text="Welcome Kuqs, try out our new version!" /> -->
            <ProgressStatus value="35" max="100" text="Downloading assets..." />
        </StatusBar>
        <Account username="heafie" accountType="Mojang" on:showSettings={() => settingsShown = true} />
        <ButtonClose />
    </TitleBar>

    <ContentWrapper>
        <LaunchArea {versionInfo} {lbVersion} {mcVersion} on:showVersionSelect={() => versionSelectShown = true} on:showClientLog={() => clientLogShown = true} />
        <NewsArea />
    </ContentWrapper>
</VerticalFlexWrapper>
