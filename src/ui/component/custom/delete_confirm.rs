use crate::app::App;
use crate::app::display_name;
use crate::ui::RED;
use crate::ui::component::dialog::draw_dialog;

pub fn draw_delete_confirm(frame: &mut ratatui::Frame<'_>, app: &App) {
    let name = app.selected_name().unwrap_or_default();
    let name = display_name(&name);
    draw_dialog(
        frame,
        46,
        7,
        RED,
        " Confirm Delete ",
        &format!("Delete connection '{name}'?\n\n  Enter confirm  ·  Esc cancel"),
    );
}
