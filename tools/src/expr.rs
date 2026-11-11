//! A small arithmetic evaluator for `check` expressions.
//!
//! Everything is computed in f64, so `7 / 2` is 3.5 (no integer division).
//! Supported: numbers, `+ - * / ^`, parentheses, unary minus, the constant
//! `pi`, and the functions sqrt, cbrt, abs, floor, ceil, round(x) or
//! round(x, places), min, max, fact(n), comb(n, k), perm(n, k), gcd(a, b)
//! and lcm(a, b).

pub fn eval(src: &str) -> Result<f64, String> {
    let tokens = lex(src)?;
    let mut p = Parser { tokens, pos: 0 };
    let v = p.expr()?;
    if p.pos != p.tokens.len() {
        return Err(format!("unexpected {:?}", p.tokens[p.pos]));
    }
    if !v.is_finite() {
        return Err("result is not a finite number".into());
    }
    Ok(v)
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push(Tok::Num(s.parse().map_err(|_| format!("bad number {s:?}"))?));
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(chars[start..i].iter().collect()));
        } else if "+-*/^(),".contains(c) {
            out.push(Tok::Op(c));
            i += 1;
        } else {
            return Err(format!("unexpected character {c:?}"));
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek_op(&self) -> Option<char> {
        match self.tokens.get(self.pos) {
            Some(Tok::Op(c)) => Some(*c),
            _ => None,
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.peek_op() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected '{c}'"))
        }
    }

    // expr := term (('+' | '-') term)*
    fn expr(&mut self) -> Result<f64, String> {
        let mut v = self.term()?;
        while let Some(op @ ('+' | '-')) = self.peek_op() {
            self.pos += 1;
            let r = self.term()?;
            v = if op == '+' { v + r } else { v - r };
        }
        Ok(v)
    }

    // term := unary (('*' | '/') unary)*
    fn term(&mut self) -> Result<f64, String> {
        let mut v = self.unary()?;
        while let Some(op @ ('*' | '/')) = self.peek_op() {
            self.pos += 1;
            let r = self.unary()?;
            if op == '/' && r == 0.0 {
                return Err("division by zero".into());
            }
            v = if op == '*' { v * r } else { v / r };
        }
        Ok(v)
    }

    // unary := '-' unary | power   (so -2^2 = -4)
    fn unary(&mut self) -> Result<f64, String> {
        if self.peek_op() == Some('-') {
            self.pos += 1;
            return Ok(-self.unary()?);
        }
        if self.peek_op() == Some('+') {
            self.pos += 1;
            return self.unary();
        }
        self.power()
    }

    // power := atom ('^' unary)?   (right-associative)
    fn power(&mut self) -> Result<f64, String> {
        let base = self.atom()?;
        if self.peek_op() == Some('^') {
            self.pos += 1;
            let exp = self.unary()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<f64, String> {
        match self.tokens.get(self.pos).cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok(n)
            }
            Some(Tok::Op('(')) => {
                self.pos += 1;
                let v = self.expr()?;
                self.expect(')')?;
                Ok(v)
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if self.peek_op() != Some('(') {
                    return match name.as_str() {
                        "pi" => Ok(std::f64::consts::PI),
                        _ => Err(format!("unknown name {name:?}")),
                    };
                }
                self.pos += 1;
                let mut args = vec![self.expr()?];
                while self.peek_op() == Some(',') {
                    self.pos += 1;
                    args.push(self.expr()?);
                }
                self.expect(')')?;
                call(&name, &args)
            }
            Some(t) => Err(format!("unexpected {t:?}")),
            None => Err("unexpected end of expression".into()),
        }
    }
}

fn call(name: &str, a: &[f64]) -> Result<f64, String> {
    let arity = |n: usize| -> Result<(), String> {
        if a.len() == n {
            Ok(())
        } else {
            Err(format!("{name} takes {n} argument(s), got {}", a.len()))
        }
    };
    match name {
        "sqrt" => {
            arity(1)?;
            if a[0] < 0.0 {
                return Err("sqrt of a negative number".into());
            }
            Ok(a[0].sqrt())
        }
        "cbrt" => arity(1).map(|_| a[0].cbrt()),
        "abs" => arity(1).map(|_| a[0].abs()),
        "floor" => arity(1).map(|_| a[0].floor()),
        "ceil" => arity(1).map(|_| a[0].ceil()),
        "round" => match a.len() {
            1 => Ok(a[0].round()),
            2 => {
                let f = 10f64.powi(whole(a[1])? as i32);
                Ok((a[0] * f).round() / f)
            }
            _ => Err("round takes 1 or 2 arguments".into()),
        },
        "gcd" => {
            arity(2)?;
            Ok(gcd(whole(a[0])?, whole(a[1])?) as f64)
        }
        "lcm" => {
            arity(2)?;
            let (x, y) = (whole(a[0])?, whole(a[1])?);
            if x == 0 || y == 0 {
                return Ok(0.0);
            }
            Ok((x / gcd(x, y) * y) as f64)
        }
        "min" => arity(2).map(|_| a[0].min(a[1])),
        "max" => arity(2).map(|_| a[0].max(a[1])),
        "fact" => {
            arity(1)?;
            Ok((1..=whole(a[0])?).map(|k| k as f64).product())
        }
        "perm" => {
            arity(2)?;
            let (n, k) = (whole(a[0])?, whole(a[1])?);
            if k > n {
                return Err("perm(n, k) needs k <= n".into());
            }
            Ok(((n - k + 1)..=n).map(|x| x as f64).product())
        }
        "comb" => {
            arity(2)?;
            let (n, k) = (whole(a[0])?, whole(a[1])?);
            if k > n {
                return Err("comb(n, k) needs k <= n".into());
            }
            let k = k.min(n - k);
            Ok((0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64).round())
        }
        _ => Err(format!("unknown function {name:?}")),
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn whole(x: f64) -> Result<u64, String> {
    if x >= 0.0 && x.fract() == 0.0 && x < 1e6 {
        Ok(x as u64)
    } else {
        Err(format!("{x} is not a non-negative whole number"))
    }
}

#[cfg(test)]
mod tests {
    use super::eval;

    #[test]
    fn arithmetic() {
        assert_eq!(eval("240 / 6 * 9").unwrap(), 360.0);
        assert_eq!(eval("7 / 2").unwrap(), 3.5);
        assert_eq!(eval("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(eval("(2 + 3) * 4").unwrap(), 20.0);
        assert_eq!(eval("-2^2").unwrap(), -4.0);
        assert_eq!(eval("2^3^2").unwrap(), 512.0);
        assert_eq!(eval("10 - 4 - 3").unwrap(), 3.0);
    }

    #[test]
    fn functions() {
        assert_eq!(eval("sqrt(3^2 + 4^2)").unwrap(), 5.0);
        assert_eq!(eval("round(pi * 2^2, 2)").unwrap(), 12.57);
        assert_eq!(eval("comb(5, 2)").unwrap(), 10.0);
        assert_eq!(eval("perm(5, 2)").unwrap(), 20.0);
        assert_eq!(eval("fact(4)").unwrap(), 24.0);
        assert_eq!(eval("gcd(36, 48)").unwrap(), 12.0);
        assert_eq!(eval("lcm(6, 8)").unwrap(), 24.0);
    }

    #[test]
    fn errors() {
        assert!(eval("1 / 0").is_err());
        assert!(eval("2 +").is_err());
        assert!(eval("foo(1)").is_err());
        assert!(eval("x + 1").is_err());
        assert!(eval("(1 + 2").is_err());
    }
}
