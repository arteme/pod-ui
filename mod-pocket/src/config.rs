use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::model::*;
use pod_gtk::*;
use pod_mod_pod2::{amps, short, def};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = pod_mod_pod2::module().config()[0].clone();

    let pro_controls: HashMap<String, Control> = convert_args!(hashmap!(
        "out_gain" => RangeControl { cc: 9, addr: 35,
            config: short!(),
            format: Format::Data(FormatData { k: 1.0/12.0, b: 0.0, format: "{val} db".into()}),
            ..def!()
        }
    ));
    let controls = pod2_config.controls.into_iter()
        .chain(pro_controls)
        .collect();

    Config {
        name: "Pocket POD".to_string(),
        family: 0x0000,
        member: 0x0600,

        program_num: 124,

        amp_models: amps!(
           "Tube Preamp" +p,
           "Line 6 Clean" +p +b,
           "Line 6 Crunch" +p +b,
           "Line 6 Drive" +p +b,
           "Line 6 Layer" +p +b +d2,
           "Small Tweed" +p,
           "Tweed Blues" +p,
           "Black Panel" +p,
           "Modern Class A" +p,
           "Brit Class A" +p,
           "Brit Blues" +p +b,
           "Brit Classic" +p,
           "Brit Hi Gain" +p,
           "Rectified" +p,
           "Modern Hi Gain" +p,
           "Fuzz Box" +p,
           "Jazz Clean" +p +b,
           "Boutique #1" +p,
           "Boutique #2" +p,
           "Brit Class A #2" +p,
           "Brit Class A #3" +p,
           "Small Tweed #2" +p,
           "Black Panel #2" +p +b,
           "Boutique #3" +p,
           "California Crunch #1" +p +b,
           "California Crunch #2" +p,
           "Rectified #2" +p,
           "Modern Hi Gain #2" +p,
           "Line 6 Twang" +p,
           "Line 6 Crunch #2" +p,
           "Line 6 Blues" +p,
           "Line 6 Insane" +p,
       ),

       controls,

       ..pod2_config
    }
});