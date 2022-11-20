<script>
    import SocialBar from "../main/content/social-bar/SocialBar.svelte";

    const messages = [
        {
            title: "3.000.000 Total Downloads",
            description: "LiquidBounce is one of the largest hacked clients."
        },
        {
            title: "Free & Open Source",
            description: "LiquidBounce is licensed under the GNU general public license."
        },
        {
            title: "Active Development",
            description: "We are constantly trying to improve LiquidBounce by adding new features."
        },
        {
            title: "Script API",
            description: "LiquidBounce's script API allows you to extend the client."
        }
    ];

    let currentMessage = 0;

    const autoMessageInterval = setInterval(() => {
        if (currentMessage < messages.length - 1) {
            currentMessage++;
        } else {
            currentMessage = 0;
        }   
    }, 5000);

    function handleNextMessage() {
        clearInterval(autoMessageInterval);

        if (currentMessage < messages.length - 1) {
            currentMessage++;
        } else {
            currentMessage = 0;
        }
    }

    function handlePrevMessage() {
        clearInterval(autoMessageInterval);

        if (currentMessage <= 0) {
            currentMessage = messages.length - 1;
        } else {
            currentMessage--;
        }
    }
</script>

<div class="about">
    <div class="title">{messages[currentMessage].title}</div>
    <div class="description">{messages[currentMessage].description}</div>
    <div class="buttons-wrapper">
        <div class="change-message prev" on:click={handlePrevMessage}></div>
        <div class="change-message next" on:click={handleNextMessage}></div>
    </div>

    <div class="social-bar-wrapper">
        <SocialBar />
    </div>
</div>

<style>
    .about {
        width: 1*;
        height: 1*;
        vertical-align: middle;
        transform: translate(0, -50px);
        margin-left: 100px;
    }

    .social-bar-wrapper {
        position: fixed;
        bottom: 32px;
        left: 130px;
        background-color: rgba(0, 0, 0, .36);
        border-radius: 6px;
    }

    .title {
        color: white;
        font-size: 40px;
        font-weight: 800;
        margin-bottom: 8px;
    }

    .description {
        color: rgba(255, 255, 255, .5);
        font-size: 18px;
        font-weight: 400;
        margin-bottom: 15px;
    }

    .buttons-wrapper {
        flow: horizontal;
        border-spacing: 8px;
    }

    .change-message {
        behavior: button;
        width: 44px;
        height: 44px;
        border-radius: 50%;
        border: solid 1px rgba(255, 255, 255, .5);
        transition: background-color ease .2s;
        background-position: center;
        background-repeat: no-repeat;
    }

    .change-message.prev {
        background-image: url("../img/icon/icon-prev.svg");
    }

    .change-message.next {
        background-image: url("../img/icon/icon-next.svg");
    }

    .change-message:hover {
        background-color: rgba(255, 255, 255, .1);
    }
</style>