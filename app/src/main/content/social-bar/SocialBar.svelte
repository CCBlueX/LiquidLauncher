<script>
    import ButtonSocial from "./ButtonSocial.svelte";
    import ButtonText from "./ButtonText.svelte";
</script>

<div class="social-bar">
    <ButtonSocial icon="nodebb" text="Forum" url="https://forums.ccbluex.net/"/>
    <ButtonSocial icon="github" text="GitHub" url="https://github.com/CCBlueX/"/>
    <ButtonSocial icon="guilded" text="Guilded" url="https://github.com/ccbluex" />
    <ButtonSocial icon="twitter" text="Twitter" url="https://twitter.com/ccbluex"/>
    <ButtonSocial icon="youtube" text="YouTube" url="https://www.youtube.com/ccbluex" />
    <ButtonText icon="url" text="liquidbounce.net" url="https://liquidbounce.net/" />
</div>

<style>
    .social-bar {
        background-color: rgba(0, 0, 0, .36);
        border-radius: 6px;
        flow: horizontal;
        border-spacing: 15px;
        margin-left: 1*;
        width: max-content;
        vertical-align: middle;
        height: 50px;
        padding: 0 20px;
    }
</style>