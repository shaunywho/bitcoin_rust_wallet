// egui::Window::new("Do you want to quit?")
// .collapsible(false)
// .resizable(false)
// .show(ctx, |ui| {
//     ui.horizontal(|ui| {
//         if ui.button("No").clicked() {
//             self.show_confirmation_dialog = false;
//             self.allowed_to_close = false;
//         }

//         if ui.button("Yes").clicked() {
//             self.show_confirmation_dialog = false;
//             self.allowed_to_close = true;
//             ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
//         }
//     });
// });
