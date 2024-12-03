use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::builders::shorthand::*;
use pod_core::def;
use pod_core::model::*;
use pod_gtk::prelude::*;
use glib::bitflags::bitflags;

use pod_mod_pod2::{short, long, steps, fmt_percent};
use crate::model::*;
use crate::builders::*;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XtPacks: u8 {
        const MS = 0x01;
        const CC = 0x02;
        const FX = 0x04;
        const BX = 0x08;
    }
}

pub static MIC_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
   convert_args!(vec!("57 On Axis", "57 Off Axis", "421 Dynamic", "67 Condenser"))
});

pub static BX_MIC_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!("Tube 47 Close", "Tube 47 Far", "112 Dynamic", "20 Dynamic"))
});

pub static REVERB_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!(
        "Lux Spring", "Std Spring", "King Spring",
        "Small Room", "Tiled Room", "Brite Room",
        "Dark Hall", "Medium Hall", "Large Hall",
        "Rich Chamber", "Chamber", "Cavernous",
        "Slap Plate", "Vintage Plate", "Large Plate"
    ))
});

pub static NOTE_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!(
        "Off","Whole Note",
        "Dotted Half Note", "Half", "Half Note Triplet",
        "Dotted Quarter", "Quarter", "Quarter Note Triplet",
        "Dotted Eighth", "Eighth", "Eighth Note Triplet",
        "Dotted Sixteenth", "Sixteenth", "Sixteenth Note Triplet",
    ))
});

pub static NOTE_DURATION: Lazy<Vec<f32>> = Lazy::new(|| {
    convert_args!(vec!(
        0.0 /* Off */, 1.0 /* Whole Note */,
        4.0/3.0 /* Dotted Half Note */, 2.0 /* Half */, 3.0 /* Half Note Triplet */,
        8.0/3.0 /* Dotted Quarter */, 4.0 /* Quarter */, 6.0 /* Quarter Note Triplet */,
        16.0/3.0 /* Dotted Eighth */, 8.0 /* Eighth */, 12.0 /* Eighth Note Triplet */,
        32.0/3.0 /* Dotted Sixteenth */, 16.0 /* Sixteenth */, 24.0 /* Sixteenth Note Triplet */,
    ))
});

pub static WAH_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!(
        "Vetta Wah", "Fassel", "Weeper", "Chrome", "Chrome Custom",
        "Throaty", "Conductor", "Colorful"
    ))
});

pub static TWEAK_PARAM_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!(
        "Compressor Threshold", "Stomp Drive", "Stomp Gain", "Stomp Tone",
        "Mod Speed", "Mod Depth", "Mod Bass", "Mod Treble", "Mod Mix",
        "Delay Time", "Delay Feedback", "Delay Bass", "Delay Treble",
        "Delay Mix", "Reverb Dwell", "Reverb Tone", "Reverb Mix",
        "???", "Wah Position", "???", "???"
    ))
});

pub static PEDAL_ASSIGN_NAMES: Lazy<Vec<String>> = Lazy::new(|| {
    convert_args!(vec!(
        "1 - Wah \t\t\t 2 - Vol", "1 - Tweak \t\t 2 - Vol", "1 - Wah/Vol \t\t 2 - Tweak"
    ))
});

