# CasioC (.csc) — C-like for ClassWiz

Target: **fx-580VN X** và **fx-880BTG**. Biên dịch ra hex payload (`hdcompiler -f hex`), nhưng cú pháp C gọn, không rối như asm.

## Ví dụ

```c
model 580vnx;
opn [E9E0];

u16 counter at [EB40] = 0;   // at [ADDR] = cố định RAM, không có at = tự cấp @ [E800+]
u8  flag = 1;

csc main() {                 // thay void main()
    screen_del();            // buffer_clear
    if (counter == 10) {
        print("hi", 112, 16);
    } else {
        draw_line(0, 0, 50, 50);
    }
    while (counter < 5) {
        counter = counter + 1;
        delay(257);
    }
    for (i = 0; i < 10; i++) {
        draw_pixel(i, i);
    }
    asm {
        "pop xr0"            // vẫn cho chèn thẳng gadget
        "mov er0, er8"
    }
    goto end;
    end:
        render();
}

csc helper(u16 x, u16 y) {
    draw_line(x, y, 50, 50);
    render();
}
```

## Quy tắc (đã đổi @ -> at)

* Số thập phân mặc định (`257` không phải `0x0101`), địa chỉ `[ABCD]` thay `0xABCD`.
* `opn [ADDR]` thay `org`, `at [ADDR]` thay `@`.
* Biến `u8/u16/u32` hoặc `let/var` + `at [ADDR]` nếu cố định. `a = b` thay `mov`, `a += 1`, `a++`.
* `if (a == b)` thay `cmp eq`, `if (a > b)` etc. Compiler tự chọn gadget `er0+=er4`/`er0-=er2`... + cấp phát `r/er/xr/qr` dynamic.
* Gadget/label đều là hàm: `screen_del()`/`draw_line()`... Stdlib đặt tên lại cho dễ hiểu (giữ alias cũ).

## Stdlib (tên mới -> tên gốc)

| Mới | Gốc (580vnx / 880btg) |
|-----|------------------------|
| screen_del/clear | buffer_clear(.ca54) |
| screen_fill | fill_screen |
| draw_line | line_draw |
| draw_pixel | pixel_draw |
| print | line_print |
| print_hex | hex_byte |
| render | render.ddd4 / render |
| delay | delay |
| get_key | getkey / getkeycode |
| mem_copy | memcpy |
| mem_set | memset |

Đủ 580vnx 167 gadgets + 106 labels, 880btg 83 + 54. Gọi tên gốc nếu chưa có alias. `add/sub/mul/div` không lộ ra ngoài — viết `a = b + c` là đủ.

## So với Guide RAC 16 mục (giữ đủ nhưng gọn)

| Guide | CasioC |
|-------|--------|
| var/reg | `let a=10;` `u16 x at [EB40]=0;` |
| hex/string/array | `10` `[1,2]` `"hi world"` (`~`->space) |
| alias `as` | `alias tmp = er0;` (giữ) |
| label/goto/adr | `label:` `goto end;` `adr(label)`/`&label` |
| call/gadget | `call()` / `gadget g at [1234];` |
| macro | `macro add(a,b) { a+b }` |
| func | `csc main(){}` |
| org/backup | `opn [E9E0];` `backup [D000];` |
| section | `section main at [E9E0] {}` |
| build | `build {}` |
| eval | `eval(1+2)` / `const` |
| loop/fill | `for`/`while` + `fill(16,255)` |
| python | `python { }` |

## Lỗi có tên (cho user)

* `[E001]` model phải là `580vnx`/`880btg`
* `[E002]` `opn` phải là `opn [ADDR];`
* `[E003]` thiếu tên biến
* `[E004]` sau `at` phải là `[ADDR]`
* `[E100]` gadget/label không tồn tại
* `[W001]` hàm không tìm thấy -> bỏ qua

## Build

```sh
cargo run -p casio-asm -- --model 580vnx -f hex hello.csc
cargo run -p casio-asm -- --model 880btg -f hex hello.csc
cargo run -p casio-asm -- -f key hello.csc < hello.csc
```
Ví dụ: `examples/hello_580.csc`, `hello_880.csc`, `demo.csc`/`demo_880.csc`.
