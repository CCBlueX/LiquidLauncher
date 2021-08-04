module.exports.fetchNews = async () => {
    const response = await fetch("https://cloud.liquidbounce.net/LiquidLauncher/news.json");
    const data = await response.json();

    return data;
};