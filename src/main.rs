use std::time::Duration;
use std::{env, fs, thread};

use break_filler::ui::Ui;
use clap::Parser;
use cli::Cli;
use color_eyre::eyre::{Context, OptionExt};
use time::zoned_now;

use break_filler::{
    cli, spawn_break_enforcer_interface, spawn_mock_break_enforcer_interface,
    time, Store,
};

mod install;

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    let (run_args, store) = match cli.command {
        cli::Command::Run(run_args) => {
            // give login process time to complete such that the display
            // server is running when iced starts. (there is no simple way to
            // check that which is why we use a sleep)
            thread::sleep(Duration::from_secs(10));
            spawn_break_enforcer_interface();
            #[expect(
                deprecated,
                reason = "windows only issue fixed in next rust version"
            )]
            let path = env::home_dir()
                .ok_or_eyre("Could not find home dir")?
                .join(".local")
                .join("share")
                .join(env!("CARGO_PKG_NAME"));
            fs::create_dir(&path)
                .accept_kind(std::io::ErrorKind::AlreadyExists)
                .wrap_err("Could not create directory to store db")?;
            let store = Store::new(path).wrap_err("Could not open database")?;
            (run_args, store)
        }
        cli::Command::Test(test_args) => {
            time::setup_mock_from_args(&test_args);
            spawn_mock_break_enforcer_interface(test_args.clone());
            #[expect(
                deprecated,
                reason = "windows only issue fixed in next rust version"
            )]
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
            (test_args.run_args, store)
        }
        cli::Command::Install(run_args) => {
            return install::add_or_modify(run_args)
        }
        cli::Command::Remove => return install::remove(),
    };

    iced::daemon(Ui::title, Ui::update, Ui::view)
        .subscription(Ui::subscription)
        .theme(Ui::theme)
        .font(Ui::FONT)
        .run_with(|| Ui::new(run_args, store))
        .wrap_err("Error running UI")
}

trait ResultAcceptKind {
    type Error;
    fn accept_kind(
        self,
        errorkind: std::io::ErrorKind,
    ) -> Result<(), Self::Error>;
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
