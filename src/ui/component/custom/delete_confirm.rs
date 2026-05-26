use crate::app::App;
use crate::ui::RED;
use crate::ui::component::dialog::draw_dialog;

pub fn draw_delete_confirm(frame: &mut ratatui::Frame<'_>, app: &App) {
    let name = app.selected_name().unwrap_or_default();
    draw_dialog(
        frame,
        46,
        7,
        RED,
        " Confirm Delete ",
        &format!("Delete connection '{name}'?\n\n  Enter confirm  ·  Esc cancel"),
    );
}
