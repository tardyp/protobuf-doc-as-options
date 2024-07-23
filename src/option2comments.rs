use std::{io::Write, path::PathBuf};

use clap::Parser;
use miette::IntoDiagnostic;
use protox::{
    prost_reflect::{
        prost_types::source_code_info, DynamicMessage, EnumDescriptor, EnumValueDescriptor, FieldDescriptor, FileDescriptor, MessageDescriptor
    },
    Compiler,
};
use std::collections::VecDeque;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// The source file(s) to compile
    #[clap(value_name = "PROTO_FILES", required = true, value_parser)]
    files: Vec<PathBuf>,
    /// The directory in which to search for imports.
    #[clap(
        short = 'I',
        long = "include",
        visible_alias = "proto_path_glob",
        value_name = "PATH_GLOB",
        default_value = ".",
        value_parser
    )]
    includes: Vec<PathBuf>,
    /// The output path to write the modified files to.
    #[clap(
        short = 'o',
        long = "output",
        visible_alias = "output_dir",
        value_name = "PATH",
        value_parser
    )]
    output: Option<PathBuf>,
}
fn main() -> miette::Result<()> {
    miette::set_panic_hook();

    let args = Args::parse();
    let first_include = args
        .includes
        .get(0)
        .expect("at least one include dir is expected")
        .clone();
    let mut compiler = Compiler::new(args.includes)?;
    compiler.include_imports(false);
    compiler.include_source_info(true);
    let out_dir = args.output.or_else(|| Some(PathBuf::from("out"))).unwrap();
    for file_glob in args.files {
        let file_glob = file_glob.to_string_lossy();
        let file_glob = shellexpand::tilde(&file_glob);
        for file in glob::glob(&file_glob).into_diagnostic()? {
            let file = file.into_diagnostic()?;
            let relative = file
                .strip_prefix(&first_include)
                .into_diagnostic()?
                .to_path_buf();
            let out_file = out_dir.join(relative.clone());
            std::fs::create_dir_all(out_file.parent().unwrap()).into_diagnostic()?;
            let in_text = std::fs::read_to_string(&file).into_diagnostic()?;
            let mut out = std::fs::File::create(out_file).into_diagnostic()?;
            compiler.open_file(file)?;
            let fd = compiler
                .descriptor_pool()
                .get_file_by_name(&relative.to_string_lossy())
                .unwrap();
            insert_comments(&fd, &in_text, &mut out)?;
        }
    }
    Ok(())
}
fn get_lines_offsets(text: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut offset = 0;
    for line in text.lines() {
        offsets.push(offset);
        offset += line.len() + 1;
    }
    offsets
}
fn insert_comments(
    fd: &FileDescriptor,
    in_text: &str,
    out: &mut std::fs::File,
) -> miette::Result<()> {
    let mut out_text = in_text.to_string();
    let offsets = get_lines_offsets(&out_text);
    let source_info = fd
        .file_descriptor_proto()
        .source_code_info
        .as_ref()
        .unwrap();
    // need to insert in reverse order
    let mut insertions = Vec::new();
    for loc in source_info.location.iter() {
        let start_line = loc.span[0] as usize;
        let start_col = loc.span[1] as usize;
        let start = offsets[start_line] + start_col;
        if let Some(comment) = get_comment(fd, loc) {
            let spaces = &in_text[start - start_col..start];
            println!("comment {}", &comment);
            insertions.push((start, format_comment(comment, spaces)));
        }
    }
    insertions.sort_by_key(|(start, _)| *start);
    for (start, text) in insertions.into_iter().rev() {
        out_text.insert_str(start, &text);
    }
    out.write_all(out_text.as_bytes()).into_diagnostic()?;
    Ok(())
}

/// Format a comment to fit within 100 characters
/// and add the correct padding
fn format_comment(comment: String, spaces: &str) -> String {
    let mut lines = Vec::new();
    let mut line = String::new();
    let padding_size = spaces.len() + 4;
    for word in comment.split_whitespace() {
        if line.len() + word.len() + padding_size > 100 {
            lines.push(line.clone());
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    let mut formatted = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            formatted.push_str(spaces);
        }
        formatted.push_str("// ");
        formatted.push_str(&line);
        formatted.push('\n');
    }
    formatted.push_str(spaces);
    formatted
}

trait Commented {
    fn get_comment(&self) -> Option<String>;
}
impl Commented for DynamicMessage {
    fn get_comment(&self) -> Option<String> {
        self.extensions()
            .find(|ext| ext.0.name().ends_with("description"))?
            .1
            .as_str()
            .map(|s| s.to_string())
    }
}
impl Commented for MessageDescriptor {
    fn get_comment(&self) -> Option<String> {
        self.options().get_comment()
    }
}
impl Commented for EnumDescriptor {
    fn get_comment(&self) -> Option<String> {
        self.options().get_comment()
    }
}
impl Commented for EnumValueDescriptor {
    fn get_comment(&self) -> Option<String> {
        self.options().get_comment()
    }
}
impl Commented for FieldDescriptor {
    fn get_comment(&self) -> Option<String> {
        self.options().get_comment()
    }
}

fn get_comment(fd: &FileDescriptor, loc: &source_code_info::Location) -> Option<String> {
    let mut path: VecDeque<i32> = loc.path.iter().copied().collect();
    // pop first element
    let typ = path.pop_front()?;
    let idx = path.pop_front()? as usize;
    match typ {
        4 => {
            let message = fd.messages().nth(idx)?;
            if path.is_empty() {
                message.get_comment()
            } else {
                get_comment_for_message(&message, path)
            }
        }
        5 => {
            let enum_ = fd.enums().nth(idx)?;
            if path.is_empty() {
                enum_.get_comment()
            } else {
                get_comment_for_enum(&enum_, path)
            }
        }
        _ => None,
    }
}

fn get_comment_for_message(message: &MessageDescriptor, mut path: VecDeque<i32>) -> Option<String> {
    let typ = path.pop_front()?;
    let idx = path.pop_front()?;
    match typ {
        2 => {
            let field = message.fields().nth(idx as usize)?;
            if path.is_empty() {
                field.get_comment()
            } else {
                None
            }
        }
        3 => {
            let nested_message = message.child_messages().nth(idx as usize)?;
            if path.is_empty() {
                nested_message.get_comment()
            
            } else {
                get_comment_for_message(&nested_message, path)
            }
        }
        4 => {
            let enum_ = message.child_enums().nth(idx as usize)?;
            if path.is_empty() {
                enum_.get_comment()
            } else {
                get_comment_for_enum(&enum_, path)
            }
        }
        _ => None,
    }
}

fn get_comment_for_enum(r#enum: &EnumDescriptor, mut path: VecDeque<i32>) -> Option<String> {
    let typ = path.pop_front()?;
    let idx = path.pop_front()?;
    match typ {
        2 => {
            let value = r#enum.values().nth(idx as usize)?;
            if path.is_empty() {
                value.get_comment()
            } else {
                None
            }
        }
        _ => None,
    }
}
