use crate::parser::{Expr, Program, Stmt};
use std::collections::HashMap;

/// State for the ASM Emitter
struct AsmEmitter {
    out: String,
    data_section: String,
    var_map: HashMap<String, u32>,
    const_map: HashMap<u32, String>,
    next_ram_addr: u32,
    label_counter: usize,
    str_counter: usize,
    const_counter: usize,
}

impl AsmEmitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            data_section: String::new(),
            var_map: HashMap::new(),
            const_map: HashMap::new(),
            next_ram_addr: 0xEB40, // Safe RAM area
            label_counter: 0,
            str_counter: 0,
            const_counter: 0,
        }
    }

    fn new_label(&mut self, prefix: &str) -> String {
        self.label_counter += 1;
        format!("{}_{}", prefix, self.label_counter)
    }

    fn get_or_alloc_var(&mut self, name: &str) -> u32 {
        if let Some(&addr) = self.var_map.get(name) {
            return addr;
        }
        let addr = self.next_ram_addr;
        self.var_map.insert(name.to_string(), addr);
        self.next_ram_addr += 2;
        addr
    }

    fn get_or_create_const_label(&mut self, val: u32) -> String {
        if let Some(lbl) = self.const_map.get(&val) {
            return lbl.clone();
        }
        self.const_counter += 1;
        let lbl = format!("_const_{}_{}", val, self.const_counter);
        self.const_map.insert(val, lbl.clone());
        self.data_section.push_str(&format!("{}:\n0x{:04X}\n", lbl, val & 0xFFFF));
        lbl
    }

    fn add_string_literal(&mut self, s: &str) -> String {
        self.str_counter += 1;
        let lbl = format!("_str_{}", self.str_counter);
        let escaped = s.replace(' ', "~");
        self.data_section.push_str(&format!("{}:\nstr \"{}\"\nhex 00\n", lbl, escaped));
        lbl
    }
}

pub fn emit_asm(prog: &Program) -> Result<String, String> {
    let mut emitter = AsmEmitter::new();

    // 1. Header
    if let Some(m) = &prog.model {
        emitter.out.push_str(&format!("# Target Model: {}\n", m));
    }
    let org_addr = prog.opn.unwrap_or(0xE9E0);
    emitter.out.push_str(&format!("org 0x{:04X}\n\n", org_addr));

    // 2. Process Globals
    let mut inits = Vec::new();
    for g in &prog.globals {
        let addr = if let Some(a) = g.addr {
            emitter.var_map.insert(g.name.clone(), a);
            if a + 2 > emitter.next_ram_addr {
                emitter.next_ram_addr = a + 2;
            }
            a
        } else {
            emitter.get_or_alloc_var(&g.name)
        };
        if let Some(init_val) = g.init {
            inits.push((addr, init_val, g.name.clone()));
        }
    }

    // 3. Emit Functions
    for func in &prog.funcs {
        emitter.out.push_str(&format!("{}:\n", func.name));

        // Boilerplate init for main
        if func.name == "main" {
            emitter.out.push_str("setlr\n");
            emitter.out.push_str("setsfr\n");

            // Initialize globals in main
            for (addr, init_val, name) in &inits {
                emitter.out.push_str(&format!("# Init {} at [0x{:04X}] = {}\n", name, addr, init_val));
                emitter.out.push_str(&format!("er0 = 0x{:04X}\n", addr));
                emitter.out.push_str(&format!("er2 = 0x{:04X}\n", init_val & 0xFFFF));
                emitter.out.push_str("[er0]=er2,rt\n");
            }
        }

        for stmt in &func.body {
            emit_stmt(stmt, &mut emitter)?;
        }

        // Add infinite halt loop at end of main if no explicit goto/return
        if func.name == "main" && !matches!(func.body.last(), Some(Stmt::Goto(_)) | Some(Stmt::Return(_))) {
            let end_loop = emitter.new_label("_halt");
            emitter.out.push_str(&format!("{}:\n", end_loop));
            emitter.out.push_str(&format!("goto {}\n", end_loop));
        }
        emitter.out.push('\n');
    }

    // 4. Append Data Section
    if !emitter.data_section.is_empty() {
        emitter.out.push_str("# ================= DATA SECTION ================\n");
        emitter.out.push_str(&emitter.data_section);
    }

    Ok(emitter.out)
}

