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
