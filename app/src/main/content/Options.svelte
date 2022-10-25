<script>
    export let options;
    export let closeOptions;

    function storeOptions() {
        Window.this.xcall("store_options", options);
    }

    function logout() {

    }

    console.log(JSON.stringify(options));
</script>

<br>

<label>
    Keep Launcher Open
    <input type=checkbox bind:checked={options.keepLauncherOpen} on:change={storeOptions}>
</label>
<br>
<label>
    Show nightly builds
    <input type=checkbox bind:checked={options.showNightlyBuilds} on:change={storeOptions}>
</label>

<br><br>

<button class="button" on:click={closeOptions}>CLOSE</button>


<style>
    label {
        color: white;
    }
</style>