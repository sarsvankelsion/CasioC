model 580vnx;
opn [E9E0];

// Game state in RAM
u16 px at [EB40] = 88;
u16 py at [EB42] = 6;
u16 score at [EB44] = 0;
u16 key at [EB46] = 0;

csc main() {
    screen_del();
    px = 88;
    py = 6;
    score = 0;

    game_loop:
        screen_del();
        
        // 1. Draw Tetris Board Frame
        draw_line(70, 4, 70, 58);
        draw_line(112, 4, 112, 58);
        draw_line(70, 58, 112, 58);
        
        // 2. Title
        print("TETRIS", 1);
        
        // 3. User Key Input
        get_key(key);
        
        // Left Key [<-] (KI/KO: 0x0440 = 1088)
        if (key == 1088) {
            if (px > 74) {
                px = px - 4;
            }
        }
        
        // Right Key [->] (KI/KO: 0x0880 = 2176)
        if (key == 2176) {
            if (px < 104) {
                px = px + 4;
            }
        }
        
        // 4. Gravity
        py = py + 2;
        
        // 5. Floor Collision
        if (py > 50) {
            py = 6;
            px = 88;
            score = score + 10;
        }
        
        // 6. Draw Falling Tetromino Box
        draw_line(px, py, px + 6, py);
        draw_line(px, py + 4, px + 6, py + 4);
        draw_line(px, py, px, py + 4);
        draw_line(px + 6, py, px + 6, py + 4);
        
        render();
        delay(120);
        
        goto game_loop;
}