use apple_device::{AppleDevice, Mode};
use clap::{Parser, Subcommand};
use env_logger::Env;

mod apple_device;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Exploits the bootrom and disables certificate checking
    Hax,
    /// Loads an image using DFU
    Load {
        /// Image file path
        file_path: String,
    },
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    // [NOTE] if we are getting RusbError(NotSupported) when connecting probably means that Windows loaded the Apple Driver
    // use Zadig(https://zadig.akeo.ie/) to replace with WinUSB
    let ipod =
        AppleDevice::in_mode(Mode::DFU).expect("Failed connecting to the iPod in DFU mode");

    log::info!("We found an Apple Device in {:?} mode.", ipod.mode);

    match cli.command {
        Some(Commands::Hax) => {
            ipod.hax().expect("Failed running the exploit");
            log::info!("Succesfully exploited the device!");
        }
        Some(Commands::Load { file_path }) => {
            ipod.load_image_from_file(&file_path).expect("Failed loading the image");
            log::info!("Image ({}) loaded to the device.",file_path);
        }
        None => {}
    }
}
