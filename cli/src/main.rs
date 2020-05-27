pub mod opts;

use pod_core::pod;


use anyhow::Context;
use std::io::{stdin, Read};
use log::*;

use opts::*;

fn main() -> Result<(), anyhow::Error> {
    simple_logger::init()?;

    let opts: Opts = Opts::parse();
    let mut midi = pod::Midi::new(opts.input, opts.output)
        .context("Failed to initialize MIDI")?;

    let pods = pod::PodConfigs::new()?;
    info!("Loaded {} POD configs", pods.count());

    pods.detect(&mut midi)?;

    /*
    let mut buffer: [u8; 1] = [0];
    stdin().read(&mut buffer)?;
     */

    Ok(())
}
