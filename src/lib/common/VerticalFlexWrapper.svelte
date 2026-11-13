<script>
    export let blur;
</script>

<div class="wrapper" class:blur>
    <slot />
</div>

<style>
    .wrapper {
        display: flex;
        height: 100%;
        flex-direction: column;
        transition: ease blur .2s;
    }

    .blur {
        pointer-events: none;
        filter: blur(5px);
    }
</style>
