mod clicker;
mod hotkey;
mod ipc;
mod keymap;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "autoclicker", about = "Wayland-native autoclicker for KDE Plasma")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Toggle,
    Start,
    Stop,
    Click {
        #[arg(default_value = "left")]
        button: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Toggle) => {
            if let Err(e) = ipc::send_command("toggle") {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Start) => {
            if let Err(e) = ipc::send_command("start") {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Stop) => {
            if let Err(e) = ipc::send_command("stop") {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Click { button }) => {
            let btn = match button.as_str() {
                "left" => 1,
                "right" => 2,
                "middle" => 3,
                _ => {
                    eprintln!("Invalid button '{}'. Use: left, right, middle", button);
                    std::process::exit(1);
                }
            };
            if let Err(e) = clicker::single_click(btn) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        None => {
            ui::run();
        }
    }
}
