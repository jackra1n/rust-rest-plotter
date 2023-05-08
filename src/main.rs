use plotters::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fs;
use tokio_postgres::{connect, Client, NoTls};
use warp::http::StatusCode;
use warp::Filter;
extern crate pretty_env_logger;

#[macro_use]
extern crate log;

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
    pretty_env_logger::init();
    let db_url = format!("{}{}", DATABASE_URL, "PerformanceTests");
    let (client, connection) = connect(&db_url, NoTls).await.expect("connection error");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("connection error: {}", e);
        }
    });

    let sql = fs::read_to_string(INIT_SQL).unwrap();
    client
        .batch_execute(&sql)
        .await
        .expect("Couldnt execute sql statement");

    let save_test_endpoint =
        warp::path!("commit" / String / String / i64 / i64).and_then(save_test_data);

    let show_test_endpoint = warp::path!("show").and_then(show_test_data);
    let show_plot_endpoint = 
		warp::path!("plot" / String / i64 / i64).and_then(show_plot);
	let generate_test_data_endpoint = warp::path!("generateTestData").and_then(generate_test_data);

    let app = save_test_endpoint
        .or(show_test_endpoint)
        .or(show_plot_endpoint)
		.or(generate_test_data_endpoint);

    warp::serve(app).run(([127, 0, 0, 1], 7777)).await;
}

async fn create_connection() -> Result<Client, tokio_postgres::Error> {
    let db_url = format!("{}{}", DATABASE_URL, "PerformanceTests");
    let (client, conn) = tokio_postgres::connect(&db_url, NoTls)
        .await
        .expect("connection error");
    tokio::spawn(conn);
    Ok(client)
}

pub async fn show_test_data() -> Result<impl warp::Reply, Infallible> {
    let client = create_connection().await.unwrap();
    let query = "SELECT * FROM DefaultTests";
    let rows = client
        .query(query, &[])
        .await
        .expect("Select query did not succeed");

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

pub async fn save_test_data(
    name: String,
    branch: String,
    build_number: i64,
    time: i64,
) -> Result<impl warp::Reply, Infallible> {
    let client = create_connection().await.unwrap();
    let query =
        "INSERT INTO DefaultTests(name, branch, build_number, runtime) VALUES ($1, $2, $3, $4)";
    let result = client
        .execute(query, &[&name, &branch, &build_number, &time])
        .await;
    match result {
        Ok(_) => Ok(warp::reply::with_status(
            "inserting was succesfull",
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            "fail",
            StatusCode::METHOD_NOT_ALLOWED,
        )),
    }
}

pub async fn show_plot(
	test_name: String,
	from_build: i64,
	mut build_count: i64,
) -> Result<impl warp::Reply, Infallible> {
	if build_count > 100 {
		build_count = 100;
	}

    let client = create_connection().await.unwrap();

    let query = "SELECT * FROM DefaultTests WHERE name = $1 AND build_number >= $2 AND build_number <= $3";
    let rows = client
        .query(query, &[&test_name, &from_build, &(from_build + build_count)])
        .await
        .expect("Select query did not succeed");

    let prepared_data = rows
        .iter()
        .map(|row| (row.get("build_number"), row.get("runtime")))
        .collect();

    let plot_name = format!("{}.png", test_name);
    create_plot_file(test_name, prepared_data).unwrap();
    let plot = fs::read(plot_name).unwrap();
    Ok(warp::reply::with_status(
        warp::reply::with_header(plot, "Content-Type", "image/png"),
        StatusCode::OK,
    ))
}

pub async fn generate_test_data() -> Result<impl warp::Reply, Infallible> {
	let name = "ivy-default-case";
	let branch = "master";
	for n in 1..31 {
		let time = rand::thread_rng().gen_range(90..110);
		save_test_data(name.to_string(), branch.to_string(), n, time).await;
	}
	Ok(warp::reply::with_status(
		"generating test data was succesfull",
		StatusCode::OK,
	))
}

fn create_plot_file(
    test_name: String,
    data: Vec<(i64, i64)>,
) -> Result<(), Box<dyn std::error::Error>> {
	let file_name = format!("{}.png", test_name);
    let root = BitMapBackend::new(&file_name, (1000, 1000)).into_drawing_area();

    root.fill(&WHITE)?;

    let max_y = data.iter().map(|(_, y)| y).max().unwrap_or(&0);
	let min_x = data.iter().map(|(x, _)| x).min().unwrap_or(&0);
	let max_x = data.iter().map(|(x, _)| x).max().unwrap_or(&0);

	let max_y = max_y + (max_y / 10);

    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption(test_name, ("sans-serif", 40))
        .build_cartesian_2d(*min_x..*max_x, 0i64..max_y)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(30)
        .max_light_lines(3)
        .y_desc("Execution Time [ms]")
        .draw()?;

    chart.draw_series(LineSeries::new(data.iter().map(|(x, y)| (*x, *y)), &BLUE))?;
    chart.draw_series(
        data.iter()
            .map(|(y, m)| Circle::new((*y, *m), 4, BLUE.filled())),
    )?;
    Ok(())
}
