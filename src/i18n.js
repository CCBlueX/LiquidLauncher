import { addMessages, date, dictionary, init, locale } from "svelte-i18n";
import { derived, get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import en from "./locales/en.json";

/** Ships with the launcher; every other language is downloaded for this version. */
export const english = { code: "en", name: "English", minecraft: "en_us" };

/** The languages to offer, each named in itself, with Minecraft's code for it. */
export const languages = writable([english]);

addMessages("en", en);
init({ fallbackLocale: "en", initialLocale: "en" });

// Picks the CJK glyph variants the language expects.
locale.subscribe(code => document.documentElement.lang = code);

/** The offered language for a BCP 47 tag such as `de-AT`, `nl-BE` or `zh-Hant-TW`, English for any other. */
export function matchLanguage(tag, codes) {
    const subtags = tag?.split(/[-_]/).map(subtag => subtag.toLowerCase()) ?? [];
    if (subtags[0] === "zh") {
        const traditional = subtags.some(subtag => ["hant", "tw", "hk", "mo"].includes(subtag));
        return codes.find(code => code === (traditional ? "zh-TW" : "zh-CN")) ?? "en";
    }
    return codes.find(code => code.toLowerCase() === subtags.join("-"))
        ?? codes.find(code => code.split("-")[0] === subtags[0])
        ?? "en";
}

let request = 0;

/** Switches to the language closest to `tag`, downloading it first. English stays when that fails. */
export async function useLanguage(tag) {
    const current = ++request;
    const code = matchLanguage(tag, get(languages).map(language => language.code));
    if (!get(dictionary)[code]) {
        try {
            addMessages(code, await invoke("get_translation", { code }));
        } catch (error) {
            console.error(`Failed to load the ${code} translation:`, error);
            return;
        }
    }
    if (current === request) locale.set(code);
}

/** `Sep 26`, with the year when it is not this one. */
export const shortDate = derived(date, format => value => {
    const day = new Date(value);
    return format(day, day.getFullYear() === new Date().getFullYear()
        ? { month: "short", day: "numeric" }
        : { month: "short", day: "numeric", year: "numeric" });
});
