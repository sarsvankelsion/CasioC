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
        // 1. Xoa man hinh
        cls();
        
        // 2. Ve khung san dau
        draw_rect(40, 4, 100, 52);
        
        // 3. In tieu de Game
        print("TETRIS PRO", 1);
        
        // 4. Kiem tra ban phim
        is_key_pressed();
        
        // 5. Trong luc roi khoi gach
        py = py + 2;
        
        // 6. Kiem tra cham day & cong diem
        if (py > 48) {
            py = 8;
            px = 88;
            score = score + 10;
        }
        
        // 7. Ve khoi gach Tetromino mini
        draw_rect(px, py, 6, 6);
        draw_pixel(px + 3, py + 3);
        
        // 8. Day ra man hinh va delay
        render();
        delay(80);
        goto game_loop;
}