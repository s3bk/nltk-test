use nnsplit::tract_backend::NNSplit;
use nnsplit::NNSplitOptions;
use nltk_test::{run};

fn main() {
    let mut options = NNSplitOptions::default();
    options.batch_size = 512;
    let splitter = NNSplit::new("data/nnsplit/en/model.onnx", options).unwrap();

    run(|paras| splitter.split(paras).into_iter().map(|splits| splits.iter().map(|s| s.text()).collect()).collect());
}
