mod app;
mod k8s;
mod views;

use app::KubeDashboard;
use eframe::egui;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Kubectl Dashboard"),
        ..Default::default()
    };

    eframe::run_native(
        "Kubectl Dashboard",
        options,
        Box::new(|cc| Ok(Box::new(KubeDashboard::new(cc)))),
    )
}
