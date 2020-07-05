use criterion_plot::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Child;
mod summary;
use self::summary::*;
use super::{
    FilledCurve as FilledArea, Line, LineCurve, PlotContext, PlotData, Plotter, PlottingBackend,
    Points as PointPlot, Rectangle, VerticalLine,
};
use crate::estimate::Statistic;
use crate::format;
use crate::model::Benchmark;
use crate::plot::Size;
use crate::report::{BenchmarkId, ValueType};
use crate::stats::univariate::Sample;
use crate::value_formatter::ValueFormatter;

fn gnuplot_escape(string: &str) -> String {
    string.replace("_", "\\_").replace("'", "''")
}

static DEFAULT_FONT: &str = "Helvetica";
static SIZE: Size = Size(1280, 720);

const LINEWIDTH: LineWidth = LineWidth(2.);
const POINT_SIZE: PointSize = PointSize(0.75);

const DARK_BLUE: Color = Color::Rgb(31, 120, 180);
const DARK_ORANGE: Color = Color::Rgb(255, 127, 0);
const DARK_RED: Color = Color::Rgb(227, 26, 28);

macro_rules! to_lines {
    ($i:ident) => {
        Lines {
            x: &[$i.start.x, $i.end.x],
            y: &[$i.start.y, $i.end.y],
        }
    };
    ($i:ident, $max_y:expr) => {
        Lines {
            x: &[$i.x, $i.x],
            y: &[0.0, $max_y],
        }
    };
}

fn debug_script(path: &Path, figure: &Figure) {
    if crate::debug_enabled() {
        let script_path = path.with_extension("gnuplot");
        info!("Writing gnuplot script to {:?}", script_path);
        let result = figure.save(&script_path);
        if let Err(e) = result {
            error!("Failed to write debug output: {}", e);
        }
    }
}

impl From<Size> for criterion_plot::Size {
    fn from(other: Size) -> Self {
        let Size(width, height) = other;
        Self(width, height)
    }
}

#[derive(Default)]
pub struct Gnuplot {
    process_list: Vec<Child>,
}
impl Gnuplot {
    pub fn new() -> Gnuplot {
        Gnuplot {
            process_list: vec![],
        }
    }
}

