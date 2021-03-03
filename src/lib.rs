use walkdir::WalkDir;
use itertools::Itertools;
use rayon::prelude::*;

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
            ('\n', AccState::Break) => self.state = AccState::Para,
            ('\n', AccState::Para) => {},
            ('\n', _) => self.state = AccState::Break,
            (c, AccState::Word) if c.is_whitespace() => self.state = AccState::Space,
            (c, _) if c.is_whitespace() => {},
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

fn tag_content<'a>(data: &'a str, tag: &str, mut cb: impl FnMut(&'a str)) {
    let start_lower = format!("<{}>", tag.to_lowercase());
    let start_upper = format!("<{}>", tag.to_uppercase());
    let end_lower = format!("</{}>", tag.to_lowercase());
    let end_upper = format!("</{}>", tag.to_uppercase());

    data
        .split(&start_lower).skip(1)
        .flat_map(|part| part.rsplitn(1, &end_lower))
        .for_each(|s| cb(s));
    
    data.split(&start_upper).skip(1)
        .flat_map(|part| part.rsplitn(1, &end_upper))
        .for_each(|s| cb(s));
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
    tag_content(input, "document", |doc| {
        tag_content(doc, "html", |html| {
            let mut reader = Reader::from_str(html);
            reader.check_end_names(false);
            loop {
                match reader.read_event(&mut buf) {
                    Ok(Event::Text(b)) => {
                        let text = ok_or_continue!(b.unescaped());
                        let s = ok_or_continue!(reader.decode(text.as_ref()));
                        accumulator.push(' ');
                        accumulator.push_str(s);
                        accumulator.push(' ');
                    }
                    Ok(Event::Start(ref e)) => {
                        match e.name() {
                            b"br" | b"BR" | b"div" | b"DIV" => accumulator.push('\n'),
                            b"p" | b"P" | b"TITLE" | b"title" => accumulator.push('\n'),
                            _ => {}
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        match e.name() {
                            b"p" | b"P" | b"TITLE" | b"title" => accumulator.push('\n'),
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
            buf.clear();
            accumulator.push('\n');
        });
    });
}