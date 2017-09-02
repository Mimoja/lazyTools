#[macro_use]
extern crate clap;
extern crate regex;

use std::fs::OpenOptions;
use std::fs::read_dir;
use std::io::Read;
use std::io::Write;
use std::str::FromStr;
use std::fmt::Display;
use std::cmp::{min,max};

fn main() {
    let matches = clap_app!(backlight =>
        (version: crate_version!())
        (about: crate_description!())
        (@subcommand set =>
            (about: "sets brightness")
            (@arg VALUE: +required +allow_hyphen_values "Brightness specification")
        )
        (@subcommand get =>
            (about: "gets brightness")
            (@arg percentage: -p --percentage "Show percentage of maximum brightness")
        )
    ).get_matches();

    let backlight  = GenericBacklight::new("intel_backlight".into());
    match matches.subcommand() {
        ("set", Some(sub_matches)) => {
            set_brightness(backlight, value_t!(sub_matches, "VALUE", BrightnessSpec).unwrap())
                .unwrap_or_else(|e| exit_err(e));
        }
        ("get", Some(_)) => {
            println!("{}", backlight.get().unwrap_or_else(|e| exit_err(e)));
        }
        _ => {}
    }
    /*
    let paths = read_dir("/sys/class/backlight").unwrap();
    for path in paths {
        let backlight  = GenericBacklight::new(path.unwrap().file_name().into_string().unwrap());
        backlight.set(backlight.max().unwrap()).unwrap_or_else(|e|exit_err(e));
    }
    */
}

fn set_brightness<B>(backlight: B, spec: BrightnessSpec) -> Result<u32, String>
    where B : Backlight {
    let next : u32 = match spec {
        BrightnessSpec::Absolute(v) => {
            v
        },
        BrightnessSpec::Relative(v) => {
            let current = backlight.get()?;
            (current as i32 + v) as u32
        },
        BrightnessSpec::Percentage(v) => {
            let max = backlight.max()?;
            (v * max)/100
        },
        BrightnessSpec::RelativePercentage(v) => {
            let current = backlight.get()?;
            if v > 0 {
                max(current + 1, ((current as i32 * (100 + v))/100) as u32)
            } else if v < 0 {
                min(current - 1, ((current as i32 * (100 + v))/100) as u32)
            } else {
                current
            }
        },
    };
    backlight.set(next)?;
    Ok(next)
}

fn exit_err<S: Display>(err: S) -> ! {
    eprintln!("{}", err);
    std::process::exit(1)
}

#[derive(Debug, PartialEq, Eq)]
enum BrightnessSpec {
    Absolute(u32),
    Relative(i32),
    Percentage(u32),
    RelativePercentage(i32),
}

impl FromStr for BrightnessSpec {
    type Err = ();
    fn from_str(s : &str) -> Result<Self, Self::Err> {
        use regex::Regex;
        let re = Regex::new(r"(?x)
            (?P<abs>^\d+$)|             # absolute value
            (?P<rel>^[\+-]\d+$)|        # relative
            (^(?P<perc>\d+)%$)|         # percentage
            (^(?P<relperc>[\+-]\d+)%$)  # relative percentage
        ").unwrap();
        match re.captures(s) {
            Some(c) => {
                if let Some(m) = c.name("abs") {
                    Ok(BrightnessSpec::Absolute(m.as_str().parse::<u32>().unwrap()))
                } else if let Some(m) = c.name("rel") {
                    Ok(BrightnessSpec::Relative(m.as_str().parse::<i32>().unwrap()))
                } else if let Some(m) = c.name("perc") {
                    Ok(BrightnessSpec::Percentage(m.as_str().parse::<u32>().unwrap()))
                } else if let Some(m) = c.name("relperc") {
                    Ok(BrightnessSpec::RelativePercentage(m.as_str().parse::<i32>().unwrap()))
                } else {
                    Err(())
                }
            },
            None => Err(())
        }
    }
}

#[test]
fn specparser_accept() {
    assert_eq!(BrightnessSpec::from_str("42"), Ok(BrightnessSpec::Absolute(42)));
    assert_eq!(BrightnessSpec::from_str("+4"), Ok(BrightnessSpec::Relative( 4)));
    assert_eq!(BrightnessSpec::from_str("-4"), Ok(BrightnessSpec::Relative(-4)));
    assert_eq!(BrightnessSpec::from_str("99%"), Ok(BrightnessSpec::Percentage(99)));
    assert_eq!(BrightnessSpec::from_str("+58%"), Ok(BrightnessSpec::RelativePercentage(58)));
    assert_eq!(BrightnessSpec::from_str("-03%"), Ok(BrightnessSpec::RelativePercentage(-3)));
}

#[test]
fn specparser_reject() {
    assert_eq!(BrightnessSpec::from_str("deadbeef"), Err(()));
    assert_eq!(BrightnessSpec::from_str("+34foo"), Err(()));
    assert_eq!(BrightnessSpec::from_str("+1337wat%"), Err(()));
    assert_eq!(BrightnessSpec::from_str("0xcaffeebabe"), Err(()));
    assert_eq!(BrightnessSpec::from_str("3.14159"), Err(()));
}


trait Backlight{
    fn get(&self) -> Result<u32, String>;
    fn set(&self, val : u32) -> Result<(), String>;
    fn max(&self) -> Result<u32, String>;
}


struct GenericBacklight{
    path : String,
}

impl GenericBacklight{
    fn new(p : String) -> GenericBacklight{
        return GenericBacklight{
            path : p,
        };
    }
    fn read_file_to_u32(path : String) -> Result<u32, String>{
        let mut file = match OpenOptions::new().read(true).open(&path){
            Ok(f) => f,
            Err(e) => return Err(format!("Error opening {}: {}", &path, e)),
        };

        let mut file_content = String::with_capacity(50);
        match file.read_to_string(&mut file_content){
            Ok(_) => {},
            Err(_) => return Err(String::from("Could not read from file")),

        };

        match file_content.trim().parse::<u32>() {
            Ok(result) => return Ok(result),
            Err(_) => return Err(String::from("Could not parse value")),
        }
    }

    fn write_u32_to_file(path : String, value : u32) -> Result<(), String>{
        let mut file = match OpenOptions::new().write(true).open(path){
            Ok(f) => f,
            Err(_) => return Err(String::from("Could not open file")),
        };
        let value_string = format!("{}", value);
        match file.write_all(value_string.as_bytes()){
            Ok(_) => return Ok(()),
            Err(_) => return Err(String::from("Could not write file")),
        }
    }
}

impl Backlight for GenericBacklight{
    fn get(&self) -> Result<u32, String> {
        return GenericBacklight::read_file_to_u32(format!("/sys/class/backlight/{}/brightness", self.path));
    }

    fn set(&self, val: u32) -> Result<(), String> {
        return GenericBacklight::write_u32_to_file(format!("/sys/class/backlight/{}/brightness", self.path), val);
    }

    fn max(&self) -> Result<u32, String> {
        return GenericBacklight::read_file_to_u32(format!("/sys/class/backlight/{}/max_brightness", self.path));
    }
}