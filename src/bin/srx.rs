use nltk_test::{run};
use srx::{SRX, Rules};

fn main() {
    let srx: SRX = include_str!("../../data/segment.srx").parse().unwrap();
    let english_rules = srx.language_rules("en").compile();
    //run(|text| english_rules.split(text)
}
