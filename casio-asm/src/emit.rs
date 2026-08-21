use crate::model::ModelDb;
use crate::parser::{Expr, Program, Stmt};

/// Very small emitter v0: turns .csc AST into hdcompiler-compatible byte stream.
/// Dynamic register allocation is done per-function via a simple bump allocator
/// for locals; globals @ [ADDR] are fixed. Gadget names are looked up in
/// ModelDb::gadgets / labels (case-insensitive) and emitted as `call` (0x30300000+addr).
pub fn emit(prog: &Program, db: &ModelDb) -> Result<(Vec<u8>, u32, String), String> {
    let mut out: Vec<u8> = Vec::new();
    let mut log = String::new();
    let home: u32 = prog.opn.unwrap_or(0xE9E0);
    let mut alloc = Allocator::new(0xE800);
    let mut labels: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut patches: Vec<(usize, String)> = Vec::new();

    for g in &prog.globals {
        let addr = g.addr.unwrap_or_else(|| alloc.alloc(&g.name, 2));
        log.push_str(&format!("global {} @ [{:04X}] = {:?}\n", g.name, addr, g.init));
        alloc.map.insert(g.name.clone(), addr);
    }

    for func in &prog.funcs {
        log.push_str(&format!("func {} ({} params)\n", func.name, func.params.len()));
        for stmt in &func.body {
            emit_stmt(stmt, db, &mut out, &mut log, &mut alloc, &mut labels, &mut patches)?;
        }
    }

    if prog.funcs.is_empty() {
        log.push_str("warning: no csc main() found\n");
    }

    // patch goto targets: each patch is 2-byte LE of home + label_pos
    for (pos, lbl) in patches {
        if let Some(&off) = labels.get(&lbl) {
            let target = home + off as u32;
            out[pos] = (target & 0xFF) as u8;
            out[pos + 1] = ((target >> 8) & 0xFF) as u8;
            log.push_str(&format!("  ; patch {} -> [{:04X}]\n", lbl, target));
        } else {
            log.push_str(&format!("  ; warning: label {lbl} not found\n"));
        }
    }

    Ok((out, home, log))
}

struct Allocator {
    next: u32,
    map: std::collections::HashMap<String, u32>,
}
impl Allocator {
    fn new(base: u32) -> Self { Self { next: base, map: std::collections::HashMap::new() } }
    fn alloc(&mut self, name: &str, sz: usize) -> u32 {
        if let Some(&a) = self.map.get(name) { return a; }
        let a = self.next;
        self.map.insert(name.to_string(), a);
        self.next += sz as u32;
        a
    }
}

