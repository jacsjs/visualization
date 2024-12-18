
mod srp_analysis;

use std::collections::BTreeMap;

use srp_analysis::*;
use axum::{
    extract,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use askama::Template;
use charming::HtmlRenderer;
use serde::{Deserialize, Serialize};

#[macro_use]
extern crate lazy_static;

fn srp_analysis_example_setup() -> Vec<Task> {

     // example task set
    // Task T1
    // Lowest priority, no resource usage
    // Single trace with WCET of 10
    let t1 = Task {
        id: "T1".to_string(),
        prio: 1,
        deadline: 100,
        inter_arrival: 100,
        trace: Trace {
            id: "T1".to_string(),
            start: 0,
            end: 10,
            inner: vec![],
        },
    };

    // Task T2
    // Middle priority
    // Two traces
    let t2 = Task {
        id: "T2".to_string(),
        prio: 2,
        deadline: 200,
        inter_arrival: 200,
        trace: Trace {
            id: "T2".to_string(),
            start: 0,
            end: 30,
            inner: vec![
                Trace {
                    id: "R1".to_string(),
                    start: 10,
                    end: 20,
                    inner: vec![Trace {
                        id: "R2".to_string(),
                        start: 12,
                        end: 16,
                        inner: vec![],
                    }],
                },
                Trace {
                    id: "R1".to_string(),
                    start: 22,
                    end: 28,
                    inner: vec![Trace {
                        id: "R3".to_string(),
                        start: 23,
                        end: 30,
                        inner: vec![],
                    },],
                },
            ],
        },
    };

    // Task T3
    let t3 = Task {
        id: "T3".to_string(),
        prio: 3,
        deadline: 50,
        inter_arrival: 50,
        trace: Trace {
            id: "T3".to_string(),
            start: 0,
            end: 30,
            inner: vec![Trace {
                id: "R2".to_string(),
                start: 10,
                end: 20,
                inner: vec![],
            },
            Trace {
                id: "R3".to_string(),
                start: 22,
                end: 30,
                inner: vec![],
            }
            ],
        },
    };
    vec![t1, t2, t3]
}
#[tokio::main]
async fn main() {

    // builds a vector of tasks t1, t2, t3
    let tasks: Tasks = srp_analysis_example_setup();

    // println!("tasks {:?}", &tasks);
    // println!("tot_util {}", tot_util(&tasks));

    //let (ip, tr) = pre_analysis(&tasks);
    let l_tot = total_load_factor(&tasks).unwrap();

    // Putting my rust learning to test: it creates a Vec of all resources found for all tasks in Vec<Task>
    let _test: Vec<&Trace> = tasks.iter().map(|t| t.resources()).flatten().collect();

    // Printing all resource aquisitions during the program runtime, for testing my recursive resource iterator
    if tasks[0].resources().next().is_none() {
        println!("IS EMPTY");
    }

    println!("Resource count for {:?} is: {}", tasks[1].id, tasks[1].resources().collect::<Vec<_>>().len());

    for trace in tasks[2].resources() {
        println!("Task is: {:?}", tasks[2].id);
        println!("Trace: {:?}", trace);
        println!("Trace WCET: {}", trace.wcet());
        println!("Task prio: {}", tasks[2].prio)
    }
    
    //println!("ip: {:?}", ip);
    //println!("tr: {:?}", tr);
    println!("Ltot {}", l_tot);

    for t in tasks.iter(){
        println!("================ Task {:?} ================", t.id);
        println!("Interference: {}", t.interference(&tasks));
        println!("Busy period: {}", t.busy_period(&tasks));
        println!("Inter arrival: {}", t.inter_arrival);
        println!("Blocking time: {}", t.blocking_time(&tasks));
    }


    let app = Router::new()
        .route("/", get(index))
        .route("/:type/:name", get(render));

    axum::Server::bind(&"127.0.0.1:5555".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

}

// Use lazy_static to define the constant
lazy_static! {
    static ref FIRST_SET: BTreeMap<&'static str, fn() -> Chart> = {
        let mut s1: BTreeMap<&'static str, fn() -> Chart> = BTreeMap::new();
        s1.insert("chart1", chart1);
        s1
    };

    static ref SECOND_SET: BTreeMap<&'static str, fn() -> Chart> = {
        let mut s2: BTreeMap<&'static str, fn() -> Chart> = BTreeMap::new();
        s2.insert("chart2", chart2);
        s2
    };

    // BTreeMap of chars avaliable, will use the tempelate and then visualize them
    static ref CHARTS: BTreeMap<&'static str, BTreeMap<&'static str, fn() -> Chart>> = {
        let mut m = BTreeMap::new();
        m.insert("FIRST_SET" , FIRST_SET.clone());
        m.insert("SECOND_SET", SECOND_SET.clone());
        m
    };
}

