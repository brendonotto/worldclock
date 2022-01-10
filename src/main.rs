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

#[derive(Deserialize, Debug)]
struct Meta {
    code: String,
    execution_time: String,
}

#[derive(Deserialize, Debug)]
struct Timezone {
    id: String,
    location: String,
    country_code: String,
    country_name: String,
    iso3166_1_alpha_2: String,
    iso3166_1_alpha_3: String,
    un_m49_code: String,
    itu: String,
    marc: String,
    wmo: String,
    ds: String,
    phone_prefix: String,
    fifa: String,
    fips: String,
    gual: String,
    ioc: String,
    currency_alpha_code: String,
    currency_country_name: String,
    currency_minor_unit: String,
    currency_name: String,
    currency_code: String,
    independent: String,
    capital: String,
    continent: String,
    tld: String,
    languages: String,
    geoname_id: String,
    edgar: String,
}

#[derive(Deserialize, Debug)]
struct Datetime {
    date: String,
    date_time: String,
    date_time_txt: String,
    date_time_wti: String,
    date_time_ymd: String,
    time: String,
    month: String,
    month_wilz: String,
    month_abbr: String,
    month_full: String,
    month_days: String,
    day: String,
    day_wilz: String,
    day_abbr: String,
    day_full: String,
    year: String,
    year_abbr: String,
    hour_12_wolz: String,
    hour_12_wilz: String,
    hour_24_wolz: String,
    hour_24_wilz: String,
    hour_am_pm: String,
    minutes: String,
    seconds: String,
    week: String,
    offset_seconds: String,
    offset_minutes: String,
    offset_hours: String,
    offset_gmt: String,
    offset_tzid: String,
    offset_tzab: String,
    offset_tzfull: Option<String>,
    tz_string: String,
    dst: String,
    dst_observes: String,
    timeday_spe: String,
    timeday_gen: String,
}

#[derive(Deserialize, Debug)]
struct Data {
    timezone: Timezone,
    datetime: Datetime,
}

#[derive(Deserialize, Debug)]
struct TimeZoneApiResponse {
    meta: Meta,
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

    let zones2 = read_lines("worldclock.config");

    let api_key: String;
    match env::var("TZ_API_KEY") {
        Ok(key) => api_key = key,
        _ => panic!("API key not found!")
    };

    let mut zones_urls: Vec<String> = Vec::new();

    for zone in &zones {
        let url = format!(
            "https://timezoneapi.io/api/timezone/?{}&token={}",
            zone,
            api_key
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
                    Ok(detail) => {
                        let output = OutputFormat {
                            time_zone: detail.data.timezone.id,
                            current_time: format!(
                                "{}:{}:{} {}",
                                detail.data.datetime.hour_12_wilz,
                                detail.data.datetime.minutes,
                                detail.data.datetime.seconds,
                                detail.data.datetime.hour_am_pm
                            ),
                            offset: detail.data.datetime.offset_hours.parse::<i8>().unwrap(),
                        };
                        output
                    }
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
