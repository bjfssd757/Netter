pub fn powi(mut a: i64, mut n: i64) -> i64 {
    let mut res = 1;
    while n != 0 {
        if n & 1 != 0 {
            res *= a;
        }
        a *= a;
        n >>= 1;
    }
    res
}