<script>
    import ButtonIcon from "./ButtonIcon.svelte";
    import ButtonIconText from "./ButtonIconText.svelte";
</script>

<div class="social-bar">
    <ButtonIcon icon="nodebb" text="Forum" url="https://forum.liquidbounce.net/"/>
    <ButtonIcon icon="github" text="GitHub" url="https://github.com/CCBlueX/"/>
    <ButtonIcon icon="discord" text="Discord" url="https://liquidbounce.net/discord" />
    <ButtonIcon icon="twitter" text="Twitter" url="https://twitter.com/ccbluex"/>
    <ButtonIcon icon="youtube" text="YouTube" url="https://www.youtube.com/ccbluex" />
    <ButtonIconText icon="url" text="liquidbounce.net" url="https://liquidbounce.net" />
</div>

<style>
    .social-bar {
        background-color: rgba(0, 0, 0, .26);
        border-radius: 6px;
        display: flex;
        width: max-content;
        padding: 0 20px;
        padding: 8px;
        column-gap: 15px;
        align-items: center;
    }
</style>
