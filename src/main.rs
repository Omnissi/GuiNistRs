#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui::plot::{Bar, BarChart, Line, LineStyle, Plot, PlotPoints, Points};
use egui::{Label, RichText, ScrollArea, TextStyle};
use egui_extras::{Size, StripBuilder};

use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use native_dialog::{FileDialog, MessageDialog, MessageType};

use statrs::function::gamma::gamma_ur;

use nistrs::prelude::*;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

#[macro_use]
extern crate lazy_static;

extern crate rayon;

mod configure_tests;
mod tests;

fn main() {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,

        #[cfg(feature = "wgpu")]
        renderer: eframe::Renderer::Wgpu,

        ..Default::default()
    };

    eframe::run_native(
        "GuiNist",
        options,
        Box::new(|cc| Box::new(GuiNist::new(cc))),
    );
}

struct GuiNist {
    p_distr: [usize; 10],
    p_p_distr: Vec<f64>,
    tresh_p_p: (f64, f64),
    result: String,

    path_to_file: String,
    n_bits: usize,
    n_blocks: usize,

    view_configure: bool,
    configure: configure_tests::ConfigureTests,

    receiver: Option<Receiver<Option<tests::ResultTestsStat>>>,
}

fn thread_test(
    path: String,
    n_bits: usize,
    n_blocks: usize,
    sender: Sender<Option<tests::ResultTestsStat>>,
) {
    let mut file = match File::open(&path) {
        Ok(v) => v,
        Err(e) => {
            MessageDialog::new()
                .set_type(MessageType::Error)
                .set_title("Error open file!")
                .set_text(&format!("Can't open file: {}", e))
                .show_alert()
                .unwrap();
            sender.send(None).unwrap();
            return;
        }
    };

    let mut stat: tests::ResultTestsStat = Default::default();

    let all_time = Instant::now();
    let mut sum_time = u128::default();
    for i in 0..n_blocks {
        let local_time = Instant::now();

        if *tests::STOP_FLAG.lock().unwrap() {
            sender.send(None).unwrap();
            return;
        }

        let mut buf = vec![0_u8; n_bits / u8::BITS as usize];
        if let Err(e) = file.read(&mut buf[..]) {
            MessageDialog::new()
                .set_title("Error!")
                .set_text(format!("Error read sequence: {}", e).as_str())
                .show_alert()
                .unwrap();
        };

        let data = BitsData::from_binary(buf);

        let tests = *tests::TESTS.lock().unwrap();

        stat.par_iter_mut().zip(tests).for_each(|(ls, nist)| {
            if !nist.enable || *tests::STOP_FLAG.lock().unwrap() {
                return;
            }

            match (nist.test_cb)(&data, nist.param) {
                Ok(v) => {
                    if ls.len() != v.len() {
                        ls.resize(v.len(), tests::TestStat::default());
                    }

                    v.into_iter().zip(&mut *ls).for_each(|((pass, p_val), st)| {
                        if pass {
                            st.ratio += 1_f64;
                        }

                        let index = ((p_val * 10_f64).floor() as usize).min(9);
                        st.p_distr[index] += 1;
                    });
                }
                Err(Some(e)) => {
                    eprintln!("ERROR {}::{}", nist.name, e)
                }
                Err(None) => {}
            }
        });

        sum_time += local_time.elapsed().as_millis();

        *tests::COMPLETE_BLOCKS.lock().unwrap() += 1;
        *tests::AVR_TIME_TO_BLOCK.lock().unwrap() = ((sum_time as usize) / (i + 1)) as u128;

        *tests::TOTAL_TIME.lock().unwrap() = all_time.elapsed();
    }

    sender.send(Some(stat)).unwrap();
}

fn start_thread(
    path: String,
    n_bits: usize,
    n_blocks: usize,
) -> Receiver<Option<tests::ResultTestsStat>> {
    let (sender, receiver) = channel::<Option<tests::ResultTestsStat>>();

    *tests::STOP_FLAG.lock().unwrap() = false;
    *tests::COMPLETE_BLOCKS.lock().unwrap() = 0;
    *tests::AVR_TIME_TO_BLOCK.lock().unwrap() = 0;
    *tests::NUMBERS_OF_BLOCKS.lock().unwrap() = n_blocks;
    *tests::TOTAL_TIME.lock().unwrap() = Duration::new(0, 0);

    std::thread::spawn(move || {
        thread_test(path, n_bits, n_blocks, sender);
    });

    receiver
}

