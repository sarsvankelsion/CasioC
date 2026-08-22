model 580vnx;
opn [E9E0];

csc main() {
    screen_del();
    // Thanh dọc (x = 96, y từ 10 đến 54)
    draw_line(96, 10, 96, 54);
    
    // Thanh ngang (y = 24, x từ 76 đến 116)
    draw_line(76, 24, 116, 24);
    
    render();
}

