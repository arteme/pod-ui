use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::builders::shorthand::*;
use pod_core::def;
use pod_core::model::*;
use pod_gtk::*;

use pod_mod_pod2::{short, fmt_percent};

pub static PODXT_CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = pod_mod_pod2::module().config()[0].clone();
    let exclude = vec!["drive2", "digiout_show", "eq_enable", "effect_enable"];

    let podxt_controls: HashMap<String, Control> = convert_args!(hashmap!(
        // switches
        "noise_gate_enable" => SwitchControl { cc: 22, addr: 32 + 22, config: SwitchConfig::Midi, ..def() },
        "wah_enable" => SwitchControl { cc: 43, addr: 32 + 43, config: SwitchConfig::Midi, ..def() },
        "stomp_enable" => SwitchControl { cc: 25, addr: 32 + 25, config: SwitchConfig::Midi, ..def() },
        "mod_enable" => SwitchControl { cc: 50, addr: 32 + 25, config: SwitchConfig::Midi, ..def() },
        "mod_position" => SwitchControl { cc: 57, addr: 32 + 57, config: SwitchConfig::Midi, ..def() },
        "delay_enable" => SwitchControl { cc: 28, addr: 32 + 28, config: SwitchConfig::Midi, ..def() },
        "delay_position" => SwitchControl { cc: 87, addr: 32 + 87, config: SwitchConfig::Midi, ..def() },
        "reverb_enable" => SwitchControl { cc: 36, addr: 32 + 36, config: SwitchConfig::Midi, ..def() },
        "reverb_position" => SwitchControl { cc: 41, addr: 32 + 41, config: SwitchConfig::Midi, ..def() },
        "amp_enable" => SwitchControl { cc: 111, addr: 32 + 111, inverted: true,
            config: SwitchConfig::Midi },
        "compressor_enable" => SwitchControl { cc: 26, addr: 32 + 26, config: SwitchConfig::Midi, ..def()  },
        "eq_enable" => SwitchControl { cc: 63, addr: 32 + 63, config: SwitchConfig::Midi, ..def() },

        // preamp
        "amp_select" => Select { cc: 12, addr: 32 + 12 , ..def() },
        "amp_select:no_def" => MidiSelect { cc: 11 }, // TODO: wire me!
        "drive" => RangeControl { cc: 13, addr: 32 + 13, format: fmt_percent!(), ..def() },
        "bass" => RangeControl { cc: 14, addr: 32 + 14, format: fmt_percent!(), ..def() },
        "mid" => RangeControl { cc: 15, addr: 32 + 15, format: fmt_percent!(), ..def() },
        "treble" => RangeControl { cc: 16, addr: 32 + 16, format: fmt_percent!(), ..def() },
        "presence" => RangeControl { cc: 21, addr: 32 + 21, format: fmt_percent!(), ..def() },
        "chan_volume" => RangeControl { cc: 17, addr: 32 + 17, format: fmt_percent!(), ..def() },


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
        flags: DeviceFlags::empty(),

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
