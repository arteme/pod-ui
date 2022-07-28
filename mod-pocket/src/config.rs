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
        manual: false,

       controls,

       ..pod2_config
    }
});