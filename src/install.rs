use std::ops::Range;

use color_eyre::eyre::Context;
use itertools::Itertools;

use crate::cli::RunArgs;
use crate::Activity;

impl Activity {
    fn into_argument(self) -> String {
        self.description + ":" + &self.count.to_string()
    }
}

fn time_argument(window: Range<jiff::civil::Time>) -> String {
    format!(
        "{}..{}",
        window.start.strftime("%H:%M"),
        window.end.strftime("%H:%M")
    )
}

pub fn add_or_modify(args: RunArgs) -> color_eyre::Result<()> {
    let steps = service_install::install_user!()
        .current_exe()
        .unwrap()
        .service_name("reminder")
        .on_boot()
        .description("Shows reminders during break-enforcer breaks")
        .args(Itertools::intersperse(
            args.activity.into_iter().map(Activity::into_argument),
            "--activity".to_string(),
        ))
        .arg("--deadline")
        .arg(time_argument(args.window))
        .prepare_install()
        .wrap_err("Could not prepare for install")?;

    service_install::tui::install::start(steps, true).wrap_err("Could not perform install")?;
    Ok(())
}

pub fn remove() -> color_eyre::Result<()> {
    service_install::install_user!()
        .prepare_remove()
        .wrap_err("Could not prepare for removing install")?
        .remove()
        .wrap_err("Could not remove install")?;
    Ok(())
}
