use std::{
    collections::{HashSet, VecDeque},
    io::Write,
};

use csv::Reader;
use itertools::Itertools;
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer,
};

static RESULT_TEMPLATE: &'static str = include_str!("template.html");

fn bool_from_str<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer)?.to_lowercase().as_str() {
        "t" | "true" | "1" | "on" | "y" | "yes" => Ok(true),
        "f" | "false" | "0" | "off" | "n" | "no" => Ok(false),
        other => Err(de::Error::invalid_value(
            Unexpected::Str(other),
            &"Must be truthy (t, true, 1, on, y, yes) or falsey (f, false, 0, off, n, no)",
        )),
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Hash)]
struct User {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "RaffleTickets")]
    tickets: u32,
    #[serde(rename = "Submitted", deserialize_with = "bool_from_str")]
    submitted: bool,
    #[serde(rename = "WonPrize", deserialize_with = "bool_from_str")]
    won_prize: bool,
}

macro_rules! get_winner {
    ($tickets:expr, $winners:expr, $insert:expr) => {{
        let winner = $tickets.pop_front().unwrap();
        if $winners.contains(&winner.id) {
            continue;
        }
        if $insert {
            $winners.insert(winner.id);
        }
        winner
    }};
    ($tickets:expr, $winners:expr) => {
        get_winner!($tickets, $winners, true)
    };
}

fn main() {
    let seed = std::env::args().nth(1).expect("Expected seed as argument");
    let seed = u64::from_str_radix(&seed, 16).expect("Failed to parse seed");
    eprintln!("Using seed 0x{:x}", seed);

    let users = Reader::from_reader(&include_bytes!("raffle.csv")[..])
        .into_deserialize::<User>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();

    let mut tickets = Vec::<&User>::new();
    for record in users.iter() {
        for _ in 0..record.tickets {
            tickets.push(record);
        }
    }
    tickets.shuffle(&mut ChaCha12Rng::seed_from_u64(seed));

    let mut result = RESULT_TEMPLATE.to_owned();
    let mut tickets = tickets.into_iter().collect::<VecDeque<_>>();
    let mut skipped_tickets = VecDeque::<&User>::new();
    let mut winners = HashSet::<u32>::new();

    let mut count = 0;
    while count < 3 {
        let winner = get_winner!(tickets, winners);
        result = result.replacen("{{ANY}}", &format!("#{}", winner.id), 1);
        count += 1;
    }

    let mut count = 0;
    while count < 19 {
        let winner = get_winner!(tickets, winners, false);
        if winner.won_prize || !winner.submitted {
            skipped_tickets.push_back(winner);
            continue;
        }
        winners.insert(winner.id);
        result = result.replacen("{{NPW}}", &format!("#{}", winner.id), 1);
        count += 1;
    }

    let grab_vouchers = 49usize;
    let fp_vouchers = 356usize;
    let vouchers = fp_vouchers + grab_vouchers;

    let mut voucher_winners = Vec::<u32>::new();
    while voucher_winners.len() < vouchers && !skipped_tickets.is_empty() {
        let winner = get_winner!(skipped_tickets, winners);
        voucher_winners.push(winner.id);
    }
    while voucher_winners.len() < vouchers {
        let winner = get_winner!(tickets, winners);
        voucher_winners.push(winner.id);
    }
    result = result.replacen(
        "{{GF10}}",
        &voucher_winners[..grab_vouchers]
            .iter()
            .map(|i| i.to_string())
            .join(" "),
        1,
    );
    result = result.replacen(
        "{{FP5}}",
        &voucher_winners[grab_vouchers..]
            .iter()
            .map(|i| i.to_string())
            .join(" "),
        1,
    );
    assert_eq!(winners.len(), 3 + 19 + vouchers);
    std::io::stdout()
        .lock()
        .write_all(result.as_bytes())
        .unwrap();
}
