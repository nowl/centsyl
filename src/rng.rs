use std::ops::Range;

use pcg_mwc::Mwc256XXA64;

#[derive(PartialEq)]
pub enum CoinFlip { Heads, Tails}

macro_rules! dnt {
    ( $name:ident ) => {
        fn $name(&mut self) -> u8;
    };
}

macro_rules! dn {
    ( $name:ident, $val:expr ) => {
        fn $name(&mut self) -> u8 {
            use rand::Rng;
            self.gen_range(1..=$val)
        }
    };
}

pub trait Rng {
    fn coin_flip(&mut self) -> CoinFlip;

    dnt!(d4);
    dnt!(d6);
    dnt!(d8);
    dnt!(d10);
    dnt!(d12);
    dnt!(d20);
    dnt!(d100);

    fn range(&mut self, range: Range<i32>) -> i32;
}

impl Rng for Mwc256XXA64 {
    fn coin_flip(&mut self) -> CoinFlip {
        use rand::Rng;
        match self.gen() {
            true => CoinFlip::Heads,
            false => CoinFlip::Tails
        }
    }

    dn!(d4, 4);
    dn!(d6, 6);
    dn!(d8, 8);
    dn!(d10, 10);
    dn!(d12, 12);
    dn!(d20, 20);
    dn!(d100, 100);

    fn range(&mut self, range: Range<i32>) -> i32 {
        use rand::Rng;
        self.gen_range(range)
    }
}