pub static STOMP_CONFIG: Lazy<Vec<StompConfig>> = Lazy::new(|| {
    convert_args!(vec!(
        /*  0 */ stomp("Facial Fuzz").control("Drive").control("Gain").control("Tone"),
        /*  1 */ stomp("Fuzz Pi").control("Drive").control("Gain").control("Tone"),
        /*  2 */ stomp("Screamer").control("Drive").control("Gain").control("Tone"),
        /*  3 */ stomp("Classic Dist").control("Drive").control("Gain").control("Tone"),
        /*  4 */ stomp("Octave Fuzz").control("Drive").control("Gain").control("Tone"),
        /*  5 */ stomp("Blue Comp").control("Sustain").control("Level"),
        /*  6 */ stomp("Red Comp").control("Sustain").control("Level"),
        /*  7 */ stomp("Vetta Comp").control("Sens").control("Level"),
        /*  8 */ stomp("Auto Swell").control("Ramp").control("Depth"),
        /*  9 */ stomp("Auto Wah").control("Sens").control("Q"),
        /* 10 */ stomp("FX-Killer Z").control("Drive").control("Contour").control("Gain").control("Mid").control("Mid Freq"),
        /* 11 */ stomp("FX-Tube Drive").control("Drive").control("Treble").control("Gain").control("Bass"),
        /* 12 */ stomp("FX-Vetta Juice").control("Amount").control("Level"),
        /* 13 */ stomp("FX-Boost + EQ").control("Gain").control("Bass").control("Treble").control("Mid").control("Mid Freq"),
        /* 14 */ stomp("FX-Blue Comp Treb").control("Level").control("Sustain"),
        /* 15 */ stomp("FX-Dingo-Tron").skip().control("Sens").control("Q"),
        /* 16 */ stomp("FX-Clean Sweep").control("Decay").control("Sens").control("Q"),
        /* 17 */ stomp("FX-Seismik Synth").wave("Wave").skip().skip().control("Mix"),
        /* 18 */ stomp("FX-Double Bass").control("-1OCTG").control("-2OCTG").skip().control("Mix"),
        /* 19 */ stomp("FX-Buzz Wave").wave("Wave").control("Filter").control("Decay").control("Mix"),
        /* 20 */ stomp("FX-Rez Synth").wave("Wave").control("Filter").control("Decay").control("Mix"),
        /* 21 */ stomp("FX-Saturn 5 Ring M").wave("Wave").skip().skip().control("Mix"),
        /* 22 */ stomp("FX-Synth Analog").wave("Wave").control("Filter").control("Decay").control("Mix"),
        /* 23 */ stomp("FX-Synth FX").wave("Wave").control("Filter").control("Decay").control("Mix"),
        /* 24 */ stomp("FX-Synth Harmony").octave("1M335").octave("1457").control("Wave").control("Mix"),
        /* 25 */ stomp("FX-Synth Lead").wave("Wave").control("Filter").control("Decay").control("Mix"),
        /* 26 */ stomp("FX-Synth String").wave("Wave").control("Filter").control("Attack").control("Mix"),
        /* 27 */ stomp("Bass Overdrive").control("Bass").control("Treble").control("Drive").control("Gain"),
        /* 28 */ stomp("Bronze Master").control("Drive").wave("Tone").skip().control("Blend"),
        /* 29 */ stomp("Sub Octaves").control("-1OCTG").control("-2OCTG").skip().control("Mix"),
        /* 30 */ stomp("Bender").control("Position").offset("Heel").offset("Toe").control("Mix"),
    ))
});

pub static MOD_CONFIG: Lazy<Vec<ModConfig>> = Lazy::new(|| {
    convert_args!(vec!(
        /*  0 */ modc("Sine Chorus").control("Depth").control("Bass").control("Treble"),
        /*  1 */ modc("Analog Chorus").control("Depth").control("Bass").control("Treble"),
        /*  2 */ modc("Line 6 Flanger").control("Depth"),
        /*  3 */ modc("Jet Flanger").control("Depth").control("Feedback").control("Manual"),
        /*  4 */ modc("Phaser").control("Feedback"),
        /*  5 */ modc("U-Vibe").control("Depth"),
        /*  6 */ modc("Opto Trem").control("Wave"),
        /*  7 */ modc("Bias Trem").control("Wave"),
        /*  8 */ modc("Rotary Drum + Horn").skip().control("Tone"),
        /*  9 */ modc("Rotary Drum").skip().control("Tone"),
        /* 10 */ modc("Auto Plan").control("Wave"),
        /* 11 */ modc("FX-Analog Square").control("Depth").control("Bass").control("Treble"),
        /* 12 */ modc("FX-Square Chorus").control("Depth").control("Pre-delay").control("Feedback"),
        /* 13 */ modc("FX-Expo Chorus").control("Depth").control("Pre-delay").control("Feedback"),
        /* 14 */ modc("FX-Random Chorus").control("Depth").control("Bass").control("Treble"),
        /* 15 */ modc("FX-Square Flange").control("Depth").control("Pre-delay").control("Feedback"),
        /* 16 */ modc("FX-Expo Flange").control("Depth").control("Pre-delay").control("Feedback"),
        /* 17 */ modc("FX-Lumpy Phase").control("Depth").control("Bass").control("Treble"),
        /* 18 */ modc("FX-Hi-Talk").control("Depth").control("Q"),
        /* 19 */ modc("FX-Sweeper").control("Depth").control("Q").control("Frequency"),
        /* 20 */ modc("FX-POD Purple X").control("Feedback").control("Depth"),
        /* 21 */ modc("FX-Random S/H").control("Depth").control("Q"),
        /* 22 */ modc("FX-Tape Eater").control("Feedback").control("Flutter").control("Distortion"),
        /* 23 */ modc("FX-Warble-Matic").control("Depth").control("Q"),
    ))
});

