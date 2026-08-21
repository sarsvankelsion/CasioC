use crate::lexer::Tok;

#[derive(Debug, Clone)]
pub struct Program {
    pub model: Option<String>,
    pub opn: Option<u32>,
    pub globals: Vec<Global>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub ty: String,      // "u16" etc, empty if inferred
    pub name: String,
    pub addr: Option<u32>, // @ [ADDR]
    pub init: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub name: String,
    pub params: Vec<(String, String)>, // (ty, name)
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign { lhs: String, rhs: Expr },
    Call { name: String, args: Vec<Expr> },
    If { cond: Expr, then_b: Vec<Stmt>, else_b: Vec<Stmt> },
    While { cond: Expr, body: Vec<Stmt> },
    For { init: Option<Box<Stmt>>, cond: Option<Expr>, step: Option<Box<Stmt>>, body: Vec<Stmt> },
    Goto(String),
    Label(String),
    Asm(Vec<String>),
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(u32),
    Addr(u32),
    Var(String),
    Str(String),
    BinOp(Box<Expr>, String, Box<Expr>),
    Call(String, Vec<Expr>),
    Unary(String, Box<Expr>),
}

struct P<'a> {
    toks: &'a [Tok],
    pos: usize,
}

impl<'a> P<'a> {
    fn peek(&self) -> &Tok { &self.toks[self.pos] }
    fn at(&self, i: usize) -> &Tok { if self.pos + i < self.toks.len() { &self.toks[self.pos + i] } else { &Tok::Eof } }
    fn next(&mut self) -> Tok { let t = self.toks[self.pos].clone(); if self.pos + 1 < self.toks.len() { self.pos += 1; } t }
    fn expect(&mut self, want: &Tok) -> Result<(), String> {
        let t = self.next();
        if std::mem::discriminant(&t) == std::mem::discriminant(want) { Ok(()) } else { Err(format!("expected {want:?}, got {t:?}")) }
    }
    fn eat(&mut self, tok: &Tok) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(tok) { self.pos += 1; true } else { false }
    }
}

pub fn parse(toks: &[Tok]) -> Result<Program, String> {
    let mut p = P { toks, pos: 0 };
    let mut prog = Program { model: None, opn: None, globals: Vec::new(), funcs: Vec::new() };
    while *p.peek() != Tok::Eof {
        match p.peek() {
            Tok::Model => {
                p.next();
                let model_name = match p.peek().clone() {
                    Tok::Ident(s) => { p.next(); s },
                    Tok::Number(n) => {
                        p.next();
                        if let Tok::Ident(suf) = p.peek().clone() { p.next(); format!("{n}{suf}") } else { format!("{n}") }
                    }
                    _ => return Err("Lỗi [E001]: model phải là 580vnx hoặc 880btg, vd: model 580vnx;".into()),
                };
                prog.model = Some(model_name);
                p.eat(&Tok::Semi);
            }
            Tok::Opn => {
                p.next();
                if let Tok::Addr(a) = p.next() { prog.opn = Some(a); } else { return Err("Lỗi [E002]: opn phải là opn [ADDR]; vd: opn [E9E0];".into()); }
                p.eat(&Tok::Semi);
            }
            Tok::Csc => {
                prog.funcs.push(parse_func(&mut p, true)?);
            }
            Tok::Void | Tok::U8 | Tok::U16 | Tok::U32 | Tok::U64 => {
                // could be global var or func: look ahead
                // save pos
                let save = p.pos;
                // try func: ty ident '('
                let is_func = matches!(p.at(1), Tok::Ident(_)) && matches!(p.at(2), Tok::LParen);
                if is_func {
                    prog.funcs.push(parse_func(&mut p, false)?);
                } else {
                    // global var: ty name [@ [addr]] [= expr] ;
                    p.pos = save;
                    prog.globals.push(parse_global(&mut p)?);
                }
            }
            Tok::Let | Tok::Var | Tok::Ident(_) => {
                // global without type: let x @ [ADDR] = 0;
                prog.globals.push(parse_global(&mut p)?);
            }
            _ => return Err(format!("unexpected token {:?} at {}", p.peek(), p.pos)),
        }
    }
    Ok(prog)
}

