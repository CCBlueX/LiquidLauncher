<div class="wrapper">
    <slot />
</div>

<style>
    .wrapper {
        background-color: rgba(0, 0, 0, 0.26);
        border-radius: 12px;
        flex: 1;
        display: grid;
        grid-template-columns: 250px auto;
        padding: 25px;
        column-gap: 30px;
        overflow: hidden;
    }
</style>
