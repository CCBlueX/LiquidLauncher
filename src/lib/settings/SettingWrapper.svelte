<script>
    export let title;
</script>

<div class="wrapper">
    <div class="title-wrapper">
        <span class="title">{title}</span>
        <slot name="title-element" />
    </div>
    <div class="wrapped-settings">
        <slot />
    </div>
</div>

<style>
    .title-wrapper {
        display: grid;
        grid-template-columns: 1fr max-content;
    }

    .title {
        color: white;
        margin-bottom: 5px;
    }

    .wrapped-settings {
        background-color: rgba(0, 0, 0, 0.26);
        border-radius: 6px;
    }

    .wrapped-settings:not(:empty) {
        padding: 10px;
        border-bottom: solid 1px #4677ff;
        display: grid;
        grid-template-rows: max-content;
        row-gap: 5px;
        max-height: 150px;
        overflow: auto;
    }
</style>