async fn render(
    extract::Path((r#type, name)): extract::Path<(String, String)>,
) -> impl IntoResponse {
    let renderer = HtmlRenderer::new(format!("{type} - {name}"), 1000, 800);

    let chart = match CHARTS.get(r#type.as_str()) {
        Some(charts) => match charts.get(name.as_str()) {
            Some(chart) => chart(),
            None => return (StatusCode::NOT_FOUND, "Chart Not Found").into_response(),
        },
        None => return (StatusCode::NOT_FOUND, "Chart Type Not Found").into_response(),
    };
    Html(renderer.render(&chart).unwrap()).into_response()
}
// basic handler that responds with a static string
async fn root() -> impl IntoResponse {
    let renderer = HtmlRenderer::new("hello", 1920, 1080);
    let chart = chart1();
    Html(renderer.render(&chart).unwrap()).into_response()
}

// Make a more interactable intex with tempelates, uses render() for responses and new data.
async fn index() -> impl IntoResponse {
    let mut template = IndexTemplate::new();
    for (key, value) in CHARTS.iter() {
        template.collection(key, value.iter().map(|(k, _)| *k).collect::<Vec<_>>());
    }
    HtmlTemplate(template)
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    collections: Vec<(String, Vec<String>)>,
}

impl IndexTemplate {
    fn new() -> Self {
        Self {
            collections: vec![],
        }
    }

    fn collection(&mut self, name: &str, charts: Vec<&str>) {
        self.collections.push((
            name.to_string(),
            charts.into_iter().map(|s| s.to_string() ).collect(),
        ));
    }
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(body) => Html(body).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response(),
        }
    }
}

use charming::{
    component::{Axis, Grid, Legend},
    element::{
        AxisPointer, AxisPointerType, AxisType, Emphasis, EmphasisFocus, LineStyle, LineStyleType,
        MarkLine, MarkLineData, MarkLineVariant, Tooltip, Trigger,
    },
    series::{bar, Bar, Series},
    Chart,
};

pub fn chart1() -> Chart {
    Chart::new()
        .tooltip(
            Tooltip::new()
                .trigger(Trigger::Axis)
                .axis_pointer(AxisPointer::new().type_(AxisPointerType::Cross)),
        )
        .legend(Legend::new())
        .grid(
            Grid::new()
                .left("3%")
                .right("4%")
                .bottom("3%")
                .contain_label(true),
        )
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(vec!["ÄNDRAT", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
        )
        .y_axis(Axis::new().type_(AxisType::Value))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Direct")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![320, 332, 301, 334, 390, 330, 320]),
        ))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Email")
                .stack("Ad")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![120, 132, 101, 134, 90, 230, 210]),
        ))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Union Ads")
                .stack("Ad")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![220, 182, 191, 234, 290, 330, 310]),
        ))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Video Ads")
                .stack("Ad")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![150, 232, 201, 154, 190, 330, 410]),
        ))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Search Engine")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .mark_line(
                    MarkLine::new()
                        .line_style(LineStyle::new().type_(LineStyleType::Dashed))
                        .data(vec![MarkLineVariant::StartToEnd(
                            MarkLineData::new().type_("min"),
                            MarkLineData::new().type_("max"),
                        )]),
                )
                .data(vec![862, 1018, 964, 1026, 1679, 1600, 1570]),
        ))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Baidu")
                .bar_width(5)
                .stack("Search Engine")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![620, 732, 701, 734, 1090, 1130, 1120]),
        ))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Google")
                .stack("Search Engine")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![120, 132, 101, 134, 290, 230, 220]),
        ))
        .series(Series::Bar(
            bar::Bar::new()
                .name("Bing")
                .stack("Search Engine")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![60, 72, 71, 74, 190, 130, 110]),
        ))
        .series(
            Bar::new()
                .name("Others")
                .stack("Search Engine")
                .emphasis(Emphasis::new().focus(EmphasisFocus::Series))
                .data(vec![62, 82, 91, 84, 109, 110, 120]),
        )
}

use charming::{
    component::{Axis3D, Grid3D, VisualMap},
    datatype::{CompositeValue, Dataset},
    element::{DimensionEncode},
    series::Bar3d,
};

pub fn chart2() -> Chart {
    let data: Vec<Vec<CompositeValue>> =
        serde_json::from_str(include_str!("life-expectancy-table.json")).unwrap();

    Chart::new()
        .grid3d(Grid3D::new())
        .tooltip(Tooltip::new())
        .x_axis3d(Axis3D::new().type_(AxisType::Category))
        .y_axis3d(Axis3D::new().type_(AxisType::Category))
        .z_axis3d(Axis3D::new())
        .visual_map(VisualMap::new().max(1e8).dimension("Population"))
        .dataset(Dataset::new().source(data))
        .series(
            Bar3d::new().shading("lambert").encode(
                DimensionEncode::new()
                    .x("Year")
                    .y("Country")
                    .z("Life Expectancy")
                    .tooltip(vec![0, 1, 2, 3, 4]),
            ),
        )
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}