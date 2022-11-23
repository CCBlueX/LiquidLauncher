<script>
    import ButtonClose from "../common/ButtonClose.svelte";
    import Logo from "../common/Logo.svelte";
    import SocialBar from "../common/social/SocialBar.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import Facts from "./Facts.svelte";
    import LoginModal from "./loginmodal/LoginModal.svelte";

    export let options;
</script>

<VerticalFlexWrapper blur={false}>
    <TitleBar>
        <Logo />
        <ButtonClose />
    </TitleBar>
    
    <div class="main-section-wrapper">
        <Facts />

        <LoginModal bind:options />
    </div>

    <SocialBar />
</VerticalFlexWrapper>

<style>
    .main-section-wrapper {
        display: grid;
        grid-template-columns: 1fr max-content;
        flex: 1;
        padding: 0 50px;
        margin-bottom: 20px;
    }
</style>