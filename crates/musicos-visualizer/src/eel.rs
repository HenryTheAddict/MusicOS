//! Bounded equation evaluator for the curated EEL sources. No JS, eval, loops,
//! allocation in evaluation, filesystem, network, or user-supplied programs.
use anyhow::{bail, ensure, Result};
use std::collections::BTreeMap;
#[derive(Clone, Default, Debug)]
pub struct Symbols {
    names: BTreeMap<String, usize>,
}
impl Symbols {
    pub fn slot(&mut self, name: &str) -> usize {
        let n = self.names.len();
        *self.names.entry(name.to_lowercase()).or_insert(n)
    }
    pub fn len(&self) -> usize {
        self.names.len()
    }
    pub fn find(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }
}
#[derive(Clone, Debug)]
enum Expr {
    Number(f64),
    Var(usize),
    Neg(Box<Expr>),
    Binary(u8, Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
}
#[derive(Clone, Copy, Debug)]
enum Func {
    Above,
    Below,
    Equal,
    Abs,
    Asin,
    Atan2,
    Bnot,
    Cos,
    If,
    Int,
    Log,
    Max,
    Min,
    Pow,
    Rand,
    Sigmoid,
    Sin,
    Sqr,
    Sqrt,
    Tan,
}
#[derive(Clone, Default, Debug)]
pub struct Program {
    statements: Vec<(usize, Expr)>,
}
#[derive(Clone)]
pub struct Values {
    pub slots: Vec<f64>,
    random: crate::random::Random,
}
impl Values {
    pub fn new(symbols: &Symbols, seed: u64) -> Self {
        Self::with_random(symbols, crate::random::Random::new(seed))
    }
    pub fn with_random(symbols: &Symbols, random: crate::random::Random) -> Self {
        Self {
            slots: vec![0.; symbols.len()],
            random,
        }
    }
    pub fn set(&mut self, symbols: &Symbols, name: &str, value: f64) {
        if let Some(i) = symbols.find(name) {
            self.slots[i] = value;
        }
    }
    pub fn get(&self, symbols: &Symbols, name: &str) -> f64 {
        symbols.find(name).map(|i| self.slots[i]).unwrap_or(0.)
    }
    fn random(&mut self) -> f64 {
        self.random.next()
    }
}
impl Program {
    pub fn compile(source: &str, symbols: &mut Symbols) -> Result<Self> {
        let mut parser = Parser {
            data: source.as_bytes(),
            at: 0,
            symbols,
            depth: 0,
        };
        let mut statements = Vec::new();
        parser.space();
        while parser.at < parser.data.len() {
            let name = parser.name()?;
            parser.space();
            parser.take(b'=')?;
            let expr = parser.expr(0)?;
            parser.space();
            parser.take(b';')?;
            statements.push((parser.symbols.slot(&name), expr));
            ensure!(statements.len() <= 4096, "Preset equation limit exceeded");
            parser.space();
        }
        ensure!(
            parser.symbols.len() <= 2048,
            "Preset variable limit exceeded"
        );
        Ok(Self { statements })
    }
    pub fn run(&self, values: &mut Values) {
        for (slot, expr) in &self.statements {
            let v = expr.eval(values);
            values.slots[*slot] = if v.is_finite() { v } else { 0. };
        }
    }
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}
impl Expr {
    fn eval(&self, v: &mut Values) -> f64 {
        match self {
            Self::Number(n) => *n,
            Self::Var(i) => v.slots[*i],
            Self::Neg(x) => -x.eval(v),
            Self::Binary(op, a, b) => {
                let a = a.eval(v);
                let b = b.eval(v);
                match op {
                    b'+' => a + b,
                    b'-' => a - b,
                    b'*' => a * b,
                    b'/' => {
                        if b == 0. {
                            0.
                        } else {
                            a / b
                        }
                    }
                    b'%' => {
                        if b == 0. {
                            0.
                        } else {
                            a.floor() % b.floor()
                        }
                    }
                    _ => 0.,
                }
            }
            Self::Call(f, args) => {
                if matches!(f, Func::If) {
                    return if args[0].eval(v).abs() > 1e-5 {
                        args[1].eval(v)
                    } else {
                        args[2].eval(v)
                    };
                }
                let a = args[0].eval(v);
                let b = if args.len() > 1 { args[1].eval(v) } else { 0. };
                let bit = |x: bool| if x { 1. } else { 0. };
                match f {
                    Func::Above => bit(a > b),
                    Func::Below => bit(a < b),
                    Func::Equal => bit((a - b).abs() < 1e-5),
                    Func::Abs => a.abs(),
                    Func::Asin => a.asin(),
                    Func::Atan2 => a.atan2(b),
                    Func::Bnot => bit(a.abs() < 1e-5),
                    Func::Cos => a.cos(),
                    Func::Int => a.trunc(),
                    Func::Log => a.ln(),
                    Func::Max => a.max(b),
                    Func::Min => a.min(b),
                    Func::Pow => {
                        let x = a.powf(b);
                        if x.is_finite() {
                            x
                        } else {
                            0.
                        }
                    }
                    Func::Rand => v.random() * a,
                    Func::Sigmoid => {
                        let x = 1. + (-a * b).exp();
                        if x.abs() > 1e-5 {
                            1. / x
                        } else {
                            0.
                        }
                    }
                    Func::Sin => a.sin(),
                    Func::Sqr => a * a,
                    Func::Sqrt => a.abs().sqrt(),
                    Func::Tan => a.tan(),
                    Func::If => unreachable!(),
                }
            }
        }
    }
}
struct Parser<'a> {
    data: &'a [u8],
    at: usize,
    symbols: &'a mut Symbols,
    depth: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        loop {
            while self.data.get(self.at).is_some_and(u8::is_ascii_whitespace) {
                self.at += 1;
            }
            if self.data.get(self.at..self.at + 2) == Some(b"//") {
                while self.data.get(self.at).is_some_and(|c| *c != b'\n') {
                    self.at += 1;
                }
            } else if self.data.get(self.at..self.at + 2) == Some(b"/*") {
                self.at += 2;
                while self.at + 1 < self.data.len()
                    && self.data.get(self.at..self.at + 2) != Some(b"*/")
                {
                    self.at += 1;
                }
                self.at = (self.at + 2).min(self.data.len());
            } else {
                break;
            }
        }
    }
    fn take(&mut self, c: u8) -> Result<()> {
        self.space();
        ensure!(
            self.data.get(self.at) == Some(&c),
            "Expected {} at {}",
            c as char,
            self.at
        );
        self.at += 1;
        Ok(())
    }
    fn name(&mut self) -> Result<String> {
        self.space();
        let start = self.at;
        while self
            .data
            .get(self.at)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            self.at += 1;
        }
        ensure!(start < self.at, "Expected identifier at {}", self.at);
        Ok(std::str::from_utf8(&self.data[start..self.at])?.to_lowercase())
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.depth += 1;
        ensure!(self.depth < 128, "Preset expression nesting limit");
        self.space();
        let mut left = match self.data.get(self.at).copied() {
            Some(b'-') => {
                self.at += 1;
                Expr::Neg(Box::new(self.expr(3)?))
            }
            Some(b'+') => {
                self.at += 1;
                self.expr(3)?
            }
            Some(b'(') => {
                self.at += 1;
                let e = self.expr(0)?;
                self.take(b')')?;
                e
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.at;
                while self
                    .data
                    .get(self.at)
                    .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
                {
                    self.at += 1;
                }
                if self
                    .data
                    .get(self.at)
                    .is_some_and(|c| *c == b'e' || *c == b'E')
                {
                    self.at += 1;
                    if self
                        .data
                        .get(self.at)
                        .is_some_and(|c| *c == b'+' || *c == b'-')
                    {
                        self.at += 1;
                    }
                    while self.data.get(self.at).is_some_and(u8::is_ascii_digit) {
                        self.at += 1;
                    }
                }
                Expr::Number(std::str::from_utf8(&self.data[start..self.at])?.parse()?)
            }
            Some(_) => {
                let name = self.name()?;
                self.space();
                if self.data.get(self.at) == Some(&b'(') {
                    self.at += 1;
                    let (f, n) = match name.as_str() {
                        "above" => (Func::Above, 2),
                        "below" => (Func::Below, 2),
                        "equal" => (Func::Equal, 2),
                        "abs" => (Func::Abs, 1),
                        "asin" => (Func::Asin, 1),
                        "atan2" => (Func::Atan2, 2),
                        "bnot" => (Func::Bnot, 1),
                        "cos" => (Func::Cos, 1),
                        "if" => (Func::If, 3),
                        "int" => (Func::Int, 1),
                        "log" => (Func::Log, 1),
                        "max" => (Func::Max, 2),
                        "min" => (Func::Min, 2),
                        "pow" => (Func::Pow, 2),
                        "rand" => (Func::Rand, 1),
                        "sigmoid" => (Func::Sigmoid, 2),
                        "sin" => (Func::Sin, 1),
                        "sqr" => (Func::Sqr, 1),
                        "sqrt" => (Func::Sqrt, 1),
                        "tan" => (Func::Tan, 1),
                        _ => bail!("Unsupported preset function {name}"),
                    };
                    let mut args = Vec::new();
                    for i in 0..n {
                        if i > 0 {
                            self.take(b',')?;
                        }
                        args.push(self.expr(0)?);
                    }
                    self.take(b')')?;
                    Expr::Call(f, args)
                } else {
                    Expr::Var(self.symbols.slot(&name))
                }
            }
            None => bail!("Unexpected end of preset equation"),
        };
        loop {
            self.space();
            let Some(op) = self.data.get(self.at).copied() else {
                break;
            };
            let precedence = match op {
                b'+' | b'-' => 1,
                b'*' | b'/' | b'%' => 2,
                _ => break,
            };
            if precedence < min {
                break;
            }
            self.at += 1;
            let right = self.expr(precedence + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        self.depth -= 1;
        Ok(left)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_bundled_equation_parses() {
        fn walk(v: &serde_json::Value, s: &mut Symbols) {
            match v {
                serde_json::Value::Object(o) => {
                    for (k, v) in o {
                        if k.ends_with("_eel") {
                            Program::compile(v.as_str().unwrap(), s)
                                .unwrap_or_else(|e| panic!("{k}: {e}"));
                        } else {
                            walk(v, s);
                        }
                    }
                }
                serde_json::Value::Array(a) => {
                    for v in a {
                        walk(v, s);
                    }
                }
                _ => {}
            }
        }
        for p in crate::catalog::catalog() {
            walk(
                &crate::catalog::source(&p.id).unwrap(),
                &mut Symbols::default(),
            );
        }
    }
    #[test]
    fn arithmetic_and_lazy_conditionals_are_safe() {
        let mut s = Symbols::default();
        let p =
            Program::compile("a=2+3*4; b=if(above(a,10),sqrt(-9),1/0); c=1/0;", &mut s).unwrap();
        let mut v = Values::new(&s, 42);
        p.run(&mut v);
        assert_eq!(v.get(&s, "a"), 14.);
        assert_eq!(v.get(&s, "b"), 3.);
        assert_eq!(v.get(&s, "c"), 0.);
    }
    #[test]
    fn version_two_random_preserves_fractional_ranges() {
        let mut symbols = Symbols::default();
        let program = Program::compile("a=rand(0.5);b=rand(3.14);", &mut symbols).unwrap();
        let mut values = Values::new(&symbols, 42);
        let mut reference = crate::random::Random::new(42);
        program.run(&mut values);
        assert_eq!(values.get(&symbols, "a"), reference.next() * 0.5);
        assert_eq!(values.get(&symbols, "b"), reference.next() * 3.14);
    }
    #[test]
    fn unsupported_code_is_rejected() {
        assert!(Program::compile("a=evil(1);", &mut Symbols::default()).is_err());
    }
}

impl Program {
    /// Reject state carried between mesh points before GPU parallelization.
    pub fn gpu_statements(
        &self,
        symbols: &Symbols,
    ) -> Result<(String, std::collections::BTreeSet<usize>)> {
        use std::collections::BTreeSet;
        let targets = self
            .statements
            .iter()
            .map(|(slot, _)| *slot)
            .collect::<BTreeSet<_>>();
        let reset = [
            "zoom", "zoomexp", "rot", "warp", "cx", "cy", "dx", "dy", "sx", "sy", "x", "y", "rad",
            "ang",
        ]
        .iter()
        .filter_map(|n| symbols.find(n))
        .collect::<BTreeSet<_>>();
        let mut written = BTreeSet::new();
        let mut all = BTreeSet::new();
        let mut output = String::new();
        for (slot, expr) in &self.statements {
            let mut reads = BTreeSet::new();
            expr.gpu_slots(&mut reads);
            for read in &reads {
                ensure!(
                    !targets.contains(read) || written.contains(read) || reset.contains(read),
                    "Pixel equation carries state between mesh points"
                );
            }
            all.extend(reads);
            all.insert(*slot);
            output += &format!("v{slot}=finite_value({});\n", expr.gpu_expression()?);
            written.insert(*slot);
        }
        Ok((output, all))
    }
}
impl Expr {
    fn gpu_slots(&self, slots: &mut std::collections::BTreeSet<usize>) {
        match self {
            Self::Var(slot) => {
                slots.insert(*slot);
            }
            Self::Neg(x) => x.gpu_slots(slots),
            Self::Binary(_, a, b) => {
                a.gpu_slots(slots);
                b.gpu_slots(slots);
            }
            Self::Call(_, args) => {
                for x in args {
                    x.gpu_slots(slots);
                }
            }
            Self::Number(_) => {}
        }
    }
    fn gpu_expression(&self) -> Result<String> {
        Ok(match self {
            Self::Number(n) => format!("{n:.12e}"),
            Self::Var(slot) => format!("v{slot}"),
            Self::Neg(x) => format!("(-({}))", x.gpu_expression()?),
            Self::Binary(op, a, b) => {
                let (a, b) = (a.gpu_expression()?, b.gpu_expression()?);
                match op {
                    b'/' => format!("divide({a},{b})"),
                    b'%' => format!("remainder({a},{b})"),
                    _ => format!("({a} {} {b})", *op as char),
                }
            }
            Self::Call(f, args) => {
                let p = args
                    .iter()
                    .map(|a| a.gpu_expression())
                    .collect::<Result<Vec<_>>>()?;
                let a = &p[0];
                let b = p.get(1).map(String::as_str).unwrap_or("0.0");
                match f {
                    Func::Above => format!("select(0.0,1.0,{a}>{b})"),
                    Func::Below => format!("select(0.0,1.0,{a}<{b})"),
                    Func::Equal => format!("select(0.0,1.0,abs({a}-({b}))<0.00001)"),
                    Func::Bnot => format!("select(0.0,1.0,abs({a})<0.00001)"),
                    Func::If => format!("select({}, {},abs({a})>0.00001)", p[2], p[1]),
                    Func::Int => format!("trunc({a})"),
                    Func::Pow => format!("power({a},{b})"),
                    Func::Sqrt => format!("sqrt(abs({a}))"),
                    Func::Sqr => format!("({a}*({a}))"),
                    Func::Sigmoid => format!("divide(1.0,1.0+exp(-({a})*({b})))"),
                    Func::Rand => bail!("Pixel random streams require ordered evaluation"),
                    Func::Abs => format!("abs({a})"),
                    Func::Asin => format!("asin({a})"),
                    Func::Atan2 => format!("atan2({a},{b})"),
                    Func::Cos => format!("cos({a})"),
                    Func::Log => format!("log({a})"),
                    Func::Max => format!("max({a},{b})"),
                    Func::Min => format!("min({a},{b})"),
                    Func::Sin => format!("sin({a})"),
                    Func::Tan => format!("tan({a})"),
                }
            }
        })
    }
}
