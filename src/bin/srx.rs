#![feature(type_alias_impl_trait)]

use nltk_test::{run};
use srx::{SRX, Rules};

fn main() {
    let srx: SRX = include_str!("../../data/segment.srx").parse().unwrap();
    let english_rules = srx.language_rules("en");
    run(|paras| paras.iter().map(|p| english_rules.split(p)).collect());
}
