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

pub static _PODS: Lazy<Vec<Config>> = Lazy::new(|| {
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
               "delay_enable" => SwitchControl { cc: 28 },
               "reverb_enable" => SwitchControl { cc: 36 },
               "noise_gate_enable" => SwitchControl { cc: 22 },
               "bright_switch_enable" => SwitchControl { cc: 75 },

               // 12: amp model
               "drive" => RangeControl { cc: 13, ..def!() },
               // drive 2 (no transmit)
               "bass" => RangeControl { cc: 14, ..def!() },
               "mid" => RangeControl { cc: 15, ..def!() },
               "treble" => RangeControl { cc: 16, ..def!() },
               "bright switch" => RangeControl { cc: 73, ..def!() },
               // 21: presence | tx on/off | rx 0-127

               "amp_select" => Select { cc: 12 },
               "cab_select" => Select { cc: 71 },

               "volume_pedal_location" => SwitchControl { cc: 47 }


           ))
       }
   ]
});

// lazy_static! confuses IDEA type inference, making using _PODS directly in code hard
#[allow(non_snake_case)]
pub fn PODS() -> &'static Vec<Config> {
    &_PODS
}
