pub use clap::Parser;

#[derive(Parser)]
pub struct Opts {
    #[clap(short, long)]
    pub input: Option<String>,

    #[clap(short, long)]
    pub output: Option<String>,
}
