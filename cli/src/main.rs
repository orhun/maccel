use std::{fs::File, io::Read};

mod binder;
mod libmaccel;
mod params;
mod tui;

use binder::{bind_device, disabling_udev_rules, unbind_device};
use clap::{builder::OsStr, CommandFactory, Parser};
use glob::glob;
use params::Param;
use tui::run_tui;

#[derive(Parser)]
#[clap(author, about, version)]
/// CLI to control the paramters for the maccel driver, and manage mice bindings
struct Cli {
    #[clap(subcommand)]
    command: Option<ParamsCommand>,
}

#[derive(clap::Subcommand, Default)]
enum ParamsCommand {
    /// Open the Terminal UI to manage the parameters
    /// and see a graph of the sensitivity
    #[default]
    Tui,
    /// Attach a device to the maccel driver
    Bind { device_id: String },
    /// Attach all detected mice to the maccel driver
    Bindall,
    /// Detach a device from the maccel driver,
    /// reattach to the generic usbhid driver
    Unbind { device_id: String },
    /// Detach all detected mice from the maccel driver
    /// reattach them to the generic usbhid driver
    Unbindall,
    /// Set the value for a parameter of the maccel driver
    Set { name: Param, value: f32 },
    /// Get the value for a parameter of the maccel driver
    Get { name: Param },
    /// Generate a completions file for a specified shell
    Completion {
        // The shell for which to generate completions
        shell: clap_complete::Shell,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command.unwrap_or_default() {
        ParamsCommand::Set { name, value } => {
            name.set(value)?;
        }
        ParamsCommand::Get { name } => {
            let value = name.get()?;
            let string_value: &str = (&value).try_into()?;
            println!("{}", string_value);
        }
        ParamsCommand::Bind { device_id } => {
            bind_device(&device_id)?;
        }
        ParamsCommand::Bindall => {
            eprintln!("[INFO] looking for all mice bound to usbhid: the generic hid driver");

            let paths = glob("/sys/bus/usb/drivers/usbhid/[0-9]*")?;

            for path in paths.flatten() {
                let basename = path.file_name();
                if path.is_dir() && basename != Some(&OsStr::from("module")) {
                    let protocol =
                        File::open(path.join("bInterfaceProtocol")).and_then(|mut f| {
                            let mut buf = String::new();
                            return f.read_to_string(&mut buf).map(|_| buf);
                        })?;

                    let subclass =
                        File::open(path.join("bInterfaceSubClass")).and_then(|mut f| {
                            let mut buf = String::new();
                            return f.read_to_string(&mut buf).map(|_| buf);
                        })?;

                    // checking for the observed invariant for a usb mouse device
                    if protocol.trim() == "02" && subclass.trim() == "01" {
                        let device_id = basename.unwrap().to_str().expect(
                        "basename of the /sys/*/drivers/usbhid device_id paths should be strings",
                    );
                        eprintln!("[INFO] found device to bind, id: {}", device_id);
                        bind_device(device_id)?;
                        eprintln!()
                    }
                }
            }
        }
        ParamsCommand::Unbind { device_id } => disabling_udev_rules(|| unbind_device(&device_id))?,
        ParamsCommand::Unbindall => {
            eprintln!("[INFO] looking for all devices bound to maccel");

            disabling_udev_rules(|| {
                let dirs = std::fs::read_dir("/sys/bus/usb/drivers/maccel")?;

                for d in dirs.flatten() {
                    let path = d.path();
                    let basename = path.file_name();
                    if path.is_dir() && basename != Some(&OsStr::from("module")) {
                        let device_id = basename.unwrap().to_str().expect(
                        "basename of the /sys/*/drivers/maccel device_id paths should be strings",
                    );
                        eprintln!("[INFO] found device to unbind, id: {}", device_id);
                        unbind_device(device_id)?;
                        eprintln!()
                    }
                }

                Ok(())
            })?;
        }
        ParamsCommand::Tui => run_tui()?,
        ParamsCommand::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "maccel", &mut std::io::stdout())
        }
    }

    Ok(())
}
