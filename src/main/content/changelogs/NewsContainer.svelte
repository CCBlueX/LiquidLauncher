<script>
    import SocialBar from "../social-bar/SocialBar.svelte";
    import News from "./News.svelte";
    import { fetchNews } from "../../../utils/news";

    let news = [];

    (async () => {
        news = await fetchNews();
    })();
</script>

<div class="wrapper">
    <div class="news">
        {#each news as n}
            <News {...n} />
        {/each}
    </div>

    <div class="scroll-indicator"></div>

    <SocialBar />
</div>

<style>
    .wrapper {
        width: 1*;
        height: 1*;
        border-spacing: 24px;
        overflow: hidden;
    }

    .news {
        background-color: rgba(0, 0, 0, .36);
        padding: 24px;
        border-radius: 6px;
        height: 1*;
        overflow: scroll-indicator;
        border-spacing: 20px;
    }

    .scroll-indicator {
        height: 14px;
        background-image: url("../img/icon/icon-changelogs-scroll.svg");
        background-position: center;
        background-repeat: no-repeat;
    }
</style>