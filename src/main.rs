use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::prelude::*;
use warp::Filter;
use plotters::prelude::*;
use tokio_postgres::NoTls;


#[derive(Serialize, Deserialize)]
struct PerformanceTest {
    name: String,
    branch: String,
    build_number: i32,
    time: i32,
}

#[tokio::main]
async fn main() {
    // let start = Instant::now();
    // create_plot_file().expect("Couldn't create a plot file");
    // let duration = start.elapsed();
    // println!("Time elapsed in create_plot_file() is: {:?}", duration);

    let (client, connection) =
        tokio_postgres::connect("postgresql://postgres:example@localhost/", NoTls)
        .await
        .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let save_test = warp::path!("commit" / String / String / i32 / i32)
        .map(|name, branch, build, time| save_test_data(name, branch, build, time));

    warp::serve(save_test)
        .run(([127, 0, 0, 1], 7777))
        .await;
}

async fn prepare_database(client: &Client, db_name: &str) -> Result<(), Error> {
    let query = format!("CREATE DATABASE IF NOT EXISTS {}", db_name);
    client.execute(&query, &[]).await?;
    client.execute("CREATE DATABASE PerformanceTests", &[]).await.unwrap();
    Ok(())
}

fn save_test_data(name: String, branch: String, build_number: i32, time: i32) -> &'static str {
    let test = PerformanceTest { name, branch, build_number, time };

    let mut file_name: String = test.name.to_owned();
    file_name.push_str(".json");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(file_name)
        .expect("Unable to open file");

    let json = serde_json::to_string(&test).expect("Unable to serialize Person");
    writeln!(file, "{}", json).expect("Unable to write to file");
    return "hello";
}

fn create_plot_file() -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new("myPlot.png", (1000, 1000)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption("Performance Chart Demo", ("sans-serif", 40))
        .build_cartesian_2d(30.0f32..40.0f32, 0.0f32..100.0f32)?;

    chart.configure_mesh()
    .disable_x_mesh()
    .disable_y_mesh()
    .x_labels(30)
    .max_light_lines(4)
    .y_desc("Execution Time [ms]")
    .draw()?;

    chart.draw_series(LineSeries::new(
        DATA.iter().map(|(x, y)| (*x, *y)),
        &BLUE,
    ))?;
    chart.draw_series(
        DATA.iter()
            .map(|(y, m)| Circle::new((*y, *m), 5, BLUE.filled())),
    )?;
    Ok(())
}

const DATA: [(f32, f32); 10] = [
    (30.0, 32.4),
    (31.0, 37.5),
    (32.0, 44.5),
    (33.0, 50.3),
    (34.0, 55.0),
    (35.0, 70.0),
    (36.0, 78.7),
    (37.0, 76.5),
    (38.0, 68.9),
    (39.0, 56.3),
];