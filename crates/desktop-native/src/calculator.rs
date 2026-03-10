#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalculatorMode {
    Basic,
    Scientific,
    Programmer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgrammerBase {
    Dec,
    Hex,
    Oct,
    Bin,
}

impl ProgrammerBase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dec => "DEC",
            Self::Hex => "HEX",
            Self::Oct => "OCT",
            Self::Bin => "BIN",
        }
    }

    pub fn radix(self) -> u32 {
        match self {
            Self::Dec => 10,
            Self::Hex => 16,
            Self::Oct => 8,
            Self::Bin => 2,
        }
    }
}

pub fn calc_eval(a: f64, op: char, b: f64) -> f64 {
    match op {
        '+' => a + b,
        '-' => a - b,
        '*' => a * b,
        '/' => {
            if b != 0.0 {
                a / b
            } else {
                f64::NAN
            }
        }
        _ => b,
    }
}

pub fn scientific_eval(label: &str, value: f64, degrees: bool) -> Option<f64> {
    let radians = if degrees { value.to_radians() } else { value };
    match label {
        "sin" => Some(radians.sin()),
        "cos" => Some(radians.cos()),
        "tan" => Some(radians.tan()),
        "asin" => Some(if degrees {
            value.asin().to_degrees()
        } else {
            value.asin()
        }),
        "acos" => Some(if degrees {
            value.acos().to_degrees()
        } else {
            value.acos()
        }),
        "atan" => Some(if degrees {
            value.atan().to_degrees()
        } else {
            value.atan()
        }),
        "ln" => Some(value.ln()),
        "log10" => Some(value.log10()),
        "log2" => Some(value.log2()),
        "x2" => Some(value.powi(2)),
        "x3" => Some(value.powi(3)),
        "sqrt" => Some(value.sqrt()),
        "cbrt" => Some(value.cbrt()),
        "1/x" => Some(1.0 / value),
        "n!" => factorial(value),
        _ => None,
    }
}

pub fn parse_programmer_value(display: &str, base: ProgrammerBase) -> Option<i64> {
    let trimmed = display.trim();
    if trimmed.is_empty() {
        return None;
    }
    let negative = trimmed.starts_with('-');
    let digits = trimmed.trim_start_matches('-');
    let parsed = i64::from_str_radix(digits, base.radix()).ok()?;
    Some(if negative { -parsed } else { parsed })
}

pub fn format_programmer_value(value: i64, base: ProgrammerBase) -> String {
    match base {
        ProgrammerBase::Dec => value.to_string(),
        ProgrammerBase::Hex => format!("{value:X}"),
        ProgrammerBase::Oct => format!("{value:o}"),
        ProgrammerBase::Bin => format!("{value:b}"),
    }
}

pub fn programmer_eval(left: i64, op: &str, right: i64) -> Option<i64> {
    match op {
        "AND" => Some(left & right),
        "OR" => Some(left | right),
        "XOR" => Some(left ^ right),
        "<<" => Some(left << right),
        ">>" => Some(left >> right),
        _ => None,
    }
}

pub fn programmer_not(value: i64) -> i64 {
    !value
}

pub fn programmer_representations(value: i64) -> [(String, String); 4] {
    [
        (
            "DEC".to_string(),
            format_programmer_value(value, ProgrammerBase::Dec),
        ),
        (
            "HEX".to_string(),
            format_programmer_value(value, ProgrammerBase::Hex),
        ),
        (
            "OCT".to_string(),
            format_programmer_value(value, ProgrammerBase::Oct),
        ),
        (
            "BIN".to_string(),
            format_programmer_value(value, ProgrammerBase::Bin),
        ),
    ]
}

pub fn programmer_ascii(value: i64) -> Option<char> {
    u32::try_from(value).ok().and_then(char::from_u32)
}

pub fn format_calc(val: f64) -> String {
    if val.fract() == 0.0 && val.abs() < 1e15 {
        format!("{}", val as i64)
    } else {
        format!("{:.8}", val)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn factorial(value: f64) -> Option<f64> {
    if value < 0.0 || value.fract() != 0.0 {
        return None;
    }
    let n = value as u64;
    let mut acc = 1.0;
    for i in 1..=n {
        acc *= i as f64;
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_add() {
        assert_eq!(calc_eval(2.0, '+', 3.0), 5.0);
    }

    #[test]
    fn eval_subtract() {
        assert_eq!(calc_eval(10.0, '-', 4.0), 6.0);
    }

    #[test]
    fn eval_multiply() {
        assert_eq!(calc_eval(3.0, '*', 7.0), 21.0);
    }

    #[test]
    fn eval_divide() {
        assert_eq!(calc_eval(15.0, '/', 3.0), 5.0);
    }

    #[test]
    fn eval_divide_by_zero_is_nan() {
        assert!(calc_eval(1.0, '/', 0.0).is_nan());
    }

    #[test]
    fn eval_unknown_op_returns_b() {
        assert_eq!(calc_eval(10.0, '%', 3.0), 3.0);
    }

    #[test]
    fn eval_negative_numbers() {
        assert_eq!(calc_eval(-5.0, '+', -3.0), -8.0);
        assert_eq!(calc_eval(-5.0, '*', 2.0), -10.0);
    }

    #[test]
    fn eval_chained_operations() {
        let r1 = calc_eval(5.0, '+', 3.0);
        let r2 = calc_eval(r1, '*', 2.0);
        assert_eq!(r2, 16.0);
    }

    #[test]
    fn scientific_eval_supports_trig_and_factorial() {
        assert_eq!(scientific_eval("x2", 4.0, true), Some(16.0));
        assert_eq!(scientific_eval("n!", 5.0, true), Some(120.0));
        assert!(scientific_eval("sin", 90.0, true).unwrap() > 0.999);
    }

    #[test]
    fn programmer_eval_supports_bitwise_operations() {
        assert_eq!(programmer_eval(0b1100, "AND", 0b1010), Some(0b1000));
        assert_eq!(programmer_eval(4, "<<", 2), Some(16));
        assert_eq!(programmer_not(0), -1);
    }

    #[test]
    fn parse_and_format_programmer_values_roundtrip() {
        assert_eq!(parse_programmer_value("FF", ProgrammerBase::Hex), Some(255));
        assert_eq!(format_programmer_value(255, ProgrammerBase::Hex), "FF");
        assert_eq!(format_programmer_value(10, ProgrammerBase::Bin), "1010");
    }

    #[test]
    fn format_integer() {
        assert_eq!(format_calc(42.0), "42");
    }

    #[test]
    fn format_decimal() {
        assert_eq!(format_calc(3.14), "3.14");
    }

    #[test]
    fn format_trailing_zeros_stripped() {
        assert_eq!(format_calc(2.50), "2.5");
    }

    #[test]
    fn format_zero() {
        assert_eq!(format_calc(0.0), "0");
    }

    #[test]
    fn format_large_integer() {
        assert_eq!(format_calc(1000000.0), "1000000");
    }

    #[test]
    fn format_negative() {
        assert_eq!(format_calc(-7.0), "-7");
    }

    #[test]
    fn format_small_decimal() {
        let s = format_calc(0.001);
        assert!(s.contains("0.001"));
    }
}
