use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::builders::shorthand::*;
use pod_core::def;
use pod_core::model::*;
//use pod_gtk::prelude::*;

use pod_mod_pod2::{short, long, steps, fmt_percent};
use pod_mod_xt::model::*;
use pod_mod_xt::builders::*;
use pod_mod_xt::config::{gain_format, gate_threshold_from_midi, gate_threshold_to_midi, freq_format};

pub static MIC_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    pod_mod_xt::config::BX_MIC_NAMES.to_vec()
});

pub static NOTE_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    pod_mod_xt::config::NOTE_NAMES.to_vec()
});

pub static TWEAK_PARAM_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!(
        "Stomp Drive", "Stomp Gain", "Stomp Bass", "Stomp Treble",
        "Mod Speed", "Mod Depth", "Mod Pre-delay", "Mod Feedback",
        "Mod Wave", "Mod X-Over", "Mod Mix",
        "Delay/Verb Time", "Delay/Verb Feedback", "Delay/Verb Bass",
        "Delay/Verb Treble", "Delay/Verb X-Over", "Delay/Verb Mix",
        "Wah Position", "???", "???", "???"
    ))
});

pub static PEDAL_ASSIGN_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!(
        "1 - Wah \t\t\t 2 - Vol", "1 - Tweak \t\t 2 - Vol", "1 - Wah/Vol \t\t 2 - Tweak"
    ))
});

trait HasName {
    fn name(&self) -> &String;
    fn with_new_name(&self, new_name: &str) -> Self;
}

impl HasName for StompConfig {
    fn name(&self) -> &String {
        &self.name
    }

    fn with_new_name(&self, new_name: &str) -> Self {
        let mut new = self.clone();
        new.name = new_name.into();
        new
    }
}

impl HasName for ModConfig {
    fn name(&self) -> &String {
        &self.name
    }

    fn with_new_name(&self, new_name: &str) -> Self {
        let mut new = self.clone();
        new.name = new_name.into();
        new
    }
}

impl HasName for DelayConfig {
    fn name(&self) -> &String {
        &self.name
    }

    fn with_new_name(&self, new_name: &str) -> Self {
        let mut new = self.clone();
        new.name = new_name.into();
        new
    }
}

fn pick_config<T: HasName>(name: &str, configs: &[T]) -> T {
    let config = configs.iter()
        .find(|config|
            config.name()
                .strip_prefix("FX-")
                .unwrap_or(config.name().as_str()) == name);
    let Some(config) = config else {
        panic!("Config {:?} not found for picking", name);
    };

    config.with_new_name(name)
}

// Copy stomp configs from PODxt, only in a different order
pub static STOMP_CONFIG: Lazy<Vec<StompConfig>> = Lazy::new(|| {
    let names = vec![
        "Bass Overdrive", "Screamer", "Classic Dist", "Facial Fuzz",
        "Fuzz Pi", "Octave Fuzz", "Bronze Master", "Blue Comp",
        "Red Comp", "Vetta Comp", "Auto Wah", "Dingo-Tron", "Buzz Wave",
        "Seismik Synth", "Rez Synth", "Saturn 5 Ring M", "Synth Analog",
        "Synth FX", "Synth Harmony", "Synth Lead", "Synth String", "Sub Octaves",
    ];

    names.into_iter()
        .map(|name| pick_config(name, &pod_mod_xt::config::STOMP_CONFIG))
        .collect::<Vec<_>>()
});

pub static MOD_CONFIG: Lazy<Vec<ModConfig>> = Lazy::new(|| {
    fn xt(name: &str) -> ModConfig {
      pick_config(name, &pod_mod_xt::config::MOD_CONFIG)
    }

    convert_args!(vec!(
        /*  0 */ modc("Deluxe Chorus").control("Depth").control("Pre-delay").control("Feedback").wave("Wave"),
        /*  1 */ xt("Analog Chorus"),
        /*  2 */ modc("Deluxe Flange").control("Depth").control("Pre-delay").control("Feedback").wave("Wave"),
        /*  3 */ xt("Jet Flanger"),
        /*  4 */ xt("Phaser"),
        /*  5 */ xt("U-Vibe"),
        /*  6 */ xt("Opto Trem"),
        /*  7 */ xt("Bias Trem"),
        /*  8 */ xt("Rotary Drum"),
        /*  9 */ xt("Hi-Talk"),
        /* 10 */ modc("Line 6 Rotor").control("Depth").control("Q"),
        /* 11 */ xt("Random S/H"),
        /* 12 */ xt("Tape Eater"),
    ))
});

