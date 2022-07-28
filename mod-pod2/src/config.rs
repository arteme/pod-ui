use std::collections::HashMap;
use maplit::*;
use once_cell::sync::Lazy;
use pod_core::model::*;

#[macro_export]
macro_rules! def {
    () => (::std::default::Default::default());
}

#[macro_export]
macro_rules! amps {
    (@amp $name:tt + p + b + d2) => (Amp { name: ($name).into(), bright_switch: true, presence: true, delay2: false });
    (@amp $name:tt + b + d2)     => (Amp { name: ($name).into(), bright_switch: true, delay2: false, ..def!() });
    (@amp $name:tt + p + b)      => (Amp { name: ($name).into(), bright_switch: true, presence: true, ..def!() });
    (@amp $name:tt + p)          => (Amp { name: ($name).into(), presence: true, ..def!() });
    (@amp $name:tt + b)          => (Amp { name: ($name).into(), bright_switch: true, ..def!() });
    (@amp $name:tt)              => (Amp { name: ($name).into(), ..def!() });

    ( $($a:tt $(+ $b:tt)* ),+ $(,)* ) => {
       vec![
         $(
           amps!(@amp $a $(+ $b)*),
         )+
       ]
    }
}

macro_rules! fx {
    ($name:tt, d=$d:tt + $dt:tt $(+ $dc:expr)? , c=$c:tt + $ct:tt $(+ $cc:expr)? ) => (
        Effect {
            name: ($name).into(),
            delay: Some(EffectEntry { id: $d, effect_tweak: ($dt).into(),
                                      $( controls: ($dc).to_vec(), )? ..def!() }),
            clean: Some(EffectEntry { id: $c, effect_tweak: ($ct).into(),
                                      $( controls: ($cc).to_vec(), )? ..def!() }),
        }
    );
    ($name:tt, d=$d:tt + $dt:tt + $dc:expr ) => (
        Effect {
            name: ($name).into(),
            delay: Some(EffectEntry { id: $d, effect_tweak: ($dt).into(), controls: ($dc).to_vec() }),
            clean: None,
        }
    );
    ($name:tt, c=$c:tt + $ct:tt + $cc:expr ) => (
        Effect {
            name: ($name).into(),
            delay: None,
            clean: Some(EffectEntry { id: $c, effect_tweak: ($ct).into(), controls: ($cc).to_vec() }),
        }
    );
}

macro_rules! fmt {
    ($f:tt) => ( Format::Callback($f) );
}
macro_rules! fmt_percent {
    (signed) => ( Format::Callback(RangeControl::fmt_percent_signed) );
    () => ( Format::Callback(RangeControl::fmt_percent) );
}

#[macro_export]
macro_rules! short {
    ( $from:expr, $to:expr ) => ( RangeConfig::Short { from: $from, to: $to } );
    () => ( short!(0, 63) );
}
#[macro_export]
macro_rules! long {
    ( $from:expr, $to:expr ) => ( RangeConfig::Long { from: $from, to: $to } )
}

macro_rules! string_vec {
    ( $($x:expr),* ) => (vec![ $($x.to_string()),* ]);
}
macro_rules! concat {
    ( $($x:expr),* ) => ( [ $(&$x[..]),* ].concat() );
}

static EFFECT_DELAY_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    string_vec!["delay_time", "delay_time:fine", "delay_feedback", "delay_level"]
});
static EFFECT_COMPRESSION_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    string_vec!["compression_ratio"]
});
static EFFECT_DELAY_COMPRESSION_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    concat![ EFFECT_COMPRESSION_CONTROLS, EFFECT_DELAY_CONTROLS ]
});
static EFFECT_CH_FL_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    string_vec!["chorus_flanger_speed", "chorus_flanger_depth",
        "chorus_flanger_feedback", "chorus_flanger_pre_delay"]
});
static EFFECT_DELAY_CH_FL_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    concat![ EFFECT_CH_FL_CONTROLS, EFFECT_DELAY_CONTROLS ]
});
static EFFECT_DELAY_SWELL_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    concat![ string_vec!["volume_swell_time"], EFFECT_DELAY_CONTROLS ]
});
static EFFECT_TREMOLO_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    string_vec!["trem_speed", "trem_depth"]
});
static EFFECT_DELAY_TREMOLO_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    concat![ EFFECT_TREMOLO_CONTROLS, EFFECT_DELAY_CONTROLS ]
});
static EFFECT_ROTARY_CONTROLS: Lazy<Vec<String>> = Lazy::new(|| {
    string_vec!["rotary_speed", "rotary_fast_speed", "rotary_slow_speed", "effect_tweak"]
});

