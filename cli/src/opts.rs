pub use clap::Clap;

#[derive(Clap)]
pub struct Opts {
    #[clap(short, long)]
    pub input: Option<usize>,

    #[clap(short, long)]
    pub output: Option<usize>,

    #[clap(short, long)]
    pub r#loop: bool
}