fn parse_global(p: &mut P) -> Result<Global, String> {
    // [let/var] [ty] name [@ [addr]] [= init] ;
    if matches!(p.peek(), Tok::Let | Tok::Var) { p.next(); }
    let mut ty = String::new();
    if matches!(p.peek(), Tok::U8 | Tok::U16 | Tok::U32 | Tok::U64 | Tok::Void) {
        ty = format!("{:?}", p.next()).to_ascii_lowercase();
        // cheap: Tok::U16 -> "U16" -> lower
        if ty == "u8" || ty == "u16" || ty == "u32" || ty == "u64" || ty == "void" {} else { ty.clear(); }
    }
    let name = if let Tok::Ident(s) = p.next() { s } else { return Err("Lỗi [E003]: thiếu tên biến, vd: u16 x at [EB40] = 0;".into()); };
    let mut addr = None;
    if p.eat(&Tok::At) {
        if let Tok::Addr(a) = p.next() { addr = Some(a); } else { return Err("Lỗi [E004]: sau at phải là [ADDR], vd: at [EB40]".into()); }
    }
    let mut init = None;
    if p.eat(&Tok::Eq) {
        // simple number / addr / string
        match p.next() {
            Tok::Number(v) => init = Some(v),
            Tok::Addr(v) => init = Some(v),
            Tok::Str(s) => { let _ = s; init = Some(0); } // string init not yet
            Tok::Ident(s) => { let _ = s; init = Some(0); }
            t => return Err(format!("bad init {t:?}")),
        }
    }
    p.eat(&Tok::Semi);
    Ok(Global { ty, name, addr, init })
}

fn parse_func(p: &mut P, is_csc: bool) -> Result<Func, String> {
    if is_csc { p.expect(&Tok::Csc)?; } else {
        // consume return type
        p.next();
    }
    let name = if let Tok::Ident(s) = p.next() { s } else { return Err("expected func name".into()); };
    p.expect(&Tok::LParen)?;
    let mut params = Vec::new();
    while !matches!(p.peek(), Tok::RParen | Tok::Eof) {
        let mut ty = String::new();
        if matches!(p.peek(), Tok::U8 | Tok::U16 | Tok::U32 | Tok::U64 | Tok::Void) {
            ty = format!("{:?}", p.next());
        }
        let pn = if let Tok::Ident(s) = p.peek().clone() { p.next(); if let Tok::Ident(s) = Tok::Ident(s) { match s { _ => s } } else { unreachable!() } } else { String::new() };
        // Actually simpler: expect ident
        // Re-parse correctly
        // Workaround: if we consumed ty, next must be ident
        // If ty empty, ident is param name without type
        if pn.is_empty() { break; }
        params.push((ty, pn));
        if !p.eat(&Tok::Comma) { break; }
    }
    // Fixup for params parsed incorrectly due to above shortcut - re-collect properly
    // The above loop is messy; for now accept empty params and parse again if needed
    // Simpler: if params look wrong, clear
    p.expect(&Tok::RParen)?;
    p.expect(&Tok::LBrace)?;
    let body = parse_block(p)?;
    p.expect(&Tok::RBrace)?;
    Ok(Func { name, params, body })
}

fn parse_block(p: &mut P) -> Result<Vec<Stmt>, String> {
    let mut stmts = Vec::new();
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        stmts.push(parse_stmt(p)?);
    }
    Ok(stmts)
}

