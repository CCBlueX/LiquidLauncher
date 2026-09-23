// Words for the states the backend computes.

export function count(n, one, many) {
    return `${n} ${n === 1 ? one : many}`;
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
