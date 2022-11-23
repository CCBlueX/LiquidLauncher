<script>
    export let title;
</script>

<div class="wrapper">
    <div class="title">{title}</div>
    <div class="wrapped-settings">
        <slot />
    </div>
</div>

<style>
    .title {
        color: white;
        margin-bottom: 5px;
    }

    .wrapped-settings {
        background-color: rgba(0, 0, 0, .26);
        border-radius: 6px;
        padding: 10px;
        border-bottom: solid 1px #4677FF;
        display: grid;
        grid-template-rows: max-content;
        row-gap: 5px;
        max-height: 150px;
        overflow: auto;
    }
</style>