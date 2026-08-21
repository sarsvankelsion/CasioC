use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    Fx580vnx,
    Fx880btg,
}

impl Model {
    pub const ALL: [Model; 2] = [Model::Fx580vnx, Model::Fx880btg];

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "580vnx" | "580" | "fx580vnx" | "fx580vnx " => Some(Self::Fx580vnx),
            "880btg" | "880" | "fx880btg" => Some(Self::Fx880btg),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Fx580vnx => "580vnx",
            Self::Fx880btg => "880btg",
        }
    }

    pub fn pretty(self) -> &'static str {
        match self {
            Self::Fx580vnx => "fx-580VN X",
            Self::Fx880btg => "fx-880BTG",
        }
    }

    pub fn default_dir(self, hd_root: &Path) -> PathBuf {
        hd_root.join(self.id())
    }
}

#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub model: Model,
    pub dir: PathBuf,
    pub rom: PathBuf,
    pub gadgets: PathBuf,
    pub labels: PathBuf,
    pub labels_sfr: PathBuf,
    pub disas: PathBuf,
    pub ropchain: PathBuf,
}

impl ModelPaths {
    pub const DEFAULT_HD_ROOT: &'static str = r"D:\casioai\hdcompiler_vn";

    pub fn resolve(model: Model, hd_root: Option<&Path>, model_dir: Option<&Path>) -> Self {
        let hd_root = hd_root
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(Self::DEFAULT_HD_ROOT));
        let dir = model_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| model.default_dir(&hd_root));
        Self {
            model,
            rom: dir.join("rom.bin"),
            gadgets: dir.join("gadgets"),
            labels: dir.join("labels"),
            labels_sfr: hd_root.join("labels_sfr"),
            disas: dir.join("disas.txt"),
            ropchain: hd_root.join(format!("{}_ropchain", model.id())),
            dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Gadget {
    pub addr: u32,
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Default)]
pub struct GadgetTable {
    /// name (canonical, lowercase) -> (addr, tags)
    pub map: HashMap<String, (u32, Vec<String>)>,
}

#[derive(Debug, Default)]
pub struct LabelTable {
    /// func / data labels: name -> addr
    pub labels: HashMap<String, u32>,
    /// raw data labels: d_... -> addr
    pub data: HashMap<String, u32>,
}

#[derive(Debug)]
pub struct ModelDb {
    pub model: Model,
    pub paths: ModelPaths,
    pub gadgets: GadgetTable,
    pub labels: LabelTable,
    pub rom: Vec<u8>,
}

impl ModelDb {
    pub fn load(model: Model, hd_root: Option<&Path>, model_dir: Option<&Path>) -> Result<Self, String> {
        let paths = ModelPaths::resolve(model, hd_root, model_dir);
        let gadgets = load_gadgets(&paths.gadgets)?;
        let labels = load_labels(&paths.labels, &paths.labels_sfr)?;
        let rom = std::fs::read(&paths.rom).map_err(|e| format!("rom {}: {e}", paths.rom.display()))?;
        Ok(Self { model, paths, gadgets, labels, rom })
    }

    /// All known callable names (gadgets + labels), for stdlib generation.
    pub fn all_callables(&self) -> Vec<(String, u32)> {
        let mut v: Vec<(String, u32)> = Vec::new();
        for (k, (addr, _)) in &self.gadgets.map {
            v.push((k.clone(), *addr));
        }
        for (k, addr) in &self.labels.labels {
            v.push((k.clone(), *addr));
        }
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }
}

fn del_inline_comment(line: &str) -> &str {
    if let Some(p) = line.find('#') {
        line[..p].trim_end()
    } else {
        line
    }
}

fn canonicalize(s: &str) -> String {
    let s = s.trim();
    let mut out = String::with_capacity(s.len());
    let mut prev_is_alnum = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' || c == '\t' {
            // look ahead to next non-space
            let mut j = i + 1;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                j += 1;
            }
            let next = if j < chars.len() { Some(chars[j]) } else { None };
            let cur_is_alnum = prev_is_alnum;
            let next_is_alnum = next.map(|ch| ch.is_ascii_alphanumeric() || ch == '_').unwrap_or(false);
            if cur_is_alnum && next_is_alnum {
                out.push(' ');
            }
            // else: remove spaces around non-alnum (re.sub(r' *([^a-z0-9]) *', r'\1', st))
            // The python regex operates on lowercased string where [^a-z0-9] matches symbols.
            // Emulate by simply not emitting spaces around symbols.
            i = j;
            continue;
        } else {
            out.push(c);
            prev_is_alnum = c.is_ascii_alphanumeric() || c == '_';
            i += 1;
        }
    }
    out
}

