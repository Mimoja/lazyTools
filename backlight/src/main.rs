
use std::fs::OpenOptions;
use std::fs::read_dir;
use std::io::Read;
use std::io::Write;

fn main() {
    let paths = read_dir("/sys/class/backlight").unwrap();

    for path in paths {
        let backlight  = GenericBacklight::New(path.unwrap().file_name().into_string().unwrap());
        backlight.set(backlight.max().unwrap());
    }
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
    fn New(p : String) -> GenericBacklight{
        return GenericBacklight{
            path : p,
        };
    }
    fn read_file_to_u32(path : String) -> Result<u32, String>{
        let mut file = match OpenOptions::new().read(true).open(path){
            Ok(f) => f,
            Err(_) => return Err(String::from("Could not open brightness file")),
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