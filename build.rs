#![allow(dead_code)]

#[path = "./src/args/cli.rs"]
mod cmd;
#[path = "./src/error.rs"]
mod error;
#[path = "./src/formats/mod.rs"]
mod formats;
#[path = "./src/args/internal.rs"]
mod internal;
#[path = "./src/utils.rs"]
mod utils;

#[cfg(not(debug_assertions))]
fn main() {
    let res = save_man_pages();
    if let Err(err) = res {
        eprintln!("Failed to create all man pages\n{err}");
    }
}

#[cfg(not(debug_assertions))]
fn save_man_pages() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME")?;
    let man_dir = std::path::Path::new(&home).join(".local/share/man/man1");

    let man_pages = std::fs::read_dir(std::path::Path::new("./man/man/"))?;
    for page in man_pages {
        let page = page?;

        std::fs::copy(page.path(), man_dir.join(page.file_name()))?;
    }

    Ok(())
}

#[cfg(debug_assertions)]
fn main() {
    use clap::CommandFactory;

    let cmd = cmd::CliArgs::command();
    let cli_name = cmd.get_name();
    let path = &format!("./man/man/{cli_name}.1");
    debug_assert!(
            std::path::Path::new(path).exists(),
            "The man page for the top level command '{cli_name}' doesn't exists. Create a man page giving an overview of this tool. The man page should be stored at '{path}'");

    let subcmds = cmd.get_subcommands().collect::<Vec<_>>();
    for cmd in subcmds {
        let cmd_name = cmd.get_name();
        let path_str = &format!("./man/man/{cli_name}-{cmd_name}.1");
        let path = std::path::Path::new(path_str);
        debug_assert!(
            path.exists(),
            "The man page for the command with the name '{cmd_name}' doesn't exists. Create a man page explaining what this command does and how to use it. The man page should be stored at '{path_str}'");

        let path_str_ronn = &format!("./man/ronn/{cli_name}-{cmd_name}.1.ronn");
        let path_ronn = std::path::Path::new(path_str_ronn);
        let man_page_ronn = std::fs::read_to_string(path_ronn).unwrap();
        let heading = man_page_ronn.lines().next().unwrap();

        // archwiki-rs-completions -- Generate scripts for shell auto-completion
        let cmd_short_desc = cmd.get_about().unwrap();
        let heading_expected = format!("{cli_name}-{cmd_name} -- {cmd_short_desc}");
        debug_assert_eq!(heading, heading_expected, "The heading of the man page for the command {cmd_name} is differs from the help message for the same command.")
    }
}
