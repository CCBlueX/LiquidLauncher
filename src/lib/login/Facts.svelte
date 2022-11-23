<script>
    import { fly } from "svelte/transition";

    const facts = [
        {
            title: "3.000.000 Total Downloads",
            description: "LiquidBounce is one of the most popular hacked clients of all time."
        },
        {
            title: "Free & Open Source",
            description: "LiquidBounce's source code is publicly available."
        },
        {
            title: "Extensible",
            description: "LiquidBounce's JavaScript API allows users to write their own modules and commands."
        }
    ];

    let currentFact = 0;

    function handlePrevClick(e) {
        if (currentFact <= 0) {
            currentFact = facts.length - 1;
        } else {
            currentFact--;
        }
    }

    function handleNextClick(e) {
        if (currentFact >= facts.length - 1) {
            currentFact = 0;
        } else {
            currentFact++;
        }
    }

    setInterval(() => {
        handleNextClick();
    }, 5000);
</script>

<div class="wrapper">
    <div class="fact">
        <div class="title">{facts[currentFact].title}</div>
        <div class="description">{facts[currentFact].description}</div>
        <div class="buttons-wrapper">
            <button type="button" class="button-switch-fact" on:click={handlePrevClick}>
                <img src="img/icon/icon-prev.svg" alt="prev">
            </button>
            <button type="button" class="button-switch-fact" on:click={handleNextClick}>
                <img src="img/icon/icon-next.svg" alt="next">
            </button>
        </div>
    </div>
</div>

<style>
    .wrapper {
        position: relative;
    }

    .fact {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        width: 400px;
        display: flex;
        flex-direction: column;
        row-gap: 10px;
    }

    .title {
        font-size: 40px;
        color: white;
        font-weight: bold;
    }

    .description {
        font-size: 18px;
        color: rgba(255, 255, 255, .5);
    }

    .buttons-wrapper {
        display: flex;
        column-gap: 10px;
    }

    .button-switch-fact {
        height :44px;
        width: 44px;
        border-radius: 50%;
        border: solid 1px rgba(255, 255, 255, .5);
        background-color: transparent;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: ease background-color .2s;
        cursor: pointer;
    }

    .button-switch-fact:hover {
        background-color: rgba(255, 255, 255, .1);
    }
</style>