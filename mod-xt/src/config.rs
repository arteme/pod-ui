use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::builders::shorthand::*;
use pod_core::def;
use pod_core::model::*;
use pod_gtk::prelude::*;
use glib::bitflags::bitflags;

use pod_mod_pod2::{short, fmt_percent};
use crate::model::*;
use crate::builders::*;

bitflags! {
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
        "Lux Spring", "Std Sping", "King Spring",
        "Small Room", "Tiled Room", "Brite Room",
        "Dark Hall", "Medium Hall", "Large Hall",
        "Rich Chamber", "Chamber", "Cavernous",
        "Slap Plate", "Vintage Plate", "Large Plate"
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
        /* 10 */ stomp("FX-Killer Z").control("Drive").control("Contour").control("Gain").control("Mid").control("Midfreq"),
        /* 11 */ stomp("FX-Tube Drive").control("Drive").control("Treble").control("Gain").control("Bass"),
        /* 12 */ stomp("FX-Vetta Juice").control("Amount").control("Level"),
        /* 13 */ stomp("FX-Boost + EQ").control("Gain").control("Bass").control("Treble").control("Mid").control("Midfreq"),
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
        /* 28 */ stomp("Bronze Master").control("Drive").control("Tone").skip().control("Blend"),
        /* 29 */ stomp("Sub Octaves").control("-1OCTG").control("-2OCTG").skip().control("Mix"),
        /* 30 */ stomp("Bender").control("Posi").offset("Heel").offset("Toe").control("Mix"),
    ))
});

fn gate_threshold_from_midi(value: u8) -> u16 {
    (96 - value.min(96)) as u16
}

fn gate_threshold_to_midi(value: u16) -> u8 {
    (96 - value.min(96)) as u8
}

fn delay_time_from_buffer(value: u32) -> u16 {
    (value / 6).min(0xffff) as u16
}

fn delay_time_to_buffer(value: u16) -> u32 {
    (value as u32) * 6
}


