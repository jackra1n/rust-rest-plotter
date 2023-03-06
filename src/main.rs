use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fs::{OpenOptions, self};
use warp::Filter;
use warp::http::StatusCode;
use plotters::prelude::*;
use tokio_postgres::{Client, Connection, NoTls, Error};

const DATABASE_URL: &str = "postgresql://postgres:example@localhost/";
const INIT_SQL: &str = "CreateDatabase.sql";

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
    let db_url = format!("{}{}",DATABASE_URL, "PerformanceTests");
    let (client, connection) = tokio_postgres::connect(&db_url, NoTls)
        .await
        .expect("Failed to connect to Postgres");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let sql = fs::read_to_string(INIT_SQL).expect("Error while trying to read 'CreateDatabase.sql'");
    client.batch_execute(&sql).await.expect("Couldnt execute sql statement");

    let save_test_endpoint = warp::path!("commit" / String / String / i32 / i32)
        .and_then(save_test_data);

    warp::serve(save_test_endpoint)
        .run(([127, 0, 0, 1], 7777))
        .await;
}

async fn create_connection() -> Result<Client, Error> {
    let db_url = format!("{}{}",DATABASE_URL, "PerformanceTests");
    let (client, conn) = tokio_postgres::connect(&db_url, NoTls).await.expect("connect");
    tokio::spawn(conn);
    Ok(client)
}

pub async fn save_test_data(name: String, branch: String, build_number: i32, time: i32) -> Result<impl warp::Reply, Infallible> {
    let client = create_connection().await.unwrap();
    // let query = "INSERT INTO DefaultTests(name, branch, build_number, runtime) VALUES ($1, $2, $3, $4)";
    let query = format!("INSERT INTO DefaultTests(name, branch, build_number, runtime) VALUES ('{}', '{}', {}, interval '{} seconds')", name, branch, build_number, time);
    client.execute(&query, &[]).await.expect("Couldnt insert into DB");

    Ok(StatusCode::OK)
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