fn gate_threshold_from_midi(value: u8) -> u16 {
    ((127.0 - value as f64) * 194.0/256.0) as u16
}

fn gate_threshold_to_midi(value: u16) -> u8 {
    (127.0 - (value as f64 * 256.0/194.0)) as u8
}

pub static POD2_CONFIG: Lazy<Config> = Lazy::new(|| {
    Config {
        name: "POD 2.0".to_string(),
        family: 0x0000,
        member: 0x0300,

        program_size: 71,
        program_num: 36,
        manual: true,

        amp_models: amps!(
           "Tube Preamp" +p,
           "Line 6 Clean" +p +b,
           "Line 6 Crunch" +p +b,
           "Line 6 Drive" +p +b,
           "Line 6 Layer" +p +b +d2,
           "Small Tweed",
           "Tweed Blues" +p,
           "Black Panel",
           "Modern Class A" +p,
           "Brit Class A",
           "Brit Blues" +p +b,
           "Brit Classic" +p,
           "Brit Hi Gain" +p,
           "Treadplate" +p,
           "Modern Hi Gain",
           "Fuzz Box" +p,
           "Jazz Clean" +p +b,
           "Boutique #1" +p,
           "Boutique #2",
           "Brit Class A #2",
           "Brit Class A #3",
           "Small Tweed #2",
           "Black Panel #2" +b,
           "Boutique #3" +p,
           "California Crunch #1" +p +b,
           "California Crunch #2" +p,
           "Treadplate #2" +p,
           "Modern Hi Gain #2" +p,
           "Line 6 Twang",
           "Line 6 Crunch #2",
           "Line 6 Blues",
           "Line 6 Insane",
        ),
        cab_models: convert_args!(vec!(
           "1x8  '60 Fender Tweed Champ",
           "1x12 ’52 Fender Tweed Deluxe",
           "1x12 ’60 Vox AC15",
           "1x12 ’64 Fender Blackface Deluxe",
           "1x12 ’98 Line 6 Flextone",
           "2x12 ’65 Fender Blackface Twin",
           "2x12 ’67 VOX AC30",
           "2x12 ’95 Matchless Chieftain",
           "2x12 ’98 Pod custom 2x12",
           "4x10 ’59 Fender Bassman",
           "4x10 ’98 Pod custom 4x10 cab",
           "4x12 ’96 Marshall with V30s",
           "4x12 ’78 Marshall with 70s",
           "4x12 ’97 Marshall with Greenbacks",
           "4x12 ’98 Pod custom 4x12",
           "No Cabinet",
       )),
        effects: vec![
            fx!("Bypass", // 0
               d=6 + "delay_level" + EFFECT_DELAY_CONTROLS,
               c=10 + ""  // no effects
           ),
            fx!("Compressor", // 1
               d=7 + "compression_ratio" + EFFECT_DELAY_COMPRESSION_CONTROLS,
               c=11 + "compression_ratio" + EFFECT_COMPRESSION_CONTROLS
           ),
            fx!("Auto Swell", // 2
               d=14 + "volume_swell_time" + EFFECT_DELAY_SWELL_CONTROLS
               // only with delay
           ),
            fx!("Chorus 1", // 3
               d=4 + "delay_level" + EFFECT_DELAY_CH_FL_CONTROLS,
               c=8 + "chorus_flanger_depth" + EFFECT_CH_FL_CONTROLS
           ),
            fx!("Chorus 2", // 4
               d=12 + "delay_level" + EFFECT_DELAY_CH_FL_CONTROLS,
               c=0 + "chorus_flanger_depth" + EFFECT_CH_FL_CONTROLS
           ),
            fx!("Flanger 1", // 5
               d=13 + "delay_level" + EFFECT_DELAY_CH_FL_CONTROLS,
               c=1 + "chorus_flanger_feedback" + EFFECT_CH_FL_CONTROLS
           ),
            fx!("Flanger 2", // 6
               d=15 + "delay_level" + EFFECT_DELAY_CH_FL_CONTROLS,
               c=3 + "chorus_flanger_feedback" + EFFECT_CH_FL_CONTROLS
           ),
            fx!("Tremolo", // 7
               d=5 + "delay_level" + EFFECT_DELAY_TREMOLO_CONTROLS,
               c=9 + "trem_depth" + EFFECT_TREMOLO_CONTROLS
           ),
            fx!("Rotary", // 8
               c=2 + "" + EFFECT_ROTARY_CONTROLS
               // no delay!
           )
        ],
        controls: convert_args!(hashmap!(
           // switches
           "distortion_enable" => SwitchControl { cc: 25, addr: 0 },
           "drive_enable" => SwitchControl { cc: 26, addr: 1 },
           "eq_enable" => SwitchControl { cc: 27, addr: 2 },
           "delay_enable" => SwitchControl { cc: 28, addr: 3 },
           "effect_enable" => SwitchControl { cc: 50, addr: 4 }, // trem/rotary speaker/chorus/flanger
           "reverb_enable" => SwitchControl { cc: 36, addr: 5 },
           "noise_gate_enable" => SwitchControl { cc: 22, addr: 6 },
           "bright_switch_enable" => SwitchControl { cc: 73, addr: 7 },
           // preamp
           "amp_select" => Select { cc: 12, addr: 8, ..def!() },
           "drive" => RangeControl { cc: 13, addr: 9, config: short!(),
               format: fmt_percent!(), ..def!() },
           "drive2" => RangeControl { cc: 20, addr: 10, config: short!(),
               format: fmt_percent!(), ..def!() }, // only "pod layer"
           "bass" => RangeControl { cc: 14, addr: 11, config: short!(),
               format: fmt_percent!(), ..def!() },
           "mid" => RangeControl { cc: 15, addr: 12, config: short!(),
               format: fmt_percent!(), ..def!() },
           "treble" => RangeControl { cc: 16, addr: 13, config: short!(),
               format: fmt_percent!(), ..def!() },
           "presence" => RangeControl { cc: 21, addr: 14, config: short!(),
               format: fmt_percent!(), ..def!() },
           "chan_volume" => RangeControl { cc: 17, addr: 15, config: short!(),
               format: fmt_percent!(), ..def!() },
           // noise gate
           "gate_threshold" => RangeControl { cc: 23, addr: 16,
               config: RangeConfig::Function { from_midi: gate_threshold_from_midi, to_midi: gate_threshold_to_midi },
               format: Format::Data(FormatData { k: 1.0, b: -96.0, format: "{val} db".into() }), ..def!() }, // todo: -96 db .. 0 db
           "gate_decay" => RangeControl { cc: 24, addr: 17, config: short!(),
                format: fmt_percent!(), ..def!() }, // todo: 8.1 msec .. 159 msec
           // wah wah
           // wah pedal on/off,  cc: 43 ??
           "wah_level" => RangeControl { cc: 4, addr: 18,format: fmt_percent!(), ..def!() },
           "wah_bottom_freq" => RangeControl { cc: 44, addr: 19,format: fmt_percent!(), ..def!() },
           "wah_top_freq" => RangeControl { cc: 45, addr: 20,format: fmt_percent!(), ..def!() },
           // volume pedal
           "vol_level" => RangeControl { cc: 7, addr: 22, format: fmt_percent!(), ..def!() },
           "vol_minimum" => RangeControl { cc: 46, addr: 23, format: fmt_percent!(), ..def!() },
           "vol_pedal_position" => SwitchControl { cc: 47, addr: 24, ..def!() },
           // delay
           "delay_time" => RangeControl { cc: 30, addr: 26, /*config: long!(7,7),*/
               format: Format::Data(FormatData { k: 6.0 * 0.03205, b: 0.0, format: "{val:1.2f} ms".into() }),
                ..def!() }, // 0 .. 3150 ms / 128 steps
           "delay_time:fine" => RangeControl { cc: 62, addr: 27, ..def!() }, // todo: what to do with this?
           "delay_feedback" => RangeControl { cc: 32, addr: 34, config: short!(),
                format: fmt_percent!(), ..def!() },
           "delay_level" => RangeControl { cc: 34, addr: 36, config: short!(),
                format: fmt_percent!(), ..def!() },
           // reverb
           "reverb_type" => SwitchControl { cc: 37, addr: 38, ..def!() }, // 0: spring, 1: hall
           "reverb_decay" => RangeControl { cc: 38, addr: 39, config: short!(),
                format: fmt_percent!(), ..def!() },
           "reverb_tone" => RangeControl { cc: 39, addr: 40, config: short!(),
                format: fmt_percent!(), ..def!() },
           "reverb_diffusion" => RangeControl { cc: 40, addr: 41, config: short!(),
                format: fmt_percent!(), ..def!() },
           "reverb_density" => RangeControl { cc: 41, addr: 42, config: short!(),
                format: fmt_percent!(), ..def!() },
           "reverb_level" => RangeControl { cc: 18, addr: 43, config: short!(),
                format: fmt_percent!(), ..def!() },
           // cabinet sim
           "cab_select" => Select { cc: 71, addr: 44, ..def!() },
           "air" => RangeControl { cc: 72, addr: 45,config: short!(),
                format: fmt_percent!(), ..def!() },
           // effect
           "effect_select:raw" => Select { cc: 19, addr: 46 }, // 0 - bypass, 1..15 - effects
           "effect_select" => VirtualSelect {}, // select control for the ui
           "effect_tweak" => RangeControl { cc: 1, addr: 47, config: short!(),
                format: fmt_percent!(), ..def!() }, // in ui: rotary depth
           // effect parameters
           // volume swell on/off,  cc: 48 ??
           "volume_swell_time" => RangeControl { cc: 49, addr: 48, config: short!(),
               format: fmt_percent!(), ..def!() },
           "compression_ratio" => RangeControl { cc: 42, addr: 48, config: short!(0,5),
               format: Format::Labels(convert_args!(vec!(
                   "off", "1.4:1", "2:1", "3:1", "6:1", "inf:1"
               ))),
               ..def!() }, // off, 1.4:1, 2:1, 3:1, 6:1, inf:1
            // TODO: how to make all these below long?
           "chorus_flanger_speed" => RangeControl { cc: 51, addr: 48, config: long!(0,6250),
               format: Format::Data(FormatData { k: 1.0, b: 0.0, format: "{val:1.0f} ms".into() }),
               ..def!() }, // todo: 200 .. 65535 ms (x * 50)
           "chorus_flanger_depth" => RangeControl { cc: 52, addr: 50, config: long!(0,312),
                format: fmt_percent!(),..def!() }, // 0..312 samples @ 31.2KHz (x * 256 / 104)
           "chorus_flanger_feedback" => RangeControl { cc: 53, addr: 52,
               format: fmt_percent!(signed), ..def!() }, // 0(max)..63(min) negative, 64(min)..127(max) positive
           "chorus_flanger_pre_delay" => RangeControl { cc: 54, addr: 53, config: long!(1,780),
               format: fmt_percent!(), ..def!() }, // 1..780 samples @31.2KHz (x * 256 / 42)
           "rotary_speed" => RangeControl { cc: 55, addr: 48, config: short!(0,1),
               format: Format::Labels(convert_args!(vec!("slow", "fast"))),
               ..def!() }, // 0: slow, 1: fast // todo: SwitchControl?
           "rotary_fast_speed" => RangeControl { cc: 56, addr: 49, /*config: long!(7,4),*/
               format: Format::Data(FormatData { k: 515.0, b: 100.0, format: "{val:1.0f} ms".into() }),
               ..def!() }, // 100 .. 65535 ms period (x * 22) + 100
           "rotary_slow_speed" => RangeControl { cc: 57, addr: 51, /*config: long!(7,4),*/
               format: Format::Data(FormatData { k: 515.0, b: 100.0, format: "{val:1.0f} ms".into() }),
               ..def!() }, // 100 .. 65535 ms period (x * 22) + 100
           "trem_speed" => RangeControl { cc: 58, addr: 48, /*config: long!(7,4),*/
               format: Format::Data(FormatData { k: 515.0, b: 150.0, format: "{val:1.0f} ms".into() }),
               ..def!() }, // 150 .. 65535 ms period (x * 25)
           "trem_depth" => RangeControl { cc: 59, addr: 50, format: fmt_percent!(), ..def!() },

            // special used for ui wiring only
            "name_change" => Button {},
            "digiout_show" => VirtualSelect {}
       )),
        init_controls: convert_args!(vec!(
           "distortion_enable",
           "drive_enable",
           "eq_enable",
           "delay_enable",
           "reverb_enable",
           "noise_gate_enable",
           "bright_switch_enable",
           "effect_select",
           "amp_select",
           "digiout_show"
       )),
        program_name_addr: 55,
        program_name_length: 16
    }
});

pub static PODPRO_CONFIG: Lazy<Config> = Lazy::new(|| {
    let pod2_config = POD2_CONFIG.clone();

    let pro_controls: HashMap<String, Control> = convert_args!(hashmap!(
        "digiout_gain" => RangeControl { cc: 9, addr: 35,
            config: short!(),
            format: Format::Data(FormatData { k: 12.0/63.0, b: 0.0, format: "{val:1.2f} db".into()}),
            ..def!()
        }
    ));
    let controls = pod2_config.controls.into_iter()
        .chain(pro_controls)
        .collect();

    Config {
        name: "POD Pro".to_string(),
        family: 0x0000,
        member: 0x0400,

        controls,

        ..pod2_config
    }
});
