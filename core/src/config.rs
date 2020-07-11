use crate::model::Config;
use once_cell::sync::Lazy;
use crate::model::*;


macro_rules! def {
    () => (::std::default::Default::default(););
}

macro_rules! amp {
    (pb $name:expr) => (Amp { name: ($name).into(), bright_switch: true, presence: true });
    (p $name:expr) => (Amp { name: ($name).into(), bright_switch: false, presence: true });
    (b $name:expr) => (Amp { name: ($name).into(), bright_switch: true, presence: false });
    ($name:expr) => (Amp { name: ($name).into(), bright_switch: false, presence: false });
}

pub static PODS: Lazy<Vec<Config>> = Lazy::new(|| {
   vec![
       Config {
           name: "POD 2.0".to_string(),
           family: 0x0000,
           member: 0x0300,

           program_size: 71,
           all_programs_size: 71 * 36,
           pod_id: 0x01,

           amp_models: vec![
               amp!(p "Tube Preamp"),
               amp!(pb "POD Clean"),
               amp!(pb "POD Crunch"),
               amp!(pb "POD Drive"),
               amp!(pb "POD Layer"), // drive 2
               amp!("Small Tweed"),
               amp!(p "Tweed Blues"),
               amp!("Black Panel"),
               amp!(p "Modern Class A"),
               amp!("Brit Class A"),
               amp!(pb "Brit Blues"),
               amp!(p "Brit Classic"),
               amp!(p "Brit Hi Gain"),
               amp!(p "Rectified"),
               amp!("Modern Hi Gain"),
               amp!(p "Fuzz Box"),
               amp!(pb "Jazz Clean"),
               amp!(p "Boutique #1"),
               amp!("Boutique #2"),
               amp!("Brit Class A #2"),
               amp!("Brit Class A #3"),
               amp!("Small Tweed #2"),
               amp!(b "Black Panel #2"),
               amp!(p "Boutique #3"),
               amp!(pb "California Crunch #1"),
               amp!(p"California Crunch #2"),
               amp!(p"Rectified #2"),
               amp!(p"Modern Hi Gain #2"),
           ],
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
               "4x12 ’97 Marshall off axis",
               "4x12 ’98 Pod custom 4x12",
               "No Cabinet",
           )),
           controls: convert_args!(hashmap!(
               // switches
               "distortion_enable" => SwitchControl { cc: 25 },
               "drive_enable" => SwitchControl { cc: 26 },
               "eq_enable" => SwitchControl { cc: 27 },
               "delay_enable" => SwitchControl { cc: 28 },
               "effect_enable" => SwitchControl { cc: 50 }, // trem/rotary speaker/chorus/flanger
               "reverb_enable" => SwitchControl { cc: 36 },
               "noise_gate_enable" => SwitchControl { cc: 22 },
               "bright_switch_enable" => SwitchControl { cc: 73 },
               // preamp
               "amp_select" => Select { cc: 12 },
               "drive" => RangeControl { cc: 13, from: 0, to: 63 },
               "drive2" => RangeControl { cc: 20, from: 0, to: 63 }, // only "pod layer"
               "bass" => RangeControl { cc: 14, from: 0, to: 63 },
               "mid" => RangeControl { cc: 15, from: 0, to: 63 },
               "treble" => RangeControl { cc: 16, from: 0, to: 63 },
               "presence" => RangeControl { cc: 21, from: 0, to: 63 },
               "chan_volume" => RangeControl { cc: 17, from: 0, to: 63 },
               // noise gate
               "gate_threshold" => RangeControl { cc: 23, from: 0, to: 96 }, // todo: -96 db .. 0 db
               "gate_decay" => RangeControl { cc: 24, from: 0, to: 63 }, // todo: 8.1 msec .. 159 msec
               // wah wah
               // wah pedal on/off,  cc: 43 ??
               "wah_level" => RangeControl { cc: 4, ..def!() },
               "wah_bottom_freq" => RangeControl { cc: 44, ..def!() },
               "wah_top_freq" => RangeControl { cc: 45, ..def!() },
               // volume pedal
               "vol_level" => RangeControl { cc: 7, ..def!() },
               "vol_minimum" => RangeControl { cc: 7, ..def!() },
               "vol_pedal_position" => SwitchControl { cc: 47 },
               // delay
               "delay_time" => RangeControl { cc: 30, from: 0, to: 127/*3150*/ }, // 0 .. 3150 ms / 128 steps
               "delay_time:fine" => RangeControl { cc: 62, ..def!() }, // todo: what to do with this?
               "delay_feedback" => RangeControl { cc: 32, from: 0, to: 63 },
               "delay_level" => RangeControl { cc: 34, from: 0, to: 63 },
               // reverb
               "reverb_type" => SwitchControl { cc: 37 }, // 0: spring, 1: hall
               "reverb_decay" => RangeControl { cc: 38, from: 0, to: 63 },
               "reverb_tone" => RangeControl { cc: 39, from: 0, to: 63 },
               "reverb_diffusion" => RangeControl { cc: 40, from: 0, to: 63 },
               "reverb_density" => RangeControl { cc: 41, from: 0, to: 63 },
               "reverb_level" => RangeControl { cc: 18, from: 0, to: 63 },
               // cabinet sim
               "cab_select" => Select { cc: 71 },
               "air" => RangeControl { cc: 72, from: 0, to: 63 },
               // effect
               "effect_select" => RangeControl { cc: 19, from: 0, to: 15 }, // 0 - bypass, 1..15 - effects
               "effect_tweak" => RangeControl { cc: 1, from: 0, to: 63 },
               // effect parameters
               // volume swell on/off,  cc: 48 ??
               "volume_swell_time" => RangeControl { cc: 49, from: 0, to: 63 },
               "compression_ratio" => RangeControl { cc: 42, from: 0, to: 6 }, // off, 1.4:1, 2:1, 3:1, 6:1, inf:1
               "chorus_flanger_speed" => RangeControl { cc: 51, ..def!() }, // todo: 200 .. 65535 ms (x * 50)
               "chorus_flanger_depth" => RangeControl { cc: 52, ..def!() }, // todo: 0..312 samples @ 31.2KHz (x * 256 / 104)
               "chorus_flanger_feedback" => RangeControl { cc: 53, ..def!() }, // todo: 0(max)..63(min) negative, 64(min)..127(max) positive
               "chorus_flanger_pre_delay" => RangeControl { cc: 54, ..def!() }, // todo: 1..780 samples @31.2KHz (x * 256 / 42) // todo2: flanger too?!
               "rotary_speed" => SwitchControl { cc: 55 }, // 0: slow, 1: fast
               "rotary_fast_speed" => RangeControl { cc: 56, ..def!() }, // 100 .. 65535 ms period (x * 22) + 100
               "rotary_slow_speed" => RangeControl { cc: 57, ..def!() }, // 100 .. 65535 ms period (x * 22) + 100
               "trem_speed" => RangeControl { cc: 58, ..def!() }, // 150 .. 65535 ms period (x * 25)
               "trem_depth" => RangeControl { cc: 59, ..def!() },
           ))
       }
   ]
});