impl Plotter for Gnuplot {
    fn pdf(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn pdf_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn pdf_comparison(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn pdf_comparison_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn iteration_times_comparison(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn iteration_times_comparison_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn iteration_times_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn iteration_times(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn regression(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn regression_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn regression_comparison(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }
    fn regression_comparison_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn abs_distributions(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn rel_distributions(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn t_test(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn line_comparison(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_benchmarks: &[(&BenchmarkId, &Benchmark)],
        value_type: ValueType,
    ) {
        let path = ctx.line_comparison_path();
        self.process_list.push(line_comparison(
            formatter,
            ctx.id.as_title(),
            all_benchmarks,
            path,
            value_type,
            ctx.context.plot_config.summary_scale,
        ));
    }

    fn violin(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
    ) {
        let violin_path = ctx.violin_path();

        self.process_list.push(violin(
            formatter,
            ctx.id.as_title(),
            all_curves,
            violin_path,
            ctx.context.plot_config.summary_scale,
        ));
    }

    fn wait(&mut self) {
        let start = std::time::Instant::now();
        let child_count = self.process_list.len();
        for child in self.process_list.drain(..) {
            match child.wait_with_output() {
                Ok(ref out) if out.status.success() => {}
                Ok(out) => error!("Error in Gnuplot: {}", String::from_utf8_lossy(&out.stderr)),
                Err(e) => error!("Got IO error while waiting for Gnuplot to complete: {}", e),
            }
        }
        let elapsed = &start.elapsed();
        info!(
            "Waiting for {} gnuplot processes took {}",
            child_count,
            format::time(crate::DurationExt::to_nanos(elapsed) as f64)
        );
    }
}
impl PlottingBackend for Gnuplot {
    fn abs_distribution(
        &mut self,
        id: &BenchmarkId,
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        x_unit: &str,
        distribution_curve: LineCurve,
        bootstrap_area: FilledArea,
        point_estimate: Line,
    ) {
        let xs_sample = Sample::new(distribution_curve.xs);

        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .set(Title(format!(
                "{}: {}",
                gnuplot_escape(id.as_title()),
                statistic
            )))
            .configure(Axis::BottomX, |a| {
                a.set(Label(format!("Average time ({})", x_unit)))
                    .set(Range::Limits(xs_sample.min(), xs_sample.max()))
            })
            .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Outside(Vertical::Top, Horizontal::Right))
            })
            .plot(
                Lines {
                    x: distribution_curve.xs,
                    y: distribution_curve.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(LINEWIDTH)
                        .set(Label("Bootstrap distribution"))
                        .set(LineType::Solid)
                },
            )
            .plot(
                FilledCurve {
                    x: bootstrap_area.xs,
                    y1: bootstrap_area.ys_1,
                    y2: bootstrap_area.ys_2,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Confidence interval"))
                        .set(Opacity(0.25))
                },
            )
            .plot(
                Lines {
                    x: &[point_estimate.start.x, point_estimate.end.x],
                    y: &[point_estimate.start.y, point_estimate.end.y],
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(LINEWIDTH)
                        .set(Label("Point estimate"))
                        .set(LineType::Dash)
                },
            );

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap());
    }

    fn rel_distribution(
        &mut self,
        id: &BenchmarkId,
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        distribution_curve: LineCurve,
        confidence_interval: FilledArea,
        point_estimate: Line,
        noise_threshold: Rectangle,
    ) {
        let xs_ = Sample::new(&distribution_curve.xs);
        let x_min = xs_.min();
        let x_max = xs_.max();

        let mut figure = Figure::new();

        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Outside(Vertical::Top, Horizontal::Right))
            })
            .set(Title(format!(
                "{}: {}",
                gnuplot_escape(id.as_title()),
                statistic
            )))
            .configure(Axis::BottomX, |a| {
                a.set(Label("Relative change (%)"))
                    .set(Range::Limits(x_min * 100., x_max * 100.))
                    .set(ScaleFactor(100.))
            })
            .plot(
                Lines {
                    x: distribution_curve.xs,
                    y: distribution_curve.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(LINEWIDTH)
                        .set(Label("Bootstrap distribution"))
                        .set(LineType::Solid)
                },
            )
            .plot(
                FilledCurve {
                    x: confidence_interval.xs,
                    y1: confidence_interval.ys_1,
                    y2: confidence_interval.ys_2,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Confidence interval"))
                        .set(Opacity(0.25))
                },
            )
            .plot(to_lines!(point_estimate), |c| {
                c.set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(Label("Point estimate"))
                    .set(LineType::Dash)
            })
            .plot(
                FilledCurve {
                    x: &[noise_threshold.left, noise_threshold.right],
                    y1: &[noise_threshold.bottom, noise_threshold.bottom],
                    y2: &[noise_threshold.top, noise_threshold.top],
                },
                |c| {
                    c.set(Axes::BottomXRightY)
                        .set(DARK_RED)
                        .set(Label("Noise threshold"))
                        .set(Opacity(0.1))
                },
            );

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn iteration_times(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        file_path: PathBuf,

        unit: &str,
        is_thumbnail: bool,
        current_times: PointPlot,
        base_times: Option<PointPlot>,
    ) {
        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::BottomX, |a| {
                a.configure(Grid::Major, |g| g.show()).set(Label("Sample"))
            })
            .configure(Axis::LeftY, |a| {
                a.configure(Grid::Major, |g| g.show())
                    .set(Label(format!("Average Iteration Time ({})", unit)))
            })
            .plot(
                Points {
                    x: current_times.xs,
                    y: current_times.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Current"))
                        .set(PointSize(0.5))
                        .set(PointType::FilledCircle)
                },
            );

        if let Some(base_times) = base_times {
            figure.plot(
                Points {
                    x: base_times.xs,
                    y: base_times.ys,
                },
                |c| {
                    c.set(DARK_RED)
                        .set(Label("Base"))
                        .set(PointSize(0.5))
                        .set(PointType::FilledCircle)
                },
            );
        }

        if !is_thumbnail {
            figure.set(Title(gnuplot_escape(id.as_title())));
            figure.configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Inside(Vertical::Top, Horizontal::Left))
            });
        } else {
            figure.configure(Key, |k| k.hide());
        }

        debug_script(&file_path, &figure);
        self.process_list
            .push(figure.set(Output(file_path)).draw().unwrap())
    }

    fn regression(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        file_path: PathBuf,
        is_thumbnail: bool,
        x_label: &str,
        x_scale: f64,
        unit: &str,
        sample: PointPlot,
        regression: Line,
        confidence_interval: FilledArea,
    ) {
        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::BottomX, |a| {
                a.configure(Grid::Major, |g| g.show())
                    .set(Label(x_label.to_owned()))
                    .set(ScaleFactor(x_scale))
            })
            .configure(Axis::LeftY, |a| {
                a.configure(Grid::Major, |g| g.show())
                    .set(Label(format!("Total sample time ({})", unit)))
            })
            .plot(
                Points {
                    x: sample.xs,
                    y: sample.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Sample"))
                        .set(PointSize(0.5))
                        .set(PointType::FilledCircle)
                },
            )
            .plot(to_lines!(regression), |c| {
                c.set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(Label("Linear regression"))
                    .set(LineType::Solid)
            })
            .plot(
                FilledCurve {
                    x: confidence_interval.xs,
                    y1: confidence_interval.ys_1,
                    y2: confidence_interval.ys_2,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Confidence interval"))
                        .set(Opacity(0.25))
                },
            );

        if !is_thumbnail {
            figure.set(Title(gnuplot_escape(id.as_title())));
            figure.configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Inside(Vertical::Top, Horizontal::Left))
            });
        } else {
            figure.configure(Key, |k| k.hide());
        }

        debug_script(&file_path, &figure);
        self.process_list
            .push(figure.set(Output(file_path)).draw().unwrap())
    }

    fn regression_comparison(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        is_thumbnail: bool,
        x_label: &str,
        x_scale: f64,
        unit: &str,
        current_regression: Line,
        current_confidence_interval: FilledArea,
        base_regression: Line,
        base_confidence_interval: FilledArea,
    ) {
        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::BottomX, |a| {
                a.configure(Grid::Major, |g| g.show())
                    .set(Label(x_label.to_owned()))
                    .set(ScaleFactor(x_scale))
            })
            .configure(Axis::LeftY, |a| {
                a.configure(Grid::Major, |g| g.show())
                    .set(Label(format!("Total sample time ({})", unit)))
            })
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Inside(Vertical::Top, Horizontal::Left))
            })
            .plot(
                FilledCurve {
                    x: base_confidence_interval.xs,
                    y1: base_confidence_interval.ys_1,
                    y2: base_confidence_interval.ys_2,
                },
                |c| c.set(DARK_RED).set(Opacity(0.25)),
            )
            .plot(
                FilledCurve {
                    x: current_confidence_interval.xs,
                    y1: current_confidence_interval.ys_1,
                    y2: current_confidence_interval.ys_2,
                },
                |c| c.set(DARK_BLUE).set(Opacity(0.25)),
            )
            .plot(to_lines!(base_regression), |c| {
                c.set(DARK_RED)
                    .set(LINEWIDTH)
                    .set(Label("Base sample"))
                    .set(LineType::Solid)
            })
            .plot(to_lines!(current_regression), |c| {
                c.set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(Label("New sample"))
                    .set(LineType::Solid)
            });

        if is_thumbnail {
            figure.configure(Key, |k| k.hide());
        } else {
            figure.set(Title(gnuplot_escape(id.as_title())));
        }

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn pdf_full(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        unit: &str,
        y_label: &str,
        y_scale: f64,
        max_iters: f64,
        pdf: FilledArea,
        mean: VerticalLine,
        fences: (VerticalLine, VerticalLine, VerticalLine, VerticalLine),
        points: (PointPlot, PointPlot, PointPlot),
    ) {
        let (low_severe, low_mild, high_mild, high_severe) = fences;
        let (not_outlier, mild, severe) = points;

        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::BottomX, |a| {
                let xs_ = Sample::new(&pdf.xs);
                a.set(Label(format!("Average time ({})", unit)))
                    .set(Range::Limits(xs_.min(), xs_.max()))
            })
            .configure(Axis::LeftY, |a| {
                a.set(Label(y_label.to_owned()))
                    .set(Range::Limits(0., max_iters * y_scale))
                    .set(ScaleFactor(y_scale))
            })
            .configure(Axis::RightY, |a| a.set(Label("Density (a.u.)")))
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Outside(Vertical::Top, Horizontal::Right))
            })
            .plot(
                FilledCurve {
                    x: pdf.xs,
                    y1: pdf.ys_1,
                    y2: pdf.ys_2,
                },
                |c| {
                    c.set(Axes::BottomXRightY)
                        .set(DARK_BLUE)
                        .set(Label("PDF"))
                        .set(Opacity(0.25))
                },
            )
            .plot(to_lines!(mean, max_iters), |c| {
                c.set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(LineType::Dash)
                    .set(Label("Mean"))
            })
            .plot(
                Points {
                    x: not_outlier.xs,
                    y: not_outlier.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("\"Clean\" sample"))
                        .set(PointType::FilledCircle)
                        .set(POINT_SIZE)
                },
            )
            .plot(
                Points {
                    x: mild.xs,
                    y: mild.ys,
                },
                |c| {
                    c.set(DARK_ORANGE)
                        .set(Label("Mild outliers"))
                        .set(POINT_SIZE)
                        .set(PointType::FilledCircle)
                },
            )
            .plot(
                Points {
                    x: severe.xs,
                    y: severe.ys,
                },
                |c| {
                    c.set(DARK_RED)
                        .set(Label("Severe outliers"))
                        .set(POINT_SIZE)
                        .set(PointType::FilledCircle)
                },
            )
            .plot(to_lines!(low_mild, max_iters), |c| {
                c.set(DARK_ORANGE).set(LINEWIDTH).set(LineType::Dash)
            })
            .plot(to_lines!(high_mild, max_iters), |c| {
                c.set(DARK_ORANGE).set(LINEWIDTH).set(LineType::Dash)
            })
            .plot(to_lines!(low_severe, max_iters), |c| {
                c.set(DARK_RED).set(LINEWIDTH).set(LineType::Dash)
            })
            .plot(to_lines!(high_severe, max_iters), |c| {
                c.set(DARK_RED).set(LINEWIDTH).set(LineType::Dash)
            });
        figure.set(Title(gnuplot_escape(id.as_title())));

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn pdf_thumbnail(
        &mut self,
        size: Option<Size>,
        path: PathBuf,
        unit: &str,
        mean: Line,
        pdf: FilledArea,
    ) {
        let xs_ = Sample::new(pdf.xs);
        let ys_ = Sample::new(pdf.ys_1);
        let y_limit = ys_.max() * 1.1;

        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::BottomX, |a| {
                a.set(Label(format!("Average time ({})", unit)))
                    .set(Range::Limits(xs_.min(), xs_.max()))
            })
            .configure(Axis::LeftY, |a| {
                a.set(Label("Density (a.u.)"))
                    .set(Range::Limits(0., y_limit))
            })
            .configure(Axis::RightY, |a| a.hide())
            .configure(Key, |k| k.hide())
            .plot(
                FilledCurve {
                    x: pdf.xs,
                    y1: pdf.ys_1,
                    y2: pdf.ys_2,
                },
                |c| {
                    c.set(Axes::BottomXRightY)
                        .set(DARK_BLUE)
                        .set(Label("PDF"))
                        .set(Opacity(0.25))
                },
            )
            .plot(to_lines!(mean), |c| {
                c.set(DARK_BLUE).set(LINEWIDTH).set(Label("Mean"))
            });

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn pdf_comparison(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        is_thumbnail: bool,
        unit: &str,
        current_mean: Line,
        current_pdf: FilledArea,
        base_mean: Line,
        base_pdf: FilledArea,
    ) {
        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::BottomX, |a| {
                a.set(Label(format!("Average time ({})", unit)))
            })
            .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
            .configure(Axis::RightY, |a| a.hide())
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Outside(Vertical::Top, Horizontal::Right))
            })
            .plot(
                FilledCurve {
                    x: base_pdf.xs,
                    y1: base_pdf.ys_1,
                    y2: base_pdf.ys_2,
                },
                |c| c.set(DARK_RED).set(Label("Base PDF")).set(Opacity(0.5)),
            )
            .plot(to_lines!(base_mean), |c| {
                c.set(DARK_RED).set(Label("Base Mean")).set(LINEWIDTH)
            })
            .plot(
                FilledCurve {
                    x: current_pdf.xs,
                    y1: current_pdf.ys_1,
                    y2: current_pdf.ys_2,
                },
                |c| c.set(DARK_BLUE).set(Label("New PDF")).set(Opacity(0.5)),
            )
            .plot(to_lines!(current_mean), |c| {
                c.set(DARK_BLUE).set(Label("New Mean")).set(LINEWIDTH)
            });

        if is_thumbnail {
            figure.configure(Key, |k| k.hide());
        } else {
            figure.set(Title(gnuplot_escape(id.as_title())));
        }
        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn t_test(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        t: VerticalLine,
        t_distribution: FilledArea,
    ) {
        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .set(Title(format!(
                "{}: Welch t test",
                gnuplot_escape(id.as_title())
            )))
            .configure(Axis::BottomX, |a| a.set(Label("t score")))
            .configure(Axis::LeftY, |a| a.set(Label("Density")))
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Outside(Vertical::Top, Horizontal::Right))
            })
            .plot(
                FilledCurve {
                    x: t_distribution.xs,
                    y1: t_distribution.ys_1,
                    y2: t_distribution.ys_2,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("t distribution"))
                        .set(Opacity(0.25))
                },
            )
            .plot(to_lines!(t, 1.0), |c| {
                c.set(Axes::BottomXRightY)
                    .set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(Label("t statistic"))
                    .set(LineType::Solid)
            });

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn wait(&mut self) {
        Plotter::wait(self)
    }
}
