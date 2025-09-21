use std::fs;
use std::fs::File;
use chrono::{Local, SecondsFormat};
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};

pub fn setup(){
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(LevelFilter::max(), Config::default(), {
            fs::create_dir_all("log").unwrap();
            let date = Local::now().to_rfc3339_opts(SecondsFormat::Secs, false);
            // this fixes windows being windows
            let date = date.replace(":", "-");
            let filename = format!("{}.log", date);
            if cfg!(windows) {
                File::create(format!("log\\{}", filename)).unwrap()
            } else {
                File::create(format!("log/{}", filename)).unwrap()
            }
        }),
    ])
        .unwrap();

    /*ctrlc::set_handler(||{
        FORCE_EXIT.call_once_force(|_|{
            println!("attempting exit");
        });
    }).unwrap();*/

    dotenv::dotenv().ok();
}