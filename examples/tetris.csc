model 580vnx;
opn [D730];

u16 px at [EB40] = 88;
u16 py at [EB42] = 6;
u16 key at [EB44] = 0;

csc main() {
    screen_del();
    px = 88;
    py = 6;

    game_loop:
        screen_del();
        
        // 1. Draw Border (Left X=70, Right X=110, Floor Y=56)
        draw_line(70, 4, 70, 56);
        draw_line(110, 4, 110, 56);
        draw_line(70, 56, 110, 56);
        
        // 2. Title
        print("TETRIS", 1);
        
        // 4. Gravity: block falls down
        py = py + 2;
        if (py > 50) {
            py = 6;
            px = 88;
        }
        
        // 5. Draw Falling Block
        draw_pixel(px, py);
        draw_pixel(px + 1, py);
        draw_pixel(px + 2, py);
        draw_pixel(px, py + 1);
        draw_pixel(px + 1, py + 1);
        draw_pixel(px + 2, py + 1);
        draw_pixel(px, py + 2);
        draw_pixel(px + 1, py + 2);
        draw_pixel(px + 2, py + 2);
        
        render();
        delay(100);
        
        goto game_loop;
}