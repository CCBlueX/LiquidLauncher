<script>
    import { invoke } from "@tauri-apps/api/tauri";
    import { listen } from '@tauri-apps/api/event'


    export let versionData;
    export let accountData;
    export let options;
    export let mods;

    let title;
    let date;
    let text;
    let previewImage;

    (async () => {
        // const news = (await fetchNews())[0];
        // console.log(news);
        // title = news.title;
        // date = news.date;
        // text = news.text;
        // previewImage = news.banner.image;
    })();

    listen('process-output', (event) => {
        console.log(event.payload);
        console.log(event);
        let logArea = document.getElementById('log-area');
        logArea.innerText += event.payload + "\n";
        logArea.scrollTop = logArea.scrollHeight;
    });

    listen('progress-update', (event) => {
        let progressUpdate = event.payload;
        console.log(event);
        let label = document.getElementById("statusLabel");
        let progressBar = document.getElementById("progress");

        switch (progressUpdate.type) {
                case "max": {
                    progressBar.max = progressUpdate.value;
                    break;
                }
                case "progress": {
                    progressBar.value = progressUpdate.value;
                    break;
                }
                case "label": {
                    label.textContent = progressUpdate.value;
                    break;
                }
            }
    });

    function launching(status) {
        document.getElementById("launch").style.display = status ? 'block' : 'none';
        document.getElementById("version-select").style.display = status ? 'none' : 'block';
        document.getElementById("play").style.display = status ? 'none' : 'block';
    }

    function startClient() {
        let playButton = document.getElementById("play");
        let label = document.getElementById("statusLabel");
        let logArea = document.getElementById('log-area');
        logArea.innerText = "";

        invoke('run_client', { buildId: versionData.buildId, accountData: accountData, options: options, mods: mods })
            .then(() => {
                label.textContent = "Idle...";
                launching(false);

                playButton.disabled = false;
            })
            .catch((e) => {
                alert(e);
            })
        
        label.textContent = "Running...";
        launching(true);
        playButton.disabled = true;
    }

    function stopClient() {
        invoke('terminate').then(() => {
                launching(false);
            })
            .catch((e) => {
                alert(e);
            });
    }


</script>

<div class="wrapper">
    <div class="current-version" style="background-image: url('{previewImage}');">
        <div class="name">{title}</div>
        <div class="date">{date}</div>
    </div>

    <div class="changelog">
        <p>
            Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor 
        invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.</p><br>
        
        <p>At vero eos et accusam 
        et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est 
        Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr,</p><br> sed 
        diam nonumy eirmod tempor 
        <p>invidunt ut labore et dolore magna aliquyam erat, sed diam 
        voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd 
        gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor 
        sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut 
        labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam 
        et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus 
        est Lorem ipsum dolor sit amet.</p>
    </div>

    <div id="version-select" class="version-select" style="display: block;">
        <div class="version">
            <img class="icon" src="img/icon/icon-version-lb.png" alt="liquidbounce">
            <div class="name">{versionData ? versionData.lbVersion : "[waiting...]"}</div>
            <div class="date">{versionData ? versionData.date : "[waiting...]"}</div>
        </div>

        <div class="version">
            <img class="icon" src="img/icon/icon-version-mc.png" alt="minecraft">
            <div class="name">{versionData ? versionData.mcVersion : "[waiting...]"}</div>
            <div class="date">TODO: Date of MC Version</div>
        </div>
    </div>

    <div id="launch" style="display: none;">
        <textarea cols="100" readonly id="log-area" style="width: 85%; max-width: 85%"></textarea><br>
        <progress value="19" min="0" max="5120" id="progress" style="width: 85%; max-width: 85%">...</progress><br>
        <span id="statusLabel" style="width: 80%; max-width: 85%"></span>

        <button id="stop" class="stop" on:click={stopClient}>Stop</button>
    </div>
    <button id="play" class="play" on:click={startClient}>Play</button>
</div>

<style>
    span textarea label {
        color: white;
        font-family: "Gilroy",serif;
    }

    .wrapper {
        width: 293px;
        height: 1*;
        border-spacing: 17px;
        background-color: rgba(0, 0, 0, .36);
        padding: 24px;
        border-radius: 6px;
    }

    .current-version {
        height: 98px;
        width: 1*;
        overflow: hidden;
        border-radius: 6px;
        position: relative;
        background-size: cover;
        background-position: center;
    }

    .current-version::before {
        content: "";
        display: block;
        height: 1*;
        width: 1*;
        position: absolute;
        top: 0;
        left: 0;
        background: linear-gradient(top, rgba(0, 0, 0, .36), #4677ffc5);
        z-index: -1;
    }

    .current-version .name {
        z-index: 1000;
        font-weight: 800;
        color: white;
        font-size: 18px;
        max-width: 50%;
        position: absolute;
        top: 50%;
        transform: translate(15px, -50%);
    }

    .current-version .date {
        font-size: 12px;
        padding: 5px 12px;
        background-color: rgba(0, 0, 0, .68);
        color: white;
        border-radius: 4;
        position: absolute;
        top: 8px;
        right: 8px;
    }

    .changelog {
        font-size: 12px;
        color: rgba(255, 255, 255, .5);
        height: 1*;
        overflow: scroll-indicator;
    }

    .version-select {
        flow: horizontal;
        border-spacing: 8px;
    }

    .version-select .version {
        background-color: rgba(0, 0, 0, .36);
        border-radius: 6px;
        text-align: center;
        width: 1*;
        border-spacing: 2px;
        padding: 8px;
        border: solid 1px rgba(0, 0, 0, .36);
        transition: border-color ease .2s;
    }

    .version-select .version:hover {
        border-color: #4677ff;
    }

    .version-select .version .icon {
        margin-bottom: 4px;
    }

    .version-select .version .name {
        color: white;
        font-size: 12px;
    }

    .version-select .version .date {
        font-size: 10px;
        color: rgba(255, 255, 255, .5);
    }

    .version-select select {
        max-width: 7rem;
    }

    .play {
        background: unset;
        border: none;
        background-color: #4677FF;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        width: 1*;
        display: block;
        height: 50px;
        transition: background-color ease .2s;
        font-family: "Gilroy",serif;
    }

    .play:hover {
        background-color: #3E69E2;
    }

    .stop {
        background: unset;
        border: none;
        background-color: #f83939;
        color: white;
        font-size: 14px;
        border-radius: 6px;
        width: 1*;
        display: block;
        height: 50px;
        transition: background-color ease .2s;
        font-family: "Gilroy",serif;
    }

    .stop:hover {
        background-color: #e23e4c;
    }
</style>