pub static DELAY_CONFIG: Lazy<Vec<DelayConfig>> = Lazy::new(|| {
    convert_args!(vec!(
        /*  0 */ delay("Analog Delay").control("Feedback").control("Bass").control("Treble"),
        /*  1 */ delay("Analog Delay w/ Mod").control("Feedback").control("Mod Speed").control("Depth"),
        /*  2 */ delay("Tube Echo").control("Feedback").control("Flutter").control("Drive"),
        /*  3 */ delay("Multi-Head").control("Feedback").heads("Heads").control("Flutter"),
        /*  4 */ delay("Sweep Echo").control("Feedback").control("Speed").control("Depth"),
        /*  5 */ delay("Digital Delay").control("Feedback").control("Bass").control("Treble"),
        /*  6 */ delay("Stereo Delay").control("Offset").control("Feedback L").control("Feedback R"),
        /*  7 */ delay("Ping Pong").control("Feedback").control("Offset").control("Spread"),
        /*  8 */ delay("Reverse").control("Feedback"),
        /*  9 */ delay("FX-Echo Platter").control("Feedback").heads("Heads").control("Flutter"),
        /* 10 */ delay("FX-Tape Echo").control("Feedback").control("Bass").control("Treble"),
        /* 11 */ delay("FX-Low Rez").control("Feedback").control("Tone").bits("Bits"),
        /* 12 */ delay("FX-Phaze Echo").control("Feedback").control("Mod Speed").control("Depth"),
        /* 13 */ delay("FX-Bubble Echo").control("Feedback").control("Speed").control("Depth"),
    ))
});

pub fn gate_threshold_from_midi(value: u8) -> u16 {
    (96 - value.min(96)) as u16
}

pub fn gate_threshold_to_midi(value: u16) -> u8 {
    (96 - value.min(96)) as u8
}

fn heel_toe_from_midi(value: u8) -> u16 {
    if value <= 17 { return 0 };
    if value >= 112 { return 48 };

    (value as u16 - 18) / 2 + 1
}

fn heel_toe_to_midi(value: u16) -> u8 {
    if value == 0 { return 0 };
    if value == 48 { return 127 };

    (value as u8 - 1) * 2 + 18
}

pub fn gain_format() -> Format<RangeConfig> {
    Format::Data(FormatData { k: 25.4/127.0, b: -12.8, format: "{val:1.1f} dB".into() })
}

pub fn freq_format(points: Vec<(u8, f64)>) -> Format<RangeConfig> {
    Format::Interpolate(FormatInterpolate { points, format: "{val:1.0f} Hz".into() })
}