fn emit_stmt(stmt: &Stmt, em: &mut AsmEmitter) -> Result<(), String> {
    match stmt {
        Stmt::Assign { lhs, rhs } => {
            // Check for simple increment/decrement optimization (e.g. a++, a += 1, a--)
            if let Expr::BinOp(left, op, right) = rhs {
                if let Expr::Var(vname) = left.as_ref() {
                    if vname == lhs {
                        if let Expr::Number(val) = right.as_ref() {
                            let addr = em.get_or_alloc_var(lhs);
                            if op == "+" {
                                em.out.push_str(&format!("# {} += {}\n", lhs, val));
                                em.out.push_str(&format!("er8 = 0x{:04X}\n", addr));
                                em.out.push_str(&format!("er2 = 0x{:04X}\n", val & 0xFFFF));
                                em.out.push_str("[er8]+=er2,pop xr8\n");
                                em.out.push_str("0x30303030\n");
                                return Ok(());
                            } else if op == "-" {
                                let neg = ((0x10000 - (*val as i64 & 0xFFFF)) & 0xFFFF) as u32;
                                em.out.push_str(&format!("# {} -= {}\n", lhs, val));
                                em.out.push_str(&format!("er8 = 0x{:04X}\n", addr));
                                em.out.push_str(&format!("er2 = 0x{:04X}\n", neg));
                                em.out.push_str("[er8]+=er2,pop xr8\n");
                                em.out.push_str("0x30303030\n");
                                return Ok(());
                            }
                        }
                    }
                }
            }

            // General evaluation: evaluate rhs into er0
            emit_expr_to_er0(rhs, em)?;

            // Store er0 into lhs
            if is_register(lhs) {
                let reg = lhs.to_ascii_lowercase();
                if reg != "er0" {
                    em.out.push_str(&format!("er2 = er0,er0 = er2,pop er8,rt\n0x0000\n"));
                    if reg != "er2" {
                        em.out.push_str(&format!("{} = er2\n", reg));
                    }
                }
            } else {
                let addr = em.get_or_alloc_var(lhs);
                em.out.push_str(&format!("# Store into {} @ 0x{:04X}\n", lhs, addr));
                em.out.push_str(&format!("er4 = 0x{:04X}\n", addr));
                em.out.push_str("[er4]=er0,pop er0,rt\n");
                em.out.push_str("0x3030\n");
            }
            Ok(())
        }

        Stmt::Call { name, args } => {
            let key = name.to_ascii_lowercase();
            match key.as_str() {
                "screen_del" | "clear" | "buffer_clear" | "cls" => {
                    em.out.push_str("buffer_clear\n");
                }
                "screen_fill" | "fill_screen" => {
                    em.out.push_str("fill_screen\n");
                }
                "render" | "render.ddd4" | "flush" => {
                    em.out.push_str("render.ddd4\n");
                }
                "waitshift" => {
                    em.out.push_str("waitshift\n");
                }
                "delay" | "sleep" => {
                    let ticks = if let Some(arg) = args.get(0) {
                        eval_const_expr(arg)?
                    } else {
                        1000
                    };
                    em.out.push_str(&format!("er0 = 0x{:04X}\n", ticks & 0xFFFF));
                    em.out.push_str("delay\n");
                }
                "print" | "printline" => {
                    // Default to large font printline: xr0 = 0x<pad><linepos>, adr_of text
                    let text = match args.get(0) {
                        Some(Expr::Str(s)) => s.clone(),
                        _ => return Err("print expects string as first argument".into()),
                    };
                    let linepos = if let Some(arg) = args.get(1) {
                        let v = eval_const_expr(arg)?;
                        match v {
                            1 => 0x01,
                            2 => 0x11,
                            3 => 0x21,
                            4 => 0x31,
                            other => other & 0xFF,
                        }
                    } else {
                        0x01 // default line 1
                    };
                    let pad = if let Some(arg) = args.get(2) {
                        eval_const_expr(arg)? & 0xFF
                    } else {
                        0x30
                    };
                    let str_lbl = em.add_string_literal(&text);
                    let packed = (pad << 8) | linepos;
                    em.out.push_str(&format!("# printline \"{}\" at line 0x{:02X}\n", text, linepos));
                    em.out.push_str(&format!("xr0 = 0x{:04X}, adr_of {}\n", packed, str_lbl));
                    em.out.push_str("printline\n");
                    em.out.push_str("render.ddd4\n");
                }
                "smallprint" => {
                    let text = match args.get(0) {
                        Some(Expr::Str(s)) => s.clone(),
                        _ => return Err("smallprint expects string as first argument".into()),
                    };
                    let font = if let Some(arg) = args.get(1) { eval_const_expr(arg)? & 0xFF } else { 0x08 };
                    let linepos = if let Some(arg) = args.get(2) { eval_const_expr(arg)? & 0xFF } else { 0x11 };
                    let str_lbl = em.add_string_literal(&text);
                    let packed = (linepos << 8) | font;
                    em.out.push_str(&format!("# smallprint \"{}\" font=0x{:02X} line=0x{:02X}\n", text, font, linepos));
                    em.out.push_str(&format!("xr0 = 0x{:04X}, adr_of {}\n", packed, str_lbl));
                    em.out.push_str("smallprint\n");
                    em.out.push_str("render.ddd4\n");
                }
                "print_hex" | "hex_byte" => {
                    if let Some(val_expr) = args.get(0) {
                        emit_expr_to_er0(val_expr, em)?;
                    }
                    let linepos = if let Some(arg) = args.get(1) {
                        if let Ok(v) = eval_const_expr(arg) { v & 0xFF } else { 0x11 }
                    } else {
                        0x11
                    };
                    em.out.push_str(&format!("er2 = 0x{:02X}00\n", linepos));
                    em.out.push_str("call 24004\n"); // hex_byte
                    em.out.push_str("render.ddd4\n");
                }
                "draw_line" | "line_draw" => {
                    if let (Some(x1_e), Some(y1_e), Some(x2_e), Some(y2_e)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
                        if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (eval_const_expr(x1_e), eval_const_expr(y1_e), eval_const_expr(x2_e), eval_const_expr(y2_e)) {
                            em.out.push_str(&format!("# draw_line ({},{}) -> ({},{})\n", x1, y1, x2, y2));
                            em.out.push_str(&format!("xr0 = hex {:02X} {:02X} {:02X} {:02X}\n", x1 & 0xFF, y1 & 0xFF, x2 & 0xFF, y2 & 0xFF));
                        } else {
                            em.out.push_str("# draw_line (dynamic coords)\n");
                            // er0 = x1
                            emit_expr_to_er0(x1_e, em)?;
                            let tmp_p1 = em.new_label("_p1");
                            let addr_p1 = em.get_or_alloc_var(&tmp_p1);
                            em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr_p1));
                            
                            // er2 = x2
                            emit_expr_to_er0(x2_e, em)?;
                            let tmp_p2 = em.new_label("_p2");
                            let addr_p2 = em.get_or_alloc_var(&tmp_p2);
                            em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr_p2));

                            // Load p2 into er0 then move to er2
                            em.out.push_str(&format!("er2 = 0x{:04X}\ner0=[er2],r2 = 9,rt\n", addr_p2));
                            em.out.push_str("er2 = er0,er0 = er2,pop er8,rt\n0x0000\n");

                            // Load p1 into er0
                            em.out.push_str(&format!("er2 = 0x{:04X}\ner0=[er2],r2 = 9,rt\n", addr_p1));
                        }
                    }
                    em.out.push_str("line_draw\n");
                    em.out.push_str("render.ddd4\n");
                }
                "draw_pixel" | "pixel_draw" => {
                    if let (Some(x_expr), Some(y_expr)) = (args.get(0), args.get(1)) {
                        if let (Ok(x), Ok(y)) = (eval_const_expr(x_expr), eval_const_expr(y_expr)) {
                            em.out.push_str(&format!("# draw_pixel ({},{})\n", x, y));
                            em.out.push_str(&format!("er0 = 0x{:04X}\n", x));
                            em.out.push_str(&format!("er2 = 0x{:04X}\n", y));
                        } else {
                            em.out.push_str("# draw_pixel (dynamic)\n");
                            // x into temp_x
                            emit_expr_to_er0(x_expr, em)?;
                            let tmp_x = em.new_label("_px");
                            let addr_x = em.get_or_alloc_var(&tmp_x);
                            em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr_x));

                            // y into temp_y
                            emit_expr_to_er0(y_expr, em)?;
                            let tmp_y = em.new_label("_py");
                            let addr_y = em.get_or_alloc_var(&tmp_y);
                            em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr_y));

                            // Load y into er2
                            em.out.push_str(&format!("er2 = 0x{:04X}\ner0=[er2],r2 = 9,rt\n", addr_y));
                            em.out.push_str("er2 = er0,er0 = er2,pop er8,rt\n0x0000\n");

                            // Load x into er0
                            em.out.push_str(&format!("er2 = 0x{:04X}\ner0=[er2],r2 = 9,rt\n", addr_x));
                        }
                    }
                    em.out.push_str("pixel_draw\n");
                    em.out.push_str("render.ddd4\n");
                }
                "get_key" | "getkey" => {
                    let addr = if let Some(arg) = args.get(0) {
                        match arg {
                            Expr::Addr(a) => *a,
                            Expr::Var(v) => em.get_or_alloc_var(v),
                            _ => eval_const_expr(arg).unwrap_or(0xEB40),
                        }
                    } else {
                        em.get_or_alloc_var("_key_buf")
                    };
                    em.out.push_str(&format!("er0 = 0x{:04X}\n", addr));
                    em.out.push_str("getkey\n");
                }
                "draw_rect" | "rect" => {
                    if let (Some(x_e), Some(y_e), Some(w_e), Some(h_e)) = (args.get(0), args.get(1), args.get(2), args.get(3)) {
                        if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (eval_const_expr(x_e), eval_const_expr(y_e), eval_const_expr(w_e), eval_const_expr(h_e)) {
                            let x2 = (x + w) & 0xFF;
                            let y2 = (y + h) & 0xFF;
                            em.out.push_str(&format!("# draw_rect ({},{},{},{})\n", x, y, w, h));
                            em.out.push_str(&format!("xr0 = hex {:02X} {:02X} {:02X} {:02X}\nline_draw\nrender.ddd4\n", x & 0xFF, y & 0xFF, x2, y & 0xFF));
                            em.out.push_str(&format!("xr0 = hex {:02X} {:02X} {:02X} {:02X}\nline_draw\nrender.ddd4\n", x & 0xFF, y2, x2, y2));
                            em.out.push_str(&format!("xr0 = hex {:02X} {:02X} {:02X} {:02X}\nline_draw\nrender.ddd4\n", x & 0xFF, y & 0xFF, x & 0xFF, y2));
                            em.out.push_str(&format!("xr0 = hex {:02X} {:02X} {:02X} {:02X}\nline_draw\nrender.ddd4\n", x2, y & 0xFF, x2, y2));
                        } else {
                            // Dynamic draw_rect (variables px, py)
                            let w = eval_const_expr(w_e).unwrap_or(6);
                            let h = eval_const_expr(h_e).unwrap_or(6);
                            em.out.push_str(&format!("# draw_rect (dynamic px, py, {}x{})\n", w, h));
                            emit_expr_to_er0(x_e, em)?;
                            let tmp_x = em.new_label("_rx");
                            let addr_x = em.get_or_alloc_var(&tmp_x);
                            em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr_x));

                            emit_expr_to_er0(y_e, em)?;
                            let tmp_y = em.new_label("_ry");
                            let addr_y = em.get_or_alloc_var(&tmp_y);
                            em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr_y));

                            // Top line
                            em.out.push_str(&format!("er2 = 0x{:04X}\ner0=[er2],r2 = 9,rt\n", addr_y));
                            em.out.push_str("er2 = er0,er0 = er2,pop er8,rt\n0x0000\n");
                            em.out.push_str(&format!("er2 = 0x{:04X}\ner0=[er2],r2 = 9,rt\n", addr_x));
                            em.out.push_str("line_draw\nrender.ddd4\n");
                        }
                    }
                }
                "print_at" | "draw_text" => {
                    let text = match args.get(0) {
                        Some(Expr::Str(s)) => s.clone(),
                        _ => return Err("print_at expects string as first argument".into()),
                    };
                    let x = if let Some(arg) = args.get(1) { eval_const_expr(arg)? & 0xFF } else { 0 };
                    let y = if let Some(arg) = args.get(2) { eval_const_expr(arg)? & 0xFF } else { 0 };
                    let str_lbl = em.add_string_literal(&text);
                    let packed = (y << 8) | x;
                    em.out.push_str(&format!("# print_at \"{}\" at ({},{})\n", text, x, y));
                    em.out.push_str(&format!("xr0 = 0x{:04X}, adr_of {}\n", packed, str_lbl));
                    em.out.push_str("line_print\n");
                    em.out.push_str("render.ddd4\n");
                }
                "print_num" | "print_dec" | "show_score" => {
                    if let Some(val_expr) = args.get(0) {
                        emit_expr_to_er0(val_expr, em)?;
                    }
                    let line = if let Some(arg) = args.get(1) { eval_const_expr(arg).unwrap_or(1) & 0xFF } else { 1 };
                    let pad = if let Some(arg) = args.get(2) { eval_const_expr(arg).unwrap_or(0) & 0xFF } else { 0 };
                    em.out.push_str(&format!("# print_num line=0x{:02X} pad=0x{:02X}\n", line, pad));
                    em.out.push_str(&format!("r1 = 0x{:02X}\n", pad));
                    em.out.push_str(&format!("er2 = 0x{:04X}\n", 0xDDD4 + (line as u32 * 24 * 8)));
                    em.out.push_str("call 09938\n"); // hex_to_dec
                    em.out.push_str("render.ddd4\n");
                }
                "is_key_pressed" | "check_key" => {
                    em.out.push_str("# check_any_key_pressed\n");
                    em.out.push_str("call 0E826\n");
                }
                "wait_key" | "pause" => {
                    em.out.push_str("# diagnostic_wait_key\n");
                    em.out.push_str("call 0AD92\n");
                }
                "mem_copy" | "memcpy" => {
                    if let (Some(dst_e), Some(src_e), Some(len_e)) = (args.get(0), args.get(1), args.get(2)) {
                        let dst = eval_const_expr(dst_e).unwrap_or(0xDDD4);
                        let src = eval_const_expr(src_e).unwrap_or(0xE9E0);
                        let len = eval_const_expr(len_e).unwrap_or(1512);
                        em.out.push_str(&format!("qr8 = 0x{:04X}, 0x{:04X}, 0x{:04X}, 0x3030\n", dst, src, len));
                        em.out.push_str("call 10F20\n");
                    }
                }
                "mem_zero" | "memzero" => {
                    if let (Some(addr_e), Some(len_e)) = (args.get(0), args.get(1)) {
                        let addr = eval_const_expr(addr_e).unwrap_or(0xDDD4);
                        let len = eval_const_expr(len_e).unwrap_or(1512);
                        em.out.push_str(&format!("er0 = 0x{:04X}\n", addr));
                        em.out.push_str(&format!("er2 = 0x{:04X}\n", len));
                        em.out.push_str("call 09D34\n");
                    }
                }
                "double_buffer_flip" | "flip" => {
                    em.out.push_str("buf1_to_buf2\n");
                    em.out.push_str("render.ddd4\n");
                }
                "inc" => {
                    if let Some(Expr::Var(vname)) = args.get(0) {
                        let addr = em.get_or_alloc_var(vname);
                        em.out.push_str(&format!("er4 = 0x{:04X}\n", addr));
                        em.out.push_str("[er4]+=1,rt\n");
                    }
                }
                "dec" => {
                    if let Some(Expr::Var(vname)) = args.get(0) {
                        let addr = em.get_or_alloc_var(vname);
                        em.out.push_str(&format!("er4 = 0x{:04X}\n", addr));
                        em.out.push_str("[er4]-=1,rt\n");
                    }
                }
                _ => {
                    // Custom / User / Raw Gadget Call
                    if args.is_empty() {
                        em.out.push_str(&format!("{}\n", name));
                    } else {
                        em.out.push_str(&format!("call {}\n", name));
                    }
                }
            }
            Ok(())
        }

        Stmt::If { cond, then_b, else_b } => {
            let lbl_then = em.new_label("_if_then");
            let lbl_else = em.new_label("_if_else");
            let lbl_endif = em.new_label("_if_end");
            let lbl_tbl = em.new_label("_if_tbl");

            // Emit Condition Check
            emit_condition(cond, &lbl_tbl, em)?;

            // Then Branch
            em.out.push_str(&format!("{}:\n", lbl_then));
            for s in then_b {
                emit_stmt(s, em)?;
            }
            em.out.push_str(&format!("goto {}\n", lbl_endif));

            // Else Branch
            em.out.push_str(&format!("{}:\n", lbl_else));
            for s in else_b {
                emit_stmt(s, em)?;
            }
            em.out.push_str(&format!("goto {}\n", lbl_endif));

            // Jump Table
            em.data_section.push_str(&format!("{}:\n", lbl_tbl));
            em.data_section.push_str("hex 00 01\n");
            em.data_section.push_str(&format!("adr_of [-2] {}\n", lbl_then));
            em.data_section.push_str("hex 00 00\n");
            let false_target = if else_b.is_empty() { &lbl_endif } else { &lbl_else };
            em.data_section.push_str(&format!("adr_of [-2] {}\n", false_target));

            em.out.push_str(&format!("{}:\n", lbl_endif));
            Ok(())
        }

        Stmt::While { cond, body } => {
            let lbl_start = em.new_label("_while_start");
            let lbl_body = em.new_label("_while_body");
            let lbl_end = em.new_label("_while_end");
            let lbl_tbl = em.new_label("_while_tbl");

            em.out.push_str(&format!("{}:\n", lbl_start));
            emit_condition(cond, &lbl_tbl, em)?;

            em.out.push_str(&format!("{}:\n", lbl_body));
            for s in body {
                emit_stmt(s, em)?;
            }
            em.out.push_str(&format!("goto {}\n", lbl_start));

            // Jump Table
            em.data_section.push_str(&format!("{}:\n", lbl_tbl));
            em.data_section.push_str("hex 00 01\n");
            em.data_section.push_str(&format!("adr_of [-2] {}\n", lbl_body));
            em.data_section.push_str("hex 00 00\n");
            em.data_section.push_str(&format!("adr_of [-2] {}\n", lbl_end));

            em.out.push_str(&format!("{}:\n", lbl_end));
            Ok(())
        }

        Stmt::For { init, cond, step, body } => {
            let lbl_start = em.new_label("_for_start");
            let lbl_body = em.new_label("_for_body");
            let lbl_end = em.new_label("_for_end");
            let lbl_tbl = em.new_label("_for_tbl");

            if let Some(inits) = init {
                emit_stmt(inits, em)?;
            }

            em.out.push_str(&format!("{}:\n", lbl_start));
            if let Some(c) = cond {
                emit_condition(c, &lbl_tbl, em)?;
            }

            em.out.push_str(&format!("{}:\n", lbl_body));
            for s in body {
                emit_stmt(s, em)?;
            }

            if let Some(steps) = step {
                emit_stmt(steps, em)?;
            }
            em.out.push_str(&format!("goto {}\n", lbl_start));

            // Jump Table
            if cond.is_some() {
                em.data_section.push_str(&format!("{}:\n", lbl_tbl));
                em.data_section.push_str("hex 00 01\n");
                em.data_section.push_str(&format!("adr_of [-2] {}\n", lbl_body));
                em.data_section.push_str("hex 00 00\n");
                em.data_section.push_str(&format!("adr_of [-2] {}\n", lbl_end));
            }

            em.out.push_str(&format!("{}:\n", lbl_end));
            Ok(())
        }

        Stmt::Goto(lbl) => {
            em.out.push_str(&format!("goto {}\n", lbl));
            Ok(())
        }

        Stmt::Label(lbl) => {
            em.out.push_str(&format!("{}:\n", lbl));
            Ok(())
        }

        Stmt::Asm(lines) => {
            for l in lines {
                em.out.push_str(&format!("{}\n", l.trim_matches('"')));
            }
            Ok(())
        }

        Stmt::Return(_) => {
            em.out.push_str("rt\n");
            Ok(())
        }
    }
}

