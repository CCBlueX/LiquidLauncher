// Words for the states the backend computes.

export function count(n, one, many) {
    return `${n} ${n === 1 ? one : many}`;
}

export function names(list) {
    if (list.length < 2) {
        return list.join("");
    }
    return `${list.slice(0, -1).join(", ")} and ${list[list.length - 1]}`;
}

export const itemTypes = {
    Addon: { name: "Add-on", title: "Add-ons", plural: "add-ons" },
    Theme: { name: "Theme", title: "Themes", plural: "themes" },
    Script: { name: "Script", title: "Scripts", plural: "scripts" }
};

export function removeQuestion(name, neededBy) {
    return `Remove ${name}? ${names(neededBy)} ${neededBy.length === 1 ? "stops" : "stop"} working without it.`;
}

export function neededByLine(dependent) {
    const type = itemTypes[dependent.type].name;
    return dependent.author ? `${type} by ${dependent.author}` : type;
}

export function rowLine(row) {
    return [row.version, row.notFor && `Not for ${row.notFor}`].filter(Boolean).join(" · ");
}

export function browseLine(item, liquidbounce) {
    const downloads = count(item.downloads, "download", "downloads");
    switch (item.fit?.kind) {
        case "fits":
            return [item.fit.version, item.fit.liquidbounce, downloads].filter(Boolean).join(" · ");
        case "noVersion":
            return `Not for ${liquidbounce} · ${downloads}`;
        default:
            return item.date ? `${item.date} · ${downloads}` : downloads;
    }
}

export function noticeLine(notice) {
    switch (notice.kind) {
        case "checking":
            return { checking: true };
        case "offline":
            return {
                tone: "strong",
                problem: true,
                text: "Could not reach the marketplace.",
                detail: notice.error,
                retry: true
            };
        case "restart":
            return {
                tone: "strong",
                text: `Restart LiquidBounce to apply ${count(notice.changes, "change", "changes")}.`
            };
        default:
            return {};
    }
}

export function versionTag(tag) {
    switch (tag?.kind) {
        case "installed":
            return { text: "Installed", strong: true };
        case "notFor":
            return { text: `Not for ${tag.liquidbounce}`, dim: true };
        default:
            return null;
    }
}
