use miette::IntoDiagnostic;
use protobuf::{self, descriptor::FileDescriptorSet, Message};

pub fn comments2option(res: &[u8]) -> miette::Result<Vec<u8>> {
    let mut res = FileDescriptorSet::parse_from_bytes(res).into_diagnostic()?;
    for file in &mut res.file {
        file.source_code_info.clear();
    }
    Ok(res.write_to_bytes().into_diagnostic()?)
}
