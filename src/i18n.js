import { addMessages, date, init, locale } from "svelte-i18n";
import { derived } from "svelte/store";
import en from "./locales/en.json";
import de from "./locales/de.json";
import zhCN from "./locales/zh-CN.json";

/** Each language named in itself, as the language setting lists them, with Minecraft's code for it. */
export const languages = {
    en: { name: "English", minecraft: "en_us" },
    de: { name: "Deutsch", minecraft: "de_de" },
    "zh-CN": { name: "简体中文", minecraft: "zh_cn" },
};

addMessages("en", en);
addMessages("de", de);
addMessages("zh-CN", zhCN);

/** The shipped language for a BCP 47 tag such as `de-AT` or `zh-Hans-CN`, English for any other. */
export function matchLanguage(tag) {
    const primary = tag?.split(/[-_]/)[0].toLowerCase();
    return Object.keys(languages).find(code => code.split("-")[0] === primary) ?? "en";
}

// The window's language, until the options name one or the OS is asked.
init({ fallbackLocale: "en", initialLocale: matchLanguage(navigator.language) });

// Picks the CJK glyph variants the language expects.
locale.subscribe(code => document.documentElement.lang = code);

/** `Sep 26`, with the year when it is not this one. */
export const shortDate = derived(date, format => value => {
    const day = new Date(value);
    return format(day, day.getFullYear() === new Date().getFullYear()
        ? { month: "short", day: "numeric" }
        : { month: "short", day: "numeric", year: "numeric" });
});
