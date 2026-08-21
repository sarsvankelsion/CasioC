model 580vnx;
opn [E9E0];

u16 counter at [EB40] = 0;

csc main() {
    screen_del();
    counter = counter + 1;
    if (counter == 10) {
        print("hi", 112, 16);
    } else {
        draw_line(0, 0, 50, 50);
    }
    render();
    delay(257);
    asm {
        "pop xr0"
    }
}

