import { addMessages, date, dictionary, init, locale } from "svelte-i18n";
import { derived, get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import metadata from "./locales/languages.json";

const catalogs = import.meta.glob(["./locales/*.json", "!./locales/languages.json"], { eager: true, import: "default" });

/** Every bundled language, named in itself, with Minecraft's code for it. */
export const bundled = Object.entries(metadata).map(([code, { name, minecraft }]) => ({ code, name, minecraft }));

export const english = bundled.find(language => language.code === "en");

/** The languages to offer: the bundled ones and those published for this version since. */
export const languages = writable(bundled);

for (const [code, { catalog }] of Object.entries(metadata)) {
    addMessages(code, catalogs[`./locales/${catalog ?? code}.json`]);
}
init({ fallbackLocale: "en", initialLocale: "en" });

// Picks the CJK glyph variants the language expects.
locale.subscribe(code => document.documentElement.lang = code);

/** The offered language for a BCP 47 tag such as `de-AT`, `nl-BE` or `zh-Hant-TW`, English for any other. */
export function matchLanguage(tag, codes) {
    const subtags = tag?.split(/[-_]/).map(subtag => subtag.toLowerCase()) ?? [];
    if (subtags[0] === "zh") {
        const traditional = subtags.some(subtag => ["hant", "tw", "hk", "mo"].includes(subtag));
        return traditional && codes.includes("zh-TW") ? "zh-TW" : "zh-CN";
    }
    return codes.find(code => code.toLowerCase() === subtags.join("-"))
        ?? codes.find(code => code.split("-")[0] === subtags[0])
        ?? "en";
}

let request = 0;
const published = new Set();

/**
 * Switches to the language closest to `tag`. A published language is downloaded first, which also
 * brings fixes for a bundled one; when that fails, a bundled copy still works.
 */
export async function useLanguage(tag) {
    const current = ++request;
    const offered = get(languages);
    const code = matchLanguage(tag, offered.map(language => language.code));
    if (offered.find(language => language.code === code)?.sha256 && !published.has(code)) {
        try {
            addMessages(code, await invoke("get_translation", { code }));
            published.add(code);
        } catch (error) {
            console.error(`Failed to load the ${code} translation:`, error);
        }
    }
    if (current === request && get(dictionary)[code]) locale.set(code);
}

/** `Sep 26`, with the year when it is not this one. */
export const shortDate = derived(date, format => value => {
    const day = new Date(value);
    return format(day, day.getFullYear() === new Date().getFullYear()
        ? { month: "short", day: "numeric" }
        : { month: "short", day: "numeric", year: "numeric" });
});
