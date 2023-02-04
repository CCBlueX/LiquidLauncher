<script>
    import { createEventDispatcher } from "svelte";

    export let value;
    export let title;
    export let disabled;

    const dispatch = createEventDispatcher();
</script>

<label class="toggle-setting">
    <input class="checkbox" type="checkbox" bind:checked={value} disabled={disabled} on:change={e => dispatch("change", e)} />
    <span class="slider" />

    <div class="title">{title}</div>
</label>

<style>
    .toggle-setting {
        position: relative;
        padding-left: 30px;
        cursor: pointer;
        display: grid;
        grid-template-columns: max-content 1fr;
    }

    .title {
        color: white;
    }

    .slider {
        position: absolute;
        top: 2px;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: #707070;
        transition: ease 0.4s;
        height: 8px;
        border-radius: 4px;
        width: 22px;
        top: 50%;
        transform: translateY(-50%);
    }

    .slider::before {
        position: absolute;
        content: "";
        height: 12px;
        width: 12px;
        top: -2px;
        left: 0px;
        background-color: white;
        transition: ease 0.4s;
        border-radius: 50%;
    }

    .checkbox {
        display: none;
    }

    .checkbox:checked + .slider {
        background-color: #4860a7;
    }

    .checkbox:checked + .slider::before {
        transform: translateX(10px);
        background-color: #4677ff;
    }

    .checkbox:disabled + .slider {
        background-color: #6d6363 !important;
    }

    .checkbox:disabled + .slider::before {
        transform: translateX(10px);
        background-color: #786d6d;
    }
</style>
