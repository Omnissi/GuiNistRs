use egui_extras::{Size, StripBuilder};

use crate::tests::*;

#[derive(Default)]
pub struct ConfigureTests {}

impl ConfigureTests {
    fn build_enbale_test_ui(&mut self, builder: StripBuilder<'_>) {
        builder
            .sizes(Size::remainder(), NUMBER_OF_TEST)
            .vertical(|mut strip| {
                TESTS.lock().unwrap().iter_mut().for_each(|test| {
                    strip.cell(|ui| {
                        ui.checkbox(&mut test.enable, test.name);
                    })
                });
            });
    }

    fn build_params_test_ui(&mut self, builder: StripBuilder<'_>) {
        builder
            .sizes(Size::remainder(), NUMBER_OF_TEST)
            .vertical(|mut strip| {
                TESTS.lock().unwrap().iter_mut().for_each(|test| {
                    if let Some(param) = &mut test.param {
                        strip.cell(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(test.name);
                                ui.add(
                                    egui::DragValue::new(&mut param.value)
                                        .clamp_range(param._min_value..=param._max_value),
                                );
                            });
                        });
                    }
                });
            });
    }

    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new("Configure tests")
            .open(open)
            .show(ctx, |ui| {
                StripBuilder::new(ui)
                    .sizes(Size::remainder(), 2)
                    .horizontal(|mut strip| {
                        strip.strip(|builder| self.build_enbale_test_ui(builder));
                        strip.strip(|builder| self.build_params_test_ui(builder));
                    });
            });
    }
}
