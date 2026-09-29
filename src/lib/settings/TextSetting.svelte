<script>
    export let title = null;
    export let placeholder;
    export let value;
</script>

<div class="text-setting">
    {#if title}
        <div class="title">{title}</div>
    {/if}
    <input class="input" type="text" {placeholder} aria-label={title ? undefined : placeholder} bind:value={value} on:keydown />
</div>

<style>
    .title {
        color: white;
        margin-bottom: 5px;
    }

    .input {
        width: 100%;
        background-color: rgba(0, 0, 0, .26);
        border: none;
        border-bottom: solid 1px #4677FF;
        color: white;
        font-family: "Inter", sans-serif;
        padding: 5px;
        border-radius: 3px;
    }
</style>