pub static DELAY_CONFIG: Lazy<Vec<DelayConfig>> = Lazy::new(|| {
    fn xt(name: &str) -> DelayConfig {
        pick_config(name, &pod_mod_xt::config::DELAY_CONFIG)
    }

    convert_args!(vec!(
        // Delay
        /*  0 */ xt("Analog Delay"),
        /*  1 */ xt("Analog Delay w/ Mod"),
        /*  2 */ xt("Tube Echo"),
        /*  3 */ xt("Multi-Head"),
        /*  4 */ xt("Sweep Echo"),
        /*  5 */ xt("Digital Delay"),
        /*  6 */ xt("Reverse").with_new_name("Reverse Delay"),
        // Reverb
        /*  7 */ delay("Lux Spring").control("Dwell").skip().control("Tone"),
        /*  8 */ delay("Std Spring").control("Dwell").skip().control("Tone"),
        /*  9 */ delay("King Spring").control("Dwell").skip().control("Tone"),
        /* 10 */ delay("Small Room").control("Decay").control("Pre-delay").control("Tone"),
        /* 11 */ delay("Tiled Room").control("Decay").control("Pre-delay").control("Tone"),
        /* 12 */ delay("Brite Room").control("Decay").control("Pre-delay").control("Tone"),
        /* 13 */ delay("Dark Hall").control("Decay").control("Pre-delay").control("Tone"),
        /* 14 */ delay("Medium Hall").control("Decay").control("Pre-delay").control("Tone"),
        /* 15 */ delay("Large Hall").control("Decay").control("Pre-delay").control("Tone"),
        /* 16 */ delay("Rich Chamber").control("Decay").control("Pre-delay").control("Tone"),
        /* 17 */ delay("Chamber").control("Decay").control("Pre-delay").control("Tone"),
        /* 18 */ delay("Cavernous").control("Decay").control("Pre-delay").control("Tone"),
        /* 19 */ delay("Slap Plate").control("Decay").control("Pre-delay").control("Tone"),
        /* 20 */ delay("Vintage Plate").control("Decay").control("Pre-delay").control("Tone"),
        /* 21 */ delay("Large Plate").control("Decay").control("Pre-delay").control("Tone"),
    ))
});


