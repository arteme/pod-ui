use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::def;
use pod_core::builders::shorthand::*;
use pod_core::model::*;
use pod_mod_pod2::{fx, fmt_percent, short, gate_threshold_from_midi, gate_threshold_to_midi};
use pod_gtk::*;

static CAB_MAPPING: Lazy<Vec<u8>> = Lazy::new(|| {
   vec![ 11, 9, 8, 0, 1, 3, 2, 6, 5, 4, 12, 13, 15, 14, 10 ]
});

fn cab_from_midi(value: u8) -> u16 {
    for (i, v) in CAB_MAPPING.iter().enumerate() {
        if value == *v { return i as u16 }
    }
    0
}

fn cab_to_midi(value: u16) -> u8 {
    if value as usize > CAB_MAPPING.len() {
        return 0
    }
    CAB_MAPPING[value as usize]
}



pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = pod_mod_pod2::module().config()[0].clone();

    Config {
        name: "Bass POD (experimental)".to_string(),
        family: 0x0002,
        member: 0x0000,

        program_size: 80,
        program_num: 36,

        amp_models: convert_args!(vec!(
            amp("Tube Preamp"),
            amp("Session"),
            amp("California"),
            amp("Jazz Tone"),
            amp("Adam & Eve"),
            amp("Eighties"),
            amp("Stadium"),
            amp("Amp 360"),
            amp("Rock Classic"),
            amp("Brit Major"),
            amp("Brit Super"),
            amp("Silver Panel"),
            amp("Brit Classic A"),
            amp("Motor City"),
            amp("Flip Top"),
            amp("Sub Dub"),
        )),
        cab_models: convert_args!(vec!(
            "8х10 '79 Ampeg SVT", // 11
            "4x10 Eden David", // 9
            "4x10 SWR Goliath", // 8
            "4x10 Hartke", // 0
            "1x12 60s Versatone Pan-O-Flex", // 1
            "4x12 '67 Marshall with Celestions", // 3
            "1x15 Ampeg B-15", // 2
            "1x15 Polytone Mini-Brute", // 6
            "2x15 Mesa Boogie", // 5
            "2x15 Fender Bassman with JBLs", // 4
            "4x15 '76 Marshal", // 12
            "1x18 SWR Big Ben", // 13
            "1x18 Acoustic 360", // 15
            "1x18+1x12 Sunn Coliseum 8028", // 14
            "No Cabinet", // 10
        )),
        // TODO: figure out effect_tweak control for each one
        effects: vec![
            fx!("Bypass", // 0
                c=10 + ""
            ),
            fx!("Octave Down", // 1
                c=11 + "" + convert_args!(vec!("fx_1_mix"))
            ),
            fx!("Analog Chorus", // 2
                c=9 + "" + convert_args!(vec!("fx_2_speed", "fx_2_depth"))
            ),
            fx!("Danish Chorus", // 3
                c=8 + "" + convert_args!(vec!("fx_3_speed", "fx_3_intensity", "fx_3_width"))
            ),
            fx!("Orange Phase", // 4
                c=0 + "" + convert_args!(vec!("fx_4_speed"))
            ),
            fx!("Gray Flanger", // 5
                c=1 + "" + convert_args!(vec!("fx_5_speed", "fx_5_regen", "fx_5_width"))
            ),
            fx!("Tron Down", // 6
                c=3 + "" + convert_args!(vec!("fx_6_peak"))
            ),
            fx!("Tron Up", // 7
                c=2 + "" + convert_args!(vec!("fx_7_peak"))
            ),
            fx!("Sample and Hold", // 8
                c=6 + "" + convert_args!(vec!("fx_8_speed"))
            ),
            fx!("S/H + Flanger", // 9
                c=7 + "" + convert_args!(vec!("fx_8_speed", "fx_5_speed", "fx_5_regen", "fx_5_width"))
            ),
            fx!("S/H + Driver", // 10
                c=5 + "" + convert_args!(vec!("fx_8_peed", "fx_12_distortion"))
            ),
            fx!("Bass Synth", // 11
                c=4 + "" + convert_args!(vec!("fx_11_decay", "fx_11_dry_level", "fx_11_low_pass_level", "fx_10_high_pass_level"))
            ),
            fx!("Danish Driver", // 12
                c=12 + "" + convert_args!(vec!("fx_12_distortion"))
            ),
            fx!("Large Pie", // 13
                c=13 + "" + convert_args!(vec!("fx_13_distortion"))
            ),
            fx!("Pig Foot", // 14
                c=15 + "" + convert_args!(vec!("fx_14_distortion"))
            ),
            fx!("Rodent", // 15
                c=14 + "" + convert_args!(vec!("fx_15_distortion", "fx_15_filter"))
            ),
        ],


        controls: convert_args!(hashmap!(
            // switches
            "noise_gate_enable" => SwitchControl { cc: 22, addr: 0, ..def() },
            "effect_enable" => SwitchControl { cc: 60, addr: 52, ..def() }, // trem/rotary speaker/chorus/flanger
            "wah_enable" => MidiSwitchControl { cc: 43 }, // MIDI-only, not part of program data
            "apply_fx_to_di" => SwitchControl { cc: 64, addr: 2, ..def() },
            // preamp
            "amp_select" => Select { cc: 12, addr: 3, ..def() },
            "drive" => RangeControl { cc: 13, addr: 4, config: short!(),
                format: fmt_percent!(), ..def() },
            "bass" => RangeControl { cc: 14, addr: 6, config: short!(),
                format: fmt_percent!(), ..def() },
            "mid" => RangeControl { cc: 15, addr: 7, config: short!(),
                format: fmt_percent!(), ..def() },
            "mid_sweep" => RangeControl { cc: 28, addr: 12, config: short!(),
                format: fmt_percent!(), ..def() },
            "treble" => RangeControl { cc: 16, addr: 8, config: short!(),
                format: fmt_percent!(), ..def() },
            "chan_volume" => RangeControl { cc: 17, addr: 9, config: short!(),
                format: fmt_percent!(), ..def() },
            // cabinet sim
            "cab_select" => Select { cc: 71, addr: 34,
                //TODO
                //config: RangeConfig::Function { from_midi: cab_from_midi, to_midi: cab_to_midi },
                ..def() },
            "air" => RangeControl { cc: 72, addr: 32,config: short!(),
                 format: fmt_percent!(), ..def() },

            // parametric eq
            "eq_freq" => RangeControl { cc: 25, addr: 13, config: short!(),
                format: fmt_percent!(), // TODO: 30Hz - 8kHz
                ..def()
            },
            "eq_shape" => RangeControl { cc: 26, addr: 14, config: short!(),
                format: fmt_percent!(), // TODO: ???
                ..def()
            },
            "eq_gain" => RangeControl { cc: 27, addr: 15, config: short!(),
                format: fmt_percent!(), // TODO: -inf - +12 dB
                ..def()
            },
            // other
            "fx_lo_cut" => RangeControl { cc: 21, addr: 51, config: short!(),
                format: fmt_percent!(), // TODO: off - 1 kHz
                ..def()
            },
            // noise gate (same as pod 2.0)
            "gate_threshold" => RangeControl { cc: 23, addr: 16,
                config: RangeConfig::Function { from_midi: gate_threshold_from_midi, to_midi: gate_threshold_to_midi },
                format: Format::Data(FormatData { k: 1.0, b: -96.0, format: "{val} db".into() }), ..def() }, // todo: -96 db .. 0 db
            "gate_decay" => RangeControl { cc: 24, addr: 17, config: short!(),
                 format: fmt_percent!(), ..def() }, // todo: 8.1 msec .. 159 msec
            // compressor
            "compression_ratio" => RangeControl { cc: 42, addr: 25, config: short!(0,5),
                format: Format::Labels(convert_args!(vec!(
                    "off", "1.4:1", "2:1", "3:1", "6:1", "inf:1"
                ))),
                ..def() }, // off, 1.4:1, 2:1, 3:1, 6:1, inf:1
            // TODO: SoundDiver sends "Compressor Thresh" here. What do we do with addr 26?
            "compression_threshold" => RangeControl { cc: 18, addr: 11, config: short!(),
                 format: fmt_percent!(), ..def() },
            "compression_attack" => RangeControl { cc: 51, addr: 28, format: fmt_percent!(), ..def() },
            "compression_decay" => RangeControl { cc: 63, addr: 27, format: fmt_percent!(), ..def() },

            // effect
            "effect_select:raw" => Select { cc: 19, addr: 49 }, // 0 - bypass, 1..15 - effects
            "effect_select" => VirtualSelect {}, // select control for the ui
            "effect_tweak" => RangeControl { cc: 1, addr: 50, config: short!(),
                 format: fmt_percent!(), ..def() },
            // wah pedal (same as pod 2.0)
            "wah_level" => RangeControl { cc: 4, addr: 18,format: fmt_percent!(), ..def() },
            "wah_bottom_freq" => RangeControl { cc: 44, addr: 19,format: fmt_percent!(), ..def() },
            "wah_top_freq" => RangeControl { cc: 45, addr: 20,format: fmt_percent!(), ..def() },
            // volume pedal (same as pod 2.0)
            "vol_level" => RangeControl { cc: 7, addr: 22, format: fmt_percent!(), ..def() },
            "vol_minimum" => RangeControl { cc: 46, addr: 23, format: fmt_percent!(), ..def() },
            "vol_pedal_position" => SwitchControl { cc: 47, addr: 24, ..def() },
            // fx ...
            "fx_1_mix" => RangeControl { cc: 29, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_2_speed" => RangeControl { cc: 30, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_2_depth" => RangeControl { cc: 31, addr: 54, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_3_speed" => RangeControl { cc: 34, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_3_intensity" => RangeControl { cc: 32, addr: 54, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_3_width" => RangeControl { cc: 33, addr: 55, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_4_speed" => RangeControl { cc: 35, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_5_speed" => RangeControl { cc: 37, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_5_regen" => RangeControl { cc: 38, addr: 54, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_5_width" => RangeControl { cc: 36, addr: 55, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_6_peak" => RangeControl { cc: 39, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_7_peak" => RangeControl { cc: 40, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_8_speed" => RangeControl { cc: 41, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_11_decay" => RangeControl { cc: 52, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_11_dry_level" => RangeControl { cc: 48, addr: 54, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_11_low_pass_level" => RangeControl { cc: 49, addr: 55, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_11_high_pass_level" => RangeControl { cc: 50, addr: 56, config: short!(),
                format: fmt_percent!(), ..def() },
            "fx_12_distortion" => RangeControl { cc: 55, addr: 53, format: fmt_percent!(), ..def() },
            "fx_13_distortion" => RangeControl { cc: 56, addr: 53, format: fmt_percent!(), ..def() },
            "fx_14_distortion" => RangeControl { cc: 57, addr: 53, format: fmt_percent!(), ..def() },
            "fx_15_distortion" => RangeControl { cc: 58, addr: 53, format: fmt_percent!(), ..def() },
            "fx_15_filter" => RangeControl { cc: 59, addr: 53, config: short!(),
                format: fmt_percent!(), ..def() },

            // special used for ui wiring only
            "name_change" => Button {},
            "digiout_show" => VirtualSelect {}
        )),
        init_controls: convert_args!(vec!(
            "noise_gate_enable",
            "effect_enable",
            "wah_enable",
            "amp_select",
            "effect_select",
            "digiout_show"
            )),

        toggles: convert_args!(vec!(
            toggle("noise_gate_enable").non_moving(0),
            toggle("volume_enable").moving("vol_pedal_position", 3, 1),
            toggle("amp_enable").non_moving(2),
            toggle("effect_enable").non_moving(4),
            toggle("wah_enable").non_moving(5),
        )),

        program_name_addr: 64,
        program_name_length: 16,

        ..pod2_config
    }
});

pub static PRO_CONFIG: Lazy<Config> = Lazy::new(|| {
    let bass_pod_config = CONFIG.clone();

    fn fmt_percent_di_mix(c: &RangeConfig, v: f64) -> String {
        let (from, to) = c.bounds();

        let n = ((to - from) / 2.0).floor();
        let p = ((to - from) / 2.0).ceil();

        let v1 = if v <= n { v - n } else { v - p };
        let n1 = v1 * 100.0 / n;
        let n2 = 100.0 - n1;
        format!("{:1.0}%/{:1.0}%", n1, n2)
    }

    let pro_controls: HashMap<String, Control> = convert_args!(hashmap!(
        "digiout_gain" => RangeControl { cc: 9, addr: 35,
            config: short!(),
            format: Format::Data(FormatData { k: 12.0/63.0, b: 0.0, format: "{val:1.2f} db".into()}),
            ..def()
        },
        "di_time_align" => RangeControl { cc: 74, addr: 34,
            format: Format::Data(FormatData { k: 8.0/127.0, b: 0.0, format: "{val:1.2f} ms".into()}),
            ..def()
        },
        "di_mix" => RangeControl { cc: 75, addr: 35,
            format: Format::Callback(fmt_percent_di_mix),
            ..def()
        },
    ));
    let controls = bass_pod_config.controls.into_iter()
        .chain(pro_controls)
        .collect();

    Config {
        name: "Bass POD Pro (experimental)".to_string(),
        family: 0x0002,
        member: 0x0001,

        controls,

        ..bass_pod_config
    }
});
