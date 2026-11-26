import { addMessages, date, init, locale } from "svelte-i18n";
import { derived } from "svelte/store";

/** Each language named in itself, as the language setting lists them, with Minecraft's code for it. */
export const languages = {
    en: { name: "English", minecraft: "en_us" },
    "en-x-pirate": { name: "Pirate Speak", minecraft: "en_pt" },
    de: { name: "Deutsch", minecraft: "de_de" },
    ja: { name: "日本語", minecraft: "ja_jp" },
    "zh-CN": { name: "简体中文", minecraft: "zh_cn" },
    "zh-TW": { name: "繁體中文", minecraft: "zh_tw" },
    ru: { name: "Русский", minecraft: "ru_ru" },
    uk: { name: "Українська", minecraft: "uk_ua" },
    "pt-BR": { name: "Português (Brasil)", minecraft: "pt_br" },
    tr: { name: "Türkçe", minecraft: "tr_tr" },
    nl: { name: "Nederlands", minecraft: "nl_nl" },
    // Without a catalog of its own, nl-BE reads nl.json.
    "nl-BE": { name: "Nederlands (België)", minecraft: "nl_be" },
};

const catalogs = import.meta.glob("./locales/*.json", { eager: true, import: "default" });
for (const [path, messages] of Object.entries(catalogs)) {
    addMessages(path.slice("./locales/".length, -".json".length), messages);
}

/** The shipped language for a BCP 47 tag such as `de-AT`, `nl-BE` or `zh-Hant-TW`, English for any other. */
export function matchLanguage(tag) {
    const subtags = tag?.split(/[-_]/).map(subtag => subtag.toLowerCase()) ?? [];
    if (subtags[0] === "zh") {
        return subtags.some(subtag => ["hant", "tw", "hk", "mo"].includes(subtag)) ? "zh-TW" : "zh-CN";
    }
    const codes = Object.keys(languages);
    return codes.find(code => code.toLowerCase() === subtags.join("-"))
        ?? codes.find(code => code.split("-")[0] === subtags[0])
        ?? "en";
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
