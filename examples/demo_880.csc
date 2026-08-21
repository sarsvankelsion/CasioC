model 880btg;
opn [E9E0];

u16 counter at [EB40] = 0;

csc main() {
    screen_del();
    if (counter == 0) {
        print("hello 880", 112, 16);
    }
    while (counter < 3) {
        counter = counter + 1;
        render();
        delay(100);
    }
}

