<script>
    import SocialBar from "../../common/social/SocialBar.svelte";
    import News from "./News.svelte";

    const news = [
        {
            bannerText: "b73",
            title: "LiquidBounce b73",
            date: "2021-05-07",
            bannerUrl: "img/b73.jpg",
            url: "https://google.de",
            description: "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam"
        },
        {
            bannerText: "b73",
            title: "LiquidBounce b73",
            date: "2021-05-07",
            bannerUrl: "img/b73.jpg",
            url: "https://google.de",
            description: "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam"
        },
        {
            bannerText: "b73",
            title: "LiquidBounce b73",
            date: "2021-05-07",
            bannerUrl: "img/b73.jpg",
            url: "https://google.de",
            description: "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam"
        },
        {
            bannerText: "b73",
            title: "LiquidBounce b73",
            date: "2021-05-07",
            bannerUrl: "img/b73.jpg",
            url: "https://google.de",
            description: "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam"
        },
        {
            bannerText: "b73",
            title: "LiquidBounce b73",
            date: "2021-05-07",
            bannerUrl: "img/b73.jpg",
            url: "https://google.de",
            description: "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam"
        },
        {
            bannerText: "b73",
            title: "LiquidBounce b73",
            date: "2021-05-07",
            bannerUrl: "img/b73.jpg",
            url: "https://google.de",
            description: "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam"
        },
    ]
</script>

<div class="news-area">
    <div class="news-wrapper">
        {#each news as n}
            <News {...n} />
        {/each}
    </div>

    <button class="button-scroll">
        <img class="icon" src="img/icon/icon-news-scroll.svg" alt="scroll">
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