pub static PODXT_CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = pod_mod_pod2::module().config()[0].clone();
    let exclude = vec![
        "drive2", "digiout_show", "eq_enable", "effect_enable",
        "reverb_type", "reverb_diffusion", "reverb_density"
    ];

    let podxt_controls: HashMap<String, Control> = convert_args!(hashmap!(
        // switches
        "noise_gate_enable" => SwitchControl { cc: 22, addr: 32 + 22, buffer_config: BufferConfig::Midi, ..def() },
        "wah_enable" => SwitchControl { cc: 43, addr: 32 + 43, buffer_config: BufferConfig::Midi, ..def() },
        "stomp_enable" => SwitchControl { cc: 25, addr: 32 + 25, buffer_config: BufferConfig::Midi, ..def() },
        "mod_enable" => SwitchControl { cc: 50, addr: 32 + 25, buffer_config: BufferConfig::Midi, ..def() },
        "mod_position" => SwitchControl { cc: 57, addr: 32 + 57, buffer_config: BufferConfig::Midi, ..def() },
        "delay_enable" => SwitchControl { cc: 28, addr: 32 + 28, buffer_config: BufferConfig::Midi, ..def() },
        "delay_position" => SwitchControl { cc: 87, addr: 32 + 87, buffer_config: BufferConfig::Midi, ..def() },
        "reverb_enable" => SwitchControl { cc: 36, addr: 32 + 36, buffer_config: BufferConfig::Midi, ..def() },
        "reverb_position" => SwitchControl { cc: 41, addr: 32 + 41, buffer_config: BufferConfig::Midi, ..def() },
        "amp_enable" => SwitchControl { cc: 111, addr: 32 + 111, inverted: true,
            buffer_config: BufferConfig::Midi },
        "compressor_enable" => SwitchControl { cc: 26, addr: 32 + 26, buffer_config: BufferConfig::Midi, ..def()  },
        "eq_enable" => SwitchControl { cc: 63, addr: 32 + 63, buffer_config: BufferConfig::Midi, ..def() },
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
        // noise gate
        // note: despite what the manual says, L6E sends "gate_threshold" as a value 0..96 (0..-96db)
        "gate_threshold" => RangeControl { cc: 23, addr: 32 + 23,
            config: RangeConfig::Function { from_midi: gate_threshold_from_midi, to_midi: gate_threshold_to_midi, buffer_config: BufferConfig::Midi },
            format: Format::Data(FormatData { k: 1.0, b: -96.0, format: "{val} db".into() }), ..def() },
        "gate_decay" => RangeControl { cc: 24, addr: 32 + 24,format: fmt_percent!(), ..def() }, // can be in milliseconds
        // compressor
        // note: despite what the manual says, L6E sends "compressor_threshold" as a value 0..127 (-63..0db)
        "compressor_threshold" => RangeControl { cc: 9, addr: 32 + 9,
            format: Format::Data(FormatData { k: 63.0/127.0, b: -63.0, format: "{val:1.1f} db".into() }),
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            ..def() },
        "compressor_gain" => RangeControl { cc: 5, addr: 32 + 5,
            format: Format::Data(FormatData { k: 16.0/127.0, b: 0.0, format: "{val:1.1f} db".into() }),
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            ..def() },
        // reverb
        "reverb_select" => Select { cc: 37, addr: 32 + 37, ..def() },
        "reverb_decay" => RangeControl { cc: 38, addr: 32 + 38,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        "reverb_tone" => RangeControl { cc: 39, addr: 32 + 39,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        "reverb_pre_delay" => RangeControl { cc: 40, addr: 32 + 40,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        "reverb_level" => RangeControl { cc: 18, addr: 32 + 18,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        // stomp
        "stomp_select" => Select { cc: 75, addr: 32 + 75, ..def() },
        "stomp_param2" => RangeControl { cc: 79, addr: 32 + 79,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        "stomp_param2_wave" => VirtualRangeControl { config: short!(0, 7),
            ..def() },
        "stomp_param2_octave" => VirtualRangeControl {
            config: short!(0, 8),
            format: Format::Labels(convert_args!(vec!(
                "-1 oct", "-maj 6th", "-min 6th", "-4th", "unison", "min 3rd", "maj 3rd", "5th", "1 oct"
            ))),
            ..def() },
        "stomp_param3" => RangeControl { cc: 80, addr: 32 + 80,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        "stomp_param3_octave" => VirtualRangeControl {
            config: short!(0, 8),
            format: Format::Labels(convert_args!(vec!(
                "-1 oct", "-5th", "-4th", "-2nd", "unison", "4th", "5th", "7th", "1 oct"
            ))),
            ..def() },
        "stomp_param3_offset" => VirtualRangeControl {
            config: short!(0, 49),
            format: Format::Data(FormatData { k: 1.0, b: -24.0, format: "{val}".into() }),
            ..def() },
        "stomp_param4" => RangeControl { cc: 81, addr: 32 + 82,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        "stomp_param4_offset" => VirtualRangeControl {
            config: short!(0, 49),
            format: Format::Data(FormatData { k: 1.0, b: -24.0, format: "{val}".into() }),
            ..def() },
        "stomp_param5" => RangeControl { cc: 82, addr: 32 + 83,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },
        "stomp_param6" => RangeControl { cc: 83, addr: 32 + 84,
            config: RangeConfig::Normal { buffer_config: BufferConfig::Midi },
            format: fmt_percent!(),
            ..def() },

        "loop_enable:show" => VirtualSelect {}
    ));
    /*
    let controls = pod2_config.controls.into_iter()
        .filter(|(k, _)| !exclude.contains(&k.as_str()))
        .chain(podxt_controls)
        .collect();
     */

    Config {
        name: "PODxt".to_string(),
        family: 0x0003,
        member: 0x0002,

        program_num: 128,
        program_size: 72*2 + 16,
        program_name_addr: 0,
        program_name_length: 16,

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

        flags: DeviceFlags::MANUAL_MODE,

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
            // show signals
            "loop_enable:show"
        )),

        // request edit buffer dump after setting `amp select` CC 12, 'reverb select' CC 37
        // todo: others?
        out_cc_edit_buffer_dump_req: vec![ 12, 37 ],

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