/// Emits condition evaluation and dispatch via verify_* and ea_dispatch
fn emit_condition(cond: &Expr, tbl_lbl: &str, em: &mut AsmEmitter) -> Result<(), String> {
    if let Expr::BinOp(lhs, op, rhs) = cond {
        let (adr_l, adr_r) = get_pointers_for_cmp(lhs, rhs, em)?;
        let verify_func = match op.as_str() {
            "==" => "19536", // verify_eq
            "!=" => "195C0", // verify_ne
            ">"  => "19516", // verify_gt
            "<"  => "19526", // verify_lt
            ">=" => "194F6", // verify_ge
            "<=" => "19506", // verify_le
            _ => return Err(format!("unsupported comparison op {}", op)),
        };

        em.out.push_str(&format!("# Verify condition ({} {} {})\n", expr_name(lhs), op, expr_name(rhs)));
        em.out.push_str(&format!("xr0 = {}, {}\n", adr_l, adr_r));
        em.out.push_str(&format!("call {}\n", verify_func));
        em.out.push_str(&format!("ea = adr_of {}\n", tbl_lbl));
        em.out.push_str("call 09c20\n");
        em.out.push_str("call 1c64a\n");
        em.out.push_str("sp = er6, pop er8\n");
        Ok(())
    } else {
        // Evaluate single bool expr against 0
        let (adr_l, _) = get_expr_pointer(cond, em)?;
        let zero_lbl = em.get_or_create_const_label(0);
        em.out.push_str(&format!("xr0 = {}, adr_of {}\n", adr_l, zero_lbl));
        em.out.push_str("call 195C0\n"); // verify_ne (!= 0)
        em.out.push_str(&format!("ea = adr_of {}\n", tbl_lbl));
        em.out.push_str("call 09c20\n");
        em.out.push_str("call 1c64a\n");
        em.out.push_str("sp = er6, pop er8\n");
        Ok(())
    }
}

