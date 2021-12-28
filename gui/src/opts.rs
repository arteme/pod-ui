pub use clap::Clap;

#[derive(Clap)]
pub struct Opts {
    #[clap(short, long)]
    pub input: Option<String>,

    #[clap(short, long)]
    pub output: Option<String>,
}
