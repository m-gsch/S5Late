use std::{thread::sleep, time::Duration};

use apple_device::{AppleDevice, AppleDeviceError, Mode};
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
    /// Loads patched WTF, then U-Boot and then Linux
    Linux {
        /// Patched WTF file path
        wtf_path: String,
        /// U-Boot file path
        u_boot_path: String,
        /// Linux kernel file path
        linux_path: String,
    },
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    // [NOTE] if we are getting RusbError(NotSupported) when connecting probably means that Windows loaded the Apple Driver
    // use Zadig(https://zadig.akeo.ie/) to replace with WinUSB
    let ipod_dfu = match AppleDevice::in_mode(Mode::DFU) {
        Ok(device) => device,
        Err(e) => {
            log::error!("Failed connecting to the iPod in DFU mode. {e}.");
            return;
        }
    };
    log::info!("We found an Apple Device in {:?} mode.", ipod_dfu.mode);

    match cli.command {
        Commands::Hax => match ipod_dfu.hax() {
            Ok(_) => log::info!("Succesfully exploited the device!"),
            Err(e) => log::error!("Failed running the exploit. {e}."),
        },
        Commands::Load { file_path } => match ipod_dfu.load_image_from_file(&file_path) {
            Ok(_) => log::info!("Image ({}) loaded to the device.", file_path),
            Err(e) => log::error!("Failed loading the image. {e}."),
        },
        Commands::Linux {
            wtf_path,
            u_boot_path,
            linux_path,
        } => {
            match ipod_dfu.hax() {
                Ok(_) => log::info!("Succesfully exploited the device!"),
                Err(e) => {
                    log::error!("Failed running the exploit. {e}.");
                    return;
                }
            }

            match ipod_dfu.load_image_from_file(&wtf_path) {
                Ok(_) => log::info!("Image ({}) loaded to the device.", wtf_path),
                Err(e) => {
                    log::error!("Failed loading the image. {e}.");
                    return;
                }
            }

            let ipod_wtf = loop {
                match AppleDevice::in_mode(Mode::WTF) {
                    Ok(device) => break device,
                    Err(AppleDeviceError::DeviceNotFound) => sleep(Duration::from_secs(1)),
                    Err(e) => {
                        log::error!("Failed connecting to the iPod in WTF mode. {e}.");
                        return;
                    }
                }
            };
            log::info!("We found an Apple Device in {:?} mode.", ipod_wtf.mode);

            match ipod_wtf.load_image_from_file(&u_boot_path) {
                Ok(_) => log::info!("Image ({}) loaded to the device.", u_boot_path),
                Err(e) => {
                    log::error!("Failed loading the image. {e}.");
                    return;
                }
            }

            let ipod_uboot = loop {
                match AppleDevice::in_mode(Mode::UBOOT) {
                    Ok(device) => break device,
                    Err(AppleDeviceError::DeviceNotFound) => sleep(Duration::from_secs(1)),
                    Err(e) => {
                        log::error!("Failed connecting to the iPod in U-Boot mode. {e}.");
                        return;
                    }
                }
            };
            log::info!("We found an Apple Device in {:?} mode.", ipod_uboot.mode);

            match ipod_uboot.load_image_from_file(&linux_path) {
                Ok(_) => log::info!("Image ({}) loaded to the device.", linux_path),
                Err(e) => {
                    log::error!("Failed loading the image. {e}.");
                    return;
                }
            }
        }
    }
}
