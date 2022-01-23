use pgx::*;
use std::ffi::CStr;
use std::str::FromStr;

pg_module_magic!();

#[derive(Debug, Copy, Clone, PostgresType)]
#[pgvarlena_inoutfuncs]
pub struct Fraction(fraction::Fraction);

impl PgVarlenaInOutFuncs for Fraction {
    fn input(input: &CStr) -> PgVarlena<Self> {
        let mut result = PgVarlena::<Self>::new();
        let input = unsafe { std::str::from_utf8_unchecked(input.to_bytes()) };

        // Note: This only accepts numbers that represent fractions. Decimal
        // numbers are currently not supported. Although, if the underlying
        // fraction library supports parsing fractions, then I totally missed
        // it, and that makes my parse fraction function useless. :D
        let frac = parse_fraction(input).expect("Could not parse fraction").1;
        result.0 = frac;

        result
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(&format!("{}", self.0))
    }
}

fn parse_fraction(input: &str) -> nom::IResult<&str, fraction::Fraction> {
    // Parse an input fraction and return the fraction type.
    nom::branch::alt((
        // Case 1: 123 / 2
        nom::combinator::map(
            // Convert "n / m" -> ("n", "m")
            nom::sequence::separated_pair(
                // Convert "- n" -> (Some('-'), "n"), or "n" -> (None, "n")
                nom::sequence::separated_pair(
                    nom::combinator::opt(nom::character::complete::char('-')),
                    nom::character::complete::space0,
                    nom::character::complete::digit1,
                ),
                nom::sequence::tuple((
                    nom::character::complete::space0,
                    nom::character::complete::char('/'),
                    nom::character::complete::space0,
                )),
                nom::character::complete::digit1,
            ),
            // Convert the parsed text into a fraction.
            |v: ((Option<char>, &str), &str)| {
                let sign = match v.0 .0 {
                    Some(_) => fraction::Sign::Minus,
                    None => fraction::Sign::Plus,
                };
                // The numerator and denominator should parse fine at this
                // point, but someone may try to use numbers that cannot be
                // represented by a u64.
                let num = u64::from_str(v.0 .1).expect("Could not parse digits");
                let den = u64::from_str(v.1).expect("Could not parse digits");

                fraction::Fraction::new_generic(sign, num, den).expect("Could not create fraction")
            },
        ),
        // Case 2: 123
        nom::combinator::map(
            // Convert "- n" -> (Some('-'), "n"), or "n" -> (None, "n")
            nom::sequence::separated_pair(
                nom::combinator::opt(nom::character::complete::char('-')),
                nom::character::complete::space0,
                nom::character::complete::digit1,
            ),
            |v: (Option<char>, &str)| {
                let sign = match v.0 {
                    Some(_) => fraction::Sign::Minus,
                    None => fraction::Sign::Plus,
                };
                // The numerator should parse fine at this point, but someone
                // may try to use numbers that cannot be represented by a u64.
                let num = u64::from_str(v.1).expect("Could not parse digits");

                // Still return a fraction, but denominator is 1.
                fraction::Fraction::new_generic(sign, num, 1).expect("Could not create fraction")
            },
        ),
    ))(input)
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use super::parse_fraction;
    use pgx::*;

    #[pg_test]
    fn test_parse_fraction() {
        for (text, expected) in [
            ("1", fraction::Fraction::new(1u64, 1u64)),
            ("-1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("- 1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("1/1", fraction::Fraction::new(1u64, 1u64)),
            ("1 /1", fraction::Fraction::new(1u64, 1u64)),
            ("1/ 1", fraction::Fraction::new(1u64, 1u64)),
            ("1 / 1", fraction::Fraction::new(1u64, 1u64)),
            ("-1/1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("-1 /1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("-1/ 1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("-1 / 1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("- 1/1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("- 1 /1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("- 1/ 1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("- 1 / 1", fraction::Fraction::new_neg(1u64, 1u64)),
            ("1/2", fraction::Fraction::new(1u64, 2u64)),
            ("2/1", fraction::Fraction::new(2u64, 1u64)),
        ] {
            let result = parse_fraction(&text).unwrap();

            assert_eq!(
                result.1, expected,
                "{} == {} ({})",
                result.1, expected, text
            );
        }
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
