use crate::worker::{Channel, Item};
use chrono::{TimeZone, Utc};
use eframe::epaint::text::{LayoutJob, TextWrapping};
use egui::{Frame, Hyperlink, Label, RichText, TextFormat};
use unicode_truncate::UnicodeTruncateStr;

pub fn truncate(string: &str, width: usize, trim_char: Option<&str>) -> String {
    let (truncated, width_t) = string.unicode_truncate(width);
    let mut truncated_string = truncated.to_string();
    if width == width_t {
        truncated_string.push_str(trim_char.unwrap_or("…"));
    }
    truncated_string
}

pub fn timestamp_to_human_readable(timestamp: i64) -> String {
    let dt = match Utc.timestamp_millis_opt(timestamp * 1000).earliest() {
        Some(dt) => dt,
        None => return String::from("???"),
    };

    let formatted = dt.format("%d %b %Y");

    formatted.to_string()
}

pub fn channel_card(ui: &mut egui::Ui, channel: &Channel) {
    Frame {
        fill: egui::Color32::from_rgb(0, 50, 0),
        inner_margin: egui::Margin::same(4.0),
        ..Default::default()
    }
    .show(ui, |ui| {
        ui.set_width(ui.available_width());
        if let Some(title) = &channel.title {
            ui.label(RichText::new(title).strong().heading());
        } else {
            ui.label(RichText::new("<no title>").strong().heading());
        }
    });
}

pub fn feed_card(ui: &mut egui::Ui, item: &Item) {
    Frame {
        fill: egui::Color32::from_rgb(0, 50, 0),
        inner_margin: egui::Margin::same(4.0),
        ..Default::default()
    }
    .show(ui, |ui| {
        ui.set_width(ui.available_width());
        if let Some(title) = &item.title {
            let mut job = LayoutJob::single_section(title.to_string(), TextFormat::default());
            job.wrap = TextWrapping {
                max_rows: 1,
                break_anywhere: true,
                overflow_character: Some('…'),
                ..Default::default()
            };
            ui.add(Hyperlink::from_label_and_url(job, &item.link));
        } else {
            ui.add(Label::new(RichText::new("<no title>")));
        }
        ui.horizontal(|ui| {
            ui.label(timestamp_to_human_readable(item.published));
            ui.add_space(5.);
            if let Some(channel_title) = &item.channel_title {
                ui.label(truncate(channel_title, 25, None));
            }
        });
    });
}
