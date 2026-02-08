use egui::{Id, Popup, PopupCloseBehavior, RectAlign, Response, SetOpenCommand};

/// Show a clipboard context menu popup below a TextEdit when it gains focus.
/// Uses egui's popup memory so the popup stays open independently of TextEdit focus,
/// preventing focus flicker between the TextEdit and popup.
///
/// - "Copy" is always enabled (copies selected text, or all text if nothing selected)
/// - "Paste" is shown only when clipboard has text (Android only)
pub fn show_clipboard_popup(_ui: &egui::Ui, response: &Response, text: &mut String) {
    let popup_id = Id::new("clipboard_popup").with(response.id);

    // Open popup when TextEdit gains focus.
    // Once open, the popup stays open via memory until closed by:
    //   - CloseOnClickOutside (user taps outside the popup)
    //   - ui.close() (after copy/paste action)
    //   - Escape key
    let set_open = if response.gained_focus() {
        Some(SetOpenCommand::Bool(true))
    } else {
        None
    };

    Popup::from_response(response)
        .id(popup_id)
        .open_memory(set_open)
        .align(RectAlign::BOTTOM_START)
        .align_alternatives(&[RectAlign::TOP_START])
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .show(|ui| {
            if ui.button("Copy").clicked() {
                let copy_text = {
                    let mut sel: Option<String> = None;
                    if let Some(state) =
                        egui::TextEdit::load_state(ui.ctx(), response.id)
                    {
                        if let Some(range) = state.cursor.char_range() {
                            let start = range.primary.index.min(range.secondary.index);
                            let end = range.primary.index.max(range.secondary.index);
                            if start != end {
                                let s: String =
                                    text.chars().skip(start).take(end - start).collect();
                                sel = Some(s);
                            }
                        }
                    }
                    sel.unwrap_or_else(|| text.clone())
                };

                #[cfg(target_os = "android")]
                {
                    let _ = crate::android_clipboard::set_text(&copy_text);
                }
                #[cfg(not(target_os = "android"))]
                {
                    ui.ctx().copy_text(copy_text);
                }
                ui.close();
            }

            #[cfg(target_os = "android")]
            let has_clip = crate::android_clipboard::has_text().unwrap_or(false);
            #[cfg(not(target_os = "android"))]
            let has_clip = false;

            if has_clip {
                if ui.button("Paste").clicked() {
                    #[cfg(target_os = "android")]
                    {
                        if let Ok(Some(clip)) = crate::android_clipboard::get_text() {
                            if let Some(state) =
                                egui::TextEdit::load_state(ui.ctx(), response.id)
                            {
                                if let Some(range) = state.cursor.char_range() {
                                    let start =
                                        range.primary.index.min(range.secondary.index);
                                    let end =
                                        range.primary.index.max(range.secondary.index);
                                    let before: String = text.chars().take(start).collect();
                                    let after: String = text.chars().skip(end).collect();
                                    *text = format!("{}{}{}", before, clip, after);
                                } else {
                                    text.push_str(&clip);
                                }
                            } else {
                                text.push_str(&clip);
                            }
                        }
                    }
                    ui.close();
                }
            }
        });
}
