<script>
    import VerticalFlexWrapper from "../common/VerticalFlexWrapper.svelte";
    import TitleBar from "../common/TitleBar.svelte";
    import Logo from "../common/Logo.svelte";
    import ButtonClose from "../common/ButtonClose.svelte";
    import {Jumper} from "svelte-loading-spinners";
</script>

<VerticalFlexWrapper blur={false}>
    <TitleBar>
        <Logo />
        <ButtonClose />
    </TitleBar>

    <div class="loading-circle">
        <Jumper size="100" color="#4677ff" unit="px" duration="1s" />
    </div>
</VerticalFlexWrapper>

<style>
    .loading-circle {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
    }
</style>