fn duration_string(dur: Duration) -> String {
    let mut msec = dur.as_millis();
    let hours = msec / 3_600_000;
    msec %= 3_600_000;

    let minutes = msec / 60_000;
    msec %= 60_000;

    let sec = msec / 1_000;
    msec %= 1_000;

    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, sec, msec)
}

impl GuiNist {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            p_distr: [0; 10],
            p_p_distr: vec![],
            tresh_p_p: (f64::default(), f64::default()),
            path_to_file: String::new(),
            result: String::new(),
            n_bits: 1_000_000,
            n_blocks: 1_000,
            receiver: None,
            configure: configure_tests::ConfigureTests::default(),
            view_configure: false,
        }
    }

    fn calc_stat(&mut self, stat: tests::ResultTestsStat) {
        if *tests::STOP_FLAG.lock().unwrap() {
            return;
        }

        self.result.clear();
        self.p_distr.fill(0_usize);
        self.tresh_p_p = (f64::MAX, f64::MIN);
        self.p_p_distr.clear();

        self.result = String::with_capacity(18700);

        let mut failed = usize::default();
        let mut sum_min_p = 0_f64;
        let mut sum_max_p = 0_f64;
        let mut number_of_test = 0_usize;

        let tests = *tests::TESTS.lock().unwrap();

        tests.into_iter().zip(stat).for_each(|(test, st)| {
            if !test.enable {
                return;
            }

            const PA: f64 = 1_f64 - TEST_THRESHOLD;

            for test_st in st {
                let sample_size = test_st.p_distr.iter().sum::<usize>() as f64;
                let p_range = 3_f64 * (PA * (1_f64 - PA) / sample_size).sqrt();
                let min_p = PA - p_range;
                let max_p = PA + p_range;

                sum_min_p += min_p;
                sum_max_p += max_p;
                number_of_test += 1;

                let c_tmp = (sample_size / 10_f64).floor() as isize;

                let mut chi_squad = f64::default();
                self.p_distr
                    .iter_mut()
                    .zip(test_st.p_distr)
                    .for_each(|(out, p)| {
                        self.result += &format!("{:>5}", p);
                        *out += p;

                        chi_squad += (p as isize - c_tmp).pow(2) as f64;
                    });

                chi_squad /= c_tmp as f64;
                if chi_squad > 0_f64 && !chi_squad.is_infinite() {
                    chi_squad = gamma_ur(9.0 / 2.0, chi_squad / 2.0);
                } else {
                    chi_squad = 0_f64;
                }

                let ratio = test_st.ratio / sample_size;

                let is_rand;
                if ratio < min_p || ratio > max_p || chi_squad < 0.0001 {
                    is_rand = false;
                    failed += 1;
                } else {
                    is_rand = true;
                }

                self.result += &format!("{:>12.5}{:>12.5}", chi_squad, ratio);

                if is_rand {
                    self.result += "   ";
                } else {
                    self.result += " * ";
                }

                self.result += test.name;
                self.result += "\n";

                self.p_p_distr.push(ratio);
            }
        });

        self.tresh_p_p = (
            sum_min_p / number_of_test as f64,
            sum_max_p / number_of_test as f64,
        );

        self.result += "\n";
        write!(self.result, "Number of failed tests (*): {}", failed).unwrap();

        let report_file = format!("{}.txt", self.path_to_file);
        match File::create(report_file) {
            Ok(mut f) => {
                f.write_all(self.result.as_bytes())
                    .expect("Error write result");
            }
            Err(e) => {
                eprintln!("Can't open file: {}", e);
            }
        };
    }
}

impl eframe::App for GuiNist {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.configure.show(ctx, &mut self.view_configure);

        egui::CentralPanel::default().show(ctx, |ui| {
            StripBuilder::new(ui)
                .sizes(Size::relative(0.5), 2)
                .horizontal(|mut strip| {
                    strip.strip(|builder| {
                        self.build_plot_ui(builder);
                    });

                    let enabled = self.receiver.is_some();

                    strip.strip(|builder| {
                        builder.sizes(Size::relative(0.5), 2).vertical(|mut strip| {
                            strip.cell(|ui| {
                                ScrollArea::vertical().show(ui, |ui| {
                                    ui.add(
                                        Label::new(
                                            RichText::new(&self.result)
                                                .text_style(TextStyle::Monospace),
                                        )
                                        .wrap(false),
                                    );
                                });
                            });

                            strip.strip(|builder| {
                                self.build_control_ui(builder, ctx, enabled);
                            });
                        });
                    });

                    if let Some(recv) = &self.receiver {
                        if let Ok(v) = recv.recv_timeout(Duration::from_millis(1)) {
                            if let Some(res) = v {
                                self.calc_stat(res);
                            }
                            self.receiver = None;
                        }
                    }
                });
        });
    }
}

