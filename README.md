# CasioC — ClassWiz, but Make It C

<p align="center">
  <img src="https://img.shields.io/badge/language-CasioC%20(.csc)-blue?style=for-the-badge" />
  <img src="https://img.shields.io/badge/models-580VN%20X%20%7C%20880BTG-green?style=for-the-badge" />
  <img src="https://img.shields.io/badge/license-GPL--2.0-orange?style=for-the-badge" />
  <img src="https://img.shields.io/badge/status-beta-yellow?style=for-the-badge" />
</p>

<p align="center">
  <b>Tiếng Việt</b> | <a href="#english">English</a>
</p>

---

### CasioC là gì?

Bạn biết `hdcompiler` — viết ROP bằng tay, `er0 = ...`, `goto label`, nhớ từng `pop xr0`. Mệt.

**CasioC** là ngôn ngữ mới cho ClassWiz: viết như C, biên dịch ra hex y hệt hdcompiler, nhưng gọn và ấm áp hơn.

```c
model 580vnx;
opn [E9E0];

u16 counter at [EB40] = 0;

csc main() {
    screen_del();
    counter = counter + 1;

    if (counter == 10) {
        print("hello", 112, 16);
    } else {
        draw_line(0, 0, 50, 50);
    }

    render();
    delay(257);
}
```

* `a = b + c` thay `er0+=er4`, `if(a==b)` thay `cmp`, compiler tự chọn gadget và tự cấp phát thanh ghi (`r/er/xr/qr`).
* Muốn chọc sâu? Vẫn được: `asm { "pop xr0" }`.
* `580VN X` 167 gadgets + 106 labels, `880BTG` 83 + 54 — càng nhiều gadget càng tốt, đã nạp đủ.

```sh
cargo run -p casio-asm -- --model 580vnx -f hex hello.csc
cargo run -p casio-asm -- --model 880btg -f hex hello.csc
```

---

### <a id="english"></a>What is CasioC?

You know `hdcompiler` — hand-written ROP, `er0 = ...`, `goto label`, remembering every `pop xr0`. Tiring.

**CasioC** is a new language for ClassWiz: write like C, compile to the same hex as hdcompiler, but clean and human.

```c
model 580vnx;
opn [E9E0];

u16 counter at [EB40] = 0;

csc main() {
    screen_del();
    counter = counter + 1;

    if (counter == 10) {
        print("hello", 112, 16);
    } else {
        draw_line(0, 0, 50, 50);
    }

    render();
    delay(257);
}
```

* `a = b + c` instead of `er0+=er4`, `if(a==b)` instead of `cmp`, compiler picks gadgets and does register allocation for you.
* Need low-level? Still there: `asm { "pop xr0" }`.
* Both models fully supported, stdlib aliased: `screen_del`→`buffer_clear`, `draw_line`→`line_draw`, `render`→`render.ddd4`...

```sh
cargo run -p casio-asm -- --model 580vnx -f hex hello.csc
cargo run -p casio-asm -- --model 880btg -f hex hello.csc
```

### Docs

* `docs/casioc.md` — full language spec (16 guide sections in C form)
* `examples/*.csc` — hello / demo for both models

### License

GPL-2.0. See `LICENSE`. Based on `hdcompiler` / `RAC-Compiler` (luongvantam) and `nX-U16` docs. Human for humans.