pub static BASS_PODXT_CONFIG: Lazy<Config> = Lazy::new(|| {
    let controls: HashMap<String, Control> = convert_args!(hashmap!(
        // switches
        "noise_gate_enable" => SwitchControl { cc: 22, addr: 32 + 22, ..def() },
        "wah_enable" => SwitchControl { cc: 43, addr: 32 + 43, ..def() },
        "stomp_enable" => SwitchControl { cc: 25, addr: 32 + 25, ..def() },
        "mod_enable" => SwitchControl { cc: 50, addr: 32 + 50, ..def() },
        "mod_position" => SwitchControl { cc: 57, addr: 32 + 57, ..def() },
        "delay_enable" => SwitchControl { cc: 28, addr: 32 + 28, ..def() },
        "amp_enable" => SwitchControl { cc: 111, addr: 32 + 111, inverted: true },
        "compressor_enable" => SwitchControl { cc: 26, addr: 32 + 26, ..def()  },
        "eq_enable" => SwitchControl { cc: 63, addr: 32 + 63, ..def() },
        "eq_position" => SwitchControl { cc: 46, addr: 32 + 46, ..def() },
        "tuner_enable" => MidiSwitchControl { cc: 69 },
        // preamp
        "amp_select" => Select { cc: 11, addr: 32 + 12 , ..def() },
        "amp_select:no_def" => MidiSelect { cc: 12 }, // TODO: wire me!
        "drive" => RangeControl { cc: 13, addr: 32 + 13, format: fmt_percent!(), ..def() },
        "bass" => RangeControl { cc: 14, addr: 32 + 14, format: fmt_percent!(), ..def() },
        "lo_mid" => RangeControl { cc: 15, addr: 32 + 15, format: fmt_percent!(), ..def() },
        "hi_mid" => RangeControl { cc: 16, addr: 32 + 16, format: fmt_percent!(), ..def() },
        "treble" => RangeControl { cc: 21, addr: 32 + 21, format: fmt_percent!(), ..def() },
        "chan_volume" => RangeControl { cc: 17, addr: 32 + 17, format: fmt_percent!(), ..def() },
        "bypass_volume" => RangeControl { cc: 105, addr: 32 + 105, format: fmt_percent!(), ..def() },
        // cab
        "cab_select" => Select { cc: 71, addr: 32 + 71, ..def() },
        "mic_select" => Select { cc: 70, addr: 32 + 70, ..def() },
        "room" => RangeControl { cc: 76, addr: 32 + 76, format: fmt_percent!(), ..def() },
        // effect
        // TODO: aka "effect setup" recalls FX setup stored into the FX banks
        //       not currently shown in the UI, but it exists. Do something with it...
        "effect_select" => Select { cc: 19, addr: 32 + 19, ..def() },
        // noise gate
        // note: despite what the manual says, L6E sends "gate_threshold" as a value 0..96 (0..-96db)
        "gate_threshold" => RangeControl { cc: 23, addr: 32 + 23,
            config: RangeConfig::Function { from_midi: gate_threshold_from_midi, to_midi: gate_threshold_to_midi },
            format: Format::Data(FormatData { k: 1.0, b: -96.0, format: "{val} db".into() }), ..def() },
        "gate_decay" => RangeControl { cc: 24, addr: 32 + 24,format: fmt_percent!(), ..def() }, // can be in milliseconds
        // compressor
        "compressor_amount" => RangeControl { cc: 5, addr: 32 + 5,
            format: fmt_percent!(),
            ..def() },
        // stomp
        "stomp_select" => Select { cc: 75, addr: 32 + 75, ..def() },
        "stomp_param2" => RangeControl { cc: 79, addr: 32 + 79,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "stomp_param2_wave" => VirtualRangeControl {
            config: steps!(0, 16, 32, 48, 64, 80, 96, 112),
            format: Format::Data(FormatData { k: 1.0, b: 1.0, format: "{val:1.0f}".into() }),
            ..def() },
        "stomp_param2_octave" => VirtualRangeControl {
            config: short!(@edge 0, 8),
            format: Format::Labels(convert_args!(vec!(
                "-1 oct", "-maj 6th", "-min 6th", "-4th", "unison", "min 3rd", "maj 3rd", "5th", "1 oct"
            ))),
            ..def() },
        "stomp_param3" => RangeControl { cc: 80, addr: 32 + 80,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "stomp_param3_octave" => VirtualRangeControl {
            config: short!(@edge 0, 8),
            format: Format::Labels(convert_args!(vec!(
                "-1 oct", "-5th", "-4th", "-2nd", "unison", "4th", "5th", "7th", "1 oct"
            ))),
            ..def() },
        // This is not really "wave", but "tone". However, since we already have
        // "wave" type defined for the stomp and the config is exactly the same,
        // we will just pretend it is "wave".
        "stomp_param3_wave" => VirtualRangeControl {
            config: steps!(0, 16, 32, 48, 64, 80, 96, 112),
            format: Format::Data(FormatData { k: 1.0, b: 1.0, format: "{val:1.0f}".into() }),
            ..def() },
        "stomp_param4" => RangeControl { cc: 81, addr: 32 + 81,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "stomp_param5" => RangeControl { cc: 82, addr: 32 + 82,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        // TODO: not used in Bass POD XT?
        "stomp_param6" => RangeControl { cc: 83, addr: 32 + 83,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        // mod
        "mod_select" => Select { cc: 58, addr: 32 + 58, ..def() },
        "mod_speed" => VirtualRangeControl {
            config: long!(0, 16383),
            format: Format::Data(FormatData { k: 14.9/16383.0, b: 0.1, format: "{val:1.2f} Hz".into() }),
            ..def() }, // 0.10 Hz - 15.00 Hz
        "mod_speed:msb" => RangeControl { cc: 29, addr: 32 + 29, ..def() },
        "mod_speed:lsb" => RangeControl { cc: 61, addr: 32 + 61, ..def() },
        "mod_note_select" => Select { cc: 51, addr: 32 + 51, ..def() },
        "mod_param2" => RangeControl { cc: 52, addr: 32 + 52,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "mod_param3" => RangeControl { cc: 53, addr: 32 + 53,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "mod_param4" => RangeControl { cc: 54, addr: 32 + 54,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "mod_param5" => RangeControl { cc: 55, addr: 32 + 55,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "mod_param5_wave" => VirtualRangeControl {
            config: steps!(0, 32, 64),
            format: Format::Labels(convert_args!(vec!(
                "sine", "square", "expon"
            ))),
            ..def() },
        "mod_mix" => RangeControl { cc: 56, addr: 32 + 56,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "mod_xover" => RangeControl { cc: 44, addr: 32 + 44,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        // delay/reverb
        "delay_select" => Select { cc: 88, addr: 32 + 88, ..def() },
        "delay_time" => VirtualRangeControl {
            config: long!(0, 16383),
            format: Format::Data(FormatData { k: 1980.0/16383.0, b: 20.0, format: "{val:1.0f} ms".into() }),
            ..def() }, // 20ms - 2000ms
        "delay_time:msb" => RangeControl { cc: 30, addr: 32 + 30, ..def() },
        "delay_time:lsb" => RangeControl { cc: 62, addr: 32 + 62, ..def() },
        "delay_note_select" => Select { cc: 31, addr: 32 + 31, ..def() },
        "delay_param2" => RangeControl { cc: 33, addr: 32 + 33,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "delay_param3" => RangeControl { cc: 35, addr: 32 + 35,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "delay_param3_heads" => VirtualRangeControl {
            config: short!(@edge 0, 8),
            format: Format::Labels(convert_args!(vec!(
                "12--", "1-3-", "1--4", "-23-", "123-", "12-4", "1-34", "-234", "1234"
            ))),
            ..def() },
        "delay_param4" => RangeControl { cc: 85, addr: 32 + 85,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "delay_param4_bits" => VirtualRangeControl {
            config: short!(@edge 0, 8),
            format: Format::Labels(convert_args!(vec!(
                "12", "11", "10", "9", "8", "7", "6", "5", "4"
            ))),
            ..def() },
        // TODO: not used in Bass POD XT?
        "delay_param5" => RangeControl { cc: 86, addr: 32 + 86,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "delay_mix" => RangeControl { cc: 34, addr: 32 + 34,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "delay_xover" => RangeControl { cc: 45, addr: 32 + 45,
            config: RangeConfig::Normal,
            format: Format::Interpolate(FormatInterpolate {
                points: vec![(0, 0.0), (128, 800.0)],
                format: "{val:1.0f} Hz".into()
            }),
            ..def() },
        // d.i.
        "di_model" => RangeControl { cc: 48, addr: 32 + 48,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "di_delay" => RangeControl { cc: 49, addr: 32 + 49,
            config: RangeConfig::Normal,
            format: Format::Data(FormatData { k: 12.7/127.0, b: 0.0, format: "{val:1.1f} ms".into() }),
            ..def() },
        // volume pedal
        "vol_level" => RangeControl { cc: 7, addr: 32 + 7, format: fmt_percent!(), ..def() },
        "vol_pedal_position" => SwitchControl { cc: 47, addr: 32 + 47, ..def() },
        // wah wah
        "wah_level" => RangeControl { cc: 4, addr: 32 + 4, format: fmt_percent!(), ..def() },
        // pedals
        "tweak_param_select" => Select { cc: 108, addr: 32 + 108, ..def() },
        "pedal_assign" => Select { cc: 65, addr: 32 + 65, ..def() },
        "pedal_assign_select" => VirtualSelect {},
        // eq
        "eq_1_freq" => RangeControl { cc: 20, addr: 32 + 20,
            // TODO: better (more?) points needed. this is often 1Hz off from what L6E shows :(
            //       Maybe the string formatter used does incorrect rounding?
            format: freq_format(
                vec![(0, 50.0), (16, 56.0), (32, 75.0), (48, 106.0), (64, 150.0),
                    (80, 206.0), (96, 275.0), (112, 356.0), (128, 450.0)]
            ),
            ..def() },
        "eq_1_gain" => RangeControl { cc: 114, addr: 32 + 114,
            format: gain_format(),
            ..def() },
        "eq_2_freq" => RangeControl { cc: 32, addr: 32 + 32,
            format: freq_format(
                vec![(0, 20.0), (128, 660.0)]
            ),
            ..def() },
        "eq_2_gain" => RangeControl { cc: 115, addr: 32 + 115,
            format: gain_format(),
            ..def() },
        "eq_3_freq" => RangeControl { cc: 42, addr: 32 + 42,
            format: freq_format(
                vec![(0, 50.0), (64, 370.0), (128, 1010.0)]
            ),
            ..def() },
        "eq_3_gain" => RangeControl { cc: 116, addr: 32 + 116,
            format: gain_format(),
            ..def() },
        "eq_4_freq" => RangeControl { cc: 60, addr: 32 + 60,
            format: freq_format(
                vec![(0, 100.0), (32, 260.0), (96, 900.0), (128, 2500.0)]
            ),
            ..def() },
        "eq_4_gain" => RangeControl { cc: 117, addr: 32 + 117,
            format: gain_format(),
            ..def() },
        "eq_5_freq" => RangeControl { cc: 68, addr: 32 + 68,
            format: freq_format(
                vec![(0, 200.0), (48, 1400.0), (80, 3000.0), (112, 6200.0), (128, 14200.0)]
            ),
            ..def() },
        "eq_5_gain" => RangeControl { cc: 118, addr: 32 + 118,
            format: gain_format(),
            ..def() },
        "eq_6_freq" => RangeControl { cc: 77, addr: 32 + 77,
            // TODO: same as "eq_1_freq", often off by 1Hz
            format: freq_format(
                vec![(0, 500.0), (16, 525.0), (32, 563.0), (48, 734.0), (64, 1000.0),
                    (80, 1359.0), (96, 1813.0), (112, 2395.0), (128, 3000.0)]
            ),
            ..def() },
        "eq_6_gain" => RangeControl { cc: 119, addr: 32 + 119,
            format: gain_format(),
            ..def() },
        // tempo
        "tempo" => VirtualRangeControl {
            config: long!(300, 2400),
            format: Format::Data(FormatData { k: 0.1, b: 0.0, format: "{val:1.1f} bpm".into() }),
            ..def() },
        "tempo:msb" => RangeControl { cc: 89, addr: 32 + 89, ..def() },
        "tempo:lsb" => RangeControl { cc: 90, addr: 32 + 90, ..def() },

        "loop_enable:show" => VirtualSelect {},
        "footswitch_mode:show" => VirtualSelect {},

        "tuner_note" => VirtualSelect {},
        "tuner_offset" => VirtualSelect {},

        // special used for ui wiring only
        "name_change" => Button {},
    ));

    Config {
        name: "Bass PODxt (experimental)".to_string(),
        family: 0x0003,
        member: 0x0006,

        program_num: 64,
        program_size: 72*2 + 16,
        program_name_addr: 0,
        program_name_length: 16,

        pc_manual_mode: Some(0),
        pc_tuner: Some(65),
        pc_offset: Some(1),

        amp_models: convert_args!(vec!(
            amp("No Amp"),
            // All the same as BX-... from PODxt BX pack
            amp("Tube Preamp"),
            amp("Line 6 Classic Jazz"),
            amp("Line 6 Brit Invader"),
            amp("Line 6 Super Thor"),
            amp("Line 6 Frankenstein"),
            amp("Line 6 Ebony Lux"),
            amp("Line 6 Doppelganger"),
            amp("Line 6 Sub Dub"),
            amp("Amp 360"),
            amp("Jaguar"),
            amp("Alchemist"),
            amp("Rock Classic"),
            amp("Flip Top"),
            amp("Adam and Eve"),
            amp("Tweed B-Man"),
            amp("Silverface Bass"),
            amp("Double Show"),
            amp("Eighties"),
            amp("Hiway 100"),
            amp("Hiway 200"),
            amp("British Major"),
            amp("British Bass"),
            amp("California"),
            amp("Jazz Tone"),
            amp("Stadium"),
            amp("Studio Tone"),
            amp("Motor City"),
            amp("Brit Class A100"),
        )),
        cab_models: convert_args!(vec!(
            "No Cab",
            // All the same as BX-... from PODxt BX pack
            "1x12 Boutique",
            "1x12 Motor City",
            "1x15 Flip Top",
            "1x15 Jazz Tone",
            "1x18 Session",
            "1x18 Amp 360",
            "1x18 California",
            "1x18+12 Stadium",
            "2x10 Modern UK",
            "2x15 Double Show",
            "2x15 California",
            "2x15 Class A",
            "4x10 Line 6",
            "4x10 Tweed",
            "4x10 Adam Eve",
            "4x10 Silvercone",
            "4x10 Session",
            "4x12 Hiway",
            "4x12 Green 20's",
            "4x12 Green 25's",
            "4x15 Big Boy",
            "8x10 Classic",
        )),
        effects: vec![], // not used

        toggles: convert_args!(vec!(
            toggle("noise_gate_enable").non_moving(0),
            toggle("volume_enable").moving("vol_pedal_position", 9, 1),
            toggle("wah_enable").non_moving(2),
            toggle("stomp_enable").non_moving(3),
            toggle("mod_enable").moving("mod_position", 10, 4),
            toggle("eq_enable").moving("eq_position", 8, 5),
            toggle("amp_enable").non_moving(6),
            toggle("compressor_enable").non_moving(7),
            toggle("delay_enable").non_moving(11)
        )),

        controls,
        init_controls: convert_args!(vec!(
            // selects
            "amp_select",
            "cab_select",
            "mic_select",
            "effect_select",
            "stomp_select",
            "mod_select",
            "mod_note_select",
            "delay_select",
            "delay_note_select",
            //"wah_select",
            "tweak_param_select",
            "pedal_assign_select",
            // switches
            "noise_gate_enable",
            "wah_enable",
            "stomp_enable",
            "mod_enable",
            "delay_enable",
            "amp_enable",
            "eq_enable",
            "compressor_enable",
            "tuner_enable",
            // misc
            "stomp_param2_wave", // wonder, why?
            // show signals
            "loop_enable:show",
            "footswitch_mode:show",
        )),

        // request edit buffer dump after setting 'amp select' CC 11, 'amp select w/o defaults'
        // CC 12, 'effect select' CC 19, 'mod select' CC 58, 'stomp select' CC 75,
        // 'delay select' CC 88
        out_cc_edit_buffer_dump_req: vec![ 11, 12, 19, 58, 75, 88 ],

        // request edit buffer dump after receiving all of the above + 'tap tempo' CC 64
        // TODO: 58?
        in_cc_edit_buffer_dump_req: vec![ 11, 12, 19, 64, 75, 88 ],

        flags: DeviceFlags::MANUAL_MODE,
        midi_quirks: MidiQuirks::empty(),
    }
});

pub static BASS_PODXT_PRO_CONFIG: Lazy<Config> = Lazy::new(|| {
    let podxt_config = BASS_PODXT_CONFIG.clone();

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
        name: "BassPODxt Pro (experimental)".to_string(),
        member: 0x0007,

        toggles,
        controls,

        ..podxt_config
    }
});

pub static BASS_PODXT_LIVE_CONFIG: Lazy<Config> = Lazy::new(|| {
    let podxt_config = BASS_PODXT_CONFIG.clone();

    let podxt_live_controls: HashMap<String, Control> = convert_args!(hashmap!(
        "footswitch_mode" => SwitchControl { cc: 84, addr: 32 + 84, ..def() }, // 0: amp, 1: comp
    ));
    let controls = podxt_config.controls.into_iter()
        .chain(podxt_live_controls)
        .collect();

    let podxt_live_init_controls: Vec<String> = convert_args!(vec!(
        "footswitch_mode"
    ));
    let init_controls = podxt_config.init_controls.into_iter()
        .chain(podxt_live_init_controls)
        .collect();


    Config {
        name: "BassPODxt Live (experimental)".to_string(),
        member: 0x000b,

        controls,
        init_controls,

        ..podxt_config
    }
});