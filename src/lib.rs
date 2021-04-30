use walkdir::WalkDir;
use itertools::Itertools;
use rayon::prelude::*;
use serde::Deserialize;
use std::borrow::Cow;

macro_rules! ok_or_continue {
    ($r:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => {
                println!("{:?}", e);
                continue;
            }
        }
    };
}



#[derive(Copy, Clone)]
enum AccState {
    Word,
    Space,
    CellSep,
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
    fn push_raw(&mut self, s: &str) {
        match self.state {
            AccState::Space | AccState::Break => self.data.push(' '),
            AccState::Para => {
                self.data.push('\n');
                self.paras.push(self.data.len());
            },
            _ => {}
        }
        self.data.push_str(s);
        self.state = AccState::Word;
    }
    fn push_cell_sep(&mut self) {
        match self.state {
            AccState::Para => {}
            _ => self.state = AccState::CellSep,
        }
    }
    fn push_break(&mut self) {
        match self.state {
            AccState::Para => {}
            _ => self.state = AccState::Break,
        }
    }
    fn push(&mut self, c: char) {
        if c == '\n' {
            match self.state {
                AccState::Break => self.state = AccState::Para,
                AccState::Para => {},
                _ => self.state = AccState::Break,
            }
        } else if c.is_whitespace() {
            match self.state {
                AccState::Word => self.state = AccState::Space,
                _ => {},
            }
        } else {
            match self.state {
                AccState::Word => {},
                AccState::CellSep => {
                    self.data.push_str(" | ");
                }
                AccState::Space => {
                    self.data.push(' ');
                }
                AccState::Break => {
                    self.data.push('\n');
                }
                AccState::Para => {
                    self.data.push('\n');
                    self.paras.push(self.data.len());
                }
            }
            self.data.push(c);
            self.state = AccState::Word;
        }
    }
    fn clear(&mut self) {
        self.data.clear();
        self.paras.clear();
    }
    fn splits(&self) -> Vec<&str> {
        let mut splits = Vec::with_capacity(self.paras.len() + 2);
        splits.extend(self.paras.iter().cloned().tuple_windows().map(|(a, b)| &self.data[a .. b]));
        if let Some(&last) = self.paras.last() {
            splits.push(&self.data[.. last]);
        }
        splits.push(&self.data[self.paras.last().cloned().unwrap_or(0)..]);
        splits
    }
}

pub fn run(splitter: impl Fn(&str) + Sync) {
    let root = std::env::args().nth(1).expect("no input filename");
    
    let files: Vec<_> = WalkDir::new(root).into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file()).collect();

    let total_bytes: usize = files.into_par_iter()
        .map(|entry| {
            let mut n_paras = 0;
            let mut n_sents = 0;
            let mut n_bytes = 0;

            let input = std::fs::read_to_string(entry.path()).expect("can't read input");

            n_bytes
        })
        .sum();
    
    println!("{} total bytes", total_bytes);
}

pub fn clean() {
    use std::io::{Write, BufWriter};
    let root = std::env::args().nth(1).expect("no input filename");
    
    let files: Vec<_> = WalkDir::new(root).into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().map(|ext| ext == "txt").unwrap_or(false))
        .collect();

    let total_bytes: usize = files.into_par_iter()
        .map(|entry| {
            let mut accumulator = TextAccumulator::new();

            let raw_input = std::fs::read(entry.path()).expect("can't read input");
            let input = String::from_utf8_lossy(&raw_input);
            parse_quick_xml(&input, &mut accumulator);
            //parse_html(&input, &mut accumulator);
            
            let paras = accumulator.splits();
            let n_bytes = accumulator.data.len();

            if n_bytes != 0 {
                let mut out = BufWriter::new(std::fs::File::create(entry.path().with_extension("plain")).unwrap());
                for para in paras {
                    out.write_all(para.as_bytes()).unwrap();
                    out.write_all("\n".as_bytes()).unwrap();
                }
            }
            accumulator.clear();

            println!("{} bytes in {:?}", n_bytes, entry.path());

            n_bytes
        })
        .sum();
    
    println!("{} total bytes", total_bytes);
}

fn parse_quick_xml(input: &str, accumulator: &mut TextAccumulator) {
    use quick_xml::{Reader, events::Event};

    let mut buf = Vec::new();
    let mut reader = Reader::from_str(input);
    reader.check_end_names(false);
    let mut in_html = false;
    let mut in_cell = false;
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Text(b)) => {
                if b.starts_with(b"\nbegin 644 ") {
                    println!("skip");
                    reader.read_until_marker(&mut buf, b"end");
                    continue;
                }
                match b.unescaped() {
                    Ok(Cow::Borrowed(text)) => {
                        let s = ok_or_continue!(reader.decode(text.as_ref()));
                        accumulator.push(' ');
                        if in_html {
                            accumulator.push_str(s);
                        } else {
                            accumulator.push_raw(s);
                        }
                        accumulator.push(' ');
                    }
                    Ok(Cow::Owned(text)) => {
                        // might contain HTML ... don't ask
                        let s = ok_or_continue!(reader.decode(text.as_ref()));
                        parse_quick_xml(s, accumulator);
                    }
                    Err(_) => continue
                }
            }
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"br" | b"BR" | b"tr" | b"TR"=> accumulator.push_break(),
                    b"p" | b"P" | b"TITLE" | b"title" => accumulator.push('\n'),
                    b"div" | b"DIV" if !in_cell => accumulator.push('\n'),
                    b"HTML" | b"html" => in_html = true,
                    b"td" | b"TD" => in_cell = true,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"p" | b"P" | b"TITLE" | b"title" => accumulator.push('\n'),
                    b"tr" | b"TR" => accumulator.push_break(),
                    b"div" | b"DIV" if !in_cell => accumulator.push('\n'),
                    b"HTML" | b"html" => in_html = false,
                    b"td" | b"TD" => {
                        in_cell = false;
                        accumulator.push_cell_sep();
                    },
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                println!("{:?}", e);
            }
            _ => {}
        }
    }
    accumulator.push('\n');
}

#[derive(Deserialize)]
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Match<'a> {
    pub span: (usize, usize),
    pub label: &'a str,
    pub text: Cow<'a, str>,
}

pub struct IndexTranslator<'a> {
    data: &'a str,
    byte_idx: usize,
    char_idx: usize,
}
impl<'a> IndexTranslator<'a> {
    pub fn new(data: &'a str) -> IndexTranslator<'a> {
        IndexTranslator {
            data, byte_idx: 0, char_idx: 0
        }
    }
    pub fn next_byte_idx_for_char_idx(&mut self, char_idx: usize) -> Option<usize> {
        assert!(char_idx >= self.char_idx);
        let mut chars = self.data[self.byte_idx..].chars();
        let n_chars = char_idx - self.char_idx;
        if chars.by_ref().take(n_chars).count() == n_chars {
            let byte_idx = self.data.len() - chars.as_str().len();
            assert!(byte_idx >= self.byte_idx);
            self.byte_idx = byte_idx;
            self.char_idx = char_idx;
            Some(byte_idx)
        } else {
            None
        }
    }
}
