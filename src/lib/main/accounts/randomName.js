const ADJECTIVES = ["Swift", "Silent", "Brave", "Lucky", "Clever", "Mighty", "Sneaky", "Happy", "Frosty", "Rapid", "Shadow", "Golden"];
const NOUNS = ["Fox", "Wolf", "Miner", "Builder", "Archer", "Knight", "Creeper", "Falcon", "Tiger", "Golem", "Panda", "Rider"];

function pick(list) {
    return list[Math.floor(Math.random() * list.length)];
}

// At most 15 characters, a Minecraft name allows 16.
export function randomName() {
    const digits = String(Math.floor(Math.random() * 100)).padStart(2, "0");
    return pick(ADJECTIVES) + pick(NOUNS) + digits;
}
