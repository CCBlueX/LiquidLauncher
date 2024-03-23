<script>
    import SocialBar from "../../common/social/SocialBar.svelte";
    import News from "./News.svelte";
    import {invoke} from "@tauri-apps/api/core";

    let news = [];

    invoke("fetch_news")
        .then((onlineNews) => {
            news = onlineNews;
        })
        .catch((e) => console.error(e));
</script>

<div class="news-area">
    <div class="news-wrapper">
        {#each news as n}
            <News {...n} />
        {/each}
    </div>

    <button class="button-scroll">
        <img class="icon" src="img/icon/icon-news-scroll.svg" alt="scroll" />
    </button>

    <div class="social-bar-wrapper">
        <SocialBar />
    </div>
</div>

<style>
    .social-bar-wrapper {
        display: flex;
        justify-content: flex-end;
    }

    .news-area {
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .news-wrapper {
        display: grid;
        grid-template-columns: 1fr 1fr;
        grid-auto-rows: max-content;
        overflow: auto;
        flex: 1;
        gap: 20px;
    }

    .button-scroll {
        background-color: transparent;
        border: none;
        margin: 20px 0;
    }
</style>
