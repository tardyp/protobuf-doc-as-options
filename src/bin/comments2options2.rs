
use std::{error::Error, fs, io::Write};
use protox_doc::comments2option::{comments2option, DescriptionIds};

fn main() -> Result<(), Box<dyn Error>> {
    let ids = DescriptionIds {
        file: Some(1000),
        message: Some(1000),
        enum_: Some(1000),
        service: Some(1000),
        method: Some(1000),
        field: Some(1000),
        enum_value: Some(1000),
        extension: Some(1000),
        oneof: Some(1000),
    };
    // get filename and output from command line args
    let mut args = std::env::args().skip(1);
    let filename = args.next().expect("filename not provided");
    let output = args.next().expect("output not provided");
    let fds = fs::read(&filename)?;
    let res = comments2option(&fds, &ids);
    std::fs::File::create(&output)?.write_all(&res)?;
    Ok(())
}
