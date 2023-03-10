extern crate clap;

use clap::App;
use cli_table::{format::Justify, print_stdout, Color, Table, WithTitle};
use futures::stream::StreamExt;
use reqwest::Client;
use serde::Deserialize;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

// #[derive(Deserialize, Debug)]
// struct Meta {
//     code: String,
//     execution_time: String,
// }

#[derive(Deserialize, Debug)]
struct Timezone {
    id: String,
}

#[derive(Deserialize, Debug)]
struct Datetime {
    hour_12_wilz: String,
    hour_am_pm: String,
    minutes: String,
    seconds: String,
    offset_hours: String,
}

#[derive(Deserialize, Debug)]
struct Data {
    timezone: Timezone,
    datetime: Datetime,
}

#[derive(Deserialize, Debug)]
struct TimeZoneApiResponse {
    data: Data,
}

#[derive(Table, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct OutputFormat {
    #[table(
        title = "UTC offset",
        justify = "Justify::Left",
        color = "Color::Green"
    )]
    offset: i8,
    #[table(title = "Location", justify = "Justify::Left", color = "Color::Cyan")]
    time_zone: String,
    #[table(title = "Current time", color = "Color::Magenta")]
    current_time: String,
}

fn read_lines(path: &str) -> std::io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().filter_map(Result::ok).collect())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _matches = App::new("World Clock")
        .version("0.1.0")
        .author("Brendon Otto")
        .about("Grabs current time for configured timezones")
        // .arg(
        //     Arg::new("timezone")
        //         .short('t')
        //         .long("timezone")
        //         .takes_value(true)
        //         .about("Timezone to retrieve time"),
        // )
        .get_matches();

    let zones = [
        "Europe/Lisbon",
        "America/Fortaleza",
        "America/Detroit",
        "America/Chicago",
        "America/Denver",
        "America/Los_Angeles",
    ];

    //let zones2 = read_lines("worldclock.config");

    let api_key: String;
    match env::var("TZ_API_KEY") {
        Ok(key) => api_key = key,
        _ => panic!("API key not found!"),
    };

    let mut zones_urls: Vec<String> = Vec::new();

    for zone in &zones {
        let url = format!(
            "https://timezoneapi.io/api/timezone/?{}&token={}",
            zone, api_key
        );
        zones_urls.push(url);
    }

    // Build a single client instance so that connections can be pooled
    let client = Client::builder().build()?;

    // This is gnarly but it maps each url to a GET request using the client
    // and then moves the processing of the result to a new thread
    // and extracts the data into the proper format. It then returns the
    // collected OutputFormats as a list into the fetches variable.
    let mut fetches = futures::stream::iter(zones_urls.into_iter().map(|zone| {
        let send_fut = client.get(&zone).send();
        async move {
            match send_fut.await {
                Ok(resp) => match resp.json::<TimeZoneApiResponse>().await {
                    Ok(detail) => OutputFormat {
                        time_zone: detail.data.timezone.id,
                        current_time: format!(
                            "{}:{}:{} {}",
                            detail.data.datetime.hour_12_wilz,
                            detail.data.datetime.minutes,
                            detail.data.datetime.seconds,
                            detail.data.datetime.hour_am_pm
                        ),
                        offset: detail.data.datetime.offset_hours.parse::<i8>().unwrap(),
                    },
                    Err(_) => panic!("Error happened {:?}", zone),
                },
                Err(_) => panic!("Error happened {:?}", zone),
            }
        }
    }))
    .buffer_unordered(zones.len())
    .collect::<Vec<OutputFormat>>()
    .await;

    fetches.sort_by(|a, b| b.offset.cmp(&a.offset));

    match print_stdout(fetches.with_title()) {
        Ok(_) => Ok(()),
        Err(err) => {
            panic!("Something went wrong {:?}", err);
        }
    }
}