pub static PODXT_CONFIG: Lazy<Config> = Lazy::new(|| {
    let podxt_controls: HashMap<String, Control> = convert_args!(hashmap!(
        // switches
        "noise_gate_enable" => SwitchControl { cc: 22, addr: 32 + 22, ..def() },
        "wah_enable" => SwitchControl { cc: 43, addr: 32 + 43, ..def() },
        "stomp_enable" => SwitchControl { cc: 25, addr: 32 + 25, ..def() },
        "mod_enable" => SwitchControl { cc: 50, addr: 32 + 50, ..def() },
        "mod_position" => SwitchControl { cc: 57, addr: 32 + 57, ..def() },
        "delay_enable" => SwitchControl { cc: 28, addr: 32 + 28, ..def() },
        "delay_position" => SwitchControl { cc: 87, addr: 32 + 87, ..def() },
        "reverb_enable" => SwitchControl { cc: 36, addr: 32 + 36, ..def() },
        "reverb_position" => SwitchControl { cc: 41, addr: 32 + 41, ..def() },
        "amp_enable" => SwitchControl { cc: 111, addr: 32 + 111, inverted: true },
        "compressor_enable" => SwitchControl { cc: 26, addr: 32 + 26, ..def()  },
        "eq_enable" => SwitchControl { cc: 63, addr: 32 + 63, ..def() },
        "tuner_enable" => MidiSwitchControl { cc: 69 },
        // preamp
        "amp_select" => Select { cc: 11, addr: 32 + 12 , ..def() },
        "amp_select:no_def" => MidiSelect { cc: 12 }, // TODO: wire me!
        "drive" => RangeControl { cc: 13, addr: 32 + 13, format: fmt_percent!(), ..def() },
        "bass" => RangeControl { cc: 14, addr: 32 + 14, format: fmt_percent!(), ..def() },
        "mid" => RangeControl { cc: 15, addr: 32 + 15, format: fmt_percent!(), ..def() },
        "treble" => RangeControl { cc: 16, addr: 32 + 16, format: fmt_percent!(), ..def() },
        "presence" => RangeControl { cc: 21, addr: 32 + 21, format: fmt_percent!(), ..def() },
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
        // note: despite what the manual says, L6E sends "compressor_threshold" as a value 0..127 (-63..0db)
        "compressor_threshold" => RangeControl { cc: 9, addr: 32 + 9,
            format: Format::Data(FormatData { k: 63.0/127.0, b: -63.0, format: "{val:1.1f} db".into() }),
            config: RangeConfig::Normal,
            ..def() },
        "compressor_gain" => RangeControl { cc: 5, addr: 32 + 5,
            format: Format::Data(FormatData { k: 16.0/127.0, b: 0.0, format: "{val:1.1f} db".into() }),
            config: RangeConfig::Normal,
            ..def() },
        // reverb
        "reverb_select" => Select { cc: 37, addr: 32 + 37, ..def() },
        "reverb_decay" => RangeControl { cc: 38, addr: 32 + 38,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "reverb_tone" => RangeControl { cc: 39, addr: 32 + 39,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "reverb_pre_delay" => RangeControl { cc: 40, addr: 32 + 40,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        "reverb_level" => RangeControl { cc: 18, addr: 32 + 18,
            config: RangeConfig::Normal,
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
        "stomp_param3_offset" => VirtualRangeControl {
            config: RangeConfig::Function { from_midi: heel_toe_from_midi, to_midi: heel_toe_to_midi },
            format: Format::Data(FormatData { k: 1.0, b: -24.0, format: "{val:+}".into() }),
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
        "stomp_param4_offset" => VirtualRangeControl {
            config: RangeConfig::Function { from_midi: heel_toe_from_midi, to_midi: heel_toe_to_midi },
            format: Format::Data(FormatData { k: 1.0, b: -24.0, format: "{val:+}".into() }),
            ..def() },
        "stomp_param5" => RangeControl { cc: 82, addr: 32 + 82,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
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
        "mod_mix" => RangeControl { cc: 56, addr: 32 + 56,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
            ..def() },
        // delay
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
        "delay_mix" => RangeControl { cc: 34, addr: 32 + 34,
            config: RangeConfig::Normal,
            format: fmt_percent!(),
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
        "di_xover" => RangeControl { cc: 45, addr: 32 + 45,
            config: RangeConfig::Normal,
            format: Format::Interpolate(FormatInterpolate {
                points: vec![(0, 0.0), (128, 800.0)],
                format: "{val:1.0f} Hz".into()
            }),
            ..def() },
        // volume pedal
        "vol_level" => RangeControl { cc: 7, addr: 32 + 7, format: fmt_percent!(), ..def() },
        "vol_minimum" => RangeControl { cc: 46, addr: 32 + 46, format: fmt_percent!(), ..def() },
        "vol_pedal_position" => SwitchControl { cc: 47, addr: 32 + 47, ..def() },
        // wah wah
        // note: wah select not in PODxt manual or MIDI reference, but sent by L6E here
        "wah_select" => Select { cc: 91, addr: 32 + 91, ..def() },
        "wah_level" => RangeControl { cc: 4, addr: 32 + 4, format: fmt_percent!(), ..def() },
        // pedals
        "tweak_param_select" => Select { cc: 108, addr: 32 + 108, ..def() },
        "pedal_assign" => Select { cc: 65, addr: 32 + 65, ..def() },
        "pedal_assign_select" => VirtualSelect {},
        // eq
        "eq_1_freq" => RangeControl { cc: 20, addr: 32 + 20,
            format: freq_format(
                vec![(0, 50.0), (128, 690.0)]
            ),
            ..def() },
        "eq_1_gain" => RangeControl { cc: 114, addr: 32 + 114,
            format: gain_format(),
            ..def() },
        "eq_2_freq" => RangeControl { cc: 42, addr: 32 + 42,
            format: freq_format(
                vec![(0, 50.0), (16, 130.0), (48, 450.0), (96, 2850.0), (128, 6050.0)]
            ),
            ..def() },
        "eq_2_gain" => RangeControl { cc: 116, addr: 32 + 116,
            format: gain_format(),
            ..def() },
        "eq_3_freq" => RangeControl { cc: 60, addr: 32 + 60,
            format: freq_format(
                vec![(0, 100.0), (32, 1700.0), (128, 11300.0)]
            ),
            ..def() },
        "eq_3_gain" => RangeControl { cc: 117, addr: 32 + 117,
            format: gain_format(),
            ..def() },
        "eq_4_freq" => RangeControl { cc: 77, addr: 32 + 77,
            format: freq_format(
                vec![(0, 500.0), (32, 1300.0), (64, 2900.0), (128, 9300.0)]
            ),
            ..def() },
        "eq_4_gain" => RangeControl { cc: 119, addr: 32 + 119,
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
        "di:show" => VirtualSelect {},
        "footswitch_mode:show" => VirtualSelect {},
        "xt_packs" => VirtualSelect {},

        "tuner_note" => VirtualSelect {},
        "tuner_offset" => VirtualSelect {},

        // special used for ui wiring only
        "name_change" => Button {},
    ));

    Config {
        name: "PODxt".to_string(),
        family: 0x0003,
        member: 0x0002,

        program_num: 128,
        program_size: 72*2 + 16,
        program_name_addr: 0,
        program_name_length: 16,

        pc_manual_mode: None,
        pc_tuner: None,
        pc_offset: None,

        amp_models: convert_args!(vec!(
            amp("No Amp"),
            amp("Tube Preamp"),
            amp("Line 6 Clean"),
            amp("Line 6 JTS-45"),
            amp("Line 6 Class A"),
            amp("Line 6 Mood"),
            amp("Line 6 Spinal Puppet"),
            amp("Line 6 Chemical X"),
            amp("Line 6 Insane"),
            amp("Line 6 Acoustic 2"),
            amp("Zen Master"),
            amp("Small Tweed"),
            amp("Tweed B-Man"),
            amp("Tiny Tweed"),
            amp("Blackface Lux"),
            amp("Double Verb"),
            amp("Two-Tone"),
            amp("Hiway 100"),
            amp("Plexi 45"),
            amp("Plexi Lead 100"),
            amp("Plexi Jump Lead"),
            amp("Plexi Variac"),
            amp("Brit J-800"),
            amp("Brit JM Pre"),
            amp("Match Chief"),
            amp("Match D-30"),
            amp("Treadplate Dual"),
            amp("Cali Crunch"),
            amp("Jazz Clean"),
            amp("Solo 100"),
            amp("Super O"),
            amp("Class A-15"),
            amp("Class A-30 TB"),
            amp("Line 6 Argo"),
            amp("Line 6 Lunatic"),
            amp("Line 6 Treadplate"),
            amp("Line 6 Variax Acoustic"),
            amp("MS-Bomber Uber"),
            amp("MS-Connor 50"),
            amp("MS-Deity Lead"),
            amp("MS-Deity's Son"),
            amp("MS-Angel P-Ball"),
            amp("MS-Silver J"),
            amp("MS-Brit J-900 Clean"),
            amp("MS-Brit J-900 Dist"),
            amp("MS-Brit J-2000"),
            amp("MS-Diamondplate"),
            amp("MS-Criminal"),
            amp("MS-Line 6 Big Bottom"),
            amp("MS-Line 6 Chunk-Chunk"),
            amp("MS-Line 6 Fuzz"),
            amp("MS-Line 6 Octone"),
            amp("MS-Line 6 Smash"),
            amp("MS-Line 6 Sparkle Clean"),
            amp("MS-Line 6 Throttle"),
            amp("CC-Bomber XTC"),
            amp("CC-Deity Crunch"),
            amp("CC-Blackface Vibro"),
            amp("CC-Double Show"),
            amp("CC-Silverface Bass"),
            amp("CC-Mini Double"),
            amp("CC-Gibtone Expo"),
            amp("CC-Brit Bass"),
            amp("CC-Brit Major"),
            amp("CC-Silver Twelve"),
            amp("CC-Super O Thunder"),
            amp("CC-Line 6 Bayou"),
            amp("CC-Line 6 Crunch"),
            amp("CC-Line 6 Purge"),
            amp("CC-Line 6 Sparkle"),
            amp("CC-Line 6 Super Clean"),
            amp("CC-Line 6 Super Sparkle"),
            amp("CC-Line 6 Twang"),
            amp("BX-Tube Preamp"),
            amp("BX-L6 Classic Jazz"),
            amp("BX-L6 Brit Invader"),
            amp("BX-L6 Super Thor"),
            amp("BX-L6 Frankenstein"),
            amp("BX-L6 Ebony Lux"),
            amp("BX-L6 Doppelganger"),
            amp("BX-L6 Sub Dub"),
            amp("BX-Amp 360"),
            amp("BX-Jaguar"),
            amp("BX-Alchemist"),
            amp("BX-Rock Classic"),
            amp("BX-Flip Top"),
            amp("BX-Adam and Eve"),
            amp("BX-Tweed B-Man"),
            amp("BX-Silverface Bass"),
            amp("BX-Double Show"),
            amp("BX-Eighties"),
            amp("BX-Hiway 100"),
            amp("BX-Hiway 200"),
            amp("BX-British Major"),
            amp("BX-British Bass"),
            amp("BX-California"),
            amp("BX-Jazz Tone"),
            amp("BX-Stadium"),
            amp("BX-Studio Tone"),
            amp("BX-Motor City"),
            amp("BX-Brit Class A100"),
            amp("Citrus D-30"),
            amp("L6 Mod Hi Gain"),
            amp("L6 Boutique #1"),
            amp("Class A-30 Fawn"),
            amp("Brit Gain 18"),
            amp("Brit J-2000 #2"),
        )),
        cab_models: convert_args!(vec!(
            "No Cab",
            "1x6 Super O",
            "1x8 Tweed",
            "1x10 Gibtone",
            "1x10 G-Band",
            "1x12 Line 6",
            "1x12 Tweed",
            "1x12 Blackface",
            "1x12 Class A",
            "2x2 Mini T",
            "2x12 Line 6",
            "2x12 Blackface",
            "2x12 Match",
            "2x12 Jazz",
            "2x12 Class A",
            "4x10 Line 6",
            "4x10 Tweed",
            "4x12 Line 6",
            "4x12 Green 20's",
            "4x12 Green 25's",
            "4x12 Brit T75",
            "4x12 Brit V30's",
            "4x12 Treadplate",
            "1x15 Thunder",
            "2x12 Wishbook",
            "BX-1x12 Boutique",
            "BX-1x12 Motor City",
            "BX-1x15 Flip Top",
            "BX-1x15 Jazz Tone",
            "BX-1x18 Session",
            "BX-1x18 Amp 360",
            "BX-1x18 California",
            "BX-1x18+12 Stadium",
            "BX-2x10 Modern UK",
            "BX-2x15 Double Show",
            "BX-2x15 California",
            "BX-2x15 Class A",
            "BX-4x10 Line 6",
            "BX-4x10 Tweed",
            "BX-4x10 Adam Eve",
            "BX-4x10 Silvercone",
            "BX-4x10 Session",
            "BX-4x12 Hiway",
            "BX-4x12 Green 20's",
            "BX-4x12 Green 25's",
            "BX-4x15 Big Boy",
            "BX-8x10 Classic",
        )),
        effects: vec![], // not used

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

        controls: podxt_controls,
        init_controls: convert_args!(vec!(
            // selects
            "amp_select",
            "cab_select",
            "mic_select",
            "reverb_select",
            "stomp_select",
            "mod_select",
            "mod_note_select",
            "delay_select",
            "delay_note_select",
            "wah_select",
            "tweak_param_select",
            "pedal_assign_select",
            "effect_select",
            // switches
            "noise_gate_enable",
            "wah_enable",
            "stomp_enable",
            "mod_enable",
            "delay_enable",
            "reverb_enable",
            "amp_enable",
            "compressor_enable",
            "eq_enable",
            "tuner_enable",
            // misc
            "stomp_param2_wave", // wonder, why?
            // show signals
            "loop_enable:show",
            "di:show",
            "footswitch_mode:show",
            "xt_packs"
        )),

        // request edit buffer dump after setting 'amp select' CC 11, 'amp select w/o defaults'
        // CC 12, 'effect select' CC 19, 'reverb select' CC 37, 'mod select' CC 58,
        // 'stomp select' CC 75, 'delay select' CC 88, 'wah select' CC 91
        out_cc_edit_buffer_dump_req: vec![ 11, 12, 19, 37, 58, 75, 88, 91 ],

        // request edit buffer dump after receiving all of the above + 'tap tempo' CC 64
        // TODO: 58?
        in_cc_edit_buffer_dump_req: vec![ 11, 12, 19, 37, 64, 75, 88, 91 ],

        flags: DeviceFlags::empty(),
        midi_quirks: MidiQuirks::empty(),
    }
});

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
        name: "PODxt Live".to_string(),
        member: 0x000a,

        controls,
        init_controls,

        ..podxt_config
    }
});
