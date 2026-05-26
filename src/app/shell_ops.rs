use anyhow::Result;

use super::{App, Mode};

impl App {
    pub fn refresh_local_shells(&mut self) -> Result<()> {
        self.enter_shell_import();
        Ok(())
    }

    pub fn enter_shell_import(&mut self) {
        self.session.shell_import.candidates = self.config.local_shell_candidates();
        self.session.shell_import.selected =
            vec![false; self.session.shell_import.candidates.len()];
        self.session.shell_import.cursor = 0;
        self.session.mode = Mode::ShellImport;
    }

    pub fn import_selected_shells(&mut self) -> Result<usize> {
        let picked: Vec<_> = self
            .session
            .shell_import
            .candidates
            .iter()
            .zip(&self.session.shell_import.selected)
            .filter_map(|(item, selected)| {
                (*selected && item.conflict.is_none()).then_some(item.clone())
            })
            .collect();
        let mut count = 0;
        for candidate in &picked {
            self.config.add_local_shell(candidate)?;
            count += 1;
        }
        if count > 0 {
            self.config.save()?;
        }
        self.session.mode = Mode::Home;
        Ok(count)
    }
}
