<script>
    export let title;
    export let items;
    export let value;
</script>


<div class="select-setting">
    <div class="title">{title}</div>
    <select class="select" bind:value={value}>
        {#each items as i}
            <option value={i}>{i}</option>
        {/each}
    </select>
</div>

<style>
    .select {
        width: 100%;
        background-color: rgba(0, 0, 0, .26);
        border: none;
        border-bottom: solid 1px #4677FF;
        color: white;
        font-family: "Gilroy", sans-serif;
        padding: 5px;
        border-radius: 3px;
        cursor: pointer;
    }

    .title {
        color: white;
        margin-bottom: 5px;
    }

    .select-setting option {
        background-color: white;
        color: black;
    }
</style>