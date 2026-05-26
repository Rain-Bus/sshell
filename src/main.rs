use anyhow::Result;

fn main() -> Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = sshell::ui::restore_terminal();
        original_hook(panic);
    }));

    sshell::cli::run()
}
