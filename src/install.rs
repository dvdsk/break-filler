use std::ops::Range;

use color_eyre::eyre::Context;

use crate::cli::RunArgs;
use break_filler::Activity;

fn into_argument(activity: Activity) -> String {
    activity.description + ":" + &activity.count.to_string()
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
        .service_name(env!("CARGO_PKG_NAME"))
        .on_boot()
        .description("Shows reminders during break-enforcer breaks")
        .arg("run")
        .args(
            args.activity
                .into_iter()
                .map(into_argument)
                .flat_map(|a| ["--activity".to_string(), a]),
        )
        .arg("--window")
        .arg(time_argument(args.window))
        .arg("--load")
        .arg(args.load.to_string())
        .args(
            args.skip_when_visible
                .into_iter()
                .flat_map(|a| ["--skip-when-visible".to_string(), a]),
        )
        .overwrite_existing(true)
        .prepare_install()
        .wrap_err("Could not prepare for install")?;

    service_install::tui::install::start(steps, true)
        .wrap_err("Could not perform install")?;
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
