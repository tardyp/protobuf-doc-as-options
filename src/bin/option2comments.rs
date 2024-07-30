use clap::Parser;
use protox_doc::option2comments::{entry_point, Args};

fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    entry_point(Args::parse())
}
