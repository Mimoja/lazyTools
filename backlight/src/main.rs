#[macro_use]
extern crate clap;
extern crate regex;

use std::fs::OpenOptions;
use std::fs::read_dir;
use std::io::Read;
use std::io::Write;
use std::str::FromStr;
use std::fmt::Display;
use std::cmp;
use clap::{App, ArgGroup, Arg};

fn main() {
    let matches = App::new("backlight")
        .version(crate_version!())
        .about(crate_description!())
        .arg(Arg::with_name("get")
            .short("-g")
            .long("--get")
            .help("Displays brightness")
            .takes_value(false))
        .arg(Arg::with_name("set")
            .short("-s")
            .long("--set")
            .help("Change brightness")
            .value_name("[+|-]VALUE[%]")
            .allow_hyphen_values(true)
            .takes_value(true))
        .arg(Arg::with_name("min")
                 .short("-m")
                 .long("--minimum-brightness")
            .value_name("VALUE")
                 .help("Don't allow brightness below VALUE")
                 .default_value("1"))
        .group(ArgGroup::with_name("op")
                   .arg("set")
                   .arg("get")
                   .multiple(true))
        .get_matches();

    if !matches.is_present("op") {
        println!("{}", matches.usage());
        std::process::exit(1);
    }

    let spec = value_t!(matches, "set", BrightnessSpec).unwrap_or_default();

    let paths = read_dir("/sys/class/backlight").unwrap();
    for path in paths {
        let backlight  = GenericBacklight::new(path.unwrap().file_name().into_string().unwrap());
        let old = backlight.get().unwrap_or_else(|e| exit_err(e));
        let max = backlight.max().unwrap_or_else(|e| exit_err(e));
        let min = matches.value_of("min").unwrap().parse().unwrap_or(1);
        let next = spec.apply(old, min, max);
        if matches.is_present("get") {
            println!("{}", next);
        }
        backlight.set(next).unwrap_or_else(|e| exit_err(e));
    }
}

fn exit_err<S: Display>(err: S) -> ! {
    eprintln!("{}", err);
    std::process::exit(1)
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum BrightnessSpec {
    Absolute(u32),
    Relative(i32),
    Percentage(u32),
    RelativePercentage(i32),
}

impl BrightnessSpec {
    fn apply(self, old: u32, min: u32, max: u32) -> u32 {
        let next = match self {
            BrightnessSpec::Absolute(v) => {
                v
            },
            BrightnessSpec::Relative(v) => {
                (old as i32 + v) as u32
            },
            BrightnessSpec::Percentage(v) => {
                min + (v * (max - min))/100
            },
            BrightnessSpec::RelativePercentage(v) => {
                if v > 0 {
                    cmp::max(old + 1, ((old as i32 * (100 + v))/100) as u32)
                } else if v < 0 {
                    cmp::min(old - 1, ((old as i32 * (100 + v))/100) as u32)
                } else {
                    old
                }
            },
        };
        cmp::max(min, cmp::min(max, next))
    }
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

impl Default for BrightnessSpec {
    fn default() -> Self {
        BrightnessSpec::Relative(0)
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