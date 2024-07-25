mod path_resolver;

use std::{io::Write, path::PathBuf};

use clap::Parser;
use miette::IntoDiagnostic;
use path_resolver::{tag, PathedChilds, PathedDescriptor};
use prost_reflect::{
    prost_types::source_code_info, DynamicMessage, EnumDescriptor, EnumValueDescriptor,
    ExtensionDescriptor, FieldDescriptor, FileDescriptor, MessageDescriptor, MethodDescriptor,
    ServiceDescriptor, Value,
};
use protox::Compiler;

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
    entry_point(Args::parse())
}
fn entry_point(args: Args) -> miette::Result<()> {
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
    let offsets = get_lines_offsets(&in_text);
    let source_info = fd
        .file_descriptor_proto()
        .source_code_info
        .as_ref()
        .unwrap();
    // need to insert in reverse order
    let mut insertions = Vec::new();
    let mut to_remove = Vec::new();
    for loc in source_info.location.iter() {
        let start_line = loc.span[0] as usize;
        let start_col = loc.span[1] as usize;
        let start = offsets[start_line] + start_col;
        if let Some(pathed) = fd.get_child_from_loc(loc) {
            if let Some(ext) = get_description(&pathed) {
                let spaces = &in_text[start - start_col..start];
                let comment = ext.value.as_str().unwrap().to_string();
                let mut to_remove_path = loc.path.clone();
                to_remove_path.push(get_option(&pathed));
                to_remove_path.push(ext.desc.number() as i32);
                to_remove.push(to_remove_path);
                insertions.push((start, format_comment(comment, spaces)));
            }
        }
    }
    let mut to_remove_spans = to_remove
        .iter()
        .filter_map(|path| {
            for loc in source_info.location.iter() {
                if loc.path == *path {
                    let span = &loc.span;
                    let start_line = span[0] as usize;
                    let start_col = span[1] as usize;
                    let (end_line, end_col) =
                    match span.len() {
                        3 => (span[0] as usize, span[2] as usize),
                        4 => (span[2] as usize, span[3] as usize),
                        _ => return None,
                    };
                    let start = offsets[start_line] + start_col;
                    let end = offsets[end_line] + end_col;
                    return Some((start, end));
                }
            }
            None
        }).collect::<Vec<_>>();
    insertions.sort_by_key(|(start, _)| *start);
    to_remove_spans.sort_by_key(|(start, _)| *start);
    let mut last = 0;
    let mut to_remove_spans = to_remove_spans.into_iter();
    let mut to_remove_span = to_remove_spans.next();
    for (start, text) in insertions.into_iter() {
        while let Some((mut remove_start, mut remove_end)) = to_remove_span {
            if in_text.as_bytes()[remove_start-1] == b'[' && in_text.as_bytes()[remove_end] == b']' {
                (remove_start, remove_end) = (remove_start-1, remove_end+1);
            }
            if remove_start < start {
                out.write_all(&in_text[last..remove_start].as_bytes())
                    .into_diagnostic()?;
                last = remove_end;
                to_remove_span = to_remove_spans.next();
            } else {
                break;
            }
        }
        out.write_all(&in_text[last..start].as_bytes())
            .into_diagnostic()?;
        last = start;
        out.write_all(text.as_bytes()).into_diagnostic()?;
    }
    while let Some((mut remove_start, mut remove_end)) = to_remove_span {
        if in_text.as_bytes()[remove_start-1] == b'[' && in_text.as_bytes()[remove_end] == b']' {
            (remove_start, remove_end) = (remove_start-1, remove_end+1);
        }
    out.write_all(&in_text[last..remove_start].as_bytes())
            .into_diagnostic()?;
        last = remove_end;
        to_remove_span = to_remove_spans.next();
    }
    out.write_all(&in_text[last..].as_bytes())
        .into_diagnostic()?;
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

struct Ext {
    desc: ExtensionDescriptor,
    value: Value,
}

trait Described {
    fn get_description(&self) -> Option<Ext>;
}
impl Described for DynamicMessage {
    fn get_description(&self) -> Option<Ext> {
        self.extensions()
            .find(|ext| ext.0.name().ends_with("description"))
            .map(|(ed, v)| Ext {
                desc: ed,
                value: v.clone(),
            })
    }
}
macro_rules! impl_commented {
    ($($t:ty),*) => {
        $(impl Described for $t {
            fn get_description(&self) -> Option<Ext> {
                self.options().get_description()
            }
        })*
    };
}
impl_commented!(
    MessageDescriptor,
    EnumDescriptor,
    EnumValueDescriptor,
    FieldDescriptor,
    ServiceDescriptor,
    MethodDescriptor
);
fn get_description(pathed: &PathedDescriptor) -> Option<Ext> {
    match pathed {
        PathedDescriptor::Message(m) => m.get_description(),
        PathedDescriptor::Enum(e) => e.get_description(),
        PathedDescriptor::Service(s) => s.get_description(),
        PathedDescriptor::Method(m) => m.get_description(),
        PathedDescriptor::Field(f) => f.get_description(),
        PathedDescriptor::EnumValue(e) => e.get_description(),
        _ => None,
    }
}
fn get_option(pathed: &PathedDescriptor) -> i32 {
    match pathed {
        PathedDescriptor::Message(_) => tag::message::OPTIONS,
        PathedDescriptor::Enum(_) => tag::enum_::OPTIONS,
        PathedDescriptor::Service(_) => tag::service::OPTIONS,
        PathedDescriptor::Method(_) => tag::method::OPTIONS,
        PathedDescriptor::Field(_) => tag::field::OPTIONS,
        PathedDescriptor::EnumValue(_) => tag::enum_value::OPTIONS,
        _ => 0,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand;
    #[test]
    fn test_get_lines_offsets() {
        let text = "a\nb\nc";
        let offsets = get_lines_offsets(text);
        assert_eq!(offsets, vec![0, 2, 4]);
    }
    #[test]
    fn test_format_comment() {
        let comment = "This is a long comment that should be split into multiple lines to fit within 100 characters".to_string();
        let spaces = "    ";
        let formatted = format_comment(comment, spaces);
        assert_eq!(formatted.lines().count(), 2);
    }

    fn run_fixture_test(fixture: &str) {
        let mut fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fixtures.push("src/fixtures");
        let path = fixtures.join(fixture);
        let temp_output_dir = std::env::temp_dir().join(rand::random::<u64>().to_string());
        let args = Args {
            files: vec![path],
            includes: vec![fixtures.clone()],
            output: Some(temp_output_dir.clone()),
        };
        entry_point(args).unwrap();
        let expected_path = fixtures.join(fixture).with_extension("expected.proto");
        let actual = std::fs::read_to_string(temp_output_dir.join(fixture)).unwrap();
        std::fs::remove_file(expected_path.clone()).unwrap();
        if expected_path.exists() {
            let expected = std::fs::read_to_string(expected_path).unwrap();
            assert_eq!(expected, actual);
        } else {
            // if it is not present, write expected file from generation
            std::fs::write(expected_path, actual).unwrap();
        }
        std::fs::remove_dir_all(temp_output_dir).unwrap();
    }
    #[test]
    fn test_basic() {
        run_fixture_test("basic.proto");
    }
    #[test]
    fn test_nested() {
        run_fixture_test("nested.proto");
    }
}