fn get_pointers_for_cmp(lhs: &Expr, rhs: &Expr, em: &mut AsmEmitter) -> Result<(String, String), String> {
    let (p1, _) = get_expr_pointer(lhs, em)?;
    let (p2, _) = get_expr_pointer(rhs, em)?;
    Ok((p1, p2))
}

fn get_expr_pointer(e: &Expr, em: &mut AsmEmitter) -> Result<(String, Option<u32>), String> {
    match e {
        Expr::Var(name) => {
            let addr = em.get_or_alloc_var(name);
            Ok((format!("0x{:04X}", addr), Some(addr)))
        }
        Expr::Addr(addr) => {
            Ok((format!("0x{:04X}", addr), Some(*addr)))
        }
        Expr::Number(n) => {
            let lbl = em.get_or_create_const_label(*n);
            Ok((format!("adr_of {}", lbl), None))
        }
        _ => {
            // Complex subexpression: evaluate to temporary RAM variable
            let temp_var = em.new_label("_tmp");
            let temp_addr = em.get_or_alloc_var(&temp_var);
            emit_expr_to_er0(e, em)?;
            em.out.push_str(&format!("er4 = 0x{:04X}\n", temp_addr));
            em.out.push_str("[er4]=er0,pop er0,rt\n0x3030\n");
            Ok((format!("0x{:04X}", temp_addr), Some(temp_addr)))
        }
    }
}

