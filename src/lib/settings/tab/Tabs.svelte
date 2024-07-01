<script>
    export let tabs;
    export let activeTab;
</script>

<div class="tabs">
    {#each tabs as t}
        <button class="tab" class:active={t === activeTab} on:click={() => activeTab = t}>{t}</button>
    {/each}
</div>

<style>
    .tabs {
        display: flex;
        background-color: rgba(0, 0, 0, 0.26);
        border-radius: 6px;
        margin-bottom: 10px;
        overflow: hidden;
    }

    .tab {
        color: white;
        flex-basis: 100%;
        text-align: center;
        padding: 8px;
        background-color: transparent;
        border: none;
        border-bottom: solid 1px transparent;
        font-family: "Inter", sans-serif;
        font-size: 14px;
        cursor: pointer;
        transition: ease border-color .2s;
    }

    .tab.active {
        border-color: #4677FF;
    }
</style>