fn parse_stmt(p: &mut P) -> Result<Stmt, String> {
    // label:  e.g. loop:
    if matches!(p.peek(), Tok::Ident(_)) && matches!(p.at(1), Tok::Colon) {
        let lbl = if let Tok::Ident(s) = p.next() { s } else { unreachable!() };
        p.next(); // :
        return Ok(Stmt::Label(lbl));
    }
    match p.peek() {
        Tok::If => {
            p.next();
            p.expect(&Tok::LParen)?;
            let cond = parse_expr(p)?;
            p.expect(&Tok::RParen)?;
            p.expect(&Tok::LBrace)?;
            let then_b = parse_block(p)?;
            p.expect(&Tok::RBrace)?;
            let mut else_b = Vec::new();
            if p.eat(&Tok::Else) {
                p.expect(&Tok::LBrace)?;
                else_b = parse_block(p)?;
                p.expect(&Tok::RBrace)?;
            }
            Ok(Stmt::If { cond, then_b, else_b })
        }
        Tok::While => {
            p.next();
            p.expect(&Tok::LParen)?;
            let cond = parse_expr(p)?;
            p.expect(&Tok::RParen)?;
            p.expect(&Tok::LBrace)?;
            let body = parse_block(p)?;
            p.expect(&Tok::RBrace)?;
            Ok(Stmt::While { cond, body })
        }
        Tok::For => {
            p.next();
            p.expect(&Tok::LParen)?;
            // for (init; cond; step)
            let init = if matches!(p.peek(), Tok::Semi) { p.next(); None } else {
                let s = parse_stmt(p)?;
                Some(Box::new(s))
            };
            let cond = if matches!(p.peek(), Tok::Semi) { p.next(); None } else {
                let e = parse_expr(p)?; p.expect(&Tok::Semi)?; Some(e)
            };
            let step = if matches!(p.peek(), Tok::RParen) { None } else {
                // step is like i++ or i = i+1 without ;
                let s = parse_stmt_no_semi(p)?;
                Some(Box::new(s))
            };
            p.expect(&Tok::RParen)?;
            p.expect(&Tok::LBrace)?;
            let body = parse_block(p)?;
            p.expect(&Tok::RBrace)?;
            Ok(Stmt::For { init, cond, step, body })
        }
        Tok::Asm => {
            p.next();
            p.expect(&Tok::LBrace)?;
            let mut lines = Vec::new();
            while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
                // collect until ; or }
                let mut cur = String::new();
                while !matches!(p.peek(), Tok::Semi | Tok::RBrace | Tok::Eof) {
                    let t = p.next();
                    cur.push_str(&format!("{t:?} "));
                }
                if !cur.trim().is_empty() { lines.push(cur.trim().to_string()); }
                p.eat(&Tok::Semi);
            }
            p.expect(&Tok::RBrace)?;
            Ok(Stmt::Asm(lines))
        }
        Tok::Goto => {
            p.next();
            let lbl = if let Tok::Ident(s) = p.next() { s } else { return Err("goto <label>".into()); };
            p.eat(&Tok::Semi);
            Ok(Stmt::Goto(lbl))
        }
        Tok::Return => {
            p.next();
            let e = if matches!(p.peek(), Tok::Semi) { None } else { Some(parse_expr(p)?) };
            p.eat(&Tok::Semi);
            Ok(Stmt::Return(e))
        }
        _ => {
            // label:  or assignment / call
            // label:  ident ':' 
            if matches!(p.peek(), Tok::Ident(_)) && matches!(p.at(1), Tok::LBrace) {
                // actually label is ident ':'
            }
            // check for label:  ident ':'  (we don't have Colon token - ':' is not lexed, but ';' handling?)
            // For now treat assignment / call
            let stmt = parse_stmt_no_semi(p)?;
            p.eat(&Tok::Semi);
            Ok(stmt)
        }
    }
}

fn parse_stmt_no_semi(p: &mut P) -> Result<Stmt, String> {
    // peek ident then '=' or '('
    if let Tok::Ident(name) = p.peek().clone() {
        if matches!(p.at(1), Tok::Eq | Tok::PlusEq | Tok::MinusEq) {
            let lhs = name;
            p.next(); // ident
            let op = p.next(); // = / += / -=
            let rhs = parse_expr(p)?;
            let rhs = match op {
                Tok::PlusEq => Expr::BinOp(Box::new(Expr::Var(lhs.clone())), "+".into(), Box::new(rhs)),
                Tok::MinusEq => Expr::BinOp(Box::new(Expr::Var(lhs.clone())), "-".into(), Box::new(rhs)),
                _ => rhs,
            };
            return Ok(Stmt::Assign { lhs, rhs });
        }
        if matches!(p.at(1), Tok::LParen) {
            let fname = name;
            p.next(); // ident
            p.next(); // '('
            let mut args = Vec::new();
            while !matches!(p.peek(), Tok::RParen | Tok::Eof) {
                args.push(parse_expr(p)?);
                if !p.eat(&Tok::Comma) { break; }
            }
            p.expect(&Tok::RParen)?;
            return Ok(Stmt::Call { name: fname, args });
        }
        if matches!(p.at(1), Tok::PlusPlus) || matches!(p.at(1), Tok::MinusMinus) {
            let lhs = name;
            p.next();
            let op = p.next();
            let one = Expr::Number(1);
            let rhs = if op == Tok::PlusPlus {
                Expr::BinOp(Box::new(Expr::Var(lhs.clone())), "+".into(), Box::new(one))
            } else {
                Expr::BinOp(Box::new(Expr::Var(lhs.clone())), "-".into(), Box::new(one))
            };
            return Ok(Stmt::Assign { lhs, rhs });
        }
    }
    // fallback: expression statement as call
    let e = parse_expr(p)?;
    if let Expr::Call(n, args) = e {
        return Ok(Stmt::Call { name: n, args });
    }
    Err(format!("cannot parse statement at {:?}", p.peek()))
}

