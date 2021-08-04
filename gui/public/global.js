try {
    const screen = Window.this.screenBox("frame");
    const screenWidth = screen[2];
    const screenHeight = screen[3];
    const windowWidth = 988;
    const windowHeight = 658;
    Window.this.minSize = [800, 500];
    Window.this.move(screenWidth / 2 - windowWidth / 2, screenHeight / 2 - windowHeight / 2, windowWidth, windowHeight);
} catch (err) {
    console.log(err);
}