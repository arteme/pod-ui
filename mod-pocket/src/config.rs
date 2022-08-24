use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::model::*;
use pod_gtk::*;
use pod_mod_pod2::{amps, short, def};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = pod_mod_pod2::module().config()[0].clone();
    let exclude = vec!["digiout_show", "vol_pedal_position"];

    let pocket_pod_controls: HashMap<String, Control> = convert_args!(hashmap!(
        // wah_enable is a MIDI-only control and is not present in the program data
        "wah_enable" => MidiSwitchControl { cc: 43 },
    ));
    let controls = pod2_config.controls.into_iter()
        .filter(|(k, v)| !exclude.contains(&k.as_str()))
        .chain(pocket_pod_controls)
        .collect();

    let pocket_pod_init_controls = convert_args!(vec!(
        "wah_enable"
    ));
    let init_controls = pod2_config.init_controls.into_iter()
        .filter(|v| !exclude.contains(&v.as_str()))
        .chain(pocket_pod_init_controls)
        .collect();

    Config {
        name: "Pocket POD".to_string(),
        family: 0x0000,
        member: 0x0600,

        program_num: 124,
        flags: DeviceFlags::MODIFIED_BUFFER_PC_AND_EDIT_BUFFER,

        controls,
        init_controls,

        ..pod2_config
    }
});