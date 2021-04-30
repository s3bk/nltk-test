#![feature(bool_to_option)]

use serde::Deserialize;
use itertools::{Itertools};
use std::{iter, fs, fmt::Write};
use std::path::PathBuf;
use nltk_test::{IndexTranslator, Match};

fn merge(plain: &str, matches: &[Vec<Vec<Match>>], out: &mut String) {
    use tuple::{TupleElements};

    let paragraphs = plain.split("\n\n");

    for (para_idx, paragraph) in paragraphs.enumerate() {
        out.push_str("<p>");

        let mut idx_translator = IndexTranslator::new(paragraph);

        let transitions = matches.iter().enumerate()
            .map(|(i, matches)| matches[para_idx].iter().flat_map(move |m| {
                assert!(m.span.1 > m.span.0);
                [
                    (m.span.0, false, i, m),
                    (m.span.1, true, i, m),
                ].into_elements()
            }))
            .kmerge()
            .sorted()
            .group_by(|a| a.0);


        let mut last = None;
        let mut active = vec![false; matches.len()];
        for (char_idx, events) in transitions.into_iter() {
            let pos = idx_translator.next_byte_idx_for_char_idx(char_idx).unwrap();
            if let Some(start) = last {
                if start < pos {
                    out.push_str(&paragraph[start..pos]);
                }
                if active.iter().any(|&high| high) {
                    out.push_str("</span>");
                }
            } else {
                out.push_str(&paragraph[..pos]);
            }
            for (_, falling, i, m) in events {
                if !falling {
                    write!(out, "<div class=\"label\" input=\"{}\" title=\"{}\">{}</div>", i+1, m.text, m.label);
                }
                active[i] = !falling;
            }
            
            if active.iter().any(|&high| high) {
                let classes = active.iter().enumerate()
                    .filter_map(|(i, &high)| high.then_some(i))
                    .format_with(" ", |i, f| f(&format_args!("i{}", i+1)));
                
                write!(out, "<span class=\"{}\">", classes);
            }
            last = Some(pos);
        }

        if let Some(start) = last {
            out.push_str(&paragraph[start..]);
            if active.iter().any(|&high| high) {
                out.push_str("</span>");
            }
        }

        out.push_str("</p>\n");
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let plain_path = args.next().expect("no plain file");
    let plain = fs::read_to_string(&plain_path).unwrap();
    let inputs: Vec<_> = args.collect();

    let match_files: Vec<_> = inputs.iter().map(|path| fs::read_to_string(&path).unwrap()).collect();
    
    let matches: Vec<_> = match_files.iter().zip(inputs.iter()).
        map(|(data, path)| {
            match serde_json::from_str(&data) {
                Ok(m) => m,
                Err(e) => panic!("{:?}:{:?}", path, e)
            }
        }).collect();

    let mut out = String::with_capacity(2 * plain.len());
    out.push_str(HEAD);
    merge(&plain, &matches, &mut out);
    out.push_str(TAIL);

    let html_path = PathBuf::from(plain_path).with_extension("html");
    fs::write(&html_path, out).unwrap();
}

const HEAD: &str = r###"<html>
  <head>
    <meta charset="UTF-8">
    <style type="text/css">
span.i1 {
    background-color: rgba(200,0,0,0.2);
}
span.i2 {
    background-color: rgba(0,0,200,0.2);
}
span.i3 {
    background-color: rgba(0,200,0,0.2);
}
span.i1.i2 {
    background-color: rgba(150,0,150,0.2);
}
span.i1.i3 {
    background-color: rgba(150,150,0,0.2);
}
span.i2.i3 {
    background-color: rgba(0,150,150,0.2);
}
span.i1.i2.i3 {
    background-color: rgba(0,0,0,0.2);
}

div.label::before {
    content: attr(input) ":";
}
div.label::after {
    content: " ";
}
div.label {
    position: relative;
    display: inline;
    font-size: 67%;
}
    </style>
  </head>
  <body>
"###;

const TAIL: &str = r###"
  </body>
</html>
"###;
