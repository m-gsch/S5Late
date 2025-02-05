use apple_device::{AppleDevice, Mode};
use clap::{Parser, Subcommand};
use env_logger::Env;

mod apple_device;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    let ipod = match AppleDevice::in_mode(Mode::DFU) {
        Ok(device) => device,
        Err(e) => {
            log::error!("Failed connecting to the iPod in DFU mode. {e}.");
            return;
        },
    };

    log::info!("We found an Apple Device in {:?} mode.", ipod.mode);

    match cli.command {
        Commands::Hax => {
            match ipod.hax() {
                Ok(_) => log::info!("Succesfully exploited the device!"),
                Err(e) => log::error!("Failed running the exploit. {e}."),
            }
        }
        Commands::Load { file_path } => {
            match ipod.load_image_from_file(&file_path){
                Ok(_) =>  log::info!("Image ({}) loaded to the device.", file_path),
                Err(e) => log::error!("Failed loading the image. {e}."),
            }
        }
    }
}
