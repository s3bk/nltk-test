use quick_xml::{Reader, events::Event};
use walkdir::WalkDir;
use itertools::Itertools;
use rayon::prelude::*;

macro_rules! ok_or_continue {
    ($r:expr) => {
        match $r {
            Ok(v) => v,
            _ => continue
        }
    };
}



#[derive(Copy, Clone)]
enum AccState {
    Word,
    Space,
    Break,
    Para
}

struct TextAccumulator {
    paras: Vec<usize>,
    data: String,
    state: AccState,
}
impl TextAccumulator {
    fn new() -> Self {
        TextAccumulator {
            paras: Vec::new(),
            data: String::new(),
            state: AccState::Para,
        }
    }
    fn push_str(&mut self, s: &str) {
        for c in s.chars() {
            self.push(c);
        }
    }
    fn push(&mut self, c: char) {
        match (c, self.state) {
            (' ', AccState::Word) | ('\u{A0}', AccState::Word) => self.state = AccState::Space,
            (' ', _) | ('\u{A0}', _) => {},
            ('\n', AccState::Break) => self.state = AccState::Para,
            ('\n', AccState::Para) => {},
            ('\n', _) => self.state = AccState::Break,
            (c, AccState::Word) => self.data.push(c),
            (c, AccState::Space) | (c, AccState::Break) => {
                self.data.push(' ');
                self.data.push(c);
                self.state = AccState::Word;
            }
            (c, AccState::Para) => {
                self.data.push('\n');
                self.paras.push(self.data.len());
                self.data.push(c);
                self.state = AccState::Word;
            }
        }
    }
    fn clear(&mut self) {
        self.data.clear();
        self.paras.clear();
    }
    fn splits(&self) -> Vec<&str> {
        let mut splits = Vec::with_capacity(self.paras.len() + 1);
        splits.extend(self.paras.iter().cloned().tuple_windows().map(|(a, b)| &self.data[a .. b]));
        if let Some(&last) = self.paras.last() {
            splits.push(&self.data[.. last]);
        }
        splits
    }
}

pub fn run(splitter: impl for<'a> Fn(&[&'a str]) -> Vec<Vec<&'a str>> + Sync) {
    let root = std::env::args().nth(1).expect("no input filename");
    
    let files: Vec<_> = WalkDir::new(root).into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file()).collect();

    let total_bytes: usize = files.into_par_iter()
        .map(|entry| {
            let mut n_paras = 0;
            let mut n_sents = 0;
            let mut n_bytes = 0;

            let mut accumulator = TextAccumulator::new();
            let mut buf = Vec::new();

            let input = std::fs::read_to_string(entry.path()).expect("can't read input");

            for doc in tag_content(&input, "<DOCUMENT>", "</DOCUMENT>") {
                for html in tag_content(doc, "<HTML>", "</HTML>") {
                    let mut reader = Reader::from_str(html);
                    while let Ok(e) = reader.read_event(&mut buf) {
                        match e {
                            Event::Text(b) => {
                                let text = ok_or_continue!(b.unescaped());
                                let s = ok_or_continue!(reader.decode(text.as_ref()));
                                accumulator.push(' ');
                                accumulator.push_str(s);
                                accumulator.push(' ');
                            }
                            Event::Start(ref e) => {
                                match e.name() {
                                    b"br" | b"BR" => accumulator.push('\n'),
                                    b"p" | b"P" | b"TITLE"  => accumulator.push('\n'),
                                    _ => {}
                                }
                            }
                            Event::End(ref e) => {
                                match e.name() {
                                    b"p" | b"P" | b"TITLE" => accumulator.push('\n'),
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    buf.clear();
                }
            }

            
            let mut paras = accumulator.splits();
            n_paras += paras.len();
            n_bytes += accumulator.data.len();

            for para in splitter(&paras) {
                for sent in para {
                    println!("{}", sent);
                    n_sents += 1;
                }
                //println!();
            }

            accumulator.clear();

            println!("{} bytes in {:?}", n_bytes, entry.path());

            n_bytes
        })
        .sum();
    
    println!("{} total bytes", total_bytes);
}

fn tag_content<'a>(data: &'a str, start: &'a str, end: &'a str) -> impl Iterator<Item=&'a str> + 'a {
    data.split(start).skip(1)
        .flat_map(move |part| part.rsplitn(1, end))
}