pub fn load_gadgets(path: &Path) -> Result<GadgetTable, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut map: HashMap<String, (u32, Vec<String>)> = HashMap::new();
    let mut in_comment = false;
    for (idx, raw) in text.lines().enumerate() {
        let mut line = raw.trim().to_string();
        if line == "/*" {
            in_comment = true;
            continue;
        }
        if line == "*/" {
            in_comment = false;
            continue;
        }
        if in_comment {
            continue;
        }
        line = del_inline_comment(&line).trim().to_string();
        if line.is_empty() {
            continue;
        }
        // split addr + command
        let mut sp = line.splitn(2, |c: char| c.is_whitespace());
        let addr_s = sp.next().unwrap().trim();
        let rest = sp.next().unwrap_or("").trim();
        if rest.is_empty() {
            continue;
        }
        let addr = u32::from_str_radix(addr_s, 16)
            .map_err(|_| format!("{}:{} invalid addr {addr_s:?}", path.display(), idx + 1))?;
        let mut cmd = canonicalize(rest);
        cmd = cmd.to_ascii_lowercase();
        let mut tags = Vec::new();
        while cmd.starts_with('{') {
            if let Some(end) = cmd.find('}') {
                tags.push(cmd[1..end].to_string());
                cmd = cmd[end + 1..].to_string();
            } else {
                return Err(format!("{}:{} unmatched {{", path.display(), idx + 1));
            }
        }
        // last wins (matches Python dict assignment)
        map.insert(cmd, (addr, tags));
    }
    Ok(GadgetTable { map })
}

fn load_labels(paths0: &Path, paths1: &Path) -> Result<LabelTable, String> {
    // Simplified: read both files, parse `hex  name` lines.
    // Full rename logic (f_/.l_ relocation via disas) is deferred to a later pass
    // that needs disas.txt. For now we collect raw hex -> name.
    let mut tbl = LabelTable::default();
    for path in [paths0, paths1] {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // format: "<addr>  <name>"  addr may be "d_0F000", "09fa8", "f_09fa8", "f_09fa8.l_10"
            let mut parts = line.split_whitespace();
            let a = parts.next();
            let b = parts.next();
            if a.is_none() || b.is_none() {
                continue;
            }
            let raw_addr = a.unwrap();
            let name = b.unwrap().to_string();
            if name.starts_with('.') {
                continue;
            }
            // raw_addr like "d_0F000" -> data label
            if raw_addr.starts_with("d_") {
                if let Ok(v) = u32::from_str_radix(&raw_addr[2..], 16) {
                    tbl.data.insert(name, v);
                }
                continue;
            }
            // hex addr
            if raw_addr.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(v) = u32::from_str_radix(raw_addr, 16) {
                    tbl.labels.insert(name.to_ascii_lowercase(), v);
                }
                continue;
            }
            // f_... / f_....l_... -> try to extract base hex
            // Keep simple: take substring after "f_" up to '.' or end
            if raw_addr.starts_with("f_") {
                let rest = &raw_addr[2..];
                let hex_part = rest.split('.').next().unwrap_or("");
                // hex may contain only hex digits before '.'
                let hex_clean: String = hex_part.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                if let Ok(v) = u32::from_str_radix(&hex_clean, 16) {
                    // local part .l_XXX
                    let mut addr = v;
                    if let Some(dot) = raw_addr.find(".l_") {
                        let lpart = &raw_addr[dot + 3..];
                        if let Ok(off) = u32::from_str_radix(lpart, 16) {
                            addr += off;
                        }
                    }
                    tbl.labels.insert(name.to_ascii_lowercase(), addr);
                }
            }
        }
    }
    Ok(tbl)
}
