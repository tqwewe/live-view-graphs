use std::{iter::Filter, time::Duration};

use lunatic::spawn_link;
use plotters::prelude::*;
use rand::SeedableRng;
use rand_distr::{DistIter, Distribution, Normal};
use rand_xorshift::XorShiftRng;
use serde::{Deserialize, Serialize};
use submillisecond::{router, Application};
use submillisecond_live_view::{maud_live_view::PreEscaped, prelude::*};

type SampleIter = Filter<DistIter<Normal<f64>, XorShiftRng, f64>, fn(&f64) -> bool>;

fn main() -> std::io::Result<()> {
    Application::new(router! {
        "/" => Chart::handler("index.html", "#app")
        // "/static" => static_router!("./static")
    })
    .serve("127.0.0.1:3000")
}

struct Chart {
    data: Vec<f64>,
    x_iter: SampleIter,
    has_started: bool,
    socket: Option<Socket>,
}

impl LiveView for Chart {
    type Events = (Start, PushData);

    fn mount(_uri: Uri, socket: Option<Socket>) -> Self {
        let norm_dist = Normal::new(500.0, 100.0).unwrap();
        let x_rand = XorShiftRng::from_seed(*b"MyFragileSeed123");
        let sample_filter = (|x: &f64| *x < 1500.0) as fn(&f64) -> bool;
        let x_iter = norm_dist.sample_iter(x_rand).filter(sample_filter);

        Chart {
            data: Vec::new(),
            x_iter,
            has_started: false,
            socket,
        }
    }

    fn render(&self) -> Rendered {
        let mut chart_string = String::new();
        {
            let drawing_area =
                SVGBackend::with_string(&mut chart_string, (1024, 768)).into_drawing_area();

            drawing_area.fill(&WHITE).unwrap();

            let mut chart = ChartBuilder::on(&drawing_area)
                .set_label_area_size(LabelAreaPosition::Left, 60)
                .set_label_area_size(LabelAreaPosition::Bottom, 60)
                .caption("Area Chart Demo", ("sans-serif", 40))
                .build_cartesian_2d(0..(self.data.len().saturating_sub(1)), 0.0..1500.0)
                .unwrap();

            chart
                .configure_mesh()
                .disable_x_mesh()
                .disable_y_mesh()
                .draw()
                .unwrap();

            chart
                .draw_series(
                    AreaSeries::new(
                        (0..).zip(self.data.iter()).map(|(x, y)| (x, *y)),
                        0.0,
                        &RED.mix(0.2),
                    )
                    .border_style(&RED),
                )
                .unwrap();
        }

        html! {
            div { (PreEscaped(chart_string)) }
            @if !self.has_started {
                button style="font-size: 24px; padding: 10px 20px; margin-left: 430px" @click=(Start) {
                    "Start"
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Start {}

impl LiveViewEvent<Start> for Chart {
    fn handle(state: &mut Self, _event: Start) {
        if !state.has_started {
            state.has_started = true;

            if let Some(socket) = state.socket.take() {
                spawn_link!(|socket, _mailbox: Mailbox<()>| {
                    loop {
                        lunatic::sleep(Duration::from_millis(10));
                        socket.send_event(PushData {}).unwrap();
                    }
                });
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PushData {}

impl LiveViewEvent<PushData> for Chart {
    fn handle(state: &mut Self, _event: PushData) {
        state.data.push(state.x_iter.next().unwrap());
    }
}
