<script>
    import { onMount } from "svelte";
    import noUiSlider from "nouislider";
    import "nouislider/dist/nouislider.min.css";
    import "./RangeSettingStyles.css";

    export let title;
    export let min;
    export let max;
    export let value;
    export let valueSuffix;
    export let step;

    function updateSliderKeypress(e) {
        if (e.key === "Enter") {
            e.preventDefault();
            slider.noUiSlider.set(value);
        }
    }

    function updateSliderBlur(e) {
        slider.noUiSlider.set(value);
    }

    let slider = null;
    onMount(() => {
        const start = value;

        noUiSlider.create(slider, {
            start: start,
            connect: value.length > 1 ? true : "lower",
            padding: [0, 0],
            range: {
                min: min,
                max: max,
            },
            step: step
        });

        slider.noUiSlider.on("update", values => {
            value = values.map(v => parseFloat(v));
            
            console.log(values);
        });
    });

    let v = 2000;
</script>


<div class="range-setting">
    <div class="title">{title}</div>
    <div class="value">
        <span class="input-value" contenteditable="true" bind:textContent={value[0]} on:keypress={updateSliderKeypress} on:blur={updateSliderBlur}></span>
        <span class="value-suffix">{valueSuffix}</span>
        {#if value.length > 1}
            <span class="value-devider">-</span>
            <span class="input-value" contenteditable="true" bind:textContent={value[1]} on:keypress={updateSliderKeypress} on:blur={updateSliderBlur}></span>
            <span class="value-suffix">{valueSuffix}</span>
        {/if}
    </div>
    <div bind:this={slider} class="slider" />
</div>

<style>
    .range-setting {
        display: grid;
        grid-template-areas:
            "a b"
            "c c";
        grid-template-columns: 1fr 1fr;
        position: relative;
        min-height: 42px; /* Fix animation glitch */
    }

    .title {
        grid-area: a;
        color: white;
    }

    .slider {
        grid-area: c;
        width: 100%;
    }

    .value {
        grid-area: b;
        color: white;
        text-align: right;
    }

    .input-value {
        background-color: transparent;
        border: none;
        font-size: 16px;
        color: white;
        display: inline;
        width: min-content;
        font-family: "Gilroy", sans-serif;
        user-select: unset;
    }

    .value-devider {
        display: inline-block;
        margin: 0 5px;
    }
</style>