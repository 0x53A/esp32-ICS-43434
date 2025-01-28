use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        run_and_return: true,
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    log::info!("Opening window…");
    eframe::run_native(
        "My Window",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp { }))),
    )
}

struct MyApp {

}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let label_text = "Hello World!";
            ui.label(label_text);

            if ui.button("Close").clicked() {
                log::info!("Pressed Close button");
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}