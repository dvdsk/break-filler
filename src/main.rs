use std::{env, fs};

use break_filler::ui::Ui;
use clap::Parser;
use cli::Cli;
use color_eyre::eyre::{Context, OptionExt};
use time::zoned_now;

use break_filler::{cli, start_break_inforcer_integration_thread, time, Store};

mod install;

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        cli::Command::Run(run_args) => {
            start_break_inforcer_integration_thread(None);
            #[expect(deprecated, reason = "windows only issue fixed in next rust version")]
            let path = env::home_dir()
                .ok_or_eyre("Could not find home dir")?
                .join(".local")
                .join("share")
                .join(env!("CARGO_PKG_NAME"));
            fs::create_dir(&path)
                .accept_kind(std::io::ErrorKind::AlreadyExists)
                .wrap_err("Could not create directory to store db")?;
            let store = Store::new(path).wrap_err("Could not open database")?;

            iced::daemon(Ui::title, Ui::update, Ui::view)
                .subscription(Ui::subscription)
                .run_with(|| Ui::new(run_args, store))
                .wrap_err("Error running UI")
        }
        cli::Command::Test(test_args) => {
            time::setup_mock_from_args(&test_args);
            start_break_inforcer_integration_thread(Some(test_args.clone()));
            #[expect(deprecated, reason = "windows only issue fixed in next rust version")]
            let path = env::home_dir()
                .ok_or_eyre("Could not find home dir")?
                .join(".local")
                .join("share")
                .join(env!("CARGO_PKG_NAME"));
            fs::create_dir(&path)
                .accept_kind(std::io::ErrorKind::AlreadyExists)
                .wrap_err("Could not create directory to store db")?;
            let store = Store::new(path).wrap_err("Could not open database")?;
            store
                .last_check()
                .set(&zoned_now().yesterday().unwrap())
                .unwrap();

            iced::daemon(Ui::title, Ui::update, Ui::view)
                .subscription(Ui::subscription)
                .run_with(|| Ui::new(test_args.run_args, store))
                .wrap_err("Error running UI")
        }
        cli::Command::Install(run_args) => install::add_or_modify(run_args),
        cli::Command::Remove => install::remove(),
    }
}

trait ResultAcceptKind {
    type Error;
    fn accept_kind(self, errorkind: std::io::ErrorKind) -> Result<(), Self::Error>;
}

impl<T> ResultAcceptKind for Result<T, std::io::Error> {
    type Error = std::io::Error;
    fn accept_kind(self, kind: std::io::ErrorKind) -> Result<(), Self::Error> {
        match self {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == kind => Ok(()),
            Err(e) => Err(e),
        }
    }
}
