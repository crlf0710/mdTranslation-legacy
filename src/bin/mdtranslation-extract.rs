use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

#[derive(Error, Debug)]
enum Error {
    #[error("io error: {0}")]
    IO(#[from] io::Error),
    #[error("format error: {0}")]
    Fmt(#[from] fmt::Error),
    #[error("from tokens error: {0}")]
    FromTokens(#[from] mdtranslation::from_tokens::FromTokensError),
}

#[derive(Debug, StructOpt)]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Output file, stdout if not present
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    let mut input = fs::File::open(opt.input)?;
    let stdout;
    let mut output_file: Box<dyn io::Write + '_> = if let Some(output_path) = opt.output.as_ref() {
        let file = fs::File::create(output_path)?;
        Box::new(file) as _
    } else {
        stdout = Some(io::stdout());
        let stdout_lock = stdout.as_ref().unwrap().lock();
        Box::new(stdout_lock) as _
    };
    let mut input_text = String::new();
    let _ = input.read_to_string(&mut input_text)?;
    let reader = pulldown_cmark::Parser::new(&input_text);
    let mut ast = mdtranslation::from_tokens::cmark_ast_from_tokens(reader)?;
    ast.perform_sentence_segment();
    let clause_list = ast.extract_clause_list(&pulldown_cmark::CowStr::Borrowed("en-US"));
    let mut output_text = String::new();
    let _ = pulldown_cmark_to_cmark::cmark(clause_list.into_tokens(), &mut output_text, None)?;
    output_file.write_all(output_text.as_bytes())?;

    Ok(())
}
