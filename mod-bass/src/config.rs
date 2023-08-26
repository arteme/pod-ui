use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::model::*;
use pod_gtk::*;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = pod_mod_pod2::module().config()[0].clone();
    /*
    let exclude = vec!["digiout_show", "vol_pedal_position"];

    let pocket_pod_controls: HashMap<String, Control> = convert_args!(hashmap!(
        // wah_enable is a MIDI-only control and is not present in the program data
        "wah_enable" => MidiSwitchControl { cc: 43 },
    ));
    let controls = pod2_config.controls.into_iter()
        .filter(|(k, _)| !exclude.contains(&k.as_str()))
        .chain(pocket_pod_controls)
        .collect();

    let pocket_pod_init_controls = convert_args!(vec!(
        "wah_enable"
    ));
    let init_controls = pod2_config.init_controls.into_iter()
        .filter(|v| !exclude.contains(&v.as_str()))
        .chain(pocket_pod_init_controls)
        .collect();
     */

    Config {
        name: "Bass POD (experimental)".to_string(),
        family: 0x0002,
        member: 0x0000,

        /*
        controls,
        init_controls,
        toggles: vec![], // PocketPOD doesn't use dynamic toggle positioning

        midi_quirks: MIDI_QUIRKS,
         */

        ..pod2_config
    }
});

pub static PRO_CONFIG: Lazy<Config> = Lazy::new(|| {
    let bass_pod_config = CONFIG.clone();

    Config {
        name: "Bass POD Pro (experimental)".to_string(),
        family: 0x0002,
        member: 0x0001,

        // TODO: something here?

        ..bass_pod_config
    }
});