fn emit_stmt(s: &Stmt, db: &ModelDb, out: &mut Vec<u8>, log: &mut String, alloc: &mut Allocator, labels: &mut std::collections::HashMap<String, usize>, patches: &mut Vec<(usize, String)>) -> Result<(), String> {
    match s {
        Stmt::Assign { lhs, rhs } => {
            let reg = lhs.as_str();
            let is_reg = is_register(reg);
            if is_reg {
                let gadget_name = format!("pop {}", reg.to_ascii_lowercase());
                emit_call(&gadget_name, db, out, log)?;
                let v = eval_expr(rhs, db)?;
                let sz = reg_size(reg) as usize;
                emit_le(v, sz, out);
            } else {
                let v = eval_expr(rhs, db)?;
                let addr = alloc.map.get(lhs).copied().unwrap_or_else(|| alloc.alloc(lhs, 2));
                // er0 = v
                emit_call("pop er0", db, out, log)?;
                emit_le(v, 2, out);
                // store: [addr] = er0  (use [er0]=er2 etc gadget if available, else log)
                log.push_str(&format!("  ; {} @ [{:04X}] = {}\n", lhs, addr, v));
                // try store gadget: "[er0]=er2" etc not yet - placeholder store
                // For now emit address as data for later store
                emit_le(addr, 2, out);
            }
            Ok(())
        }
        Stmt::Call { name, args } => {
            let key = name.to_ascii_lowercase();
            let found = stdlib_candidates(&key).iter().any(|c|
                db.gadgets.map.contains_key(c) || db.labels.labels.contains_key(c) || db.labels.data.contains_key(c)
            );
            if found {
                emit_call(&key, db, out, log)?;
                for a in args {
                    if let Expr::Str(s) = a {
                        match crate::charset::encode_str(s) {
                            Ok(b) => { out.extend_from_slice(&b); log.push_str(&format!("  ; str {:?} -> {} bytes\n", s, b.len())); }
                            Err(e) => return Err(e),
                        }
                    } else {
                        let v = eval_expr(a, db)?;
                        emit_le(v, 2, out);
                    }
                }
            } else {
                // unknown -> try as raw gadget name as-is
                if db.gadgets.map.contains_key(&key) || db.labels.labels.contains_key(&key) {
                    emit_call(&key, db, out, log)?;
                } else {
                    log.push_str(&format!("  ; Cảnh báo [W001]: hàm {name} không tìm thấy trong gadgets/labels (kiểm tra model 580vnx/880btg) -> bỏ qua\n"));
                    // still emit args as bytes to keep shape
                    for a in args {
                        let v = eval_expr(a, db).unwrap_or(0);
                        emit_le(v, 2, out);
                    }
                }
            }
            Ok(())
        }
        Stmt::If { cond, then_b, else_b } => {
            let c = eval_cond(cond, db, out, log)?;
            if c == 0 {
                for s in else_b { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
            } else if c == 1 {
                for s in then_b { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
            } else {
                let else_lbl = format!("else_{}", out.len());
                let end_lbl = format!("end_{}", out.len());
                // then
                for s in then_b { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
                // goto end
                patches.push((out.len(), end_lbl.clone()));
                out.extend_from_slice(&[0, 0]); // placeholder for adr_of end
                log.push_str(&format!("  ; goto {end_lbl}\n"));
                let _ = emit_call("sp=er6,pop er8", db, out, log);
                // else label
                labels.insert(else_lbl.clone(), out.len());
                log.push_str(&format!("{else_lbl}:\n"));
                for s in else_b { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
                labels.insert(end_lbl.clone(), out.len());
                log.push_str(&format!("{end_lbl}:\n"));
            }
            Ok(())
        }
        Stmt::While { cond, body } => {
            let loop_lbl = format!("loop_{}", out.len());
            let end_lbl = format!("end_{}", out.len());
            labels.insert(loop_lbl.clone(), out.len());
            log.push_str(&format!("{loop_lbl}:\n"));
            let c = eval_cond(cond, db, out, log)?;
            if c == 0 {
                log.push_str("  ; while false -> skip\n");
                labels.insert(end_lbl.clone(), out.len());
            } else if c == 1 {
                for s in body { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
                patches.push((out.len(), loop_lbl.clone()));
                out.extend_from_slice(&[0, 0]);
                let _ = emit_call("sp=er6,pop er8", db, out, log);
                log.push_str(&format!("  ; goto {loop_lbl}\n"));
                labels.insert(end_lbl.clone(), out.len());
                log.push_str(&format!("{end_lbl}:\n"));
            } else {
                // dynamic: need cmp then conditional goto (placeholder)
                for s in body { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
                patches.push((out.len(), loop_lbl.clone()));
                out.extend_from_slice(&[0, 0]);
                let _ = emit_call("sp=er6,pop er8", db, out, log);
                labels.insert(end_lbl.clone(), out.len());
            }
            Ok(())
        }
        Stmt::For { init, cond, step, body } => {
            if let Some(s) = init { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
            let loop_lbl = format!("for_{}", out.len());
            let end_lbl = format!("for_end_{}", out.len());
            labels.insert(loop_lbl.clone(), out.len());
            let c = if let Some(e) = cond { eval_cond(e, db, out, log)? } else { 1 };
            let do_body = c != 0;
            if do_body {
                for s in body { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
                if let Some(s) = step { emit_stmt(s, db, out, log, alloc, labels, patches)?; }
                patches.push((out.len(), loop_lbl.clone()));
                out.extend_from_slice(&[0, 0]);
                let _ = emit_call("sp=er6,pop er8", db, out, log);
            }
            labels.insert(end_lbl.clone(), out.len());
            Ok(())
        }
        Stmt::Goto(lbl) => {
            log.push_str(&format!("  ; goto {lbl}\n"));
            patches.push((out.len(), lbl.clone()));
            out.extend_from_slice(&[0, 0]); // placeholder for adr_of
            let _ = emit_call("sp=er6,pop er8", db, out, log);
            Ok(())
        }
        Stmt::Label(lbl) => {
            labels.insert(lbl.clone(), out.len());
            log.push_str(&format!("{lbl}:\n"));
            Ok(())
        }
        Stmt::Asm(lines) => {
            for l in lines {
                log.push_str(&format!("  ; asm {}\n", l));
                let key = l.to_ascii_lowercase();
                if db.gadgets.map.contains_key(&key) || db.labels.labels.contains_key(&key) {
                    emit_call(&key, db, out, log)?;
                } else if key.starts_with("0x") || key.starts_with("hex ") {
                    // raw hex: emit bytes directly
                    let hexpart = key.trim_start_matches("hex").trim().trim_start_matches("0x");
                    if let Ok(b) = (0..hexpart.len()).step_by(2).map(|i| u8::from_str_radix(&hexpart[i..(i+2).min(hexpart.len())], 16)).collect::<Result<Vec<u8>, _>>() {
                        out.extend_from_slice(&b);
                    }
                }
            }
            Ok(())
        }
        Stmt::Return(_) => {
            log.push_str("  ; return\n");
            let _ = emit_call("rt", db, out, log);
            Ok(())
        }
    }
}

fn is_register(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    matches!(s.as_str(), "r0"|"r1"|"r2"|"r3"|"r4"|"r5"|"r6"|"r7"|"r8"|"r9"|"r12"
        | "er0"|"er2"|"er4"|"er6"|"er8"|"er10"|"er12"|"er14"
        | "xr0"|"xr4"|"xr8"|"xr12"|"qr0"|"qr8"|"ea"|"sp")
}

fn reg_size(reg: &str) -> u32 {
    let r = reg.to_ascii_lowercase();
    if r.starts_with('q') { 8 } else if r.starts_with('x') { 4 } else if r.starts_with('e') { 2 } else { 1 }
}

fn find_global_var(name: &str, _db: &ModelDb) -> Option<u32> {
    // placeholder: globals are tracked in Program, not in db
    // For now return None; real lookup needs Program globals table
    let _ = name;
    None
}

fn eval_expr(e: &Expr, db: &ModelDb) -> Result<u32, String> {
    match e {
        Expr::Number(v) => Ok(*v),
        Expr::Addr(v) => Ok(*v),
        Expr::Var(name) => {
            // try data label
            if let Some(a) = db.labels.data.get(&name.to_ascii_lowercase()) { Ok(*a) }
            else if let Some((a, _)) = db.gadgets.map.get(&name.to_ascii_lowercase()) { Ok(*a) }
            else { Ok(0) } // local var -> 0 placeholder
        }
        Expr::Str(s) => {
            // encode via char_to_hex? for now length
            Ok(s.len() as u32)
        }
        Expr::BinOp(a, op, b) => {
            let av = eval_expr(a, db)?;
            let bv = eval_expr(b, db)?;
            Ok(match op.as_str() {
                "+" => av.wrapping_add(bv),
                "-" => av.wrapping_sub(bv),
                "*" => av.wrapping_mul(bv),
                "/" => if bv == 0 { 0 } else { av / bv },
                "%" => if bv == 0 { 0 } else { av % bv },
                "&" => av & bv,
                "|" => av | bv,
                "^" => av ^ bv,
                "<<" => av << (bv & 31),
                ">>" => av >> (bv & 31),
                "==" => if av == bv { 1 } else { 0 },
                "!=" => if av != bv { 1 } else { 0 },
                "<" => if av < bv { 1 } else { 0 },
                ">" => if av > bv { 1 } else { 0 },
                "<=" => if av <= bv { 1 } else { 0 },
                ">=" => if av >= bv { 1 } else { 0 },
                "&&" => if av != 0 && bv != 0 { 1 } else { 0 },
                "||" => if av != 0 || bv != 0 { 1 } else { 0 },
                _ => 0,
            })
        }
        Expr::Call(_, args) => {
            if !args.is_empty() { eval_expr(&args[0], db) } else { Ok(0) }
        }
        Expr::Unary(op, a) => {
            let av = eval_expr(a, db)?;
            Ok(match op.as_str() { "!" => if av == 0 { 1 } else { 0 }, _ => av })
        }
    }
}

fn eval_cond(e: &Expr, db: &ModelDb, _out: &mut Vec<u8>, _log: &mut String) -> Result<i32, String> {
    // try constant fold; return 0/1 if known, else 2 = dynamic
    let v = eval_expr(e, db)?;
    // If expr contains only numbers/addrs that are known, it's constant
    // For variables, eval returns 0 placeholder -> treat as dynamic
    match e {
        Expr::Number(_) | Expr::Addr(_) => Ok(if v != 0 { 1 } else { 0 }),
        Expr::BinOp(a, op, b) if matches!(op.as_str(), "=="|"!="|"<"|">"|"<="|">=") => {
            // constant if both sides are numbers
            let is_const = matches!(**a, Expr::Number(_)|Expr::Addr(_)) && matches!(**b, Expr::Number(_)|Expr::Addr(_));
            if is_const { Ok(if v != 0 { 1 } else { 0 }) } else { Ok(2) }
        }
        _ => Ok(2),
    }
}

fn emit_le(mut v: u32, sz: usize, out: &mut Vec<u8>) {
    for _ in 0..sz {
        out.push((v & 0xFF) as u8);
        v >>= 8;
    }
}

fn stdlib_candidates(name: &str) -> Vec<String> {
    let k = name.to_ascii_lowercase();
    let mut v = Vec::new();
    match k.as_str() {
        "screen_del" | "screen_clear" => { v.push("buffer_clear".into()); v.push("buffer_clear.ca54".into()); v.push("buffer_clear.d654".into()); v.push("memzero".into()); }
        "screen_fill" => v.push("fill_screen".into()),
        "draw_line" => v.push("line_draw".into()),
        "draw_pixel" => v.push("pixel_draw".into()),
        "draw_byte" => v.push("draw_byte".into()),
        "print" | "print_line" => { v.push("line_print".into()); v.push("printline".into()); v.push("smallprint".into()); v.push("basen_base_print".into()); }
        "print_hex" => v.push("hex_byte".into()),
        "render" => { v.push("render".into()); v.push("render.ddd4".into()); v.push("render.ca54".into()); v.push("render_bitmap".into()); }
        "delay" => v.push("delay".into()),
        "get_key" => { v.push("getkey".into()); v.push("getkeycode".into()); v.push("getscancode".into()); v.push("cvt_key".into()); }
        "mem_copy" => { v.push("memcpy".into()); v.push("smart_strcpy".into()); v.push("memmove".into()); }
        "mem_set" => { v.push("memset".into()); v.push("memzero".into()); }
        "mem_move" => v.push("memmove".into()),
        "str_copy" => { v.push("strcpy".into()); v.push("smart_strcpy".into()); }
        "str_cat" => { v.push("strcat".into()); v.push("smart_strcat".into()); }
        "str_len" => { v.push("strlen".into()); v.push("smart_strlen_n".into()); v.push("byte_strlen_n".into()); }
        "fill_screen" => v.push("fill_screen".into()),
        "buffer_clear" => { v.push("buffer_clear".into()); v.push("buffer_clear.ca54".into()); }
        _ => {}
    }
    v.push(k);
    v
}


fn emit_call(name: &str, db: &ModelDb, out: &mut Vec<u8>, log: &mut String) -> Result<(), String> {
    let key = name.to_ascii_lowercase();
    let mut addr = 0u32;
    let mut found_name = String::new();
    for cand in stdlib_candidates(&key) {
        if let Some((a, _)) = db.gadgets.map.get(&cand) { addr = *a; found_name = cand; break; }
        if let Some(a) = db.labels.labels.get(&cand) { addr = *a; found_name = cand; break; }
        if let Some(a) = db.labels.data.get(&cand) { addr = *a; found_name = cand; break; }
    }
    if addr == 0 {
        log.push_str(&format!("  ; Lỗi [E100]: gadget/label {name} không tồn tại (đã thử alias) -> bỏ qua\n"));
        return Ok(());
    }
    // hdcompiler: call <addr>  ->  optimize_adr_for_npress + 0x30300000, emit 4 bytes LE
    let raw = addr.wrapping_add(0x30300000);
    emit_le(raw, 4, out);
    if found_name != key {
        log.push_str(&format!("  ; call {name} -> {found_name} @ {:05X}\n", addr));
    } else {
        log.push_str(&format!("  ; call {name} @ {:05X}\n", addr));
    }
    Ok(())
}
