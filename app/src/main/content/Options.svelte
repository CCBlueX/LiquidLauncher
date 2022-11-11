<script>
    export let options;
    export let logout;

    export let versionData;
    export let branches;
    export let builds;

    export let updateBuilds;

    console.log(JSON.stringify(versionData));

    let branch = versionData.branch;
    let build = versionData.buildId;

    function storeOptions() {
        Window.this.xcall("store_options", options);
        syncBuilds();
    }

    function syncBuilds() {
        let branch = document.getElementById("branches").value;

        options.preferredBranch = branch;
        updateBuilds(branch);
    }

    function syncVersionData() {
        let selectedBuild = parseInt(document.getElementById("builds").value);
        options.preferredBuild = selectedBuild;

        let newVersionData = builds.find(el => selectedBuild === el.buildId);

        if (newVersionData === undefined) {
            console.error("failed to sync version data to user selection", selectedBuild);
            return;
        }

        versionData = newVersionData;
    }
</script>

<div class="wrapper">
<h1>Options</h1>

<div class="inner_wrapper">
    <h2>Version Selection</h2>

    <select name="branches" id="branches" on:change={syncBuilds}>
        {#each branches as branch}
            <option value={branch}>{branch}</option>
        {/each}
    </select>
    <select name="builds" id="builds" on:change={syncVersionData}>
        <option value={builds[0].buildId}>Latest</option>

        {#each builds as build}
            <option value={build.buildId}>{build.lbVersion + " git-" + build.commitId.substring(0, 7)}</option>
        {/each}
    </select>
    <br>
    <label>
        Show nightly builds
        <input type=checkbox bind:checked={options.showNightlyBuilds} on:change={storeOptions}>
    </label>
</div>

<div class="inner_wrapper">
    <h2>Launcher options</h2>

    <label>
        Keep Launcher Open
        <input type=checkbox bind:checked={options.keepLauncherOpen} on:change={storeOptions}><br>
        <button class="logout" on:click={logout}>Logout</button>
    </label>
<br>
</div>

<div class="inner_wrapper">
    <h2>Game options</h2>

    <label>

        Keep Launcher Open
        <input type=checkbox bind:checked={options.keepLauncherOpen} on:change={storeOptions}>
    </label>
</div>
</div>

<style>
    label, h1, h2 {
        color: white;
    }

    .wrapper {
        width: 1*;
        height: 1*;
        border-spacing: 24px;
        overflow: hidden;
    }

    .inner_wrapper {
        background-color: rgba(0, 0, 0, .36);
        padding: 24px;
        border-radius: 6px;
        height: 1*;
        overflow: scroll-indicator;
        border-spacing: 20px;
    }

    .scroll-indicator {
        height: 14px;
        background-image: url("/img/icon/icon-changelogs-scroll.svg");
        background-position: center;
        background-repeat: no-repeat;
    }

    .logout {
        margin-top: 0.5rem;
        margin-bottom: 0.5rem;
        background: unset;
        border: none;
        background-color: #f83939;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        width: 0.1*;
        display: block;
        height: 2rem;
        transition: background-color ease .2s;
        font-family: "Gilroy",serif;
    }

    .logout:hover {
        background-color: #e23e4c;
    }
</style>