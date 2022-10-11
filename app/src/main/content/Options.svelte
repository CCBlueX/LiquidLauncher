<script>
    export let closeOptions;

    function getOptions() {
        return Window.this.xcall("get_options")
    }

    function storeOptions() {

    }

    let options = getOptions();
    console.log(JSON.stringify(options));
</script>

<br>

<label>
    Keep Launcher Open
    <input type=checkbox bind:checked={options.keepLauncherOpen} on:change={storeOptions()}>
</label>
<br>
<label>
    Show nightly builds
    <input type=checkbox bind:checked={options.showNightlyBuilds} on:change={storeOptions()}>
</label>

<br><br>

<button class="button" on:click={closeOptions}>CLOSE</button>


<style>
    label {
        color: white;
    }
</style>