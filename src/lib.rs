#![feature(test)]
#![allow(unused_must_use)]
/// This library exposes a single function to generate a random `u64` using Lemire's nearly divisionless
/// approach as documented on [his blog](https://lemire.me/blog/2019/06/06/nearly-divisionless-random-integer-generation-on-various-systems/)
use rand::prelude::*;

/// Simple error returned by the ndl_rand function
#[derive(Debug)]
pub struct RandError {}
impl std::fmt::Display for RandError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.write_str("Random number generator error")?;
        Ok(())
    }
}
impl std::error::Error for RandError {}

/// Genrates a random number between 0 and the given `max` paramter.
/// Returns an error if the `max` parameter is 0 or we could not reach
/// a reasonable random number within 10 iterations.
pub fn ndl_rand(max: u64) -> Result<u64, RandError> {
    if max == 0 {
        return Err(RandError {});
    }

    let mut rand_seed = thread_rng().gen::<u64>();
    let mut rand_dividend = (rand_seed as u128) * (max as u128);
    // the cast operations truncates the leading bytes from the u128.
    // the cast_uints_same_as_c test function confirms that the behavior
    // of this operation in rust is the same as in C.
    let mut rand_dividend_u64 = rand_dividend as u64;

    // Rejecting values to correct for bias. E.g. in tossing a coin it is
    // probably easier to make two consecutive tosses independent than to
    // toss heads with probability exactly one-half. If independence of
    // successive tosses is assumed, we can reconstruct a 50-50 chance out
    // of even a badly biased coin by tossing twice. If we get heads-heads
    // or tails-tails, we reject the tosses and try again. If we get
    // heads-tails (or tails-heads), we accept the result as heads, etc
    // https://mcnp.lanl.gov/pdf_files/nbs_vonneumann.pdf
    if rand_dividend_u64 < max {
        // rust only lets me apply the unary minus to signed types.
        // The unary_minus_same_as_c test validates that this behaves
        // in the same way as the unary minus on an unsigned C type.
        let t = -(max as i64) % (max as i64);
        while rand_dividend_u64 < t as u64 {
            rand_seed = thread_rng().gen::<u64>();
            rand_dividend = (rand_seed as u128) * (max as u128);
            rand_dividend_u64 = rand_dividend as u64;
        }
    }
    // (x*s)/2^L - 2^64 is the divsor so we shift right
    Ok((rand_dividend >> 64) as u64)
}

#[cfg(test)]
mod tests {
    extern crate test;

    use super::ndl_rand;
    use kolmogorov_smirnov;
    use rand::prelude::*;

    static ITERATIONS: usize = 10_000;
    static MAX_RANGE: u64 = 10_000;

    #[test]
    fn errors_on_0_max() {
        assert!(ndl_rand(0).is_err());
    }

    // Kolmogorov-Smirnov test for distributions comparing the random numbers vs
    // the smooth increment curve
    #[test]
    fn kolmogorov_smirnov_test() {
        let mut rands: Vec<u64> = Vec::with_capacity(ITERATIONS);
        let mut smooth: Vec<u64> = Vec::with_capacity(ITERATIONS);
        for i in 0..ITERATIONS {
            rands.push(ndl_rand(MAX_RANGE).unwrap());
            smooth.push(i as u64);
        }

        assert_eq!(ITERATIONS, rands.len());
        let stats = kolmogorov_smirnov::test(rands.as_slice(), smooth.as_slice(), 0.99999999999999);
        println!("is_rejected: {}", stats.is_rejected);
        println!("statistic: {}", stats.statistic);
        println!("critical value: {}", stats.critical_value);
        println!("confidence: {}", stats.confidence);
        assert!(!stats.is_rejected);
    }

    #[test]
    fn cast_uints_same_as_c() {
        let mut rnd = thread_rng().gen::<u128>();
        let mut attempts_cnt = 0;
        while rnd < std::u64::MAX as u128 {
            assert!(attempts_cnt < 50);
            rnd = thread_rng().gen::<u128>();
            attempts_cnt += 1;
        }

        let cast = rnd as u64;
        // section 4.7 §2 and §3: If the destination type is unsigned,
        // the resulting value is the least unsigned integer congruent
        // to the source integer (modulo 2^n where n is the number of
        // bits used to represent the unsigned type). [Note: In a two's
        // complement representation, this conversion is conceptual and
        // there is no change in the bit pattern (if there is no truncation).]
        let c_assigned = rnd % (2 as u128).pow(64);
        assert!(c_assigned <= std::u64::MAX as u128);
        assert_eq!(cast, c_assigned as u64);
    }

    #[test]
    fn unary_minus_same_as_c() {
        let x = thread_rng().gen::<u64>();
        // 6.2.5c9 says: A computation involving unsigned operands can never
        // overflow, because a result that cannot be represented by the resulting
        // unsigned integer type is reduced modulo the number that is one greater
        // than the largest value that can be represented by the resulting type.
        // or from C++
        // The negative of an unsigned quantity is computed by subtracting its
        // value from 2^n, where n is the number of bits in the promoted operand
        let c_neg: u64 = std::u64::MAX - x + 1;
        assert!(c_neg <= std::u64::MAX);
        let rust_neg: i64 = -(x as i64);
        assert_eq!(c_neg, rust_neg as u64)
    }

    #[bench]
    fn gen_1000_randoms_to_1000(b: &mut test::Bencher) {
        b.iter(|| {
            ndl_rand(1000);
        })
    }

    #[bench]
    fn rand_gen_1000_randoms_to_1000(b: &mut test::Bencher) {
        b.iter(|| {
            thread_rng().gen_range(0, 1000);
        })
    }
}