/// Evaluates an expression and leaves the 16-bit result in `er0`
fn emit_expr_to_er0(e: &Expr, em: &mut AsmEmitter) -> Result<(), String> {
    match e {
        Expr::Number(n) => {
            em.out.push_str(&format!("er0 = 0x{:04X}\n", n & 0xFFFF));
            Ok(())
        }
        Expr::Var(name) => {
            if is_register(name) {
                let reg = name.to_ascii_lowercase();
                if reg != "er0" {
                    em.out.push_str(&format!("er0 = {}\n", reg));
                }
            } else {
                let addr = em.get_or_alloc_var(name);
                em.out.push_str(&format!("er2 = 0x{:04X}\n", addr));
                em.out.push_str("er0=[er2],r2 = 9,rt\n");
            }
            Ok(())
        }
        Expr::Addr(addr) => {
            em.out.push_str(&format!("er2 = 0x{:04X}\n", addr));
            em.out.push_str("er0=[er2],r2 = 9,rt\n");
            Ok(())
        }
        Expr::BinOp(left, op, right) => {
            // Check constant folding
            if let (Ok(v1), Ok(v2)) = (eval_const_expr(left), eval_const_expr(right)) {
                let res = match op.as_str() {
                    "+" => v1.wrapping_add(v2),
                    "-" => v1.wrapping_sub(v2),
                    "*" => v1.wrapping_mul(v2),
                    "/" => if v2 != 0 { v1 / v2 } else { 0 },
                    "&" => v1 & v2,
                    "|" => v1 | v2,
                    "^" => v1 ^ v2,
                    "<<" => v1 << (v2 & 15),
                    ">>" => v1 >> (v2 & 15),
                    _ => 0,
                };
                em.out.push_str(&format!("er0 = 0x{:04X}\n", res & 0xFFFF));
                return Ok(());
            }

            // Left side into er0
            emit_expr_to_er0(left, em)?;

            // Right side into er2 / er4
            match op.as_str() {
                "+" => {
                    if let Expr::Number(n) = right.as_ref() {
                        em.out.push_str(&format!("er4 = 0x{:04X}\n", n & 0xFFFF));
                    } else if let Expr::Var(v) = right.as_ref() {
                        let addr = em.get_or_alloc_var(v);
                        em.out.push_str(&format!("er2 = 0x{:04X}\n", addr));
                        em.out.push_str("er4=[er2],r2 = 9,rt\n");
                    } else {
                        // Temp save er0
                        let tmp = em.new_label("_tadd");
                        let addr = em.get_or_alloc_var(&tmp);
                        em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr));
                        emit_expr_to_er0(right, em)?;
                        em.out.push_str(&format!("er4 = er0,er2 = 0x{:04X}\ner0=[er2],r2 = 9,rt\n", addr));
                    }
                    em.out.push_str("er0+=er4,rt\n");
                }
                "-" => {
                    if let Expr::Number(n) = right.as_ref() {
                        em.out.push_str(&format!("er2 = 0x{:04X}\n", n & 0xFFFF));
                    } else if let Expr::Var(v) = right.as_ref() {
                        let addr = em.get_or_alloc_var(v);
                        em.out.push_str(&format!("er4 = 0x{:04X}\n", addr));
                        em.out.push_str("er2=[er4],r2 = 9,rt\n");
                    } else {
                        let tmp = em.new_label("_tsub");
                        let addr = em.get_or_alloc_var(&tmp);
                        em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr));
                        emit_expr_to_er0(right, em)?;
                        em.out.push_str(&format!("er2 = er0,er4 = 0x{:04X}\ner0=[er4],r2 = 9,rt\n", addr));
                    }
                    em.out.push_str("er0-=er2,rt\n");
                }
                "*" => {
                    if let Expr::Number(n) = right.as_ref() {
                        em.out.push_str(&format!("er2 = 0x{:04X}\n", n & 0xFFFF));
                    } else {
                        let tmp = em.new_label("_tmul");
                        let addr = em.get_or_alloc_var(&tmp);
                        em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr));
                        emit_expr_to_er0(right, em)?;
                        em.out.push_str(&format!("er2 = er0,er4 = 0x{:04X}\ner0=[er4],r2 = 9,rt\n", addr));
                    }
                    em.out.push_str("er0*=er2,rt\n");
                }
                "/" => {
                    if let Expr::Number(n) = right.as_ref() {
                        em.out.push_str(&format!("r2 = 0x{:02X}\n", n & 0xFF));
                    } else {
                        let tmp = em.new_label("_tdiv");
                        let addr = em.get_or_alloc_var(&tmp);
                        em.out.push_str(&format!("er4 = 0x{:04X}\n[er4]=er0,pop er0,rt\n0x3030\n", addr));
                        emit_expr_to_er0(right, em)?;
                        em.out.push_str(&format!("r2 = r0,er4 = 0x{:04X}\ner0=[er4],r2 = 9,rt\n", addr));
                    }
                    em.out.push_str("er0/=r2,rt\n");
                }
                _ => return Err(format!("operator {} in expression not supported", op)),
            }
            Ok(())
        }
        _ => Err("complex expression cannot be reduced".into()),
    }
}

