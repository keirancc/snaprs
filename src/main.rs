use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use structopt::StructOpt;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct Message {
    From: String,
    #[serde(rename = "Media Type")]
    media_type: String,
    Created: String,
    Content: Option<String>,
    #[serde(rename = "Conversation Title")]
    conversation_title: Option<String>,
    IsSender: bool,
    #[serde(rename = "Created(microseconds)")]
    created_microseconds: i64,
    IsSaved: bool,
}

#[derive(Debug, Deserialize)]
struct ChatData(HashMap<String, Vec<Message>>);

#[derive(StructOpt, Debug)]
#[structopt(name = "snapchat-analyzer", about = "Analyze Snapchat chat data")]
struct Opt {
    #[structopt(short, long)]
    input: String,

    #[structopt(short, long)]
    user: Option<String>,

    #[structopt(long)]
    from_date: Option<String>,

    #[structopt(long)]
    to_date: Option<String>,

    #[structopt(short, long)]
    detailed: bool,

    #[structopt(long)]
    saved_only: bool,

    #[structopt(long)]
    media_type: Option<String>,
}

struct Statistics {
    total_messages: usize,
    messages_sent: usize,
    messages_received: usize,
    saved_messages: usize,
    media_type_counts: HashMap<String, usize>,
    users_interaction_counts: HashMap<String, (usize, usize)>, // (sent, received)
    earliest_message: Option<DateTime<Utc>>,
    latest_message: Option<DateTime<Utc>>,
}

impl Statistics {
    fn new() -> Self {
        Statistics {
            total_messages: 0,
            messages_sent: 0,
            messages_received: 0,
            saved_messages: 0,
            media_type_counts: HashMap::new(),
            users_interaction_counts: HashMap::new(),
            earliest_message: None,
            latest_message: None,
        }
    }

    fn update_time_range(&mut self, created: &str) {
        if let Ok(dt) = DateTime::parse_from_str(created, "%Y-%m-%d %H:%M:%S %Z") {
            let utc_dt = dt.with_timezone(&Utc);
            match (self.earliest_message, self.latest_message) {
                (None, None) => {
                    self.earliest_message = Some(utc_dt);
                    self.latest_message = Some(utc_dt);
                }
                _ => {
                    if self.earliest_message.map_or(true, |t| utc_dt < t) {
                        self.earliest_message = Some(utc_dt);
                    }
                    if self.latest_message.map_or(true, |t| utc_dt > t) {
                        self.latest_message = Some(utc_dt);
                    }
                }
            }
        }
    }
}

fn analyze_messages(data: &ChatData, opt: &Opt) -> Statistics {
    let mut stats = Statistics::new();
    
    for (_, messages) in data.0.iter() {
        for msg in messages {
            if let Some(ref user) = opt.user {
                if (!msg.IsSender && msg.From != *user) && (msg.IsSender && msg.From == *user) {
                    continue;
                }
            }

            if let Some(ref from_date) = opt.from_date {
                if msg.Created.split_whitespace().next().unwrap() < from_date {
                    continue;
                }
            }

            if let Some(ref to_date) = opt.to_date {
                if msg.Created.split_whitespace().next().unwrap() > to_date {
                    continue;
                }
            }

            if opt.saved_only && !msg.IsSaved {
                continue;
            }

            if let Some(ref media_type) = opt.media_type {
                if msg.media_type != *media_type {
                    continue;
                }
            }

            stats.total_messages += 1;
            if msg.IsSender {
                stats.messages_received += 1;
            } else {
                stats.messages_sent += 1;
            }

            if msg.IsSaved {
                stats.saved_messages += 1;
            }

            *stats.media_type_counts.entry(msg.media_type.clone()).or_insert(0) += 1;

            let (sent, received) = stats.users_interaction_counts
                .entry(msg.From.clone())
                .or_insert((0, 0));
            if msg.IsSender {
                *received += 1;
            } else {
                *sent += 1;
            }

            stats.update_time_range(&msg.Created);
        }
    }

    stats
}

fn print_statistics(stats: &Statistics, opt: &Opt) {
    println!("\nSnapchat Chat Statistics:");
    println!("-------------------------");
    println!("Total messages: {}", stats.total_messages);
    println!("Messages sent: {}", stats.messages_sent);
    println!("Messages received: {}", stats.messages_received);
    println!("Saved messages: {}", stats.saved_messages);

    if let (Some(earliest), Some(latest)) = (stats.earliest_message, stats.latest_message) {
        let duration = latest.signed_duration_since(earliest);
        let days = duration.num_days();
        if days > 0 {
            println!("\nDate range: {} days", days);
            println!("Average messages per day: {:.2}", stats.total_messages as f64 / days as f64);
        }
    }

    if opt.detailed {
        println!("\nMedia Type Breakdown:");
        for (media_type, count) in &stats.media_type_counts {
            println!("  {}: {}", media_type, count);
        }

        println!("\nUser Interaction Breakdown:");
        for (user, (sent, received)) in &stats.users_interaction_counts {
            println!("  {}:", user);
            println!("    Sent: {}", sent);
            println!("    Received: {}", received);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let data_str = fs::read_to_string(&opt.input)?;
    let chat_data: ChatData = serde_json::from_str(&data_str)?;

    let stats = analyze_messages(&chat_data, &opt);

    print_statistics(&stats, &opt);

    Ok(())
}
