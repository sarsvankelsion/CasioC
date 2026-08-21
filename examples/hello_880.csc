model 880btg;
opn [E9E0];

u16 counter at [EB40] = 0;

csc main() {
    screen_del();
    if (counter < 5) {
        counter = counter + 1;
    }
    while (counter < 3) {
        counter = counter + 1;
    }
    print("hello", 112, 16);
    render();
    delay(257);
}

