use serde::{Deserialize, Serialize};
use std::{convert::Infallible};
use std::fs;
use warp::Filter;
use warp::http::StatusCode;
use plotters::prelude::*;
use tokio_postgres::{Client, NoTls};



const DATABASE_URL: &str = "postgresql://postgres:example@localhost/";
const INIT_SQL: &str = "CreateDatabase.sql";

#[derive(Serialize, Deserialize)]
struct PerformanceTest {
    name: String,
    branch: String,
    build_number: i64,
    time: i64,
}


#[tokio::main]
async fn main() {
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

    let save_test_endpoint = warp::path!("commit" / String / String / i64 / i64)
        .and_then(save_test_data);

    let show_test_endpoint = warp::path!("show")
        .and_then(show_test_data);

    let show_plot_endpoint = warp::path!("plot")
        .and_then(show_plot);

    let app = save_test_endpoint.or(show_test_endpoint).or(show_plot_endpoint);

    warp::serve(app)
        .run(([127, 0, 0, 1], 7777))
        .await;
}

async fn create_connection() -> Result<Client, tokio_postgres::Error> {
    let db_url = format!("{}{}",DATABASE_URL, "PerformanceTests");
    let (client, conn) = tokio_postgres::connect(&db_url, NoTls).await.expect("connection error");
    tokio::spawn(conn);
    Ok(client)
}

pub async fn show_test_data() -> Result<impl warp::Reply, Infallible> {
    let client = create_connection().await.unwrap();
    let query = "SELECT * FROM DefaultTests";
    let rows = client.query(query, &[]).await.expect("Select query did not succeed");
    
    let mut tests: Vec<PerformanceTest> = Vec::new();
    for row in rows {
        let test = PerformanceTest {
            name: row.get("name"),
            branch: row.get("branch"),
            build_number: row.get("build_number"),
            time: row.get("runtime"),
        };
        tests.push(test);
    }
    Ok(warp::reply::with_status(
        warp::reply::json(&tests),
        StatusCode::OK,
    ))
}

pub async fn save_test_data(name: String, branch: String, build_number: i64, time: i64) -> Result<impl warp::Reply, Infallible> {
    let client = create_connection().await.unwrap();
    let query = "INSERT INTO DefaultTests(name, branch, build_number, runtime) VALUES ($1, $2, $3, $4)";
    let result = client.execute(query, &[&name, &branch, &build_number, &time]).await;
    match result {
        Ok(_) => Ok(warp::reply::with_status(
            "inserting was succesfull",
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            "fail",
            StatusCode::METHOD_NOT_ALLOWED,
        ))
    }
}

pub async fn show_plot() -> Result<impl warp::Reply, Infallible> {
    let client = create_connection().await.unwrap();
    let query = "SELECT * FROM DefaultTests";
    let rows = client.query(query, &[]).await.expect("Select query did not succeed");

    let prepared_data = rows.iter().map(|row| (row.get("build_number"), row.get("runtime"))).collect();

    let plot_name = "plot.png";
    create_plot_file(plot_name.to_owned(), prepared_data).unwrap();
    let plot = fs::read(plot_name).unwrap();
    Ok(warp::reply::with_status(
        warp::reply::with_header(plot, "Content-Type", "image/png"),
        StatusCode::OK,
    ))
}

fn create_plot_file(file_name: String, data: Vec<(i64, i64)>) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(&file_name, (1000, 1000)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption("Performance Chart Demo", ("sans-serif", 40))
        .build_cartesian_2d(30i64..40i64, 0i64..2500i64)?;

    chart.configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_labels(30)
        .max_light_lines(4)
        .y_desc("Execution Time [ms]")
        .draw()?;

    chart.draw_series(LineSeries::new(
        data.iter().map(|(x, y)| (*x, *y)),
        &BLUE,
    ))?;
    chart.draw_series(
        data.iter()
            .map(|(y, m)| Circle::new((*y, *m), 5, BLUE.filled())),
    )?;
    Ok(())
}