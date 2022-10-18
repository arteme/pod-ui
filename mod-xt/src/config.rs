use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::builders::shorthand::*;
use pod_core::model::*;
use pod_gtk::*;

pub static PODXT_CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = pod_mod_pod2::module().config()[0].clone();
    let exclude = vec!["drive2", "digiout_show", "eq_enable", "effect_enable"];

    let podxt_controls: HashMap<String, Control> = convert_args!(hashmap!(
        // drive => cc: 13
        // bass => cc: 14
        // mid => cc: 15
        // treble => cc: 16
        // presence => cc: 21
        // chan volume => cc: 17

        // noise_gate_enable => cc: 22
        "wah_enable" => MidiSwitchControl { cc: 43 },
        "stomp_enable" => MidiSwitchControl { cc: 25 },
        "mod_enable" => MidiSwitchControl { cc: 50 },
        "mod_position" => MidiSwitchControl { cc: 57 },
        "reverb_position" => MidiSwitchControl { cc: 57 },
        // delay_enable => cc: 28
        "delay_position" => MidiSwitchControl { cc: 87 },
        // reverb_enable => cc: 36
        "reverb_position" => MidiSwitchControl { cc: 41 },
        "amp_enable" => MidiSwitchControl { cc: 111 },
        "compressor_enable" => MidiSwitchControl { cc: 26 },
        "eq_enable" => MidiSwitchControl { cc: 63 },

        "loop_enable:show" => VirtualSelect {}
    ));
    let controls = pod2_config.controls.into_iter()
        .filter(|(k, _)| !exclude.contains(&k.as_str()))
        .chain(podxt_controls)
        .collect();

    let pocket_pod_init_controls = convert_args!(vec!(
        "wah_enable",
        "loop_enable:show"
    ));
    let init_controls = pod2_config.init_controls.into_iter()
        .filter(|v| !exclude.contains(&v.as_str()))
        .chain(pocket_pod_init_controls)
        .collect();

    Config {
        name: "PODxt".to_string(),
        family: 0x0003,
        member: 0x0002,

        program_num: 128,
        flags: DeviceFlags::MODIFIED_BUFFER_PC_AND_EDIT_BUFFER,

        toggles: convert_args!(vec!(
            toggle("noise_gate_enable").non_moving(0),
            toggle("volume_enable").moving("vol_pedal_position", 10, 1),
            toggle("wah_enable").non_moving(2),
            toggle("stomp_enable").non_moving(3),
            toggle("mod_enable").moving("mod_position", 11, 4),
            toggle("delay_enable").moving("delay_position", 12, 5),
            toggle("reverb_enable").moving("reverb_position", 13, 6),
            toggle("amp_enable").non_moving(7),
            toggle("compressor_enable").non_moving(8),
            toggle("eq_enable").non_moving(9),
        )),

        controls,
        init_controls,

        program_size: 72*2 + 16,
        program_name_addr: 0,
        program_name_length: 16,

        ..pod2_config
    }
});

// TODO: is not recognized
pub static PODXT_PRO_CONFIG: Lazy<Config> = Lazy::new(|| {
    let podxt_config = PODXT_CONFIG.clone();

    let podxt_pro_controls: HashMap<String, Control> = convert_args!(hashmap!(
        "loop_enable" => MidiSwitchControl { cc: 107 },
    ));
    let controls = podxt_config.controls.into_iter()
        .chain(podxt_pro_controls)
        .collect();

    let podxt_pro_toggles = convert_args!(vec!(
        toggle("loop_enable").non_moving(14)
    ));
    let toggles = podxt_config.toggles.into_iter()
        .chain(podxt_pro_toggles)
        .collect();

    Config {
        name: "PODxt Pro".to_string(),
        member: 0x0005,

        toggles,
        controls,

        ..podxt_config
    }
});

pub static PODXT_LIVE_CONFIG: Lazy<Config> = Lazy::new(|| {
    let podxt_config = PODXT_CONFIG.clone();

    Config {
        name: "PODxt Live".to_string(),
        member: 0x000a,

        ..podxt_config
    }
});