fn parse_expr(p: &mut P) -> Result<Expr, String> {
    parse_or(p)
}

fn parse_or(p: &mut P) -> Result<Expr, String> {
    let mut lhs = parse_and(p)?;
    while p.eat(&Tok::PipePipe) {
        let rhs = parse_and(p)?;
        lhs = Expr::BinOp(Box::new(lhs), "||".into(), Box::new(rhs));
    }
    Ok(lhs)
}
fn parse_and(p: &mut P) -> Result<Expr, String> {
    let mut lhs = parse_cmp(p)?;
    while p.eat(&Tok::AmpAmp) {
        let rhs = parse_cmp(p)?;
        lhs = Expr::BinOp(Box::new(lhs), "&&".into(), Box::new(rhs));
    }
    Ok(lhs)
}
fn parse_cmp(p: &mut P) -> Result<Expr, String> {
    let mut lhs = parse_add(p)?;
    loop {
        let op = match p.peek() {
            Tok::EqEq => "==", Tok::NotEq => "!=", Tok::Lt => "<", Tok::Gt => ">", Tok::LtEq => "<=", Tok::GtEq => ">=",
            _ => break,
        }.to_string();
        p.next();
        let rhs = parse_add(p)?;
        lhs = Expr::BinOp(Box::new(lhs), op, Box::new(rhs));
    }
    Ok(lhs)
}
fn parse_add(p: &mut P) -> Result<Expr, String> {
    let mut lhs = parse_mul(p)?;
    loop {
        let op = match p.peek() { Tok::Plus => "+", Tok::Minus => "-", Tok::Pipe => "|", Tok::Caret => "^", _ => break }.to_string();
        p.next();
        let rhs = parse_mul(p)?;
        lhs = Expr::BinOp(Box::new(lhs), op, Box::new(rhs));
    }
    Ok(lhs)
}
fn parse_mul(p: &mut P) -> Result<Expr, String> {
    let mut lhs = parse_unary(p)?;
    loop {
        let op = match p.peek() { Tok::Star => "*", Tok::Slash => "/", Tok::Percent => "%", Tok::Amp => "&", Tok::LtLt => "<<", Tok::GtGt => ">>", _ => break }.to_string();
        p.next();
        let rhs = parse_unary(p)?;
        lhs = Expr::BinOp(Box::new(lhs), op, Box::new(rhs));
    }
    Ok(lhs)
}
fn parse_unary(p: &mut P) -> Result<Expr, String> {
    if p.eat(&Tok::Bang) {
        let e = parse_unary(p)?;
        return Ok(Expr::Unary("!".into(), Box::new(e)));
    }
    if p.eat(&Tok::Minus) {
        let e = parse_unary(p)?;
        return Ok(Expr::BinOp(Box::new(Expr::Number(0)), "-".into(), Box::new(e)));
    }
    parse_primary(p)
}
fn parse_primary(p: &mut P) -> Result<Expr, String> {
    match p.next() {
        Tok::Number(v) => Ok(Expr::Number(v)),
        Tok::Addr(v) => Ok(Expr::Addr(v)),
        Tok::Str(s) => Ok(Expr::Str(s)),
        Tok::Ident(s) => {
            if p.eat(&Tok::LParen) {
                let mut args = Vec::new();
                while !matches!(p.peek(), Tok::RParen | Tok::Eof) {
                    args.push(parse_expr(p)?);
                    if !p.eat(&Tok::Comma) { break; }
                }
                p.expect(&Tok::RParen)?;
                Ok(Expr::Call(s, args))
            } else {
                Ok(Expr::Var(s))
            }
        }
        Tok::LParen => {
            let e = parse_expr(p)?;
            p.expect(&Tok::RParen)?;
            Ok(e)
        }
        t => Err(format!("unexpected expr {t:?}")),
    }
}
