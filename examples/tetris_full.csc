model 580vnx;
opn [D730];

u16 px at [EB40] = 88;
u16 py at [EB42] = 8;
u16 score at [EB44] = 0;

csc main() {
    screen_del();
    px = 88;
    py = 8;
    score = 0;

    game_loop:
        screen_del();
        
        // 1. Draw Arena Boundary Walls (Left X=60, Right X=120, Floor Y=56)
        draw_line(60, 4, 60, 56);
        draw_line(120, 4, 120, 56);
        draw_line(60, 56, 120, 56);
        
        // 2. UI Title
        print("TETRIS", 1);
        
        // 3. Gravity: Block falls down
        py = py + 2;
        
        // 4. Floor Collision: Land block and score points
        if (py > 50) {
            py = 8;
            px = 88;
            score = score + 10;
        }
        
        // 5. Render Tetromino (Block with border and center dot)
        draw_line(px, py, px + 6, py);
        draw_line(px, py + 6, px + 6, py + 6);
        draw_line(px, py, px, py + 6);
        draw_line(px + 6, py, px + 6, py + 6);
        draw_pixel(px + 3, py + 3);
        
        // 6. Draw Landed Pile Blocks at the bottom
        draw_line(64, 52, 70, 52);
        draw_line(64, 55, 70, 55);
        draw_line(110, 52, 116, 52);
        draw_line(110, 55, 116, 55);
        
        render();
        delay(100);
        goto game_loop;
}