impl GuiNist {
    fn build_plot_ui(&mut self, builder: StripBuilder<'_>) {
        builder.sizes(Size::relative(0.5), 2).vertical(|mut strip| {
            let bars = BarChart::new(
                self.p_distr
                    .iter()
                    .enumerate()
                    .map(|(x, y)| Bar::new(x as f64, *y as f64))
                    .collect(),
            );

            let points = Points::new(PlotPoints::new(
                self.p_p_distr
                    .iter()
                    .enumerate()
                    .map(|(x, y)| [x as f64, *y])
                    .collect(),
            ));

            let line_max_theshold: PlotPoints = (0..self.p_p_distr.len())
                .map(|i| [i as f64, self.tresh_p_p.1])
                .collect();

            let line_min_theshold: PlotPoints = (0..self.p_p_distr.len())
                .map(|i| [i as f64, self.tresh_p_p.0])
                .collect();

            strip.cell(|ui| {
                Plot::new("P-value").show(ui, |plot_ui| plot_ui.bar_chart(bars));
            });

            strip.cell(|ui| {
                Plot::new("P from P-value").show(ui, |plot_ui| {
                    plot_ui.points(points);
                    plot_ui.line(
                        Line::new(line_min_theshold).style(LineStyle::Dashed { length: 0.5 }),
                    );
                    plot_ui.line(
                        Line::new(line_max_theshold).style(LineStyle::Dashed { length: 0.5 }),
                    );
                });
            });
        });
    }

    fn build_control_ui(&mut self, builder: StripBuilder<'_>, ctx: &egui::Context, enabled: bool) {
        builder
            .sizes(Size::relative(0.1), 10)
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("File: ");
                        ui.add_enabled(
                            !enabled,
                            egui::TextEdit::singleline(&mut self.path_to_file),
                        );
                        if ui
                            .add_enabled(!enabled, egui::Button::new("Open file"))
                            .clicked()
                        {
                            let path = FileDialog::new().show_open_single_file().unwrap();
                            if let Some(v) = path {
                                self.path_to_file = v.into_os_string().into_string().unwrap();
                                let file = File::open(&self.path_to_file).unwrap();
                                let file_size = file.metadata().unwrap().len() as usize * 8;
                                self.n_blocks = file_size / self.n_bits;
                            }
                        }
                    });
                });

                strip.cell(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Bits: ");
                        ui.add_enabled(!enabled, egui::DragValue::new(&mut self.n_bits));
                    });
                });

                strip.cell(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Blocks: ");
                        ui.add_enabled(!enabled, egui::DragValue::new(&mut self.n_blocks));
                    });
                });

                strip.cell(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!enabled, egui::Button::new("Start"))
                            .clicked()
                        {
                            self.receiver = Some(start_thread(
                                self.path_to_file.clone(),
                                self.n_bits,
                                self.n_blocks,
                            ));
                        };

                        if ui.add_enabled(enabled, egui::Button::new("Stop")).clicked() {
                            *tests::STOP_FLAG.lock().unwrap() = true;
                        }

                        if ui
                            .add_enabled(!enabled, egui::Button::new("Configure tests"))
                            .clicked()
                        {
                            self.view_configure = true;
                            self.configure.show(ctx, &mut self.view_configure);
                        }
                    });
                });

                let avr = *tests::AVR_TIME_TO_BLOCK.lock().unwrap() as u64;
                let blocks = *tests::COMPLETE_BLOCKS.lock().unwrap() as u64;
                let n_blocks = *tests::NUMBERS_OF_BLOCKS.lock().unwrap() as u64;

                let progress = blocks as f32 / n_blocks as f32;

                strip.cell(|ui| {
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .show_percentage()
                            .animate(enabled),
                    );
                });

                strip.cell(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Avr time to block: {}",
                            duration_string(Duration::from_millis(avr))
                        ))
                        .text_style(TextStyle::Monospace),
                    );
                });

                strip.cell(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "        Time left: {}",
                            duration_string(Duration::from_millis(
                                avr * (n_blocks - blocks)
                            ))
                        ))
                        .text_style(TextStyle::Monospace),
                    );
                });

                strip.cell(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "      Time passed: {}",
                            duration_string(*tests::TOTAL_TIME.lock().unwrap())
                        ))
                        .text_style(TextStyle::Monospace),
                    );
                });
            });
    }
}
