model 580vnx;
opn [E9E0];

u16 counter at [EB40] = 0;
u16 ticks at [EB42] = 0;

csc main() {
    screen_del();
    // gán thay mov: er0 = 10
    counter = 0;
    ticks = 0;

    if (counter == 0) {
        print("start", 20, 10);
    } else {
        draw_line(0, 0, 100, 50);
    }

    while (counter < 5) {
        counter = counter + 1;
        print_hex(counter, 112, 16);
        render();
        delay(100);
    }

    for (ticks = 0; ticks < 3; ticks++) {
        draw_pixel(ticks, ticks);
    }

    // asm trong suốt
    asm {
        "pop xr0"
        "buffer_clear"
    }

    goto end;
    end:
        render();
}

csc helper(u16 x, u16 y) {
    draw_line(x, y, 50, 50);
    render();
}

