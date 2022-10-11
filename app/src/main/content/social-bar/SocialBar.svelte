<script>
    import ButtonSocial from "./ButtonSocial.svelte";
    import ButtonText from "./ButtonText.svelte";
</script>

<div class="social-bar">
    <ButtonSocial icon="nodebb" text="Forum" />
    <ButtonSocial icon="github" text="GitHub" />
    <ButtonSocial icon="guilded" text="Guilded" />
    <ButtonSocial icon="twitter" text="Twitter" />
    <ButtonSocial icon="youtube" text="YouTube" />
    <ButtonText icon="url" text="liquidbounce.net" />
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