# 📖 TỔNG HỢP TOÀN BỘ 336 GADGETS & HÀM CASIOC (.CSC) CHO CASIO FX-580VN X

> Tài liệu này được đồng bộ và nâng cấp toàn diện từ file `message.txt` và bộ ROM Gadget gốc của `hdcompiler_vn` (tổng cộng **336 gadgets & labels**).

---

## 1. ⚡ NHÓM SO SÁNH NÂNG CAO (CMP Gadgets++ / Verify Series)

Nhóm gadget này cho phép so sánh 2 vùng nhớ trong RAM và trả về kết quả `Boolean` (`er0 = 1` nếu Đúng, `er0 = 0` nếu Sai) cực kỳ tối ưu cho các câu lệnh điều kiện `if / while / for`:

| Gadget Name | Địa chỉ ROM | Cú pháp CasioC | Chức năng chi tiết |
| :--- | :--- | :--- | :--- |
| `verify_eq` | `0x19536` | `if (a == b)` | So sánh bằng giữa `[addr1]` và `[addr2]` |
| `verify_ne` | `0x195C0` | `if (a != b)` | So sánh khác nhau |
| `verify_gt` | `0x19516` | `if (a > b)` | So sánh lớn hơn |
| `verify_lt` | `0x19528` | `if (a < b)` | So sánh nhỏ hơn |
| `verify_ge` | `0x194F8` | `if (a >= b)` | So sánh lớn hơn hoặc bằng |
| `verify_le` | `0x19506` | `if (a <= b)` | So sánh nhỏ hơn hoặc bằng |

*Cách dùng trong ASM ROP:*
```asm
xr0 = adr_of var_a, adr_of var_b
call 19516   # verify_gt
# er0 trả về 1 nếu a > b, ngược lại er0 = 0
```

---

## 2. 🗃️ NHÓM TRUY CẬP MẢNG & BẢNG DỮ LIỆU (Array & Jump Table)

| Gadget Name | Địa chỉ ROM | Cú pháp Assembly / CasioC | Chức năng |
| :--- | :--- | :--- | :--- |
| `load_table` | `0x13B9A` | `er0 = table[er2]` | Đọc phần tử mảng tại chỉ số `er2` vào `er0` |
| `load_table_er8` | `0x13B66` | `er8 = table[er0]` | Đọc mảng vào thanh ghi `er8` |
| `ea_dispatch` | `0x09C20` | `cmp_ea` + `call 1c64a` | Bảng điều hướng nhảy nhánh (Switch - Case) |
| `jump_table_idx`| `0x11974` | `setlr_pc, b 1:[er12+r0<<1]` | Nhảy theo bảng địa chỉ hàm với index `r0` |

---

## 3. ➕ NHÓM TOÁN HỌC & TĂNG GIẢM BỘ NHỚ TRỰC TIẾP (Direct Memory Math)

| Gadget Name | Địa chỉ ROM | Cú pháp | Ứng dụng trong Game |
| :--- | :--- | :--- | :--- |
| `[er4]+=1,rt` | `0x1332A` | `var++;` | Tăng trực tiếp biến trong RAM không cần load |
| `[er4]-=1,rt` | `0x13336` | `var--;` | Giảm trực tiếp biến trong RAM |
| `5[er8]+=1` | `0x0E254` | `player.hp++;` | Tăng biến thuộc tính của Struct tại offset +5 |
| `5[er8]-=1` | `0x2EB7C` | `player.hp--;` | Giảm biến thuộc tính Struct tại offset +5 |
| `er0*=er2,rt` | `0x1EDC8` | `a = b * c;` | Nhân 2 số nguyên 16-bit |
| `er0/=r2,rt` | `0x28C54` | `a = b / c;` | Chia số nguyên 16-bit cho 8-bit |
| `er0*=r2,er0+=er4`| `0x14BD4`| `a = (b * c) + d;`| Nhân cộng kết hợp tính tọa độ ma trận 2D |
| `daa r1,rt` | `0x1982E` | `daa(score);` | Chỉnh lý số thập phân BCD (cộng điểm) |
| `das r1,rt` | `0x19CBC` | `das(score);` | Chỉnh lý trừ số thập phân BCD |

---

## 4. 🎮 NHÓM NHẬP LIỆU & VÒNG LẶP (Input & Game Loop Pro)

| Gadget Name | Địa chỉ ROM | Cú pháp | Chức năng |
| :--- | :--- | :--- | :--- |
| `input_func` | `0x2F210` | `call 2F210` | Đọc input từ bàn phím phần cứng |
| `getkey` | `0x2F5EA` | `call 2F5EA` | Lấy Keycode của phím vừa bấm |
| `getscancode` | `0x1F24E` | `call 1F24E` | Quét ma trận quét phím phần cứng |
| `check_any_key` | `0x0E826` | `call 0E826` | Non-blocking check xem có nhấn phím không |
| `memcpy_auto_jump`| `0x2B2BA`| `call 2B2BA` | Chép lại Stack và nhảy lặp Game Loop |
| `loop_from_buffer`| `0x27738`| `call 27738` | Khởi tạo vòng lặp từ Buffer RAM |
| `num_to_hex` | `0x1ED58` | `call 1ED58` | Đổi số nguyên sang chuỗi Hex |
| `hex_to_num` | `0x24672` | `call 24672` | Đổi chuỗi Hex sang số nguyên |
| `r0_isdigit` | `0x1F422` | `call 1F422` | Kiểm tra ký tự có phải là chữ số 0..9 không |

---

## 5. 🚀 NHÓM NHẢY NHANH CON TRỎ STACK (SP Pro Gadgets)

Cho phép chương trình bỏ qua (skip) nhanh các byte dữ liệu/padding mà không cần nạp nhiều lệnh:

- `sp+=2` (`0x168F0`), `sp+=4` (`0x1D3C8`), `sp+=10` (`0x162D4`), `sp+=20` (`0x160C2`)
- `sp+=30, pop qr8` (`0x13320`)
- `sp+=40, pop qr8, pop xr4` (`0x13412`)
- `sp+=50, pop qr8` (`0x13184`)
- `sp+=60, pop xr8` (`0x21B9C`)
- `sp+=120, pop xr8` (`0x21B9A`)

---

## 6. 🎨 NHÓM ĐỒ HỌA & HIỂN THỊ (Core Rendering)

- `buffer_clear` (`0x08C60`): Xóa đệm màn hình `0xDDD4`.
- `render.ddd4` (`0x0947C`): Đẩy VRAM ra màn hình LCD `0xF800`.
- `line_draw` (`0x08E62`): Vẽ đoạn thẳng `(x1, y1, x2, y2)`.
- `pixel_draw` (`0x091FC`): Vẽ chấm pixel `(x, y)`.
- `render_bitmap` (`0x09848`): Vẽ sprite khối hình `(x, y, w, h)`.
- `fill_screen` (`0x08C0C`): Đổ màu toàn màn hình (`pattern`, `screen_id`).
- `printline` (`0x23DC8`): In dòng chữ (`row`, `pad`, `addr`).
- `smallprint` (`0x23DCC`): In chữ font nhỏ (`font_size`, `row`, `addr`).
- `hex_to_dec` (`0x09938`): In số thập phân (`r0`=số, `r1`=pad, `er2`=addr).