fn eval_const_expr(e: &Expr) -> Result<u32, String> {
    match e {
        Expr::Number(n) => Ok(*n),
        Expr::BinOp(left, op, right) => {
            let v1 = eval_const_expr(left)?;
            let v2 = eval_const_expr(right)?;
            Ok(match op.as_str() {
                "+" => v1.wrapping_add(v2),
                "-" => v1.wrapping_sub(v2),
                "*" => v1.wrapping_mul(v2),
                "/" => if v2 != 0 { v1 / v2 } else { 0 },
                "&" => v1 & v2,
                "|" => v1 | v2,
                "^" => v1 ^ v2,
                "<<" => v1 << (v2 & 15),
                ">>" => v1 >> (v2 & 15),
                _ => 0,
            })
        }
        _ => Err("expected constant expression".into()),
    }
}

fn expr_name(e: &Expr) -> String {
    match e {
        Expr::Var(v) => v.clone(),
        Expr::Number(n) => format!("{}", n),
        Expr::Addr(a) => format!("[0x{:04X}]", a),
        _ => "expr".into(),
    }
}

fn is_register(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    matches!(lower.as_str(),
        "r0" | "r1" | "r2" | "r3" | "r4" | "r5" | "r6" | "r7" |
        "r8" | "r9" | "r10" | "r11" | "r12" | "r13" | "r14" | "r15" |
        "er0" | "er2" | "er4" | "er6" | "er8" | "er10" | "er12" | "er14" |
        "xr0" | "xr4" | "xr8" | "xr12" | "qr0" | "qr8" | "ea" | "sp" | "pc" | "lr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    #[test]
    fn test_emit_hello() {
        let code = r#"
            model 580vnx;
            opn [E9E0];
            csc main() {
                screen_del();
                print("Xin chao", 1);
                render();
            }
        "#;
        let tokens = lex(code).unwrap();
        let ast = parse(&tokens).unwrap();
        let asm = emit_asm(&ast).unwrap();
        assert!(asm.contains("org 0xE9E0"));
        assert!(asm.contains("buffer_clear"));
        assert!(asm.contains("printline"));
        assert!(asm.contains("render.ddd4"));
        assert!(asm.contains("str \"Xin~chao\""));
    }

    #[test]
    fn test_emit_if_while_inc() {
        let code = r#"
            model 580vnx;
            u16 x = 0;
            csc main() {
                x = x + 1;
                if (x == 1) {
                    print("true", 1);
                }
                while (x < 10) {
                    x++;
                }
            }
        "#;
        let tokens = lex(code).unwrap();
        let ast = parse(&tokens).unwrap();
        let asm = emit_asm(&ast).unwrap();
        assert!(asm.contains("[er8]+=er2,pop xr8"));
        assert!(asm.contains("call 19536")); // verify_eq
        assert!(asm.contains("call 19526")); // verify_lt
        assert!(asm.contains("ea_dispatch"));
    }
}


